// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WSC store port contract tests.

#![allow(clippy::expect_used)]

use warp_core::causal_wal::{
    RecoveredReceiptIndex, RecoveredSubmissionIndex, SubmissionAcceptanceRecord,
    SubmissionRetryPosture, TickReceiptRecord, WalReceiptCorrelationRecord, WalTickDecision,
};
use warp_core::wsc::{
    accepted_submission_records_from_wsc_envelope, accepted_submission_records_to_wsc_envelope,
    receipt_correlation_records_from_wsc_envelope, receipt_correlation_records_to_wsc_envelope,
    write_wsc_one_warp, InMemoryWscStore, OneWarpInput, WscStoreEnvelope, WscStoreObstructionKind,
    WscStorePort, WscStoreRecordKind, WscStoreSubject,
};

#[test]
fn wsc_store_envelope_round_trips_deterministically() {
    let bytes = fixture_wsc_bytes(7);
    let basis_digest = [9; 32];
    let envelope = WscStoreEnvelope::validated(
        WscStoreRecordKind::CausalHistory,
        basis_digest,
        bytes.clone(),
    )
    .expect("valid WSC envelope");

    let encoded_a = envelope.encode();
    let encoded_b = envelope.encode();
    assert_eq!(encoded_a, encoded_b);

    let decoded = WscStoreEnvelope::decode(&encoded_a).expect("decoded envelope");
    assert_eq!(decoded, envelope);
    assert_eq!(decoded.wsc_bytes(), bytes.as_slice());
    assert_eq!(decoded.basis_digest(), &basis_digest);
}

#[test]
fn in_memory_wsc_store_writes_reads_and_lists_envelopes() {
    let envelope =
        WscStoreEnvelope::validated(WscStoreRecordKind::Snapshot, [3; 32], fixture_wsc_bytes(11))
            .expect("valid WSC envelope");
    let id = envelope.id();
    let mut store = InMemoryWscStore::default();

    let receipt = store
        .write_envelope(envelope.clone())
        .expect("write envelope");
    assert_eq!(receipt.envelope_id, id);
    assert_eq!(store.list_envelopes(), vec![id]);
    assert_eq!(store.read_envelope(id), Ok(envelope));
}

#[test]
fn in_memory_wsc_store_missing_envelope_returns_typed_obstruction() {
    let store = InMemoryWscStore::default();
    let missing_id = WscStoreEnvelope::validated(
        WscStoreRecordKind::RetainedEvidence,
        [4; 32],
        fixture_wsc_bytes(13),
    )
    .expect("valid WSC envelope")
    .id();

    let obstruction = store
        .read_envelope(missing_id)
        .expect_err("missing envelope obstructs");
    assert_eq!(obstruction.kind, WscStoreObstructionKind::MissingEnvelope);
    assert_eq!(
        obstruction.subject,
        WscStoreSubject::Envelope {
            envelope_id: missing_id
        }
    );
}

#[test]
fn wsc_store_decode_rejects_digest_mismatch() {
    let envelope =
        WscStoreEnvelope::validated(WscStoreRecordKind::Snapshot, [5; 32], fixture_wsc_bytes(17))
            .expect("valid WSC envelope");
    let mut encoded = envelope.encode();
    let last = encoded.last_mut().expect("encoded envelope byte");
    *last ^= 0xff;

    let obstruction = WscStoreEnvelope::decode(&encoded).expect_err("digest mismatch obstructs");
    assert_eq!(obstruction.kind, WscStoreObstructionKind::DigestMismatch);
}

#[test]
fn wsc_store_module_has_no_jedit_nouns() {
    let source = include_str!("../src/wsc/store.rs");
    assert!(!source.to_lowercase().contains("jedit"));
}

#[test]
fn accepted_submission_records_round_trip_through_wsc_envelope() {
    let duplicate = submission_acceptance(1, 11);
    let envelope = accepted_submission_records_to_wsc_envelope(&[
        submission_acceptance(2, 22),
        duplicate,
        duplicate,
    ])
    .expect("accepted submission WSC envelope");

    let recovered =
        accepted_submission_records_from_wsc_envelope(&envelope).expect("recovered records");
    assert_eq!(recovered, vec![duplicate, submission_acceptance(2, 22)]);

    let index =
        RecoveredSubmissionIndex::from_acceptance_records(recovered).expect("recovered index");
    assert_eq!(
        index.retry_posture([1; 32], [11; 32]),
        SubmissionRetryPosture::AlreadyAcceptedPending
    );
}

#[test]
fn receipt_correlation_records_round_trip_through_wsc_envelope() {
    let receipt = tick_receipt(7, 17, 27, WalTickDecision::Applied);
    let correlation = receipt_correlation(7, 17, 27);
    let envelope = receipt_correlation_records_to_wsc_envelope(&[receipt], &[correlation])
        .expect("receipt correlation WSC envelope");

    let recovered = receipt_correlation_records_from_wsc_envelope(&envelope)
        .expect("recovered receipt correlations");
    assert_eq!(recovered.receipts, vec![receipt]);
    assert_eq!(recovered.correlations, vec![correlation]);

    let index = RecoveredReceiptIndex::from_receipt_correlation_records(
        recovered.receipts,
        recovered.correlations,
    );
    assert_eq!(index.receipt_by_submission.get(&[7; 32]), Some(&[27; 32]));
    assert_eq!(index.receipt_by_ticket.get(&[17; 32]), Some(&[27; 32]));
    assert_eq!(
        index.decisions_by_receipt.get(&[27; 32]),
        Some(&WalTickDecision::Applied)
    );
}

fn fixture_wsc_bytes(tick: u64) -> Vec<u8> {
    let input = OneWarpInput {
        warp_id: [1; 32],
        root_node_id: [2; 32],
        nodes: vec![warp_core::wsc::types::NodeRow {
            node_id: [2; 32],
            node_type: [3; 32],
        }],
        edges: vec![],
        out_index: vec![warp_core::wsc::types::Range::default()],
        out_edges: vec![],
        node_atts_index: vec![warp_core::wsc::types::Range::default()],
        node_atts: vec![],
        edge_atts_index: vec![],
        edge_atts: vec![],
        blobs: vec![],
    };
    write_wsc_one_warp(&input, [8; 32], tick).expect("fixture WSC bytes")
}

fn submission_acceptance(submission_byte: u8, envelope_byte: u8) -> SubmissionAcceptanceRecord {
    SubmissionAcceptanceRecord {
        submission_id: [submission_byte; 32],
        canonical_envelope_digest: [envelope_byte; 32],
        idempotency_key_digest: None,
        acceptance_evidence_digest: [submission_byte ^ envelope_byte; 32],
    }
}

fn tick_receipt(
    submission_byte: u8,
    ticket_byte: u8,
    receipt_byte: u8,
    decision: WalTickDecision,
) -> TickReceiptRecord {
    TickReceiptRecord {
        submission_id: [submission_byte; 32],
        ticket_digest: [ticket_byte; 32],
        receipt_digest: [receipt_byte; 32],
        decision,
    }
}

fn receipt_correlation(
    submission_byte: u8,
    ticket_byte: u8,
    receipt_byte: u8,
) -> WalReceiptCorrelationRecord {
    WalReceiptCorrelationRecord {
        submission_id: [submission_byte; 32],
        ticket_digest: [ticket_byte; 32],
        receipt_digest: [receipt_byte; 32],
    }
}
