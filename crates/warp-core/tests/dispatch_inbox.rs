// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tests for the `sys/dispatch_inbox` rule.

use bytes::Bytes;
use warp_core::{
    inbox::dispatch_inbox_rule, make_node_id, make_type_id, AtomPayload, Engine, GraphStore,
    NodeId, NodeRecord,
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
        make_type_id("intent:route_push"),
        Bytes::from_static(br#"{ "path": "/" }"#),
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
