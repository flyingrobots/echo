// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Causal WAL foundation tests.

use warp_core::causal_wal::{
    apply_committed_transaction, lint_wal_schema_terms, recover_in_memory_store, AffectedFrontier,
    AffectedFrontierKind, InMemoryWalStore, Lsn, PayloadCodecId, PayloadSchemaId, RecoveredState,
    RecoveryAccessMode, RecoveryTailPosture, TransactionLocalIndex, WalAppendAuthority,
    WalBuildError, WalCommittedTransaction, WalDurabilityMode, WalManifest, WalRecordKind,
    WalSchemaLintError, WalSegmentId, WalStoreError, WalStorePort, WalTransactionBuilder,
    WalTransactionId, WalTransactionKind, WriterEpochId, WriterEpochRequest,
};
use warp_core::Hash;

fn digest(label: &str) -> Hash {
    blake3::hash(label.as_bytes()).into()
}

fn must_ok<T, E>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(_) => std::process::abort(),
    }
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
