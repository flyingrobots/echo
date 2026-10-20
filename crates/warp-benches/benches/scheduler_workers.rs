// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::items_after_statements,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
//! Benchmark: commit cost of an engine tick by worker count.
//!
//! Each of `n` rewrites targets its own node, so nothing conflicts and the
//! rewrites spread over the 256 shards. Its executor chains `work` blake3
//! hashes from its node id, then writes the result as the node's attachment,
//! so a tick has real per-rewrite work to share between workers. Rewrites
//! are enqueued before the clock starts; the table reports the commit alone.
//!
//! Settings (environment, comma-separated lists):
//! - `ECHO_BENCH_REWRITES` (default 1,100,10000)
//! - `ECHO_BENCH_WORK`: hashes per rewrite (default 0,100,1000)
//! - `ECHO_BENCH_WORKERS` (default 1,2,4,8,16,18,32,64,128,256)
//! - `ECHO_BENCH_ROUNDS`: ticks measured per row (default 15)

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use warp_core::{
    make_node_id, make_type_id, AtomPayload, AttachmentKey, AttachmentValue, ConflictPolicy,
    EngineBuilder, ExecutionGraphView, Footprint, GraphStore, GraphView, Hash, NodeId, NodeKey,
    NodeRecord, PatternGraph, RewriteRule, RuleExecutor, TickDelta, WarpOp,
};

const RULE_NAME: &str = "bench/work";
static WORK: AtomicU64 = AtomicU64::new(0);
/// Hashed once, so the executor's time is its work and its write.
static RESULT_TYPE: std::sync::OnceLock<warp_core::TypeId> = std::sync::OnceLock::new();

fn work_rule() -> RewriteRule {
    let id: Hash = {
        let mut h = blake3::Hasher::new();
        h.update(b"rule:");
        h.update(RULE_NAME.as_bytes());
        h.finalize().into()
    };
    fn matcher(_view: GraphView<'_>, _n: &NodeId) -> bool {
        true
    }
    fn executor(view: &mut ExecutionGraphView<'_, '_>, n: &NodeId, delta: &mut TickDelta) {
        let mut value: [u8; 32] = n.0;
        for _ in 0..WORK.load(Ordering::Relaxed) {
            value = blake3::hash(&value).into();
        }
        delta.push(WarpOp::SetAttachment {
            key: AttachmentKey::node_alpha(NodeKey {
                warp_id: view.warp_id(),
                local_id: *n,
            }),
            value: Some(AttachmentValue::Atom(AtomPayload::new(
                *RESULT_TYPE.get_or_init(|| make_type_id("bench/work-result")),
                bytes::Bytes::copy_from_slice(&value),
            ))),
        });
    }
    fn footprint(view: GraphView<'_>, n: &NodeId) -> Footprint {
        let warp_id = view.warp_id();
        let mut footprint = Footprint::default();
        footprint.n_read.insert_with_warp(warp_id, *n);
        footprint.a_write.insert(AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: *n,
        }));
        footprint
    }
    RewriteRule {
        id,
        name: RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher,
        executor: RuleExecutor::observed(executor),
        compute_footprint: footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn setting_list(name: &str, default: &[usize]) -> Vec<usize> {
    std::env::var(name).ok().map_or_else(
        || default.to_vec(),
        |value| {
            value
                .split(',')
                .map(|part| part.trim().parse().expect("a numeric list setting"))
                .collect()
        },
    )
}

fn percentile(sorted: &[Duration], p: f64) -> f64 {
    let index = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[index].as_secs_f64() * 1_000.0
}

fn main() {
    let sizes = setting_list("ECHO_BENCH_REWRITES", &[1, 100, 10_000]);
    let works = setting_list("ECHO_BENCH_WORK", &[0, 100, 1_000]);
    let workers = setting_list(
        "ECHO_BENCH_WORKERS",
        &[1, 2, 4, 8, 16, 18, 32, 64, 128, 256],
    );
    let rounds = setting_list("ECHO_BENCH_ROUNDS", &[15])[0];
    println!("| rewrites | hashes/rewrite | workers | commit p50 ms | commit p90 ms |");
    println!("| ---: | ---: | ---: | ---: | ---: |");
    for &n in &sizes {
        for &work in &works {
            WORK.store(work as u64, Ordering::Relaxed);
            for &worker_count in &workers {
                let mut times = Vec::with_capacity(rounds);
                for _ in 0..rounds {
                    let root = make_node_id("bench-root");
                    let mut store = GraphStore::default();
                    store.insert_node(
                        root,
                        NodeRecord {
                            ty: make_type_id("bench-root"),
                        },
                    );
                    let ids: Vec<NodeId> = (0..n)
                        .map(|i| make_node_id(&format!("bench-entity-{i}")))
                        .collect();
                    for id in &ids {
                        store.insert_node(
                            *id,
                            NodeRecord {
                                ty: make_type_id("bench-entity"),
                            },
                        );
                    }
                    let mut engine = EngineBuilder::new(store, root)
                        .workers(worker_count)
                        .build();
                    engine
                        .register_rule(work_rule())
                        .expect("the rule registers");
                    let tx = engine.begin();
                    for id in &ids {
                        engine.apply(tx, RULE_NAME, id).expect("the rule applies");
                    }
                    let started = Instant::now();
                    let result = engine.commit_with_receipt(tx).expect("the tick commits");
                    times.push(started.elapsed());
                    std::hint::black_box(result);
                }
                times.sort();
                println!(
                    "| {n} | {work} | {worker_count} | {:.3} | {:.3} |",
                    percentile(&times, 0.5),
                    percentile(&times, 0.9)
                );
            }
        }
    }
}
