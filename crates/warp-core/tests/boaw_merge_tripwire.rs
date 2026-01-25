// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Merge tripwire tests for BOAW Phase 6.
#![cfg(feature = "delta_validate")]
//!
//! These tests verify that footprint model violations are caught at merge time,
//! proving the safety net exists.
//!
//! # Feature Requirements
//! ```sh
//! cargo test --package warp-core --test boaw_merge_tripwire --features delta_validate
//! ```

use warp_core::{
    make_node_id, make_type_id, make_warp_id, merge_deltas_ok, AtomPayload, AttachmentKey,
    AttachmentValue, MergeError, NodeKey, OpOrigin, TickDelta, WarpOp, WarpOpKey,
};

// =============================================================================
// Helper functions
// =============================================================================

/// Creates a test origin with specified intent_id and rule_id.
fn make_origin(intent_id: u64, rule_id: u32) -> OpOrigin {
    OpOrigin {
        intent_id,
        rule_id,
        match_ix: 0,
        op_ix: 0,
    }
}

/// Creates a SetAttachment op with an Atom value.
fn make_set_attachment(key: AttachmentKey, value_bytes: &[u8]) -> WarpOp {
    WarpOp::SetAttachment {
        key,
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("test:atom"),
            value_bytes.to_vec().into(),
        ))),
    }
}

// =============================================================================
// T6.1: Merge Tripwire Tests
// =============================================================================

/// T6.1 - Two ops claim disjoint footprints but write same key.
///
/// This is the canonical footprint model violation: two writers produce
/// different values for the same logical key, indicating the footprint
/// model failed to prevent concurrent writes to the same target.
#[test]
fn deliberately_incorrect_footprint_explodes_at_merge() {
    let warp_id = make_warp_id("tripwire-warp");
    let node_id = make_node_id("contested-node");
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id,
    });

    // Two different workers with different origins
    let origin1 = make_origin(1, 100);
    let origin2 = make_origin(2, 200);

    // Create deltas with different values for the SAME key
    let mut delta1 = TickDelta::new();
    let mut delta2 = TickDelta::new();

    let op1 = make_set_attachment(key, b"value-from-worker-1");
    let op2 = make_set_attachment(key, b"value-from-worker-2");

    delta1.push_with_origin(op1, origin1);
    delta2.push_with_origin(op2, origin2);

    // Merge should detect the conflict and return an error
    let result = merge_deltas_ok(vec![delta1, delta2]);

    assert!(
        result.is_err(),
        "Merge must fail when two writers produce different values for the same key"
    );
}

/// Verify MergeConflict.writers contains both OpOrigins.
///
/// When a conflict is detected, the error must identify all writers
/// that contributed conflicting ops, enabling debugging and auditing.
#[test]
fn merge_conflict_contains_both_writers() {
    let warp_id = make_warp_id("conflict-writers-warp");
    let node_id = make_node_id("conflict-writers-node");
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id,
    });

    let origin1 = make_origin(42, 1);
    let origin2 = make_origin(99, 2);

    let mut delta1 = TickDelta::new();
    let mut delta2 = TickDelta::new();

    delta1.push_with_origin(make_set_attachment(key, b"alpha"), origin1);
    delta2.push_with_origin(make_set_attachment(key, b"beta"), origin2);

    let result = merge_deltas_ok(vec![delta1, delta2]);

    let err = result.expect_err("Merge should fail with conflict");

    // Extract the conflict from the error enum
    let MergeError::Conflict(conflict) = err else {
        panic!("Expected MergeError::Conflict, got: {:?}", err);
    };

    // Both writers must be reported
    assert_eq!(
        conflict.writers.len(),
        2,
        "MergeConflict must report exactly 2 writers"
    );
    assert!(
        conflict.writers.contains(&origin1),
        "MergeConflict.writers must contain origin1"
    );
    assert!(
        conflict.writers.contains(&origin2),
        "MergeConflict.writers must contain origin2"
    );
}

/// Verify MergeConflict.key is populated for the conflicting key.
///
/// The conflict error must report which key was contested, enabling
/// precise identification of the footprint model failure.
#[test]
fn merge_conflict_reports_correct_key() {
    let warp_id = make_warp_id("conflict-key-warp");
    let node_id = make_node_id("conflict-key-node");
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id,
    });

    let origin1 = make_origin(1, 1);
    let origin2 = make_origin(2, 2);

    let mut delta1 = TickDelta::new();
    let mut delta2 = TickDelta::new();

    let op1 = make_set_attachment(key, b"first");
    let op2 = make_set_attachment(key, b"second");

    // Compute the expected key before ops are moved into deltas.
    // Both ops target the same attachment key, so they have the same sort_key.
    let expected_key: WarpOpKey = op1.sort_key();

    delta1.push_with_origin(op1, origin1);
    delta2.push_with_origin(op2, origin2);

    let result = merge_deltas_ok(vec![delta1, delta2]);

    let err = result.expect_err("Merge should fail with conflict");

    // Extract the conflict from the error enum
    let MergeError::Conflict(conflict) = err else {
        panic!("Expected MergeError::Conflict, got: {:?}", err);
    };

    assert_eq!(
        conflict.key, expected_key,
        "MergeConflict.key should match the sort_key of the conflicting ops"
    );
}

/// Same key + same value + same origin = dedupe, not conflict.
///
/// This is the idempotent case: if multiple workers emit identical ops
/// (same key, same value), they should be deduplicated rather than
/// treated as a conflict. This supports replay safety and allows
/// redundant work to be coalesced.
#[test]
fn no_false_merge_conflicts_for_identical_ops() {
    let warp_id = make_warp_id("dedupe-warp");
    let node_id = make_node_id("dedupe-node");
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id,
    });

    // Same origin for both (simulating replay or redundant emission)
    let origin = make_origin(1, 1);

    let mut delta1 = TickDelta::new();
    let mut delta2 = TickDelta::new();

    // Identical ops: same key, same value
    let value = b"identical-value";
    let op1 = make_set_attachment(key, value);
    let op2 = make_set_attachment(key, value);

    delta1.push_with_origin(op1, origin);
    delta2.push_with_origin(op2, origin);

    // Merge should succeed and dedupe the identical ops
    let result = merge_deltas_ok(vec![delta1, delta2]);

    assert!(
        result.is_ok(),
        "Identical ops with same origin should be deduped, not conflicted"
    );

    let merged_ops = result.expect("merge should succeed");
    assert_eq!(
        merged_ops.len(),
        1,
        "Identical ops should be deduplicated to a single op"
    );
}

// =============================================================================
// Additional tripwire edge cases
// =============================================================================

/// Verify that different origins with identical values still dedupe.
///
/// This tests that value equality, not origin equality, determines deduplication.
#[test]
fn identical_values_different_origins_dedupe() {
    let warp_id = make_warp_id("value-dedupe-warp");
    let node_id = make_node_id("value-dedupe-node");
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id,
    });

    // Different origins (different workers)
    let origin1 = make_origin(1, 100);
    let origin2 = make_origin(2, 200);

    let mut delta1 = TickDelta::new();
    let mut delta2 = TickDelta::new();

    // Same value despite different origins
    let value = b"convergent-value";
    delta1.push_with_origin(make_set_attachment(key, value), origin1);
    delta2.push_with_origin(make_set_attachment(key, value), origin2);

    let result = merge_deltas_ok(vec![delta1, delta2]);

    assert!(
        result.is_ok(),
        "Identical values from different origins should be deduped, not conflicted"
    );

    let merged_ops = result.expect("merge should succeed");
    assert_eq!(
        merged_ops.len(),
        1,
        "Identical values should dedupe to single op"
    );
}

/// Three-way conflict: three writers all produce different values.
///
/// Verifies the conflict reports all three writers.
#[test]
fn three_way_conflict_reports_all_writers() {
    let warp_id = make_warp_id("three-way-warp");
    let node_id = make_node_id("three-way-node");
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id,
    });

    let origin1 = make_origin(1, 1);
    let origin2 = make_origin(2, 2);
    let origin3 = make_origin(3, 3);

    let mut delta1 = TickDelta::new();
    let mut delta2 = TickDelta::new();
    let mut delta3 = TickDelta::new();

    delta1.push_with_origin(make_set_attachment(key, b"value-1"), origin1);
    delta2.push_with_origin(make_set_attachment(key, b"value-2"), origin2);
    delta3.push_with_origin(make_set_attachment(key, b"value-3"), origin3);

    let result = merge_deltas_ok(vec![delta1, delta2, delta3]);

    let err = result.expect_err("Three-way conflict must fail");

    // Extract the conflict from the error enum
    let MergeError::Conflict(conflict) = err else {
        panic!("Expected MergeError::Conflict, got: {:?}", err);
    };

    assert_eq!(
        conflict.writers.len(),
        3,
        "MergeConflict must report all 3 writers"
    );
    assert!(conflict.writers.contains(&origin1));
    assert!(conflict.writers.contains(&origin2));
    assert!(conflict.writers.contains(&origin3));
}

/// Mixed scenario: one key conflicts, others merge cleanly.
///
/// Verifies that the merge correctly identifies the specific conflicting key
/// while other keys would have merged successfully.
#[test]
fn conflict_on_one_key_while_others_would_merge() {
    let warp_id = make_warp_id("mixed-merge-warp");

    // Key A: will conflict
    let node_a = make_node_id("node-a");
    let key_a = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_a,
    });

    // Key B: will merge cleanly (different key per delta)
    let node_b = make_node_id("node-b");
    let key_b = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_b,
    });

    let node_c = make_node_id("node-c");
    let key_c = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_c,
    });

    let origin1 = make_origin(1, 1);
    let origin2 = make_origin(2, 2);

    let mut delta1 = TickDelta::new();
    let mut delta2 = TickDelta::new();

    // Delta 1: writes to A and B
    delta1.push_with_origin(make_set_attachment(key_a, b"conflict-val-1"), origin1);
    delta1.push_with_origin(make_set_attachment(key_b, b"clean-val-b"), origin1);

    // Delta 2: writes to A (conflict!) and C (clean)
    delta2.push_with_origin(make_set_attachment(key_a, b"conflict-val-2"), origin2);
    delta2.push_with_origin(make_set_attachment(key_c, b"clean-val-c"), origin2);

    let result = merge_deltas_ok(vec![delta1, delta2]);

    // Should fail due to key_a conflict
    assert!(result.is_err(), "Should fail due to key_a conflict");

    let err = result.expect_err("Should have conflict error");

    // Extract the conflict from the error enum
    let MergeError::Conflict(conflict) = err else {
        panic!("Expected MergeError::Conflict, got: {:?}", err);
    };

    // Verify the conflict involves both origins that wrote to key_a
    assert_eq!(
        conflict.writers.len(),
        2,
        "Conflict should report exactly 2 writers for key_a"
    );
    assert!(conflict.writers.contains(&origin1));
    assert!(conflict.writers.contains(&origin2));
}

/// Empty deltas merge to empty result.
#[test]
fn empty_deltas_merge_successfully() {
    let delta1 = TickDelta::new();
    let delta2 = TickDelta::new();

    let result = merge_deltas_ok(vec![delta1, delta2]);

    assert!(result.is_ok(), "Empty deltas should merge successfully");
    let merged = result.expect("merge should succeed");
    assert!(merged.is_empty(), "Merged result should be empty");
}

/// Single delta passes through unchanged.
#[test]
fn single_delta_passes_through() {
    let warp_id = make_warp_id("single-warp");
    let node_id = make_node_id("single-node");
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id,
    });

    let origin = make_origin(1, 1);
    let mut delta = TickDelta::new();
    delta.push_with_origin(make_set_attachment(key, b"solo-value"), origin);

    let result = merge_deltas_ok(vec![delta]);

    assert!(result.is_ok(), "Single delta should merge successfully");
    let merged = result.expect("merge should succeed");
    assert_eq!(merged.len(), 1, "Single op should pass through");
}

/// Multiple non-overlapping keys merge without conflict.
#[test]
fn disjoint_keys_merge_cleanly() {
    let warp_id = make_warp_id("disjoint-warp");

    let node_a = make_node_id("node-a");
    let node_b = make_node_id("node-b");
    let node_c = make_node_id("node-c");

    let key_a = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_a,
    });
    let key_b = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_b,
    });
    let key_c = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_c,
    });

    let origin1 = make_origin(1, 1);
    let origin2 = make_origin(2, 2);
    let origin3 = make_origin(3, 3);

    let mut delta1 = TickDelta::new();
    let mut delta2 = TickDelta::new();
    let mut delta3 = TickDelta::new();

    // Each delta writes to a different key
    delta1.push_with_origin(make_set_attachment(key_a, b"value-a"), origin1);
    delta2.push_with_origin(make_set_attachment(key_b, b"value-b"), origin2);
    delta3.push_with_origin(make_set_attachment(key_c, b"value-c"), origin3);

    let result = merge_deltas_ok(vec![delta1, delta2, delta3]);

    assert!(
        result.is_ok(),
        "Disjoint keys should merge without conflict"
    );
    let merged = result.expect("merge should succeed");
    assert_eq!(merged.len(), 3, "All three ops should be in merged result");
}

/// Verify that merge preserves canonical ordering.
#[test]
fn merged_ops_are_canonically_ordered() {
    let warp_id = make_warp_id("ordered-warp");

    // Create nodes with predictable ordering (lexicographic on hash)
    let node_a = make_node_id("aaa");
    let node_b = make_node_id("bbb");
    let node_c = make_node_id("ccc");

    let key_a = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_a,
    });
    let key_b = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_b,
    });
    let key_c = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_c,
    });

    let origin = make_origin(1, 1);

    // Push in non-canonical order: C, A, B
    let mut delta = TickDelta::new();
    delta.push_with_origin(make_set_attachment(key_c, b"c"), origin);
    delta.push_with_origin(make_set_attachment(key_a, b"a"), origin);
    delta.push_with_origin(make_set_attachment(key_b, b"b"), origin);

    let result = merge_deltas_ok(vec![delta]);

    assert!(result.is_ok());
    let merged = result.expect("merge should succeed");
    assert_eq!(merged.len(), 3);

    // Verify ops are in canonical order using WarpOp::sort_key() directly
    // (less coupling to AttachmentKey ordering implementation)
    let keys: Vec<_> = merged.iter().map(|op| op.sort_key()).collect();
    for i in 1..keys.len() {
        assert!(
            keys[i - 1] <= keys[i],
            "Merged ops should be in canonical sorted order"
        );
    }
}
