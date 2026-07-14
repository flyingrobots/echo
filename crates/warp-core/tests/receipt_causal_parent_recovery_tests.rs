// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Durable receipt-parent recovery witnesses for contract-defined inverse intents.
#![allow(clippy::expect_used)]

use warp_core::causal_wal::{
    RecoveredReceiptIndex, TickReceiptRecord, WalDecodeError, WalReceiptCorrelationRecord,
    WalRecoveryIndexError, WalTickDecision,
};
use warp_core::{
    make_intent_kind, CausalTickReceiptRef, GlobalTick, HeadId, IngressCausalParent,
    IngressEnvelope, IngressTarget, WorldlineId, WorldlineTick, WriterHeadKey,
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

fn receipt_ref(label: &str, tick: u64, content_digest: [u8; 32]) -> CausalTickReceiptRef {
    CausalTickReceiptRef {
        worldline_id: writer_head().worldline_id,
        worldline_tick_after: WorldlineTick::from_raw(tick),
        commit_global_tick: GlobalTick::from_raw(tick),
        commit_hash: digest(&format!("{label}:commit")),
        submission_id: digest(&format!("{label}:submission")),
        ticket_digest: digest(&format!("{label}:ticket")),
        receipt_content_digest: content_digest,
    }
}

fn distinct_receipt_ref(label: &str, tick: u64) -> CausalTickReceiptRef {
    receipt_ref(label, tick, digest(&format!("{label}:content")))
}

#[test]
fn identical_receipt_content_has_distinct_causal_receipt_refs() {
    let shared_content_digest = digest("identical-receipt-content");
    let first = receipt_ref("first", 1, shared_content_digest);
    let second = receipt_ref("second", 2, shared_content_digest);
    let recovered = RecoveredReceiptIndex::from_receipt_correlation_records(
        [
            TickReceiptRecord {
                receipt_ref: first,
                decision: WalTickDecision::Applied,
            },
            TickReceiptRecord {
                receipt_ref: second,
                decision: WalTickDecision::Applied,
            },
        ],
        [],
    )
    .expect("distinct receipt evidence should recover");

    assert_ne!(
        recovered.receipt_by_submission[&first.submission_id],
        recovered.receipt_by_submission[&second.submission_id],
        "receipt content equality must not alias two admitted causal events"
    );
    assert_eq!(recovered.decisions_by_receipt.len(), 2);
}

#[test]
fn causal_parent_lookup_does_not_alias_identical_receipt_content() {
    let shared_content_digest = digest("identical-receipt-content");
    let first = receipt_ref("first", 1, shared_content_digest);
    let second = receipt_ref("second", 2, shared_content_digest);
    let shared_parent = distinct_receipt_ref("shared-parent", 3);
    let recovered = RecoveredReceiptIndex::from_receipt_correlation_records(
        [],
        [
            WalReceiptCorrelationRecord {
                receipt_ref: first,
                causal_parent_receipts: vec![shared_parent],
            },
            WalReceiptCorrelationRecord {
                receipt_ref: second,
                causal_parent_receipts: vec![shared_parent],
            },
        ],
    )
    .expect("distinct causal child evidence should recover");

    assert_eq!(recovered.receipts_citing(&shared_parent), [first, second]);
}

#[test]
fn causal_parent_receipt_changes_ingress_identity_and_is_canonicalized() {
    let target_receipt = distinct_receipt_ref("target-receipt", 4);
    let other_receipt = distinct_receipt_ref("other-receipt", 5);
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
                receipt_ref: target_receipt,
            },
            IngressCausalParent::TickReceipt {
                receipt_ref: other_receipt,
            },
            IngressCausalParent::TickReceipt {
                receipt_ref: target_receipt,
            },
        ],
    );
    let second = IngressEnvelope::local_intent_with_causal_parents(
        target,
        intent_kind,
        intent_bytes,
        vec![IngressCausalParent::TickReceipt {
            receipt_ref: other_receipt,
        }],
    );

    assert_ne!(first.ingress_id(), second.ingress_id());
    let mut expected_parents = vec![
        IngressCausalParent::TickReceipt {
            receipt_ref: target_receipt,
        },
        IngressCausalParent::TickReceipt {
            receipt_ref: other_receipt,
        },
    ];
    expected_parents.sort_unstable();
    assert_eq!(first.causal_parents(), expected_parents.as_slice());
}

#[test]
fn legacy_bare_receipt_digest_is_reported_as_ambiguous() {
    let submission_id = digest("legacy-submission");
    let ticket_digest = digest("legacy-ticket");
    let receipt_digest = digest("legacy-receipt");
    let mut legacy_payload = Vec::new();
    legacy_payload.extend_from_slice(&submission_id);
    legacy_payload.extend_from_slice(&ticket_digest);
    legacy_payload.extend_from_slice(&receipt_digest);

    assert_eq!(
        WalReceiptCorrelationRecord::from_payload_bytes(&legacy_payload),
        Err(WalDecodeError::LegacyCausalReceiptIdentityUnavailable {
            record_kind: "receipt-correlation",
        })
    );
}

#[test]
fn tick_receipt_decoder_rejects_legacy_content_only_identity() {
    let mut legacy_payload = Vec::new();
    legacy_payload.extend_from_slice(&digest("legacy-submission"));
    legacy_payload.extend_from_slice(&digest("legacy-ticket"));
    legacy_payload.extend_from_slice(&digest("legacy-receipt"));
    legacy_payload.push(1);

    assert_eq!(
        TickReceiptRecord::from_payload_bytes(&legacy_payload),
        Err(WalDecodeError::LegacyCausalReceiptIdentityUnavailable {
            record_kind: "tick-receipt",
        })
    );
}

#[test]
fn versioned_receipt_records_reject_corrupt_magic_as_corruption() {
    let receipt_ref = distinct_receipt_ref("corrupt-magic", 6);
    let mut tick_payload = TickReceiptRecord {
        receipt_ref,
        decision: WalTickDecision::Applied,
    }
    .to_payload_bytes();
    tick_payload[0] ^= 0xff;
    assert_eq!(
        TickReceiptRecord::from_payload_bytes(&tick_payload),
        Err(WalDecodeError::InvalidRecordMagic {
            record_kind: "tick-receipt",
        })
    );

    let mut correlation_payload = WalReceiptCorrelationRecord {
        receipt_ref,
        causal_parent_receipts: Vec::new(),
    }
    .to_payload_bytes();
    correlation_payload[0] ^= 0xff;
    assert_eq!(
        WalReceiptCorrelationRecord::from_payload_bytes(&correlation_payload),
        Err(WalDecodeError::InvalidRecordMagic {
            record_kind: "receipt-correlation",
        })
    );
}

#[test]
fn receipt_correlation_decoder_rejects_parent_count_beyond_payload() {
    let mut forged_payload = b"ERCOR002".to_vec();
    forged_payload
        .extend_from_slice(&distinct_receipt_ref("forged-correlation", 6).to_canonical_bytes());
    forged_payload.extend_from_slice(&u64::MAX.to_le_bytes());

    assert_eq!(
        WalReceiptCorrelationRecord::from_payload_bytes(&forged_payload),
        Err(WalDecodeError::UnexpectedEof)
    );
}

#[test]
fn recovered_receipt_index_preserves_causal_parent_receipts() {
    let inverse_receipt = distinct_receipt_ref("inverse-receipt", 7);
    let target_receipt = distinct_receipt_ref("target-receipt", 8);
    let receipt = TickReceiptRecord {
        receipt_ref: inverse_receipt,
        decision: WalTickDecision::Applied,
    };
    let correlation = WalReceiptCorrelationRecord {
        receipt_ref: inverse_receipt,
        causal_parent_receipts: vec![target_receipt],
    };

    let recovered =
        RecoveredReceiptIndex::from_receipt_correlation_records([receipt], [correlation])
            .expect("canonical receipt correlation should recover");

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
fn recovered_receipt_index_rejects_conflicting_parent_evidence() {
    let receipt_ref = distinct_receipt_ref("replaced-receipt", 9);
    let old_parent = distinct_receipt_ref("old-parent", 10);
    let new_parent = distinct_receipt_ref("new-parent", 11);
    let correlation = |parent| WalReceiptCorrelationRecord {
        receipt_ref,
        causal_parent_receipts: vec![parent],
    };

    let error = RecoveredReceiptIndex::from_receipt_correlation_records(
        [],
        [correlation(old_parent), correlation(new_parent)],
    )
    .expect_err("conflicting parent evidence must not replace admitted ancestry");

    assert_eq!(
        error,
        WalRecoveryIndexError::ConflictingReceiptCausalParents {
            receipt_identity_digest: receipt_ref.identity_digest(),
        }
    );
}

#[test]
fn recovered_receipt_index_rejects_conflicting_decisions() {
    let receipt_ref = distinct_receipt_ref("conflicting-decision", 12);
    let record = |decision| TickReceiptRecord {
        receipt_ref,
        decision,
    };

    let error = RecoveredReceiptIndex::from_receipt_correlation_records(
        [
            record(WalTickDecision::Applied),
            record(WalTickDecision::Obstructed),
        ],
        [],
    )
    .expect_err("conflicting decisions must not replace admitted receipt evidence");

    assert_eq!(
        error,
        WalRecoveryIndexError::ConflictingReceiptDecision {
            receipt_identity_digest: receipt_ref.identity_digest(),
        }
    );
}

#[test]
fn recovered_receipt_index_rejects_conflicting_submission_and_ticket_mappings() {
    let first = distinct_receipt_ref("mapping-first", 13);
    let mut same_submission = distinct_receipt_ref("mapping-second", 14);
    same_submission.submission_id = first.submission_id;
    let submission_error = RecoveredReceiptIndex::from_receipt_correlation_records(
        [
            TickReceiptRecord {
                receipt_ref: first,
                decision: WalTickDecision::Applied,
            },
            TickReceiptRecord {
                receipt_ref: same_submission,
                decision: WalTickDecision::Applied,
            },
        ],
        [],
    )
    .expect_err("one submission must not name two receipt coordinates");
    assert_eq!(
        submission_error,
        WalRecoveryIndexError::ConflictingReceiptForSubmission {
            submission_id: first.submission_id,
        }
    );

    let mut same_ticket = distinct_receipt_ref("mapping-third", 15);
    same_ticket.ticket_digest = first.ticket_digest;
    let ticket_error = RecoveredReceiptIndex::from_receipt_correlation_records(
        [
            TickReceiptRecord {
                receipt_ref: first,
                decision: WalTickDecision::Applied,
            },
            TickReceiptRecord {
                receipt_ref: same_ticket,
                decision: WalTickDecision::Applied,
            },
        ],
        [],
    )
    .expect_err("one ticket must not name two receipt coordinates");
    assert_eq!(
        ticket_error,
        WalRecoveryIndexError::ConflictingReceiptForTicket {
            ticket_digest: first.ticket_digest,
        }
    );
}

#[test]
fn recovered_receipt_index_accepts_exact_duplicate_evidence() {
    let receipt_ref = distinct_receipt_ref("exact-duplicate", 16);
    let parent = distinct_receipt_ref("exact-duplicate-parent", 17);
    let receipt = TickReceiptRecord {
        receipt_ref,
        decision: WalTickDecision::Applied,
    };
    let correlation = WalReceiptCorrelationRecord {
        receipt_ref,
        causal_parent_receipts: vec![parent],
    };

    let recovered = RecoveredReceiptIndex::from_receipt_correlation_records(
        [receipt, receipt],
        [correlation.clone(), correlation],
    )
    .expect("exact duplicate receipt evidence should be idempotent");

    assert_eq!(recovered.causal_parent_receipts(&receipt_ref), [parent]);
    assert_eq!(recovered.receipts_citing(&parent), [receipt_ref]);
}
