// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(missing_docs, dead_code, clippy::expect_used, clippy::print_stdout)]
//! F5 tier M under the application-root profile, repeated for a sampling
//! profiler. Not a measurement: `scene_fixtures` is the measurement. This
//! only keeps the append path hot long enough to sample it.
//!
//! `ECHO_PROFILE_ROUNDS` sets the number of rounds (default 40).

#[path = "support/fixtures/mod.rs"]
mod fixtures;

use fixtures::think_append;

fn main() {
    let rounds: usize = std::env::var("ECHO_PROFILE_ROUNDS")
        .ok()
        .map_or(40, |value| value.parse().expect("a numeric round count"));
    let mut append = std::time::Duration::ZERO;
    for _ in 0..rounds {
        let obs = think_append::run_with_profile(
            think_append::M,
            7,
            warp_core::application_root::CommitmentProfile::ApplicationRadixV0,
        );
        append += obs.timings.append_end_to_end;
    }
    println!(
        "rounds={rounds} mean_append_ms={:.2}",
        append.as_secs_f64() * 1e3 / rounds as f64
    );
}
