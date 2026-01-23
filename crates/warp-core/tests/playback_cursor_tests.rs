// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Playback cursor tests for SPEC-0004: Worldlines, Playback, and TruthBus.
//!
//! These tests verify cursor seek operations and hash verification.

mod common;

use common::{
    create_add_node_patch, create_initial_store, setup_worldline_with_ticks, test_cursor_id,
    test_warp_id, test_worldline_id,
};
use warp_core::{
    compute_commit_hash_v2, compute_state_root_for_warp_store, CursorRole, Hash, HashTriplet,
    LocalProvenanceStore, PlaybackCursor, ProvenanceStore, SeekError,
};

/// T14: cursor_seek_fails_on_corrupt_patch_or_hash_mismatch
///
/// This test verifies that seeking across a tick with a corrupted/mismatched
/// expected hash triggers a `SeekError::StateRootMismatch`.
#[test]
fn cursor_seek_fails_on_corrupt_patch_or_hash_mismatch() {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_store = create_initial_store(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    provenance.register_worldline(worldline_id, warp_id);

    // Build up 10 ticks, but corrupt the expected state_root at tick 6
    let mut current_store = initial_store.clone();
    let mut parents: Vec<Hash> = Vec::new();

    for tick in 0..10u64 {
        let patch = create_add_node_patch(warp_id, tick, &format!("node-{}", tick));

        // Apply patch to get the resulting state
        patch
            .apply_to_store(&mut current_store)
            .expect("apply should succeed");

        // Compute the actual state root after applying
        let state_root = if tick == 6 {
            // CORRUPT: Use wrong hash for tick 6
            [
                0xDE, 0xAD, 0xBE, 0xEF, 0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]
        } else {
            compute_state_root_for_warp_store(&current_store, warp_id)
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

        provenance
            .append(worldline_id, patch, triplet, vec![])
            .expect("append should succeed");

        parents = vec![commit_hash];
    }

    // Create a cursor starting at tick 0
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    // Seeking to tick 5 should succeed (before the corrupted tick)
    let result = cursor.seek_to(5, &provenance, &initial_store);
    assert!(result.is_ok(), "seek to tick 5 should succeed");
    assert_eq!(cursor.tick, 5);

    // Seeking from tick 5 to tick 8 should fail at tick 6 due to hash mismatch
    let result = cursor.seek_to(8, &provenance, &initial_store);
    assert!(
        matches!(result, Err(SeekError::StateRootMismatch { tick: 6 })),
        "expected StateRootMismatch at tick 6, got: {:?}",
        result
    );
}

/// T15: seek_past_available_history_returns_history_unavailable
///
/// This test verifies that seeking beyond recorded history returns
/// `SeekError::HistoryUnavailable`.
#[test]
fn seek_past_available_history_returns_history_unavailable() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    // Create a cursor
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        100, // pin_max_tick is high but provenance only has 10 ticks
    );

    // With 10 patches in history (indices 0..9), valid ticks are 0..=10.
    // Tick 10 represents the state after all patches have been applied.
    let result = cursor.seek_to(10, &provenance, &initial_store);
    assert!(
        result.is_ok(),
        "seek to tick 10 should succeed: {:?}",
        result
    );
    assert_eq!(cursor.tick, 10);

    // Seeking to tick 50 should fail with HistoryUnavailable
    let result = cursor.seek_to(50, &provenance, &initial_store);
    assert!(
        matches!(result, Err(SeekError::HistoryUnavailable { tick: 50 })),
        "expected HistoryUnavailable at tick 50, got: {:?}",
        result
    );
}

/// Additional test: verify that seeking backwards works correctly by
/// rebuilding from initial state.
#[test]
fn seek_backward_rebuilds_from_initial_state() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    // Seek to tick 8
    cursor
        .seek_to(8, &provenance, &initial_store)
        .expect("seek to 8 should succeed");
    assert_eq!(cursor.tick, 8);

    // Get hash at tick 8
    let hash_at_8 = compute_state_root_for_warp_store(&cursor.store, warp_id);

    // Seek backward to tick 3
    cursor
        .seek_to(3, &provenance, &initial_store)
        .expect("seek to 3 should succeed");
    assert_eq!(cursor.tick, 3);

    // Get hash at tick 3 - must differ from tick 8 since different patches are applied
    let hash_at_3 = compute_state_root_for_warp_store(&cursor.store, warp_id);
    assert_ne!(
        hash_at_8, hash_at_3,
        "state at tick 3 should differ from state at tick 8"
    );

    // Seek forward again to tick 8 - should get same hash
    cursor
        .seek_to(8, &provenance, &initial_store)
        .expect("seek back to 8 should succeed");
    assert_eq!(cursor.tick, 8);

    let hash_at_8_again = compute_state_root_for_warp_store(&cursor.store, warp_id);
    assert_eq!(
        hash_at_8, hash_at_8_again,
        "seeking back and forth should produce same state"
    );

    // Also verify we can seek to 0 (initial state with patches applied from 0..0 = none)
    cursor
        .seek_to(0, &provenance, &initial_store)
        .expect("seek to 0 should succeed");
    assert_eq!(cursor.tick, 0);

    // At tick 0, no patches have been applied, so store should be initial state
    let initial_hash = compute_state_root_for_warp_store(&initial_store, warp_id);
    let cursor_hash_at_0 = compute_state_root_for_warp_store(&cursor.store, warp_id);
    assert_eq!(
        initial_hash, cursor_hash_at_0,
        "tick 0 should match initial state"
    );
}

/// Test that seek to current tick is a no-op.
#[test]
fn seek_to_current_tick_is_noop() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        5,
    );

    // Seek to tick 3
    cursor
        .seek_to(3, &provenance, &initial_store)
        .expect("seek to 3 should succeed");

    let hash_before = compute_state_root_for_warp_store(&cursor.store, warp_id);

    // Seek to same tick
    cursor
        .seek_to(3, &provenance, &initial_store)
        .expect("seek to same tick should succeed");

    let hash_after = compute_state_root_for_warp_store(&cursor.store, warp_id);
    assert_eq!(
        hash_before, hash_after,
        "seeking to current tick should be no-op"
    );
}

// =============================================================================
// Edge case tests (hitlist #57)
// =============================================================================

/// Edge case: A cursor with `pin_max_tick=0` cannot advance because it is
/// already at the frontier. Stepping in Play mode should immediately return
/// `ReachedFrontier` and transition the mode to Paused.
#[test]
fn pin_max_tick_zero_cursor_cannot_advance() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(3),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        0, // pin_max_tick = 0: cursor is already at frontier
    );

    // Cursor starts at tick 0, pin_max_tick is 0 => already at frontier
    assert_eq!(cursor.tick, 0);
    assert_eq!(cursor.pin_max_tick, 0);

    // Switch to Play mode and step
    cursor.mode = warp_core::PlaybackMode::Play;
    let result = cursor.step(&provenance, &initial_store);

    assert!(result.is_ok(), "step should not error: {:?}", result);
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
    assert_eq!(cursor.tick, 0);
}

/// Edge case: Seeking to `u64::MAX` on a worldline with only a few ticks
/// should return `SeekError::HistoryUnavailable` since the target is far
/// beyond the recorded history.
#[test]
fn seek_to_u64_max_returns_history_unavailable() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(4),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        u64::MAX,
    );

    let result = cursor.seek_to(u64::MAX, &provenance, &initial_store);

    assert!(
        matches!(result, Err(SeekError::HistoryUnavailable { tick }) if tick == u64::MAX),
        "expected HistoryUnavailable at u64::MAX, got: {:?}",
        result
    );

    // Cursor tick should remain at 0 (seek failed, no state change)
    assert_eq!(cursor.tick, 0);
}

/// Edge case: A worldline that is registered but has no patches appended.
/// The cursor should start at tick 0, and seeking forward should fail since
/// there is no recorded history to apply.
#[test]
fn empty_worldline_cursor_at_tick_zero() {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_store = create_initial_store(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    provenance.register_worldline(worldline_id, warp_id);

    // Do NOT append any patches -- worldline is empty

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(5),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        100,
    );

    // Cursor starts at tick 0
    assert_eq!(cursor.tick, 0);

    // Seeking to tick 0 should be a no-op (already there)
    let result = cursor.seek_to(0, &provenance, &initial_store);
    assert!(
        result.is_ok(),
        "seek to 0 on empty worldline should succeed (no-op)"
    );
    assert_eq!(cursor.tick, 0);

    // Seeking to tick 1 should fail: no patches available
    let result = cursor.seek_to(1, &provenance, &initial_store);
    assert!(
        matches!(result, Err(SeekError::HistoryUnavailable { tick: 1 })),
        "expected HistoryUnavailable at tick 1 on empty worldline, got: {:?}",
        result
    );

    // Cursor should remain at tick 0
    assert_eq!(cursor.tick, 0);
}

/// Edge case: Registering the same `WorldlineId` twice should be idempotent.
/// `LocalProvenanceStore::register_worldline` uses `entry().or_insert_with()`,
/// so a duplicate registration should not overwrite existing history.
#[test]
fn duplicate_worldline_registration_is_idempotent() {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();

    let mut provenance = LocalProvenanceStore::new();

    // First registration
    provenance.register_worldline(worldline_id, warp_id);

    // Append a tick so we can verify history survives re-registration
    let patch = create_add_node_patch(warp_id, 0, "dup-node-0");
    let initial_store = create_initial_store(warp_id);
    let mut current_store = initial_store.clone();
    patch
        .apply_to_store(&mut current_store)
        .expect("apply should succeed");
    let state_root = compute_state_root_for_warp_store(&current_store, warp_id);
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
    provenance
        .append(worldline_id, patch, triplet, vec![])
        .expect("append should succeed");

    // Verify history length is 1
    assert_eq!(
        provenance.len(worldline_id).unwrap(),
        1,
        "worldline should have 1 tick before re-registration"
    );

    // Second registration with the same ID -- should be idempotent
    let different_warp = warp_core::WarpId([99u8; 32]);
    provenance.register_worldline(worldline_id, different_warp);

    // History should NOT be reset; length should still be 1
    assert_eq!(
        provenance.len(worldline_id).unwrap(),
        1,
        "duplicate registration must not overwrite existing history"
    );

    // U0 ref should still be the original warp_id (not the new one)
    assert_eq!(
        provenance.u0(worldline_id).unwrap(),
        warp_id,
        "duplicate registration must not overwrite U0 ref"
    );

    // Cursor can still seek to the existing tick
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(6),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );
    let result = cursor.seek_to(1, &provenance, &initial_store);
    assert!(
        result.is_ok(),
        "seek should succeed after duplicate registration: {:?}",
        result
    );
    assert_eq!(cursor.tick, 1);
}
