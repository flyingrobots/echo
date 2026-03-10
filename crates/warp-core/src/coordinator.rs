// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Worldline-aware scheduler coordinator (ADR-0008 Phase 2).
//!
//! The [`SchedulerCoordinator`] runs the **SuperTick** loop: iterating over
//! runnable writer heads in canonical `(worldline_id, head_id)` order and
//! dispatching commit operations against each head's worldline frontier.
//!
//! # Serial Canonical Scheduling
//!
//! This phase permits multiple writer heads per worldline but runs them
//! **serially** in canonical order. Same-worldline co-advance optimization
//! is deferred until footprint machinery (Phase 9) is in place.
//!
//! # Migration
//!
//! The coordinator exists in parallel with the current [`Engine`](super::Engine).
//! Full integration happens incrementally across Phases 2–4. In Phase 2, the
//! coordinator demonstrates correct scheduling order and frontier advancement.

use crate::head::{PlaybackHeadRegistry, RunnableWriterSet, WriterHeadKey};
use crate::worldline_registry::WorldlineRegistry;

// =============================================================================
// WorldlineRuntime
// =============================================================================

/// Top-level runtime state for the worldline model.
///
/// Bundles the worldline registry, head registry, runnable set, and global tick
/// into one coherent object. This is the structure that the coordinator
/// operates on during each SuperTick.
#[derive(Clone, Debug, Default)]
pub struct WorldlineRuntime {
    /// Registry of all worldline frontiers.
    pub worldlines: WorldlineRegistry,
    /// Registry of all writer heads.
    pub heads: PlaybackHeadRegistry,
    /// Ordered set of currently runnable (non-paused) writer heads.
    pub runnable: RunnableWriterSet,
    /// Global tick counter (metadata only; not per-worldline identity).
    pub global_tick: u64,
}

impl WorldlineRuntime {
    /// Creates an empty runtime.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds the runnable set from the current head registry.
    pub fn refresh_runnable(&mut self) {
        self.runnable.rebuild(&self.heads);
    }
}

// =============================================================================
// StepRecord
// =============================================================================

/// Record of a single head being stepped during a SuperTick.
///
/// This is the coordinator's output: an ordered log of which heads were
/// stepped and what their worldline tick was after the step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepRecord {
    /// The head that was stepped.
    pub head_key: WriterHeadKey,
    /// The worldline tick after this step.
    pub frontier_tick_after: u64,
}

// =============================================================================
// SchedulerCoordinator
// =============================================================================

/// Coordinator for worldline-aware serial canonical scheduling.
///
/// Each call to [`super_tick()`](SchedulerCoordinator::super_tick) iterates
/// all runnable writer heads in canonical `(worldline_id, head_id)` order,
/// advances each head's worldline frontier tick, and records the step.
///
/// # Commit Integration
///
/// In Phase 2, the coordinator demonstrates correct scheduling order and
/// frontier advancement. Full integration with the engine's commit pipeline
/// (via `commit_with_state`) is wired incrementally in later phases.
pub struct SchedulerCoordinator;

impl SchedulerCoordinator {
    /// Executes one SuperTick: iterates all runnable writer heads in canonical
    /// order and advances each head's worldline frontier.
    ///
    /// Returns an ordered list of [`StepRecord`]s documenting which heads were
    /// stepped and in what order.
    ///
    /// # Panics
    ///
    /// Panics if a runnable head references a worldline that is not in the registry.
    /// This is a programmer error (invariant violation), not a runtime condition.
    #[allow(clippy::expect_used)]
    pub fn super_tick(runtime: &mut WorldlineRuntime) -> Vec<StepRecord> {
        let mut records = Vec::new();

        // Snapshot the runnable keys so we don't hold an immutable borrow
        // on runtime while mutating worldline frontiers.
        let keys: Vec<WriterHeadKey> = runtime.runnable.iter().copied().collect();

        for key in &keys {
            let frontier = runtime
                .worldlines
                .get_mut(&key.worldline_id)
                .expect("runnable head references unregistered worldline (invariant violation)");

            frontier.frontier_tick += 1;

            records.push(StepRecord {
                head_key: *key,
                frontier_tick_after: frontier.frontier_tick,
            });
        }

        runtime.global_tick += 1;
        records
    }

    /// Returns the canonical ordering of heads that would be stepped,
    /// without actually mutating any state.
    ///
    /// Useful for testing and dry-run inspection.
    #[must_use]
    pub fn peek_order(runtime: &WorldlineRuntime) -> Vec<WriterHeadKey> {
        runtime.runnable.iter().copied().collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::redundant_clone)]
mod tests {
    use super::*;
    use crate::head::{make_head_id, WriterHead};
    use crate::playback::PlaybackMode;
    use crate::worldline::WorldlineId;
    use crate::worldline_state::WorldlineState;

    fn wl(n: u8) -> WorldlineId {
        WorldlineId([n; 32])
    }

    fn setup_runtime(worldline_heads: &[(u8, &[&str])]) -> WorldlineRuntime {
        let mut runtime = WorldlineRuntime::new();

        for (wl_id, head_labels) in worldline_heads {
            let worldline_id = wl(*wl_id);
            runtime
                .worldlines
                .register(worldline_id, WorldlineState::empty());

            for label in *head_labels {
                let key = WriterHeadKey {
                    worldline_id,
                    head_id: make_head_id(label),
                };
                runtime
                    .heads
                    .insert(WriterHead::new(key, PlaybackMode::Play));
            }
        }

        runtime.refresh_runnable();
        runtime
    }

    #[test]
    fn two_heads_two_worldlines_canonical_order() {
        let mut runtime = setup_runtime(&[(2, &["h1"]), (1, &["h1"])]);

        let records = SchedulerCoordinator::super_tick(&mut runtime);

        // Must step worldline 1 before worldline 2 (canonical order)
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].head_key.worldline_id, wl(1));
        assert_eq!(records[1].head_key.worldline_id, wl(2));
    }

    #[test]
    fn two_heads_one_worldline_canonical_order() {
        let mut runtime = setup_runtime(&[(1, &["beta", "alpha"])]);

        let records = SchedulerCoordinator::super_tick(&mut runtime);

        // Two heads on same worldline: ordered by head_id
        assert_eq!(records.len(), 2);
        // Both on worldline 1
        assert_eq!(records[0].head_key.worldline_id, wl(1));
        assert_eq!(records[1].head_key.worldline_id, wl(1));
        // In canonical head_id order
        assert!(records[0].head_key.head_id < records[1].head_key.head_id);
    }

    #[test]
    fn paused_heads_never_advance() {
        let mut runtime = setup_runtime(&[(1, &["active", "paused"])]);

        // Pause one head
        let paused_key = WriterHeadKey {
            worldline_id: wl(1),
            head_id: make_head_id("paused"),
        };
        runtime.heads.get_mut(&paused_key).unwrap().pause();
        runtime.refresh_runnable();

        let records = SchedulerCoordinator::super_tick(&mut runtime);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].head_key.head_id, make_head_id("active"));
    }

    #[test]
    fn frontier_tick_increments_per_step() {
        let mut runtime = setup_runtime(&[(1, &["h1"]), (2, &["h2"])]);

        let records = SchedulerCoordinator::super_tick(&mut runtime);

        // Each worldline's frontier should be at tick 1
        assert_eq!(records[0].frontier_tick_after, 1);
        assert_eq!(records[1].frontier_tick_after, 1);

        // Second SuperTick: tick 2
        let records2 = SchedulerCoordinator::super_tick(&mut runtime);
        assert_eq!(records2[0].frontier_tick_after, 2);
        assert_eq!(records2[1].frontier_tick_after, 2);
    }

    #[test]
    fn two_heads_same_worldline_share_frontier_tick() {
        let mut runtime = setup_runtime(&[(1, &["h1", "h2"])]);

        let records = SchedulerCoordinator::super_tick(&mut runtime);

        // Both heads step the same worldline: first sees tick 1, second sees tick 2
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].frontier_tick_after, 1);
        assert_eq!(records[1].frontier_tick_after, 2);
    }

    #[test]
    fn global_tick_increments_once_per_super_tick() {
        let mut runtime = setup_runtime(&[(1, &["h1"]), (2, &["h2"])]);
        assert_eq!(runtime.global_tick, 0);

        SchedulerCoordinator::super_tick(&mut runtime);
        assert_eq!(runtime.global_tick, 1);

        SchedulerCoordinator::super_tick(&mut runtime);
        assert_eq!(runtime.global_tick, 2);
    }

    #[test]
    fn no_host_clock_dependency() {
        // Running the same setup twice must produce identical results.
        // This verifies no host-clock or thread-scheduling dependency.
        let mut rt1 = setup_runtime(&[(2, &["b"]), (1, &["a"])]);
        let mut rt2 = setup_runtime(&[(2, &["b"]), (1, &["a"])]);

        let r1 = SchedulerCoordinator::super_tick(&mut rt1);
        let r2 = SchedulerCoordinator::super_tick(&mut rt2);

        assert_eq!(
            r1, r2,
            "identical setups must produce identical step records"
        );
    }

    #[test]
    fn empty_runtime_produces_no_steps() {
        let mut runtime = WorldlineRuntime::new();
        let records = SchedulerCoordinator::super_tick(&mut runtime);
        assert!(records.is_empty());
        assert_eq!(runtime.global_tick, 1);
    }

    #[test]
    fn peek_order_matches_super_tick_order() {
        let runtime = setup_runtime(&[(3, &["c"]), (1, &["a"]), (2, &["b"])]);
        let peeked = SchedulerCoordinator::peek_order(&runtime);
        let mut runtime_mut = runtime.clone();
        let stepped = SchedulerCoordinator::super_tick(&mut runtime_mut);

        assert_eq!(peeked.len(), stepped.len());
        for (p, s) in peeked.iter().zip(stepped.iter()) {
            assert_eq!(*p, s.head_key);
        }
    }
}
