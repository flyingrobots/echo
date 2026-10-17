// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// criterion_group!/criterion_main! expand to undocumented functions that cannot
// carry #[allow] (attributes on macro invocations are ignored). Crate-level
// suppress is required for benchmark binaries using Criterion.
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]
//! Commit time for one edge change on a high-degree node.
//!
//! The allocation witness (`echo-alloc-witness`, `commit_hub_bucket`) proves a
//! commit does not copy the hub's adjacency bucket. This measures the time
//! side, at two hub degrees. Adding an edge pushes onto the bucket; deleting
//! one finds and removes it from a `Vec`, which is O(degree). A time ratio
//! near 1 between degrees means the commit is independent of bucket size; a
//! ratio that tracks degree marks the remaining O(degree) step.
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use echo_dry_tests::build_motion_demo_engine;
use warp_core::{
    make_edge_id, make_node_id, make_type_id, make_warp_id, ConflictPolicy, EdgeId, EdgeRecord,
    Engine, ExecutionGraphView, Footprint, GraphView, Hash, NodeId, NodeRecord, PatternGraph,
    RewriteRule, RuleExecutor, TickDelta, WarpOp,
};

const ADD: &str = "bench/hub/add-edge";
const DELETE: &str = "bench/hub/delete-edge";
const DEGREES: [usize; 2] = [1_000, 16_000];

fn hub() -> NodeId {
    make_node_id("bench/hub")
}

fn link() -> warp_core::TypeId {
    make_type_id("bench/hub/link")
}

fn new_edge() -> EdgeId {
    make_edge_id("bench/hub/new-edge")
}

/// The hub's first edge, which sits at the front of its bucket, so removing
/// it shifts every other edge.
fn first_edge() -> EdgeId {
    make_edge_id("bench/hub/edge/0")
}

fn rule(
    name: &'static str,
    executor: fn(&mut ExecutionGraphView<'_, '_>, &NodeId, &mut TickDelta),
) -> RewriteRule {
    fn matcher(_view: GraphView<'_>, _n: &NodeId) -> bool {
        true
    }
    fn footprint(view: GraphView<'_>, _n: &NodeId) -> Footprint {
        let mut footprint = Footprint::default();
        footprint.n_write.insert_with_warp(view.warp_id(), hub());
        footprint
            .e_write
            .insert_with_warp(view.warp_id(), new_edge());
        footprint
            .e_write
            .insert_with_warp(view.warp_id(), first_edge());
        footprint
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

fn add_edge(view: &mut ExecutionGraphView<'_, '_>, _n: &NodeId, delta: &mut TickDelta) {
    delta.emit(WarpOp::UpsertEdge {
        warp_id: view.warp_id(),
        record: EdgeRecord {
            id: new_edge(),
            from: hub(),
            to: hub(),
            ty: link(),
        },
    });
}

fn delete_edge(view: &mut ExecutionGraphView<'_, '_>, _n: &NodeId, delta: &mut TickDelta) {
    delta.emit(WarpOp::DeleteEdge {
        warp_id: view.warp_id(),
        from: hub(),
        edge_id: first_edge(),
    });
}

/// An engine whose hub already has `degree` outbound edges.
fn engine_with_hub(degree: usize) -> Engine {
    let mut engine = build_motion_demo_engine();
    engine.register_rule(rule(ADD, add_edge)).expect("add rule");
    engine
        .register_rule(rule(DELETE, delete_edge))
        .expect("delete rule");
    let ty = make_type_id("bench/hub/entity");
    engine.insert_node(hub(), NodeRecord { ty }).expect("hub");
    let store = engine
        .state_mut()
        .store_mut(&make_warp_id("root"))
        .expect("root store");
    for i in 0..degree {
        let leaf = make_node_id(&format!("bench/hub/leaf/{i}"));
        store.insert_node(leaf, NodeRecord { ty });
        store.insert_edge(
            hub(),
            EdgeRecord {
                id: make_edge_id(&format!("bench/hub/edge/{i}")),
                from: hub(),
                to: leaf,
                ty: link(),
            },
        );
    }
    engine
}

fn bench_hub_commit(c: &mut Criterion) {
    let mut group = c.benchmark_group("commit_hub_bucket");
    for (op, rule_name) in [("add", ADD), ("delete", DELETE)] {
        for degree in DEGREES {
            group.bench_with_input(BenchmarkId::new(op, degree), &degree, |b, &degree| {
                b.iter_batched(
                    || {
                        let mut engine = engine_with_hub(degree);
                        let tx = engine.begin();
                        engine.apply(tx, rule_name, &hub()).expect("rule applies");
                        (engine, tx)
                    },
                    |(mut engine, tx)| {
                        let snapshot = engine.commit(tx).expect("tick commits");
                        black_box(snapshot);
                        engine
                    },
                    BatchSize::LargeInput,
                );
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_hub_commit);
criterion_main!(benches);
