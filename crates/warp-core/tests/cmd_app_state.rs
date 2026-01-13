// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tests for AppState command rules: `cmd/set_theme` and `cmd/toggle_nav`.

use bytes::Bytes;
use warp_core::{
    cmd::{set_theme_rule, toggle_nav_rule, SET_THEME_RULE_NAME, TOGGLE_NAV_RULE_NAME},
    make_node_id, make_type_id, AtomPayload, AttachmentValue, Engine, GraphStore, NodeId,
    NodeRecord,
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
fn cmd_set_theme_updates_theme_state() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    engine
        .register_rule(set_theme_rule())
        .expect("register set_theme");

    let payload_bytes = Bytes::from_static(br#"{ "mode": "DARK" }"#);
    let payload = AtomPayload::new(make_type_id("intent:set_theme"), payload_bytes.clone());

    engine.ingest_inbox_event(1, &payload).unwrap();

    let event_id = make_node_id("sim/inbox/event:0000000000000001");
    let theme_id = make_node_id("sim/state/theme");

    let tx = engine.begin();
    let applied = engine
        .apply(tx, SET_THEME_RULE_NAME, &event_id)
        .expect("apply");
    assert!(matches!(applied, warp_core::ApplyResult::Applied));
    engine.commit(tx).expect("commit");

    let store = engine.store_clone();
    let AttachmentValue::Atom(atom) = store.node_attachment(&theme_id).expect("theme attachment")
    else {
        panic!("expected atom attachment on theme node");
    };
    assert_eq!(atom.type_id, make_type_id("state:theme"));
    assert_eq!(atom.bytes, payload_bytes);

    // Verify ViewOp was emitted
    let view_id = make_node_id("sim/view");
    let op_id = make_node_id("sim/view/op:0000000000000000");
    assert!(store.node(&view_id).is_some());
    assert!(store.node(&op_id).is_some());

    let AttachmentValue::Atom(op_atom) = store.node_attachment(&op_id).expect("op attachment")
    else {
        panic!("expected atom attachment on op node");
    };
    assert_eq!(op_atom.type_id, make_type_id("view_op:SetTheme"));
    assert_eq!(op_atom.bytes, payload_bytes);
}

#[test]
fn cmd_toggle_nav_flips_nav_open_state() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    engine
        .register_rule(toggle_nav_rule())
        .expect("register toggle_nav");

    // Initially false (or doesn't exist)
    let nav_id = make_node_id("sim/state/navOpen");

    // First toggle -> true
    let payload = AtomPayload::new(make_type_id("intent:toggle_nav"), Bytes::new());
    engine.ingest_inbox_event(1, &payload).unwrap();
    let event_1 = make_node_id("sim/inbox/event:0000000000000001");

    let tx = engine.begin();
    engine
        .apply(tx, TOGGLE_NAV_RULE_NAME, &event_1)
        .expect("apply 1");
    engine.commit(tx).expect("commit 1");

    let store = engine.store_clone();
    let AttachmentValue::Atom(atom) = store.node_attachment(&nav_id).expect("nav attachment 1")
    else {
        panic!("expected atom attachment on navOpen node");
    };
    assert_eq!(atom.bytes, Bytes::from_static(b"true"));

    // Second toggle -> false
    engine.ingest_inbox_event(2, &payload).unwrap();
    let event_2 = make_node_id("sim/inbox/event:0000000000000002");

    let tx = engine.begin();
    engine
        .apply(tx, TOGGLE_NAV_RULE_NAME, &event_2)
        .expect("apply 2");
    engine.commit(tx).expect("commit 2");

    let store = engine.store_clone();
    let AttachmentValue::Atom(atom) = store.node_attachment(&nav_id).expect("nav attachment 2")
    else {
        panic!("expected atom attachment on navOpen node");
    };
    assert_eq!(atom.bytes, Bytes::from_static(b"false"));
}
