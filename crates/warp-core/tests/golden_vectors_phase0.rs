// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Phase 0 golden vector suite for ADR-0008/0009 refactor safety.
//!
//! These tests pin the exact deterministic hash outputs of the current engine
//! before the worldline runtime refactor begins. If any golden vector breaks,
//! the refactor has changed commit semantics and must be investigated.
//!
//! ## Coverage
//!
//! | Vector | What it pins |
//! |--------|-------------|
//! | GV-001 | Single-head single-worldline commit (empty tick) |
//! | GV-002 | Provenance replay integrity (5-tick worldline) |
//! | GV-003 | Fork reproducibility (prefix identity) |
//! | GV-004 | Idempotent ingress (content-addressed intent dedup) |
//!
//! ## Future vectors (populated in later phases)
//!
//! | Vector | Phase | What it will pin |
//! |--------|-------|-----------------|
//! | GV-005 | 2 | Multi-worldline scheduling order |
//! | GV-006 | 10 | Application-message idempotence |
//! | GV-007 | 11 | Transport state convergence |
//! | GV-008 | 9C | Explicit conflict artifact recording |
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::unreadable_literal,
    clippy::panic,
    clippy::format_collect,
    clippy::match_wildcard_for_single_variants,
    clippy::redundant_clone
)]

mod common;
use common::{create_initial_store, setup_worldline_with_ticks, test_cursor_id, test_warp_id};

use warp_core::{
    CursorRole, EngineBuilder, PlaybackCursor, ProvenanceStore, WorldlineId, WorldlineTick,
};

// =============================================================================
// Helper: parse hex string to [u8; 32]
// =============================================================================

fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn wt(raw: u64) -> WorldlineTick {
    WorldlineTick::from_raw(raw)
}

// =============================================================================
// GV-001: Single-head single-worldline commit determinism (empty tick)
// =============================================================================

/// Pinned golden vectors for a commit with no rewrites on a minimal graph.
///
/// This establishes the baseline: even an empty commit must produce the exact
/// same hashes across all platforms and Rust versions.
#[test]
fn gv001_single_commit_determinism() {
    const EXPECTED_STATE_ROOT: &str =
        "ca5b20c5da9c999a1ed795a93dfb7ce057fa26f84f1be99c9daa6b57c8725b5c";
    const EXPECTED_PATCH_DIGEST: &str =
        "b1b99e0b4ecb7f32c3bfeb335e3213593f80e98047e7e61822079953e1984ac1";
    const EXPECTED_COMMIT_HASH: &str =
        "16fc18a0622b1c4a177cbaf1618fc48f5433f9b1bebb92a522b15923ec9f75fe";

    let warp_id = test_warp_id();
    let initial_store = create_initial_store(warp_id);
    let root = warp_core::make_node_id("root");

    let mut engine = EngineBuilder::new(initial_store, root).workers(1).build();
    let tx = engine.begin();
    let snapshot = engine.commit(tx).expect("commit should succeed");

    assert_eq!(
        hex(&snapshot.state_root),
        EXPECTED_STATE_ROOT,
        "GV-001: state_root mismatch — commit semantics have changed"
    );
    assert_eq!(
        hex(&snapshot.patch_digest),
        EXPECTED_PATCH_DIGEST,
        "GV-001: patch_digest mismatch — commit semantics have changed"
    );
    assert_eq!(
        hex(&snapshot.hash),
        EXPECTED_COMMIT_HASH,
        "GV-001: commit_hash mismatch — commit semantics have changed"
    );
}

// =============================================================================
// GV-002: Provenance replay integrity (5-tick worldline)
// =============================================================================

/// Pinned golden vectors for a 5-tick worldline's provenance chain.
///
/// Each tick adds a deterministic node. The hash triplet (state_root,
/// patch_digest, commit_hash) at every tick must be reproducible.
#[test]
fn gv002_provenance_replay_integrity() {
    // (state_root, patch_digest, commit_hash) per tick
    const EXPECTED: [(&str, &str, &str); 5] = [
        (
            "c867d82d58d4d32dbba9b3df68fd2db5b5fac7d798b863c31ae219593b15941d",
            "c8a5742eac00bd749b047eb370a216550d89506db974f33ea8e38267fbb99c30",
            "3a812930ebb500193e04eb54dd9d91464dc083bc5831d1f56cda003a979ba79b",
        ),
        (
            "c867d82d58d4d32dbba9b3df68fd2db5b5fac7d798b863c31ae219593b15941d",
            "6233bde837b9ed096bb9cf4da088b473774ef93c6ac5b81d802671198f4ce1d7",
            "f636478b62ab0a9ba53fcb4265f16b249094715827c383d58ed508cf2033936d",
        ),
        (
            "c867d82d58d4d32dbba9b3df68fd2db5b5fac7d798b863c31ae219593b15941d",
            "28e6bfae2d50e1a6844e5b2296b3919763ead2456193abb76afb11d71feb09e7",
            "cf7104578e38084fd967da50094d12001dd1c26bb43c3cf08d1469c2f1128355",
        ),
        (
            "c867d82d58d4d32dbba9b3df68fd2db5b5fac7d798b863c31ae219593b15941d",
            "1a22ffbe3349630eca690769929680a1916c2cff141d03884c395e8174b7c13e",
            "3b0e8f0ba658ef7d0bd434335050804197f02916480571a43fb2266208e82daf",
        ),
        (
            "c867d82d58d4d32dbba9b3df68fd2db5b5fac7d798b863c31ae219593b15941d",
            "449e6e3b7af6c76b0052747afaf5ff2779c6e3881523326926c0699e005ef9c4",
            "b2800a319ccc208e8d627b9b78c8c4d88c2494cfc9eeb4d98a2720b2a881b92c",
        ),
    ];

    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    let actual: Vec<(String, String, String)> = (0..EXPECTED.len())
        .map(|tick| {
            let triplet = provenance
                .entry(worldline_id, wt(tick as u64))
                .unwrap_or_else(|e| panic!("tick {tick}: {e}"))
                .expected;
            (
                hex(&triplet.state_root),
                hex(&triplet.patch_digest),
                hex(&triplet.commit_hash),
            )
        })
        .collect();

    // Verify each tick's hash triplet against pinned values
    for (tick, (exp_sr, exp_pd, exp_ch)) in EXPECTED.iter().enumerate() {
        let (actual_sr, actual_pd, actual_ch) = &actual[tick];
        assert_eq!(
            actual_sr, *exp_sr,
            "GV-002 tick {tick}: state_root mismatch"
        );
        assert_eq!(
            actual_pd, *exp_pd,
            "GV-002 tick {tick}: patch_digest mismatch"
        );
        assert_eq!(
            actual_ch, *exp_ch,
            "GV-002 tick {tick}: commit_hash mismatch"
        );
    }

    // Verify cursor replay reaches the same final state
    let mut cursor = PlaybackCursor::new(
        test_cursor_id(1),
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &initial_store,
        wt(5),
    );
    cursor
        .seek_to(wt(5), &provenance, &initial_store)
        .expect("seek should succeed");
    let final_state_root = cursor.current_state_root();

    assert_eq!(
        hex(&final_state_root),
        EXPECTED[4].0,
        "GV-002: cursor replay state_root must match final tick (index 4)"
    );
}

// =============================================================================
// GV-003: Fork reproducibility (prefix identity)
// =============================================================================

/// Fork a 10-tick worldline at tick 5. The forked worldline must have
/// identical hash triplets for ticks 0..=5 (6 entries, fork-tick inclusive).
#[test]
fn gv003_fork_reproducibility() {
    // Pinned commit hashes for ticks 0..=5 of the 10-tick worldline (fork-tick inclusive)
    const EXPECTED_PREFIX_COMMITS: [&str; 6] = [
        "3a812930ebb500193e04eb54dd9d91464dc083bc5831d1f56cda003a979ba79b",
        "f636478b62ab0a9ba53fcb4265f16b249094715827c383d58ed508cf2033936d",
        "cf7104578e38084fd967da50094d12001dd1c26bb43c3cf08d1469c2f1128355",
        "3b0e8f0ba658ef7d0bd434335050804197f02916480571a43fb2266208e82daf",
        "b2800a319ccc208e8d627b9b78c8c4d88c2494cfc9eeb4d98a2720b2a881b92c",
        "983390108e4cc9d8d0c21fa47f0ee84eae115757d63d0b3f599925877c5bc30e",
    ];

    let (mut provenance, _initial_store, _warp_id, worldline_id) = setup_worldline_with_ticks(10);
    let forked_id = WorldlineId::from_bytes([2u8; 32]);

    provenance
        .fork(worldline_id, wt(5), forked_id)
        .expect("fork should succeed");

    // fork(src, 5, dst) copies ticks 0..=5 (6 entries)
    let forked_len = provenance.len(forked_id).unwrap();
    assert_eq!(forked_len, 6, "GV-003: fork at 5 should yield 6 entries");

    let actual_prefix_commits: Vec<String> = (0..EXPECTED_PREFIX_COMMITS.len())
        .map(|tick| {
            provenance
                .entry(worldline_id, wt(tick as u64))
                .unwrap()
                .expected
                .commit_hash
        })
        .map(|hash| hex(&hash))
        .collect();

    // Prefix ticks 0..5 must be identical between original and fork
    for (tick, exp_ch) in EXPECTED_PREFIX_COMMITS.iter().enumerate() {
        let original = provenance
            .entry(worldline_id, wt(tick as u64))
            .unwrap()
            .expected;
        let forked = provenance
            .entry(forked_id, wt(tick as u64))
            .unwrap()
            .expected;

        assert_eq!(
            original, forked,
            "GV-003 tick {tick}: forked prefix must match original"
        );
        assert_eq!(
            actual_prefix_commits[tick].as_str(),
            *exp_ch,
            "GV-003 tick {tick}: commit_hash mismatch"
        );
    }
}

// =============================================================================
// GV-004: Idempotent ingress (content-addressed intent dedup)
// =============================================================================

/// The same intent bytes must produce the same content-addressed intent_id,
/// and re-ingestion must be detected as a duplicate.
#[test]
fn gv004_idempotent_ingress() {
    const EXPECTED_INTENT_ID: &str =
        "b79ec7afbbe66524a17ae9bb1820f1551655ff5266bd8a3fad2dcb437ec3db5a";
    const EXPECTED_STATE_ROOT: &str =
        "ac7ac3aa3655a6c26de76668f4e19d562b7c48c9fa5aabfe3080fbb03d70e1c4";
    const EXPECTED_PATCH_DIGEST: &str =
        "b1b99e0b4ecb7f32c3bfeb335e3213593f80e98047e7e61822079953e1984ac1";
    const EXPECTED_COMMIT_HASH: &str =
        "33cb8f904a8c3124fd8a2b09125190d49a783c3367e1d044c6708e8015f4716d";

    let warp_id = test_warp_id();
    let initial_store = create_initial_store(warp_id);
    let root = warp_core::make_node_id("root");
    let intent_bytes = b"test-intent-payload-001";

    // First engine: ingest once
    let mut engine1 = EngineBuilder::new(initial_store.clone(), root)
        .workers(1)
        .build();
    let disp1 = engine1.ingest_intent(intent_bytes).unwrap();

    // Second engine: ingest same bytes independently
    let mut engine2 = EngineBuilder::new(initial_store.clone(), root)
        .workers(1)
        .build();
    let disp2 = engine2.ingest_intent(intent_bytes).unwrap();

    // Both must produce the same intent_id (content-addressed)
    let id1 = match disp1 {
        warp_core::IngestDisposition::Accepted { intent_id } => intent_id,
        other => panic!("expected Accepted, got {other:?}"),
    };
    let id2 = match disp2 {
        warp_core::IngestDisposition::Accepted { intent_id } => intent_id,
        other => panic!("expected Accepted, got {other:?}"),
    };

    assert_eq!(hex(&id1), EXPECTED_INTENT_ID, "GV-004: intent_id mismatch");
    assert_eq!(id1, id2, "GV-004: same bytes must produce same intent_id");

    // Re-ingestion into the same engine must be Duplicate
    let disp_dup = engine1.ingest_intent(intent_bytes).unwrap();
    match disp_dup {
        warp_core::IngestDisposition::Duplicate { intent_id } => {
            assert_eq!(
                intent_id, id1,
                "GV-004: duplicate must report same intent_id"
            );
        }
        other => panic!("expected Duplicate, got {other:?}"),
    }

    // Commits from both engines must produce identical pinned artifacts
    let tx1 = engine1.begin();
    let snap1 = engine1.commit(tx1).expect("commit 1");
    let tx2 = engine2.begin();
    let snap2 = engine2.commit(tx2).expect("commit 2");

    assert_eq!(
        hex(&snap1.state_root),
        EXPECTED_STATE_ROOT,
        "GV-004: state_root mismatch — commit semantics have changed"
    );
    assert_eq!(
        hex(&snap1.patch_digest),
        EXPECTED_PATCH_DIGEST,
        "GV-004: patch_digest mismatch — commit semantics have changed"
    );
    assert_eq!(
        hex(&snap1.hash),
        EXPECTED_COMMIT_HASH,
        "GV-004: commit_hash mismatch — commit semantics have changed"
    );
    assert_eq!(
        snap1.state_root, snap2.state_root,
        "GV-004: same ingested intent must produce same state root"
    );
    assert_eq!(
        hex(&snap2.state_root),
        EXPECTED_STATE_ROOT,
        "GV-004: second state_root mismatch — golden artifact drifted"
    );
    assert_eq!(
        hex(&snap2.patch_digest),
        EXPECTED_PATCH_DIGEST,
        "GV-004: second patch_digest mismatch — golden artifact drifted"
    );
    assert_eq!(
        hex(&snap2.hash),
        EXPECTED_COMMIT_HASH,
        "GV-004: second commit_hash mismatch — golden artifact drifted"
    );
    assert_eq!(
        snap1.patch_digest, snap2.patch_digest,
        "GV-004: same ingested intent must produce same patch digest"
    );
    assert_eq!(
        snap1.hash, snap2.hash,
        "GV-004: same ingested intent must produce same commit hash"
    );
}
