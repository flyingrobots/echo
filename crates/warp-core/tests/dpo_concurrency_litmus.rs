// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! DPO concurrency litmus tests for `warp-core`.
//!
//! These tests pin “order independence” at the engine boundary:
//! enqueue order must not change the terminal digests for commuting cases, and
//! overlap must yield deterministic rejection for conflicting cases.

use warp_core::demo::ports::{port_rule, PORT_RULE_NAME};
use warp_core::{
    encode_motion_atom_payload, make_node_id, make_type_id, ApplyResult, AttachmentValue, Engine,
    EngineError, Footprint, GraphStore, NodeId, NodeRecord, PatternGraph, RewriteRule,
};

const LITMUS_PORT_READ_0: &str = "litmus/port_read_0";
const LITMUS_PORT_READ_1: &str = "litmus/port_read_1";

fn litmus_rule_id(name: &'static str) -> warp_core::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:");
    hasher.update(name.as_bytes());
    hasher.finalize().into()
}

fn litmus_port_read_matcher(store: &GraphStore, scope: &NodeId) -> bool {
    store.node(scope).is_some()
}

fn litmus_port_read_executor(_store: &mut GraphStore, _scope: &NodeId) {}

fn litmus_port_read_0_footprint(store: &GraphStore, scope: &NodeId) -> Footprint {
    let mut fp = Footprint::default();
    if store.node(scope).is_some() {
        fp.n_read.insert_node(scope);
        fp.b_in.insert(warp_core::pack_port_key(scope, 0, true));
    }
    fp
}

fn litmus_port_read_1_footprint(store: &GraphStore, scope: &NodeId) -> Footprint {
    let mut fp = Footprint::default();
    if store.node(scope).is_some() {
        fp.n_read.insert_node(scope);
        fp.b_in.insert(warp_core::pack_port_key(scope, 1, true));
    }
    fp
}

fn litmus_port_read_0_rule() -> RewriteRule {
    RewriteRule {
        id: litmus_rule_id(LITMUS_PORT_READ_0),
        name: LITMUS_PORT_READ_0,
        left: PatternGraph { nodes: Vec::new() },
        matcher: litmus_port_read_matcher,
        executor: litmus_port_read_executor,
        compute_footprint: litmus_port_read_0_footprint,
        factor_mask: 0,
        conflict_policy: warp_core::ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn litmus_port_read_1_rule() -> RewriteRule {
    RewriteRule {
        id: litmus_rule_id(LITMUS_PORT_READ_1),
        name: LITMUS_PORT_READ_1,
        left: PatternGraph { nodes: Vec::new() },
        matcher: litmus_port_read_matcher,
        executor: litmus_port_read_executor,
        compute_footprint: litmus_port_read_1_footprint,
        factor_mask: 0,
        conflict_policy: warp_core::ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn build_litmus_engine() -> Result<Engine, EngineError> {
    let mut store = GraphStore::default();
    let root = make_node_id("litmus-world-root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );

    let mut engine = Engine::new(store, root);
    engine.register_rule(warp_core::motion_rule())?;
    engine.register_rule(port_rule())?;
    engine.register_rule(litmus_port_read_0_rule())?;
    engine.register_rule(litmus_port_read_1_rule())?;
    Ok(engine)
}

fn assert_terminal_digests_equal(a: &warp_core::Snapshot, b: &warp_core::Snapshot) {
    assert_eq!(a.hash, b.hash, "commit hash must be order-independent");
    assert_eq!(
        a.plan_digest, b.plan_digest,
        "plan digest must be order-independent"
    );
    assert_eq!(
        a.rewrites_digest, b.rewrites_digest,
        "rewrites digest must be order-independent"
    );
    assert_eq!(
        a.decision_digest, b.decision_digest,
        "decision (receipt) digest must be order-independent"
    );
    assert_eq!(
        a.patch_digest, b.patch_digest,
        "patch digest must be order-independent"
    );
    assert_eq!(a.policy_id, b.policy_id, "policy id must match");
    assert_eq!(a.parents, b.parents, "parents must match in litmus setup");
}

#[test]
fn dpo_litmus_commuting_independent_pair() -> Result<(), EngineError> {
    // Two candidates are independent (disjoint footprints) and commute:
    // enqueue order must not change the terminal digest.
    fn setup() -> Result<(Engine, NodeId, NodeId), EngineError> {
        let mut engine = build_litmus_engine()?;

        let entity_motion = make_node_id("litmus-entity-motion");
        engine.insert_node(
            entity_motion,
            NodeRecord {
                ty: make_type_id("entity"),
            },
        )?;
        let pos = [10.0, -2.0, 3.5];
        let vel = [0.125, 2.0, -1.5];
        engine.set_node_attachment(
            entity_motion,
            Some(AttachmentValue::Atom(encode_motion_atom_payload(pos, vel))),
        )?;

        let entity_port = make_node_id("litmus-entity-port");
        engine.insert_node(
            entity_port,
            NodeRecord {
                ty: make_type_id("entity"),
            },
        )?;

        Ok((engine, entity_motion, entity_port))
    }

    let (mut a, entity_motion, entity_port) = setup()?;
    let tx_a = a.begin();
    assert!(matches!(
        a.apply(tx_a, warp_core::MOTION_RULE_NAME, &entity_motion)?,
        ApplyResult::Applied
    ));
    assert!(matches!(
        a.apply(tx_a, PORT_RULE_NAME, &entity_port)?,
        ApplyResult::Applied
    ));
    let (snap_a, receipt_a, _patch_a) = a.commit_with_receipt(tx_a)?;

    let (mut b, entity_motion, entity_port) = setup()?;
    let tx_b = b.begin();
    assert!(matches!(
        b.apply(tx_b, PORT_RULE_NAME, &entity_port)?,
        ApplyResult::Applied
    ));
    assert!(matches!(
        b.apply(tx_b, warp_core::MOTION_RULE_NAME, &entity_motion)?,
        ApplyResult::Applied
    ));
    let (snap_b, receipt_b, _patch_b) = b.commit_with_receipt(tx_b)?;

    assert_terminal_digests_equal(&snap_a, &snap_b);
    assert_eq!(
        receipt_a.digest(),
        receipt_b.digest(),
        "receipt digest must match across enqueue orders"
    );

    // Verify both candidates were actually applied (not just that digests match).
    let a_entries = receipt_a.entries();
    assert_eq!(a_entries.len(), 2, "fixture must produce two candidates");
    assert!(
        a_entries
            .iter()
            .all(|e| e.disposition == warp_core::TickReceiptDisposition::Applied),
        "independent candidates must both be applied"
    );

    let b_entries = receipt_b.entries();
    assert_eq!(b_entries.len(), 2, "fixture must produce two candidates");
    assert!(
        b_entries
            .iter()
            .all(|e| e.disposition == warp_core::TickReceiptDisposition::Applied),
        "independent candidates must both be applied (order B)"
    );
    Ok(())
}

#[test]
fn dpo_litmus_conflicting_pair_is_deterministically_rejected() -> Result<(), EngineError> {
    // Two candidates overlap (critical-pair style): only one should be admitted,
    // and the winner must be deterministic (enqueue-order independent).
    //
    // Conflict surface: both `motion/update` and `demo/port_nop` write the same
    // α-plane node attachment slot (the motion payload), so their `a_write`
    // footprints intersect when scoped to the same node.
    fn setup() -> Result<(Engine, NodeId), EngineError> {
        let mut engine = build_litmus_engine()?;

        let entity = make_node_id("litmus-entity-conflict");
        engine.insert_node(
            entity,
            NodeRecord {
                ty: make_type_id("entity"),
            },
        )?;
        let pos = [0.0, 0.0, 0.0];
        let vel = [1.0, 0.0, 0.0];
        engine.set_node_attachment(
            entity,
            Some(AttachmentValue::Atom(encode_motion_atom_payload(pos, vel))),
        )?;

        Ok((engine, entity))
    }

    let (mut a, entity) = setup()?;
    let tx_a = a.begin();
    assert!(matches!(
        a.apply(tx_a, warp_core::MOTION_RULE_NAME, &entity)?,
        ApplyResult::Applied
    ));
    assert!(matches!(
        a.apply(tx_a, PORT_RULE_NAME, &entity)?,
        ApplyResult::Applied
    ));
    let (snap_a, receipt_a, _patch_a) = a.commit_with_receipt(tx_a)?;

    let (mut b, entity) = setup()?;
    let tx_b = b.begin();
    assert!(matches!(
        b.apply(tx_b, PORT_RULE_NAME, &entity)?,
        ApplyResult::Applied
    ));
    assert!(matches!(
        b.apply(tx_b, warp_core::MOTION_RULE_NAME, &entity)?,
        ApplyResult::Applied
    ));
    let (snap_b, receipt_b, _patch_b) = b.commit_with_receipt(tx_b)?;

    assert_terminal_digests_equal(&snap_a, &snap_b);
    assert_eq!(receipt_a.digest(), receipt_b.digest());

    let a_entries = receipt_a.entries();
    assert_eq!(a_entries.len(), 2, "fixture must produce two candidates");
    let applied = a_entries
        .iter()
        .filter(|e| e.disposition == warp_core::TickReceiptDisposition::Applied)
        .count();
    let rejected = a_entries
        .iter()
        .filter(|e| {
            e.disposition
                == warp_core::TickReceiptDisposition::Rejected(
                    warp_core::TickReceiptRejection::FootprintConflict,
                )
        })
        .count();
    assert_eq!(applied, 1, "exactly one candidate must be applied");
    assert_eq!(rejected, 1, "exactly one candidate must be rejected");

    let b_entries = receipt_b.entries();
    assert_eq!(b_entries.len(), 2, "fixture must produce two candidates");
    assert_eq!(
        a_entries, b_entries,
        "receipt structure must be order-independent (not just digest)"
    );

    Ok(())
}

#[test]
fn dpo_litmus_overlapping_scope_disjoint_ports_are_composable() -> Result<(), EngineError> {
    // Two candidates share the same scope node but reserve disjoint boundary ports.
    // This is an “overlap but still composable” case: both should be admitted and
    // the terminal digest must be enqueue-order independent.
    fn setup() -> Result<(Engine, NodeId), EngineError> {
        let mut engine = build_litmus_engine()?;
        let entity = make_node_id("litmus-entity-ports");
        engine.insert_node(
            entity,
            NodeRecord {
                ty: make_type_id("entity"),
            },
        )?;
        Ok((engine, entity))
    }

    let (mut a, entity) = setup()?;
    let tx_a = a.begin();
    assert!(matches!(
        a.apply(tx_a, LITMUS_PORT_READ_0, &entity)?,
        ApplyResult::Applied
    ));
    assert!(matches!(
        a.apply(tx_a, LITMUS_PORT_READ_1, &entity)?,
        ApplyResult::Applied
    ));
    let (snap_a, receipt_a, _patch_a) = a.commit_with_receipt(tx_a)?;

    let (mut b, entity) = setup()?;
    let tx_b = b.begin();
    assert!(matches!(
        b.apply(tx_b, LITMUS_PORT_READ_1, &entity)?,
        ApplyResult::Applied
    ));
    assert!(matches!(
        b.apply(tx_b, LITMUS_PORT_READ_0, &entity)?,
        ApplyResult::Applied
    ));
    let (snap_b, receipt_b, _patch_b) = b.commit_with_receipt(tx_b)?;

    assert_terminal_digests_equal(&snap_a, &snap_b);
    assert_eq!(receipt_a.digest(), receipt_b.digest());

    let entries = receipt_a.entries();
    assert_eq!(entries.len(), 2);
    assert!(
        entries
            .iter()
            .all(|e| e.disposition == warp_core::TickReceiptDisposition::Applied),
        "both disjoint-port candidates should be admitted"
    );

    let b_entries = receipt_b.entries();
    assert_eq!(b_entries.len(), 2);
    assert_eq!(
        entries, b_entries,
        "receipt structure must be order-independent (not just digest)"
    );

    Ok(())
}
