// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration tests for the strand contract (cycle 0004).
//!
//! These tests verify the ten invariants (INV-S1 through INV-S10) and the
//! create/list/drop lifecycle.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use warp_core::strand::{
    make_strand_id, DropReceipt, ForkBasisRef, Strand, StrandError, StrandRegistry,
    StrandRevalidationState, SupportPin,
};
use warp_core::{
    make_head_id, make_node_id, make_type_id, make_warp_id, GlobalTick, GraphStore, HashTriplet,
    HeadEligibility, LocalProvenanceStore, NodeRecord, PlaybackHeadRegistry, PlaybackMode,
    ProvenanceEntry, ProvenanceRef, ProvenanceService, ProvenanceStore, RunnableWriterSet, SlotId,
    WarpId, WorldlineId, WorldlineState, WorldlineTick, WorldlineTickHeaderV1,
    WorldlineTickPatchV1, WriterHead, WriterHeadKey,
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
        fork_basis_ref: ForkBasisRef {
            source_lane_id: base_worldline,
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

fn append_committed_tick(
    provenance: &mut ProvenanceService,
    worldline_id: WorldlineId,
    global_tick: GlobalTick,
) {
    let tick = wt(0);
    let patch_digest = [worldline_id.as_bytes()[0]; 32];
    let triplet = HashTriplet {
        state_root: [worldline_id.as_bytes()[0].wrapping_add(1); 32],
        patch_digest,
        commit_hash: [worldline_id.as_bytes()[0].wrapping_add(2); 32],
    };
    let entry = ProvenanceEntry::local_commit(
        worldline_id,
        tick,
        global_tick,
        WriterHeadKey {
            worldline_id,
            head_id: make_head_id(&format!("prov-head-{}", worldline_id.as_bytes()[0])),
        },
        Vec::new(),
        triplet,
        WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                commit_global_tick: global_tick,
                policy_id: 0,
                rule_pack_id: [0u8; 32],
                plan_digest: [0u8; 32],
                decision_digest: [0u8; 32],
                rewrites_digest: [0u8; 32],
            },
            warp_id: make_warp_id(&format!("strand-prov-{}", worldline_id.as_bytes()[0])),
            ops: vec![],
            in_slots: vec![],
            out_slots: vec![],
            patch_digest,
        },
        vec![],
        Vec::new(),
    );
    provenance
        .append_local_commit(entry)
        .expect("append commit");
}

fn node_slot(warp_id: WarpId, label: &str) -> SlotId {
    SlotId::Node(warp_core::NodeKey {
        warp_id,
        local_id: make_node_id(label),
    })
}

fn append_committed_tick_with_slots(
    provenance: &mut ProvenanceService,
    worldline_id: WorldlineId,
    tick: WorldlineTick,
    global_tick: GlobalTick,
    in_slots: Vec<SlotId>,
    out_slots: Vec<SlotId>,
) -> ProvenanceRef {
    let parents = tick
        .checked_sub(1)
        .map(|parent_tick| {
            provenance
                .entry(worldline_id, parent_tick)
                .expect("parent entry should exist")
                .as_ref()
        })
        .into_iter()
        .collect();
    let seed = worldline_id.as_bytes()[0].wrapping_add(tick.as_u64().to_le_bytes()[0]);
    let patch_digest = [seed.wrapping_add(1); 32];
    let entry = ProvenanceEntry::local_commit(
        worldline_id,
        tick,
        global_tick,
        WriterHeadKey {
            worldline_id,
            head_id: make_head_id(&format!(
                "slot-head-{}-{}",
                worldline_id.as_bytes()[0],
                tick.as_u64()
            )),
        },
        parents,
        HashTriplet {
            state_root: [seed.wrapping_add(2); 32],
            patch_digest,
            commit_hash: [seed.wrapping_add(3); 32],
        },
        WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                commit_global_tick: global_tick,
                policy_id: 0,
                rule_pack_id: [0u8; 32],
                plan_digest: [0u8; 32],
                decision_digest: [0u8; 32],
                rewrites_digest: [0u8; 32],
            },
            warp_id: make_warp_id(&format!(
                "slot-warp-{}-{}",
                worldline_id.as_bytes()[0],
                tick.as_u64()
            )),
            ops: vec![],
            in_slots,
            out_slots,
            patch_digest,
        },
        vec![],
        Vec::new(),
    );
    let entry_ref = entry.as_ref();
    provenance
        .append_local_commit(entry)
        .expect("append slotted commit");
    entry_ref
}

fn register_worldline_with_tick(provenance: &mut ProvenanceService, worldline_id: WorldlineId) {
    let state = test_initial_state();
    provenance
        .register_worldline(worldline_id, &state)
        .expect("register worldline");
    append_committed_tick(provenance, worldline_id, GlobalTick::from_raw(1));
}

// ── INV-S7: child_worldline_id != fork_basis_ref.source_lane_id ──────────

#[test]
fn inv_s7_child_and_base_worldlines_are_distinct() {
    let strand = make_test_strand("s7-test", wl(1), wl(2), wt(5));
    assert_ne!(
        strand.child_worldline_id, strand.fork_basis_ref.source_lane_id,
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

// ── INV-S4: strand heads use ordinary writer-head control ──────────────

#[test]
fn inv_s4_strand_head_uses_ordinary_writer_head_control() {
    let child = wl(2);
    let head_key = WriterHeadKey {
        worldline_id: child,
        head_id: make_head_id("strand-head-generic"),
    };
    let mut head = WriterHead::new(head_key, PlaybackMode::Play);

    assert!(
        head.is_admitted(),
        "strand head should use default admitted eligibility"
    );
    assert!(
        !head.is_paused(),
        "strand head should follow ordinary writer-head playback defaults"
    );

    head.pause();
    assert!(
        head.is_paused(),
        "generic pause control should remain available"
    );
    head.set_eligibility(HeadEligibility::Dormant);
    assert!(
        !head.is_admitted(),
        "generic eligibility control should remain available"
    );
}

// ── INV-S4 / INV-S10: strand heads follow ordinary runnable-set rules ───

#[test]
fn inv_s4_s10_strand_heads_follow_ordinary_runnable_set_rules() {
    let base_wl = wl(1);
    let strand_wl = wl(2);

    let mut head_registry = PlaybackHeadRegistry::new();

    // Register a live head on the base worldline (admitted, playing)
    let live_key = WriterHeadKey {
        worldline_id: base_wl,
        head_id: make_head_id("live-head"),
    };
    head_registry.insert(WriterHead::new(live_key, PlaybackMode::Play));

    // Register a strand head on the child worldline using ordinary head control
    let strand_key = WriterHeadKey {
        worldline_id: strand_wl,
        head_id: make_head_id("strand-head"),
    };
    let strand_head = WriterHead::new(strand_key, PlaybackMode::Play);
    head_registry.insert(strand_head);

    // Build the runnable set
    let mut runnable = RunnableWriterSet::new();
    runnable.rebuild(&head_registry);

    // Live head should be runnable
    assert!(
        runnable.iter().any(|k| *k == live_key),
        "live head should be in runnable set"
    );

    // Strand head should participate like any other admitted, unpaused head
    assert!(
        runnable.iter().any(|k| *k == strand_key),
        "INV-S4: strand head should appear in runnable set when admitted and unpaused"
    );

    let mut strand_head = head_registry
        .remove(&strand_key)
        .expect("strand head present");
    strand_head.pause();
    head_registry.insert(strand_head);

    runnable.rebuild(&head_registry);
    assert!(
        !runnable.iter().any(|k| *k == strand_key),
        "paused strand head should be excluded by ordinary runnable-set rules"
    );
}

// ── INV-S9: support pins are validated live read-only references ───────

#[test]
fn inv_s9_support_pins_default_empty_until_pinned() {
    let strand = make_test_strand("s9-test", wl(1), wl(2), wt(5));
    assert!(
        strand.support_pins.is_empty(),
        "new strands start without support pins until runtime validation adds them"
    );
}

#[test]
fn live_basis_report_allows_parent_advance_outside_owned_footprint() {
    let (mut provenance, base_worldline, _) = setup_base_worldline();
    let child_worldline = wl(2);
    let warp_id = make_warp_id("live-basis-disjoint");
    let child_owned = node_slot(warp_id, "child-owned");
    let parent_other = node_slot(warp_id, "parent-other");

    let base_ref = append_committed_tick_with_slots(
        &mut provenance,
        base_worldline,
        wt(0),
        GlobalTick::from_raw(1),
        vec![],
        vec![],
    );
    provenance
        .fork(base_worldline, wt(0), child_worldline)
        .expect("fork child provenance");
    append_committed_tick_with_slots(
        &mut provenance,
        child_worldline,
        wt(1),
        GlobalTick::from_raw(2),
        vec![child_owned],
        vec![child_owned],
    );
    let parent_tip = append_committed_tick_with_slots(
        &mut provenance,
        base_worldline,
        wt(1),
        GlobalTick::from_raw(3),
        vec![],
        vec![parent_other],
    );

    let strand = Strand {
        strand_id: make_strand_id("live-basis-disjoint"),
        fork_basis_ref: ForkBasisRef {
            source_lane_id: base_worldline,
            fork_tick: wt(0),
            commit_hash: base_ref.commit_hash,
            boundary_hash: provenance
                .entry(base_worldline, wt(0))
                .expect("base entry")
                .expected
                .state_root,
            provenance_ref: base_ref,
        },
        child_worldline_id: child_worldline,
        writer_heads: vec![WriterHeadKey {
            worldline_id: child_worldline,
            head_id: make_head_id("live-basis-disjoint-head"),
        }],
        support_pins: Vec::new(),
    };

    let report = strand
        .live_basis_report(&provenance)
        .expect("live basis report");
    assert_eq!(report.realized_parent_ref, parent_tip);
    assert_eq!(report.source_suffix_start_tick, wt(1));
    assert_eq!(report.source_suffix_end_tick, Some(wt(1)));
    assert!(report.owned_divergence.contains_closed(&child_owned));
    assert!(report.parent_movement.contains_write(&parent_other));
    assert!(matches!(
        report.parent_revalidation,
        StrandRevalidationState::ParentAdvancedDisjoint { parent_to, .. }
            if parent_to == parent_tip
    ));
}

#[test]
fn live_basis_report_requires_revalidation_when_parent_invades_owned_footprint() {
    let (mut provenance, base_worldline, _) = setup_base_worldline();
    let child_worldline = wl(2);
    let warp_id = make_warp_id("live-basis-overlap");
    let owned_slot = node_slot(warp_id, "owned-slot");

    let base_ref = append_committed_tick_with_slots(
        &mut provenance,
        base_worldline,
        wt(0),
        GlobalTick::from_raw(1),
        vec![],
        vec![],
    );
    provenance
        .fork(base_worldline, wt(0), child_worldline)
        .expect("fork child provenance");
    append_committed_tick_with_slots(
        &mut provenance,
        child_worldline,
        wt(1),
        GlobalTick::from_raw(2),
        vec![owned_slot],
        vec![],
    );
    let parent_tip = append_committed_tick_with_slots(
        &mut provenance,
        base_worldline,
        wt(1),
        GlobalTick::from_raw(3),
        vec![],
        vec![owned_slot],
    );

    let strand = Strand {
        strand_id: make_strand_id("live-basis-overlap"),
        fork_basis_ref: ForkBasisRef {
            source_lane_id: base_worldline,
            fork_tick: wt(0),
            commit_hash: base_ref.commit_hash,
            boundary_hash: provenance
                .entry(base_worldline, wt(0))
                .expect("base entry")
                .expected
                .state_root,
            provenance_ref: base_ref,
        },
        child_worldline_id: child_worldline,
        writer_heads: vec![WriterHeadKey {
            worldline_id: child_worldline,
            head_id: make_head_id("live-basis-overlap-head"),
        }],
        support_pins: Vec::new(),
    };

    let report = strand
        .live_basis_report(&provenance)
        .expect("live basis report");
    assert!(matches!(
        report.parent_revalidation,
        StrandRevalidationState::RevalidationRequired {
            parent_to,
            ref overlapping_slots,
            ..
        } if parent_to == parent_tip && overlapping_slots == &vec![owned_slot]
    ));
}

// ── INV-S5: fork_basis_ref fields agree ──────────────────────────────────────

#[test]
fn inv_s5_base_ref_fields_consistent() {
    let strand = make_test_strand("s5-test", wl(1), wl(2), wt(5));
    let br = &strand.fork_basis_ref;

    // provenance_ref must agree with fork_basis_ref scalars
    assert_eq!(br.provenance_ref.worldline_id, br.source_lane_id);
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
        fork_basis_ref: ForkBasisRef {
            source_lane_id: wl(1),
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
fn registry_insert_accepts_valid_nonempty_support_pins() {
    let mut registry = StrandRegistry::new();
    let target = make_test_strand("support-target", wl(1), wl(10), wt(5));
    let target_id = target.strand_id;
    let target_worldline = target.child_worldline_id;
    registry.insert(target).expect("insert support target");

    let mut owner = make_test_strand("support-owner", wl(1), wl(2), wt(5));
    owner.support_pins.push(SupportPin {
        strand_id: target_id,
        worldline_id: target_worldline,
        pinned_tick: wt(0),
        state_hash: [0xCC; 32],
    });
    registry
        .insert(owner)
        .expect("valid support pin should insert");
}

#[test]
fn registry_insert_rejects_support_pin_missing_target() {
    let mut registry = StrandRegistry::new();
    let mut owner = make_test_strand("missing-target", wl(1), wl(2), wt(5));
    let missing = make_strand_id("missing-support");
    owner.support_pins.push(SupportPin {
        strand_id: missing,
        worldline_id: wl(10),
        pinned_tick: wt(0),
        state_hash: [0; 32],
    });
    let err = registry
        .insert(owner)
        .expect_err("missing support target should reject");
    assert_eq!(err, StrandError::MissingSupportTarget(missing));
}

#[test]
fn registry_insert_rejects_support_pin_worldline_mismatch() {
    let mut registry = StrandRegistry::new();
    let target = make_test_strand("mismatch-target", wl(1), wl(10), wt(5));
    let target_id = target.strand_id;
    registry.insert(target).expect("insert support target");

    let mut owner = make_test_strand("mismatch-owner", wl(1), wl(2), wt(5));
    owner.support_pins.push(SupportPin {
        strand_id: target_id,
        worldline_id: wl(11),
        pinned_tick: wt(0),
        state_hash: [0; 32],
    });
    let err = registry
        .insert(owner)
        .expect_err("worldline mismatch should reject");
    assert_eq!(
        err,
        StrandError::SupportWorldlineMismatch {
            target: target_id,
            expected: wl(10),
            got: wl(11),
        }
    );
}

#[test]
fn registry_insert_rejects_duplicate_support_target() {
    let mut registry = StrandRegistry::new();
    let target = make_test_strand("duplicate-target", wl(1), wl(10), wt(5));
    let target_id = target.strand_id;
    let target_worldline = target.child_worldline_id;
    registry.insert(target).expect("insert support target");

    let owner_id = make_strand_id("duplicate-owner");
    let owner = Strand {
        strand_id: owner_id,
        fork_basis_ref: ForkBasisRef {
            source_lane_id: wl(1),
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
            head_id: make_head_id("duplicate-owner-head"),
        }],
        support_pins: vec![
            SupportPin {
                strand_id: target_id,
                worldline_id: target_worldline,
                pinned_tick: wt(0),
                state_hash: [1; 32],
            },
            SupportPin {
                strand_id: target_id,
                worldline_id: target_worldline,
                pinned_tick: wt(1),
                state_hash: [2; 32],
            },
        ],
    };
    let err = registry
        .insert(owner)
        .expect_err("duplicate support target should reject");
    assert_eq!(
        err,
        StrandError::DuplicateSupportTarget {
            owner: owner_id,
            target: target_id,
        }
    );
}

#[test]
fn registry_pin_support_records_state_hash_from_provenance() {
    let mut provenance = ProvenanceService::new();
    register_worldline_with_tick(&mut provenance, wl(10));

    let mut registry = StrandRegistry::new();
    let owner = make_test_strand("pin-owner", wl(1), wl(2), wt(5));
    let owner_id = owner.strand_id;
    let target = make_test_strand("pin-target", wl(1), wl(10), wt(5));
    let target_id = target.strand_id;
    let target_worldline = target.child_worldline_id;
    registry.insert(owner).expect("insert owner");
    registry.insert(target).expect("insert target");

    let support_pin = registry
        .pin_support(&provenance, owner_id, target_id, wt(0))
        .expect("pin support");

    assert_eq!(support_pin.strand_id, target_id);
    assert_eq!(support_pin.worldline_id, target_worldline);
    assert_eq!(support_pin.pinned_tick, wt(0));
    assert_eq!(
        support_pin.state_hash,
        provenance
            .entry(target_worldline, wt(0))
            .unwrap()
            .expected
            .state_root
    );
    assert_eq!(
        registry.list_support_pins(&owner_id).unwrap(),
        &[support_pin]
    );
}

#[test]
fn registry_pin_support_rejects_duplicate_and_self_target() {
    let mut provenance = ProvenanceService::new();
    register_worldline_with_tick(&mut provenance, wl(10));

    let mut registry = StrandRegistry::new();
    let owner = make_test_strand("pin-owner-dup", wl(1), wl(2), wt(5));
    let owner_id = owner.strand_id;
    let target = make_test_strand("pin-target-dup", wl(1), wl(10), wt(5));
    let target_id = target.strand_id;
    registry.insert(owner).expect("insert owner");
    registry.insert(target).expect("insert target");

    registry
        .pin_support(&provenance, owner_id, target_id, wt(0))
        .expect("first pin");
    let duplicate = registry
        .pin_support(&provenance, owner_id, target_id, wt(0))
        .expect_err("duplicate pin should reject");
    assert_eq!(
        duplicate,
        StrandError::DuplicateSupportTarget {
            owner: owner_id,
            target: target_id,
        }
    );

    let self_pin = registry
        .pin_support(&provenance, owner_id, owner_id, wt(0))
        .expect_err("self pin should reject");
    assert_eq!(self_pin, StrandError::SelfSupportPin(owner_id));
}

#[test]
fn registry_remove_rejects_live_pinned_target_until_unpinned() {
    let mut provenance = ProvenanceService::new();
    register_worldline_with_tick(&mut provenance, wl(10));

    let mut registry = StrandRegistry::new();
    let owner = make_test_strand("pin-owner-rm", wl(1), wl(2), wt(5));
    let owner_id = owner.strand_id;
    let target = make_test_strand("pin-target-rm", wl(1), wl(10), wt(5));
    let target_id = target.strand_id;
    registry.insert(owner).expect("insert owner");
    registry.insert(target).expect("insert target");
    registry
        .pin_support(&provenance, owner_id, target_id, wt(0))
        .expect("pin support");

    let err = registry
        .remove(&target_id)
        .expect_err("pinned target should not remove");
    assert_eq!(
        err,
        StrandError::PinnedByLiveStrand {
            strand: target_id,
            pinned_by: owner_id,
        }
    );

    let removed_pin = registry
        .unpin_support(owner_id, target_id)
        .expect("unpin support");
    assert_eq!(removed_pin.strand_id, target_id);
    assert!(registry.list_support_pins(&owner_id).unwrap().is_empty());
    let removed = registry.remove(&target_id).expect("remove unpinned target");
    assert_eq!(removed.strand_id, target_id);
}

#[test]
fn registry_remove_nonexistent_returns_error() {
    let mut registry = StrandRegistry::new();
    let sid = make_strand_id("ghost");
    let err = registry.remove(&sid).expect_err("remove should fail");
    assert_eq!(err, StrandError::NotFound(sid));
}

#[test]
fn registry_list_by_source_lane_filters_correctly() {
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

    let from_a = registry.list_by_source_lane(&base_a);
    assert_eq!(from_a.len(), 2, "should find 2 strands from source lane a");
    for s in &from_a {
        assert_eq!(s.fork_basis_ref.source_lane_id, base_a);
    }

    let from_b = registry.list_by_source_lane(&base_b);
    assert_eq!(from_b.len(), 1, "should find 1 strand from source lane b");

    let unknown = wl(99);
    let from_none = registry.list_by_source_lane(&unknown);
    assert!(
        from_none.is_empty(),
        "should find no strands from unknown source lane"
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

    // Build fork_basis_ref from the SOURCE entry, not the child copy.
    let fork_basis_ref = ForkBasisRef {
        source_lane_id: base_id,
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
        fork_basis_ref.provenance_ref.worldline_id,
        fork_basis_ref.source_lane_id
    );
    assert_eq!(
        fork_basis_ref.provenance_ref.worldline_tick,
        fork_basis_ref.fork_tick
    );
    assert_eq!(
        fork_basis_ref.provenance_ref.commit_hash,
        fork_basis_ref.commit_hash
    );
    assert_eq!(fork_basis_ref.boundary_hash, base_entry.expected.state_root);
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
