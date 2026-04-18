// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Worldline state wrapper and frontier management.
//!
//! [`WorldlineState`] wraps [`WarpState`] from day one to prevent public APIs
//! from calcifying around an abstraction the system already knows is too small.
//! Later phases will extend this wrapper with additional state dimensions
//! (e.g., causal frontier, head metadata).
//!
//! [`WorldlineFrontier`] owns the single mutable frontier state for a worldline.
//! All live mutation for a worldline goes through deterministic commit against
//! this frontier state.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::clock::WorldlineTick;
use crate::graph::GraphStore;
use crate::head::WriterHeadKey;
use crate::ident::{make_node_id, make_type_id, make_warp_id, Hash, NodeId, NodeKey};
use crate::materialization::{ChannelConflict, FinalizedChannel};
use crate::receipt::TickReceipt;
use crate::record::NodeRecord;
use crate::snapshot::Snapshot;
use crate::tick_patch::WarpTickPatchV1;
use crate::warp_state::WarpInstance;
use crate::warp_state::WarpState;
use crate::worldline::WorldlineId;

// =============================================================================
// WorldlineState
// =============================================================================

/// Error returned when a [`WorldlineState`] cannot validate its root invariant.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum WorldlineStateError {
    /// The supplied [`WarpState`] has no parentless root instance.
    #[error("worldline state has no parentless root instance")]
    NoRootInstance,
    /// The supplied [`WarpState`] has more than one parentless root instance.
    #[error("worldline state has multiple parentless root instances")]
    MultipleRootInstances,
    /// The caller-supplied root warp does not match the unique root instance.
    #[error("worldline root warp mismatch: expected {expected:?}, got {actual:?}")]
    RootWarpMismatch {
        /// The unique root warp discovered in the state.
        expected: crate::ident::WarpId,
        /// The warp id supplied by the caller.
        actual: crate::ident::WarpId,
    },
    /// The caller-supplied root node does not match the unique root instance.
    #[error(
        "worldline root node mismatch for warp {warp_id:?}: expected {expected:?}, got {actual:?}"
    )]
    RootNodeMismatch {
        /// The warp whose root node disagreed.
        warp_id: crate::ident::WarpId,
        /// The root node declared by the warp instance metadata.
        expected: crate::ident::NodeId,
        /// The root node supplied by the caller.
        actual: crate::ident::NodeId,
    },
    /// The unique root instance has no backing graph store.
    #[error("worldline root store missing for warp {0:?}")]
    MissingRootStore(crate::ident::WarpId),
    /// The supplied root node does not exist in the provided root store.
    #[error("worldline root node {root:?} is missing from the supplied store")]
    MissingRootNode {
        /// Root key that was requested but not found in the store.
        root: NodeKey,
    },
}

/// Broad worldline state abstraction wrapping [`WarpState`].
///
/// This wrapper exists so that public APIs don't cement around `GraphStore`
/// or `WarpState` directly. When later phases need full `WorldlineState`
/// replay (portals, instances), this wrapper expands without breaking callers.
#[derive(Debug, Clone)]
pub struct WorldlineState {
    /// The underlying multi-instance warp state.
    pub(crate) warp_state: WarpState,
    /// Root key for snapshot hashing and commit execution.
    pub(crate) root: NodeKey,
    /// Initial worldline state preserved for replay.
    pub(crate) initial_state: WarpState,
    /// Most recent snapshot committed for this worldline.
    pub(crate) last_snapshot: Option<Snapshot>,
    /// Sequential history of committed ticks for this worldline.
    pub(crate) tick_history: Vec<(Snapshot, TickReceipt, WarpTickPatchV1)>,
    /// Last finalized materialization channels for this worldline.
    pub(crate) last_materialization: Vec<FinalizedChannel>,
    /// Last materialization errors for this worldline.
    pub(crate) last_materialization_errors: Vec<ChannelConflict>,
    /// Monotonic transaction counter for this worldline's commit history.
    pub(crate) tx_counter: u64,
    /// Committed ingress ids scoped to the writer head that accepted them.
    ///
    /// This is an in-memory lifetime dedupe ledger for a live
    /// [`WorldlineRuntime`](crate::coordinator::WorldlineRuntime). Entries live
    /// for as long as the frontier lives and are not persisted across process
    /// restarts; Phase 3 intentionally keeps lifetime idempotence rather than a
    /// bounded replay horizon.
    pub(crate) committed_ingress: BTreeSet<(WriterHeadKey, Hash)>,
}

impl Default for WorldlineState {
    fn default() -> Self {
        Self::empty()
    }
}

impl WorldlineState {
    /// Creates a new worldline state from an existing warp state and root key.
    ///
    /// # Errors
    ///
    /// Returns [`WorldlineStateError`] if the supplied state does not contain
    /// exactly one parentless root instance with a backing store, or if the
    /// caller-supplied `root` does not match that unique root instance.
    pub fn new(warp_state: WarpState, root: NodeKey) -> Result<Self, WorldlineStateError> {
        Self::validate_root(&warp_state, root)?;
        Ok(Self::build_validated(warp_state, root))
    }

    fn build_validated(warp_state: WarpState, root: NodeKey) -> Self {
        Self {
            initial_state: warp_state.clone(),
            warp_state,
            root,
            last_snapshot: None,
            tick_history: Vec::new(),
            last_materialization: Vec::new(),
            last_materialization_errors: Vec::new(),
            tx_counter: 0,
            committed_ingress: BTreeSet::new(),
        }
    }

    fn discovered_root(state: &WarpState) -> Result<NodeKey, WorldlineStateError> {
        let mut parentless = state.iter_instances().filter_map(|(warp_id, instance)| {
            instance.parent.is_none().then_some(NodeKey {
                warp_id: *warp_id,
                local_id: instance.root_node,
            })
        });

        let Some(root) = parentless.next() else {
            return Err(WorldlineStateError::NoRootInstance);
        };

        if parentless.next().is_some() {
            return Err(WorldlineStateError::MultipleRootInstances);
        }

        if state.store(&root.warp_id).is_none() {
            return Err(WorldlineStateError::MissingRootStore(root.warp_id));
        }

        Ok(root)
    }

    fn validate_root(state: &WarpState, root: NodeKey) -> Result<(), WorldlineStateError> {
        let discovered = Self::discovered_root(state)?;
        if discovered.warp_id != root.warp_id {
            return Err(WorldlineStateError::RootWarpMismatch {
                expected: discovered.warp_id,
                actual: root.warp_id,
            });
        }
        if discovered.local_id != root.local_id {
            return Err(WorldlineStateError::RootNodeMismatch {
                warp_id: root.warp_id,
                expected: discovered.local_id,
                actual: root.local_id,
            });
        }
        Ok(())
    }

    /// Creates an empty worldline state with a canonical root instance.
    #[must_use]
    pub fn empty() -> Self {
        let root_warp = make_warp_id("root");
        let root_node = make_node_id("root");
        let root = NodeKey {
            warp_id: root_warp,
            local_id: root_node,
        };

        let mut store = GraphStore::new(root_warp);
        store.insert_node(
            root_node,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );

        let mut warp_state = WarpState::new();
        warp_state.upsert_instance(
            WarpInstance {
                warp_id: root_warp,
                root_node,
                parent: None,
            },
            store,
        );

        Self::build_validated(warp_state, root)
    }

    /// Creates a worldline state from a single root store and root node.
    ///
    /// This is the minimal public constructor for root-only worldlines. It is
    /// primarily useful for tests, replay bases, and adapters that need a
    /// deterministic `WorldlineState` without going through live engine setup.
    pub fn from_root_store(
        store: GraphStore,
        root_node: NodeId,
    ) -> Result<Self, WorldlineStateError> {
        let warp_id = store.warp_id();
        let root = NodeKey {
            warp_id,
            local_id: root_node,
        };
        if store.node(&root_node).is_none() {
            return Err(WorldlineStateError::MissingRootNode { root });
        }

        let mut warp_state = WarpState::new();
        warp_state.upsert_instance(
            WarpInstance {
                warp_id,
                root_node,
                parent: None,
            },
            store,
        );

        Self::new(warp_state, root)
    }

    /// Returns a reference to the underlying warp state.
    #[must_use]
    pub fn warp_state(&self) -> &WarpState {
        &self.warp_state
    }

    /// Returns the canonical full-state root hash for this worldline.
    #[must_use]
    pub fn state_root(&self) -> Hash {
        crate::snapshot::compute_state_root_for_warp_state(&self.warp_state, &self.root)
    }

    /// Returns the graph store for a specific warp instance, if present.
    #[must_use]
    pub fn store(&self, warp_id: &crate::ident::WarpId) -> Option<&GraphStore> {
        self.warp_state.store(warp_id)
    }

    /// Returns the root key used for hashing and commit execution.
    #[must_use]
    pub fn root(&self) -> &NodeKey {
        &self.root
    }

    /// Returns the current replay base for this worldline.
    #[must_use]
    pub fn initial_state(&self) -> &WarpState {
        &self.initial_state
    }

    /// Returns the last committed snapshot for this worldline, if any.
    #[must_use]
    pub fn last_snapshot(&self) -> Option<&Snapshot> {
        self.last_snapshot.as_ref()
    }

    /// Returns the committed tick history for this worldline.
    #[must_use]
    pub fn tick_history(&self) -> &[(Snapshot, TickReceipt, WarpTickPatchV1)] {
        &self.tick_history
    }

    /// Returns the most recent finalized materialization channels.
    #[must_use]
    pub fn last_materialization(&self) -> &[FinalizedChannel] {
        &self.last_materialization
    }

    /// Returns the most recent materialization errors.
    #[must_use]
    pub fn last_materialization_errors(&self) -> &[ChannelConflict] {
        &self.last_materialization_errors
    }

    /// Returns the current committed frontier tick implied by this state's history.
    #[must_use]
    pub fn current_tick(&self) -> WorldlineTick {
        WorldlineTick::from_raw(self.tick_history.len() as u64)
    }

    /// Returns `true` if this worldline already committed the ingress for the given head.
    #[must_use]
    pub(crate) fn contains_committed_ingress(
        &self,
        head_key: &WriterHeadKey,
        ingress_id: &Hash,
    ) -> bool {
        self.committed_ingress.contains(&(*head_key, *ingress_id))
    }

    /// Records a committed ingress batch for the given writer head.
    pub(crate) fn record_committed_ingress<I>(&mut self, head_key: WriterHeadKey, ingress_ids: I)
    where
        I: IntoIterator<Item = Hash>,
    {
        self.committed_ingress.extend(
            ingress_ids
                .into_iter()
                .map(|ingress_id| (head_key, ingress_id)),
        );
    }

    /// Clones the deterministic replay-relevant state for checkpoint storage.
    ///
    /// Checkpoints preserve already-validated replay artifacts so exact
    /// checkpoint restore does not need to rehydrate prefix metadata from
    /// provenance entries. Only process-local ingress dedupe state is cleared.
    pub(crate) fn replay_checkpoint_clone(&self) -> Self {
        Self {
            warp_state: self.warp_state.clone(),
            root: self.root,
            initial_state: self.initial_state.clone(),
            last_snapshot: self.last_snapshot.clone(),
            tick_history: self.tick_history.clone(),
            last_materialization: self.last_materialization.clone(),
            last_materialization_errors: self.last_materialization_errors.clone(),
            tx_counter: self.tx_counter,
            committed_ingress: BTreeSet::new(),
        }
    }

    /// Clones the canonical `U0` replay base used for suffix replay.
    ///
    /// This preserves the deterministic initial boundary and root coordinate
    /// while dropping frontier-only materialization and replay metadata.
    pub(crate) fn replay_base_from_initial(&self) -> Self {
        let initial_state = self.initial_state.clone();
        Self {
            warp_state: initial_state.clone(),
            root: self.root,
            initial_state,
            last_snapshot: None,
            tick_history: Vec::new(),
            last_materialization: Vec::new(),
            last_materialization_errors: Vec::new(),
            tx_counter: 0,
            committed_ingress: BTreeSet::new(),
        }
    }

    /// Records one replayable committed tick into frontier metadata.
    pub(crate) fn record_replayed_tick(
        &mut self,
        snapshot: Snapshot,
        receipt: TickReceipt,
        replay_patch: WarpTickPatchV1,
        materialization: Vec<FinalizedChannel>,
    ) {
        self.tick_history
            .push((snapshot.clone(), receipt, replay_patch));
        self.last_snapshot = Some(snapshot);
        self.last_materialization = materialization;
        self.last_materialization_errors.clear();
        self.tx_counter = self.tick_history.len() as u64;
    }
}

impl TryFrom<WarpState> for WorldlineState {
    type Error = WorldlineStateError;

    fn try_from(warp_state: WarpState) -> Result<Self, Self::Error> {
        let root = Self::discovered_root(&warp_state)?;
        Self::new(warp_state, root)
    }
}

// =============================================================================
// WorldlineFrontier
// =============================================================================

/// The single mutable frontier for a worldline.
///
/// A worldline has exactly one frontier state object. Writer heads are control
/// objects that schedule work against this frontier; they do not own private
/// mutable stores.
///
/// # Fields
///
/// - `worldline_id`: identity of this worldline.
/// - `state`: the mutable frontier state.
/// - `frontier_tick`: the current tick count (will be typed as `WorldlineTick`
///   in Phase 6).
#[derive(Debug, Clone)]
pub struct WorldlineFrontier {
    /// Identity of this worldline (immutable after construction).
    worldline_id: WorldlineId,
    /// The single mutable state for this worldline.
    pub(crate) state: WorldlineState,
    /// Current frontier tick (typed in Phase 6 as `WorldlineTick`).
    ///
    /// `pub(crate)` — only the coordinator may advance this.
    pub(crate) frontier_tick: WorldlineTick,
}

impl WorldlineFrontier {
    /// Creates a new frontier for the given worldline.
    #[must_use]
    pub fn new(worldline_id: WorldlineId, state: WorldlineState) -> Self {
        Self {
            worldline_id,
            state,
            frontier_tick: WorldlineTick::ZERO,
        }
    }

    /// Returns the identity of this worldline.
    #[must_use]
    pub fn worldline_id(&self) -> WorldlineId {
        self.worldline_id
    }

    /// Returns the current frontier tick.
    #[must_use]
    pub fn frontier_tick(&self) -> WorldlineTick {
        self.frontier_tick
    }

    /// Returns a reference to the worldline state.
    #[must_use]
    pub fn state(&self) -> &WorldlineState {
        &self.state
    }

    /// Returns a mutable reference to the worldline state for internal commit flow.
    pub(crate) fn state_mut(&mut self) -> &mut WorldlineState {
        &mut self.state
    }

    /// Advances the frontier tick by one, returning the new value.
    pub(crate) fn advance_tick(&mut self) -> Option<WorldlineTick> {
        self.frontier_tick = self.frontier_tick.checked_increment()?;
        Some(self.frontier_tick)
    }

    /// Creates a frontier at a specific tick (used for fork/rebuild).
    #[must_use]
    pub fn at_tick(worldline_id: WorldlineId, state: WorldlineState, tick: WorldlineTick) -> Self {
        Self {
            worldline_id,
            state,
            frontier_tick: tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::{AttachmentKey, AttachmentOwner, AttachmentPlane};
    use crate::warp_state::WarpState;

    fn wl(n: u8) -> WorldlineId {
        WorldlineId::from_bytes([n; 32])
    }

    #[test]
    fn worldline_state_wraps_warp_state() {
        let ws = WorldlineState::empty();
        // WorldlineState is a transparent wrapper
        assert_eq!(ws.root().local_id, make_node_id("root"));
    }

    #[test]
    fn worldline_frontier_starts_at_tick_zero() {
        let frontier = WorldlineFrontier::new(wl(1), WorldlineState::empty());
        assert_eq!(frontier.frontier_tick(), WorldlineTick::ZERO);
        assert_eq!(frontier.worldline_id(), wl(1));
    }

    #[test]
    fn worldline_frontier_at_tick() {
        let frontier =
            WorldlineFrontier::at_tick(wl(1), WorldlineState::empty(), WorldlineTick::from_raw(42));
        assert_eq!(frontier.frontier_tick(), WorldlineTick::from_raw(42));
    }

    #[test]
    fn try_from_warp_state() {
        let result = WorldlineState::try_from(WorldlineState::empty().warp_state().clone());
        assert!(
            result.is_ok(),
            "worldline state conversion failed: {result:?}"
        );
        let Ok(ws) = result else {
            return;
        };
        assert_eq!(ws.root().warp_id, make_warp_id("root"));
        assert!(ws.tick_history().is_empty());
    }

    #[test]
    fn rejects_multiple_parentless_instances() {
        let root_a = make_warp_id("root-a");
        let root_b = make_warp_id("root-b");
        let node_a = make_node_id("root-a");
        let node_b = make_node_id("root-b");

        let mut store_a = GraphStore::new(root_a);
        store_a.insert_node(
            node_a,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );
        let mut store_b = GraphStore::new(root_b);
        store_b.insert_node(
            node_b,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );

        let mut state = WarpState::new();
        state.upsert_instance(
            WarpInstance {
                warp_id: root_a,
                root_node: node_a,
                parent: None,
            },
            store_a,
        );
        state.upsert_instance(
            WarpInstance {
                warp_id: root_b,
                root_node: node_b,
                parent: None,
            },
            store_b,
        );

        let result = WorldlineState::try_from(state);
        assert!(
            matches!(result, Err(WorldlineStateError::MultipleRootInstances)),
            "expected MultipleRootInstances, got {result:?}"
        );
    }

    #[test]
    fn rejects_mismatched_explicit_root() {
        let state = WorldlineState::empty().warp_state().clone();
        let wrong_root = NodeKey {
            warp_id: make_warp_id("root"),
            local_id: make_node_id("wrong-root"),
        };

        let result = WorldlineState::new(state, wrong_root);
        assert_eq!(
            result.err(),
            Some(WorldlineStateError::RootNodeMismatch {
                warp_id: make_warp_id("root"),
                expected: make_node_id("root"),
                actual: make_node_id("wrong-root"),
            })
        );
    }

    #[test]
    fn rejects_root_without_backing_store() {
        let root_warp = make_warp_id("root");
        let root_node = make_node_id("root");
        let mut state = WarpState::new();
        state.instances.insert(
            root_warp,
            WarpInstance {
                warp_id: root_warp,
                root_node,
                parent: None,
            },
        );
        state.instances.insert(
            make_warp_id("child"),
            WarpInstance {
                warp_id: make_warp_id("child"),
                root_node: make_node_id("child-root"),
                parent: Some(AttachmentKey {
                    owner: AttachmentOwner::Node(NodeKey {
                        warp_id: root_warp,
                        local_id: root_node,
                    }),
                    plane: AttachmentPlane::Alpha,
                }),
            },
        );

        let result = WorldlineState::try_from(state);
        assert_eq!(
            result.err(),
            Some(WorldlineStateError::MissingRootStore(root_warp))
        );
    }

    #[test]
    fn from_root_store_rejects_missing_root_node() {
        let warp_id = make_warp_id("root");
        let mut store = GraphStore::new(warp_id);
        store.insert_node(
            make_node_id("present-root"),
            NodeRecord {
                ty: make_type_id("world"),
            },
        );

        let missing_root = make_node_id("missing-root");
        let result = WorldlineState::from_root_store(store, missing_root);
        assert_eq!(
            result.err(),
            Some(WorldlineStateError::MissingRootNode {
                root: NodeKey {
                    warp_id,
                    local_id: missing_root,
                },
            })
        );
    }

    #[test]
    fn replay_checkpoint_clone_preserves_replay_artifacts_but_clears_ingress_ledger() {
        let mut state = WorldlineState::empty();
        state.last_snapshot = Some(Snapshot {
            root: *state.root(),
            hash: [3u8; 32],
            state_root: [1u8; 32],
            parents: Vec::new(),
            plan_digest: [4u8; 32],
            decision_digest: [5u8; 32],
            rewrites_digest: [6u8; 32],
            patch_digest: [2u8; 32],
            policy_id: 7,
            tx: crate::tx::TxId::from_raw(8),
        });
        state.tick_history.push((
            Snapshot {
                root: *state.root(),
                hash: [9u8; 32],
                state_root: [7u8; 32],
                parents: vec![[19u8; 32]],
                plan_digest: [10u8; 32],
                decision_digest: [11u8; 32],
                rewrites_digest: [12u8; 32],
                patch_digest: [8u8; 32],
                policy_id: 13,
                tx: crate::tx::TxId::from_raw(14),
            },
            TickReceipt::new(crate::tx::TxId::from_raw(15), Vec::new(), Vec::new()),
            WarpTickPatchV1::new(
                0,
                [16u8; 32],
                crate::tick_patch::TickCommitStatus::Committed,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        ));
        state.last_materialization.push(FinalizedChannel {
            channel: crate::make_type_id("materialized"),
            data: vec![1, 2, 3],
        });
        state.last_materialization_errors.push(ChannelConflict {
            channel: crate::make_type_id("materialized"),
            emission_count: 2,
            kind: crate::materialization::MaterializationErrorKind::StrictSingleConflict,
        });
        state.tx_counter = 42;
        let head_key = WriterHeadKey {
            worldline_id: WorldlineId::from_bytes([17u8; 32]),
            head_id: crate::head::make_head_id("checkpoint"),
        };
        state.record_committed_ingress(head_key, [[18u8; 32]]);

        let checkpoint = state.replay_checkpoint_clone();

        assert_eq!(checkpoint.root, state.root);
        assert_eq!(checkpoint.state_root(), state.state_root());
        assert_eq!(
            crate::snapshot::compute_state_root_for_warp_state(
                checkpoint.initial_state(),
                checkpoint.root()
            ),
            crate::snapshot::compute_state_root_for_warp_state(state.initial_state(), state.root())
        );
        assert_eq!(
            checkpoint
                .last_snapshot
                .as_ref()
                .map(|snapshot| snapshot.hash),
            state.last_snapshot.as_ref().map(|snapshot| snapshot.hash)
        );
        assert_eq!(checkpoint.tick_history.len(), state.tick_history.len());
        assert_eq!(
            checkpoint.tick_history[0].0.hash,
            state.tick_history[0].0.hash
        );
        assert_eq!(
            checkpoint.tick_history[0].0.state_root,
            state.tick_history[0].0.state_root
        );
        assert_eq!(
            checkpoint.tick_history[0].1.tx(),
            state.tick_history[0].1.tx()
        );
        assert_eq!(
            checkpoint.tick_history[0].2.digest(),
            state.tick_history[0].2.digest()
        );
        assert_eq!(
            checkpoint.last_materialization.len(),
            state.last_materialization.len()
        );
        assert_eq!(
            checkpoint.last_materialization[0].channel,
            state.last_materialization[0].channel
        );
        assert_eq!(
            checkpoint.last_materialization[0].data,
            state.last_materialization[0].data
        );
        assert_eq!(
            checkpoint.last_materialization_errors.len(),
            state.last_materialization_errors.len()
        );
        assert_eq!(checkpoint.tx_counter, state.tx_counter);
        assert!(checkpoint.committed_ingress.is_empty());
    }

    #[test]
    fn replay_base_from_initial_resets_frontier_metadata() {
        let mut state = WorldlineState::empty();
        let snapshot = Snapshot {
            root: *state.root(),
            hash: [1u8; 32],
            state_root: [2u8; 32],
            parents: vec![[3u8; 32]],
            plan_digest: [4u8; 32],
            decision_digest: [5u8; 32],
            rewrites_digest: [6u8; 32],
            patch_digest: [7u8; 32],
            policy_id: 8,
            tx: crate::tx::TxId::from_raw(9),
        };
        state.last_snapshot = Some(snapshot.clone());
        state.tick_history.push((
            snapshot,
            TickReceipt::new(crate::tx::TxId::from_raw(10), Vec::new(), Vec::new()),
            WarpTickPatchV1::new(
                0,
                [11u8; 32],
                crate::tick_patch::TickCommitStatus::Committed,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        ));
        state.last_materialization.push(FinalizedChannel {
            channel: crate::make_type_id("materialized"),
            data: vec![1, 2, 3],
        });
        state.last_materialization_errors.push(ChannelConflict {
            channel: crate::make_type_id("materialized"),
            emission_count: 2,
            kind: crate::materialization::MaterializationErrorKind::StrictSingleConflict,
        });
        state.tx_counter = 11;
        let head_key = WriterHeadKey {
            worldline_id: WorldlineId::from_bytes([12u8; 32]),
            head_id: crate::head::make_head_id("replay-base"),
        };
        state.record_committed_ingress(head_key, [[13u8; 32]]);

        let replay_base = state.replay_base_from_initial();

        assert_eq!(replay_base.root, state.root);
        assert_eq!(
            crate::snapshot::compute_state_root_for_warp_state(
                replay_base.initial_state(),
                replay_base.root()
            ),
            crate::snapshot::compute_state_root_for_warp_state(state.initial_state(), state.root())
        );
        assert_eq!(
            crate::snapshot::compute_state_root_for_warp_state(
                replay_base.warp_state(),
                replay_base.root()
            ),
            crate::snapshot::compute_state_root_for_warp_state(state.initial_state(), state.root())
        );
        assert!(replay_base.last_snapshot.is_none());
        assert!(replay_base.tick_history.is_empty());
        assert!(replay_base.last_materialization.is_empty());
        assert!(replay_base.last_materialization_errors.is_empty());
        assert_eq!(replay_base.tx_counter, 0);
        assert!(replay_base.committed_ingress.is_empty());
    }
}
