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
    compute_state_root_for_warp_store, CursorRole, EngineBuilder, PlaybackCursor, ProvenanceStore,
    WorldlineId,
};

// =============================================================================
// Helper: parse hex string to [u8; 32]
// =============================================================================

fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
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

    let mut engine = EngineBuilder::new(initial_store, root).build();
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
            "96266268301910b9ba3d4b329e57b3ffc4dd14f86c0135bc522e4f39e61f3225",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "a2a95c7cf7826dd958efa34b67001cdb51ed0bdc5186e35f5801881011bdcf12",
        ),
        (
            "ffbdc6137114e50c7650e8e89256de68ffbc6309586e260ad03b4a26a02ea1c1",
            "0101010101010101010101010101010101010101010101010101010101010101",
            "17d403ac3ee32ae651b0a229829c9d498d2ca98cc5cff2ae00a36b4f3a4ee786",
        ),
        (
            "abfb7ff4864f246e970b192aa899b5c07ec06ea09f6ace47055c0b3ad61dc7b3",
            "0202020202020202020202020202020202020202020202020202020202020202",
            "6287d50b02bdfd201512e632ca6318f0f2df8432270e524eeeabb7312fe59785",
        ),
        (
            "c4c992d30ad7f83b4fb6e8a506313952653625497538e0e135eec9bd2cf82f8f",
            "0303030303030303030303030303030303030303030303030303030303030303",
            "f1b9996112f2bda21c391ed68c31caca2c650f200cc8b2ead86076a9ce7ea116",
        ),
        (
            "107238c92550c9561a9df3d6668b4c6e01ad06355e3ff82602c64eb476c539d5",
            "0404040404040404040404040404040404040404040404040404040404040404",
            "bb36ae47ea312a0199718bb137f508aee00fded15834f1b726c879b7a6174cda",
        ),
    ];

    let (provenance, initial_store, warp_id, worldline_id) = setup_worldline_with_ticks(5);

    // Verify each tick's hash triplet against pinned values
    for (tick, (exp_sr, exp_pd, exp_ch)) in EXPECTED.iter().enumerate() {
        let triplet = provenance
            .expected(worldline_id, tick as u64)
            .unwrap_or_else(|e| panic!("tick {tick}: {e}"));

        assert_eq!(
            hex(&triplet.state_root),
            *exp_sr,
            "GV-002 tick {tick}: state_root mismatch"
        );
        assert_eq!(
            hex(&triplet.patch_digest),
            *exp_pd,
            "GV-002 tick {tick}: patch_digest mismatch"
        );
        assert_eq!(
            hex(&triplet.commit_hash),
            *exp_ch,
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
        5,
    );
    cursor
        .seek_to(5, &provenance, &initial_store)
        .expect("seek should succeed");
    let final_state_root = compute_state_root_for_warp_store(&cursor.store, warp_id);

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
        "a2a95c7cf7826dd958efa34b67001cdb51ed0bdc5186e35f5801881011bdcf12",
        "17d403ac3ee32ae651b0a229829c9d498d2ca98cc5cff2ae00a36b4f3a4ee786",
        "6287d50b02bdfd201512e632ca6318f0f2df8432270e524eeeabb7312fe59785",
        "f1b9996112f2bda21c391ed68c31caca2c650f200cc8b2ead86076a9ce7ea116",
        "bb36ae47ea312a0199718bb137f508aee00fded15834f1b726c879b7a6174cda",
        "d59644dd0529c0216dd54567fdf7f6b71c4103be17ea6eff71e2449e58a677e5",
    ];

    let (mut provenance, _initial_store, _warp_id, worldline_id) = setup_worldline_with_ticks(10);
    let forked_id = WorldlineId([2u8; 32]);

    provenance
        .fork(worldline_id, 5, forked_id)
        .expect("fork should succeed");

    // fork(src, 5, dst) copies ticks 0..=5 (6 entries)
    let forked_len = provenance.len(forked_id).unwrap();
    assert_eq!(forked_len, 6, "GV-003: fork at 5 should yield 6 entries");

    // Prefix ticks 0..5 must be identical between original and fork
    for (tick, exp_ch) in EXPECTED_PREFIX_COMMITS.iter().enumerate() {
        let original = provenance.expected(worldline_id, tick as u64).unwrap();
        let forked = provenance.expected(forked_id, tick as u64).unwrap();

        assert_eq!(
            original, forked,
            "GV-003 tick {tick}: forked prefix must match original"
        );
        assert_eq!(
            hex(&original.commit_hash),
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
    let mut engine1 = EngineBuilder::new(initial_store.clone(), root).build();
    let disp1 = engine1.ingest_intent(intent_bytes).unwrap();

    // Second engine: ingest same bytes independently
    let mut engine2 = EngineBuilder::new(initial_store.clone(), root).build();
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
}
