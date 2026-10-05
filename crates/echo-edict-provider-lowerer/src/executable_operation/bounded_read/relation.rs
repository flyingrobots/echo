// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Preserve ordered Core nodes and bind each read to its exported signature.

mod types;

use std::collections::{BTreeMap, BTreeSet};

use super::super::semantic_lawpack_member_coordinate;
use super::{
    array, as_map, field, invalid, map_field, require_exact_fields, require_resource_ref,
    required_u64, text, text_field, ClosureInputs, ProviderRefusalV1, Value, READ_PROFILE, SUBJECT,
};
use types::{ReadType, Scope};

const INTRINSIC: &str = "echo.dpo@1.node-atom-read";
const FAILURES: [&str; 4] = [
    "echo.atom-missing/v1",
    "echo.atom-required/v1",
    "echo.atom-type-mismatch/v1",
    "echo.atom-byte-bound-exceeded/v1",
];

pub(super) struct ReadRelation<'a> {
    pub core: &'a Value,
    pub exports: &'a Value,
    pub adapter: &'a Value,
    pub source: &'a str,
    pub closure: &'a ClosureInputs<'a>,
    pub max_reads: u64,
    pub max_bytes: u64,
}

impl ReadRelation<'_> {
    pub(super) fn validate(
        &self,
        intent: &Value,
        target: &Value,
        projection: &Value,
        coordinate: &str,
    ) -> Result<(), ProviderRefusalV1> {
        let mut target_fields = vec![
            "operationProfile",
            "basis",
            "inputConstraints",
            "coreEvaluationBudget",
            "requirements",
            "steps",
            "result",
            "executionOrder",
        ];
        if map_field(target, "pureBindings").is_some() {
            target_fields.push("pureBindings");
        }
        require_exact_fields(target, &target_fields, SUBJECT)?;
        if text(target, "operationProfile")? != READ_PROFILE
            || field(target, "coreEvaluationBudget")? != field(intent, "coreEvaluationBudget")?
            || field(target, "inputConstraints")? != field(intent, "inputConstraints")?
            || map_field(target, "basis") != map_field(intent, "basis")
            || map_field(target, "externalActionRequests").is_some()
        {
            return Err(invalid());
        }
        let (_, profile) =
            super::super::single_text_map_entry(field(self.adapter, "operationProfiles")?)
                .ok_or_else(invalid)?;
        let budget = field(intent, "coreEvaluationBudget")?;
        require_exact_fields(
            budget,
            &["maxSteps", "maxAllocatedBytes", "maxOutputBytes"],
            SUBJECT,
        )?;
        if field(
            field(self.adapter, "budgets")?,
            text(profile, "budgetObligation")?,
        )? != budget
        {
            return Err(invalid());
        }
        for key in ["maxSteps", "maxAllocatedBytes", "maxOutputBytes"] {
            if required_u64(budget, key, SUBJECT)? == 0 {
                return Err(invalid());
            }
        }
        let body = field(intent, "body")?;
        require_exact_fields(body, &["locals", "nodes", "result"], SUBJECT)?;
        let mut scope = Scope::new(self.core, array(body, "locals")?, text(intent, "input")?)?;
        if scope.expression(field(intent, "basis")?, 0)? != ReadType::Bytes(32, 32) {
            return Err(invalid());
        }
        for constraint in array(intent, "inputConstraints")? {
            scope.predicate(field(constraint, "predicate")?)?;
        }
        let mut ordered = BTreeMap::new();
        for (collection, kind) in [
            ("steps", "effect"),
            ("requirements", "require"),
            ("pureBindings", "let"),
        ] {
            if collection == "pureBindings" && map_field(target, collection).is_none() {
                continue;
            }
            for node in array(target, collection)? {
                if ordered.insert(text(node, "id")?, (kind, node)).is_some() {
                    return Err(invalid());
                }
            }
        }
        let nodes = array(body, "nodes")?;
        let order = array(target, "executionOrder")?;
        if nodes.len() != order.len() || nodes.len() != ordered.len() {
            return Err(invalid());
        }
        let mut producers = BTreeMap::new();
        let mut reads = 0;
        for (node, id) in nodes.iter().zip(order) {
            let Value::Text(id) = id else {
                return Err(invalid());
            };
            let (kind, lowered) = ordered.remove(id.as_str()).ok_or_else(invalid)?;
            if text(node, "kind")? != kind {
                return Err(invalid());
            }
            match kind {
                "let" => {
                    require_exact_fields(node, &["kind", "binding", "value"], SUBJECT)?;
                    require_exact_fields(lowered, &["id", "binding", "value"], SUBJECT)?;
                    for key in ["binding", "value"] {
                        equal(node, lowered, key)?;
                    }
                    let ty = scope.expression(field(node, "value")?, 0)?;
                    scope.introduce(field(node, "binding")?, Some(ty))?;
                    producers.insert(id.as_str(), ("pureBinding", field(node, "binding")?));
                }
                "effect" => {
                    reads += 1;
                    if reads > self.max_reads {
                        return Err(invalid());
                    }
                    self.read(node, lowered, profile, &mut scope)?;
                    producers.insert(id.as_str(), ("capabilityResult", field(node, "binding")?));
                }
                "require" => {
                    require_exact_fields(node, &["kind", "predicate", "onFailure"], SUBJECT)?;
                    require_exact_fields(lowered, &["id", "predicate", "onFailure"], SUBJECT)?;
                    equal(node, lowered, "predicate")?;
                    equal(node, lowered, "onFailure")?;
                    scope.predicate(field(node, "predicate")?)?;
                    let failure = field(node, "onFailure")?;
                    require_exact_fields(failure, &["kind", "reason"], SUBJECT)?;
                    if text(failure, "kind")? != "terminal" {
                        return Err(invalid());
                    }
                    let reason = field(failure, "reason")?;
                    require_exact_fields(reason, &["reasonKind", "payload"], SUBJECT)?;
                    if !entries(field(reason, "payload")?)?.is_empty() {
                        return Err(invalid());
                    }
                    self.obstruction(text(reason, "reasonKind")?, "domainMappable")?;
                }
                _ => return Err(invalid()),
            }
        }
        if reads == 0 || !ordered.is_empty() || field(body, "result")? != field(target, "result")? {
            return Err(invalid());
        }
        scope.matches_output(field(body, "result")?, text(intent, "output")?)?;
        scope.finish()?;
        require_exact_fields(
            projection,
            &[
                "schema",
                "expression",
                "operationCoordinate",
                "outputType",
                "maxOutputBytes",
            ],
            SUBJECT,
        )?;
        if text(projection, "schema")? != "edict.result-projection/v1"
            || text(projection, "operationCoordinate")? != coordinate
            || self.closure.result_projection.artifact.reference.coordinate != coordinate
            || text(projection, "outputType")? != text(intent, "output")?
            || field(projection, "maxOutputBytes")? != field(budget, "maxOutputBytes")?
        {
            return Err(invalid());
        }
        let projected = project(
            field(projection, "expression")?,
            scope.input(),
            &producers,
            0,
        )?;
        if super::encode_canonical_cbor_v1(&projected).map_err(|_| invalid())?
            != super::encode_canonical_cbor_v1(field(body, "result")?).map_err(|_| invalid())?
        {
            return Err(invalid());
        }
        Ok(())
    }

    fn member(&self, name: &str) -> Result<String, ProviderRefusalV1> {
        semantic_lawpack_member_coordinate(self.source, self.closure.lawpack, name)
    }

    fn obstruction(&self, name: &str, authority: &str) -> Result<(), ProviderRefusalV1> {
        let coordinate = self.member(name)?;
        let obstruction = unique_export(self.exports, "obstructions", &coordinate)?;
        if text(obstruction, "authorityClass")? != authority {
            return Err(invalid());
        }
        Ok(())
    }

    fn read<'a>(
        &self,
        node: &'a Value,
        lowered: &Value,
        profile: &Value,
        scope: &mut Scope<'a>,
    ) -> Result<(), ProviderRefusalV1> {
        require_exact_fields(
            node,
            &["kind", "binding", "effect", "input", "obstructionMap"],
            SUBJECT,
        )?;
        require_exact_fields(
            lowered,
            &[
                "id",
                "binding",
                "effect",
                "input",
                "targetIntrinsic",
                "obstructionArms",
                "obstructionFailures",
            ],
            SUBJECT,
        )?;
        for key in ["binding", "effect", "input"] {
            equal(node, lowered, key)?;
        }
        let coordinate = self.member(text(node, "effect")?)?;
        let effect = unique_export(self.exports, "effects", &coordinate)?;
        let implementation = field(field(self.adapter, "effectImplementations")?, &coordinate)?;
        if text(effect, "executionClass")? != "runtime"
            || text(effect, "effectKindHint")? != "read"
            || field(effect, "guardSupport")? != &Value::Bool(true)
            || text(effect, "costObligation")? != text(profile, "budgetObligation")?
            || !array(effect, "typeParameters")?.is_empty()
            || !array(profile, "semanticEffects")?.contains(&Value::Text(coordinate))
            || text(implementation, "writeClass")? != "read"
            || text(implementation, "targetIntrinsic")? != INTRINSIC
            || text(lowered, "targetIntrinsic")? != INTRINSIC
        {
            return Err(invalid());
        }
        for key in ["footprintObligation", "costObligation"] {
            equal(effect, implementation, key)?;
        }
        require_resource_ref(
            field(implementation, "targetConfiguration")?,
            &self.closure.configuration.artifact,
            SUBJECT,
        )?;
        let address = scope.ty(text(effect, "inputType")?)?;
        let address_export = unique_export(self.exports, "types", text(effect, "inputType")?)?;
        if text(address_export, "definition")?
            != "Record<nodeId:Bytes<exact=32>,typeId:Bytes<exact=32>,warpId:Bytes<exact=32>>"
        {
            return Err(invalid());
        }
        if !address.is_address() || scope.expression(field(node, "input")?, 0)? != address {
            return Err(invalid());
        }
        let output = scope.ty(text(effect, "outputType")?)?;
        if !matches!(output, ReadType::Bytes(0, max) if max > 0 && max <= self.max_bytes)
            || text(field(node, "binding")?, "type")? != text(effect, "outputType")?
        {
            return Err(invalid());
        }
        let ReadType::Bytes(_, max) = output else {
            return Err(invalid());
        };
        let output_export = unique_export(self.exports, "types", text(effect, "outputType")?)?;
        if text(output_export, "definition")? != format!("Bytes<max={max}>") {
            return Err(invalid());
        }
        let mappings = entries(field(implementation, "failureMappings")?)?;
        let arms = field(node, "obstructionMap")?;
        let failures = field(effect, "effectFailures")?;
        let lowered_arms = field(lowered, "obstructionArms")?;
        if mappings.len() != FAILURES.len()
            || entries(arms)?.len() != mappings.len()
            || entries(failures)?.len() != mappings.len()
            || entries(lowered_arms)?.len() != mappings.len()
            || array(lowered, "obstructionFailures")?.len() != mappings.len()
        {
            return Err(invalid());
        }
        let mut seen = BTreeSet::new();
        for (key, value) in mappings {
            let (Value::Text(name), Value::Text(target_failure)) = (key, value) else {
                return Err(invalid());
            };
            if !FAILURES.contains(&target_failure.as_str())
                || !seen.insert(target_failure)
                || !array(lowered, "obstructionFailures")?.contains(value)
            {
                return Err(invalid());
            }
            let failure = field(failures, name)?;
            let arm = field(arms, name)?;
            require_exact_fields(failure, &["authorityClass", "payloadType"], SUBJECT)?;
            require_exact_fields(arm, &["binder", "value"], SUBJECT)?;
            if arm != field(lowered_arms, target_failure)? {
                return Err(invalid());
            }
            let authority = if target_failure == "echo.atom-missing/v1" {
                "resourceFault"
            } else {
                "integrityFault"
            };
            if text(failure, "authorityClass")? != authority
                || text(failure, "payloadType")? != text(effect, "inputType")?
            {
                return Err(invalid());
            }
            scope.obstruction_binder(field(arm, "binder")?, &address)?;
            let value = field(arm, "value")?;
            require_exact_fields(value, &["kind", "callee", "args", "typeArgs"], SUBJECT)?;
            if text(value, "kind")? != "call"
                || !array(value, "args")?.is_empty()
                || !array(value, "typeArgs")?.is_empty()
            {
                return Err(invalid());
            }
            self.obstruction(text(value, "callee")?, authority)?;
        }
        scope.introduce(field(node, "binding")?, Some(output))
    }
}

fn equal(left: &Value, right: &Value, key: &str) -> Result<(), ProviderRefusalV1> {
    if field(left, key)? != field(right, key)? {
        return Err(invalid());
    }
    Ok(())
}

fn entries(value: &Value) -> Result<&[(Value, Value)], ProviderRefusalV1> {
    Ok(as_map(value).ok_or_else(invalid)?.as_slice())
}

fn unique_export<'a>(
    exports: &'a Value,
    collection: &str,
    coordinate: &str,
) -> Result<&'a Value, ProviderRefusalV1> {
    let mut matching = array(exports, collection)?
        .iter()
        .filter(|value| text_field(value, "coordinate") == Some(coordinate));
    let found = matching.next().ok_or_else(invalid)?;
    if matching.next().is_some() {
        return Err(invalid());
    }
    Ok(found)
}

fn project(
    value: &Value,
    input: &Value,
    producers: &BTreeMap<&str, (&str, &Value)>,
    depth: usize,
) -> Result<Value, ProviderRefusalV1> {
    use super::super::{canonical_map, canonical_text};
    if depth > 64 {
        return Err(invalid());
    }
    if text(value, "kind")? == "record" {
        require_exact_fields(value, &["kind", "fields"], SUBJECT)?;
        let fields = entries(field(value, "fields")?)?
            .iter()
            .map(|(key, expression)| {
                Ok((
                    key.clone(),
                    project(expression, input, producers, depth + 1)?,
                ))
            })
            .collect::<Result<Vec<_>, ProviderRefusalV1>>()?;
        return Ok(canonical_map([
            ("kind", canonical_text("record")),
            ("fields", Value::Map(fields)),
        ]));
    }
    if text(value, "kind")? != "source" {
        return Err(invalid());
    }
    require_exact_fields(value, &["kind", "path", "source"], SUBJECT)?;
    let source = field(value, "source")?;
    let kind = text(source, "kind")?;
    let local = if kind == "applicationInput" {
        require_exact_fields(source, &["kind"], SUBJECT)?;
        input
    } else {
        let key = match kind {
            "pureBinding" => "bindingId",
            "capabilityResult" => "stepId",
            _ => return Err(invalid()),
        };
        require_exact_fields(source, &["kind", key], SUBJECT)?;
        let (expected, local) = producers.get(text(source, key)?).ok_or_else(invalid)?;
        if kind != *expected {
            return Err(invalid());
        }
        local
    };
    let mut expression = canonical_map([("kind", canonical_text("local")), ("ref", local.clone())]);
    for part in array(value, "path")? {
        let Value::Text(name) = part else {
            return Err(invalid());
        };
        expression = canonical_map([
            ("kind", canonical_text("field")),
            ("base", expression),
            ("field", canonical_text(name)),
        ]);
    }
    Ok(expression)
}
