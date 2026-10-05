// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Invert target instructions into Core, then compare the complete ordered body.

mod reads;
mod types;

use super::super::{
    canonical_map, canonical_text, semantic_lawpack_member_coordinate, validate_compiler_lawpack,
    validate_source,
};
use super::{
    exact, get, members, number, package, reference, same, sequence, string, text, Check, Evidence,
    Value, PROFILE,
};
use std::collections::BTreeMap;
use types::{Schema, Symbols};

pub(super) fn check(e: &Evidence<'_>) -> Check<()> {
    exact(
        &e.configuration,
        &["apiVersion", "programKind", "maxReads", "maxReadBytes"],
    )?;
    if !(1..=65_536).contains(&number(get(&e.configuration, "maxReads")?)?)
        || !(1..=64 * 1024 * 1024).contains(&number(get(&e.configuration, "maxReadBytes")?)?)
    {
        return Err(());
    }
    exact(
        &e.core,
        &[
            "apiVersion",
            "coordinate",
            "imports",
            "types",
            "intents",
            "requiredCoreCapabilities",
        ],
    )?;
    if text(&e.core, "apiVersion")? != "edict.core/v1"
        || text(&e.core, "coordinate")? != e.request.core.reference.coordinate
        || !sequence(&e.core, "requiredCoreCapabilities")?.is_empty()
    {
        return Err(());
    }
    let [(name, intent)] = members(get(&e.core, "intents")?)? else {
        return Err(());
    };
    let name = string(name)?;
    let coordinate = format!("{}.{}", e.request.core.reference.coordinate, name);
    exact(
        intent,
        &[
            "input",
            "output",
            "body",
            "basis",
            "inputConstraints",
            "coreEvaluationBudget",
            "requiredOperationProfile",
        ],
    )?;
    if text(intent, "requiredOperationProfile")? != PROFILE {
        return Err(());
    }
    validate_compiler_lawpack(
        &e.lawpack,
        &e.exports,
        &e.adapter,
        e.closure,
        &e.request.target_profile,
        intent,
        e.request,
    )
    .map_err(|_| ())?;
    let [import] = sequence(&e.core, "imports")? else {
        return Err(());
    };
    if text(import, "kind")? != "lawpack" {
        return Err(());
    }
    reference(get(import, "ref")?, &e.closure.lawpack.artifact)?;
    exact(
        &e.target,
        &[
            "kind",
            "domain",
            "intents",
            "targetProfile",
            "semanticClosure",
            "sourceCoreCoordinate",
        ],
    )?;
    if text(&e.target, "domain")? != "echo.span-ir/v2"
        || e.request.target_ir.reference.coordinate != "echo.span-ir/v2"
        || text(&e.target, "kind")? != "orderedTargetIrArtifact"
        || text(&e.target, "sourceCoreCoordinate")? != e.request.core.reference.coordinate
    {
        return Err(());
    }
    reference(get(&e.target, "targetProfile")?, &e.request.target_profile)?;
    let semantic = get(&e.target, "semanticClosure")?;
    reference(get(semantic, "sourceCore")?, &e.request.core)?;
    let [lawpack] = sequence(semantic, "lawpacks")? else {
        return Err(());
    };
    reference(lawpack, &e.closure.lawpack.artifact)?;
    let [(target_name, target)] = members(get(&e.target, "intents")?)? else {
        return Err(());
    };
    if string(target_name)? != name {
        return Err(());
    }
    let mut expected_fields = vec![
        "basis",
        "steps",
        "result",
        "requirements",
        "executionOrder",
        "inputConstraints",
        "operationProfile",
        "coreEvaluationBudget",
    ];
    if super::map_field(target, "pureBindings").is_some() {
        expected_fields.push("pureBindings");
    }
    exact(target, &expected_fields)?;
    if text(target, "operationProfile")? != PROFILE {
        return Err(());
    }
    for key in ["basis", "inputConstraints", "coreEvaluationBudget"] {
        same(get(target, key)?, get(intent, key)?)?;
    }
    let budget = get(intent, "coreEvaluationBudget")?;
    exact(budget, &["maxSteps", "maxAllocatedBytes", "maxOutputBytes"])?;
    for key in ["maxSteps", "maxAllocatedBytes", "maxOutputBytes"] {
        if number(get(budget, key)?)? == 0 {
            return Err(());
        }
    }
    let [(_, profile)] = members(get(&e.adapter, "operationProfiles")?)? else {
        return Err(());
    };
    same(
        get(
            get(&e.adapter, "budgets")?,
            text(profile, "budgetObligation")?,
        )?,
        budget,
    )?;
    let body = get(intent, "body")?;
    exact(body, &["locals", "nodes", "result"])?;
    let mut symbols = Symbols::new(&e.core, sequence(body, "locals")?, text(intent, "input")?)?;
    if symbols.expression(get(target, "basis")?, 0)?
        != (Schema::Bytes {
            lower: 32,
            upper: 32,
        })
    {
        return Err(());
    }
    for constraint in sequence(target, "inputConstraints")? {
        symbols.predicate(get(constraint, "predicate")?)?;
    }
    let mut instructions = BTreeMap::new();
    for collection in ["steps", "requirements", "pureBindings"] {
        if collection == "pureBindings" && super::map_field(target, collection).is_none() {
            continue;
        }
        for instruction in sequence(target, collection)? {
            if instructions
                .insert(text(instruction, "id")?, (collection, instruction))
                .is_some()
            {
                return Err(());
            }
        }
    }
    let order = sequence(target, "executionOrder")?;
    if order.len() != instructions.len() {
        return Err(());
    }
    let mut reconstructed = Vec::new();
    let mut projection_locals = BTreeMap::new();
    let mut reads = 0;
    for id in order {
        let id = string(id)?;
        let (category, instruction) = instructions.remove(id).ok_or(())?;
        let node = match category {
            "steps" => {
                reads += 1;
                let node = reads::invert(e, profile, instruction, &mut symbols)?;
                projection_locals.insert(id, ("capabilityResult", get(instruction, "binding")?));
                node
            }
            "pureBindings" => {
                exact(instruction, &["id", "binding", "value"])?;
                let ty = symbols.expression(get(instruction, "value")?, 0)?;
                symbols.bind(get(instruction, "binding")?, ty)?;
                projection_locals.insert(id, ("pureBinding", get(instruction, "binding")?));
                canonical_map([
                    ("kind", canonical_text("let")),
                    ("binding", get(instruction, "binding")?.clone()),
                    ("value", get(instruction, "value")?.clone()),
                ])
            }
            "requirements" => {
                exact(instruction, &["id", "predicate", "onFailure"])?;
                symbols.predicate(get(instruction, "predicate")?)?;
                let failure = get(instruction, "onFailure")?;
                exact(failure, &["kind", "reason"])?;
                if text(failure, "kind")? != "terminal" {
                    return Err(());
                }
                let reason = get(failure, "reason")?;
                exact(reason, &["reasonKind", "payload"])?;
                if !members(get(reason, "payload")?)?.is_empty() {
                    return Err(());
                }
                obstruction(e, text(reason, "reasonKind")?, "domainMappable")?;
                canonical_map([
                    ("kind", canonical_text("require")),
                    ("predicate", get(instruction, "predicate")?.clone()),
                    ("onFailure", failure.clone()),
                ])
            }
            _ => return Err(()),
        };
        reconstructed.push(node);
    }
    if reads == 0 || reads > number(get(&e.configuration, "maxReads")?)? || !instructions.is_empty()
    {
        return Err(());
    }
    same(&Value::Array(reconstructed), get(body, "nodes")?)?;
    same(get(target, "result")?, get(body, "result")?)?;
    symbols.finish(get(target, "result")?, text(intent, "output")?)?;
    exact(
        &e.projection,
        &[
            "schema",
            "expression",
            "operationCoordinate",
            "outputType",
            "maxOutputBytes",
        ],
    )?;
    if text(&e.projection, "schema")? != "edict.result-projection/v1"
        || text(&e.projection, "operationCoordinate")? != coordinate
        || e.closure.result_projection.artifact.reference.coordinate != coordinate
        || text(&e.projection, "outputType")? != text(intent, "output")?
    {
        return Err(());
    }
    same(
        get(&e.projection, "maxOutputBytes")?,
        get(budget, "maxOutputBytes")?,
    )?;
    let result = projection(
        get(&e.projection, "expression")?,
        symbols.input()?,
        &projection_locals,
        0,
    )?;
    same(&result, get(target, "result")?)?;
    package(e, name, intent, &coordinate)
}

fn member(e: &Evidence<'_>, name: &str) -> Check<String> {
    let source = validate_source(&e.source, e.closure.source, &e.request.core).map_err(|_| ())?;
    semantic_lawpack_member_coordinate(source, e.closure.lawpack, name).map_err(|_| ())
}

fn exported<'a>(e: &'a Evidence<'_>, collection: &str, name: &str) -> Check<&'a Value> {
    let mut matching = sequence(&e.exports, collection)?
        .iter()
        .filter(|entry| super::text_field(entry, "coordinate") == Some(name));
    let found = matching.next().ok_or(())?;
    if matching.next().is_some() {
        return Err(());
    }
    Ok(found)
}

fn obstruction(e: &Evidence<'_>, name: &str, authority: &str) -> Check<()> {
    if text(
        exported(e, "obstructions", &member(e, name)?)?,
        "authorityClass",
    )? != authority
    {
        return Err(());
    }
    Ok(())
}

fn projection(
    value: &Value,
    input: &Value,
    locals: &BTreeMap<&str, (&str, &Value)>,
    depth: usize,
) -> Check<Value> {
    if depth > 64 {
        return Err(());
    }
    if text(value, "kind")? == "record" {
        exact(value, &["kind", "fields"])?;
        let mut fields = Vec::new();
        for (name, value) in members(get(value, "fields")?)? {
            string(name)?;
            fields.push((name.clone(), projection(value, input, locals, depth + 1)?));
        }
        return Ok(canonical_map([
            ("kind", canonical_text("record")),
            ("fields", Value::Map(fields)),
        ]));
    }
    exact(value, &["kind", "path", "source"])?;
    if text(value, "kind")? != "source" {
        return Err(());
    }
    let source = get(value, "source")?;
    let kind = text(source, "kind")?;
    let local = match kind {
        "applicationInput" => {
            exact(source, &["kind"])?;
            input
        }
        "pureBinding" | "capabilityResult" => {
            let key = if kind == "pureBinding" {
                "bindingId"
            } else {
                "stepId"
            };
            exact(source, &["kind", key])?;
            let (expected, local) = locals.get(text(source, key)?).ok_or(())?;
            if kind != *expected {
                return Err(());
            }
            local
        }
        _ => return Err(()),
    };
    let mut result = canonical_map([("kind", canonical_text("local")), ("ref", local.clone())]);
    for name in sequence(value, "path")? {
        result = canonical_map([
            ("kind", canonical_text("field")),
            ("base", result),
            ("field", canonical_text(string(name)?)),
        ]);
    }
    Ok(result)
}
