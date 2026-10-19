// SPDX-License-Identifier: Apache-2.0
//! Verifier-owned inventory of canonical-expression caps and independent composition.
use super::{get, sequence, string, unsigned, Check, Types, Value};

pub(super) fn present(core: &Value) -> Check<bool> {
    let mut layers = vec![get(core, "intents")?];
    while !layers.is_empty() {
        let mut next = Vec::new();
        for value in layers {
            if let Value::Map(entries) = value {
                if string(get(value, "kind").unwrap_or(&Value::Null)) == Ok("call")
                    && string(get(value, "callee").unwrap_or(&Value::Null))
                        == Ok("core.integer.add")
                {
                    return Ok(true);
                }
                next.extend(entries.iter().map(|(_, child)| child));
            } else if let Value::Array(entries) = value {
                next.extend(entries);
            }
        }
        layers = next;
    }
    Ok(false)
}

pub(super) fn prove(
    args: &[Value],
    max: u64,
    facts: &[&Value],
    types: &mut Types<'_>,
) -> Check<()> {
    let [left, right] = args else { return Err(()) };
    let mut pending: Vec<_> = facts.iter().map(|value| (*value, 0)).collect();
    let mut caps = Vec::new();
    while let Some((predicate, depth)) = pending.pop() {
        types.step(depth)?;
        match string(get(predicate, "kind")?)? {
            "all" => pending.extend(
                sequence(get(predicate, "values")?)?
                    .iter()
                    .map(|value| (value, depth + 1)),
            ),
            "compare" => {
                let op = string(get(predicate, "op")?)?;
                let a = get(predicate, "left")?;
                let b = get(predicate, "right")?;
                if matches!(op, "<=" | "==") {
                    if let Some(n) = constant(b) {
                        caps.push((key(a, types)?, n));
                    }
                }
                if op == "==" {
                    if let Some(n) = constant(a) {
                        caps.push((key(b, types)?, n));
                    }
                }
            }
            _ => {}
        }
    }
    let a = ceiling(left, max, &caps, types, 0)?;
    let b = ceiling(right, max, &caps, types, 0)?;
    if u128::from(a) + u128::from(b) > u128::from(max) {
        return Err(());
    }
    Ok(())
}
fn constant(expr: &Value) -> Option<u64> {
    if string(get(expr, "kind").ok()?).ok()? != "const" {
        return None;
    }
    unsigned(get(get(expr, "value").ok()?, "value").ok()?).ok()
}
fn key(expr: &Value, types: &mut Types<'_>) -> Check<Vec<u8>> {
    let bytes = echo_edict_canonical::encode_canonical_cbor_v1(expr).map_err(|_| ())?;
    types.proof_work(bytes.len())?;
    Ok(bytes)
}
fn ceiling(
    expr: &Value,
    max: u64,
    caps: &[(Vec<u8>, u64)],
    types: &mut Types<'_>,
    depth: usize,
) -> Check<u64> {
    types.step(depth)?;
    let mut value = if string(get(expr, "kind")?)? == "call"
        && string(get(expr, "callee")?)? == "core.integer.add"
    {
        let [a, b] = sequence(get(expr, "args")?)? else {
            return Err(());
        };
        u128::from(ceiling(a, max, caps, types, depth + 1)?)
            + u128::from(ceiling(b, max, caps, types, depth + 1)?)
    } else {
        constant(expr).map_or_else(|| u128::from(max), u128::from)
    };
    let encoded = key(expr, types)?;
    for (term, cap) in caps {
        types.proof_work(encoded.len().min(term.len()).checked_add(1).ok_or(())?)?;
        if term == &encoded {
            value = value.min(u128::from(*cap));
        }
    }
    u64::try_from(value).map_err(|_| ())
}
