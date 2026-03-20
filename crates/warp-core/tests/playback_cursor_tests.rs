// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Playback cursor tests for SPEC-0004: Worldlines, Playback, and TruthBus.
//!
//! These tests verify cursor seek operations and hash verification.
//! Fixture helpers such as `create_add_node_patch(...)` deliberately append
//! isolated nodes, so some tests assert that commit history diverges while the
//! reachable `state_root()` remains unchanged across ticks.

mod common;

use std::sync::{Arc, Mutex};

use common::{
    append_fixture_entry, create_add_node_patch, create_initial_worldline_state,
    register_fixture_worldline, setup_worldline_with_ticks, test_cursor_id, test_warp_id,
    test_worldline_id,
};
use warp_core::materialization::make_channel_id;
use warp_core::{
    compute_commit_hash_v2, make_node_id, make_type_id, CheckpointRef, CursorRole, Hash,
    HashTriplet, HistoryError, LocalProvenanceStore, NodeRecord, PlaybackCursor, ProvenanceEntry,
    ProvenanceRef, ProvenanceStore, ReplayCheckpoint, SeekError, WarpId, WorldlineId,
    WorldlineState, WorldlineTick,
};

fn wt(raw: u64) -> WorldlineTick {
    WorldlineTick::from_raw(raw)
}

struct RecordingProvenance {
    inner: LocalProvenanceStore,
    events: Arc<Mutex<Vec<String>>>,
}

impl RecordingProvenance {
    fn new(inner: LocalProvenanceStore) -> Self {
        Self {
            inner,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn record(&self, event: impl Into<String>) {
        self.events
            .lock()
            .expect("recording mutex should not be poisoned")
            .push(event.into());
    }

    fn events(&self) -> Vec<String> {
        self.events
            .lock()
            .expect("recording mutex should not be poisoned")
            .clone()
    }
}

impl ProvenanceStore for RecordingProvenance {
    fn u0(&self, w: WorldlineId) -> Result<WarpId, HistoryError> {
        self.record("u0");
        self.inner.u0(w)
    }

    fn initial_boundary_hash(&self, w: WorldlineId) -> Result<Hash, HistoryError> {
        self.record("initial_boundary_hash");
        self.inner.initial_boundary_hash(w)
    }

    fn len(&self, w: WorldlineId) -> Result<u64, HistoryError> {
        self.record("len");
        self.inner.len(w)
    }

    fn entry(&self, w: WorldlineId, tick: WorldlineTick) -> Result<ProvenanceEntry, HistoryError> {
        self.record(format!("entry:{}", tick.as_u64()));
        self.inner.entry(w, tick)
    }

    fn parents(
        &self,
        w: WorldlineId,
        tick: WorldlineTick,
    ) -> Result<Vec<ProvenanceRef>, HistoryError> {
        self.record(format!("parents:{}", tick.as_u64()));
        self.inner.parents(w, tick)
    }

    fn append_local_commit(&mut self, entry: ProvenanceEntry) -> Result<(), HistoryError> {
        self.record(format!(
            "append_local_commit:{}",
            entry.worldline_tick.as_u64()
        ));
        self.inner.append_local_commit(entry)
    }

    fn checkpoint_before(&self, w: WorldlineId, tick: WorldlineTick) -> Option<CheckpointRef> {
        self.record(format!("checkpoint_before:{}", tick.as_u64()));
        self.inner.checkpoint_before(w, tick)
    }

    fn checkpoint_state_before(
        &self,
        w: WorldlineId,
        tick: WorldlineTick,
    ) -> Option<ReplayCheckpoint> {
        self.record(format!("checkpoint_state_before:{}", tick.as_u64()));
        self.inner.checkpoint_state_before(w, tick)
    }
}

/// T14: cursor_seek_fails_on_corrupt_patch_or_hash_mismatch
///
/// This test verifies that seeking across a tick with a corrupted/mismatched
/// expected hash triggers a `SeekError::StateRootMismatch`.
#[test]
fn cursor_seek_fails_on_corrupt_patch_or_hash_mismatch() {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_state = create_initial_worldline_state(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    register_fixture_worldline(&mut provenance, worldline_id, &initial_state).unwrap();

    // Build up 10 ticks, but corrupt the expected state_root at tick 6
    let mut current_state = initial_state.clone();
    let mut parents: Vec<Hash> = Vec::new();

    for tick in 0..10u64 {
        let patch = create_add_node_patch(warp_id, tick, &format!("node-{tick}"));

        // Apply patch to get the resulting state
        patch
            .apply_to_worldline_state(&mut current_state)
            .expect("apply should succeed");

        // Compute the actual state root after applying
        let state_root = if tick == 6 {
            // CORRUPT: Use wrong hash for tick 6
            [
                0xDE, 0xAD, 0xBE, 0xEF, 0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]
        } else {
            current_state.state_root()
        };

        // Compute real commit_hash for valid Merkle chain (even for corrupt tick 6,
        // the state_root mismatch is caught before commit_hash is checked)
        let commit_hash = compute_commit_hash_v2(
            &state_root,
            &parents,
            &patch.patch_digest,
            patch.header.policy_id,
        );

        let triplet = HashTriplet {
            state_root,
            patch_digest: patch.patch_digest,
            commit_hash,
        };

        append_fixture_entry(&mut provenance, worldline_id, patch, triplet, vec![])
            .expect("append should succeed");

        parents = vec![commit_hash];
    }

    // Create a cursor starting at tick 0
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        wt(10),
    );

    // Seeking to tick 5 should succeed (before the corrupted tick)
    let result = cursor.seek_to(wt(5), &provenance, &initial_state);
    assert!(result.is_ok(), "seek to tick 5 should succeed");
    assert_eq!(cursor.tick, wt(5));

    // Seeking from tick 5 to tick 8 should fail at tick 6 due to hash mismatch
    let result = cursor.seek_to(wt(8), &provenance, &initial_state);
    assert!(
        matches!(result, Err(SeekError::StateRootMismatch { tick }) if tick == wt(6)),
        "expected StateRootMismatch at tick 6, got: {result:?}"
    );
}

/// T15: seek_past_available_history_returns_history_unavailable
///
/// This test verifies that seeking beyond recorded history returns
/// `SeekError::HistoryUnavailable`.
#[test]
fn seek_past_available_history_returns_history_unavailable() {
    let (provenance, initial_state, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    // Create a cursor
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        wt(100), // pin_max_tick is high but provenance only has 10 ticks
    );

    // With 10 patches in history (indices 0..9), valid ticks are 0..=10.
    // Tick 10 represents the state after all patches have been applied.
    let result = cursor.seek_to(wt(10), &provenance, &initial_state);
    assert!(result.is_ok(), "seek to tick 10 should succeed: {result:?}");
    assert_eq!(cursor.tick, wt(10));

    // Seeking to tick 50 should fail with HistoryUnavailable
    let result = cursor.seek_to(wt(50), &provenance, &initial_state);
    assert!(
        matches!(result, Err(SeekError::HistoryUnavailable { tick }) if tick == wt(50)),
        "expected HistoryUnavailable at tick 50, got: {result:?}"
    );
}

/// Additional test: verify that seeking backwards works correctly by
/// rebuilding from initial state.
#[test]
fn seek_backward_rebuilds_from_initial_state() {
    let (provenance, initial_state, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        wt(10),
    );

    // Seek to tick 8
    cursor
        .seek_to(wt(8), &provenance, &initial_state)
        .expect("seek to 8 should succeed");
    assert_eq!(cursor.tick, wt(8));

    // Get hash at tick 8
    let hash_at_8 = cursor.current_state_root();
    let commit_at_8 = provenance
        .entry(worldline_id, wt(7))
        .expect("tick 7 should exist")
        .expected
        .commit_hash;

    // Seek backward to tick 3
    cursor
        .seek_to(wt(3), &provenance, &initial_state)
        .expect("seek to 3 should succeed");
    assert_eq!(cursor.tick, wt(3));

    // The logical history position must differ even if the reachable state root
    // matches because these fixtures only append isolated nodes.
    let hash_at_3 = cursor.current_state_root();
    let commit_at_3 = provenance
        .entry(worldline_id, wt(2))
        .expect("tick 2 should exist")
        .expected
        .commit_hash;
    assert_ne!(
        commit_at_8, commit_at_3,
        "tick 3 and tick 8 should be distinct history positions"
    );

    // Seek forward again to tick 8 - should get same hash
    cursor
        .seek_to(wt(8), &provenance, &initial_state)
        .expect("seek back to 8 should succeed");
    assert_eq!(cursor.tick, wt(8));

    let hash_at_8_again = cursor.current_state_root();
    assert_eq!(
        hash_at_8, hash_at_8_again,
        "seeking back and forth should produce same state"
    );
    assert_eq!(
        hash_at_3, hash_at_8,
        "create_add_node_patch fixtures append isolated nodes, so the reachable state root may stay unchanged across ticks"
    );

    // Also verify we can seek to 0 (initial state with patches applied from 0..0 = none)
    cursor
        .seek_to(wt(0), &provenance, &initial_state)
        .expect("seek to 0 should succeed");
    assert_eq!(cursor.tick, wt(0));

    // At tick 0, no patches have been applied, so store should be initial state
    let initial_hash = initial_state.state_root();
    let cursor_hash_at_0 = cursor.current_state_root();
    assert_eq!(
        initial_hash, cursor_hash_at_0,
        "tick 0 should match initial state"
    );
}

/// Test that seek to current tick is a no-op.
#[test]
fn seek_to_current_tick_is_noop() {
    let (provenance, initial_state, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        wt(5),
    );

    // Seek to tick 3
    cursor
        .seek_to(wt(3), &provenance, &initial_state)
        .expect("seek to 3 should succeed");

    let hash_before = cursor.current_state_root();

    // Seek to same tick
    cursor
        .seek_to(wt(3), &provenance, &initial_state)
        .expect("seek to same tick should succeed");

    let hash_after = cursor.current_state_root();
    assert_eq!(
        hash_before, hash_after,
        "seeking to current tick should be no-op"
    );
}

#[test]
fn seek_from_checkpoint_hydrates_metadata_and_outputs() {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_state = create_initial_worldline_state(warp_id);
    let root = *initial_state.root();
    let checkpoint_output_channel = make_channel_id("playback:checkpoint-output");
    let checkpoint_output_bytes = vec![0xAA, 0xBB, 0xCC];
    let suffix_output_channel = make_channel_id("playback:checkpoint-suffix-output");
    let suffix_output_bytes = vec![0xDD, 0xEE];

    let mut provenance = LocalProvenanceStore::new();
    register_fixture_worldline(&mut provenance, worldline_id, &initial_state).unwrap();

    let patch = create_add_node_patch(warp_id, 0, "checkpoint-root");
    let mut checkpoint_state = initial_state.clone();
    patch
        .apply_to_worldline_state(&mut checkpoint_state)
        .expect("fixture patch should apply");

    let state_root = checkpoint_state.state_root();
    let commit_hash = compute_commit_hash_v2(
        &state_root,
        &[],
        &patch.patch_digest,
        patch.header.policy_id,
    );
    let triplet = HashTriplet {
        state_root,
        patch_digest: patch.patch_digest,
        commit_hash,
    };
    append_fixture_entry(
        &mut provenance,
        worldline_id,
        patch,
        triplet,
        vec![(checkpoint_output_channel, checkpoint_output_bytes)],
    )
    .expect("append should succeed");
    provenance
        .add_checkpoint(
            worldline_id,
            ReplayCheckpoint {
                checkpoint: CheckpointRef {
                    worldline_tick: wt(1),
                    state_hash: state_root,
                },
                state: checkpoint_state.clone(),
            },
        )
        .expect("checkpoint should be accepted");

    let suffix_patch = create_add_node_patch(warp_id, 1, "checkpoint-suffix");
    let mut final_state = checkpoint_state.clone();
    suffix_patch
        .apply_to_worldline_state(&mut final_state)
        .expect("suffix fixture patch should apply");

    let suffix_state_root = final_state.state_root();
    let suffix_commit_hash = compute_commit_hash_v2(
        &suffix_state_root,
        &[commit_hash],
        &suffix_patch.patch_digest,
        suffix_patch.header.policy_id,
    );
    let suffix_triplet = HashTriplet {
        state_root: suffix_state_root,
        patch_digest: suffix_patch.patch_digest,
        commit_hash: suffix_commit_hash,
    };
    append_fixture_entry(
        &mut provenance,
        worldline_id,
        suffix_patch,
        suffix_triplet,
        vec![(suffix_output_channel, suffix_output_bytes.clone())],
    )
    .expect("suffix append should succeed");

    let provenance = RecordingProvenance::new(provenance);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(7),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        wt(2),
    );
    cursor
        .seek_to(wt(2), &provenance, &initial_state)
        .expect("checkpoint seek should succeed");

    let events = provenance.events();
    assert!(
        events
            .iter()
            .any(|event| event == "checkpoint_state_before:3"),
        "expected seek to restore from a replay checkpoint, got events: {events:?}"
    );

    assert_eq!(cursor.tick, wt(2));
    assert_eq!(cursor.state.tick_history().len(), 2);
    assert_eq!(
        cursor.state.last_snapshot().map(|snapshot| snapshot.hash),
        Some(suffix_commit_hash)
    );
    assert_eq!(cursor.state.last_materialization().len(), 1);
    assert_eq!(
        cursor.state.last_materialization()[0].channel,
        suffix_output_channel
    );
    assert_eq!(
        cursor.state.last_materialization()[0].data,
        suffix_output_bytes
    );
    assert_eq!(cursor.current_state_root(), suffix_state_root);
    assert_eq!(
        cursor
            .focused_store()
            .expect("focused store should exist")
            .node(&make_node_id("checkpoint-root"))
            .expect("checkpoint node should exist")
            .ty,
        make_type_id("Type0")
    );
    assert_eq!(
        cursor
            .focused_store()
            .expect("focused store should exist")
            .node(&make_node_id("checkpoint-suffix"))
            .expect("suffix node should exist")
            .ty,
        make_type_id("Type1")
    );
    assert_eq!(cursor.state.root(), &root);
}

#[test]
fn seek_rejects_wrong_initial_boundary_up_front() {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_state = create_initial_worldline_state(warp_id);
    let root_node = initial_state.root().local_id;

    let mut provenance = LocalProvenanceStore::new();
    register_fixture_worldline(&mut provenance, worldline_id, &initial_state).unwrap();

    let mut wrong_store = initial_state
        .store(&warp_id)
        .expect("root store should exist")
        .clone();
    wrong_store.insert_node(
        root_node,
        NodeRecord {
            ty: make_type_id("wrong-base"),
        },
    );
    let wrong_base =
        WorldlineState::from_root_store(wrong_store, root_node).expect("wrong base should build");

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(8),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &wrong_base,
        wt(0),
    );
    let result = cursor.seek_to(wt(0), &provenance, &wrong_base);

    assert!(
        matches!(result, Err(SeekError::InitialBoundaryHashMismatch { .. })),
        "expected wrong replay base rejection, got: {result:?}"
    );
    assert_eq!(cursor.tick, wt(0));
}

// =============================================================================
// Edge case tests (hitlist #57)
// =============================================================================

/// Edge case: A cursor with `pin_max_tick=0` cannot advance because it is
/// already at the frontier. Stepping in Play mode should immediately return
/// `ReachedFrontier` and transition the mode to Paused.
#[test]
fn pin_max_tick_zero_cursor_cannot_advance() {
    let (provenance, initial_state, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(3),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        wt(0), // pin_max_tick = 0: cursor is already at frontier
    );

    // Cursor starts at tick 0, pin_max_tick is 0 => already at frontier
    assert_eq!(cursor.tick, wt(0));
    assert_eq!(cursor.pin_max_tick, wt(0));

    // Switch to Play mode and step
    cursor.mode = warp_core::PlaybackMode::Play;
    let result = cursor.step(&provenance, &initial_state);

    assert!(result.is_ok(), "step should not error: {result:?}");
    assert_eq!(
        result.unwrap(),
        warp_core::StepResult::ReachedFrontier,
        "cursor at pin_max_tick=0 should immediately reach frontier"
    );

    // Mode should have transitioned to Paused
    assert_eq!(
        cursor.mode,
        warp_core::PlaybackMode::Paused,
        "mode should be Paused after reaching frontier"
    );

    // Tick should remain at 0
    assert_eq!(cursor.tick, wt(0));
}

#[test]
fn step_forward_at_pinned_frontier_returns_reached_frontier() {
    let (provenance, initial_state, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(4),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        wt(0),
    );
    cursor.mode = warp_core::PlaybackMode::StepForward;

    let result = cursor.step(&provenance, &initial_state).unwrap();
    assert_eq!(result, warp_core::StepResult::ReachedFrontier);
    assert_eq!(cursor.mode, warp_core::PlaybackMode::Paused);
    assert_eq!(cursor.tick, wt(0));
}

#[test]
#[should_panic(expected = "playback cursor initial_state must be an unadvanced replay base")]
fn new_rejects_advanced_initial_state() {
    let (provenance, initial_state, warp_id, worldline_id) = setup_worldline_with_ticks(1);

    let mut seed_cursor = PlaybackCursor::new(
        test_cursor_id(11),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        wt(1),
    );
    seed_cursor
        .seek_to(wt(1), &provenance, &initial_state)
        .expect("seed cursor should materialize tick 1");

    let _ = PlaybackCursor::new(
        test_cursor_id(12),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &seed_cursor.state,
        wt(1),
    );
}

/// Edge case: Seeking to `u64::MAX` on a worldline with only a few ticks
/// should return `SeekError::HistoryUnavailable` since the target is far
/// beyond the recorded history.
#[test]
fn seek_to_u64_max_returns_history_unavailable() {
    let (provenance, initial_state, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(4),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        WorldlineTick::MAX,
    );

    let result = cursor.seek_to(WorldlineTick::MAX, &provenance, &initial_state);

    assert!(
        matches!(result, Err(SeekError::HistoryUnavailable { tick }) if tick == WorldlineTick::MAX),
        "expected HistoryUnavailable at u64::MAX, got: {result:?}"
    );

    // Cursor tick should remain at 0 (seek failed, no state change)
    assert_eq!(cursor.tick, wt(0));
}

/// Edge case: A worldline that is registered but has no patches appended.
/// The cursor should start at tick 0, and seeking forward should fail since
/// there is no recorded history to apply.
#[test]
fn empty_worldline_cursor_at_tick_zero() {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_state = create_initial_worldline_state(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    register_fixture_worldline(&mut provenance, worldline_id, &initial_state).unwrap();

    // Do NOT append any patches -- worldline is empty

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(5),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        wt(100),
    );

    // Cursor starts at tick 0
    assert_eq!(cursor.tick, wt(0));

    // Seeking to tick 0 should be a no-op (already there)
    let result = cursor.seek_to(wt(0), &provenance, &initial_state);
    assert!(
        result.is_ok(),
        "seek to 0 on empty worldline should succeed (no-op)"
    );
    assert_eq!(cursor.tick, wt(0));

    // Seeking to tick 1 should fail: no patches available
    let result = cursor.seek_to(wt(1), &provenance, &initial_state);
    assert!(
        matches!(result, Err(SeekError::HistoryUnavailable { tick }) if tick == wt(1)),
        "expected HistoryUnavailable at tick 1 on empty worldline, got: {result:?}"
    );

    // Cursor should remain at tick 0
    assert_eq!(cursor.tick, wt(0));
}

/// Edge case: Registering the same `WorldlineId` twice with the same `u0_ref` is
/// idempotent, and re-registering with a different `u0_ref` returns an error
/// without overwriting existing history.
#[test]
fn duplicate_worldline_registration_is_idempotent() {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_state = create_initial_worldline_state(warp_id);

    let mut provenance = LocalProvenanceStore::new();

    // First registration
    register_fixture_worldline(&mut provenance, worldline_id, &initial_state).unwrap();

    // Append a tick so we can verify history survives re-registration attempts
    let patch = create_add_node_patch(warp_id, 0, "dup-node-0");
    let mut current_state = initial_state.clone();
    patch
        .apply_to_worldline_state(&mut current_state)
        .expect("apply should succeed");
    let state_root = current_state.state_root();
    let commit_hash = compute_commit_hash_v2(
        &state_root,
        &[], // No parents for first tick
        &patch.patch_digest,
        patch.header.policy_id,
    );
    let triplet = HashTriplet {
        state_root,
        patch_digest: patch.patch_digest,
        commit_hash,
    };
    append_fixture_entry(&mut provenance, worldline_id, patch, triplet, vec![])
        .expect("append should succeed");

    // Verify history length is 1
    assert_eq!(
        provenance.len(worldline_id).unwrap(),
        1,
        "worldline should have 1 tick before re-registration"
    );

    // Re-registration with the same u0_ref -- should be idempotent no-op
    register_fixture_worldline(&mut provenance, worldline_id, &initial_state).unwrap();

    // History should NOT be reset; length should still be 1
    assert_eq!(
        provenance.len(worldline_id).unwrap(),
        1,
        "duplicate registration with same u0_ref must not overwrite existing history"
    );

    // Re-registration with a different u0_ref -- should return an error
    let different_warp = warp_core::WarpId([99u8; 32]);
    let err = provenance
        .register_worldline(worldline_id, different_warp)
        .unwrap_err();
    assert!(
        matches!(err, warp_core::HistoryError::WorldlineAlreadyExists(_)),
        "expected WorldlineAlreadyExists error, got: {err:?}"
    );

    // History should still be intact after the failed re-registration
    assert_eq!(
        provenance.len(worldline_id).unwrap(),
        1,
        "failed re-registration must not overwrite existing history"
    );

    // U0 ref should still be the original warp_id (not the new one)
    assert_eq!(
        provenance.u0(worldline_id).unwrap(),
        warp_id,
        "failed re-registration must not overwrite U0 ref"
    );

    // Cursor can still seek to the existing tick
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(6),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_state,
        wt(10),
    );
    let result = cursor.seek_to(wt(1), &provenance, &initial_state);
    assert!(
        result.is_ok(),
        "seek should succeed after failed re-registration: {result:?}"
    );
    assert_eq!(cursor.tick, wt(1));
}
