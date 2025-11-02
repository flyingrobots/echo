#![allow(missing_docs)]
use blake3::Hasher;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use rmg_core::{
    make_node_id, make_type_id, ApplyResult, ConflictPolicy, Engine, Footprint, Hash, NodeRecord,
    PatternGraph, RewriteRule,
};

fn bench_noop_rule() -> RewriteRule {
    const NAME: &str = "bench/noop";
    // Deterministic rule id: blake3("rule:" ++ name)
    let id: Hash = {
        let mut h = Hasher::new();
        h.update(b"rule:");
        h.update(NAME.as_bytes());
        h.finalize().into()
    };
    fn matcher(_s: &rmg_core::GraphStore, _n: &rmg_core::NodeId) -> bool {
        true
    }
    fn executor(_s: &mut rmg_core::GraphStore, _n: &rmg_core::NodeId) {}
    fn footprint(_s: &rmg_core::GraphStore, _n: &rmg_core::NodeId) -> Footprint {
        Footprint::default()
    }
    RewriteRule {
        id,
        name: NAME,
        left: PatternGraph { nodes: vec![] },
        matcher,
        executor,
        compute_footprint: footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn build_engine_with_entities(n: usize) -> (Engine, Vec<String>) {
    let mut engine = rmg_core::build_motion_demo_engine();
    // Register a no-op rule to isolate scheduler overhead from executor work.
    engine
        .register_rule(bench_noop_rule())
        .expect("register noop rule");

    let ty = make_type_id("entity");
    let mut labels = Vec::with_capacity(n);
    for i in 0..n {
        let label = format!("sched-ent-{}", i);
        let id = make_node_id(&label);
        engine.insert_node(id, NodeRecord { ty, payload: None });
        labels.push(label);
    }
    (engine, labels)
}

fn bench_scheduler_drain(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_drain");
    for &n in &[10usize, 100, 1_000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || build_engine_with_entities(n),
                |(mut engine, labels)| {
                    // Apply the no-op rule to all entities, then commit.
                    let tx = engine.begin();
                    for label in &labels {
                        let id = make_node_id(label);
                        let res = engine.apply(tx, "bench/noop", &id).expect("apply");
                        assert!(matches!(res, ApplyResult::Applied));
                    }
                    let _snap = engine.commit(tx).expect("commit");
                },
                BatchSize::PerIteration,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench_scheduler_drain);
criterion_main!(benches);
