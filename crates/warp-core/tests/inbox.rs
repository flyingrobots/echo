// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Inbox ingestion scaffolding tests.

use bytes::Bytes;
use warp_core::{
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
fn ingest_inbox_event_creates_path_and_payload() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    let payload_bytes = Bytes::from_static(br#"{\"path\":\"/\"}"#);
    let payload = AtomPayload::new(make_type_id("intent:route_push"), payload_bytes.clone());

    engine
        .ingest_inbox_event(42, &payload)
        .expect("ingest should succeed");

    let store = engine.store_clone();

    let sim_id = make_node_id("sim");
    let inbox_id = make_node_id("sim/inbox");
    let event_id = make_node_id("sim/inbox/event:0000000000000042");

    // Nodes exist with expected types
    assert_eq!(store.node(&sim_id).unwrap().ty, make_type_id("sim"));
    assert_eq!(store.node(&inbox_id).unwrap().ty, make_type_id("sim/inbox"));
    assert_eq!(
        store.node(&event_id).unwrap().ty,
        make_type_id("sim/inbox/event")
    );

    // Event attachment is present and matches payload
    let attachment = store
        .node_attachment(&event_id)
        .and_then(|v| match v {
            AttachmentValue::Atom(a) => Some(a),
            _ => None,
        })
        .expect("event attachment");
    assert_eq!(attachment.type_id, payload.type_id);
    assert_eq!(attachment.bytes, payload.bytes);
}
