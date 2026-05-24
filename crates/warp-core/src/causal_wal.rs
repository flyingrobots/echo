// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo-owned causal write-ahead log primitives.
//!
//! This module is the first in-memory foundation for the causal WAL described in
//! `docs/design/causal-wal-end-to-end.md`. It deliberately stops short of
//! filesystem durability and live scheduler integration. The core invariant is
//! already enforced here:
//!
//! ```text
//! Records are recorded.
//! Transactions are committed.
//! History begins at WalTransactionCommit.
//! ```

use std::collections::BTreeMap;

use thiserror::Error;

use crate::ident::Hash;

const WAL_FRAME_DOMAIN: &[u8] = b"echo:causal_wal:frame:v1\0";
const WAL_PAYLOAD_DOMAIN: &[u8] = b"echo:causal_wal:payload:v1\0";
const WAL_RECORDS_ROOT_DOMAIN: &[u8] = b"echo:causal_wal:records_root:v1\0";
const WAL_FRONTIERS_ROOT_DOMAIN: &[u8] = b"echo:causal_wal:frontiers_root:v1\0";
const WAL_COMMIT_DOMAIN: &[u8] = b"echo:causal_wal:commit:v1\0";
const WAL_HEADER_CHECKSUM_DOMAIN: &[u8] = b"echo:causal_wal:header_checksum:v1\0";
const WAL_FRAME_CHECKSUM_DOMAIN: &[u8] = b"echo:causal_wal:frame_checksum:v1\0";

/// Current in-memory causal WAL version.
pub const CAUSAL_WAL_VERSION: u16 = 1;

/// Logical sequence number assigned to a WAL frame.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lsn(u64);

impl Lsn {
    /// Builds an LSN from its raw value.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw LSN value.
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Returns the next LSN, or `None` on overflow.
    pub fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Stable identifier for a WAL transaction.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WalTransactionId(Hash);

impl WalTransactionId {
    /// Builds a transaction id from a canonical digest.
    pub const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Returns the canonical transaction id bytes.
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identifier for a writer epoch.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WriterEpochId(Hash);

impl WriterEpochId {
    /// Builds an epoch id from a canonical digest.
    pub const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Returns the canonical epoch id bytes.
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Stable identifier for a WAL segment.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WalSegmentId(u64);

impl WalSegmentId {
    /// Builds a segment id from its raw value.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw segment id.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Position of a frame inside its transaction.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionLocalIndex(u32);

impl TransactionLocalIndex {
    /// Builds a transaction-local frame index from its raw value.
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw transaction-local frame index.
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the next local index, or `None` on overflow.
    pub fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Configured durability mode for a WAL transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalDurabilityMode {
    /// File and directory sync semantics satisfy the ACK contract.
    StrictFilesystem,
    /// Object and manifest commit semantics satisfy the ACK contract.
    StrictObjectStore,
    /// Development or test mode only; not release durability.
    Buffered,
    /// Recover and inspect without appending or truncating.
    ReadOnlyRecovery,
    /// Process-local mode with no durable causal-history claim.
    Disabled,
}

impl WalDurabilityMode {
    fn code(self) -> u8 {
        match self {
            Self::StrictFilesystem => 1,
            Self::StrictObjectStore => 2,
            Self::Buffered => 3,
            Self::ReadOnlyRecovery => 4,
            Self::Disabled => 5,
        }
    }
}

/// Authority plane allowed to record a WAL fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalAppendAuthority {
    /// Echo submission-intake authority.
    SubmissionIntake,
    /// Trusted scheduler/runtime authority.
    TrustedScheduler,
    /// Trusted runtime-control authority.
    RuntimeControl,
    /// Recovery authority.
    Recovery,
}

/// First-cut transaction kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalTransactionKind {
    /// Submission intake acceptance.
    SubmissionIntake,
    /// Scheduler-owned tick transaction.
    SchedulerTick,
    /// Runtime posture or control transaction.
    RuntimePosture,
    /// Checkpoint publication evidence.
    Checkpoint,
    /// Side-effect outbox transaction.
    MaterializationOutbox,
}

impl WalTransactionKind {
    fn code(self) -> u8 {
        match self {
            Self::SubmissionIntake => 1,
            Self::SchedulerTick => 2,
            Self::RuntimePosture => 3,
            Self::Checkpoint => 4,
            Self::MaterializationOutbox => 5,
        }
    }

    fn required_authority(self) -> WalAppendAuthority {
        match self {
            Self::SubmissionIntake => WalAppendAuthority::SubmissionIntake,
            Self::SchedulerTick | Self::MaterializationOutbox => {
                WalAppendAuthority::TrustedScheduler
            }
            Self::RuntimePosture => WalAppendAuthority::RuntimeControl,
            Self::Checkpoint => WalAppendAuthority::Recovery,
        }
    }
}

/// Causal WAL record kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WalRecordKind {
    /// Echo accepted a canonical submission into semantic ingress history.
    SubmissionAcceptedRecorded,
    /// Echo recorded submission acceptance evidence.
    SubmissionAcceptanceEvidenceRecorded,
    /// Trusted runtime recorded a law witness.
    RuntimeLawWitnessRecorded,
    /// Trusted runtime issued runtime admission-ticket evidence.
    RuntimeAdmissionTicketIssued,
    /// Trusted runtime recorded ticketed runtime ingress.
    TicketedRuntimeIngressRecorded,
    /// Trusted scheduler recorded a tick receipt.
    TickReceiptRecorded,
    /// Trusted scheduler recorded a runtime state delta.
    RuntimeStateDeltaRecorded,
    /// Trusted scheduler recorded receipt-correlation index material.
    ReceiptCorrelationRecorded,
    /// Runtime recorded a retained reading envelope reference.
    ReadingEnvelopeRetained,
    /// Runtime recorded a durable retained-material reference.
    RetainedMaterialRefRecorded,
    /// Runtime recorded scoped scheduler-fault quarantine posture.
    SchedulerFaultQuarantined,
    /// Trusted runtime recorded runtime-control posture.
    TrustedRuntimeControlRecorded,
    /// Runtime recorded checkpoint publication evidence.
    CheckpointPublicationRecorded,
    /// Runtime recorded side-effect materialization intent.
    MaterializationIntentRecorded,
    /// Runtime recorded side-effect materialization observation.
    MaterializationEffectObserved,
    /// Runtime recorded recovery posture.
    RecoveryPostureRecorded,
}

impl WalRecordKind {
    /// Returns the canonical record kind label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::SubmissionAcceptedRecorded => "SubmissionAcceptedRecorded",
            Self::SubmissionAcceptanceEvidenceRecorded => "SubmissionAcceptanceEvidenceRecorded",
            Self::RuntimeLawWitnessRecorded => "RuntimeLawWitnessRecorded",
            Self::RuntimeAdmissionTicketIssued => "RuntimeAdmissionTicketIssued",
            Self::TicketedRuntimeIngressRecorded => "TicketedRuntimeIngressRecorded",
            Self::TickReceiptRecorded => "TickReceiptRecorded",
            Self::RuntimeStateDeltaRecorded => "RuntimeStateDeltaRecorded",
            Self::ReceiptCorrelationRecorded => "ReceiptCorrelationRecorded",
            Self::ReadingEnvelopeRetained => "ReadingEnvelopeRetained",
            Self::RetainedMaterialRefRecorded => "RetainedMaterialRefRecorded",
            Self::SchedulerFaultQuarantined => "SchedulerFaultQuarantined",
            Self::TrustedRuntimeControlRecorded => "TrustedRuntimeControlRecorded",
            Self::CheckpointPublicationRecorded => "CheckpointPublicationRecorded",
            Self::MaterializationIntentRecorded => "MaterializationIntentRecorded",
            Self::MaterializationEffectObserved => "MaterializationEffectObserved",
            Self::RecoveryPostureRecorded => "RecoveryPostureRecorded",
        }
    }

    /// Returns the append authority required for this record kind.
    pub const fn required_authority(self) -> WalAppendAuthority {
        match self {
            Self::SubmissionAcceptedRecorded | Self::SubmissionAcceptanceEvidenceRecorded => {
                WalAppendAuthority::SubmissionIntake
            }
            Self::RuntimeLawWitnessRecorded
            | Self::RuntimeAdmissionTicketIssued
            | Self::TicketedRuntimeIngressRecorded
            | Self::TickReceiptRecorded
            | Self::RuntimeStateDeltaRecorded
            | Self::ReceiptCorrelationRecorded
            | Self::ReadingEnvelopeRetained
            | Self::RetainedMaterialRefRecorded
            | Self::MaterializationIntentRecorded
            | Self::MaterializationEffectObserved => WalAppendAuthority::TrustedScheduler,
            Self::SchedulerFaultQuarantined | Self::TrustedRuntimeControlRecorded => {
                WalAppendAuthority::RuntimeControl
            }
            Self::CheckpointPublicationRecorded | Self::RecoveryPostureRecorded => {
                WalAppendAuthority::Recovery
            }
        }
    }

    /// Returns `true` when the label obeys the recorded-not-committed grammar.
    pub fn obeys_recorded_not_committed_grammar(self) -> bool {
        !self.label().contains("Committed")
    }
}

/// Identity of the canonical payload codec.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PayloadCodecId(Hash);

impl PayloadCodecId {
    /// Builds a codec id from a hash.
    pub const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Returns the codec id bytes.
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Identity of the payload schema.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PayloadSchemaId(Hash);

impl PayloadSchemaId {
    /// Builds a schema id from a hash.
    pub const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Returns the schema id bytes.
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// WAL payload compression posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalCompressionKind {
    /// Payload bytes are not compressed.
    None,
}

impl WalCompressionKind {
    fn code(self) -> u8 {
        match self {
            Self::None => 0,
        }
    }
}

/// WAL payload revelation posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalRedactionPosture {
    /// Full payload bytes are present.
    Present,
    /// WAL carries digest-only evidence.
    DigestOnly,
    /// WAL carries retained-reference evidence.
    RetainedRef,
    /// Payload bytes are encrypted.
    Encrypted,
    /// Payload bytes are redacted by policy.
    RedactedByPolicy,
}

impl WalRedactionPosture {
    fn code(self) -> u8 {
        match self {
            Self::Present => 1,
            Self::DigestOnly => 2,
            Self::RetainedRef => 3,
            Self::Encrypted => 4,
            Self::RedactedByPolicy => 5,
        }
    }
}

/// Runtime frontier family affected by a transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AffectedFrontierKind {
    /// Submission queue frontier.
    SubmissionQueue,
    /// Runtime state frontier.
    RuntimeState,
    /// Receipt/correlation frontier.
    ReceiptIndex,
    /// Reading/index frontier.
    ReadingIndex,
    /// Runtime control posture frontier.
    RuntimeControl,
    /// Checkpoint/index frontier.
    CheckpointIndex,
}

impl AffectedFrontierKind {
    fn code(self) -> u8 {
        match self {
            Self::SubmissionQueue => 1,
            Self::RuntimeState => 2,
            Self::ReceiptIndex => 3,
            Self::ReadingIndex => 4,
            Self::RuntimeControl => 5,
            Self::CheckpointIndex => 6,
        }
    }
}

/// Digest transition for a frontier touched by a committed transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AffectedFrontier {
    /// Frontier kind.
    pub kind: AffectedFrontierKind,
    /// Digest before the transaction.
    pub before_digest: Hash,
    /// Digest after the transaction.
    pub after_digest: Hash,
}

/// Writer epoch metadata and storage fencing evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriterEpoch {
    /// Epoch id.
    pub epoch_id: WriterEpochId,
    /// Storage fencing token or lease token.
    pub storage_fencing_token: Hash,
    /// Process identity evidence.
    pub process_identity: Hash,
    /// Host identity evidence.
    pub host_identity: Hash,
    /// First LSN owned by the epoch.
    pub started_at_lsn: Lsn,
    /// Previous epoch id, if any.
    pub previous_epoch_id: Option<WriterEpochId>,
    /// Previous epoch final commit digest, if any.
    pub previous_epoch_final_commit_digest: Option<Hash>,
    /// Lease or lock evidence.
    pub lease_or_lock_evidence: Hash,
}

/// Canonical WAL record payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalRecordPayload {
    /// Record kind.
    pub kind: WalRecordKind,
    /// Payload schema version.
    pub schema_version: u16,
    /// Canonical payload bytes.
    pub canonical_bytes: Vec<u8>,
}

impl WalRecordPayload {
    /// Creates a canonical payload.
    pub fn new(kind: WalRecordKind, schema_version: u16, canonical_bytes: Vec<u8>) -> Self {
        Self {
            kind,
            schema_version,
            canonical_bytes,
        }
    }

    /// Computes the domain-separated payload digest.
    pub fn digest(&self) -> Hash {
        let mut h = blake3::Hasher::new();
        h.update(WAL_PAYLOAD_DOMAIN);
        h.update(self.kind.label().as_bytes());
        h.update(&self.schema_version.to_le_bytes());
        update_len_prefixed(&mut h, &self.canonical_bytes);
        h.finalize().into()
    }
}

/// WAL frame header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalFrameHeader {
    /// WAL version.
    pub wal_version: u16,
    /// Writer epoch.
    pub writer_epoch: WriterEpochId,
    /// Segment id.
    pub segment_id: WalSegmentId,
    /// Frame LSN.
    pub lsn: Lsn,
    /// Transaction id.
    pub transaction_id: WalTransactionId,
    /// Frame index inside the transaction.
    pub transaction_local_index: TransactionLocalIndex,
    /// Record kind.
    pub record_kind: WalRecordKind,
    /// Payload byte length.
    pub payload_len: u64,
    /// Payload digest.
    pub payload_digest: Hash,
    /// Payload codec id.
    pub payload_codec_id: PayloadCodecId,
    /// Payload schema id.
    pub payload_schema_id: PayloadSchemaId,
    /// Payload schema version.
    pub payload_schema_version: u16,
    /// Canonical encoding version.
    pub canonical_encoding_version: u16,
    /// Digest domain id.
    pub digest_domain: Hash,
    /// Compression kind.
    pub compression_kind: WalCompressionKind,
    /// Encryption or redaction posture.
    pub encryption_or_redaction_posture: WalRedactionPosture,
    /// Previous frame digest.
    pub previous_frame_digest: Hash,
    /// Header checksum.
    pub header_checksum: u32,
}

impl WalFrameHeader {
    fn checksum_input(&self, include_checksum: bool) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.wal_version.to_le_bytes());
        out.extend_from_slice(&self.writer_epoch.as_hash());
        out.extend_from_slice(&self.segment_id.as_u64().to_le_bytes());
        out.extend_from_slice(&self.lsn.as_u64().to_le_bytes());
        out.extend_from_slice(&self.transaction_id.as_hash());
        out.extend_from_slice(&self.transaction_local_index.as_u32().to_le_bytes());
        out.extend_from_slice(self.record_kind.label().as_bytes());
        out.extend_from_slice(&self.payload_len.to_le_bytes());
        out.extend_from_slice(&self.payload_digest);
        out.extend_from_slice(&self.payload_codec_id.as_hash());
        out.extend_from_slice(&self.payload_schema_id.as_hash());
        out.extend_from_slice(&self.payload_schema_version.to_le_bytes());
        out.extend_from_slice(&self.canonical_encoding_version.to_le_bytes());
        out.extend_from_slice(&self.digest_domain);
        out.push(self.compression_kind.code());
        out.push(self.encryption_or_redaction_posture.code());
        out.extend_from_slice(&self.previous_frame_digest);
        if include_checksum {
            out.extend_from_slice(&self.header_checksum.to_le_bytes());
        }
        out
    }

    fn compute_checksum(&self) -> u32 {
        checksum32(WAL_HEADER_CHECKSUM_DOMAIN, &self.checksum_input(false))
    }
}

/// WAL frame trailer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalFrameTrailer {
    /// Frame checksum over the header, payload, and payload digest.
    pub frame_checksum: u32,
}

/// WAL frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalFrame {
    /// Frame header.
    pub header: WalFrameHeader,
    /// Frame payload.
    pub payload: WalRecordPayload,
    /// Frame trailer.
    pub trailer: WalFrameTrailer,
}

impl WalFrame {
    /// Creates a frame and computes checksums.
    pub fn new(mut header: WalFrameHeader, payload: WalRecordPayload) -> Self {
        header.header_checksum = header.compute_checksum();
        let frame_checksum = compute_frame_checksum(&header, &payload);
        Self {
            header,
            payload,
            trailer: WalFrameTrailer { frame_checksum },
        }
    }

    /// Computes the frame digest.
    pub fn digest(&self) -> Hash {
        let mut h = blake3::Hasher::new();
        h.update(WAL_FRAME_DOMAIN);
        h.update(&self.header.checksum_input(true));
        h.update(&self.payload.digest());
        h.update(&self.trailer.frame_checksum.to_le_bytes());
        h.finalize().into()
    }

    /// Validates checksums and payload/header alignment.
    pub fn validate_integrity(&self) -> Result<(), WalValidationError> {
        if self.payload.kind != self.header.record_kind {
            return Err(WalValidationError::RecordKindMismatch);
        }
        if self.payload.digest() != self.header.payload_digest {
            return Err(WalValidationError::PayloadDigestMismatch);
        }
        if self.header.compute_checksum() != self.header.header_checksum {
            return Err(WalValidationError::HeaderChecksumMismatch);
        }
        if compute_frame_checksum(&self.header, &self.payload) != self.trailer.frame_checksum {
            return Err(WalValidationError::FrameChecksumMismatch);
        }
        Ok(())
    }
}

/// WAL transaction commit marker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalTransactionCommit {
    /// Writer epoch.
    pub writer_epoch: WriterEpochId,
    /// Transaction id.
    pub transaction_id: WalTransactionId,
    /// Transaction kind.
    pub transaction_kind: WalTransactionKind,
    /// First transaction frame LSN.
    pub first_lsn: Lsn,
    /// Last transaction frame LSN.
    pub last_lsn: Lsn,
    /// Number of records.
    pub record_count: u64,
    /// Root over transaction record frame digests.
    pub records_root: Hash,
    /// Root over affected frontier transitions.
    pub affected_frontiers_root: Hash,
    /// Previous committed transaction digest.
    pub previous_committed_transaction_digest: Hash,
    /// Durability mode.
    pub durability_mode: WalDurabilityMode,
    /// Commit schema version.
    pub schema_version: u16,
    /// Commit digest.
    pub commit_digest: Hash,
}

impl WalTransactionCommit {
    fn compute_digest(&self) -> Hash {
        let mut h = blake3::Hasher::new();
        h.update(WAL_COMMIT_DOMAIN);
        h.update(&self.writer_epoch.as_hash());
        h.update(&self.transaction_id.as_hash());
        h.update(&[self.transaction_kind.code()]);
        h.update(&self.first_lsn.as_u64().to_le_bytes());
        h.update(&self.last_lsn.as_u64().to_le_bytes());
        h.update(&self.record_count.to_le_bytes());
        h.update(&self.records_root);
        h.update(&self.affected_frontiers_root);
        h.update(&self.previous_committed_transaction_digest);
        h.update(&[self.durability_mode.code()]);
        h.update(&self.schema_version.to_le_bytes());
        h.finalize().into()
    }
}

/// Committed WAL transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalCommittedTransaction {
    /// Transaction frames.
    pub frames: Vec<WalFrame>,
    /// Affected frontiers.
    pub affected_frontiers: Vec<AffectedFrontier>,
    /// Commit marker.
    pub commit: WalTransactionCommit,
}

impl WalCommittedTransaction {
    /// Validates the transaction's structural and semantic commit invariants.
    pub fn validate(&self) -> Result<(), WalValidationError> {
        validate_transaction_frames(&self.frames, &self.commit)?;
        validate_transaction_semantics(&self.frames, self.commit.transaction_kind)?;
        if affected_frontiers_root(&self.affected_frontiers) != self.commit.affected_frontiers_root
        {
            return Err(WalValidationError::AffectedFrontiersRootMismatch);
        }
        if self.commit.compute_digest() != self.commit.commit_digest {
            return Err(WalValidationError::CommitDigestMismatch);
        }
        Ok(())
    }
}

/// Builder for a contiguous WAL transaction.
#[derive(Clone, Debug)]
pub struct WalTransactionBuilder {
    writer_epoch: WriterEpochId,
    segment_id: WalSegmentId,
    transaction_id: WalTransactionId,
    transaction_kind: WalTransactionKind,
    authority: WalAppendAuthority,
    next_lsn: Lsn,
    next_local_index: TransactionLocalIndex,
    previous_frame_digest: Hash,
    previous_committed_transaction_digest: Hash,
    durability_mode: WalDurabilityMode,
    payload_codec_id: PayloadCodecId,
    payload_schema_id: PayloadSchemaId,
    payload_schema_version: u16,
    canonical_encoding_version: u16,
    digest_domain: Hash,
    frames: Vec<WalFrame>,
    closed: bool,
}

impl WalTransactionBuilder {
    /// Creates a transaction builder.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        writer_epoch: WriterEpochId,
        segment_id: WalSegmentId,
        transaction_id: WalTransactionId,
        transaction_kind: WalTransactionKind,
        authority: WalAppendAuthority,
        first_lsn: Lsn,
        previous_frame_digest: Hash,
        previous_committed_transaction_digest: Hash,
        durability_mode: WalDurabilityMode,
        payload_codec_id: PayloadCodecId,
        payload_schema_id: PayloadSchemaId,
        payload_schema_version: u16,
        canonical_encoding_version: u16,
        digest_domain: Hash,
    ) -> Self {
        Self {
            writer_epoch,
            segment_id,
            transaction_id,
            transaction_kind,
            authority,
            next_lsn: first_lsn,
            next_local_index: TransactionLocalIndex::from_raw(0),
            previous_frame_digest,
            previous_committed_transaction_digest,
            durability_mode,
            payload_codec_id,
            payload_schema_id,
            payload_schema_version,
            canonical_encoding_version,
            digest_domain,
            frames: Vec::new(),
            closed: false,
        }
    }

    /// Appends a record to the transaction.
    pub fn push_record(
        &mut self,
        kind: WalRecordKind,
        canonical_bytes: Vec<u8>,
    ) -> Result<(), WalBuildError> {
        if self.closed {
            return Err(WalBuildError::TransactionClosed);
        }
        if kind.required_authority() != self.authority {
            return Err(WalBuildError::WrongAppendAuthority {
                record_kind: kind,
                required: kind.required_authority(),
                actual: self.authority,
            });
        }
        let payload = WalRecordPayload::new(kind, self.payload_schema_version, canonical_bytes);
        let payload_len = len_u64(payload.canonical_bytes.len());
        let header = WalFrameHeader {
            wal_version: CAUSAL_WAL_VERSION,
            writer_epoch: self.writer_epoch,
            segment_id: self.segment_id,
            lsn: self.next_lsn,
            transaction_id: self.transaction_id,
            transaction_local_index: self.next_local_index,
            record_kind: kind,
            payload_len,
            payload_digest: payload.digest(),
            payload_codec_id: self.payload_codec_id,
            payload_schema_id: self.payload_schema_id,
            payload_schema_version: self.payload_schema_version,
            canonical_encoding_version: self.canonical_encoding_version,
            digest_domain: self.digest_domain,
            compression_kind: WalCompressionKind::None,
            encryption_or_redaction_posture: WalRedactionPosture::Present,
            previous_frame_digest: self.previous_frame_digest,
            header_checksum: 0,
        };
        let frame = WalFrame::new(header, payload);
        self.previous_frame_digest = frame.digest();
        self.next_lsn = self
            .next_lsn
            .checked_next()
            .ok_or(WalBuildError::LsnOverflow)?;
        self.next_local_index = self
            .next_local_index
            .checked_next()
            .ok_or(WalBuildError::TransactionLocalIndexOverflow)?;
        self.frames.push(frame);
        Ok(())
    }

    /// Commits the transaction.
    pub fn commit(
        mut self,
        affected_frontiers: Vec<AffectedFrontier>,
    ) -> Result<WalCommittedTransaction, WalBuildError> {
        if self.frames.is_empty() {
            return Err(WalBuildError::EmptyTransaction);
        }
        self.closed = true;
        let first_lsn = self
            .frames
            .first()
            .map(|frame| frame.header.lsn)
            .ok_or(WalBuildError::EmptyTransaction)?;
        let last_lsn = self
            .frames
            .last()
            .map(|frame| frame.header.lsn)
            .ok_or(WalBuildError::EmptyTransaction)?;
        let record_count = len_u64(self.frames.len());
        let mut commit = WalTransactionCommit {
            writer_epoch: self.writer_epoch,
            transaction_id: self.transaction_id,
            transaction_kind: self.transaction_kind,
            first_lsn,
            last_lsn,
            record_count,
            records_root: records_root(&self.frames),
            affected_frontiers_root: affected_frontiers_root(&affected_frontiers),
            previous_committed_transaction_digest: self.previous_committed_transaction_digest,
            durability_mode: self.durability_mode,
            schema_version: CAUSAL_WAL_VERSION,
            commit_digest: [0; 32],
        };
        commit.commit_digest = commit.compute_digest();
        let transaction = WalCommittedTransaction {
            frames: self.frames,
            affected_frontiers,
            commit,
        };
        transaction.validate()?;
        Ok(transaction)
    }
}

/// WAL storage port.
pub trait WalStorePort {
    /// Acquires a writer epoch.
    fn acquire_writer_epoch(
        &mut self,
        request: WriterEpochRequest,
    ) -> Result<WriterEpoch, WalStoreError>;

    /// Appends a frame.
    fn append_frame(
        &mut self,
        epoch_id: WriterEpochId,
        frame: WalFrame,
    ) -> Result<(), WalStoreError>;

    /// Flushes a transaction commit marker under the store's durability mode.
    fn flush_commit(
        &mut self,
        epoch_id: WriterEpochId,
        commit: WalTransactionCommit,
    ) -> Result<(), WalStoreError>;

    /// Reads the recorded frames.
    fn read_frames(&self) -> Vec<WalFrame>;

    /// Reads the flushed commit markers.
    fn read_commits(&self) -> Vec<WalTransactionCommit>;

    /// Seals a segment.
    fn seal_segment(
        &mut self,
        epoch_id: WriterEpochId,
        segment_id: WalSegmentId,
    ) -> Result<WalSegmentSeal, WalStoreError>;

    /// Truncates the uncommitted tail after the given LSN.
    fn truncate_tail_after(&mut self, after_lsn: Lsn) -> Result<(), WalStoreError>;

    /// Publishes a manifest.
    fn publish_manifest(
        &mut self,
        epoch_id: WriterEpochId,
        manifest: WalManifest,
    ) -> Result<(), WalStoreError>;

    /// Closes the writer epoch.
    fn close_epoch(&mut self, epoch_id: WriterEpochId) -> Result<(), WalStoreError>;
}

/// Writer epoch acquisition request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriterEpochRequest {
    /// Epoch id.
    pub epoch_id: WriterEpochId,
    /// Storage fencing token.
    pub storage_fencing_token: Hash,
    /// Process identity.
    pub process_identity: Hash,
    /// Host identity.
    pub host_identity: Hash,
    /// Start LSN.
    pub started_at_lsn: Lsn,
    /// Previous epoch id.
    pub previous_epoch_id: Option<WriterEpochId>,
    /// Previous final commit digest.
    pub previous_epoch_final_commit_digest: Option<Hash>,
    /// Lease or lock evidence.
    pub lease_or_lock_evidence: Hash,
}

/// Segment seal result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalSegmentSeal {
    /// Sealed segment id.
    pub segment_id: WalSegmentId,
    /// Last sealed LSN.
    pub sealed_lsn: Option<Lsn>,
    /// Segment digest.
    pub segment_digest: Hash,
}

/// Published WAL manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalManifest {
    /// Manifest digest.
    pub manifest_digest: Hash,
    /// Last committed LSN.
    pub last_committed_lsn: Option<Lsn>,
    /// Last commit digest.
    pub last_commit_digest: Option<Hash>,
    /// Number of sealed segments.
    pub sealed_segment_count: u64,
}

/// Deterministic in-memory WAL store.
#[derive(Clone, Debug, Default)]
pub struct InMemoryWalStore {
    active_epoch: Option<WriterEpoch>,
    closed_epochs: Vec<WriterEpoch>,
    frames: Vec<WalFrame>,
    commits: Vec<WalTransactionCommit>,
    sealed_segments: Vec<WalSegmentSeal>,
    manifests: Vec<WalManifest>,
}

impl InMemoryWalStore {
    /// Creates an empty in-memory WAL store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends and flushes a committed transaction.
    pub fn append_transaction(
        &mut self,
        transaction: WalCommittedTransaction,
    ) -> Result<(), WalStoreError> {
        transaction.validate()?;
        let epoch_id = transaction.commit.writer_epoch;
        for frame in transaction.frames {
            self.append_frame(epoch_id, frame)?;
        }
        self.flush_commit(epoch_id, transaction.commit)
    }

    /// Appends a frame without a commit marker to simulate an uncommitted tail.
    pub fn append_uncommitted_frame(
        &mut self,
        epoch_id: WriterEpochId,
        frame: WalFrame,
    ) -> Result<(), WalStoreError> {
        self.append_frame(epoch_id, frame)
    }

    /// Returns published manifests.
    pub fn manifests(&self) -> &[WalManifest] {
        &self.manifests
    }
}

impl WalStorePort for InMemoryWalStore {
    fn acquire_writer_epoch(
        &mut self,
        request: WriterEpochRequest,
    ) -> Result<WriterEpoch, WalStoreError> {
        if self.active_epoch.is_some() {
            return Err(WalStoreError::WriterEpochAlreadyActive);
        }
        if let Some(previous_epoch_id) = request.previous_epoch_id {
            if self
                .closed_epochs
                .iter()
                .all(|epoch| epoch.epoch_id != previous_epoch_id)
            {
                return Err(WalStoreError::UnknownPreviousWriterEpoch);
            }
        }
        let epoch = WriterEpoch {
            epoch_id: request.epoch_id,
            storage_fencing_token: request.storage_fencing_token,
            process_identity: request.process_identity,
            host_identity: request.host_identity,
            started_at_lsn: request.started_at_lsn,
            previous_epoch_id: request.previous_epoch_id,
            previous_epoch_final_commit_digest: request.previous_epoch_final_commit_digest,
            lease_or_lock_evidence: request.lease_or_lock_evidence,
        };
        self.active_epoch = Some(epoch.clone());
        Ok(epoch)
    }

    fn append_frame(
        &mut self,
        epoch_id: WriterEpochId,
        frame: WalFrame,
    ) -> Result<(), WalStoreError> {
        let active_epoch = self
            .active_epoch
            .as_ref()
            .ok_or(WalStoreError::NoActiveWriterEpoch)?;
        if active_epoch.epoch_id != epoch_id || frame.header.writer_epoch != epoch_id {
            return Err(WalStoreError::WriterEpochMismatch);
        }
        frame.validate_integrity()?;
        self.frames.push(frame);
        Ok(())
    }

    fn flush_commit(
        &mut self,
        epoch_id: WriterEpochId,
        commit: WalTransactionCommit,
    ) -> Result<(), WalStoreError> {
        let active_epoch = self
            .active_epoch
            .as_ref()
            .ok_or(WalStoreError::NoActiveWriterEpoch)?;
        if active_epoch.epoch_id != epoch_id || commit.writer_epoch != epoch_id {
            return Err(WalStoreError::WriterEpochMismatch);
        }
        self.commits.push(commit);
        Ok(())
    }

    fn read_frames(&self) -> Vec<WalFrame> {
        self.frames.clone()
    }

    fn read_commits(&self) -> Vec<WalTransactionCommit> {
        self.commits.clone()
    }

    fn seal_segment(
        &mut self,
        epoch_id: WriterEpochId,
        segment_id: WalSegmentId,
    ) -> Result<WalSegmentSeal, WalStoreError> {
        let active_epoch = self
            .active_epoch
            .as_ref()
            .ok_or(WalStoreError::NoActiveWriterEpoch)?;
        if active_epoch.epoch_id != epoch_id {
            return Err(WalStoreError::WriterEpochMismatch);
        }
        let segment_frames: Vec<&WalFrame> = self
            .frames
            .iter()
            .filter(|frame| frame.header.segment_id == segment_id)
            .collect();
        let sealed_lsn = segment_frames.iter().map(|frame| frame.header.lsn).max();
        let segment_digest = segment_digest(segment_id, &segment_frames);
        let seal = WalSegmentSeal {
            segment_id,
            sealed_lsn,
            segment_digest,
        };
        self.sealed_segments.push(seal.clone());
        Ok(seal)
    }

    fn truncate_tail_after(&mut self, after_lsn: Lsn) -> Result<(), WalStoreError> {
        self.frames.retain(|frame| frame.header.lsn <= after_lsn);
        Ok(())
    }

    fn publish_manifest(
        &mut self,
        epoch_id: WriterEpochId,
        manifest: WalManifest,
    ) -> Result<(), WalStoreError> {
        let active_epoch = self
            .active_epoch
            .as_ref()
            .ok_or(WalStoreError::NoActiveWriterEpoch)?;
        if active_epoch.epoch_id != epoch_id {
            return Err(WalStoreError::WriterEpochMismatch);
        }
        self.manifests.push(manifest);
        Ok(())
    }

    fn close_epoch(&mut self, epoch_id: WriterEpochId) -> Result<(), WalStoreError> {
        let epoch = self
            .active_epoch
            .take()
            .ok_or(WalStoreError::NoActiveWriterEpoch)?;
        if epoch.epoch_id != epoch_id {
            self.active_epoch = Some(epoch);
            return Err(WalStoreError::WriterEpochMismatch);
        }
        self.closed_epochs.push(epoch);
        Ok(())
    }
}

/// Recovery access mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryAccessMode {
    /// Writable recovery may truncate incomplete tails after validation.
    Writable,
    /// Read-only recovery reports incomplete tails without mutating storage.
    ReadOnly,
}

/// Recovery tail posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryTailPosture {
    /// No uncommitted tail was present.
    Clean,
    /// Writable recovery truncated every frame because no committed
    /// transaction was present.
    TruncatedAll,
    /// Writable recovery truncated after the given LSN.
    TruncatedAfter(Lsn),
    /// Read-only recovery would truncate every frame because no committed
    /// transaction was present.
    WouldTruncateAll,
    /// Read-only recovery would truncate after the given LSN.
    WouldTruncateAfter(Lsn),
}

/// Recovery scan report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryScanReport {
    /// Valid committed transactions.
    pub transactions: Vec<WalRecoveredTransaction>,
    /// Tail posture.
    pub tail_posture: RecoveryTailPosture,
}

/// Recovered committed transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalRecoveredTransaction {
    /// Commit marker.
    pub commit: WalTransactionCommit,
    /// Transaction frames.
    pub frames: Vec<WalFrame>,
}

/// Scans an in-memory store for recoverable transactions.
pub fn recover_in_memory_store(
    store: &mut InMemoryWalStore,
    mode: RecoveryAccessMode,
) -> Result<RecoveryScanReport, WalRecoveryError> {
    let frames = store.read_frames();
    let commits = store.read_commits();
    let report = recover_from_frames_and_commits(&frames, &commits, mode)?;
    if let (RecoveryAccessMode::Writable, RecoveryTailPosture::TruncatedAfter(lsn)) =
        (mode, report.tail_posture)
    {
        store.truncate_tail_after(lsn)?;
    }
    if (mode, report.tail_posture)
        == (
            RecoveryAccessMode::Writable,
            RecoveryTailPosture::TruncatedAll,
        )
    {
        store.frames.clear();
    }
    Ok(report)
}

/// Recovers committed transactions from frames and commit markers.
pub fn recover_from_frames_and_commits(
    frames: &[WalFrame],
    commits: &[WalTransactionCommit],
    mode: RecoveryAccessMode,
) -> Result<RecoveryScanReport, WalRecoveryError> {
    let mut recovered = Vec::new();
    let mut last_committed_lsn = None;
    for commit in commits {
        let tx_frames: Vec<WalFrame> = frames
            .iter()
            .filter(|frame| {
                frame.header.transaction_id == commit.transaction_id
                    && frame.header.lsn >= commit.first_lsn
                    && frame.header.lsn <= commit.last_lsn
            })
            .cloned()
            .collect();
        validate_transaction_frames(&tx_frames, commit)?;
        recovered.push(WalRecoveredTransaction {
            commit: commit.clone(),
            frames: tx_frames,
        });
        last_committed_lsn = Some(commit.last_lsn);
    }
    let tail_exists = frames
        .iter()
        .any(|frame| last_committed_lsn.is_none_or(|lsn| frame.header.lsn > lsn));
    let tail_posture = match (tail_exists, mode, last_committed_lsn) {
        (false, _, _) => RecoveryTailPosture::Clean,
        (true, RecoveryAccessMode::Writable, Some(lsn)) => RecoveryTailPosture::TruncatedAfter(lsn),
        (true, RecoveryAccessMode::ReadOnly, Some(lsn)) => {
            RecoveryTailPosture::WouldTruncateAfter(lsn)
        }
        (true, RecoveryAccessMode::Writable, None) => RecoveryTailPosture::TruncatedAll,
        (true, RecoveryAccessMode::ReadOnly, None) => RecoveryTailPosture::WouldTruncateAll,
    };
    Ok(RecoveryScanReport {
        transactions: recovered,
        tail_posture,
    })
}

/// Minimal recovered state used by the pure replay reducer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveredState {
    /// Applied transaction ids in replay order.
    pub applied_transactions: Vec<WalTransactionId>,
    /// Current frontier digests by frontier kind.
    pub frontiers: BTreeMap<AffectedFrontierKind, Hash>,
}

/// Applies a validated committed transaction to a recovered state.
pub fn apply_committed_transaction(
    mut before: RecoveredState,
    transaction: &WalCommittedTransaction,
) -> Result<RecoveredState, WalReplayError> {
    transaction.validate()?;
    for frontier in &transaction.affected_frontiers {
        if let Some(current) = before.frontiers.get(&frontier.kind) {
            if current != &frontier.before_digest {
                return Err(WalReplayError::FrontierMismatch {
                    kind: frontier.kind,
                    expected: *current,
                    actual: frontier.before_digest,
                });
            }
        }
        before
            .frontiers
            .insert(frontier.kind, frontier.after_digest);
    }
    before
        .applied_transactions
        .push(transaction.commit.transaction_id);
    Ok(before)
}

/// Lints proposed WAL schema terms for app nouns and authority leaks.
pub fn lint_wal_schema_terms<'a>(
    terms: impl IntoIterator<Item = &'a str>,
    forbidden_app_terms: &[&str],
) -> Result<(), WalSchemaLintError> {
    for term in terms {
        for forbidden in forbidden_app_terms {
            if term.contains(forbidden) {
                return Err(WalSchemaLintError::ForbiddenAppNoun {
                    term: term.to_owned(),
                    forbidden: (*forbidden).to_owned(),
                });
            }
        }
        for forbidden in AUTHORITY_LEAK_TERMS {
            if term.contains(forbidden) {
                return Err(WalSchemaLintError::ForbiddenAuthorityLeak {
                    term: term.to_owned(),
                    forbidden: (*forbidden).to_owned(),
                });
            }
        }
        if term.contains("Committed") && term != "WalTransactionCommit" {
            return Err(WalSchemaLintError::RecordNameImpliesCommit {
                term: term.to_owned(),
            });
        }
    }
    Ok(())
}

const AUTHORITY_LEAK_TERMS: &[&str] = &[
    "AppTick",
    "ApplicationReceipt",
    "ClientRuntimeControl",
    "DocumentStateDelta",
];

/// WAL build errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WalBuildError {
    /// Transaction already closed.
    #[error("WAL transaction is already closed")]
    TransactionClosed,
    /// Record requires a different append authority.
    #[error(
        "wrong append authority for {record_kind:?}: required {required:?}, actual {actual:?}"
    )]
    WrongAppendAuthority {
        /// Record kind.
        record_kind: WalRecordKind,
        /// Required authority.
        required: WalAppendAuthority,
        /// Actual authority.
        actual: WalAppendAuthority,
    },
    /// LSN overflow.
    #[error("WAL LSN overflow")]
    LsnOverflow,
    /// Transaction-local index overflow.
    #[error("WAL transaction-local index overflow")]
    TransactionLocalIndexOverflow,
    /// Empty transaction.
    #[error("WAL transaction has no records")]
    EmptyTransaction,
    /// Validation failed.
    #[error(transparent)]
    Validation(#[from] WalValidationError),
}

/// WAL validation errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WalValidationError {
    /// Frame payload kind does not match header kind.
    #[error("WAL frame record kind mismatch")]
    RecordKindMismatch,
    /// Payload digest mismatch.
    #[error("WAL payload digest mismatch")]
    PayloadDigestMismatch,
    /// Header checksum mismatch.
    #[error("WAL header checksum mismatch")]
    HeaderChecksumMismatch,
    /// Frame checksum mismatch.
    #[error("WAL frame checksum mismatch")]
    FrameChecksumMismatch,
    /// Record kind does not match transaction kind authority.
    #[error("WAL record kind authority does not match transaction kind")]
    RecordAuthorityMismatch,
    /// Transaction contains no frames.
    #[error("WAL transaction contains no frames")]
    EmptyTransaction,
    /// Frame transaction id mismatch.
    #[error("WAL frame transaction id mismatch")]
    TransactionIdMismatch,
    /// Frame writer epoch mismatch.
    #[error("WAL frame writer epoch mismatch")]
    WriterEpochMismatch,
    /// Transaction local index mismatch.
    #[error("WAL transaction local index mismatch")]
    TransactionLocalIndexMismatch,
    /// Transaction LSN continuity mismatch.
    #[error("WAL transaction LSN continuity mismatch")]
    LsnContinuityMismatch,
    /// First LSN mismatch.
    #[error("WAL transaction first LSN mismatch")]
    FirstLsnMismatch,
    /// Last LSN mismatch.
    #[error("WAL transaction last LSN mismatch")]
    LastLsnMismatch,
    /// Record count mismatch.
    #[error("WAL transaction record count mismatch")]
    RecordCountMismatch,
    /// Records root mismatch.
    #[error("WAL transaction records root mismatch")]
    RecordsRootMismatch,
    /// Affected frontiers root mismatch.
    #[error("WAL transaction affected frontiers root mismatch")]
    AffectedFrontiersRootMismatch,
    /// Commit digest mismatch.
    #[error("WAL transaction commit digest mismatch")]
    CommitDigestMismatch,
}

/// WAL store errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WalStoreError {
    /// A writer epoch is already active.
    #[error("WAL writer epoch already active")]
    WriterEpochAlreadyActive,
    /// No writer epoch is active.
    #[error("no active WAL writer epoch")]
    NoActiveWriterEpoch,
    /// Writer epoch mismatch.
    #[error("WAL writer epoch mismatch")]
    WriterEpochMismatch,
    /// Previous writer epoch is unknown.
    #[error("unknown previous WAL writer epoch")]
    UnknownPreviousWriterEpoch,
    /// Validation failed.
    #[error(transparent)]
    Validation(#[from] WalValidationError),
}

/// WAL recovery errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WalRecoveryError {
    /// Validation failed.
    #[error(transparent)]
    Validation(#[from] WalValidationError),
    /// Store operation failed.
    #[error(transparent)]
    Store(#[from] WalStoreError),
}

/// WAL replay errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WalReplayError {
    /// Validation failed.
    #[error(transparent)]
    Validation(#[from] WalValidationError),
    /// Frontier mismatch.
    #[error("WAL replay frontier mismatch for {kind:?}")]
    FrontierMismatch {
        /// Frontier kind.
        kind: AffectedFrontierKind,
        /// Expected digest.
        expected: Hash,
        /// Actual digest.
        actual: Hash,
    },
}

/// WAL schema lint errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WalSchemaLintError {
    /// Forbidden app noun.
    #[error("WAL schema term {term:?} contains forbidden app noun {forbidden:?}")]
    ForbiddenAppNoun {
        /// Schema term.
        term: String,
        /// Forbidden noun.
        forbidden: String,
    },
    /// Forbidden authority leak.
    #[error("WAL schema term {term:?} contains forbidden authority leak {forbidden:?}")]
    ForbiddenAuthorityLeak {
        /// Schema term.
        term: String,
        /// Forbidden authority leak term.
        forbidden: String,
    },
    /// Record kind name implies commit before transaction commit.
    #[error("WAL schema term {term:?} implies commit outside WalTransactionCommit")]
    RecordNameImpliesCommit {
        /// Schema term.
        term: String,
    },
}

fn validate_transaction_frames(
    frames: &[WalFrame],
    commit: &WalTransactionCommit,
) -> Result<(), WalValidationError> {
    if frames.is_empty() {
        return Err(WalValidationError::EmptyTransaction);
    }
    let first = frames.first().ok_or(WalValidationError::EmptyTransaction)?;
    let last = frames.last().ok_or(WalValidationError::EmptyTransaction)?;
    if first.header.lsn != commit.first_lsn {
        return Err(WalValidationError::FirstLsnMismatch);
    }
    if last.header.lsn != commit.last_lsn {
        return Err(WalValidationError::LastLsnMismatch);
    }
    if len_u64(frames.len()) != commit.record_count {
        return Err(WalValidationError::RecordCountMismatch);
    }
    for (expected_index, frame) in frames.iter().enumerate() {
        frame.validate_integrity()?;
        if frame.header.transaction_id != commit.transaction_id {
            return Err(WalValidationError::TransactionIdMismatch);
        }
        if frame.header.writer_epoch != commit.writer_epoch {
            return Err(WalValidationError::WriterEpochMismatch);
        }
        if frame.header.transaction_local_index.as_u32() != len_u32(expected_index) {
            return Err(WalValidationError::TransactionLocalIndexMismatch);
        }
        let expected_lsn = commit
            .first_lsn
            .as_u64()
            .checked_add(len_u64(expected_index))
            .map(Lsn::from_raw)
            .ok_or(WalValidationError::LsnContinuityMismatch)?;
        if frame.header.lsn != expected_lsn {
            return Err(WalValidationError::LsnContinuityMismatch);
        }
    }
    if records_root(frames) != commit.records_root {
        return Err(WalValidationError::RecordsRootMismatch);
    }
    if commit.compute_digest() != commit.commit_digest {
        return Err(WalValidationError::CommitDigestMismatch);
    }
    Ok(())
}

fn validate_transaction_semantics(
    frames: &[WalFrame],
    transaction_kind: WalTransactionKind,
) -> Result<(), WalValidationError> {
    let expected_authority = transaction_kind.required_authority();
    for frame in frames {
        if frame.header.record_kind.required_authority() != expected_authority {
            return Err(WalValidationError::RecordAuthorityMismatch);
        }
    }
    Ok(())
}

fn records_root(frames: &[WalFrame]) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(WAL_RECORDS_ROOT_DOMAIN);
    h.update(&len_u64(frames.len()).to_le_bytes());
    for frame in frames {
        h.update(&frame.digest());
    }
    h.finalize().into()
}

fn affected_frontiers_root(frontiers: &[AffectedFrontier]) -> Hash {
    let mut sorted = frontiers.to_vec();
    sorted.sort_by_key(|frontier| frontier.kind);
    let mut h = blake3::Hasher::new();
    h.update(WAL_FRONTIERS_ROOT_DOMAIN);
    h.update(&len_u64(sorted.len()).to_le_bytes());
    for frontier in &sorted {
        h.update(&[frontier.kind.code()]);
        h.update(&frontier.before_digest);
        h.update(&frontier.after_digest);
    }
    h.finalize().into()
}

fn compute_frame_checksum(header: &WalFrameHeader, payload: &WalRecordPayload) -> u32 {
    let mut input = header.checksum_input(true);
    input.extend_from_slice(&payload.digest());
    input.extend_from_slice(&len_u64(payload.canonical_bytes.len()).to_le_bytes());
    input.extend_from_slice(&payload.canonical_bytes);
    checksum32(WAL_FRAME_CHECKSUM_DOMAIN, &input)
}

fn segment_digest(segment_id: WalSegmentId, frames: &[&WalFrame]) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(b"echo:causal_wal:segment:v1\0");
    h.update(&segment_id.as_u64().to_le_bytes());
    h.update(&len_u64(frames.len()).to_le_bytes());
    for frame in frames {
        h.update(&frame.digest());
    }
    h.finalize().into()
}

fn checksum32(domain: &[u8], bytes: &[u8]) -> u32 {
    let mut h = blake3::Hasher::new();
    h.update(domain);
    h.update(bytes);
    let digest = h.finalize();
    let mut out = [0; 4];
    out.copy_from_slice(&digest.as_bytes()[..4]);
    u32::from_le_bytes(out)
}

fn update_len_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&len_u64(bytes.len()).to_le_bytes());
    hasher.update(bytes);
}

fn len_u64(len: usize) -> u64 {
    match u64::try_from(len) {
        Ok(value) => value,
        Err(_) => u64::MAX,
    }
}

fn len_u32(len: usize) -> u32 {
    match u32::try_from(len) {
        Ok(value) => value,
        Err(_) => u32::MAX,
    }
}
