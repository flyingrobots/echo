// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical replayable-provenance retention codec witnesses.
#![allow(clippy::expect_used)]

use bytes::Bytes;
use warp_core::{
    causal_wal::WalRuntimeStateDeltaRecord, compute_commit_hash_v2, make_edge_id, make_head_id,
    make_node_id, make_type_id, make_warp_id, AtomPayload, AtomWrite, AttachmentKey,
    AttachmentValue, ContractEvidenceIdentity, ContractOperationKind, EdgeKey, EdgeRecord,
    GlobalTick, Hash, HashTriplet, InstalledContractPackageId, NodeKey, NodeRecord, PortalInit,
    ProvenanceEntry, ProvenanceEventKind, ProvenanceRef, RetainedProvenanceError, SlotId,
    TickCommitStatus, TickReceipt, TickReceiptDisposition, TickReceiptEntry, TxId, WarpInstance,
    WarpOp, WarpTickPatchV1, WorldlineId, WorldlineTick, WorldlineTickHeaderV1,
    WorldlineTickPatchV1, WriterHeadKey,
};

const OUTER_HEADER_LEN: usize = 8 + 32 + 8;
const ENTRY_PREFIX_THROUGH_PARENTS: usize = 8 + 32 + 8 + 8 + 1 + 64 + 8 + 72;
const EXPECTED_COMMIT_HASH_OFFSET: usize =
    OUTER_HEADER_LEN + ENTRY_PREFIX_THROUGH_PARENTS + 32 + 32;

fn digest(label: &str) -> Hash {
    blake3::hash(label.as_bytes()).into()
}

fn fixture_entry() -> ProvenanceEntry {
    let worldline_id = WorldlineId::from_bytes(digest("worldline"));
    let root_warp = make_warp_id("root");
    let child_warp = make_warp_id("child");
    let other_child_warp = make_warp_id("other-child");
    let root_node = NodeKey {
        warp_id: root_warp,
        local_id: make_node_id("root"),
    };
    let sibling_node = NodeKey {
        warp_id: root_warp,
        local_id: make_node_id("sibling"),
    };
    let edge_id = make_edge_id("root-to-sibling");
    let edge_key = EdgeKey {
        warp_id: root_warp,
        local_id: edge_id,
    };
    let root_attachment = AttachmentKey::node_alpha(root_node);
    let sibling_attachment = AttachmentKey::node_alpha(sibling_node);
    let edge_attachment = AttachmentKey::edge_beta(edge_key);
    let node_type = make_type_id("fixture/node");
    let edge_type = make_type_id("fixture/edge");
    let atom_type = make_type_id("fixture/atom");
    let ops = vec![
        WarpOp::OpenPortal {
            key: root_attachment,
            child_warp,
            child_root: make_node_id("child-root"),
            init: PortalInit::Empty {
                root_record: NodeRecord { ty: node_type },
            },
        },
        WarpOp::OpenPortal {
            key: sibling_attachment,
            child_warp: other_child_warp,
            child_root: make_node_id("other-child-root"),
            init: PortalInit::RequireExisting,
        },
        WarpOp::UpsertWarpInstance {
            instance: WarpInstance {
                warp_id: child_warp,
                root_node: make_node_id("child-root"),
                parent: Some(root_attachment),
            },
        },
        WarpOp::DeleteWarpInstance {
            warp_id: make_warp_id("deleted-child"),
        },
        WarpOp::UpsertNode {
            node: root_node,
            record: NodeRecord { ty: node_type },
        },
        WarpOp::DeleteNode { node: sibling_node },
        WarpOp::UpsertEdge {
            warp_id: root_warp,
            record: EdgeRecord {
                id: edge_id,
                from: root_node.local_id,
                to: sibling_node.local_id,
                ty: edge_type,
            },
        },
        WarpOp::DeleteEdge {
            warp_id: root_warp,
            from: root_node.local_id,
            edge_id: make_edge_id("deleted-edge"),
        },
        WarpOp::SetAttachment {
            key: root_attachment,
            value: Some(AttachmentValue::Atom(AtomPayload::new(
                atom_type,
                Bytes::from_static(b"atom-bytes"),
            ))),
        },
        WarpOp::SetAttachment {
            key: sibling_attachment,
            value: Some(AttachmentValue::Descend(other_child_warp)),
        },
        WarpOp::SetAttachment {
            key: edge_attachment,
            value: None,
        },
    ];
    let patch = WarpTickPatchV1::new(
        17,
        digest("rule-pack"),
        TickCommitStatus::Committed,
        vec![
            SlotId::Node(root_node),
            SlotId::Edge(edge_key),
            SlotId::Attachment(root_attachment),
            SlotId::Port((root_warp, 41)),
        ],
        vec![
            SlotId::Node(sibling_node),
            SlotId::Edge(edge_key),
            SlotId::Attachment(edge_attachment),
            SlotId::Port((root_warp, 42)),
        ],
        ops,
    );
    let parent = ProvenanceRef {
        worldline_id,
        worldline_tick: WorldlineTick::from_raw(6),
        commit_hash: digest("parent-commit"),
    };
    let state_root = digest("state-root");
    let patch_digest = patch.digest();
    let commit_hash = compute_commit_hash_v2(
        &state_root,
        &[parent.commit_hash],
        &patch_digest,
        patch.policy_id(),
    );
    let receipt = TickReceipt::try_from_retained_parts(
        TxId::from_raw(8),
        vec![TickReceiptEntry {
            rule_id: digest("receipt-rule"),
            scope_hash: digest("receipt-scope"),
            scope: root_node,
            disposition: TickReceiptDisposition::Applied,
        }],
        vec![Vec::new()],
    )
    .expect("fixture receipt parts should be parallel");
    let decision_digest = receipt.digest();
    ProvenanceEntry::local_commit(
        worldline_id,
        WorldlineTick::from_raw(7),
        GlobalTick::from_raw(19),
        WriterHeadKey {
            worldline_id,
            head_id: make_head_id("writer"),
        },
        vec![parent],
        HashTriplet {
            state_root,
            patch_digest,
            commit_hash,
        },
        WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                commit_global_tick: GlobalTick::from_raw(19),
                policy_id: patch.policy_id(),
                rule_pack_id: patch.rule_pack_id(),
                plan_digest: digest("plan"),
                decision_digest,
                rewrites_digest: digest("rewrites"),
            },
            warp_id: root_warp,
            ops: patch.ops().to_vec(),
            in_slots: patch.in_slots().to_vec(),
            out_slots: patch.out_slots().to_vec(),
            patch_digest,
        },
        vec![(make_type_id("fixture/channel"), b"output".to_vec())],
        vec![AtomWrite::new(
            root_node,
            digest("rule"),
            19,
            Some(b"before".to_vec()),
            b"after".to_vec(),
        )],
    )
    .with_tick_receipt(receipt)
}

fn fixture_receipt_digest(entry: &ProvenanceEntry) -> Hash {
    entry
        .tick_receipt
        .as_ref()
        .expect("fixture local commit has a receipt")
        .digest()
}

fn fixture_contract() -> ContractEvidenceIdentity {
    ContractEvidenceIdentity {
        package_id: InstalledContractPackageId::from_bytes(digest("package")),
        echo_abi_version: 3,
        package_name: "fixture-package".to_owned(),
        package_version: "1.2.3".to_owned(),
        artifact_hash_hex: "0123456789abcdef".to_owned(),
        codec_id: "fixture-codec".to_owned(),
        registry_version: 5,
        wesley_generator_version: "6.7.8".to_owned(),
        helper_api_version: 9,
        schema_sha256_hex: "abcdef0123456789".to_owned(),
        op_id: 42,
        op_kind: ContractOperationKind::Mutation,
    }
}

#[test]
fn replayable_state_delta_round_trips_every_operation_and_slot_variant() {
    let entry = fixture_entry();
    let receipt_digest = fixture_receipt_digest(&entry);
    let contract = fixture_contract();
    let record = WalRuntimeStateDeltaRecord::from_provenance_entry(
        receipt_digest,
        Some(contract.clone()),
        entry.clone(),
    )
    .expect("canonical local commit should be retainable");
    let bytes = record
        .to_payload_bytes()
        .expect("validated record should encode");
    let decoded = WalRuntimeStateDeltaRecord::from_payload_bytes(&bytes)
        .expect("retained state delta should decode");

    assert_eq!(decoded.receipt_digest(), receipt_digest);
    assert_eq!(decoded.contract(), Some(&contract));
    assert_eq!(decoded.provenance_entry(), &entry);
    assert_eq!(
        decoded.to_payload_bytes().expect("decoded record encodes"),
        bytes
    );
}

#[test]
fn replayable_state_delta_rejects_every_truncation_and_trailing_material() {
    let entry = fixture_entry();
    let record = WalRuntimeStateDeltaRecord::from_provenance_entry(
        fixture_receipt_digest(&entry),
        None,
        entry,
    )
    .expect("canonical local commit should be retainable");
    let bytes = record
        .to_payload_bytes()
        .expect("validated record should encode");

    for end in 0..bytes.len() {
        assert!(WalRuntimeStateDeltaRecord::from_payload_bytes(&bytes[..end]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        WalRuntimeStateDeltaRecord::from_payload_bytes(&trailing),
        Err(RetainedProvenanceError::TrailingBytes)
    );
}

#[test]
fn replayable_state_delta_rejects_corrupt_commitment_and_non_local_event() {
    let entry = fixture_entry();
    let record = WalRuntimeStateDeltaRecord::from_provenance_entry(
        fixture_receipt_digest(&entry),
        None,
        entry.clone(),
    )
    .expect("canonical local commit should be retainable");
    let mut bytes = record
        .to_payload_bytes()
        .expect("validated record should encode");
    bytes[EXPECTED_COMMIT_HASH_OFFSET] ^= 0x80;
    assert!(WalRuntimeStateDeltaRecord::from_payload_bytes(&bytes).is_err());

    let mut missing_receipt = entry.clone();
    missing_receipt.tick_receipt = None;
    assert_eq!(
        WalRuntimeStateDeltaRecord::from_provenance_entry(
            fixture_receipt_digest(&entry),
            None,
            missing_receipt,
        ),
        Err(RetainedProvenanceError::MissingTickReceipt)
    );

    let non_local = ProvenanceEntry::recorded_event(
        entry.worldline_id,
        entry.worldline_tick,
        entry.commit_global_tick,
        entry.parents,
        ProvenanceEventKind::ConflictArtifact {
            artifact_id: digest("conflict"),
        },
        entry.expected,
        entry.patch.expect("fixture local commit has patch"),
        entry.outputs,
        entry.atom_writes,
    );
    assert_eq!(
        WalRuntimeStateDeltaRecord::from_provenance_entry(digest("receipt"), None, non_local),
        Err(RetainedProvenanceError::UnsupportedEventKind)
    );
}
