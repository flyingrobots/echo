// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Adversarial causal WAL hardening tests.

use warp_core::causal_wal::{
    build_submission_acceptance_transaction, build_tick_transaction, recover_filesystem_store,
    recover_receipt_index, recover_submission_index, AffectedFrontier, AffectedFrontierKind,
    FilesystemWalStore, Lsn, PayloadCodecId, PayloadSchemaId, RecoveredSubmissionPosture,
    RecoveryAccessMode, RecoveryTailPosture, SubmissionAcceptanceRecord, SubmissionRetryPosture,
    TickReceiptRecord, WalAppendAuthority, WalCommittedTransaction, WalDurabilityMode,
    WalReceiptCorrelationRecord, WalRecoveryError, WalSegmentId, WalStoreError, WalStorePort,
    WalTickDecision, WalTransactionBuilder, WalTransactionId, WalTransactionKind, WriterEpochId,
    WriterEpochRequest,
};
use warp_core::Hash;

use std::fs::{self, OpenOptions};
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
        Err(error) => {
            let ok = false;
            assert!(ok, "expected Ok(..), got {error:?}");
            std::process::abort();
        }
    }
}

fn must_err<T: std::fmt::Debug, E>(result: Result<T, E>, context: &str) -> E {
    match result {
        Err(error) => error,
        Ok(value) => {
            let ok = false;
            assert!(ok, "{context}: expected Err(..), got Ok({value:?})");
            std::process::abort();
        }
    }
}

fn must_some<T>(option: Option<T>, context: &str) -> T {
    if let Some(value) = option {
        value
    } else {
        let ok = false;
        assert!(ok, "{context}: expected Some(..), got None");
        std::process::abort();
    }
}

fn transaction_id(label: &str) -> WalTransactionId {
    WalTransactionId::from_hash(digest(label))
}

fn epoch_id() -> WriterEpochId {
    WriterEpochId::from_hash(digest("hardening:epoch:1"))
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

fn builder(
    label: &str,
    first_lsn: Lsn,
    authority: WalAppendAuthority,
    transaction_kind: WalTransactionKind,
) -> WalTransactionBuilder {
    WalTransactionBuilder::new(
        epoch_id(),
        WalSegmentId::from_raw(1),
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
    must_ok(build_submission_acceptance_transaction(
        builder(
            label,
            first_lsn,
            WalAppendAuthority::SubmissionIntake,
            WalTransactionKind::SubmissionIntake,
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

fn temp_wal_root(label: &str) -> PathBuf {
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "echo-wal-hardening-{label}-{}-{unique}",
        std::process::id()
    ));
    must_ok(fs::create_dir_all(&root));
    root
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
