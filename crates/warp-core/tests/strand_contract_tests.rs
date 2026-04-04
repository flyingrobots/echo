// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration tests for the strand contract (cycle 0004).
//!
//! These tests verify the ten invariants (INV-S1 through INV-S10) and the
//! create/list/drop lifecycle.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use warp_core::strand::{
    make_strand_id, BaseRef, DropReceipt, Strand, StrandError, StrandRegistry,
};
use warp_core::{
    make_head_id, make_node_id, make_type_id, make_warp_id, GlobalTick, GraphStore, HashTriplet,
    HeadEligibility, LocalProvenanceStore, NodeRecord, PlaybackHeadRegistry, PlaybackMode,
    ProvenanceEntry, ProvenanceRef, ProvenanceService, ProvenanceStore, RunnableWriterSet,
    WorldlineId, WorldlineState, WorldlineTick, WorldlineTickHeaderV1, WorldlineTickPatchV1,
    WriterHead, WriterHeadKey,
};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn wl(n: u8) -> WorldlineId {
    WorldlineId::from_bytes([n; 32])
}

fn wt(n: u64) -> WorldlineTick {
    WorldlineTick::from_raw(n)
}

fn test_initial_state() -> WorldlineState {
    let warp_id = make_warp_id("strand-test-warp");
    let root = make_node_id("strand-test-root");
    let mut store = GraphStore::new(warp_id);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("StrandTestRoot"),
        },
    );
    WorldlineState::from_root_store(store, root).expect("test initial state")
}

/// Create a provenance service with a registered worldline that has some
/// committed ticks, suitable for forking.
fn setup_base_worldline() -> (ProvenanceService, WorldlineId, WorldlineState) {
    let mut provenance = ProvenanceService::new();
    let base_id = wl(1);
    let initial_state = test_initial_state();

    provenance
        .register_worldline(base_id, &initial_state)
        .expect("register base worldline");

    (provenance, base_id, initial_state)
}

/// Build a strand with explicit base/child worldlines for invariant violation tests.
fn make_test_strand_raw(base_worldline: WorldlineId, child_worldline: WorldlineId) -> Strand {
    make_test_strand("raw", base_worldline, child_worldline, wt(5))
}

/// Build a strand by hand (without full engine integration) to test the
/// registry and type invariants.
fn make_test_strand(
    strand_label: &str,
    base_worldline: WorldlineId,
    child_worldline: WorldlineId,
    fork_tick: WorldlineTick,
) -> Strand {
    let strand_id = make_strand_id(strand_label);
    let head_key = WriterHeadKey {
        worldline_id: child_worldline,
        head_id: make_head_id(&format!("strand-head-{strand_label}")),
    };
    let commit_hash = [0xAA; 32];
    let boundary_hash = [0xBB; 32];

    Strand {
        strand_id,
        base_ref: BaseRef {
            source_worldline_id: base_worldline,
            fork_tick,
            commit_hash,
            boundary_hash,
            provenance_ref: ProvenanceRef {
                worldline_id: base_worldline,
                worldline_tick: fork_tick,
                commit_hash,
            },
        },
        child_worldline_id: child_worldline,
        writer_heads: vec![head_key],
        support_pins: Vec::new(),
    }
}

// ── INV-S7: child_worldline_id != base_ref.source_worldline_id ──────────

#[test]
fn inv_s7_child_and_base_worldlines_are_distinct() {
    let strand = make_test_strand("s7-test", wl(1), wl(2), wt(5));
    assert_ne!(
        strand.child_worldline_id, strand.base_ref.source_worldline_id,
        "INV-S7: child worldline must differ from base"
    );
}

// ── INV-S2 / INV-S8: own heads, head ownership ─────────────────────────

#[test]
fn inv_s2_s8_strand_heads_belong_to_child_worldline() {
    let base = wl(1);
    let child = wl(2);
    let strand = make_test_strand("s2-test", base, child, wt(5));

    for head_key in &strand.writer_heads {
        assert_eq!(
            head_key.worldline_id, child,
            "INV-S8: every writer head must belong to child_worldline_id"
        );
        assert_ne!(
            head_key.worldline_id, base,
            "INV-S2: writer heads must not belong to base worldline"
        );
    }
}

// ── INV-S4: strand heads are Dormant and Paused ────────────────────────

#[test]
fn inv_s4_strand_head_created_dormant_and_paused() {
    let child = wl(2);
    let head_key = WriterHeadKey {
        worldline_id: child,
        head_id: make_head_id("strand-head-dormant"),
    };
    let head = WriterHead::new(head_key, PlaybackMode::Paused);

    assert!(head.is_paused(), "strand head must be created paused");
    // Dormant must be set explicitly
    let mut head = head;
    head.set_eligibility(HeadEligibility::Dormant);
    assert!(
        !head.is_admitted(),
        "strand head must not be admitted (Dormant)"
    );
}

// ── INV-S4 / INV-S10: strand heads excluded from live scheduler ────────

#[test]
fn inv_s4_s10_dormant_strand_heads_excluded_from_runnable_set() {
    let base_wl = wl(1);
    let strand_wl = wl(2);

    let mut head_registry = PlaybackHeadRegistry::new();

    // Register a live head on the base worldline (admitted, playing)
    let live_key = WriterHeadKey {
        worldline_id: base_wl,
        head_id: make_head_id("live-head"),
    };
    head_registry.insert(WriterHead::new(live_key, PlaybackMode::Play));

    // Register a strand head on the child worldline (dormant, paused)
    let strand_key = WriterHeadKey {
        worldline_id: strand_wl,
        head_id: make_head_id("strand-head"),
    };
    let mut strand_head = WriterHead::new(strand_key, PlaybackMode::Paused);
    strand_head.set_eligibility(HeadEligibility::Dormant);
    head_registry.insert(strand_head);

    // Build the runnable set
    let mut runnable = RunnableWriterSet::new();
    runnable.rebuild(&head_registry);

    // Live head should be runnable
    assert!(
        runnable.iter().any(|k| *k == live_key),
        "live head should be in runnable set"
    );

    // Strand head must NOT be runnable
    assert!(
        !runnable.iter().any(|k| *k == strand_key),
        "INV-S4/S10: dormant strand head must not appear in runnable set"
    );
}

// ── INV-S9: support_pins must be empty in v1 ───────────────────────────

#[test]
fn inv_s9_support_pins_empty_on_creation() {
    let strand = make_test_strand("s9-test", wl(1), wl(2), wt(5));
    assert!(
        strand.support_pins.is_empty(),
        "INV-S9: support_pins must be empty in v1"
    );
}

// ── INV-S5: base_ref fields agree ──────────────────────────────────────

#[test]
fn inv_s5_base_ref_fields_consistent() {
    let strand = make_test_strand("s5-test", wl(1), wl(2), wt(5));
    let br = &strand.base_ref;

    // provenance_ref must agree with base_ref scalars
    assert_eq!(br.provenance_ref.worldline_id, br.source_worldline_id);
    assert_eq!(br.provenance_ref.worldline_tick, br.fork_tick);
    assert_eq!(br.provenance_ref.commit_hash, br.commit_hash);
}

// ── StrandRegistry: insert / get / contains / list / remove ─────────────

#[test]
fn registry_insert_and_get() {
    let mut registry = StrandRegistry::new();
    let strand = make_test_strand("reg-1", wl(1), wl(2), wt(5));
    let sid = strand.strand_id;

    registry.insert(strand).expect("insert");
    assert!(registry.contains(&sid));
    assert!(registry.get(&sid).is_some());
    assert_eq!(registry.len(), 1);
}

#[test]
fn registry_duplicate_insert_fails() {
    let mut registry = StrandRegistry::new();
    let strand = make_test_strand("dup", wl(1), wl(2), wt(5));
    let sid = strand.strand_id;

    registry.insert(strand.clone()).expect("first insert");
    let err = registry.insert(strand).expect_err("duplicate insert");
    assert_eq!(err, StrandError::AlreadyExists(sid));
}

#[test]
fn registry_remove_returns_strand_and_clears() {
    let mut registry = StrandRegistry::new();
    let strand = make_test_strand("rm-1", wl(1), wl(2), wt(5));
    let sid = strand.strand_id;

    registry.insert(strand).expect("insert");
    let removed = registry.remove(&sid).expect("remove should succeed");
    assert_eq!(removed.strand_id, sid);
    assert!(
        !registry.contains(&sid),
        "strand should be gone after remove"
    );
    assert!(registry.get(&sid).is_none());
    assert_eq!(registry.len(), 0);
}

#[test]
fn registry_insert_rejects_inv_s7_same_worldline() {
    let mut registry = StrandRegistry::new();
    // child == base violates INV-S7
    let strand = make_test_strand_raw(wl(1), wl(1));
    let err = registry.insert(strand).expect_err("INV-S7 should reject");
    assert!(
        matches!(err, StrandError::InvariantViolation(_)),
        "expected InvariantViolation, got {err:?}"
    );
}

#[test]
fn registry_insert_rejects_inv_s8_wrong_head_worldline() {
    let mut registry = StrandRegistry::new();
    let strand_id = make_strand_id("s8-bad");
    let strand = Strand {
        strand_id,
        base_ref: BaseRef {
            source_worldline_id: wl(1),
            fork_tick: wt(5),
            commit_hash: [0xAA; 32],
            boundary_hash: [0xBB; 32],
            provenance_ref: ProvenanceRef {
                worldline_id: wl(1),
                worldline_tick: wt(5),
                commit_hash: [0xAA; 32],
            },
        },
        child_worldline_id: wl(2),
        // Head belongs to wl(3), not wl(2) — violates INV-S8
        writer_heads: vec![WriterHeadKey {
            worldline_id: wl(3),
            head_id: make_head_id("wrong-wl-head"),
        }],
        support_pins: Vec::new(),
    };
    let err = registry.insert(strand).expect_err("INV-S8 should reject");
    assert!(
        matches!(err, StrandError::InvariantViolation(_)),
        "expected InvariantViolation, got {err:?}"
    );
}

#[test]
fn registry_insert_rejects_inv_s9_nonempty_support_pins() {
    use warp_core::strand::SupportPin;

    let mut registry = StrandRegistry::new();
    let strand_id = make_strand_id("s9-bad");
    let strand = Strand {
        strand_id,
        base_ref: BaseRef {
            source_worldline_id: wl(1),
            fork_tick: wt(5),
            commit_hash: [0xAA; 32],
            boundary_hash: [0xBB; 32],
            provenance_ref: ProvenanceRef {
                worldline_id: wl(1),
                worldline_tick: wt(5),
                commit_hash: [0xAA; 32],
            },
        },
        child_worldline_id: wl(2),
        writer_heads: vec![WriterHeadKey {
            worldline_id: wl(2),
            head_id: make_head_id("s9-head"),
        }],
        support_pins: vec![SupportPin {
            strand_id: make_strand_id("pinned"),
            worldline_id: wl(10),
            pinned_tick: wt(0),
            state_hash: [0; 32],
        }],
    };
    let err = registry.insert(strand).expect_err("INV-S9 should reject");
    assert!(
        matches!(err, StrandError::InvariantViolation(_)),
        "expected InvariantViolation, got {err:?}"
    );
}

#[test]
fn registry_remove_nonexistent_returns_error() {
    let mut registry = StrandRegistry::new();
    let sid = make_strand_id("ghost");
    let err = registry.remove(&sid).expect_err("remove should fail");
    assert_eq!(err, StrandError::NotFound(sid));
}

#[test]
fn registry_list_by_base_filters_correctly() {
    let mut registry = StrandRegistry::new();
    let base_a = wl(1);
    let base_b = wl(10);

    registry
        .insert(make_test_strand("a1", base_a, wl(2), wt(5)))
        .unwrap();
    registry
        .insert(make_test_strand("a2", base_a, wl(3), wt(5)))
        .unwrap();
    registry
        .insert(make_test_strand("b1", base_b, wl(4), wt(5)))
        .unwrap();

    let from_a = registry.list_by_base(&base_a);
    assert_eq!(from_a.len(), 2, "should find 2 strands from base_a");
    for s in &from_a {
        assert_eq!(s.base_ref.source_worldline_id, base_a);
    }

    let from_b = registry.list_by_base(&base_b);
    assert_eq!(from_b.len(), 1, "should find 1 strand from base_b");

    let unknown = wl(99);
    let from_none = registry.list_by_base(&unknown);
    assert!(
        from_none.is_empty(),
        "should find no strands from unknown base"
    );
}

// ── Writer heads cardinality (v1: exactly 1) ────────────────────────────

#[test]
fn v1_strand_has_exactly_one_writer_head() {
    let strand = make_test_strand("card-1", wl(1), wl(2), wt(5));
    assert_eq!(
        strand.writer_heads.len(),
        1,
        "v1 strands must have exactly one writer head"
    );
}

// ── Provenance fork creates child worldline with correct prefix ─────────

#[test]
fn provenance_fork_creates_child_with_prefix() {
    let (mut provenance, base_id, _initial_state) = setup_base_worldline();
    let child_id = wl(2);

    // The base worldline has 0 ticks committed (just registered).
    // We need at least one committed tick to fork from.
    // For now, verify that fork on an empty worldline fails gracefully.
    let result = provenance.fork(base_id, wt(0), child_id);

    // With no committed entries, fork at tick 0 should fail because
    // there's no provenance entry at that tick.
    assert!(
        result.is_err(),
        "fork should fail when no entries exist at fork_tick"
    );
}

// ── Happy-path fork: commit entries, fork, verify child prefix ──────────

#[test]
fn provenance_fork_happy_path_child_has_correct_prefix() {
    let base_id = wl(1);
    let child_id = wl(2);
    let warp_id = make_warp_id("fork-test-warp");

    let mut store = LocalProvenanceStore::new();
    store
        .register_worldline(base_id, warp_id)
        .expect("register");

    let head_key = WriterHeadKey {
        worldline_id: base_id,
        head_id: make_head_id("fork-test-head"),
    };

    // Commit 3 ticks (0, 1, 2) to the base worldline.
    let mut parents = Vec::new();
    for tick in 0_u8..3 {
        let tick_u64 = u64::from(tick);
        let triplet = HashTriplet {
            state_root: [tick + 1; 32],
            patch_digest: [tick + 0x10; 32],
            commit_hash: [tick + 0x20; 32],
        };
        let entry = ProvenanceEntry::local_commit(
            base_id,
            wt(tick_u64),
            GlobalTick::from_raw(tick_u64),
            head_key,
            parents,
            triplet,
            WorldlineTickPatchV1 {
                header: WorldlineTickHeaderV1 {
                    commit_global_tick: GlobalTick::from_raw(tick_u64),
                    policy_id: 0,
                    rule_pack_id: [0u8; 32],
                    plan_digest: [0u8; 32],
                    decision_digest: [0u8; 32],
                    rewrites_digest: [0u8; 32],
                },
                warp_id,
                ops: vec![],
                in_slots: vec![],
                out_slots: vec![],
                patch_digest: [tick; 32],
            },
            vec![],
            Vec::new(),
        );
        parents = vec![entry.as_ref()];
        store.append_local_commit(entry).expect("append");
    }

    // Fork at tick 1 (last included tick = 1, child gets ticks 0 and 1).
    store.fork(base_id, wt(1), child_id).expect("fork");

    // Verify child has exactly 2 entries (ticks 0 and 1).
    assert_eq!(store.len(child_id).expect("child len"), 2);

    // Fetch the SOURCE entry (not the child copy) for ground-truth comparison.
    let base_entry = store.entry(base_id, wt(1)).expect("base entry at tick 1");
    let child_entry = store.entry(child_id, wt(1)).expect("child entry at tick 1");

    // Verify fork preserved commit hashes between source and child.
    assert_eq!(
        child_entry.expected.commit_hash, base_entry.expected.commit_hash,
        "child entry commit_hash should match base entry"
    );
    assert_eq!(
        child_entry.expected.state_root, base_entry.expected.state_root,
        "child entry state_root should match base entry"
    );

    // Verify the child's worldline ID was rewritten.
    assert_eq!(child_entry.worldline_id, child_id);

    // Build base_ref from the SOURCE entry, not the child copy.
    let base_ref = BaseRef {
        source_worldline_id: base_id,
        fork_tick: wt(1),
        commit_hash: base_entry.expected.commit_hash,
        boundary_hash: base_entry.expected.state_root,
        provenance_ref: ProvenanceRef {
            worldline_id: base_id,
            worldline_tick: wt(1),
            commit_hash: base_entry.expected.commit_hash,
        },
    };

    // INV-S5: all fields agree with source coordinate.
    assert_eq!(
        base_ref.provenance_ref.worldline_id,
        base_ref.source_worldline_id
    );
    assert_eq!(base_ref.provenance_ref.worldline_tick, base_ref.fork_tick);
    assert_eq!(base_ref.provenance_ref.commit_hash, base_ref.commit_hash);
    assert_eq!(base_ref.boundary_hash, base_entry.expected.state_root);
}

// ── Drop receipt carries correct fields ─────────────────────────────────

#[test]
fn drop_receipt_carries_correct_fields() {
    let strand = make_test_strand("drop-test", wl(1), wl(2), wt(5));
    let receipt = DropReceipt {
        strand_id: strand.strand_id,
        child_worldline_id: strand.child_worldline_id,
        final_tick: wt(10),
    };

    assert_eq!(receipt.strand_id, strand.strand_id);
    assert_eq!(receipt.child_worldline_id, wl(2));
    assert_eq!(receipt.final_tick, wt(10));
}
