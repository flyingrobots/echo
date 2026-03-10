// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Registry of worldline frontiers.
//!
//! The [`WorldlineRegistry`] owns all [`WorldlineFrontier`] instances in the
//! runtime. Each worldline has exactly one mutable frontier state. The registry
//! provides deterministic iteration order via `BTreeMap`.

use std::collections::BTreeMap;

use crate::worldline::WorldlineId;
use crate::worldline_state::{WorldlineFrontier, WorldlineState};

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
    /// Returns `false` if a worldline with this ID already exists (no-op).
    pub fn register(&mut self, worldline_id: WorldlineId, state: WorldlineState) -> bool {
        use std::collections::btree_map::Entry;
        match self.worldlines.entry(worldline_id) {
            Entry::Vacant(v) => {
                v.insert(WorldlineFrontier::new(worldline_id, state));
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    /// Returns a reference to the frontier for the given worldline.
    #[must_use]
    pub fn get(&self, worldline_id: &WorldlineId) -> Option<&WorldlineFrontier> {
        self.worldlines.get(worldline_id)
    }

    /// Returns a mutable reference to the frontier for the given worldline.
    pub fn get_mut(&mut self, worldline_id: &WorldlineId) -> Option<&mut WorldlineFrontier> {
        self.worldlines.get_mut(worldline_id)
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

    /// Iterates mutably over all worldlines in deterministic order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&WorldlineId, &mut WorldlineFrontier)> {
        self.worldlines.iter_mut()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn wl(n: u8) -> WorldlineId {
        WorldlineId([n; 32])
    }

    #[test]
    fn register_and_retrieve() {
        let mut reg = WorldlineRegistry::new();
        assert!(reg.is_empty());

        assert!(reg.register(wl(1), WorldlineState::empty()));
        assert_eq!(reg.len(), 1);
        assert!(reg.contains(&wl(1)));
        assert!(!reg.contains(&wl(2)));

        let frontier = reg.get(&wl(1)).unwrap();
        assert_eq!(frontier.worldline_id, wl(1));
        assert_eq!(frontier.frontier_tick, 0);
    }

    #[test]
    fn duplicate_registration_is_noop() {
        let mut reg = WorldlineRegistry::new();
        assert!(reg.register(wl(1), WorldlineState::empty()));
        assert!(!reg.register(wl(1), WorldlineState::empty()));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn deterministic_iteration_order() {
        let mut reg = WorldlineRegistry::new();
        // Insert in non-sorted order
        reg.register(wl(3), WorldlineState::empty());
        reg.register(wl(1), WorldlineState::empty());
        reg.register(wl(2), WorldlineState::empty());

        let ids: Vec<_> = reg.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![wl(1), wl(2), wl(3)]);
    }

    #[test]
    fn mutable_access_to_frontier() {
        let mut reg = WorldlineRegistry::new();
        reg.register(wl(1), WorldlineState::empty());

        let frontier = reg.get_mut(&wl(1)).unwrap();
        frontier.frontier_tick = 42;

        assert_eq!(reg.get(&wl(1)).unwrap().frontier_tick, 42);
    }
}
