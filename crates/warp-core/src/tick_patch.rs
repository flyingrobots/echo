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

use crate::attachment::{
    AtomPayload, AttachmentKey, AttachmentOwner, AttachmentPlane, AttachmentValue,
};
use crate::footprint::WarpScopedPortKey;
use crate::graph::GraphStore;
use crate::ident::{EdgeId, EdgeKey, Hash as ContentHash, NodeId, NodeKey, WarpId};
use crate::record::{EdgeRecord, NodeRecord};
use crate::warp_state::{WarpInstance, WarpState};

/// Commit status of a tick patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
///
/// All variants are warp-scoped: they include both `WarpId` and local identifiers.
/// This ensures resources in different warps are tracked distinctly for receipt
/// attribution and provenance bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SlotId {
    /// Full node record at `NodeKey` (warp-scoped skeleton record).
    Node(NodeKey),
    /// Full edge record at `EdgeKey` (warp-scoped skeleton record).
    Edge(EdgeKey),
    /// Attachment slot (node/edge plane payload, including `Descend` links).
    Attachment(AttachmentKey),
    /// Boundary port value (warp-scoped: `(WarpId, PortKey)`).
    Port(WarpScopedPortKey),
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WarpOp {
    /// Open a descended attachment portal atomically (Stage B1.1).
    ///
    /// This is the canonical authoring operation for descended attachments:
    /// it is illegal for a replay/slice to observe a “dangling portal”
    /// (`Descend(child_warp)` without a corresponding `WarpInstance`), or an
    /// “orphan instance” (a `WarpInstance` whose `parent` slot does not point to it).
    ///
    /// Semantics:
    /// - Ensure `WarpInstance(child_warp)` exists with `parent = Some(key)` and
    ///   `root_node = child_root`.
    /// - Ensure the child root node exists (via `init`).
    /// - Set `Attachment[key] = Descend(child_warp)`.
    OpenPortal {
        /// Attachment slot key that will point to the child instance.
        key: AttachmentKey,
        /// Child instance identifier.
        child_warp: WarpId,
        /// Root node id within the child instance.
        child_root: NodeId,
        /// How to initialize/validate the child instance root.
        init: PortalInit,
    },
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

/// Initialization policy for [`WarpOp::OpenPortal`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PortalInit {
    /// Create a new child instance with only a root node (using `root_record`).
    Empty {
        /// Record to use when creating the child root node.
        root_record: NodeRecord,
    },
    /// Require that the child instance and root node already exist.
    RequireExisting,
}

/// Canonical ordering key for [`WarpOp`] used by patch construction and merge sorting.
///
/// This is a compact, byte-stable representation of an op's ordering identity:
/// - `kind` defines the global phase ordering across op variants
/// - `warp`, `a`, and `b` encode the op's target within that phase
///
/// # Invariants
///
/// - Keys are totally ordered and deterministic across runs.
/// - Two ops with identical keys are considered duplicates for deduplication purposes.
/// - The ordering ensures structural dependencies (instances before nodes, deletes before upserts).
///
/// # Usage
///
/// Primarily used internally by [`WarpTickPatchV1::new`] for canonicalization and
/// `merge_deltas` (feature-gated) for deterministic merge ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WarpOpKey {
    kind: u8,
    warp: ContentHash,
    a: ContentHash,
    b: ContentHash,
}

impl WarpOp {
    /// Canonical replay ordering key for this operation.
    ///
    /// This ordering is used for two purposes:
    /// - to define the deterministic replay order of a tick patch, and
    /// - to define which operations are considered "the same" for patch construction
    ///   (see [`WarpTickPatchV1::new`], which dedupes by this key with last-wins semantics).
    ///
    /// Ordering rationale (v2):
    /// - Instance/portal operations sort before per-instance skeleton edits so that stores exist
    ///   before nodes/edges/attachments are applied.
    /// - Skeleton deletions sort before skeleton upserts (delete-before-upsert) to support
    ///   within-tick replacement semantics for nodes/edges.
    /// - Attachment writes sort last so they cannot reference missing skeleton elements.
    ///
    /// Note: `UpsertWarpInstance` sorts before `DeleteWarpInstance` even though node/edge ops
    /// use delete-before-upsert. Patches are expected not to contain both operations for the
    /// same `warp_id`; if they do, this ordering makes the resulting state (and any subsequent
    /// invalid references) deterministic rather than silently ambiguous.
    pub fn sort_key(&self) -> WarpOpKey {
        match self {
            Self::OpenPortal { key, .. } => {
                let (owner_tag, plane_tag) = key.tag();
                let (warp, local) = match key.owner {
                    AttachmentOwner::Node(node) => ((node.warp_id).0, (node.local_id).0),
                    AttachmentOwner::Edge(edge) => ((edge.warp_id).0, (edge.local_id).0),
                };
                WarpOpKey {
                    kind: 1,
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
            Self::UpsertWarpInstance { instance } => WarpOpKey {
                kind: 2,
                warp: (instance.warp_id).0,
                a: (instance.warp_id).0,
                b: [0u8; 32],
            },
            Self::DeleteWarpInstance { warp_id } => WarpOpKey {
                kind: 3,
                warp: warp_id.0,
                a: warp_id.0,
                b: [0u8; 32],
            },
            Self::DeleteEdge {
                warp_id,
                from,
                edge_id,
            } => WarpOpKey {
                kind: 4,
                warp: warp_id.0,
                a: from.0,
                b: edge_id.0,
            },
            Self::DeleteNode { node } => WarpOpKey {
                kind: 5,
                warp: (node.warp_id).0,
                a: (node.local_id).0,
                b: [0u8; 32],
            },
            Self::UpsertNode { node, .. } => WarpOpKey {
                kind: 6,
                warp: (node.warp_id).0,
                a: (node.local_id).0,
                b: [0u8; 32],
            },
            Self::UpsertEdge { warp_id, record } => WarpOpKey {
                kind: 7,
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
                    kind: 8,
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        ops: Vec<WarpOp>,
    ) -> Self {
        in_slots.sort();
        in_slots.dedup();
        out_slots.sort();
        out_slots.dedup();
        let ops = {
            let mut op_map: std::collections::BTreeMap<WarpOpKey, WarpOp> =
                std::collections::BTreeMap::new();
            for op in ops {
                // Last-wins: if multiple ops share the same canonical sort key, keep the
                // last op provided by the caller and drop earlier duplicates.
                op_map.insert(op.sort_key(), op);
            }
            op_map.into_values().collect::<Vec<_>>()
        };
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

    /// Verifies that the internal digest matches the computed digest of the patch contents.
    ///
    /// # Errors
    /// Returns `TickPatchError::DigestMismatch` if the digests do not match.
    pub fn validate_digest(&self) -> Result<(), TickPatchError> {
        let expected = compute_patch_digest_v2(
            self.policy_id,
            &self.rule_pack_id,
            self.commit_status,
            &self.in_slots,
            &self.out_slots,
            &self.ops,
        );
        if self.digest == expected {
            Ok(())
        } else {
            Err(TickPatchError::DigestMismatch)
        }
    }

    /// Applies the patch delta to `state`.
    ///
    /// # Errors
    /// Returns an error if an operation is invalid for the current store
    /// state (e.g., deleting a missing edge).
    pub fn apply_to_state(&self, state: &mut WarpState) -> Result<(), TickPatchError> {
        for op in &self.ops {
            apply_op_to_state(state, op)?;
        }
        validate_portal_invariants(state)?;
        Ok(())
    }
}

fn validate_portal_invariants(state: &WarpState) -> Result<(), TickPatchError> {
    // 1) No orphan instances: every instance that declares a parent slot must be reachable
    // via that slot (`AttachmentValue::Descend(warp_id)`).
    for (warp_id, instance) in state.iter_instances() {
        let Some(parent) = instance.parent else {
            continue;
        };
        // Ensure the parent key is well-formed and its owner exists.
        let _parent_warp = validate_attachment_owner_exists(state, &parent)?;
        match attachment_value_for_key(state, &parent) {
            Some(AttachmentValue::Descend(child_warp)) if *child_warp == *warp_id => {}
            _ => return Err(TickPatchError::PortalInvariantViolation),
        }
    }

    // 2) No dangling portals: every `Descend(warp_id)` attachment must refer to an existing
    // instance whose `parent` points back at this exact attachment slot.
    for (warp_id, store) in state.iter_stores() {
        for (node_id, value) in store.iter_node_attachments() {
            if let AttachmentValue::Descend(child_warp) = value {
                let key = AttachmentKey::node_alpha(NodeKey {
                    warp_id: *warp_id,
                    local_id: *node_id,
                });
                validate_descend_target(state, key, *child_warp)?;
            }
        }
        for (edge_id, value) in store.iter_edge_attachments() {
            if let AttachmentValue::Descend(child_warp) = value {
                let key = AttachmentKey::edge_beta(EdgeKey {
                    warp_id: *warp_id,
                    local_id: *edge_id,
                });
                validate_descend_target(state, key, *child_warp)?;
            }
        }
    }

    Ok(())
}

fn validate_descend_target(
    state: &WarpState,
    key: AttachmentKey,
    child_warp: WarpId,
) -> Result<(), TickPatchError> {
    let Some(child_instance) = state.instance(&child_warp) else {
        return Err(TickPatchError::PortalInvariantViolation);
    };
    if child_instance.parent != Some(key) {
        return Err(TickPatchError::PortalInvariantViolation);
    }
    if state.store(&child_warp).is_none() {
        return Err(TickPatchError::PortalInvariantViolation);
    }
    Ok(())
}

fn apply_op_to_state(state: &mut WarpState, op: &WarpOp) -> Result<(), TickPatchError> {
    match op {
        WarpOp::OpenPortal {
            key,
            child_warp,
            child_root,
            init,
        } => apply_open_portal(state, key, *child_warp, *child_root, init),
        WarpOp::UpsertWarpInstance { instance } => {
            let store = state.take_or_create_store(instance.warp_id);
            state.upsert_instance(instance.clone(), store);
            Ok(())
        }
        WarpOp::DeleteWarpInstance { warp_id } => {
            if !state.delete_instance(warp_id) {
                return Err(TickPatchError::MissingWarp(*warp_id));
            }
            Ok(())
        }
        WarpOp::UpsertNode { node, record } => {
            let Some(store) = state.store_mut(&node.warp_id) else {
                return Err(TickPatchError::MissingWarp(node.warp_id));
            };
            store.insert_node(node.local_id, record.clone());
            Ok(())
        }
        WarpOp::DeleteNode { node } => {
            let Some(store) = state.store_mut(&node.warp_id) else {
                return Err(TickPatchError::MissingWarp(node.warp_id));
            };
            if !store.delete_node_cascade(node.local_id) {
                return Err(TickPatchError::MissingNode(*node));
            }
            Ok(())
        }
        WarpOp::UpsertEdge { warp_id, record } => {
            let Some(store) = state.store_mut(warp_id) else {
                return Err(TickPatchError::MissingWarp(*warp_id));
            };
            store.upsert_edge_record(record.from, record.clone());
            Ok(())
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
            Ok(())
        }
        WarpOp::SetAttachment { key, value } => apply_set_attachment(state, key, value.as_ref()),
    }
}

fn apply_open_portal(
    state: &mut WarpState,
    key: &AttachmentKey,
    child_warp: WarpId,
    child_root: NodeId,
    init: &PortalInit,
) -> Result<(), TickPatchError> {
    let parent_warp = validate_attachment_owner_exists(state, key)?;

    // Ensure the child instance exists and is consistent with the portal key.
    let mut created_instance = false;
    if let Some(existing) = state.instance(&child_warp) {
        if existing.parent != Some(*key) || existing.root_node != child_root {
            return Err(TickPatchError::PortalInvariantViolation);
        }
    } else {
        match init {
            PortalInit::Empty { root_record } => {
                let mut store = GraphStore::new(child_warp);
                store.insert_node(child_root, root_record.clone());
                state.upsert_instance(
                    WarpInstance {
                        warp_id: child_warp,
                        root_node: child_root,
                        parent: Some(*key),
                    },
                    store,
                );
                created_instance = true;
            }
            PortalInit::RequireExisting => return Err(TickPatchError::PortalInitRequired),
        }
    }

    if !created_instance {
        ensure_child_root(state, child_warp, child_root, init)?;
    }

    // Finally, set the portal attachment slot to point at the child warp id.
    let Some(parent_store) = state.store_mut(&parent_warp) else {
        return Err(TickPatchError::MissingWarp(parent_warp));
    };
    match key.owner {
        AttachmentOwner::Node(node) => {
            parent_store
                .set_node_attachment(node.local_id, Some(AttachmentValue::Descend(child_warp)));
        }
        AttachmentOwner::Edge(edge) => {
            parent_store
                .set_edge_attachment(edge.local_id, Some(AttachmentValue::Descend(child_warp)));
        }
    }
    Ok(())
}

fn validate_attachment_plane(key: &AttachmentKey) -> Result<(), TickPatchError> {
    match key.owner {
        AttachmentOwner::Node(_) => {
            if key.plane != AttachmentPlane::Alpha {
                return Err(TickPatchError::InvalidAttachmentKey(*key));
            }
        }
        AttachmentOwner::Edge(_) => {
            if key.plane != AttachmentPlane::Beta {
                return Err(TickPatchError::InvalidAttachmentKey(*key));
            }
        }
    }
    Ok(())
}

fn validate_attachment_owner_exists(
    state: &WarpState,
    key: &AttachmentKey,
) -> Result<WarpId, TickPatchError> {
    validate_attachment_plane(key)?;
    match key.owner {
        AttachmentOwner::Node(node) => {
            let Some(store) = state.store(&node.warp_id) else {
                return Err(TickPatchError::MissingWarp(node.warp_id));
            };
            if store.node(&node.local_id).is_none() {
                return Err(TickPatchError::MissingNode(node));
            }
            Ok(node.warp_id)
        }
        AttachmentOwner::Edge(edge) => {
            let Some(store) = state.store(&edge.warp_id) else {
                return Err(TickPatchError::MissingWarp(edge.warp_id));
            };
            if !store.has_edge(&edge.local_id) {
                return Err(TickPatchError::MissingEdge(edge));
            }
            Ok(edge.warp_id)
        }
    }
}

fn ensure_child_root(
    state: &mut WarpState,
    child_warp: WarpId,
    child_root: NodeId,
    init: &PortalInit,
) -> Result<(), TickPatchError> {
    match init {
        PortalInit::Empty { root_record } => {
            let Some(store) = state.store_mut(&child_warp) else {
                return Err(TickPatchError::MissingWarp(child_warp));
            };
            match store.node(&child_root) {
                None => {
                    store.insert_node(child_root, root_record.clone());
                }
                Some(existing) => {
                    if existing != root_record {
                        return Err(TickPatchError::PortalInvariantViolation);
                    }
                }
            }
            Ok(())
        }
        PortalInit::RequireExisting => {
            let Some(store) = state.store(&child_warp) else {
                return Err(TickPatchError::MissingWarp(child_warp));
            };
            if store.node(&child_root).is_none() {
                return Err(TickPatchError::MissingNode(NodeKey {
                    warp_id: child_warp,
                    local_id: child_root,
                }));
            }
            Ok(())
        }
    }
}

fn apply_set_attachment(
    state: &mut WarpState,
    key: &AttachmentKey,
    value: Option<&AttachmentValue>,
) -> Result<(), TickPatchError> {
    validate_attachment_plane(key)?;
    match key.owner {
        AttachmentOwner::Node(node) => {
            let Some(store) = state.store_mut(&node.warp_id) else {
                return Err(TickPatchError::MissingWarp(node.warp_id));
            };
            if store.node(&node.local_id).is_none() {
                return Err(TickPatchError::MissingNode(node));
            }
            store.set_node_attachment(node.local_id, value.cloned());
            Ok(())
        }
        AttachmentOwner::Edge(edge) => {
            let Some(store) = state.store_mut(&edge.warp_id) else {
                return Err(TickPatchError::MissingWarp(edge.warp_id));
            };
            if !store.has_edge(&edge.local_id) {
                return Err(TickPatchError::MissingEdge(edge));
            }
            store.set_edge_attachment(edge.local_id, value.cloned());
            Ok(())
        }
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
    /// `OpenPortal` requires `PortalInit::Empty` when the child instance does not exist.
    #[error("portal init required")]
    PortalInitRequired,
    /// `OpenPortal` invariants were violated (dangling portal / inconsistent parent chain / root mismatch).
    #[error("portal invariant violation")]
    PortalInvariantViolation,
    /// The patch digest did not match its contents.
    #[error("patch digest mismatch")]
    DigestMismatch,
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
            SlotId::Port((warp_id, port_key)) => {
                h.update(&[4u8]);
                h.update(&warp_id.0);
                h.update(&port_key.to_le_bytes());
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
            WarpOp::OpenPortal {
                key,
                child_warp,
                child_root,
                init,
            } => {
                h.update(&[8u8]);
                encode_attachment_key(h, key);
                h.update(&child_warp.0);
                h.update(&child_root.0);
                encode_portal_init(h, init);
            }
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

fn encode_portal_init(h: &mut Hasher, init: &PortalInit) {
    match init {
        PortalInit::RequireExisting => {
            h.update(&[0u8]);
        }
        PortalInit::Empty { root_record } => {
            h.update(&[1u8]);
            h.update(&(root_record.ty).0);
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
    let mut portal_warps: std::collections::BTreeSet<WarpId> = std::collections::BTreeSet::new();
    let mut skip_node_upserts: std::collections::BTreeSet<NodeKey> =
        std::collections::BTreeSet::new();
    let mut skip_attachment_ops: std::collections::BTreeSet<AttachmentKey> =
        std::collections::BTreeSet::new();

    // Canonicalize portal authoring: when we observe a new descended instance that is
    // linked via `parent` and whose parent slot is set to `Descend(child_warp)`,
    // emit a single `OpenPortal` op instead of separate instance + attachment edits.
    //
    // This prevents slices from “forgetting the baby exists” by ensuring that
    // portal creation and instance creation are inseparable at the patch level.
    for (warp_id, inst_after) in &after.instances {
        if before.instances.contains_key(warp_id) {
            continue;
        }
        let Some(parent_key) = inst_after.parent else {
            continue;
        };
        let Some(parent_value) = attachment_value_for_key(after, &parent_key) else {
            continue;
        };
        if parent_value != &AttachmentValue::Descend(*warp_id) {
            continue;
        }
        let Some(child_store) = after.store(warp_id) else {
            continue;
        };
        let Some(root_record) = child_store.node(&inst_after.root_node) else {
            continue;
        };
        ops.push(WarpOp::OpenPortal {
            key: parent_key,
            child_warp: *warp_id,
            child_root: inst_after.root_node,
            init: PortalInit::Empty {
                root_record: root_record.clone(),
            },
        });
        portal_warps.insert(*warp_id);
        skip_node_upserts.insert(NodeKey {
            warp_id: *warp_id,
            local_id: inst_after.root_node,
        });
        skip_attachment_ops.insert(parent_key);
    }

    // WarpInstances: deletions and upserts.
    for warp_id in before.instances.keys() {
        if !after.instances.contains_key(warp_id) {
            ops.push(WarpOp::DeleteWarpInstance { warp_id: *warp_id });
        }
    }
    for (warp_id, inst_after) in &after.instances {
        match before.instances.get(warp_id) {
            None => {
                if !portal_warps.contains(warp_id) {
                    ops.push(WarpOp::UpsertWarpInstance {
                        instance: inst_after.clone(),
                    });
                }
            }
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
        diff_instance(
            &mut ops,
            *warp_id,
            before_store,
            after_store,
            &skip_node_upserts,
            &skip_attachment_ops,
        );
    }

    ops.sort_by_key(WarpOp::sort_key);
    ops
}

fn diff_instance(
    ops: &mut Vec<WarpOp>,
    warp_id: WarpId,
    before: &GraphStore,
    after: &GraphStore,
    skip_node_upserts: &std::collections::BTreeSet<NodeKey>,
    skip_attachment_ops: &std::collections::BTreeSet<AttachmentKey>,
) {
    diff_nodes(ops, warp_id, before, after, skip_node_upserts);
    diff_node_attachments(ops, warp_id, before, after, skip_attachment_ops);

    // Edges (skeleton plane): map by EdgeId for stable diff independent of insertion order.
    let before_edges = edges_by_id(before);
    let after_edges = edges_by_id(after);
    diff_edges(ops, warp_id, &before_edges, &after_edges);
    diff_edge_attachments(
        ops,
        warp_id,
        before,
        after,
        &after_edges,
        skip_attachment_ops,
    );
}

fn diff_nodes(
    ops: &mut Vec<WarpOp>,
    warp_id: WarpId,
    before: &GraphStore,
    after: &GraphStore,
    skip_node_upserts: &std::collections::BTreeSet<NodeKey>,
) {
    for (id, rec_before) in &before.nodes {
        let node = NodeKey {
            warp_id,
            local_id: *id,
        };
        if skip_node_upserts.contains(&node) {
            continue;
        }
        let Some(rec_after) = after.nodes.get(id) else {
            ops.push(WarpOp::DeleteNode { node });
            continue;
        };
        if rec_before != rec_after {
            ops.push(WarpOp::UpsertNode {
                node,
                record: rec_after.clone(),
            });
        }
    }
    for (id, rec_after) in &after.nodes {
        let node = NodeKey {
            warp_id,
            local_id: *id,
        };
        if skip_node_upserts.contains(&node) {
            continue;
        }
        if !before.nodes.contains_key(id) {
            ops.push(WarpOp::UpsertNode {
                node,
                record: rec_after.clone(),
            });
        }
    }
}

fn diff_node_attachments(
    ops: &mut Vec<WarpOp>,
    warp_id: WarpId,
    before: &GraphStore,
    after: &GraphStore,
    skip_attachment_ops: &std::collections::BTreeSet<AttachmentKey>,
) {
    for node_id in after.nodes.keys() {
        let before_val = before.node_attachment(node_id);
        let after_val = after.node_attachment(node_id);
        if before_val == after_val {
            continue;
        }

        let key = AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: *node_id,
        });
        if skip_attachment_ops.contains(&key) {
            continue;
        }
        ops.push(WarpOp::SetAttachment {
            key,
            value: after_val.cloned(),
        });
    }
}

fn diff_edges(
    ops: &mut Vec<WarpOp>,
    warp_id: WarpId,
    before_edges: &std::collections::BTreeMap<ContentHash, EdgeRecord>,
    after_edges: &std::collections::BTreeMap<ContentHash, EdgeRecord>,
) {
    for (id, rec_before) in before_edges {
        if !after_edges.contains_key(id) {
            ops.push(WarpOp::DeleteEdge {
                warp_id,
                from: rec_before.from,
                edge_id: EdgeId(*id),
            });
        }
    }

    for (id, rec_after) in after_edges {
        match before_edges.get(id) {
            None => {
                ops.push(WarpOp::UpsertEdge {
                    warp_id,
                    record: rec_after.clone(),
                });
            }
            Some(rec_before) => {
                if rec_before == rec_after {
                    continue;
                }
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

fn diff_edge_attachments(
    ops: &mut Vec<WarpOp>,
    warp_id: WarpId,
    before: &GraphStore,
    after: &GraphStore,
    after_edges: &std::collections::BTreeMap<ContentHash, EdgeRecord>,
    skip_attachment_ops: &std::collections::BTreeSet<AttachmentKey>,
) {
    for id in after_edges.keys() {
        let edge_id = EdgeId(*id);
        let before_val = before.edge_attachment(&edge_id);
        let after_val = after.edge_attachment(&edge_id);
        if before_val == after_val {
            continue;
        }

        let key = AttachmentKey::edge_beta(EdgeKey {
            warp_id,
            local_id: edge_id,
        });
        if skip_attachment_ops.contains(&key) {
            continue;
        }
        ops.push(WarpOp::SetAttachment {
            key,
            value: after_val.cloned(),
        });
    }
}

fn attachment_value_for_key<'a>(
    state: &'a WarpState,
    key: &AttachmentKey,
) -> Option<&'a AttachmentValue> {
    match key.owner {
        AttachmentOwner::Node(node) => state.store(&node.warp_id)?.node_attachment(&node.local_id),
        AttachmentOwner::Edge(edge) => state.store(&edge.warp_id)?.edge_attachment(&edge.local_id),
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
    let work_cap = core::cmp::max(1, patches.len() / 4);
    let mut work: Vec<(SlotId, usize)> = Vec::with_capacity(work_cap);
    work.push((target, patches.len()));
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
    fn new_dedupes_duplicate_ops_by_sort_key_last_wins() {
        let warp_id = make_warp_id("dedupe-warp");
        let node_id = make_node_id("dedupe-node");
        let node = NodeKey {
            warp_id,
            local_id: node_id,
        };

        let op_a = WarpOp::UpsertNode {
            node,
            record: NodeRecord {
                ty: make_type_id("A"),
            },
        };
        let op_b = WarpOp::UpsertNode {
            node,
            record: NodeRecord {
                ty: make_type_id("B"),
            },
        };

        let patch = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![],
            vec![op_a, op_b],
        );

        let ops = patch.ops();
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], WarpOp::UpsertNode { .. }));
        let WarpOp::UpsertNode {
            node: got_node,
            record,
        } = &ops[0]
        else {
            return;
        };
        assert_eq!(got_node, &node);
        assert_eq!(record.ty, make_type_id("B"));
    }

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
        let child_root_key = NodeKey {
            warp_id: child_warp,
            local_id: child_root,
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
            vec![SlotId::Attachment(portal_key), SlotId::Node(child_root_key)],
            vec![WarpOp::OpenPortal {
                key: portal_key,
                child_warp,
                child_root,
                init: PortalInit::Empty {
                    root_record: NodeRecord {
                        ty: make_type_id("ChildRootTy"),
                    },
                },
            }],
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

    #[test]
    fn apply_to_state_rejects_dangling_portal_set_attachment() {
        let root_warp = make_warp_id("root");
        let child_warp = make_warp_id("child");

        let root_node = make_node_id("root-node");
        let root_key = NodeKey {
            warp_id: root_warp,
            local_id: root_node,
        };
        let portal_key = AttachmentKey::node_alpha(root_key);

        let mut state = WarpState::new();
        let init_root = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![SlotId::Node(root_key)],
            vec![
                WarpOp::UpsertWarpInstance {
                    instance: WarpInstance {
                        warp_id: root_warp,
                        root_node,
                        parent: None,
                    },
                },
                WarpOp::UpsertNode {
                    node: root_key,
                    record: NodeRecord {
                        ty: make_type_id("RootTy"),
                    },
                },
            ],
        );
        assert!(init_root.apply_to_state(&mut state).is_ok());

        // Dangling portal: sets Descend(child) without creating the child instance.
        let dangling = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![SlotId::Attachment(portal_key)],
            vec![WarpOp::SetAttachment {
                key: portal_key,
                value: Some(AttachmentValue::Descend(child_warp)),
            }],
        );
        assert!(matches!(
            dangling.apply_to_state(&mut state),
            Err(TickPatchError::PortalInvariantViolation)
        ));
    }

    #[test]
    fn apply_to_state_rejects_orphan_instance_missing_portal() {
        let root_warp = make_warp_id("root");
        let child_warp = make_warp_id("child");

        let root_node = make_node_id("root-node");
        let root_key = NodeKey {
            warp_id: root_warp,
            local_id: root_node,
        };
        let portal_key = AttachmentKey::node_alpha(root_key);

        let mut state = WarpState::new();
        let init_root = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![SlotId::Node(root_key)],
            vec![
                WarpOp::UpsertWarpInstance {
                    instance: WarpInstance {
                        warp_id: root_warp,
                        root_node,
                        parent: None,
                    },
                },
                WarpOp::UpsertNode {
                    node: root_key,
                    record: NodeRecord {
                        ty: make_type_id("RootTy"),
                    },
                },
            ],
        );
        assert!(init_root.apply_to_state(&mut state).is_ok());

        // Orphan instance: create child metadata that declares a parent portal key
        // without establishing the portal slot value.
        let orphan = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![],
            vec![WarpOp::UpsertWarpInstance {
                instance: WarpInstance {
                    warp_id: child_warp,
                    root_node: make_node_id("child-root"),
                    parent: Some(portal_key),
                },
            }],
        );
        assert!(matches!(
            orphan.apply_to_state(&mut state),
            Err(TickPatchError::PortalInvariantViolation)
        ));
    }

    #[test]
    fn apply_to_state_rejects_delete_instance_without_clearing_portal() {
        let root_warp = make_warp_id("root");
        let child_warp = make_warp_id("child");

        let root_node = make_node_id("root-node");
        let root_key = NodeKey {
            warp_id: root_warp,
            local_id: root_node,
        };
        let portal_key = AttachmentKey::node_alpha(root_key);

        let child_root = make_node_id("child-root");

        let mut state = WarpState::new();
        let init_root = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![SlotId::Node(root_key)],
            vec![
                WarpOp::UpsertWarpInstance {
                    instance: WarpInstance {
                        warp_id: root_warp,
                        root_node,
                        parent: None,
                    },
                },
                WarpOp::UpsertNode {
                    node: root_key,
                    record: NodeRecord {
                        ty: make_type_id("RootTy"),
                    },
                },
            ],
        );
        assert!(init_root.apply_to_state(&mut state).is_ok());

        let open = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![SlotId::Attachment(portal_key)],
            vec![WarpOp::OpenPortal {
                key: portal_key,
                child_warp,
                child_root,
                init: PortalInit::Empty {
                    root_record: NodeRecord {
                        ty: make_type_id("ChildRootTy"),
                    },
                },
            }],
        );
        assert!(open.apply_to_state(&mut state).is_ok());

        // Dangling portal: delete the child instance but forget to clear the portal slot.
        let delete_child = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![],
            vec![WarpOp::DeleteWarpInstance {
                warp_id: child_warp,
            }],
        );
        assert!(matches!(
            delete_child.apply_to_state(&mut state),
            Err(TickPatchError::PortalInvariantViolation)
        ));
    }

    #[test]
    fn apply_to_state_rejects_clear_portal_without_deleting_instance() {
        let root_warp = make_warp_id("root");
        let child_warp = make_warp_id("child");

        let root_node = make_node_id("root-node");
        let root_key = NodeKey {
            warp_id: root_warp,
            local_id: root_node,
        };
        let portal_key = AttachmentKey::node_alpha(root_key);

        let child_root = make_node_id("child-root");

        let mut state = WarpState::new();
        let init_root = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![SlotId::Node(root_key)],
            vec![
                WarpOp::UpsertWarpInstance {
                    instance: WarpInstance {
                        warp_id: root_warp,
                        root_node,
                        parent: None,
                    },
                },
                WarpOp::UpsertNode {
                    node: root_key,
                    record: NodeRecord {
                        ty: make_type_id("RootTy"),
                    },
                },
            ],
        );
        assert!(init_root.apply_to_state(&mut state).is_ok());

        let open = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![SlotId::Attachment(portal_key)],
            vec![WarpOp::OpenPortal {
                key: portal_key,
                child_warp,
                child_root,
                init: PortalInit::Empty {
                    root_record: NodeRecord {
                        ty: make_type_id("ChildRootTy"),
                    },
                },
            }],
        );
        assert!(open.apply_to_state(&mut state).is_ok());

        // Orphan instance: clear the portal slot while leaving the child instance metadata present.
        let clear_portal = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            [1u8; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![],
            vec![WarpOp::SetAttachment {
                key: portal_key,
                value: None,
            }],
        );
        assert!(matches!(
            clear_portal.apply_to_state(&mut state),
            Err(TickPatchError::PortalInvariantViolation)
        ));
    }

    #[test]
    fn warp_op_key_distinguishes_by_warp() {
        use std::collections::BTreeSet;

        let warp_a = make_warp_id("warp-a");
        let warp_b = make_warp_id("warp-b");
        let same_node = make_node_id("same-node");

        // Create two UpsertNode ops targeting the same local node but different warps
        let op_a = WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_a,
                local_id: same_node,
            },
            record: NodeRecord {
                ty: make_type_id("test"),
            },
        };
        let op_b = WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_b,
                local_id: same_node,
            },
            record: NodeRecord {
                ty: make_type_id("test"),
            },
        };

        let key_a = op_a.sort_key();
        let key_b = op_b.sort_key();

        // Keys must be distinct
        assert_ne!(key_a, key_b, "WarpOpKey must distinguish different warps");

        // Keys must have total order
        assert!(
            key_a < key_b || key_b < key_a,
            "WarpOpKey must have total order"
        );

        // No collision in sets
        let mut set = BTreeSet::new();
        set.insert(key_a);
        set.insert(key_b);
        assert_eq!(set.len(), 2, "WarpOpKeys must not collide in BTreeSet");
    }
}
