// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Evaluate immutable operations after charging the complete result storage.
use super::super::{
    evaluate::{expression, validate, Meter, VALUE_CELL_BYTES},
    model::{Expr, Helper, RuntimeType},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

pub(in crate::edict_pure) fn evaluate(
    expr: &Expr,
    locals: &BTreeMap<&str, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<Value, Error> {
    match expr {
        Expr::ListLength { ty, value } => {
            let value = expression(value, locals, helpers, meter, depth + 1)?;
            validate(&value, ty, meter, depth + 1)?;
            let Value::Array(values) = value else {
                return Err(Error::InvalidArtifact);
            };
            meter.copy(&Value::Integer(
                i128::try_from(values.len()).map_err(|_| Error::InvalidArtifact)?,
            ))
        }
        Expr::ListAt { ty, value, index } => {
            let value = expression(value, locals, helpers, meter, depth + 1)?;
            let index = expression(index, locals, helpers, meter, depth + 1)?;
            validate(&value, ty, meter, depth + 1)?;
            validate(&index, &RuntimeType::Unsigned(u64::MAX), meter, depth + 1)?;
            let (Value::Array(values), Value::Integer(index)) = (value, index) else {
                return Err(Error::InvalidArtifact);
            };
            let index = usize::try_from(index).map_err(|_| Error::InvalidArtifact)?;
            meter.copy(values.get(index).ok_or(Error::InvalidArtifact)?)
        }
        Expr::ListAppend { ty, value, item } => {
            let value = expression(value, locals, helpers, meter, depth + 1)?;
            let entry = expression(item, locals, helpers, meter, depth + 1)?;
            let RuntimeType::List {
                item: Some(item),
                max,
            } = ty
            else {
                return Err(Error::InvalidArtifact);
            };
            validate(&value, ty, meter, depth + 1)?;
            validate(&entry, item, meter, depth + 1)?;
            let Value::Array(mut values) = value else {
                return Err(Error::InvalidArtifact);
            };
            if u64::try_from(values.len()).map_err(|_| Error::InvalidArtifact)? >= *max {
                return Err(Error::InvalidArtifact);
            }
            // Moving values avoids an extra host copy, but does not remove the
            // portable charge for a complete immutable result.
            charge_result(values.iter().chain(std::iter::once(&entry)), meter)?;
            values
                .try_reserve_exact(1)
                .map_err(|_| Error::HostAllocationFailed)?;
            values.push(entry);
            Ok(Value::Array(values))
        }
        Expr::ListConcat {
            left_type,
            right_type,
            left,
            right,
        } => {
            let left = expression(left, locals, helpers, meter, depth + 1)?;
            let right = expression(right, locals, helpers, meter, depth + 1)?;
            validate(&left, left_type, meter, depth + 1)?;
            validate(&right, right_type, meter, depth + 1)?;
            let (Value::Array(mut left), Value::Array(right)) = (left, right) else {
                return Err(Error::InvalidArtifact);
            };
            charge_result(left.iter().chain(&right), meter)?;
            left.try_reserve_exact(right.len())
                .map_err(|_| Error::HostAllocationFailed)?;
            left.extend(right);
            Ok(Value::Array(left))
        }
        _ => Err(Error::InvalidArtifact),
    }
}
fn charge_result<'a>(
    items: impl Iterator<Item = &'a Value>,
    meter: &mut Meter,
) -> Result<(), Error> {
    meter.step(0)?;
    meter.allocate(VALUE_CELL_BYTES)?;
    for item in items {
        meter.charge(item, 1)?;
    }
    Ok(())
}
