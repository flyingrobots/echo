// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Playback cursor tests for SPEC-0004: Worldlines, Playback, and TruthBus.
//!
//! These tests verify cursor seek operations and hash verification.

use warp_core::{
    compute_state_root_for_warp_store, make_node_id, make_type_id, make_warp_id, CursorId,
    CursorRole, GraphStore, HashTriplet, LocalProvenanceStore, NodeKey, NodeRecord, PlaybackCursor,
    SeekError, WarpOp, WorldlineId, WorldlineTickHeaderV1, WorldlineTickPatchV1,
};

/// Creates a deterministic worldline ID for testing.
fn test_worldline_id() -> WorldlineId {
    WorldlineId([1u8; 32])
}

/// Creates a deterministic cursor ID for testing.
fn test_cursor_id() -> CursorId {
    CursorId([2u8; 32])
}

/// Creates a test warp ID.
fn test_warp_id() -> warp_core::WarpId {
    make_warp_id("test-warp")
}

/// Creates a test header for a specific tick.
fn test_header(tick: u64) -> WorldlineTickHeaderV1 {
    WorldlineTickHeaderV1 {
        global_tick: tick,
        policy_id: 0,
        rule_pack_id: [0u8; 32],
        plan_digest: [0u8; 32],
        decision_digest: [0u8; 32],
        rewrites_digest: [0u8; 32],
    }
}

/// Creates an initial store with a root node.
fn create_initial_store(warp_id: warp_core::WarpId) -> GraphStore {
    let mut store = GraphStore::new(warp_id);
    let root_id = make_node_id("root");
    let ty = make_type_id("RootType");
    store.insert_node(root_id, NodeRecord { ty });
    store
}

/// Creates a patch that adds a node at a specific tick.
fn create_add_node_patch(
    warp_id: warp_core::WarpId,
    tick: u64,
    node_name: &str,
) -> WorldlineTickPatchV1 {
    let node_id = make_node_id(node_name);
    let node_key = NodeKey {
        warp_id,
        local_id: node_id,
    };
    let ty = make_type_id(&format!("Type{}", tick));

    WorldlineTickPatchV1 {
        header: test_header(tick),
        warp_id,
        ops: vec![WarpOp::UpsertNode {
            node: node_key,
            record: NodeRecord { ty },
        }],
        in_slots: vec![],
        out_slots: vec![],
        patch_digest: [tick as u8; 32],
    }
}

/// Sets up a worldline with N ticks and returns the provenance store and initial store.
fn setup_worldline_with_ticks(
    num_ticks: u64,
) -> (
    LocalProvenanceStore,
    GraphStore,
    warp_core::WarpId,
    WorldlineId,
) {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_store = create_initial_store(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    provenance.register_worldline(worldline_id, warp_id);

    // Build up the worldline by applying patches and recording correct hashes
    let mut current_store = initial_store.clone();

    for tick in 0..num_ticks {
        let patch = create_add_node_patch(warp_id, tick, &format!("node-{}", tick));

        // Apply patch to get the resulting state
        patch
            .apply_to_store(&mut current_store)
            .expect("apply should succeed");

        // Compute the actual state root after applying
        let state_root = compute_state_root_for_warp_store(&current_store, warp_id);

        let triplet = HashTriplet {
            state_root,
            patch_digest: patch.patch_digest,
            commit_hash: [(tick + 100) as u8; 32], // Placeholder commit hash
        };

        provenance
            .append(worldline_id, patch, triplet, vec![])
            .expect("append should succeed");
    }

    (provenance, initial_store, warp_id, worldline_id)
}

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

    // Build up 10 ticks, but corrupt the expected hash at tick 6
    let mut current_store = initial_store.clone();

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

        let triplet = HashTriplet {
            state_root,
            patch_digest: patch.patch_digest,
            commit_hash: [(tick + 100) as u8; 32],
        };

        provenance
            .append(worldline_id, patch, triplet, vec![])
            .expect("append should succeed");
    }

    // Create a cursor starting at tick 0
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(),
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
        test_cursor_id(),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        100, // pin_max_tick is high but provenance only has 10 ticks
    );

    // Seeking to tick 10 should succeed (we have 10 patches: 0-9, so tick 10 is valid)
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
        test_cursor_id(),
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

    // Get hash at tick 3
    let _hash_at_3 = compute_state_root_for_warp_store(&cursor.store, warp_id);

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
        test_cursor_id(),
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
