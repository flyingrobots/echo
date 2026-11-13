// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned typing, proof and charge audit of an authored effect
//! program's body, independent of the lowerer and the runtime.
//!
//! Proofs follow Edict: each input constraint is judged with no facts, then
//! all of them dominate the whole body; an unsigned `let` carries its proven
//! cap to later nodes, earlier conjuncts of one `all` prove later ones, and a
//! `require` proves nothing.
use super::{
    exact, get, members, prepare, sequence, string, unsigned, Charge, Check, Checker, Frame,
    Schema, Types, Value,
};
use crate::executable_operation::effect_program::GraphReads;
use crate::executable_operation::intrinsic_names::operation;
use std::collections::{BTreeMap, BTreeSet};

/// The largest atom an authored effect may read or write, in bytes.
const STORAGE_CEILING: u64 = 16 * 1024 * 1024;
/// The row the runtime reserves for each emitted mutation, in bytes, beside
/// the 64-byte unit value the effect binds.
const EMISSION_RESERVATION: u64 = 280;
/// The most mutations one runtime evaluation emits.
const EFFECT_COUNT_CEILING: usize = 1024;
/// What one root-warp atom read records beside its payload, in bytes: two
/// 128-byte observations, two 64-byte authorizations and the 64-byte value.
const READ_RECORDS: u128 = 448;
/// Recorded per root-warp range beside its bucket: the source observation,
/// the source and range authorizations and the range's observation row.
const RANGE_RECORDS: u128 = 384;
/// Recorded per scanned bucket edge on top of a plain view's 128 bytes: the
/// frame keeps a second row buffer.
const SECOND_ROW_BYTES: u128 = 128;
/// Recorded per root-warp closure node beside its atom: two observations and
/// two authorizations.
const NODE_RECORDS: u128 = 384;

/// What the verifier's relation established about one effect node.
pub(in crate::executable_operation) struct EffectSite {
    pub(in crate::executable_operation) intrinsic: String,
    pub(in crate::executable_operation) input: String,
    pub(in crate::executable_operation) output: String,
}

impl EffectSite {
    /// The graph-read operation the site calls, if it calls one.
    pub(in crate::executable_operation) fn graph_read(&self) -> Option<&'static str> {
        match operation(&self.intrinsic) {
            Some("outgoing-edge-read") => Some("outgoing-edge-read"),
            Some("closure-read") => Some("closure-read"),
            _ => None,
        }
    }
}

/// Audits the selected intent's body against the module's helpers.
pub(in crate::executable_operation) fn judge(
    core: &Value,
    exports: &Value,
    intent: &Value,
    sites: &BTreeMap<usize, EffectSite>,
    graph: Option<GraphReads<'_>>,
) -> Check<()> {
    // Core names an unsigned width directly; the runtime has no `Int` kind.
    // A nominal entry has exactly its kind, contract and representation.
    if let Some(types) = super::super::map_field(core, "types") {
        for (_, definition) in members(types)? {
            match super::super::map_field(definition, "kind") {
                Some(Value::Text(kind)) if kind == "Int" => return Err(()),
                Some(Value::Text(kind)) if kind == "Nominal" => {
                    exact(definition, &["kind", "contract", "representation"])?;
                }
                _ => {}
            }
        }
    }
    let mut types = Types::new(core);
    let (definitions, summaries) = prepare(core, exports, &mut types, 0)?;
    Checker {
        types: &mut types,
        definitions: &definitions,
        summaries: &summaries,
        proven: Vec::new(),
        branch: Vec::new(),
        path: Vec::new(),
        bound: Vec::new(),
        proven_sum: None,
        edge_scans: 0,
    }
    .effect_intent(intent, sites, graph)
}

impl<'a> Checker<'a, '_> {
    #[allow(
        clippy::too_many_lines,
        reason = "One ordered walk keeps scope, proof and charge in step with the body."
    )]
    fn effect_intent(
        &mut self,
        intent: &'a Value,
        sites: &BTreeMap<usize, EffectSite>,
        graph: Option<GraphReads<'_>>,
    ) -> Check<()> {
        self.proven.clear();
        self.branch.clear();
        self.path.clear();
        self.bound.clear();
        let body = get(intent, "body")?;
        exact(body, &["locals", "nodes", "result"])?;
        let mut frame = Frame::new(sequence(get(body, "locals")?)?, self.types)?;
        // Failure binders are spent by their own arm and never enter scope.
        let mut binders = BTreeSet::new();
        let input = *frame.declared.get("arg.0").ok_or(())?;
        if get(input, "type")? != get(intent, "input")? {
            return Err(());
        }
        let input_type = self.types.local(input)?;
        let size = input_type.measure()?;
        let mut charge = size.copy;
        charge.append(Charge {
            ticks: size.checks,
            storage: 0,
        })?;
        frame.define(input, input_type)?;
        if let Some(basis) = super::super::map_field(intent, "basis") {
            let checked = self.expression(basis, &frame, 0)?;
            // The runtime refuses any evaluation basis but exactly 32 bytes.
            if checked.schema.representation() != &(Schema::Blob { low: 32, high: 32 }) {
                return Err(());
            }
            charge.append(checked.charge)?;
        }
        // Each input constraint is judged with no facts, as the runtime does;
        // only then do they all dominate the body. One constraint never
        // proves another.
        let constraints = sequence(get(intent, "inputConstraints")?)?;
        for constraint in constraints {
            // The runtime names each constraint by its coordinate when it fails.
            exact(constraint, &["coordinate", "predicate"])?;
            string(get(constraint, "coordinate")?)?;
            charge.append(self.predicate(get(constraint, "predicate")?, &frame, 0)?.0)?;
        }
        for constraint in constraints {
            self.proven.push(get(constraint, "predicate")?);
        }
        let nodes = sequence(get(body, "nodes")?)?;
        if sites.keys().any(|index| *index >= nodes.len()) {
            return Err(());
        }
        // The largest obstruction payload a failure path encodes, and the
        // mutations the body emits.
        let mut obstruction_output = 0_u128;
        let mut mutations = 0_usize;
        for (index, node) in nodes.iter().enumerate() {
            charge.append(Charge {
                ticks: 1,
                storage: 0,
            })?;
            let kind = string(get(node, "kind")?)?;
            if (kind == "effect") != sites.contains_key(&index) {
                return Err(());
            }
            match kind {
                "let" => {
                    exact(node, &["kind", "binding", "value"])?;
                    let value = self.expression(get(node, "value")?, &frame, 0)?;
                    let local = get(node, "binding")?;
                    let expected = self.types.local(local)?;
                    if !expected.encloses(&value.schema) {
                        return Err(());
                    }
                    self.record_bound(local, get(node, "value")?, &expected)?;
                    charge.append(value.charge)?;
                    charge.append(Charge {
                        ticks: expected.measure()?.checks,
                        storage: 0,
                    })?;
                    frame.define(local, expected)?;
                }
                "require" => {
                    exact(node, &["kind", "predicate", "onFailure"])?;
                    // Refuses later work; proves nothing for it.
                    charge.append(self.predicate(get(node, "predicate")?, &frame, 0)?.0)?;
                    let failure = get(node, "onFailure")?;
                    exact(failure, &["kind", "reason"])?;
                    if string(get(failure, "kind")?)? != "terminal" {
                        return Err(());
                    }
                    let reason = get(failure, "reason")?;
                    exact(reason, &["reasonKind", "payload"])?;
                    if string(get(reason, "reasonKind")?)?.is_empty() {
                        return Err(());
                    }
                    // On failure the runtime builds this record and encodes it
                    // as the obstruction payload, under the result's output
                    // ceiling.
                    let mut fields = BTreeMap::new();
                    charge.append(Charge {
                        ticks: 1,
                        storage: 64,
                    })?;
                    for (name, value) in members(get(reason, "payload")?)? {
                        let name = string(name)?;
                        let checked = self.expression(value, &frame, 0)?;
                        charge.append(checked.charge)?;
                        charge.append(Charge {
                            ticks: 0,
                            storage: 64 + name.len() as u128,
                        })?;
                        if fields.insert(name.to_owned(), checked.schema).is_some() {
                            return Err(());
                        }
                    }
                    let payload = Schema::Fields(fields).measure()?;
                    charge.append(payload.copy)?;
                    obstruction_output = obstruction_output.max(payload.encoded);
                }
                _ => {
                    exact(
                        node,
                        &["kind", "binding", "effect", "input", "obstructionMap"],
                    )?;
                    let site = sites.get(&index).ok_or(())?;
                    let value = self.expression(get(node, "input")?, &frame, 0)?;
                    let input = self.types.resolve(&site.input, 0)?;
                    if !input.encloses(&value.schema) {
                        return Err(());
                    }
                    let output = self.types.resolve(&site.output, 0)?;
                    let cost = match site.graph_read() {
                        Some(read) => self.graph_read_charge(read, &input, &output, graph)?,
                        None => abi(&site.intrinsic, &input, &output)?,
                    };
                    if operation(&site.intrinsic) != Some("node-atom-read")
                        && site.graph_read().is_none()
                    {
                        mutations = mutations.checked_add(1).ok_or(())?;
                    }
                    let arms = members(get(node, "obstructionMap")?)?;
                    if !arms.is_empty() {
                        // A mapped read failure encodes an empty payload.
                        let payload = Schema::Fields(BTreeMap::new()).measure()?;
                        charge.append(payload.copy)?;
                        obstruction_output = obstruction_output.max(payload.encoded);
                    }
                    for (_, arm) in arms {
                        exact(arm, &["binder", "value"])?;
                        let binder = get(arm, "binder")?;
                        let id = string(get(binder, "id")?)?;
                        if frame.declared.get(id).copied() != Some(binder)
                            || frame.values.contains_key(id)
                            || !binders.insert(id)
                        {
                            return Err(());
                        }
                        let reason = get(arm, "value")?;
                        exact(reason, &["kind", "callee", "args", "typeArgs"])?;
                        if string(get(reason, "kind")?)? != "call"
                            || !sequence(get(reason, "args")?)?.is_empty()
                            || !sequence(get(reason, "typeArgs")?)?.is_empty()
                        {
                            return Err(());
                        }
                    }
                    let local = get(node, "binding")?;
                    let declared = self.types.local(local)?;
                    if declared != output {
                        return Err(());
                    }
                    charge.append(value.charge)?;
                    charge.append(Charge {
                        ticks: input.measure()?.checks,
                        storage: 0,
                    })?;
                    charge.append(cost)?;
                    frame.define(local, declared)?;
                }
            }
        }
        if frame.values.len() + binders.len() != frame.declared.len()
            || binders.iter().any(|id| frame.values.contains_key(id))
        {
            return Err(());
        }
        let output = self.expression(get(body, "result")?, &frame, 0)?;
        let expected = self.types.resolve(string(get(intent, "output")?)?, 0)?;
        if !expected.encloses(&output.schema) {
            return Err(());
        }
        let size = expected.measure()?;
        charge.append(output.charge)?;
        charge.append(size.copy)?;
        charge.append(Charge {
            ticks: size.checks,
            storage: 0,
        })?;
        let ceiling = get(intent, "coreEvaluationBudget")?;
        exact(
            ceiling,
            &["maxSteps", "maxAllocatedBytes", "maxOutputBytes"],
        )?;
        let max_output = u128::from(unsigned(get(ceiling, "maxOutputBytes")?)?);
        if charge.ticks > u128::from(unsigned(get(ceiling, "maxSteps")?)?)
            || charge.storage > u128::from(unsigned(get(ceiling, "maxAllocatedBytes")?)?)
            || size.encoded > max_output
            || obstruction_output > max_output
            || mutations > EFFECT_COUNT_CEILING
        {
            return Err(());
        }
        Ok(())
    }
}

/// Audits an effect's input and result against its intrinsic's runtime ABI
/// and returns the charge of the effect itself.
fn abi(intrinsic: &str, input: &Schema, output: &Schema) -> Check<Charge> {
    let Schema::Fields(fields) = input.representation() else {
        return Err(());
    };
    let identity = |name: &str| {
        fields.get(name).map(Schema::representation) == Some(&Schema::Blob { low: 32, high: 32 })
    };
    let atom = |name: &str| {
        matches!(
            fields.get(name).map(Schema::representation),
            Some(Schema::Blob { low: 0, high }) if *high > 0 && *high <= STORAGE_CEILING
        )
    };
    // The runtime requires the result itself to be the ABI's type, not a
    // nominal wrapper around it.
    let unit = matches!(output, Schema::Fields(fields) if fields.is_empty());
    let mutation = |names: &[&str]| -> Check<Charge> {
        if fields.len() == names.len()
            && names.iter().all(|name| {
                if *name == "atomBytes" {
                    atom(name)
                } else {
                    identity(name)
                }
            })
            && unit
        {
            Ok(Charge {
                ticks: 1,
                storage: 64 + u128::from(EMISSION_RESERVATION),
            })
        } else {
            Err(())
        }
    };
    match operation(intrinsic).ok_or(())? {
        "node-atom-read" => {
            let Schema::Blob { low: 0, high } = output else {
                return Err(());
            };
            if fields.len() != 3
                || !["nodeId", "typeId", "warpId"]
                    .iter()
                    .all(|name| identity(name))
                || *high == 0
                || *high > STORAGE_CEILING
            {
                return Err(());
            }
            Ok(Charge {
                ticks: u128::from(*high) + 1,
                // The payload is retained as read evidence and copied into
                // the value, beside the read's fixed records.
                storage: 2 * u128::from(*high) + READ_RECORDS,
            })
        }
        "ensure-node" | "create-node" => {
            mutation(&["warpId", "nodeId", "nodeType", "atomType", "atomBytes"])
        }
        "replace-atom" => mutation(&[
            "warpId",
            "nodeId",
            "expectedAtomDigest",
            "atomType",
            "atomBytes",
        ]),
        "ensure-edge" => mutation(&["warpId", "edgeId", "fromId", "toId", "edgeType"]),
        "replace-edge" => mutation(&["warpId", "edgeId", "expectedToId", "toId"]),
        _ => Err(()),
    }
}

/// Every closure position and count is a full word.
const WORD: Schema = Schema::Word(u64::MAX);
/// Warp, node, edge and type identifiers are exactly 32 bytes.
const IDENTIFIER: Schema = Schema::Blob { low: 32, high: 32 };

/// The fields of a record schema's representation.
fn fields_of(schema: &Schema) -> Check<&BTreeMap<String, Schema>> {
    match schema.representation() {
        Schema::Fields(fields) => Ok(fields),
        _ => Err(()),
    }
}

/// A sequence with an element schema and a bound within `1..=ceiling`.
fn rows(schema: &Schema, ceiling: u64) -> Check<(&Schema, u64)> {
    match schema.representation() {
        Schema::Sequence {
            element: Some(element),
            bound,
        } if (1..=ceiling).contains(bound) => Ok((element, *bound)),
        _ => Err(()),
    }
}

/// Whether `fields` has exactly `names`, each represented as `expected`.
fn each_is(fields: &BTreeMap<String, Schema>, names: &[&str], expected: &Schema) -> bool {
    fields.len() == names.len()
        && names
            .iter()
            .all(|name| fields.get(*name).map(Schema::representation) == Some(expected))
}

/// What a closure read's ABI fixes for the charge and the selector audit.
struct ClosureLayout<'s> {
    node: &'s Schema,
    guide: &'s Schema,
    node_bound: u64,
    atom_high: u64,
    occurrence_bound: u64,
    edge_type_bound: u64,
}

/// The outgoing-edge read takes `{ warpId, nodeId, edgeType }` and delivers
/// a bounded sequence of `{ edgeId, toId }`.
fn audit_edge_abi(input: &BTreeMap<String, Schema>, output: &Schema) -> Check<()> {
    let (row, _) = rows(output, 65_536)?;
    if each_is(input, &["warpId", "nodeId", "edgeType"], &IDENTIFIER)
        && each_is(fields_of(row)?, &["edgeId", "toId"], &IDENTIFIER)
    {
        Ok(())
    } else {
        Err(())
    }
}

/// The closure read takes `{ warpId, nodeId, edgeTypes, maxNodes, maxDepth,
/// guide }` and delivers `{ nodes, edges, occurrences }`, every occurrence
/// carrying exactly the guide schema the read starts with.
fn audit_closure_abi<'s>(
    input: &'s BTreeMap<String, Schema>,
    output: &'s Schema,
) -> Check<ClosureLayout<'s>> {
    let guide = input.get("guide").ok_or(())?;
    let (edge_type, edge_type_bound) = rows(input.get("edgeTypes").ok_or(())?, 16)?;
    let words = ["maxNodes", "maxDepth"]
        .iter()
        .all(|name| input.get(*name).map(Schema::representation) == Some(&WORD));
    let identifiers = ["warpId", "nodeId"]
        .iter()
        .all(|name| input.get(*name).map(Schema::representation) == Some(&IDENTIFIER));
    if input.len() != 6 || !words || !identifiers || edge_type.representation() != &IDENTIFIER {
        return Err(());
    }
    let delivered = fields_of(output)?;
    if delivered.len() != 3 {
        return Err(());
    }
    let (node, node_bound) = rows(delivered.get("nodes").ok_or(())?, 4096)?;
    let (edge, _) = rows(delivered.get("edges").ok_or(())?, 65_536)?;
    let (occurrence, occurrence_bound) = rows(delivered.get("occurrences").ok_or(())?, 65_536)?;
    let node_fields = fields_of(node)?;
    let edge_fields = fields_of(edge)?;
    let occurrence_fields = fields_of(occurrence)?;
    let atom_high = match node_fields.get("atomBytes").map(Schema::representation) {
        Some(Schema::Blob { low: 0, high }) if *high > 0 && *high <= STORAGE_CEILING => *high,
        _ => return Err(()),
    };
    let node_row = node_fields.len() == 4
        && ["nodeId", "nodeType"]
            .iter()
            .all(|name| node_fields.get(*name).map(Schema::representation) == Some(&IDENTIFIER))
        && node_fields.get("atomType").map(Schema::representation)
            == Some(&Schema::Blob { low: 0, high: 32 });
    let edge_row = edge_fields.len() == 4
        && edge_fields.get("edgeId").map(Schema::representation) == Some(&IDENTIFIER)
        && ["from", "slot", "node"]
            .iter()
            .all(|name| edge_fields.get(*name).map(Schema::representation) == Some(&WORD));
    let occurrence_row = occurrence_fields.len() == 4
        && ["node", "edge", "depth"]
            .iter()
            .all(|name| occurrence_fields.get(*name).map(Schema::representation) == Some(&WORD))
        && occurrence_fields.get("guide") == Some(guide);
    if !(node_row && edge_row && occurrence_row) {
        return Err(());
    }
    Ok(ClosureLayout {
        node,
        guide,
        node_bound,
        atom_high,
        occurrence_bound,
        edge_type_bound,
    })
}

fn repeated(charge: Charge, count: u64) -> Check<Charge> {
    Ok(Charge {
        ticks: charge.ticks.checked_mul(u128::from(count)).ok_or(())?,
        storage: charge.storage.checked_mul(u128::from(count)).ok_or(())?,
    })
}

impl Checker<'_, '_> {
    /// Audits a graph read's ABI and returns what a recording frame charges
    /// for it over root-warp storage: a plain view's charge for the declared
    /// scan allowance and the delivered value (`read_charge`), the second
    /// row buffer of each scanned edge, and every range and closure node the
    /// frame records. Each graph read pays the whole allowance, as on the
    /// bounded-read route.
    fn graph_read_charge(
        &self,
        read: &str,
        input: &Schema,
        output: &Schema,
        graph: Option<GraphReads<'_>>,
    ) -> Check<Charge> {
        let graph = graph.ok_or(())?;
        let input = fields_of(input)?;
        let scans = graph.max_edge_scans;
        let mut charge = super::read_charge::read_charge(output, self.summaries, scans)?;
        charge.append(Charge {
            ticks: 0,
            storage: u128::from(scans) * SECOND_ROW_BYTES,
        })?;
        if read == "outgoing-edge-read" {
            audit_edge_abi(input, output)?;
            charge.append(Charge {
                ticks: 1,
                storage: RANGE_RECORDS,
            })?;
            return Ok(charge);
        }
        let layout = audit_closure_abi(input, output)?;
        self.audit_selector(graph.selector, layout.node, layout.guide)?;
        let ranges = layout
            .occurrence_bound
            .checked_mul(layout.edge_type_bound)
            .ok_or(())?;
        charge.append(repeated(
            Charge {
                ticks: 2,
                storage: NODE_RECORDS + u128::from(layout.atom_high),
            },
            layout.node_bound,
        )?)?;
        charge.append(repeated(
            Charge {
                ticks: 1,
                storage: RANGE_RECORDS,
            },
            ranges,
        )?)?;
        Ok(charge)
    }

    /// The declared selector is one of the module's own source helpers from
    /// `{ guide, depth, slot, parent, child }` to `{ expand, guide,
    /// wellFormed, sufficient }`, over the closure's node and guide schemas.
    fn audit_selector(&self, name: Option<&str>, node: &Schema, guide: &Schema) -> Check<()> {
        let module = string(get(self.types.module, "coordinate")?)?;
        let definition = self
            .definitions
            .get(&format!("{module}.{}", name.ok_or(())?))
            .ok_or(())?;
        let [candidate] = definition.parameter_types.as_slice() else {
            return Err(());
        };
        let candidate = fields_of(candidate)?;
        let decision = fields_of(&definition.result)?;
        let conforming = definition.source
            && candidate.len() == 5
            && ["depth", "slot"]
                .iter()
                .all(|field| candidate.get(*field).map(Schema::representation) == Some(&WORD))
            && candidate.get("guide") == Some(guide)
            && candidate.get("parent") == Some(node)
            && candidate.get("child") == Some(node)
            && decision.len() == 4
            && ["expand", "wellFormed", "sufficient"].iter().all(|field| {
                decision.get(*field).map(Schema::representation) == Some(&Schema::Truth)
            })
            && decision.get("guide") == Some(guide);
        if conforming {
            Ok(())
        } else {
            Err(())
        }
    }
}
