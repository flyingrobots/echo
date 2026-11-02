// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo-owned order proofs for slicing/subtraction and strict byte lookup.
//!
//! `left >= right` holds for equal terms, a zero right side, ordered
//! constants, or a dominating fact: an earlier conjunct of an enclosing
//! `all`, or a pure intent's input constraint. Conditional predicates,
//! negations and later conjuncts prove nothing for this order judgment.
//! The separate strict lookup judgment below also uses signed conditional and
//! lazy-prefix truths. Neither judgment trusts annotations in the artifact.
use echo_edict_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};

use super::syntax::Parser;
use super::values::{array, field, number, text_field};
use super::EvaluationError as Error;

/// Whether `left >= right` is established at this evaluation point.
pub(crate) fn proven(
    left: &Value,
    right: &Value,
    facts: &[&Value],
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<bool, Error> {
    parser.enter(depth)?;
    let left_key = key(left, parser)?;
    let right_key = key(right, parser)?;
    if left_key == right_key || constant(right) == Some(0) {
        return Ok(true);
    }
    if let (Some(a), Some(b)) = (constant(left), constant(right)) {
        return Ok(a >= b);
    }
    for fact in facts {
        if supports(fact, &left_key, &right_key, parser, depth + 1)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The Core term `core.bytes.length<coordinate>(bytes)`, for slice end proofs.
pub(crate) fn length_of(coordinate: &Value, bytes: &Value) -> Value {
    Value::Map(vec![
        (Value::Text("kind".into()), Value::Text("call".into())),
        (
            Value::Text("callee".into()),
            Value::Text("core.bytes.length".into()),
        ),
        (
            Value::Text("typeArgs".into()),
            Value::Array(vec![coordinate.clone()]),
        ),
        (
            Value::Text("args".into()),
            Value::Array(vec![bytes.clone()]),
        ),
    ])
}

fn supports(
    fact: &Value,
    left: &[u8],
    right: &[u8],
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<bool, Error> {
    parser.enter(depth)?;
    match text_field(fact, "kind")? {
        "all" => {
            for item in array(field(fact, "values")?)? {
                if supports(item, left, right, parser, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "compare" => {
            let a = key(field(fact, "left")?, parser)?;
            let b = key(field(fact, "right")?, parser)?;
            Ok(match text_field(fact, "op")? {
                ">=" | ">" => a == left && b == right,
                "<=" | "<" => a == right && b == left,
                "==" => (a == left && b == right) || (a == right && b == left),
                _ => false,
            })
        }
        _ => Ok(false),
    }
}

/// Terms compare by canonical encoding, independent of in-memory map order.
pub(super) fn key(value: &Value, parser: &mut Parser<'_>) -> Result<Vec<u8>, Error> {
    let bytes = encode_canonical_cbor_v1(value).map_err(|_| Error::InvalidArtifact)?;
    parser.remaining = parser
        .remaining
        .checked_sub(bytes.len())
        .ok_or(Error::UnsupportedProgram)?;
    Ok(bytes)
}

fn constant(value: &Value) -> Option<u64> {
    if text_field(value, "kind").ok()? != "const" {
        return None;
    }
    let literal = field(value, "value").ok()?;
    if text_field(literal, "kind").ok()? != "int" {
        return None;
    }
    number(field(literal, "value").ok()?).ok()
}

/// Strict proof for a byte lookup, including signed branch/lazy-path facts.
pub(crate) fn strict_index(
    coordinate: &Value,
    args: &[Value],
    minimum: u64,
    facts: &[&Value],
    path: &[(&Value, bool)],
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<bool, Error> {
    let [bytes, index] = args else {
        return Err(Error::InvalidArtifact);
    };
    if constant(index).is_some_and(|i| i < minimum) {
        return Ok(true);
    }
    let length = length_of(coordinate, bytes);
    let a = key(index, parser)?;
    let b = key(&length, parser)?;
    for p in facts {
        if strict_fact(p, true, &a, &b, parser, depth)? {
            return Ok(true);
        }
    }
    for (p, truth) in path {
        if strict_fact(p, *truth, &a, &b, parser, depth)? {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) fn strict_fact(
    p: &Value,
    truth: bool,
    index: &[u8],
    length: &[u8],
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<bool, Error> {
    parser.enter(depth)?;
    match text_field(p, "kind")? {
        "not" => strict_fact(field(p, "value")?, !truth, index, length, parser, depth + 1),
        "all" if truth => {
            for p in array(field(p, "values")?)? {
                if strict_fact(p, true, index, length, parser, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "any" if !truth => {
            for p in array(field(p, "values")?)? {
                if strict_fact(p, false, index, length, parser, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "compare" => {
            let a = key(field(p, "left")?, parser)?;
            let b = key(field(p, "right")?, parser)?;
            Ok(match (text_field(p, "op")?, truth) {
                ("<", true) | (">=", false) => a == index && b == length,
                (">", true) | ("<=", false) => b == index && a == length,
                _ => false,
            })
        }
        _ => Ok(false),
    }
}
