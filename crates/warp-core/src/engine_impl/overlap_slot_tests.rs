// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Reservation reads a tick's slots back from the scheduler's conflict sets.
//! This witness commits rewrites whose footprints overlap, both across
//! rewrites (shared reads) and within one footprint (a slot that is read and
//! written), and checks that the committed patch carries exactly the sorted,
//! repeat-free union that the former per-footprint `BTreeSet` collection
//! produced.
//!
//! It also witnesses that a commit which fails after reservation, whether a
//! rule panics or the commit returns an error, closes its transaction: the
//! transaction is refused afterwards, and a new one is judged only on its own
//! reservations.
#![allow(clippy::expect_used, clippy::panic)] // Test assertions may abort the witness; one rule panics on purpose.
use std::collections::BTreeSet;

use super::*;
use crate::{AtomPayload, EdgeId, EdgeKey, Footprint, OutgoingEdgeRange, PatternGraph, TickDelta};

const RULE: &str = "test/overlap-slots";
const OWN: [&str; 3] = [
    "test/overlap-own-0",
    "test/overlap-own-1",
    "test/overlap-own-2",
];

fn shared_node() -> NodeId {
    make_node_id("test/overlap-shared")
}

fn shared_edge() -> EdgeId {
    make_edge_id("test/overlap-shared-edge")
}

fn shared_range(warp_id: WarpId) -> OutgoingEdgeRange {
    OutgoingEdgeRange {
        source: NodeKey {
            warp_id,
            local_id: shared_node(),
        },
        edge_type: make_type_id("test/overlap-link"),
    }
}

fn port_of(scope: &NodeId) -> u64 {
    u64::from(scope.0[0])
}

/// Offset of the port each rewrite only reads, above the one it writes.
const IN_ONLY_PORT_OFFSET: u64 = 1_000;

fn own_edge(warp_id: WarpId, scope: &NodeId) -> EdgeKey {
    EdgeKey {
        warp_id,
        local_id: make_edge_id(&format!("test/overlap-own-edge/{}", port_of(scope))),
    }
}

fn own_range(warp_id: WarpId, scope: &NodeId) -> OutgoingEdgeRange {
    OutgoingEdgeRange {
        source: NodeKey {
            warp_id,
            local_id: *scope,
        },
        edge_type: make_type_id("test/overlap-link"),
    }
}

fn footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let shared = NodeKey {
        warp_id,
        local_id: shared_node(),
    };
    let own = NodeKey {
        warp_id,
        local_id: *scope,
    };
    let mut footprint = Footprint::default();
    // Read by every rewrite.
    footprint.n_read.insert(shared);
    footprint.a_read.insert(AttachmentKey::node_alpha(shared));
    footprint.e_read.insert(EdgeKey {
        warp_id,
        local_id: shared_edge(),
    });
    footprint.r_read.insert(shared_range(warp_id));
    // Read and written by this rewrite alone.
    footprint.n_read.insert(own);
    footprint.n_write.insert(own);
    footprint.a_read.insert(AttachmentKey::node_alpha(own));
    footprint.a_write.insert(AttachmentKey::node_alpha(own));
    footprint.b_in.insert(warp_id, port_of(scope));
    footprint.b_out.insert(warp_id, port_of(scope));
    // Only read (a port) and only written (an edge and a range), so the
    // read-only side of ports and the write-only side of edges and ranges
    // are exercised too.
    footprint
        .b_in
        .insert(warp_id, port_of(scope) + IN_ONLY_PORT_OFFSET);
    footprint.e_write.insert(own_edge(warp_id, scope));
    footprint.r_write.insert(own_range(warp_id, scope));
    footprint
}

fn executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    delta.push(WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(NodeKey {
            warp_id: view.warp_id(),
            local_id: *scope,
        }),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("test/overlap-value"),
            bytes::Bytes::from_static(b"written"),
        ))),
    });
}

#[test]
fn overlapping_footprints_commit_the_same_sorted_slot_sets() {
    let mut store = GraphStore::default();
    let ty = make_type_id("test/overlap-node");
    store.insert_node(shared_node(), NodeRecord { ty });
    let own: Vec<NodeId> = OWN.iter().map(|label| make_node_id(label)).collect();
    for node in &own {
        store.insert_node(*node, NodeRecord { ty });
    }
    let ports: BTreeSet<u64> = own.iter().map(port_of).collect();
    assert_eq!(ports.len(), own.len(), "each rewrite needs its own port");
    let mut engine = Engine::new(store, shared_node());
    engine
        .register_rule(RewriteRule {
            id: [7; 32],
            name: RULE,
            left: PatternGraph { nodes: vec![] },
            matcher: |_, _| true,
            executor: RuleExecutor::legacy(executor),
            compute_footprint: footprint,
            factor_mask: 0,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        })
        .expect("rule registers");
    let tx = engine.begin();
    for node in &own {
        assert!(matches!(
            engine.apply(tx, RULE, node),
            Ok(ApplyResult::Applied)
        ));
    }
    let (_, receipt, patch) = engine.commit_with_receipt(tx).expect("tick commits");
    assert!(receipt
        .entries()
        .iter()
        .all(|entry| entry.disposition == TickReceiptDisposition::Applied));

    // What the former `BTreeSet` collection held: the union, once each.
    let warp_id = engine.current_root.warp_id;
    let shared = NodeKey {
        warp_id,
        local_id: shared_node(),
    };
    let mut expected_in: BTreeSet<SlotId> = BTreeSet::from([
        SlotId::Node(shared),
        SlotId::Attachment(AttachmentKey::node_alpha(shared)),
        SlotId::Edge(EdgeKey {
            warp_id,
            local_id: shared_edge(),
        }),
        SlotId::OutgoingEdges(shared_range(warp_id)),
    ]);
    let mut expected_out: BTreeSet<SlotId> = BTreeSet::new();
    for node in &own {
        let key = NodeKey {
            warp_id,
            local_id: *node,
        };
        for slot in [
            SlotId::Node(key),
            SlotId::Attachment(AttachmentKey::node_alpha(key)),
            SlotId::Port((warp_id, port_of(node))),
            SlotId::Edge(own_edge(warp_id, node)),
            SlotId::OutgoingEdges(own_range(warp_id, node)),
        ] {
            expected_in.insert(slot);
            expected_out.insert(slot);
        }
        expected_in.insert(SlotId::Port((warp_id, port_of(node) + IN_ONLY_PORT_OFFSET)));
    }
    let expected_in: Vec<SlotId> = expected_in.into_iter().collect();
    let expected_out: Vec<SlotId> = expected_out.into_iter().collect();
    assert_eq!(patch.in_slots(), expected_in.as_slice());
    assert_eq!(patch.out_slots(), expected_out.as_slice());
    assert!(patch.in_slots().windows(2).all(|w| w[0] < w[1]));
    assert!(patch.out_slots().windows(2).all(|w| w[0] < w[1]));
    assert_eq!(patch.ops().len(), own.len());
}

const FAILING_RULE: &str = "test/overlap-fails";
const RETRY_RULE: &str = "test/overlap-retry";

fn retry_target() -> NodeId {
    make_node_id("test/overlap-retry-target")
}

fn failed_only() -> NodeId {
    make_node_id("test/overlap-failed-only")
}

fn writes(view: GraphView<'_>, nodes: &[NodeId]) -> Footprint {
    let mut footprint = Footprint::default();
    for node in nodes {
        let key = NodeKey {
            warp_id: view.warp_id(),
            local_id: *node,
        };
        footprint.n_read.insert(key);
        footprint.n_write.insert(key);
        footprint.a_read.insert(AttachmentKey::node_alpha(key));
        footprint.a_write.insert(AttachmentKey::node_alpha(key));
    }
    footprint
}

fn failing_rule() -> RewriteRule {
    RewriteRule {
        id: [8; 32],
        name: FAILING_RULE,
        left: PatternGraph { nodes: vec![] },
        matcher: |_, _| true,
        executor: RuleExecutor::legacy(|_, _, _| panic!("rule fails on purpose")),
        compute_footprint: |view, _| writes(view, &[retry_target(), failed_only()]),
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn retry_rule() -> RewriteRule {
    RewriteRule {
        id: [9; 32],
        name: RETRY_RULE,
        left: PatternGraph { nodes: vec![] },
        matcher: |_, _| true,
        executor: RuleExecutor::legacy(executor),
        compute_footprint: |view, _| writes(view, &[retry_target()]),
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// A commit whose rule panics closes its transaction: applying to it or
/// committing it again is refused with `UnknownTx`. A new transaction then
/// commits with only its own slots. (Conflict sets belong to one transaction,
/// so that last check holds on `main` too; what this test adds is the
/// refusal.)
#[test]
fn a_failed_commit_closes_its_transaction() {
    let mut store = GraphStore::default();
    let ty = make_type_id("test/overlap-node");
    for node in [retry_target(), failed_only()] {
        store.insert_node(node, NodeRecord { ty });
    }
    let mut engine = Engine::new(store, retry_target());
    engine
        .register_rule(failing_rule())
        .expect("rule registers");
    engine.register_rule(retry_rule()).expect("rule registers");
    let failed_tx = engine.begin();
    assert!(matches!(
        engine.apply(failed_tx, FAILING_RULE, &retry_target()),
        Ok(ApplyResult::Applied)
    ));
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine.commit_with_receipt(failed_tx)
    }));
    let payload = failed.expect_err("the failing rule must panic out of commit");
    assert_eq!(
        payload.downcast_ref::<&str>(),
        Some(&"rule fails on purpose"),
        "commit resumes the rule's own panic"
    );

    assert!(matches!(
        engine.apply(failed_tx, RETRY_RULE, &retry_target()),
        Err(EngineError::UnknownTx)
    ));
    assert!(matches!(
        engine.commit_with_receipt(failed_tx),
        Err(EngineError::UnknownTx)
    ));

    let tx = engine.begin();
    assert!(matches!(
        engine.apply(tx, RETRY_RULE, &retry_target()),
        Ok(ApplyResult::Applied)
    ));
    let (_, receipt, patch) = engine
        .commit_with_receipt(tx)
        .expect("a new transaction is judged on its own reservations");
    assert!(receipt
        .entries()
        .iter()
        .all(|entry| entry.disposition == TickReceiptDisposition::Applied));
    let key = NodeKey {
        warp_id: engine.current_root.warp_id,
        local_id: retry_target(),
    };
    let expected = [
        SlotId::Node(key),
        SlotId::Attachment(AttachmentKey::node_alpha(key)),
    ];
    assert_eq!(patch.in_slots(), expected.as_slice());
    assert_eq!(patch.out_slots(), expected.as_slice());
}

const ERROR_RULE: &str = "test/overlap-errors";

fn missing_node() -> NodeId {
    make_node_id("test/overlap-missing")
}

fn fresh_node() -> NodeId {
    make_node_id("test/overlap-fresh")
}

/// A rule whose ops cannot be applied: it deletes a node that does not
/// exist. The footprint declares the node, so the footprint guard passes and
/// commit reaches the error exit after reservation, not a panic.
fn erroring_rule() -> RewriteRule {
    RewriteRule {
        id: [10; 32],
        name: ERROR_RULE,
        left: PatternGraph { nodes: vec![] },
        matcher: |_, _| true,
        executor: RuleExecutor::legacy(|view, _, delta| {
            let warp_id = view.warp_id();
            // Canonical order puts the failing `DeleteNode` first, so this op
            // is never applied; the test does not cover an error that leaves
            // earlier ops applied (#957).
            delta.push(WarpOp::UpsertNode {
                node: NodeKey {
                    warp_id,
                    local_id: fresh_node(),
                },
                record: NodeRecord {
                    ty: make_type_id("test/overlap-node"),
                },
            });
            delta.push(WarpOp::DeleteNode {
                node: NodeKey {
                    warp_id,
                    local_id: missing_node(),
                },
            });
        }),
        compute_footprint: |view, _| writes(view, &[fresh_node(), missing_node()]),
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// The error path of a failed commit, as distinct from the panic path above:
/// commit returns `InternalCorruption` after reservation, the transaction is
/// closed, and a new transaction commits.
#[test]
fn an_errored_commit_closes_its_transaction() {
    let mut store = GraphStore::default();
    let ty = make_type_id("test/overlap-node");
    store.insert_node(retry_target(), NodeRecord { ty });
    let mut engine = Engine::new(store, retry_target());
    engine
        .register_rule(erroring_rule())
        .expect("rule registers");
    engine.register_rule(retry_rule()).expect("rule registers");

    let failed_tx = engine.begin();
    assert!(matches!(
        engine.apply(failed_tx, ERROR_RULE, &retry_target()),
        Ok(ApplyResult::Applied)
    ));
    assert!(matches!(
        engine.commit_with_receipt(failed_tx),
        Err(EngineError::InternalCorruption(
            "apply_reserved_rewrites: failed to apply ops"
        ))
    ));
    // Closing the transaction keeps the failed attempt's evidence readable:
    // it is a failure artifact, never history.
    assert!(
        !engine.last_aborted_execution_footprints().is_empty(),
        "a failed commit keeps its attempt's evidence readable"
    );
    assert!(matches!(
        engine.apply(failed_tx, RETRY_RULE, &retry_target()),
        Err(EngineError::UnknownTx)
    ));
    assert!(matches!(
        engine.commit_with_receipt(failed_tx),
        Err(EngineError::UnknownTx)
    ));

    let tx = engine.begin();
    assert!(matches!(
        engine.apply(tx, RETRY_RULE, &retry_target()),
        Ok(ApplyResult::Applied)
    ));
    engine
        .commit_with_receipt(tx)
        .expect("a new transaction commits after an errored one");
}

/// Records the scheduler's summary events.
#[derive(Default)]
struct Summaries(std::sync::Mutex<Vec<(TxId, u64, u64)>>);

impl crate::telemetry::TelemetrySink for Summaries {
    fn on_summary(&self, tx: TxId, reserved: u64, conflicts: u64) {
        if let Ok(mut seen) = self.0.lock() {
            seen.push((tx, reserved, conflicts));
        }
    }
}

/// Closing a failed commit finalizes its scheduler state, which reports the
/// transaction's summary once, as a committed transaction's does.
#[test]
fn a_failed_commit_reports_its_scheduler_summary() {
    let mut store = GraphStore::default();
    let ty = make_type_id("test/overlap-node");
    store.insert_node(retry_target(), NodeRecord { ty });
    let summaries = std::sync::Arc::new(Summaries::default());
    let mut engine = Engine::with_telemetry(
        store,
        retry_target(),
        SchedulerKind::Radix,
        0,
        summaries.clone(),
    );
    engine
        .register_rule(erroring_rule())
        .expect("rule registers");
    let tx = engine.begin();
    assert!(matches!(
        engine.apply(tx, ERROR_RULE, &retry_target()),
        Ok(ApplyResult::Applied)
    ));
    assert!(engine.commit_with_receipt(tx).is_err());
    let seen = summaries
        .0
        .lock()
        .map(|seen| seen.clone())
        .unwrap_or_default();
    assert_eq!(
        seen,
        vec![(tx, 1, 0)],
        "a failed commit reports its own summary once: one reserved, no conflicts"
    );
}

/// Panics while the scheduler reports a reservation, before the conflict
/// sets are taken for the patch.
struct PanicsOnReserve;

impl crate::telemetry::TelemetrySink for PanicsOnReserve {
    fn on_reserved(&self, _tx: TxId, _rule_id: &Hash) {
        panic!("telemetry fails on purpose");
    }
}

/// A commit that fails during reservation, before the conflict sets are
/// taken, also closes its transaction.
#[test]
fn a_commit_that_fails_during_reservation_closes_its_transaction() {
    let mut store = GraphStore::default();
    let ty = make_type_id("test/overlap-node");
    store.insert_node(retry_target(), NodeRecord { ty });
    let mut engine = Engine::with_telemetry(
        store,
        retry_target(),
        SchedulerKind::Radix,
        0,
        std::sync::Arc::new(PanicsOnReserve),
    );
    engine.register_rule(retry_rule()).expect("rule registers");
    let tx = engine.begin();
    assert!(matches!(
        engine.apply(tx, RETRY_RULE, &retry_target()),
        Ok(ApplyResult::Applied)
    ));
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine.commit_with_receipt(tx)
    }));
    assert!(failed.is_err(), "the sink's panic leaves commit");
    assert!(matches!(
        engine.apply(tx, RETRY_RULE, &retry_target()),
        Err(EngineError::UnknownTx)
    ));
}
