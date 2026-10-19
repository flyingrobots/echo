// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![cfg(feature = "host_test")]
#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    missing_docs
)]
//! Study probe, not for merge: how many whole-state root computations does
//! each F5 append perform, and what share of the append time do they take?

#[path = "../benches/support/action_scene.rs"]
#[allow(dead_code)]
mod action_scene;
#[path = "../benches/support/fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::think_append;

#[test]
fn f5_state_root_share() {
    for tier in [think_append::S, think_append::M] {
        let obs = think_append::run(tier, 7);
        let appends = obs.timings.per_append.len();
        assert_eq!(appends as u64, tier.appends, "every append ran");
        let probe = &obs.root_probe_append;
        let root_nanos: f64 = probe.iter().map(|(_, d)| d.as_nanos() as f64).sum();
        let append_nanos = obs.timings.append_end_to_end.as_nanos() as f64;
        println!(
            "PROBE tier={} appends={appends} root_calls={} calls_per_append={:.3} root_ms={:.2} append_ms={:.2} root_share={:.3}",
            tier.name,
            probe.len(),
            probe.len() as f64 / appends as f64,
            root_nanos / 1e6,
            append_nanos / 1e6,
            root_nanos / append_nanos
        );
        // Ten windows of calls: reachable nodes and mean time per call.
        let window = probe.len().div_ceil(10).max(1);
        for (index, chunk) in probe.chunks(window).enumerate() {
            let nodes = chunk.iter().map(|(n, _)| *n as f64).sum::<f64>() / chunk.len() as f64;
            let nanos =
                chunk.iter().map(|(_, d)| d.as_nanos() as f64).sum::<f64>() / chunk.len() as f64;
            println!(
                "PROBE tier={} window={index} calls={} mean_reachable={nodes:.0} mean_root_us={:.1} ns_per_node={:.1}",
                tier.name,
                chunk.len(),
                nanos / 1e3,
                nanos / nodes.max(1.0)
            );
        }
        // Per-append time over the same ten windows.
        let window = appends.div_ceil(10);
        for (index, chunk) in obs.timings.per_append.chunks(window).enumerate() {
            let mean = chunk.iter().map(|d| d.as_nanos() as f64).sum::<f64>() / chunk.len() as f64;
            println!(
                "PROBE tier={} append_window={index} mean_append_us={:.1}",
                tier.name,
                mean / 1e3
            );
        }
    }
}
