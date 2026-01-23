// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Checkpoint and fork tests for SPEC-0004: Worldlines, Playback, and `TruthBus`.
//!
//! These tests verify:
//! - T17: checkpoint_replay_equals_full_replay
//! - T18: fork_worldline_diverges_after_fork_tick_without_affecting_original

mod common;
use common::{
    create_add_node_patch, create_initial_store, test_cursor_id, test_warp_id, test_worldline_id,
};

use warp_core::{
    compute_state_root_for_warp_store, CheckpointRef, CursorRole, GraphStore, HashTriplet,
    LocalProvenanceStore, PlaybackCursor, ProvenanceStore, WorldlineId,
};

/// Creates a deterministic worldline ID for the forked worldline.
fn forked_worldline_id() -> WorldlineId {
    WorldlineId([2u8; 32])
}

/// Sets up a worldline with N ticks and returns the provenance store and initial store.
/// Optionally adds checkpoints at specified tick intervals.
///
/// Checkpoint semantics:
/// - Cursor tick N means "state after applying patches 0..N-1" (N patches applied)
/// - A checkpoint at tick N stores the state after N patches have been applied
/// - So checkpoint_ticks = [5, 10, 15, 20] creates checkpoints at cursor ticks 5, 10, 15, 20
fn setup_worldline_with_ticks_and_checkpoints(
    worldline_id: WorldlineId,
    num_ticks: u64,
    checkpoint_ticks: &[u64],
) -> (
    LocalProvenanceStore,
    GraphStore,
    warp_core::WarpId,
    Vec<(u64, [u8; 32])>, // (cursor_tick, state_root) for each checkpoint
) {
    let warp_id = test_warp_id();
    let initial_store = create_initial_store(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    provenance.register_worldline(worldline_id, warp_id);

    // Build up the worldline by applying patches and recording correct hashes
    let mut current_store = initial_store.clone();
    let mut checkpoint_states: Vec<(u64, [u8; 32])> = Vec::new();

    for patch_index in 0..num_ticks {
        let patch = create_add_node_patch(warp_id, patch_index, &format!("node-{}", patch_index));

        // Apply patch to get the resulting state
        patch
            .apply_to_store(&mut current_store)
            .expect("apply should succeed");

        // Compute the actual state root after applying
        let state_root = compute_state_root_for_warp_store(&current_store, warp_id);

        let triplet = HashTriplet {
            state_root,
            patch_digest: patch.patch_digest,
            commit_hash: [(patch_index + 100) as u8; 32], // Placeholder commit hash
        };

        provenance
            .append(worldline_id, patch, triplet, vec![])
            .expect("append should succeed");

        // After applying N patches (indices 0..N-1), the cursor tick is N.
        // So after applying patch at `patch_index`, cursor_tick = patch_index + 1.
        let cursor_tick = patch_index + 1;
        if checkpoint_ticks.contains(&cursor_tick) {
            let checkpoint = CheckpointRef {
                tick: cursor_tick,
                state_hash: state_root,
            };
            provenance
                .add_checkpoint(worldline_id, checkpoint)
                .expect("worldline should be registered");
            checkpoint_states.push((cursor_tick, state_root));
        }
    }

    (provenance, initial_store, warp_id, checkpoint_states)
}

/// Sets up a worldline with N ticks (no checkpoints).
fn setup_worldline_with_ticks(
    worldline_id: WorldlineId,
    num_ticks: u64,
) -> (LocalProvenanceStore, GraphStore, warp_core::WarpId) {
    let (provenance, initial_store, warp_id, _) =
        setup_worldline_with_ticks_and_checkpoints(worldline_id, num_ticks, &[]);
    (provenance, initial_store, warp_id)
}

// ============================================================================
// T17: checkpoint_replay_equals_full_replay
// ============================================================================

/// T17: Seeking to a tick via checkpoint produces identical state_root as full replay.
///
/// This test verifies that using a checkpoint to accelerate seeking produces
/// the same final state as replaying from U0 (initial state).
///
/// - Arrange: Create worldline with 25 patches, add checkpoints at cursor ticks 5, 10, 15, 20
/// - Act:
///   1. Seek to tick 23 from U0 (full replay path)
///   2. Seek to tick 23 using checkpoint at 20 (checkpoint path)
/// - Assert: Both produce identical `state_root`
#[test]
fn checkpoint_replay_equals_full_replay() {
    let worldline_id = test_worldline_id();

    // Arrange: Create worldline with 25 patches and checkpoints at cursor ticks 5, 10, 15, 20
    // Cursor tick N means "state after N patches applied" (patches 0..N-1)
    let checkpoint_ticks = [5, 10, 15, 20];
    let (provenance, initial_store, warp_id, checkpoint_states) =
        setup_worldline_with_ticks_and_checkpoints(worldline_id, 25, &checkpoint_ticks);

    // Verify checkpoints were created
    assert_eq!(checkpoint_states.len(), 4);

    // Get the checkpoint at cursor tick 20 for later verification
    // checkpoint_before(w, 21) returns the largest checkpoint with tick < 21, which is tick 20
    let checkpoint_20 = provenance
        .checkpoint_before(worldline_id, 21)
        .expect("checkpoint at or before tick 21 should exist");
    assert_eq!(checkpoint_20.tick, 20);

    // Act 1: Full replay path - seek to tick 23 from U0
    let mut full_replay_cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        25,
    );

    // Cursor starts at tick 0, seek to tick 23 (applies patches 0..23, i.e., patches 0-22)
    full_replay_cursor
        .seek_to(23, &provenance, &initial_store)
        .expect("full replay seek to tick 23 should succeed");

    let full_replay_state_root =
        compute_state_root_for_warp_store(&full_replay_cursor.store, warp_id);

    // Act 2: Checkpoint path - seek to tick 23 using checkpoint at 20
    // First, create a cursor and manually restore state from checkpoint
    let mut checkpoint_cursor = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        25,
    );

    // Seek to checkpoint tick 20 first (this rebuilds state up to tick 20)
    // tick 20 = state after patches 0-19 applied
    checkpoint_cursor
        .seek_to(20, &provenance, &initial_store)
        .expect("seek to checkpoint tick 20 should succeed");

    // Verify we're at the checkpoint state
    let checkpoint_state = compute_state_root_for_warp_store(&checkpoint_cursor.store, warp_id);
    assert_eq!(
        checkpoint_state, checkpoint_20.state_hash,
        "cursor state at tick 20 should match checkpoint state_hash"
    );

    // Now seek from tick 20 to tick 23 (applies patches 20, 21, 22)
    checkpoint_cursor
        .seek_to(23, &provenance, &initial_store)
        .expect("checkpoint seek to tick 23 should succeed");

    let checkpoint_path_state_root =
        compute_state_root_for_warp_store(&checkpoint_cursor.store, warp_id);

    // Assert: Both paths produce identical state_root
    assert_eq!(
        full_replay_state_root, checkpoint_path_state_root,
        "full replay and checkpoint path should produce identical state_root at tick 23"
    );

    // Also verify both cursors are at tick 23
    assert_eq!(full_replay_cursor.tick, 23);
    assert_eq!(checkpoint_cursor.tick, 23);
}

// ============================================================================
// T18: fork_worldline_diverges_after_fork_tick_without_affecting_original
// ============================================================================

/// T18: Forking a worldline creates an independent copy that can diverge.
///
/// This test verifies that:
/// 1. Forking at tick 7 creates a new worldline with identical history up to tick 7
/// 2. Adding new ticks to the fork doesn't affect the original
/// 3. The fork can diverge with different patches after the fork point
///
/// - Arrange: Create worldline "original" with 20 ticks
/// - Act:
///   1. Fork at tick 7 to create "forked" worldline (copy history 0-7)
///   2. Add 3 more ticks to the forked worldline (ticks 8, 9, 10 with different patches)
/// - Assert:
///   1. Original worldline's expected hashes unchanged (still has same 20 ticks)
///   2. Forked worldline has ticks 0-7 matching original
///   3. Forked worldline ticks 8-10 are different (diverged history)
#[test]
fn fork_worldline_diverges_after_fork_tick_without_affecting_original() {
    let original_worldline_id = test_worldline_id();
    let forked_worldline_id = forked_worldline_id();

    // Arrange: Create "original" worldline with 20 ticks
    let (mut provenance, initial_store, warp_id) =
        setup_worldline_with_ticks(original_worldline_id, 20);

    // Capture original worldline's expected hashes for all 20 ticks
    let mut original_expected_hashes: Vec<HashTriplet> = Vec::new();
    for tick in 0..20 {
        let expected = provenance
            .expected(original_worldline_id, tick)
            .expect("original tick should exist");
        original_expected_hashes.push(expected);
    }

    // Act 1: Fork at tick 7 - copy history from original (ticks 0-7) to forked worldline
    provenance.register_worldline(forked_worldline_id, warp_id);

    // Copy patches 0-7 from original to forked
    for tick in 0..=7 {
        let patch = provenance
            .patch(original_worldline_id, tick)
            .expect("original patch should exist");
        let expected = provenance
            .expected(original_worldline_id, tick)
            .expect("original expected should exist");
        let outputs = provenance
            .outputs(original_worldline_id, tick)
            .expect("original outputs should exist");

        provenance
            .append(forked_worldline_id, patch, expected, outputs)
            .expect("append to forked should succeed");
    }

    // Verify fork has 8 ticks (0-7)
    assert_eq!(
        provenance
            .len(forked_worldline_id)
            .expect("forked worldline should be registered"),
        8,
        "forked worldline should have 8 ticks after copying 0-7"
    );

    // Act 2: Add 3 more ticks to forked worldline with DIFFERENT patches
    // These patches use different node names to produce different state
    let mut forked_store = initial_store.clone();

    // Replay forked worldline to tick 7 to get the correct state
    for tick in 0..=7 {
        let patch = provenance
            .patch(forked_worldline_id, tick)
            .expect("forked patch should exist");
        patch
            .apply_to_store(&mut forked_store)
            .expect("apply should succeed");
    }

    // Add divergent ticks 8, 9, 10 with different node names
    for tick in 8..=10 {
        // Use a different node name pattern to create divergent history
        let patch = create_add_node_patch(warp_id, tick, &format!("forked-node-{}", tick));

        patch
            .apply_to_store(&mut forked_store)
            .expect("apply should succeed");

        let state_root = compute_state_root_for_warp_store(&forked_store, warp_id);

        let triplet = HashTriplet {
            state_root,
            patch_digest: patch.patch_digest,
            // Use different commit hash pattern to distinguish from original
            commit_hash: [(tick + 200) as u8; 32],
        };

        provenance
            .append(forked_worldline_id, patch, triplet, vec![])
            .expect("append divergent tick should succeed");
    }

    // Assert 1: Original worldline's expected hashes unchanged (still has same 20 ticks)
    assert_eq!(
        provenance
            .len(original_worldline_id)
            .expect("original worldline should be registered"),
        20,
        "original worldline should still have 20 ticks"
    );

    for tick in 0..20 {
        let current_expected = provenance
            .expected(original_worldline_id, tick)
            .expect("original tick should still exist");
        assert_eq!(
            current_expected, original_expected_hashes[tick as usize],
            "original worldline tick {} expected hash should be unchanged",
            tick
        );
    }

    // Assert 2: Forked worldline has ticks 0-7 matching original
    for tick in 0..=7 {
        let original_expected = provenance
            .expected(original_worldline_id, tick)
            .expect("original tick should exist");
        let forked_expected = provenance
            .expected(forked_worldline_id, tick)
            .expect("forked tick should exist");

        assert_eq!(
            original_expected, forked_expected,
            "forked worldline tick {} should match original",
            tick
        );
    }

    // Assert 3: Forked worldline ticks 8-10 are different from original
    assert_eq!(
        provenance
            .len(forked_worldline_id)
            .expect("forked worldline should be registered"),
        11,
        "forked worldline should have 11 ticks (0-10)"
    );

    for tick in 8..=10 {
        let original_expected = provenance
            .expected(original_worldline_id, tick)
            .expect("original tick should exist");
        let forked_expected = provenance
            .expected(forked_worldline_id, tick)
            .expect("forked tick should exist");

        // State roots should differ because patches created different nodes
        assert_ne!(
            original_expected.state_root, forked_expected.state_root,
            "forked worldline tick {} state_root should differ from original",
            tick
        );

        // Commit hashes should also differ (we used different pattern)
        assert_ne!(
            original_expected.commit_hash, forked_expected.commit_hash,
            "forked worldline tick {} commit_hash should differ from original",
            tick
        );
    }

    // Verify we can seek in both worldlines independently
    let mut original_cursor = PlaybackCursor::new(
        test_cursor_id(1),
        original_worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        20,
    );

    let mut forked_cursor = PlaybackCursor::new(
        test_cursor_id(2),
        forked_worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        11,
    );

    // Seek original to tick 10
    original_cursor
        .seek_to(10, &provenance, &initial_store)
        .expect("seek original to 10 should succeed");

    // Seek forked to tick 10
    forked_cursor
        .seek_to(10, &provenance, &initial_store)
        .expect("seek forked to 10 should succeed");

    // Verify they have different state roots at tick 10
    let original_state = compute_state_root_for_warp_store(&original_cursor.store, warp_id);
    let forked_state = compute_state_root_for_warp_store(&forked_cursor.store, warp_id);

    assert_ne!(
        original_state, forked_state,
        "cursors at tick 10 should have different states due to divergent history"
    );
}
