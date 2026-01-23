// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Outputs playback tests for SPEC-0004 Commit 5: Record Outputs Per Tick + Seek/Playback.
//!
//! These tests verify:
//! - T4: seek_moves_cursor_without_mutating_writer_store
//! - T5: step_back_is_seek_minus_one_then_pause
//! - T6: reader_play_consumes_existing_then_pauses_at_frontier
//! - T8: outputs_match_recorded_bytes_for_same_tick
//! - MBUS v2 integration: truth_frames_encode_to_mbus_v2

mod common;
use common::{
    create_add_node_patch, create_initial_store, setup_worldline_with_ticks, test_cursor_id,
    test_session_id, test_warp_id, test_worldline_id,
};

use warp_core::materialization::{
    compute_value_hash, decode_v2_packet, encode_v2_packet, make_channel_id, V2Entry,
    V2PacketHeader,
};
use warp_core::{
    compute_commit_hash_v2, compute_state_root_for_warp_store, CursorRole, GraphStore, HashTriplet,
    LocalProvenanceStore, PlaybackCursor, PlaybackMode, ProvenanceStore, SeekError, StepResult,
    TruthSink, ViewSession, WorldlineId,
};

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
    provenance
        .register_worldline(worldline_id, warp_id)
        .unwrap();

    // Build up the worldline by applying patches and recording correct hashes
    let mut current_store = initial_store.clone();
    let mut parents: Vec<warp_core::Hash> = Vec::new();

    let position_channel = make_channel_id("entity:position");
    let velocity_channel = make_channel_id("entity:velocity");

    assert!(
        num_ticks <= 127,
        "num_ticks must be <= 127 to avoid u8 overflow in test output data"
    );

    for tick in 0..num_ticks {
        let patch = create_add_node_patch(warp_id, tick, &format!("node-{}", tick));

        // Apply patch to get the resulting state
        patch
            .apply_to_store(&mut current_store)
            .expect("apply should succeed");

        // Compute the actual state root after applying
        let state_root = compute_state_root_for_warp_store(&current_store, warp_id);

        // Compute real commit_hash for Merkle chain verification
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

        // Advance parent chain
        parents = vec![commit_hash];
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

    // 12 ticks: enough to exercise mid-history playback without hitting boundaries (0 or 15)
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

    // Get recorded outputs from provenance store.
    // publish_truth queries prov_tick = cursor.tick - 1 (0-based index of last applied patch).
    let prov_tick = k - 1;
    let recorded_outputs = provenance
        .outputs(worldline_id, prov_tick)
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

    // Also verify the expected values match our test setup.
    // publish_truth at cursor.tick=k returns provenance outputs from index k-1.
    // Position: [k-1, k-1, k-1] = [11, 11, 11]
    // Velocity: [(k-1) * 2] = [22]
    assert_eq!(
        frame_map.get(&position_channel),
        Some(&vec![11u8, 11u8, 11u8]),
        "position value should be [11, 11, 11] (prov_tick = k-1)"
    );
    assert_eq!(
        frame_map.get(&velocity_channel),
        Some(&vec![22u8]),
        "velocity value should be [22] (prov_tick = k-1)"
    );

    // Verify value_hash is blake3 of value
    for frame in frames {
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
    let encoded = encode_v2_packet(&v2_header, &v2_entries).expect("encode should succeed");

    // Decode back
    let decoded = decode_v2_packet(&encoded).expect("decode should succeed");

    // Verify roundtrip
    assert_eq!(decoded.header.session_id, session_id.0);
    assert_eq!(decoded.header.cursor_id, cursor.cursor_id.0);
    assert_eq!(decoded.header.worldline_id, worldline_id.0);
    assert_eq!(decoded.header.warp_id, warp_id);
    assert_eq!(decoded.header.tick, 7);
    assert_eq!(
        decoded.header.commit_hash, v2_header.commit_hash,
        "commit_hash should round-trip through v2 encode/decode"
    );

    assert_eq!(decoded.entries.len(), v2_entries.len());
    for (decoded_entry, original_entry) in decoded.entries.iter().zip(v2_entries.iter()) {
        assert_eq!(decoded_entry.channel, original_entry.channel);
        assert_eq!(decoded_entry.value, original_entry.value);
        assert_eq!(decoded_entry.value_hash, original_entry.value_hash);
    }

    // Verify the actual values match expected from test setup.
    // At cursor.tick=7, publish_truth queries prov_tick=6, so position is [6, 6, 6].
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
        vec![6u8, 6u8, 6u8],
        "position value should be [6, 6, 6] at cursor.tick 7 (prov_tick=6)"
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
    let other_channel = make_channel_id("entity:unsubscribed");

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
    for frame in frames {
        assert_ne!(
            frame.channel, velocity_channel,
            "should not have velocity frame"
        );
        assert_ne!(
            frame.channel, other_channel,
            "should not have unsubscribed channel frame"
        );
    }
}

// ============================================================================
// Additional test: publish_truth returns error for unavailable tick
// ============================================================================

/// Test boundary behavior of publish_truth after the off-by-one fix.
///
/// With 5 patches (indices 0-4), cursor.tick=5 means all patches applied.
/// publish_truth queries prov_tick = 5-1 = 4, which is the last valid index.
/// This should succeed and return outputs from provenance[4].
/// We also verify that seek_to properly rejects ticks far beyond history,
/// and that publish_truth at tick 0 (no patches applied) returns Ok with no frames.
#[test]
fn publish_truth_returns_error_for_unavailable_tick() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_outputs(5);

    let position_channel = make_channel_id("entity:position");

    // First, verify seek_to properly rejects ticks beyond available history.
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        100,
    );
    let seek_result = cursor.seek_to(100, &provenance, &initial_store);
    assert!(
        matches!(
            seek_result,
            Err(SeekError::HistoryUnavailable { tick: 100 })
        ),
        "seek_to(100) should fail with HistoryUnavailable, got: {:?}",
        seek_result
    );

    // Seek to tick 5 (boundary: valid since 5 <= history_len).
    // With the off-by-one fix, publish_truth queries prov_tick=4 which IS valid.
    cursor
        .seek_to(5, &provenance, &initial_store)
        .expect("seek_to(5) should succeed at boundary tick");

    // Create session
    let session_id = test_session_id(1);
    let mut session = ViewSession::new(session_id, cursor.cursor_id);
    session.subscribe(position_channel);

    // publish_truth at boundary tick 5 should SUCCEED (prov_tick=4 exists).
    let mut sink = TruthSink::new();
    let result = session.publish_truth(&cursor, &provenance, &mut sink);
    assert!(
        result.is_ok(),
        "publish_truth should succeed at boundary tick 5 (prov_tick=4), got: {:?}",
        result
    );

    // Verify it returns correct data from provenance[4]: position=[4, 4, 4]
    let frames = sink.collect_frames(session_id);
    assert_eq!(frames.len(), 1, "should have 1 frame for position channel");
    assert_eq!(
        frames[0].value,
        vec![4u8, 4u8, 4u8],
        "position at prov_tick=4 should be [4, 4, 4]"
    );

    // Verify publish_truth at tick 0 returns Ok with no frames (no patches applied).
    let cursor_zero = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        5,
    );
    // cursor starts at tick 0 by default
    assert_eq!(cursor_zero.tick, 0);

    let mut session_zero = ViewSession::new(test_session_id(2), cursor_zero.cursor_id);
    session_zero.subscribe(position_channel);

    let mut sink_zero = TruthSink::new();
    let result_zero = session_zero.publish_truth(&cursor_zero, &provenance, &mut sink_zero);
    assert!(
        result_zero.is_ok(),
        "publish_truth at tick 0 should succeed with no output"
    );
    let frames_zero = sink_zero.collect_frames(test_session_id(2));
    assert!(
        frames_zero.is_empty(),
        "publish_truth at tick 0 should produce no frames"
    );
}

// ============================================================================
// T1: writer_play_advances_and_records_outputs
// ============================================================================

/// T1: Simulate a writer advancing and recording outputs via hexagonal architecture.
///
/// This test demonstrates that we can simulate the engine's write behavior by manually
/// calling `provenance.append()`. The ProvenanceStore trait (port) allows us to test
/// the write side of the provenance contract without needing the real engine.
#[test]
fn writer_play_advances_and_records_outputs() {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_store = create_initial_store(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    provenance
        .register_worldline(worldline_id, warp_id)
        .unwrap();

    // Simulate writer advancing 10 ticks
    let mut current_store = initial_store.clone();
    let output_channel = make_channel_id("writer:output");
    let mut parents: Vec<warp_core::Hash> = Vec::new();

    for tick in 0..10u64 {
        // Create a patch for this tick
        let patch = create_add_node_patch(warp_id, tick, &format!("writer-node-{}", tick));

        // Apply the patch to get the resulting state
        patch
            .apply_to_store(&mut current_store)
            .expect("apply should succeed");

        // Compute state_root from the store
        let state_root = compute_state_root_for_warp_store(&current_store, warp_id);

        // Compute real commit_hash for Merkle chain validity
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

        // Create outputs with deterministic values: (channel, vec![tick as u8])
        let outputs = vec![(output_channel, vec![tick as u8])];

        // Call provenance.append() - this is the hexagonal architecture pattern
        provenance
            .append(worldline_id, patch, triplet, outputs)
            .expect("append should succeed");

        // Advance parent chain for next iteration's Merkle computation
        parents = vec![commit_hash];
    }

    // Assert: provenance.len(worldline) == 10
    assert_eq!(
        provenance.len(worldline_id).expect("len should succeed"),
        10,
        "provenance should have 10 entries"
    );

    // Assert: provenance.expected(worldline, t) exists for t in 0..10
    // Recompute the Merkle chain to verify stored commit_hashes match
    let mut verify_store = initial_store.clone();
    let mut verify_parents: Vec<warp_core::Hash> = Vec::new();
    for tick in 0..10u64 {
        let triplet = provenance
            .expected(worldline_id, tick)
            .expect("expected should exist for tick");

        // Recompute commit_hash from scratch to verify Merkle chain integrity
        let patch = provenance
            .patch(worldline_id, tick)
            .expect("patch should exist");
        patch
            .apply_to_store(&mut verify_store)
            .expect("apply should succeed");
        let state_root = compute_state_root_for_warp_store(&verify_store, warp_id);
        let expected_commit = compute_commit_hash_v2(
            &state_root,
            &verify_parents,
            &patch.patch_digest,
            patch.header.policy_id,
        );

        assert_eq!(
            triplet.commit_hash, expected_commit,
            "commit_hash should match recomputed value for tick {}",
            tick
        );
        verify_parents = vec![expected_commit];
    }

    // Assert: provenance.outputs(worldline, t) contains expected values
    for tick in 0..10u64 {
        let outputs = provenance
            .outputs(worldline_id, tick)
            .expect("outputs should exist for tick");

        assert_eq!(outputs.len(), 1, "should have 1 output for tick {}", tick);
        assert_eq!(
            outputs[0].0, output_channel,
            "output channel should match for tick {}",
            tick
        );
        assert_eq!(
            outputs[0].1,
            vec![tick as u8],
            "output value should be [{}] for tick {}",
            tick,
            tick
        );
    }
}

// ============================================================================
// T7: truth_frames_are_cursor_addressed_and_authoritative
// ============================================================================

/// T7: TruthFrames contain proper cursor context and are authoritative.
///
/// This test verifies that TruthFrames include correct cursor addressing (tick, commit_hash)
/// and that seeking to multiple different ticks produces consistent, authoritative frames.
#[test]
fn truth_frames_are_cursor_addressed_and_authoritative() {
    // Setup worldline with 10 ticks and outputs
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_outputs(10);

    let position_channel = make_channel_id("entity:position");

    // Create reader cursor
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    // Create session subscribed to position channel
    let session_id = test_session_id(1);
    let mut session = ViewSession::new(session_id, cursor.cursor_id);
    session.subscribe(position_channel);

    // Test 1: Seek to tick 3, verify receipt and frame values
    cursor
        .seek_to(3, &provenance, &initial_store)
        .expect("seek to tick 3 should succeed");
    assert_eq!(cursor.tick, 3);

    let mut sink = TruthSink::new();
    session
        .publish_truth(&cursor, &provenance, &mut sink)
        .expect("publish_truth at tick 3 should succeed");

    let receipt_3 = sink.last_receipt(session_id).expect("receipt should exist");

    // Verify receipt has tick == 3
    assert_eq!(receipt_3.tick, 3, "receipt tick should be 3");

    // publish_truth queries prov_tick = cursor.tick - 1 = 2
    let expected_triplet_3 = provenance
        .expected(worldline_id, 2)
        .expect("expected triplet for prov_tick 2 should exist");
    assert_eq!(
        receipt_3.commit_hash, expected_triplet_3.commit_hash,
        "receipt commit_hash should match provenance.expected(worldline, 2)"
    );

    // Verify frame values match provenance.outputs(worldline, 2) (prov_tick = cursor.tick - 1)
    let frames_3 = sink.collect_frames(session_id);
    let recorded_outputs_3 = provenance
        .outputs(worldline_id, 2)
        .expect("outputs for prov_tick 2 should exist");

    let position_output_3 = recorded_outputs_3
        .iter()
        .find(|(ch, _)| *ch == position_channel)
        .map(|(_, v)| v.clone())
        .expect("position output should exist at tick 3");

    let position_frame_3 = frames_3
        .iter()
        .find(|f| f.channel == position_channel)
        .expect("position frame should exist at tick 3");

    assert_eq!(
        position_frame_3.value, position_output_3,
        "frame value at tick 3 should match recorded output"
    );
    // Expected: at cursor.tick=3, prov_tick=2, position is [2, 2, 2]
    assert_eq!(
        position_frame_3.value,
        vec![2u8, 2u8, 2u8],
        "position value should be [2, 2, 2] at cursor.tick 3 (prov_tick=2)"
    );

    // Test 2: Seek to tick 7, verify same invariants
    cursor
        .seek_to(7, &provenance, &initial_store)
        .expect("seek to tick 7 should succeed");
    assert_eq!(cursor.tick, 7);

    let mut sink = TruthSink::new();
    session
        .publish_truth(&cursor, &provenance, &mut sink)
        .expect("publish_truth at tick 7 should succeed");

    let receipt_7 = sink.last_receipt(session_id).expect("receipt should exist");

    // Verify receipt has tick == 7
    assert_eq!(receipt_7.tick, 7, "receipt tick should be 7");

    // publish_truth queries prov_tick = cursor.tick - 1 = 6
    let expected_triplet_7 = provenance
        .expected(worldline_id, 6)
        .expect("expected triplet for prov_tick 6 should exist");
    assert_eq!(
        receipt_7.commit_hash, expected_triplet_7.commit_hash,
        "receipt commit_hash should match provenance.expected(worldline, 6)"
    );

    // Verify frame values match provenance.outputs(worldline, 6) (prov_tick = cursor.tick - 1)
    let frames_7 = sink.collect_frames(session_id);
    let recorded_outputs_7 = provenance
        .outputs(worldline_id, 6)
        .expect("outputs for prov_tick 6 should exist");

    let position_output_7 = recorded_outputs_7
        .iter()
        .find(|(ch, _)| *ch == position_channel)
        .map(|(_, v)| v.clone())
        .expect("position output should exist at tick 7");

    let position_frame_7 = frames_7
        .iter()
        .find(|f| f.channel == position_channel)
        .expect("position frame should exist at tick 7");

    assert_eq!(
        position_frame_7.value, position_output_7,
        "frame value at tick 7 should match recorded output"
    );
    // Expected: at cursor.tick=7, prov_tick=6, position is [6, 6, 6]
    assert_eq!(
        position_frame_7.value,
        vec![6u8, 6u8, 6u8],
        "position value should be [6, 6, 6] at cursor.tick 7 (prov_tick=6)"
    );

    // Verify different ticks have different commit_hashes (authoritative)
    assert_ne!(
        receipt_3.commit_hash, receipt_7.commit_hash,
        "commit_hash at tick 3 should differ from tick 7"
    );
}
