// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Strict byte lookup proof, separately metered from non-strict arithmetic order.
use super::{get, list, literal, string, Check, Value};
pub(super) fn valid(
    coordinate: &Value,
    args: &[Value],
    minimum: u64,
    facts: &[&Value],
    path: &[(&Value, bool)],
    work: &mut usize,
) -> Check<bool> {
    let [bytes, index] = args else { return Err(()) };
    if literal(index).is_ok_and(|i| i < minimum) {
        return Ok(true);
    }
    let length = super::order::length_of(coordinate, bytes);
    let index = key(index, work)?;
    let length = key(&length, work)?;
    for fact in facts {
        if strict(fact, true, &index, &length, work, 0)? {
            return Ok(true);
        }
    }
    for (fact, truth) in path {
        if strict(fact, *truth, &index, &length, work, 0)? {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) fn strict(
    fact: &Value,
    truth: bool,
    index: &[u8],
    length: &[u8],
    work: &mut usize,
    depth: usize,
) -> Check<bool> {
    *work = work.checked_sub(1).ok_or(())?;
    if depth > 64 {
        return Err(());
    }
    match string(get(fact, "kind")?)? {
        "not" => strict(get(fact, "value")?, !truth, index, length, work, depth + 1),
        "all" if truth => {
            for f in list(get(fact, "values")?)? {
                if strict(f, true, index, length, work, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "any" if !truth => {
            for f in list(get(fact, "values")?)? {
                if strict(f, false, index, length, work, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "compare" => {
            let a = key(get(fact, "left")?, work)?;
            let b = key(get(fact, "right")?, work)?;
            let op = string(get(fact, "op")?)?;
            Ok(match (op, truth) {
                ("<", true) | (">=", false) => a == index && b == length,
                (">", true) | ("<=", false) => b == index && a == length,
                _ => false,
            })
        }
        _ => Ok(false),
    }
}
pub(super) fn key(value: &Value, work: &mut usize) -> Check<Vec<u8>> {
    let b = echo_edict_canonical::encode_canonical_cbor_v1(value).map_err(|_| ())?;
    *work = work.checked_sub(b.len()).ok_or(())?;
    Ok(b)
}
