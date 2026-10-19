// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)] // Test assertions may abort the witness.
//! Native scheduler witness for declared range claims, not an Edict bridge.
use super::*;
use crate::{EdgeKey, EdgeRecord, Footprint, OutgoingEdgeRange, PatternGraph, TickDelta};

const OBSERVE: &str = "test/range-observe";
const INSERT: &str = "test/range-insert";

fn range(view: GraphView<'_>, scope: &NodeId) -> OutgoingEdgeRange {
    OutgoingEdgeRange {
        source: NodeKey {
            warp_id: view.warp_id(),
            local_id: *scope,
        },
        edge_type: make_type_id("test/range-type"),
    }
}

fn observe_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut footprint = Footprint::default();
    footprint.r_read.insert(range(view, scope));
    footprint
}

fn insert_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut footprint = Footprint::default();
    footprint.r_write.insert(range(view, scope));
    footprint.n_write.insert(NodeKey {
        warp_id: view.warp_id(),
        local_id: *scope,
    });
    footprint.e_write.insert(EdgeKey {
        warp_id: view.warp_id(),
        local_id: make_edge_id("test/range-edge"),
    });
    footprint
}

// The native fixture supplies a conservative range claim. It does not test
// interpreted read instrumentation or the broader node-granted adjacency API.
fn observe(_: GraphView<'_>, _: &NodeId, _: &mut TickDelta) {}

fn insert(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    delta.push(WarpOp::UpsertEdge {
        warp_id: view.warp_id(),
        record: EdgeRecord {
            id: make_edge_id("test/range-edge"),
            from: *scope,
            to: make_node_id("test/range-target"),
            ty: make_type_id("test/range-type"),
        },
    });
}

fn rule(
    id: Hash,
    name: &'static str,
    footprint: fn(GraphView<'_>, &NodeId) -> Footprint,
    executor: fn(GraphView<'_>, &NodeId, &mut TickDelta),
) -> RewriteRule {
    RewriteRule {
        id,
        name,
        left: PatternGraph { nodes: vec![] },
        matcher: |_, _| true,
        executor: RuleExecutor::legacy(executor),
        compute_footprint: footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

#[test]
fn a_real_tick_rejects_insertion_against_an_empty_range_and_retains_both_roles() {
    let source = make_node_id("test/range-source");
    let target = make_node_id("test/range-target");
    let mut store = GraphStore::default();
    for node in [source, target] {
        store.insert_node(
            node,
            NodeRecord {
                ty: make_type_id("test/range-node"),
            },
        );
    }
    assert_eq!(store.edges_from(&source).count(), 0);
    let mut engine = Engine::new(store, source);
    let key = OutgoingEdgeRange {
        source: engine.current_root,
        edge_type: make_type_id("test/range-type"),
    };
    let mut ids = [[1; 32], [2; 32]];
    ids.sort_by_key(|id| scope_hash(id, &key.source));
    engine
        .register_rule(rule(ids[0], OBSERVE, observe_footprint, observe))
        .expect("reader");
    engine
        .register_rule(rule(ids[1], INSERT, insert_footprint, insert))
        .expect("writer");
    let initial = engine.state.clone();
    let tx = engine.begin();
    assert!(matches!(
        engine.apply(tx, INSERT, &source),
        Ok(ApplyResult::Applied)
    ));
    assert!(matches!(
        engine.apply(tx, OBSERVE, &source),
        Ok(ApplyResult::Applied)
    ));
    let (_, receipt, first) = engine.commit_with_receipt(tx).expect("first Tick");
    assert_eq!(receipt.entries().len(), 2);
    assert_eq!(receipt.entries()[0].rule_id, ids[0]);
    assert_eq!(
        receipt.entries()[0].disposition,
        TickReceiptDisposition::Applied
    );
    assert_eq!(
        receipt.entries()[1].disposition,
        TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict)
    );
    assert_eq!(receipt.blocked_by(1), &[0]);
    let slot = SlotId::OutgoingEdges(key);
    assert!(first.in_slots().contains(&slot));
    assert!(!first.out_slots().contains(&slot));
    assert!(first.ops().is_empty());
    let tx = engine.begin();
    assert!(matches!(
        engine.apply(tx, INSERT, &source),
        Ok(ApplyResult::Applied)
    ));
    let (_, receipt, second) = engine.commit_with_receipt(tx).expect("second Tick");
    assert_eq!(
        receipt.entries()[0].disposition,
        TickReceiptDisposition::Applied
    );
    assert!(second.in_slots().contains(&slot));
    assert!(second.out_slots().contains(&slot));
    let mut replay = initial;
    first.apply_to_state(&mut replay).expect("first replay");
    second.apply_to_state(&mut replay).expect("second replay");
    assert_eq!(
        compute_state_root(&replay, &key.source),
        compute_state_root(&engine.state, &key.source)
    );
}
