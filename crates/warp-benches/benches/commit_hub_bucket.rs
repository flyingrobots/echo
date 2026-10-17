// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// criterion_group!/criterion_main! expand to undocumented functions that cannot
// carry #[allow] (attributes on macro invocations are ignored). Crate-level
// suppress is required for benchmark binaries using Criterion.
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]
//! Time for one edge change on a high-degree node.
//!
//! The allocation witness (`echo-alloc-witness`, `commit_hub_bucket`) proves a
//! commit does not copy the hub's adjacency bucket. This measures time.
//!
//! Every case has the same graph: `LEAVES` leaves, the hub, and a counterweight
//! node, with exactly `LEAVES` edges from exactly two sources. The hub has `d`
//! of them and the counterweight the other `LEAVES - d`. Nodes, edges, and
//! non-empty buckets are all constant, so any cost that follows the graph's
//! size or shape, such as hashing the state root, is the same in every case;
//! a difference between cases is the hub's degree.
//!
//! Each timed iteration gets a freshly built store rather than a clone, so
//! buckets keep the capacity they grew to, as they do in a running engine
//! (a clone's buckets have no spare capacity, which would make every push
//! copy its bucket).
//!
//! Two groups:
//! - `hub_bucket`: the bucket operation alone on a `GraphStore`, with no
//!   commit. Adding pushes onto the bucket; deleting the first edge finds and
//!   removes it from a `Vec`, which is O(degree).
//! - `commit_hub_bucket`: a whole commit making that one change.
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use echo_dry_tests::build_motion_demo_engine;
use warp_core::{
    make_edge_id, make_node_id, make_type_id, make_warp_id, ConflictPolicy, EdgeId, EdgeRecord,
    Engine, ExecutionGraphView, Footprint, GraphStore, GraphView, Hash, NodeId, NodeRecord,
    PatternGraph, RewriteRule, RuleExecutor, TickDelta, WarpOp,
};

const ADD: &str = "bench/hub/add-edge";
const DELETE: &str = "bench/hub/delete-edge";
/// Leaf nodes, and total edges, in every case.
const LEAVES: usize = 16_000;
/// Hub degrees measured.
const DEGREES: [usize; 3] = [16, 1_000, LEAVES - 16];

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

fn counterweight() -> NodeId {
    make_node_id("bench/hub/counterweight")
}

fn leaf(i: usize) -> NodeId {
    make_node_id(&format!("bench/hub/leaf/{i}"))
}

/// Fills `store` with the hub, the counterweight, `LEAVES` leaves, and
/// `LEAVES` edges: `degree` from the hub and the rest from the counterweight.
fn fill(store: &mut GraphStore, degree: usize) {
    let ty = make_type_id("bench/hub/entity");
    store.insert_node(hub(), NodeRecord { ty });
    store.insert_node(counterweight(), NodeRecord { ty });
    for i in 0..LEAVES {
        store.insert_node(leaf(i), NodeRecord { ty });
    }
    for i in 0..LEAVES {
        let from = if i < degree { hub() } else { counterweight() };
        let to = leaf(i);
        store.insert_edge(
            from,
            EdgeRecord {
                id: make_edge_id(&format!("bench/hub/edge/{i}")),
                from,
                to,
                ty: link(),
            },
        );
    }
}

/// An engine whose root store is [`fill`]ed for `degree`.
fn engine_with_hub(degree: usize) -> Engine {
    let mut engine = build_motion_demo_engine();
    engine.register_rule(rule(ADD, add_edge)).expect("add rule");
    engine
        .register_rule(rule(DELETE, delete_edge))
        .expect("delete rule");
    let store = engine
        .state_mut()
        .store_mut(&make_warp_id("root"))
        .expect("root store");
    fill(store, degree);
    engine
}

fn bench_hub_bucket(c: &mut Criterion) {
    let mut group = c.benchmark_group("hub_bucket");
    for degree in DEGREES {
        let built = move || {
            let mut store = GraphStore::default();
            fill(&mut store, degree);
            store
        };
        group.bench_function(BenchmarkId::new("add", degree), |b| {
            b.iter_batched(
                built,
                |mut store| {
                    store.insert_edge(
                        hub(),
                        EdgeRecord {
                            id: new_edge(),
                            from: hub(),
                            to: hub(),
                            ty: link(),
                        },
                    );
                    store
                },
                BatchSize::LargeInput,
            );
        });
        group.bench_function(BenchmarkId::new("delete", degree), |b| {
            b.iter_batched(
                built,
                |mut store| {
                    black_box(store.delete_edge_exact(hub(), first_edge()));
                    store
                },
                BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

fn bench_hub_commit(c: &mut Criterion) {
    let mut group = c.benchmark_group("commit_hub_bucket");
    for (op, rule_name) in [("add", ADD), ("delete", DELETE)] {
        for degree in DEGREES {
            group.bench_function(BenchmarkId::new(op, degree), |b| {
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

criterion_group!(benches, bench_hub_bucket, bench_hub_commit);
criterion_main!(benches);
