// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tests for the generic `sys/dispatch_inbox` rule.

use bytes::Bytes;
use warp_core::{
    inbox::dispatch_inbox_rule, make_edge_id, make_node_id, make_type_id, AtomPayload,
    EdgeRecord, Engine, GraphStore, NodeId, NodeRecord,
};

fn build_engine_with_root(root: NodeId) -> Engine {
    let mut store = GraphStore::default();
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("root"),
        },
    );
    Engine::new(store, root)
}

fn make_payload() -> AtomPayload {
    AtomPayload::new(
        make_type_id("intent:generic"),
        Bytes::from_static(br#"{ "data": "payload" }"#),
    )
}

#[test]
fn dispatch_inbox_drains_events() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    // Register rule
    engine
        .register_rule(dispatch_inbox_rule())
        .expect("register rule");

    // Seed two inbox events
    let payload = make_payload();
    engine.ingest_inbox_event(1, &payload).unwrap();
    engine.ingest_inbox_event(2, &payload).unwrap();

    let inbox_id = make_node_id("sim/inbox");

    // Apply + commit
    let tx = engine.begin();
    let applied = engine
        .apply(tx, warp_core::inbox::DISPATCH_INBOX_RULE_NAME, &inbox_id)
        .expect("apply rule");
    assert!(matches!(applied, warp_core::ApplyResult::Applied));
    engine.commit(tx).expect("commit");

    let store = engine.store_clone();

    // Inbox remains
    assert!(store.node(&inbox_id).is_some());

    // Events are gone
    let event1 = make_node_id("sim/inbox/event:0000000000000001");
    let event2 = make_node_id("sim/inbox/event:0000000000000002");
    assert!(store.node(&event1).is_none());
    assert!(store.node(&event2).is_none());

    // Inbox attachment cleared
    assert!(store.node_attachment(&inbox_id).is_none());

    // No outbound edges from inbox
    assert!(store.edges_from(&inbox_id).next().is_none());
}

#[test]
fn dispatch_inbox_handles_missing_event_attachments() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    engine
        .register_rule(dispatch_inbox_rule())
        .expect("register rule");

    let payload = make_payload();
    engine.ingest_inbox_event(1, &payload).unwrap();

    // Simulate corrupted state: event exists, but attachment was cleared.
    let event1 = make_node_id("sim/inbox/event:0000000000000001");
    engine.set_node_attachment(event1, None).unwrap();

    let inbox_id = make_node_id("sim/inbox");

    let tx = engine.begin();
    let applied = engine
        .apply(tx, warp_core::inbox::DISPATCH_INBOX_RULE_NAME, &inbox_id)
        .expect("apply rule");
    assert!(matches!(applied, warp_core::ApplyResult::Applied));
    engine.commit(tx).expect("commit");

    let store = engine.store_clone();
    assert!(store.node(&event1).is_none());
    assert!(store.edges_from(&inbox_id).next().is_none());
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
    assert!(matches!(applied, warp_core::ApplyResult::NoMatch));
    engine.commit(tx).expect("commit");
}

#[test]
fn dispatch_inbox_deletes_events_without_atom_attachment() {
    let root = make_node_id("root");
    let inbox_id = make_node_id("sim/inbox");
    let event_id = make_node_id("sim/inbox/event:0000000000000001");

    let mut store = GraphStore::default();
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("root"),
        },
    );
    store.insert_node(
        make_node_id("sim"),
        NodeRecord {
            ty: make_type_id("sim"),
        },
    );
    store.insert_node(
        inbox_id,
        NodeRecord {
            ty: make_type_id("sim/inbox"),
        },
    );
    store.insert_node(
        event_id,
        NodeRecord {
            ty: make_type_id("sim/inbox/event"),
        },
    );

    store.insert_edge(
        root,
        EdgeRecord {
            id: make_edge_id("edge:root/sim"),
            from: root,
            to: make_node_id("sim"),
            ty: make_type_id("edge:sim"),
        },
    );
    store.insert_edge(
        make_node_id("sim"),
        EdgeRecord {
            id: make_edge_id("edge:sim/inbox"),
            from: make_node_id("sim"),
            to: inbox_id,
            ty: make_type_id("edge:inbox"),
        },
    );
    store.insert_edge(
        inbox_id,
        EdgeRecord {
            id: make_edge_id("edge:event:0000000000000001"),
            from: inbox_id,
            to: event_id,
            ty: make_type_id("edge:event"),
        },
    );

    let mut engine = Engine::new(store, root);
    engine
        .register_rule(dispatch_inbox_rule())
        .expect("register rule");

    let tx = engine.begin();
    let applied = engine
        .apply(tx, warp_core::inbox::DISPATCH_INBOX_RULE_NAME, &inbox_id)
        .expect("apply rule");
    assert!(matches!(applied, warp_core::ApplyResult::Applied));
    engine.commit(tx).expect("commit");

    let store = engine.store_clone();
    assert!(store.node(&event_id).is_none());
}