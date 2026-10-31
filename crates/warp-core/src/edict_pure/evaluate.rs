// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use std::collections::BTreeMap;

use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};

use super::model::{Comparison, Expr, Helper, Predicate, Program, RuntimeType, MAX_DEPTH};
use super::{EvaluationError as Error, EvaluationLimits, EvaluationResult};

// Fixed interpreter storage units; Rust layout and pointer width cannot alter cost.
pub(crate) const VALUE_CELL_BYTES: u64 = 64;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Meter {
    limits: EvaluationLimits,
    steps: u64,
    allocated: u64,
    atomic_charges: bool,
}

impl Meter {
    pub(crate) fn new(limits: EvaluationLimits) -> Self {
        Self {
            limits,
            steps: 0,
            allocated: 0,
            atomic_charges: false,
        }
    }

    pub(crate) fn new_atomic(limits: EvaluationLimits) -> Self {
        Self {
            atomic_charges: true,
            ..Self::new(limits)
        }
    }

    pub(crate) const fn usage(&self) -> (u64, u64) {
        (self.steps, self.allocated)
    }

    pub(crate) fn with_component<T, E>(
        &mut self,
        limits: EvaluationLimits,
        run: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        let saved = self.limits;
        self.limits.max_steps = saved
            .max_steps
            .min(self.steps.saturating_add(limits.max_steps));
        self.limits.max_allocated_bytes = saved
            .max_allocated_bytes
            .min(self.allocated.saturating_add(limits.max_allocated_bytes));
        let result = run(self);
        self.limits = saved;
        result
    }

    // The caller already charged work and storage; only atomic frame paths
    // translate allocator refusal into a host fault. Standalone behavior stays.
    pub(crate) fn copy_precharged_read_bytes(&self, bytes: &[u8]) -> Result<Value, Error> {
        if !self.atomic_charges {
            return Ok(Value::Bytes(bytes.to_vec()));
        }
        let mut value = Vec::new();
        value
            .try_reserve_exact(bytes.len())
            .map_err(|_| Error::HostAllocationFailed)?;
        value.extend_from_slice(bytes);
        Ok(Value::Bytes(value))
    }

    pub(crate) fn step(&mut self, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
        self.charge_steps(1)
    }

    pub(crate) fn charge_steps(&mut self, count: u64) -> Result<(), Error> {
        let next = self
            .steps
            .checked_add(count)
            .ok_or(Error::StepBudgetExceeded)?;
        if self.atomic_charges && next > self.limits.max_steps {
            return Err(Error::StepBudgetExceeded);
        }
        self.steps = next;
        if self.steps > self.limits.max_steps {
            return Err(Error::StepBudgetExceeded);
        }
        Ok(())
    }

    pub(crate) fn allocate(&mut self, bytes: u64) -> Result<(), Error> {
        let next = self
            .allocated
            .checked_add(bytes)
            .ok_or(Error::AllocationBudgetExceeded)?;
        if self.atomic_charges && next > self.limits.max_allocated_bytes {
            return Err(Error::AllocationBudgetExceeded);
        }
        self.allocated = next;
        if self.allocated > self.limits.max_allocated_bytes {
            return Err(Error::AllocationBudgetExceeded);
        }
        Ok(())
    }

    pub(crate) fn copy(&mut self, value: &Value) -> Result<Value, Error> {
        self.charge(value, 0)?;
        Ok(value.clone())
    }

    fn charge_bytes(&mut self, length: usize) -> Result<(), Error> {
        let length = u64::try_from(length).map_err(|_| Error::AllocationBudgetExceeded)?;
        // Byte work and ordinary result storage are admitted before materialization.
        self.charge_steps(length)?;
        self.step(0)?;
        self.allocate(VALUE_CELL_BYTES)?;
        self.allocate(length)
    }

    fn copy_bytes(&mut self, bytes: &[u8]) -> Result<Value, Error> {
        self.charge_bytes(bytes.len())?;
        Ok(Value::Bytes(bytes.to_vec()))
    }

    fn concat_bytes(&mut self, left: &[u8], right: &[u8]) -> Result<Value, Error> {
        let length = left
            .len()
            .checked_add(right.len())
            .ok_or(Error::AllocationBudgetExceeded)?;
        self.charge_bytes(length)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| {
            if self.atomic_charges {
                Error::HostAllocationFailed
            } else {
                Error::AllocationBudgetExceeded
            }
        })?;
        bytes.extend_from_slice(left);
        bytes.extend_from_slice(right);
        Ok(Value::Bytes(bytes))
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
    run_with_meter(program, input, &mut Meter::new(program.limits))
}

pub(crate) fn run_with_meter(
    program: &Program,
    input: Value,
    meter: &mut Meter,
) -> Result<EvaluationResult, Error> {
    meter.with_component(program.limits, |meter| run_body(program, input, meter))
}

fn run_body(program: &Program, input: Value, meter: &mut Meter) -> Result<EvaluationResult, Error> {
    validate(&input, &program.input_type, meter, 0).map_err(|error| {
        if error == Error::InvalidArtifact {
            Error::InvalidInput
        } else {
            error
        }
    })?;
    meter.charge(&input, 0)?;
    let mut locals = BTreeMap::from([(program.input_id.as_str(), input)]);
    for (coordinate, constraint) in &program.constraints {
        if !predicate(constraint, &locals, &program.helpers, meter, 0)? {
            return Err(Error::InputConstraintFailed(coordinate.clone()));
        }
    }
    for binding in &program.bindings {
        let value = expression(&binding.value, &locals, &program.helpers, meter, 0)?;
        validate(&value, &binding.ty, meter, 0)?;
        locals.insert(binding.id.as_str(), value);
    }
    let result = expression(&program.result, &locals, &program.helpers, meter, 0)?;
    validate(&result, &program.output_type, meter, 0)?;
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
    locals: &BTreeMap<&str, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<Value, Error> {
    meter.step(depth)?;
    match expr {
        Expr::Constant(value) => meter.copy(value),
        Expr::Variant {
            identity,
            case,
            payload,
        } => super::variants::evaluate_constructor(
            (identity, case),
            payload.as_deref(),
            locals,
            helpers,
            meter,
            depth,
        ),
        Expr::Local(id) => meter.copy(locals.get(id.as_str()).ok_or(Error::InvalidArtifact)?),
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
        Expr::Call(name, args) => {
            let helper = helpers.get(name).ok_or(Error::UnsupportedProgram)?;
            if args.len() != helper.params.len() {
                return Err(Error::InvalidArtifact);
            }
            // Arguments run once, left to right in the caller's frame. Move each
            // value into the fresh frame; later local reads charge their own copies.
            let mut frame = BTreeMap::new();
            for (argument, parameter) in args.iter().zip(&helper.params) {
                let value = expression(argument, locals, helpers, meter, depth + 1)?;
                validate(&value, &parameter.ty, meter, depth + 1)?;
                frame.insert(parameter.id.as_str(), value);
            }
            for binding in &helper.bindings {
                let value = expression(&binding.value, &frame, helpers, meter, depth + 1)?;
                validate(&value, &binding.ty, meter, depth + 1)?;
                frame.insert(binding.id.as_str(), value);
            }
            let value = expression(&helper.result, &frame, helpers, meter, depth + 1)?;
            validate(&value, &helper.ty, meter, depth + 1)?;
            Ok(value)
        }
        Expr::UnsignedSubtract { max, left, right } | Expr::UnsignedAdd { max, left, right } => {
            let left = expression(left, locals, helpers, meter, depth + 1)?;
            let right = expression(right, locals, helpers, meter, depth + 1)?;
            let ty = RuntimeType::Unsigned(*max);
            validate(&left, &ty, meter, depth + 1)?;
            validate(&right, &ty, meter, depth + 1)?;
            let (Value::Integer(left), Value::Integer(right)) = (left, right) else {
                return Err(Error::InvalidArtifact);
            };
            let difference = if matches!(expr, Expr::UnsignedAdd { .. }) {
                left.checked_add(right)
                    .filter(|value| *value <= i128::from(*max))
            } else {
                left.checked_sub(right)
            }
            .filter(|value| *value >= 0)
            .ok_or(Error::InvalidArtifact)?;
            meter.copy(&Value::Integer(difference))
        }
        Expr::ByteUtf8 { min, max, value } => {
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
            let length = u64::try_from(bytes.len()).map_err(|_| Error::StepBudgetExceeded)?;
            meter.charge_steps(length)?;
            meter.copy(&Value::Bool(std::str::from_utf8(&bytes).is_ok()))
        }
        Expr::ByteCount {
            min,
            max,
            value,
            needle,
        } => {
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
            let length = u64::try_from(bytes.len()).map_err(|_| Error::StepBudgetExceeded)?;
            meter.charge_steps(length)?;
            // Never exceeds the already-charged byte length.
            let count = bytes
                .iter()
                .fold(0_u64, |count, byte| count + u64::from(byte == needle));
            meter.copy(&Value::Integer(i128::from(count)))
        }
        Expr::ByteAt {
            min,
            max,
            value,
            index,
        } => {
            let value = expression(value, locals, helpers, meter, depth + 1)?;
            let index = expression(index, locals, helpers, meter, depth + 1)?;
            validate(
                &value,
                &RuntimeType::Bytes {
                    min: *min,
                    max: *max,
                },
                meter,
                depth + 1,
            )?;
            validate(&index, &RuntimeType::Unsigned(u64::MAX), meter, depth + 1)?;
            let (Value::Bytes(bytes), Value::Integer(index)) = (value, index) else {
                return Err(Error::InvalidArtifact);
            };
            let index = usize::try_from(index).map_err(|_| Error::InvalidArtifact)?;
            let byte = *bytes.get(index).ok_or(Error::InvalidArtifact)?;
            meter.copy(&Value::Integer(i128::from(byte)))
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
        Expr::ByteConcat {
            left_min,
            left_max,
            right_min,
            right_max,
            left,
            right,
        } => {
            let left = expression(left, locals, helpers, meter, depth + 1)?;
            let right = expression(right, locals, helpers, meter, depth + 1)?;
            validate(
                &left,
                &RuntimeType::Bytes {
                    min: *left_min,
                    max: *left_max,
                },
                meter,
                depth + 1,
            )?;
            validate(
                &right,
                &RuntimeType::Bytes {
                    min: *right_min,
                    max: *right_max,
                },
                meter,
                depth + 1,
            )?;
            let (Value::Bytes(left), Value::Bytes(right)) = (left, right) else {
                return Err(Error::InvalidArtifact);
            };
            meter.concat_bytes(&left, &right)
        }
        Expr::ByteSlice {
            min,
            max,
            value,
            start,
            end,
        } => {
            let value = expression(value, locals, helpers, meter, depth + 1)?;
            let start = expression(start, locals, helpers, meter, depth + 1)?;
            let end = expression(end, locals, helpers, meter, depth + 1)?;
            byte_slice(
                value,
                start,
                end,
                &RuntimeType::Bytes {
                    min: *min,
                    max: *max,
                },
                meter,
                depth + 1,
            )
        }
    }
}

fn byte_slice(
    value: Value,
    start: Value,
    end: Value,
    ty: &RuntimeType,
    meter: &mut Meter,
    depth: usize,
) -> Result<Value, Error> {
    validate(&value, ty, meter, depth)?;
    validate(&start, &RuntimeType::Unsigned(u64::MAX), meter, depth)?;
    validate(&end, &RuntimeType::Unsigned(u64::MAX), meter, depth)?;
    let (Value::Bytes(bytes), Value::Integer(start), Value::Integer(end)) = (value, start, end)
    else {
        return Err(Error::InvalidArtifact);
    };
    let start = usize::try_from(start).map_err(|_| Error::InvalidArtifact)?;
    let end = usize::try_from(end).map_err(|_| Error::InvalidArtifact)?;
    let selected = bytes.get(start..end).ok_or(Error::InvalidArtifact)?;
    meter.copy_bytes(selected)
}

pub(crate) fn predicate(
    predicate: &Predicate,
    locals: &BTreeMap<&str, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<bool, Error> {
    meter.step(depth)?;
    let (op, left, right) = match predicate {
        Predicate::Compare { op, left, right } => (op, left, right),
        Predicate::Not(value) => {
            return self::predicate(value, locals, helpers, meter, depth + 1).map(|value| !value);
        }
        Predicate::Any(values) => {
            for value in values {
                if self::predicate(value, locals, helpers, meter, depth + 1)? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        Predicate::All(values) => {
            for value in values {
                if !self::predicate(value, locals, helpers, meter, depth + 1)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
    };
    let left = expression(left, locals, helpers, meter, depth + 1)?;
    let right = expression(right, locals, helpers, meter, depth + 1)?;
    match (op, left, right) {
        (Comparison::Equal, Value::Bool(left), Value::Bool(right)) => Ok(left == right),
        (Comparison::Equal, Value::Integer(left), Value::Integer(right)) => Ok(left == right),
        (Comparison::LessOrEqual, Value::Integer(left), Value::Integer(right)) => Ok(left <= right),
        (Comparison::Less, Value::Integer(left), Value::Integer(right)) => Ok(left < right),
        (Comparison::Greater, Value::Integer(left), Value::Integer(right)) => Ok(left > right),
        (Comparison::GreaterOrEqual, Value::Integer(left), Value::Integer(right)) => {
            Ok(left >= right)
        }
        (Comparison::NotEqual, Value::Integer(left), Value::Integer(right)) => Ok(left != right),
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
    match (value, ty.representation()) {
        (_, RuntimeType::Variant { identity, cases }) => {
            super::variants::validate_value(value, identity, cases, meter, depth)
        }
        (Value::Bool(_), RuntimeType::Bool) => Ok(()),
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
    fn unsigned_addition_checks_both_widths_and_charges_the_result() -> Result<(), Error> {
        let limits = EvaluationLimits {
            max_package_bytes: 1024,
            max_input_bytes: 1024,
            max_steps: 8,
            max_allocated_bytes: 192,
            max_output_bytes: 1024,
        };
        let sum = |max, left, right| Expr::UnsignedAdd {
            max,
            left: Box::new(Expr::Constant(Value::Integer(left))),
            right: Box::new(Expr::Constant(Value::Integer(right))),
        };
        for max in [u64::from(u32::MAX), u64::MAX] {
            let expr = sum(max, i128::from(max), 0);
            let mut exact = Meter::new(limits);
            assert_eq!(
                expression(&expr, &BTreeMap::new(), &BTreeMap::new(), &mut exact, 0)?,
                Value::Integer(i128::from(max))
            );
            assert_eq!(exact.usage(), (8, 192));
            for (left, right) in [(i128::from(max), 1), (-1, 0), (i128::from(max) + 1, 0)] {
                assert_eq!(
                    expression(
                        &sum(max, left, right),
                        &BTreeMap::new(),
                        &BTreeMap::new(),
                        &mut Meter::new(limits),
                        0
                    ),
                    Err(Error::InvalidArtifact)
                );
            }
            assert_eq!(
                expression(
                    &expr,
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    &mut Meter::new(EvaluationLimits {
                        max_steps: 7,
                        ..limits
                    }),
                    0
                ),
                Err(Error::StepBudgetExceeded)
            );
            assert_eq!(
                expression(
                    &expr,
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    &mut Meter::new(EvaluationLimits {
                        max_allocated_bytes: 191,
                        ..limits
                    }),
                    0
                ),
                Err(Error::AllocationBudgetExceeded)
            );
        }
        Ok(())
    }

    #[test]
    fn byte_concat_admits_both_copy_lengths_before_allocation() -> Result<(), Error> {
        let limits = EvaluationLimits {
            max_package_bytes: 1024,
            max_input_bytes: 1024,
            max_steps: 4,
            max_allocated_bytes: 67,
            max_output_bytes: 1024,
        };
        let mut exact = Meter::new(limits);
        assert_eq!(
            exact.concat_bytes(&[0], &[128, 255])?,
            Value::Bytes(vec![0, 128, 255])
        );
        assert_eq!(exact.usage(), (4, 67));
        let mut work = Meter::new(EvaluationLimits {
            max_steps: 3,
            ..limits
        });
        assert_eq!(
            work.concat_bytes(&[0], &[128, 255]),
            Err(Error::StepBudgetExceeded)
        );
        assert_eq!(work.allocated, 0);
        let mut storage = Meter::new(EvaluationLimits {
            max_allocated_bytes: 66,
            ..limits
        });
        assert_eq!(
            storage.concat_bytes(&[0], &[128, 255]),
            Err(Error::AllocationBudgetExceeded)
        );
        let mut empty = Meter::new(EvaluationLimits {
            max_steps: 1,
            max_allocated_bytes: 64,
            ..limits
        });
        assert_eq!(empty.concat_bytes(&[], &[])?, Value::Bytes(vec![]));
        assert_eq!(empty.usage(), (1, 64));
        Ok(())
    }

    #[test]
    fn byte_slice_admits_work_and_storage_before_materializing_result() -> Result<(), Error> {
        let limits = EvaluationLimits {
            max_package_bytes: 1024,
            max_input_bytes: 1024,
            max_steps: 4,
            max_allocated_bytes: 67,
            max_output_bytes: 1024,
        };
        let mut meter = Meter {
            limits,
            steps: 0,
            allocated: 0,
            atomic_charges: false,
        };
        assert_eq!(
            meter.copy_bytes(&[0, 128, 255])?,
            Value::Bytes(vec![0, 128, 255])
        );
        assert_eq!((meter.steps, meter.allocated), (4, 67));
        let mut work = Meter {
            limits: EvaluationLimits {
                max_steps: 2,
                ..limits
            },
            steps: 0,
            allocated: 0,
            atomic_charges: false,
        };
        assert_eq!(
            work.copy_bytes(&[0, 128, 255]),
            Err(Error::StepBudgetExceeded)
        );
        assert_eq!(work.allocated, 0);
        let mut storage = Meter {
            limits: EvaluationLimits {
                max_allocated_bytes: 66,
                ..limits
            },
            steps: 0,
            allocated: 0,
            atomic_charges: false,
        };
        assert_eq!(
            storage.copy_bytes(&[0, 128, 255]),
            Err(Error::AllocationBudgetExceeded)
        );
        Ok(())
    }

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
            atomic_charges: false,
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
            atomic_charges: false,
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
            atomic_charges: false,
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
