// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(missing_docs)]
//! Benchmark: snapshot hash over a linear chain graph
//!
//! Builds a chain of `n` entities reachable from a single root node and
//! measures the cost of computing the snapshot (state_root) hash over the
//! reachable subgraph. Sizes (10, 100, 1000) provide an order-of-magnitude
//! progression to observe scaling trends without long runtimes.
//!
//! Throughput "elements" are the number of nodes in the reachable set
//! (n entities + 1 root).
//!
//! TODO(PR-14/15): Persist JSON artifacts and add a regression gate.
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use warp_core::{
    make_edge_id, make_node_id, make_type_id, EdgeRecord, Engine, GraphStore, NodeRecord,
};

// String constants to avoid magic literals drifting silently.
const ROOT_ID_STR: &str = "root";
const WORLD_TYPE_STR: &str = "world";
const ENTITY_TYPE_STR: &str = "entity";
const LINK_TYPE_STR: &str = "link";
const ENT_LABEL_PREFIX: &str = "ent-";

fn build_chain_engine(n: usize) -> Engine {
    let mut store = GraphStore::default();
    let root = make_node_id(ROOT_ID_STR);
    let world = make_type_id(WORLD_TYPE_STR);
    store.insert_node(root, NodeRecord { ty: world });
    // Insert N nodes and connect them in a chain so all are reachable.
    let entity_ty = make_type_id(ENTITY_TYPE_STR);
    let link_ty = make_type_id(LINK_TYPE_STR);
    let mut chain_tail = root;
    for i in 0..n {
        let to_label = format!("{}{}", ENT_LABEL_PREFIX, i);
        let id = make_node_id(&to_label);
        store.insert_node(id, NodeRecord { ty: entity_ty });
        // Human-friendly edge id: <from>-to-<to>.
        let from_label = if i == 0 {
            ROOT_ID_STR.to_string()
        } else {
            format!("{}{}", ENT_LABEL_PREFIX, i - 1)
        };
        let edge_id = make_edge_id(&format!("edge-{}-to-{}", from_label, to_label));
        store.insert_edge(
            chain_tail,
            EdgeRecord {
                id: edge_id,
                from: chain_tail,
                to: id,
                ty: link_ty,
            },
        );
        chain_tail = id;
    }
    Engine::new(store, root)
}

fn bench_snapshot_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_hash");
    // Stabilize CI runs across environments.
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(80);
    for &n in &[10usize, 100, 1_000, 3_000, 10_000, 30_000] {
        // Throughput: total nodes in reachable set (n entities + 1 root).
        group.throughput(Throughput::Elements(n as u64 + 1));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            // Build engine in setup (not timed) and measure only hashing.
            b.iter_batched(
                || build_chain_engine(n),
                |engine| {
                    let snap = engine.snapshot();
                    criterion::black_box(snap.hash);
                },
                BatchSize::PerIteration,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench_snapshot_hash);
criterion_main!(benches);
