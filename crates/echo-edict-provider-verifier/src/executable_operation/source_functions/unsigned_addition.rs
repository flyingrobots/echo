// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned inventory of canonical-expression caps and independent composition.
use super::{get, sequence, string, unsigned, Check, Schema, Types, Value};

/// Whether an intent body calls an operation only this judgment checks: an
/// addition's no-overflow proof, a subtraction's or slice's range proof,
/// a count's full-byte scan charge, exhaustive variant dispatch, or bounded state iteration, or bounded lists.
/// A Core without a function table still needs the judgment for any of them.
pub(super) fn present(core: &Value) -> Check<bool> {
    let mut remaining = 65_536_usize;
    let mut layers = vec![core];
    while !layers.is_empty() {
        let mut next = Vec::new();
        for value in layers {
            remaining = remaining.checked_sub(1).ok_or(())?;
            if matches!(value, Value::Text(name) if name.starts_with("List<")) {
                return Ok(true);
            }
            if let Value::Map(entries) = value {
                if matches!(
                    get(value, "kind").and_then(string),
                    Ok("match" | "fold" | "list" | "List")
                ) {
                    return Ok(true);
                }
                // Each of these needs an intent-level proof, even without a
                // source-function table.
                if string(get(value, "kind").unwrap_or(&Value::Null)) == Ok("call")
                    && matches!(
                        string(get(value, "callee").unwrap_or(&Value::Null)),
                        Ok("core.integer.add"
                            | "core.integer.subtract"
                            | "core.bytes.slice"
                            | "core.bytes.at"
                            | "core.bytes.count"
                            | "core.list.length"
                            | "core.list.at"
                            | "core.list.append"
                            | "core.list.concat")
                    )
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

/// Proves an addition total and returns its refined cap, exactly the value
/// [`ceiling_of`] gives the same expression, so a binding need not recompute it.
pub(super) fn prove(
    expr: &Value,
    max: u64,
    facts: &[&Value],
    locals: &[(&Value, u64)],
    types: &mut Types<'_>,
) -> Check<u64> {
    let [left, right] = sequence(get(expr, "args")?)? else {
        return Err(());
    };
    let caps = caps(facts, types)?;
    let a = ceiling(left, max, &caps, locals, types, 0)?;
    let b = ceiling(right, max, &caps, locals, types, 0)?;
    let sum = u128::from(a) + u128::from(b);
    if sum > u128::from(max) {
        return Err(());
    }
    refined(expr, sum, &caps, types)
}
/// The proven cap of one expression, for a binding's carried bound.
pub(super) fn ceiling_of(
    expr: &Value,
    max: u64,
    facts: &[&Value],
    locals: &[(&Value, u64)],
    types: &mut Types<'_>,
) -> Check<u64> {
    let caps = caps(facts, types)?;
    ceiling(expr, max, &caps, locals, types, 0)
}
fn caps(facts: &[&Value], types: &mut Types<'_>) -> Check<Vec<(Vec<u8>, u64)>> {
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
    Ok(caps)
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
    locals: &[(&Value, u64)],
    types: &mut Types<'_>,
    depth: usize,
) -> Check<u64> {
    types.step(depth)?;
    let value = if string(get(expr, "kind")?)? == "call"
        && string(get(expr, "callee")?)? == "core.integer.add"
    {
        let [a, b] = sequence(get(expr, "args")?)? else {
            return Err(());
        };
        u128::from(ceiling(a, max, caps, locals, types, depth + 1)?)
            + u128::from(ceiling(b, max, caps, locals, types, depth + 1)?)
    } else if string(get(expr, "kind")?)? == "call"
        && string(get(expr, "callee")?)? == "core.bytes.count"
    {
        // A count never exceeds its operand's structural byte maximum, resolved
        // through the module's type table so named byte types count too.
        let [coordinate] = sequence(get(expr, "typeArgs")?)? else {
            return Err(());
        };
        let Schema::Blob { high, .. } = types
            .resolve(string(coordinate)?, depth + 1)?
            .representation()
            .clone()
        else {
            return Err(());
        };
        u128::from(high.min(max))
    } else if string(get(expr, "kind")?)? == "local" {
        let reference = get(expr, "ref")?;
        let mut upper = max;
        for (local, cap) in locals {
            types.proof_work(1)?;
            if *local == reference {
                upper = upper.min(*cap);
            }
        }
        u128::from(upper)
    } else {
        constant(expr).map_or_else(|| u128::from(max), u128::from)
    };
    refined(expr, value, caps, types)
}
/// Narrows a structural cap by every proven cap on exactly `expr`.
fn refined(
    expr: &Value,
    mut value: u128,
    caps: &[(Vec<u8>, u64)],
    types: &mut Types<'_>,
) -> Check<u64> {
    let encoded = key(expr, types)?;
    for (term, cap) in caps {
        types.proof_work(encoded.len().min(term.len()).checked_add(1).ok_or(())?)?;
        if term == &encoded {
            value = value.min(u128::from(*cap));
        }
    }
    u64::try_from(value).map_err(|_| ())
}
