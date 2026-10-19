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
//! Benchmark: commit cost of an engine tick in which rewrites conflict.
//!
//! `scheduler_drain` gives every rewrite an empty footprint, so nothing is
//! ever rejected. Here a chosen share of `n` rewrites also write one shared
//! node. The first of them in canonical order is applied and every other is
//! rejected, so a tick has many rejections and many applied rewrites at once.
//! That is the case in which the receipt's blocker attribution compares each
//! rejected rewrite against every applied one.
//!
//! The executor does nothing, so the time is scheduling, patch and hashing
//! work. Rewrites are enqueued before the clock starts; the table reports the
//! commit alone (drain, reserve, execute, receipt, snapshot). Where the engine
//! derives blockers on demand, the last column times one such query for the
//! whole tick, from the state the tick began with.
//!
//! Settings (environment):
//! - `ECHO_BENCH_REWRITES`: rewrites per tick, comma-separated
//!   (default 1000,3000,10000)
//! - `ECHO_BENCH_OVERLAP`: percent of rewrites that write the shared node,
//!   comma-separated (default 0,10,50,90)
//! - `ECHO_BENCH_ROUNDS`: ticks measured per row (default 15)

use std::time::{Duration, Instant};

use blake3::Hasher;
use echo_dry_tests::build_motion_demo_engine;
use warp_core::{
    make_node_id, make_type_id, ApplyResult, ConflictPolicy, Engine, ExecutionGraphView, Footprint,
    GraphView, Hash, NodeId, NodeRecord, PatternGraph, RewriteRule, RuleExecutor, TickDelta,
    TickReceiptDisposition,
};

const RULE_NAME: &str = "bench/contend";
const FREE_TYPE: &str = "bench-free";
const CONTENDED_TYPE: &str = "bench-contended";
const SHARED_NODE: &str = "bench-shared";

fn contend_rule() -> RewriteRule {
    let id: Hash = {
        let mut h = Hasher::new();
        h.update(b"rule:");
        h.update(RULE_NAME.as_bytes());
        h.finalize().into()
    };
    fn matcher(_view: GraphView<'_>, _n: &NodeId) -> bool {
        true
    }
    fn executor(_view: &mut ExecutionGraphView<'_, '_>, _n: &NodeId, _delta: &mut TickDelta) {}
    fn footprint(view: GraphView<'_>, n: &NodeId) -> Footprint {
        let warp_id = view.warp_id();
        let mut footprint = Footprint::default();
        footprint.n_write.insert_with_warp(warp_id, *n);
        if view
            .node(n)
            .is_some_and(|record| record.ty == make_type_id(CONTENDED_TYPE))
        {
            footprint
                .n_write
                .insert_with_warp(warp_id, make_node_id(SHARED_NODE));
        }
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

fn build(n: usize, contended: usize) -> (Engine, Vec<NodeId>) {
    let mut engine = build_motion_demo_engine();
    engine
        .register_rule(contend_rule())
        .expect("the benchmark rule registers");
    let mut ids = Vec::with_capacity(n);
    for i in 0..n {
        let ty = if i < contended {
            CONTENDED_TYPE
        } else {
            FREE_TYPE
        };
        let id = make_node_id(&format!("bench-entity-{i}"));
        engine
            .insert_node(
                id,
                NodeRecord {
                    ty: make_type_id(ty),
                },
            )
            .expect("a benchmark node inserts");
        ids.push(id);
    }
    (engine, ids)
}

fn setting(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .map_or(default, |value| value.parse().expect("a numeric setting"))
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
    let sizes = setting_list("ECHO_BENCH_REWRITES", &[1_000, 3_000, 10_000]);
    let overlaps = setting_list("ECHO_BENCH_OVERLAP", &[0, 10, 50, 90]);
    let rounds = setting("ECHO_BENCH_ROUNDS", 15);
    println!("| rewrites | overlap % | applied | rejected | commit p50 ms | commit p90 ms | blame query ms |");
    println!("| ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
    for &n in &sizes {
        for &overlap in &overlaps {
            let contended = n * overlap / 100;
            let mut times = Vec::with_capacity(rounds);
            let mut applied = 0;
            let mut rejected = 0;
            let mut blame = Duration::ZERO;
            for round in 0..rounds {
                let (mut engine, ids) = build(n, contended);
                let tx = engine.begin();
                for id in &ids {
                    let result = engine
                        .apply(tx, RULE_NAME, id)
                        .expect("the benchmark rule applies");
                    assert!(matches!(result, ApplyResult::Applied));
                }
                let state_before = (round + 1 == rounds).then(|| engine.state().clone());
                let started = Instant::now();
                let (_snapshot, receipt, _patch) =
                    engine.commit_with_receipt(tx).expect("the tick commits");
                times.push(started.elapsed());
                if let Some(state_before) = state_before {
                    let started = Instant::now();
                    let blockers = engine
                        .footprint_blockers(&state_before, &receipt)
                        .expect("blockers are derivable");
                    blame = started.elapsed();
                    std::hint::black_box(blockers);
                }
                applied = receipt
                    .entries()
                    .iter()
                    .filter(|entry| entry.disposition == TickReceiptDisposition::Applied)
                    .count();
                rejected = receipt.entries().len() - applied;
                std::hint::black_box(receipt);
            }
            times.sort();
            println!(
                "| {n} | {overlap} | {applied} | {rejected} | {:.2} | {:.2} | {:.2} |",
                percentile(&times, 0.5),
                percentile(&times, 0.9),
                blame.as_secs_f64() * 1_000.0
            );
        }
    }
}
