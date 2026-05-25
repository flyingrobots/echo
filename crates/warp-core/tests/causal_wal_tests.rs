// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Causal WAL foundation tests.

use warp_core::causal_wal::{
    apply_committed_transaction, audit_wal_release_readiness,
    build_checkpoint_publication_transaction, build_materialization_outbox_transaction,
    build_recovery_certificate, build_retained_reading_transaction,
    build_submission_acceptance_transaction, build_tick_transaction, doctor_in_memory_store,
    evaluate_checkpoint_publication, lint_wal_schema_terms, missing_material_scope,
    project_causal_commit_evidence, read_checkpoint_record, recover_checkpoint_publications,
    recover_filesystem_store, recover_in_memory_store, recover_materialization_outbox,
    recover_receipt_index, recover_retention_index, recover_submission_index,
    retained_material_obstructions, shadow_replay_matches, validate_checkpoint_record,
    validate_strict_object_store_capabilities, write_checkpoint_record_atomic, AffectedFrontier,
    AffectedFrontierKind, CheckpointPublicationRecord, CheckpointRecord,
    CheckpointValidationPosture, EvidenceMaterialPosture, ExistingMaterializedArtifact,
    FilesystemWalStore, InMemoryWalStore, Lsn, MaterializationIntentRecord,
    MaterializationObservationRecord, MaterializationReplayPosture, MissingMaterialScope,
    ObjectStoreCapabilityError, ObjectStoreReadAfterWritePosture, ObjectStoreWalCapabilities,
    PayloadCodecId, PayloadSchemaId, ReadingRefRecord, RecoveredState, RecoveredSubmissionPosture,
    RecoveryAccessMode, RecoveryTailPosture, RetainedMaterialKind, RetainedMaterialRecord,
    SubmissionAcceptanceRecord, SubmissionRetryPosture, TickReceiptRecord, TransactionLocalIndex,
    WalAppendAuthority, WalBuildError, WalCommittedTransaction, WalDoctorPosture,
    WalDurabilityMode, WalManifest, WalReceiptCorrelationRecord, WalRecordKind,
    WalReleaseReadinessGates, WalSchemaLintError, WalSegmentId, WalStoreError, WalStorePort,
    WalTickDecision, WalTransactionBuilder, WalTransactionId, WalTransactionKind, WriterEpochId,
    WriterEpochRequest,
};
use warp_core::Hash;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn digest(label: &str) -> Hash {
    blake3::hash(label.as_bytes()).into()
}

fn must_ok<T, E>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(_) => std::process::abort(),
    }
}

fn must_some<T>(option: Option<T>) -> T {
    match option {
        Some(value) => value,
        None => std::process::abort(),
    }
}

fn temp_checkpoint_path(label: &str) -> std::path::PathBuf {
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "echo-causal-wal-{label}-{}-{unique}",
        std::process::id()
    ));
    must_ok(fs::create_dir_all(&dir));
    dir.join("checkpoint.ecwal")
}

fn temp_wal_dir(label: &str) -> std::path::PathBuf {
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "echo-causal-wal-store-{label}-{}-{unique}",
        std::process::id()
    ));
    must_ok(fs::create_dir_all(&dir));
    dir
}

fn epoch_id() -> WriterEpochId {
    WriterEpochId::from_hash(digest("epoch:1"))
}

fn transaction_id(label: &str) -> WalTransactionId {
    WalTransactionId::from_hash(digest(label))
}

fn writer_epoch_request() -> WriterEpochRequest {
    WriterEpochRequest {
        epoch_id: epoch_id(),
        storage_fencing_token: digest("fencing"),
        process_identity: digest("process"),
        host_identity: digest("host"),
        started_at_lsn: Lsn::from_raw(0),
        previous_epoch_id: None,
        previous_epoch_final_commit_digest: None,
        lease_or_lock_evidence: digest("lease"),
    }
}

fn builder(
    transaction_id: WalTransactionId,
    first_lsn: Lsn,
    authority: WalAppendAuthority,
    kind: WalTransactionKind,
) -> WalTransactionBuilder {
    WalTransactionBuilder::new(
        epoch_id(),
        WalSegmentId::from_raw(1),
        transaction_id,
        kind,
        authority,
        first_lsn,
        digest("previous-frame"),
        digest("previous-commit"),
        WalDurabilityMode::Buffered,
        PayloadCodecId::from_hash(digest("codec")),
        PayloadSchemaId::from_hash(digest("schema")),
        1,
        1,
        digest("domain"),
    )
}

fn frontier(kind: AffectedFrontierKind, before: &str, after: &str) -> AffectedFrontier {
    AffectedFrontier {
        kind,
        before_digest: digest(before),
        after_digest: digest(after),
    }
}

fn submission_acceptance(label: &str) -> SubmissionAcceptanceRecord {
    SubmissionAcceptanceRecord {
        submission_id: digest(&format!("submission:{label}")),
        canonical_envelope_digest: digest(&format!("envelope:{label}")),
        idempotency_key_digest: None,
        acceptance_evidence_digest: digest(&format!("accepted-evidence:{label}")),
    }
}

fn receipt_record(label: &str, decision: WalTickDecision) -> TickReceiptRecord {
    TickReceiptRecord {
        submission_id: digest(&format!("submission:{label}")),
        ticket_digest: digest(&format!("ticket:{label}")),
        receipt_digest: digest(&format!("receipt:{label}")),
        decision,
    }
}

fn correlation_record(label: &str) -> WalReceiptCorrelationRecord {
    WalReceiptCorrelationRecord {
        submission_id: digest(&format!("submission:{label}")),
        ticket_digest: digest(&format!("ticket:{label}")),
        receipt_digest: digest(&format!("receipt:{label}")),
    }
}

fn retained_material(
    label: &str,
    kind: RetainedMaterialKind,
    posture: EvidenceMaterialPosture,
) -> RetainedMaterialRecord {
    RetainedMaterialRecord {
        material_digest: digest(&format!("material:{label}")),
        semantic_coordinate_digest: digest(&format!("coordinate:{label}")),
        kind,
        posture,
    }
}

fn reading_ref(label: &str, posture: EvidenceMaterialPosture) -> ReadingRefRecord {
    ReadingRefRecord {
        reading_id: digest(&format!("reading:{label}")),
        semantic_coordinate_digest: digest(&format!("coordinate:{label}")),
        payload_digest: digest(&format!("material:{label}:payload")),
        envelope_digest: digest(&format!("material:{label}:envelope")),
        posture,
    }
}

fn submission_transaction(first_lsn: Lsn) -> WalCommittedTransaction {
    let mut builder = builder(
        transaction_id("tx:submission"),
        first_lsn,
        WalAppendAuthority::SubmissionIntake,
        WalTransactionKind::SubmissionIntake,
    );
    must_ok(builder.push_record(
        WalRecordKind::SubmissionAcceptedRecorded,
        b"accepted".to_vec(),
    ));
    must_ok(builder.commit(vec![frontier(
        AffectedFrontierKind::SubmissionQueue,
        "queue:before",
        "queue:after",
    )]))
}

fn durable_submission_transaction(label: &str, first_lsn: Lsn) -> WalCommittedTransaction {
    let builder = builder(
        transaction_id(&format!("tx:submission:{label}")),
        first_lsn,
        WalAppendAuthority::SubmissionIntake,
        WalTransactionKind::SubmissionIntake,
    );
    must_ok(build_submission_acceptance_transaction(
        builder,
        submission_acceptance(label),
        vec![frontier(
            AffectedFrontierKind::SubmissionQueue,
            &format!("queue:{label}:before"),
            &format!("queue:{label}:after"),
        )],
    ))
}

fn tick_transaction(first_lsn: Lsn) -> WalCommittedTransaction {
    let mut builder = builder(
        transaction_id("tx:tick"),
        first_lsn,
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(builder.push_record(WalRecordKind::TickReceiptRecorded, b"receipt".to_vec()));
    must_ok(builder.push_record(WalRecordKind::RuntimeStateDeltaRecorded, b"delta".to_vec()));
    must_ok(builder.commit(vec![
        frontier(
            AffectedFrontierKind::RuntimeState,
            "state:before",
            "state:after",
        ),
        frontier(
            AffectedFrontierKind::ReceiptIndex,
            "receipt:before",
            "receipt:after",
        ),
    ]))
}

fn durable_tick_transaction(
    label: &str,
    first_lsn: Lsn,
    decision: WalTickDecision,
) -> WalCommittedTransaction {
    let builder = builder(
        transaction_id(&format!("tx:tick:{label}")),
        first_lsn,
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(build_tick_transaction(
        builder,
        receipt_record(label, decision),
        correlation_record(label),
        digest(&format!("state-delta:{label}")),
        vec![
            frontier(
                AffectedFrontierKind::RuntimeState,
                &format!("state:{label}:before"),
                &format!("state:{label}:after"),
            ),
            frontier(
                AffectedFrontierKind::ReceiptIndex,
                &format!("receipt:{label}:before"),
                &format!("receipt:{label}:after"),
            ),
        ],
    ))
}

#[test]
fn record_kind_name_does_not_imply_commit_before_transaction_commit() {
    let kinds = [
        WalRecordKind::SubmissionAcceptedRecorded,
        WalRecordKind::SubmissionAcceptanceEvidenceRecorded,
        WalRecordKind::RuntimeLawWitnessRecorded,
        WalRecordKind::RuntimeAdmissionTicketIssued,
        WalRecordKind::TicketedRuntimeIngressRecorded,
        WalRecordKind::TickReceiptRecorded,
        WalRecordKind::RuntimeStateDeltaRecorded,
        WalRecordKind::ReceiptCorrelationRecorded,
        WalRecordKind::ReadingEnvelopeRetained,
        WalRecordKind::RetainedMaterialRefRecorded,
        WalRecordKind::SchedulerFaultQuarantined,
        WalRecordKind::TrustedRuntimeControlRecorded,
        WalRecordKind::CheckpointPublicationRecorded,
        WalRecordKind::MaterializationIntentRecorded,
        WalRecordKind::MaterializationEffectObserved,
        WalRecordKind::RecoveryPostureRecorded,
    ];

    assert!(kinds
        .iter()
        .all(|kind| kind.obeys_recorded_not_committed_grammar()));
}

#[test]
fn application_cannot_append_tick_or_runtime_control_records() {
    let mut builder = builder(
        transaction_id("tx:bad-authority"),
        Lsn::from_raw(0),
        WalAppendAuthority::SubmissionIntake,
        WalTransactionKind::SubmissionIntake,
    );

    let error = builder.push_record(WalRecordKind::TickReceiptRecorded, b"receipt".to_vec());

    assert!(matches!(
        error,
        Err(WalBuildError::WrongAppendAuthority {
            record_kind: WalRecordKind::TickReceiptRecorded,
            required: WalAppendAuthority::TrustedScheduler,
            actual: WalAppendAuthority::SubmissionIntake
        })
    ));
}

#[test]
fn transaction_builder_creates_contiguous_lsn_and_local_indexes() {
    let tx = tick_transaction(Lsn::from_raw(7));

    assert_eq!(tx.frames[0].header.lsn, Lsn::from_raw(7));
    assert_eq!(tx.frames[1].header.lsn, Lsn::from_raw(8));
    assert_eq!(
        tx.frames[0].header.transaction_local_index,
        TransactionLocalIndex::from_raw(0)
    );
    assert_eq!(
        tx.frames[1].header.transaction_local_index,
        TransactionLocalIndex::from_raw(1)
    );
    assert_eq!(tx.commit.first_lsn, Lsn::from_raw(7));
    assert_eq!(tx.commit.last_lsn, Lsn::from_raw(8));
    assert!(tx.validate().is_ok());
}

#[test]
fn commit_validation_rejects_record_count_mismatch() {
    let mut tx = tick_transaction(Lsn::from_raw(0));
    tx.commit.record_count = 99;

    assert!(tx.validate().is_err());
}

#[test]
fn commit_validation_rejects_corrupt_payload_even_when_commit_shape_exists() {
    let mut tx = submission_transaction(Lsn::from_raw(0));
    tx.frames[0].payload.canonical_bytes = b"tampered".to_vec();

    assert!(tx.validate().is_err());
}

#[test]
fn semantic_validation_rejects_record_authority_mismatch() {
    let mut builder = builder(
        transaction_id("tx:mismatched-kind"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SubmissionIntake,
    );
    must_ok(builder.push_record(WalRecordKind::TickReceiptRecorded, b"receipt".to_vec()));
    let tx = builder.commit(vec![frontier(
        AffectedFrontierKind::ReceiptIndex,
        "receipt:before",
        "receipt:after",
    )]);

    assert!(matches!(
        tx,
        Err(WalBuildError::Validation(
            warp_core::causal_wal::WalValidationError::RecordAuthorityMismatch
        ))
    ));
}

#[test]
fn in_memory_store_appends_and_recovers_committed_transactions() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(submission_transaction(Lsn::from_raw(0))));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));

    assert_eq!(report.transactions.len(), 1);
    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(store.read_frames().len(), 1);
}

#[test]
fn wal_store_port_seals_segment_and_publishes_manifest() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(submission_transaction(Lsn::from_raw(0))));

    let seal = must_ok(store.seal_segment(epoch_id(), WalSegmentId::from_raw(1)));
    assert_eq!(seal.segment_id, WalSegmentId::from_raw(1));
    assert_eq!(seal.sealed_lsn, Some(Lsn::from_raw(0)));

    must_ok(store.publish_manifest(
        epoch_id(),
        WalManifest {
            manifest_digest: digest("manifest"),
            last_committed_lsn: Some(Lsn::from_raw(0)),
            last_commit_digest: Some(store.read_commits()[0].commit_digest),
            sealed_segment_count: 1,
        },
    ));
    assert_eq!(store.manifests().len(), 1);
}

#[test]
fn overlapping_writer_epochs_block_recovery() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));

    let second = store.acquire_writer_epoch(WriterEpochRequest {
        epoch_id: WriterEpochId::from_hash(digest("epoch:2")),
        ..writer_epoch_request()
    });

    assert!(matches!(
        second,
        Err(WalStoreError::WriterEpochAlreadyActive)
    ));
}

#[test]
fn read_only_recovery_reports_uncommitted_tail_without_truncating() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(submission_transaction(Lsn::from_raw(0))));
    let uncommitted = tick_transaction(Lsn::from_raw(1)).frames.remove(0);
    must_ok(store.append_uncommitted_frame(epoch_id(), uncommitted));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(0))
    );
    assert_eq!(store.read_frames().len(), 2);
}

#[test]
fn writable_recovery_truncates_uncommitted_tail() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(submission_transaction(Lsn::from_raw(0))));
    let uncommitted = tick_transaction(Lsn::from_raw(1)).frames.remove(0);
    must_ok(store.append_uncommitted_frame(epoch_id(), uncommitted));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::Writable,
    ));

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::TruncatedAfter(Lsn::from_raw(0))
    );
    assert_eq!(store.read_frames().len(), 1);
}

#[test]
fn pure_replay_reducer_is_deterministic_over_committed_transactions() {
    let tx = tick_transaction(Lsn::from_raw(0));

    let first = must_ok(apply_committed_transaction(RecoveredState::default(), &tx));
    let second = must_ok(apply_committed_transaction(RecoveredState::default(), &tx));

    assert_eq!(first, second);
    assert_eq!(first.applied_transactions, vec![tx.commit.transaction_id]);
    assert_eq!(
        first
            .frontiers
            .get(&AffectedFrontierKind::RuntimeState)
            .copied(),
        Some(digest("state:after"))
    );
}

#[test]
fn schema_linter_rejects_app_nouns_and_authority_leaks() {
    let app_noun = lint_wal_schema_terms(["TextBufferCommit"], &["TextBuffer"]);
    assert!(matches!(
        app_noun,
        Err(WalSchemaLintError::ForbiddenAppNoun { .. })
    ));

    let authority_leak = lint_wal_schema_terms(["ClientRuntimeControl"], &[]);
    assert!(matches!(
        authority_leak,
        Err(WalSchemaLintError::ForbiddenAuthorityLeak { .. })
    ));

    let commit_name = lint_wal_schema_terms(["TickReceiptCommitted"], &[]);
    assert!(matches!(
        commit_name,
        Err(WalSchemaLintError::RecordNameImpliesCommit { .. })
    ));

    assert!(lint_wal_schema_terms(
        [
            "SubmissionAcceptedRecorded",
            "TickReceiptRecorded",
            "WalTransactionCommit"
        ],
        &["TextBuffer"]
    )
    .is_ok());
}

#[test]
fn accepted_submission_is_not_returned_before_wal_commit() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let tx = durable_submission_transaction("accept", Lsn::from_raw(0));
    let uncommitted = tx.frames[0].clone();
    must_ok(store.append_uncommitted_frame(epoch_id(), uncommitted));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_submission_index(&report));

    assert!(index.is_empty());
    assert_eq!(
        index.retry_posture(
            submission_acceptance("accept").submission_id,
            submission_acceptance("accept").canonical_envelope_digest
        ),
        SubmissionRetryPosture::NotAccepted
    );
}

#[test]
fn crash_after_submission_commit_recovers_pending_submission() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction("pending", Lsn::from_raw(0))));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_submission_index(&report));
    let entry = must_some(index.get(&submission_acceptance("pending").submission_id));

    assert_eq!(entry.posture, RecoveredSubmissionPosture::AcceptedPending);
    assert_eq!(index.len(), 1);
}

#[test]
fn crash_after_submission_commit_before_ack_retry_returns_duplicate_posture() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction("retry", Lsn::from_raw(0))));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_submission_index(&report));
    let record = submission_acceptance("retry");

    assert_eq!(
        index.retry_posture(record.submission_id, record.canonical_envelope_digest),
        SubmissionRetryPosture::AlreadyAcceptedPending
    );
}

#[test]
fn same_payload_new_submission_id_is_not_duplicate_without_policy() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "same-payload",
        Lsn::from_raw(0),
    )));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let index = must_ok(recover_submission_index(&report));
    let existing = submission_acceptance("same-payload");

    assert_eq!(
        index.retry_posture(
            digest("submission:new-id"),
            existing.canonical_envelope_digest
        ),
        SubmissionRetryPosture::NewSubmissionWithoutPolicyDedupe
    );
    assert_eq!(
        index.retry_posture(existing.submission_id, digest("envelope:different")),
        SubmissionRetryPosture::ConflictSameIdDifferentEnvelope
    );
}

#[test]
fn recovery_certificate_reports_clean_submission_posture() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "certificate",
        Lsn::from_raw(0),
    )));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let certificate =
        build_recovery_certificate(&report, None, 0, digest("frontiers"), digest("indexes"));

    assert_eq!(certificate.committed_transactions_replayed, 1);
    assert_eq!(certificate.first_lsn, Some(Lsn::from_raw(0)));
    assert_eq!(certificate.last_lsn, Some(Lsn::from_raw(1)));
    assert_eq!(certificate.tail_posture, RecoveryTailPosture::Clean);
}

#[test]
fn crash_before_tick_commit_commits_no_receipt() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "tick-tail",
        Lsn::from_raw(0),
    )));
    let tx = durable_tick_transaction("tick-tail", Lsn::from_raw(2), WalTickDecision::Applied);
    must_ok(store.append_uncommitted_frame(epoch_id(), tx.frames[0].clone()));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let receipt_index = must_ok(recover_receipt_index(&report));

    assert!(receipt_index.receipt_by_submission.is_empty());
    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
}

#[test]
fn crash_after_tick_commit_recovers_receipt_and_state_delta() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction("applied", Lsn::from_raw(0))));
    must_ok(store.append_transaction(durable_tick_transaction(
        "applied",
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    )));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let submissions = must_ok(recover_submission_index(&report));
    let receipts = must_ok(recover_receipt_index(&report));

    assert_eq!(
        must_some(submissions.get(&submission_acceptance("applied").submission_id)).posture,
        RecoveredSubmissionPosture::DecidedApplied
    );
    assert_eq!(
        receipts
            .receipt_by_submission
            .get(&digest("submission:applied"))
            .copied(),
        Some(digest("receipt:applied"))
    );
}

#[test]
fn committed_receipt_correlation_rebuilds_after_restart() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_tick_transaction(
        "correlated",
        Lsn::from_raw(0),
        WalTickDecision::Applied,
    )));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let receipts = must_ok(recover_receipt_index(&report));

    assert_eq!(
        receipts
            .receipt_by_ticket
            .get(&digest("ticket:correlated"))
            .copied(),
        Some(digest("receipt:correlated"))
    );
    assert_eq!(
        receipts
            .ticket_by_submission
            .get(&digest("submission:correlated"))
            .copied(),
        Some(digest("ticket:correlated"))
    );
}

#[test]
fn lawful_rejection_recovers_without_fault_posture() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction("rejected", Lsn::from_raw(0))));
    must_ok(store.append_transaction(durable_tick_transaction(
        "rejected",
        Lsn::from_raw(2),
        WalTickDecision::RejectedFootprintConflict,
    )));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let submissions = must_ok(recover_submission_index(&report));
    let receipts = must_ok(recover_receipt_index(&report));

    assert_eq!(
        must_some(submissions.get(&digest("submission:rejected"))).posture,
        RecoveredSubmissionPosture::DecidedRejected
    );
    assert!(must_some(
        receipts
            .decisions_by_receipt
            .get(&digest("receipt:rejected"))
            .copied()
    )
    .is_lawful_rejection());
}

#[test]
fn retained_queryview_reading_lookup_recovers_by_semantic_coordinate() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let material = [
        retained_material(
            "query",
            RetainedMaterialKind::ReadingPayload,
            EvidenceMaterialPosture::Present,
        ),
        RetainedMaterialRecord {
            material_digest: digest("material:query:envelope"),
            semantic_coordinate_digest: digest("coordinate:query"),
            kind: RetainedMaterialKind::ReadingEnvelope,
            posture: EvidenceMaterialPosture::Present,
        },
    ];
    let builder = builder(
        transaction_id("tx:reading"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(
        store.append_transaction(must_ok(build_retained_reading_transaction(
            builder,
            &material,
            reading_ref("query", EvidenceMaterialPosture::Present),
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "reading:before",
                "reading:after",
            )],
        ))),
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));

    assert!(must_some(
        retention
            .readings_by_semantic_coordinate
            .get(&digest("coordinate:query"))
    )
    .contains(&digest("reading:query")));
}

#[test]
fn same_payload_with_different_query_coordinate_remains_distinct() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let payload = digest("shared-payload");
    let first = RetainedMaterialRecord {
        material_digest: payload,
        semantic_coordinate_digest: digest("coordinate:first"),
        kind: RetainedMaterialKind::ReadingPayload,
        posture: EvidenceMaterialPosture::Present,
    };
    let second = RetainedMaterialRecord {
        material_digest: payload,
        semantic_coordinate_digest: digest("coordinate:second"),
        kind: RetainedMaterialKind::ReadingPayload,
        posture: EvidenceMaterialPosture::Present,
    };
    let builder = builder(
        transaction_id("tx:semantic-identity"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(
        store.append_transaction(must_ok(build_retained_reading_transaction(
            builder,
            &[first, second],
            reading_ref("semantic-identity", EvidenceMaterialPosture::Present),
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "reading:before",
                "reading:after",
            )],
        ))),
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));

    assert!(retention
        .material_by_semantic_coordinate
        .contains_key(&digest("coordinate:first")));
    assert!(retention
        .material_by_semantic_coordinate
        .contains_key(&digest("coordinate:second")));
}

#[test]
fn security_and_redaction_postures_decode_without_becoming_missing() {
    for posture in [
        EvidenceMaterialPosture::Present,
        EvidenceMaterialPosture::RedactedByPolicy,
        EvidenceMaterialPosture::EncryptedKeyUnavailable,
        EvidenceMaterialPosture::Missing,
        EvidenceMaterialPosture::Corrupt,
        EvidenceMaterialPosture::Obstructed,
    ] {
        let record = retained_material("posture", RetainedMaterialKind::ReadingPayload, posture);
        let decoded = must_ok(RetainedMaterialRecord::from_payload_bytes(
            &record.to_payload_bytes(),
        ));
        assert_eq!(decoded.posture, posture);
    }
}

#[test]
fn missing_retained_material_scope_matrix_is_precise() {
    assert_eq!(
        missing_material_scope(RetainedMaterialKind::SubmissionPayload),
        MissingMaterialScope::Submission
    );
    assert_eq!(
        missing_material_scope(RetainedMaterialKind::TickReceipt),
        MissingMaterialScope::ReceiptOrTicket
    );
    assert_eq!(
        missing_material_scope(RetainedMaterialKind::RuntimeStateDelta),
        MissingMaterialScope::RuntimeGlobal
    );
    assert_eq!(
        missing_material_scope(RetainedMaterialKind::ReadingEnvelope),
        MissingMaterialScope::Reading
    );
    assert_eq!(
        missing_material_scope(RetainedMaterialKind::Diagnostic),
        MissingMaterialScope::DiagnosticLoss
    );
}

#[test]
fn missing_retained_material_returns_typed_obstruction() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let missing = retained_material(
        "missing",
        RetainedMaterialKind::RuntimeStateDelta,
        EvidenceMaterialPosture::Present,
    );
    let builder = builder(
        transaction_id("tx:missing-material"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::SchedulerTick,
    );
    must_ok(
        store.append_transaction(must_ok(build_retained_reading_transaction(
            builder,
            &[missing],
            reading_ref("missing", EvidenceMaterialPosture::Present),
            vec![frontier(
                AffectedFrontierKind::ReadingIndex,
                "reading:before",
                "reading:after",
            )],
        ))),
    );
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));
    let obstructions = retained_material_obstructions(&retention, &BTreeSet::new());

    assert_eq!(obstructions.len(), 1);
    assert_eq!(obstructions[0].scope, MissingMaterialScope::RuntimeGlobal);
    assert_eq!(obstructions[0].posture, EvidenceMaterialPosture::Missing);
}

#[test]
fn valid_checkpoint_without_checkpoint_published_record_can_be_used_after_validation() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "checkpoint",
        Lsn::from_raw(0),
    )));
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let checkpoint = CheckpointRecord {
        checkpoint_id: digest("checkpoint"),
        last_included_lsn: must_some(report.last_committed_lsn()),
        last_included_commit_digest: must_some(report.last_commit_digest()),
        state_root: digest("state-root"),
        index_root: digest("index-root"),
        retained_material_root: digest("material-root"),
        schema_version: 1,
        created_from_wal_digest: digest("wal-chain"),
    };

    assert_eq!(
        validate_checkpoint_record(&checkpoint, &report, &[]),
        CheckpointValidationPosture::UsableWithoutPublicationRecord
    );
}

#[test]
fn checkpoint_writer_roundtrips_atomic_checkpoint_file() {
    let checkpoint = CheckpointRecord {
        checkpoint_id: digest("checkpoint-file"),
        last_included_lsn: Lsn::from_raw(42),
        last_included_commit_digest: digest("commit:file"),
        state_root: digest("state:file"),
        index_root: digest("index:file"),
        retained_material_root: digest("material:file"),
        schema_version: 1,
        created_from_wal_digest: digest("wal:file"),
    };
    let path = temp_checkpoint_path("roundtrip");

    must_ok(write_checkpoint_record_atomic(&path, &checkpoint));
    let restored = must_ok(read_checkpoint_record(&path));

    assert_eq!(restored, checkpoint);
    let temp_path = path.with_file_name(".checkpoint.ecwal.tmp");
    assert!(!temp_path.exists());
}

#[test]
fn checkpoint_published_without_checkpoint_blocks_or_obstructs_according_to_scope() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "checkpoint-missing",
        Lsn::from_raw(0),
    )));
    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let publication = CheckpointPublicationRecord {
        checkpoint_id: digest("checkpoint-missing"),
        checkpoint_digest: digest("checkpoint-digest"),
    };

    assert_eq!(
        evaluate_checkpoint_publication(&publication, None, &report),
        CheckpointValidationPosture::PublishedCheckpointMaterialMissing
    );
}

#[test]
fn checkpoint_publication_roundtrip_recovers_from_wal() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let publication = CheckpointPublicationRecord {
        checkpoint_id: digest("checkpoint-publication"),
        checkpoint_digest: digest("checkpoint-publication-digest"),
    };
    let builder = builder(
        transaction_id("tx:checkpoint-publication"),
        Lsn::from_raw(0),
        WalAppendAuthority::Recovery,
        WalTransactionKind::Checkpoint,
    );
    must_ok(
        store.append_transaction(must_ok(build_checkpoint_publication_transaction(
            builder,
            publication,
            vec![frontier(
                AffectedFrontierKind::CheckpointIndex,
                "checkpoint:before",
                "checkpoint:after",
            )],
        ))),
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let publications = must_ok(recover_checkpoint_publications(&report));

    assert_eq!(publications, vec![publication]);
}

#[test]
fn read_only_wal_doctor_reports_uncommitted_tail_without_truncating() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction("doctor", Lsn::from_raw(0))));
    let tx = durable_tick_transaction("doctor", Lsn::from_raw(2), WalTickDecision::Applied);
    must_ok(store.append_uncommitted_frame(epoch_id(), tx.frames[0].clone()));

    let before = store.read_frames().len();
    let report = must_ok(doctor_in_memory_store(&store));

    assert_eq!(
        report.posture,
        WalDoctorPosture::RecoverableWithUncommittedTail
    );
    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert_eq!(store.read_frames().len(), before);
}

#[test]
fn filesystem_wal_adapter_recovers_committed_transaction_from_disk() {
    let dir = temp_wal_dir("filesystem-committed");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "filesystem",
        Lsn::from_raw(0),
    )));

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let index = must_ok(recover_submission_index(&report));

    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(
        index.retry_posture(
            submission_acceptance("filesystem").submission_id,
            submission_acceptance("filesystem").canonical_envelope_digest
        ),
        SubmissionRetryPosture::AlreadyAcceptedPending
    );
}

#[test]
fn filesystem_read_only_recovery_reports_uncommitted_tail_without_truncating() {
    let dir = temp_wal_dir("filesystem-tail-read-only");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "filesystem-tail",
        Lsn::from_raw(0),
    )));
    let tx = durable_tick_transaction(
        "filesystem-tail",
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    );
    must_ok(store.append_uncommitted_frame(epoch_id(), tx.frames[0].clone()));
    let before = must_ok(fs::metadata(store.segment_path())).len();

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let after = must_ok(fs::metadata(store.segment_path())).len();

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert_eq!(after, before);
}

#[test]
fn filesystem_writable_recovery_truncates_uncommitted_tail() {
    let dir = temp_wal_dir("filesystem-tail-writable");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "filesystem-writable",
        Lsn::from_raw(0),
    )));
    let tx = durable_tick_transaction(
        "filesystem-writable",
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    );
    must_ok(store.append_uncommitted_frame(epoch_id(), tx.frames[0].clone()));

    let writable = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::Writable));
    let read_only_after = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));

    assert_eq!(
        writable.tail_posture,
        RecoveryTailPosture::TruncatedAfter(Lsn::from_raw(1))
    );
    assert_eq!(read_only_after.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(read_only_after.transactions.len(), 1);
}

#[test]
fn filesystem_torn_final_record_is_reported_as_tail_not_history() {
    let dir = temp_wal_dir("filesystem-torn-tail");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(durable_submission_transaction(
        "filesystem-torn",
        Lsn::from_raw(0),
    )));
    let tx = durable_tick_transaction(
        "filesystem-torn",
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    );
    must_ok(store.append_uncommitted_frame(epoch_id(), tx.frames[0].clone()));
    let segment_path = store.segment_path();
    let original_len = must_ok(fs::metadata(&segment_path)).len();
    let file = must_ok(OpenOptions::new().write(true).open(&segment_path));
    must_ok(file.set_len(original_len.saturating_sub(13)));

    let report = must_ok(recover_filesystem_store(&dir, RecoveryAccessMode::ReadOnly));
    let receipts = must_ok(recover_receipt_index(&report));

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert!(receipts.receipt_by_submission.is_empty());
}

#[test]
fn filesystem_manifest_publish_writes_manifest_material() {
    let dir = temp_wal_dir("filesystem-manifest");
    let mut store = must_ok(FilesystemWalStore::open(&dir, WalSegmentId::from_raw(1)));
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let manifest = WalManifest {
        manifest_digest: digest("manifest:filesystem"),
        last_committed_lsn: Some(Lsn::from_raw(9)),
        last_commit_digest: Some(digest("commit:filesystem")),
        sealed_segment_count: 1,
    };

    must_ok(store.publish_manifest(epoch_id(), manifest));

    assert!(dir.join("manifest.ecwal").exists());
}

#[test]
fn strict_object_store_requires_conditional_manifest_commit() {
    let invalid = ObjectStoreWalCapabilities {
        content_addressed_object_write: true,
        verify_object_version: true,
        conditional_manifest_commit: false,
        read_after_write: ObjectStoreReadAfterWritePosture::Verified,
    };
    assert!(matches!(
        validate_strict_object_store_capabilities(invalid),
        Err(ObjectStoreCapabilityError::MissingConditionalManifestCommit)
    ));

    let valid = ObjectStoreWalCapabilities {
        conditional_manifest_commit: true,
        ..invalid
    };
    assert!(validate_strict_object_store_capabilities(valid).is_ok());
}

#[test]
fn materialization_replay_detects_existing_artifact_before_retry() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let intent = MaterializationIntentRecord {
        effect_id: digest("effect:existing"),
        expected_artifact_digest: digest("artifact:expected"),
        materialization_intent_digest: digest("materialization:intent"),
        idempotency_token: digest("idempotency:effect"),
        target_metadata_digest: digest("artifact:metadata"),
    };
    let builder = builder(
        transaction_id("tx:outbox"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::MaterializationOutbox,
    );
    must_ok(
        store.append_transaction(must_ok(build_materialization_outbox_transaction(
            builder,
            intent,
            None,
            vec![frontier(
                AffectedFrontierKind::ReceiptIndex,
                "outbox:before",
                "outbox:after",
            )],
        ))),
    );
    let mut existing = BTreeMap::new();
    existing.insert(
        intent.effect_id,
        ExistingMaterializedArtifact {
            effect_id: intent.effect_id,
            artifact_digest: intent.expected_artifact_digest,
            metadata_digest: intent.target_metadata_digest,
        },
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let outbox = must_ok(recover_materialization_outbox(&report, &existing));

    assert_eq!(
        must_some(outbox.get(&intent.effect_id)).posture,
        MaterializationReplayPosture::ExistingArtifactMatches
    );
}

#[test]
fn materialization_observation_marks_effect_already_observed() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let intent = MaterializationIntentRecord {
        effect_id: digest("effect:observed"),
        expected_artifact_digest: digest("artifact:observed"),
        materialization_intent_digest: digest("materialization:observed"),
        idempotency_token: digest("idempotency:observed"),
        target_metadata_digest: digest("artifact:observed:metadata"),
    };
    let observation = MaterializationObservationRecord {
        effect_id: intent.effect_id,
        observed_artifact_digest: intent.expected_artifact_digest,
        observed_metadata_digest: intent.target_metadata_digest,
    };
    let builder = builder(
        transaction_id("tx:outbox-observed"),
        Lsn::from_raw(0),
        WalAppendAuthority::TrustedScheduler,
        WalTransactionKind::MaterializationOutbox,
    );
    must_ok(
        store.append_transaction(must_ok(build_materialization_outbox_transaction(
            builder,
            intent,
            Some(observation),
            vec![frontier(
                AffectedFrontierKind::ReceiptIndex,
                "outbox:before",
                "outbox:after",
            )],
        ))),
    );

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let outbox = must_ok(recover_materialization_outbox(&report, &BTreeMap::new()));

    assert_eq!(
        must_some(outbox.get(&intent.effect_id)).posture,
        MaterializationReplayPosture::AlreadyObserved
    );
}

#[test]
fn causal_commit_evidence_projects_wal_commit_anchor() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let tx = durable_submission_transaction("commit-evidence", Lsn::from_raw(0));
    let commit_digest = tx.commit.commit_digest;
    must_ok(store.append_transaction(tx));

    let report = must_ok(recover_in_memory_store(
        &mut store,
        RecoveryAccessMode::ReadOnly,
    ));
    let evidence = project_causal_commit_evidence(&report);

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].commit_digest, commit_digest);
    assert_eq!(evidence[0].lsn, Lsn::from_raw(1));
}

#[test]
fn wal_shadow_replay_detects_recovered_state_match() {
    let tx = tick_transaction(Lsn::from_raw(0));
    let live_state = must_ok(apply_committed_transaction(RecoveredState::default(), &tx));

    assert!(must_ok(shadow_replay_matches(&live_state, &[tx])));
}

#[test]
fn wal_release_readiness_audit_reports_blocked_and_ready_gates() {
    let blocked = audit_wal_release_readiness(WalReleaseReadinessGates {
        filesystem_adapter: true,
        object_store_capability_gate: true,
        ..WalReleaseReadinessGates::default()
    });
    assert!(!blocked.ready);
    assert!(blocked.passed_gates.contains(&"filesystem_adapter"));
    assert!(blocked.blocked_gates.contains(&"shadow_replay"));

    let ready = audit_wal_release_readiness(WalReleaseReadinessGates {
        filesystem_adapter: true,
        object_store_capability_gate: true,
        segment_repair: true,
        segment_layout_policy: true,
        segment_manifest_validation: true,
        crash_matrix: true,
        crashpoint_manifest: true,
        shadow_replay: true,
        outbox: true,
        commit_evidence: true,
        wal_doctor: true,
        semantic_validator: true,
        filesystem_sync_evidence: true,
        object_store_manifest_negatives: true,
        security_redaction: true,
        app_noun_guard: true,
        external_consumer_gate: true,
    });
    assert!(ready.ready);
    assert!(ready.blocked_gates.is_empty());
}
