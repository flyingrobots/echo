// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical retained-ingress codec witnesses.
#![allow(clippy::expect_used)]

use warp_core::{
    make_head_id, make_intent_kind, CausalTickReceiptRef, ContinuumOccurrence,
    ContinuumOccurrenceError, GlobalTick, InboxAddress, IngressCausalParent, IngressEnvelope,
    IngressEnvelopeDecodeError, IngressTarget, WorldlineId, WorldlineTick, WriterHeadKey,
    CAUSAL_TICK_RECEIPT_REF_LEN,
};

const PARENT_COUNT_OFFSET: usize = 8 + 1 + 32;
const FIRST_PARENT_OFFSET: usize = PARENT_COUNT_OFFSET + 8;
const PARENT_ENCODED_LEN: usize = 1 + CAUSAL_TICK_RECEIPT_REF_LEN;

fn digest(label: &str) -> [u8; 32] {
    blake3::hash(label.as_bytes()).into()
}

fn round_trip(envelope: IngressEnvelope) {
    let retained = envelope.to_retained_bytes();
    let decoded =
        IngressEnvelope::from_retained_bytes(&retained).expect("retained ingress should decode");

    assert_eq!(decoded, envelope);
    assert_eq!(decoded.to_retained_bytes(), retained);
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
    let retained = envelope.to_retained_bytes();

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
    let mut retained = envelope.to_retained_bytes();
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

// Golden vector produced by Continuum's own `packages/core/src/index.mjs`
// (flyingrobots/continuum main @ c33caa3e): `makeOccurrenceRef({ issuer:
// { kind: 'client', id: 'kitten-a' }, localId: 'act-1' })`, an applied digest
// of `hashCanonicalJson({ kind: 'intent', label: 'increment' })`, and
// `hashCanonicalJson({ occurrenceKeyDigest, appliedDigest })`.
const CONTINUUM_OCCURRENCE_KEY: &str =
    "2e87cc841b0c436681496707b3b326afc29e0f618ea7cb4fe199602d7a8ba6f2";
const CONTINUUM_APPLIED_DIGEST: &str =
    "4c77258ae4512c9cea4b1f072ed0800be3be241ace6ae3d2bc28ece3e2777bda";
const CONTINUUM_OCCURRED_DIGEST: &str =
    "7beefe7d630208f6a3efae98ed89939db624c995d260708865a57e2b5801b7a5";

fn from_hex(hex: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).expect("hex digit pair");
    }
    out
}

fn continuum_occurrence(label: &str) -> ContinuumOccurrence {
    ContinuumOccurrence::new(digest(&format!("{label}:key")), digest("applied"))
}

fn occurred_envelope(occurrence: ContinuumOccurrence, bytes: &[u8]) -> IngressEnvelope {
    IngressEnvelope::continuum_occurred_intent(
        IngressTarget::DefaultWriter {
            worldline_id: WorldlineId::from_bytes(digest("worldline")),
        },
        make_intent_kind("fixture.intent/occurred"),
        bytes.to_vec(),
        Vec::new(),
        occurrence,
    )
}

#[test]
fn continuum_occurred_digest_matches_continuums_own_vector() {
    let occurrence = ContinuumOccurrence::from_occurred_digests(
        from_hex(CONTINUUM_OCCURRENCE_KEY),
        from_hex(CONTINUUM_APPLIED_DIGEST),
        from_hex(CONTINUUM_OCCURRED_DIGEST),
    )
    .expect("Continuum's occurred digest binds its key and applied digest");
    assert_eq!(
        occurrence.occurred_digest(),
        from_hex(CONTINUUM_OCCURRED_DIGEST)
    );

    let mut wrong = from_hex(CONTINUUM_OCCURRED_DIGEST);
    wrong[0] ^= 1;
    assert_eq!(
        ContinuumOccurrence::from_occurred_digests(
            from_hex(CONTINUUM_OCCURRENCE_KEY),
            from_hex(CONTINUUM_APPLIED_DIGEST),
            wrong,
        ),
        Err(ContinuumOccurrenceError::OccurredDigestMismatch {
            expected: from_hex(CONTINUUM_OCCURRED_DIGEST),
            found: wrong,
        })
    );
}

#[test]
fn occurrence_identity_separates_occurrences_and_leaves_local_identity_alone() {
    let local = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter {
            worldline_id: WorldlineId::from_bytes(digest("worldline")),
        },
        make_intent_kind("fixture.intent/occurred"),
        b"same-content".to_vec(),
    );
    let first = occurred_envelope(continuum_occurrence("first"), b"same-content");
    let redelivered = occurred_envelope(continuum_occurrence("first"), b"same-content");
    let second = occurred_envelope(continuum_occurrence("second"), b"same-content");

    assert_eq!(first.ingress_id(), redelivered.ingress_id());
    assert_ne!(first.ingress_id(), second.ingress_id());
    assert_ne!(first.ingress_id(), local.ingress_id());
    assert_eq!(first.content_ingress_id(), local.ingress_id());
    assert_eq!(second.content_ingress_id(), local.ingress_id());
    assert_eq!(local.content_ingress_id(), local.ingress_id());
    assert_eq!(local.occurrence(), None);
    assert_eq!(&local.to_retained_bytes()[..8], b"EINGR002");
}

#[test]
fn retained_ingress_round_trips_a_continuum_occurrence() {
    let envelope = occurred_envelope(continuum_occurrence("retained"), b"occurred");
    let retained = envelope.to_retained_bytes();
    assert_eq!(&retained[..8], b"EINGR003");
    round_trip(envelope.clone());
    let decoded = IngressEnvelope::from_retained_bytes(&retained).expect("EINGR003 decodes");
    assert_eq!(decoded.occurrence(), envelope.occurrence());
    assert_eq!(decoded.ingress_id(), envelope.ingress_id());

    // Dropping the occurrence tail, or presenting it under the v2 magic, is
    // refused rather than read as a different envelope.
    let truncated = &retained[..retained.len() - 32];
    assert_eq!(
        IngressEnvelope::from_retained_bytes(truncated),
        Err(IngressEnvelopeDecodeError::UnexpectedEof)
    );
    let mut as_v2 = retained.clone();
    as_v2[..8].copy_from_slice(b"EINGR002");
    assert_eq!(
        IngressEnvelope::from_retained_bytes(&as_v2),
        Err(IngressEnvelopeDecodeError::TrailingBytes)
    );
    let mut trailing = retained;
    trailing.push(0);
    assert_eq!(
        IngressEnvelope::from_retained_bytes(&trailing),
        Err(IngressEnvelopeDecodeError::TrailingBytes)
    );
}

/// Code review of #884: the occurrence hash input must not be reproducible by
/// a local intent. Local ingress hashes `"ingress:" || kind || bytes`, and the
/// kind is any 32 bytes, so an occurrence domain beginning with `"ingress:"`
/// would let a crafted local intent take an occurrence's ingress id.
#[test]
fn a_local_intent_cannot_take_an_occurrence_ingress_id() {
    let occurred = occurred_envelope(continuum_occurrence("prefix"), b"content");
    let key = occurred
        .occurrence()
        .expect("occurred envelope carries its occurrence")
        .occurrence_key_digest();
    let old_domain = b"ingress:continuum-occurrence:v1\0";
    let mut kind = [0u8; 32];
    kind[..24].copy_from_slice(&old_domain[8..]);
    kind[24..].copy_from_slice(&key[..8]);
    let mut bytes = key[8..].to_vec();
    bytes.extend_from_slice(&occurred.content_ingress_id());
    let crafted = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter {
            worldline_id: WorldlineId::from_bytes(digest("worldline")),
        },
        warp_core::IntentKind::from_hash(kind),
        bytes,
    );
    assert_ne!(crafted.ingress_id(), occurred.ingress_id());
}
