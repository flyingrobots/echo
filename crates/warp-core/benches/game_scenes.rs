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
//! F6 game scenes: start each scene, run its frames, tear it down.
//!
//! ```bash
//! cargo bench -p warp-core --features native_rule_bootstrap,trusted_runtime \
//!   --bench game_scenes
//! ```
//!
//! Each scene runs once per commitment profile. A frame is one submitted and
//! admitted step intent, one tick, and a render read of every body. The
//! headline `F6` line reports only measured durations: frame latency
//! percentiles and maximum, frames over the 60 Hz budget, start-up, the
//! render read, the reference step timed alone outside Echo, teardown and
//! replay. A separate `F6-residual` line reports a derived estimate, described
//! where it is computed. Only a valid run reports figures.

#[path = "support/fixtures/mod.rs"]
mod fixtures;

use std::time::Duration;

use fixtures::game::{self, Observation, Scene, FRAME_BUDGET};
use fixtures::manifest::hex32;

const SEED: u64 = 6;

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

/// The sample at `per_mille` thousandths of the way through the sorted
/// samples, rounding half up.
fn percentile(samples: &[Duration], per_mille: usize) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = (sorted.len().saturating_sub(1) * per_mille + 500) / 1000;
    sorted.get(index).copied().map_or(0.0, ms)
}

fn report(scene: Scene, profile: &str, obs: &Observation) {
    let class = game::validate(scene, obs);
    let spec = game::workload(scene, SEED);
    println!(
        "F6 scene={} profile={profile} workload_id={} class={}",
        scene.name,
        hex32(&spec.workload_id()),
        class.label()
    );
    if !class.comparable() {
        println!("F6 scene={} detail: {class:?}", scene.name);
        return;
    }
    let t = &obs.timings;
    let over = t
        .frames
        .iter()
        .filter(|frame| **frame > FRAME_BUDGET)
        .count();
    println!(
        "F6 scene={} profile={profile} frames={} peak_bodies={} peak_contacts={} \
         startup_ms={:.2} frame_p50_ms={:.3} frame_p90_ms={:.3} frame_p99_ms={:.3} \
         frame_max_ms={:.3} over_16.7ms={over} render_p50_ms={:.3} \
         reference_step_p50_ms={:.3} teardown_ms={:.2} replay_ms={:.1}",
        scene.name,
        t.frames.len(),
        obs.peak_bodies,
        obs.peak_contacts,
        ms(t.startup),
        percentile(&t.frames, 500),
        percentile(&t.frames, 900),
        percentile(&t.frames, 990),
        percentile(&t.frames, 1000),
        percentile(&t.render, 500),
        percentile(&t.physics, 500),
        ms(t.teardown),
        ms(t.replay),
    );
    // A derived residual, not a measurement: per frame, the frame time less
    // twice the reference step time for that frame (the rule runs the step
    // once in its footprint and once in its executor). It is neither Echo's
    // cost nor an upper bound on it. The reference step is timed in a
    // separate pass outside Echo, so cache state and measurement conditions
    // differ from the in-frame runs, and the residual also holds the rule's
    // own work beyond the step: reading the world out of the graph and
    // diffing it. Attribution of the residual is uncertain.
    let residual: Vec<Duration> = t
        .frames
        .iter()
        .zip(&t.physics)
        .map(|(frame, step)| frame.saturating_sub(*step * 2))
        .collect();
    println!(
        "F6-residual scene={} profile={profile} \
         frame_minus_2x_reference_step_estimate_p50_ms={:.3} \
         frame_minus_2x_reference_step_estimate_p99_ms={:.3} \
         frame_minus_2x_reference_step_estimate_max_ms={:.3}",
        scene.name,
        percentile(&residual, 500),
        percentile(&residual, 990),
        percentile(&residual, 1000),
    );
}

fn main() {
    for scene in game::SCENES {
        report(scene, "legacy-reachable-v1", &game::run(scene, SEED));
        #[cfg(feature = "application_root_profile")]
        report(
            scene,
            "application-radix-v0",
            &game::run_with_profile(
                scene,
                SEED,
                warp_core::application_root::CommitmentProfile::ApplicationRadixV0,
            ),
        );
    }
}
