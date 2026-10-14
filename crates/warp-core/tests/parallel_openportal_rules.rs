// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! OpenPortal rule tests for parallel execution.
//!
//! These tests verify the "no same-tick new warp writes" rule - rewrites cannot
//! target a warp that was created in the same tick.
//!
//! # Feature Requirements
//! ```sh
//! cargo test --package warp-core --test parallel_openportal_rules --features delta_validate
//! ```

#![cfg(feature = "delta_validate")]

mod common;

use warp_core::{
    make_node_id, make_type_id, make_warp_id, merge_deltas_ok, AttachmentKey, MergeError, NodeId,
    NodeKey, NodeRecord, OpOrigin, PortalInit, TickDelta, WarpId, WarpOp,
};

// =============================================================================
// T7.1: No Same-Tick New Warp Writes
// =============================================================================

/// Creates an OpenPortal op for testing with PortalInit::Empty.
fn make_open_portal_op(
    parent_warp: WarpId,
    parent_node: NodeId,
    child_warp: WarpId,
    child_root: NodeId,
) -> WarpOp {
    let parent_key = AttachmentKey::node_alpha(NodeKey {
        warp_id: parent_warp,
        local_id: parent_node,
    });
    WarpOp::OpenPortal {
        key: parent_key,
        child_warp,
        child_root,
        init: PortalInit::Empty {
            root_record: NodeRecord {
                ty: make_type_id("test/child-root"),
            },
        },
    }
}

/// Creates an OpenPortal op for testing with PortalInit::RequireExisting.
fn make_open_portal_require_existing(
    parent_warp: WarpId,
    parent_node: NodeId,
    child_warp: WarpId,
    child_root: NodeId,
) -> WarpOp {
    let parent_key = AttachmentKey::node_alpha(NodeKey {
        warp_id: parent_warp,
        local_id: parent_node,
    });
    WarpOp::OpenPortal {
        key: parent_key,
        child_warp,
        child_root,
        init: PortalInit::RequireExisting,
    }
}

/// Creates a test origin with specified intent_id and rule_id.
fn make_origin(intent_id: u64, rule_id: u32) -> OpOrigin {
    OpOrigin {
        intent_id,
        rule_id,
        match_ix: 0,
        op_ix: 0,
    }
}

/// T7.1: OpenPortal(Empty) in same tick as write to child_warp should fail.
///
/// This test verifies merge-phase enforcement of the "no same-tick new warp writes"
/// rule. When an OpenPortal with init=Empty creates a new warp, no other ops in
/// the same tick can target that warp.
///
/// Setup:
/// - Delta 1: R1 emits OpenPortal { child_warp=W_child, init=PortalInit::Empty }
/// - Delta 2: R2 emits UpsertNode targeting W_child
///
/// Expected: merge_deltas_ok returns Err with WriteToNewWarp (or similar error)
#[test]
fn openportal_child_warp_not_executable_same_tick() {
    let parent_warp = make_warp_id("test/parent");
    let parent_node = make_node_id("test/parent-node");
    let child_warp = make_warp_id("test/child");
    let child_root = make_node_id("test/child-root");

    // R1's op: create the child warp via OpenPortal(Empty)
    let r1_op = make_open_portal_op(parent_warp, parent_node, child_warp, child_root);

    // R2's op: try to write to the child warp (should be rejected)
    let r2_target_node = make_node_id("test/new-node-in-child");
    let r2_op = WarpOp::UpsertNode {
        node: NodeKey {
            warp_id: child_warp,
            local_id: r2_target_node,
        },
        record: NodeRecord {
            ty: make_type_id("test/new-node"),
        },
    };

    // Build two deltas from different rules
    let mut delta1 = TickDelta::new();
    delta1.emit_with_origin(r1_op, make_origin(1, 100));

    let mut delta2 = TickDelta::new();
    delta2.emit_with_origin(r2_op, make_origin(2, 200));

    // Merge should fail: R2 targets a warp created by R1 in the same tick
    let result = merge_deltas_ok(vec![delta1, delta2]);

    assert!(
        result.is_err(),
        "merge_deltas_ok should reject writes to newly created warps in the same tick"
    );

    // Verify it's specifically a WriteToNewWarp error
    match result.expect_err("should have error") {
        MergeError::WriteToNewWarp {
            warp_id,
            op_kind,
            op_origin,
        } => {
            assert_eq!(warp_id, child_warp, "error should identify the child warp");
            assert_eq!(op_kind, "UpsertNode", "error should identify the op kind");
            assert_eq!(
                op_origin.rule_id, 200,
                "error should identify R2 as the violator"
            );
        }
        MergeError::Conflict(_) => {
            panic!("Expected MergeError::WriteToNewWarp, got Conflict");
        }
        MergeError::PoisonedDelta(_) => {
            panic!("Expected MergeError::WriteToNewWarp, got PoisonedDelta");
        }
    }
}

/// T7.2: OpenPortal(RequireExisting) allows same-tick writes to child_warp.
///
/// When OpenPortal uses init=RequireExisting, the child warp already exists,
/// so same-tick writes are permitted (no new warp is being created).
///
/// Setup:
/// - Delta 1: R1 emits OpenPortal { child_warp=W_child, init=PortalInit::RequireExisting }
/// - Delta 2: R2 emits UpsertNode targeting W_child
///
/// Expected: merge_deltas_ok succeeds (RequireExisting means warp already exists)
#[test]
fn openportal_require_existing_allows_same_tick_writes() {
    let parent_warp = make_warp_id("test/parent-existing");
    let parent_node = make_node_id("test/parent-node-existing");
    let child_warp = make_warp_id("test/child-existing");
    let child_root = make_node_id("test/child-root-existing");

    // R1's op: OpenPortal with RequireExisting (warp already exists)
    let r1_op = make_open_portal_require_existing(parent_warp, parent_node, child_warp, child_root);

    // R2's op: write to the child warp (should be allowed)
    let r2_target_node = make_node_id("test/new-node-in-existing");
    let r2_op = WarpOp::UpsertNode {
        node: NodeKey {
            warp_id: child_warp,
            local_id: r2_target_node,
        },
        record: NodeRecord {
            ty: make_type_id("test/new-node"),
        },
    };

    // Build two deltas from different rules
    let mut delta1 = TickDelta::new();
    delta1.emit_with_origin(r1_op.clone(), make_origin(1, 100));

    let mut delta2 = TickDelta::new();
    delta2.emit_with_origin(r2_op.clone(), make_origin(2, 200));

    // Merge should succeed: RequireExisting means the warp already exists
    let result = merge_deltas_ok(vec![delta1, delta2]);

    assert!(
        result.is_ok(),
        "merge_deltas_ok should allow writes when OpenPortal uses RequireExisting"
    );

    let merged_ops = result.expect("merge should succeed");
    assert_eq!(merged_ops.len(), 2, "Both ops should be in merged result");

    // Verify both ops are present
    assert!(
        merged_ops.iter().any(|op| op == &r1_op),
        "OpenPortal op must be present"
    );
    assert!(
        merged_ops.iter().any(|op| op == &r2_op),
        "UpsertNode op must be present"
    );
}

/// T7.3: Two OpenPortal(Empty) ops targeting same child_warp in same tick.
///
/// When two rules both emit OpenPortal(Empty) for the same child_warp and
/// neither has any other writes to that warp, the behavior depends on policy:
/// - If they have the same attachment key, they conflict (same WarpOpKey)
/// - If they have different attachment keys, both should succeed
///
/// This test covers the case with different attachment keys (no other writes).
#[test]
fn two_creators_same_tick_no_other_writes() {
    let parent_warp = make_warp_id("test/parent-dual");
    let parent_node_a = make_node_id("test/parent-node-a");
    let parent_node_b = make_node_id("test/parent-node-b");
    let child_warp = make_warp_id("test/child-dual");
    let child_root = make_node_id("test/child-root-dual");

    // R1's op: OpenPortal(Empty) from node A
    let r1_op = make_open_portal_op(parent_warp, parent_node_a, child_warp, child_root);

    // R2's op: OpenPortal(Empty) from node B (different attachment key)
    let r2_op = make_open_portal_op(parent_warp, parent_node_b, child_warp, child_root);

    // Build two deltas from different rules
    let mut delta1 = TickDelta::new();
    delta1.emit_with_origin(r1_op.clone(), make_origin(1, 100));

    let mut delta2 = TickDelta::new();
    delta2.emit_with_origin(r2_op.clone(), make_origin(2, 200));

    // Merge: Two OpenPortal(Empty) ops with different attachment keys should succeed
    // (no writes to the new warp, just two portals pointing to it)
    let result = merge_deltas_ok(vec![delta1, delta2]);

    assert!(
        result.is_ok(),
        "Two OpenPortal(Empty) ops with different keys and no writes should merge"
    );

    let merged_ops = result.expect("merge should succeed");
    assert_eq!(
        merged_ops.len(),
        2,
        "Both OpenPortal ops should be in merged result"
    );

    // Verify both ops are present
    assert!(
        merged_ops.iter().any(|op| op == &r1_op),
        "First OpenPortal op must be present"
    );
    assert!(
        merged_ops.iter().any(|op| op == &r2_op),
        "Second OpenPortal op must be present"
    );
}

/// T7.3b: Two OpenPortal(Empty) ops targeting same attachment key conflict.
///
/// When two OpenPortal(Empty) ops have the same attachment key (same sort_key),
/// they conflict if they have different child_warp values.
#[test]
fn two_creators_same_attachment_key_conflicts() {
    let parent_warp = make_warp_id("test/parent-conflict");
    let parent_node = make_node_id("test/parent-node-conflict");
    let child_warp_a = make_warp_id("test/child-a");
    let child_warp_b = make_warp_id("test/child-b");
    let child_root = make_node_id("test/child-root-conflict");

    // R1's op: OpenPortal(Empty) pointing to child_warp_a
    let r1_op = make_open_portal_op(parent_warp, parent_node, child_warp_a, child_root);

    // R2's op: OpenPortal(Empty) pointing to child_warp_b (same attachment key!)
    let r2_op = make_open_portal_op(parent_warp, parent_node, child_warp_b, child_root);

    // Both ops target the same attachment key (parent_node's alpha attachment)
    // but point to different child warps - this should conflict.

    // Build two deltas from different rules
    let mut delta1 = TickDelta::new();
    delta1.emit_with_origin(r1_op, make_origin(1, 100));

    let mut delta2 = TickDelta::new();
    delta2.emit_with_origin(r2_op, make_origin(2, 200));

    // Merge should fail: same attachment key with different values
    let result = merge_deltas_ok(vec![delta1, delta2]);

    assert!(
        result.is_err(),
        "Two OpenPortal ops with same attachment key but different child_warp must conflict"
    );

    // Verify it's a conflict error with both writers
    match result.expect_err("should have conflict") {
        MergeError::Conflict(conflict) => {
            assert_eq!(
                conflict.writers.len(),
                2,
                "Conflict should report both writers"
            );
        }
        MergeError::WriteToNewWarp { .. } => {
            panic!("Expected MergeError::Conflict, got WriteToNewWarp");
        }
        MergeError::PoisonedDelta(_) => {
            panic!("Expected MergeError::Conflict, got PoisonedDelta");
        }
    }
}

// =============================================================================
// Additional unit tests (not dependent on merge-phase enforcement)
// =============================================================================

#[test]
fn openportal_op_structure_is_correct() {
    // Unit test: verify WarpOp::OpenPortal has the expected structure.
    // This test does NOT require the scheduling infrastructure.

    let parent_warp = make_warp_id("test/parent-struct");
    let parent_node = make_node_id("test/parent-node-struct");
    let child_warp = make_warp_id("test/child-struct");
    let child_root = make_node_id("test/child-root-struct");

    let op = make_open_portal_op(parent_warp, parent_node, child_warp, child_root);

    // Verify the op matches expected structure
    match op {
        WarpOp::OpenPortal {
            key,
            child_warp: op_child_warp,
            child_root: op_child_root,
            init,
        } => {
            // Verify attachment key points to parent node
            match key.owner {
                warp_core::AttachmentOwner::Node(node_key) => {
                    assert_eq!(
                        node_key.warp_id, parent_warp,
                        "attachment key should reference parent warp"
                    );
                    assert_eq!(
                        node_key.local_id, parent_node,
                        "attachment key should reference parent node"
                    );
                }
                warp_core::AttachmentOwner::Edge(_) => {
                    panic!("expected node attachment, got edge attachment");
                }
            }

            // Verify child warp and root
            assert_eq!(op_child_warp, child_warp, "child_warp should match input");
            assert_eq!(op_child_root, child_root, "child_root should match input");

            // Verify init policy is Empty with correct root record
            match init {
                PortalInit::Empty { root_record } => {
                    assert_eq!(
                        root_record.ty,
                        make_type_id("test/child-root"),
                        "root_record type should match"
                    );
                }
                PortalInit::RequireExisting => {
                    panic!("expected PortalInit::Empty, got RequireExisting");
                }
            }
        }
        _ => panic!("expected WarpOp::OpenPortal, got {op:?}"),
    }
}

#[test]
fn openportal_canonical_ordering_via_patch() {
    // Unit test: verify OpenPortal ops are canonically ordered in a patch.
    // Tests canonical ordering indirectly through WarpTickPatchV1.
    //
    // WarpTickPatchV1::new() sorts and dedupes ops by their sort_key,
    // so we can verify ordering behavior through the patch API.

    use warp_core::{TickCommitStatus, WarpTickPatchV1, POLICY_ID_NO_POLICY_V0};

    let parent_warp = make_warp_id("test/parent-order");
    let parent_node_a = make_node_id("test/parent-node-a");
    let parent_node_b = make_node_id("test/parent-node-b");
    let child_warp_a = make_warp_id("test/child-order-a");
    let child_warp_b = make_warp_id("test/child-order-b");
    let child_root = make_node_id("test/child-root-order");

    let op_a = make_open_portal_op(parent_warp, parent_node_a, child_warp_a, child_root);
    let op_b = make_open_portal_op(parent_warp, parent_node_b, child_warp_b, child_root);

    // Insert in reverse order - patch should canonicalize
    let ops_reverse = vec![op_b.clone(), op_a.clone()];
    let ops_forward = vec![op_a, op_b];

    let patch_reverse = WarpTickPatchV1::new(
        POLICY_ID_NO_POLICY_V0,
        [1u8; 32],
        TickCommitStatus::Committed,
        vec![],
        vec![],
        ops_reverse,
    );

    let patch_forward = WarpTickPatchV1::new(
        POLICY_ID_NO_POLICY_V0,
        [1u8; 32],
        TickCommitStatus::Committed,
        vec![],
        vec![],
        ops_forward,
    );

    // Both patches should have the same digest (same canonical order)
    assert_eq!(
        patch_reverse.digest(),
        patch_forward.digest(),
        "patches with same ops in different input order should have identical digest"
    );

    // Both patches should have 2 ops
    assert_eq!(patch_reverse.ops().len(), 2, "patch should have 2 ops");
    assert_eq!(patch_forward.ops().len(), 2, "patch should have 2 ops");
}

#[test]
fn openportal_identical_ops_dedupe_in_patch() {
    // Unit test: verify identical OpenPortal ops are deduped in a patch.
    // WarpTickPatchV1::new() dedupes by sort_key with last-wins semantics.

    use warp_core::{TickCommitStatus, WarpTickPatchV1, POLICY_ID_NO_POLICY_V0};

    let parent_warp = make_warp_id("test/parent-dedupe");
    let parent_node = make_node_id("test/parent-node-dedupe");
    let child_warp = make_warp_id("test/child-dedupe");
    let child_root = make_node_id("test/child-root-dedupe");

    let op1 = make_open_portal_op(parent_warp, parent_node, child_warp, child_root);
    let op2 = make_open_portal_op(parent_warp, parent_node, child_warp, child_root);

    // Insert duplicate ops
    let ops = vec![op1, op2];

    let patch = WarpTickPatchV1::new(
        POLICY_ID_NO_POLICY_V0,
        [1u8; 32],
        TickCommitStatus::Committed,
        vec![],
        vec![],
        ops,
    );

    // Should dedupe to 1 op (same attachment key = same sort_key)
    assert_eq!(
        patch.ops().len(),
        1,
        "identical OpenPortal ops should be deduped to 1"
    );
}

#[test]
fn openportal_with_require_existing_init() {
    // Unit test: verify PortalInit::RequireExisting variant works.

    let parent_warp = make_warp_id("test/parent-existing");
    let parent_node = make_node_id("test/parent-node-existing");
    let child_warp = make_warp_id("test/child-existing");
    let child_root = make_node_id("test/child-root-existing");

    let parent_key = AttachmentKey::node_alpha(NodeKey {
        warp_id: parent_warp,
        local_id: parent_node,
    });

    let op = WarpOp::OpenPortal {
        key: parent_key,
        child_warp,
        child_root,
        init: PortalInit::RequireExisting,
    };

    match op {
        WarpOp::OpenPortal { init, .. } => {
            assert!(
                matches!(init, PortalInit::RequireExisting),
                "init should be RequireExisting"
            );
        }
        _ => panic!("expected WarpOp::OpenPortal"),
    }
}
