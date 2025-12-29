// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]

use warp_core::{
    encode_motion_payload, make_node_id, make_type_id, Engine, GraphStore, Hash, NodeRecord,
    RewriteRule, TickReceiptDisposition, TickReceiptEntry, TickReceiptRejection, TxId,
    MOTION_RULE_NAME,
};

fn rule_id(name: &str) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:");
    hasher.update(name.as_bytes());
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

    assert_eq!(snapshot.plan_digest, compute_plan_digest(entries));
    assert_eq!(snapshot.rewrites_digest, compute_rewrites_digest(entries));
    assert_eq!(snapshot.decision_digest, receipt.digest());
    assert_ne!(
        snapshot.decision_digest,
        *warp_core::DIGEST_LEN0_U64,
        "non-empty tick receipt should not use the canonical empty digest"
    );
}
