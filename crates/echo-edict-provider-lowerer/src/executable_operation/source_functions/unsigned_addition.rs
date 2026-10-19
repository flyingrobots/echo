// SPDX-License-Identifier: Apache-2.0
//! Lowerer-owned recursive upper bounds over checked, dominating predicates.
use super::{get, list, literal, string, Check, Value};

pub(super) fn present(core: &Value) -> Check<bool> {
    let mut pending = vec![get(core, "intents")?];
    while let Some(value) = pending.pop() {
        match value {
            Value::Map(fields) => {
                if get(value, "kind").and_then(string) == Ok("call")
                    && get(value, "callee").and_then(string) == Ok("core.integer.add")
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

pub(super) fn prove(args: &[Value], max: u64, facts: &[&Value], work: &mut usize) -> Check<()> {
    let [left, right] = args else { return Err(()) };
    let left = bound(left, max, facts, work, 0)?;
    let right = bound(right, max, facts, work, 0)?;
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
fn bound(expr: &Value, max: u64, facts: &[&Value], work: &mut usize, depth: usize) -> Check<u64> {
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
        bound(left, max, facts, work, depth + 1)?
            .checked_add(bound(right, max, facts, work, depth + 1)?)
            .ok_or(())?
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
