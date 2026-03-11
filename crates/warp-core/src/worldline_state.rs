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
#[derive(Debug, Clone, Default)]
pub struct WorldlineState {
    /// The underlying multi-instance warp state.
    pub(crate) warp_state: WarpState,
}

impl WorldlineState {
    /// Creates a new worldline state from an existing warp state.
    #[must_use]
    pub fn new(warp_state: WarpState) -> Self {
        Self { warp_state }
    }

    /// Creates an empty worldline state.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns a reference to the underlying warp state.
    #[must_use]
    pub fn warp_state(&self) -> &WarpState {
        &self.warp_state
    }
}

impl From<WarpState> for WorldlineState {
    fn from(warp_state: WarpState) -> Self {
        Self { warp_state }
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
        let ws = WorldlineState::new(warp);
        // WorldlineState is a transparent wrapper
        assert!(ws.warp_state.stores.is_empty());
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
        assert!(ws.warp_state.stores.is_empty());
    }
}
