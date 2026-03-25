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

pub use echo_runtime_schema::WorldlineId;

use crate::clock::GlobalTick;
use crate::ident::{EdgeKey, Hash, NodeKey, WarpId};
use crate::materialization::ChannelId;
use crate::tick_patch::{apply_ops_to_state, SlotId, TickPatchError, WarpOp};
use crate::worldline_state::WorldlineState;

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
    /// Runtime cycle stamp for the commit that produced this patch.
    pub commit_global_tick: GlobalTick,
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
#[derive(Clone, PartialEq, Eq, Debug)]
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
    /// Returns the runtime cycle stamp from the header.
    #[inline]
    #[must_use]
    pub fn commit_global_tick(&self) -> GlobalTick {
        self.header.commit_global_tick
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

    /// Apply this patch to a full [`WorldlineState`].
    ///
    /// Replay is worldline-scoped: portal and warp-instance operations are
    /// applied against the full multi-instance state, not a warp-local store.
    ///
    /// # Errors
    ///
    /// Returns [`ApplyError`] if the patch does not belong to the worldline root,
    /// or if any operation violates full-state replay invariants.
    pub fn apply_to_worldline_state(&self, state: &mut WorldlineState) -> Result<(), ApplyError> {
        if state.root().warp_id != self.warp_id {
            return Err(ApplyError::WarpMismatch {
                expected: state.root().warp_id,
                actual: self.warp_id,
            });
        }

        apply_ops_to_state(&mut state.warp_state, &self.ops)?;
        Ok(())
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

/// Records which atom was written by which rule during a tick.
///
/// This enables provenance tracking for the TTD "Show Me Why" feature and
/// footprint verification. Each `AtomWrite` captures:
/// - The atom (node) that was modified
/// - The rule that performed the modification
/// - The tick when it happened
/// - The before/after values for diff visualization
///
/// # Usage
///
/// Atom writes are collected during tick execution and stored in the provenance
/// store alongside patches and outputs. The WARPSITE demo uses this to render
/// causal arrows from rules to atoms in the 4D provenance view.
///
/// # Serialization
///
/// Values are stored as raw bytes since the provenance store is codec-agnostic.
/// Higher-level layers can interpret these bytes using the appropriate codec
/// based on the atom's type.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AtomWrite {
    /// The atom (node) that was written.
    pub atom: NodeKey,
    /// The canonical rule ID (256-bit hash) that performed this write.
    pub rule_id: Hash,
    /// The global tick when this write occurred.
    pub tick: u64,
    /// The atom's value before this write, if it existed.
    ///
    /// `None` means the atom was created (didn't exist before).
    pub old_value: Option<Vec<u8>>,
    /// The atom's value after this write.
    ///
    /// For deletions, this would be empty (but deletion is typically
    /// tracked via `WarpOp::DeleteNode` rather than `AtomWrite`).
    pub new_value: Vec<u8>,
}

impl AtomWrite {
    /// Creates a new `AtomWrite` record.
    pub fn new(
        atom: NodeKey,
        rule_id: Hash,
        tick: u64,
        old_value: Option<Vec<u8>>,
        new_value: Vec<u8>,
    ) -> Self {
        Self {
            atom,
            rule_id,
            tick,
            old_value,
            new_value,
        }
    }

    /// Returns `true` if this was a create operation (atom didn't exist before).
    pub fn is_create(&self) -> bool {
        self.old_value.is_none()
    }

    /// Returns `true` if the value actually changed.
    pub fn is_mutation(&self) -> bool {
        self.old_value.as_deref() != Some(self.new_value.as_slice())
    }
}

/// Collection of atom writes for a single tick.
///
/// This type alias exists for clarity at API boundaries. The writes are
/// ordered by execution sequence within the tick.
pub type AtomWriteSet = Vec<AtomWrite>;

/// Errors produced while applying a worldline patch to a worldline state.
///
/// These errors indicate structural violations when replaying patches:
/// - Missing nodes/edges that were expected to exist
/// - Missing warp instance that was expected to exist
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

    /// Invalid attachment key (wrong plane for owner type).
    #[error("invalid attachment key: node owners use Alpha plane, edge owners use Beta plane")]
    InvalidAttachmentKey,

    /// Tried to delete a node that has incident edges.
    ///
    /// `DeleteNode` must not cascade. Emit explicit `DeleteEdge` ops first.
    #[error("node not isolated (has edges): {0:?}")]
    NodeNotIsolated(NodeKey),

    /// Full-state replay failed while applying a worldline patch.
    #[error(transparent)]
    TickPatch(TickPatchError),
}

impl From<TickPatchError> for ApplyError {
    fn from(value: TickPatchError) -> Self {
        match value {
            TickPatchError::MissingNode(node) => Self::MissingNode(node),
            TickPatchError::MissingEdge(edge) => Self::MissingEdge(edge),
            TickPatchError::InvalidAttachmentKey(_) => Self::InvalidAttachmentKey,
            TickPatchError::NodeNotIsolated(node) => Self::NodeNotIsolated(node),
            other => Self::TickPatch(other),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::redundant_clone)]

    use super::*;
    use crate::attachment::{AttachmentKey, AttachmentValue};
    use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id};
    use crate::record::{EdgeRecord, NodeRecord};
    use crate::tick_patch::PortalInit;
    use crate::worldline_state::WorldlineState;

    #[test]
    fn worldline_id_is_transparent_wrapper() {
        let hash = [42u8; 32];
        let id = WorldlineId::from_bytes(hash);
        assert_eq!(*id.as_bytes(), hash);
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
            commit_global_tick: GlobalTick::from_raw(42),
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
        assert_eq!(patch.commit_global_tick(), GlobalTick::from_raw(42));
        assert_eq!(patch.policy_id(), 1);
    }

    fn test_header() -> WorldlineTickHeaderV1 {
        WorldlineTickHeaderV1 {
            commit_global_tick: GlobalTick::ZERO,
            policy_id: 0,
            rule_pack_id: [0u8; 32],
            plan_digest: [0u8; 32],
            decision_digest: [0u8; 32],
            rewrites_digest: [0u8; 32],
        }
    }

    fn single_root_state(warp_id: WarpId) -> WorldlineState {
        let root = make_node_id("root");
        let mut store = crate::GraphStore::new(warp_id);
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("RootType"),
            },
        );
        WorldlineState::from_root_store(store, root).expect("single-root worldline should be valid")
    }

    #[test]
    fn apply_to_worldline_state_upsert_node() {
        let warp_id = make_warp_id("test-warp");
        let mut state = single_root_state(warp_id);
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

        assert!(state.store(&warp_id).unwrap().node(&node_id).is_none());
        patch
            .apply_to_worldline_state(&mut state)
            .expect("apply failed");
        assert!(state.store(&warp_id).unwrap().node(&node_id).is_some());
        assert_eq!(
            state.store(&warp_id).unwrap().node(&node_id).map(|n| n.ty),
            Some(ty)
        );
    }

    #[test]
    fn apply_to_worldline_state_delete_node() {
        let warp_id = make_warp_id("test-warp");
        let mut state = single_root_state(warp_id);
        let node_id = make_node_id("node-1");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };

        state
            .warp_state
            .store_mut(&warp_id)
            .expect("root store missing")
            .insert_node(
                node_id,
                NodeRecord {
                    ty: make_type_id("TestType"),
                },
            );

        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::DeleteNode { node: node_key }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        patch
            .apply_to_worldline_state(&mut state)
            .expect("apply failed");
        assert!(state.store(&warp_id).unwrap().node(&node_id).is_none());
    }

    #[test]
    fn apply_to_worldline_state_delete_missing_node_fails() {
        let warp_id = make_warp_id("test-warp");
        let mut state = single_root_state(warp_id);
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

        let result = patch.apply_to_worldline_state(&mut state);
        assert!(matches!(result, Err(ApplyError::MissingNode(_))));
    }

    #[test]
    fn apply_to_worldline_state_upsert_and_delete_edge() {
        let warp_id = make_warp_id("test-warp");
        let mut state = single_root_state(warp_id);
        let from_id = make_node_id("from");
        let to_id = make_node_id("to");
        let edge_id = make_edge_id("edge-1");
        let ty = make_type_id("EdgeType");

        let store = state
            .warp_state
            .store_mut(&warp_id)
            .expect("root store missing");
        store.insert_node(from_id, NodeRecord { ty });
        store.insert_node(to_id, NodeRecord { ty });

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
            .apply_to_worldline_state(&mut state)
            .expect("apply failed");
        assert!(state.store(&warp_id).unwrap().has_edge(&edge_id));

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
            .apply_to_worldline_state(&mut state)
            .expect("apply failed");
        assert!(!state.store(&warp_id).unwrap().has_edge(&edge_id));
    }

    #[test]
    fn apply_to_worldline_state_upsert_edge_allows_missing_endpoint_references() {
        let warp_id = make_warp_id("test-warp");
        let mut state = single_root_state(warp_id);
        let from_id = make_node_id("from");
        let to_id = make_node_id("to");
        let edge_id = make_edge_id("edge-1");
        let ty = make_type_id("EdgeType");

        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id,
            ops: vec![WarpOp::UpsertEdge {
                warp_id,
                record: EdgeRecord {
                    id: edge_id,
                    from: from_id,
                    to: to_id,
                    ty,
                },
            }],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [0u8; 32],
        };

        patch
            .apply_to_worldline_state(&mut state)
            .expect("edge insert should follow tick-patch semantics");
        assert!(state.store(&warp_id).unwrap().has_edge(&edge_id));
    }

    #[test]
    fn apply_to_worldline_state_warp_mismatch_fails() {
        let warp_a = make_warp_id("warp-a");
        let warp_b = make_warp_id("warp-b");
        let mut state = single_root_state(warp_a);
        let node_id = make_node_id("node-1");
        let node_key = NodeKey {
            warp_id: warp_b,
            local_id: node_id,
        };

        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id: warp_b,
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

        let result = patch.apply_to_worldline_state(&mut state);
        assert!(matches!(result, Err(ApplyError::WarpMismatch { .. })));
    }

    #[test]
    fn apply_to_worldline_state_set_node_attachment() {
        use crate::attachment::{AtomPayload, AttachmentKey};

        let warp_id = make_warp_id("test-warp");
        let mut state = single_root_state(warp_id);
        let node_id = make_node_id("node-1");
        let node_key = NodeKey {
            warp_id,
            local_id: node_id,
        };
        let ty = make_type_id("TestType");

        state
            .warp_state
            .store_mut(&warp_id)
            .expect("root store missing")
            .insert_node(node_id, NodeRecord { ty });

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

        assert!(state
            .store(&warp_id)
            .unwrap()
            .node_attachment(&node_id)
            .is_none());
        patch
            .apply_to_worldline_state(&mut state)
            .expect("apply failed");
        assert!(state
            .store(&warp_id)
            .unwrap()
            .node_attachment(&node_id)
            .is_some());
    }

    #[test]
    fn apply_to_worldline_state_open_portal_creates_child_instance() {
        let mut state = WorldlineState::empty();
        let root = *state.root();
        let portal_key = AttachmentKey::node_alpha(root);
        let child_warp = make_warp_id("child");
        let child_root = make_node_id("child-root");

        let patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id: root.warp_id,
            ops: vec![WarpOp::OpenPortal {
                key: portal_key,
                child_warp,
                child_root,
                init: PortalInit::Empty {
                    root_record: NodeRecord {
                        ty: make_type_id("ChildRootTy"),
                    },
                },
            }],
            in_slots: vec![],
            out_slots: vec![SlotId::Attachment(portal_key)],
            patch_digest: [0u8; 32],
        };

        patch
            .apply_to_worldline_state(&mut state)
            .expect("apply failed");

        let child_instance = state
            .warp_state()
            .instance(&child_warp)
            .expect("child instance missing");
        assert_eq!(child_instance.parent, Some(portal_key));
        assert_eq!(child_instance.root_node, child_root);

        let child_store = state
            .warp_state()
            .store(&child_warp)
            .expect("child store missing");
        assert!(child_store.node(&child_root).is_some());

        let root_store = state
            .warp_state()
            .store(&root.warp_id)
            .expect("root store missing");
        assert_eq!(
            root_store.node_attachment(&root.local_id),
            Some(&AttachmentValue::Descend(child_warp))
        );
    }

    #[test]
    fn apply_to_worldline_state_delete_instance_after_clearing_portal() {
        let mut state = WorldlineState::empty();
        let root = *state.root();
        let portal_key = AttachmentKey::node_alpha(root);
        let child_warp = make_warp_id("child");
        let child_root = make_node_id("child-root");

        let open_patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id: root.warp_id,
            ops: vec![WarpOp::OpenPortal {
                key: portal_key,
                child_warp,
                child_root,
                init: PortalInit::Empty {
                    root_record: NodeRecord {
                        ty: make_type_id("ChildRootTy"),
                    },
                },
            }],
            in_slots: vec![],
            out_slots: vec![SlotId::Attachment(portal_key)],
            patch_digest: [0u8; 32],
        };
        open_patch
            .apply_to_worldline_state(&mut state)
            .expect("open portal failed");

        let delete_patch = WorldlineTickPatchV1 {
            header: test_header(),
            warp_id: root.warp_id,
            ops: vec![
                WarpOp::SetAttachment {
                    key: portal_key,
                    value: None,
                },
                WarpOp::DeleteWarpInstance {
                    warp_id: child_warp,
                },
            ],
            in_slots: vec![],
            out_slots: vec![SlotId::Attachment(portal_key)],
            patch_digest: [0u8; 32],
        };
        delete_patch
            .apply_to_worldline_state(&mut state)
            .expect("delete portal failed");

        assert!(state.warp_state().instance(&child_warp).is_none());
        assert!(state.warp_state().store(&child_warp).is_none());
        let root_store = state
            .warp_state()
            .store(&root.warp_id)
            .expect("root store missing");
        assert!(root_store.node_attachment(&root.local_id).is_none());
    }
}
