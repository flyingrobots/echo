// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]

use bytes::Bytes;

use warp_core::{
    make_node_id, make_type_id, AtomPayload, Engine, GraphStore, NodeRecord, TickCommitStatus,
    WarpOp, WarpTickPatchV1,
};

#[test]
fn commit_hash_changes_when_payload_type_changes_even_if_bytes_match() {
    // This is a safety/determinism invariant for typed atoms:
    // same bytes + different type id must not collide at the boundary hash.
    let root = make_node_id("root");
    let node_ty = make_type_id("entity");
    let bytes = Bytes::from_static(b"same-bytes");

    let payload_a = AtomPayload::new(make_type_id("payload/a"), bytes.clone());
    let payload_b = AtomPayload::new(make_type_id("payload/b"), bytes);

    let mut store_a = GraphStore::default();
    store_a.insert_node(
        root,
        NodeRecord {
            ty: node_ty,
            payload: Some(payload_a),
        },
    );
    let mut store_b = GraphStore::default();
    store_b.insert_node(
        root,
        NodeRecord {
            ty: node_ty,
            payload: Some(payload_b),
        },
    );

    let engine_a = Engine::new(store_a, root);
    let engine_b = Engine::new(store_b, root);
    assert_ne!(
        engine_a.snapshot().hash,
        engine_b.snapshot().hash,
        "commit hash must change when payload type_id changes"
    );
}

#[test]
fn tick_patch_digest_changes_when_payload_type_changes_even_if_bytes_match() {
    let node = make_node_id("node");
    let node_ty = make_type_id("entity");
    let bytes = Bytes::from_static(b"same-bytes");

    let record_a = NodeRecord {
        ty: node_ty,
        payload: Some(AtomPayload::new(make_type_id("payload/a"), bytes.clone())),
    };
    let record_b = NodeRecord {
        ty: node_ty,
        payload: Some(AtomPayload::new(make_type_id("payload/b"), bytes)),
    };

    let op_a = WarpOp::UpsertNode {
        node,
        record: record_a,
    };
    let op_b = WarpOp::UpsertNode {
        node,
        record: record_b,
    };

    let policy_id = warp_core::POLICY_ID_NO_POLICY_V0;
    let rule_pack_id = [0u8; 32];
    let patch_a = WarpTickPatchV1::new(
        policy_id,
        rule_pack_id,
        TickCommitStatus::Committed,
        Vec::new(),
        Vec::new(),
        vec![op_a],
    );
    let patch_b = WarpTickPatchV1::new(
        policy_id,
        rule_pack_id,
        TickCommitStatus::Committed,
        Vec::new(),
        Vec::new(),
        vec![op_b],
    );

    assert_ne!(
        patch_a.digest(),
        patch_b.digest(),
        "patch_digest must change when payload type_id changes"
    );
}
