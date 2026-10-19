// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Ordered Core/Target correspondence and exact lexical local identities.
use crate::edict_pure::{
    functions,
    model::{MAX_DEPTH, MAX_PROGRAM_NODES},
    syntax::Parser,
    values::{array, exact_fields, field, map, require_text, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(module: &Value, intent: &Value, target: &Value) -> Result<(), Error> {
    let mut fields = vec![
        "operationProfile",
        "inputConstraints",
        "coreEvaluationBudget",
        "steps",
        "requirements",
        "result",
        "executionOrder",
    ];
    for key in ["basis", "pureBindings"] {
        if field(target, key).is_ok() {
            fields.push(key);
        }
    }
    exact_fields(target, &fields)?;
    if field(intent, "optic").is_ok() {
        return Err(Error::UnsupportedProgram);
    }
    let body = field(intent, "body")?;
    let mut scope = Scope::new(
        module,
        array(field(body, "locals")?)?,
        text_field(intent, "input")?,
    )?;
    scope.parser.ty(text_field(intent, "output")?, 0)?;
    if let Ok(basis) = field(intent, "basis") {
        scope.expression(basis, None)?;
    }
    for constraint in array(field(intent, "inputConstraints")?)? {
        scope.predicate(field(constraint, "predicate")?)?;
    }
    let mut lowered = BTreeMap::new();
    for (collection, kind) in [
        ("steps", "effect"),
        ("requirements", "require"),
        ("pureBindings", "let"),
    ] {
        if collection == "pureBindings" && field(target, collection).is_err() {
            continue;
        }
        for node in array(field(target, collection)?)? {
            let id = text_field(node, "id")?;
            if id.is_empty() || lowered.insert(id, (kind, node)).is_some() {
                return Err(Error::InvalidArtifact);
            }
        }
    }
    let nodes = array(field(body, "nodes")?)?;
    let order = array(field(target, "executionOrder")?)?;
    if nodes.len() != order.len() || nodes.len() != lowered.len() {
        return Err(Error::InvalidArtifact);
    }
    for (node, id) in nodes.iter().zip(order) {
        scope.parser.enter(0)?;
        let (kind, target) = lowered.remove(text(id)?).ok_or(Error::InvalidArtifact)?;
        require_text(node, "kind", kind)?;
        match kind {
            "let" => {
                exact_fields(node, &["kind", "binding", "value"])?;
                exact_fields(target, &["id", "binding", "value"])?;
                equal(node, target, "binding")?;
                equal(node, target, "value")?;
                scope.expression(field(node, "value")?, None)?;
                scope.introduce(field(node, "binding")?)?;
            }
            "require" => {
                exact_fields(node, &["kind", "predicate", "onFailure"])?;
                exact_fields(target, &["id", "predicate", "onFailure"])?;
                equal(node, target, "predicate")?;
                equal(node, target, "onFailure")?;
                scope.predicate(field(node, "predicate")?)?;
                let failure = field(node, "onFailure")?;
                exact_fields(failure, &["kind", "reason"])?;
                require_text(failure, "kind", "terminal")?;
                let reason = field(failure, "reason")?;
                exact_fields(reason, &["reasonKind", "payload"])?;
                if text_field(reason, "reasonKind")?.is_empty() {
                    return Err(Error::InvalidArtifact);
                }
                for (_, value) in map(field(reason, "payload")?)? {
                    scope.expression(value, None)?;
                }
            }
            "effect" => effect(node, target, &mut scope)?,
            _ => return Err(Error::UnsupportedProgram),
        }
    }
    scope.expression(field(body, "result")?, None)?;
    if scope.used.len() != scope.inventory.len() {
        return Err(Error::InvalidArtifact);
    }
    Ok(())
}
fn equal(core: &Value, target: &Value, key: &str) -> Result<(), Error> {
    if field(core, key)? != field(target, key)? {
        return Err(Error::InvalidArtifact);
    }
    Ok(())
}
fn effect(core: &Value, target: &Value, scope: &mut Scope<'_>) -> Result<(), Error> {
    exact_fields(
        core,
        &["kind", "binding", "effect", "input", "obstructionMap"],
    )?;
    exact_fields(
        target,
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
    for key in ["binding", "effect", "input"] {
        equal(core, target, key)?;
    }
    if text_field(target, "targetIntrinsic")?.is_empty() || text_field(core, "effect")?.is_empty() {
        return Err(Error::InvalidArtifact);
    }
    scope.expression(field(core, "input")?, None)?;
    let mut arms = BTreeSet::new();
    for (_, arm) in map(field(core, "obstructionMap")?)? {
        exact_fields(arm, &["binder", "value"])?;
        let binder = scope.claim(field(arm, "binder")?)?;
        scope.expression(field(arm, "value")?, Some(binder))?;
        if !arms.insert(encode(arm).map_err(|_| Error::InvalidArtifact)?) {
            return Err(Error::InvalidArtifact);
        }
    }
    let mut failures = BTreeSet::new();
    for failure in array(field(target, "obstructionFailures")?)? {
        let failure = text(failure)?;
        if failure.is_empty() || !failures.insert(failure) {
            return Err(Error::InvalidArtifact);
        }
    }
    for (name, arm) in map(field(target, "obstructionArms")?)? {
        if !failures.remove(text(name)?)
            || !arms.remove(&encode(arm).map_err(|_| Error::InvalidArtifact)?)
        {
            return Err(Error::InvalidArtifact);
        }
    }
    if !arms.is_empty() || !failures.is_empty() {
        return Err(Error::InvalidArtifact);
    }
    scope.introduce(field(core, "binding")?)?;
    Ok(())
}
struct Scope<'a> {
    inventory: BTreeMap<&'a str, &'a Value>,
    available: BTreeMap<&'a str, &'a Value>,
    used: BTreeSet<&'a str>,
    parser: Parser<'a>,
    remaining_references: usize,
}
impl<'a> Scope<'a> {
    fn new(module: &'a Value, locals: &'a [Value], input_type: &str) -> Result<Self, Error> {
        let mut parser = Parser {
            types: field(module, "types")?,
            coordinate: text_field(module, "coordinate")?,
            remaining: MAX_PROGRAM_NODES,
        };
        let mut inventory = BTreeMap::new();
        for local in locals {
            functions::local(local)?;
            parser.ty(text_field(local, "type")?, 0)?;
            if inventory.insert(text_field(local, "id")?, local).is_some() {
                return Err(Error::InvalidArtifact);
            }
        }
        let input = *inventory.get("arg.0").ok_or(Error::InvalidArtifact)?;
        if text_field(input, "type")? != input_type {
            return Err(Error::InvalidArtifact);
        }
        Ok(Self {
            inventory,
            available: BTreeMap::from([("arg.0", input)]),
            used: BTreeSet::from(["arg.0"]),
            parser,
            remaining_references: MAX_PROGRAM_NODES,
        })
    }
    fn claim(&mut self, reference: &Value) -> Result<&'a Value, Error> {
        let id = text_field(reference, "id")?;
        let declared = *self.inventory.get(id).ok_or(Error::InvalidArtifact)?;
        let id = text_field(declared, "id")?;
        if declared != reference || self.available.contains_key(id) || !self.used.insert(id) {
            return Err(Error::InvalidArtifact);
        }
        Ok(declared)
    }
    fn introduce(&mut self, reference: &Value) -> Result<(), Error> {
        let declared = self.claim(reference)?;
        self.available.insert(text_field(declared, "id")?, declared);
        Ok(())
    }
    fn expression(&mut self, value: &Value, binder: Option<&Value>) -> Result<(), Error> {
        self.parser.expr(value, 0)?;
        self.references(value, binder, 0)
    }
    fn predicate(&mut self, value: &Value) -> Result<(), Error> {
        self.parser.predicate(value, 0)?;
        self.references(value, None, 0)
    }
    fn references(
        &mut self,
        value: &Value,
        binder: Option<&Value>,
        depth: usize,
    ) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
        self.remaining_references = self
            .remaining_references
            .checked_sub(1)
            .ok_or(Error::UnsupportedProgram)?;
        match value {
            Value::Map(fields) => {
                if text_field(value, "kind") == Ok("local") {
                    let reference = field(value, "ref")?;
                    let id = text_field(reference, "id")?;
                    let declared = binder
                        .filter(|binder| text_field(binder, "id") == Ok(id))
                        .or_else(|| self.available.get(id).copied());
                    if declared != Some(reference) {
                        return Err(Error::InvalidArtifact);
                    }
                }
                for (_, value) in fields {
                    self.references(value, binder, depth + 1)?;
                }
            }
            Value::Array(values) => {
                for value in values {
                    self.references(value, binder, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
