// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned recursive upper bounds over checked, dominating predicates.
use super::{get, list, literal, string, Check, Value};

pub(super) fn present(core: &Value) -> Check<bool> {
    let mut pending = vec![get(core, "intents")?];
    while let Some(value) = pending.pop() {
        match value {
            Value::Map(fields) => {
                // Each of these needs an intent-level proof, even without a
                // source-function table.
                if get(value, "kind").and_then(string) == Ok("call")
                    && matches!(
                        get(value, "callee").and_then(string),
                        Ok("core.integer.add" | "core.integer.subtract" | "core.bytes.slice")
                    )
                {
                    return Ok(true);
                }
                pending.extend(fields.iter().map(|(_, value)| value));
            }
            Value::Array(values) => pending.extend(values),
            _ => {}
        }
    }
    Ok(false)
}

pub(super) fn prove(
    args: &[Value],
    max: u64,
    facts: &[&Value],
    locals: &[(&Value, u64)],
    work: &mut usize,
) -> Check<()> {
    let [left, right] = args else { return Err(()) };
    let left = bound(left, max, facts, locals, work, 0)?;
    let right = bound(right, max, facts, locals, work, 0)?;
    left.checked_add(right)
        .filter(|sum| *sum <= max)
        .ok_or(())?;
    Ok(())
}
fn debit(work: &mut usize, amount: usize) -> Check<()> {
    *work = work.checked_sub(amount).ok_or(())?;
    Ok(())
}
fn same(a: &Value, b: &Value, work: &mut usize) -> Check<bool> {
    for value in [a, b] {
        let bytes = echo_edict_canonical::encode_canonical_cbor_v1(value).map_err(|_| ())?;
        debit(work, bytes.len())?;
    }
    Ok(a == b)
}
/// The proven cap of one expression, for a binding's carried bound.
pub(super) fn ceiling(
    expr: &Value,
    max: u64,
    facts: &[&Value],
    locals: &[(&Value, u64)],
    work: &mut usize,
) -> Check<u64> {
    bound(expr, max, facts, locals, work, 0)
}
fn bound(
    expr: &Value,
    max: u64,
    facts: &[&Value],
    locals: &[(&Value, u64)],
    work: &mut usize,
    depth: usize,
) -> Check<u64> {
    debit(work, 1)?;
    if depth > 64 {
        return Err(());
    }
    let mut upper = if string(get(expr, "kind")?)? == "call"
        && string(get(expr, "callee")?)? == "core.integer.add"
    {
        let [left, right] = list(get(expr, "args")?)? else {
            return Err(());
        };
        bound(left, max, facts, locals, work, depth + 1)?
            .checked_add(bound(right, max, facts, locals, work, depth + 1)?)
            .ok_or(())?
    } else if string(get(expr, "kind")?)? == "call"
        && string(get(expr, "callee")?)? == "core.bytes.count"
    {
        // A count never exceeds its operand's structural byte maximum.
        let [coordinate] = list(get(expr, "typeArgs")?)? else {
            return Err(());
        };
        bytes_max(string(coordinate)?).map_or(max, |bytes| bytes.min(max))
    } else if string(get(expr, "kind")?)? == "local" {
        let reference = get(expr, "ref")?;
        let mut upper = max;
        for (local, cap) in locals {
            if same(local, reference, work)? {
                upper = upper.min(*cap);
            }
        }
        upper
    } else {
        literal(expr).unwrap_or(max)
    };
    for predicate in facts {
        refine(predicate, expr, &mut upper, work, depth + 1)?;
    }
    Ok(upper)
}
fn refine(
    predicate: &Value,
    expr: &Value,
    upper: &mut u64,
    work: &mut usize,
    depth: usize,
) -> Check<()> {
    debit(work, 1)?;
    if depth > 64 {
        return Err(());
    }
    match string(get(predicate, "kind")?)? {
        "all" => {
            for child in list(get(predicate, "values")?)? {
                refine(child, expr, upper, work, depth + 1)?;
            }
        }
        "compare" => {
            let op = string(get(predicate, "op")?)?;
            let left = get(predicate, "left")?;
            let right = get(predicate, "right")?;
            if matches!(op, "<=" | "==") {
                if let Ok(limit) = literal(right) {
                    if same(left, expr, work)? {
                        *upper = (*upper).min(limit);
                    }
                }
            }
            if op == "==" {
                if let Ok(limit) = literal(left) {
                    if same(right, expr, work)? {
                        *upper = (*upper).min(limit);
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
fn bytes_max(coordinate: &str) -> Option<u64> {
    let bounds = coordinate.strip_prefix("Bytes<")?.strip_suffix('>')?;
    let max = bounds
        .strip_prefix("exact=")
        .or_else(|| bounds.strip_prefix("max="))
        .or_else(|| bounds.split_once(",max=").map(|(_, max)| max))?;
    max.parse().ok()
}
