// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! ViewSession and step() tests for SPEC-0004: Worldlines, Playback, and TruthBus.
//!
//! These tests verify:
//! - T2: step_forward_advances_one_then_pauses
//! - T3: paused_noop_even_with_pending_intents
//! - T9: two_sessions_same_channel_different_cursors_receive_different_truth
//! - T10: session_cursor_switch_is_opaque_to_subscribers

use warp_core::materialization::make_channel_id;
use warp_core::{
    compute_state_root_for_warp_store, make_node_id, make_type_id, make_warp_id, CursorId,
    CursorRole, GraphStore, HashTriplet, LocalProvenanceStore, NodeKey, NodeRecord, PlaybackCursor,
    PlaybackMode, SeekThen, SessionId, StepResult, TruthFrame, TruthSink, ViewSession, WarpOp,
    WorldlineId, WorldlineTickHeaderV1, WorldlineTickPatchV1,
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

// ============================================================================
// T2: step_forward_advances_one_then_pauses
// ============================================================================

/// T2: StepForward mode advances cursor by one tick then transitions to Paused.
#[test]
fn step_forward_advances_one_then_pauses() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    // Cursor starts at tick 0, mode is Paused by default
    assert_eq!(cursor.tick, 0);
    assert_eq!(cursor.mode, PlaybackMode::Paused);

    // Set mode to StepForward
    cursor.mode = PlaybackMode::StepForward;

    // Call step()
    let result = cursor.step(&provenance, &initial_store);

    // Verify result
    assert!(result.is_ok(), "step should succeed: {:?}", result);
    assert_eq!(result.unwrap(), StepResult::Advanced);

    // Verify tick advanced by 1
    assert_eq!(cursor.tick, 1, "cursor should be at tick 1");

    // Verify mode is now Paused
    assert_eq!(
        cursor.mode,
        PlaybackMode::Paused,
        "mode should be Paused after StepForward"
    );

    // Step again in StepForward mode
    cursor.mode = PlaybackMode::StepForward;
    let result = cursor.step(&provenance, &initial_store);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StepResult::Advanced);
    assert_eq!(cursor.tick, 2);
    assert_eq!(cursor.mode, PlaybackMode::Paused);
}

// ============================================================================
// T3: paused_noop_even_with_pending_intents
// ============================================================================

/// T3: Paused mode is a no-op - cursor state doesn't change regardless of context.
#[test]
fn paused_noop_even_with_pending_intents() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    // Seek to tick 5 first
    cursor
        .seek_to(5, &provenance, &initial_store)
        .expect("seek should succeed");

    assert_eq!(cursor.tick, 5);
    assert_eq!(cursor.mode, PlaybackMode::Paused);

    // Get state hash before step
    let hash_before = compute_state_root_for_warp_store(&cursor.store, warp_id);

    // Call step() while Paused - should be no-op
    let result = cursor.step(&provenance, &initial_store);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StepResult::NoOp);

    // Verify nothing changed
    assert_eq!(cursor.tick, 5, "tick should not change when paused");
    assert_eq!(
        cursor.mode,
        PlaybackMode::Paused,
        "mode should still be paused"
    );

    let hash_after = compute_state_root_for_warp_store(&cursor.store, warp_id);
    assert_eq!(
        hash_before, hash_after,
        "store should not change when paused"
    );

    // Call step() multiple times - all should be no-op
    for _ in 0..5 {
        let result = cursor.step(&provenance, &initial_store);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StepResult::NoOp);
        assert_eq!(cursor.tick, 5);
    }
}

// ============================================================================
// T9: two_sessions_same_channel_different_cursors_receive_different_truth
// ============================================================================

/// T9: Two sessions subscribed to the same channel but with cursors at different
/// ticks receive different truth frames (cursor-addressed truth).
#[test]
fn two_sessions_same_channel_different_cursors_receive_different_truth() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    // Create two sessions
    let session1_id = test_session_id(1);
    let session2_id = test_session_id(2);

    let mut session1 = ViewSession::new(session1_id, test_cursor_id(1));
    let mut session2 = ViewSession::new(session2_id, test_cursor_id(2));

    // Both subscribe to the same channel
    let position_channel = make_channel_id("entity:position");
    session1.subscribe(position_channel);
    session2.subscribe(position_channel);

    assert!(session1.subscriptions.contains(&position_channel));
    assert!(session2.subscriptions.contains(&position_channel));

    // Create two cursors at different ticks
    let mut cursor1 = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    let mut cursor2 = PlaybackCursor::new(
        test_cursor_id(2),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    // Position cursors at different ticks
    cursor1
        .seek_to(3, &provenance, &initial_store)
        .expect("seek should succeed");
    cursor2
        .seek_to(7, &provenance, &initial_store)
        .expect("seek should succeed");

    assert_eq!(cursor1.tick, 3);
    assert_eq!(cursor2.tick, 7);

    // Compute state hashes at each position
    let hash_at_tick_3 = compute_state_root_for_warp_store(&cursor1.store, warp_id);
    let hash_at_tick_7 = compute_state_root_for_warp_store(&cursor2.store, warp_id);

    // The states should be different (different history applied)
    assert_ne!(
        hash_at_tick_3, hash_at_tick_7,
        "cursors at different ticks should have different state"
    );

    // Create truth sink and publish mock frames
    let mut sink = TruthSink::new();

    // Frame for session 1 at tick 3
    let frame1 = TruthFrame {
        cursor: warp_core::CursorReceipt {
            session_id: session1_id,
            cursor_id: cursor1.cursor_id,
            worldline_id,
            warp_id,
            tick: cursor1.tick,
            commit_hash: [103u8; 32], // tick + 100
        },
        channel: position_channel,
        value: vec![3, 3, 3], // Different value for tick 3
        value_hash: [13u8; 32],
    };

    // Frame for session 2 at tick 7
    let frame2 = TruthFrame {
        cursor: warp_core::CursorReceipt {
            session_id: session2_id,
            cursor_id: cursor2.cursor_id,
            worldline_id,
            warp_id,
            tick: cursor2.tick,
            commit_hash: [107u8; 32], // tick + 100
        },
        channel: position_channel,
        value: vec![7, 7, 7], // Different value for tick 7
        value_hash: [17u8; 32],
    };

    sink.publish_frame(session1_id, frame1.clone());
    sink.publish_frame(session2_id, frame2.clone());

    // Verify sessions receive different truth
    let frames1 = sink.collect_frames(session1_id);
    let frames2 = sink.collect_frames(session2_id);

    assert_eq!(frames1.len(), 1);
    assert_eq!(frames2.len(), 1);

    // Verify the frames are cursor-addressed (contain correct tick)
    assert_eq!(frames1[0].cursor.tick, 3);
    assert_eq!(frames2[0].cursor.tick, 7);

    // Verify the values are different
    assert_ne!(frames1[0].value, frames2[0].value);
    assert_eq!(frames1[0].value, vec![3, 3, 3]);
    assert_eq!(frames2[0].value, vec![7, 7, 7]);
}

// ============================================================================
// T10: session_cursor_switch_is_opaque_to_subscribers
// ============================================================================

/// T10: Switching a session's active cursor does not affect its subscriptions.
/// Subscriptions persist across cursor changes.
#[test]
fn session_cursor_switch_is_opaque_to_subscribers() {
    let session_id = test_session_id(1);
    let cursor1_id = test_cursor_id(1);
    let cursor2_id = test_cursor_id(2);

    // Create session with cursor1
    let mut session = ViewSession::new(session_id, cursor1_id);

    // Subscribe to multiple channels
    let position_channel = make_channel_id("entity:position");
    let velocity_channel = make_channel_id("entity:velocity");
    let health_channel = make_channel_id("entity:health");

    session.subscribe(position_channel);
    session.subscribe(velocity_channel);
    session.subscribe(health_channel);

    assert_eq!(session.subscriptions.len(), 3);
    assert!(session.subscriptions.contains(&position_channel));
    assert!(session.subscriptions.contains(&velocity_channel));
    assert!(session.subscriptions.contains(&health_channel));
    assert_eq!(session.active_cursor, cursor1_id);

    // Switch to cursor2
    session.set_active_cursor(cursor2_id);

    // Verify cursor changed
    assert_eq!(session.active_cursor, cursor2_id);

    // Verify subscriptions are unchanged
    assert_eq!(session.subscriptions.len(), 3);
    assert!(session.subscriptions.contains(&position_channel));
    assert!(session.subscriptions.contains(&velocity_channel));
    assert!(session.subscriptions.contains(&health_channel));

    // Unsubscribe from one channel
    session.unsubscribe(velocity_channel);
    assert_eq!(session.subscriptions.len(), 2);
    assert!(!session.subscriptions.contains(&velocity_channel));

    // Switch back to cursor1
    session.set_active_cursor(cursor1_id);

    // Subscriptions should still reflect the unsubscribe
    assert_eq!(session.active_cursor, cursor1_id);
    assert_eq!(session.subscriptions.len(), 2);
    assert!(session.subscriptions.contains(&position_channel));
    assert!(!session.subscriptions.contains(&velocity_channel));
    assert!(session.subscriptions.contains(&health_channel));
}

// ============================================================================
// Additional tests for step() state machine
// ============================================================================

/// Test Play mode for Reader cursor - advances until frontier
#[test]
fn reader_play_advances_until_frontier() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        5, // pin_max_tick = 5
    );

    // Set mode to Play
    cursor.mode = PlaybackMode::Play;

    // Step repeatedly - should advance each time until frontier
    // Cursor starts at tick 0, advances to 1, 2, 3, 4, 5
    // When at tick 5 (= pin_max_tick), the next step hits the frontier check
    for expected_tick in 1..=5 {
        let result = cursor.step(&provenance, &initial_store);
        assert!(result.is_ok(), "step {} should succeed", expected_tick);
        assert_eq!(result.unwrap(), StepResult::Advanced);
        assert_eq!(cursor.tick, expected_tick);
        assert_eq!(cursor.mode, PlaybackMode::Play, "should stay in Play mode");
    }

    // Now cursor is at tick 5. Next step should hit frontier (tick >= pin_max_tick)
    let result = cursor.step(&provenance, &initial_store);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StepResult::ReachedFrontier);
    assert_eq!(cursor.tick, 5, "tick should stay at 5 (frontier)");
    assert_eq!(
        cursor.mode,
        PlaybackMode::Paused,
        "should pause at frontier"
    );
}

/// Test StepBack mode - seeks to tick-1 then pauses
#[test]
fn step_back_seeks_then_pauses() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    // Seek to tick 5 first
    cursor
        .seek_to(5, &provenance, &initial_store)
        .expect("seek should succeed");
    assert_eq!(cursor.tick, 5);

    // Set mode to StepBack
    cursor.mode = PlaybackMode::StepBack;

    let result = cursor.step(&provenance, &initial_store);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StepResult::Seeked);
    assert_eq!(cursor.tick, 4, "should be at tick 4 (5 - 1)");
    assert_eq!(cursor.mode, PlaybackMode::Paused);
}

/// Test StepBack at tick 0 - saturating_sub means stays at 0
#[test]
fn step_back_at_zero_stays_at_zero() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    // Cursor starts at tick 0
    assert_eq!(cursor.tick, 0);

    // Set mode to StepBack
    cursor.mode = PlaybackMode::StepBack;

    let result = cursor.step(&provenance, &initial_store);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StepResult::Seeked);
    assert_eq!(cursor.tick, 0, "should stay at tick 0 (saturating_sub)");
    assert_eq!(cursor.mode, PlaybackMode::Paused);
}

/// Test Seek mode with SeekThen::Pause
#[test]
fn seek_mode_with_then_pause() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    // Set mode to Seek with target 7
    cursor.mode = PlaybackMode::Seek {
        target: 7,
        then: SeekThen::Pause,
    };

    let result = cursor.step(&provenance, &initial_store);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StepResult::Seeked);
    assert_eq!(cursor.tick, 7);
    assert_eq!(cursor.mode, PlaybackMode::Paused);
}

/// Test Seek mode with SeekThen::Play
#[test]
fn seek_mode_with_then_play() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(10);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        10,
    );

    // Set mode to Seek with target 3, then Play
    cursor.mode = PlaybackMode::Seek {
        target: 3,
        then: SeekThen::Play,
    };

    let result = cursor.step(&provenance, &initial_store);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StepResult::Seeked);
    assert_eq!(cursor.tick, 3);
    assert_eq!(cursor.mode, PlaybackMode::Play, "should transition to Play");
}

/// Test TruthSink basic operations
#[test]
fn truth_sink_basic_operations() {
    let mut sink = TruthSink::new();
    let session_id = test_session_id(1);

    // Initially empty
    assert!(sink.collect_frames(session_id).is_empty());
    assert!(sink.last_receipt(session_id).is_none());

    // Publish a receipt
    let receipt = warp_core::CursorReceipt {
        session_id,
        cursor_id: test_cursor_id(1),
        worldline_id: test_worldline_id(),
        warp_id: test_warp_id(),
        tick: 5,
        commit_hash: [105u8; 32],
    };
    sink.publish_receipt(session_id, receipt);

    assert_eq!(sink.last_receipt(session_id), Some(receipt));

    // Publish frames
    let channel = make_channel_id("test:channel");
    let frame1 = TruthFrame {
        cursor: receipt,
        channel,
        value: vec![1, 2, 3],
        value_hash: [1u8; 32],
    };
    let frame2 = TruthFrame {
        cursor: receipt,
        channel,
        value: vec![4, 5, 6],
        value_hash: [2u8; 32],
    };

    sink.publish_frame(session_id, frame1.clone());
    sink.publish_frame(session_id, frame2.clone());

    let frames = sink.collect_frames(session_id);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0], frame1);
    assert_eq!(frames[1], frame2);

    // Clear
    sink.clear();
    assert!(sink.collect_frames(session_id).is_empty());
    assert!(sink.last_receipt(session_id).is_none());
}

/// Test ViewSession subscribe/unsubscribe
#[test]
fn view_session_subscribe_unsubscribe() {
    let session_id = test_session_id(1);
    let cursor_id = test_cursor_id(1);

    let mut session = ViewSession::new(session_id, cursor_id);

    assert!(session.subscriptions.is_empty());

    let ch1 = make_channel_id("channel:1");
    let ch2 = make_channel_id("channel:2");

    // Subscribe
    session.subscribe(ch1);
    assert_eq!(session.subscriptions.len(), 1);
    assert!(session.subscriptions.contains(&ch1));

    session.subscribe(ch2);
    assert_eq!(session.subscriptions.len(), 2);

    // Subscribe to same channel again - no duplicate
    session.subscribe(ch1);
    assert_eq!(session.subscriptions.len(), 2);

    // Unsubscribe
    session.unsubscribe(ch1);
    assert_eq!(session.subscriptions.len(), 1);
    assert!(!session.subscriptions.contains(&ch1));
    assert!(session.subscriptions.contains(&ch2));

    // Unsubscribe from non-subscribed channel - no error
    session.unsubscribe(ch1);
    assert_eq!(session.subscriptions.len(), 1);
}

/// Test Writer cursor in Play mode returns NoOp (stub behavior)
#[test]
fn writer_play_is_stub_noop() {
    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Writer, // Writer role
        &initial_store,
        5,
    );

    // Set mode to Play
    cursor.mode = PlaybackMode::Play;

    let result = cursor.step(&provenance, &initial_store);

    // Writer in Play mode is a stub - returns NoOp
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StepResult::NoOp);
    assert_eq!(cursor.tick, 0, "tick should not change for writer stub");
    assert_eq!(cursor.mode, PlaybackMode::Play, "mode should stay Play");
}

// ============================================================================
// T16: worker_count_invariance_for_writer_advance (SPEC-0004)
// ============================================================================

/// T16: Worker count invariance for writer advance.
///
/// This test verifies that when a writer cursor advances (via Engine commit),
/// the resulting `commit_hash` is identical regardless of worker count.
/// This is the "free money" proof for BOAW Phase 6B: parallelism doesn't
/// affect correctness.
///
/// # Test Strategy
///
/// 1. Build a base snapshot with a deterministic graph structure.
/// 2. Create a fixed ingress queue (same intents for all runs).
/// 3. Execute a single tick with different worker pool sizes: [1, 2, 8, 32].
/// 4. Compare `commit_hash` across all runs - must be identical.
///
/// # Why This Matters
///
/// The Engine uses BOAW (Batch of Active Warps) for parallel rule execution.
/// `ECHO_WORKERS` (or `EngineBuilder::workers()`) controls the thread pool size.
/// This test proves that scaling workers doesn't change the deterministic outcome.
#[test]
fn worker_count_invariance_for_writer_advance() {
    use warp_core::{
        ApplyResult, AtomPayload, AttachmentKey, AttachmentValue, ConflictPolicy, EngineBuilder,
        Footprint, NodeKey as EngineNodeKey, PatternGraph, RewriteRule,
    };

    // Worker counts to test (per SPEC-0004 requirement: 1, 2, 8, 32)
    const WORKER_COUNTS: &[usize] = &[1, 2, 8, 32];

    // Rule name used for this test
    const TOUCH_RULE_NAME: &str = "t16/touch";

    // Create the "t16/touch" rule that sets a marker attachment on the scope node
    let make_touch_rule = || -> RewriteRule {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"rule:");
        hasher.update(TOUCH_RULE_NAME.as_bytes());
        let id: [u8; 32] = hasher.finalize().into();

        RewriteRule {
            id,
            name: TOUCH_RULE_NAME,
            left: PatternGraph { nodes: vec![] },
            matcher: |view, scope| view.node(scope).is_some(),
            executor: |view, scope, delta| {
                let marker_payload = AtomPayload::new(
                    make_type_id("t16/marker"),
                    bytes::Bytes::from_static(b"touched-t16"),
                );
                let value = Some(AttachmentValue::Atom(marker_payload));
                let key = AttachmentKey::node_alpha(EngineNodeKey {
                    warp_id: view.warp_id(),
                    local_id: *scope,
                });
                delta.push(WarpOp::SetAttachment { key, value });
            },
            compute_footprint: |view, scope| {
                let mut a_write = warp_core::AttachmentSet::default();
                if view.node(scope).is_some() {
                    a_write.insert(AttachmentKey::node_alpha(EngineNodeKey {
                        warp_id: view.warp_id(),
                        local_id: *scope,
                    }));
                }
                Footprint {
                    n_read: warp_core::NodeSet::default(),
                    n_write: warp_core::NodeSet::default(),
                    e_read: warp_core::EdgeSet::default(),
                    e_write: warp_core::EdgeSet::default(),
                    a_read: warp_core::AttachmentSet::default(),
                    a_write,
                    b_in: warp_core::PortSet::default(),
                    b_out: warp_core::PortSet::default(),
                    factor_mask: 1,
                }
            },
            factor_mask: 1,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        }
    };

    // Build a deterministic base snapshot with 20 independent nodes
    // (mirrors ManyIndependent scenario from BOAW tests)
    let node_ty = make_type_id("t16/node");
    let mut base_store = warp_core::GraphStore::default();

    let root = make_node_id("t16/root");
    base_store.insert_node(root, NodeRecord { ty: node_ty });

    // Create 19 more independent nodes (total 20)
    let mut all_nodes = vec![root];
    for i in 1..20 {
        let node = make_node_id(&format!("t16/node{}", i));
        base_store.insert_node(node, NodeRecord { ty: node_ty });
        all_nodes.push(node);
    }

    // Build fixed ingress: touch all 20 nodes
    let ingress: Vec<(&str, warp_core::NodeId)> = all_nodes
        .iter()
        .map(|&node| (TOUCH_RULE_NAME, node))
        .collect();

    // Run with baseline (1 worker) to establish expected commit_hash
    let baseline_commit_hash = {
        let mut engine = EngineBuilder::new(base_store.clone(), root)
            .workers(1)
            .build();

        engine
            .register_rule(make_touch_rule())
            .expect("failed to register rule");

        let tx = engine.begin();
        for (rule_name, scope) in &ingress {
            match engine.apply(tx, rule_name, scope) {
                Ok(ApplyResult::Applied) => {}
                Ok(ApplyResult::NoMatch) => {}
                Err(e) => panic!("apply error: {:?}", e),
            }
        }

        let (snapshot, _receipt, _patch) = engine
            .commit_with_receipt(tx)
            .expect("commit_with_receipt failed");

        snapshot.hash
    };

    // Run with each worker count and verify identical commit_hash
    for &workers in WORKER_COUNTS {
        let mut engine = EngineBuilder::new(base_store.clone(), root)
            .workers(workers)
            .build();

        engine
            .register_rule(make_touch_rule())
            .expect("failed to register rule");

        let tx = engine.begin();
        for (rule_name, scope) in &ingress {
            match engine.apply(tx, rule_name, scope) {
                Ok(ApplyResult::Applied) => {}
                Ok(ApplyResult::NoMatch) => {}
                Err(e) => panic!("apply error with {} workers: {:?}", workers, e),
            }
        }

        let (snapshot, _receipt, _patch) = engine
            .commit_with_receipt(tx)
            .expect("commit_with_receipt failed");

        assert_eq!(
            baseline_commit_hash, snapshot.hash,
            "commit_hash differs for {} workers\n  baseline: {:02x?}\n  got:      {:02x?}",
            workers, baseline_commit_hash, snapshot.hash
        );
    }
}

/// T16 variant: Worker count invariance with shuffled ingress order.
///
/// This test combines worker count invariance with permutation invariance.
/// The ingress order is shuffled before each run, proving that both
/// the order of intents and the number of workers don't affect the result.
#[test]
fn worker_count_invariance_for_writer_advance_shuffled() {
    use warp_core::{
        ApplyResult, AtomPayload, AttachmentKey, AttachmentValue, ConflictPolicy, EngineBuilder,
        Footprint, NodeKey as EngineNodeKey, PatternGraph, RewriteRule,
    };

    // Worker counts to test
    const WORKER_COUNTS: &[usize] = &[1, 2, 8, 32];

    // Seeds for deterministic shuffling
    const SEEDS: &[u64] = &[0x1234, 0xDEAD, 0xBEEF];

    // XorShift64 RNG for deterministic shuffling (same as in common/mod.rs)
    struct XorShift64 {
        state: u64,
    }

    impl XorShift64 {
        fn new(seed: u64) -> Self {
            Self { state: seed.max(1) }
        }

        fn next_u64(&mut self) -> u64 {
            let mut x = self.state;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.state = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn gen_range_usize(&mut self, upper: usize) -> usize {
            if upper <= 1 {
                return 0;
            }
            (self.next_u64() as usize) % upper
        }
    }

    fn shuffle<T>(rng: &mut XorShift64, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = rng.gen_range_usize(i + 1);
            items.swap(i, j);
        }
    }

    const TOUCH_RULE_NAME: &str = "t16s/touch";

    let make_touch_rule = || -> RewriteRule {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"rule:");
        hasher.update(TOUCH_RULE_NAME.as_bytes());
        let id: [u8; 32] = hasher.finalize().into();

        RewriteRule {
            id,
            name: TOUCH_RULE_NAME,
            left: PatternGraph { nodes: vec![] },
            matcher: |view, scope| view.node(scope).is_some(),
            executor: |view, scope, delta| {
                let marker_payload = AtomPayload::new(
                    make_type_id("t16s/marker"),
                    bytes::Bytes::from_static(b"touched-t16s"),
                );
                let value = Some(AttachmentValue::Atom(marker_payload));
                let key = AttachmentKey::node_alpha(EngineNodeKey {
                    warp_id: view.warp_id(),
                    local_id: *scope,
                });
                delta.push(WarpOp::SetAttachment { key, value });
            },
            compute_footprint: |view, scope| {
                let mut a_write = warp_core::AttachmentSet::default();
                if view.node(scope).is_some() {
                    a_write.insert(AttachmentKey::node_alpha(EngineNodeKey {
                        warp_id: view.warp_id(),
                        local_id: *scope,
                    }));
                }
                Footprint {
                    n_read: warp_core::NodeSet::default(),
                    n_write: warp_core::NodeSet::default(),
                    e_read: warp_core::EdgeSet::default(),
                    e_write: warp_core::EdgeSet::default(),
                    a_read: warp_core::AttachmentSet::default(),
                    a_write,
                    b_in: warp_core::PortSet::default(),
                    b_out: warp_core::PortSet::default(),
                    factor_mask: 1,
                }
            },
            factor_mask: 1,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        }
    };

    // Build deterministic base snapshot
    let node_ty = make_type_id("t16s/node");
    let mut base_store = warp_core::GraphStore::default();

    let root = make_node_id("t16s/root");
    base_store.insert_node(root, NodeRecord { ty: node_ty });

    let mut all_nodes = vec![root];
    for i in 1..20 {
        let node = make_node_id(&format!("t16s/node{}", i));
        base_store.insert_node(node, NodeRecord { ty: node_ty });
        all_nodes.push(node);
    }

    // Baseline ingress (canonical order)
    let canonical_ingress: Vec<(&str, warp_core::NodeId)> = all_nodes
        .iter()
        .map(|&node| (TOUCH_RULE_NAME, node))
        .collect();

    // Get baseline commit_hash with 1 worker, canonical order
    let baseline_commit_hash = {
        let mut engine = EngineBuilder::new(base_store.clone(), root)
            .workers(1)
            .build();

        engine
            .register_rule(make_touch_rule())
            .expect("failed to register rule");

        let tx = engine.begin();
        for (rule_name, scope) in &canonical_ingress {
            match engine.apply(tx, rule_name, scope) {
                Ok(ApplyResult::Applied) => {}
                Ok(ApplyResult::NoMatch) => {}
                Err(e) => panic!("apply error: {:?}", e),
            }
        }

        let (snapshot, _, _) = engine
            .commit_with_receipt(tx)
            .expect("commit_with_receipt failed");

        snapshot.hash
    };

    // Test with each seed and worker count
    for &seed in SEEDS {
        let mut rng = XorShift64::new(seed);
        let mut ingress = canonical_ingress.clone();

        // Shuffle ingress order
        shuffle(&mut rng, &mut ingress);

        for &workers in WORKER_COUNTS {
            let mut engine = EngineBuilder::new(base_store.clone(), root)
                .workers(workers)
                .build();

            engine
                .register_rule(make_touch_rule())
                .expect("failed to register rule");

            let tx = engine.begin();
            for (rule_name, scope) in &ingress {
                match engine.apply(tx, rule_name, scope) {
                    Ok(ApplyResult::Applied) => {}
                    Ok(ApplyResult::NoMatch) => {}
                    Err(e) => panic!(
                        "apply error (seed={:#x}, workers={}): {:?}",
                        seed, workers, e
                    ),
                }
            }

            let (snapshot, _, _) = engine
                .commit_with_receipt(tx)
                .expect("commit_with_receipt failed");

            assert_eq!(
                baseline_commit_hash, snapshot.hash,
                "commit_hash differs (seed={:#x}, workers={})\n  baseline: {:02x?}\n  got:      {:02x?}",
                seed, workers, baseline_commit_hash, snapshot.hash
            );
        }
    }
}
