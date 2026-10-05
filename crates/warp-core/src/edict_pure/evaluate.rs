// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use std::collections::BTreeMap;

use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};

use super::model::{Comparison, Expr, Helper, Predicate, Program, RuntimeType, MAX_DEPTH};
use super::{EvaluationError as Error, EvaluationLimits, EvaluationResult};

// Fixed interpreter storage units; Rust layout and pointer width cannot alter cost.
pub(crate) const VALUE_CELL_BYTES: u64 = 64;

pub(crate) struct Meter {
    limits: EvaluationLimits,
    steps: u64,
    allocated: u64,
}

impl Meter {
    pub(crate) fn new(limits: EvaluationLimits) -> Self {
        Self {
            limits,
            steps: 0,
            allocated: 0,
        }
    }

    pub(crate) const fn usage(&self) -> (u64, u64) {
        (self.steps, self.allocated)
    }

    pub(crate) fn step(&mut self, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
        self.charge_steps(1)
    }

    pub(crate) fn charge_steps(&mut self, count: u64) -> Result<(), Error> {
        self.steps = self
            .steps
            .checked_add(count)
            .ok_or(Error::StepBudgetExceeded)?;
        if self.steps > self.limits.max_steps {
            return Err(Error::StepBudgetExceeded);
        }
        Ok(())
    }

    pub(crate) fn allocate(&mut self, bytes: u64) -> Result<(), Error> {
        self.allocated = self
            .allocated
            .checked_add(bytes)
            .ok_or(Error::AllocationBudgetExceeded)?;
        if self.allocated > self.limits.max_allocated_bytes {
            return Err(Error::AllocationBudgetExceeded);
        }
        Ok(())
    }

    pub(crate) fn copy(&mut self, value: &Value) -> Result<Value, Error> {
        self.charge(value, 0)?;
        Ok(value.clone())
    }

    pub(crate) fn charge(&mut self, value: &Value, depth: usize) -> Result<(), Error> {
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

pub(crate) fn expression(
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
        Expr::UnsignedSubtract { max, left, right } => {
            let left = expression(left, locals, helpers, meter, depth + 1)?;
            let right = expression(right, locals, helpers, meter, depth + 1)?;
            let ty = RuntimeType::Unsigned(*max);
            validate(&left, &ty, meter, depth + 1)?;
            validate(&right, &ty, meter, depth + 1)?;
            let (Value::Integer(left), Value::Integer(right)) = (left, right) else {
                return Err(Error::InvalidArtifact);
            };
            let difference = left
                .checked_sub(right)
                .filter(|value| *value >= 0)
                .ok_or(Error::InvalidArtifact)?;
            meter.copy(&Value::Integer(difference))
        }
        Expr::ByteLength { min, max, value } => {
            let value = expression(value, locals, helpers, meter, depth + 1)?;
            validate(
                &value,
                &RuntimeType::Bytes {
                    min: *min,
                    max: *max,
                },
                meter,
                depth + 1,
            )?;
            let Value::Bytes(bytes) = value else {
                return Err(Error::InvalidArtifact);
            };
            let length = u64::try_from(bytes.len()).map_err(|_| Error::InvalidArtifact)?;
            meter.copy(&Value::Integer(i128::from(length)))
        }
    }
}

pub(crate) fn predicate(
    predicate: &Predicate,
    locals: &BTreeMap<String, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<bool, Error> {
    meter.step(depth)?;
    let left = expression(&predicate.left, locals, helpers, meter, depth + 1)?;
    let right = expression(&predicate.right, locals, helpers, meter, depth + 1)?;
    match (&predicate.op, left, right) {
        (Comparison::Equal, Value::Integer(left), Value::Integer(right)) => Ok(left == right),
        (Comparison::LessOrEqual, Value::Integer(left), Value::Integer(right)) => Ok(left <= right),
        (Comparison::Equal, Value::Bytes(left), Value::Bytes(right)) => {
            // Charge the full operand aperture before comparing, including unequal
            // lengths. Cost cannot depend on a library's short-circuit behavior.
            let count = u64::try_from(left.len().max(right.len()))
                .map_err(|_| Error::StepBudgetExceeded)?;
            meter.charge_steps(count)?;
            Ok(left == right)
        }
        _ => Err(Error::UnsupportedProgram),
    }
}

pub(crate) fn validate(
    value: &Value,
    ty: &RuntimeType,
    meter: &mut Meter,
    depth: usize,
) -> Result<(), Error> {
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
    fn byte_length_charges_operand_validation_and_u64_result() -> Result<(), Error> {
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
        let length = Expr::ByteLength {
            min: 0,
            max: 32,
            value: Box::new(Expr::Constant(Value::Bytes(vec![0, 1, 255]))),
        };
        assert_eq!(
            expression(&length, &BTreeMap::new(), &BTreeMap::new(), &mut meter, 0)?,
            Value::Integer(3)
        );
        // Operator, operand visit/copy, byte-bound validation, result copy.
        assert_eq!(meter.steps, 5);
        assert_eq!(meter.allocated, 131);
        Ok(())
    }

    #[test]
    fn unsigned_subtraction_charges_operands_validation_and_result() -> Result<(), Error> {
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
        let subtract = Expr::UnsignedSubtract {
            max: u64::MAX,
            left: Box::new(Expr::Constant(Value::Integer(9))),
            right: Box::new(Expr::Constant(Value::Integer(4))),
        };
        assert_eq!(
            expression(&subtract, &BTreeMap::new(), &BTreeMap::new(), &mut meter, 0)?,
            Value::Integer(5)
        );
        // Operator, two constant visits/copies, two validations, result copy.
        assert_eq!(meter.steps, 8);
        assert_eq!(meter.allocated, 192);
        Ok(())
    }

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
