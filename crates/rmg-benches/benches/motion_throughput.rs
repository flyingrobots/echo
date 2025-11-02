#![allow(missing_docs)]
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use rmg_core::{
    decode_motion_payload, encode_motion_payload, make_node_id, make_type_id, ApplyResult, Engine,
    NodeRecord, MOTION_RULE_NAME,
};

fn build_engine_with_n_entities(n: usize) -> (Engine, Vec<String>) {
    // Start from the demo engine (root + motion rule registered).
    let mut engine = rmg_core::build_motion_demo_engine();
    let ty = make_type_id("entity");
    let mut labels = Vec::with_capacity(n);
    // Insert N entities with a simple payload.
    for i in 0..n {
        let label = format!("ent-{}", i);
        let id = make_node_id(&label);
        let pos = [i as f32, 0.0, 0.0];
        let vel = [1.0, 0.0, 0.0];
        let payload = encode_motion_payload(pos, vel);
        engine.insert_node(
            id,
            NodeRecord {
                ty,
                payload: Some(payload),
            },
        );
        labels.push(label);
    }
    (engine, labels)
}

fn bench_motion_apply(c: &mut Criterion) {
    let mut group = c.benchmark_group("motion_apply");
    for &n in &[1usize, 10, 100, 1_000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let (engine, labels) = build_engine_with_n_entities(n);
                    (engine, labels)
                },
                |(mut engine, labels)| {
                    // Apply motion once to each entity.
                    let tx = engine.begin();
                    for label in &labels {
                        let id = make_node_id(label);
                        let res = engine.apply(tx, MOTION_RULE_NAME, &id).expect("apply");
                        assert!(matches!(res, ApplyResult::Applied | ApplyResult::NoMatch));
                    }
                    engine.commit(tx).expect("commit");

                    // Quick decode sanity for the first entity to keep benchmark honest.
                    let first = make_node_id(&labels[0]);
                    let node = engine.node(&first).expect("node exists");
                    let _ = decode_motion_payload(node.payload.as_ref().expect("payload"));
                },
                BatchSize::PerIteration,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench_motion_apply);
criterion_main!(benches);
