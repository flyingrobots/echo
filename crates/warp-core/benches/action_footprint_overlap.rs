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
//! Benchmark: frame cost when Actions in one frame touch the same cells.
//!
//! Every other benchmark of the Action path gives each Action its own cell,
//! so no footprint ever overlaps. A real frame has contention. This one
//! submits `K` compare-and-set Actions per frame and makes a chosen share of
//! them target a cell that another Action of the same frame also targets.
//!
//! The scheduler applies one Action per cell in a tick and gives every other
//! Action on that cell the terminal outcome `RejectedFootprintConflict`. The
//! table reports what a frame costs and how many Actions it commits, so the
//! cost of contention reads as time per committed Action.
//!
//! All Actions of a frame are built before the frame's first tick, as a host
//! that batches input does. A head takes a bounded number of Actions per
//! tick, so a frame larger than that needs several ticks, and the table shows
//! what happens to the Actions that wait: their outcome is counted, and the
//! reason for the first obstruction is printed under the row.
//!
//! No write-ahead log is enabled; `wal_record_cost` measures that.
//!
//! Settings (environment):
//! - `ECHO_BENCH_CELLS`: cells in the scene (default 10000)
//! - `ECHO_BENCH_ACTIONS`: Actions per frame, comma-separated (default 64,256)
//! - `ECHO_BENCH_OVERLAP`: percent of Actions that collide (default 0,10,25,50,90)
//! - `ECHO_BENCH_FRAMES`: frames measured per row (default 30)

#[path = "support/action_scene.rs"]
mod action_scene;

use std::time::{Duration, Instant};

use action_scene::{log_parent, percentile_us, setting, setting_list, Durability, Scene};
use warp_core::{EchoOperationActionOutcomeV1, EchoOperationObstructionKindV1};

struct Row {
    actions: usize,
    overlap_percent: usize,
    frame: Vec<Duration>,
    ticks: usize,
    committed: usize,
    rejected: usize,
    obstructed: usize,
    /// Why the first obstructed Action was obstructed.
    first_obstruction: Option<EchoOperationObstructionKindV1>,
}

fn run(cells: usize, actions: usize, overlap_percent: usize, frames: usize) -> Row {
    let colliding = actions * overlap_percent / 100;
    let distinct = actions - colliding;
    assert!(distinct > 0, "at least one Action must have its own cell");
    assert!(distinct <= cells, "the scene has too few cells");
    let mut scene = Scene::new(cells, Durability::None, &log_parent());
    let mut row = Row {
        actions,
        overlap_percent,
        frame: Vec::with_capacity(frames),
        ticks: 0,
        committed: 0,
        rejected: 0,
        obstructed: 0,
        first_obstruction: None,
    };
    for frame in 0..frames {
        let next = format!("f{frame:06}");
        // The first `distinct` Actions each take their own cell. The rest
        // take a cell one of those already took.
        let targets: Vec<usize> = (0..actions)
            .map(|index| {
                if index < distinct {
                    index
                } else {
                    index % distinct
                }
            })
            .collect();
        let envelopes: Vec<_> = targets
            .iter()
            .enumerate()
            .map(|(nonce, cell)| scene.envelope(*cell, &next, nonce))
            .collect();

        let started = Instant::now();
        let mut submissions = Vec::with_capacity(actions);
        for envelope in envelopes {
            let handle = scene
                .host
                .app()
                .submit_intent(envelope)
                .expect("an Action is accepted into intake");
            submissions.push(handle.submission_id);
        }
        let mut decided = 0usize;
        let mut ticks = 0usize;
        while decided < actions {
            let steps = scene.host.tick_once().expect("a tick commits");
            let admitted: usize = steps.iter().map(|step| step.admitted_count).sum();
            assert!(
                admitted > 0,
                "a tick decided nothing with {decided} of {actions} Actions decided"
            );
            decided += admitted;
            ticks += 1;
        }
        row.frame.push(started.elapsed());
        row.ticks += ticks;

        for (nonce, submission) in submissions.iter().enumerate() {
            match scene.host.echo_operation_action_outcome_v1(submission) {
                Some(EchoOperationActionOutcomeV1::Committed(_)) => {
                    row.committed += 1;
                    scene.committed(targets[nonce], &next, nonce);
                }
                Some(EchoOperationActionOutcomeV1::RejectedFootprintConflict(_)) => {
                    row.rejected += 1;
                }
                Some(EchoOperationActionOutcomeV1::Obstructed(obstruction)) => {
                    row.obstructed += 1;
                    row.first_obstruction
                        .get_or_insert_with(|| obstruction.kind());
                }
                None => panic!("a decided Action has no outcome"),
            }
        }
    }
    row
}

fn main() {
    let cells = setting("ECHO_BENCH_CELLS", 10_000);
    let frames = setting("ECHO_BENCH_FRAMES", 30);
    let action_counts = setting_list("ECHO_BENCH_ACTIONS", &[64, 256]);
    let overlaps = setting_list("ECHO_BENCH_OVERLAP", &[0, 10, 25, 50, 90]);

    println!("action_footprint_overlap: {cells} cells, {frames} frames per row, no log");
    println!(
        "{:>8} {:>8} {:>10} {:>10} {:>7} {:>10} {:>9} {:>10} {:>14} {:>14}",
        "actions",
        "overlap%",
        "p50_ms",
        "p95_ms",
        "ticks",
        "committed",
        "rejected",
        "obstructed",
        "us/submitted",
        "us/committed"
    );
    for actions in &action_counts {
        for overlap in &overlaps {
            let row = run(cells, *actions, *overlap, frames);
            let frames_run = row.frame.len();
            let p50 = percentile_us(&row.frame, 50);
            let committed_per_frame = row.committed as f64 / frames_run as f64;
            println!(
                "{:>8} {:>8} {:>10.3} {:>10.3} {:>7.1} {:>10.1} {:>9.1} {:>10.1} {:>14.1} {:>14.1}",
                row.actions,
                row.overlap_percent,
                p50 / 1e3,
                percentile_us(&row.frame, 95) / 1e3,
                row.ticks as f64 / frames_run as f64,
                committed_per_frame,
                row.rejected as f64 / frames_run as f64,
                row.obstructed as f64 / frames_run as f64,
                p50 / row.actions as f64,
                if committed_per_frame > 0.0 {
                    p50 / committed_per_frame
                } else {
                    0.0
                },
            );
            if let Some(kind) = row.first_obstruction {
                println!("         obstructed Actions: first reason {kind:?}");
            }
        }
    }
}
