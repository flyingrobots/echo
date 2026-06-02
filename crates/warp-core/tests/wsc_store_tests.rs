// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WSC store port contract tests.

#![allow(clippy::expect_used)]

use std::collections::BTreeSet;

use warp_core::causal_wal::{
    retained_material_obstructions, EvidenceMaterialPosture, MissingMaterialScope,
    ReadingRefRecord, RecoveredReceiptIndex, RecoveredRetentionIndex, RecoveredSubmissionIndex,
    RecoveredSubmissionPosture, RetainedMaterialKind, RetainedMaterialRecord,
    SubmissionAcceptanceRecord, SubmissionRetryPosture, TickReceiptRecord,
    WalReceiptCorrelationRecord, WalTickDecision,
};
use warp_core::wsc::{
    accepted_submission_records_from_wsc_envelope, accepted_submission_records_from_wsc_store,
    accepted_submission_records_to_wsc_envelope, receipt_correlation_records_from_wsc_envelope,
    receipt_correlation_records_from_wsc_store, receipt_correlation_records_to_wsc_envelope,
    retention_records_from_wsc_envelope, retention_records_from_wsc_store,
    retention_records_to_wsc_envelope, validate_wsc_causal_history_store, write_wsc_one_warp,
    InMemoryWscStore, OneWarpInput, WscStoreEnvelope, WscStoreObstructionKind, WscStorePort,
    WscStoreRecordKind, WscStoreSubject,
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
fn in_memory_wsc_store_ignores_uncommitted_staged_write() {
    let envelope =
        WscStoreEnvelope::validated(WscStoreRecordKind::Snapshot, [6; 32], fixture_wsc_bytes(19))
            .expect("valid WSC envelope");
    let id = envelope.id();
    let mut store = InMemoryWscStore::default();

    let staged_id = store
        .stage_envelope_without_commit_marker(envelope)
        .expect("staged WSC envelope");

    assert_eq!(staged_id, id);
    assert!(store.list_envelopes().is_empty());
    let obstruction = store
        .read_envelope(id)
        .expect_err("uncommitted staged write obstructs");
    assert_eq!(
        obstruction.kind,
        WscStoreObstructionKind::IncompleteEnvelopeWrite
    );
}

#[test]
fn in_memory_wsc_store_commits_staged_write_idempotently() {
    let envelope =
        WscStoreEnvelope::validated(WscStoreRecordKind::Snapshot, [7; 32], fixture_wsc_bytes(23))
            .expect("valid WSC envelope");
    let id = envelope.id();
    let mut store = InMemoryWscStore::default();

    store
        .stage_envelope_without_commit_marker(envelope.clone())
        .expect("staged WSC envelope");
    let committed = store
        .commit_staged_envelope(id)
        .expect("committed staged WSC envelope");
    let committed_again = store
        .write_envelope(envelope.clone())
        .expect("idempotent write");

    assert_eq!(committed, committed_again);
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
fn pending_submission_recovers_from_committed_wsc_store_without_decision() {
    let pending = submission_acceptance(3, 33);
    let envelope =
        accepted_submission_records_to_wsc_envelope(&[pending]).expect("accepted WSC envelope");
    let mut store = InMemoryWscStore::default();
    store
        .write_envelope(envelope)
        .expect("committed accepted WSC envelope");

    let recovered_records =
        accepted_submission_records_from_wsc_store(&store).expect("recovered accepted records");
    let recovered =
        RecoveredSubmissionIndex::from_acceptance_records(recovered_records).expect("index");

    let entry = recovered
        .get(&pending.submission_id)
        .expect("recovered pending submission");
    assert_eq!(entry.posture, RecoveredSubmissionPosture::AcceptedPending);
    assert_eq!(entry.receipt_digest, None);
    assert_eq!(
        recovered.retry_posture(pending.submission_id, pending.canonical_envelope_digest),
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

#[test]
fn receipt_correlation_records_reject_conflicting_duplicate_receipt() {
    let receipt = tick_receipt(7, 17, 27, WalTickDecision::Applied);
    let conflicting_receipt = tick_receipt(8, 18, 27, WalTickDecision::Obstructed);

    let obstruction = receipt_correlation_records_to_wsc_envelope(
        &[receipt, conflicting_receipt],
        &[receipt_correlation(7, 17, 27)],
    )
    .expect_err("conflicting duplicate receipt obstructs");

    assert_eq!(
        obstruction.kind,
        WscStoreObstructionKind::DuplicateEnvelopeMismatch
    );
}

#[test]
fn decided_submissions_recover_from_committed_wsc_store() {
    let applied = submission_acceptance(8, 38);
    let rejected = submission_acceptance(9, 39);
    let applied_receipt = tick_receipt(8, 18, 28, WalTickDecision::Applied);
    let rejected_receipt = tick_receipt(9, 19, 29, WalTickDecision::RejectedFootprintConflict);
    let mut store = InMemoryWscStore::default();
    store
        .write_envelope(
            accepted_submission_records_to_wsc_envelope(&[rejected, applied])
                .expect("accepted WSC envelope"),
        )
        .expect("committed accepted WSC envelope");
    store
        .write_envelope(
            receipt_correlation_records_to_wsc_envelope(
                &[rejected_receipt, applied_receipt],
                &[
                    receipt_correlation(8, 18, 28),
                    receipt_correlation(9, 19, 29),
                ],
            )
            .expect("receipt WSC envelope"),
        )
        .expect("committed receipt WSC envelope");

    let accepted =
        accepted_submission_records_from_wsc_store(&store).expect("recovered accepted records");
    let receipt_records =
        receipt_correlation_records_from_wsc_store(&store).expect("recovered receipt records");
    let submissions = RecoveredSubmissionIndex::from_acceptance_and_receipt_records(
        accepted,
        receipt_records.receipts.clone(),
    )
    .expect("decided submission index");
    let receipts = RecoveredReceiptIndex::from_receipt_correlation_records(
        receipt_records.receipts,
        receipt_records.correlations,
    );

    let applied_entry = submissions
        .get(&applied.submission_id)
        .expect("recovered applied submission");
    assert_eq!(
        applied_entry.posture,
        RecoveredSubmissionPosture::DecidedApplied
    );
    assert_eq!(
        applied_entry.receipt_digest,
        Some(applied_receipt.receipt_digest)
    );
    assert_eq!(
        submissions.retry_posture(applied.submission_id, applied.canonical_envelope_digest),
        SubmissionRetryPosture::AlreadyDecidedApplied
    );

    let rejected_entry = submissions
        .get(&rejected.submission_id)
        .expect("recovered rejected submission");
    assert_eq!(
        rejected_entry.posture,
        RecoveredSubmissionPosture::DecidedRejected
    );
    assert_eq!(
        rejected_entry.receipt_digest,
        Some(rejected_receipt.receipt_digest)
    );
    assert_eq!(
        submissions.retry_posture(rejected.submission_id, rejected.canonical_envelope_digest),
        SubmissionRetryPosture::AlreadyDecidedRejected
    );
    assert_eq!(
        receipts.receipt_by_submission.get(&applied.submission_id),
        Some(&applied_receipt.receipt_digest)
    );
    assert_eq!(
        receipts
            .decisions_by_receipt
            .get(&rejected_receipt.receipt_digest),
        Some(&WalTickDecision::RejectedFootprintConflict)
    );
}

#[test]
fn wsc_causal_history_rejects_receipt_without_committed_acceptance() {
    let acceptance = submission_acceptance(10, 40);
    let receipt = tick_receipt(10, 20, 30, WalTickDecision::Applied);
    let mut store = InMemoryWscStore::default();
    store
        .stage_envelope_without_commit_marker(
            accepted_submission_records_to_wsc_envelope(&[acceptance])
                .expect("accepted WSC envelope"),
        )
        .expect("staged accepted WSC envelope");
    store
        .write_envelope(
            receipt_correlation_records_to_wsc_envelope(
                &[receipt],
                &[receipt_correlation(10, 20, 30)],
            )
            .expect("receipt WSC envelope"),
        )
        .expect("committed receipt WSC envelope");

    let obstruction =
        validate_wsc_causal_history_store(&store).expect_err("half-accepted history obstructs");

    assert_eq!(
        obstruction.kind,
        WscStoreObstructionKind::IncompleteCausalHistory
    );
    assert_eq!(
        obstruction.subject,
        WscStoreSubject::CausalHistory {
            subject_digest: receipt.receipt_digest
        }
    );
}

#[test]
fn wsc_causal_history_rejects_receipt_without_correlation() {
    let acceptance = submission_acceptance(11, 41);
    let receipt = tick_receipt(11, 21, 31, WalTickDecision::Applied);
    let mut store = InMemoryWscStore::default();
    store
        .write_envelope(
            accepted_submission_records_to_wsc_envelope(&[acceptance])
                .expect("accepted WSC envelope"),
        )
        .expect("committed accepted WSC envelope");
    store
        .write_envelope(
            receipt_correlation_records_to_wsc_envelope(&[receipt], &[])
                .expect("receipt WSC envelope"),
        )
        .expect("committed receipt WSC envelope");

    let obstruction =
        validate_wsc_causal_history_store(&store).expect_err("missing correlation obstructs");

    assert_eq!(
        obstruction.kind,
        WscStoreObstructionKind::IncompleteCausalHistory
    );
    assert_eq!(
        obstruction.subject,
        WscStoreSubject::CausalHistory {
            subject_digest: receipt.receipt_digest
        }
    );
}

#[test]
fn retention_records_round_trip_through_wsc_envelope() {
    let material = retained_material(
        31,
        41,
        RetainedMaterialKind::ReadingEnvelope,
        EvidenceMaterialPosture::Present,
    );
    let missing_material = retained_material(
        32,
        42,
        RetainedMaterialKind::ReadingPayload,
        EvidenceMaterialPosture::Missing,
    );
    let reading = reading_ref(51, 41, 61, 71, EvidenceMaterialPosture::Present);
    let envelope = retention_records_to_wsc_envelope(
        &[missing_material, material, material],
        &[reading, reading],
    )
    .expect("retention WSC envelope");

    let recovered = retention_records_from_wsc_envelope(&envelope).expect("recovered retention");
    assert_eq!(recovered.materials, vec![material, missing_material]);
    assert_eq!(recovered.readings, vec![reading]);

    let index =
        RecoveredRetentionIndex::from_retention_records(recovered.materials, recovered.readings);
    assert_eq!(index.material_by_digest.get(&[31; 32]), Some(&material));
    assert_eq!(
        index.material_by_digest.get(&[32; 32]),
        Some(&missing_material)
    );
    assert_eq!(index.reading_by_id.get(&[51; 32]), Some(&reading));
    assert!(index
        .material_by_semantic_coordinate
        .get(&[41; 32])
        .expect("material semantic coordinate")
        .contains(&[31; 32]));
    assert!(index
        .readings_by_semantic_coordinate
        .get(&[41; 32])
        .expect("reading semantic coordinate")
        .contains(&[51; 32]));

    let available_material = BTreeSet::from([[31; 32]]);
    let obstructions = retained_material_obstructions(&index, &available_material);
    assert_eq!(obstructions.len(), 1);
    let obstruction = obstructions[0];
    assert_eq!(obstruction.material_digest, [32; 32]);
    assert_eq!(obstruction.kind, RetainedMaterialKind::ReadingPayload);
    assert_eq!(obstruction.scope, MissingMaterialScope::Reading);
    assert_eq!(obstruction.posture, EvidenceMaterialPosture::Missing);
}

#[test]
fn retention_records_recover_from_committed_wsc_store() {
    let material = retained_material(
        33,
        43,
        RetainedMaterialKind::ReadingEnvelope,
        EvidenceMaterialPosture::Present,
    );
    let reading = reading_ref(53, 43, 63, 73, EvidenceMaterialPosture::Present);
    let mut store = InMemoryWscStore::default();
    store
        .write_envelope(
            retention_records_to_wsc_envelope(&[material], &[reading])
                .expect("retention WSC envelope"),
        )
        .expect("committed retention WSC envelope");

    let recovered = retention_records_from_wsc_store(&store).expect("recovered retention");

    assert_eq!(recovered.materials, vec![material]);
    assert_eq!(recovered.readings, vec![reading]);
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

fn retained_material(
    material_byte: u8,
    semantic_byte: u8,
    kind: RetainedMaterialKind,
    posture: EvidenceMaterialPosture,
) -> RetainedMaterialRecord {
    RetainedMaterialRecord {
        material_digest: [material_byte; 32],
        semantic_coordinate_digest: [semantic_byte; 32],
        kind,
        posture,
    }
}

fn reading_ref(
    reading_byte: u8,
    semantic_byte: u8,
    payload_byte: u8,
    envelope_byte: u8,
    posture: EvidenceMaterialPosture,
) -> ReadingRefRecord {
    ReadingRefRecord {
        reading_id: [reading_byte; 32],
        semantic_coordinate_digest: [semantic_byte; 32],
        payload_digest: [payload_byte; 32],
        envelope_digest: [envelope_byte; 32],
        posture,
    }
}
