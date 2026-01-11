// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tests for the `cmd/route_push` rule.

use bytes::Bytes;
use warp_core::{
    cmd::route_push_rule, make_node_id, make_type_id, AtomPayload, AttachmentValue, Engine,
    GraphStore, NodeId, NodeRecord,
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

#[test]
fn cmd_route_push_updates_route_path_state() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    engine
        .register_rule(route_push_rule())
        .expect("register route_push");

    let payload_bytes = Bytes::from_static(br#"{ "path": "/chronos" }"#);
    let payload = AtomPayload::new(make_type_id("intent:route_push"), payload_bytes.clone());

    engine.ingest_inbox_event(7, &payload).unwrap();

    let event_id = make_node_id("sim/inbox/event:0000000000000007");
    let route_id = make_node_id("sim/state/routePath");

    let tx = engine.begin();
    let applied = engine
        .apply(tx, warp_core::cmd::ROUTE_PUSH_RULE_NAME, &event_id)
        .expect("apply");
    assert!(matches!(applied, warp_core::ApplyResult::Applied));
    engine.commit(tx).expect("commit");

    let store = engine.store_clone();

    let route_node = store.node(&route_id).expect("route node exists");
    assert_eq!(route_node.ty, make_type_id("sim/state/routePath"));

    let AttachmentValue::Atom(atom) = store.node_attachment(&route_id).expect("route attachment")
    else {
        panic!("expected atom attachment on routePath");
    };
    assert_eq!(atom.type_id, make_type_id("state:route_path"));
    assert_eq!(atom.bytes, payload_bytes);
}

#[test]
fn cmd_route_push_no_match_for_non_route_push_events() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    engine
        .register_rule(route_push_rule())
        .expect("register route_push");

    let payload_bytes = Bytes::from_static(br#"{ "foo": "bar" }"#);
    let payload = AtomPayload::new(make_type_id("intent:unknown"), payload_bytes);

    engine.ingest_inbox_event(1, &payload).unwrap();

    let event_id = make_node_id("sim/inbox/event:0000000000000001");

    let tx = engine.begin();
    let applied = engine
        .apply(tx, warp_core::cmd::ROUTE_PUSH_RULE_NAME, &event_id)
        .expect("apply");
    assert!(matches!(applied, warp_core::ApplyResult::NoMatch));
    engine.commit(tx).expect("commit");

    let store = engine.store_clone();
    // cmd/route_push must not create the route node when it doesn't match.
    assert!(store.node(&make_node_id("sim/state/routePath")).is_none());
}
