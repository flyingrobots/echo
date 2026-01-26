// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WarpOpKey collision safety tests for BOAW Phase 6.
//!
//! These tests verify that WarpOpKey correctly distinguishes ops by warp,
//! ensuring global merge doesn't coalesce cross-warp operations.
//!
//! # Feature Requirements
//! ```sh
//! cargo test --package warp-core --test boaw_merge_warpopkey --features delta_validate
//! ```

#![cfg(feature = "delta_validate")]

use warp_core::{
    make_node_id, make_type_id, make_warp_id, merge_deltas_ok, AtomPayload, AttachmentKey,
    AttachmentValue, MergeError, NodeKey, NodeRecord, OpOrigin, TickDelta, WarpOp,
};

// =============================================================================
// T1.1: WarpOpKey Collision Safety
// =============================================================================

/// T1.1.1 - Two deltas with same local target but different warp survive merge.
///
/// This test verifies that operations targeting the same local node ID but in
/// different warp instances are NOT coalesced during merge. The WarpOpKey must
/// include the warp_id to distinguish cross-warp operations.
#[test]
fn warp_op_key_distinguishes_by_warp_and_survives_merge() {
    let warp_a = make_warp_id("warp-a");
    let warp_b = make_warp_id("warp-b");
    let local_node = make_node_id("shared-local");

    // Create two SetAttachment ops targeting the same local_id but different warps
    let key_a = AttachmentKey::node_alpha(NodeKey {
        warp_id: warp_a,
        local_id: local_node,
    });
    let key_b = AttachmentKey::node_alpha(NodeKey {
        warp_id: warp_b,
        local_id: local_node,
    });

    // Verify the keys are distinct (this is the core invariant)
    assert_ne!(
        key_a, key_b,
        "AttachmentKeys with different warps must differ"
    );

    let value_a = Some(AttachmentValue::Atom(AtomPayload::new(
        make_type_id("test"),
        bytes::Bytes::from_static(b"warp-a-data"),
    )));
    let value_b = Some(AttachmentValue::Atom(AtomPayload::new(
        make_type_id("test"),
        bytes::Bytes::from_static(b"warp-b-data"),
    )));

    let op_a = WarpOp::SetAttachment {
        key: key_a,
        value: value_a,
    };
    let op_b = WarpOp::SetAttachment {
        key: key_b,
        value: value_b,
    };

    // Create separate deltas (simulating parallel execution in different warps)
    let mut delta_a = TickDelta::new();
    delta_a.emit_with_origin(
        op_a.clone(),
        OpOrigin {
            intent_id: 1,
            rule_id: 1,
            match_ix: 0,
            op_ix: 0,
        },
    );

    let mut delta_b = TickDelta::new();
    delta_b.emit_with_origin(
        op_b.clone(),
        OpOrigin {
            intent_id: 2,
            rule_id: 1,
            match_ix: 0,
            op_ix: 0,
        },
    );

    // Merge should succeed without conflict - ops target different logical keys
    let merged = merge_deltas_ok(vec![delta_a, delta_b]);
    let ops = merged.expect("merge should succeed: ops target different warps");

    // Both ops must survive
    assert_eq!(ops.len(), 2, "Both cross-warp ops must survive merge");

    // Verify both ops are present
    let has_op_a = ops.iter().any(|op| op == &op_a);
    let has_op_b = ops.iter().any(|op| op == &op_b);
    assert!(has_op_a, "Op for warp-a must be present");
    assert!(has_op_b, "Op for warp-b must be present");
}

/// T1.1.2 - Verify deterministic ordering in BTreeMap.
///
/// WarpOpKey must have a total, stable ordering so that merge results are
/// deterministic regardless of insertion order. We verify this by checking
/// that merge produces identical results regardless of delta input order.
#[test]
fn warp_op_key_ordering_stability_btreemap() {
    let warp_a = make_warp_id("warp-a");
    let warp_b = make_warp_id("warp-b");
    let warp_c = make_warp_id("warp-c");
    let local_node = make_node_id("shared-local");

    let ops = vec![
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_a,
                local_id: local_node,
            },
            record: NodeRecord {
                ty: make_type_id("test"),
            },
        },
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_b,
                local_id: local_node,
            },
            record: NodeRecord {
                ty: make_type_id("test"),
            },
        },
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_c,
                local_id: local_node,
            },
            record: NodeRecord {
                ty: make_type_id("test"),
            },
        },
    ];

    // Create deltas from ops
    let make_delta = |idx: usize, op: &WarpOp| {
        let mut delta = TickDelta::new();
        delta.emit_with_origin(
            op.clone(),
            OpOrigin {
                intent_id: idx as u64,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        );
        delta
    };

    // Merge in forward order
    let deltas_forward: Vec<_> = ops
        .iter()
        .enumerate()
        .map(|(i, op)| make_delta(i, op))
        .collect();
    let result_forward = merge_deltas_ok(deltas_forward).expect("merge should succeed");

    // Merge in reverse order
    let deltas_reverse: Vec<_> = ops
        .iter()
        .enumerate()
        .rev()
        .map(|(i, op)| make_delta(i, op))
        .collect();
    let result_reverse = merge_deltas_ok(deltas_reverse).expect("merge should succeed");

    // Must have all 3 ops (no collisions)
    assert_eq!(result_forward.len(), 3, "All 3 ops must survive merge");
    assert_eq!(result_reverse.len(), 3, "All 3 ops must survive merge");

    // Results must be identical regardless of input order
    assert_eq!(
        result_forward, result_reverse,
        "BTreeMap iteration order must be deterministic"
    );

    // Verify all original ops are present
    for op in &ops {
        assert!(
            result_forward.contains(op),
            "All original ops must be present"
        );
    }
}

/// T1.1.3 - Same key + same value = dedupe; same key + diff value = conflict.
///
/// When two ops share the same WarpOpKey:
/// - If they are identical, they should dedupe to a single op
/// - If they differ, merge should report a conflict (footprint model violation)
#[test]
fn warp_op_key_same_warp_same_target_merges_correctly() {
    let warp_id = make_warp_id("single-warp");
    let local_node = make_node_id("target-node");

    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: local_node,
    });

    // Case 1: Identical ops should dedupe
    {
        let value = Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("test"),
            bytes::Bytes::from_static(b"same-data"),
        )));

        let op = WarpOp::SetAttachment {
            key,
            value: value.clone(),
        };

        let mut delta1 = TickDelta::new();
        delta1.emit_with_origin(
            op.clone(),
            OpOrigin {
                intent_id: 1,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        );

        let mut delta2 = TickDelta::new();
        delta2.emit_with_origin(
            op.clone(),
            OpOrigin {
                intent_id: 2,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        );

        let merged = merge_deltas_ok(vec![delta1, delta2]);
        let ops = merged.expect("identical ops should dedupe without conflict");
        assert_eq!(ops.len(), 1, "Identical ops must dedupe to 1");
        assert_eq!(ops[0], op);
    }

    // Case 2: Different values for same key should conflict
    {
        let value_1 = Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("test"),
            bytes::Bytes::from_static(b"data-from-delta-1"),
        )));
        let value_2 = Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("test"),
            bytes::Bytes::from_static(b"data-from-delta-2"),
        )));

        let op1 = WarpOp::SetAttachment {
            key,
            value: value_1,
        };
        let op2 = WarpOp::SetAttachment {
            key,
            value: value_2,
        };

        let mut delta1 = TickDelta::new();
        delta1.emit_with_origin(
            op1,
            OpOrigin {
                intent_id: 1,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        );

        let mut delta2 = TickDelta::new();
        delta2.emit_with_origin(
            op2,
            OpOrigin {
                intent_id: 2,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        );

        let merged = merge_deltas_ok(vec![delta1, delta2]);
        assert!(
            merged.is_err(),
            "Different values for same key must produce MergeConflict"
        );

        let err = merged.unwrap_err();
        let MergeError::Conflict(conflict) = err else {
            panic!("Expected MergeError::Conflict, got: {:?}", err);
        };
        assert_eq!(
            conflict.writers.len(),
            2,
            "Conflict must report both writers"
        );
    }
}

/// T1.1.4 - 3 warps with same local target: all 3 ops survive merge.
///
/// Validates that cross-warp independence holds for arbitrary numbers of warps.
#[test]
fn merge_preserves_all_warp_distinct_ops() {
    let warp_alpha = make_warp_id("warp-alpha");
    let warp_beta = make_warp_id("warp-beta");
    let warp_gamma = make_warp_id("warp-gamma");
    let shared_local = make_node_id("shared-local-target");

    // Create 3 UpsertNode ops targeting the same local_id in different warps
    let ops: Vec<WarpOp> = vec![
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_alpha,
                local_id: shared_local,
            },
            record: NodeRecord {
                ty: make_type_id("alpha-type"),
            },
        },
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_beta,
                local_id: shared_local,
            },
            record: NodeRecord {
                ty: make_type_id("beta-type"),
            },
        },
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_gamma,
                local_id: shared_local,
            },
            record: NodeRecord {
                ty: make_type_id("gamma-type"),
            },
        },
    ];

    // Create one delta per warp (simulating parallel execution)
    let mut deltas = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        let mut delta = TickDelta::new();
        delta.emit_with_origin(
            op.clone(),
            OpOrigin {
                intent_id: i as u64,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        );
        deltas.push(delta);
    }

    // Merge should succeed
    let merged = merge_deltas_ok(deltas);
    let result_ops = merged.expect("merge should succeed: all ops target different warps");

    // All 3 ops must survive
    assert_eq!(
        result_ops.len(),
        3,
        "All 3 warp-distinct ops must survive merge"
    );

    // Verify each original op is present
    for expected_op in &ops {
        assert!(
            result_ops.contains(expected_op),
            "Merged result must contain all original ops"
        );
    }
}

// =============================================================================
// Additional robustness tests
// =============================================================================

/// Verify that different op types targeting the same node do not conflict.
///
/// UpsertNode and DeleteNode have different `kind` values in WarpOpKey,
/// so they should not collide even when targeting the same node.
#[test]
fn different_op_types_do_not_conflict() {
    let warp_id = make_warp_id("test-warp");
    let node_id = make_node_id("test-node");
    let node_key = NodeKey {
        warp_id,
        local_id: node_id,
    };

    let upsert_op = WarpOp::UpsertNode {
        node: node_key,
        record: NodeRecord {
            ty: make_type_id("test"),
        },
    };
    let delete_op = WarpOp::DeleteNode { node: node_key };

    // Create deltas with different op types targeting the same node
    let mut delta1 = TickDelta::new();
    delta1.emit_with_origin(
        delete_op.clone(),
        OpOrigin {
            intent_id: 1,
            rule_id: 1,
            match_ix: 0,
            op_ix: 0,
        },
    );

    let mut delta2 = TickDelta::new();
    delta2.emit_with_origin(
        upsert_op.clone(),
        OpOrigin {
            intent_id: 2,
            rule_id: 1,
            match_ix: 0,
            op_ix: 0,
        },
    );

    // Merge should succeed: different op types have different WarpOpKeys
    let merged = merge_deltas_ok(vec![delta1, delta2]);
    let result_ops = merged.expect("different op types should not conflict");

    assert_eq!(result_ops.len(), 2, "Both ops must survive");
    assert!(
        result_ops.contains(&delete_op),
        "DeleteNode must be present"
    );
    assert!(
        result_ops.contains(&upsert_op),
        "UpsertNode must be present"
    );

    // Verify ordering: DeleteNode (kind=5) should come before UpsertNode (kind=6)
    let delete_idx = result_ops.iter().position(|op| op == &delete_op).unwrap();
    let upsert_idx = result_ops.iter().position(|op| op == &upsert_op).unwrap();
    assert!(
        delete_idx < upsert_idx,
        "DeleteNode must sort before UpsertNode in canonical order"
    );
}

/// Verify that attachment ops on nodes vs edges are distinguished.
#[test]
fn attachment_ops_distinguish_node_vs_edge_owners() {
    let warp_id = make_warp_id("test-warp");
    let node_id = make_node_id("test-node");
    let edge_id = warp_core::make_edge_id("test-edge");

    let node_key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id,
    });
    let edge_key = AttachmentKey::edge_beta(warp_core::EdgeKey {
        warp_id,
        local_id: warp_core::EdgeId(edge_id.0),
    });

    let node_op = WarpOp::SetAttachment {
        key: node_key,
        value: None,
    };
    let edge_op = WarpOp::SetAttachment {
        key: edge_key,
        value: None,
    };

    // Create deltas
    let mut delta1 = TickDelta::new();
    delta1.emit_with_origin(
        node_op.clone(),
        OpOrigin {
            intent_id: 1,
            rule_id: 1,
            match_ix: 0,
            op_ix: 0,
        },
    );

    let mut delta2 = TickDelta::new();
    delta2.emit_with_origin(
        edge_op.clone(),
        OpOrigin {
            intent_id: 2,
            rule_id: 1,
            match_ix: 0,
            op_ix: 0,
        },
    );

    // Merge should succeed: node and edge attachments have different WarpOpKeys
    let merged = merge_deltas_ok(vec![delta1, delta2]);
    let result_ops = merged.expect("node vs edge attachment ops should not conflict");

    assert_eq!(result_ops.len(), 2, "Both ops must survive");
    assert!(
        result_ops.contains(&node_op),
        "Node attachment op must be present"
    );
    assert!(
        result_ops.contains(&edge_op),
        "Edge attachment op must be present"
    );
}

/// Verify merge determinism: same inputs in different order yield same result.
#[test]
fn merge_is_deterministic_regardless_of_delta_order() {
    let warp_a = make_warp_id("warp-a");
    let warp_b = make_warp_id("warp-b");
    let warp_c = make_warp_id("warp-c");
    let local_node = make_node_id("shared-local");

    let ops: Vec<WarpOp> = vec![
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_a,
                local_id: local_node,
            },
            record: NodeRecord {
                ty: make_type_id("type-a"),
            },
        },
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_b,
                local_id: local_node,
            },
            record: NodeRecord {
                ty: make_type_id("type-b"),
            },
        },
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id: warp_c,
                local_id: local_node,
            },
            record: NodeRecord {
                ty: make_type_id("type-c"),
            },
        },
    ];

    // Create deltas in forward order
    let make_delta = |idx: usize, op: &WarpOp| {
        let mut delta = TickDelta::new();
        delta.emit_with_origin(
            op.clone(),
            OpOrigin {
                intent_id: idx as u64,
                rule_id: 1,
                match_ix: 0,
                op_ix: 0,
            },
        );
        delta
    };

    // Merge in forward order
    let deltas_forward: Vec<_> = ops
        .iter()
        .enumerate()
        .map(|(i, op)| make_delta(i, op))
        .collect();
    let result_forward = merge_deltas_ok(deltas_forward).expect("merge should succeed");

    // Merge in reverse order
    let deltas_reverse: Vec<_> = ops
        .iter()
        .enumerate()
        .rev()
        .map(|(i, op)| make_delta(i, op))
        .collect();
    let result_reverse = merge_deltas_ok(deltas_reverse).expect("merge should succeed");

    // Results must be identical
    assert_eq!(
        result_forward, result_reverse,
        "Merge result must be deterministic regardless of delta order"
    );
}

/// Verify that many warps with the same local target all survive merge.
///
/// Stress test: 10 warps targeting the same local node.
#[test]
fn many_warps_same_local_target_all_survive() {
    let local_node = make_node_id("shared-local");
    let num_warps = 10;

    let ops: Vec<WarpOp> = (0..num_warps)
        .map(|i| {
            let warp_id = make_warp_id(&format!("warp-{i}"));
            WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: local_node,
                },
                record: NodeRecord {
                    ty: make_type_id(&format!("type-{i}")),
                },
            }
        })
        .collect();

    // Create deltas
    let deltas: Vec<_> = ops
        .iter()
        .enumerate()
        .map(|(i, op)| {
            let mut delta = TickDelta::new();
            delta.emit_with_origin(
                op.clone(),
                OpOrigin {
                    intent_id: i as u64,
                    rule_id: 1,
                    match_ix: 0,
                    op_ix: 0,
                },
            );
            delta
        })
        .collect();

    // Merge should succeed
    let merged = merge_deltas_ok(deltas);
    let result_ops = merged.expect("all cross-warp ops should merge successfully");

    assert_eq!(
        result_ops.len(),
        num_warps,
        "All {} warp-distinct ops must survive merge",
        num_warps
    );

    // Verify all original ops are present
    for op in &ops {
        assert!(
            result_ops.contains(op),
            "Merged result must contain all original ops"
        );
    }
}

/// Verify conflict detection with mixed op types.
///
/// Two UpsertNode ops targeting the same NodeKey (same warp, same local)
/// with different records should conflict.
#[test]
fn conflict_detected_for_same_nodekey_different_records() {
    let warp_id = make_warp_id("test-warp");
    let node_id = make_node_id("test-node");
    let node_key = NodeKey {
        warp_id,
        local_id: node_id,
    };

    let op1 = WarpOp::UpsertNode {
        node: node_key,
        record: NodeRecord {
            ty: make_type_id("type-a"),
        },
    };
    let op2 = WarpOp::UpsertNode {
        node: node_key,
        record: NodeRecord {
            ty: make_type_id("type-b"),
        },
    };

    let mut delta1 = TickDelta::new();
    delta1.emit_with_origin(
        op1,
        OpOrigin {
            intent_id: 1,
            rule_id: 1,
            match_ix: 0,
            op_ix: 0,
        },
    );

    let mut delta2 = TickDelta::new();
    delta2.emit_with_origin(
        op2,
        OpOrigin {
            intent_id: 2,
            rule_id: 1,
            match_ix: 0,
            op_ix: 0,
        },
    );

    let merged = merge_deltas_ok(vec![delta1, delta2]);
    assert!(
        merged.is_err(),
        "UpsertNode ops with same NodeKey but different records must conflict"
    );

    let err = merged.unwrap_err();
    let MergeError::Conflict(conflict) = err else {
        panic!("Expected MergeError::Conflict, got: {:?}", err);
    };
    assert_eq!(conflict.writers.len(), 2, "Both writers must be reported");
}
