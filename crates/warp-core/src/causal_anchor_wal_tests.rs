// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WAL admission and recovery witnesses for causal anchors.

#![allow(clippy::panic)]

use std::fmt::Debug;

use crate::causal_wal::{
    build_causal_anchor_admission_transaction, recover_causal_anchor_admissions,
    recover_from_frames_and_commits, AffectedFrontier, AffectedFrontierKind, Lsn, PayloadCodecId,
    PayloadSchemaId, RecoveryAccessMode, WalAppendAuthority, WalBuildError,
    WalCommittedTransaction, WalDurabilityMode, WalRecordKind, WalRecoveryIndexError, WalSegmentId,
    WalTransactionBuilder, WalTransactionId, WalTransactionKind, WriterEpochId,
};
use crate::wsc::{causal_anchor_records_from_wsc_envelope, causal_anchor_records_to_wsc_envelope};
use crate::{
    CausalAnchorAdmissionRequest, CausalAnchorAppRootRole, CausalAnchorCasRole, CausalAnchorClaim,
    CausalAnchorError, CausalAnchorGraphRole, CausalAnchorPurpose, CausalAnchorRoot,
    CausalAnchorSubject, CausalFrontierRef, Hash, CAUSAL_ANCHOR_SCHEMA_VERSION,
};

fn digest(label: &str) -> Hash {
    *blake3::hash(label.as_bytes()).as_bytes()
}

fn must_ok<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected Ok(..), got Err({error:?})"),
    }
}

fn claim(label: &str) -> CausalAnchorClaim {
    must_ok(CausalAnchorClaim::from_admission_request(
        CausalAnchorAdmissionRequest {
            schema_version: CAUSAL_ANCHOR_SCHEMA_VERSION,
            subject: CausalAnchorSubject::new(
                "jedit",
                "BufferWorldline",
                format!("worldline:{label}"),
            ),
            basis_frontier: CausalFrontierRef::from_digest(digest(&format!("basis:{label}"))),
            retained_roots: vec![CausalAnchorRoot::AppSubjectRoot {
                app_id: "jedit".to_owned(),
                subject_kind: "RopeHead".to_owned(),
                id: format!("head:{label}"),
                role: CausalAnchorAppRootRole::Authority,
            }],
            materialization_roots: vec![CausalAnchorRoot::CasObject {
                id: digest(&format!("flat-text:{label}")),
                role: CausalAnchorCasRole::Materialization,
            }],
            purpose: CausalAnchorPurpose::UserSave,
        },
    ))
}

fn claim_with_two_graph_roots(label: &str) -> CausalAnchorClaim {
    must_ok(CausalAnchorClaim::from_admission_request(
        CausalAnchorAdmissionRequest {
            schema_version: CAUSAL_ANCHOR_SCHEMA_VERSION,
            subject: CausalAnchorSubject::new(
                "jedit",
                "BufferWorldline",
                format!("worldline:{label}"),
            ),
            basis_frontier: CausalFrontierRef::from_digest(digest(&format!("basis:{label}"))),
            retained_roots: vec![
                CausalAnchorRoot::GraphFact {
                    id: [1; 32],
                    role: CausalAnchorGraphRole::Authority,
                },
                CausalAnchorRoot::GraphFact {
                    id: [2; 32],
                    role: CausalAnchorGraphRole::Evidence,
                },
            ],
            materialization_roots: Vec::new(),
            purpose: CausalAnchorPurpose::Recovery,
        },
    ))
}

fn builder(label: &str, first_lsn: u64) -> WalTransactionBuilder {
    WalTransactionBuilder::new(
        WriterEpochId::from_hash(digest("anchor-writer-epoch")),
        WalSegmentId::from_raw(1),
        WalTransactionId::from_hash(digest(&format!("anchor-transaction:{label}"))),
        WalTransactionKind::CausalAnchorAdmission,
        WalAppendAuthority::AdmissionKernel,
        Lsn::from_raw(first_lsn),
        digest("previous-frame"),
        digest("previous-commit"),
        WalDurabilityMode::StrictFilesystem,
        PayloadCodecId::from_hash(digest("anchor-codec")),
        PayloadSchemaId::from_hash(digest("anchor-schema")),
        1,
        1,
        digest("anchor-domain"),
    )
}

fn frontier(label: &str) -> AffectedFrontier {
    AffectedFrontier {
        kind: AffectedFrontierKind::CausalAnchorIndex,
        before_digest: digest(&format!("anchor-index:{label}:before")),
        after_digest: digest(&format!("anchor-index:{label}:after")),
    }
}

fn transaction(label: &str) -> Result<WalCommittedTransaction, WalBuildError> {
    build_causal_anchor_admission_transaction(
        builder(label, 10),
        claim(label),
        digest("anchor-support-policy"),
        vec![frontier(label)],
    )
}

fn report_for(
    transaction: &WalCommittedTransaction,
) -> Result<crate::causal_wal::RecoveryScanReport, crate::causal_wal::WalRecoveryError> {
    recover_from_frames_and_commits(
        &transaction.frames,
        std::slice::from_ref(&transaction.commit),
        RecoveryAccessMode::ReadOnly,
    )
}

fn malformed_transaction(
    label: &str,
    fact_payloads: &[Vec<u8>],
    receipt_payloads: &[Vec<u8>],
) -> WalCommittedTransaction {
    let mut builder = builder(label, 20);
    for payload in fact_payloads {
        must_ok(builder.push_record(WalRecordKind::CausalAnchorFactRecorded, payload.clone()));
    }
    for payload in receipt_payloads {
        must_ok(builder.push_record(
            WalRecordKind::CausalAnchorAdmissionReceiptRecorded,
            payload.clone(),
        ));
    }
    must_ok(builder.commit(vec![frontier(label)]))
}

#[test]
fn causal_anchor_wal_codes_are_stable() {
    assert_eq!(WalTransactionKind::CausalAnchorAdmission.stable_code(), 7);
    assert_eq!(WalRecordKind::CausalAnchorFactRecorded.stable_code(), 23);
    assert_eq!(
        WalRecordKind::CausalAnchorAdmissionReceiptRecorded.stable_code(),
        24
    );
    assert_eq!(AffectedFrontierKind::CausalAnchorIndex.stable_code(), 8);
}

#[test]
fn committed_anchor_transaction_recovers_one_cross_checked_admission() {
    let expected_claim = claim("round-trip");
    let transaction = must_ok(transaction("round-trip"));
    assert_eq!(
        transaction
            .frames
            .iter()
            .map(|frame| frame.header.record_kind)
            .collect::<Vec<_>>(),
        vec![
            WalRecordKind::CausalAnchorFactRecorded,
            WalRecordKind::CausalAnchorAdmissionReceiptRecorded,
        ]
    );

    let report = must_ok(report_for(&transaction));
    let admissions = must_ok(recover_causal_anchor_admissions(&report));
    assert_eq!(admissions.len(), 1);
    let admission = &admissions[0];
    assert_eq!(admission.fact().claim(), &expected_claim);
    assert_eq!(
        admission.fact().admitted_by_receipt_id(),
        admission.receipt().receipt_id()
    );
    assert_eq!(
        admission.fact().anchor_id(),
        admission.receipt().anchor_id()
    );
    assert_eq!(
        admission.transaction_id(),
        transaction.commit.transaction_id
    );
    assert_eq!(admission.committed_lsn(), transaction.commit.last_lsn);
    assert_eq!(admission.commit_digest(), &transaction.commit.commit_digest);
}

#[test]
fn causal_anchor_history_round_trips_through_wsc() {
    let transaction = must_ok(transaction("wsc-round-trip"));
    let report = must_ok(report_for(&transaction));
    let admissions = must_ok(recover_causal_anchor_admissions(&report));

    let envelope = must_ok(causal_anchor_records_to_wsc_envelope(&admissions));
    let recovered = must_ok(causal_anchor_records_from_wsc_envelope(&envelope));

    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].fact(), admissions[0].fact());
    assert_eq!(recovered[0].receipt(), admissions[0].receipt());
    assert_eq!(
        recovered[0].transaction_id(),
        admissions[0].transaction_id()
    );
    assert_eq!(recovered[0].committed_lsn(), admissions[0].committed_lsn());
    assert_eq!(recovered[0].commit_digest(), admissions[0].commit_digest());
}

#[test]
fn admission_receipt_identity_binds_host_support_policy() {
    let first = must_ok(build_causal_anchor_admission_transaction(
        builder("policy-binding", 10),
        claim("policy-binding"),
        digest("support-policy:first"),
        vec![frontier("policy-binding")],
    ));
    let second = must_ok(build_causal_anchor_admission_transaction(
        builder("policy-binding", 10),
        claim("policy-binding"),
        digest("support-policy:second"),
        vec![frontier("policy-binding")],
    ));
    let first_admission = must_ok(recover_causal_anchor_admissions(&must_ok(report_for(
        &first,
    ))))
    .remove(0);
    let second_admission = must_ok(recover_causal_anchor_admissions(&must_ok(report_for(
        &second,
    ))))
    .remove(0);

    assert_ne!(
        first_admission.receipt().receipt_id(),
        second_admission.receipt().receipt_id()
    );
    assert_ne!(first_admission.fact(), second_admission.fact());
}

#[test]
fn uncommitted_anchor_frames_never_recover_as_admitted() {
    let transaction = must_ok(transaction("uncommitted"));
    let report = must_ok(recover_from_frames_and_commits(
        &transaction.frames,
        &[],
        RecoveryAccessMode::ReadOnly,
    ));

    assert!(must_ok(recover_causal_anchor_admissions(&report)).is_empty());
}

#[test]
fn anchor_recovery_rejects_missing_or_duplicate_required_frames() {
    let valid = must_ok(transaction("frame-cardinality"));
    let fact = valid.frames[0].payload.canonical_bytes.clone();
    let receipt = valid.frames[1].payload.canonical_bytes.clone();

    let missing = malformed_transaction("missing-receipt", std::slice::from_ref(&fact), &[]);
    let missing_report = must_ok(report_for(&missing));
    assert!(matches!(
        recover_causal_anchor_admissions(&missing_report),
        Err(WalRecoveryIndexError::MissingCausalAnchorAdmissionReceiptFrame { .. })
    ));

    let duplicate = malformed_transaction(
        "duplicate-fact",
        &[fact.clone(), fact],
        std::slice::from_ref(&receipt),
    );
    let duplicate_report = must_ok(report_for(&duplicate));
    assert!(matches!(
        recover_causal_anchor_admissions(&duplicate_report),
        Err(WalRecoveryIndexError::DuplicateCausalAnchorFactFrame { .. })
    ));
}

#[test]
fn anchor_recovery_rejects_noncanonical_required_frame_order() {
    let valid = must_ok(transaction("frame-order"));
    let fact = valid.frames[0].payload.canonical_bytes.clone();
    let receipt = valid.frames[1].payload.canonical_bytes.clone();
    let mut builder = builder("frame-order", 10);
    must_ok(builder.push_record(WalRecordKind::CausalAnchorAdmissionReceiptRecorded, receipt));
    must_ok(builder.push_record(WalRecordKind::CausalAnchorFactRecorded, fact));
    let reversed = must_ok(builder.commit(vec![frontier("frame-order")]));
    let report = must_ok(report_for(&reversed));

    assert!(matches!(
        recover_causal_anchor_admissions(&report),
        Err(WalRecoveryIndexError::NonCanonicalCausalAnchorAdmissionFrameOrder { .. })
    ));
}

#[test]
fn anchor_payload_codec_rejects_truncation_trailing_bytes_and_unknown_enums() {
    let valid = must_ok(transaction("payload-corruption"));
    let fact = valid.frames[0].payload.canonical_bytes.clone();
    let receipt = valid.frames[1].payload.canonical_bytes.clone();

    let mut truncated = fact.clone();
    truncated.pop();
    let mut trailing = fact.clone();
    trailing.push(0);

    let mut unknown_purpose = fact;
    let mut claim_len_bytes = [0; 8];
    claim_len_bytes.copy_from_slice(&unknown_purpose[8..16]);
    let claim_len = must_ok(usize::try_from(u64::from_le_bytes(claim_len_bytes)));
    let purpose_offset = 16 + claim_len - 33;
    unknown_purpose[purpose_offset] = u8::MAX;

    for (label, payload) in [
        ("truncated", truncated),
        ("trailing", trailing),
        ("unknown-purpose", unknown_purpose),
    ] {
        let malformed = malformed_transaction(
            label,
            std::slice::from_ref(&payload),
            std::slice::from_ref(&receipt),
        );
        let report = must_ok(report_for(&malformed));
        assert!(matches!(
            recover_causal_anchor_admissions(&report),
            Err(WalRecoveryIndexError::CausalAnchorPayload(_))
        ));
    }
}

#[test]
fn anchor_payload_codec_rejects_noncanonical_root_order() {
    let valid = must_ok(build_causal_anchor_admission_transaction(
        builder("noncanonical-order", 10),
        claim_with_two_graph_roots("noncanonical-order"),
        digest("anchor-support-policy"),
        vec![frontier("noncanonical-order")],
    ));
    let mut fact = valid.frames[0].payload.canonical_bytes.clone();
    let receipt = valid.frames[1].payload.canonical_bytes.clone();

    let claim_start = 16;
    let mut offset = claim_start + 8 + 4;
    for _ in 0..3 {
        let mut len_bytes = [0; 8];
        len_bytes.copy_from_slice(&fact[offset..offset + 8]);
        let len = must_ok(usize::try_from(u64::from_le_bytes(len_bytes)));
        offset += 8 + len;
    }
    offset += 32;
    let mut count_bytes = [0; 8];
    count_bytes.copy_from_slice(&fact[offset..offset + 8]);
    assert_eq!(u64::from_le_bytes(count_bytes), 2);
    offset += 8;

    let first_root = fact[offset..offset + 34].to_vec();
    let second_root = fact[offset + 34..offset + 68].to_vec();
    fact[offset..offset + 34].copy_from_slice(&second_root);
    fact[offset + 34..offset + 68].copy_from_slice(&first_root);

    let malformed = malformed_transaction(
        "noncanonical-order",
        std::slice::from_ref(&fact),
        std::slice::from_ref(&receipt),
    );
    let report = must_ok(report_for(&malformed));
    assert_eq!(
        recover_causal_anchor_admissions(&report),
        Err(WalRecoveryIndexError::CausalAnchorPayload(
            CausalAnchorError::NonCanonicalPayload
        ))
    );
}

#[test]
fn anchor_recovery_rejects_fact_and_receipt_from_different_admissions() {
    let first = must_ok(transaction("first"));
    let second = must_ok(transaction("second"));
    let mismatched = malformed_transaction(
        "mismatched",
        std::slice::from_ref(&first.frames[0].payload.canonical_bytes),
        std::slice::from_ref(&second.frames[1].payload.canonical_bytes),
    );
    let report = must_ok(report_for(&mismatched));

    assert!(matches!(
        recover_causal_anchor_admissions(&report),
        Err(WalRecoveryIndexError::CausalAnchorPayload(
            CausalAnchorError::AdmissionEvidenceMismatch
        ))
    ));
}
