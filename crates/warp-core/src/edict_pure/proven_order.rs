// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo-owned order proofs for partial byte slicing and unsigned subtraction.
//!
//! `left >= right` holds for equal terms, a zero right side, ordered
//! constants, or a dominating fact: an earlier conjunct of an enclosing
//! `all`, or a pure intent's input constraint. Conditional predicates,
//! negations and later conjuncts prove nothing. This follows Edict's
//! proven-range semantics without trusting any annotation in the artifact.
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

/// The smallest constant upper bound a dominating fact gives `term`, as in
/// Edict: `term <= c`, `term == c` or `c >= term` give `c`, and the strict forms
/// give `c - 1` (saturating). Conjunctions contribute every conjunct.
pub(crate) fn upper_limit(
    term: &Value,
    facts: &[&Value],
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<Option<u64>, Error> {
    if facts.is_empty() {
        return Ok(None);
    }
    parser.enter(depth)?;
    let term = key(term, parser)?;
    let mut limit = None;
    for fact in facts {
        limit_from(fact, &term, &mut limit, parser, depth + 1)?;
    }
    Ok(limit)
}

fn limit_from(
    fact: &Value,
    term: &[u8],
    limit: &mut Option<u64>,
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<(), Error> {
    parser.enter(depth)?;
    match text_field(fact, "kind")? {
        "all" => {
            for item in array(field(fact, "values")?)? {
                limit_from(item, term, limit, parser, depth + 1)?;
            }
        }
        "compare" => {
            let op = text_field(fact, "op")?;
            let left = field(fact, "left")?;
            let right = field(fact, "right")?;
            let strict = matches!(op, "<" | ">");
            let mut bound = |value: u64| {
                let value = if strict {
                    value.saturating_sub(1)
                } else {
                    value
                };
                *limit = Some(limit.map_or(value, |current| current.min(value)));
            };
            if matches!(op, "<=" | "<" | "==") {
                if let Some(value) = constant(right) {
                    if key(left, parser)? == term {
                        bound(value);
                    }
                }
            }
            if matches!(op, ">=" | ">" | "==") {
                if let Some(value) = constant(left) {
                    if key(right, parser)? == term {
                        bound(value);
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
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
fn key(value: &Value, parser: &mut Parser<'_>) -> Result<Vec<u8>, Error> {
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
