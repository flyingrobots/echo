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

/// Creates a 32-byte hash where every byte position is distinguishable.
/// This catches serializer bugs that drop, reorder, or endian-flip tail bytes
/// (a uniform [n, 0, 0, ...] fixture would mask such issues).
fn make_hash(seed: u8) -> [u8; 32] {
    #[expect(clippy::cast_possible_truncation, reason = "i ∈ 0..32, fits in u8")]
    core::array::from_fn(|i| seed.wrapping_add((i as u8).wrapping_mul(17)))
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
        hex, "cbb1f5f73d0b5da137b0bde7b7d242fb44b8b6d0f5d4f8391105b6c36aa7a974",
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
    let schema_hash = make_hash(0xAB);
    let worldline_id = WorldlineId::from_bytes(make_hash(0xCD));
    let tick = 42u64;
    let parent = make_hash(0x11);
    let patch_digest = make_hash(0x22);
    let state_root = make_hash(0x33);

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
        hex, "8a769a8d8dd847be4ff546f1214a44d49446f05a0a10450b5d0ec21bd68613e9",
        "tick_commit_hash_v2 full-chain golden vector mismatch!\nactual: {hex}"
    );
}
