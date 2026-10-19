// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![cfg(all(feature = "host_test", feature = "shadow_root_probe"))]
#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    missing_docs
)]
//! Study probe, not for merge: an incremental shadow application root kept
//! beside the legacy root through F5, measured inside the runtime.

#[path = "../benches/support/action_scene.rs"]
#[allow(dead_code)]
mod action_scene;
#[path = "../benches/support/fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::think_append;

#[test]
fn m1_shadow_root_through_f5() {
    for tier in [think_append::S, think_append::M] {
        warp_core::shadow_root_probe::reset();
        let obs = think_append::run(tier, 7);
        let log = warp_core::shadow_root_probe::take_log();
        println!(
            "SHADOW tier={} parity={:?} diffs={}",
            tier.name,
            obs.shadow_parity,
            obs.shadow_diff.len()
        );
        for (slot, in_shadow, in_state) in obs.shadow_diff.iter().take(12) {
            println!("SHADOW diff in_shadow={in_shadow} in_state={in_state} {slot}");
        }
        for u in log.iter().filter(|u| !u.sample.is_empty()) {
            println!(
                "SHADOW sample {}",
                u.sample.chars().take(700).collect::<String>()
            );
        }
        let bootstrap: Vec<_> = log.iter().filter(|u| u.bootstrap).collect();
        let ticks: Vec<_> = log.iter().filter(|u| !u.bootstrap).collect();
        let declared: usize = ticks.iter().map(|u| u.declared).sum();
        let from_ops: usize = ticks.iter().map(|u| u.from_ops).sum();
        let undeclared: usize = ticks.iter().map(|u| u.undeclared).sum();
        let instance_ops: usize = log.iter().map(|u| u.instance_ops).sum();
        println!("SHADOW tier={} slots declared(out_slots)={declared} written(ops)={from_ops} written_not_declared={undeclared} instance_ops={instance_ops}", tier.name);
        let shadow_ns: f64 = log.iter().map(|u| u.elapsed.as_nanos() as f64).sum();
        let root_ns: f64 = obs
            .root_probe_append
            .iter()
            .map(|(_, d)| d.as_nanos() as f64)
            .sum();
        let append_ns = obs.timings.append_end_to_end.as_nanos() as f64;
        println!(
            "SHADOW tier={} commits={} bootstrap={} ports={} append_ms={:.2} legacy_root_ms={:.2} shadow_ms={:.2} shadow_share={:.4} replay_root_eq_live={}",
            tier.name,
            log.len(),
            bootstrap.len(),
            log.iter().map(|u| u.port_slots).sum::<usize>(),
            append_ns / 1e6,
            root_ns / 1e6,
            shadow_ns / 1e6,
            shadow_ns / append_ns,
            obs.replay_root == Some(obs.live_root)
        );
        let window = ticks.len().div_ceil(10).max(1);
        for (index, chunk) in ticks.chunks(window).enumerate() {
            let n = chunk.len() as f64;
            println!(
                "SHADOW tier={} window={index} slots={:.1} hash_bytes={:.0} nodes_created={:.1} shadow_us={:.2}",
                tier.name,
                chunk.iter().map(|u| u.slots as f64).sum::<f64>() / n,
                chunk.iter().map(|u| u.hash_bytes as f64).sum::<f64>() / n,
                chunk.iter().map(|u| u.nodes_created as f64).sum::<f64>() / n,
                chunk.iter().map(|u| u.elapsed.as_nanos() as f64).sum::<f64>() / n / 1e3
            );
        }
    }
}
