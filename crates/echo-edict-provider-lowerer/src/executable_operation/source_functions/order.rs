// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned order proofs for partial byte slicing and unsigned subtraction.
//!
//! `left >= right` holds for equal terms, a zero right side, ordered
//! constants, or a dominating fact: an input constraint or an earlier
//! conjunct of an enclosing `all`. Conditional predicates, negations and
//! later conjuncts prove nothing.
use super::{get, list, literal, string, Check, Value};

pub(super) fn proven(
    left: &Value,
    right: &Value,
    facts: &[&Value],
    work: &mut usize,
) -> Check<bool> {
    let left_key = key(left, work)?;
    let right_key = key(right, work)?;
    if left_key == right_key || literal(right) == Ok(0) {
        return Ok(true);
    }
    if let (Ok(a), Ok(b)) = (literal(left), literal(right)) {
        return Ok(a >= b);
    }
    for fact in facts {
        if supports(fact, &left_key, &right_key, work, 0)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The Core term `core.bytes.length<coordinate>(bytes)`, for slice end proofs.
pub(super) fn length_of(coordinate: &Value, bytes: &Value) -> Value {
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
    work: &mut usize,
    depth: usize,
) -> Check<bool> {
    debit(work, 1)?;
    if depth > 64 {
        return Err(());
    }
    match string(get(fact, "kind")?)? {
        "all" => {
            for item in list(get(fact, "values")?)? {
                if supports(item, left, right, work, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "compare" => {
            let a = key(get(fact, "left")?, work)?;
            let b = key(get(fact, "right")?, work)?;
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
fn key(value: &Value, work: &mut usize) -> Check<Vec<u8>> {
    let bytes = echo_edict_canonical::encode_canonical_cbor_v1(value).map_err(|_| ())?;
    debit(work, bytes.len())?;
    Ok(bytes)
}

fn debit(work: &mut usize, amount: usize) -> Check<()> {
    *work = work.checked_sub(amount).ok_or(())?;
    Ok(())
}
