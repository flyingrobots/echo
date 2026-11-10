// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independently reconstruct a Core effect from a declared target read.

use super::graph;
use super::types::{Schema, Symbols};
use super::{
    canonical_map, canonical_text, exact, exported, get, member, members, number, obstruction,
    reference, same, sequence, string, text, Check, Evidence, Value,
};
use std::collections::BTreeSet;

const NODE_ATOM: &str = "echo.dpo@1.node-atom-read";

/// Runtime failures of a node-atom read, each with its required authority class.
const NODE_ATOM_OUTCOMES: [(&str, &str); 4] = [
    ("echo.atom-byte-bound-exceeded/v1", "integrityFault"),
    ("echo.atom-missing/v1", "resourceFault"),
    ("echo.atom-required/v1", "integrityFault"),
    ("echo.atom-type-mismatch/v1", "integrityFault"),
];

/// One reconstructed Core effect and whether it discovers graph material.
pub(super) struct Inverted {
    pub node: Value,
    pub graph_read: bool,
}

pub(super) fn invert(
    e: &Evidence<'_>,
    profile: &Value,
    step: &Value,
    declared: Option<&graph::Declaration>,
    symbols: &mut Symbols<'_>,
) -> Check<Inverted> {
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
    let intrinsic = text(step, "targetIntrinsic")?;
    if text(implementation, "targetIntrinsic")? != intrinsic
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
    let input_name = text(effect, "inputType")?;
    let output_name = text(effect, "outputType")?;
    let input = symbols.schema(input_name, 0)?;
    let output = symbols.schema(output_name, 0)?;
    if symbols.expression(get(step, "input")?, 0)? != input
        || text(get(step, "binding")?, "type")? != output_name
    {
        return Err(());
    }
    let read_bytes = number(get(&e.configuration, "maxReadBytes")?)?;
    let outcomes: &[(&str, &str)] = match intrinsic {
        NODE_ATOM => {
            node_atom(e, input_name, output_name, &input, &output, read_bytes)?;
            &NODE_ATOM_OUTCOMES
        }
        graph::EDGE_INTRINSIC if declared.is_some() => {
            definitions(e, symbols, [(input_name, &input), (output_name, &output)])?;
            graph::edge_signature(&input, &output)?;
            &graph::EDGE_OUTCOMES
        }
        graph::CLOSURE_INTRINSIC if declared.is_some() => {
            definitions(e, symbols, [(input_name, &input), (output_name, &output)])?;
            let guided = graph::closure_signature(&input, &output, read_bytes)?;
            graph::selector(&e.core, symbols, declared, &guided)?;
            &graph::CLOSURE_OUTCOMES
        }
        _ => return Err(()),
    };
    let original_arms = arms(e, effect, implementation, step, outcomes, &input, symbols)?;
    symbols.bind(get(step, "binding")?, output)?;
    Ok(Inverted {
        node: canonical_map([
            ("kind", canonical_text("effect")),
            ("effect", get(step, "effect")?.clone()),
            ("binding", get(step, "binding")?.clone()),
            ("input", get(step, "input")?.clone()),
            ("obstructionMap", Value::Map(original_arms)),
        ]),
        graph_read: intrinsic != NODE_ATOM,
    })
}

/// A node-atom read takes the exact typed address and returns bounded bytes.
fn node_atom(
    e: &Evidence<'_>,
    address_name: &str,
    output_name: &str,
    address: &Schema,
    output: &Schema,
    read_bytes: u64,
) -> Check<()> {
    if !address.address()
        || text(exported(e, "types", address_name)?, "definition")?
            != "Record<nodeId:Bytes<exact=32>,typeId:Bytes<exact=32>,warpId:Bytes<exact=32>>"
    {
        return Err(());
    }
    let Schema::Bytes { lower: 0, upper } = *output else {
        return Err(());
    };
    if upper == 0
        || upper > read_bytes
        || text(exported(e, "types", output_name)?, "definition")? != format!("Bytes<max={upper}>")
    {
        return Err(());
    }
    Ok(())
}

/// Each exported definition must expand to the schema Core gives its coordinate.
fn definitions(
    e: &Evidence<'_>,
    symbols: &mut Symbols<'_>,
    signature: [(&str, &Schema); 2],
) -> Check<()> {
    for (name, resolved) in signature {
        let definition = text(exported(e, "types", name)?, "definition")?;
        if &symbols.schema(definition, 0)? != resolved {
            return Err(());
        }
    }
    Ok(())
}

/// Reverses the adapter's failure mapping into the authored obstruction map,
/// requiring one arm of the right authority class for every runtime outcome.
fn arms(
    e: &Evidence<'_>,
    effect: &Value,
    implementation: &Value,
    step: &Value,
    outcomes: &[(&str, &str)],
    payload: &Schema,
    symbols: &mut Symbols<'_>,
) -> Check<Vec<(Value, Value)>> {
    let failures = get(effect, "effectFailures")?;
    let mappings = members(get(implementation, "failureMappings")?)?;
    let declared = get(step, "obstructionArms")?;
    if mappings.len() != outcomes.len()
        || members(failures)?.len() != outcomes.len()
        || members(declared)?.len() != outcomes.len()
    {
        return Err(());
    }
    // The outcome tables are sorted, as the canonical target failure set is.
    let expected: Vec<_> = outcomes
        .iter()
        .map(|(outcome, _)| canonical_text(outcome))
        .collect();
    if sequence(step, "obstructionFailures")? != expected {
        return Err(());
    }
    let mut seen = BTreeSet::new();
    let mut original = Vec::new();
    for (name, target_failure) in mappings {
        let target_failure = string(target_failure)?;
        let authority = outcomes
            .iter()
            .find_map(|(outcome, class)| (*outcome == target_failure).then_some(*class))
            .ok_or(())?;
        if !seen.insert(target_failure) {
            return Err(());
        }
        let failure = get(failures, string(name)?)?;
        exact(failure, &["payloadType", "authorityClass"])?;
        if text(failure, "payloadType")? != text(effect, "inputType")?
            || text(failure, "authorityClass")? != authority
        {
            return Err(());
        }
        let arm = get(declared, target_failure)?;
        exact(arm, &["binder", "value"])?;
        symbols.claim_failure(get(arm, "binder")?, payload)?;
        let call = get(arm, "value")?;
        exact(call, &["kind", "callee", "args", "typeArgs"])?;
        if text(call, "kind")? != "call"
            || !sequence(call, "args")?.is_empty()
            || !sequence(call, "typeArgs")?.is_empty()
        {
            return Err(());
        }
        obstruction(e, text(call, "callee")?, authority)?;
        original.push((name.clone(), arm.clone()));
    }
    Ok(original)
}
