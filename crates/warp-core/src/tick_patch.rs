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
use thiserror::Error;

use crate::attachment::{AtomPayload, AttachmentKey, AttachmentOwner, AttachmentValue};
use crate::footprint::PortKey;
use crate::graph::GraphStore;
use crate::ident::{EdgeId, EdgeKey, Hash as ContentHash, NodeId, NodeKey, WarpId};
use crate::record::{EdgeRecord, NodeRecord};
use crate::warp_state::{WarpInstance, WarpState};

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
    /// Full node record at `NodeKey` (instance-scoped skeleton record).
    Node(NodeKey),
    /// Full edge record at `EdgeKey` (instance-scoped skeleton record).
    Edge(EdgeKey),
    /// Attachment slot (node/edge plane payload, including `Descend` links).
    Attachment(AttachmentKey),
    /// Boundary port value (opaque key).
    Port(PortKey),
}

impl SlotId {
    const fn tag(self) -> u8 {
        match self {
            Self::Node(_) => 1,
            Self::Edge(_) => 2,
            Self::Attachment(_) => 3,
            Self::Port(_) => 4,
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
            (Self::Node(a), Self::Node(b)) => a.cmp(&b),
            (Self::Edge(a), Self::Edge(b)) => a.cmp(&b),
            (Self::Attachment(a), Self::Attachment(b)) => a.cmp(&b),
            (Self::Port(a), Self::Port(b)) => a.cmp(&b),
            // SAFETY: tag comparison above guarantees matching variants.
            _ => unreachable!("tag mismatch in SlotId::cmp"),
        }
    }
}

/// A canonical delta operation applied to the graph store.
#[derive(Debug, Clone)]
pub enum WarpOp {
    /// Insert or replace warp instance metadata (Stage B1).
    UpsertWarpInstance {
        /// Instance metadata record.
        instance: WarpInstance,
    },
    /// Delete a warp instance and all its contents.
    DeleteWarpInstance {
        /// Instance identifier to delete.
        warp_id: WarpId,
    },
    /// Insert or replace a node record.
    UpsertNode {
        /// Node identifier being inserted or replaced (instance-scoped).
        node: NodeKey,
        /// Full node record contents.
        record: NodeRecord,
    },
    /// Delete a node record.
    DeleteNode {
        /// Node identifier being deleted (instance-scoped).
        node: NodeKey,
    },
    /// Insert or replace an edge record.
    UpsertEdge {
        /// Instance containing the edge.
        warp_id: WarpId,
        /// Full edge record contents.
        record: EdgeRecord,
    },
    /// Delete an edge record from the outbound edge list of `from`.
    DeleteEdge {
        /// Instance containing the edge.
        warp_id: WarpId,
        /// Source node bucket holding the edge.
        from: NodeId,
        /// Edge identifier being deleted.
        edge_id: EdgeId,
    },
    /// Set (or clear) an attachment slot value.
    SetAttachment {
        /// Attachment slot key.
        key: AttachmentKey,
        /// New value (`None` clears the slot).
        value: Option<AttachmentValue>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct WarpOpKey {
    kind: u8,
    warp: ContentHash,
    a: ContentHash,
    b: ContentHash,
}

impl WarpOp {
    fn sort_key(&self) -> WarpOpKey {
        match self {
            Self::UpsertWarpInstance { instance } => WarpOpKey {
                kind: 1,
                warp: (instance.warp_id).0,
                a: (instance.warp_id).0,
                b: [0u8; 32],
            },
            Self::DeleteWarpInstance { warp_id } => WarpOpKey {
                kind: 2,
                warp: warp_id.0,
                a: warp_id.0,
                b: [0u8; 32],
            },
            Self::DeleteEdge {
                warp_id,
                from,
                edge_id,
            } => WarpOpKey {
                kind: 3,
                warp: warp_id.0,
                a: from.0,
                b: edge_id.0,
            },
            Self::DeleteNode { node } => WarpOpKey {
                kind: 4,
                warp: (node.warp_id).0,
                a: (node.local_id).0,
                b: [0u8; 32],
            },
            Self::UpsertNode { node, .. } => WarpOpKey {
                kind: 5,
                warp: (node.warp_id).0,
                a: (node.local_id).0,
                b: [0u8; 32],
            },
            Self::UpsertEdge { warp_id, record } => WarpOpKey {
                kind: 6,
                warp: warp_id.0,
                a: record.from.0,
                b: record.id.0,
            },
            Self::SetAttachment { key, .. } => {
                let (owner_tag, plane_tag) = key.tag();
                // Stable ordering: (kind, owner_tag, plane_tag, warp_id, local_id).
                let (warp, local) = match key.owner {
                    AttachmentOwner::Node(node) => ((node.warp_id).0, (node.local_id).0),
                    AttachmentOwner::Edge(edge) => ((edge.warp_id).0, (edge.local_id).0),
                };
                WarpOpKey {
                    kind: 7,
                    warp,
                    a: {
                        let mut buf = [0u8; 32];
                        buf[0] = owner_tag;
                        buf[1] = plane_tag;
                        buf
                    },
                    b: local,
                }
            }
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
    rule_pack_id: ContentHash,
    commit_status: TickCommitStatus,
    in_slots: Vec<SlotId>,
    out_slots: Vec<SlotId>,
    ops: Vec<WarpOp>,
    digest: ContentHash,
}

impl WarpTickPatchV1 {
    /// Constructs a new patch and canonicalizes ordering.
    ///
    /// Canonicalization:
    /// - `in_slots` and `out_slots` are sorted and deduped.
    /// - `ops` are sorted into canonical op order (see spec) and deduped by
    ///   the same sort key used for canonical ordering (`WarpOp::sort_key`);
    ///   duplicate ops are collapsed as “last wins” after canonical sorting.
    #[must_use]
    pub fn new(
        policy_id: u32,
        rule_pack_id: ContentHash,
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
        ops.dedup_by(|a, b| {
            if a.sort_key() == b.sort_key() {
                // Last-wins: after stable sorting, equal-key ops preserve input order.
                // Replace the retained op (`a`) with the later op (`b`) and drop `b`.
                *a = b.clone();
                true
            } else {
                false
            }
        });
        let digest = compute_patch_digest_v2(
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
    pub fn rule_pack_id(&self) -> ContentHash {
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
    pub fn digest(&self) -> ContentHash {
        self.digest
    }

    /// Applies the patch delta to `state`.
    ///
    /// # Errors
    /// Returns an error if an operation is invalid for the current store
    /// state (e.g., deleting a missing edge).
    pub fn apply_to_state(&self, state: &mut WarpState) -> Result<(), TickPatchError> {
        for op in &self.ops {
            match op {
                WarpOp::UpsertWarpInstance { instance } => {
                    let store = state
                        .stores
                        .remove(&instance.warp_id)
                        .unwrap_or_else(|| GraphStore::new(instance.warp_id));
                    state.upsert_instance(instance.clone(), store);
                }
                WarpOp::DeleteWarpInstance { warp_id } => {
                    if !state.delete_instance(warp_id) {
                        return Err(TickPatchError::MissingWarp(*warp_id));
                    }
                }
                WarpOp::UpsertNode { node, record } => {
                    let Some(store) = state.store_mut(&node.warp_id) else {
                        return Err(TickPatchError::MissingWarp(node.warp_id));
                    };
                    store.insert_node(node.local_id, record.clone());
                }
                WarpOp::DeleteNode { node } => {
                    let Some(store) = state.store_mut(&node.warp_id) else {
                        return Err(TickPatchError::MissingWarp(node.warp_id));
                    };
                    if !store.delete_node_cascade(node.local_id) {
                        return Err(TickPatchError::MissingNode(*node));
                    }
                }
                WarpOp::UpsertEdge { warp_id, record } => {
                    let Some(store) = state.store_mut(warp_id) else {
                        return Err(TickPatchError::MissingWarp(*warp_id));
                    };
                    store.upsert_edge_record(record.from, record.clone());
                }
                WarpOp::DeleteEdge {
                    warp_id,
                    from,
                    edge_id,
                } => {
                    let Some(store) = state.store_mut(warp_id) else {
                        return Err(TickPatchError::MissingWarp(*warp_id));
                    };
                    if !store.delete_edge_exact(*from, *edge_id) {
                        return Err(TickPatchError::MissingEdge(EdgeKey {
                            warp_id: *warp_id,
                            local_id: *edge_id,
                        }));
                    }
                }
                WarpOp::SetAttachment { key, value } => match key.owner {
                    AttachmentOwner::Node(node) => {
                        if key.plane != crate::attachment::AttachmentPlane::Alpha {
                            return Err(TickPatchError::InvalidAttachmentKey(*key));
                        }
                        let Some(store) = state.store_mut(&node.warp_id) else {
                            return Err(TickPatchError::MissingWarp(node.warp_id));
                        };
                        if store.node(&node.local_id).is_none() {
                            return Err(TickPatchError::MissingNode(node));
                        }
                        store.set_node_attachment(node.local_id, value.clone());
                    }
                    AttachmentOwner::Edge(edge) => {
                        if key.plane != crate::attachment::AttachmentPlane::Beta {
                            return Err(TickPatchError::InvalidAttachmentKey(*key));
                        }
                        let Some(store) = state.store_mut(&edge.warp_id) else {
                            return Err(TickPatchError::MissingWarp(edge.warp_id));
                        };
                        if !store.edge_index.contains_key(&edge.local_id) {
                            return Err(TickPatchError::MissingEdge(edge));
                        }
                        store.set_edge_attachment(edge.local_id, value.clone());
                    }
                },
            }
        }
        Ok(())
    }
}

/// Errors produced while applying a tick patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TickPatchError {
    /// Referenced a warp instance that did not exist.
    #[error("missing warp: {0:?}")]
    MissingWarp(WarpId),
    /// Tried to delete a node that did not exist.
    #[error("missing node: {0:?}")]
    MissingNode(NodeKey),
    /// Tried to delete an edge that did not exist.
    #[error("missing edge: {0:?}")]
    MissingEdge(EdgeKey),
    /// Tried to set an attachment slot that is not valid in v1.
    #[error("invalid attachment key: {0:?}")]
    InvalidAttachmentKey(AttachmentKey),
}

fn compute_patch_digest_v2(
    policy_id: u32,
    rule_pack_id: &ContentHash,
    commit_status: TickCommitStatus,
    in_slots: &[SlotId],
    out_slots: &[SlotId],
    ops: &[WarpOp],
) -> ContentHash {
    let mut h = Hasher::new();
    // Patch format version.
    h.update(&2u16.to_le_bytes());
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
                h.update(&(id.warp_id).0);
                h.update(&(id.local_id).0);
            }
            SlotId::Edge(id) => {
                h.update(&[2u8]);
                h.update(&(id.warp_id).0);
                h.update(&(id.local_id).0);
            }
            SlotId::Attachment(key) => {
                h.update(&[3u8]);
                encode_attachment_key(h, key);
            }
            SlotId::Port(key) => {
                h.update(&[4u8]);
                h.update(&key.to_le_bytes());
            }
        }
    }
}

/// Encodes ops into the patch digest stream.
///
/// The op tag bytes are part of the patch format and exist solely to provide a
/// stable, versioned encoding for hashing (`patch_digest`). They are
/// intentionally distinct from `WarpOp::sort_key`’s `kind` values, which exist
/// only to define deterministic replay ordering.
fn encode_ops(h: &mut Hasher, ops: &[WarpOp]) {
    h.update(&(ops.len() as u64).to_le_bytes());
    for op in ops {
        match op {
            WarpOp::UpsertWarpInstance { instance } => {
                h.update(&[1u8]);
                h.update(&(instance.warp_id).0);
                h.update(&(instance.root_node).0);
                encode_attachment_key_opt(h, instance.parent.as_ref());
            }
            WarpOp::DeleteWarpInstance { warp_id } => {
                h.update(&[2u8]);
                h.update(&warp_id.0);
            }
            WarpOp::UpsertNode { node, record } => {
                h.update(&[3u8]);
                h.update(&(node.warp_id).0);
                h.update(&(node.local_id).0);
                h.update(&(record.ty).0);
            }
            WarpOp::DeleteNode { node } => {
                h.update(&[4u8]);
                h.update(&(node.warp_id).0);
                h.update(&(node.local_id).0);
            }
            WarpOp::UpsertEdge { warp_id, record } => {
                h.update(&[5u8]);
                h.update(&warp_id.0);
                h.update(&(record.from).0);
                h.update(&(record.id).0);
                h.update(&(record.to).0);
                h.update(&(record.ty).0);
            }
            WarpOp::DeleteEdge {
                warp_id,
                from,
                edge_id,
            } => {
                h.update(&[6u8]);
                h.update(&warp_id.0);
                h.update(&from.0);
                h.update(&edge_id.0);
            }
            WarpOp::SetAttachment { key, value } => {
                h.update(&[7u8]);
                encode_attachment_key(h, key);
                encode_attachment_value_opt(h, value.as_ref());
            }
        }
    }
}

fn encode_attachment_key_opt(h: &mut Hasher, key: Option<&AttachmentKey>) {
    match key {
        None => {
            h.update(&[0u8]);
        }
        Some(key) => {
            h.update(&[1u8]);
            encode_attachment_key(h, key);
        }
    }
}

fn encode_attachment_key(h: &mut Hasher, key: &AttachmentKey) {
    let (owner_tag, plane_tag) = key.tag();
    h.update(&[owner_tag]);
    h.update(&[plane_tag]);
    match key.owner {
        AttachmentOwner::Node(node) => {
            h.update(&(node.warp_id).0);
            h.update(&(node.local_id).0);
        }
        AttachmentOwner::Edge(edge) => {
            h.update(&(edge.warp_id).0);
            h.update(&(edge.local_id).0);
        }
    }
}

fn encode_attachment_value_opt(h: &mut Hasher, value: Option<&AttachmentValue>) {
    match value {
        None => {
            h.update(&[0u8]);
        }
        Some(value) => {
            h.update(&[1u8]);
            encode_attachment_value(h, value);
        }
    }
}

fn encode_attachment_value(h: &mut Hasher, value: &AttachmentValue) {
    match value {
        AttachmentValue::Atom(atom) => {
            h.update(&[1u8]);
            encode_atom_payload(h, atom);
        }
        AttachmentValue::Descend(warp_id) => {
            h.update(&[2u8]);
            h.update(&warp_id.0);
        }
    }
}

fn encode_atom_payload(h: &mut Hasher, atom: &AtomPayload) {
    h.update(&(atom.type_id).0);
    h.update(&(atom.bytes.len() as u64).to_le_bytes());
    h.update(&atom.bytes);
}

/// Computes a canonical delta op list that transforms one multi-instance state into another.
///
/// This is the engine’s “diff constructor” for [`WarpTickPatchV1`]. The engine:
/// 1) snapshots `before`,
/// 2) executes rewrites to produce `after`, then
/// 3) calls `diff_state(before, after)` to obtain a canonical list of [`WarpOp`]
///    edits suitable for deterministic replay and hashing (`patch_digest`).
///
/// Typical use cases:
/// - Engine commit path: derive the tick’s delta patch without re-searching.
/// - Tooling/tests: validate that two states differ only by a specific op set.
///
/// # Required invariants
/// Callers must provide internally consistent `WarpState` inputs:
/// - Every `WarpInstance` in `instances` has a corresponding `GraphStore` in `stores`
///   and `GraphStore.warp_id == WarpInstance.warp_id`.
/// - Per-store referential integrity holds (edges reference existing nodes, etc).
/// - Deletions are *cascading*: deleting a node/instance implicitly deletes its
///   incident edges and its attachment slots. This function therefore does not emit
///   explicit “clear attachment” ops for deleted owners.
///
/// # Returned ops (semantic meaning)
/// - `UpsertWarpInstance` / `DeleteWarpInstance`: instance metadata changes.
/// - `UpsertNode` / `DeleteNode`: skeleton-plane node record edits scoped to an instance.
/// - `UpsertEdge` / `DeleteEdge`: skeleton-plane edge record edits scoped to an instance.
/// - `SetAttachment`: attachment-plane slot edits (atoms and `Descend` links).
///
/// # Edge cases & performance
/// - If the inputs are identical, returns an empty vector.
/// - Complexity is linear in the number of nodes/edges/attachments across all instances,
///   plus a per-instance edge id map build (`edges_by_id`).
///
/// Determinism contract:
/// - Instance iteration is deterministic (`BTreeMap`).
/// - The returned ops are canonicalized by `WarpOp::sort_key` and are suitable
///   for both replay ordering and `patch_digest` hashing.
pub(crate) fn diff_state(before: &WarpState, after: &WarpState) -> Vec<WarpOp> {
    let mut ops: Vec<WarpOp> = Vec::new();

    // WarpInstances: deletions and upserts.
    for warp_id in before.instances.keys() {
        if !after.instances.contains_key(warp_id) {
            ops.push(WarpOp::DeleteWarpInstance { warp_id: *warp_id });
        }
    }
    for (warp_id, inst_after) in &after.instances {
        match before.instances.get(warp_id) {
            None => ops.push(WarpOp::UpsertWarpInstance {
                instance: inst_after.clone(),
            }),
            Some(inst_before) => {
                if inst_before != inst_after {
                    ops.push(WarpOp::UpsertWarpInstance {
                        instance: inst_after.clone(),
                    });
                }
            }
        }
    }

    // Per-instance skeleton and attachment-plane diffs.
    let empty = GraphStore::default();
    for (warp_id, after_store) in &after.stores {
        let before_store = before.stores.get(warp_id).unwrap_or(&empty);
        diff_instance(&mut ops, *warp_id, before_store, after_store);
    }

    ops.sort_by_key(WarpOp::sort_key);
    ops
}

fn diff_instance(ops: &mut Vec<WarpOp>, warp_id: WarpId, before: &GraphStore, after: &GraphStore) {
    // Nodes (skeleton plane)
    for (id, rec_before) in &before.nodes {
        let Some(rec_after) = after.nodes.get(id) else {
            ops.push(WarpOp::DeleteNode {
                node: NodeKey {
                    warp_id,
                    local_id: *id,
                },
            });
            continue;
        };
        if rec_before != rec_after {
            ops.push(WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: *id,
                },
                record: rec_after.clone(),
            });
        }
    }
    for (id, rec_after) in &after.nodes {
        if !before.nodes.contains_key(id) {
            ops.push(WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: *id,
                },
                record: rec_after.clone(),
            });
        }
    }

    // Node attachments (α plane): diff only for nodes that exist in `after`.
    for node_id in after.nodes.keys() {
        let before_val = before.node_attachment(node_id);
        let after_val = after.node_attachment(node_id);
        if before_val != after_val {
            ops.push(WarpOp::SetAttachment {
                key: AttachmentKey::node_alpha(NodeKey {
                    warp_id,
                    local_id: *node_id,
                }),
                value: after_val.cloned(),
            });
        }
    }

    // Edges (skeleton plane): map by EdgeId for stable diff independent of insertion order.
    let before_edges = edges_by_id(before);
    let after_edges = edges_by_id(after);
    for (id, rec_before) in &before_edges {
        if !after_edges.contains_key(id) {
            ops.push(WarpOp::DeleteEdge {
                warp_id,
                from: rec_before.from,
                edge_id: EdgeId(*id),
            });
        }
    }
    for (id, rec_after) in &after_edges {
        match before_edges.get(id) {
            None => {
                ops.push(WarpOp::UpsertEdge {
                    warp_id,
                    record: rec_after.clone(),
                });
            }
            Some(rec_before) => {
                if rec_before != rec_after {
                    if rec_before.from != rec_after.from {
                        ops.push(WarpOp::DeleteEdge {
                            warp_id,
                            from: rec_before.from,
                            edge_id: EdgeId(*id),
                        });
                    }
                    ops.push(WarpOp::UpsertEdge {
                        warp_id,
                        record: rec_after.clone(),
                    });
                }
            }
        }
    }

    // Edge attachments (β plane): diff only for edges that exist in `after`.
    for id in after_edges.keys() {
        let edge_id = EdgeId(*id);
        let before_val = before.edge_attachment(&edge_id);
        let after_val = after.edge_attachment(&edge_id);
        if before_val != after_val {
            ops.push(WarpOp::SetAttachment {
                key: AttachmentKey::edge_beta(EdgeKey {
                    warp_id,
                    local_id: edge_id,
                }),
                value: after_val.cloned(),
            });
        }
    }
}

fn edges_by_id(store: &GraphStore) -> std::collections::BTreeMap<ContentHash, EdgeRecord> {
    let mut out = std::collections::BTreeMap::new();
    for edges in store.edges_from.values() {
        for e in edges {
            out.insert(e.id.0, e.clone());
        }
    }
    out
}

/// Extracts a Paper III-style slice from a linear worldline payload.
///
/// This function implements the “unversioned slots, interpretive SSA” rule:
/// a slot version is `slot@i`, where `i` is the index of the producing patch
/// in the worldline payload `P = (μ0, …, μn-1)`.
///
/// Returns the set of tick indices (in ascending order) that must be retained
/// to replay the dependency cone for `target` at the end of the worldline.
pub fn slice_worldline_indices(patches: &[WarpTickPatchV1], target: SlotId) -> Vec<usize> {
    // Build a producer index: slot -> sorted list of producing tick indices.
    let mut producers: std::collections::BTreeMap<SlotId, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (tick, patch) in patches.iter().enumerate() {
        for slot in patch.out_slots() {
            producers.entry(*slot).or_default().push(tick);
        }
    }

    let mut needed_ticks: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let mut work: Vec<(SlotId, usize)> = vec![(target, patches.len())];
    let mut seen: std::collections::BTreeSet<(SlotId, usize)> = std::collections::BTreeSet::new();

    while let Some((slot, consumer_tick)) = work.pop() {
        if !seen.insert((slot, consumer_tick)) {
            continue;
        }
        let Some(ticks) = producers.get(&slot) else {
            continue; // Value originates from U0 (or is absent); no producing patch.
        };
        let Some(producer_tick) = producer_before(ticks, consumer_tick) else {
            continue;
        };
        if needed_ticks.insert(producer_tick) {
            for in_slot in patches[producer_tick].in_slots() {
                work.push((*in_slot, producer_tick));
            }
        }
    }

    needed_ticks.into_iter().collect()
}

fn producer_before(producers: &[usize], consumer_tick: usize) -> Option<usize> {
    let insertion = match producers.binary_search(&consumer_tick) {
        Ok(pos) | Err(pos) => pos,
    };
    if insertion == 0 {
        None
    } else {
        Some(producers[insertion - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ident::{make_node_id, make_type_id, make_warp_id};

    #[test]
    fn slice_includes_portal_chain_for_descended_instance() {
        let root_warp = make_warp_id("root");
        let child_warp = make_warp_id("child");

        let root_node = make_node_id("root-node");
        let portal_key = AttachmentKey::node_alpha(NodeKey {
            warp_id: root_warp,
            local_id: root_node,
        });

        let child_root = make_node_id("child-root");
        let child_instance = WarpInstance {
            warp_id: child_warp,
            root_node: child_root,
            parent: Some(portal_key),
        };

        let child_node = make_node_id("child-node");
        let child_node_key = NodeKey {
            warp_id: child_warp,
            local_id: child_node,
        };

        // Tick 0: establish the portal by setting the root attachment slot to Descend(child).
        let patch0 = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![SlotId::Attachment(portal_key)],
            vec![
                WarpOp::UpsertWarpInstance {
                    instance: child_instance,
                },
                WarpOp::UpsertNode {
                    node: NodeKey {
                        warp_id: root_warp,
                        local_id: root_node,
                    },
                    record: NodeRecord {
                        ty: make_type_id("RootTy"),
                    },
                },
                WarpOp::SetAttachment {
                    key: portal_key,
                    value: Some(AttachmentValue::Descend(child_warp)),
                },
            ],
        );

        // Tick 1: execute inside the descended instance; the engine enforces that
        // the descent chain attachment slot is read (in_slots includes portal_key).
        let patch1 = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![SlotId::Attachment(portal_key)],
            vec![SlotId::Node(child_node_key)],
            vec![WarpOp::UpsertNode {
                node: child_node_key,
                record: NodeRecord {
                    ty: make_type_id("ChildTy"),
                },
            }],
        );

        let worldline = [patch0, patch1];
        let ticks = slice_worldline_indices(&worldline, SlotId::Node(child_node_key));
        assert_eq!(ticks, vec![0, 1]);
    }
}
