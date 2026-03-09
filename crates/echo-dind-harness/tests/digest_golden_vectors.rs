// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::unwrap_used)]
//! Golden-vector tests for the TTD digest public API surface.
//!
//! These tests exercise `compute_emissions_digest`, `compute_op_emission_index_digest`,
//! and `compute_tick_commit_hash_v2` through warp-core's **crate-root re-exports** —
//! not via internal module paths. If a re-export is removed or a wire format changes,
//! these tests catch it.
//!
//! Each function is pinned to a known-good hash. The full chain is then exercised:
//! emissions → op-emission-index → tick-commit, so drift in any layer is detected.

use warp_core::materialization::{make_channel_id, FinalizedChannel};
use warp_core::{
    compute_emissions_digest, compute_op_emission_index_digest, compute_tick_commit_hash_v2,
    OpEmissionEntry, WorldlineId,
};

// ─── Test vectors ────────────────────────────────────────────────────────────

fn make_hash(n: u8) -> [u8; 32] {
    let mut h = [0u8; 32];
    h[0] = n;
    h
}

// ─── compute_emissions_digest ────────────────────────────────────────────────

#[test]
fn emissions_digest_golden_vector() {
    let ch_a = make_channel_id("alpha");
    let ch_b = make_channel_id("beta");

    let channels = vec![
        FinalizedChannel {
            channel: ch_a,
            data: vec![1, 2, 3],
        },
        FinalizedChannel {
            channel: ch_b,
            data: vec![4, 5],
        },
    ];

    let digest = compute_emissions_digest(&channels);
    let hex = hex::encode(digest);

    // Pinned golden value. If this changes, the emissions wire format changed.
    assert_eq!(
        hex, "9cd163d40fd2b8b089c5bed80c328ffc5695926cdbe2aaf1a99c83adf0bbe2ea",
        "emissions_digest golden vector mismatch — wire format may have changed!\nactual: {hex}"
    );
}

// ─── compute_op_emission_index_digest ────────────────────────────────────────

#[test]
fn op_emission_index_digest_golden_vector() {
    let ch_a = make_channel_id("alpha");
    let ch_b = make_channel_id("beta");

    let entries = vec![
        OpEmissionEntry {
            op_id: make_hash(0xAA),
            channels: vec![ch_a, ch_b],
        },
        OpEmissionEntry {
            op_id: make_hash(0xBB),
            channels: vec![ch_a],
        },
    ];

    let digest = compute_op_emission_index_digest(&entries);
    let hex = hex::encode(digest);

    // Pinned golden value. If this changes, the op-emission-index wire format changed.
    assert_eq!(
        hex, "162f7a1537231acd5e4138229900b46c51e6c4b5f1968c1fea1eb24c2e51a6ef",
        "op_emission_index_digest golden vector mismatch — wire format may have changed!\nactual: {hex}"
    );
}

// ─── compute_tick_commit_hash_v2 (full chain) ────────────────────────────────

#[test]
fn tick_commit_hash_v2_full_chain_golden_vector() {
    // Build the hash chain: emissions → op-emission-index → tick-commit.
    // This exercises the full TTD provenance digest surface end-to-end.

    let ch_a = make_channel_id("alpha");
    let ch_b = make_channel_id("beta");

    // Step 1: Compute emissions digest
    let channels = vec![
        FinalizedChannel {
            channel: ch_a,
            data: vec![1, 2, 3],
        },
        FinalizedChannel {
            channel: ch_b,
            data: vec![4, 5],
        },
    ];
    let emissions_digest = compute_emissions_digest(&channels);

    // Step 2: Compute op-emission-index digest
    let entries = vec![OpEmissionEntry {
        op_id: make_hash(0xAA),
        channels: vec![ch_a, ch_b],
    }];
    let op_emission_index_digest = compute_op_emission_index_digest(&entries);

    // Step 3: Compute tick commit hash using the above digests
    let schema_hash = [0xABu8; 32];
    let worldline_id = WorldlineId([0xCDu8; 32]);
    let tick = 42u64;
    let parent = [0x11u8; 32];
    let patch_digest = [0x22u8; 32];
    let state_root = [0x33u8; 32];

    let commit_hash = compute_tick_commit_hash_v2(
        &schema_hash,
        &worldline_id,
        tick,
        &[parent],
        &patch_digest,
        Some(&state_root),
        &emissions_digest,
        Some(&op_emission_index_digest),
    );
    let hex = hex::encode(commit_hash);

    // Pinned golden value for the full chain. If this changes, any layer's
    // wire format may have changed — check individual tests above to isolate.
    assert_eq!(
        hex, "8851ee5eada0d69032db7680e71ad1fc2c9bcf871b053b193d80dfea706eac1b",
        "tick_commit_hash_v2 full-chain golden vector mismatch!\nactual: {hex}"
    );
}
