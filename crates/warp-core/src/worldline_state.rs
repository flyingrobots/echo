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
use std::sync::Arc;

use thiserror::Error;

use crate::clock::WorldlineTick;
use crate::execution_evidence::ExecutionFootprintEvidence;
use crate::graph::GraphStore;
use crate::head::WriterHeadKey;
use crate::ident::{make_node_id, make_type_id, make_warp_id, Hash, NodeId, NodeKey};
use crate::materialization::{ChannelConflict, FinalizedChannel};
use crate::receipt::TickReceipt;
use crate::record::NodeRecord;
use crate::snapshot::Snapshot;
use crate::tick_patch::{StateJournal, WarpTickPatchV1};
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
    /// Private to this module: every mutation goes through
    /// [`Self::warp_state_mut`], which drops the retained state root.
    warp_state: WarpState,
    /// Root key for snapshot hashing and commit execution.
    pub(crate) root: NodeKey,
    /// Initial worldline state preserved for replay.
    ///
    /// The boundary never changes after construction, so clones of this state
    /// share it instead of copying it.
    pub(crate) initial_state: Arc<WarpState>,
    /// Most recent snapshot committed for this worldline.
    pub(crate) last_snapshot: Option<Snapshot>,
    /// Sequential history of committed ticks for this worldline.
    pub(crate) tick_history: Vec<(Snapshot, TickReceipt, WarpTickPatchV1)>,
    /// Canonically ordered per-Action footprints from the latest execution.
    pub(crate) last_execution_footprints: Arc<[ExecutionFootprintEvidence]>,
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
    /// The state root of `warp_state` under the given root key, when it is
    /// known without hashing. A commit retains the root it computed; any
    /// mutable access to the graph drops it.
    retained_state_root: Option<(NodeKey, Hash)>,
    /// Experimental incremental application root (M1, shadow mode).
    #[cfg(feature = "application_root_shadow")]
    application_root: ApplicationRootShadow,
    /// Counts whole-state root computations for tests, excluding the hash by
    /// which a debug build checks a retained root.
    #[cfg(any(test, feature = "host_test"))]
    state_root_computations: TestCallCount,
}

/// The fields from which a worldline state is assembled outside this module.
pub(crate) struct WorldlineStateParts {
    pub(crate) warp_state: WarpState,
    pub(crate) root: NodeKey,
    pub(crate) initial_state: Arc<WarpState>,
    pub(crate) last_snapshot: Option<Snapshot>,
    pub(crate) tick_history: Vec<(Snapshot, TickReceipt, WarpTickPatchV1)>,
    pub(crate) last_materialization: Vec<FinalizedChannel>,
    pub(crate) last_materialization_errors: Vec<ChannelConflict>,
    pub(crate) tx_counter: u64,
    pub(crate) committed_ingress: BTreeSet<(WriterHeadKey, Hash)>,
    pub(crate) last_execution_footprints: Arc<[ExecutionFootprintEvidence]>,
    /// The application trie handed back by the runtime commit guard, or
    /// `None` to rebuild it from `warp_state`.
    #[cfg(feature = "application_root_shadow")]
    pub(crate) application_root: Option<ApplicationRootShadow>,
}

/// Test instrumentation that counts calls.
///
/// It is not runtime state, so it has a constant debug rendering: two values
/// that differ only in instrumentation render alike.
#[cfg(any(test, feature = "host_test"))]
#[derive(Default)]
pub(crate) struct TestCallCount(std::sync::atomic::AtomicUsize);

#[cfg(any(test, feature = "host_test"))]
impl Clone for TestCallCount {
    fn clone(&self) -> Self {
        Self(std::sync::atomic::AtomicUsize::new(self.get()))
    }
}

#[cfg(any(test, feature = "host_test"))]
impl TestCallCount {
    pub(crate) fn increment(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub(crate) fn get(&self) -> usize {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(crate) fn reset(&self) {
        self.0.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(any(test, feature = "host_test"))]
impl std::fmt::Debug for TestCallCount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TestCallCount")
    }
}

#[cfg(feature = "application_root_shadow")]
/// The application trie of the last graph it is known to describe.
///
/// Like the retained state root, mutable graph access marks it no longer
/// current; a commit that knows its write set advances it incrementally, and
/// anything else falls back to a full rebuild. Rollback markers, replay
/// checkpoints and forks clone it in constant time.
#[derive(Clone, Default)]
pub(crate) struct ApplicationRootShadow {
    trie: crate::application_root::ApplicationTrie,
    current: bool,
}

#[cfg(feature = "application_root_shadow")]
impl std::fmt::Debug for ApplicationRootShadow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApplicationRootShadow")
            .field("current", &self.current)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "application_root_shadow")]
impl ApplicationRootShadow {
    /// The same trie, no longer claimed current: the graph it described is
    /// being changed by a commit that will advance it.
    pub(crate) fn stale(mut self) -> Self {
        self.current = false;
        self
    }

    fn of(warp_state: &WarpState) -> Self {
        Self {
            trie: crate::application_root::ApplicationTrie::rebuild(warp_state),
            current: true,
        }
    }
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
        #[cfg(feature = "application_root_shadow")]
        let application_root = ApplicationRootShadow::of(&warp_state);
        Self {
            initial_state: Arc::new(warp_state.clone()),
            warp_state,
            root,
            last_snapshot: None,
            tick_history: Vec::new(),
            last_execution_footprints: Arc::default(),
            last_materialization: Vec::new(),
            last_materialization_errors: Vec::new(),
            tx_counter: 0,
            committed_ingress: BTreeSet::new(),
            retained_state_root: None,
            #[cfg(feature = "application_root_shadow")]
            application_root,
            #[cfg(any(test, feature = "host_test"))]
            state_root_computations: TestCallCount::default(),
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

    /// Returns the canonical hash of WARP state reachable from this worldline's root.
    ///
    /// Detached nodes and their attachments are outside this hash boundary.
    /// Equal state roots therefore do not prove equal stores. A committed
    /// detached write remains bound by the tick patch digest and commit identity.
    ///
    /// A call hashes the whole reachable state, unless the state holds a root
    /// retained by its last commit: then the retained root is returned
    /// without hashing. Debug builds hash in that case as well, to check the
    /// retained root.
    #[must_use]
    pub fn state_root(&self) -> Hash {
        if let Some((root, state_root)) = self.retained_state_root {
            if root == self.root {
                debug_assert_eq!(
                    state_root,
                    crate::snapshot::compute_state_root_for_warp_state(
                        &self.warp_state,
                        &self.root
                    ),
                    "the retained state root must be the root of the current graph"
                );
                return state_root;
            }
        }
        #[cfg(any(test, feature = "host_test"))]
        self.state_root_computations.increment();
        crate::snapshot::compute_state_root_for_warp_state(&self.warp_state, &self.root)
    }

    /// Returns the graph for mutation and drops the retained state root.
    pub(crate) fn warp_state_mut(&mut self) -> &mut WarpState {
        self.retained_state_root = None;
        #[cfg(feature = "application_root_shadow")]
        {
            self.application_root.current = false;
        }
        &mut self.warp_state
    }

    /// Retains `state_root` as the root of the current graph.
    ///
    /// The caller has just computed it from this state. Later calls to
    /// [`Self::state_root`] return it without hashing until the graph is
    /// accessed mutably or the root key changes.
    pub(crate) fn retain_state_root(&mut self, state_root: Hash) {
        self.retained_state_root = Some((self.root, state_root));
    }

    /// Takes the application trie for the runtime commit guard, which
    /// rebuilds this state from parts.
    #[cfg(feature = "application_root_shadow")]
    pub(crate) fn take_application_root(&mut self) -> ApplicationRootShadow {
        std::mem::take(&mut self.application_root)
    }

    /// Whether the application trie is known to describe the current graph.
    #[cfg(feature = "application_root_shadow")]
    pub(crate) fn application_root_is_current(&self) -> bool {
        self.application_root.current
    }

    /// Advances the application trie after a commit that wrote `ops` and
    /// declared `out_slots`, given whether it was current before the commit.
    ///
    /// A trie that was current advances incrementally from the commit's
    /// written records. Otherwise, or when an op changes a whole warp
    /// instance, the trie is rebuilt from every record.
    #[cfg(feature = "application_root_shadow")]
    pub(crate) fn advance_application_root(
        &mut self,
        was_current: bool,
        ops: &[crate::tick_patch::WarpOp],
        out_slots: &[crate::tick_patch::SlotId],
    ) {
        let advanced = if was_current {
            self.application_root
                .trie
                .advanced(&self.warp_state, ops, out_slots)
        } else {
            None
        };
        self.application_root = advanced.map_or_else(
            || ApplicationRootShadow::of(&self.warp_state),
            |trie| ApplicationRootShadow {
                trie,
                current: true,
            },
        );
        #[cfg(feature = "delta_validate")]
        assert_eq!(
            self.application_root.trie.root(),
            crate::application_root::ApplicationTrie::rebuild(&self.warp_state).root(),
            "the incremental application root must equal its full rebuild"
        );
    }

    /// Returns the experimental application root (M1, shadow mode) of every
    /// node, edge and attachment record in this state, reachable or not.
    ///
    /// It is computed incrementally across commits that know their write
    /// set, and by a full rebuild otherwise. It is not the state root, which
    /// it neither replaces nor affects.
    #[cfg(feature = "application_root_shadow")]
    #[must_use]
    pub fn application_root(&self) -> Hash {
        if self.application_root.current {
            self.application_root.trie.root()
        } else {
            self.application_root_rebuilt()
        }
    }

    /// Returns the application root and a canonical membership or absence
    /// opening for `slot` against it.
    ///
    /// # Errors
    ///
    /// Returns
    /// [`ApplicationOpeningError::OutOfScope`](crate::application_root::ApplicationOpeningError::OutOfScope)
    /// for a boundary port slot.
    #[cfg(feature = "application_root_shadow")]
    pub fn application_opening(
        &self,
        slot: &crate::tick_patch::SlotId,
    ) -> Result<(Hash, Vec<u8>), crate::application_root::ApplicationOpeningError> {
        let rebuilt;
        let trie = if self.application_root.current {
            &self.application_root.trie
        } else {
            rebuilt = crate::application_root::ApplicationTrie::rebuild(&self.warp_state);
            &rebuilt
        };
        Ok((trie.root(), trie.opening(slot)?))
    }

    /// The application root by a full rebuild, ignoring the incremental trie.
    #[cfg(feature = "application_root_shadow")]
    #[must_use]
    pub fn application_root_rebuilt(&self) -> Hash {
        crate::application_root::ApplicationTrie::rebuild(&self.warp_state).root()
    }

    /// Whether [`Self::application_root`] answers from the incremental trie,
    /// without a rebuild.
    #[cfg(feature = "application_root_shadow")]
    #[must_use]
    pub fn application_root_is_incremental(&self) -> bool {
        self.application_root.current
    }

    /// Applies `patch` to the graph, recording in `journal` what is needed to
    /// take it back. On error the state, including its retained root, is
    /// unchanged.
    pub(crate) fn apply_patch_journaled(
        &mut self,
        patch: &WarpTickPatchV1,
        journal: &mut StateJournal,
    ) -> Result<(), crate::tick_patch::TickPatchError> {
        #[cfg(feature = "application_root_shadow")]
        let was_current = self.application_root.current;
        patch.apply_journaled(&mut self.warp_state, journal)?;
        self.retained_state_root = None;
        #[cfg(feature = "application_root_shadow")]
        self.advance_application_root(was_current, patch.ops(), patch.out_slots());
        Ok(())
    }

    /// Assembles a state from parts built outside this module.
    pub(crate) fn from_parts(parts: WorldlineStateParts) -> Self {
        #[cfg(feature = "application_root_shadow")]
        let application_root = parts
            .application_root
            .unwrap_or_else(|| ApplicationRootShadow::of(&parts.warp_state));
        Self {
            warp_state: parts.warp_state,
            root: parts.root,
            initial_state: parts.initial_state,
            last_snapshot: parts.last_snapshot,
            tick_history: parts.tick_history,
            last_materialization: parts.last_materialization,
            last_materialization_errors: parts.last_materialization_errors,
            tx_counter: parts.tx_counter,
            committed_ingress: parts.committed_ingress,
            last_execution_footprints: parts.last_execution_footprints,
            retained_state_root: None,
            #[cfg(feature = "application_root_shadow")]
            application_root,
            #[cfg(any(test, feature = "host_test"))]
            state_root_computations: TestCallCount::default(),
        }
    }

    /// Resets test instrumentation for whole-state root computations.
    #[cfg(any(test, feature = "host_test"))]
    pub fn reset_state_root_computation_count_for_test(&self) {
        self.state_root_computations.reset();
    }

    /// Returns the number of whole-state root computations on this state
    /// since the last instrumentation reset.
    ///
    /// It counts the computations that every build performs. It does not
    /// count the extra hash by which a debug build checks a retained root, so
    /// the count is the same in debug and release builds.
    #[cfg(any(test, feature = "host_test"))]
    #[must_use]
    pub fn state_root_computation_count_for_test(&self) -> usize {
        self.state_root_computations.get()
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

    /// Returns canonically ordered per-Action footprints from the latest
    /// execution attempt against this worldline.
    #[must_use]
    pub fn last_execution_footprints(&self) -> &[ExecutionFootprintEvidence] {
        &self.last_execution_footprints
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
            last_execution_footprints: self.last_execution_footprints.clone(),
            last_materialization: self.last_materialization.clone(),
            last_materialization_errors: self.last_materialization_errors.clone(),
            tx_counter: self.tx_counter,
            committed_ingress: BTreeSet::new(),
            retained_state_root: self.retained_state_root,
            #[cfg(feature = "application_root_shadow")]
            application_root: self.application_root.clone(),
            #[cfg(any(test, feature = "host_test"))]
            state_root_computations: TestCallCount::default(),
        }
    }

    /// Clones every field except the graph, the append-only commit history and
    /// the ingress ledger, which a rollback marker restores from the live
    /// frontier.
    fn clone_without_graph_or_append_only_history(&self) -> Self {
        Self {
            warp_state: WarpState::new(),
            root: self.root,
            initial_state: self.initial_state.clone(),
            last_snapshot: self.last_snapshot.clone(),
            tick_history: Vec::new(),
            last_execution_footprints: self.last_execution_footprints.clone(),
            last_materialization: self.last_materialization.clone(),
            last_materialization_errors: self.last_materialization_errors.clone(),
            tx_counter: self.tx_counter,
            committed_ingress: BTreeSet::new(),
            retained_state_root: self.retained_state_root,
            #[cfg(feature = "application_root_shadow")]
            application_root: self.application_root.clone(),
            #[cfg(any(test, feature = "host_test"))]
            state_root_computations: TestCallCount::default(),
        }
    }

    /// Clones the canonical `U0` replay base used for suffix replay.
    ///
    /// This preserves the deterministic initial boundary and root coordinate
    /// while dropping frontier-only materialization and replay metadata.
    pub(crate) fn replay_base_from_initial(&self) -> Self {
        let initial_state = Arc::clone(&self.initial_state);
        #[cfg(feature = "application_root_shadow")]
        let application_root = ApplicationRootShadow::of(&initial_state);
        Self {
            warp_state: (*initial_state).clone(),
            root: self.root,
            initial_state,
            last_snapshot: None,
            tick_history: Vec::new(),
            last_execution_footprints: Arc::default(),
            last_materialization: Vec::new(),
            last_materialization_errors: Vec::new(),
            tx_counter: 0,
            committed_ingress: BTreeSet::new(),
            retained_state_root: None,
            #[cfg(feature = "application_root_shadow")]
            application_root,
            #[cfg(any(test, feature = "host_test"))]
            state_root_computations: TestCallCount::default(),
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

/// Rollback marker for one worldline frontier.
///
/// A committing frontier only appends to `tick_history` and only inserts into
/// `committed_ingress`. The marker therefore records the history length
/// instead of copying either container, so its cost does not grow with the
/// number of committed ticks. Restoring truncates the live history and removes
/// the ingress ids that the failed attempt journaled.
///
/// The marker does not copy the graph either. It restores the graph from its
/// [`StateJournal`], so every mutation of the frontier's graph made after the
/// marker was captured must be recorded there: apply patches with
/// [`WarpTickPatchV1::apply_journaled`], and call
/// [`StateJournal::retain_copy`] before any other mutation.
#[derive(Debug, Clone)]
pub(crate) struct WorldlineFrontierRollback {
    /// The frontier at capture, with an empty graph and no history.
    frontier: WorldlineFrontier,
    tick_history_len: usize,
    state_journal: StateJournal,
    /// The graph at capture, kept only to check the journal in test and
    /// `delta_validate` builds. Ordinary builds, debug included, never copy
    /// the graph per tick (#903).
    #[cfg(any(test, feature = "delta_validate"))]
    graph_at_capture: WarpState,
}

impl WorldlineFrontierRollback {
    /// Returns the journal that must record every graph mutation made after
    /// this marker was captured.
    pub(crate) fn state_journal_mut(&mut self) -> &mut StateJournal {
        &mut self.state_journal
    }

    /// Counts append-only history elements copied into this marker.
    #[cfg(test)]
    pub(crate) fn retained_history_entries(&self) -> usize {
        self.frontier.state.tick_history.len() + self.frontier.state.committed_ingress.len()
    }

    /// Returns whether this marker holds a copy of the graph for rollback.
    ///
    /// The copy that debug builds keep to check the journal does not count.
    #[cfg(test)]
    pub(crate) fn retains_graph_copy(&self) -> bool {
        self.state_journal.holds_state_copy()
            || self
                .frontier
                .state
                .warp_state
                .iter_instances()
                .next()
                .is_some()
    }
}

impl WorldlineFrontier {
    /// Captures a rollback marker without copying the graph or append-only
    /// history.
    pub(crate) fn rollback_marker(&self) -> WorldlineFrontierRollback {
        WorldlineFrontierRollback {
            frontier: Self {
                worldline_id: self.worldline_id,
                state: self.state.clone_without_graph_or_append_only_history(),
                frontier_tick: self.frontier_tick,
            },
            tick_history_len: self.state.tick_history.len(),
            state_journal: StateJournal::default(),
            #[cfg(any(test, feature = "delta_validate"))]
            graph_at_capture: self.state.warp_state.clone(),
        }
    }

    /// Restores this frontier to `marker`.
    ///
    /// `attempted_ingress` names every ingress id the abandoned attempt
    /// inserted into this frontier's ledger. Each id must have been absent
    /// when the marker was captured.
    pub(crate) fn restore_rollback_marker<I>(
        &mut self,
        marker: WorldlineFrontierRollback,
        attempted_ingress: I,
    ) where
        I: IntoIterator<Item = (WriterHeadKey, Hash)>,
    {
        let mut tick_history = std::mem::take(&mut self.state.tick_history);
        debug_assert!(
            tick_history.len() >= marker.tick_history_len,
            "committed history must not shrink below its rollback marker"
        );
        tick_history.truncate(marker.tick_history_len);
        let mut committed_ingress = std::mem::take(&mut self.state.committed_ingress);
        for entry in attempted_ingress {
            committed_ingress.remove(&entry);
        }
        let mut warp_state = std::mem::take(&mut self.state.warp_state);
        marker.state_journal.restore(&mut warp_state);
        #[cfg(any(test, feature = "delta_validate"))]
        assert_eq!(
            format!("{warp_state:?}"),
            format!("{:?}", marker.graph_at_capture),
            "a graph mutation after the rollback marker was not journaled"
        );
        *self = marker.frontier;
        self.state.warp_state = warp_state;
        self.state.tick_history = tick_history;
        self.state.committed_ingress = committed_ingress;
    }

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
    fn detached_creates_preserve_state_root_but_change_patch_and_commit_identity() {
        use crate::attachment::{AtomPayload, AttachmentValue};
        use crate::snapshot::compute_commit_hash_v2;
        use crate::tick_patch::{TickCommitStatus, WarpOp};

        let mut state = WorldlineState::empty();
        let root = state.state_root();
        let mut patch_digests = Vec::new();
        let mut commit_ids = Vec::new();
        for label in ["detached-a", "detached-b"] {
            let node = NodeKey {
                warp_id: state.root().warp_id,
                local_id: make_node_id(label),
            };
            let slot = AttachmentKey::node_alpha(node);
            let patch = WarpTickPatchV1::new(
                0,
                [1; 32],
                TickCommitStatus::Committed,
                vec![crate::SlotId::Node(node), crate::SlotId::Attachment(slot)],
                vec![crate::SlotId::Node(node), crate::SlotId::Attachment(slot)],
                vec![
                    WarpOp::UpsertNode {
                        node,
                        record: NodeRecord {
                            ty: make_type_id("cell"),
                        },
                    },
                    WarpOp::SetAttachment {
                        key: slot,
                        value: Some(AttachmentValue::Atom(AtomPayload::new(
                            make_type_id("value"),
                            bytes::Bytes::from_static(b"retained"),
                        ))),
                    },
                ],
            );
            let result = patch.apply_to_state(&mut state.warp_state);
            assert!(result.is_ok(), "detached create must apply: {result:?}");
            assert_eq!(state.state_root(), root);
            assert!(state.store(&node.warp_id).is_some_and(|store| {
                store.node(&node.local_id).is_some()
                    && store.node_attachment(&node.local_id).is_some()
            }));
            patch_digests.push(patch.digest());
            // Hold root, parents, and policy fixed to isolate the patch binding.
            commit_ids.push(compute_commit_hash_v2(&root, &[], &patch.digest(), 0));
        }
        assert_ne!(patch_digests[0], patch_digests[1]);
        assert_ne!(commit_ids[0], commit_ids[1]);
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
    fn a_retained_state_root_is_dropped_by_any_mutable_graph_access() {
        let mut state = WorldlineState::empty();
        let root = state.state_root();
        assert_eq!(state.state_root_computation_count_for_test(), 1);

        state.retain_state_root(root);
        assert_eq!(state.state_root(), root);
        assert_eq!(
            state.state_root_computation_count_for_test(),
            1,
            "a retained root is returned without hashing"
        );
        assert_eq!(state.clone().state_root_computation_count_for_test(), 1);

        let node = *state.root();
        if let Some(store) = state.warp_state_mut().store_mut(&node.warp_id) {
            store.insert_node(
                crate::ident::make_node_id("retained-root-extra"),
                crate::record::NodeRecord {
                    ty: crate::ident::make_type_id("retained-root-extra"),
                },
            );
        }
        let _ = state.state_root();
        assert_eq!(
            state.state_root_computation_count_for_test(),
            2,
            "mutable access to the graph drops the retained root"
        );
    }

    #[test]
    fn a_retained_state_root_is_bound_to_the_root_key() {
        let mut state = WorldlineState::empty();
        let root = state.state_root();
        state.retain_state_root(root);
        state.reset_state_root_computation_count_for_test();

        state.root = NodeKey {
            warp_id: state.root.warp_id,
            local_id: crate::ident::make_node_id("retained-root-other-key"),
        };
        let _ = state.state_root();
        assert_eq!(
            state.state_root_computation_count_for_test(),
            1,
            "a root retained under another root key is not reused"
        );
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
        state.last_execution_footprints = vec![sample_execution_evidence(*state.root())].into();
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
        assert_eq!(
            checkpoint.last_execution_footprints,
            state.last_execution_footprints
        );
        assert!(checkpoint.committed_ingress.is_empty());
    }

    #[test]
    fn replay_base_from_initial_resets_frontier_metadata() {
        let mut state = WorldlineState::empty();
        state.last_execution_footprints = vec![sample_execution_evidence(*state.root())].into();
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
        assert!(replay_base.last_execution_footprints.is_empty());
        assert!(replay_base.last_materialization.is_empty());
        assert!(replay_base.last_materialization_errors.is_empty());
        assert_eq!(replay_base.tx_counter, 0);
        assert!(replay_base.committed_ingress.is_empty());
    }

    fn sample_execution_evidence(node: NodeKey) -> crate::ExecutionFootprintEvidence {
        let mut actual = crate::ActualFootprint::new();
        actual.record_node_read(node.local_id);
        crate::ExecutionFootprintEvidence::new(
            crate::ExecutionEvidenceKey::new(
                7,
                node.warp_id,
                node.local_id,
                crate::OpOrigin::default(),
            ),
            actual,
            crate::ActualFootprintPosture::RecordedWithoutEnforcement,
        )
    }

    #[cfg(feature = "application_root_shadow")]
    #[allow(clippy::panic, clippy::unnecessary_wraps)]
    mod application_root_shadow {
        use super::*;
        use crate::attachment::{AtomPayload, AttachmentValue};
        use crate::record::EdgeRecord;
        use crate::tick_patch::{PortalInit, StateJournal, TickCommitStatus, WarpOp};

        fn patch(ops: Vec<WarpOp>) -> WarpTickPatchV1 {
            // Declare nothing, as runtime-ingress materialization does: the
            // trie must read every write from the ops.
            WarpTickPatchV1::new(
                0,
                [3; 32],
                TickCommitStatus::Committed,
                Vec::new(),
                Vec::new(),
                ops,
            )
        }

        fn atom(bytes: &'static [u8]) -> Option<AttachmentValue> {
            Some(AttachmentValue::Atom(AtomPayload::new(
                make_type_id("value"),
                bytes::Bytes::from_static(bytes),
            )))
        }

        fn check(state: &WorldlineState, incremental: bool, step: &str) {
            assert_eq!(
                state.application_root_is_incremental(),
                incremental,
                "{step}"
            );
            assert_eq!(
                state.application_root(),
                state.application_root_rebuilt(),
                "{step}"
            );
        }

        #[test]
        fn application_root_follows_ordinary_and_structural_patches() {
            let mut state = WorldlineState::empty();
            let warp_id = state.root().warp_id;
            let root_node = state.root().local_id;
            let legacy_before = state.state_root();
            check(&state, true, "construction");
            let node = |label: &str| NodeKey {
                warp_id,
                local_id: make_node_id(label),
            };
            let (a, b) = (node("a"), node("b"));
            let edge = EdgeRecord {
                id: crate::make_edge_id("a-b"),
                from: a.local_id,
                to: b.local_id,
                ty: make_type_id("link"),
            };
            let steps: Vec<(&str, Vec<WarpOp>, bool)> = vec![
                (
                    "upsert nodes and attachments",
                    vec![
                        WarpOp::UpsertNode {
                            node: a,
                            record: NodeRecord {
                                ty: make_type_id("cell"),
                            },
                        },
                        WarpOp::UpsertNode {
                            node: b,
                            record: NodeRecord {
                                ty: make_type_id("cell"),
                            },
                        },
                        WarpOp::SetAttachment {
                            key: AttachmentKey::node_alpha(a),
                            value: atom(b"one"),
                        },
                    ],
                    true,
                ),
                (
                    "upsert an edge",
                    vec![WarpOp::UpsertEdge {
                        warp_id,
                        record: edge.clone(),
                    }],
                    true,
                ),
                (
                    "replace an attachment",
                    vec![WarpOp::SetAttachment {
                        key: AttachmentKey::node_alpha(a),
                        value: atom(b"two"),
                    }],
                    true,
                ),
                (
                    "delete the edge",
                    vec![WarpOp::DeleteEdge {
                        warp_id,
                        from: a.local_id,
                        edge_id: edge.id,
                    }],
                    true,
                ),
                (
                    "delete a node with its attachment",
                    vec![WarpOp::DeleteNode { node: a }],
                    true,
                ),
                (
                    "open a portal to a new child instance",
                    vec![WarpOp::OpenPortal {
                        key: AttachmentKey::node_alpha(NodeKey {
                            warp_id,
                            local_id: root_node,
                        }),
                        child_warp: crate::make_warp_id("child"),
                        child_root: make_node_id("child-root"),
                        init: PortalInit::Empty {
                            root_record: NodeRecord {
                                ty: make_type_id("child"),
                            },
                        },
                    }],
                    true,
                ),
                (
                    "delete the child instance",
                    vec![
                        WarpOp::SetAttachment {
                            key: AttachmentKey::node_alpha(NodeKey {
                                warp_id,
                                local_id: root_node,
                            }),
                            value: None,
                        },
                        WarpOp::DeleteWarpInstance {
                            warp_id: crate::make_warp_id("child"),
                        },
                    ],
                    true,
                ),
            ];
            for (step, ops, incremental) in steps {
                let mut journal = StateJournal::default();
                state
                    .apply_patch_journaled(&patch(ops), &mut journal)
                    .unwrap_or_else(|error| panic!("{step}: {error:?}"));
                check(&state, incremental, step);
            }
            // An untracked mutation leaves the trie stale; the root is still
            // answered correctly, by a rebuild.
            if let Some(store) = state.warp_state_mut().store_mut(&warp_id) {
                store.insert_node(
                    make_node_id("untracked"),
                    NodeRecord {
                        ty: make_type_id("cell"),
                    },
                );
            }
            check(&state, false, "untracked mutation");
            let mut journal = StateJournal::default();
            state
                .apply_patch_journaled(
                    &patch(vec![WarpOp::SetAttachment {
                        key: AttachmentKey::node_alpha(b),
                        value: atom(b"three"),
                    }]),
                    &mut journal,
                )
                .unwrap_or_else(|error| panic!("{error:?}"));
            check(&state, true, "a patch after a stale trie rebuilds once");
            // Every record here is unreachable from the root, so the legacy
            // reachable root is unchanged while the application root, which
            // covers every record, is not.
            assert_eq!(state.state_root(), legacy_before);
            assert_ne!(
                state.application_root(),
                WorldlineState::empty().application_root()
            );
        }

        #[test]
        fn application_root_rolls_back_with_its_frontier() {
            let mut frontier = WorldlineFrontier::new(wl(9), WorldlineState::empty());
            let warp_id = frontier.state().root().warp_id;
            let before = frontier.state().application_root();
            let mut marker = frontier.rollback_marker();
            let node = NodeKey {
                warp_id,
                local_id: make_node_id("attempt"),
            };
            frontier
                .state_mut()
                .apply_patch_journaled(
                    &patch(vec![WarpOp::UpsertNode {
                        node,
                        record: NodeRecord {
                            ty: make_type_id("cell"),
                        },
                    }]),
                    marker.state_journal_mut(),
                )
                .unwrap_or_else(|error| panic!("{error:?}"));
            assert_ne!(frontier.state().application_root(), before);
            frontier.restore_rollback_marker(marker, std::iter::empty());
            assert_eq!(frontier.state().application_root(), before);
            check(frontier.state(), true, "after rollback");
        }
    }
}
