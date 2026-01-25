// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Slice Theorem Executable Proof.
//!
//! Proves that parallel execution with footprint enforcement produces
//! deterministic, replayable results across all valid execution orderings.
//!
//! # Seven Phases
//!
//! 1. **Parallel Execution** — 5 ticks with dependent + independent rules
//! 2. **Playback Replay** — seek cursor matches recorded state
//! 3. **Per-Tick Verification** — every intermediate hash matches
//! 4. **Permutation Independence** — shuffled independent items → same result
//! 5. **Multi-Worker Invariance** — 1/2/4/8 workers → same hashes
//! 6. **Semantic Correctness** — dependent chain produces correct values
//! 7. **Cross-Warp Enforcement** — cross-warp emission is rejected

mod common;

use std::panic::{catch_unwind, AssertUnwindSafe};

use common::XorShift64;
use warp_core::{
    compute_commit_hash_v2, compute_state_root_for_warp_store, HashTriplet, LocalProvenanceStore,
    WorldlineTickHeaderV1, WorldlineTickPatchV1,
};
use warp_core::{
    make_node_id, make_type_id, make_warp_id, ApplyResult, AtomPayload, AttachmentKey,
    AttachmentSet, AttachmentValue, ConflictPolicy, CursorId, CursorRole, EdgeSet, EngineBuilder,
    Footprint, FootprintViolation, GraphStore, GraphView, NodeId, NodeKey, NodeRecord, NodeSet,
    PatternGraph, PlaybackCursor, PortSet, RewriteRule, TickDelta, ViolationKind, WarpOp,
    WorldlineId,
};

// =============================================================================
// Constants
// =============================================================================

const R1_NAME: &str = "slice/r1";
const R2_NAME: &str = "slice/r2";
const R3_NAME: &str = "slice/r3";
const R4_NAME: &str = "slice/r4";
const R5_NAME: &str = "slice/r5";
const R6_NAME: &str = "slice/r6_cross_warp";

const NUM_TICKS: u64 = 5;

// Deterministic node IDs
const NODE_NAMES: [&str; 10] = [
    "slice/A", "slice/B", "slice/C", "slice/D", "slice/E", "slice/F", "slice/G", "slice/H",
    "slice/I", "slice/J",
];

fn node_id(idx: usize) -> NodeId {
    make_node_id(NODE_NAMES[idx])
}

fn slice_marker_type() -> warp_core::TypeId {
    make_type_id("slice/marker")
}

fn rule_id(name: &str) -> warp_core::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:");
    hasher.update(name.as_bytes());
    hasher.finalize().into()
}

// =============================================================================
// Rule definitions
// =============================================================================

// R1: reads A, writes B attachment (writes known value V)
fn r1_executor(view: GraphView<'_>, _scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(&node_id(0));
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id: view.warp_id(),
        local_id: node_id(1),
    });
    delta.push(WarpOp::SetAttachment {
        key,
        value: Some(AttachmentValue::Atom(AtomPayload {
            type_id: slice_marker_type(),
            bytes: bytes::Bytes::from_static(b"r1-wrote-this"),
        })),
    });
}

fn r1_footprint(view: GraphView<'_>, _scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut a_write = AttachmentSet::default();
    n_read.insert_with_warp(warp_id, node_id(0));
    a_write.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id(1),
    }));
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write,
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 1,
    }
}

fn r1_rule() -> RewriteRule {
    RewriteRule {
        id: rule_id(R1_NAME),
        name: R1_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |view, scope| view.node(scope).is_some(),
        executor: r1_executor,
        compute_footprint: r1_footprint,
        factor_mask: 1,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

// R2: reads C, writes D attachment (independent)
fn r2_executor(view: GraphView<'_>, _scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(&node_id(2));
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id: view.warp_id(),
        local_id: node_id(3),
    });
    delta.push(WarpOp::SetAttachment {
        key,
        value: Some(AttachmentValue::Atom(AtomPayload {
            type_id: slice_marker_type(),
            bytes: bytes::Bytes::from_static(b"r2-marker"),
        })),
    });
}

fn r2_footprint(view: GraphView<'_>, _scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut a_write = AttachmentSet::default();
    n_read.insert_with_warp(warp_id, node_id(2));
    a_write.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id(3),
    }));
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write,
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 1,
    }
}

fn r2_rule() -> RewriteRule {
    RewriteRule {
        id: rule_id(R2_NAME),
        name: R2_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |view, scope| view.node(scope).is_some(),
        executor: r2_executor,
        compute_footprint: r2_footprint,
        factor_mask: 1,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

// R3: reads E, writes F attachment (independent)
fn r3_executor(view: GraphView<'_>, _scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(&node_id(4));
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id: view.warp_id(),
        local_id: node_id(5),
    });
    delta.push(WarpOp::SetAttachment {
        key,
        value: Some(AttachmentValue::Atom(AtomPayload {
            type_id: slice_marker_type(),
            bytes: bytes::Bytes::from_static(b"r3-marker"),
        })),
    });
}

fn r3_footprint(view: GraphView<'_>, _scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut a_write = AttachmentSet::default();
    n_read.insert_with_warp(warp_id, node_id(4));
    a_write.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id(5),
    }));
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write,
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 1,
    }
}

fn r3_rule() -> RewriteRule {
    RewriteRule {
        id: rule_id(R3_NAME),
        name: R3_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |view, scope| view.node(scope).is_some(),
        executor: r3_executor,
        compute_footprint: r3_footprint,
        factor_mask: 1,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

// R4: reads B attachment, writes G attachment (DEPENDENT on R1 — R1 writes B)
fn r4_executor(view: GraphView<'_>, _scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(&node_id(1));
    let attachment = view.node_attachment(&node_id(1));
    // Transform: if R1 has written, produce "r4-saw-r1", else "r4-no-input"
    let output = match attachment {
        Some(AttachmentValue::Atom(payload)) if payload.bytes.as_ref() == b"r1-wrote-this" => {
            b"r4-saw-r1" as &[u8]
        }
        _ => b"r4-no-input" as &[u8],
    };
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id: view.warp_id(),
        local_id: node_id(6),
    });
    delta.push(WarpOp::SetAttachment {
        key,
        value: Some(AttachmentValue::Atom(AtomPayload {
            type_id: slice_marker_type(),
            bytes: bytes::Bytes::copy_from_slice(output),
        })),
    });
}

fn r4_footprint(view: GraphView<'_>, _scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut a_read = AttachmentSet::default();
    let mut a_write = AttachmentSet::default();
    n_read.insert_with_warp(warp_id, node_id(1));
    a_read.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id(1),
    }));
    a_write.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id(6),
    }));
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read,
        a_write,
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 1,
    }
}

fn r4_rule() -> RewriteRule {
    RewriteRule {
        id: rule_id(R4_NAME),
        name: R4_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |view, scope| view.node(scope).is_some(),
        executor: r4_executor,
        compute_footprint: r4_footprint,
        factor_mask: 1,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

// R5: reads H, writes I attachment (independent)
fn r5_executor(view: GraphView<'_>, _scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(&node_id(7));
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id: view.warp_id(),
        local_id: node_id(8),
    });
    delta.push(WarpOp::SetAttachment {
        key,
        value: Some(AttachmentValue::Atom(AtomPayload {
            type_id: slice_marker_type(),
            bytes: bytes::Bytes::from_static(b"r5-marker"),
        })),
    });
}

fn r5_footprint(view: GraphView<'_>, _scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut a_write = AttachmentSet::default();
    n_read.insert_with_warp(warp_id, node_id(7));
    a_write.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: node_id(8),
    }));
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write,
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 1,
    }
}

fn r5_rule() -> RewriteRule {
    RewriteRule {
        id: rule_id(R5_NAME),
        name: R5_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |view, scope| view.node(scope).is_some(),
        executor: r5_executor,
        compute_footprint: r5_footprint,
        factor_mask: 1,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

// R6: reads J (in engine's root warp), attempts cross-warp emission into W2
fn r6_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let _ = view.node(scope);
    // Attempt to emit into W2 (wrong warp — our engine always uses make_warp_id("root"))
    let w2 = make_warp_id("slice-w2");
    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id: w2,
            local_id: node_id(0),
        },
        record: NodeRecord {
            ty: make_type_id("attack"),
        },
    });
}

fn r6_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    n_read.insert_with_warp(warp_id, *scope);
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read: AttachmentSet::default(),
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 1,
    }
}

fn r6_rule() -> RewriteRule {
    RewriteRule {
        id: rule_id(R6_NAME),
        name: R6_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: |view, scope| view.node(scope).is_some(),
        executor: r6_executor,
        compute_footprint: r6_footprint,
        factor_mask: 1,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

// =============================================================================
// Store setup
// =============================================================================

/// Creates a store with nodes A-J for the root warp.
fn create_slice_store() -> (GraphStore, NodeId) {
    let mut store = GraphStore::default(); // warp = make_warp_id("root")
    let node_ty = make_type_id("slice/node");
    let root = node_id(0);

    for node in &[
        node_id(0),
        node_id(1),
        node_id(2),
        node_id(3),
        node_id(4),
        node_id(5),
        node_id(6),
        node_id(7),
        node_id(8),
        node_id(9),
    ] {
        store.insert_node(*node, NodeRecord { ty: node_ty });
    }

    (store, root)
}

/// Runs one full execution of NUM_TICKS ticks with given worker count.
/// Returns recorded (state_roots, patch_digests, commit_hashes) per tick,
/// plus the final store clone.
#[allow(clippy::type_complexity)]
fn run_n_ticks(workers: usize) -> (Vec<[u8; 32]>, Vec<[u8; 32]>, Vec<[u8; 32]>, GraphStore) {
    let (store, root) = create_slice_store();
    let mut engine = EngineBuilder::new(store, root).workers(workers).build();

    engine.register_rule(r1_rule()).expect("r1");
    engine.register_rule(r2_rule()).expect("r2");
    engine.register_rule(r3_rule()).expect("r3");
    engine.register_rule(r4_rule()).expect("r4");
    engine.register_rule(r5_rule()).expect("r5");

    let mut state_roots = Vec::new();
    let mut patch_digests = Vec::new();
    let mut commit_hashes = Vec::new();

    for _tick in 0..NUM_TICKS {
        let tx = engine.begin();
        // Apply R1-R5 to their respective scope nodes
        assert!(matches!(
            engine.apply(tx, R1_NAME, &node_id(0)).unwrap(),
            ApplyResult::Applied
        ));
        assert!(matches!(
            engine.apply(tx, R2_NAME, &node_id(2)).unwrap(),
            ApplyResult::Applied
        ));
        assert!(matches!(
            engine.apply(tx, R3_NAME, &node_id(4)).unwrap(),
            ApplyResult::Applied
        ));
        assert!(matches!(
            engine.apply(tx, R4_NAME, &node_id(1)).unwrap(),
            ApplyResult::Applied
        ));
        assert!(matches!(
            engine.apply(tx, R5_NAME, &node_id(7)).unwrap(),
            ApplyResult::Applied
        ));

        let (snapshot, _receipt, _patch) = engine.commit_with_receipt(tx).expect("commit");
        state_roots.push(snapshot.state_root);
        patch_digests.push(snapshot.patch_digest);
        commit_hashes.push(snapshot.hash);
    }

    let final_store = engine.store_clone();
    (state_roots, patch_digests, commit_hashes, final_store)
}

// =============================================================================
// Phase 1 + 5: Parallel Execution + Multi-Worker Invariance
// =============================================================================

#[test]
fn phase_1_and_5_multi_worker_invariance() {
    // Execute with multiple worker counts and verify identical hashes.
    let worker_counts = [1, 2, 4, 8];

    let (ref_roots, ref_patches, ref_commits, _) = run_n_ticks(worker_counts[0]);

    for &workers in &worker_counts[1..] {
        let (roots, patches, commits, _) = run_n_ticks(workers);

        for tick in 0..NUM_TICKS as usize {
            assert_eq!(
                roots[tick], ref_roots[tick],
                "state_root mismatch at tick {tick} with {workers} workers"
            );
            assert_eq!(
                patches[tick], ref_patches[tick],
                "patch_digest mismatch at tick {tick} with {workers} workers"
            );
            assert_eq!(
                commits[tick], ref_commits[tick],
                "commit_hash mismatch at tick {tick} with {workers} workers"
            );
        }
    }
}

// =============================================================================
// Phase 2 + 3: Playback Replay + Per-Tick Verification
// =============================================================================

#[test]
fn phase_2_and_3_playback_replay_matches_execution() {
    let (store, root) = create_slice_store();
    let warp_id = store.warp_id();
    let mut engine = EngineBuilder::new(store.clone(), root).workers(4).build();

    engine.register_rule(r1_rule()).expect("r1");
    engine.register_rule(r2_rule()).expect("r2");
    engine.register_rule(r3_rule()).expect("r3");
    engine.register_rule(r4_rule()).expect("r4");
    engine.register_rule(r5_rule()).expect("r5");

    // Build provenance store from execution.
    // IMPORTANT: We must compute state_root using compute_state_root_for_warp_store
    // (same function PlaybackCursor::seek_to uses), NOT the engine's snapshot.state_root
    // (which uses the multi-instance reachability-based compute_state_root).
    let worldline_id = WorldlineId([0x42; 32]);
    let cursor_id = CursorId([0x01; 32]);
    let mut provenance = LocalProvenanceStore::new();
    provenance
        .register_worldline(worldline_id, warp_id)
        .unwrap();

    let mut recorded_roots = Vec::new();
    let mut parents: Vec<warp_core::Hash> = Vec::new();
    let mut replay_store = store.clone(); // Track state by applying patches

    for tick in 0..NUM_TICKS {
        let tx = engine.begin();
        assert!(matches!(
            engine.apply(tx, R1_NAME, &node_id(0)).unwrap(),
            ApplyResult::Applied
        ));
        assert!(matches!(
            engine.apply(tx, R2_NAME, &node_id(2)).unwrap(),
            ApplyResult::Applied
        ));
        assert!(matches!(
            engine.apply(tx, R3_NAME, &node_id(4)).unwrap(),
            ApplyResult::Applied
        ));
        assert!(matches!(
            engine.apply(tx, R4_NAME, &node_id(1)).unwrap(),
            ApplyResult::Applied
        ));
        assert!(matches!(
            engine.apply(tx, R5_NAME, &node_id(7)).unwrap(),
            ApplyResult::Applied
        ));

        let (snapshot, _receipt, patch) = engine.commit_with_receipt(tx).expect("commit");

        // Convert to WorldlineTickPatchV1 for provenance
        let wl_patch = WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                global_tick: tick,
                policy_id: 0,
                rule_pack_id: [0u8; 32],
                plan_digest: snapshot.plan_digest,
                decision_digest: snapshot.decision_digest,
                rewrites_digest: snapshot.rewrites_digest,
            },
            warp_id,
            ops: patch.ops().to_vec(),
            in_slots: patch.in_slots().to_vec(),
            out_slots: patch.out_slots().to_vec(),
            patch_digest: snapshot.patch_digest,
        };

        // Apply patch to replay_store and compute correct state_root
        wl_patch
            .apply_to_store(&mut replay_store)
            .expect("apply to replay store");
        let state_root = compute_state_root_for_warp_store(&replay_store, warp_id);
        recorded_roots.push(state_root);

        let commit_hash = warp_core::compute_commit_hash_v2(
            &state_root,
            &parents,
            &snapshot.patch_digest,
            0, // policy_id
        );

        let triplet = HashTriplet {
            state_root,
            patch_digest: snapshot.patch_digest,
            commit_hash,
        };

        provenance
            .append(worldline_id, wl_patch, triplet, vec![])
            .expect("append");
        parents = vec![commit_hash];
    }

    // Phase 2: Replay from tick 0 to NUM_TICKS
    let mut cursor = PlaybackCursor::new(
        cursor_id,
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &store,
        NUM_TICKS,
    );
    cursor
        .seek_to(NUM_TICKS, &provenance, &store)
        .expect("seek_to should succeed");

    let replayed_root = compute_state_root_for_warp_store(&cursor.store, warp_id);
    assert_eq!(
        replayed_root,
        recorded_roots[NUM_TICKS as usize - 1],
        "Replayed state_root must match recorded state_root at final tick"
    );

    // Phase 3: Per-tick verification
    for tick in 1..=NUM_TICKS {
        let mut cursor_tick = PlaybackCursor::new(
            CursorId([tick as u8; 32]),
            worldline_id,
            warp_id,
            CursorRole::Reader,
            &store,
            NUM_TICKS,
        );
        cursor_tick
            .seek_to(tick, &provenance, &store)
            .expect("seek_to tick");
        let tick_root = compute_state_root_for_warp_store(&cursor_tick.store, warp_id);
        assert_eq!(
            tick_root,
            recorded_roots[tick as usize - 1],
            "Per-tick state_root mismatch at tick {tick}"
        );
    }
}

// =============================================================================
// Phase 4: Permutation Independence
// =============================================================================

#[test]
fn phase_4_permutation_independence() {
    // Apply ONLY the independent rules (R1, R2, R3, R5 — NOT R4) in different orders.
    // Since they have disjoint footprints, the result must be identical regardless of order.
    let mut rng = XorShift64::new(0x51CE_7E07_E0E1_CAFE);

    // Get reference result (canonical order)
    let (store, root) = create_slice_store();
    let mut ref_engine = EngineBuilder::new(store.clone(), root).workers(1).build();
    ref_engine.register_rule(r1_rule()).expect("r1");
    ref_engine.register_rule(r2_rule()).expect("r2");
    ref_engine.register_rule(r3_rule()).expect("r3");
    ref_engine.register_rule(r5_rule()).expect("r5");

    let tx = ref_engine.begin();
    assert!(matches!(
        ref_engine.apply(tx, R1_NAME, &node_id(0)).unwrap(),
        ApplyResult::Applied
    ));
    assert!(matches!(
        ref_engine.apply(tx, R2_NAME, &node_id(2)).unwrap(),
        ApplyResult::Applied
    ));
    assert!(matches!(
        ref_engine.apply(tx, R3_NAME, &node_id(4)).unwrap(),
        ApplyResult::Applied
    ));
    assert!(matches!(
        ref_engine.apply(tx, R5_NAME, &node_id(7)).unwrap(),
        ApplyResult::Applied
    ));
    let (ref_snap, _, _) = ref_engine.commit_with_receipt(tx).expect("commit");

    // Try 10 random permutations of the apply order
    let mut items: Vec<(&str, NodeId)> = vec![
        (R1_NAME, node_id(0)),
        (R2_NAME, node_id(2)),
        (R3_NAME, node_id(4)),
        (R5_NAME, node_id(7)),
    ];

    for perm in 0..10 {
        // Fisher-Yates shuffle
        for i in (1..items.len()).rev() {
            let j = rng.gen_range_usize(i + 1);
            items.swap(i, j);
        }

        let mut engine = EngineBuilder::new(store.clone(), root).workers(1).build();
        engine.register_rule(r1_rule()).expect("r1");
        engine.register_rule(r2_rule()).expect("r2");
        engine.register_rule(r3_rule()).expect("r3");
        engine.register_rule(r5_rule()).expect("r5");

        let tx = engine.begin();
        for (rule_name, scope) in &items {
            assert!(matches!(
                engine.apply(tx, rule_name, scope).unwrap(),
                ApplyResult::Applied
            ));
        }
        let (snap, _, _) = engine.commit_with_receipt(tx).expect("commit");

        assert_eq!(
            snap.state_root, ref_snap.state_root,
            "Permutation {perm}: state_root must be order-independent"
        );
        assert_eq!(
            snap.patch_digest, ref_snap.patch_digest,
            "Permutation {perm}: patch_digest must be order-independent"
        );
    }
}

// =============================================================================
// Phase 6: Semantic Correctness (Dependent Chain)
// =============================================================================

#[test]
fn phase_6_semantic_correctness_dependent_chain() {
    let (store, root) = create_slice_store();
    let warp_id = store.warp_id();

    // Runtime: execute R1 in tick 1 (writes B attachment), then R4 in tick 2 (reads B).
    // BOAW uses snapshot semantics: executors within a tick read the SAME pre-tick view.
    // R4 can only see R1's write after it's committed to the store (separate tick).
    let mut engine = EngineBuilder::new(store.clone(), root).workers(4).build();
    engine.register_rule(r1_rule()).expect("r1");
    engine.register_rule(r4_rule()).expect("r4");

    // Tick 1: R1 writes to B attachment
    let tx1 = engine.begin();
    assert!(matches!(
        engine.apply(tx1, R1_NAME, &node_id(0)).unwrap(),
        ApplyResult::Applied
    ));
    engine.commit(tx1).expect("commit tick 1");

    // Capture store after tick 1 (R1's write is committed, R4 hasn't run yet)
    let post_r1_store = engine.store_clone();

    // Tick 2: R4 reads B attachment (now sees R1's write), writes to G
    let tx2 = engine.begin();
    assert!(matches!(
        engine.apply(tx2, R4_NAME, &node_id(1)).unwrap(),
        ApplyResult::Applied
    ));
    let (snapshot, _, patch) = engine.commit_with_receipt(tx2).expect("commit tick 2");

    // Verify R4 saw R1's output (semantic correctness)
    let final_store = engine.store_clone();
    let g_attach = final_store.node_attachment(&node_id(6));
    match g_attach {
        Some(AttachmentValue::Atom(payload)) => {
            assert_eq!(
                payload.bytes.as_ref(),
                b"r4-saw-r1",
                "R4 must see R1's write and produce the correct transform"
            );
        }
        other => panic!("Expected Atom attachment on G, got {other:?}"),
    }

    // Replay: build provenance with the tick-2 patch, seek, verify same semantic result.
    // We only store the second patch since that's the one producing the G attachment.
    // The initial store for the cursor is the store AFTER tick 1 (R1's write is committed).
    let worldline_id = WorldlineId([0x66; 32]);
    let cursor_id = CursorId([0x77; 32]);
    let mut provenance = LocalProvenanceStore::new();
    provenance
        .register_worldline(worldline_id, warp_id)
        .unwrap();

    let wl_patch = WorldlineTickPatchV1 {
        header: WorldlineTickHeaderV1 {
            global_tick: 0,
            policy_id: 0,
            rule_pack_id: [0u8; 32],
            plan_digest: snapshot.plan_digest,
            decision_digest: snapshot.decision_digest,
            rewrites_digest: snapshot.rewrites_digest,
        },
        warp_id,
        ops: patch.ops().to_vec(),
        in_slots: patch.in_slots().to_vec(),
        out_slots: patch.out_slots().to_vec(),
        patch_digest: snapshot.patch_digest,
    };

    // Compute state_root using the same function seek_to uses
    let mut replay_store = post_r1_store.clone();
    wl_patch
        .apply_to_store(&mut replay_store)
        .expect("apply to replay store");
    let state_root = compute_state_root_for_warp_store(&replay_store, warp_id);
    let commit_hash = compute_commit_hash_v2(&state_root, &[], &snapshot.patch_digest, 0);

    let triplet = HashTriplet {
        state_root,
        patch_digest: snapshot.patch_digest,
        commit_hash,
    };

    provenance
        .append(worldline_id, wl_patch, triplet, vec![])
        .expect("append");

    let mut cursor = PlaybackCursor::new(
        cursor_id,
        worldline_id,
        warp_id,
        CursorRole::Reader,
        &post_r1_store,
        1,
    );
    cursor
        .seek_to(1, &provenance, &post_r1_store)
        .expect("seek");

    // Verify same semantic result after replay
    let replayed_g = cursor.store.node_attachment(&node_id(6));
    match replayed_g {
        Some(AttachmentValue::Atom(payload)) => {
            assert_eq!(
                payload.bytes.as_ref(),
                b"r4-saw-r1",
                "Replay must produce the same semantic result"
            );
        }
        other => panic!("Expected Atom attachment on G after replay, got {other:?}"),
    }

    // Verify hash agreement between runtime and replay
    let replayed_root = compute_state_root_for_warp_store(&cursor.store, warp_id);
    assert_eq!(
        replayed_root, state_root,
        "Replay state_root must match expected state_root (slice theorem trifecta)"
    );
}

// =============================================================================
// Phase 7: Cross-Warp Enforcement (End-to-End)
// =============================================================================

#[test]
fn phase_7_cross_warp_enforcement() {
    // Engine always uses make_warp_id("root") as its warp (W1).
    // R6 attempts to emit UpsertNode into make_warp_id("slice-w2") (W2).
    let mut store = GraphStore::default(); // warp = make_warp_id("root")
    let j = node_id(9);
    store.insert_node(
        j,
        NodeRecord {
            ty: make_type_id("slice/node"),
        },
    );

    let mut engine = warp_core::Engine::new(store, j);
    engine.register_rule(r6_rule()).expect("r6");

    let tx = engine.begin();
    assert!(matches!(
        engine.apply(tx, R6_NAME, &j).unwrap(),
        ApplyResult::Applied
    ));

    let result = catch_unwind(AssertUnwindSafe(move || {
        engine.commit(tx).expect("commit");
    }));

    let err = result.expect_err("cross-warp emission should panic");
    let violation = err
        .downcast_ref::<FootprintViolation>()
        .expect("panic must be FootprintViolation");
    assert_eq!(violation.rule_name, R6_NAME);
    let w2 = make_warp_id("slice-w2");
    assert!(
        matches!(violation.kind, ViolationKind::CrossWarpEmission { op_warp } if op_warp == w2),
        "expected CrossWarpEmission targeting W2, got {:?}",
        violation.kind
    );
}

// =============================================================================
// Dependency verification (R1 ∩ R4 footprints are NOT independent)
// =============================================================================

#[test]
fn verify_r1_r4_dependency() {
    // R1 writes B attachment, R4 reads B attachment → NOT independent
    let store = GraphStore::default();
    let view = GraphView::new(&store);

    let fp1 = r1_footprint(view, &node_id(0));
    let fp4 = r4_footprint(view, &node_id(1));

    assert!(
        !fp1.independent(&fp4),
        "R1 and R4 must NOT be independent (R1 writes B attachment, R4 reads it)"
    );

    // R1, R2, R3, R5 are all independent of each other
    let fp2 = r2_footprint(view, &node_id(2));
    let fp3 = r3_footprint(view, &node_id(4));
    let fp5 = r5_footprint(view, &node_id(7));

    assert!(fp1.independent(&fp2), "R1 and R2 must be independent");
    assert!(fp1.independent(&fp3), "R1 and R3 must be independent");
    assert!(fp1.independent(&fp5), "R1 and R5 must be independent");
    assert!(fp2.independent(&fp3), "R2 and R3 must be independent");
    assert!(fp2.independent(&fp5), "R2 and R5 must be independent");
    assert!(fp3.independent(&fp5), "R3 and R5 must be independent");
}
