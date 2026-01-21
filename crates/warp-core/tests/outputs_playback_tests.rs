// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Outputs playback tests for SPEC-0004 Commit 5: Record Outputs Per Tick + Seek/Playback.
//!
//! These tests verify:
//! - T4: seek_moves_cursor_without_mutating_writer_store
//! - T5: step_back_is_seek_minus_one_then_pause
//! - T6: reader_play_consumes_existing_then_pauses_at_frontier
//! - T8: outputs_match_recorded_bytes_for_same_tick
//! - MBUS v2 integration: truth_frames_encode_to_mbus_v2

use warp_core::materialization::{
    compute_value_hash, decode_v2_packet, encode_v2_packet, make_channel_id, V2Entry,
    V2PacketHeader,
};
use warp_core::{
    compute_state_root_for_warp_store, make_node_id, make_type_id, make_warp_id, CursorId,
    CursorRole, GraphStore, HashTriplet, HistoryError, LocalProvenanceStore, NodeKey, NodeRecord,
    PlaybackCursor, PlaybackMode, ProvenanceStore, SessionId, StepResult, TruthSink, ViewSession,
    WarpOp, WorldlineId, WorldlineTickHeaderV1, WorldlineTickPatchV1,
};

/// Creates a deterministic worldline ID for testing.
fn test_worldline_id() -> WorldlineId {
    WorldlineId([1u8; 32])
}

/// Creates a deterministic cursor ID for testing.
fn test_cursor_id(n: u8) -> CursorId {
    CursorId([n; 32])
}

/// Creates a deterministic session ID for testing.
fn test_session_id(n: u8) -> SessionId {
    SessionId([n; 32])
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

/// Sets up a worldline with N ticks and outputs, returns the provenance store and initial store.
fn setup_worldline_with_outputs(
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

    let position_channel = make_channel_id("entity:position");
    let velocity_channel = make_channel_id("entity:velocity");

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

        // Create deterministic outputs for this tick
        // Position output: [tick, tick, tick] as simple test data
        // Velocity output: [tick * 2] as simple test data
        let outputs = vec![
            (position_channel, vec![tick as u8, tick as u8, tick as u8]),
            (velocity_channel, vec![(tick * 2) as u8]),
        ];

        provenance
            .append(worldline_id, patch, triplet, outputs)
            .expect("append should succeed");
    }

    (provenance, initial_store, warp_id, worldline_id)
}

// ============================================================================
// T4: seek_moves_cursor_without_mutating_writer_store
// ============================================================================

/// T4: Reader cursor seeking does not mutate the writer's store.
///
/// This test verifies invariant CUR-001: Cursor never mutates worldline unless
/// role is Writer and mode requires advance.
#[test]
fn seek_moves_cursor_without_mutating_writer_store() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(20);

    // Create a "writer" cursor and advance it to tick 20 (simulate writer position)
    let mut writer_cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Writer,
        &initial_store,
        20,
    );

    // Simulate writer being at tick 20 by seeking (this is how we simulate writer state)
    writer_cursor
        .seek_to(20, &provenance, &initial_store)
        .expect("seek to 20 should succeed");
    assert_eq!(writer_cursor.tick, 20);

    // Snapshot writer's state_root
    let writer_state_root = compute_state_root_for_warp_store(&writer_cursor.store, warp_id);

    // Create a reader cursor at tick 20
    let mut reader_cursor = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        20,
    );

    // Seek reader to tick 20 first
    reader_cursor
        .seek_to(20, &provenance, &initial_store)
        .expect("reader seek to 20 should succeed");
    assert_eq!(reader_cursor.tick, 20);

    // Reader seeks back to tick 5
    reader_cursor
        .seek_to(5, &provenance, &initial_store)
        .expect("reader seek to 5 should succeed");

    // Assert reader is at tick 5
    assert_eq!(reader_cursor.tick, 5, "reader should be at tick 5");

    // Assert writer's state_root is unchanged
    let writer_state_root_after = compute_state_root_for_warp_store(&writer_cursor.store, warp_id);
    assert_eq!(
        writer_state_root, writer_state_root_after,
        "writer's state_root should be unchanged after reader seeks"
    );

    // Also verify writer is still at tick 20
    assert_eq!(writer_cursor.tick, 20, "writer tick should be unchanged");
}

// ============================================================================
// T5: step_back_is_seek_minus_one_then_pause
// ============================================================================

/// T5: StepBack mode seeks to tick-1 then transitions to Paused.
#[test]
fn step_back_is_seek_minus_one_then_pause() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(15);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        15,
    );

    // Position cursor at tick 10
    cursor
        .seek_to(10, &provenance, &initial_store)
        .expect("seek to 10 should succeed");
    assert_eq!(cursor.tick, 10);

    // Set mode to StepBack
    cursor.mode = PlaybackMode::StepBack;

    // Call step()
    let result = cursor.step(&provenance, &initial_store);

    // Assert result is Seeked
    assert!(result.is_ok(), "step should succeed: {:?}", result);
    assert_eq!(
        result.expect("step should succeed"),
        StepResult::Seeked,
        "StepBack should return Seeked"
    );

    // Assert tick == 9 (10 - 1)
    assert_eq!(cursor.tick, 9, "cursor should be at tick 9 after StepBack");

    // Assert mode == Paused
    assert_eq!(
        cursor.mode,
        PlaybackMode::Paused,
        "mode should be Paused after StepBack"
    );
}

// ============================================================================
// T6: reader_play_consumes_existing_then_pauses_at_frontier
// ============================================================================

/// T6: Reader in Play mode advances through existing history then pauses at frontier.
///
/// This test verifies that a reader cursor in Play mode consumes existing patches
/// until it reaches the pinned maximum tick (frontier), at which point it transitions
/// to Paused and does not create new patches.
#[test]
fn reader_play_consumes_existing_then_pauses_at_frontier() {
    // Worldline length 7 (patches 0-6, allowing ticks 0-7)
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(7);

    // Verify worldline has 7 patches
    assert_eq!(
        provenance.len(worldline_id).expect("len should succeed"),
        7,
        "worldline should have 7 patches"
    );

    // Create reader cursor at tick 0, mode = Play, pin_max_tick = 6
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        6, // pin_max_tick = 6
    );

    assert_eq!(cursor.tick, 0);
    cursor.mode = PlaybackMode::Play;

    // Step until tick reaches frontier (6)
    // Ticks: 0 -> 1 -> 2 -> 3 -> 4 -> 5 -> 6
    for expected_tick in 1..=6 {
        let result = cursor.step(&provenance, &initial_store);
        assert!(
            result.is_ok(),
            "step {} should succeed: {:?}",
            expected_tick,
            result
        );
        assert_eq!(
            result.expect("step should succeed"),
            StepResult::Advanced,
            "step {} should return Advanced",
            expected_tick
        );
        assert_eq!(
            cursor.tick, expected_tick,
            "cursor should be at tick {}",
            expected_tick
        );
        assert_eq!(
            cursor.mode,
            PlaybackMode::Play,
            "cursor should stay in Play mode until frontier"
        );
    }

    // Now cursor is at tick 6 (== pin_max_tick). Next step should hit frontier.
    let result = cursor.step(&provenance, &initial_store);
    assert!(result.is_ok(), "frontier step should succeed");
    assert_eq!(
        result.expect("step should succeed"),
        StepResult::ReachedFrontier,
        "should return ReachedFrontier at frontier"
    );

    // Assert mode transitions to Paused at frontier
    assert_eq!(
        cursor.mode,
        PlaybackMode::Paused,
        "mode should be Paused after reaching frontier"
    );

    // Assert tick stays at 6 (frontier)
    assert_eq!(cursor.tick, 6, "tick should stay at frontier (6)");

    // Assert no new patches were created (worldline length unchanged)
    assert_eq!(
        provenance.len(worldline_id).expect("len should succeed"),
        7,
        "worldline length should be unchanged (no new patches)"
    );
}

// ============================================================================
// T8: outputs_match_recorded_bytes_for_same_tick
// ============================================================================

/// T8: Outputs retrieved via publish_truth match recorded bytes from provenance store.
///
/// This test verifies invariant OUT-002: Playback at tick t reproduces the same
/// TruthFrames recorded at tick t.
#[test]
fn outputs_match_recorded_bytes_for_same_tick() {
    // Setup worldline with 15 ticks and outputs
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_outputs(15);

    let position_channel = make_channel_id("entity:position");
    let velocity_channel = make_channel_id("entity:velocity");

    // Create reader cursor
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        15,
    );

    // Create session subscribed to both channels
    let session_id = test_session_id(1);
    let mut session = ViewSession::new(session_id, cursor.cursor_id);
    session.subscribe(position_channel);
    session.subscribe(velocity_channel);

    // Test at tick k = 12
    let k = 12u64;

    // Seek cursor to tick k
    cursor
        .seek_to(k, &provenance, &initial_store)
        .expect("seek to tick k should succeed");
    assert_eq!(cursor.tick, k);

    // Publish truth
    let mut sink = TruthSink::new();
    session
        .publish_truth(&cursor, &provenance, &mut sink)
        .expect("publish_truth should succeed");

    // Get recorded outputs from provenance store
    let recorded_outputs = provenance
        .outputs(worldline_id, k)
        .expect("outputs should exist");

    // Get published frames
    let frames = sink.collect_frames(session_id);

    // Assert we got 2 frames (one per subscribed channel)
    assert_eq!(
        frames.len(),
        2,
        "should have 2 frames for 2 subscribed channels"
    );

    // Build a map of channel -> value from frames for easy lookup
    let frame_map: std::collections::BTreeMap<_, _> = frames
        .iter()
        .map(|f| (f.channel, f.value.clone()))
        .collect();

    // Assert each frame's value is byte-identical to recorded output
    for (channel, expected_value) in &recorded_outputs {
        let frame_value = frame_map
            .get(channel)
            .expect("frame should exist for channel");
        assert_eq!(
            frame_value, expected_value,
            "frame value for channel {:?} should match recorded output",
            channel
        );
    }

    // Also verify the expected values match our test setup
    // Position: [k, k, k] = [12, 12, 12]
    // Velocity: [k * 2] = [24]
    assert_eq!(
        frame_map.get(&position_channel),
        Some(&vec![12u8, 12u8, 12u8]),
        "position value should be [12, 12, 12]"
    );
    assert_eq!(
        frame_map.get(&velocity_channel),
        Some(&vec![24u8]),
        "velocity value should be [24]"
    );

    // Verify value_hash is blake3 of value
    for frame in &frames {
        let expected_hash: [u8; 32] = blake3::hash(&frame.value).into();
        assert_eq!(
            frame.value_hash, expected_hash,
            "value_hash should be blake3 of value"
        );
    }

    // Verify cursor receipt information
    let receipt = sink.last_receipt(session_id).expect("receipt should exist");
    assert_eq!(receipt.tick, k, "receipt tick should match cursor tick");
    assert_eq!(
        receipt.worldline_id, worldline_id,
        "receipt worldline should match"
    );
    assert_eq!(receipt.warp_id, warp_id, "receipt warp should match");
    assert_eq!(
        receipt.session_id, session_id,
        "receipt session should match"
    );
}

// ============================================================================
// MBUS v2 Integration Test
// ============================================================================

/// Integration test: TruthFrames can be converted to V2Packet and roundtrip through encode/decode.
///
/// This verifies the full flow: recorded outputs -> TruthFrame -> V2Packet encoding.
#[test]
fn truth_frames_encode_to_mbus_v2() {
    // Setup worldline with outputs
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_outputs(10);

    let position_channel = make_channel_id("entity:position");

    // Create reader cursor and seek to tick 7
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );
    cursor
        .seek_to(7, &provenance, &initial_store)
        .expect("seek should succeed");

    // Create session subscribed to position channel
    let session_id = test_session_id(1);
    let mut session = ViewSession::new(session_id, cursor.cursor_id);
    session.subscribe(position_channel);

    // Publish truth to sink
    let mut sink = TruthSink::new();
    session
        .publish_truth(&cursor, &provenance, &mut sink)
        .expect("publish_truth should succeed");

    // Get frames
    let frames = sink.collect_frames(session_id);
    assert!(!frames.is_empty(), "should have at least one frame");

    let frame = &frames[0];

    // Convert TruthFrame to V2Packet
    let v2_header = V2PacketHeader {
        session_id: frame.cursor.session_id.0,
        cursor_id: frame.cursor.cursor_id.0,
        worldline_id: frame.cursor.worldline_id.0,
        warp_id: frame.cursor.warp_id,
        tick: frame.cursor.tick,
        commit_hash: frame.cursor.commit_hash,
    };

    let v2_entries: Vec<V2Entry> = frames
        .iter()
        .filter(|f| f.cursor == frame.cursor) // Same cursor receipt
        .map(|f| V2Entry {
            channel: f.channel,
            value_hash: compute_value_hash(&f.value),
            value: f.value.clone(),
        })
        .collect();

    // Encode to bytes
    let encoded = encode_v2_packet(&v2_header, &v2_entries);

    // Decode back
    let decoded = decode_v2_packet(&encoded).expect("decode should succeed");

    // Verify roundtrip
    assert_eq!(decoded.header.session_id, session_id.0);
    assert_eq!(decoded.header.cursor_id, cursor.cursor_id.0);
    assert_eq!(decoded.header.worldline_id, worldline_id.0);
    assert_eq!(decoded.header.warp_id, warp_id);
    assert_eq!(decoded.header.tick, 7);

    assert_eq!(decoded.entries.len(), v2_entries.len());
    for (decoded_entry, original_entry) in decoded.entries.iter().zip(v2_entries.iter()) {
        assert_eq!(decoded_entry.channel, original_entry.channel);
        assert_eq!(decoded_entry.value, original_entry.value);
        assert_eq!(decoded_entry.value_hash, original_entry.value_hash);
    }

    // Verify the actual values match expected from test setup
    // At tick 7, position should be [7, 7, 7]
    let position_entry = decoded
        .entries
        .iter()
        .find(|e| e.channel == position_channel);
    assert!(
        position_entry.is_some(),
        "should have position channel entry"
    );
    assert_eq!(
        position_entry.expect("position entry should exist").value,
        vec![7u8, 7u8, 7u8],
        "position value should be [7, 7, 7] at tick 7"
    );
}

// ============================================================================
// Additional test: publish_truth only publishes subscribed channels
// ============================================================================

/// Test that publish_truth only publishes frames for subscribed channels.
#[test]
fn publish_truth_filters_by_subscription() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_outputs(5);

    let position_channel = make_channel_id("entity:position");
    let velocity_channel = make_channel_id("entity:velocity");
    let unsubscribed_channel = make_channel_id("entity:unsubscribed");

    // Create reader cursor
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        5,
    );
    cursor
        .seek_to(3, &provenance, &initial_store)
        .expect("seek should succeed");

    // Create session subscribed ONLY to position channel
    let session_id = test_session_id(1);
    let mut session = ViewSession::new(session_id, cursor.cursor_id);
    session.subscribe(position_channel);
    // Note: NOT subscribing to velocity_channel

    // Publish truth
    let mut sink = TruthSink::new();
    session
        .publish_truth(&cursor, &provenance, &mut sink)
        .expect("publish_truth should succeed");

    // Get frames
    let frames = sink.collect_frames(session_id);

    // Should only have position frame, not velocity
    assert_eq!(
        frames.len(),
        1,
        "should only have 1 frame for subscribed channel"
    );
    assert_eq!(
        frames[0].channel, position_channel,
        "frame should be for position channel"
    );

    // Verify no frames for unsubscribed channels
    for frame in &frames {
        assert_ne!(
            frame.channel, velocity_channel,
            "should not have velocity frame"
        );
        assert_ne!(
            frame.channel, unsubscribed_channel,
            "should not have unsubscribed channel frame"
        );
    }
}

// ============================================================================
// Additional test: publish_truth returns error for unavailable tick
// ============================================================================

/// Test that publish_truth returns HistoryError for unavailable tick.
#[test]
fn publish_truth_returns_error_for_unavailable_tick() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_outputs(5);

    let position_channel = make_channel_id("entity:position");

    // Create cursor that claims to be at tick 100 (beyond history)
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        100,
    );
    // Manually set tick beyond history (bypassing seek validation)
    cursor.tick = 100;

    // Create session
    let session_id = test_session_id(1);
    let mut session = ViewSession::new(session_id, cursor.cursor_id);
    session.subscribe(position_channel);

    // Publish truth should fail
    let mut sink = TruthSink::new();
    let result = session.publish_truth(&cursor, &provenance, &mut sink);

    assert!(
        result.is_err(),
        "publish_truth should fail for unavailable tick"
    );
    assert!(
        matches!(result, Err(HistoryError::HistoryUnavailable { tick: 100 })),
        "should be HistoryUnavailable error for tick 100, got: {:?}",
        result
    );
}
