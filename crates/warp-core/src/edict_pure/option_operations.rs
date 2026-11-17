// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owned Option signatures, presence proofs and evaluation.
//!
//! Amblicit encodes `Option<T>` as typed `core.option.<operation><T>` calls
//! under `core.option/v1`. `unwrap` is total only on a local that a dominating
//! fact proves present; this module rechecks that proof and never trusts the
//! compiler's source refinement.
use super::{
    evaluate::{expression, validate, Meter, VALUE_CELL_BYTES},
    functions::accepts,
    model::{Expr, Helper, OptionOperation, RuntimeType, MAX_DEPTH},
    syntax::Parser,
    values::{array, field, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

/// The Core capability every `core.option.*` call requires.
pub(super) const CAPABILITY: &str = "core.option/v1";
const IS_SOME: &str = "core.option.isSome";

/// Every `core.option.*` callee is gated by the capability; unknown
/// operations are then refused by [`operation`].
pub(super) fn is_operation(name: &str) -> bool {
    name.starts_with("core.option.")
}

fn operation(name: &str) -> Result<OptionOperation, Error> {
    Ok(match name {
        "core.option.some" => OptionOperation::Some,
        "core.option.none" => OptionOperation::None,
        "core.option.isSome" => OptionOperation::IsSome,
        "core.option.default" => OptionOperation::Default,
        "core.option.unwrap" => OptionOperation::Unwrap,
        _ => return Err(Error::UnsupportedProgram),
    })
}

const fn arity(operation: OptionOperation) -> usize {
    match operation {
        OptionOperation::None => 0,
        OptionOperation::Some | OptionOperation::IsSome | OptionOperation::Unwrap => 1,
        OptionOperation::Default => 2,
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
    let [item] = coordinates else {
        return Err(Error::InvalidArtifact);
    };
    if args.len() != arity(operation) {
        return Err(Error::InvalidArtifact);
    }
    Ok(Expr::Option {
        operation,
        item: parser.ty(text(item)?, depth + 1)?,
        args: args
            .iter()
            .map(|arg| parser.expr(arg, depth + 1))
            .collect::<Result<_, _>>()?,
    })
}

/// The result type of a checked Option call. `actual` are the argument
/// types; `facts` and `path` are what dominates the call.
pub(super) fn infer(
    call: &Value,
    actual: &[RuntimeType],
    item: &RuntimeType,
    facts: &[&Value],
    path: &[(&Value, bool)],
) -> Result<RuntimeType, Error> {
    let operation = operation(text_field(call, "callee")?)?;
    // The operand's item must be exactly the type argument, as in Target.
    let holds_item =
        |operand: &RuntimeType| matches!(operand, RuntimeType::Option(inner) if **inner == *item);
    let option = RuntimeType::Option(Box::new(item.clone()));
    match (operation, actual) {
        (OptionOperation::Some, [value]) if accepts(item, value) => Ok(option),
        (OptionOperation::None, []) => Ok(option),
        (OptionOperation::IsSome, [operand]) if holds_item(operand) => Ok(RuntimeType::Bool),
        (OptionOperation::Default, [operand, fallback])
            if holds_item(operand) && accepts(item, fallback) =>
        {
            Ok(item.clone())
        }
        (OptionOperation::Unwrap, [operand]) if holds_item(operand) => {
            let [argument] = array(field(call, "args")?)? else {
                return Err(Error::InvalidArtifact);
            };
            if !present(argument, facts, path)? {
                return Err(Error::UnsupportedProgram);
            }
            Ok(item.clone())
        }
        _ => Err(Error::InvalidArtifact),
    }
}

/// Whether a dominating fact proves `argument`, which must be a local,
/// present.
fn present(argument: &Value, facts: &[&Value], path: &[(&Value, bool)]) -> Result<bool, Error> {
    if text_field(argument, "kind")? != "local" {
        return Ok(false);
    }
    let reference = field(argument, "ref")?;
    for (predicate, holds) in facts.iter().map(|p| (*p, true)).chain(path.iter().copied()) {
        if establishes(predicate, holds, reference, 0)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether `predicate` evaluating to `holds` proves `isSome(reference)`.
fn establishes(
    predicate: &Value,
    holds: bool,
    reference: &Value,
    depth: usize,
) -> Result<bool, Error> {
    if depth > MAX_DEPTH {
        return Err(Error::UnsupportedProgram);
    }
    match text_field(predicate, "kind")? {
        "compare" => {
            let left = field(predicate, "left")?;
            let right = field(predicate, "right")?;
            if text_field(left, "kind")? != "call" || text_field(left, "callee")? != IS_SOME {
                return Ok(false);
            }
            let [tested] = array(field(left, "args")?)? else {
                return Ok(false);
            };
            if text_field(tested, "kind")? != "local" || field(tested, "ref")? != reference {
                return Ok(false);
            }
            if text_field(right, "kind")? != "const" {
                return Ok(false);
            }
            let constant = field(right, "value")?;
            if text_field(constant, "kind")? != "bool" {
                return Ok(false);
            }
            let Value::Bool(value) = field(constant, "value")? else {
                return Ok(false);
            };
            Ok(match text_field(predicate, "op")? {
                "==" => holds == *value,
                "!=" => holds != *value,
                _ => false,
            })
        }
        "not" => establishes(field(predicate, "value")?, !holds, reference, depth + 1),
        kind @ ("all" | "any") => {
            // A true `all` makes every item true; a false `any` every item false.
            if (kind == "all") != holds {
                return Ok(false);
            }
            for item in array(field(predicate, "values")?)? {
                if establishes(item, holds, reference, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

pub(super) fn evaluate(
    operation: OptionOperation,
    item: &RuntimeType,
    args: &[Expr],
    locals: &BTreeMap<&str, Value>,
    helpers: &BTreeMap<String, Helper>,
    meter: &mut Meter,
    depth: usize,
) -> Result<Value, Error> {
    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        values.push(expression(arg, locals, helpers, meter, depth + 1)?);
    }
    let option = RuntimeType::Option(Box::new(item.clone()));
    match (operation, values.as_mut_slice()) {
        (OptionOperation::Some, [value]) => {
            validate(value, item, meter, depth + 1)?;
            meter.allocate(VALUE_CELL_BYTES)?;
            Ok(Value::Array(vec![std::mem::replace(value, Value::Null)]))
        }
        (OptionOperation::None, []) => meter.copy(&Value::Null),
        (OptionOperation::IsSome, [operand]) => {
            validate(operand, &option, meter, depth + 1)?;
            meter.copy(&Value::Bool(matches!(operand, Value::Array(_))))
        }
        (OptionOperation::Default, [operand, fallback]) => {
            validate(operand, &option, meter, depth + 1)?;
            Ok(match operand {
                Value::Array(present) => present.pop().ok_or(Error::InvalidArtifact)?,
                _ => std::mem::replace(fallback, Value::Null),
            })
        }
        (OptionOperation::Unwrap, [operand]) => {
            validate(operand, &option, meter, depth + 1)?;
            match operand {
                // A checked artifact proved presence; absence is a forged artifact.
                Value::Array(present) => present.pop().ok_or(Error::InvalidArtifact),
                _ => Err(Error::InvalidArtifact),
            }
        }
        _ => Err(Error::InvalidArtifact),
    }
}

/// Validates an Option value: `null`, or a one-item array of the item type.
pub(super) fn validate_value(
    value: &Value,
    item: &RuntimeType,
    meter: &mut Meter,
    depth: usize,
) -> Result<(), Error> {
    match value {
        Value::Null => Ok(()),
        Value::Array(values) => match values.as_slice() {
            [value] => validate(value, item, meter, depth + 1),
            _ => Err(Error::InvalidArtifact),
        },
        _ => Err(Error::InvalidArtifact),
    }
}
