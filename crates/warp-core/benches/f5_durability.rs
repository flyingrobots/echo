// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    missing_docs,
    dead_code,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_precision_loss
)]
//! F5 tier M with the host's runtime WAL off, in memory, and on disk, and a
//! census of the records in the fixture's warp.
//!
//! ```bash
//! ECHO_F5_WAL_DIR=/some/empty/dir cargo bench -p warp-core \
//!   --features native_rule_bootstrap,trusted_runtime,application_root_profile \
//!   --bench f5_durability
//! ```
//!
//! The filesystem runs need `ECHO_F5_WAL_DIR`; each round writes a fresh
//! subdirectory there and removes it afterwards. Without it, they are
//! skipped and the output says so. Every round is validated; only a run
//! whose rounds are all valid prints a summary.

#[path = "support/fixtures/mod.rs"]
mod fixtures;

use std::time::Duration;

use fixtures::manifest::hex32;
use fixtures::think_append::{self, Durability};
use warp_core::WorldlineState;

const SEED: u64 = 7;
const ROUNDS: usize = 5;

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

fn median_ms(mut samples: Vec<Duration>) -> f64 {
    samples.sort_unstable();
    samples.get(samples.len() / 2).copied().map_or(0.0, ms)
}

fn dir_bytes(path: &std::path::Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => dir_bytes(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}

fn run(profile: &str, prepare: fn(WorldlineState) -> WorldlineState, mode: &str) {
    let tier = think_append::M;
    let wal_root = std::env::var_os("ECHO_F5_WAL_DIR").map(std::path::PathBuf::from);
    if mode == "filesystem-wal" && wal_root.is_none() {
        println!(
            "F5D tier={} profile={profile} durability={mode} skipped: ECHO_F5_WAL_DIR unset",
            tier.name
        );
        return;
    }
    let script = think_append::script(tier, SEED);
    let digest = hex32(&think_append::script_digest(&script));
    let expect = think_append::expectation(tier, Some(&digest));
    let mut appends = Vec::new();
    let mut per_append = Vec::new();
    let mut wal_bytes = Vec::new();
    let mut comparable = true;
    for round in 1..=ROUNDS {
        let durability = match mode {
            "none" => Durability::None,
            "in-memory-wal" => Durability::InMemory,
            _ => {
                let dir = wal_root
                    .as_ref()
                    .expect("checked above")
                    .join(format!("{profile}-r{round}"));
                let _ = std::fs::remove_dir_all(&dir);
                std::fs::create_dir_all(&dir).expect("WAL directory creates");
                Durability::Filesystem(dir)
            }
        };
        if round == 1 {
            let spec = think_append::workload_with(tier, SEED, &durability);
            println!(
                "F5D tier={} profile={profile} durability={mode} workload_id={}",
                tier.name,
                hex32(&spec.workload_id())
            );
        }
        let obs = think_append::run_durable(tier, SEED, prepare, &durability);
        let class = think_append::validate(tier, &script, &expect, &obs);
        let bytes = match &durability {
            Durability::Filesystem(dir) => {
                let bytes = dir_bytes(dir);
                let _ = std::fs::remove_dir_all(dir);
                Some(bytes)
            }
            _ => None,
        };
        println!(
            "F5D round {round} profile={profile} durability={mode} class={} append_ms={:.2} wal_bytes={}",
            class.label(),
            ms(obs.timings.append_end_to_end),
            bytes.map_or_else(|| "-".to_owned(), |b| b.to_string())
        );
        if round == 1 {
            let c = obs.census;
            println!(
                "F5D census profile={profile} durability={mode} fixture_nodes={} other_nodes={} \
                 fixture_edges={} other_edges={} fixture_node_attachments={} \
                 other_node_attachments={} edge_attachments={}",
                c.fixture_nodes,
                c.other_nodes,
                c.fixture_edges,
                c.other_edges,
                c.fixture_node_attachments,
                c.other_node_attachments,
                c.edge_attachments
            );
        }
        if !class.comparable() {
            println!("F5D round {round} detail: {class:?}");
            comparable = false;
            continue;
        }
        appends.push(obs.timings.append_end_to_end);
        per_append.extend(obs.timings.per_append);
        wal_bytes.extend(bytes);
    }
    if comparable && !appends.is_empty() {
        println!(
            "F5D tier={} profile={profile} durability={mode} summary valid_rounds={ROUNDS} \
             append_p50_ms={:.2} per_append_p50_ms={:.4} wal_bytes_max={}",
            tier.name,
            median_ms(appends),
            median_ms(per_append),
            wal_bytes
                .iter()
                .max()
                .map_or_else(|| "-".to_owned(), u64::to_string)
        );
    } else {
        println!(
            "F5D tier={} profile={profile} durability={mode} summary withheld: not every round was valid",
            tier.name
        );
    }
}

fn main() {
    for mode in ["none", "in-memory-wal", "filesystem-wal"] {
        run("legacy-reachable-v1", |state| state, mode);
        #[cfg(feature = "application_root_profile")]
        run(
            "application-radix-v0",
            |state| {
                state.with_commitment_profile(
                    warp_core::application_root::CommitmentProfile::ApplicationRadixV0,
                )
            },
            mode,
        );
    }
}
