// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use std::collections::BTreeMap;

use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};

use super::model::{Comparison, Expr, Helper, Predicate, Program, RuntimeType, MAX_DEPTH};
use super::{EvaluationError as Error, EvaluationLimits, EvaluationResult};

// Fixed interpreter storage units; Rust layout and pointer width cannot alter cost.
const VALUE_CELL_BYTES: u64 = 64;

struct Meter {
    limits: EvaluationLimits,
    steps: u64,
    allocated: u64,
}

impl Meter {
    fn step(&mut self, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
        self.steps = self.steps.checked_add(1).ok_or(Error::StepBudgetExceeded)?;
        if self.steps > self.limits.max_steps {
            return Err(Error::StepBudgetExceeded);
        }
        Ok(())
    }

    fn allocate(&mut self, bytes: u64) -> Result<(), Error> {
        self.allocated = self
            .allocated
            .checked_add(bytes)
            .ok_or(Error::AllocationBudgetExceeded)?;
        if self.allocated > self.limits.max_allocated_bytes {
            return Err(Error::AllocationBudgetExceeded);
        }
        Ok(())
    }

    fn copy(&mut self, value: &Value) -> Result<Value, Error> {
        self.charge(value, 0)?;
        Ok(value.clone())
    }

    fn charge(&mut self, value: &Value, depth: usize) -> Result<(), Error> {
        self.step(depth)?;
        self.allocate(VALUE_CELL_BYTES)?;
        match value {
            Value::Bytes(value) => self.allocate(value.len() as u64),
            Value::Text(value) => self.allocate(value.len() as u64),
            Value::Array(values) => {
                for value in values {
                    self.charge(value, depth + 1)?;
                }
                Ok(())
            }
            Value::Map(values) => {
                for (key, value) in values {
                    self.charge(key, depth + 1)?;
                    self.charge(value, depth + 1)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

pub(super) fn run(program: &Program, input: Value) -> Result<EvaluationResult, Error> {
    let mut meter = Meter {
        limits: program.limits,
        steps: 0,
        allocated: 0,
    };
    validate(&input, &program.input_type, &mut meter, 0).map_err(|error| {
        if error == Error::InvalidArtifact {
            Error::InvalidInput
        } else {
            error
        }
    })?;
    meter.charge(&input, 0)?;
    let mut locals = BTreeMap::from([(program.input_id.clone(), input)]);
    for (coordinate, constraint) in &program.constraints {
        if !predicate(constraint, &locals, &program.helpers, &mut meter, 0)? {
            return Err(Error::InputConstraintFailed(coordinate.clone()));
        }
    }
    for binding in &program.bindings {
        let value = expression(&binding.value, &locals, &program.helpers, &mut meter, 0)?;
        validate(&value, &binding.ty, &mut meter, 0)?;
        locals.insert(binding.id.clone(), value);
    }
    let result = expression(&program.result, &locals, &program.helpers, &mut meter, 0)?;
    validate(&result, &program.output_type, &mut meter, 0)?;
    // Encoding scratch is charged separately from the materialized result.
    meter.charge(&result, 0)?;
    let output = encode(&result).map_err(|_| Error::InvalidArtifact)?;
    if output.len() as u64 > program.limits.max_output_bytes {
        return Err(Error::OutputBudgetExceeded);
    }
    Ok(EvaluationResult {
        output,
        steps: meter.steps,
        allocated_bytes: meter.allocated,
    })
}

fn expression(
    expr: &Expr,
    locals: &BTreeMap<String, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<Value, Error> {
    meter.step(depth)?;
    match expr {
        Expr::Constant(value) => meter.copy(value),
        Expr::Local(id) => meter.copy(locals.get(id).ok_or(Error::InvalidArtifact)?),
        Expr::Field(base, name) => {
            let record = expression(base, locals, helpers, meter, depth + 1)?;
            let Value::Map(fields) = record else {
                return Err(Error::InvalidArtifact);
            };
            // Move the selected value, retaining the charge for all intermediate copies.
            fields
                .into_iter()
                .find_map(|(key, value)| {
                    if matches!(key, Value::Text(key) if key == *name) {
                        Some(value)
                    } else {
                        None
                    }
                })
                .ok_or(Error::InvalidArtifact)
        }
        Expr::Record(fields) => {
            meter.allocate(VALUE_CELL_BYTES)?;
            let mut values = Vec::new();
            for (key, value) in fields {
                meter.allocate(VALUE_CELL_BYTES + key.len() as u64)?;
                values.push((
                    Value::Text(key.clone()),
                    expression(value, locals, helpers, meter, depth + 1)?,
                ));
            }
            Ok(Value::Map(values))
        }
        Expr::If(condition, yes, no) => {
            let branch = if predicate(condition, locals, helpers, meter, depth + 1)? {
                yes
            } else {
                no
            };
            expression(branch, locals, helpers, meter, depth + 1)
        }
        Expr::Call(name) => {
            let helper = helpers.get(name).ok_or(Error::UnsupportedProgram)?;
            // A helper has its own lexical scope, never the caller's local bindings.
            let value = expression(&helper.result, &BTreeMap::new(), helpers, meter, depth + 1)?;
            validate(&value, &helper.ty, meter, depth + 1)?;
            Ok(value)
        }
    }
}

fn predicate(
    predicate: &Predicate,
    locals: &BTreeMap<String, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<bool, Error> {
    meter.step(depth)?;
    let left = expression(&predicate.left, locals, helpers, meter, depth + 1)?;
    let right = expression(&predicate.right, locals, helpers, meter, depth + 1)?;
    let (Value::Integer(left), Value::Integer(right)) = (left, right) else {
        return Err(Error::UnsupportedProgram);
    };
    Ok(match predicate.op {
        Comparison::Equal => left == right,
        Comparison::LessOrEqual => left <= right,
    })
}

fn validate(value: &Value, ty: &RuntimeType, meter: &mut Meter, depth: usize) -> Result<(), Error> {
    meter.step(depth)?;
    match (value, ty) {
        (Value::Integer(value), RuntimeType::Unsigned(max))
            if *value >= 0 && *value <= i128::from(*max) =>
        {
            Ok(())
        }
        (Value::Bytes(value), RuntimeType::Bytes { min, max })
            if (value.len() as u64) >= *min && (value.len() as u64) <= *max =>
        {
            Ok(())
        }
        (Value::Map(fields), RuntimeType::Record(types)) if fields.len() == types.len() => {
            for (name, ty) in types {
                let value = fields
                    .iter()
                    .find_map(|(key, value)| {
                        if matches!(key, Value::Text(key) if key == name) {
                            Some(value)
                        } else {
                            None
                        }
                    })
                    .ok_or(Error::InvalidArtifact)?;
                validate(value, ty, meter, depth + 1)?;
            }
            Ok(())
        }
        _ => Err(Error::InvalidArtifact),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_storage_meter_uses_architecture_independent_units() -> Result<(), Error> {
        let mut meter = Meter {
            limits: EvaluationLimits {
                max_package_bytes: 1024,
                max_input_bytes: 1024,
                max_steps: 100,
                max_allocated_bytes: 1024,
                max_output_bytes: 1024,
            },
            steps: 0,
            allocated: 0,
        };
        let value = Value::Map(vec![(
            Value::Text("id".into()),
            Value::Bytes(vec![1, 2, 3]),
        )]);
        meter.charge(&value, 0)?;
        // Three fixed 64-byte value cells plus two key bytes and three data bytes.
        assert_eq!(meter.allocated, 197);
        assert_eq!(meter.steps, 3);
        Ok(())
    }
}
