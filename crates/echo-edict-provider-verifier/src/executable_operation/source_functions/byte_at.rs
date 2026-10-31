// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent verifier strict-domain proof for a single raw-byte lookup.
use super::{get, sequence, string, unsigned, Check, Types, Value};
pub(super) fn valid(
    coordinate: &Value,
    args: &[Value],
    minimum: u64,
    facts: &[&Value],
    path: &[(&Value, bool)],
    types: &mut Types<'_>,
) -> Check<bool> {
    let [bytes, index] = args else { return Err(()) };
    if string(get(index, "kind")?)? == "const"
        && unsigned(get(get(index, "value")?, "value")?).is_ok_and(|i| i < minimum)
    {
        return Ok(true);
    }
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
    let a = canonical(index, types)?;
    let b = canonical(&length, types)?;
    for p in facts {
        if establishes(p, true, &a, &b, types, 0)? {
            return Ok(true);
        }
    }
    for (p, holds) in path {
        if establishes(p, *holds, &a, &b, types, 0)? {
            return Ok(true);
        }
    }
    Ok(false)
}
fn establishes(
    p: &Value,
    holds: bool,
    index: &[u8],
    length: &[u8],
    types: &mut Types<'_>,
    depth: usize,
) -> Check<bool> {
    types.step(depth)?;
    match string(get(p, "kind")?)? {
        "compare" => {
            let a = canonical(get(p, "left")?, types)?;
            let b = canonical(get(p, "right")?, types)?;
            let op = string(get(p, "op")?)?;
            let forward =
                a == index && b == length && matches!((op, holds), ("<", true) | (">=", false));
            let mirrored =
                b == index && a == length && matches!((op, holds), (">", true) | ("<=", false));
            Ok(forward || mirrored)
        }
        "not" => establishes(get(p, "value")?, !holds, index, length, types, depth + 1),
        "all" if holds => {
            for item in sequence(get(p, "values")?)? {
                if establishes(item, true, index, length, types, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "any" if !holds => {
            for item in sequence(get(p, "values")?)? {
                if establishes(item, false, index, length, types, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}
fn canonical(v: &Value, types: &mut Types<'_>) -> Check<Vec<u8>> {
    let encoded = echo_edict_canonical::encode_canonical_cbor_v1(v).map_err(|_| ())?;
    types.proof_work(encoded.len())?;
    Ok(encoded)
}
