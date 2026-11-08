// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned order proofs for partial byte slicing and unsigned subtraction.
//!
//! `left >= right` holds for equal terms, a zero right side, ordered
//! constants, or a dominating fact: an input constraint or an earlier
//! conjunct of an enclosing `all`. Conditional predicates, negations and
//! later conjuncts prove nothing.
use super::{get, sequence, string, unsigned, Check, Types, Value};

pub(super) fn proven(
    left: &Value,
    right: &Value,
    facts: &[&Value],
    types: &mut Types<'_>,
) -> Check<bool> {
    let left_key = key(left, types)?;
    let right_key = key(right, types)?;
    if left_key == right_key || constant(right) == Some(0) {
        return Ok(true);
    }
    if let (Some(a), Some(b)) = (constant(left), constant(right)) {
        return Ok(a >= b);
    }
    for fact in facts {
        if supports(fact, &left_key, &right_key, types, 0)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether a slice's offsets are `start <= end <= len(bytes)` here.
pub(super) fn slice_in_range(
    coordinate: &Value,
    arguments: &[Value],
    facts: &[&Value],
    types: &mut Types<'_>,
) -> Check<bool> {
    let [bytes, start, end] = arguments else {
        return Err(());
    };
    let length = Value::Map(vec![
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
    ]);
    Ok(proven(end, start, facts, types)? && proven(&length, end, facts, types)?)
}

fn supports(
    fact: &Value,
    left: &[u8],
    right: &[u8],
    types: &mut Types<'_>,
    depth: usize,
) -> Check<bool> {
    types.step(depth)?;
    match string(get(fact, "kind")?)? {
        "all" => {
            for item in sequence(get(fact, "values")?)? {
                if supports(item, left, right, types, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "compare" => {
            let a = key(get(fact, "left")?, types)?;
            let b = key(get(fact, "right")?, types)?;
            Ok(match string(get(fact, "op")?)? {
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
fn key(value: &Value, types: &mut Types<'_>) -> Check<Vec<u8>> {
    let bytes = echo_edict_canonical::encode_canonical_cbor_v1(value).map_err(|_| ())?;
    types.proof_work(bytes.len())?;
    Ok(bytes)
}

fn constant(value: &Value) -> Option<u64> {
    if string(get(value, "kind").ok()?).ok()? != "const" {
        return None;
    }
    let literal = get(value, "value").ok()?;
    if string(get(literal, "kind").ok()?).ok()? != "int" {
        return None;
    }
    unsigned(get(literal, "value").ok()?).ok()
}
