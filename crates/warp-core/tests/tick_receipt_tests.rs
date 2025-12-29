// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]

use warp_core::{
    encode_motion_payload, make_node_id, make_type_id, ConflictPolicy, Engine, Footprint,
    GraphStore, Hash, NodeId, NodeRecord, PatternGraph, RewriteRule, TickReceiptDisposition,
    TickReceiptEntry, TickReceiptRejection, TxId, MOTION_RULE_NAME,
};

fn rule_id(name: &str) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:");
    hasher.update(name.as_bytes());
    hasher.finalize().into()
}

fn scope_hash(rule_id: &Hash, scope: &NodeId) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(rule_id);
    hasher.update(&scope.0);
    hasher.finalize().into()
}

fn compute_plan_digest(entries: &[TickReceiptEntry]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(entries.len() as u64).to_le_bytes());
    for entry in entries {
        hasher.update(&entry.scope_hash);
        hasher.update(&entry.rule_id);
    }
    hasher.finalize().into()
}

fn compute_rewrites_digest(entries: &[TickReceiptEntry]) -> Hash {
    let applied: Vec<TickReceiptEntry> = entries
        .iter()
        .copied()
        .filter(|e| e.disposition == TickReceiptDisposition::Applied)
        .collect();

    let mut hasher = blake3::Hasher::new();
    hasher.update(&(applied.len() as u64).to_le_bytes());
    for entry in applied {
        hasher.update(&entry.rule_id);
        hasher.update(&entry.scope_hash);
        hasher.update(&(entry.scope).0);
    }
    hasher.finalize().into()
}

fn always_match(_: &GraphStore, _: &NodeId) -> bool {
    true
}

fn exec_noop(_: &mut GraphStore, _: &NodeId) {}

fn other_of(scope: &NodeId) -> NodeId {
    NodeId(blake3::hash(&scope.0).into())
}

fn fp_write_scope(_: &GraphStore, scope: &NodeId) -> Footprint {
    let mut fp = Footprint::default();
    fp.n_write.insert_node(scope);
    fp.factor_mask = 1;
    fp
}

fn fp_write_scope_and_other(_: &GraphStore, scope: &NodeId) -> Footprint {
    let mut fp = Footprint::default();
    fp.n_write.insert_node(scope);
    fp.n_write.insert_node(&other_of(scope));
    fp.factor_mask = 1;
    fp
}

fn pick_rule_ids_for_blocker_test(scope_a: &NodeId, scope_b: &NodeId) -> (Hash, Hash, Hash) {
    // Pick three distinct synthetic ids such that:
    // - the combined-write rule (C) sorts last by scope_hash
    // - the two single-write rules (A, B) sort before it
    //
    // This ensures the first two candidates are accepted and the last candidate
    // is rejected with *two* blockers, making the test stable even if rule IDs
    // or scope hashing semantics evolve.
    let mut candidates: Vec<(Hash, u8)> = (0u8..=255)
        .map(|b| {
            let id = [b; 32];
            (scope_hash(&id, scope_a), b)
        })
        .collect();
    candidates.sort_by(|(ha, ba), (hb, bb)| ha.cmp(hb).then(ba.cmp(bb)));

    for (_, c_byte) in candidates.iter().rev() {
        let c_id: Hash = [*c_byte; 32];
        let h_c = scope_hash(&c_id, scope_a);

        let Some((_, a_byte)) = candidates.iter().find(|(h, b)| *b != *c_byte && *h < h_c) else {
            continue;
        };
        let a_id: Hash = [*a_byte; 32];

        let b_byte = (0u8..=255)
            .find(|b| *b != *c_byte && *b != *a_byte && scope_hash(&[*b; 32], scope_b) < h_c);
        let Some(b_byte) = b_byte else {
            continue;
        };
        let b_id: Hash = [b_byte; 32];

        return (a_id, b_id, c_id);
    }

    panic!("failed to find stable test ids for blocker ordering");
}

#[test]
fn commit_with_receipt_records_accept_reject_and_matches_snapshot_digests() {
    let entity = make_node_id("tick-receipt-entity");
    let entity_type = make_type_id("entity");
    let payload = encode_motion_payload([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);

    let mut store = GraphStore::default();
    store.insert_node(
        entity,
        NodeRecord {
            ty: entity_type,
            payload: Some(payload),
        },
    );

    let mut engine = Engine::new(store, entity);
    engine
        .register_rule(warp_core::motion_rule())
        .expect("motion rule registers");

    // Register a second rule with a distinct id and name but the same matcher/executor/footprint.
    // This lets us generate two candidates that touch the same node without being de-duped by
    // the scheduler’s last-wins key.
    let rule2_name: &'static str = "motion/update-alt";
    let base = warp_core::motion_rule();
    let rule2 = RewriteRule {
        id: rule_id(rule2_name),
        name: rule2_name,
        left: warp_core::PatternGraph { nodes: vec![] },
        matcher: base.matcher,
        executor: base.executor,
        compute_footprint: base.compute_footprint,
        factor_mask: base.factor_mask,
        conflict_policy: base.conflict_policy,
        join_fn: base.join_fn,
    };
    engine.register_rule(rule2).expect("alt rule registers");

    let tx: TxId = engine.begin();
    engine
        .apply(tx, MOTION_RULE_NAME, &entity)
        .expect("first apply succeeds");
    engine
        .apply(tx, rule2_name, &entity)
        .expect("second apply succeeds");

    let (snapshot, receipt) = engine.commit_with_receipt(tx).expect("commit_with_receipt");

    let entries = receipt.entries();
    assert_eq!(
        entries.len(),
        2,
        "expected two candidates in the tick receipt"
    );
    assert_eq!(entries[0].disposition, TickReceiptDisposition::Applied);
    assert_eq!(
        entries[1].disposition,
        TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict)
    );
    assert!(
        receipt.blocked_by(0).is_empty(),
        "applied entries should not have blockers"
    );
    assert_eq!(
        receipt.blocked_by(1),
        &[0],
        "the rejected candidate should be blocked by the applied candidate"
    );

    assert_eq!(snapshot.plan_digest, compute_plan_digest(entries));
    assert_eq!(snapshot.rewrites_digest, compute_rewrites_digest(entries));
    assert_eq!(snapshot.decision_digest, receipt.digest());
    assert_ne!(
        snapshot.decision_digest,
        *warp_core::DIGEST_LEN0_U64,
        "non-empty tick receipt should not use the canonical empty digest"
    );
}

#[test]
fn commit_with_receipt_records_multi_blocker_causality() {
    let scope_a = make_node_id("tick-receipt-poset-a");
    let scope_b = other_of(&scope_a);
    assert_ne!(scope_a, scope_b, "expected distinct nodes for the test");

    let ty = make_type_id("entity");
    let mut store = GraphStore::default();
    store.insert_node(scope_a, NodeRecord { ty, payload: None });
    store.insert_node(scope_b, NodeRecord { ty, payload: None });

    let mut engine = Engine::new(store, scope_a);

    let (id_a, id_b, id_c) = pick_rule_ids_for_blocker_test(&scope_a, &scope_b);

    const RULE_A: &str = "test/write-scope-a";
    const RULE_B: &str = "test/write-scope-b";
    const RULE_C: &str = "test/write-scope-a-and-b";

    engine
        .register_rule(RewriteRule {
            id: id_a,
            name: RULE_A,
            left: PatternGraph { nodes: vec![] },
            matcher: always_match,
            executor: exec_noop,
            compute_footprint: fp_write_scope,
            factor_mask: 1,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        })
        .expect("rule A registers");
    engine
        .register_rule(RewriteRule {
            id: id_b,
            name: RULE_B,
            left: PatternGraph { nodes: vec![] },
            matcher: always_match,
            executor: exec_noop,
            compute_footprint: fp_write_scope,
            factor_mask: 1,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        })
        .expect("rule B registers");
    engine
        .register_rule(RewriteRule {
            id: id_c,
            name: RULE_C,
            left: PatternGraph { nodes: vec![] },
            matcher: always_match,
            executor: exec_noop,
            compute_footprint: fp_write_scope_and_other,
            factor_mask: 1,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        })
        .expect("rule C registers");

    let tx = engine.begin();
    engine.apply(tx, RULE_A, &scope_a).expect("apply A");
    engine.apply(tx, RULE_B, &scope_b).expect("apply B");
    engine.apply(tx, RULE_C, &scope_a).expect("apply C");

    let (_snapshot, receipt) = engine.commit_with_receipt(tx).expect("commit_with_receipt");
    let entries = receipt.entries();
    assert_eq!(entries.len(), 3);

    assert_eq!(entries[2].rule_id, id_c, "combined write should sort last");
    assert_eq!(entries[0].disposition, TickReceiptDisposition::Applied);
    assert_eq!(entries[1].disposition, TickReceiptDisposition::Applied);
    assert_eq!(
        entries[2].disposition,
        TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict)
    );

    assert!(receipt.blocked_by(0).is_empty());
    assert!(receipt.blocked_by(1).is_empty());
    assert_eq!(
        receipt.blocked_by(2),
        &[0, 1],
        "combined write should be blocked by both prior applied candidates"
    );
}
