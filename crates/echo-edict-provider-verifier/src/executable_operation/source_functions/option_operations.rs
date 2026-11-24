// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned Option signatures, presence proofs and conservative charges.
//!
//! Each `core.option.<operation><T>` call is rechecked from its operands and
//! its single item argument; `unwrap` additionally needs a dominating fact
//! that proves its local operand present.
use super::{get, sequence, string, types::Types, Charge, Check, Schema, Value};

const CAPABILITY: &str = "core.option/v1";
const IS_SOME: &str = "core.option.isSome";
const UNWRAP: &str = "core.option.unwrap";

/// Every `core.option.*` callee is gated; unknown operations are refused.
pub(super) fn is_operation(name: &str) -> bool {
    name.starts_with("core.option.")
}

pub(super) fn authority(core: &Value) -> Check<()> {
    if sequence(get(core, "requiredCoreCapabilities")?)?
        .iter()
        .any(|v| string(v) == Ok(CAPABILITY))
    {
        Ok(())
    } else {
        Err(())
    }
}

/// Whether `schema` contains an Option anywhere. Echo defines no Option wire
/// encoding, so an intent's input and output contain none.
pub(super) fn contains_option(schema: &Schema) -> bool {
    match schema {
        Schema::Optional(_) => true,
        Schema::Sequence { element, .. } => element.as_deref().is_some_and(contains_option),
        Schema::Fields(fields) => fields.values().any(contains_option),
        Schema::Nominal { representation, .. } => contains_option(representation),
        Schema::Alternatives { cases, .. } => cases.values().flatten().any(contains_option),
        Schema::Truth | Schema::Word(_) | Schema::Blob { .. } => false,
    }
}

/// The result schema, the call's own charge and the Option's height.
/// `present` says whether a dominating fact proves an `unwrap` operand.
pub(super) fn infer(
    name: &str,
    actual: &[Schema],
    declared: &[Schema],
    present: bool,
) -> Check<(Schema, Charge, usize)> {
    let [item] = declared else {
        return Err(());
    };
    let option = Schema::Optional(Box::new(item.clone()));
    let inner = item.measure()?;
    let outer = option.measure()?;
    let checks = |extra: u128, storage: u128| -> Check<Charge> {
        Ok(Charge {
            ticks: outer.checks.checked_add(extra).ok_or(())?,
            storage,
        })
    };
    let (schema, charge) = match (name, actual) {
        ("core.option.some", [payload]) if item.encloses(payload) => (
            option,
            Charge {
                ticks: inner.checks,
                storage: 64,
            },
        ),
        ("core.option.none", []) => (
            option,
            Charge {
                ticks: 1,
                storage: 64,
            },
        ),
        (IS_SOME, [operand]) if *operand == option => (Schema::Truth, checks(1, 64)?),
        ("core.option.default", [operand, fallback])
            if *operand == option && item.encloses(fallback) =>
        {
            (item.clone(), checks(0, 0)?)
        }
        (UNWRAP, [operand]) if *operand == option && present => (item.clone(), checks(0, 0)?),
        _ => return Err(()),
    };
    Ok((schema, charge, outer.height))
}

/// Whether `arguments` is one local that a fact in `proven` (true) or `path`
/// (with its truth) shows present.
pub(super) fn present(
    arguments: &[Value],
    proven: &[&Value],
    path: &[(&Value, bool)],
    types: &mut Types<'_>,
) -> Check<bool> {
    let [argument] = arguments else {
        return Ok(false);
    };
    if string(get(argument, "kind")?)? != "local" {
        return Ok(false);
    }
    let local = get(argument, "ref")?;
    let mut pending: Vec<(&Value, bool, usize)> = proven
        .iter()
        .map(|fact| (*fact, true, 0))
        .chain(path.iter().map(|(fact, truth)| (*fact, *truth, 0)))
        .collect();
    while let Some((fact, truth, depth)) = pending.pop() {
        types.step(depth)?;
        match string(get(fact, "kind")?)? {
            "not" => pending.push((get(fact, "value")?, !truth, depth + 1)),
            // Only a true `all` or a false `any` fixes each item's truth.
            "all" if truth => {
                for item in sequence(get(fact, "values")?)? {
                    pending.push((item, true, depth + 1));
                }
            }
            "any" if !truth => {
                for item in sequence(get(fact, "values")?)? {
                    pending.push((item, false, depth + 1));
                }
            }
            "compare" if tests(fact, truth, local)? => return Ok(true),
            _ => {}
        }
    }
    Ok(false)
}

/// Whether the comparison `fact`, with the given truth, is
/// `isSome(local) == true` (or an equivalent `!=`/`false` spelling).
fn tests(fact: &Value, truth: bool, local: &Value) -> Check<bool> {
    let call = get(fact, "left")?;
    let constant = get(fact, "right")?;
    if string(get(call, "kind")?)? != "call" || string(get(call, "callee")?)? != IS_SOME {
        return Ok(false);
    }
    let [operand] = sequence(get(call, "args")?)? else {
        return Ok(false);
    };
    if string(get(operand, "kind")?)? != "local"
        || get(operand, "ref")? != local
        || string(get(constant, "kind")?)? != "const"
    {
        return Ok(false);
    }
    let literal = get(constant, "value")?;
    let (Ok("bool"), Value::Bool(expected)) =
        (string(get(literal, "kind")?), get(literal, "value")?)
    else {
        return Ok(false);
    };
    Ok(match string(get(fact, "op")?)? {
        "==" => truth == *expected,
        "!=" => truth != *expected,
        _ => false,
    })
}
