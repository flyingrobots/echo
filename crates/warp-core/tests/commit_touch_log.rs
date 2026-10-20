// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A commit's patch is computed from the slots it touches. A commit that
//! aborts must not leave that record running into the next commit's patch.
#![cfg(feature = "native_rule_bootstrap")]
#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use echo_dry_tests::build_motion_demo_engine;
use warp_core::{
    make_node_id, make_type_id, ConflictPolicy, ExecutionGraphView, Footprint, GraphView, Hash,
    NodeId, NodeRecord, PatternGraph, RewriteRule, RuleExecutor, TickDelta, WarpOp,
};

fn rule(
    name: &'static str,
    executor: fn(&mut ExecutionGraphView<'_, '_>, &NodeId, &mut TickDelta),
) -> RewriteRule {
    fn matcher(_view: GraphView<'_>, _n: &NodeId) -> bool {
        true
    }
    fn footprint(_view: GraphView<'_>, _n: &NodeId) -> Footprint {
        Footprint::default()
    }
    let id: Hash = blake3::hash(name.as_bytes()).into();
    RewriteRule {
        id,
        name,
        left: PatternGraph { nodes: vec![] },
        matcher,
        executor: RuleExecutor::observed(executor),
        compute_footprint: footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn panicking(_view: &mut ExecutionGraphView<'_, '_>, _n: &NodeId, _delta: &mut TickDelta) {
    panic!("commit-touch-log: executor aborts this commit");
}

fn noop(_view: &mut ExecutionGraphView<'_, '_>, _n: &NodeId, _delta: &mut TickDelta) {}

#[test]
fn an_aborted_commit_does_not_leak_writes_into_the_next_patch() {
    let mut engine = build_motion_demo_engine();
    engine
        .register_rule(rule("commit-touch-log/panic", panicking))
        .expect("rule registers");
    engine
        .register_rule(rule("commit-touch-log/noop", noop))
        .expect("rule registers");
    let ty = make_type_id("entity");
    let scope = make_node_id("commit-touch-log/scope");
    engine
        .insert_node(scope, NodeRecord { ty })
        .expect("node inserts");

    // A commit whose executor panics; the caller catches it and continues.
    let tx = engine.begin();
    engine
        .apply(tx, "commit-touch-log/panic", &scope)
        .expect("rule applies");
    let aborted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine.commit_with_receipt(tx)
    }));
    assert!(
        matches!(aborted, Err(_) | Ok(Err(_))),
        "the executor panic aborts the commit"
    );

    // A write between commits, outside any tick.
    let between = make_node_id("commit-touch-log/between");
    engine
        .insert_node(between, NodeRecord { ty })
        .expect("node inserts");

    // The next commit changes nothing, so its patch must be empty: the write
    // above belongs to no tick.
    let tx = engine.begin();
    engine
        .apply(tx, "commit-touch-log/noop", &scope)
        .expect("rule applies");
    let (_, _, patch) = engine.commit_with_receipt(tx).expect("tick commits");
    assert!(
        !patch.ops().iter().any(|op| matches!(
            op,
            WarpOp::UpsertNode { node, .. } if node.local_id == between
        )),
        "the patch carries a write from outside its tick: {:?}",
        patch.ops()
    );
    assert!(patch.ops().is_empty(), "unexpected ops: {:?}", patch.ops());
}
