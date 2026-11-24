// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned typing, proof and cost judgment of an authored effect
//! program's body, independent of the runtime's own judgment.
//!
//! Proofs follow Edict: each input constraint is judged with no facts, then
//! all of them dominate the whole body; an unsigned `let` carries its proven
//! cap to later nodes, earlier conjuncts of one `all` prove later ones, and a
//! `require` proves nothing.
use super::effect_block::{claim, Walk};
use super::{
    exact, get, includes, judged, list, members, number, shape, string, Check, Cost, Judgment, Ty,
    Value,
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
const EFFECT_COUNT_CEILING: u64 = 1024;
/// What one root-warp atom read records beside its payload, in bytes: two
/// 128-byte observations, two 64-byte authorizations and the 64-byte value.
const READ_RECORDS: u64 = 448;

/// What a recording frame retains for one root-warp range beside its bucket:
/// the source observation, the source and range authorizations, and the
/// range's own observation row.
const RANGE_RECORDS: u64 = 384;
/// What a recording frame retains for each scanned bucket edge beyond the
/// 128 bytes a plain view reserves: the second recorded row buffer.
const RECORDED_EDGE_BYTES: u64 = 128;
/// What a recording frame retains for one root-warp closure node beside its
/// atom: two observations and two authorizations.
const NODE_RECORDS: u64 = 384;

/// What the lowering relation established about one effect node.
pub(in crate::executable_operation) struct EffectSite {
    pub(in crate::executable_operation) intrinsic: String,
    pub(in crate::executable_operation) input: String,
    pub(in crate::executable_operation) output: String,
}

impl EffectSite {
    /// The graph-read operation this site calls, if it calls one.
    pub(in crate::executable_operation) fn graph_read(&self) -> Option<&'static str> {
        match operation(&self.intrinsic)? {
            "outgoing-edge-read" => Some("outgoing-edge-read"),
            "closure-read" => Some("closure-read"),
            _ => None,
        }
    }
}

/// Judges the selected intent's body with the module's helpers.
pub(in crate::executable_operation) fn judge(
    core: &Value,
    exports: &Value,
    intent: &Value,
    sites: &BTreeMap<usize, EffectSite>,
    graph: Option<GraphReads<'_>>,
    identity: bool,
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
    judged(core, exports, 0, identity)?.effect_intent(intent, sites, graph)
}

impl<'a> Judgment<'a> {
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
        self.effect_start();
        let body = get(intent, "body")?;
        let mut pending = self.block_locals(body)?;
        let mut walk = Walk {
            sites,
            index: 0,
            claimed: BTreeSet::new(),
            graph,
        };
        let input = *pending.get("arg.0").ok_or(())?;
        if get(input, "type")? != get(intent, "input")? {
            return Err(());
        }
        claim(input, &mut pending, &mut walk)?;
        let mut scope = BTreeMap::new();
        let ty = self.declaration(input, &mut scope)?;
        let mut cost = self.boundary(&ty)?;
        if let Some(basis) = super::super::map_field(intent, "basis") {
            let fact = self.expression(basis, &scope, true, 0)?;
            // The runtime refuses any evaluation basis but exactly 32 bytes.
            if fact.ty.representation() != &Ty::Bytes(32, 32) {
                return Err(());
            }
            cost = cost.plus(self.effect_value(basis, fact.cost)?)?;
        }
        // Each input constraint is judged with no facts, as the runtime does;
        // only then do they all dominate the body. One constraint never
        // proves another.
        let constraints = list(get(intent, "inputConstraints")?)?;
        for constraint in constraints {
            // The runtime names each constraint by its coordinate when it fails.
            // Exactly the compiler's Core shape: an authored `where` conjunct or
            // a compiler-derived constraint.
            exact(constraint, &["coordinate", "source", "predicate"])?;
            if !matches!(string(get(constraint, "source")?)?, "where" | "compiler") {
                return Err(());
            }
            string(get(constraint, "coordinate")?)?;
            let predicate = get(constraint, "predicate")?;
            let copying = self.predicate(predicate, &scope, true, 0)?.0;
            cost = cost.plus(self.effect_condition(predicate, copying)?)?;
        }
        for constraint in constraints {
            self.proven.push(get(constraint, "predicate")?);
        }
        let totals = self.effect_block(body, &mut scope, pending, &mut walk)?;
        // Every effect site the relation established is an instruction of
        // this body.
        if sites.keys().any(|index| *index >= walk.index) {
            return Err(());
        }
        cost = cost.plus(totals.cost)?;
        let result_value = get(body, "result")?;
        let result = self.expression(result_value, &scope, true, 0)?;
        let result_cost = self.effect_value(result_value, result.cost)?;
        let expected = self.ty(string(get(intent, "output")?)?, 0)?;
        if !includes(&expected, &result.ty) {
            return Err(());
        }
        let output = shape(&expected)?.2;
        cost = cost.plus(result_cost)?.plus(self.boundary(&expected)?)?;
        let budget = get(intent, "coreEvaluationBudget")?;
        exact(budget, &["maxSteps", "maxAllocatedBytes", "maxOutputBytes"])?;
        let max_output = number(get(budget, "maxOutputBytes")?)?;
        if cost.steps > number(get(budget, "maxSteps")?)?
            || cost.bytes > number(get(budget, "maxAllocatedBytes")?)?
            || output > max_output
            || totals.obstruction_output > max_output
            || totals.mutations > EFFECT_COUNT_CEILING
        {
            return Err(());
        }
        Ok(())
    }
}
/// Checks an effect's input and result against its intrinsic's runtime ABI
/// and returns the work the effect itself costs.
pub(super) fn abi(intrinsic: &str, input: &Ty, output: &Ty) -> Check<Cost> {
    let Ty::Record(fields) = input.representation() else {
        return Err(());
    };
    let id = |name: &str| fields.get(name).map(Ty::representation) == Some(&Ty::Bytes(32, 32));
    let atom = |name: &str| {
        matches!(
            fields.get(name).map(Ty::representation),
            Some(Ty::Bytes(0, max)) if *max > 0 && *max <= STORAGE_CEILING
        )
    };
    // The runtime requires the result itself to be the ABI's type, not a
    // nominal wrapper around it.
    let unit = matches!(output, Ty::Record(fields) if fields.is_empty());
    let mutation = |names: &[&str]| -> Check<Cost> {
        let conforming = fields.len() == names.len()
            && names.iter().all(|name| {
                if *name == "atomBytes" {
                    atom(name)
                } else {
                    id(name)
                }
            });
        if conforming && unit {
            Ok(Cost {
                steps: 1,
                bytes: 64 + EMISSION_RESERVATION,
            })
        } else {
            Err(())
        }
    };
    match operation(intrinsic).ok_or(())? {
        "node-atom-read" => {
            let Ty::Bytes(0, max) = output else {
                return Err(());
            };
            if fields.len() != 3
                || !["nodeId", "typeId", "warpId"].iter().all(|name| id(name))
                || *max == 0
                || *max > STORAGE_CEILING
            {
                return Err(());
            }
            Ok(Cost {
                steps: max.checked_add(1).ok_or(())?,
                // The payload is retained as read evidence and copied into
                // the value, beside the read's fixed records.
                bytes: max
                    .checked_mul(2)
                    .and_then(|bytes| bytes.checked_add(READ_RECORDS))
                    .ok_or(())?,
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

/// The word every closure position and count is.
const WORD: Ty = Ty::Word(u64::MAX);
/// The 32-byte identifiers of warps, nodes, edges and types.
const ID: Ty = Ty::Bytes(32, 32);

/// The capacities and row types a closure read's ABI fixes.
struct ClosureAbi<'t> {
    node: &'t Ty,
    guide: &'t Ty,
    nodes: u64,
    atom: u64,
    occurrences: u64,
    edge_types: u64,
}

/// A record of exactly `names`, each of whose representations is `expected`.
fn all_are(fields: &BTreeMap<String, Ty>, names: &[&str], expected: &Ty) -> bool {
    fields.len() == names.len()
        && names
            .iter()
            .all(|name| fields.get(*name).map(Ty::representation) == Some(expected))
}

/// A bounded list of `item`s whose capacity is within `1..=ceiling`.
fn bounded_list(ty: &Ty, ceiling: u64) -> Check<(&Ty, u64)> {
    match ty.representation() {
        Ty::List(Some(item), max) if (1..=ceiling).contains(max) => Ok((item, *max)),
        _ => Err(()),
    }
}

fn record(ty: &Ty) -> Check<&BTreeMap<String, Ty>> {
    match ty.representation() {
        Ty::Record(fields) => Ok(fields),
        _ => Err(()),
    }
}

/// `outgoing-edge-read` takes `{ warpId, nodeId, edgeType }` and returns a
/// bounded list of `{ edgeId, toId }`, as on the bounded-read route.
fn edge_abi(input: &BTreeMap<String, Ty>, output: &Ty) -> Check<()> {
    let (row, _) = bounded_list(output, 65_536)?;
    if all_are(input, &["warpId", "nodeId", "edgeType"], &ID)
        && all_are(record(row)?, &["edgeId", "toId"], &ID)
    {
        Ok(())
    } else {
        Err(())
    }
}

/// `closure-read` takes `{ warpId, nodeId, edgeTypes, maxNodes, maxDepth,
/// guide }` and returns `{ nodes, edges, occurrences }`; every occurrence
/// carries exactly the guide type the read is started with.
fn closure_abi<'t>(input: &'t BTreeMap<String, Ty>, output: &'t Ty) -> Check<ClosureAbi<'t>> {
    let field = |fields: &'t BTreeMap<String, Ty>, name: &str| fields.get(name).ok_or(());
    let (edge_type, edge_types) = bounded_list(field(input, "edgeTypes")?, 16)?;
    let guide = field(input, "guide")?;
    if input.len() != 6
        || edge_type.representation() != &ID
        || ["warpId", "nodeId"]
            .iter()
            .any(|name| input.get(*name).map(Ty::representation) != Some(&ID))
        || ["maxNodes", "maxDepth"]
            .iter()
            .any(|name| input.get(*name).map(Ty::representation) != Some(&WORD))
    {
        return Err(());
    }
    let result = record(output)?;
    if result.len() != 3 {
        return Err(());
    }
    let (node, nodes) = bounded_list(field(result, "nodes")?, 4096)?;
    let row = record(node)?;
    let atom = match field(row, "atomBytes")?.representation() {
        Ty::Bytes(0, max) if *max > 0 && *max <= STORAGE_CEILING => *max,
        _ => return Err(()),
    };
    let (edge, _) = bounded_list(field(result, "edges")?, 65_536)?;
    let edge = record(edge)?;
    let (occurrence, occurrences) = bounded_list(field(result, "occurrences")?, 65_536)?;
    let occurrence = record(occurrence)?;
    let conforming = row.len() == 4
        && ["nodeId", "nodeType"]
            .iter()
            .all(|name| row.get(*name).map(Ty::representation) == Some(&ID))
        && row.get("atomType").map(Ty::representation) == Some(&Ty::Bytes(0, 32))
        && edge.len() == 4
        && edge.get("edgeId").map(Ty::representation) == Some(&ID)
        && ["from", "slot", "node"]
            .iter()
            .all(|name| edge.get(*name).map(Ty::representation) == Some(&WORD))
        && occurrence.len() == 4
        && ["node", "edge", "depth"]
            .iter()
            .all(|name| occurrence.get(*name).map(Ty::representation) == Some(&WORD))
        && occurrence.get("guide") == Some(guide);
    if !conforming {
        return Err(());
    }
    Ok(ClosureAbi {
        node,
        guide,
        nodes,
        atom,
        occurrences,
        edge_types,
    })
}

fn times(cost: Cost, count: u64) -> Check<Cost> {
    Ok(Cost {
        steps: cost.steps.checked_mul(count).ok_or(())?,
        bytes: cost.bytes.checked_mul(count).ok_or(())?,
    })
}

impl Judgment<'_> {
    /// The charge for converting the intent's input or output at its
    /// boundary. The copying schedule copies and validates the value; the
    /// shared-read schedule charges two steps per cell and the value's bytes,
    /// the rule its pure programs use (`shared_intent_cost`).
    fn boundary(&self, ty: &Ty) -> Check<Cost> {
        let (copy, validations, _, _) = shape(ty)?;
        // A copy charges one step per cell, so the shared rule adds one more.
        let steps = if self.shared { copy.steps } else { validations };
        copy.plus(Cost { steps, bytes: 0 })
    }
}

impl Judgment<'_> {
    /// Checks a graph read's ABI and returns the work a recording frame
    /// charges for it on root-warp storage: the plain view's charge for the
    /// declared scan allowance and the result (`read_cost`), plus what the
    /// frame records — every range and closure node it retains and the second
    /// row buffer of every scanned edge. The allowance is charged to every
    /// graph read, as on the bounded-read route.
    pub(super) fn graph_read(
        &mut self,
        read: &str,
        input: &Ty,
        output: &Ty,
        graph: Option<GraphReads<'_>>,
    ) -> Check<Cost> {
        let graph = graph.ok_or(())?;
        if read == "closure-read" {
            let closure = closure_abi(record(input)?, output)?;
            self.selector(graph.selector, closure.node, closure.guide)?;
        }
        // Under the shared-read schedule a closure charges its declared
        // selector, the only helper the runtime calls, at its shared-read
        // call cost: the runtime evaluates it with the shared evaluator. The
        // copying schedule keeps the costliest helper of the module at its
        // copying cost.
        let selected: BTreeMap<String, super::Fact>;
        let helpers = if self.shared {
            let coordinate = string(get(self.core, "coordinate")?)?;
            let declared = graph.selector.map(|name| format!("{coordinate}.{name}"));
            let mut chosen = BTreeMap::new();
            if let Some(declared) = declared {
                if let Some(fact) = self.completed.get(&declared).cloned() {
                    let cost = self.shared_selector(&declared)?;
                    chosen.insert(declared, super::Fact { cost, ..fact });
                }
            }
            selected = chosen;
            &selected
        } else {
            &self.completed
        };
        graph_charge(read, input, output, graph.max_edge_scans, helpers)
    }

    /// The declared selector is one of the module's own source helpers,
    /// `{ guide, depth, slot, parent, child } -> { expand, guide, wellFormed,
    /// sufficient }`, over the closure's own node and guide types.
    fn selector(&mut self, name: Option<&str>, node: &Ty, guide: &Ty) -> Check<()> {
        let coordinate = string(get(self.core, "coordinate")?)?;
        let qualified = format!("{coordinate}.{}", name.ok_or(())?);
        let function = *self.functions.get(&qualified).ok_or(())?;
        let [parameter] = function.params else {
            return Err(());
        };
        if !function.source {
            return Err(());
        }
        let candidate = self.ty(string(get(parameter, "type")?)?, 0)?;
        let decision = self.ty(function.result, 0)?;
        let candidate = record(&candidate)?;
        let decision = record(&decision)?;
        let conforming = candidate.len() == 5
            && ["depth", "slot"]
                .iter()
                .all(|name| candidate.get(*name).map(Ty::representation) == Some(&WORD))
            && candidate.get("guide") == Some(guide)
            && candidate.get("parent") == Some(node)
            && candidate.get("child") == Some(node)
            && decision.len() == 4
            && ["expand", "wellFormed", "sufficient"]
                .iter()
                .all(|name| decision.get(*name).map(Ty::representation) == Some(&Ty::Bool))
            && decision.get("guide") == Some(guide);
        if conforming {
            Ok(())
        } else {
            Err(())
        }
    }
}

/// The work one graph read costs once its ABI and selector are judged: its
/// declared scan allowance and result (`read_cost`), plus what the frame
/// records. `helpers` are the source helpers the read may call.
pub(super) fn graph_charge(
    read: &str,
    input: &Ty,
    output: &Ty,
    scans: u64,
    helpers: &BTreeMap<String, super::Fact>,
) -> Check<Cost> {
    let fields = record(input)?;
    let records = if read == "outgoing-edge-read" {
        edge_abi(fields, output)?;
        Cost {
            steps: 1,
            bytes: RANGE_RECORDS,
        }
    } else {
        let closure = closure_abi(fields, output)?;
        let ranges = closure
            .occurrences
            .checked_mul(closure.edge_types)
            .ok_or(())?;
        let node = Cost {
            steps: 2,
            bytes: NODE_RECORDS.checked_add(closure.atom).ok_or(())?,
        };
        let range = Cost {
            steps: 1,
            bytes: RANGE_RECORDS,
        };
        times(node, closure.nodes)?.plus(times(range, ranges)?)?
    };
    super::read_cost::read_cost(output, helpers, scans)?
        .plus(records)?
        .plus(Cost {
            steps: 0,
            bytes: scans.checked_mul(RECORDED_EDGE_BYTES).ok_or(())?,
        })
}
