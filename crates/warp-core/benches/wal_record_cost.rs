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
//! Benchmark: what one update costs as the write-ahead log grows.
//!
//! One Action is submitted and one tick commits it, over and over, against
//! one cell. Nothing about the work changes from one update to the next. If
//! the cost of an update rises with the number of updates already made, the
//! log, the host's bookkeeping or retained history is doing work in
//! proportion to its size.
//!
//! Each row is the median over a window of updates that ends at the stated
//! history length. The same run is made with no log, with the in-memory log
//! and with the filesystem log, so a rise that appears without a log is not
//! the log's.
//!
//! Settings (environment):
//! - `ECHO_BENCH_HISTORY`: history lengths to report (default 250,500,1000,2000)
//! - `ECHO_BENCH_WINDOW`: updates in each measured window (default 100)
//! - `ECHO_BENCH_CELLS`: cells in the scene, each reachable from the root
//!   (default 1)
//! - `ECHO_BENCH_UNLINKED`: leave the cells unreachable, outside the state root
//! - `ECHO_BENCH_DURABILITY`: none, in-memory, filesystem (default: all three)
//! - `ECHO_BENCH_LOG_DIR`: where the filesystem log is written (default: the
//!   system temporary directory). The numbers depend on this device.

#[path = "support/action_scene.rs"]
mod action_scene;

use std::time::{Duration, Instant};

use action_scene::{
    log_parent, percentile_us, reachability_label, setting, setting_list, Durability, Scene,
};

fn durabilities() -> Vec<Durability> {
    let all = [
        Durability::None,
        Durability::InMemory,
        Durability::Filesystem,
    ];
    let Ok(wanted) = std::env::var("ECHO_BENCH_DURABILITY") else {
        return all.to_vec();
    };
    let names: Vec<&str> = wanted.split(',').map(str::trim).collect();
    for name in &names {
        assert!(
            all.iter().any(|durability| durability.label() == *name),
            "ECHO_BENCH_DURABILITY: unknown label `{name}` (none, in-memory, filesystem)"
        );
    }
    all.into_iter()
        .filter(|durability| names.contains(&durability.label()))
        .collect()
}

fn main() {
    let cells = setting("ECHO_BENCH_CELLS", 1);
    let window = setting("ECHO_BENCH_WINDOW", 100);
    let mut history = setting_list("ECHO_BENCH_HISTORY", &[250, 500, 1000, 2000]);
    history.sort_unstable();
    let last = history
        .last()
        .copied()
        .expect("at least one history length");
    assert!(window > 0, "ECHO_BENCH_WINDOW must be positive");
    assert!(
        history.first().is_some_and(|first| *first >= window),
        "the first history length must be at least one window"
    );

    println!(
        "wal_record_cost: {cells} cell(s) ({}), one Action and one tick per update, window {window}, log directory {}",
        reachability_label(),
        log_parent().display()
    );
    println!(
        "{:>11} {:>9} {:>14} {:>12} {:>13}",
        "durability", "history", "submit_p50_us", "tick_p50_us", "update_p50_us"
    );
    for durability in durabilities() {
        let mut scene = Scene::new(cells, durability, &log_parent());
        let mut submit: Vec<Duration> = Vec::with_capacity(last);
        let mut tick: Vec<Duration> = Vec::with_capacity(last);
        // Each update's submission and tick together, so the update median
        // is the median of real updates, not a sum of two medians.
        let mut update_total: Vec<Duration> = Vec::with_capacity(last);
        let mut first_update = None;
        for update in 0..last {
            let next = format!("u{update:08}");
            let envelope = scene.envelope(0, &next, 0);
            let submit_started = Instant::now();
            if durability == Durability::None {
                scene
                    .host
                    .app()
                    .submit_intent(envelope)
                    .expect("an Action is accepted into intake");
            } else {
                scene
                    .host
                    .app()
                    .submit_intent_with_runtime_wal_ack(envelope)
                    .expect("an Action is acknowledged by the log");
            }
            let submit_elapsed = submit_started.elapsed();
            submit.push(submit_elapsed);
            let tick_started = Instant::now();
            let steps = scene.host.tick_once().expect("a tick commits");
            let tick_elapsed = tick_started.elapsed();
            tick.push(tick_elapsed);
            update_total.push(submit_elapsed + tick_elapsed);
            assert_eq!(
                steps.iter().map(|step| step.admitted_count).sum::<usize>(),
                1,
                "each tick commits the one pending Action"
            );
            scene.committed(0, &next, 0);

            let done = update + 1;
            if history.contains(&done) {
                let submit_p50 = percentile_us(&submit[done - window..], 50);
                let tick_p50 = percentile_us(&tick[done - window..], 50);
                let total = percentile_us(&update_total[done - window..], 50);
                let first = *first_update.get_or_insert(total);
                println!(
                    "{:>11} {:>9} {:>14.1} {:>12.1} {:>13.1}  ({:.2}x the first row)",
                    durability.label(),
                    done,
                    submit_p50,
                    tick_p50,
                    total,
                    total / first
                );
            }
        }
    }
}
