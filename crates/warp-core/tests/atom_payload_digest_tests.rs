// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]

use bytes::Bytes;

use warp_core::{
    make_node_id, make_type_id, make_warp_id, AtomPayload, AttachmentKey, AttachmentValue, Engine,
    GraphStore, NodeKey, NodeRecord, TickCommitStatus, WarpInstance, WarpOp, WarpTickPatchV1,
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
    store_a.insert_node(root, NodeRecord { ty: node_ty });
    store_a.set_node_attachment(root, Some(AttachmentValue::Atom(payload_a)));
    let mut store_b = GraphStore::default();
    store_b.insert_node(root, NodeRecord { ty: node_ty });
    store_b.set_node_attachment(root, Some(AttachmentValue::Atom(payload_b)));

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
    let warp_id = make_warp_id("patch-digest-payload-type-test");
    let root = make_node_id("root");
    let node = make_node_id("node");
    let node_ty = make_type_id("entity");
    let bytes = Bytes::from_static(b"same-bytes");

    let instance = WarpInstance {
        warp_id,
        root_node: root,
        parent: None,
    };
    let node_key = NodeKey {
        warp_id,
        local_id: node,
    };

    let base_ops = vec![
        WarpOp::UpsertWarpInstance {
            instance: instance.clone(),
        },
        WarpOp::UpsertNode {
            node: node_key,
            record: NodeRecord { ty: node_ty },
        },
    ];

    let op_a = WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(node_key),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("payload/a"),
            bytes.clone(),
        ))),
    };
    let op_b = WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(node_key),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("payload/b"),
            bytes,
        ))),
    };

    let policy_id = warp_core::POLICY_ID_NO_POLICY_V0;
    let rule_pack_id = [0u8; 32];
    let patch_a = WarpTickPatchV1::new(
        policy_id,
        rule_pack_id,
        TickCommitStatus::Committed,
        Vec::new(),
        Vec::new(),
        base_ops.iter().cloned().chain([op_a]).collect(),
    );
    let patch_b = WarpTickPatchV1::new(
        policy_id,
        rule_pack_id,
        TickCommitStatus::Committed,
        Vec::new(),
        Vec::new(),
        base_ops.into_iter().chain([op_b]).collect(),
    );

    assert_ne!(
        patch_a.digest(),
        patch_b.digest(),
        "patch_digest must change when payload type_id changes"
    );
}
