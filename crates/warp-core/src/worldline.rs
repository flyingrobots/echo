// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Worldline types for SPEC-0004: Worldlines, Playback, and `TruthBus`.
//!
//! A worldline is a linear sequence of tick patches representing the history of a
//! single warp instance. Each tick produces a [`WorldlineTickPatchV1`] that captures
//! the delta operations, slot dependencies, and cryptographic commitments needed
//! for deterministic replay and verification.
//!
//! # Key Types
//!
//! - [`WorldlineId`]: Unique identifier for a worldline (derived from initial state hash).
//! - [`HashTriplet`]: Three-way commitment for verification (state root, patch digest, commit hash).
//! - [`WorldlineTickHeaderV1`]: Shared header metadata across all warps for a global tick.
//! - [`WorldlineTickPatchV1`]: Per-warp projection of a global tick with ops and slot dependencies.
//! - [`OutputFrameSet`]: Recorded channel outputs for a tick.

use thiserror::Error;

use crate::attachment::{AttachmentKey, AttachmentOwner, AttachmentPlane, AttachmentValue};
use crate::graph::GraphStore;
use crate::ident::{EdgeKey, Hash, NodeKey, WarpId};
use crate::materialization::ChannelId;
use crate::tick_patch::{SlotId, WarpOp};

/// Unique identifier for a worldline.
///
/// A worldline ID is typically derived from the initial state hash of the warp,
/// ensuring that worldlines with different starting points have distinct IDs.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldlineId(pub Hash);

impl WorldlineId {
    /// Returns the canonical byte representation of this id.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Three-way cryptographic commitment for worldline verification.
///
/// This triplet commits to the state, the patch that produced it, and the
/// overall commit hash. Cursors verify all three values match after seeking
/// to ensure replay integrity.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HashTriplet {
    /// Hash of the resulting state after applying the patch.
    pub state_root: Hash,
    /// Digest of the patch contents (ops, slots, policy).
    pub patch_digest: Hash,
    /// Overall commit hash for this tick (includes state root and other metadata).
    pub commit_hash: Hash,
}

/// Shared header metadata for a global tick.
///
/// This header contains information that is common across all warps for a given
/// global tick. It captures the policy, rule pack, and digests of planning and
/// decision phases.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldlineTickHeaderV1 {
    /// Global tick number (monotonically increasing).
    pub global_tick: u64,
    /// Policy identifier governing this tick.
    pub policy_id: u32,
    /// Hash identifying the rule pack used for this tick.
    pub rule_pack_id: Hash,
    /// Digest of the execution plan (scheduler output).
    pub plan_digest: Hash,
    /// Digest of the decision phase (which rules fired).
    pub decision_digest: Hash,
    /// Digest of the rewrites phase (deltas produced).
    pub rewrites_digest: Hash,
}

/// Per-warp projection of a global tick.
///
/// This is the primary artifact for worldline replay. It contains:
/// - The shared header metadata
/// - The warp-specific operations (filtered from the global ops)
/// - Input and output slot dependencies for slicing
/// - The patch digest for verification
///
/// Unlike [`WarpTickPatchV1`](crate::tick_patch::WarpTickPatchV1) which is the
/// engine's internal format, this type is designed for external worldline storage
/// and includes the header context needed for independent replay.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldlineTickPatchV1 {
    /// Shared tick header metadata.
    pub header: WorldlineTickHeaderV1,
    /// The warp this patch applies to.
    pub warp_id: WarpId,
    /// Canonical delta operations for this warp during this tick.
    pub ops: Vec<WarpOp>,
    /// Slots read by this tick (conservative set).
    pub in_slots: Vec<SlotId>,
    /// Slots produced by this tick.
    pub out_slots: Vec<SlotId>,
    /// Canonical digest of the patch contents.
    pub patch_digest: Hash,
}

impl WorldlineTickPatchV1 {
    /// Returns the global tick number from the header.
    #[inline]
    #[must_use]
    pub fn global_tick(&self) -> u64 {
        self.header.global_tick
    }

    /// Returns the policy ID from the header.
    #[inline]
    #[must_use]
    pub fn policy_id(&self) -> u32 {
        self.header.policy_id
    }

    /// Returns the rule pack ID from the header.
    #[inline]
    #[must_use]
    pub fn rule_pack_id(&self) -> Hash {
        self.header.rule_pack_id
    }

    /// Apply this patch to a warp-local [`GraphStore`].
    ///
    /// This method applies all operations in the patch to the provided store.
    /// The store must be for the same warp as this patch.
    ///
    /// # Errors
    ///
    /// Returns [`ApplyError`] if:
    /// - The store's warp ID doesn't match the patch's warp ID
    /// - An operation references a missing node or edge
    /// - An operation is not supported for warp-local replay (e.g., portal operations)
    ///
    /// # Note
    ///
    /// This is designed for replay scenarios where the cursor applies recorded
    /// patches to reconstruct historical state. It does NOT execute rules or
    /// create new warp instances.
    pub fn apply_to_store(&self, store: &mut GraphStore) -> Result<(), ApplyError> {
        // Verify warp ID matches
        if store.warp_id() != self.warp_id {
            return Err(ApplyError::WarpMismatch {
                expected: store.warp_id(),
                actual: self.warp_id,
            });
        }

        for op in &self.ops {
            apply_warp_op_to_store(store, op)?;
        }
        Ok(())
    }
}

/// Apply a single [`WarpOp`] to a warp-local [`GraphStore`].
///
/// This function handles ALL `WarpOp` variants explicitly - either applying
/// them or returning a typed error for unsupported operations.
///
/// # Supported Operations
///
/// - `UpsertNode`: Insert or replace a node record
/// - `DeleteNode`: Delete a node and cascade to edges/attachments
/// - `UpsertEdge`: Insert or replace an edge record
/// - `DeleteEdge`: Delete an edge record
/// - `SetAttachment`: Set or clear a node/edge attachment
///
/// # Unsupported Operations (for warp-local replay)
///
/// - `OpenPortal`: Requires multi-warp instance management
/// - `UpsertWarpInstance`: Requires multi-warp instance management
/// - `DeleteWarpInstance`: Requires multi-warp instance management
///
/// # Errors
///
/// Returns [`ApplyError`] if:
/// - The operation targets a different warp than the store
/// - The operation references a missing node or edge
/// - The operation is not supported for warp-local replay
pub(crate) fn apply_warp_op_to_store(
    store: &mut GraphStore,
    op: &WarpOp,
) -> Result<(), ApplyError> {
    let store_warp = store.warp_id();

    match op {
        WarpOp::OpenPortal { .. } => {
            // Portal operations require multi-warp state management.
            // Warp-local replay cannot handle instance creation/linking.
            Err(ApplyError::UnsupportedOperation {
                op_name: "OpenPortal",
            })
        }

        WarpOp::UpsertWarpInstance { .. } => {
            // Instance metadata operations require WarpState, not a single store.
            Err(ApplyError::UnsupportedOperation {
                op_name: "UpsertWarpInstance",
            })
        }

        WarpOp::DeleteWarpInstance { .. } => {
            // Instance deletion requires WarpState, not a single store.
            Err(ApplyError::UnsupportedOperation {
                op_name: "DeleteWarpInstance",
            })
        }

        WarpOp::UpsertNode { node, record } => {
            if node.warp_id != store_warp {
                return Err(ApplyError::WarpMismatch {
                    expected: store_warp,
                    actual: node.warp_id,
                });
            }
            store.insert_node(node.local_id, record.clone());
            Ok(())
        }

        WarpOp::DeleteNode { node } => {
            if node.warp_id != store_warp {
                return Err(ApplyError::WarpMismatch {
                    expected: store_warp,
                    actual: node.warp_id,
                });
            }
            if !store.delete_node_cascade(node.local_id) {
                return Err(ApplyError::MissingNode(*node));
            }
            Ok(())
        }

        WarpOp::UpsertEdge { warp_id, record } => {
            if *warp_id != store_warp {
                return Err(ApplyError::WarpMismatch {
                    expected: store_warp,
                    actual: *warp_id,
                });
            }
            // Verify both endpoint nodes exist before inserting the edge
            if store.node(&record.from).is_none() {
                return Err(ApplyError::MissingNode(NodeKey {
                    warp_id: *warp_id,
                    local_id: record.from,
                }));
            }
            if store.node(&record.to).is_none() {
                return Err(ApplyError::MissingNode(NodeKey {
                    warp_id: *warp_id,
                    local_id: record.to,
                }));
            }
            store.upsert_edge_record(record.from, record.clone());
            Ok(())
        }

        WarpOp::DeleteEdge {
            warp_id,
            from,
            edge_id,
        } => {
            if *warp_id != store_warp {
                return Err(ApplyError::WarpMismatch {
                    expected: store_warp,
                    actual: *warp_id,
                });
            }
            if !store.delete_edge_exact(*from, *edge_id) {
                return Err(ApplyError::MissingEdge(EdgeKey {
                    warp_id: *warp_id,
                    local_id: *edge_id,
                }));
            }
            Ok(())
        }

        WarpOp::SetAttachment { key, value } => {
            apply_set_attachment(store, store_warp, key, value.clone())
        }
    }
}

/// Applies a `SetAttachment` op to a store, validating plane and existence.
fn apply_set_attachment(
    store: &mut GraphStore,
    store_warp: WarpId,
    key: &AttachmentKey,
    value: Option<AttachmentValue>,
) -> Result<(), ApplyError> {
    match key.owner {
        AttachmentOwner::Node(node_key) => {
            if key.plane != AttachmentPlane::Alpha {
                return Err(ApplyError::InvalidAttachmentKey);
            }
            if node_key.warp_id != store_warp {
                return Err(ApplyError::WarpMismatch {
                    expected: store_warp,
                    actual: node_key.warp_id,
                });
            }
            if store.node(&node_key.local_id).is_none() {
                return Err(ApplyError::MissingNode(node_key));
            }
            store.set_node_attachment(node_key.local_id, value);
            Ok(())
        }
        AttachmentOwner::Edge(edge_key) => {
            if key.plane != AttachmentPlane::Beta {
                return Err(ApplyError::InvalidAttachmentKey);
            }
            if edge_key.warp_id != store_warp {
                return Err(ApplyError::WarpMismatch {
                    expected: store_warp,
                    actual: edge_key.warp_id,
                });
            }
            if !store.has_edge(&edge_key.local_id) {
                return Err(ApplyError::MissingEdge(edge_key));
            }
            store.set_edge_attachment(edge_key.local_id, value);
            Ok(())
        }
    }
}

/// Recorded channel outputs for a single tick.
///
/// This captures all materialization bus emissions for a tick, keyed by channel ID.
/// During playback, these outputs are replayed byte-for-byte to ensure truth
/// frame consistency.
///
/// # Structure
///
/// Each entry is a `(ChannelId, Vec<u8>)` pair where:
/// - [`ChannelId`] identifies the materialization channel that emitted the frame.
/// - `Vec<u8>` is the serialized truth frame payload (codec-dependent).
///
/// The vector is ordered by emission sequence within the tick. An empty
/// `OutputFrameSet` means no channels emitted during that tick.
///
/// # Usage
///
/// This is a type alias rather than a newtype to avoid friction at call sites
/// that build, iterate, or destructure the inner `Vec` directly. If stronger
/// type safety is needed in the future, this can be promoted to a newtype
/// wrapper with a `Deref<Target = [(ChannelId, Vec<u8>)]>` impl.
pub type OutputFrameSet = Vec<(ChannelId, Vec<u8>)>;

/// Errors produced while applying a worldline patch to a warp-local store.
///
/// These errors indicate structural violations when replaying patches:
/// - Missing nodes/edges that were expected to exist
/// - Missing warp instance that was expected to exist
/// - Unsupported operations for warp-local replay
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApplyError {
    /// Referenced a node that did not exist in the store.
    #[error("missing node: {0:?}")]
    MissingNode(NodeKey),

    /// Referenced an edge that did not exist in the store.
    #[error("missing edge: {0:?}")]
    MissingEdge(EdgeKey),

    /// Operation targets a different warp than the store.
    ///
    /// Warp-local apply only handles ops for the store's own warp.
    #[error("warp mismatch: expected {expected:?}, got {actual:?}")]
    WarpMismatch {
        /// The warp ID of the store.
        expected: WarpId,
        /// The warp ID from the operation.
        actual: WarpId,
    },

    /// Operation type is not supported for warp-local replay.
    ///
    /// Some operations (like `OpenPortal`, `UpsertWarpInstance`, `DeleteWarpInstance`)
    /// require multi-warp state management and cannot be applied to a single
    /// warp-local store.
    ///
    /// `op_name` is a `&'static str` rather than an enum because [`WarpOp`] variants
    /// are open-ended (new ops may be added across crate versions) and this field is
    /// used only for diagnostic/debug messages. A dedicated enum would couple this
    /// error type to the full set of unsupported variants and require updating in
    /// lockstep with `WarpOp`, adding maintenance burden with no runtime benefit.
    #[error("unsupported operation for warp-local apply: {op_name}")]
    UnsupportedOperation {
        /// Name of the unsupported operation variant (debug-only, not matched on).
        op_name: &'static str,
    },

    /// Invalid attachment key (wrong plane for owner type).
    #[error("invalid attachment key: node owners use Alpha plane, edge owners use Beta plane")]
    InvalidAttachmentKey,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::redundant_clone)]

    use super::*;
    use crate::attachment::AttachmentValue;
    use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id};
    use crate::record::{EdgeRecord, NodeRecord};
    use crate::tick_patch::PortalInit;

    #[test]
    fn worldline_id_is_transparent_wrapper() {
        let hash = [42u8; 32];
        let id = WorldlineId(hash);
        assert_eq!(id.0, hash);
        assert_eq!(id.as_bytes(), &hash);
    }

    #[test]
    fn hash_triplet_equality() {
        let triplet1 = HashTriplet {
            state_root: [1u8; 32],
            patch_digest: [2u8; 32],
            commit_hash: [3u8; 32],
        };
        let triplet2 = HashTriplet {
            state_root: [1u8; 32],
            patch_digest: [2u8; 32],
            commit_hash: [3u8; 32],
        };
        assert_eq!(triplet1, triplet2);
    }

    #[test]
    fn worldline_tick_patch_accessors() {
        let header = WorldlineTickHeaderV1 {
            global_tick: 42,
            policy_id: 1,
            rule_pack_id: [0u8; 32],
            plan_digest: [1u8; 32],
            decision_digest: [2u8; 32],
            rewrites_digest: [3u8; 32],
        };
        let patch = WorldlineTickPatchV1 {
            header,
            warp_id: crate::ident::WarpId([0u8; 32]),
            ops: vec![],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [4u8; 32],
        };
        assert_eq!(patch.global_tick(), 42);
        assert_eq!(patch.policy_id(), 1);
    }

    fn test_header() -> WorldlineTickHeaderV1 {
        WorldlineTickHeaderV1 {
            global_tick: 0,
            policy_id: 0,
            rule_pack_id: [0u8; 32],
            plan_digest: [0u8; 32],
            decision_digest: [0u8; 32],
            rewrites_digest: [0u8; 32],
        }
    }

    #[test]
    fn apply_to_store_upsert_node() {
        let warp_id = make_warp_id("test-warp");
        let mut store = GraphStore::new(warp_id);
        let node_id = make_node_id("node-1");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };
        let ty = make_type_id("TestType");

        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::UpsertNode {
                node: node_key,
                record: NodeRecord { ty },
            }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        assert!(store.node(&node_id).is_none());
        patch.apply_to_store(&mut store).expect("apply failed");
        assert!(store.node(&node_id).is_some());
        assert_eq!(store.node(&node_id).map(|n| n.ty), Some(ty));
    }

    #[test]
    fn apply_to_store_delete_node() {
        let warp_id = make_warp_id("test-warp");
        let mut store = GraphStore::new(warp_id);
        let node_id = make_node_id("node-1");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };
        let ty = make_type_id("TestType");

        // First insert the node
        store.insert_node(node_id, NodeRecord { ty });
        assert!(store.node(&node_id).is_some());

        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::DeleteNode { node: node_key }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        patch.apply_to_store(&mut store).expect("apply failed");
        assert!(store.node(&node_id).is_none());
    }

    #[test]
    fn apply_to_store_delete_missing_node_fails() {
        let warp_id = make_warp_id("test-warp");
        let mut store = GraphStore::new(warp_id);
        let node_id = make_node_id("missing-node");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };

        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::DeleteNode { node: node_key }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        let result = patch.apply_to_store(&mut store);
        assert!(matches!(result, Err(ApplyError::MissingNode(_))));
    }

    #[test]
    fn apply_to_store_upsert_and_delete_edge() {
        let warp_id = make_warp_id("test-warp");
        let mut store = GraphStore::new(warp_id);
        let from_id = make_node_id("from");
        let to_id = make_node_id("to");
        let edge_id = make_edge_id("edge-1");
        let ty = make_type_id("EdgeType");

        // Insert nodes first
        store.insert_node(from_id, NodeRecord { ty });
        store.insert_node(to_id, NodeRecord { ty });

        // Upsert edge
        let edge_record = EdgeRecord {
            id: edge_id,
            from: from_id,
            to: to_id,
            ty,
        };
        let upsert_patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::UpsertEdge {
                warp_id,
                record: edge_record.clone(),
            }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        upsert_patch
            .apply_to_store(&mut store)
            .expect("apply failed");
        assert!(store.has_edge(&edge_id));

        // Delete edge
        let delete_patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::DeleteEdge {
                warp_id,
                from: from_id,
                edge_id,
            }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        delete_patch
            .apply_to_store(&mut store)
            .expect("apply failed");
        assert!(!store.has_edge(&edge_id));
    }

    #[test]
    fn apply_to_store_upsert_edge_missing_both_nodes() {
        let warp_id = make_warp_id("test-warp");
        let mut store = GraphStore::new(warp_id);
        let from_id = make_node_id("from");
        let to_id = make_node_id("to");
        let edge_id = make_edge_id("edge-1");
        let ty = make_type_id("EdgeType");

        // Neither node inserted — UpsertEdge should fail on the `from` node first
        let edge_record = EdgeRecord {
            id: edge_id,
            from: from_id,
            to: to_id,
            ty,
        };
        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::UpsertEdge {
                warp_id,
                record: edge_record,
            }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        let result = patch.apply_to_store(&mut store);
        assert!(
            matches!(result, Err(ApplyError::MissingNode(ref k)) if k.local_id == from_id),
            "expected MissingNode for 'from' endpoint, got {result:?}"
        );
    }

    #[test]
    fn apply_to_store_upsert_edge_missing_from_node() {
        let warp_id = make_warp_id("test-warp");
        let mut store = GraphStore::new(warp_id);
        let from_id = make_node_id("from");
        let to_id = make_node_id("to");
        let edge_id = make_edge_id("edge-1");
        let ty = make_type_id("EdgeType");

        // Only insert `to` node — `from` is missing
        store.insert_node(to_id, NodeRecord { ty });

        let edge_record = EdgeRecord {
            id: edge_id,
            from: from_id,
            to: to_id,
            ty,
        };
        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::UpsertEdge {
                warp_id,
                record: edge_record,
            }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        let result = patch.apply_to_store(&mut store);
        assert!(
            matches!(result, Err(ApplyError::MissingNode(ref k)) if k.local_id == from_id),
            "expected MissingNode for 'from' endpoint, got {result:?}"
        );
    }

    #[test]
    fn apply_to_store_upsert_edge_missing_to_node() {
        let warp_id = make_warp_id("test-warp");
        let mut store = GraphStore::new(warp_id);
        let from_id = make_node_id("from");
        let to_id = make_node_id("to");
        let edge_id = make_edge_id("edge-1");
        let ty = make_type_id("EdgeType");

        // Only insert `from` node — `to` is missing
        store.insert_node(from_id, NodeRecord { ty });

        let edge_record = EdgeRecord {
            id: edge_id,
            from: from_id,
            to: to_id,
            ty,
        };
        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::UpsertEdge {
                warp_id,
                record: edge_record,
            }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        let result = patch.apply_to_store(&mut store);
        assert!(
            matches!(result, Err(ApplyError::MissingNode(ref k)) if k.local_id == to_id),
            "expected MissingNode for 'to' endpoint, got {result:?}"
        );
    }

    #[test]
    fn apply_to_store_warp_mismatch_fails() {
        let warp_a = make_warp_id("warp-a");
        let warp_b = make_warp_id("warp-b");
        let mut store = GraphStore::new(warp_a);
        let node_id = make_node_id("node-1");
        let node_key = NodeKey {
            warp_id: warp_b, // Different warp!
            local_id: node_id,
        };

        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id: warp_b, // Patch is for warp_b
            ops: vec![WarpOp::UpsertNode {
                node: node_key,
                record: NodeRecord {
                    ty: make_type_id("Test"),
                },
            }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        // Store is for warp_a, but patch is for warp_b
        let result = patch.apply_to_store(&mut store);
        assert!(matches!(result, Err(ApplyError::WarpMismatch { .. })));
    }

    #[test]
    fn apply_to_store_unsupported_portal_op() {
        let warp_id = make_warp_id("test-warp");
        let mut store = GraphStore::new(warp_id);
        let node_id = make_node_id("node-1");

        // First insert a node to use as portal owner
        store.insert_node(
            node_id,
            NodeRecord {
                ty: make_type_id("Test"),
            },
        );

        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::OpenPortal {
                key: crate::attachment::AttachmentKey::node_alpha(NodeKey {
                    warp_id,
                    local_id: node_id,
                }),
                child_warp: make_warp_id("child"),
                child_root: make_node_id("child-root"),
                init: PortalInit::RequireExisting,
            }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        let result = patch.apply_to_store(&mut store);
        assert!(matches!(
            result,
            Err(ApplyError::UnsupportedOperation {
                op_name: "OpenPortal"
            })
        ));
    }

    #[test]
    fn apply_to_store_set_node_attachment() {
        use crate::attachment::{AtomPayload, AttachmentKey};

        let warp_id = make_warp_id("test-warp");
        let mut store = GraphStore::new(warp_id);
        let node_id = make_node_id("node-1");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };
        let ty = make_type_id("TestType");

        // Insert node first
        store.insert_node(node_id, NodeRecord { ty });

        let attachment_key = AttachmentKey::node_alpha(node_key);
        let payload = AtomPayload::new(ty, bytes::Bytes::from_static(b"test-data"));
        let value = Some(AttachmentValue::Atom(payload));

        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::SetAttachment {
                key: attachment_key,
                value: value.clone(),
            }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        assert!(store.node_attachment(&node_id).is_none());
        patch.apply_to_store(&mut store).expect("apply failed");
        assert!(store.node_attachment(&node_id).is_some());
    }
}
