// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independently reconstruct a Core effect from a declared target read.

use super::types::{Schema, Symbols};
use super::{
    canonical_map, canonical_text, exact, exported, get, member, members, number, obstruction,
    reference, same, sequence, string, text, Check, Evidence, Value,
};
use std::collections::BTreeSet;

pub(super) fn invert(
    e: &Evidence<'_>,
    profile: &Value,
    step: &Value,
    symbols: &mut Symbols<'_>,
) -> Check<Value> {
    exact(
        step,
        &[
            "id",
            "binding",
            "effect",
            "input",
            "targetIntrinsic",
            "obstructionArms",
            "obstructionFailures",
        ],
    )?;
    let effect_name = member(e, text(step, "effect")?)?;
    let effect = exported(e, "effects", &effect_name)?;
    let implementation = get(get(&e.adapter, "effectImplementations")?, &effect_name)?;
    if text(step, "targetIntrinsic")? != "echo.dpo@1.node-atom-read"
        || text(implementation, "targetIntrinsic")? != "echo.dpo@1.node-atom-read"
        || text(implementation, "writeClass")? != "read"
        || text(effect, "executionClass")? != "runtime"
        || text(effect, "effectKindHint")? != "read"
        || get(effect, "guardSupport")? != &Value::Bool(true)
        || !sequence(effect, "typeParameters")?.is_empty()
        || text(effect, "costObligation")? != text(profile, "budgetObligation")?
        || !sequence(profile, "semanticEffects")?.contains(&canonical_text(&effect_name))
    {
        return Err(());
    }
    for key in ["costObligation", "footprintObligation"] {
        same(get(effect, key)?, get(implementation, key)?)?;
    }
    reference(
        get(implementation, "targetConfiguration")?,
        &e.closure.configuration.artifact,
    )?;
    let address_name = text(effect, "inputType")?;
    let address = symbols.schema(address_name, 0)?;
    if !address.address()
        || symbols.expression(get(step, "input")?, 0)? != address
        || text(exported(e, "types", address_name)?, "definition")?
            != "Record<nodeId:Bytes<exact=32>,typeId:Bytes<exact=32>,warpId:Bytes<exact=32>>"
    {
        return Err(());
    }
    let output_name = text(effect, "outputType")?;
    let output = symbols.schema(output_name, 0)?;
    let Schema::Bytes { lower: 0, upper } = output else {
        return Err(());
    };
    if upper == 0
        || upper > number(get(&e.configuration, "maxReadBytes")?)?
        || text(get(step, "binding")?, "type")? != output_name
        || text(exported(e, "types", output_name)?, "definition")? != format!("Bytes<max={upper}>")
    {
        return Err(());
    }
    let failures = get(effect, "effectFailures")?;
    let mappings = members(get(implementation, "failureMappings")?)?;
    let arms = get(step, "obstructionArms")?;
    let expected = BTreeSet::from([
        "echo.atom-missing/v1",
        "echo.atom-required/v1",
        "echo.atom-type-mismatch/v1",
        "echo.atom-byte-bound-exceeded/v1",
    ]);
    if mappings.len() != expected.len()
        || members(failures)?.len() != expected.len()
        || members(arms)?.len() != expected.len()
    {
        return Err(());
    }
    let failure_list: Vec<_> = expected
        .iter()
        .map(|failure| canonical_text(failure))
        .collect();
    if sequence(step, "obstructionFailures")? != failure_list {
        return Err(());
    }
    let mut seen = BTreeSet::new();
    let mut original_arms = Vec::new();
    for (original, target_failure) in mappings {
        let name = string(original)?;
        let target_failure = string(target_failure)?;
        if !expected.contains(target_failure) || !seen.insert(target_failure) {
            return Err(());
        }
        let failure = get(failures, name)?;
        exact(failure, &["payloadType", "authorityClass"])?;
        let authority = if target_failure == "echo.atom-missing/v1" {
            "resourceFault"
        } else {
            "integrityFault"
        };
        if text(failure, "payloadType")? != address_name
            || text(failure, "authorityClass")? != authority
        {
            return Err(());
        }
        let arm = get(arms, target_failure)?;
        exact(arm, &["binder", "value"])?;
        symbols.claim_failure(get(arm, "binder")?, &address)?;
        let call = get(arm, "value")?;
        exact(call, &["kind", "callee", "args", "typeArgs"])?;
        if text(call, "kind")? != "call"
            || !sequence(call, "args")?.is_empty()
            || !sequence(call, "typeArgs")?.is_empty()
        {
            return Err(());
        }
        obstruction(e, text(call, "callee")?, authority)?;
        original_arms.push((original.clone(), arm.clone()));
    }
    symbols.bind(get(step, "binding")?, output)?;
    Ok(canonical_map([
        ("kind", canonical_text("effect")),
        ("effect", get(step, "effect")?.clone()),
        ("binding", get(step, "binding")?.clone()),
        ("input", get(step, "input")?.clone()),
        ("obstructionMap", Value::Map(original_arms)),
    ]))
}
