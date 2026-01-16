// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tests for the generic `sys/dispatch_inbox` rule.

use echo_dry_tests::build_engine_with_root;
use warp_core::{
    inbox::{ack_pending_rule, dispatch_inbox_rule},
    make_node_id, make_type_id, ApplyResult, IngestDisposition, NodeId,
};

#[test]
fn dispatch_inbox_drains_pending_edges_but_keeps_event_nodes() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    // Register rule
    engine
        .register_rule(dispatch_inbox_rule())
        .expect("register rule");

    // Seed two intents (opaque bytes - core is byte-blind).
    let intent1: &[u8] = b"intent-one";
    let intent2: &[u8] = b"intent-two";

    let intent_id1 = match engine.ingest_intent(intent1).expect("ingest") {
        IngestDisposition::Accepted { intent_id } => intent_id,
        other => panic!("expected Accepted, got {other:?}"),
    };
    let intent_id2 = match engine.ingest_intent(intent2).expect("ingest") {
        IngestDisposition::Accepted { intent_id } => intent_id,
        other => panic!("expected Accepted, got {other:?}"),
    };

    let inbox_id = make_node_id("sim/inbox");

    // Apply + commit
    let tx = engine.begin();
    let applied = engine
        .apply(tx, warp_core::inbox::DISPATCH_INBOX_RULE_NAME, &inbox_id)
        .expect("apply rule");
    assert!(matches!(applied, ApplyResult::Applied));
    engine.commit(tx).expect("commit");

    let store = engine.store_clone();

    // Inbox remains
    assert!(store.node(&inbox_id).is_some());

    // Ledger nodes remain (append-only).
    let event1 = NodeId(intent_id1);
    let event2 = NodeId(intent_id2);
    assert!(store.node(&event1).is_some());
    assert!(store.node(&event2).is_some());

    // Pending edges drained (queue maintenance).
    let pending_ty = make_type_id("edge:pending");
    assert!(!store.edges_from(&inbox_id).any(|e| e.ty == pending_ty));
}

#[test]
fn dispatch_inbox_handles_missing_event_attachments() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    engine
        .register_rule(dispatch_inbox_rule())
        .expect("register rule");

    let intent = b"test-intent";
    let intent_id = match engine.ingest_intent(intent).expect("ingest") {
        IngestDisposition::Accepted { intent_id } => intent_id,
        other => panic!("expected Accepted, got {other:?}"),
    };
    let event_id = NodeId(intent_id);

    // Simulate corrupted state: event exists, but attachment was cleared.
    engine.set_node_attachment(event_id, None).unwrap();

    let inbox_id = make_node_id("sim/inbox");

    let tx = engine.begin();
    let applied = engine
        .apply(tx, warp_core::inbox::DISPATCH_INBOX_RULE_NAME, &inbox_id)
        .expect("apply rule");
    assert!(matches!(applied, ApplyResult::Applied));
    engine.commit(tx).expect("commit");

    let store = engine.store_clone();
    assert!(
        store.node(&event_id).is_some(),
        "ledger nodes are append-only"
    );
    let pending_ty = make_type_id("edge:pending");
    assert!(!store.edges_from(&inbox_id).any(|e| e.ty == pending_ty));
}

#[test]
fn dispatch_inbox_no_match_when_scope_is_not_inbox() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    engine
        .register_rule(dispatch_inbox_rule())
        .expect("register rule");

    let tx = engine.begin();
    let applied = engine
        .apply(tx, warp_core::inbox::DISPATCH_INBOX_RULE_NAME, &root)
        .expect("apply rule");
    assert!(matches!(applied, ApplyResult::NoMatch));
    engine.commit(tx).expect("commit");
}

#[test]
fn ack_pending_consumes_one_event_edge() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    engine
        .register_rule(ack_pending_rule())
        .expect("register rule");

    let intent1 = b"intent-alpha";
    let intent2 = b"intent-beta";

    let intent_id1 = match engine.ingest_intent(intent1).expect("ingest") {
        IngestDisposition::Accepted { intent_id } => intent_id,
        other => panic!("expected Accepted, got {other:?}"),
    };
    let intent_id2 = match engine.ingest_intent(intent2).expect("ingest") {
        IngestDisposition::Accepted { intent_id } => intent_id,
        other => panic!("expected Accepted, got {other:?}"),
    };
    let event1 = NodeId(intent_id1);
    let event2 = NodeId(intent_id2);
    let inbox_id = make_node_id("sim/inbox");

    let tx = engine.begin();
    let applied = engine
        .apply(tx, warp_core::inbox::ACK_PENDING_RULE_NAME, &event1)
        .expect("apply rule");
    assert!(matches!(applied, ApplyResult::Applied));
    engine.commit(tx).expect("commit");

    let store = engine.store_clone();
    let pending_ty = make_type_id("edge:pending");
    let mut pending: Vec<_> = store
        .edges_from(&inbox_id)
        .filter(|e| e.ty == pending_ty)
        .map(|e| e.to)
        .collect();
    pending.sort_unstable();
    assert_eq!(pending, vec![event2]);
}
