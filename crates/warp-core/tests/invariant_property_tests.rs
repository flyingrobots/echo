// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Phase 0 property tests for ADR-0008/0009 invariant harness.
//!
//! These tests use `proptest` to verify structural invariants that must hold
//! throughout the worldline runtime refactor, regardless of specific hash values.
//!
//! ## Invariants tested
//!
//! | ID | Invariant | ADR |
//! |----|-----------|-----|
//! | INV-001 | Monotonic worldline tick (append-only) | 0008 |
//! | INV-002 | Canonical head ordering (deterministic) | 0008 |
//! | INV-003 | Idempotent ingress (content-addressed) | 0008 |
//! | INV-004 | No shared mutable leakage across worldline boundaries | 0008 |
//! | INV-005 | Commit determinism (same input → same output) | 0008 |
//! | INV-006 | Provenance append-only (no overwrites) | 0008 |
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::redundant_clone,
    clippy::clone_on_copy,
    clippy::match_wildcard_for_single_variants,
    clippy::panic
)]

mod common;
use common::{create_add_node_patch, create_initial_store, test_warp_id, test_worldline_id};

use proptest::prelude::*;

use warp_core::{
    compute_commit_hash_v2, compute_state_root_for_warp_store, EngineBuilder, Hash, HashTriplet,
    LocalProvenanceStore, ProvenanceStore, WorldlineId,
};

// =============================================================================
// INV-001: Monotonic worldline tick (append-only provenance)
// =============================================================================

proptest! {
    /// The provenance store enforces append-only semantics: you can only append
    /// at exactly `len()`, never at a gap or duplicate tick.
    #[test]
    fn inv001_monotonic_worldline_tick(num_ticks in 1u64..20) {
        let warp_id = test_warp_id();
        let worldline_id = test_worldline_id();
        let initial_store = create_initial_store(warp_id);

        let mut provenance = LocalProvenanceStore::new();
        provenance.register_worldline(worldline_id, warp_id).unwrap();

        let mut current_store = initial_store.clone();
        let mut parents: Vec<Hash> = Vec::new();

        for tick in 0..num_ticks {
            let patch = create_add_node_patch(warp_id, tick, &format!("node-{tick}"));
            patch.apply_to_store(&mut current_store).expect("apply");
            let state_root = compute_state_root_for_warp_store(&current_store, warp_id);
            let commit_hash = compute_commit_hash_v2(
                &state_root, &parents, &patch.patch_digest, patch.header.policy_id,
            );
            let triplet = HashTriplet { state_root, patch_digest: patch.patch_digest, commit_hash };

            provenance.append(worldline_id, patch, triplet, vec![]).unwrap();
            parents = vec![commit_hash];

            // Invariant: length must equal tick + 1
            prop_assert_eq!(provenance.len(worldline_id).unwrap(), tick + 1);
        }

        // Invariant: attempting to append at a gap must fail
        let gap_tick = num_ticks + 1; // skip one
        let gap_patch = create_add_node_patch(warp_id, gap_tick, &format!("node-gap-{gap_tick}"));
        let gap_triplet = HashTriplet {
            state_root: [0u8; 32],
            patch_digest: gap_patch.patch_digest,
            commit_hash: [0u8; 32],
        };
        let result = provenance.append(worldline_id, gap_patch, gap_triplet, vec![]);
        prop_assert!(result.is_err(), "appending at tick gap must fail");
    }
}

// =============================================================================
// INV-003: Idempotent ingress (content-addressed)
// =============================================================================

proptest! {
    /// Any byte string ingested into two independent engines must produce
    /// the same content-addressed intent_id.
    #[test]
    fn inv003_idempotent_ingress(intent_bytes in proptest::collection::vec(any::<u8>(), 1..256)) {
        let warp_id = test_warp_id();
        let initial_store = create_initial_store(warp_id);
        let root = warp_core::make_node_id("root");

        let mut engine1 = EngineBuilder::new(initial_store.clone(), root).build();
        let mut engine2 = EngineBuilder::new(initial_store, root).build();

        let disp1 = engine1.ingest_intent(&intent_bytes).unwrap();
        let disp2 = engine2.ingest_intent(&intent_bytes).unwrap();

        // Both must be Accepted with the same intent_id
        match (disp1, disp2) {
            (
                warp_core::IngestDisposition::Accepted { intent_id: id1 },
                warp_core::IngestDisposition::Accepted { intent_id: id2 },
            ) => {
                prop_assert_eq!(id1, id2, "same bytes must produce same intent_id");
            }
            _ => prop_assert!(false, "both should be Accepted"),
        }

        // Re-ingestion must be Duplicate
        let dup = engine1.ingest_intent(&intent_bytes).unwrap();
        match dup {
            warp_core::IngestDisposition::Duplicate { .. } => {}
            _ => prop_assert!(false, "re-ingestion must be Duplicate"),
        }
    }
}

// =============================================================================
// INV-004: No shared mutable leakage across worldline boundaries
// =============================================================================

/// Operations on one worldline must not affect another worldline's provenance.
#[test]
fn inv004_no_cross_worldline_leakage() {
    let warp_id = test_warp_id();
    let worldline_a = WorldlineId([1u8; 32]);
    let worldline_b = WorldlineId([2u8; 32]);
    let initial_store = create_initial_store(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    provenance.register_worldline(worldline_a, warp_id).unwrap();
    provenance.register_worldline(worldline_b, warp_id).unwrap();

    // Append 5 ticks to worldline A
    let mut store_a = initial_store.clone();
    let mut parents_a: Vec<Hash> = Vec::new();
    for tick in 0..5 {
        let patch = create_add_node_patch(warp_id, tick, &format!("a-node-{tick}"));
        patch.apply_to_store(&mut store_a).expect("apply A");
        let sr = compute_state_root_for_warp_store(&store_a, warp_id);
        let ch = compute_commit_hash_v2(&sr, &parents_a, &patch.patch_digest, 0);
        let triplet = HashTriplet {
            state_root: sr,
            patch_digest: patch.patch_digest,
            commit_hash: ch,
        };
        provenance
            .append(worldline_a, patch, triplet, vec![])
            .unwrap();
        parents_a = vec![ch];
    }

    // Append 3 ticks to worldline B
    let mut store_b = initial_store.clone();
    let mut parents_b: Vec<Hash> = Vec::new();
    for tick in 0..3 {
        let patch = create_add_node_patch(warp_id, tick, &format!("b-node-{tick}"));
        patch.apply_to_store(&mut store_b).expect("apply B");
        let sr = compute_state_root_for_warp_store(&store_b, warp_id);
        let ch = compute_commit_hash_v2(&sr, &parents_b, &patch.patch_digest, 0);
        let triplet = HashTriplet {
            state_root: sr,
            patch_digest: patch.patch_digest,
            commit_hash: ch,
        };
        provenance
            .append(worldline_b, patch, triplet, vec![])
            .unwrap();
        parents_b = vec![ch];
    }

    // Worldline lengths must be independent
    assert_eq!(provenance.len(worldline_a).unwrap(), 5);
    assert_eq!(provenance.len(worldline_b).unwrap(), 3);

    // State roots must differ (different node names)
    let sr_a = provenance.expected(worldline_a, 4).unwrap().state_root;
    let sr_b = provenance.expected(worldline_b, 2).unwrap().state_root;
    assert_ne!(
        sr_a, sr_b,
        "different worldlines must have different state roots"
    );

    // Appending to A must not change B's length
    let patch = create_add_node_patch(warp_id, 5, "a-node-5");
    let mut store_a_cont = store_a;
    patch.apply_to_store(&mut store_a_cont).expect("apply A+1");
    let sr = compute_state_root_for_warp_store(&store_a_cont, warp_id);
    let ch = compute_commit_hash_v2(&sr, &parents_a, &patch.patch_digest, 0);
    let triplet = HashTriplet {
        state_root: sr,
        patch_digest: patch.patch_digest,
        commit_hash: ch,
    };
    provenance
        .append(worldline_a, patch, triplet, vec![])
        .unwrap();
    assert_eq!(provenance.len(worldline_a).unwrap(), 6);
    assert_eq!(
        provenance.len(worldline_b).unwrap(),
        3,
        "appending to A must not change B"
    );
}

// =============================================================================
// INV-005: Commit determinism (same input → same output)
// =============================================================================

proptest! {
    /// Two engines built from identical initial state must produce identical
    /// commit hashes when no rewrites are applied.
    #[test]
    fn inv005_commit_determinism(seed in 0u8..255) {
        let warp_id = test_warp_id();
        let initial_store = create_initial_store(warp_id);
        let root = warp_core::make_node_id("root");

        // Optionally ingest a deterministic intent to vary the scenario
        let intent_bytes = format!("intent-seed-{seed}");

        let mut engine1 = EngineBuilder::new(initial_store.clone(), root).build();
        engine1.ingest_intent(intent_bytes.as_bytes()).unwrap();
        let tx1 = engine1.begin();
        let snap1 = engine1.commit(tx1).expect("commit 1");

        let mut engine2 = EngineBuilder::new(initial_store, root).build();
        engine2.ingest_intent(intent_bytes.as_bytes()).unwrap();
        let tx2 = engine2.begin();
        let snap2 = engine2.commit(tx2).expect("commit 2");

        prop_assert_eq!(snap1.hash, snap2.hash);
        prop_assert_eq!(snap1.state_root, snap2.state_root);
        prop_assert_eq!(snap1.patch_digest, snap2.patch_digest);
    }
}

// =============================================================================
// INV-006: Provenance append-only (no overwrites)
// =============================================================================

/// Once a tick is appended, its hash triplet must never change.
#[test]
fn inv006_provenance_immutable_after_append() {
    let warp_id = test_warp_id();
    let worldline_id = test_worldline_id();
    let initial_store = create_initial_store(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    provenance
        .register_worldline(worldline_id, warp_id)
        .unwrap();

    let mut current_store = initial_store;
    let mut parents: Vec<Hash> = Vec::new();
    let mut recorded_triplets: Vec<HashTriplet> = Vec::new();

    for tick in 0..10 {
        let patch = create_add_node_patch(warp_id, tick, &format!("node-{tick}"));
        patch.apply_to_store(&mut current_store).expect("apply");
        let sr = compute_state_root_for_warp_store(&current_store, warp_id);
        let ch = compute_commit_hash_v2(&sr, &parents, &patch.patch_digest, 0);
        let triplet = HashTriplet {
            state_root: sr,
            patch_digest: patch.patch_digest,
            commit_hash: ch,
        };
        recorded_triplets.push(triplet.clone());
        provenance
            .append(worldline_id, patch, triplet, vec![])
            .unwrap();
        parents = vec![ch];
    }

    // Verify all triplets remain unchanged after all appends
    for (tick, expected) in recorded_triplets.iter().enumerate() {
        let actual = provenance.expected(worldline_id, tick as u64).unwrap();
        assert_eq!(
            actual, *expected,
            "tick {tick}: triplet must not change after append"
        );
    }
}
