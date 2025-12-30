// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(missing_docs)]
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use std::{hint::black_box, time::Duration};
use warp_core::{
    decode_motion_atom_payload, encode_motion_atom_payload, make_node_id, make_type_id,
    ApplyResult, Engine, NodeId, NodeRecord, MOTION_RULE_NAME,
};

fn build_engine_with_n_entities(n: usize) -> (Engine, Vec<NodeId>) {
    // Start from the demo engine (root + motion rule registered).
    let mut engine = warp_core::build_motion_demo_engine();
    let ty = make_type_id("entity");
    let mut ids = Vec::with_capacity(n);
    // Insert N entities with a simple payload.
    for i in 0..n {
        let label = format!("ent-{}", i);
        // Precompute NodeId so hashing is not part of the hot loop.
        let id = make_node_id(&label);
        let pos = [i as f32, 0.0, 0.0];
        let vel = [1.0, 0.0, 0.0];
        let payload = encode_motion_atom_payload(pos, vel);
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
    // Bench 1: Build-only — measures engine construction + inserts.
    let mut build_group = c.benchmark_group("motion_build_only");
    build_group.sample_size(50);
    build_group.warm_up_time(Duration::from_secs(3));
    build_group.measurement_time(Duration::from_secs(6));
    build_group.noise_threshold(0.02);
    for &n in &[1usize, 10, 100, 1_000] {
        build_group.throughput(Throughput::Elements(n as u64));
        build_group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            // Measure just the build; keep work observable via black_box.
            b.iter(|| {
                let (engine, ids) = build_engine_with_n_entities(n);
                // Optional quick sanity on the first entity to keep side effects visible.
                let node = engine.node(&ids[0]).expect("node exists");
                let decoded = decode_motion_atom_payload(node.payload.as_ref().expect("payload"))
                    .expect("decode");
                debug_assert!(decoded.0.iter().all(|v| v.is_finite()));
                debug_assert!(decoded.1.iter().all(|v| v.is_finite()));
                black_box(engine);
                black_box(ids);
                black_box(decoded);
            })
        });
    }
    build_group.finish();

    // Bench 2: Apply+Commit — measure only the rewrite/commit path.
    let mut apply_group = c.benchmark_group("motion_apply_commit");
    apply_group.sample_size(50);
    apply_group.warm_up_time(Duration::from_secs(3));
    apply_group.measurement_time(Duration::from_secs(6));
    apply_group.noise_threshold(0.02);
    for &n in &[1usize, 10, 100, 1_000] {
        apply_group.throughput(Throughput::Elements(n as u64));
        apply_group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            // Build a fresh engine/ids in setup (not timed), measure only the apply/commit work.
            b.iter_batched(
                || build_engine_with_n_entities(n),
                |(mut engine, ids)| {
                    let tx = engine.begin();
                    for id in &ids {
                        let res = engine.apply(tx, MOTION_RULE_NAME, id).expect("apply");
                        debug_assert!(matches!(res, ApplyResult::Applied | ApplyResult::NoMatch));
                    }
                    engine.commit(tx).expect("commit");

                    // Decode and validate the first entity's payload and black_box the result.
                    let node = engine.node(&ids[0]).expect("node exists");
                    let decoded =
                        decode_motion_atom_payload(node.payload.as_ref().expect("payload"))
                            .expect("decode");
                    debug_assert!(decoded.0.iter().all(|v| v.is_finite()));
                    debug_assert!(decoded.1.iter().all(|v| v.is_finite()));
                    black_box(decoded);
                },
                BatchSize::PerIteration,
            )
        });
    }
    apply_group.finish();
}

criterion_group!(benches, bench_motion_apply);
criterion_main!(benches);
