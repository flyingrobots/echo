// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical retained-ingress codec witnesses.
#![allow(clippy::expect_used)]

use warp_core::{
    make_head_id, make_intent_kind, CausalTickReceiptRef, GlobalTick, InboxAddress,
    IngressCausalParent, IngressEnvelope, IngressEnvelopeDecodeError, IngressTarget, WorldlineId,
    WorldlineTick, WriterHeadKey, CAUSAL_TICK_RECEIPT_REF_LEN,
};

const PARENT_COUNT_OFFSET: usize = 8 + 1 + 32;
const FIRST_PARENT_OFFSET: usize = PARENT_COUNT_OFFSET + 8;
const PARENT_ENCODED_LEN: usize = 1 + CAUSAL_TICK_RECEIPT_REF_LEN;

fn digest(label: &str) -> [u8; 32] {
    blake3::hash(label.as_bytes()).into()
}

fn round_trip(envelope: IngressEnvelope) {
    let retained = envelope.to_retained_bytes_v2();
    let decoded =
        IngressEnvelope::from_retained_bytes(&retained).expect("retained ingress should decode");

    assert_eq!(decoded, envelope);
    assert_eq!(decoded.to_retained_bytes_v2(), retained);
}

fn receipt_ref(worldline_id: WorldlineId, label: &str) -> CausalTickReceiptRef {
    CausalTickReceiptRef {
        worldline_id,
        worldline_tick_after: WorldlineTick::from_raw(7),
        commit_global_tick: GlobalTick::from_raw(11),
        commit_hash: digest(&format!("{label}:commit")),
        submission_id: digest(&format!("{label}:submission")),
        ticket_digest: digest(&format!("{label}:ticket")),
        receipt_content_digest: digest(&format!("{label}:content")),
    }
}

#[test]
fn retained_ingress_round_trips_every_target_and_causal_parent() {
    let worldline_id = WorldlineId::from_bytes(digest("worldline"));
    let intent_kind = make_intent_kind("fixture.intent/retained-v2");
    let parent = IngressCausalParent::TickReceipt {
        receipt_ref: receipt_ref(worldline_id, "parent-receipt"),
    };
    let inverse_target = IngressCausalParent::ContractInverseTarget {
        receipt_ref: receipt_ref(worldline_id, "inverse-target"),
    };

    round_trip(IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::DefaultWriter { worldline_id },
        intent_kind,
        b"default".to_vec(),
        vec![parent, inverse_target],
    ));
    round_trip(IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::InboxAddress {
            worldline_id,
            inbox: InboxAddress("commands/primary".to_owned()),
        },
        intent_kind,
        b"inbox".to_vec(),
        vec![parent, inverse_target],
    ));
    round_trip(IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::ExactHead {
            key: WriterHeadKey {
                worldline_id,
                head_id: make_head_id("fixture-head"),
            },
        },
        intent_kind,
        b"exact".to_vec(),
        vec![parent, inverse_target],
    ));
}

#[test]
fn causal_parent_role_changes_ingress_identity() {
    let worldline_id = WorldlineId::from_bytes(digest("worldline"));
    let receipt_ref = receipt_ref(worldline_id, "shared-receipt");
    let dependency = IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("fixture.intent/retained-v2"),
        b"payload".to_vec(),
        vec![IngressCausalParent::TickReceipt { receipt_ref }],
    );
    let inverse = IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("fixture.intent/retained-v2"),
        b"payload".to_vec(),
        vec![IngressCausalParent::ContractInverseTarget { receipt_ref }],
    );

    assert_ne!(dependency.ingress_id(), inverse.ingress_id());
}

#[test]
fn retained_ingress_rejects_truncation_and_trailing_bytes() {
    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter {
            worldline_id: WorldlineId::from_bytes(digest("worldline")),
        },
        make_intent_kind("fixture.intent/retained-v2"),
        b"payload".to_vec(),
    );
    let retained = envelope.to_retained_bytes_v2();

    for end in 0..retained.len() {
        assert!(IngressEnvelope::from_retained_bytes(&retained[..end]).is_err());
    }
    let mut trailing = retained;
    trailing.push(0);
    assert_eq!(
        IngressEnvelope::from_retained_bytes(&trailing),
        Err(IngressEnvelopeDecodeError::TrailingBytes)
    );
}

#[test]
fn retained_ingress_rejects_non_canonical_duplicate_parents() {
    let worldline_id = WorldlineId::from_bytes(digest("worldline"));
    let parent = IngressCausalParent::TickReceipt {
        receipt_ref: receipt_ref(worldline_id, "parent-receipt"),
    };
    let envelope = IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("fixture.intent/retained-v2"),
        b"payload".to_vec(),
        vec![parent],
    );
    let mut retained = envelope.to_retained_bytes_v2();
    retained[PARENT_COUNT_OFFSET..FIRST_PARENT_OFFSET].copy_from_slice(&2_u64.to_le_bytes());
    let duplicate =
        retained[FIRST_PARENT_OFFSET..FIRST_PARENT_OFFSET + PARENT_ENCODED_LEN].to_vec();
    retained.splice(
        FIRST_PARENT_OFFSET + PARENT_ENCODED_LEN..FIRST_PARENT_OFFSET + PARENT_ENCODED_LEN,
        duplicate,
    );

    assert_eq!(
        IngressEnvelope::from_retained_bytes(&retained),
        Err(IngressEnvelopeDecodeError::NonCanonical)
    );
}

#[test]
fn legacy_parentless_retained_ingress_remains_decodable() {
    let worldline_id = WorldlineId::from_bytes(digest("legacy-worldline"));
    let intent_kind = make_intent_kind("fixture.intent/retained-v1");
    let payload = b"legacy-parentless";
    let mut retained = b"EINGR001".to_vec();
    retained.push(1);
    retained.extend_from_slice(worldline_id.as_bytes());
    retained.extend_from_slice(&0_u64.to_le_bytes());
    retained.push(1);
    retained.extend_from_slice(intent_kind.as_hash());
    retained.extend_from_slice(
        &u64::try_from(payload.len())
            .expect("fixture length")
            .to_le_bytes(),
    );
    retained.extend_from_slice(payload);

    let decoded = IngressEnvelope::from_retained_bytes(&retained)
        .expect("parentless v1 ingress should remain recoverable");
    assert_eq!(
        decoded.target(),
        &IngressTarget::DefaultWriter { worldline_id }
    );
    assert!(decoded.causal_parents().is_empty());
}

#[test]
fn legacy_tick_receipt_parent_is_rejected_as_ambiguous() {
    let receipt_digest = digest("legacy-parent-receipt");
    let intent_kind = make_intent_kind("fixture.intent/legacy-parent");
    let payload = b"legacy-parent-payload";
    let mut retained = b"EINGR001".to_vec();
    retained.push(1);
    retained.extend_from_slice(WorldlineId::from_bytes(digest("legacy-worldline")).as_bytes());
    retained.extend_from_slice(&1_u64.to_le_bytes());
    retained.push(1);
    retained.extend_from_slice(&receipt_digest);
    retained.push(1);
    retained.extend_from_slice(intent_kind.as_hash());
    retained.extend_from_slice(
        &u64::try_from(payload.len())
            .expect("fixture length")
            .to_le_bytes(),
    );
    retained.extend_from_slice(payload);

    assert_eq!(
        IngressEnvelope::from_retained_bytes(&retained),
        Err(IngressEnvelopeDecodeError::AmbiguousLegacyTickReceiptParent { receipt_digest })
    );
}

#[test]
fn malformed_legacy_parent_list_is_not_mislabeled_as_ambiguous() {
    let mut retained = b"EINGR001".to_vec();
    retained.push(1);
    retained.extend_from_slice(WorldlineId::from_bytes(digest("legacy-worldline")).as_bytes());
    retained.extend_from_slice(&2_u64.to_le_bytes());
    retained.push(1);
    retained.extend_from_slice(&digest("only-parent"));

    assert_eq!(
        IngressEnvelope::from_retained_bytes(&retained),
        Err(IngressEnvelopeDecodeError::UnexpectedEof)
    );
}
