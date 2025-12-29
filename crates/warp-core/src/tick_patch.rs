// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tick patches (Paper III): replayable delta boundary artifacts.
//!
//! A tick patch is the *prescriptive* boundary record for one worldline step:
//! it is sufficient to deterministically replay the tick as a pure delta
//! without re-running rule matching or scheduling.
//!
//! V1 is intentionally minimal:
//! - Ops are canonical graph edits (upserts/deletes) for node + edge records.
//! - `in_slots` / `out_slots` are *unversioned* slot ids (Paper III-compatible).
//!   Value versioning is recovered by interpretation along a worldline payload
//!   `P = (μ0, …, μn-1)` via `ValueVersionId := (slot_id, tick_index)`.

use blake3::Hasher;

use crate::footprint::PortKey;
use crate::graph::GraphStore;
use crate::ident::{EdgeId, Hash, NodeId};
use crate::record::{EdgeRecord, NodeRecord};

/// Commit status of a tick patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickCommitStatus {
    /// Tick committed successfully.
    Committed,
    /// Tick aborted (reserved for future transactional semantics).
    Aborted,
}

impl TickCommitStatus {
    const fn code(self) -> u8 {
        match self {
            Self::Committed => 1,
            Self::Aborted => 2,
        }
    }
}

/// Unversioned slot identifier for slicing and provenance bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotId {
    /// Full node record at `NodeId` (type id + payload bytes).
    Node(NodeId),
    /// Full edge record at `EdgeId` (from/to/type/payload).
    Edge(EdgeId),
    /// Boundary port value (opaque key).
    Port(PortKey),
}

impl SlotId {
    const fn tag(self) -> u8 {
        match self {
            Self::Node(_) => 1,
            Self::Edge(_) => 2,
            Self::Port(_) => 3,
        }
    }
}

impl PartialOrd for SlotId {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SlotId {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let tag_cmp = self.tag().cmp(&other.tag());
        if tag_cmp != core::cmp::Ordering::Equal {
            return tag_cmp;
        }
        match (*self, *other) {
            (Self::Node(a), Self::Node(b)) => a.0.cmp(&b.0),
            (Self::Edge(a), Self::Edge(b)) => a.0.cmp(&b.0),
            (Self::Port(a), Self::Port(b)) => a.cmp(&b),
            // SAFETY: tag comparison above guarantees matching variants.
            _ => unreachable!("tag mismatch in SlotId::cmp"),
        }
    }
}

/// A canonical delta operation applied to the graph store.
#[derive(Debug, Clone)]
pub enum WarpOp {
    /// Insert or replace a node record.
    UpsertNode {
        /// Node identifier being inserted or replaced.
        node: NodeId,
        /// Full node record contents.
        record: NodeRecord,
    },
    /// Delete a node record.
    DeleteNode {
        /// Node identifier being deleted.
        node: NodeId,
    },
    /// Insert or replace an edge record.
    UpsertEdge {
        /// Full edge record contents.
        record: EdgeRecord,
    },
    /// Delete an edge record from the outbound edge list of `from`.
    DeleteEdge {
        /// Source node bucket holding the edge.
        from: NodeId,
        /// Edge identifier being deleted.
        edge_id: EdgeId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct WarpOpKey {
    kind: u8,
    a: Hash,
    b: Hash,
}

impl WarpOp {
    fn sort_key(&self) -> WarpOpKey {
        match self {
            Self::DeleteEdge { from, edge_id } => WarpOpKey {
                kind: 1,
                a: from.0,
                b: edge_id.0,
            },
            Self::DeleteNode { node } => WarpOpKey {
                kind: 2,
                a: node.0,
                b: [0u8; 32],
            },
            Self::UpsertNode { node, .. } => WarpOpKey {
                kind: 3,
                a: node.0,
                b: [0u8; 32],
            },
            Self::UpsertEdge { record } => WarpOpKey {
                kind: 4,
                a: record.from.0,
                b: record.id.0,
            },
        }
    }
}

/// A replayable delta patch for one tick.
///
/// The patch is the boundary artifact for deterministic replay: applying
/// `ops` to the prior state yields the successor state for this tick.
///
/// The patch digest commits to:
/// - patch format version,
/// - `policy_id`,
/// - `rule_pack_id`,
/// - `commit_status`,
/// - `in_slots` / `out_slots`, and
/// - `ops`.
#[derive(Debug, Clone)]
pub struct WarpTickPatchV1 {
    policy_id: u32,
    rule_pack_id: Hash,
    commit_status: TickCommitStatus,
    in_slots: Vec<SlotId>,
    out_slots: Vec<SlotId>,
    ops: Vec<WarpOp>,
    digest: Hash,
}

impl WarpTickPatchV1 {
    /// Constructs a new patch and canonicalizes ordering.
    ///
    /// Canonicalization:
    /// - `in_slots` and `out_slots` are sorted and deduped.
    /// - `ops` are sorted into canonical op order (see spec).
    #[must_use]
    pub fn new(
        policy_id: u32,
        rule_pack_id: Hash,
        commit_status: TickCommitStatus,
        mut in_slots: Vec<SlotId>,
        mut out_slots: Vec<SlotId>,
        mut ops: Vec<WarpOp>,
    ) -> Self {
        in_slots.sort();
        in_slots.dedup();
        out_slots.sort();
        out_slots.dedup();
        ops.sort_by_key(WarpOp::sort_key);
        let digest = compute_patch_digest_v1(
            policy_id,
            &rule_pack_id,
            commit_status,
            &in_slots,
            &out_slots,
            &ops,
        );
        Self {
            policy_id,
            rule_pack_id,
            commit_status,
            in_slots,
            out_slots,
            ops,
            digest,
        }
    }

    /// Policy identifier associated with this patch.
    #[must_use]
    pub fn policy_id(&self) -> u32 {
        self.policy_id
    }

    /// Rule-pack identifier associated with this patch.
    ///
    /// This pins the producing rule-pack for auditability but does not affect
    /// replay semantics (replay executes `ops` only).
    #[must_use]
    pub fn rule_pack_id(&self) -> Hash {
        self.rule_pack_id
    }

    /// Commit status (Committed vs Aborted).
    #[must_use]
    pub fn commit_status(&self) -> TickCommitStatus {
        self.commit_status
    }

    /// Slots read by this tick (conservative set).
    #[must_use]
    pub fn in_slots(&self) -> &[SlotId] {
        &self.in_slots
    }

    /// Slots produced by this tick.
    #[must_use]
    pub fn out_slots(&self) -> &[SlotId] {
        &self.out_slots
    }

    /// Canonical delta operations for this tick.
    #[must_use]
    pub fn ops(&self) -> &[WarpOp] {
        &self.ops
    }

    /// Canonical digest of the patch contents.
    #[must_use]
    pub fn digest(&self) -> Hash {
        self.digest
    }

    /// Applies the patch delta to `store`.
    ///
    /// # Errors
    /// Returns an error if an operation is invalid for the current store
    /// state (e.g., deleting a missing edge).
    pub fn apply_to_store(&self, store: &mut GraphStore) -> Result<(), TickPatchError> {
        for op in &self.ops {
            match op {
                WarpOp::UpsertNode { node, record } => {
                    store.nodes.insert(*node, record.clone());
                }
                WarpOp::DeleteNode { node } => {
                    if store.nodes.remove(node).is_none() {
                        return Err(TickPatchError::MissingNode(*node));
                    }
                }
                WarpOp::UpsertEdge { record } => {
                    // Remove any existing edge with the same id from its current bucket.
                    //
                    // NOTE: `remove_edge_by_id` is O(total_edges). This is acceptable for v0
                    // correctness, but replay performance will require a reverse index
                    // (`EdgeId -> NodeId`) once graph sizes grow.
                    remove_edge_by_id(store, &record.id);
                    store
                        .edges_from
                        .entry(record.from)
                        .or_default()
                        .push(record.clone());
                }
                WarpOp::DeleteEdge { from, edge_id } => {
                    let Some(edges) = store.edges_from.get_mut(from) else {
                        return Err(TickPatchError::MissingEdge(*edge_id));
                    };
                    let before = edges.len();
                    edges.retain(|e| e.id != *edge_id);
                    if edges.len() == before {
                        return Err(TickPatchError::MissingEdge(*edge_id));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Errors produced while applying a tick patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickPatchError {
    /// Tried to delete a node that did not exist.
    MissingNode(NodeId),
    /// Tried to delete an edge that did not exist.
    MissingEdge(EdgeId),
}

impl core::fmt::Display for TickPatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingNode(id) => write!(f, "missing node: {id:?}"),
            Self::MissingEdge(id) => write!(f, "missing edge: {id:?}"),
        }
    }
}

impl std::error::Error for TickPatchError {}

fn remove_edge_by_id(store: &mut GraphStore, id: &EdgeId) {
    for edges in store.edges_from.values_mut() {
        edges.retain(|e| e.id != *id);
    }
}

fn compute_patch_digest_v1(
    policy_id: u32,
    rule_pack_id: &Hash,
    commit_status: TickCommitStatus,
    in_slots: &[SlotId],
    out_slots: &[SlotId],
    ops: &[WarpOp],
) -> Hash {
    let mut h = Hasher::new();
    // Patch format version.
    h.update(&1u16.to_le_bytes());
    h.update(&policy_id.to_le_bytes());
    h.update(rule_pack_id);
    h.update(&[commit_status.code()]);

    encode_slots(&mut h, in_slots);
    encode_slots(&mut h, out_slots);
    encode_ops(&mut h, ops);
    h.finalize().into()
}

fn encode_slots(h: &mut Hasher, slots: &[SlotId]) {
    h.update(&(slots.len() as u64).to_le_bytes());
    for slot in slots {
        match slot {
            SlotId::Node(id) => {
                h.update(&[1u8]);
                h.update(&id.0);
            }
            SlotId::Edge(id) => {
                h.update(&[2u8]);
                h.update(&id.0);
            }
            SlotId::Port(key) => {
                h.update(&[3u8]);
                h.update(&key.to_le_bytes());
            }
        }
    }
}

fn encode_ops(h: &mut Hasher, ops: &[WarpOp]) {
    h.update(&(ops.len() as u64).to_le_bytes());
    for op in ops {
        match op {
            WarpOp::UpsertNode { node, record } => {
                h.update(&[1u8]);
                h.update(&node.0);
                h.update(&(record.ty).0);
                match &record.payload {
                    Some(payload) => {
                        h.update(&(payload.len() as u64).to_le_bytes());
                        h.update(payload);
                    }
                    None => {
                        h.update(&0u64.to_le_bytes());
                    }
                }
            }
            WarpOp::DeleteNode { node } => {
                h.update(&[2u8]);
                h.update(&node.0);
            }
            WarpOp::UpsertEdge { record } => {
                h.update(&[3u8]);
                h.update(&(record.from).0);
                h.update(&(record.id).0);
                h.update(&(record.to).0);
                h.update(&(record.ty).0);
                match &record.payload {
                    Some(payload) => {
                        h.update(&(payload.len() as u64).to_le_bytes());
                        h.update(payload);
                    }
                    None => {
                        h.update(&0u64.to_le_bytes());
                    }
                }
            }
            WarpOp::DeleteEdge { from, edge_id } => {
                h.update(&[4u8]);
                h.update(&from.0);
                h.update(&edge_id.0);
            }
        }
    }
}

/// Computes a canonical delta op list from `before` and `after`.
pub(crate) fn diff_store(before: &GraphStore, after: &GraphStore) -> Vec<WarpOp> {
    let mut ops: Vec<WarpOp> = Vec::new();

    // Nodes
    for (id, rec_before) in &before.nodes {
        if !after.nodes.contains_key(id) {
            ops.push(WarpOp::DeleteNode { node: *id });
            continue;
        }
        let rec_after = &after.nodes[id];
        if rec_before.ty != rec_after.ty || rec_before.payload != rec_after.payload {
            ops.push(WarpOp::UpsertNode {
                node: *id,
                record: rec_after.clone(),
            });
        }
    }
    for (id, rec_after) in &after.nodes {
        if !before.nodes.contains_key(id) {
            ops.push(WarpOp::UpsertNode {
                node: *id,
                record: rec_after.clone(),
            });
        }
    }

    // Edges: map by EdgeId for stable diffing independent of insertion order.
    let before_edges = edges_by_id(before);
    let after_edges = edges_by_id(after);
    for (id, rec_before) in &before_edges {
        if !after_edges.contains_key(id) {
            ops.push(WarpOp::DeleteEdge {
                from: rec_before.from,
                edge_id: EdgeId(*id),
            });
        }
    }
    for (id, rec_after) in &after_edges {
        match before_edges.get(id) {
            None => {
                ops.push(WarpOp::UpsertEdge {
                    record: rec_after.clone(),
                });
            }
            Some(rec_before) => {
                if !edge_record_eq(rec_before, rec_after) {
                    if rec_before.from != rec_after.from {
                        ops.push(WarpOp::DeleteEdge {
                            from: rec_before.from,
                            edge_id: EdgeId(*id),
                        });
                    }
                    ops.push(WarpOp::UpsertEdge {
                        record: rec_after.clone(),
                    });
                }
            }
        }
    }

    ops.sort_by_key(WarpOp::sort_key);
    ops
}

fn edges_by_id(store: &GraphStore) -> std::collections::BTreeMap<Hash, EdgeRecord> {
    let mut out = std::collections::BTreeMap::new();
    for edges in store.edges_from.values() {
        for e in edges {
            out.insert(e.id.0, e.clone());
        }
    }
    out
}

fn edge_record_eq(a: &EdgeRecord, b: &EdgeRecord) -> bool {
    a.id == b.id && a.from == b.from && a.to == b.to && a.ty == b.ty && a.payload == b.payload
}
