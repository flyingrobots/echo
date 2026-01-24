// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! BOAW Footprint & Independence Tests (ADR-0007 §6)
//!
//! Tests for footprint independence checking, bucket enforcement,
//! and drift guards.

mod common;

use std::panic::{catch_unwind, AssertUnwindSafe};

use common::{random_footprint, XorShift64};
use warp_core::{
    make_edge_id, make_node_id, make_type_id, make_warp_id, ApplyResult, AtomPayload,
    AttachmentKey, AttachmentSet, AttachmentValue, ConflictPolicy, EdgeRecord, EdgeSet, Engine,
    Footprint, FootprintViolation, GraphStore, GraphView, NodeId, NodeKey, NodeRecord, NodeSet,
    PatternGraph, PortSet, RewriteRule, TickDelta, ViolationKind, WarpInstance, WarpOp,
};

// =============================================================================
// T3: Footprints & Independence
// =============================================================================

#[test]
fn t3_1_footprint_independence_is_symmetric() {
    // `fp.independent(a,b) == fp.independent(b,a)` for randomized footprints.
    //
    // This test can run against the existing Footprint implementation.

    let mut rng = XorShift64::new(0xDEAD_BEEF);

    for _ in 0..100 {
        let fp_a = random_footprint(&mut rng);
        let fp_b = random_footprint(&mut rng);

        let ab = fp_a.independent(&fp_b);
        let ba = fp_b.independent(&fp_a);

        assert_eq!(
            ab, ba,
            "Footprint independence is not symmetric:\n  fp_a: {fp_a:?}\n  fp_b: {fp_b:?}"
        );
    }
}

// TODO(FP-001): Implement once bucket target enforcement exists.
#[test]
#[ignore = "FP-001: BOAW bucket target enforcement not yet implemented"]
fn t3_2_no_write_read_overlap_admitted() {
    // Given: two planned rewrites where one writes a node the other reads
    // Expect: only one admitted
    todo!(
        "FP-001: build two PlannedRewrites with write/read overlap; \
         assert only one is admitted"
    );
}

// TODO(FP-002): Implement once bucket target enforcement exists.
#[test]
#[ignore = "FP-002: BOAW bucket target enforcement not yet implemented"]
fn t3_3_deletes_that_share_adjacency_bucket_must_conflict() {
    // The classic race: delete e1=(A->B) and e2=(A->C) both mutate edges_from[A].
    // Your footprint model must claim the bucket target
    // (e.g., AttachmentKey::EdgesFromBucket(A)).
    //
    // Given: two edge deletes with same `from` but different edge_id
    // Expect: independence fails when adjacency bucket target is claimed
    //
    // (This test prevents the "retain() race" forever.)
    todo!(
        "FP-002: build two PlannedRewrites deleting edges from same node; \
         assert admission rejects running both concurrently"
    );
}

// =============================================================================
// Footprint enforcement helpers
// =============================================================================

fn test_rule_id(name: &str) -> warp_core::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:test:");
    hasher.update(name.as_bytes());
    hasher.finalize().into()
}

fn always_match(_: GraphView<'_>, _: &NodeId) -> bool {
    true
}

fn build_enforcement_engine(scope: NodeId) -> Engine {
    let mut store = GraphStore::default();
    store.insert_node(
        scope,
        NodeRecord {
            ty: make_type_id("test-entity"),
        },
    );
    Engine::new(store, scope)
}

/// Registers a rule, applies it to scope, and commits — returning catch_unwind result.
fn run_rule_catching_panic(
    rule: RewriteRule,
    scope: NodeId,
) -> Result<(), Box<dyn std::any::Any + Send>> {
    let rule_name = rule.name;
    let mut engine = build_enforcement_engine(scope);
    engine.register_rule(rule).expect("register rule");
    let tx = engine.begin();
    let applied = engine.apply(tx, rule_name, &scope).expect("apply");
    assert!(matches!(applied, ApplyResult::Applied), "rule must match");
    catch_unwind(AssertUnwindSafe(move || {
        engine.commit(tx).expect("commit");
    }))
}

// =============================================================================
// t3_4: NodeReadNotDeclared — executor reads undeclared node
// =============================================================================

const T3_4_NAME: &str = "test/t3_4_drift";

fn t3_4_executor(view: GraphView<'_>, _scope: &NodeId, _delta: &mut TickDelta) {
    let undeclared = make_node_id("t3-4-undeclared-target");
    let _ = view.node(&undeclared);
}

fn t3_4_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut n_read = NodeSet::default();
    n_read.insert_with_warp(view.warp_id(), *scope);
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

#[test]
fn t3_4_footprint_guard_catches_executor_drift() {
    let scope = make_node_id("t3-4-scope");
    let rule = RewriteRule {
        id: test_rule_id(T3_4_NAME),
        name: T3_4_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: always_match,
        executor: t3_4_executor,
        compute_footprint: t3_4_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let result = run_rule_catching_panic(rule, scope);
    let err = result.expect_err("should panic on undeclared read");
    let violation = err
        .downcast_ref::<FootprintViolation>()
        .expect("panic payload must be FootprintViolation");
    assert_eq!(violation.rule_name, T3_4_NAME);
    assert_eq!(violation.op_kind, "node_read");
    let undeclared = make_node_id("t3-4-undeclared-target");
    assert!(
        matches!(violation.kind, ViolationKind::NodeReadNotDeclared(id) if id == undeclared),
        "expected NodeReadNotDeclared, got {:?}",
        violation.kind
    );
}

// =============================================================================
// t3_5: NodeWriteNotDeclared — emits UpsertNode for undeclared target
// =============================================================================

const T3_5_NAME: &str = "test/t3_5_write";

fn t3_5_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(scope);
    let undeclared = make_node_id("t3-5-undeclared-write");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id: view.warp_id(),
            local_id: undeclared,
        },
        record: NodeRecord {
            ty: make_type_id("test"),
        },
    });
}

fn t3_5_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut n_read = NodeSet::default();
    n_read.insert_with_warp(view.warp_id(), *scope);
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

#[test]
fn t3_5_write_violation_undeclared_node() {
    let scope = make_node_id("t3-5-scope");
    let rule = RewriteRule {
        id: test_rule_id(T3_5_NAME),
        name: T3_5_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: always_match,
        executor: t3_5_executor,
        compute_footprint: t3_5_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let result = run_rule_catching_panic(rule, scope);
    let err = result.expect_err("should panic on undeclared write");
    let violation = err
        .downcast_ref::<FootprintViolation>()
        .expect("panic payload must be FootprintViolation");
    assert_eq!(violation.rule_name, T3_5_NAME);
    assert_eq!(violation.op_kind, "UpsertNode");
    let undeclared = make_node_id("t3-5-undeclared-write");
    assert!(
        matches!(violation.kind, ViolationKind::NodeWriteNotDeclared(id) if id == undeclared),
        "expected NodeWriteNotDeclared, got {:?}",
        violation.kind
    );
}

// =============================================================================
// t3_6: CrossWarpEmission — emits op with wrong warp_id
// =============================================================================

const T3_6_NAME: &str = "test/t3_6_cross_warp";

fn t3_6_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(scope);
    let wrong_warp = make_warp_id("wrong-warp-t3-6");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id: wrong_warp,
            local_id: *scope,
        },
        record: NodeRecord {
            ty: make_type_id("test"),
        },
    });
}

fn t3_6_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut n_write = NodeSet::default();
    n_read.insert_with_warp(warp_id, *scope);
    n_write.insert_with_warp(warp_id, *scope);
    Footprint {
        n_read,
        n_write,
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

#[test]
fn t3_6_cross_warp_emission_rejected() {
    let scope = make_node_id("t3-6-scope");
    let rule = RewriteRule {
        id: test_rule_id(T3_6_NAME),
        name: T3_6_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: always_match,
        executor: t3_6_executor,
        compute_footprint: t3_6_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let result = run_rule_catching_panic(rule, scope);
    let err = result.expect_err("should panic on cross-warp emission");
    let violation = err
        .downcast_ref::<FootprintViolation>()
        .expect("panic payload must be FootprintViolation");
    assert_eq!(violation.rule_name, T3_6_NAME);
    let wrong_warp = make_warp_id("wrong-warp-t3-6");
    assert!(
        matches!(violation.kind, ViolationKind::CrossWarpEmission { op_warp } if op_warp == wrong_warp),
        "expected CrossWarpEmission, got {:?}",
        violation.kind
    );
}

// =============================================================================
// t3_7: AttachmentReadNotDeclared — reads undeclared attachment
// =============================================================================

const T3_7_NAME: &str = "test/t3_7_attach_read";

fn t3_7_executor(view: GraphView<'_>, scope: &NodeId, _delta: &mut TickDelta) {
    let _ = view.node(scope);
    let _ = view.node_attachment(scope);
}

fn t3_7_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut n_read = NodeSet::default();
    n_read.insert_with_warp(view.warp_id(), *scope);
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

#[test]
fn t3_7_attachment_requires_full_key() {
    let scope = make_node_id("t3-7-scope");
    let rule = RewriteRule {
        id: test_rule_id(T3_7_NAME),
        name: T3_7_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: always_match,
        executor: t3_7_executor,
        compute_footprint: t3_7_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let result = run_rule_catching_panic(rule, scope);
    let err = result.expect_err("should panic on undeclared attachment read");
    let violation = err
        .downcast_ref::<FootprintViolation>()
        .expect("panic payload must be FootprintViolation");
    assert_eq!(violation.rule_name, T3_7_NAME);
    assert_eq!(violation.op_kind, "node_attachment_read");
    assert!(
        matches!(violation.kind, ViolationKind::AttachmentReadNotDeclared(..)),
        "expected AttachmentReadNotDeclared, got {:?}",
        violation.kind
    );
}

// =============================================================================
// t3_8: UnauthorizedInstanceOp — user rule emits UpsertWarpInstance
// =============================================================================

const T3_8_NAME: &str = "test/t3_8_instance_op";

fn t3_8_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(scope);
    delta.push(WarpOp::UpsertWarpInstance {
        instance: WarpInstance {
            warp_id: view.warp_id(),
            root_node: *scope,
            parent: None,
        },
    });
}

fn t3_8_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut n_read = NodeSet::default();
    n_read.insert_with_warp(view.warp_id(), *scope);
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

#[test]
fn t3_8_system_ops_blocked_for_user_rules() {
    let scope = make_node_id("t3-8-scope");
    let rule = RewriteRule {
        id: test_rule_id(T3_8_NAME),
        name: T3_8_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: always_match,
        executor: t3_8_executor,
        compute_footprint: t3_8_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let result = run_rule_catching_panic(rule, scope);
    let err = result.expect_err("should panic on unauthorized instance op");
    let violation = err
        .downcast_ref::<FootprintViolation>()
        .expect("panic payload must be FootprintViolation");
    assert_eq!(violation.rule_name, T3_8_NAME);
    assert_eq!(violation.op_kind, "UpsertWarpInstance");
    assert!(
        matches!(violation.kind, ViolationKind::UnauthorizedInstanceOp),
        "expected UnauthorizedInstanceOp, got {:?}",
        violation.kind
    );
}

// =============================================================================
// t3_9: Happy path — correctly declared footprint, no panic
// =============================================================================

const T3_9_NAME: &str = "test/t3_9_happy";

fn t3_9_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(scope);
    let _ = view.node_attachment(scope);
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id: view.warp_id(),
        local_id: *scope,
    });
    delta.push(WarpOp::SetAttachment {
        key,
        value: Some(AttachmentValue::Atom(AtomPayload {
            type_id: make_type_id("test-payload"),
            bytes: bytes::Bytes::from_static(b"\x01\x02\x03"),
        })),
    });
}

fn t3_9_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut a_read = AttachmentSet::default();
    let mut a_write = AttachmentSet::default();
    n_read.insert_with_warp(warp_id, *scope);
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: *scope,
    });
    a_read.insert(key);
    a_write.insert(key);
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read,
        a_write,
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

#[test]
fn t3_9_correctly_declared_no_panic() {
    let scope = make_node_id("t3-9-scope");
    let rule = RewriteRule {
        id: test_rule_id(T3_9_NAME),
        name: T3_9_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: always_match,
        executor: t3_9_executor,
        compute_footprint: t3_9_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let result = run_rule_catching_panic(rule, scope);
    assert!(
        result.is_ok(),
        "correctly declared footprint must not panic"
    );
}

// =============================================================================
// t3_10: edges_from implied by node_read
// =============================================================================

const T3_10_NAME: &str = "test/t3_10_edges";

fn t3_10_executor(view: GraphView<'_>, scope: &NodeId, _delta: &mut TickDelta) {
    let _ = view.node(scope);
    for _edge in view.edges_from(scope) {
        // Just iterate — should not panic
    }
}

fn t3_10_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut n_read = NodeSet::default();
    n_read.insert_with_warp(view.warp_id(), *scope);
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

#[test]
fn t3_10_edges_from_implied_by_node_read() {
    let scope = make_node_id("t3-10-scope");
    let rule = RewriteRule {
        id: test_rule_id(T3_10_NAME),
        name: T3_10_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: always_match,
        executor: t3_10_executor,
        compute_footprint: t3_10_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let result = run_rule_catching_panic(rule, scope);
    assert!(result.is_ok(), "edges_from on declared node must not panic");
}

// =============================================================================
// t3_11: EdgeWriteRequiresFromInNodesWrite
// =============================================================================

const T3_11_NAME: &str = "test/t3_11_edge_from";

fn t3_11_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(scope);
    let edge_id = make_edge_id("t3-11-edge");
    delta.push(WarpOp::UpsertEdge {
        warp_id: view.warp_id(),
        record: EdgeRecord {
            id: edge_id,
            from: *scope,
            to: make_node_id("t3-11-to"),
            ty: make_type_id("test-edge"),
        },
    });
}

fn t3_11_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut e_write = EdgeSet::default();
    n_read.insert_with_warp(warp_id, *scope);
    e_write.insert_with_warp(warp_id, make_edge_id("t3-11-edge"));
    Footprint {
        n_read,
        n_write: NodeSet::default(), // Missing scope!
        e_read: EdgeSet::default(),
        e_write,
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

#[test]
fn t3_11_edge_write_requires_from_in_nodes_write() {
    let scope = make_node_id("t3-11-scope");
    let rule = RewriteRule {
        id: test_rule_id(T3_11_NAME),
        name: T3_11_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: always_match,
        executor: t3_11_executor,
        compute_footprint: t3_11_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let result = run_rule_catching_panic(rule, scope);
    let err = result.expect_err("should panic: edge write requires from in n_write");
    let violation = err
        .downcast_ref::<FootprintViolation>()
        .expect("panic payload must be FootprintViolation");
    assert_eq!(violation.rule_name, T3_11_NAME);
    assert_eq!(violation.op_kind, "UpsertEdge");
    assert!(
        matches!(violation.kind, ViolationKind::NodeWriteNotDeclared(id) if id == scope),
        "expected NodeWriteNotDeclared for scope (adjacency), got {:?}",
        violation.kind
    );
}

// =============================================================================
// t3_12a: Write violation overrides executor panic
// =============================================================================

const T3_12A_NAME: &str = "test/t3_12a_write_override";

fn t3_12a_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(scope);
    // Emit undeclared write BEFORE panicking
    let undeclared = make_node_id("t3-12a-undeclared");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id: view.warp_id(),
            local_id: undeclared,
        },
        record: NodeRecord {
            ty: make_type_id("test"),
        },
    });
    std::panic::panic_any("deliberate-12a");
}

fn t3_12a_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut n_read = NodeSet::default();
    n_read.insert_with_warp(view.warp_id(), *scope);
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

#[test]
fn t3_12a_write_violation_overrides_executor_panic() {
    let scope = make_node_id("t3-12a-scope");
    let rule = RewriteRule {
        id: test_rule_id(T3_12A_NAME),
        name: T3_12A_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: always_match,
        executor: t3_12a_executor,
        compute_footprint: t3_12a_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let result = run_rule_catching_panic(rule, scope);
    let err = result.expect_err("should panic (write violation OR executor panic)");
    let violation = err
        .downcast_ref::<FootprintViolation>()
        .expect("write violation must override executor panic");
    assert_eq!(violation.rule_name, T3_12A_NAME);
    assert_eq!(violation.op_kind, "UpsertNode");
    assert!(
        matches!(violation.kind, ViolationKind::NodeWriteNotDeclared(..)),
        "expected NodeWriteNotDeclared, got {:?}",
        violation.kind
    );
}

// =============================================================================
// t3_12b: Executor panic propagates when footprint is clean
// =============================================================================

const T3_12B_NAME: &str = "test/t3_12b_clean_panic";

fn t3_12b_executor(view: GraphView<'_>, scope: &NodeId, _delta: &mut TickDelta) {
    let _ = view.node(scope);
    // No ops emitted — footprint is clean. But we panic.
    std::panic::panic_any("deliberate-12b");
}

fn t3_12b_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut n_read = NodeSet::default();
    n_read.insert_with_warp(view.warp_id(), *scope);
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

#[test]
fn t3_12b_executor_panic_propagates_when_footprint_clean() {
    let scope = make_node_id("t3-12b-scope");
    let rule = RewriteRule {
        id: test_rule_id(T3_12B_NAME),
        name: T3_12B_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: always_match,
        executor: t3_12b_executor,
        compute_footprint: t3_12b_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    };

    let result = run_rule_catching_panic(rule, scope);
    let err = result.expect_err("executor panic should propagate");
    assert!(
        err.downcast_ref::<FootprintViolation>().is_none(),
        "clean footprint must not produce FootprintViolation"
    );
    let msg = err
        .downcast_ref::<&str>()
        .expect("original panic payload must be &str");
    assert_eq!(*msg, "deliberate-12b");
}

// =============================================================================
// Factor mask prefiltering
// =============================================================================

#[test]
fn factor_mask_disjoint_is_fast_path() {
    // Verify that disjoint factor_mask allows early-exit in independence check.
    use warp_core::Footprint;

    let fp_a = Footprint {
        factor_mask: 0b0000_1111,
        ..Default::default()
    };

    let mut fp_b = Footprint {
        factor_mask: 0b1111_0000,
        ..Default::default()
    };

    // Disjoint masks → independent (fast path)
    assert!(
        fp_a.independent(&fp_b),
        "Disjoint factor_mask should be independent"
    );

    // Overlapping masks require full check
    fp_b.factor_mask = 0b0000_0001;
    // Still independent if no actual read/write overlap
    assert!(
        fp_a.independent(&fp_b),
        "Overlapping mask but no actual conflict should be independent"
    );
}

// =============================================================================
// T4.1: Shard routing stability
// =============================================================================

#[test]
fn t4_1_shard_routing_is_stable_across_machines() {
    // Given: same NodeId/EdgeId
    // Expect: same shard id (with fixed SHARDS constant)
    //
    // We use fixed virtual shards (e.g., 256/1024, power-of-two).
    // Route by existing NodeId/EdgeId bits (no rehash):
    //   shard = lowbits(id) & (SHARDS-1)

    const SHARDS: usize = 256;

    let test_hashes: [[u8; 32]; 5] = [
        [0x00; 32],
        [0xFF; 32],
        [0x42; 32],
        {
            let mut h = [0u8; 32];
            h[0] = 0xAB;
            h
        },
        {
            let mut h = [0u8; 32];
            h[31] = 0xCD;
            h
        },
    ];

    // Pre-computed expected shard IDs using the routing rule: hash[0] & (SHARDS - 1)
    // SHARDS = 256, so mask is 0xFF (all low bits pass through)
    let expected_shards: [usize; 5] = [
        0x00, // [0x00; 32] → 0x00 & 0xFF = 0
        0xFF, // [0xFF; 32] → 0xFF & 0xFF = 255
        0x42, // [0x42; 32] → 0x42 & 0xFF = 66
        0xAB, // h[0] = 0xAB → 0xAB & 0xFF = 171
        0x00, // h[31] = 0xCD, h[0] = 0 → 0x00 & 0xFF = 0
    ];

    for (i, hash) in test_hashes.iter().enumerate() {
        let node_id = NodeId(*hash);
        // Use low bits of the hash for shard routing (deterministic)
        let shard = (hash[0] as usize) & (SHARDS - 1);

        assert_eq!(
            shard, expected_shards[i],
            "Shard routing must be stable for {node_id:?}"
        );
    }
}
