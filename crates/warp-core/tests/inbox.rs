// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Inbox ingestion scaffolding tests.

use bytes::Bytes;
use echo_dry_tests::build_engine_with_root;
use warp_core::{make_node_id, make_type_id, AtomPayload, AttachmentValue, Hash, NodeId};

#[test]
fn ingest_inbox_event_creates_path_and_pending_edge_from_opaque_intent_bytes() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    // Core is byte-blind: any bytes are valid intents.
    let intent_bytes: &[u8] = b"opaque-test-intent";
    let payload_bytes = Bytes::copy_from_slice(intent_bytes);
    let payload = AtomPayload::new(make_type_id("legacy/payload"), payload_bytes.clone());

    engine
        .ingest_inbox_event(42, &payload)
        .expect("ingest should succeed");

    let store = engine.store_clone();

    let sim_id = make_node_id("sim");
    let inbox_id = make_node_id("sim/inbox");
    let intent_id: Hash = {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"intent:");
        hasher.update(intent_bytes);
        hasher.finalize().into()
    };
    let event_id = NodeId(intent_id);

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
    assert_eq!(attachment.type_id, make_type_id("intent"));
    assert_eq!(attachment.bytes, payload_bytes);

    // Pending membership is an edge from inbox → event.
    let pending_ty = make_type_id("edge:pending");
    assert!(
        store
            .edges_from(&inbox_id)
            .any(|e| e.ty == pending_ty && e.to == event_id),
        "expected a pending edge from sim/inbox → event"
    );
}

#[test]
fn ingest_inbox_event_is_idempotent_by_intent_bytes_not_seq() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    let intent_bytes: &[u8] = b"idempotent-intent";
    let payload_bytes = Bytes::copy_from_slice(intent_bytes);
    let payload = AtomPayload::new(make_type_id("legacy/payload"), payload_bytes.clone());

    engine.ingest_inbox_event(1, &payload).unwrap();
    engine.ingest_inbox_event(2, &payload).unwrap();

    let store = engine.store_clone();

    let sim_id = make_node_id("sim");
    let inbox_id = make_node_id("sim/inbox");

    // Only one structural edge root->sim and sim->inbox should exist.
    let root_edges: Vec<_> = store.edges_from(&root).collect();
    assert_eq!(root_edges.len(), 1);
    assert_eq!(root_edges[0].to, sim_id);

    let sim_edges: Vec<_> = store.edges_from(&sim_id).collect();
    assert_eq!(sim_edges.len(), 1);
    assert_eq!(sim_edges[0].to, inbox_id);

    // Ingress idempotency is keyed by intent_id, so the same intent_bytes must not create
    // additional events or pending edges even if callers vary the seq input.
    let pending_ty = make_type_id("edge:pending");
    let inbox_pending_edges: Vec<_> = store
        .edges_from(&inbox_id)
        .filter(|e| e.ty == pending_ty)
        .collect();
    assert_eq!(inbox_pending_edges.len(), 1);

    let intent_id: Hash = {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"intent:");
        hasher.update(intent_bytes);
        hasher.finalize().into()
    };
    assert!(store.node(&NodeId(intent_id)).is_some());
}

#[test]
fn ingest_inbox_event_creates_distinct_events_for_distinct_intents() {
    let root = make_node_id("root");
    let mut engine = build_engine_with_root(root);

    let intent_a: &[u8] = b"intent-alpha";
    let intent_b: &[u8] = b"intent-beta";
    let payload_a = AtomPayload::new(
        make_type_id("legacy/payload"),
        Bytes::copy_from_slice(intent_a),
    );
    let payload_b = AtomPayload::new(
        make_type_id("legacy/payload"),
        Bytes::copy_from_slice(intent_b),
    );

    engine.ingest_inbox_event(1, &payload_a).unwrap();
    engine.ingest_inbox_event(2, &payload_b).unwrap();

    let store = engine.store_clone();
    let inbox_id = make_node_id("sim/inbox");

    let pending_ty = make_type_id("edge:pending");
    let inbox_pending_edges: Vec<_> = store
        .edges_from(&inbox_id)
        .filter(|e| e.ty == pending_ty)
        .collect();
    assert_eq!(inbox_pending_edges.len(), 2);

    let intent_id_a: Hash = {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"intent:");
        hasher.update(intent_a);
        hasher.finalize().into()
    };
    let intent_id_b: Hash = {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"intent:");
        hasher.update(intent_b);
        hasher.finalize().into()
    };

    assert!(store.node(&NodeId(intent_id_a)).is_some());
    assert!(store.node(&NodeId(intent_id_b)).is_some());
}

// NOTE: The `ingest_inbox_event_ignores_invalid_intent_bytes_without_mutating_graph` test
// was removed because the core is now byte-blind: all bytes are valid intents and
// validation is the caller's responsibility (hexagonal architecture).
