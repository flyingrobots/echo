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

use crate::graph::GraphStore;
use crate::head::WriterHeadKey;
use crate::ident::{make_node_id, make_type_id, make_warp_id, Hash, NodeKey};
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
    pub(crate) committed_ingress: BTreeSet<(WriterHeadKey, Hash)>,
}

impl Default for WorldlineState {
    fn default() -> Self {
        Self::empty()
    }
}

impl WorldlineState {
    /// Creates a new worldline state from an existing warp state and root key.
    #[must_use]
    pub fn new(warp_state: WarpState, root: NodeKey) -> Self {
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

        Self::new(warp_state, root)
    }

    /// Returns a reference to the underlying warp state.
    #[must_use]
    pub fn warp_state(&self) -> &WarpState {
        &self.warp_state
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
}

impl From<WarpState> for WorldlineState {
    fn from(warp_state: WarpState) -> Self {
        let root = warp_state
            .iter_instances()
            .find(|(_, instance)| instance.parent.is_none())
            .map_or_else(
                || NodeKey {
                    warp_id: make_warp_id("root"),
                    local_id: make_node_id("root"),
                },
                |(warp_id, instance)| NodeKey {
                    warp_id: *warp_id,
                    local_id: instance.root_node,
                },
            );
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
    pub(crate) frontier_tick: u64,
}

impl WorldlineFrontier {
    /// Creates a new frontier for the given worldline.
    #[must_use]
    pub fn new(worldline_id: WorldlineId, state: WorldlineState) -> Self {
        Self {
            worldline_id,
            state,
            frontier_tick: 0,
        }
    }

    /// Returns the identity of this worldline.
    #[must_use]
    pub fn worldline_id(&self) -> WorldlineId {
        self.worldline_id
    }

    /// Returns the current frontier tick.
    #[must_use]
    pub fn frontier_tick(&self) -> u64 {
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
    pub(crate) fn advance_tick(&mut self) -> Option<u64> {
        self.frontier_tick = self.frontier_tick.checked_add(1)?;
        Some(self.frontier_tick)
    }

    /// Creates a frontier at a specific tick (used for fork/rebuild).
    #[must_use]
    pub fn at_tick(worldline_id: WorldlineId, state: WorldlineState, tick: u64) -> Self {
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
    use crate::warp_state::WarpState;

    fn wl(n: u8) -> WorldlineId {
        WorldlineId([n; 32])
    }

    #[test]
    fn worldline_state_wraps_warp_state() {
        let warp = WarpState::new();
        let ws = WorldlineState::from(warp);
        // WorldlineState is a transparent wrapper
        assert_eq!(ws.root().local_id, make_node_id("root"));
    }

    #[test]
    fn worldline_frontier_starts_at_tick_zero() {
        let frontier = WorldlineFrontier::new(wl(1), WorldlineState::empty());
        assert_eq!(frontier.frontier_tick(), 0);
        assert_eq!(frontier.worldline_id(), wl(1));
    }

    #[test]
    fn worldline_frontier_at_tick() {
        let frontier = WorldlineFrontier::at_tick(wl(1), WorldlineState::empty(), 42);
        assert_eq!(frontier.frontier_tick(), 42);
    }

    #[test]
    fn from_warp_state() {
        let ws: WorldlineState = WarpState::new().into();
        assert_eq!(ws.root().warp_id, make_warp_id("root"));
        assert!(ws.tick_history().is_empty());
    }
}
