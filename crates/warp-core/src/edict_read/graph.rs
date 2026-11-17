// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Typed outgoing-edge reads and the guided bounded closure read.
//!
//! Both primitives are generic graph observations. The closure read knows no
//! application structure: an authored pure selector decides every expansion,
//! and the runtime only enforces the declared bounds, deduplicates material by
//! node identity and preserves each occurrence's edge path and guide.
use super::{
    access::{Material, Outgoing},
    model::GraphReads,
    GraphReadObstruction as Obstruction, NodeAtomAccess, ReadError, ReadLimits,
};
use crate::edict_pure::{
    evaluate::{expression, validate, Meter, VALUE_CELL_BYTES},
    model::{Expr, Helper, RuntimeType},
    values::{array, bytes, exact_fields, field, number},
    EvaluationError as Error,
};
use crate::{NodeId, NodeKey, OutgoingEdgeRange, TypeId, WarpId};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

pub(super) const EDGE_READ: &str = "echo.dpo@1.outgoing-edge-read";
pub(super) const CLOSURE_READ: &str = "echo.dpo@1.closure-read";

/// Every failure an outgoing-edge read step must map to an authored arm.
pub(super) const EDGE_FAILURES: [Obstruction; 3] = [
    Obstruction::EdgeSourceMissing,
    Obstruction::EdgeBoundExceeded,
    Obstruction::EdgeScanBoundExceeded,
];
/// Every failure a closure read step must map to an authored arm.
pub(super) const CLOSURE_FAILURES: [Obstruction; 7] = [
    Obstruction::ClosureStartMissing,
    Obstruction::ClosureNodeBoundExceeded,
    Obstruction::ClosureDepthBoundExceeded,
    Obstruction::ClosureScanBoundExceeded,
    Obstruction::ClosureWorkBoundExceeded,
    Obstruction::ClosureSelectorInsufficient,
    Obstruction::ClosureStorageMalformed,
];

const MAX_EDGE_TYPES: usize = 16;
const MAX_RESULT_EDGES: u64 = 65_536;
const MAX_RESULT_NODES: u64 = 4096;
const MAX_NODE_BYTES: u64 = 67_108_864;

const ID: RuntimeType = RuntimeType::Bytes { min: 32, max: 32 };
const WORD: RuntimeType = RuntimeType::Unsigned(u64::MAX);

/// Running totals shared by every read step of one evaluation.
#[derive(Default)]
pub(crate) struct Usage {
    pub reads: u64,
    pub read_bytes: u64,
    pub edge_scans: u64,
}

impl Usage {
    pub(super) fn attempt(&mut self, limits: &ReadLimits) -> Result<(), ReadError> {
        self.reads = self
            .reads
            .checked_add(1)
            .ok_or(ReadError::ReadCountExceeded)?;
        if self.reads > limits.max_reads {
            return Err(ReadError::ReadCountExceeded);
        }
        Ok(())
    }

    pub(super) fn deliver(&mut self, count: u64, limits: &ReadLimits) -> Result<(), ReadError> {
        self.read_bytes = self
            .read_bytes
            .checked_add(count)
            .ok_or(ReadError::ReadBytesExceeded)?;
        if self.read_bytes > limits.max_read_bytes {
            return Err(ReadError::ReadBytesExceeded);
        }
        Ok(())
    }
}

/// Declared result bounds and the two types the selector must share.
pub(crate) struct ClosureShape {
    node_bytes: u64,
    max_nodes: u64,
    max_edges: u64,
    max_occurrences: u64,
    node: RuntimeType,
    guide: RuntimeType,
}

impl ClosureShape {
    /// The guide type every occurrence carries, which the read is started with.
    pub(crate) const fn guide(&self) -> &RuntimeType {
        &self.guide
    }
}

fn fields<'a, const N: usize>(
    ty: &'a RuntimeType,
    names: [&str; N],
) -> Result<[&'a RuntimeType; N], Error> {
    let RuntimeType::Record(declared) = ty.representation() else {
        return Err(Error::UnsupportedProgram);
    };
    if declared.len() != N {
        return Err(Error::UnsupportedProgram);
    }
    let mut found = [ty; N];
    for (slot, name) in found.iter_mut().zip(names) {
        *slot = declared
            .iter()
            .find_map(|(field, ty)| (field == name).then_some(ty))
            .ok_or(Error::UnsupportedProgram)?;
    }
    Ok(found)
}

fn list(ty: &RuntimeType, ceiling: u64) -> Result<(&RuntimeType, u64), Error> {
    let RuntimeType::List {
        item: Some(item),
        max,
    } = ty.representation()
    else {
        return Err(Error::UnsupportedProgram);
    };
    if *max == 0 || *max > ceiling {
        return Err(Error::UnsupportedProgram);
    }
    Ok((item, *max))
}

fn exactly(ty: &RuntimeType, expected: &RuntimeType) -> Result<(), Error> {
    if ty.representation() == expected {
        Ok(())
    } else {
        Err(Error::UnsupportedProgram)
    }
}

/// The edge-read result: a nonempty-capacity list of `{ edgeId, toId }`.
pub(crate) fn edge_shape(ty: &RuntimeType) -> Result<u64, Error> {
    let (row, max_edges) = list(ty, MAX_RESULT_EDGES)?;
    for field in fields(row, ["edgeId", "toId"])? {
        exactly(field, &ID)?;
    }
    Ok(max_edges)
}

fn node_shape(ty: &RuntimeType) -> Result<u64, Error> {
    let [id, node_type, atom_type, atom] =
        fields(ty, ["nodeId", "nodeType", "atomType", "atomBytes"])?;
    exactly(id, &ID)?;
    exactly(node_type, &ID)?;
    exactly(atom_type, &RuntimeType::Bytes { min: 0, max: 32 })?;
    match atom.representation() {
        RuntimeType::Bytes { min: 0, max } if *max > 0 && *max <= MAX_NODE_BYTES => Ok(*max),
        _ => Err(Error::UnsupportedProgram),
    }
}

/// The closure result: `{ nodes, edges, occurrences }` with fixed row shapes.
pub(crate) fn closure_shape(ty: &RuntimeType) -> Result<ClosureShape, Error> {
    let [nodes, edges, occurrences] = fields(ty, ["nodes", "edges", "occurrences"])?;
    let (node, max_nodes) = list(nodes, MAX_RESULT_NODES)?;
    let node_bytes = node_shape(node)?;
    let (edge, max_edges) = list(edges, MAX_RESULT_EDGES)?;
    let [from, slot, edge_id, target] = fields(edge, ["from", "slot", "edgeId", "node"])?;
    exactly(edge_id, &ID)?;
    let (occurrence, max_occurrences) = list(occurrences, MAX_RESULT_EDGES)?;
    let [index, via, depth, guide] = fields(occurrence, ["node", "edge", "depth", "guide"])?;
    for word in [from, slot, target, index, via, depth] {
        exactly(word, &WORD)?;
    }
    Ok(ClosureShape {
        node_bytes,
        max_nodes,
        max_edges,
        max_occurrences,
        node: node.clone(),
        guide: guide.clone(),
    })
}

/// A selector is one source-owned helper `Candidate -> Decision` over the
/// exact node and guide types of the closure result it guides.
pub(crate) fn selector_conforms(helper: &Helper, shape: &ClosureShape) -> Result<(), Error> {
    let [parameter] = helper.params.as_slice() else {
        return Err(Error::UnsupportedProgram);
    };
    if !helper.source_owned {
        return Err(Error::UnsupportedProgram);
    }
    let [guide, depth, slot, parent, child] =
        fields(&parameter.ty, ["guide", "depth", "slot", "parent", "child"])?;
    let [expand, next, well_formed, sufficient] =
        fields(&helper.ty, ["expand", "guide", "wellFormed", "sufficient"])?;
    exactly(depth, &WORD)?;
    exactly(slot, &WORD)?;
    for flag in [expand, well_formed, sufficient] {
        exactly(flag, &RuntimeType::Bool)?;
    }
    if guide != &shape.guide
        || next != &shape.guide
        || parent != &shape.node
        || child != &shape.node
    {
        return Err(Error::UnsupportedProgram);
    }
    Ok(())
}

fn record<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(name, value)| (Value::Text(name.to_owned()), value))
            .collect(),
    )
}

fn identifier(value: &Value) -> Result<[u8; 32], Error> {
    bytes(value)?.try_into().map_err(|_| Error::InvalidArtifact)
}

fn word(value: u64) -> Value {
    Value::Integer(i128::from(value))
}

fn index(length: usize) -> Result<u64, Error> {
    u64::try_from(length).map_err(|_| Error::AllocationBudgetExceeded)
}

fn flag(decision: &Value, name: &str) -> Result<bool, Error> {
    match field(decision, name)? {
        Value::Bool(value) => Ok(*value),
        _ => Err(Error::InvalidArtifact),
    }
}

/// Everything one read step borrows from the running evaluation.
pub(crate) struct Session<'a, A> {
    pub graph: &'a GraphReads,
    pub helpers: &'a BTreeMap<String, Helper>,
    pub view: &'a mut A,
    pub meter: &'a mut Meter,
    pub usage: &'a mut Usage,
    pub limits: &'a ReadLimits,
}

impl<A: NodeAtomAccess> Session<'_, A> {
    /// Reads one typed range under the evaluation's remaining scan allowance.
    fn scan(
        &mut self,
        range: OutgoingEdgeRange,
        source_type: Option<TypeId>,
    ) -> Result<Outgoing, A::Error> {
        self.usage.attempt(self.limits)?;
        let remaining = self
            .graph
            .max_edge_scans
            .min(self.view.scan_ceiling())
            .saturating_sub(self.usage.edge_scans);
        let outgoing = self
            .view
            .outgoing(range, source_type, remaining, self.meter)?;
        if let Outgoing::Edges { inspected, .. } = &outgoing {
            self.usage.edge_scans = self.usage.edge_scans.saturating_add(*inspected);
        }
        Ok(outgoing)
    }
}

/// Reads one node's outgoing edges of one type, in edge-identifier order.
///
/// A present source with no matching edge yields the empty list; an absent
/// source is a distinct failure. Every edge in the source's bucket counts
/// toward the evaluation's scan bound, matching or not.
pub(crate) fn edges<A: NodeAtomAccess>(
    max_edges: u64,
    input: &Value,
    session: &mut Session<'_, A>,
) -> Result<Result<Value, Obstruction>, A::Error> {
    exact_fields(input, &["warpId", "nodeId", "edgeType"])?;
    let source = NodeKey {
        warp_id: WarpId(identifier(field(input, "warpId")?)?),
        local_id: NodeId(identifier(field(input, "nodeId")?)?),
    };
    let range = OutgoingEdgeRange {
        source,
        edge_type: TypeId(identifier(field(input, "edgeType")?)?),
    };
    if !session.view.contains(source) {
        return Err(ReadError::OutsideAperture(source).into());
    }
    let found = match session.scan(range, None)? {
        Outgoing::SourceMissing => return Ok(Err(Obstruction::EdgeSourceMissing)),
        Outgoing::ScanBound => return Ok(Err(Obstruction::EdgeScanBoundExceeded)),
        Outgoing::Edges { edges, .. } => edges,
    };
    if index(found.len())? > max_edges {
        return Ok(Err(Obstruction::EdgeBoundExceeded));
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(found.len())
        .map_err(|_| Error::HostAllocationFailed)?;
    for edge in found {
        rows.push(record([
            ("edgeId", Value::Bytes(edge.id.0.to_vec())),
            ("toId", Value::Bytes(edge.to.0.to_vec())),
        ]));
    }
    let value = Value::Array(rows);
    session.meter.charge(&value, 0)?;
    Ok(Ok(value))
}

struct Occurrence {
    node: usize,
    depth: u64,
    guide: Value,
}

struct Position {
    from: usize,
    slot: usize,
    via: usize,
    child: usize,
}

struct Walk<'a> {
    shape: &'a ClosureShape,
    warp: WarpId,
    max_nodes: u64,
    max_depth: u64,
    decide: Expr,
    known: BTreeMap<NodeId, usize>,
    keys: Vec<(NodeId, TypeId)>,
    nodes: Vec<Value>,
    edges: Vec<Value>,
    occurrences: Vec<Value>,
    pending: Vec<Occurrence>,
}

impl Walk<'_> {
    /// Resolves one node's record and alpha atom once per distinct identity.
    fn materialize<A: NodeAtomAccess>(
        &mut self,
        id: NodeId,
        referrer: Option<TypeId>,
        session: &mut Session<'_, A>,
    ) -> Result<Result<usize, Obstruction>, A::Error> {
        if let Some(known) = self.known.get(&id) {
            return Ok(Ok(*known));
        }
        if index(self.nodes.len())? >= self.max_nodes {
            return Ok(Err(Obstruction::ClosureNodeBoundExceeded));
        }
        session.usage.attempt(session.limits)?;
        let key = NodeKey {
            warp_id: self.warp,
            local_id: id,
        };
        let absent = if referrer.is_some() {
            Obstruction::ClosureStorageMalformed
        } else {
            Obstruction::ClosureStartMissing
        };
        let (node_type, atom_type, atom) =
            match session
                .view
                .material(key, referrer, self.shape.node_bytes, session.meter)?
            {
                Material::Missing => return Ok(Err(absent)),
                Material::Portal => return Ok(Err(Obstruction::ClosureStorageMalformed)),
                Material::Node { ty, atom } => {
                    let (atom_type, payload) = match atom {
                        Some((atom_type, payload)) => (atom_type.0.to_vec(), payload),
                        None => (Vec::new(), &[][..]),
                    };
                    let count = index(payload.len())?;
                    if count > self.shape.node_bytes {
                        return Ok(Err(Obstruction::ClosureStorageMalformed));
                    }
                    session.usage.deliver(count, session.limits)?;
                    // Charge deterministic I/O work and storage before copying.
                    session.meter.charge_steps(count)?;
                    session.meter.allocate(
                        VALUE_CELL_BYTES
                            .checked_add(count)
                            .ok_or(Error::AllocationBudgetExceeded)?,
                    )?;
                    (
                        ty,
                        atom_type,
                        session.meter.copy_precharged_read_bytes(payload)?,
                    )
                }
            };
        let row = record([
            ("nodeId", Value::Bytes(id.0.to_vec())),
            ("nodeType", Value::Bytes(node_type.0.to_vec())),
            ("atomType", Value::Bytes(atom_type)),
            ("atomBytes", atom),
        ]);
        session.meter.charge(&row, 0)?;
        let position = self.nodes.len();
        self.known.insert(id, position);
        self.keys.push((id, node_type));
        self.nodes.push(row);
        Ok(Ok(position))
    }

    /// Records one selected position: its node, the edge row it arrived by,
    /// its depth and the guide the selector assigned to it.
    fn occupy(
        &mut self,
        node: usize,
        edge: usize,
        depth: u64,
        guide: Value,
        meter: &mut Meter,
    ) -> Result<Result<(), Obstruction>, Error> {
        if index(self.occurrences.len())? >= self.shape.max_occurrences {
            return Ok(Err(Obstruction::ClosureWorkBoundExceeded));
        }
        let row = record([
            ("node", word(index(node)?)),
            ("edge", word(index(edge)?)),
            ("depth", word(depth)),
            ("guide", meter.copy(&guide)?),
        ]);
        meter.charge(&row, 0)?;
        self.occurrences.push(row);
        self.pending.push(Occurrence { node, depth, guide });
        Ok(Ok(()))
    }

    /// Offers one materialized child to the selector and applies its decision.
    fn offer(
        &mut self,
        at: &Position,
        helpers: &BTreeMap<String, Helper>,
        meter: &mut Meter,
    ) -> Result<Result<(), Obstruction>, Error> {
        let (Some(current), Some(child_row)) =
            (self.pending.get(at.from), self.nodes.get(at.child))
        else {
            return Err(Error::InvalidArtifact);
        };
        let parent_row = self.nodes.get(current.node).ok_or(Error::InvalidArtifact)?;
        let depth = current.depth.checked_add(1).ok_or(Error::InvalidArtifact)?;
        let candidate = record([
            ("guide", meter.copy(&current.guide)?),
            ("depth", word(depth)),
            ("slot", word(index(at.slot)?)),
            ("parent", meter.copy(parent_row)?),
            ("child", meter.copy(child_row)?),
        ]);
        let locals = BTreeMap::from([("candidate", candidate)]);
        let decision = expression(&self.decide, &locals, helpers, meter, 0)?;
        if !flag(&decision, "wellFormed")? {
            return Ok(Err(Obstruction::ClosureStorageMalformed));
        }
        if !flag(&decision, "sufficient")? {
            return Ok(Err(Obstruction::ClosureSelectorInsufficient));
        }
        if !flag(&decision, "expand")? {
            return Ok(Ok(()));
        }
        if depth > self.max_depth {
            return Ok(Err(Obstruction::ClosureDepthBoundExceeded));
        }
        let guide = meter.copy(field(&decision, "guide")?)?;
        self.occupy(at.child, at.via, depth, guide, meter)
    }
}

/// Reads the closure an authored selector asks for from one start node.
///
/// Occurrences are expanded breadth-first in selection order. For each
/// expanded occurrence and each declared edge type, in declared order, the
/// matching outgoing edges are read in edge-identifier order; every target is
/// materialized once per distinct node identity, recorded as an edge row, and
/// offered to the selector as a candidate. A selected candidate becomes a new
/// occurrence one level deeper, carrying the guide the selector assigned.
pub(crate) fn closure<A: NodeAtomAccess>(
    shape: &ClosureShape,
    input: &Value,
    session: &mut Session<'_, A>,
) -> Result<Result<Value, Obstruction>, A::Error> {
    exact_fields(
        input,
        &[
            "warpId",
            "nodeId",
            "edgeTypes",
            "maxNodes",
            "maxDepth",
            "guide",
        ],
    )?;
    let graph = session.graph;
    let selector = graph.selector.as_deref().ok_or(Error::UnsupportedProgram)?;
    let warp = WarpId(identifier(field(input, "warpId")?)?);
    let start = NodeId(identifier(field(input, "nodeId")?)?);
    let mut edge_types = Vec::new();
    for value in array(field(input, "edgeTypes")?)? {
        let edge_type = TypeId(identifier(value)?);
        if edge_types.len() >= MAX_EDGE_TYPES || edge_types.contains(&edge_type) {
            return Err(Error::InvalidArtifact.into());
        }
        edge_types.push(edge_type);
    }
    // A closure is a graph read. Without a granted graph aperture it observes
    // nothing, not even its host-selected start node.
    let first = *edge_types.first().ok_or(Error::InvalidArtifact)?;
    if session.view.scan_ceiling() == 0 {
        return Err(ReadError::OutsideGraphAperture(first).into());
    }
    let guide = field(input, "guide")?;
    validate(guide, &shape.guide, session.meter, 0)?;
    let key = NodeKey {
        warp_id: warp,
        local_id: start,
    };
    if !session.view.contains(key) {
        return Err(ReadError::OutsideAperture(key).into());
    }
    let mut walk = Walk {
        shape,
        warp,
        max_nodes: number(field(input, "maxNodes")?)?
            .min(shape.max_nodes)
            .min(graph.max_closure_nodes),
        max_depth: number(field(input, "maxDepth")?)?.min(graph.max_closure_depth),
        decide: Expr::Call(
            selector.to_owned(),
            vec![Expr::Local("candidate".to_owned())],
        ),
        known: BTreeMap::new(),
        keys: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        occurrences: Vec::new(),
        pending: Vec::new(),
    };
    let root = match walk.materialize(start, None, session)? {
        Ok(root) => root,
        Err(obstruction) => return Ok(Err(obstruction)),
    };
    let guide = session.meter.copy(guide)?;
    if let Err(obstruction) = walk.occupy(root, 0, 0, guide, session.meter)? {
        return Ok(Err(obstruction));
    }
    let mut next = 0;
    while let Some(occurrence) = walk.pending.get(next) {
        session.meter.step(0)?;
        let (parent_id, parent_type) = *walk
            .keys
            .get(occurrence.node)
            .ok_or(Error::InvalidArtifact)?;
        for (slot, edge_type) in edge_types.iter().enumerate() {
            let range = OutgoingEdgeRange {
                source: NodeKey {
                    warp_id: warp,
                    local_id: parent_id,
                },
                edge_type: *edge_type,
            };
            let found = match session.scan(range, Some(parent_type))? {
                // The parent was materialized from this same immutable view.
                Outgoing::SourceMissing => return Err(Error::InvalidArtifact.into()),
                Outgoing::ScanBound => return Ok(Err(Obstruction::ClosureScanBoundExceeded)),
                Outgoing::Edges { edges, .. } => edges,
            };
            for edge in found {
                // A full edge list is decided before the target is read.
                if index(walk.edges.len())? >= shape.max_edges {
                    return Ok(Err(Obstruction::ClosureWorkBoundExceeded));
                }
                let child = match walk.materialize(edge.to, Some(parent_type), session)? {
                    Ok(child) => child,
                    Err(obstruction) => return Ok(Err(obstruction)),
                };
                let row = record([
                    ("from", word(index(next)?)),
                    ("slot", word(index(slot)?)),
                    ("edgeId", Value::Bytes(edge.id.0.to_vec())),
                    ("node", word(index(child)?)),
                ]);
                session.meter.charge(&row, 0)?;
                let via = walk.edges.len();
                walk.edges.push(row);
                let at = Position {
                    from: next,
                    slot,
                    via,
                    child,
                };
                if let Err(obstruction) = walk.offer(&at, session.helpers, session.meter)? {
                    return Ok(Err(obstruction));
                }
            }
        }
        next += 1;
    }
    let value = record([
        ("nodes", Value::Array(walk.nodes)),
        ("edges", Value::Array(walk.edges)),
        ("occurrences", Value::Array(walk.occurrences)),
    ]);
    session.meter.allocate(4 * VALUE_CELL_BYTES)?;
    Ok(Ok(value))
}
