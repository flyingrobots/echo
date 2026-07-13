// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Durable receipt-parent recovery witnesses for contract-defined inverse intents.

use warp_core::causal_wal::{
    RecoveredReceiptIndex, TickReceiptRecord, WalReceiptCorrelationRecord, WalTickDecision,
};
use warp_core::{
    make_intent_kind, HeadId, IngressCausalParent, IngressEnvelope, IngressTarget, WorldlineId,
    WriterHeadKey,
};

fn digest(label: &str) -> [u8; 32] {
    blake3::hash(label.as_bytes()).into()
}

fn writer_head() -> WriterHeadKey {
    WriterHeadKey {
        worldline_id: WorldlineId::from_bytes(digest("worldline")),
        head_id: HeadId::from_bytes(digest("head")),
    }
}

#[test]
fn causal_parent_receipt_changes_ingress_identity_and_is_canonicalized() {
    let target_receipt = digest("target-receipt");
    let other_receipt = digest("other-receipt");
    let target = IngressTarget::DefaultWriter {
        worldline_id: writer_head().worldline_id,
    };
    let intent_kind = make_intent_kind("fixture.intent/replace-v1");
    let intent_bytes = b"same inverse payload".to_vec();

    let first = IngressEnvelope::local_intent_with_causal_parents(
        target.clone(),
        intent_kind,
        intent_bytes.clone(),
        vec![
            IngressCausalParent::TickReceipt {
                receipt_digest: target_receipt,
            },
            IngressCausalParent::TickReceipt {
                receipt_digest: other_receipt,
            },
            IngressCausalParent::TickReceipt {
                receipt_digest: target_receipt,
            },
        ],
    );
    let second = IngressEnvelope::local_intent_with_causal_parents(
        target,
        intent_kind,
        intent_bytes,
        vec![IngressCausalParent::TickReceipt {
            receipt_digest: other_receipt,
        }],
    );

    assert_ne!(first.ingress_id(), second.ingress_id());
    let mut expected_parents = vec![
        IngressCausalParent::TickReceipt {
            receipt_digest: target_receipt,
        },
        IngressCausalParent::TickReceipt {
            receipt_digest: other_receipt,
        },
    ];
    expected_parents.sort_unstable();
    assert_eq!(first.causal_parents(), expected_parents.as_slice());
}

#[test]
fn receipt_correlation_decoder_accepts_legacy_parentless_payload() {
    let submission_id = digest("legacy-submission");
    let ticket_digest = digest("legacy-ticket");
    let receipt_digest = digest("legacy-receipt");
    let mut legacy_payload = Vec::new();
    legacy_payload.extend_from_slice(&submission_id);
    legacy_payload.extend_from_slice(&ticket_digest);
    legacy_payload.extend_from_slice(&receipt_digest);

    let decoded = WalReceiptCorrelationRecord::from_payload_bytes(&legacy_payload)
        .expect("legacy correlation payload should remain readable");

    assert_eq!(decoded.submission_id, submission_id);
    assert_eq!(decoded.ticket_digest, ticket_digest);
    assert_eq!(decoded.receipt_digest, receipt_digest);
    assert!(decoded.causal_parent_receipts.is_empty());
    assert_eq!(decoded.to_payload_bytes(), legacy_payload);
}

#[test]
fn receipt_correlation_decoder_rejects_parent_count_beyond_payload() {
    let mut forged_payload = vec![0; 3 * core::mem::size_of::<[u8; 32]>()];
    forged_payload.extend_from_slice(&u64::MAX.to_le_bytes());

    assert!(WalReceiptCorrelationRecord::from_payload_bytes(&forged_payload).is_err());
}

#[test]
fn recovered_receipt_index_preserves_causal_parent_receipts() {
    let submission_id = digest("inverse-submission");
    let ticket_digest = digest("inverse-ticket");
    let inverse_receipt = digest("inverse-receipt");
    let target_receipt = digest("target-receipt");
    let receipt = TickReceiptRecord {
        submission_id,
        ticket_digest,
        receipt_digest: inverse_receipt,
        decision: WalTickDecision::Applied,
    };
    let correlation = WalReceiptCorrelationRecord {
        submission_id,
        ticket_digest,
        receipt_digest: inverse_receipt,
        causal_parent_receipts: vec![target_receipt],
    };

    let recovered =
        RecoveredReceiptIndex::from_receipt_correlation_records([receipt], [correlation]);

    assert_eq!(
        recovered.causal_parent_receipts(&inverse_receipt),
        [target_receipt].as_slice()
    );
    assert_eq!(
        recovered.receipts_citing(&target_receipt),
        [inverse_receipt].as_slice()
    );
}

#[test]
fn recovered_receipt_index_replaces_reverse_parent_links_consistently() {
    let receipt_digest = digest("replaced-receipt");
    let old_parent = digest("old-parent");
    let new_parent = digest("new-parent");
    let correlation = |parent| WalReceiptCorrelationRecord {
        submission_id: digest("replaced-submission"),
        ticket_digest: digest("replaced-ticket"),
        receipt_digest,
        causal_parent_receipts: vec![parent],
    };

    let recovered = RecoveredReceiptIndex::from_receipt_correlation_records(
        [],
        [correlation(old_parent), correlation(new_parent)],
    );

    assert!(recovered.receipts_citing(&old_parent).is_empty());
    assert_eq!(
        recovered.causal_parent_receipts(&receipt_digest),
        [new_parent].as_slice()
    );
    assert_eq!(
        recovered.receipts_citing(&new_parent),
        [receipt_digest].as_slice()
    );
}
