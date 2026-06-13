// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Adversarial causal WAL hardening tests.

use warp_core::causal_wal::{
    apply_committed_transaction, audit_wal_release_readiness,
    build_materialization_outbox_transaction, build_retained_reading_transaction,
    build_submission_acceptance_transaction, build_tick_transaction,
    canonical_segment_relative_path, doctor_filesystem_store, doctor_in_memory_store,
    doctor_in_memory_store_with_materials, evaluate_checkpoint_publication,
    inspect_evidence_material_posture, next_segment_id, project_absent_causal_commit_evidence,
    project_causal_commit_evidence, project_obstructed_causal_commit_evidence,
    read_checkpoint_record, read_filesystem_manifest, recover_filesystem_store,
    recover_from_frames_and_commits, recover_materialization_outbox, recover_receipt_index,
    recover_retention_index, recover_submission_index, retained_material_obstructions,
    segment_manifest_entry, shadow_replay_matches, shadow_replay_report,
    validate_checkpoint_record, validate_filesystem_manifest,
    validate_filesystem_strict_sync_evidence, validate_segment_placement_policy,
    validate_strict_object_store_capabilities, validate_strict_object_store_manifest_commit,
    wal_crashpoint_manifest, write_checkpoint_record_atomic,
    write_checkpoint_record_atomic_with_evidence, AffectedFrontier, AffectedFrontierKind,
    CausalCommitEvidencePosture, CausalCommitEvidenceSource, CheckpointPublicationRecord,
    CheckpointRecord, CheckpointValidationPosture, EvidenceMaterialPosture,
    ExistingMaterializedArtifact, FilesystemSyncBoundary, FilesystemSyncEvidenceError,
    FilesystemWalStore, InMemoryWalStore, Lsn, MaterialInspectionPosture,
    MaterializationIntentRecord, MaterializationObservationRecord, MaterializationReplayPosture,
    MissingMaterialScope, ObjectStoreCapabilityError, ObjectStoreManifestCommitMode,
    ObjectStoreManifestCommitShape, ObjectStoreReadAfterWritePosture, ObjectStoreWalCapabilities,
    PayloadCodecId, PayloadSchemaId, ReadingRefRecord, RecoveredState, RecoveredSubmissionPosture,
    RecoveryAccessMode, RecoveryTailPosture, RetainedMaterialKind, RetainedMaterialRecord,
    ShadowReplayMismatch, SubmissionAcceptanceRecord, SubmissionRetryPosture, TickReceiptRecord,
    WalAppendAuthority, WalBuildError, WalCommittedTransaction, WalCrashpointBoundary,
    WalCrashpointExecution, WalDoctorPosture, WalDoctorReport, WalDurabilityMode, WalManifest,
    WalReceiptCorrelationRecord, WalRecordKind, WalRecoveryError, WalReleaseReadinessGates,
    WalReplayError, WalSegmentId, WalSegmentIdError, WalSegmentPlacementKind,
    WalSegmentPlacementPolicy, WalSegmentPlacementPolicyError, WalStoreError, WalStorePort,
    WalTickDecision, WalTransactionBuilder, WalTransactionId, WalTransactionKind,
    WalValidationError, WriterEpochId, WriterEpochRequest,
};
use warp_core::Hash;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const WAL_SEGMENT_RECORD_MAGIC: &[u8; 8] = b"ECWALR1!";
const WAL_DISK_RECORD_DOMAIN: &[u8] = b"echo:causal_wal:disk_record:v1\0";

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn digest(label: &str) -> Hash {
    blake3::hash(label.as_bytes()).into()
}

fn must_ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected Ok(..), got {error:?}"),
    }
}

fn must_err<T: std::fmt::Debug, E>(result: Result<T, E>, context: &str) -> E {
    match result {
        Err(error) => error,
        Ok(value) => panic!("{context}: expected Err(..), got Ok({value:?})"),
    }
}

fn must_some<T>(option: Option<T>, context: &str) -> T {
    if let Some(value) = option {
        value
    } else {
        panic!("{context}: expected Some(..), got None");
    }
}

fn transaction_id(label: &str) -> WalTransactionId {
    WalTransactionId::from_hash(digest(label))
}

fn epoch_id() -> WriterEpochId {
    WriterEpochId::from_hash(digest("hardening:epoch:1"))
}

fn epoch_id_for(label: &str) -> WriterEpochId {
    WriterEpochId::from_hash(digest(&format!("hardening:epoch:{label}")))
}

fn writer_epoch_request() -> WriterEpochRequest {
    WriterEpochRequest {
        epoch_id: epoch_id(),
        storage_fencing_token: digest("hardening:fence:1"),
        process_identity: digest("hardening:process:1"),
        host_identity: digest("hardening:host:1"),
        started_at_lsn: Lsn::from_raw(0),
        previous_epoch_id: None,
        previous_epoch_final_commit_digest: None,
        lease_or_lock_evidence: digest("hardening:lock:1"),
    }
}

fn writer_epoch_request_for(
    label: &str,
    started_at_lsn: Lsn,
    previous_epoch_id: Option<WriterEpochId>,
    previous_epoch_final_commit_digest: Option<Hash>,
) -> WriterEpochRequest {
    WriterEpochRequest {
        epoch_id: epoch_id_for(label),
        storage_fencing_token: digest(&format!("hardening:fence:{label}")),
        process_identity: digest(&format!("hardening:process:{label}")),
        host_identity: digest("hardening:host:1"),
        started_at_lsn,
        previous_epoch_id,
        previous_epoch_final_commit_digest,
        lease_or_lock_evidence: digest(&format!("hardening:lock:{label}")),
    }
}

fn builder(
    label: &str,
    first_lsn: Lsn,
    authority: WalAppendAuthority,
    transaction_kind: WalTransactionKind,
) -> WalTransactionBuilder {
    builder_on_segment(
        label,
        first_lsn,
        authority,
        transaction_kind,
        WalSegmentId::from_raw(1),
    )
}

fn builder_on_segment(
    label: &str,
    first_lsn: Lsn,
    authority: WalAppendAuthority,
    transaction_kind: WalTransactionKind,
    segment_id: WalSegmentId,
) -> WalTransactionBuilder {
    WalTransactionBuilder::new(
        epoch_id(),
        segment_id,
        transaction_id(&format!("hardening:tx:{label}")),
        transaction_kind,
        authority,
        first_lsn,
        digest("hardening:previous-frame"),
        digest("hardening:previous-commit"),
        WalDurabilityMode::StrictFilesystem,
        PayloadCodecId::from_hash(digest("hardening:codec")),
        PayloadSchemaId::from_hash(digest("hardening:schema")),
        1,
        1,
        digest("hardening:domain"),
    )
}

fn frontier(label: &str, kind: AffectedFrontierKind) -> AffectedFrontier {
    AffectedFrontier {
        kind,
        before_digest: digest(&format!("hardening:frontier:{label}:before")),
        after_digest: digest(&format!("hardening:frontier:{label}:after")),
    }
}

fn submission_acceptance(label: &str) -> SubmissionAcceptanceRecord {
    SubmissionAcceptanceRecord {
        submission_id: digest(&format!("hardening:submission:{label}")),
        canonical_envelope_digest: digest(&format!("hardening:envelope:{label}")),
        idempotency_key_digest: None,
        acceptance_evidence_digest: digest(&format!("hardening:acceptance:{label}")),
    }
}

fn submission_transaction(label: &str, first_lsn: Lsn) -> WalCommittedTransaction {
    submission_transaction_on_segment(label, first_lsn, WalSegmentId::from_raw(1))
}

fn submission_transaction_on_segment(
    label: &str,
    first_lsn: Lsn,
    segment_id: WalSegmentId,
) -> WalCommittedTransaction {
    must_ok(build_submission_acceptance_transaction(
        builder_on_segment(
            label,
            first_lsn,
            WalAppendAuthority::SubmissionIntake,
            WalTransactionKind::SubmissionIntake,
            segment_id,
        ),
        submission_acceptance(label),
        vec![frontier(label, AffectedFrontierKind::SubmissionQueue)],
    ))
}

fn tick_transaction(
    label: &str,
    first_lsn: Lsn,
    decision: WalTickDecision,
) -> WalCommittedTransaction {
    let receipt = TickReceiptRecord {
        submission_id: digest(&format!("hardening:submission:{label}")),
        ticket_digest: digest(&format!("hardening:ticket:{label}")),
        receipt_digest: digest(&format!("hardening:receipt:{label}")),
        decision,
    };
    let correlation = WalReceiptCorrelationRecord {
        submission_id: receipt.submission_id,
        ticket_digest: receipt.ticket_digest,
        receipt_digest: receipt.receipt_digest,
    };
    must_ok(build_tick_transaction(
        builder(
            &format!("tick:{label}"),
            first_lsn,
            WalAppendAuthority::TrustedScheduler,
            WalTransactionKind::SchedulerTick,
        ),
        receipt,
        correlation,
        digest(&format!("hardening:state-delta:{label}")),
        vec![frontier(label, AffectedFrontierKind::RuntimeState)],
    ))
}

fn retained_material(
    label: &str,
    kind: RetainedMaterialKind,
    posture: EvidenceMaterialPosture,
) -> RetainedMaterialRecord {
    RetainedMaterialRecord {
        material_digest: digest(&format!("hardening:material:{label}")),
        semantic_coordinate_digest: digest(&format!("hardening:coordinate:{label}")),
        kind,
        posture,
    }
}

fn reading_ref(label: &str, posture: EvidenceMaterialPosture) -> ReadingRefRecord {
    ReadingRefRecord {
        reading_id: digest(&format!("hardening:reading:{label}")),
        semantic_coordinate_digest: digest(&format!("hardening:coordinate:{label}")),
        payload_digest: digest(&format!("hardening:material:{label}:payload")),
        envelope_digest: digest(&format!("hardening:material:{label}:envelope")),
        posture,
    }
}

fn retention_transaction(
    label: &str,
    material: &[RetainedMaterialRecord],
    first_lsn: Lsn,
) -> WalCommittedTransaction {
    must_ok(build_retained_reading_transaction(
        builder(
            &format!("retention:{label}"),
            first_lsn,
            WalAppendAuthority::TrustedScheduler,
            WalTransactionKind::SchedulerTick,
        ),
        material,
        reading_ref(label, EvidenceMaterialPosture::Present),
        vec![frontier(label, AffectedFrontierKind::ReadingIndex)],
    ))
}

fn materialization_intent(label: &str) -> MaterializationIntentRecord {
    MaterializationIntentRecord {
        effect_id: digest(&format!("hardening:effect:{label}")),
        expected_artifact_digest: digest(&format!("hardening:artifact:{label}")),
        materialization_intent_digest: digest(&format!("hardening:materialization:{label}")),
        idempotency_token: digest(&format!("hardening:idempotency:{label}")),
        target_metadata_digest: digest(&format!("hardening:metadata:{label}")),
    }
}

fn materialization_observation(label: &str) -> MaterializationObservationRecord {
    MaterializationObservationRecord {
        effect_id: digest(&format!("hardening:effect:{label}")),
        observed_artifact_digest: digest(&format!("hardening:artifact:{label}")),
        observed_metadata_digest: digest(&format!("hardening:metadata:{label}")),
    }
}

fn materialization_transaction(
    label: &str,
    first_lsn: Lsn,
    observation: Option<MaterializationObservationRecord>,
) -> WalCommittedTransaction {
    must_ok(build_materialization_outbox_transaction(
        builder(
            &format!("materialization:{label}"),
            first_lsn,
            WalAppendAuthority::TrustedScheduler,
            WalTransactionKind::MaterializationOutbox,
        ),
        materialization_intent(label),
        observation,
        vec![frontier(label, AffectedFrontierKind::ReceiptIndex)],
    ))
}

fn matching_existing_artifact(label: &str) -> ExistingMaterializedArtifact {
    let intent = materialization_intent(label);
    ExistingMaterializedArtifact {
        effect_id: intent.effect_id,
        artifact_digest: intent.expected_artifact_digest,
        metadata_digest: intent.target_metadata_digest,
    }
}

fn replay_state(transactions: &[WalCommittedTransaction]) -> RecoveredState {
    let mut state = RecoveredState::default();
    for transaction in transactions {
        state = must_ok(apply_committed_transaction(state, transaction));
    }
    state
}

fn refresh_commit_digest(transaction: &mut WalCommittedTransaction) {
    transaction.commit.commit_digest = transaction.commit.expected_digest();
}

fn temp_wal_root(label: &str) -> PathBuf {
    let parent = PathBuf::from("target").join("warp-core-test-tmp");
    must_ok(fs::create_dir_all(&parent));
    for _ in 0..1024 {
        let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = parent.join(format!("echo-wal-hardening-{label}-{unique}"));
        match fs::create_dir(&root) {
            Ok(()) => return root,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                must_ok(fs::remove_dir_all(&root));
                match fs::create_dir(&root) {
                    Ok(()) => return root,
                    Err(retry_error) if retry_error.kind() == ErrorKind::AlreadyExists => continue,
                    Err(retry_error) => {
                        panic!("failed to recreate deterministic WAL root {root:?}: {retry_error}")
                    }
                }
            }
            Err(error) => panic!("failed to create deterministic WAL root {root:?}: {error}"),
        }
    }
    panic!("exhausted deterministic WAL root attempts for {label}");
}

struct WalHardeningFixture {
    root: PathBuf,
    store: FilesystemWalStore,
}

impl WalHardeningFixture {
    fn new(label: &str) -> Self {
        let root = temp_wal_root(label);
        let mut store = must_ok(FilesystemWalStore::open(&root, WalSegmentId::from_raw(1)));
        must_ok(store.acquire_writer_epoch(writer_epoch_request()));
        Self { root, store }
    }

    fn segment_path(&self) -> PathBuf {
        self.store.segment_path()
    }

    fn segment_len(&self) -> u64 {
        must_ok(fs::metadata(self.segment_path())).len()
    }

    fn segment_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        must_ok(must_ok(fs::File::open(self.segment_path())).read_to_end(&mut bytes));
        bytes
    }

    fn replace_segment_bytes(&self, bytes: &[u8]) {
        must_ok(fs::write(self.segment_path(), bytes));
    }

    fn truncate_segment(&self, len: u64) {
        must_ok(must_ok(OpenOptions::new().write(true).open(self.segment_path())).set_len(len));
    }

    fn append_submission(&mut self, label: &str, first_lsn: Lsn) -> WalCommittedTransaction {
        let transaction = submission_transaction(label, first_lsn);
        must_ok(self.store.append_transaction(transaction.clone()));
        transaction
    }

    fn append_submission_on_segment(
        &mut self,
        label: &str,
        first_lsn: Lsn,
        segment_id: WalSegmentId,
    ) -> WalCommittedTransaction {
        let transaction = submission_transaction_on_segment(label, first_lsn, segment_id);
        must_ok(self.store.append_transaction(transaction.clone()));
        transaction
    }

    fn append_tick(
        &mut self,
        label: &str,
        first_lsn: Lsn,
        decision: WalTickDecision,
    ) -> WalCommittedTransaction {
        let transaction = tick_transaction(label, first_lsn, decision);
        must_ok(self.store.append_transaction(transaction.clone()));
        transaction
    }

    fn append_uncommitted_submission_frame(&mut self, label: &str, first_lsn: Lsn) {
        let transaction = submission_transaction(label, first_lsn);
        must_ok(
            self.store
                .append_uncommitted_frame(epoch_id(), transaction.frames[0].clone()),
        );
    }

    fn append_uncommitted_tick_frame(&mut self, label: &str, first_lsn: Lsn) {
        let transaction = tick_transaction(label, first_lsn, WalTickDecision::Applied);
        must_ok(
            self.store
                .append_uncommitted_frame(epoch_id(), transaction.frames[0].clone()),
        );
    }

    fn append_uncommitted_outbox_frame(&mut self, label: &str, first_lsn: Lsn) {
        let transaction = materialization_transaction(label, first_lsn, None);
        must_ok(
            self.store
                .append_uncommitted_frame(epoch_id(), transaction.frames[0].clone()),
        );
    }

    fn recover_read_only(
        &self,
    ) -> Result<warp_core::causal_wal::RecoveryScanReport, WalRecoveryError> {
        recover_filesystem_store(&self.root, RecoveryAccessMode::ReadOnly)
    }
}

fn disk_record_digest(kind: u8, payload: &[u8]) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(WAL_DISK_RECORD_DOMAIN);
    h.update(&[kind]);
    h.update(&(payload.len() as u64).to_le_bytes());
    h.update(payload);
    h.finalize().into()
}

fn write_manual_disk_record(path: &Path, kind: u8, payload: &[u8]) {
    let mut file = must_ok(OpenOptions::new().create(true).append(true).open(path));
    must_ok(file.write_all(WAL_SEGMENT_RECORD_MAGIC));
    must_ok(file.write_all(&[kind]));
    must_ok(file.write_all(&(payload.len() as u64).to_le_bytes()));
    must_ok(file.write_all(payload));
    must_ok(file.write_all(&disk_record_digest(kind, payload)));
    must_ok(file.sync_all());
}

fn write_torn_disk_record(path: &Path, kind: u8, declared_len: u64, payload: &[u8]) {
    let mut file = must_ok(OpenOptions::new().write(true).truncate(true).open(path));
    must_ok(file.write_all(WAL_SEGMENT_RECORD_MAGIC));
    must_ok(file.write_all(&[kind]));
    must_ok(file.write_all(&declared_len.to_le_bytes()));
    must_ok(file.write_all(payload));
    must_ok(file.sync_all());
}

fn segment_file_name(segment_id: u64) -> String {
    format!("segment-{segment_id:020}.ecwal")
}

fn duplicate_segment_file_name(segment_id: u64) -> String {
    format!("segment-{segment_id:020}-duplicate.ecwal")
}

fn checkpoint(label: &str, last_lsn: Lsn, last_commit_digest: Hash) -> CheckpointRecord {
    CheckpointRecord {
        checkpoint_id: digest(&format!("hardening:checkpoint:{label}")),
        last_included_lsn: last_lsn,
        last_included_commit_digest: last_commit_digest,
        state_root: digest(&format!("hardening:checkpoint:{label}:state")),
        index_root: digest(&format!("hardening:checkpoint:{label}:index")),
        retained_material_root: digest(&format!("hardening:checkpoint:{label}:retention")),
        schema_version: 1,
        created_from_wal_digest: digest(&format!("hardening:checkpoint:{label}:wal")),
    }
}

#[test]
fn hardening_fixture_recovers_committed_submission() {
    let mut fixture = WalHardeningFixture::new("fixture-committed");
    fixture.append_submission("fixture", Lsn::from_raw(0));

    let report = must_ok(fixture.recover_read_only());
    let index = must_ok(recover_submission_index(&report));

    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(report.transactions.len(), 1);
    assert_eq!(
        index.retry_posture(
            submission_acceptance("fixture").submission_id,
            submission_acceptance("fixture").canonical_envelope_digest,
        ),
        SubmissionRetryPosture::AlreadyAcceptedPending,
        "fixture should recover a committed submission as accepted pending"
    );
}

#[test]
fn hardening_fixture_appends_uncommitted_tail() {
    let mut fixture = WalHardeningFixture::new("fixture-tail");
    fixture.append_submission("fixture-tail", Lsn::from_raw(0));
    fixture.append_uncommitted_submission_frame("tail", Lsn::from_raw(2));

    let report = must_ok(fixture.recover_read_only());

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1)),
        "read-only recovery should report the uncommitted tail without mutating it"
    );
    assert_eq!(report.transactions.len(), 1);
}

#[test]
fn hardening_fixture_read_only_recovery_does_not_mutate_segment() {
    let mut fixture = WalHardeningFixture::new("fixture-read-only");
    fixture.append_submission("read-only", Lsn::from_raw(0));
    fixture.append_uncommitted_submission_frame("read-only-tail", Lsn::from_raw(2));
    let before_len = fixture.segment_len();

    let report = must_ok(fixture.recover_read_only());
    let after_len = fixture.segment_len();

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert_eq!(after_len, before_len, "read-only recovery mutated the WAL");
}

#[test]
fn hardening_fixture_truncates_segment_for_torn_tail() {
    let mut fixture = WalHardeningFixture::new("fixture-torn");
    fixture.append_submission("torn", Lsn::from_raw(0));
    fixture.append_uncommitted_submission_frame("torn-tail", Lsn::from_raw(2));
    fixture.truncate_segment(fixture.segment_len().saturating_sub(11));

    let report = must_ok(fixture.recover_read_only());

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert_eq!(report.transactions.len(), 1);
}

#[test]
fn wal_recovery_golden_empty_store() {
    let fixture = WalHardeningFixture::new("golden-empty");

    let report = must_ok(fixture.recover_read_only());

    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    assert!(report.transactions.is_empty());
    assert_eq!(report.first_committed_lsn(), None);
    assert_eq!(report.last_committed_lsn(), None);
}

#[test]
fn wal_recovery_golden_clean_committed_segment() {
    let mut fixture = WalHardeningFixture::new("golden-clean");
    fixture.append_submission("clean", Lsn::from_raw(0));
    fixture.append_tick("clean", Lsn::from_raw(2), WalTickDecision::Applied);

    let report = must_ok(fixture.recover_read_only());
    let receipts = must_ok(recover_receipt_index(&report));

    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(report.transactions.len(), 2);
    assert_eq!(report.first_committed_lsn(), Some(Lsn::from_raw(0)));
    assert_eq!(report.last_committed_lsn(), Some(Lsn::from_raw(4)));
    assert_eq!(
        receipts
            .receipt_by_submission
            .get(&submission_acceptance("clean").submission_id),
        Some(&digest("hardening:receipt:clean"))
    );
}

#[test]
fn wal_recovery_golden_uncommitted_tail() {
    let mut fixture = WalHardeningFixture::new("golden-tail");
    fixture.append_submission("tail-base", Lsn::from_raw(0));
    fixture.append_uncommitted_tick_frame("tail-extra", Lsn::from_raw(2));

    let report = must_ok(fixture.recover_read_only());

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert_eq!(report.transactions.len(), 1);
}

#[test]
fn wal_recovery_golden_torn_record() {
    let mut fixture = WalHardeningFixture::new("golden-torn");
    fixture.append_submission("torn-base", Lsn::from_raw(0));
    fixture.append_uncommitted_tick_frame("torn-extra", Lsn::from_raw(2));
    fixture.truncate_segment(fixture.segment_len().saturating_sub(3));

    let report = must_ok(fixture.recover_read_only());

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert_eq!(report.transactions.len(), 1);
}

#[test]
fn wal_recovery_golden_corrupt_digest() {
    let mut fixture = WalHardeningFixture::new("golden-corrupt-digest");
    fixture.append_submission("corrupt-digest", Lsn::from_raw(0));
    let mut bytes = fixture.segment_bytes();
    assert!(
        !bytes.is_empty(),
        "fixture should contain at least one disk record"
    );
    let last_index = bytes.len() - 1;
    bytes[last_index] ^= 0x55;
    fixture.replace_segment_bytes(&bytes);

    let error = must_err(
        fixture.recover_read_only(),
        "corrupt disk record digest should block recovery",
    );

    assert!(matches!(
        error,
        WalRecoveryError::Store(WalStoreError::SegmentRecordDigestMismatch)
    ));
}

#[test]
fn wal_recovery_golden_bad_magic() {
    let mut fixture = WalHardeningFixture::new("golden-bad-magic");
    fixture.append_submission("bad-magic", Lsn::from_raw(0));
    let mut bytes = fixture.segment_bytes();
    bytes[0] ^= 0x11;
    fixture.replace_segment_bytes(&bytes);

    let error = must_err(
        fixture.recover_read_only(),
        "bad segment magic should block recovery",
    );

    assert!(matches!(
        error,
        WalRecoveryError::Store(WalStoreError::SegmentRecordDigestMismatch)
    ));
}

#[test]
fn wal_recovery_golden_unknown_record_kind() {
    let fixture = WalHardeningFixture::new("golden-unknown-kind");
    write_manual_disk_record(&fixture.segment_path(), 99, &[]);

    let error = must_err(
        fixture.recover_read_only(),
        "unknown disk record kind should block recovery",
    );

    assert!(matches!(
        error,
        WalRecoveryError::Store(WalStoreError::UnknownDiskRecordKind(99))
    ));
}

#[test]
fn torn_segment_header_reports_tail() {
    let fixture = WalHardeningFixture::new("segment-torn-header");
    fixture.replace_segment_bytes(&WAL_SEGMENT_RECORD_MAGIC[..4]);

    let report = must_ok(fixture.recover_read_only());

    assert_eq!(report.tail_posture, RecoveryTailPosture::WouldTruncateAll);
    assert!(report.transactions.is_empty());
}

#[test]
fn torn_segment_payload_reports_tail() {
    let fixture = WalHardeningFixture::new("segment-torn-payload");
    write_torn_disk_record(&fixture.segment_path(), 1, 64, b"partial-payload");

    let report = must_ok(fixture.recover_read_only());

    assert_eq!(report.tail_posture, RecoveryTailPosture::WouldTruncateAll);
    assert!(report.transactions.is_empty());
}

#[test]
fn torn_segment_digest_reports_tail() {
    let fixture = WalHardeningFixture::new("segment-torn-digest");
    write_torn_disk_record(&fixture.segment_path(), 99, 0, &[]);

    let report = must_ok(fixture.recover_read_only());

    assert_eq!(report.tail_posture, RecoveryTailPosture::WouldTruncateAll);
    assert!(report.transactions.is_empty());
}

#[test]
fn segment_gap_blocks_recovery() {
    let fixture = WalHardeningFixture::new("segment-gap");
    must_ok(fs::write(
        fixture.root.join(segment_file_name(3)),
        b"gap-segment",
    ));

    let error = must_err(
        fixture.recover_read_only(),
        "segment gap should block recovery",
    );

    assert!(matches!(
        error,
        WalRecoveryError::Store(WalStoreError::SegmentGap {
            expected,
            actual
        }) if expected == WalSegmentId::from_raw(2) && actual == WalSegmentId::from_raw(3)
    ));
}

#[test]
fn duplicate_segment_id_blocks_recovery() {
    let fixture = WalHardeningFixture::new("segment-duplicate");
    must_ok(fs::write(
        fixture.root.join(duplicate_segment_file_name(1)),
        b"duplicate-segment",
    ));

    let error = must_err(
        fixture.recover_read_only(),
        "duplicate segment id should block recovery",
    );

    assert!(matches!(
        error,
        WalRecoveryError::Store(WalStoreError::DuplicateSegment(segment_id))
            if segment_id == WalSegmentId::from_raw(1)
    ));
}

#[test]
fn closed_epoch_allows_next_epoch_with_matching_fence_evidence() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let transaction = submission_transaction("epoch-chain", Lsn::from_raw(0));
    let previous_commit_digest = transaction.commit.commit_digest;
    must_ok(store.append_transaction(transaction));
    must_ok(store.close_epoch(epoch_id()));

    let next = writer_epoch_request_for(
        "2",
        Lsn::from_raw(2),
        Some(epoch_id()),
        Some(previous_commit_digest),
    );

    let epoch = must_ok(store.acquire_writer_epoch(next));
    assert_eq!(epoch.epoch_id, epoch_id_for("2"));
    assert_eq!(epoch.previous_epoch_id, Some(epoch_id()));
    assert_eq!(
        epoch.previous_epoch_final_commit_digest,
        Some(previous_commit_digest)
    );
}

#[test]
fn unknown_previous_writer_epoch_rejected() {
    let mut store = InMemoryWalStore::new();

    let error = must_err(
        store.acquire_writer_epoch(writer_epoch_request_for(
            "2",
            Lsn::from_raw(0),
            Some(epoch_id_for("missing")),
            Some(digest("missing-final")),
        )),
        "unknown previous writer epoch should be rejected",
    );

    assert!(matches!(error, WalStoreError::UnknownPreviousWriterEpoch));
}

#[test]
fn writer_epoch_chain_requires_previous_final_commit_digest() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let transaction = submission_transaction("epoch-final", Lsn::from_raw(0));
    must_ok(store.append_transaction(transaction));
    must_ok(store.close_epoch(epoch_id()));

    let error = must_err(
        store.acquire_writer_epoch(writer_epoch_request_for(
            "2",
            Lsn::from_raw(2),
            Some(epoch_id()),
            None,
        )),
        "missing previous final digest should be rejected",
    );

    assert!(matches!(
        error,
        WalStoreError::WriterEpochFinalCommitDigestMismatch
    ));
}

#[test]
fn writer_epoch_fencing_token_mismatch_blocks_recovery() {
    let mut store = InMemoryWalStore::new();
    let first = writer_epoch_request();
    must_ok(store.acquire_writer_epoch(first.clone()));
    let transaction = submission_transaction("epoch-fence", Lsn::from_raw(0));
    let previous_commit_digest = transaction.commit.commit_digest;
    must_ok(store.append_transaction(transaction));
    must_ok(store.close_epoch(epoch_id()));
    let mut next = writer_epoch_request_for(
        "2",
        Lsn::from_raw(2),
        Some(epoch_id()),
        Some(previous_commit_digest),
    );
    next.storage_fencing_token = first.storage_fencing_token;

    let error = must_err(
        store.acquire_writer_epoch(next),
        "reused storage fencing token should be rejected",
    );

    assert!(matches!(error, WalStoreError::WriterEpochFencingMismatch));
}

#[test]
fn writer_epoch_chain_gap_rejected() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.close_epoch(epoch_id()));

    let error = must_err(
        store.acquire_writer_epoch(writer_epoch_request_for("2", Lsn::from_raw(0), None, None)),
        "new epoch should cite the previous closed epoch",
    );

    assert!(matches!(error, WalStoreError::WriterEpochChainGap));
}

#[test]
fn interleaved_transactions_rejected() {
    let first = submission_transaction("interleaved:first", Lsn::from_raw(0));
    let second = submission_transaction("interleaved:second", Lsn::from_raw(1));
    let mut frames = first.frames.clone();
    frames.extend(second.frames.clone());
    let commits = vec![first.commit, second.commit];

    let error = must_err(
        recover_from_frames_and_commits(&frames, &commits, RecoveryAccessMode::ReadOnly),
        "overlapping/interleaved global LSN order should be rejected",
    );

    assert!(matches!(
        error,
        WalRecoveryError::Validation(WalValidationError::LsnContinuityMismatch)
    ));
}

#[test]
fn commit_record_count_mismatch_rejected() {
    let mut transaction = submission_transaction("record-count", Lsn::from_raw(0));
    transaction.commit.record_count += 1;
    refresh_commit_digest(&mut transaction);

    let error = must_err(
        transaction.validate(),
        "record count mismatch should reject",
    );

    assert!(matches!(error, WalValidationError::RecordCountMismatch));
}

#[test]
fn commit_lsn_range_gap_rejected() {
    let mut transaction = submission_transaction("lsn-gap", Lsn::from_raw(0));
    transaction.commit.last_lsn = Lsn::from_raw(2);
    refresh_commit_digest(&mut transaction);

    let error = must_err(transaction.validate(), "LSN range gap should reject");

    assert!(matches!(error, WalValidationError::LastLsnMismatch));
}

#[test]
fn commit_records_root_mismatch_rejected() {
    let mut transaction = submission_transaction("records-root", Lsn::from_raw(0));
    transaction.commit.records_root = digest("hardening:records-root:wrong");
    refresh_commit_digest(&mut transaction);

    let error = must_err(
        transaction.validate(),
        "records root mismatch should reject",
    );

    assert!(matches!(error, WalValidationError::RecordsRootMismatch));
}

#[test]
fn byte_valid_submission_with_tick_record_rejected() {
    let mut transaction = tick_transaction(
        "semantic-submission",
        Lsn::from_raw(0),
        WalTickDecision::Applied,
    );
    transaction.commit.transaction_kind = WalTransactionKind::SubmissionIntake;
    refresh_commit_digest(&mut transaction);

    let error = must_err(
        transaction.validate(),
        "byte-valid tick records cannot masquerade as submission intake",
    );

    assert!(matches!(error, WalValidationError::RecordAuthorityMismatch));
}

#[test]
fn runtime_control_record_without_runtime_authority_rejected() {
    let mut builder = builder(
        "runtime-control-semantic",
        Lsn::from_raw(0),
        WalAppendAuthority::RuntimeControl,
        WalTransactionKind::RuntimePosture,
    );
    must_ok(builder.push_record(
        WalRecordKind::TrustedRuntimeControlRecorded,
        digest("hardening:runtime-control").to_vec(),
    ));
    let mut transaction = must_ok(builder.commit(vec![frontier(
        "runtime-control-semantic",
        AffectedFrontierKind::RuntimeControl,
    )]));
    transaction.commit.transaction_kind = WalTransactionKind::SchedulerTick;
    refresh_commit_digest(&mut transaction);

    let error = must_err(
        transaction.validate(),
        "runtime-control record without runtime authority should reject",
    );

    assert!(matches!(error, WalValidationError::RecordAuthorityMismatch));
}

#[test]
fn frontier_transition_kind_mismatch_rejected() {
    let mut builder = builder(
        "frontier-kind",
        Lsn::from_raw(0),
        WalAppendAuthority::SubmissionIntake,
        WalTransactionKind::SubmissionIntake,
    );
    must_ok(builder.push_record(
        WalRecordKind::SubmissionAcceptedRecorded,
        submission_acceptance("frontier-kind").to_payload_bytes(),
    ));

    let error = must_err(
        builder.commit(vec![frontier(
            "frontier-kind",
            AffectedFrontierKind::RuntimeState,
        )]),
        "submission intake cannot mutate runtime-state frontier",
    );

    assert!(matches!(
        error,
        WalBuildError::Validation(WalValidationError::FrontierTransitionKindMismatch)
    ));
}

#[test]
fn crash_before_submission_commit_recovers_not_accepted() {
    let mut fixture = WalHardeningFixture::new("submission-before-commit");
    fixture.append_uncommitted_submission_frame("before-commit", Lsn::from_raw(0));

    let report = must_ok(fixture.recover_read_only());
    let index = must_ok(recover_submission_index(&report));

    assert_eq!(report.tail_posture, RecoveryTailPosture::WouldTruncateAll);
    assert_eq!(
        index.retry_posture(
            submission_acceptance("before-commit").submission_id,
            submission_acceptance("before-commit").canonical_envelope_digest,
        ),
        SubmissionRetryPosture::NotAccepted
    );
}

#[test]
fn crash_after_submission_commit_before_ack_recovers_pending() {
    let mut fixture = WalHardeningFixture::new("submission-commit-before-ack");
    fixture.append_submission("commit-before-ack", Lsn::from_raw(0));

    let report = must_ok(fixture.recover_read_only());
    let index = must_ok(recover_submission_index(&report));
    let entry = must_some(
        index.get(&submission_acceptance("commit-before-ack").submission_id),
        "accepted submission should recover",
    );

    assert_eq!(entry.posture, RecoveredSubmissionPosture::AcceptedPending);
    assert_eq!(
        index.retry_posture(
            submission_acceptance("commit-before-ack").submission_id,
            submission_acceptance("commit-before-ack").canonical_envelope_digest,
        ),
        SubmissionRetryPosture::AlreadyAcceptedPending
    );
}

#[test]
fn crash_after_submission_commit_before_ack_different_envelope_is_protocol_violation() {
    let mut fixture = WalHardeningFixture::new("submission-conflict");
    fixture.append_submission("conflict", Lsn::from_raw(0));

    let report = must_ok(fixture.recover_read_only());
    let index = must_ok(recover_submission_index(&report));

    assert_eq!(
        index.retry_posture(
            submission_acceptance("conflict").submission_id,
            digest("hardening:envelope:conflict:changed"),
        ),
        SubmissionRetryPosture::ConflictSameIdDifferentEnvelope
    );
}

#[test]
fn new_submission_id_same_envelope_is_not_duplicate_without_policy() {
    let mut fixture = WalHardeningFixture::new("submission-new-id-same-envelope");
    fixture.append_submission("same-envelope", Lsn::from_raw(0));

    let report = must_ok(fixture.recover_read_only());
    let index = must_ok(recover_submission_index(&report));

    assert_eq!(
        index.retry_posture(
            digest("hardening:submission:same-envelope:new-id"),
            submission_acceptance("same-envelope").canonical_envelope_digest,
        ),
        SubmissionRetryPosture::NewSubmissionWithoutPolicyDedupe
    );
}

#[test]
fn crash_after_tick_commit_before_publish_rebuilds_receipt_indexes() {
    let mut fixture = WalHardeningFixture::new("tick-commit-before-publish");
    fixture.append_submission("tick", Lsn::from_raw(0));
    fixture.append_tick("tick", Lsn::from_raw(2), WalTickDecision::Applied);

    let report = must_ok(fixture.recover_read_only());
    let submission_index = must_ok(recover_submission_index(&report));
    let receipt_index = must_ok(recover_receipt_index(&report));

    assert_eq!(
        submission_index.retry_posture(
            submission_acceptance("tick").submission_id,
            submission_acceptance("tick").canonical_envelope_digest,
        ),
        SubmissionRetryPosture::AlreadyDecidedApplied
    );
    assert_eq!(
        receipt_index
            .receipt_by_submission
            .get(&submission_acceptance("tick").submission_id),
        Some(&digest("hardening:receipt:tick"))
    );
}

#[test]
fn uncommitted_tick_tail_does_not_advance_frontier() {
    let mut fixture = WalHardeningFixture::new("tick-uncommitted-tail");
    fixture.append_submission("tick-tail", Lsn::from_raw(0));
    fixture.append_uncommitted_tick_frame("tick-tail", Lsn::from_raw(2));

    let report = must_ok(fixture.recover_read_only());
    let submission_index = must_ok(recover_submission_index(&report));
    let receipt_index = must_ok(recover_receipt_index(&report));

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert_eq!(
        submission_index.retry_posture(
            submission_acceptance("tick-tail").submission_id,
            submission_acceptance("tick-tail").canonical_envelope_digest,
        ),
        SubmissionRetryPosture::AlreadyAcceptedPending
    );
    assert!(receipt_index.receipt_by_submission.is_empty());
}

#[test]
fn crash_before_checkpoint_rename_uses_full_replay() {
    let mut fixture = WalHardeningFixture::new("checkpoint-before-rename");
    fixture.append_submission("checkpoint-before-rename", Lsn::from_raw(0));
    let temp_checkpoint = fixture.root.join(".checkpoint.ecwal.tmp");
    must_ok(fs::write(&temp_checkpoint, b"incomplete-checkpoint-temp"));

    let report = must_ok(fixture.recover_read_only());
    let final_checkpoint = fixture.root.join("checkpoint.ecwal");

    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(report.transactions.len(), 1);
    assert!(!final_checkpoint.exists());
}

#[test]
fn checkpoint_ahead_of_wal_chain_is_rejected() {
    let mut fixture = WalHardeningFixture::new("checkpoint-ahead");
    let transaction = fixture.append_submission("checkpoint-ahead", Lsn::from_raw(0));
    let report = must_ok(fixture.recover_read_only());
    let ahead = checkpoint("ahead", Lsn::from_raw(99), transaction.commit.commit_digest);

    assert_eq!(
        validate_checkpoint_record(&ahead, &report, &[]),
        CheckpointValidationPosture::Invalid
    );
}

#[test]
fn published_checkpoint_missing_material_obstructs() {
    let mut fixture = WalHardeningFixture::new("checkpoint-missing-material");
    fixture.append_submission("checkpoint-missing-material", Lsn::from_raw(0));
    let report = must_ok(fixture.recover_read_only());
    let publication = CheckpointPublicationRecord {
        checkpoint_id: digest("hardening:checkpoint:missing"),
        checkpoint_digest: digest("hardening:checkpoint:missing:digest"),
    };

    assert_eq!(
        evaluate_checkpoint_publication(&publication, None, &report),
        CheckpointValidationPosture::PublishedCheckpointMaterialMissing
    );
}

#[test]
fn corrupt_latest_checkpoint_falls_back_to_prior_valid_checkpoint() {
    let mut fixture = WalHardeningFixture::new("checkpoint-fallback");
    let transaction = fixture.append_submission("checkpoint-fallback", Lsn::from_raw(0));
    let report = must_ok(fixture.recover_read_only());
    let prior = checkpoint(
        "prior",
        transaction.commit.last_lsn,
        transaction.commit.commit_digest,
    );
    let prior_path = fixture.root.join("checkpoint-prior.ecwal");
    let latest_path = fixture.root.join("checkpoint-latest.ecwal");
    must_ok(write_checkpoint_record_atomic(&prior_path, &prior));
    must_ok(fs::write(&latest_path, b"corrupt-latest-checkpoint"));

    let latest_error = must_err(
        read_checkpoint_record(&latest_path),
        "corrupt latest checkpoint should not parse",
    );
    let recovered_prior = must_ok(read_checkpoint_record(&prior_path));

    assert!(matches!(
        latest_error,
        warp_core::causal_wal::WalCheckpointIoError::InvalidMagic
    ));
    assert_eq!(
        validate_checkpoint_record(&recovered_prior, &report, &[]),
        CheckpointValidationPosture::UsableWithoutPublicationRecord
    );
}

#[test]
fn missing_submission_payload_recovers_submission_obstruction() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let material = retained_material(
        "submission-payload",
        RetainedMaterialKind::SubmissionPayload,
        EvidenceMaterialPosture::Present,
    );
    must_ok(store.append_transaction(retention_transaction(
        "submission-payload",
        &[material],
        Lsn::from_raw(0),
    )));
    let report = must_ok(recover_from_frames_and_commits(
        &store.read_frames(),
        &store.read_commits(),
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));
    let obstructions = retained_material_obstructions(&retention, &BTreeSet::new());

    assert_eq!(obstructions.len(), 1);
    assert_eq!(obstructions[0].scope, MissingMaterialScope::Submission);
}

#[test]
fn missing_tick_receipt_material_recovers_receipt_obstruction() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let material = retained_material(
        "tick-receipt",
        RetainedMaterialKind::TickReceipt,
        EvidenceMaterialPosture::Present,
    );
    must_ok(store.append_transaction(retention_transaction(
        "tick-receipt",
        &[material],
        Lsn::from_raw(0),
    )));
    let report = must_ok(recover_from_frames_and_commits(
        &store.read_frames(),
        &store.read_commits(),
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));
    let obstructions = retained_material_obstructions(&retention, &BTreeSet::new());

    assert_eq!(obstructions.len(), 1);
    assert_eq!(obstructions[0].scope, MissingMaterialScope::ReceiptOrTicket);
}

#[test]
fn missing_tick_state_delta_blocks_frontier_recovery() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let material = retained_material(
        "state-delta",
        RetainedMaterialKind::RuntimeStateDelta,
        EvidenceMaterialPosture::Present,
    );
    must_ok(store.append_transaction(retention_transaction(
        "state-delta",
        &[material],
        Lsn::from_raw(0),
    )));
    let report = must_ok(recover_from_frames_and_commits(
        &store.read_frames(),
        &store.read_commits(),
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));
    let obstructions = retained_material_obstructions(&retention, &BTreeSet::new());

    assert_eq!(obstructions.len(), 1);
    assert_eq!(obstructions[0].scope, MissingMaterialScope::RuntimeGlobal);
}

#[test]
fn missing_reading_material_returns_obstruction() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let material = retained_material(
        "reading-payload",
        RetainedMaterialKind::ReadingPayload,
        EvidenceMaterialPosture::Present,
    );
    must_ok(store.append_transaction(retention_transaction(
        "reading-payload",
        &[material],
        Lsn::from_raw(0),
    )));
    let report = must_ok(recover_from_frames_and_commits(
        &store.read_frames(),
        &store.read_commits(),
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));
    let obstructions = retained_material_obstructions(&retention, &BTreeSet::new());

    assert_eq!(obstructions.len(), 1);
    assert_eq!(obstructions[0].scope, MissingMaterialScope::Reading);
}

#[test]
fn missing_diagnostic_material_does_not_block_recovery() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    let material = retained_material(
        "diagnostic",
        RetainedMaterialKind::Diagnostic,
        EvidenceMaterialPosture::Present,
    );
    must_ok(store.append_transaction(retention_transaction(
        "diagnostic",
        &[material],
        Lsn::from_raw(0),
    )));
    let report = must_ok(recover_from_frames_and_commits(
        &store.read_frames(),
        &store.read_commits(),
        RecoveryAccessMode::ReadOnly,
    ));
    let retention = must_ok(recover_retention_index(&report));
    let obstructions = retained_material_obstructions(&retention, &BTreeSet::new());

    assert_eq!(obstructions.len(), 1);
    assert_eq!(obstructions[0].scope, MissingMaterialScope::DiagnosticLoss);
}

#[test]
fn external_effect_requires_committed_outbox_authorization() {
    let mut fixture = WalHardeningFixture::new("outbox-uncommitted");
    fixture.append_uncommitted_outbox_frame("uncommitted", Lsn::from_raw(0));

    let report = must_ok(fixture.recover_read_only());
    let outbox = must_ok(recover_materialization_outbox(&report, &BTreeMap::new()));

    assert_eq!(report.tail_posture, RecoveryTailPosture::WouldTruncateAll);
    assert!(
        outbox.is_empty(),
        "uncommitted outbox intent must not authorize an external effect"
    );
}

#[test]
fn crash_after_effect_before_observation_detects_existing_artifact() {
    let mut fixture = WalHardeningFixture::new("outbox-existing");
    must_ok(
        fixture
            .store
            .append_transaction(materialization_transaction(
                "existing",
                Lsn::from_raw(0),
                None,
            )),
    );
    let report = must_ok(fixture.recover_read_only());
    let intent = materialization_intent("existing");
    let existing = BTreeMap::from([(intent.effect_id, matching_existing_artifact("existing"))]);

    let outbox = must_ok(recover_materialization_outbox(&report, &existing));

    assert_eq!(
        must_some(outbox.get(&intent.effect_id), "outbox entry").posture,
        MaterializationReplayPosture::ExistingArtifactMatches
    );
}

#[test]
fn existing_artifact_digest_mismatch_obstructs() {
    let mut fixture = WalHardeningFixture::new("outbox-mismatch");
    must_ok(
        fixture
            .store
            .append_transaction(materialization_transaction(
                "mismatch",
                Lsn::from_raw(0),
                None,
            )),
    );
    let report = must_ok(fixture.recover_read_only());
    let intent = materialization_intent("mismatch");
    let existing = BTreeMap::from([(
        intent.effect_id,
        ExistingMaterializedArtifact {
            effect_id: intent.effect_id,
            artifact_digest: digest("hardening:artifact:mismatch:wrong"),
            metadata_digest: intent.target_metadata_digest,
        },
    )]);

    let outbox = must_ok(recover_materialization_outbox(&report, &existing));

    assert_eq!(
        must_some(outbox.get(&intent.effect_id), "outbox entry").posture,
        MaterializationReplayPosture::Obstructed
    );
}

#[test]
fn materialization_observation_marks_effect_already_observed() {
    let mut fixture = WalHardeningFixture::new("outbox-observed");
    must_ok(
        fixture
            .store
            .append_transaction(materialization_transaction(
                "observed",
                Lsn::from_raw(0),
                Some(materialization_observation("observed")),
            )),
    );
    let report = must_ok(fixture.recover_read_only());
    let intent = materialization_intent("observed");

    let outbox = must_ok(recover_materialization_outbox(&report, &BTreeMap::new()));
    let entry = must_some(outbox.get(&intent.effect_id), "outbox entry");

    assert_eq!(entry.posture, MaterializationReplayPosture::AlreadyObserved);
    assert_eq!(
        entry.observation,
        Some(materialization_observation("observed"))
    );
}

#[test]
fn outbox_replay_uses_idempotency_token() {
    let mut fixture = WalHardeningFixture::new("outbox-idempotency");
    must_ok(
        fixture
            .store
            .append_transaction(materialization_transaction(
                "idempotency",
                Lsn::from_raw(0),
                None,
            )),
    );
    let report = must_ok(fixture.recover_read_only());
    let intent = materialization_intent("idempotency");
    let existing = BTreeMap::from([(intent.effect_id, matching_existing_artifact("idempotency"))]);

    let outbox = must_ok(recover_materialization_outbox(&report, &existing));
    let entry = must_some(outbox.get(&intent.effect_id), "outbox entry");

    assert_eq!(entry.intent.idempotency_token, intent.idempotency_token);
    assert_eq!(
        entry.posture,
        MaterializationReplayPosture::ExistingArtifactMatches
    );
}

#[test]
fn pure_replay_same_transactions_same_roots() {
    let transactions = vec![
        submission_transaction("pure-replay", Lsn::from_raw(0)),
        tick_transaction("pure-replay", Lsn::from_raw(2), WalTickDecision::Applied),
    ];

    let first = replay_state(&transactions);
    let second = replay_state(&transactions);

    assert_eq!(first, second);
}

#[test]
fn pure_replay_order_is_commit_chain_order() {
    let submission = submission_transaction("order-submission", Lsn::from_raw(0));
    let tick = tick_transaction("order-tick", Lsn::from_raw(2), WalTickDecision::Applied);

    let forward = replay_state(&[submission.clone(), tick.clone()]);
    let reversed = replay_state(&[tick.clone(), submission.clone()]);

    assert_eq!(
        forward.applied_transactions,
        vec![submission.commit.transaction_id, tick.commit.transaction_id,]
    );
    assert_eq!(
        reversed.applied_transactions,
        vec![tick.commit.transaction_id, submission.commit.transaction_id,]
    );
    assert_ne!(forward.applied_transactions, reversed.applied_transactions);
}

#[test]
fn pure_replay_rejects_frontier_mismatch() {
    let first = submission_transaction("frontier-a", Lsn::from_raw(0));
    let second = submission_transaction("frontier-b", Lsn::from_raw(2));
    let state = must_ok(apply_committed_transaction(
        RecoveredState::default(),
        &first,
    ));

    let error = must_err(
        apply_committed_transaction(state, &second),
        "frontier mismatch should reject",
    );

    assert!(matches!(
        error,
        WalReplayError::FrontierMismatch {
            kind: AffectedFrontierKind::SubmissionQueue,
            ..
        }
    ));
}

#[test]
fn recovery_reducer_does_not_require_scheduler() {
    fn reducer_signature(
        reducer: fn(
            RecoveredState,
            &WalCommittedTransaction,
        ) -> Result<RecoveredState, WalReplayError>,
    ) -> fn(RecoveredState, &WalCommittedTransaction) -> Result<RecoveredState, WalReplayError>
    {
        reducer
    }

    let reducer = reducer_signature(apply_committed_transaction);
    let transaction = tick_transaction("no-scheduler", Lsn::from_raw(0), WalTickDecision::Applied);
    let state = must_ok(reducer(RecoveredState::default(), &transaction));

    assert_eq!(state.applied_transactions.len(), 1);
}

#[test]
fn recovery_reducer_does_not_require_app_callbacks() {
    let transaction = retention_transaction(
        "no-app-callbacks",
        &[retained_material(
            "no-app-callbacks",
            RetainedMaterialKind::ReadingPayload,
            EvidenceMaterialPosture::Present,
        )],
        Lsn::from_raw(0),
    );

    let state = must_ok(apply_committed_transaction(
        RecoveredState::default(),
        &transaction,
    ));

    assert_eq!(
        state.applied_transactions,
        vec![transaction.commit.transaction_id]
    );
}

#[test]
fn shadow_replay_submission_path_matches_live() {
    let transaction = submission_transaction("shadow-submission", Lsn::from_raw(0));
    let live_state = replay_state(std::slice::from_ref(&transaction));

    assert!(must_ok(shadow_replay_matches(&live_state, &[transaction])));
}

#[test]
fn shadow_replay_tick_path_matches_live() {
    let transaction = tick_transaction("shadow-tick", Lsn::from_raw(0), WalTickDecision::Applied);
    let live_state = replay_state(std::slice::from_ref(&transaction));

    assert!(must_ok(shadow_replay_matches(&live_state, &[transaction])));
}

#[test]
fn shadow_replay_retention_path_matches_live() {
    let transaction = retention_transaction(
        "shadow-retention",
        &[retained_material(
            "shadow-retention",
            RetainedMaterialKind::ReadingPayload,
            EvidenceMaterialPosture::Present,
        )],
        Lsn::from_raw(0),
    );
    let live_state = replay_state(std::slice::from_ref(&transaction));

    assert!(must_ok(shadow_replay_matches(&live_state, &[transaction])));
}

#[test]
fn shadow_replay_outbox_path_matches_live() {
    let transaction = materialization_transaction("shadow-outbox", Lsn::from_raw(0), None);
    let live_state = replay_state(std::slice::from_ref(&transaction));

    assert!(must_ok(shadow_replay_matches(&live_state, &[transaction])));
}

#[test]
fn shadow_replay_reports_first_mismatch() {
    let transaction = submission_transaction("shadow-mismatch", Lsn::from_raw(0));

    let report = must_ok(shadow_replay_report(
        &RecoveredState::default(),
        &[transaction],
    ));

    assert!(!report.matches);
    assert!(matches!(
        report.first_mismatch,
        Some(ShadowReplayMismatch::AppliedTransactionCount {
            live: 0,
            replayed: 1
        })
    ));
}

#[test]
fn commit_evidence_projects_accepted_pending() {
    let mut fixture = WalHardeningFixture::new("evidence-accepted");
    let transaction = fixture.append_submission("evidence-accepted", Lsn::from_raw(0));
    let report = must_ok(fixture.recover_read_only());

    let evidence = project_causal_commit_evidence(&report);

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].posture, CausalCommitEvidencePosture::Present);
    assert_eq!(evidence[0].source, CausalCommitEvidenceSource::EchoWal);
    assert_eq!(
        evidence[0].durability_mode,
        WalDurabilityMode::StrictFilesystem
    );
    assert_eq!(
        evidence[0].transaction_id,
        transaction.commit.transaction_id
    );
    assert_eq!(evidence[0].lsn, transaction.commit.last_lsn);
}

#[test]
fn commit_evidence_projects_decided_applied() {
    let mut fixture = WalHardeningFixture::new("evidence-applied");
    fixture.append_submission("evidence-applied", Lsn::from_raw(0));
    let tick = fixture.append_tick(
        "evidence-applied",
        Lsn::from_raw(2),
        WalTickDecision::Applied,
    );
    let report = must_ok(fixture.recover_read_only());
    let submission_index = must_ok(recover_submission_index(&report));

    let evidence = project_causal_commit_evidence(&report);

    assert_eq!(
        must_some(
            submission_index.get(&submission_acceptance("evidence-applied").submission_id),
            "submission should recover",
        )
        .posture,
        RecoveredSubmissionPosture::DecidedApplied
    );
    assert_eq!(
        must_some(evidence.last(), "tick evidence").transaction_id,
        tick.commit.transaction_id
    );
}

#[test]
fn commit_evidence_projects_decided_rejected() {
    let mut fixture = WalHardeningFixture::new("evidence-rejected");
    fixture.append_submission("evidence-rejected", Lsn::from_raw(0));
    let tick = fixture.append_tick(
        "evidence-rejected",
        Lsn::from_raw(2),
        WalTickDecision::RejectedFootprintConflict,
    );
    let report = must_ok(fixture.recover_read_only());
    let submission_index = must_ok(recover_submission_index(&report));

    let evidence = project_causal_commit_evidence(&report);

    assert_eq!(
        must_some(
            submission_index.get(&submission_acceptance("evidence-rejected").submission_id),
            "submission should recover",
        )
        .posture,
        RecoveredSubmissionPosture::DecidedRejected
    );
    assert_eq!(
        must_some(evidence.last(), "tick evidence").commit_digest,
        tick.commit.commit_digest
    );
}

#[test]
fn commit_evidence_projects_obstructed() {
    let transaction = tick_transaction(
        "evidence-obstructed",
        Lsn::from_raw(0),
        WalTickDecision::Obstructed,
    );
    let obstruction_digest = digest("hardening:evidence:obstruction");

    let evidence =
        project_obstructed_causal_commit_evidence(&transaction.commit, obstruction_digest);

    assert_eq!(evidence.posture, CausalCommitEvidencePosture::Obstructed);
    assert_eq!(evidence.obstruction_digest, Some(obstruction_digest));
    assert_eq!(evidence.commit_digest, transaction.commit.commit_digest);
}

#[test]
fn commit_evidence_absent_is_explicit() {
    let evidence = project_absent_causal_commit_evidence(
        digest("hardening:evidence:absent"),
        WalDurabilityMode::Disabled,
    );

    assert_eq!(evidence.posture, CausalCommitEvidencePosture::Absent);
    assert_eq!(evidence.durability_mode, WalDurabilityMode::Disabled);
    assert_eq!(evidence.commit_digest, [0; 32]);
}

#[test]
fn wal_doctor_clean_report_is_stable() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(submission_transaction("doctor-clean", Lsn::from_raw(0))));

    let report = must_ok(doctor_in_memory_store(&store));

    assert_eq!(report.posture, WalDoctorPosture::Recoverable);
    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(
        WalDoctorReport::stable_field_names(),
        &[
            "posture",
            "tail_posture",
            "committed_transactions_replayed",
            "obstruction_count",
            "recovered_frontier_root",
            "recovered_indexes_root",
        ]
    );
}

#[test]
fn wal_doctor_would_truncate_does_not_mutate() {
    let mut fixture = WalHardeningFixture::new("doctor-would-truncate");
    fixture.append_submission("doctor-would-truncate", Lsn::from_raw(0));
    fixture.append_uncommitted_tick_frame("doctor-would-truncate-tail", Lsn::from_raw(2));
    let before = fixture.segment_bytes();

    let report = must_ok(doctor_filesystem_store(&fixture.root));
    let after = fixture.segment_bytes();

    assert_eq!(
        report.posture,
        WalDoctorPosture::RecoverableWithUncommittedTail
    );
    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(1))
    );
    assert_eq!(
        after, before,
        "doctor must not mutate files in read-only mode"
    );
}

#[test]
fn wal_doctor_corrupt_committed_record_reports_obstructed() {
    let mut fixture = WalHardeningFixture::new("doctor-corrupt");
    fixture.append_submission("doctor-corrupt", Lsn::from_raw(0));
    let mut bytes = fixture.segment_bytes();
    let last_index = bytes.len() - 1;
    bytes[last_index] ^= 0x77;
    fixture.replace_segment_bytes(&bytes);

    let report = must_ok(doctor_filesystem_store(&fixture.root));

    assert_eq!(report.posture, WalDoctorPosture::Obstructed);
    assert_eq!(report.recovery_certificate.obstruction_count, 1);
}

#[test]
fn wal_doctor_missing_material_reports_obstruction() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(retention_transaction(
        "doctor-missing-material",
        &[retained_material(
            "doctor-missing-material",
            RetainedMaterialKind::ReadingPayload,
            EvidenceMaterialPosture::Present,
        )],
        Lsn::from_raw(0),
    )));

    let report = must_ok(doctor_in_memory_store_with_materials(
        &store,
        &BTreeSet::new(),
    ));

    assert_eq!(report.posture, WalDoctorPosture::Obstructed);
    assert_eq!(report.recovery_certificate.obstruction_count, 1);
}

#[test]
fn recovery_certificate_has_stable_json_shape() {
    let mut store = InMemoryWalStore::new();
    must_ok(store.acquire_writer_epoch(writer_epoch_request()));
    must_ok(store.append_transaction(submission_transaction(
        "doctor-certificate",
        Lsn::from_raw(0),
    )));

    let report = must_ok(doctor_in_memory_store(&store));
    let certificate = report.recovery_certificate;

    assert_eq!(certificate.first_lsn, Some(Lsn::from_raw(0)));
    assert_eq!(certificate.last_lsn, Some(Lsn::from_raw(1)));
    assert_eq!(certificate.committed_transactions_replayed, 1);
    assert_eq!(certificate.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(certificate.obstruction_count, 0);
    assert_eq!(WalDoctorReport::stable_field_names().len(), 6);
}

#[test]
fn crashpoint_manifest_lists_submission_boundaries() {
    let manifest = wal_crashpoint_manifest();

    assert!(manifest.iter().any(|entry| {
        entry.name == "submission.before_commit"
            && entry.boundary == WalCrashpointBoundary::Submission
            && entry.execution == WalCrashpointExecution::SimulatedInProcess
    }));
    assert!(manifest.iter().any(|entry| {
        entry.name == "submission.after_commit_before_ack"
            && entry.boundary == WalCrashpointBoundary::Submission
            && entry.execution == WalCrashpointExecution::SimulatedInProcess
    }));
}

#[test]
fn crashpoint_manifest_lists_tick_boundaries() {
    let manifest = wal_crashpoint_manifest();

    assert!(manifest.iter().any(|entry| {
        entry.name == "tick.before_commit"
            && entry.boundary == WalCrashpointBoundary::Tick
            && entry.execution == WalCrashpointExecution::SimulatedInProcess
    }));
    assert!(manifest.iter().any(|entry| {
        entry.name == "tick.after_commit_before_publish"
            && entry.boundary == WalCrashpointBoundary::Tick
            && entry.execution == WalCrashpointExecution::SimulatedInProcess
    }));
}

#[test]
fn crashpoint_manifest_lists_checkpoint_boundaries() {
    let manifest = wal_crashpoint_manifest();

    assert!(manifest.iter().any(|entry| {
        entry.name == "checkpoint.before_rename"
            && entry.boundary == WalCrashpointBoundary::Checkpoint
            && entry.execution == WalCrashpointExecution::SimulatedInProcess
    }));
    assert!(manifest.iter().any(|entry| {
        entry.name == "checkpoint.after_rename_before_publication"
            && entry.boundary == WalCrashpointBoundary::Checkpoint
            && entry.execution == WalCrashpointExecution::SimulatedInProcess
    }));
}

#[test]
fn crashpoint_manifest_marks_process_kill_as_future_until_runner_exists() {
    let process_entries = wal_crashpoint_manifest()
        .iter()
        .filter(|entry| entry.boundary == WalCrashpointBoundary::Process)
        .collect::<Vec<_>>();

    assert!(!process_entries.is_empty());
    assert!(process_entries
        .iter()
        .all(|entry| entry.execution == WalCrashpointExecution::ProcessKillFuture));
}

#[test]
fn filesystem_commit_flush_is_ack_boundary() {
    let mut fixture = WalHardeningFixture::new("sync-commit");
    let transaction = fixture.append_submission("sync-commit", Lsn::from_raw(0));

    assert!(fixture.store.sync_evidence().iter().any(|entry| {
        entry.boundary == FilesystemSyncBoundary::CommitFileSynced
            && entry.transaction_id == Some(transaction.commit.transaction_id)
    }));
}

#[test]
fn filesystem_segment_creation_syncs_directory() {
    let fixture = WalHardeningFixture::new("sync-segment");

    must_ok(validate_filesystem_strict_sync_evidence(
        fixture.store.sync_evidence(),
        &[
            FilesystemSyncBoundary::SegmentNamespaceDirectorySynced,
            FilesystemSyncBoundary::SegmentFileCreated,
            FilesystemSyncBoundary::SegmentDirectorySynced,
        ],
    ));
}

#[test]
fn filesystem_segment_namespace_creation_syncs_root_directory() {
    let fixture = WalHardeningFixture::new("sync-segment-namespace");

    assert!(fixture.store.sync_evidence().iter().any(|entry| {
        entry.boundary == FilesystemSyncBoundary::SegmentNamespaceDirectorySynced
            && entry.segment_id.is_none()
            && entry.transaction_id.is_none()
    }));
}

#[test]
fn filesystem_manifest_rename_syncs_directory() {
    let mut fixture = WalHardeningFixture::new("sync-manifest");
    let manifest = WalManifest {
        manifest_digest: digest("hardening:manifest:sync"),
        last_committed_lsn: None,
        last_commit_digest: None,
        sealed_segment_count: 0,
    };

    must_ok(fixture.store.publish_manifest(epoch_id(), manifest));

    must_ok(validate_filesystem_strict_sync_evidence(
        fixture.store.sync_evidence(),
        &[
            FilesystemSyncBoundary::ManifestTempFileSynced,
            FilesystemSyncBoundary::ManifestRenamedDirectorySynced,
        ],
    ));
}

#[test]
fn filesystem_checkpoint_rename_syncs_directory() {
    let root = temp_wal_root("sync-checkpoint");
    let checkpoint = checkpoint(
        "sync-checkpoint",
        Lsn::from_raw(1),
        digest("hardening:checkpoint:sync:commit"),
    );

    let evidence = must_ok(write_checkpoint_record_atomic_with_evidence(
        root.join("checkpoint.ecwal"),
        &checkpoint,
    ));

    must_ok(validate_filesystem_strict_sync_evidence(
        &evidence,
        &[
            FilesystemSyncBoundary::CheckpointTempFileSynced,
            FilesystemSyncBoundary::CheckpointRenamedDirectorySynced,
        ],
    ));
}

#[test]
fn filesystem_strict_mode_rejects_missing_sync_evidence() {
    let error = must_err(
        validate_filesystem_strict_sync_evidence(&[], &[FilesystemSyncBoundary::CommitFileSynced]),
        "missing sync evidence should block strict filesystem claim",
    );

    assert!(matches!(
        error,
        FilesystemSyncEvidenceError::Missing(FilesystemSyncBoundary::CommitFileSynced)
    ));
}

#[test]
fn strict_object_store_requires_content_addressed_objects() {
    let error = must_err(
        validate_strict_object_store_capabilities(ObjectStoreWalCapabilities {
            content_addressed_object_write: false,
            verify_object_version: true,
            conditional_manifest_commit: true,
            read_after_write: ObjectStoreReadAfterWritePosture::Verified,
        }),
        "strict object store needs content-addressed writes",
    );

    assert_eq!(
        error,
        ObjectStoreCapabilityError::MissingContentAddressedObjectWrite
    );
}

#[test]
fn strict_object_store_requires_object_version_verification() {
    let error = must_err(
        validate_strict_object_store_capabilities(ObjectStoreWalCapabilities {
            content_addressed_object_write: true,
            verify_object_version: false,
            conditional_manifest_commit: true,
            read_after_write: ObjectStoreReadAfterWritePosture::Verified,
        }),
        "strict object store needs version evidence",
    );

    assert_eq!(
        error,
        ObjectStoreCapabilityError::MissingObjectVersionVerification
    );
}

#[test]
fn strict_object_store_requires_conditional_manifest_commit_negative() {
    let error = must_err(
        validate_strict_object_store_capabilities(ObjectStoreWalCapabilities {
            content_addressed_object_write: true,
            verify_object_version: true,
            conditional_manifest_commit: false,
            read_after_write: ObjectStoreReadAfterWritePosture::Verified,
        }),
        "strict object store needs conditional manifest commit",
    );

    assert_eq!(
        error,
        ObjectStoreCapabilityError::MissingConditionalManifestCommit
    );
}

#[test]
fn strict_object_store_requires_verified_read_after_write() {
    let error = must_err(
        validate_strict_object_store_capabilities(ObjectStoreWalCapabilities {
            content_addressed_object_write: true,
            verify_object_version: true,
            conditional_manifest_commit: true,
            read_after_write: ObjectStoreReadAfterWritePosture::Unverified,
        }),
        "strict object store needs verified read-after-write",
    );

    assert_eq!(error, ObjectStoreCapabilityError::MissingReadAfterWrite);
}

#[test]
fn strict_object_store_rejects_unconditional_manifest_overwrite() {
    let error = must_err(
        validate_strict_object_store_manifest_commit(ObjectStoreManifestCommitShape {
            mode: ObjectStoreManifestCommitMode::UnconditionalOverwrite,
            expected_previous_manifest_digest: None,
            new_manifest_digest: digest("hardening:object-store:manifest"),
        }),
        "strict object store cannot overwrite manifest unconditionally",
    );

    assert_eq!(
        error,
        ObjectStoreCapabilityError::UnconditionalManifestOverwrite
    );
}

#[test]
fn redacted_material_is_policy_posture_not_missing() {
    assert_eq!(
        inspect_evidence_material_posture(EvidenceMaterialPosture::RedactedByPolicy),
        MaterialInspectionPosture::PolicyHidden
    );
}

#[test]
fn encrypted_key_unavailable_is_policy_posture_not_corruption() {
    assert_eq!(
        inspect_evidence_material_posture(EvidenceMaterialPosture::EncryptedKeyUnavailable),
        MaterialInspectionPosture::EncryptedKeyUnavailable
    );
}

#[test]
fn missing_material_is_not_redaction() {
    assert_eq!(
        inspect_evidence_material_posture(EvidenceMaterialPosture::Missing),
        MaterialInspectionPosture::Missing
    );
}

#[test]
fn corrupt_material_is_not_redaction() {
    assert_eq!(
        inspect_evidence_material_posture(EvidenceMaterialPosture::Corrupt),
        MaterialInspectionPosture::Corrupt
    );
}

#[test]
fn inspector_reports_redaction_posture_explicitly() {
    let postures = [
        (
            EvidenceMaterialPosture::Present,
            MaterialInspectionPosture::Present,
        ),
        (
            EvidenceMaterialPosture::RedactedByPolicy,
            MaterialInspectionPosture::PolicyHidden,
        ),
        (
            EvidenceMaterialPosture::EncryptedKeyUnavailable,
            MaterialInspectionPosture::EncryptedKeyUnavailable,
        ),
        (
            EvidenceMaterialPosture::Missing,
            MaterialInspectionPosture::Missing,
        ),
        (
            EvidenceMaterialPosture::Corrupt,
            MaterialInspectionPosture::Corrupt,
        ),
        (
            EvidenceMaterialPosture::Obstructed,
            MaterialInspectionPosture::Obstructed,
        ),
    ];

    for (source, expected) in postures {
        assert_eq!(inspect_evidence_material_posture(source), expected);
    }
}

#[test]
fn wal_hardening_gate_reports_blocked_categories() {
    let report = audit_wal_release_readiness(WalReleaseReadinessGates {
        filesystem_adapter: true,
        object_store_capability_gate: true,
        segment_repair: true,
        ..WalReleaseReadinessGates::default()
    });

    assert!(!report.ready);
    assert!(report.blocked_gates.contains(&"crashpoint_manifest"));
    assert!(report.blocked_gates.contains(&"wal_doctor"));
    assert!(report.blocked_gates.contains(&"app_noun_guard"));
}

#[test]
fn wal_hardening_gate_passes_when_all_categories_are_green() {
    let report = audit_wal_release_readiness(WalReleaseReadinessGates {
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

    assert!(report.ready);
    assert!(report.blocked_gates.is_empty());
}

#[test]
fn canonical_segment_path_uses_logical_segments_directory() {
    assert_eq!(
        canonical_segment_relative_path(WalSegmentId::from_raw(1)),
        PathBuf::from("segments").join("segment-00000000000000000001.ecwal")
    );
}

#[test]
fn wall_clock_segment_placement_cannot_be_authoritative() {
    let error = must_err(
        validate_segment_placement_policy(WalSegmentPlacementPolicy {
            kind: WalSegmentPlacementKind::WallClockPartition,
            authoritative: true,
        }),
        "wall-clock placement must not be authoritative",
    );

    assert_eq!(
        error,
        WalSegmentPlacementPolicyError::WallClockPlacementCannotBeAuthoritative
    );
}

#[test]
fn wall_clock_segment_placement_may_be_non_authoritative() {
    must_ok(validate_segment_placement_policy(
        WalSegmentPlacementPolicy {
            kind: WalSegmentPlacementKind::WallClockPartition,
            authoritative: false,
        },
    ));
}

#[test]
fn recovery_scans_canonical_segments_directory() {
    let mut fixture = WalHardeningFixture::new("layout-canonical-scan");
    fixture.append_submission("layout-canonical-scan", Lsn::from_raw(0));

    let report = must_ok(fixture.recover_read_only());

    assert_eq!(report.transactions.len(), 1);
    assert!(fixture
        .store
        .segment_path()
        .starts_with(fixture.root.join("segments")));
}

#[test]
fn legacy_flat_segment_scan_remains_readable() {
    let mut fixture = WalHardeningFixture::new("layout-legacy-source");
    fixture.append_submission("layout-legacy-source", Lsn::from_raw(0));
    let segment_bytes = fixture.segment_bytes();
    let legacy_root = temp_wal_root("layout-legacy-target");
    must_ok(fs::write(
        legacy_root.join(segment_file_name(1)),
        segment_bytes,
    ));

    let report = must_ok(recover_filesystem_store(
        &legacy_root,
        RecoveryAccessMode::ReadOnly,
    ));

    assert_eq!(report.transactions.len(), 1);
}

#[test]
fn segment_gap_in_canonical_directory_blocks_recovery() {
    let fixture = WalHardeningFixture::new("layout-gap");
    must_ok(fs::write(
        fixture.root.join("segments").join(segment_file_name(3)),
        b"gap-segment",
    ));

    let error = must_err(
        fixture.recover_read_only(),
        "canonical segment gap should block recovery",
    );

    assert!(matches!(
        error,
        WalRecoveryError::Store(WalStoreError::SegmentGap {
            expected,
            actual
        }) if expected == WalSegmentId::from_raw(2) && actual == WalSegmentId::from_raw(3)
    ));
}

#[test]
fn duplicate_segment_id_across_layouts_blocks_recovery() {
    let fixture = WalHardeningFixture::new("layout-duplicate");
    must_ok(fs::write(
        fixture.root.join(segment_file_name(1)),
        b"legacy-duplicate",
    ));

    let error = must_err(
        fixture.recover_read_only(),
        "duplicate segment id across canonical and legacy layout should block recovery",
    );

    assert!(matches!(
        error,
        WalRecoveryError::Store(WalStoreError::DuplicateSegment(segment_id))
            if segment_id == WalSegmentId::from_raw(1)
    ));
}

#[test]
fn writable_recovery_rewrite_preserves_canonical_segments_directory() {
    let mut fixture = WalHardeningFixture::new("layout-rewrite");
    fixture.append_submission("layout-rewrite", Lsn::from_raw(0));
    fixture.append_uncommitted_tick_frame("layout-rewrite-tail", Lsn::from_raw(2));

    let report = must_ok(recover_filesystem_store(
        &fixture.root,
        RecoveryAccessMode::Writable,
    ));

    assert_eq!(
        report.tail_posture,
        RecoveryTailPosture::TruncatedAfter(Lsn::from_raw(1))
    );
    assert!(fixture
        .root
        .join("segments")
        .join(segment_file_name(1))
        .exists());
}

#[test]
fn next_segment_id_overflow_blocks_rotation() {
    let error = must_err(
        next_segment_id(Some(WalSegmentId::from_raw(u64::MAX))),
        "segment id overflow should block rotation",
    );

    assert_eq!(error, WalSegmentIdError::Overflow);
}

#[test]
fn segment_manifest_entry_binds_logical_id_not_wall_clock_path() {
    let transaction = submission_transaction("layout-manifest-entry", Lsn::from_raw(0));
    let entry = segment_manifest_entry(WalSegmentId::from_raw(1), &transaction.frames);

    assert_eq!(entry.segment_id, WalSegmentId::from_raw(1));
    assert_eq!(
        entry.relative_path,
        PathBuf::from("segments").join(segment_file_name(1))
    );
    assert!(!entry.relative_path.to_string_lossy().contains("2026/"));
    assert_eq!(entry.first_lsn, Some(Lsn::from_raw(0)));
    assert_eq!(entry.last_lsn, Some(Lsn::from_raw(1)));
}

#[test]
fn segment_layout_gate_is_part_of_wal_release_readiness() {
    let report = audit_wal_release_readiness(WalReleaseReadinessGates {
        filesystem_adapter: true,
        object_store_capability_gate: true,
        segment_repair: true,
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
        ..WalReleaseReadinessGates::default()
    });

    assert!(
        report.blocked_gates.contains(&"segment_layout_policy"),
        "layout policy should be an explicit WAL release gate"
    );
}

#[test]
fn filesystem_append_frame_rejects_inactive_segment_id() {
    let mut fixture = WalHardeningFixture::new("rotation-segment-mismatch");
    fixture.append_submission("rotation-segment-mismatch", Lsn::from_raw(0));
    must_ok(fixture.store.rotate_segment(epoch_id()));

    let error = must_err(
        fixture
            .store
            .append_transaction(submission_transaction("wrong-segment", Lsn::from_raw(2))),
        "active segment 2 should reject frame declaring segment 1",
    );

    assert!(matches!(
        error,
        WalStoreError::SegmentMismatch { expected, actual }
            if expected == WalSegmentId::from_raw(2) && actual == WalSegmentId::from_raw(1)
    ));
}

#[test]
fn filesystem_rotate_segment_creates_next_canonical_segment() {
    let mut fixture = WalHardeningFixture::new("rotation-next-segment");
    fixture.append_submission("rotation-next-segment", Lsn::from_raw(0));

    let seal = must_ok(fixture.store.rotate_segment(epoch_id()));

    assert_eq!(seal.segment_id, WalSegmentId::from_raw(1));
    assert_eq!(seal.sealed_lsn, Some(Lsn::from_raw(1)));
    assert_eq!(fixture.store.active_segment_id(), WalSegmentId::from_raw(2));
    assert!(fixture
        .root
        .join("segments")
        .join(segment_file_name(2))
        .exists());
    assert!(fixture.store.sync_evidence().iter().any(|entry| {
        entry.boundary == FilesystemSyncBoundary::SegmentDirectorySynced
            && entry.segment_id == Some(WalSegmentId::from_raw(2))
    }));
}

#[test]
fn filesystem_rotate_segment_does_not_overwrite_existing_next_segment() {
    let mut fixture = WalHardeningFixture::new("rotation-existing-next-segment");
    fixture.append_submission("rotation-existing-next-segment", Lsn::from_raw(0));
    let next_path = fixture.root.join("segments").join(segment_file_name(2));
    must_ok(fs::write(&next_path, b"existing-segment-material"));

    let error = must_err(
        fixture.store.rotate_segment(epoch_id()),
        "rotation should not overwrite an existing next segment",
    );

    assert_eq!(
        error,
        WalStoreError::DuplicateSegment(WalSegmentId::from_raw(2))
    );
    assert_eq!(
        must_ok(fs::read(next_path)),
        b"existing-segment-material",
        "existing next segment material should not be truncated"
    );
}

#[test]
fn filesystem_rotate_segment_rejects_uncommitted_tail() {
    let mut fixture = WalHardeningFixture::new("rotation-uncommitted-tail");
    fixture.append_uncommitted_submission_frame("rotation-uncommitted-tail", Lsn::from_raw(0));

    let error = must_err(
        fixture.store.rotate_segment(epoch_id()),
        "rotation should not seal a segment with uncommitted frames",
    );

    assert_eq!(
        error,
        WalStoreError::SegmentHasUncommittedTail(WalSegmentId::from_raw(1))
    );
}

#[test]
fn filesystem_rotate_segment_rejects_epoch_mismatch() {
    let mut fixture = WalHardeningFixture::new("rotation-epoch-mismatch");

    let error = must_err(
        fixture.store.rotate_segment(epoch_id_for("wrong")),
        "rotation should require the active writer epoch",
    );

    assert_eq!(error, WalStoreError::WriterEpochMismatch);
}

#[test]
fn filesystem_recovery_reads_transactions_across_rotated_segments() {
    let mut fixture = WalHardeningFixture::new("rotation-recover-multi");
    fixture.append_submission("rotation-recover-first", Lsn::from_raw(0));
    must_ok(fixture.store.rotate_segment(epoch_id()));
    fixture.append_submission_on_segment(
        "rotation-recover-second",
        Lsn::from_raw(2),
        WalSegmentId::from_raw(2),
    );

    let report = must_ok(fixture.recover_read_only());

    assert_eq!(report.tail_posture, RecoveryTailPosture::Clean);
    assert_eq!(report.transactions.len(), 2);
    assert_eq!(report.last_committed_lsn(), Some(Lsn::from_raw(3)));
}

#[test]
fn filesystem_manifest_read_roundtrips_published_manifest() {
    let mut fixture = WalHardeningFixture::new("manifest-roundtrip");
    let transaction = fixture.append_submission("manifest-roundtrip", Lsn::from_raw(0));
    let manifest = WalManifest {
        manifest_digest: digest("hardening:manifest:roundtrip"),
        last_committed_lsn: Some(transaction.commit.last_lsn),
        last_commit_digest: Some(transaction.commit.commit_digest),
        sealed_segment_count: 1,
    };

    must_ok(fixture.store.publish_manifest(epoch_id(), manifest.clone()));

    assert_eq!(
        must_ok(read_filesystem_manifest(&fixture.root)),
        Some(manifest)
    );
}

#[test]
fn filesystem_manifest_validation_accepts_matching_segment_summary() {
    let mut fixture = WalHardeningFixture::new("manifest-valid");
    fixture.append_submission("manifest-valid-first", Lsn::from_raw(0));
    must_ok(fixture.store.rotate_segment(epoch_id()));
    let second = fixture.append_submission_on_segment(
        "manifest-valid-second",
        Lsn::from_raw(2),
        WalSegmentId::from_raw(2),
    );
    let manifest = WalManifest {
        manifest_digest: digest("hardening:manifest:valid"),
        last_committed_lsn: Some(second.commit.last_lsn),
        last_commit_digest: Some(second.commit.commit_digest),
        sealed_segment_count: 2,
    };

    must_ok(fixture.store.publish_manifest(epoch_id(), manifest.clone()));
    let report = must_ok(validate_filesystem_manifest(&fixture.root));

    assert_eq!(report.manifest, manifest);
    assert_eq!(report.segment_count, 2);
    assert_eq!(report.last_committed_lsn, Some(Lsn::from_raw(3)));
    assert_eq!(report.last_commit_digest, Some(second.commit.commit_digest));
}

#[test]
fn filesystem_manifest_validation_rejects_segment_count_mismatch() {
    let mut fixture = WalHardeningFixture::new("manifest-count-mismatch");
    let transaction = fixture.append_submission("manifest-count-mismatch", Lsn::from_raw(0));
    let manifest = WalManifest {
        manifest_digest: digest("hardening:manifest:count-mismatch"),
        last_committed_lsn: Some(transaction.commit.last_lsn),
        last_commit_digest: Some(transaction.commit.commit_digest),
        sealed_segment_count: 2,
    };

    must_ok(fixture.store.publish_manifest(epoch_id(), manifest));
    let error = must_err(
        validate_filesystem_manifest(&fixture.root),
        "manifest segment count mismatch should reject",
    );

    assert_eq!(
        error,
        WalStoreError::ManifestSegmentCountMismatch {
            expected: 1,
            actual: 2
        }
    );
}

#[test]
fn filesystem_manifest_validation_rejects_last_lsn_mismatch() {
    let mut fixture = WalHardeningFixture::new("manifest-lsn-mismatch");
    let transaction = fixture.append_submission("manifest-lsn-mismatch", Lsn::from_raw(0));
    let manifest = WalManifest {
        manifest_digest: digest("hardening:manifest:lsn-mismatch"),
        last_committed_lsn: Some(Lsn::from_raw(99)),
        last_commit_digest: Some(transaction.commit.commit_digest),
        sealed_segment_count: 1,
    };

    must_ok(fixture.store.publish_manifest(epoch_id(), manifest));
    let error = must_err(
        validate_filesystem_manifest(&fixture.root),
        "manifest last LSN mismatch should reject",
    );

    assert_eq!(
        error,
        WalStoreError::ManifestLastCommittedLsnMismatch {
            expected: Some(transaction.commit.last_lsn),
            actual: Some(Lsn::from_raw(99))
        }
    );
}

#[test]
fn filesystem_manifest_validation_rejects_last_digest_mismatch() {
    let mut fixture = WalHardeningFixture::new("manifest-digest-mismatch");
    let transaction = fixture.append_submission("manifest-digest-mismatch", Lsn::from_raw(0));
    let wrong_digest = digest("hardening:manifest:digest-mismatch:wrong");
    let manifest = WalManifest {
        manifest_digest: digest("hardening:manifest:digest-mismatch"),
        last_committed_lsn: Some(transaction.commit.last_lsn),
        last_commit_digest: Some(wrong_digest),
        sealed_segment_count: 1,
    };

    must_ok(fixture.store.publish_manifest(epoch_id(), manifest));
    let error = must_err(
        validate_filesystem_manifest(&fixture.root),
        "manifest last digest mismatch should reject",
    );

    assert_eq!(
        error,
        WalStoreError::ManifestLastCommitDigestMismatch {
            expected: Some(transaction.commit.commit_digest),
            actual: Some(wrong_digest)
        }
    );
}

#[test]
fn filesystem_manifest_validation_rejects_uncommitted_tail() {
    let mut fixture = WalHardeningFixture::new("manifest-tail");
    let transaction = fixture.append_submission("manifest-tail", Lsn::from_raw(0));
    let manifest = WalManifest {
        manifest_digest: digest("hardening:manifest:tail"),
        last_committed_lsn: Some(transaction.commit.last_lsn),
        last_commit_digest: Some(transaction.commit.commit_digest),
        sealed_segment_count: 1,
    };
    must_ok(fixture.store.publish_manifest(epoch_id(), manifest));
    fixture.append_uncommitted_submission_frame("manifest-tail-uncommitted", Lsn::from_raw(2));

    let error = must_err(
        validate_filesystem_manifest(&fixture.root),
        "manifest validation should reject uncommitted tails",
    );

    assert_eq!(error, WalStoreError::ManifestCannotValidateUncommittedTail);
}

#[test]
fn segment_manifest_validation_gate_is_part_of_wal_release_readiness() {
    let report = audit_wal_release_readiness(WalReleaseReadinessGates {
        filesystem_adapter: true,
        object_store_capability_gate: true,
        segment_repair: true,
        segment_layout_policy: true,
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
        ..WalReleaseReadinessGates::default()
    });

    assert!(
        report
            .blocked_gates
            .contains(&"segment_manifest_validation"),
        "manifest validation should be an explicit WAL release gate"
    );
}
