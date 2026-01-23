// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Provenance store trait and implementations for SPEC-0004.
//!
//! The provenance store provides the historical data needed for worldline replay:
//! patches, expected hashes, recorded outputs, and checkpoints. This module defines
//! the trait interface (seam for future wormhole integration) and a simple in-memory
//! implementation for local use.
//!
//! # Key Types
//!
//! - [`ProvenanceStore`]: Trait defining the provenance data access interface.
//! - [`LocalProvenanceStore`]: In-memory Vec-backed implementation.
//! - [`HistoryError`]: Error type for history access failures.
//! - [`CheckpointRef`]: Reference to a checkpoint for fast seek.
//!
//! # `U0Ref` = `WarpId`
//!
//! Per SPEC-0004 correction #1, the `U0Ref` (initial state reference) for MVP is
//! simply the `WarpId`. The engine's `initial_state` for a warp serves as the U0
//! starting point for replay.

// The crate uses u64 ticks but Vec lengths are usize; on 64-bit platforms these
// are the same size, and we don't support 32-bit targets for this crate.
#![allow(clippy::cast_possible_truncation)]

use std::collections::BTreeMap;

use thiserror::Error;

use crate::graph::GraphStore;
use crate::ident::{Hash, WarpId};
use crate::snapshot::compute_state_root_for_warp_store;

use super::worldline::{HashTriplet, OutputFrameSet, WorldlineId, WorldlineTickPatchV1};

/// Errors that can occur when accessing worldline history.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HistoryError {
    /// The requested tick is not available in the store.
    ///
    /// This can occur when seeking beyond recorded history or when
    /// retention policy has pruned older ticks.
    #[error("history unavailable for tick {tick}")]
    HistoryUnavailable {
        /// The tick that was requested but not found.
        tick: u64,
    },

    /// The requested worldline does not exist.
    #[error("worldline not found: {0:?}")]
    WorldlineNotFound(WorldlineId),

    /// The provided tick does not match the expected next tick (append-only invariant).
    ///
    /// This occurs when attempting to append a tick that would create a gap or
    /// overlap in the history sequence.
    #[error("tick gap: expected tick {expected}, got {got}")]
    TickGap {
        /// The tick that was expected (current history length).
        expected: u64,
        /// The tick that was provided.
        got: u64,
    },
}

/// Reference to a checkpoint within the provenance store.
///
/// Checkpoints enable fast seeking by providing a known-good state snapshot
/// at a specific tick. Instead of replaying from U0, cursors can replay
/// from the nearest checkpoint before the target tick.
///
/// This type is only meaningful within the provenance/checkpoint subsystem.
/// It is created via [`LocalProvenanceStore::create_checkpoint`] and consumed
/// by [`ProvenanceStore::checkpoint_before`] during cursor seek operations.
///
/// [`LocalProvenanceStore::create_checkpoint`]: LocalProvenanceStore::create_checkpoint
/// [`ProvenanceStore::checkpoint_before`]: ProvenanceStore::checkpoint_before
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CheckpointRef {
    /// Tick number where this checkpoint was taken.
    pub tick: u64,
    /// Hash of the state at this checkpoint.
    pub state_hash: Hash,
}

/// Trait for accessing worldline provenance data.
///
/// This trait defines the seam for provenance data access, allowing different
/// backing stores (local memory, disk, wormhole network) to provide the same
/// interface for cursor replay operations.
///
/// # Thread Safety
///
/// Implementations should be thread-safe (`Send + Sync`) to allow concurrent
/// cursor access from multiple sessions.
///
/// # `U0Ref` = `WarpId`
///
/// The `u0` method returns a `WarpId` which serves as a handle to the engine's
/// `initial_state` for the warp. This is the MVP approach; future versions may
/// return a richer checkpoint reference.
pub trait ProvenanceStore: Send + Sync {
    /// Returns the U0 reference (initial state handle) for a worldline.
    ///
    /// For MVP, this is the `WarpId` that can be used to retrieve the initial
    /// state from the engine.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline doesn't exist.
    fn u0(&self, w: WorldlineId) -> Result<WarpId, HistoryError>;

    /// Returns the number of recorded ticks for a worldline.
    ///
    /// This is the length of the patch history, not the current tick number
    /// (which may be `len() - 1` if 0-indexed).
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline doesn't exist.
    fn len(&self, w: WorldlineId) -> Result<u64, HistoryError>;

    /// Returns the patch for a specific tick.
    ///
    /// # Errors
    ///
    /// - [`HistoryError::WorldlineNotFound`] if the worldline doesn't exist.
    /// - [`HistoryError::HistoryUnavailable`] if the tick is out of range or pruned.
    fn patch(&self, w: WorldlineId, tick: u64) -> Result<WorldlineTickPatchV1, HistoryError>;

    /// Returns the expected hash triplet for verification at a specific tick.
    ///
    /// Cursors use this to verify their replayed state matches the recorded
    /// state root, patch digest, and commit hash.
    ///
    /// # Errors
    ///
    /// - [`HistoryError::WorldlineNotFound`] if the worldline doesn't exist.
    /// - [`HistoryError::HistoryUnavailable`] if the tick is out of range or pruned.
    fn expected(&self, w: WorldlineId, tick: u64) -> Result<HashTriplet, HistoryError>;

    /// Returns the recorded channel outputs for a specific tick.
    ///
    /// These are the materialization bus outputs that were emitted during
    /// the original tick execution. Playback uses these for truth frame
    /// delivery rather than re-executing rules.
    ///
    /// # Errors
    ///
    /// - [`HistoryError::WorldlineNotFound`] if the worldline doesn't exist.
    /// - [`HistoryError::HistoryUnavailable`] if the tick is out of range or pruned.
    fn outputs(&self, w: WorldlineId, tick: u64) -> Result<OutputFrameSet, HistoryError>;

    /// Returns the nearest checkpoint before a given tick, if any.
    ///
    /// This enables fast seeking by starting replay from a checkpoint rather
    /// than from U0. Returns `None` if no checkpoint exists before the given
    /// tick, or if the worldline doesn't exist in the store.
    fn checkpoint_before(&self, w: WorldlineId, tick: u64) -> Option<CheckpointRef>;

    /// Returns whether the worldline has any recorded history.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline doesn't exist.
    fn is_empty(&self, w: WorldlineId) -> Result<bool, HistoryError> {
        Ok(self.len(w)? == 0)
    }
}

// Per-worldline history storage.
#[derive(Debug, Clone)]
struct WorldlineHistory {
    // U0 reference (`WarpId` for MVP).
    u0_ref: WarpId,
    // Patches in tick order.
    patches: Vec<WorldlineTickPatchV1>,
    // Expected hash triplets in tick order.
    expected: Vec<HashTriplet>,
    // Recorded outputs in tick order.
    outputs: Vec<OutputFrameSet>,
    // Checkpoints for fast seeking.
    checkpoints: Vec<CheckpointRef>,
}

/// In-memory provenance store backed by `Vec`s.
///
/// This is the simplest implementation suitable for testing and single-process
/// scenarios. For production use with large histories, consider a disk-backed
/// or network-backed implementation.
///
/// # Invariant
///
/// For each worldline: `patches.len() == expected.len() == outputs.len()`.
/// This maintains index alignment so tick N's data is at index N.
#[derive(Debug, Clone, Default)]
pub struct LocalProvenanceStore {
    /// Per-worldline history, keyed by worldline ID.
    worldlines: BTreeMap<WorldlineId, WorldlineHistory>,
}

impl LocalProvenanceStore {
    /// Creates a new empty provenance store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new worldline with its U0 reference.
    ///
    /// This must be called before appending any history for a worldline.
    pub fn register_worldline(&mut self, id: WorldlineId, u0_ref: WarpId) {
        self.worldlines
            .entry(id)
            .or_insert_with(|| WorldlineHistory {
                u0_ref,
                patches: Vec::new(),
                expected: Vec::new(),
                outputs: Vec::new(),
                checkpoints: Vec::new(),
            });
    }

    /// Appends a tick's data to a worldline's history.
    ///
    /// The tick number must equal the current length (append-only, no gaps).
    ///
    /// # Errors
    ///
    /// - Returns [`HistoryError::WorldlineNotFound`] if the worldline hasn't been registered.
    /// - Returns [`HistoryError::TickGap`] if the patch's `global_tick` doesn't equal the
    ///   current history length (the expected next tick).
    pub fn append(
        &mut self,
        w: WorldlineId,
        patch: WorldlineTickPatchV1,
        expected: HashTriplet,
        outputs: OutputFrameSet,
    ) -> Result<(), HistoryError> {
        let history = self
            .worldlines
            .get_mut(&w)
            .ok_or(HistoryError::WorldlineNotFound(w))?;

        let expected_tick = history.patches.len() as u64;
        let got_tick = patch.global_tick();
        if got_tick != expected_tick {
            return Err(HistoryError::TickGap {
                expected: expected_tick,
                got: got_tick,
            });
        }

        history.patches.push(patch);
        history.expected.push(expected);
        history.outputs.push(outputs);
        Ok(())
    }

    /// Records a checkpoint for a worldline.
    ///
    /// Checkpoints are stored in tick order for efficient binary search.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline hasn't been registered.
    pub fn add_checkpoint(
        &mut self,
        w: WorldlineId,
        checkpoint: CheckpointRef,
    ) -> Result<(), HistoryError> {
        let history = self
            .worldlines
            .get_mut(&w)
            .ok_or(HistoryError::WorldlineNotFound(w))?;

        // Maintain sorted order by tick
        let pos = history
            .checkpoints
            .binary_search_by_key(&checkpoint.tick, |c| c.tick)
            .unwrap_or_else(|e| e);
        history.checkpoints.insert(pos, checkpoint);
        Ok(())
    }

    /// Creates a checkpoint at the given tick by computing the state hash.
    ///
    /// This computes the canonical state hash for the given `GraphStore` and
    /// records a checkpoint at the specified tick. The checkpoint enables fast
    /// seeking during cursor replay.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::WorldlineNotFound`] if the worldline hasn't been registered.
    pub fn checkpoint(
        &mut self,
        w: WorldlineId,
        tick: u64,
        state: &GraphStore,
    ) -> Result<CheckpointRef, HistoryError> {
        let history = self
            .worldlines
            .get_mut(&w)
            .ok_or(HistoryError::WorldlineNotFound(w))?;

        let state_hash = compute_state_root_for_warp_store(state, history.u0_ref);
        let checkpoint_ref = CheckpointRef { tick, state_hash };

        // Insert in sorted order by tick (same logic as add_checkpoint)
        let pos = history
            .checkpoints
            .binary_search_by_key(&checkpoint_ref.tick, |c| c.tick)
            .unwrap_or_else(|e| e);
        history.checkpoints.insert(pos, checkpoint_ref);

        Ok(checkpoint_ref)
    }

    /// Creates a new worldline that is a prefix-copy of the source up to `fork_tick`.
    ///
    /// The new worldline shares the same U0 reference as the source and contains
    /// copies of all history data (patches, expected hashes, outputs, checkpoints)
    /// from tick 0 through `fork_tick` inclusive.
    ///
    /// # Errors
    ///
    /// - Returns [`HistoryError::WorldlineNotFound`] if the source worldline doesn't exist.
    /// - Returns [`HistoryError::HistoryUnavailable`] if `fork_tick` is beyond the
    ///   available history in the source worldline.
    pub fn fork(
        &mut self,
        source: WorldlineId,
        fork_tick: u64,
        new_id: WorldlineId,
    ) -> Result<(), HistoryError> {
        let source_history = self
            .worldlines
            .get(&source)
            .ok_or(HistoryError::WorldlineNotFound(source))?;

        // Validate fork_tick is within available history
        let source_len = source_history.patches.len();
        // SAFETY: cast_possible_truncation — history length fits in u64 because Vec
        // cannot exceed isize::MAX elements, and on 64-bit platforms usize == u64.
        if fork_tick >= source_len as u64 {
            return Err(HistoryError::HistoryUnavailable { tick: fork_tick });
        }

        // Copy prefix data up to and including fork_tick
        // SAFETY: cast_possible_truncation — fork_tick < source_len (checked above),
        // so fork_tick + 1 <= source_len <= usize::MAX; the cast back to usize is lossless.
        let end_idx = (fork_tick + 1) as usize;
        let new_history = WorldlineHistory {
            u0_ref: source_history.u0_ref,
            patches: source_history.patches[..end_idx].to_vec(),
            expected: source_history.expected[..end_idx].to_vec(),
            outputs: source_history.outputs[..end_idx].to_vec(),
            checkpoints: source_history
                .checkpoints
                .iter()
                .filter(|c| c.tick <= fork_tick)
                .copied()
                .collect(),
        };

        self.worldlines.insert(new_id, new_history);
        Ok(())
    }
}

impl ProvenanceStore for LocalProvenanceStore {
    fn u0(&self, w: WorldlineId) -> Result<WarpId, HistoryError> {
        self.worldlines
            .get(&w)
            .map(|h| h.u0_ref)
            .ok_or(HistoryError::WorldlineNotFound(w))
    }

    fn len(&self, w: WorldlineId) -> Result<u64, HistoryError> {
        self.worldlines
            .get(&w)
            .map(|h| h.patches.len() as u64)
            .ok_or(HistoryError::WorldlineNotFound(w))
    }

    fn patch(&self, w: WorldlineId, tick: u64) -> Result<WorldlineTickPatchV1, HistoryError> {
        let history = self
            .worldlines
            .get(&w)
            .ok_or(HistoryError::WorldlineNotFound(w))?;

        history
            .patches
            .get(tick as usize)
            .cloned()
            .ok_or(HistoryError::HistoryUnavailable { tick })
    }

    fn expected(&self, w: WorldlineId, tick: u64) -> Result<HashTriplet, HistoryError> {
        let history = self
            .worldlines
            .get(&w)
            .ok_or(HistoryError::WorldlineNotFound(w))?;

        history
            .expected
            .get(tick as usize)
            .copied()
            .ok_or(HistoryError::HistoryUnavailable { tick })
    }

    fn outputs(&self, w: WorldlineId, tick: u64) -> Result<OutputFrameSet, HistoryError> {
        let history = self
            .worldlines
            .get(&w)
            .ok_or(HistoryError::WorldlineNotFound(w))?;

        history
            .outputs
            .get(tick as usize)
            .cloned()
            .ok_or(HistoryError::HistoryUnavailable { tick })
    }

    fn checkpoint_before(&self, w: WorldlineId, tick: u64) -> Option<CheckpointRef> {
        let history = self.worldlines.get(&w)?;

        // Binary search for the largest checkpoint tick < target tick
        let pos = history
            .checkpoints
            .binary_search_by_key(&tick, |c| c.tick)
            .unwrap_or_else(|e| e);

        if pos == 0 {
            None
        } else {
            Some(history.checkpoints[pos - 1])
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::cast_possible_truncation)]
    #![allow(clippy::redundant_clone)]

    use super::*;
    use crate::ident::WarpId;
    use crate::worldline::WorldlineTickHeaderV1;

    fn test_worldline_id() -> WorldlineId {
        WorldlineId([1u8; 32])
    }

    fn test_warp_id() -> WarpId {
        WarpId([2u8; 32])
    }

    fn test_patch(tick: u64) -> WorldlineTickPatchV1 {
        WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                global_tick: tick,
                policy_id: 0,
                rule_pack_id: [0u8; 32],
                plan_digest: [0u8; 32],
                decision_digest: [0u8; 32],
                rewrites_digest: [0u8; 32],
            },
            warp_id: test_warp_id(),
            ops: vec![],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest: [tick as u8; 32],
        }
    }

    fn test_triplet(tick: u64) -> HashTriplet {
        HashTriplet {
            state_root: [tick as u8; 32],
            patch_digest: [(tick + 1) as u8; 32],
            commit_hash: [(tick + 2) as u8; 32],
        }
    }

    #[test]
    fn worldline_not_found() {
        let store = LocalProvenanceStore::new();
        let result = store.u0(test_worldline_id());
        assert!(matches!(result, Err(HistoryError::WorldlineNotFound(_))));
    }

    #[test]
    fn register_and_query_u0() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        let warp = test_warp_id();

        store.register_worldline(w, warp);

        assert_eq!(store.u0(w).unwrap(), warp);
        assert_eq!(store.len(w).unwrap(), 0);
        assert!(store.is_empty(w).unwrap());
    }

    #[test]
    fn append_and_query() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        let warp = test_warp_id();

        store.register_worldline(w, warp);

        let patch = test_patch(0);
        let triplet = test_triplet(0);
        let outputs = vec![];

        store.append(w, patch, triplet, outputs.clone()).unwrap();

        assert_eq!(store.len(w).unwrap(), 1);
        assert!(!store.is_empty(w).unwrap());
        assert_eq!(store.patch(w, 0).unwrap().global_tick(), 0);
        assert_eq!(store.expected(w, 0).unwrap(), triplet);
        assert_eq!(store.outputs(w, 0).unwrap(), outputs);
    }

    #[test]
    fn history_unavailable_for_missing_tick() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        let warp = test_warp_id();

        store.register_worldline(w, warp);
        store
            .append(w, test_patch(0), test_triplet(0), vec![])
            .unwrap();

        let result = store.patch(w, 1);
        assert!(matches!(
            result,
            Err(HistoryError::HistoryUnavailable { tick: 1 })
        ));
    }

    #[test]
    fn checkpoint_before() {
        let mut store = LocalProvenanceStore::new();
        let w = test_worldline_id();
        let warp = test_warp_id();

        store.register_worldline(w, warp);

        // Add checkpoints at ticks 0, 5, 10
        store
            .add_checkpoint(
                w,
                CheckpointRef {
                    tick: 0,
                    state_hash: [0u8; 32],
                },
            )
            .unwrap();
        store
            .add_checkpoint(
                w,
                CheckpointRef {
                    tick: 5,
                    state_hash: [5u8; 32],
                },
            )
            .unwrap();
        store
            .add_checkpoint(
                w,
                CheckpointRef {
                    tick: 10,
                    state_hash: [10u8; 32],
                },
            )
            .unwrap();

        // No checkpoint before tick 0
        assert!(store.checkpoint_before(w, 0).is_none());

        // Checkpoint at 0 is before tick 1
        assert_eq!(store.checkpoint_before(w, 1).unwrap().tick, 0);

        // Checkpoint at 5 is before tick 7
        assert_eq!(store.checkpoint_before(w, 7).unwrap().tick, 5);

        // Checkpoint at 10 is before tick 15
        assert_eq!(store.checkpoint_before(w, 15).unwrap().tick, 10);

        // Checkpoint at 5 is before tick 10 (not inclusive)
        assert_eq!(store.checkpoint_before(w, 10).unwrap().tick, 5);
    }
}
