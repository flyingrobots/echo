// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owned checked integer arithmetic and `min`/`max`.
//!
//! Each call takes exactly one width type argument (`U8`, `U32` or `U64`)
//! and two operands of exactly that width. A checked call returns
//! `Option<W>`, absent on overflow, underflow or a zero divisor, and needs
//! `core.option/v1`; `min` and `max` return `W`.
use super::{
    evaluate::{expression, validate, Meter, VALUE_CELL_BYTES},
    model::{Expr, Helper, IntegerOperation, RuntimeType},
    syntax::Parser,
    values::text,
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

/// The checked callees, which need `core.option/v1`.
pub(super) fn is_checked(name: &str) -> bool {
    matches!(
        name,
        "core.integer.checkedAdd"
            | "core.integer.checkedSub"
            | "core.integer.checkedMul"
            | "core.integer.checkedDiv"
            | "core.integer.checkedRem"
    )
}

pub(super) fn is_operation(name: &str) -> bool {
    is_checked(name) || name == "core.integer.min" || name == "core.integer.max"
}

fn operation(name: &str) -> Result<IntegerOperation, Error> {
    Ok(match name {
        "core.integer.checkedAdd" => IntegerOperation::CheckedAdd,
        "core.integer.checkedSub" => IntegerOperation::CheckedSub,
        "core.integer.checkedMul" => IntegerOperation::CheckedMul,
        "core.integer.checkedDiv" => IntegerOperation::CheckedDiv,
        "core.integer.checkedRem" => IntegerOperation::CheckedRem,
        "core.integer.min" => IntegerOperation::Min,
        "core.integer.max" => IntegerOperation::Max,
        _ => return Err(Error::UnsupportedProgram),
    })
}

/// The width's maximum, from its exact coordinate.
pub(super) fn width(coordinates: &[Value]) -> Result<u64, Error> {
    let [width] = coordinates else {
        return Err(Error::InvalidArtifact);
    };
    match text(width)? {
        "U8" => Ok(u64::from(u8::MAX)),
        "U32" => Ok(u64::from(u32::MAX)),
        "U64" => Ok(u64::MAX),
        _ => Err(Error::InvalidArtifact),
    }
}

pub(super) fn parse(
    parser: &mut Parser<'_>,
    name: &str,
    args: &[Value],
    coordinates: &[Value],
    depth: usize,
) -> Result<Expr, Error> {
    let operation = operation(name)?;
    let max = width(coordinates)?;
    let [left, right] = args else {
        return Err(Error::InvalidArtifact);
    };
    Ok(Expr::Integer {
        operation,
        max,
        left: Box::new(parser.expr(left, depth + 1)?),
        right: Box::new(parser.expr(right, depth + 1)?),
    })
}

/// The result type, given the argument types. Operands must be exactly the
/// width, as in Target.
pub(super) fn infer(
    name: &str,
    coordinates: &[Value],
    actual: &[RuntimeType],
) -> Result<RuntimeType, Error> {
    let checked = is_checked(name);
    operation(name)?;
    let max = width(coordinates)?;
    let word = RuntimeType::Unsigned(max);
    match actual {
        [left, right] if *left == word && *right == word => Ok(if checked {
            RuntimeType::Option(Box::new(word))
        } else {
            word
        }),
        _ => Err(Error::InvalidArtifact),
    }
}

/// The exact result, or `None` for a checked call's absent case.
const fn apply(operation: IntegerOperation, max: u64, left: u64, right: u64) -> Option<u64> {
    let value = match operation {
        IntegerOperation::CheckedAdd => left.checked_add(right),
        IntegerOperation::CheckedSub => left.checked_sub(right),
        IntegerOperation::CheckedMul => left.checked_mul(right),
        IntegerOperation::CheckedDiv => left.checked_div(right),
        IntegerOperation::CheckedRem => left.checked_rem(right),
        IntegerOperation::Min => Some(if left < right { left } else { right }),
        IntegerOperation::Max => Some(if left > right { left } else { right }),
    };
    match value {
        Some(value) if value <= max => Some(value),
        _ => None,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the evaluator's frame plus the call's own fields"
)]
pub(super) fn evaluate(
    operation: IntegerOperation,
    max: u64,
    left: &Expr,
    right: &Expr,
    locals: &BTreeMap<&str, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<Value, Error> {
    let left = expression(left, locals, helpers, meter, depth + 1)?;
    let right = expression(right, locals, helpers, meter, depth + 1)?;
    let word = RuntimeType::Unsigned(max);
    validate(&left, &word, meter, depth + 1)?;
    validate(&right, &word, meter, depth + 1)?;
    let (Value::Integer(left), Value::Integer(right)) = (left, right) else {
        return Err(Error::InvalidArtifact);
    };
    let left = u64::try_from(left).map_err(|_| Error::InvalidArtifact)?;
    let right = u64::try_from(right).map_err(|_| Error::InvalidArtifact)?;
    let result = apply(operation, max, left, right);
    if matches!(operation, IntegerOperation::Min | IntegerOperation::Max) {
        // Min and max are total on validated operands.
        let value = result.ok_or(Error::InvalidArtifact)?;
        return meter.copy(&Value::Integer(i128::from(value)));
    }
    match result {
        Some(value) => {
            let value = meter.copy(&Value::Integer(i128::from(value)))?;
            meter.allocate(VALUE_CELL_BYTES)?;
            Ok(Value::Array(vec![value]))
        }
        None => meter.copy(&Value::Null),
    }
}
