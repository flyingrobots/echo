#![allow(missing_docs)]
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use rmg_core::{
    decode_motion_payload, encode_motion_payload, make_node_id, make_type_id, ApplyResult, Engine,
    NodeId, NodeRecord, MOTION_RULE_NAME,
};
use std::{hint::black_box, time::Duration};

fn build_engine_with_n_entities(n: usize) -> (Engine, Vec<NodeId>) {
    // Start from the demo engine (root + motion rule registered).
    let mut engine = rmg_core::build_motion_demo_engine();
    let ty = make_type_id("entity");
    let mut ids = Vec::with_capacity(n);
    // Insert N entities with a simple payload.
    for i in 0..n {
        let label = format!("ent-{}", i);
        // Precompute NodeId so hashing is not part of the hot loop.
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
        ids.push(id);
    }
    (engine, ids)
}

fn bench_motion_apply(c: &mut Criterion) {
    let mut group = c.benchmark_group("motion_apply");
    // Stabilize measurements: fixed warmup and sample size.
    group.sample_size(50);
    group.warm_up_time(Duration::from_secs(3));
    for &n in &[1usize, 10, 100, 1_000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let (engine, ids) = build_engine_with_n_entities(n);
                    (engine, ids)
                },
                |(mut engine, ids)| {
                    // Apply motion once to each entity.
                    let tx = engine.begin();
                    for id in &ids {
                        let res = engine.apply(tx, MOTION_RULE_NAME, id).expect("apply");
                        // Avoid penalizing release runs.
                        debug_assert!(matches!(res, ApplyResult::Applied | ApplyResult::NoMatch));
                    }
                    engine.commit(tx).expect("commit");

                    // Decode and validate the first entity's payload to keep work observable,
                    // then black_box to prevent the optimizer from eliminating it.
                    let first = ids[0];
                    let node = engine.node(&first).expect("node exists");
                    let decoded = decode_motion_payload(node.payload.as_ref().expect("payload"))
                        .expect("decode");
                    debug_assert!(decoded.0.iter().all(|v| v.is_finite()));
                    debug_assert!(decoded.1.iter().all(|v| v.is_finite()));
                    black_box(decoded);
                },
                BatchSize::PerIteration,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench_motion_apply);
criterion_main!(benches);
