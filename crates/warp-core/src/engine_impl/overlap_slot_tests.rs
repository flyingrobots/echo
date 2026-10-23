// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)] // Test assertions may abort the witness; one rule panics on purpose.
//! Reservation reads a tick's slots back from the scheduler's conflict sets.
//! This witness commits rewrites whose footprints overlap, both across
//! rewrites (shared reads) and within one footprint (a slot that is read and
//! written), and checks that the committed patch carries exactly the sorted,
//! repeat-free union that the former per-footprint `BTreeSet` collection
//! produced.
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
        ] {
            expected_in.insert(slot);
            expected_out.insert(slot);
        }
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

/// A commit whose rule panics closes its transaction. Applying to it or
/// committing it again is refused with `UnknownTx`, and a new transaction is
/// judged only on its own reservations: the failed attempt leaves neither
/// conflicts nor slots behind.
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
    assert!(failed.is_err(), "the failing rule must panic out of commit");

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
