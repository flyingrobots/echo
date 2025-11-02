#![allow(missing_docs)]
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rmg_core::{
    make_edge_id, make_node_id, make_type_id, EdgeRecord, Engine, GraphStore, NodeRecord,
};

fn build_chain_engine(n: usize) -> Engine {
    let mut store = GraphStore::default();
    let root = make_node_id("root");
    let world = make_type_id("world");
    store.insert_node(
        root,
        NodeRecord {
            ty: world,
            payload: None,
        },
    );
    // Insert N nodes and connect them in a chain so all are reachable.
    let entity_ty = make_type_id("entity");
    let link_ty = make_type_id("link");
    let mut prev = root;
    for i in 0..n {
        let id = make_node_id(&format!("ent-{}", i));
        store.insert_node(
            id,
            NodeRecord {
                ty: entity_ty,
                payload: None,
            },
        );
        let edge_id = make_edge_id(&format!("e-{}-{}", i, i + 1));
        store.insert_edge(
            prev,
            EdgeRecord {
                id: edge_id,
                from: prev,
                to: id,
                ty: link_ty,
                payload: None,
            },
        );
        prev = id;
    }
    Engine::new(store, root)
}

fn bench_snapshot_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_hash");
    for &n in &[10usize, 100, 1_000] {
        let engine = build_chain_engine(n);
        // Throughput: number of nodes included in the reachable set.
        group.throughput(Throughput::Elements(n as u64 + 1)); // +1 for root
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &_n| {
            b.iter(|| {
                // Hash the reachable graph (engine.snapshot performs state_root hashing).
                let snap = engine.snapshot();
                criterion::black_box(snap.hash);
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_snapshot_hash);
criterion_main!(benches);
