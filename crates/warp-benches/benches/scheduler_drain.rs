// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(missing_docs)]
//! Benchmark: scheduler drain throughput with a no-op rule
//!
//! Applies a trivial no-op rule across `n` entity nodes to measure scheduler
//! overhead rather than executor work. Construction happens in the setup phase;
//! measurement covers applying the rule to each node and committing a tx.
//!
//! Throughput "elements" are rule applications (`n`).
//! BatchSize::PerIteration ensures engine construction is excluded from timing.
//!
//! TODO(PR-14/15): Persist JSON artifacts and add a regression gate.
use blake3::Hasher;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use warp_core::{
    make_node_id, make_type_id, ApplyResult, ConflictPolicy, Engine, Footprint, Hash, NodeId,
    NodeRecord, PatternGraph, RewriteRule,
};

// Bench constants to avoid magic strings.
const BENCH_NOOP_RULE_NAME: &str = "bench/noop";
const RULE_ID_PREFIX: &[u8] = b"rule:";
const ENTITY_TYPE_STR: &str = "entity";
const ENT_LABEL_PREFIX: &str = "sched-ent-";

fn bench_noop_rule() -> RewriteRule {
    // Deterministic rule id: blake3("rule:" ++ name)
    let id: Hash = {
        let mut h = Hasher::new();
        h.update(RULE_ID_PREFIX);
        h.update(BENCH_NOOP_RULE_NAME.as_bytes());
        h.finalize().into()
    };
    fn matcher(_s: &warp_core::GraphStore, _n: &warp_core::NodeId) -> bool {
        true
    }
    fn executor(_s: &mut warp_core::GraphStore, _n: &warp_core::NodeId) {}
    fn footprint(_s: &warp_core::GraphStore, _n: &warp_core::NodeId) -> Footprint {
        Footprint::default()
    }
    RewriteRule {
        id,
        name: BENCH_NOOP_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher,
        executor,
        compute_footprint: footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn build_engine_with_entities(n: usize) -> (Engine, Vec<NodeId>) {
    let mut engine = warp_core::build_motion_demo_engine();
    // Register a no-op rule to isolate scheduler overhead from executor work.
    engine
        .register_rule(bench_noop_rule())
        .expect("Failed to register benchmark noop rule");

    let ty = make_type_id(ENTITY_TYPE_STR);
    let mut ids = Vec::with_capacity(n);
    for i in 0..n {
        let label = format!("{}{}", ENT_LABEL_PREFIX, i);
        let id = make_node_id(&label);
        engine.insert_node(id, NodeRecord { ty });
        ids.push(id);
    }
    (engine, ids)
}

fn bench_scheduler_drain(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_drain");
    // Stabilize CI runs: explicit warmup/measurement and sample size.
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(60);
    for &n in &[10usize, 100, 1_000, 3_000, 10_000, 30_000] {
        // Throughput: number of rule applications in this run (n entities).
        group.throughput(Throughput::Elements(n as u64));

        // Full apply+commit cycle (original benchmark)
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || build_engine_with_entities(n),
                |(mut engine, ids)| {
                    // Apply the no-op rule to all entities, then commit.
                    let tx = engine.begin();
                    for id in &ids {
                        let res = engine.apply(tx, BENCH_NOOP_RULE_NAME, id).unwrap();
                        // Avoid affecting timing; check only in debug builds.
                        debug_assert!(matches!(res, ApplyResult::Applied));
                    }
                    let snap = engine.commit(tx).unwrap();
                    // Ensure the commit work is not optimized away.
                    criterion::black_box(snap);
                },
                BatchSize::PerIteration,
            )
        });

        // Enqueue phase only (apply without commit)
        group.bench_function(BenchmarkId::new("enqueue", n), |b| {
            b.iter_batched(
                || build_engine_with_entities(n),
                |(mut engine, ids)| {
                    let tx = engine.begin();
                    for id in &ids {
                        let res = engine.apply(tx, BENCH_NOOP_RULE_NAME, id).unwrap();
                        debug_assert!(matches!(res, ApplyResult::Applied));
                    }
                    criterion::black_box(tx);
                },
                BatchSize::PerIteration,
            )
        });

        // Drain phase only (commit with pre-enqueued rewrites)
        group.bench_function(BenchmarkId::new("drain", n), |b| {
            b.iter_batched(
                || {
                    let (mut engine, ids) = build_engine_with_entities(n);
                    let tx = engine.begin();
                    // Pre-enqueue all rewrites (not timed)
                    for id in &ids {
                        let _ = engine.apply(tx, BENCH_NOOP_RULE_NAME, id).unwrap();
                    }
                    (engine, tx)
                },
                |(mut engine, tx)| {
                    // Only measure the commit (drain + execute)
                    let snap = engine.commit(tx).unwrap();
                    criterion::black_box(snap);
                },
                BatchSize::PerIteration,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench_scheduler_drain);
criterion_main!(benches);
