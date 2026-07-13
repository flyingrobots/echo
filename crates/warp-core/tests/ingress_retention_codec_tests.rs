// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical retained-ingress codec witnesses.

use warp_core::{
    make_head_id, make_intent_kind, InboxAddress, IngressCausalParent, IngressEnvelope,
    IngressEnvelopeDecodeError, IngressTarget, WorldlineId, WriterHeadKey,
};

fn digest(label: &str) -> [u8; 32] {
    blake3::hash(label.as_bytes()).into()
}

fn round_trip(envelope: IngressEnvelope) {
    let retained = envelope.to_retained_bytes_v1();
    let decoded =
        IngressEnvelope::from_retained_bytes_v1(&retained).expect("retained ingress should decode");

    assert_eq!(decoded, envelope);
    assert_eq!(decoded.to_retained_bytes_v1(), retained);
}

#[test]
fn retained_ingress_round_trips_every_target_and_causal_parent() {
    let worldline_id = WorldlineId::from_bytes(digest("worldline"));
    let intent_kind = make_intent_kind("fixture.intent/retained-v1");
    let parent = IngressCausalParent::TickReceipt {
        receipt_digest: digest("parent-receipt"),
    };

    round_trip(IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::DefaultWriter { worldline_id },
        intent_kind,
        b"default".to_vec(),
        vec![parent],
    ));
    round_trip(IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::InboxAddress {
            worldline_id,
            inbox: InboxAddress("commands/primary".to_owned()),
        },
        intent_kind,
        b"inbox".to_vec(),
        vec![parent],
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
        vec![parent],
    ));
}

#[test]
fn retained_ingress_rejects_truncation_and_trailing_bytes() {
    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter {
            worldline_id: WorldlineId::from_bytes(digest("worldline")),
        },
        make_intent_kind("fixture.intent/retained-v1"),
        b"payload".to_vec(),
    );
    let retained = envelope.to_retained_bytes_v1();

    for end in 0..retained.len() {
        assert!(IngressEnvelope::from_retained_bytes_v1(&retained[..end]).is_err());
    }
    let mut trailing = retained;
    trailing.push(0);
    assert_eq!(
        IngressEnvelope::from_retained_bytes_v1(&trailing),
        Err(IngressEnvelopeDecodeError::TrailingBytes)
    );
}

#[test]
fn retained_ingress_rejects_non_canonical_duplicate_parents() {
    let parent = IngressCausalParent::TickReceipt {
        receipt_digest: digest("parent-receipt"),
    };
    let envelope = IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::DefaultWriter {
            worldline_id: WorldlineId::from_bytes(digest("worldline")),
        },
        make_intent_kind("fixture.intent/retained-v1"),
        b"payload".to_vec(),
        vec![parent],
    );
    let mut retained = envelope.to_retained_bytes_v1();
    const PARENT_COUNT_OFFSET: usize = 8 + 1 + 32;
    const FIRST_PARENT_OFFSET: usize = PARENT_COUNT_OFFSET + 8;
    const PARENT_ENCODED_LEN: usize = 1 + 32;
    retained[PARENT_COUNT_OFFSET..FIRST_PARENT_OFFSET].copy_from_slice(&2_u64.to_le_bytes());
    let duplicate =
        retained[FIRST_PARENT_OFFSET..FIRST_PARENT_OFFSET + PARENT_ENCODED_LEN].to_vec();
    retained.splice(
        FIRST_PARENT_OFFSET + PARENT_ENCODED_LEN..FIRST_PARENT_OFFSET + PARENT_ENCODED_LEN,
        duplicate,
    );

    assert_eq!(
        IngressEnvelope::from_retained_bytes_v1(&retained),
        Err(IngressEnvelopeDecodeError::NonCanonical)
    );
}
