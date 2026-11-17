// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Ordered or structured Core/Target correspondence and exact lexical local
//! identities, with nested loop and branch scopes.
use crate::edict_pure::{
    functions,
    model::{Expr, Helper, Predicate, RuntimeType, MAX_DEPTH, MAX_PROGRAM_NODES},
    syntax::Parser,
    values::{array, exact_fields, field, map, number, require_text, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
use std::collections::{BTreeMap, BTreeSet};

/// Checks one authored effect intent against its Target form: the ordered
/// Core/Target correspondence, the lexical local identities and the range
/// proofs of its expressions. Each input constraint is checked with no facts.
/// All of them then hold for the whole body, and an unsigned `let` carries
/// its proven ceiling to later nodes. A `require` adds no fact.
pub(super) fn validate<'a>(
    module: &'a Value,
    intent: &'a Value,
    target: &Value,
    exports: &Value,
    signatures: &BTreeMap<&str, &Value>,
) -> Result<(), Error> {
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
        exports,
        array(field(body, "locals")?)?,
        text_field(intent, "input")?,
    )?;
    scope.parser.ty(text_field(intent, "output")?, 0)?;
    if let Ok(basis) = field(intent, "basis") {
        scope.expression(basis, None)?;
        scope.data_type(basis)?;
    }
    // As in Edict, each input constraint is checked without facts, and all of
    // them then dominate the whole body.
    let constraints = array(field(intent, "inputConstraints")?)?;
    for constraint in constraints {
        scope.predicate(field(constraint, "predicate")?)?;
    }
    for constraint in constraints {
        scope.facts.push(field(constraint, "predicate")?);
    }
    // The order tree corresponds to Core node for node; this pairs each Core
    // instruction with its Target instruction by identity.
    let lowered = super::control_order::instructions(intent, target)?
        .into_iter()
        .map(|(node, target)| (std::ptr::from_ref(node), target))
        .collect::<BTreeMap<_, _>>();
    validate_block(body, &mut scope, &lowered, signatures, 0)?;
    scope.typed_expression(field(body, "result")?, text_field(intent, "output")?)?;
    functions::check_roots(
        &scope.helpers,
        scope.expressions.iter(),
        scope.predicates.iter(),
    )?;
    Ok(())
}
/// Each Core instruction's Target instruction, keyed by the node's address.
type Lowered<'a> = BTreeMap<*const Value, &'a Value>;
/// Checks one block's nodes in order. A loop body or branch arm is checked
/// in a child scope: its locals are claimed from its own table and are not
/// available after the block.
fn validate_block<'a>(
    body: &'a Value,
    scope: &mut Scope<'a>,
    lowered: &Lowered<'_>,
    signatures: &BTreeMap<&str, &Value>,
    depth: usize,
) -> Result<(), Error> {
    if depth > MAX_DEPTH {
        return Err(Error::UnsupportedProgram);
    }
    for node in array(field(body, "nodes")?)? {
        scope.parser.enter(depth)?;
        match text_field(node, "kind")? {
            "for" => validate_loop(node, scope, lowered, signatures, depth)?,
            "branch" => {
                // A branch predicate is not a fact for either arm; a missing
                // fact only refuses more programs.
                scope.predicate(field(node, "predicate")?)?;
                for arm in ["then", "else"] {
                    validate_child(field(node, arm)?, None, scope, lowered, signatures, depth)?;
                }
            }
            _ => {
                let target = *lowered
                    .get(&std::ptr::from_ref(node))
                    .ok_or(Error::InvalidArtifact)?;
                validate_instruction(node, target, scope, signatures)?;
            }
        }
    }
    if scope.inventory.keys().any(|id| !scope.used.contains(id)) {
        return Err(Error::InvalidArtifact);
    }
    Ok(())
}
/// A bounded loop: a literal bound that covers the iterable's list type, and
/// a binder that accepts its items, fresh in the body's own scope.
fn validate_loop<'a>(
    node: &'a Value,
    scope: &mut Scope<'a>,
    lowered: &Lowered<'_>,
    signatures: &BTreeMap<&str, &Value>,
    depth: usize,
) -> Result<(), Error> {
    let iterable = field(node, "iter")?;
    scope.expression(iterable, None)?;
    let RuntimeType::List { item, max } = scope.data_type(iterable)?.representation().clone()
    else {
        return Err(Error::InvalidArtifact);
    };
    let bound = field(node, "bound")?;
    exact_fields(bound, &["kind", "value"])?;
    require_text(bound, "kind", "literal")?;
    if max > number(field(bound, "value")?)? {
        return Err(Error::InvalidArtifact);
    }
    let binder = field(node, "binder")?;
    let expected = scope.parser.ty(text_field(binder, "type")?, 0)?;
    if item.is_some_and(|item| !functions::accepts(&expected, &item)) {
        return Err(Error::InvalidArtifact);
    }
    validate_child(
        field(node, "body")?,
        Some(binder),
        scope,
        lowered,
        signatures,
        depth,
    )
}
/// Checks a nested statement block in a child scope and restores the
/// enclosing scope afterwards, so no local leaks out of the block.
fn validate_child<'a>(
    body: &'a Value,
    binder: Option<&'a Value>,
    scope: &mut Scope<'a>,
    lowered: &Lowered<'_>,
    signatures: &BTreeMap<&str, &Value>,
    depth: usize,
) -> Result<(), Error> {
    let mut inventory = BTreeMap::new();
    for local in array(field(body, "locals")?)? {
        scope.parser.enter(depth)?;
        functions::local(local)?;
        scope.parser.ty(text_field(local, "type")?, 0)?;
        let id = text_field(local, "id")?;
        // Local identities are unique across the whole intent.
        if inventory.insert(id, local).is_some() || scope.used.contains(id) {
            return Err(Error::InvalidArtifact);
        }
    }
    let parent_inventory = std::mem::replace(&mut scope.inventory, inventory);
    let parent_available = scope.available.clone();
    if let Some(binder) = binder {
        scope.introduce(binder)?;
    }
    validate_block(body, scope, lowered, signatures, depth + 1)?;
    scope.inventory = parent_inventory;
    scope.available = parent_available;
    Ok(())
}
fn validate_instruction<'a>(
    node: &'a Value,
    target: &Value,
    scope: &mut Scope<'a>,
    signatures: &BTreeMap<&str, &Value>,
) -> Result<(), Error> {
    {
        let kind = text_field(node, "kind")?;
        match kind {
            "let" => {
                exact_fields(node, &["kind", "binding", "value"])?;
                exact_fields(target, &["id", "binding", "value"])?;
                equal(node, target, "binding")?;
                equal(node, target, "value")?;
                scope.typed_expression(
                    field(node, "value")?,
                    text_field(field(node, "binding")?, "type")?,
                )?;
                scope
                    .lets
                    .push((field(node, "binding")?, field(node, "value")?));
                scope.introduce(field(node, "binding")?)?;
            }
            // Deliberately pushed to neither `facts` nor `caps`: a require ends
            // evaluation on failure, but it is not a dominating fact for the
            // nodes after it (Edict totality).
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
                    scope.data_type(value)?;
                }
            }
            "effect" => effect(
                node,
                target,
                scope,
                signatures
                    .get(text_field(target, "id")?)
                    .copied()
                    .ok_or(Error::InvalidArtifact)?,
            )?,
            _ => return Err(Error::UnsupportedProgram),
        }
    }
    Ok(())
}
fn equal(core: &Value, target: &Value, key: &str) -> Result<(), Error> {
    if field(core, key)? != field(target, key)? {
        return Err(Error::InvalidArtifact);
    }
    Ok(())
}
fn effect(
    core: &Value,
    target: &Value,
    scope: &mut Scope<'_>,
    signature: &Value,
) -> Result<(), Error> {
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
    scope.typed_expression(field(core, "input")?, text_field(signature, "inputType")?)?;
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
    definitions: BTreeMap<String, functions::Definition<'a>>,
    helpers: BTreeMap<String, Helper>,
    /// Input constraints, which dominate the whole body.
    facts: Vec<&'a Value>,
    /// Earlier `let` bindings and their values, in body order. Their
    /// ceilings are carried into `caps` only when a judgment needs them.
    lets: Vec<(&'a Value, &'a Value)>,
    /// How many of `lets` have been carried into `caps`.
    carried: usize,
    /// Proven ceilings of earlier unsigned bindings.
    caps: functions::Caps<'a>,
    expressions: Vec<Expr>,
    predicates: Vec<Predicate>,
}
impl<'a> Scope<'a> {
    fn new(
        module: &'a Value,
        exports: &'a Value,
        locals: &'a [Value],
        input_type: &str,
    ) -> Result<Self, Error> {
        let mut parser = Parser {
            types: field(module, "types")?,
            coordinate: text_field(module, "coordinate")?,
            remaining: MAX_PROGRAM_NODES,
        };
        let helpers = functions::decode(module, exports, &mut parser)?;
        let definitions = functions::definitions(module, exports, parser.coordinate)?;
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
            definitions,
            helpers,
            expressions: Vec::new(),
            predicates: Vec::new(),
            facts: Vec::new(),
            lets: Vec::new(),
            carried: 0,
            caps: functions::Caps::new(),
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
    /// Infers the runtime type of `value` and records its parsed expression.
    ///
    /// The judgment first runs with no facts and no carried ceilings. Only
    /// when that refuses does it carry the earlier ceilings and run again
    /// with the input constraints, so a node that needs no fact spends no
    /// proof budget on facts. Only when that also refuses does a last run
    /// read the predicates of enclosing `then` arms, so a node that needs no
    /// branch fact spends nothing on them.
    fn data_type(&mut self, value: &Value) -> Result<RuntimeType, Error> {
        let actual = self.judged(|scope, proof| {
            functions::infer(
                value,
                &scope.available,
                proof,
                &scope.definitions,
                true,
                &mut scope.parser,
                0,
            )
        })?;
        self.expressions.push(self.parser.expr(value, 0)?);
        Ok(actual)
    }
    /// Runs `judge` without facts, then, after a refusal, with the carried
    /// ceilings and input constraints, then, after another refusal, with the
    /// branch facts as well. An earlier attempt never reads what a later one
    /// adds.
    fn judged<T>(
        &mut self,
        judge: impl Fn(&mut Self, functions::Proof<'_, '_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let none = functions::Caps::new();
        let facts = match judge(self, functions::Proof::new(&none, &[])) {
            Err(Error::UnsupportedProgram) if !self.facts.is_empty() || !self.lets.is_empty() => {
                self.carry_ceilings()?;
                let (caps, facts) = (self.caps.clone(), self.facts.clone());
                match judge(self, functions::Proof::new(&caps, &facts)) {
                    Err(Error::UnsupportedProgram) => Some((caps, facts)),
                    judged => return judged,
                }
            }
            Err(Error::UnsupportedProgram) => None,
            judged => return judged,
        };
        let (caps, facts) = facts.unwrap_or_default();
        judge(self, functions::Proof::with_branch_facts(&caps, &facts))
    }
    /// Carries the ceilings of the `let` bindings not yet carried, in body
    /// order. An unsigned binding whose value is proven below its width
    /// carries that ceiling to later nodes. Local identities are unique and
    /// immutable, so the cap names exactly the bound value.
    fn carry_ceilings(&mut self) -> Result<(), Error> {
        while let Some(&(binding, value)) = self.lets.get(self.carried) {
            self.carried += 1;
            let RuntimeType::Unsigned(max) = self.parser.ty(text_field(binding, "type")?, 0)?
            else {
                continue;
            };
            let cap = functions::unsigned_ceiling(
                value,
                max,
                &self.caps,
                &self.facts,
                &mut self.parser,
                0,
            )?;
            if cap < max {
                self.caps.insert(text_field(binding, "id")?, cap);
            }
        }
        Ok(())
    }
    fn typed_expression(&mut self, value: &Value, expected: &str) -> Result<(), Error> {
        self.expression(value, None)?;
        let actual = self.data_type(value)?;
        let expected = self.parser.ty(expected, 0)?;
        if !functions::accepts(&expected, &actual) {
            return Err(Error::InvalidArtifact);
        }
        Ok(())
    }
    /// Checks `value` as a predicate, records its parsed form and claims the
    /// references it reads. As in `data_type`, the facts and carried ceilings
    /// are used only when the judgment without them refuses.
    fn predicate(&mut self, value: &Value) -> Result<(), Error> {
        self.judged(|scope, proof| {
            functions::check_predicate(
                value,
                &scope.available,
                proof,
                &scope.definitions,
                true,
                &mut scope.parser,
                0,
            )
        })?;
        self.predicates.push(self.parser.predicate(value, 0)?);
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
