// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    missing_docs,
    dead_code,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::cast_precision_loss
)]
//! Scene fixture runner, first slice: C1 parity, then F5 append and replay.
//!
//! ```bash
//! cargo bench -p warp-core --features native_rule_bootstrap,trusted_runtime \
//!   --bench scene_fixtures
//! ```
//!
//! The run is a fixed matrix, so every invocation measures the same
//! workloads: tiers S and M of F5, seed 7, five rounds each. It reads no
//! settings from the environment; changing the matrix is a source change,
//! which changes the workload identities it prints.
//!
//! Every round is validated outside the timed regions and classified. Only
//! valid rounds enter the summary; a run with any invalid, unsupported or
//! incomplete round says so instead of reporting a figure. This is a normal
//! (uninstrumented) build. Peak memory is not measured by this slice.

#[path = "support/fixtures/mod.rs"]
mod fixtures;

use std::time::Duration;

use fixtures::manifest::hex32;
use fixtures::{star, think_append};

const SEED: u64 = 7;
const ROUNDS: usize = 5;

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

fn median_ms(mut samples: Vec<Duration>) -> f64 {
    samples.sort_unstable();
    samples
        .get(samples.len() / 2)
        .copied()
        .map(ms)
        .expect("summary is printed only when samples exist")
}

fn main() {
    // C1 is a precondition: the star control must build and hash.
    let c1 = star::workload(1_000);
    println!(
        "C1 workload_id={} state_root={} construction={}",
        hex32(&c1.workload_id()),
        hex32(&star::state_root(1_000)),
        c1.construction.label()
    );

    for tier in [think_append::S, think_append::M] {
        run_f5(tier, SEED, ROUNDS);
    }
}

fn run_f5(tier: think_append::Tier, seed: u64, rounds: usize) {
    let spec = think_append::workload(tier, seed);
    let script = think_append::script(tier, seed);
    let digest = hex32(&think_append::script_digest(&script));
    let expect = think_append::expectation(tier, Some(&digest));
    let workload_id = spec.workload_id();
    println!(
        "F5 tier={} seed={seed} workload_id={} expectation_id={} script_digest={digest}",
        tier.name,
        hex32(&workload_id),
        hex32(&expect.expectation_id(&workload_id))
    );
    println!(
        "F5 evidence={} construction={} history={:?}",
        spec.evidence.label(),
        spec.construction.label(),
        think_append::HISTORY_OPERATIONS
    );
    for (operation, why) in think_append::NOT_COVERED {
        println!("F5 not-covered {operation}: {why}");
    }
    println!(
        "F5 clocks: append = submit + admit + tick per append, envelopes built before; \
         replay = apply retained patches from the initial state; \
         validation, outcome collection and checkpoint readings are outside both"
    );

    let mut appends = Vec::new();
    let mut replays = Vec::new();
    let mut per_append = Vec::new();
    let mut comparable = true;
    for round in 1..=rounds {
        let obs = think_append::run(tier, seed);
        let class = think_append::validate(tier, &script, &expect, &obs);
        let a = &obs.accounting;
        println!(
            "round {round} class={} append_ms={:.2} replay_ms={:.2} occurrences={} \
             delivery_attempts={} submitted={} admitted={} committed={} refused={} \
             unfinished={} idle_passes={}",
            class.label(),
            ms(obs.timings.append_end_to_end),
            ms(obs.timings.replay),
            a.occurrences,
            a.delivery_attempts,
            a.submitted,
            a.admitted,
            a.committed,
            a.refused,
            a.unfinished,
            obs.idle_passes
        );
        if !class.comparable() {
            println!("round {round} detail: {class:?}");
            comparable = false;
            continue;
        }
        appends.push(obs.timings.append_end_to_end);
        replays.push(obs.timings.replay);
        per_append.extend(obs.timings.per_append);
    }
    if comparable && !appends.is_empty() {
        let committed = tier.appends as f64;
        let append_p50 = median_ms(appends);
        println!(
            "F5 tier={} summary valid_rounds={rounds} append_p50_ms={append_p50:.2} \
             per_append_p50_ms={:.4} useful_throughput_appends_per_s={:.0} replay_p50_ms={:.2}",
            tier.name,
            median_ms(per_append),
            committed / (append_p50 / 1e3),
            median_ms(replays)
        );
    } else {
        println!(
            "F5 tier={} summary withheld: not every round was valid",
            tier.name
        );
    }
}
