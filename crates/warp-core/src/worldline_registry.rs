// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Registry of worldline frontiers.
//!
//! The [`WorldlineRegistry`] owns all [`WorldlineFrontier`] instances in the
//! runtime. Each worldline has exactly one mutable frontier state. The registry
//! provides deterministic iteration order via `BTreeMap`.

use std::collections::BTreeMap;
use std::fmt;

use crate::worldline::WorldlineId;
use crate::worldline_state::{WorldlineFrontier, WorldlineState};

/// Error returned when worldline registration conflicts with existing runtime state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegisterWorldlineError {
    /// The runtime already owns a frontier for this worldline.
    DuplicateWorldline(WorldlineId),
}

impl fmt::Display for RegisterWorldlineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateWorldline(worldline_id) => {
                write!(f, "worldline already registered: {worldline_id:?}")
            }
        }
    }
}

impl std::error::Error for RegisterWorldlineError {}

// =============================================================================
// WorldlineRegistry
// =============================================================================

/// Registry of all worldline frontiers in the runtime.
///
/// Worldlines are stored in a `BTreeMap` keyed by [`WorldlineId`], providing
/// deterministic iteration order for scheduling and inspection.
#[derive(Clone, Debug, Default)]
pub struct WorldlineRegistry {
    worldlines: BTreeMap<WorldlineId, WorldlineFrontier>,
}

impl WorldlineRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new worldline with the given initial state.
    ///
    /// # Errors
    ///
    /// Returns a `DuplicateWorldline` registration error if a worldline with
    /// this ID is already registered.
    pub fn register(
        &mut self,
        worldline_id: WorldlineId,
        state: WorldlineState,
    ) -> Result<(), RegisterWorldlineError> {
        use std::collections::btree_map::Entry;
        match self.worldlines.entry(worldline_id) {
            Entry::Vacant(v) => {
                let frontier_tick = state.current_tick();
                v.insert(WorldlineFrontier::at_tick(
                    worldline_id,
                    state,
                    frontier_tick,
                ));
                Ok(())
            }
            Entry::Occupied(_) => Err(RegisterWorldlineError::DuplicateWorldline(worldline_id)),
        }
    }

    /// Returns a reference to the frontier for the given worldline.
    #[must_use]
    pub fn get(&self, worldline_id: &WorldlineId) -> Option<&WorldlineFrontier> {
        self.worldlines.get(worldline_id)
    }

    /// Returns a mutable reference to the frontier for the given worldline.
    pub(crate) fn frontier_mut(
        &mut self,
        worldline_id: &WorldlineId,
    ) -> Option<&mut WorldlineFrontier> {
        self.worldlines.get_mut(worldline_id)
    }

    /// Replaces the stored frontier for a worldline.
    pub(crate) fn replace_frontier(
        &mut self,
        frontier: WorldlineFrontier,
    ) -> Option<WorldlineFrontier> {
        self.worldlines.insert(frontier.worldline_id(), frontier)
    }

    /// Returns the number of registered worldlines.
    #[must_use]
    pub fn len(&self) -> usize {
        self.worldlines.len()
    }

    /// Returns `true` if no worldlines are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.worldlines.is_empty()
    }

    /// Returns `true` if a worldline with the given ID is registered.
    #[must_use]
    pub fn contains(&self, worldline_id: &WorldlineId) -> bool {
        self.worldlines.contains_key(worldline_id)
    }

    /// Iterates over all worldlines in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&WorldlineId, &WorldlineFrontier)> {
        self.worldlines.iter()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::clock::WorldlineTick;
    use crate::receipt::TickReceipt;
    use crate::snapshot::Snapshot;
    use crate::tick_patch::{TickCommitStatus, WarpTickPatchV1};
    use crate::{blake3_empty, TxId};

    fn wl(n: u8) -> WorldlineId {
        WorldlineId([n; 32])
    }

    #[test]
    fn register_and_retrieve() {
        let mut reg = WorldlineRegistry::new();
        assert!(reg.is_empty());

        reg.register(wl(1), WorldlineState::empty()).unwrap();
        assert_eq!(reg.len(), 1);
        assert!(reg.contains(&wl(1)));
        assert!(!reg.contains(&wl(2)));

        let frontier = reg.get(&wl(1)).unwrap();
        assert_eq!(frontier.worldline_id(), wl(1));
        assert_eq!(frontier.frontier_tick(), WorldlineTick::ZERO);
    }

    #[test]
    fn duplicate_registration_returns_error() {
        let mut reg = WorldlineRegistry::new();
        reg.register(wl(1), WorldlineState::empty()).unwrap();
        assert_eq!(
            reg.register(wl(1), WorldlineState::empty()),
            Err(RegisterWorldlineError::DuplicateWorldline(wl(1)))
        );
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn deterministic_iteration_order() {
        let mut reg = WorldlineRegistry::new();
        // Insert in non-sorted order
        reg.register(wl(3), WorldlineState::empty()).unwrap();
        reg.register(wl(1), WorldlineState::empty()).unwrap();
        reg.register(wl(2), WorldlineState::empty()).unwrap();

        let ids: Vec<_> = reg.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![wl(1), wl(2), wl(3)]);
    }

    #[test]
    fn mutable_access_to_frontier() {
        let mut reg = WorldlineRegistry::new();
        reg.register(wl(1), WorldlineState::empty()).unwrap();

        let frontier = reg.frontier_mut(&wl(1)).unwrap();
        frontier.frontier_tick = WorldlineTick::from_raw(42);

        assert_eq!(
            reg.get(&wl(1)).unwrap().frontier_tick(),
            WorldlineTick::from_raw(42)
        );
    }

    #[test]
    fn register_preserves_restored_frontier_tick() {
        let mut state = WorldlineState::empty();
        let root = *state.root();
        state.tick_history.push((
            Snapshot {
                root,
                hash: [1; 32],
                state_root: [2; 32],
                parents: Vec::new(),
                plan_digest: [3; 32],
                decision_digest: [4; 32],
                rewrites_digest: [5; 32],
                patch_digest: [6; 32],
                policy_id: crate::POLICY_ID_NO_POLICY_V0,
                tx: TxId::from_raw(1),
            },
            TickReceipt::new(TxId::from_raw(1), Vec::new(), Vec::new()),
            WarpTickPatchV1::new(
                crate::POLICY_ID_NO_POLICY_V0,
                blake3_empty(),
                TickCommitStatus::Committed,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        ));

        let mut reg = WorldlineRegistry::new();
        reg.register(wl(1), state).unwrap();

        assert_eq!(
            reg.get(&wl(1)).unwrap().frontier_tick(),
            WorldlineTick::from_raw(1)
        );
    }
}
