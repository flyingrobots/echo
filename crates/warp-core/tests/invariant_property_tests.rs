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
use common::{
    append_fixture_entry, create_add_node_patch, create_initial_store,
    create_initial_worldline_state, fixture_head_key, test_warp_id, test_worldline_id,
};

use proptest::prelude::*;

use warp_core::{
    compute_commit_hash_v2, make_head_id, make_intent_kind, EngineBuilder, Hash, HashTriplet,
    InboxPolicy, IngressDisposition, IngressEnvelope, IngressTarget, LocalProvenanceStore,
    PlaybackHeadRegistry, PlaybackMode, ProvenanceEntry, ProvenanceStore, RunnableWriterSet,
    WorldlineId, WorldlineRuntime, WorldlineState, WorldlineTick, WriterHead, WriterHeadKey,
};

fn wt(raw: u64) -> WorldlineTick {
    WorldlineTick::from_raw(raw)
}

fn runtime_with_default_writer(worldline_id: WorldlineId) -> (WorldlineRuntime, WriterHeadKey) {
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let head_key = WriterHeadKey {
        worldline_id,
        head_id: make_head_id("default"),
    };
    runtime
        .register_writer_head(WriterHead::with_routing(
            head_key,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .unwrap();
    (runtime, head_key)
}

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
        let initial_state = create_initial_worldline_state(warp_id);

        let mut provenance = LocalProvenanceStore::new();
        provenance.register_worldline(worldline_id, warp_id).unwrap();

        let mut current_state = initial_state.clone();
        let mut parents: Vec<Hash> = Vec::new();

        for tick in 0..num_ticks {
            let patch = create_add_node_patch(
                warp_id,
                tick,
                warp_core::GlobalTick::from_raw(tick + 1),
                &format!("node-{tick}"),
            );
            patch
                .apply_to_worldline_state(&mut current_state)
                .expect("apply");
            let state_root = current_state.state_root();
            let commit_hash = compute_commit_hash_v2(
                &state_root, &parents, &patch.patch_digest, patch.header.policy_id,
            );
            let triplet = HashTriplet { state_root, patch_digest: patch.patch_digest, commit_hash };

            append_fixture_entry(&mut provenance, worldline_id, patch, triplet, vec![]).unwrap();
            parents = vec![commit_hash];

            // Invariant: length must equal tick + 1
            prop_assert_eq!(provenance.len(worldline_id).unwrap(), tick + 1);
        }

        // Invariant: attempting to append at a gap must fail
        let gap_tick = num_ticks + 1; // skip one
        let gap_patch = create_add_node_patch(
            warp_id,
            gap_tick,
            warp_core::GlobalTick::from_raw(gap_tick + 1),
            &format!("node-gap-{gap_tick}"),
        );
        let gap_triplet = HashTriplet {
            state_root: [0u8; 32],
            patch_digest: gap_patch.patch_digest,
            commit_hash: [0u8; 32],
        };
        let gap_entry = ProvenanceEntry::local_commit(
            worldline_id,
            wt(gap_tick),
            gap_patch.commit_global_tick(),
            fixture_head_key(worldline_id),
            provenance.tip_ref(worldline_id).unwrap().into_iter().collect(),
            gap_triplet,
            gap_patch,
            vec![],
            vec![],
        );
        let result = provenance.append_local_commit(gap_entry);
        prop_assert!(result.is_err(), "appending at tick gap must fail");

        // Invariant: attempting to re-append at an existing tick must fail
        let dup_tick = num_ticks - 1;
        let dup_patch = create_add_node_patch(
            warp_id,
            dup_tick,
            warp_core::GlobalTick::from_raw(dup_tick + 1),
            &format!("node-dup-{dup_tick}"),
        );
        let dup_triplet = HashTriplet {
            state_root: [0u8; 32],
            patch_digest: dup_patch.patch_digest,
            commit_hash: [0u8; 32],
        };
        let dup_entry = ProvenanceEntry::local_commit(
            worldline_id,
            wt(dup_tick),
            dup_patch.commit_global_tick(),
            fixture_head_key(worldline_id),
            provenance.tip_ref(worldline_id).unwrap().into_iter().collect(),
            dup_triplet,
            dup_patch,
            vec![],
            vec![],
        );
        let dup_result = provenance.append_local_commit(dup_entry);
        prop_assert!(dup_result.is_err(), "re-appending at existing tick must fail");
    }
}

// =============================================================================
// INV-002: Canonical head ordering (deterministic)
// =============================================================================

proptest! {
    /// Heads inserted in any order must always iterate in canonical
    /// `(worldline_id, head_id)` order in the RunnableWriterSet.
    #[test]
    fn inv002_canonical_head_ordering(
        num_worldlines in 1usize..5,
        num_heads_per in 1usize..5,
        shuffle_seed in any::<u64>(),
    ) {
        // Build all keys in canonical order first
        let mut keys: Vec<WriterHeadKey> = Vec::new();
        for w in 0..num_worldlines {
            for h in 0..num_heads_per {
                keys.push(WriterHeadKey {
                    worldline_id: WorldlineId([w as u8; 32]),
                    head_id: make_head_id(&format!("h-{h}")),
                });
            }
        }

        // Shuffle the insertion order deterministically
        let mut insertion_order: Vec<usize> = (0..keys.len()).collect();
        let mut rng = shuffle_seed;
        for i in (1..insertion_order.len()).rev() {
            // Simple xorshift for deterministic shuffle
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let j = (rng as usize) % (i + 1);
            insertion_order.swap(i, j);
        }

        // Insert in shuffled order
        let mut reg = PlaybackHeadRegistry::new();
        for &idx in &insertion_order {
            reg.insert(WriterHead::new(keys[idx], PlaybackMode::Play));
        }

        let mut runnable = RunnableWriterSet::new();
        runnable.rebuild(&reg);

        // Verify set identity: the output must contain exactly the same keys.
        let result: Vec<_> = runnable.iter().copied().collect();
        let mut expected = keys.clone();
        expected.sort_by(|a, b| {
            a.worldline_id
                .cmp(&b.worldline_id)
                .then_with(|| a.head_id.cmp(&b.head_id))
        });
        expected.dedup();
        prop_assert_eq!(result, expected, "runnable set must preserve exact head identity");
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
        let worldline_id = test_worldline_id();
        let (mut runtime1, head_key_1) = runtime_with_default_writer(worldline_id);
        let (mut runtime2, head_key_2) = runtime_with_default_writer(worldline_id);
        let kind = make_intent_kind("test/inv003");

        let env1 = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            kind,
            intent_bytes.clone(),
        );
        let env2 = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            kind,
            intent_bytes.clone(),
        );

        let disp1 = runtime1.ingest(env1.clone()).unwrap();
        let disp2 = runtime2.ingest(env2.clone()).unwrap();

        // Both must be Accepted with the same intent_id
        match (disp1, disp2) {
            (
                IngressDisposition::Accepted {
                    ingress_id: id1,
                    head_key: routed_1,
                },
                IngressDisposition::Accepted {
                    ingress_id: id2,
                    head_key: routed_2,
                },
            ) => {
                prop_assert_eq!(id1, id2, "same bytes must produce same intent_id");
                prop_assert_eq!(routed_1, head_key_1);
                prop_assert_eq!(routed_2, head_key_2);
            }
            _ => prop_assert!(false, "both should be Accepted"),
        }

        // Re-ingestion must be Duplicate
        let dup = runtime1.ingest(env1).unwrap();
        match dup {
            IngressDisposition::Duplicate {
                ingress_id,
                head_key,
            } => {
                prop_assert_eq!(ingress_id, env2.ingress_id());
                prop_assert_eq!(head_key, head_key_1);
            }
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
    let initial_state = create_initial_worldline_state(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    provenance.register_worldline(worldline_a, warp_id).unwrap();
    provenance.register_worldline(worldline_b, warp_id).unwrap();

    // Append 5 ticks to worldline A
    let mut store_a = initial_state.clone();
    let mut parents_a: Vec<Hash> = Vec::new();
    for tick in 0..5 {
        let patch = create_add_node_patch(
            warp_id,
            tick,
            warp_core::GlobalTick::from_raw(tick + 1),
            &format!("a-node-{tick}"),
        );
        patch
            .apply_to_worldline_state(&mut store_a)
            .expect("apply A");
        let sr = store_a.state_root();
        let ch = compute_commit_hash_v2(&sr, &parents_a, &patch.patch_digest, 0);
        let triplet = HashTriplet {
            state_root: sr,
            patch_digest: patch.patch_digest,
            commit_hash: ch,
        };
        append_fixture_entry(&mut provenance, worldline_a, patch, triplet, vec![]).unwrap();
        parents_a = vec![ch];
    }

    // Append 3 ticks to worldline B
    let mut store_b = initial_state.clone();
    let mut parents_b: Vec<Hash> = Vec::new();
    for tick in 0..3 {
        let patch = create_add_node_patch(
            warp_id,
            tick,
            warp_core::GlobalTick::from_raw(101 + tick),
            &format!("b-node-{tick}"),
        );
        patch
            .apply_to_worldline_state(&mut store_b)
            .expect("apply B");
        let sr = store_b.state_root();
        let ch = compute_commit_hash_v2(&sr, &parents_b, &patch.patch_digest, 0);
        let triplet = HashTriplet {
            state_root: sr,
            patch_digest: patch.patch_digest,
            commit_hash: ch,
        };
        append_fixture_entry(&mut provenance, worldline_b, patch, triplet, vec![]).unwrap();
        parents_b = vec![ch];
    }

    // Worldline lengths must be independent
    assert_eq!(provenance.len(worldline_a).unwrap(), 5);
    assert_eq!(provenance.len(worldline_b).unwrap(), 3);

    let triplet_b_before = provenance.entry(worldline_b, wt(2)).unwrap().expected;
    // Isolation is about independent provenance and append behavior, not
    // guaranteed distinct reachable state roots. These fixtures only add
    // unreachable nodes, so both worldlines can legitimately share the same
    // canonical state root while still remaining causally isolated.

    // Appending to A must not change B's length
    let patch = create_add_node_patch(warp_id, 5, warp_core::GlobalTick::from_raw(6), "a-node-5");
    let mut store_a_cont = store_a;
    patch
        .apply_to_worldline_state(&mut store_a_cont)
        .expect("apply A+1");
    let sr = store_a_cont.state_root();
    let ch = compute_commit_hash_v2(&sr, &parents_a, &patch.patch_digest, 0);
    let triplet = HashTriplet {
        state_root: sr,
        patch_digest: patch.patch_digest,
        commit_hash: ch,
    };
    append_fixture_entry(&mut provenance, worldline_a, patch, triplet, vec![]).unwrap();
    assert_eq!(provenance.len(worldline_a).unwrap(), 6);
    assert_eq!(
        provenance.entry(worldline_b, wt(2)).unwrap().expected,
        triplet_b_before,
        "appending to A must not mutate B's latest committed triplet"
    );
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

        let mut engine1 = EngineBuilder::new(initial_store.clone(), root)
            .workers(1)
            .build();
        engine1.ingest_intent(intent_bytes.as_bytes()).unwrap();
        let tx1 = engine1.begin();
        let (snap1, receipt1, patch1) = engine1.commit_with_receipt(tx1).expect("commit 1");

        let mut engine2 = EngineBuilder::new(initial_store, root).workers(1).build();
        engine2.ingest_intent(intent_bytes.as_bytes()).unwrap();
        let tx2 = engine2.begin();
        let (snap2, receipt2, patch2) = engine2.commit_with_receipt(tx2).expect("commit 2");

        prop_assert_eq!(snap1.hash, snap2.hash);
        prop_assert_eq!(snap1.state_root, snap2.state_root);
        prop_assert_eq!(snap1.plan_digest, snap2.plan_digest);
        prop_assert_eq!(snap1.decision_digest, snap2.decision_digest);
        prop_assert_eq!(snap1.rewrites_digest, snap2.rewrites_digest);
        prop_assert_eq!(snap1.patch_digest, snap2.patch_digest);
        prop_assert_eq!(receipt1.digest(), receipt2.digest());
        prop_assert_eq!(patch1.digest(), patch2.digest());
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
    let initial_state = create_initial_worldline_state(warp_id);

    let mut provenance = LocalProvenanceStore::new();
    provenance
        .register_worldline(worldline_id, warp_id)
        .unwrap();

    let mut current_state = initial_state;
    let mut parents: Vec<Hash> = Vec::new();
    let mut recorded_triplets: Vec<HashTriplet> = Vec::new();

    for tick in 0..10 {
        let patch = create_add_node_patch(
            warp_id,
            tick,
            warp_core::GlobalTick::from_raw(tick + 1),
            &format!("node-{tick}"),
        );
        patch
            .apply_to_worldline_state(&mut current_state)
            .expect("apply");
        let sr = current_state.state_root();
        let ch = compute_commit_hash_v2(&sr, &parents, &patch.patch_digest, 0);
        let triplet = HashTriplet {
            state_root: sr,
            patch_digest: patch.patch_digest,
            commit_hash: ch,
        };
        recorded_triplets.push(triplet.clone());
        append_fixture_entry(&mut provenance, worldline_id, patch, triplet, vec![]).unwrap();
        parents = vec![ch];
    }

    // Verify all triplets remain unchanged after all appends
    for (tick, expected) in recorded_triplets.iter().enumerate() {
        let actual = provenance
            .entry(worldline_id, wt(tick as u64))
            .unwrap()
            .expected;
        assert_eq!(
            actual, *expected,
            "tick {tick}: triplet must not change after append"
        );
    }
}
