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

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use bytes::Bytes;
use thiserror::Error;

use crate::attachment::{AtomPayload, AttachmentValue};
use crate::braid::{BraidEvent, BraidStatus};
use crate::braid_shell::BraidMemberRef;
use crate::clock::WorldlineTick;
use crate::graph::GraphStore;
use crate::head::{HeadId, WriterHeadKey};
use crate::ident::{EdgeId, Hash, NodeId};
use crate::record::{EdgeRecord, NodeRecord};
use crate::revelation::{AuthorityDomainId, AuthorityDomainRef, OriginId};
use crate::strand::StrandId;
use crate::worldline::WorldlineId;
use crate::wsc::{validate_wsc, WscFile};

const WAL_FRAME_DOMAIN: &[u8] = b"echo:causal_wal:frame:v1\0";
const WAL_PAYLOAD_DOMAIN: &[u8] = b"echo:causal_wal:payload:v1\0";
const WAL_RECORDS_ROOT_DOMAIN: &[u8] = b"echo:causal_wal:records_root:v1\0";
const WAL_FRONTIERS_ROOT_DOMAIN: &[u8] = b"echo:causal_wal:frontiers_root:v1\0";
const WAL_COMMIT_DOMAIN: &[u8] = b"echo:causal_wal:commit:v1\0";
const WAL_RECOVERED_INDEX_ROOT_DOMAIN: &[u8] = b"echo:causal_wal:recovered_index_root:v1\0";
const WAL_HEADER_CHECKSUM_DOMAIN: &[u8] = b"echo:causal_wal:header_checksum:v1\0";
const WAL_FRAME_CHECKSUM_DOMAIN: &[u8] = b"echo:causal_wal:frame_checksum:v1\0";
const WAL_DISK_RECORD_DOMAIN: &[u8] = b"echo:causal_wal:disk_record:v1\0";
const WAL_PROJECTION_ROOT_DOMAIN: &[u8] = b"echo:causal_wal:projection:root:v1\0";
const WAL_PROJECTION_WRITER_EPOCH_DOMAIN: &[u8] = b"echo:causal_wal:projection:writer_epoch:v1\0";
const WAL_PROJECTION_SEGMENT_DOMAIN: &[u8] = b"echo:causal_wal:projection:segment:v1\0";
const WAL_PROJECTION_COMMIT_ANCHOR_DOMAIN: &[u8] = b"echo:causal_wal:projection:commit_anchor:v1\0";
const WAL_PROJECTION_RECOVERY_CERTIFICATE_DOMAIN: &[u8] =
    b"echo:causal_wal:projection:recovery_certificate:v1\0";
const WAL_PROJECTION_GRAPH_NODE_DOMAIN: &[u8] = b"echo:causal_wal:projection_graph:node:v1\0";
const WAL_PROJECTION_GRAPH_EDGE_DOMAIN: &[u8] = b"echo:causal_wal:projection_graph:edge:v1\0";
const WAL_RECOVERY_CERTIFICATE_DOMAIN: &[u8] = b"echo:causal_wal:recovery_certificate:v1\0";
const WRITER_HEAD_KEY_PAYLOAD_LEN: usize = 64;
const CHECKPOINT_FILE_MAGIC: &[u8; 8] = b"ECWALCP1";
const WAL_SEGMENT_RECORD_MAGIC: &[u8; 8] = b"ECWALR1!";
const WAL_SEGMENTS_DIR: &str = "segments";

/// WSC schema label for materialized WAL projection graph facts.
pub const WAL_PROJECTION_GRAPH_SCHEMA: &str = "echo/wal-projection-graph/schema/v1";
/// WARP id label for materialized WAL projection graph facts.
pub const WAL_PROJECTION_GRAPH_WARP: &str = "echo/wal-projection-graph/warp/v1";
/// Node type for the materialized WAL projection root fact.
pub const WAL_PROJECTION_GRAPH_ROOT_NODE_TYPE: &str = "echo/wal-projection-graph/root-node/v1";
/// Node type for materialized WAL writer epoch facts.
pub const WAL_PROJECTION_GRAPH_WRITER_EPOCH_NODE_TYPE: &str =
    "echo/wal-projection-graph/writer-epoch-node/v1";
/// Node type for materialized WAL segment facts.
pub const WAL_PROJECTION_GRAPH_SEGMENT_NODE_TYPE: &str =
    "echo/wal-projection-graph/segment-node/v1";
/// Node type for materialized WAL commit-anchor facts.
pub const WAL_PROJECTION_GRAPH_COMMIT_ANCHOR_NODE_TYPE: &str =
    "echo/wal-projection-graph/commit-anchor-node/v1";
/// Node type for materialized recovery-certificate reference facts.
pub const WAL_PROJECTION_GRAPH_RECOVERY_CERTIFICATE_NODE_TYPE: &str =
    "echo/wal-projection-graph/recovery-certificate-node/v1";
/// Edge type from the WAL projection root to writer epoch facts.
pub const WAL_PROJECTION_GRAPH_WRITER_EPOCH_EDGE_TYPE: &str =
    "echo/wal-projection-graph/root-writer-epoch-edge/v1";
/// Edge type from the WAL projection root to segment facts.
pub const WAL_PROJECTION_GRAPH_SEGMENT_EDGE_TYPE: &str =
    "echo/wal-projection-graph/root-segment-edge/v1";
/// Edge type from the WAL projection root to commit-anchor facts.
pub const WAL_PROJECTION_GRAPH_ROOT_COMMIT_ANCHOR_EDGE_TYPE: &str =
    "echo/wal-projection-graph/root-commit-anchor-edge/v1";
/// Edge type from the WAL projection root to recovery-certificate facts.
pub const WAL_PROJECTION_GRAPH_RECOVERY_CERTIFICATE_EDGE_TYPE: &str =
    "echo/wal-projection-graph/root-recovery-certificate-edge/v1";
/// Edge type from a WAL segment fact to its commit-anchor facts.
pub const WAL_PROJECTION_GRAPH_SEGMENT_COMMIT_ANCHOR_EDGE_TYPE: &str =
    "echo/wal-projection-graph/segment-commit-anchor-edge/v1";
/// Attachment type for WAL projection root fact payloads.
pub const WAL_PROJECTION_GRAPH_ROOT_ATTACHMENT_TYPE: &str =
    "echo/wal-projection-graph/root-attachment/v1";
/// Attachment type for WAL writer epoch fact payloads.
pub const WAL_PROJECTION_GRAPH_WRITER_EPOCH_ATTACHMENT_TYPE: &str =
    "echo/wal-projection-graph/writer-epoch-attachment/v1";
/// Attachment type for WAL segment fact payloads.
pub const WAL_PROJECTION_GRAPH_SEGMENT_ATTACHMENT_TYPE: &str =
    "echo/wal-projection-graph/segment-attachment/v1";
/// Attachment type for WAL commit-anchor fact payloads.
pub const WAL_PROJECTION_GRAPH_COMMIT_ANCHOR_ATTACHMENT_TYPE: &str =
    "echo/wal-projection-graph/commit-anchor-attachment/v1";
/// Attachment type for recovery-certificate reference fact payloads.
pub const WAL_PROJECTION_GRAPH_RECOVERY_CERTIFICATE_ATTACHMENT_TYPE: &str =
    "echo/wal-projection-graph/recovery-certificate-attachment/v1";

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

    fn from_code(code: u8) -> Result<Self, WalDecodeError> {
        match code {
            1 => Ok(Self::StrictFilesystem),
            2 => Ok(Self::StrictObjectStore),
            3 => Ok(Self::Buffered),
            4 => Ok(Self::ReadOnlyRecovery),
            5 => Ok(Self::Disabled),
            _ => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "WalDurabilityMode",
                code,
            }),
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
    /// Topology-changing strand, braid, or suffix-import intent evidence.
    TopologyIntent,
}

impl WalTransactionKind {
    fn code(self) -> u8 {
        match self {
            Self::SubmissionIntake => 1,
            Self::SchedulerTick => 2,
            Self::RuntimePosture => 3,
            Self::Checkpoint => 4,
            Self::MaterializationOutbox => 5,
            Self::TopologyIntent => 6,
        }
    }

    fn required_authority(self) -> WalAppendAuthority {
        match self {
            Self::SubmissionIntake => WalAppendAuthority::SubmissionIntake,
            Self::SchedulerTick | Self::MaterializationOutbox | Self::TopologyIntent => {
                WalAppendAuthority::TrustedScheduler
            }
            Self::RuntimePosture => WalAppendAuthority::RuntimeControl,
            Self::Checkpoint => WalAppendAuthority::Recovery,
        }
    }

    fn from_code(code: u8) -> Result<Self, WalDecodeError> {
        match code {
            1 => Ok(Self::SubmissionIntake),
            2 => Ok(Self::SchedulerTick),
            3 => Ok(Self::RuntimePosture),
            4 => Ok(Self::Checkpoint),
            5 => Ok(Self::MaterializationOutbox),
            6 => Ok(Self::TopologyIntent),
            _ => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "WalTransactionKind",
                code,
            }),
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
    /// Runtime recorded accepted strand fork topology evidence.
    TopologyStrandForkRecorded,
    /// Runtime recorded accepted strand drop topology evidence.
    TopologyStrandDropRecorded,
    /// Runtime recorded accepted braid lifecycle event evidence.
    TopologyBraidEventRecorded,
    /// Runtime recorded retained braid-shell topology evidence.
    TopologyBraidShellRetained,
    /// Runtime recorded witnessed suffix import topology evidence.
    TopologySuffixImportRecorded,
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
            Self::TopologyStrandForkRecorded => "TopologyStrandForkRecorded",
            Self::TopologyStrandDropRecorded => "TopologyStrandDropRecorded",
            Self::TopologyBraidEventRecorded => "TopologyBraidEventRecorded",
            Self::TopologyBraidShellRetained => "TopologyBraidShellRetained",
            Self::TopologySuffixImportRecorded => "TopologySuffixImportRecorded",
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
            | Self::MaterializationEffectObserved
            | Self::TopologyStrandForkRecorded
            | Self::TopologyStrandDropRecorded
            | Self::TopologyBraidEventRecorded
            | Self::TopologyBraidShellRetained
            | Self::TopologySuffixImportRecorded => WalAppendAuthority::TrustedScheduler,
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

    fn code(self) -> u8 {
        match self {
            Self::SubmissionAcceptedRecorded => 1,
            Self::SubmissionAcceptanceEvidenceRecorded => 2,
            Self::RuntimeLawWitnessRecorded => 3,
            Self::RuntimeAdmissionTicketIssued => 4,
            Self::TicketedRuntimeIngressRecorded => 5,
            Self::TickReceiptRecorded => 6,
            Self::RuntimeStateDeltaRecorded => 7,
            Self::ReceiptCorrelationRecorded => 8,
            Self::ReadingEnvelopeRetained => 9,
            Self::RetainedMaterialRefRecorded => 10,
            Self::SchedulerFaultQuarantined => 11,
            Self::TrustedRuntimeControlRecorded => 12,
            Self::CheckpointPublicationRecorded => 13,
            Self::MaterializationIntentRecorded => 14,
            Self::MaterializationEffectObserved => 15,
            Self::RecoveryPostureRecorded => 16,
            Self::TopologyStrandForkRecorded => 17,
            Self::TopologyStrandDropRecorded => 18,
            Self::TopologyBraidEventRecorded => 19,
            Self::TopologyBraidShellRetained => 20,
            Self::TopologySuffixImportRecorded => 21,
        }
    }

    fn from_code(code: u8) -> Result<Self, WalDecodeError> {
        match code {
            1 => Ok(Self::SubmissionAcceptedRecorded),
            2 => Ok(Self::SubmissionAcceptanceEvidenceRecorded),
            3 => Ok(Self::RuntimeLawWitnessRecorded),
            4 => Ok(Self::RuntimeAdmissionTicketIssued),
            5 => Ok(Self::TicketedRuntimeIngressRecorded),
            6 => Ok(Self::TickReceiptRecorded),
            7 => Ok(Self::RuntimeStateDeltaRecorded),
            8 => Ok(Self::ReceiptCorrelationRecorded),
            9 => Ok(Self::ReadingEnvelopeRetained),
            10 => Ok(Self::RetainedMaterialRefRecorded),
            11 => Ok(Self::SchedulerFaultQuarantined),
            12 => Ok(Self::TrustedRuntimeControlRecorded),
            13 => Ok(Self::CheckpointPublicationRecorded),
            14 => Ok(Self::MaterializationIntentRecorded),
            15 => Ok(Self::MaterializationEffectObserved),
            16 => Ok(Self::RecoveryPostureRecorded),
            17 => Ok(Self::TopologyStrandForkRecorded),
            18 => Ok(Self::TopologyStrandDropRecorded),
            19 => Ok(Self::TopologyBraidEventRecorded),
            20 => Ok(Self::TopologyBraidShellRetained),
            21 => Ok(Self::TopologySuffixImportRecorded),
            _ => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "WalRecordKind",
                code,
            }),
        }
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

    fn from_code(code: u8) -> Result<Self, WalDecodeError> {
        match code {
            0 => Ok(Self::None),
            _ => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "WalCompressionKind",
                code,
            }),
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

    fn from_code(code: u8) -> Result<Self, WalDecodeError> {
        match code {
            1 => Ok(Self::Present),
            2 => Ok(Self::DigestOnly),
            3 => Ok(Self::RetainedRef),
            4 => Ok(Self::Encrypted),
            5 => Ok(Self::RedactedByPolicy),
            _ => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "WalRedactionPosture",
                code,
            }),
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
    /// Topology recovery index frontier.
    TopologyIndex,
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
            Self::TopologyIndex => 7,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct WriterEpochClosure {
    final_lsn: Option<Lsn>,
    final_commit_digest: Option<Hash>,
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
    /// Computes the digest expected for the current commit fields.
    #[must_use]
    pub fn expected_digest(&self) -> Hash {
        self.compute_digest()
    }

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
        validate_transaction_frontiers(&self.affected_frontiers, self.commit.transaction_kind)?;
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

fn validate_writer_epoch_request(
    active_epoch: Option<&WriterEpoch>,
    closed_epochs: &[WriterEpoch],
    epoch_closures: &BTreeMap<WriterEpochId, WriterEpochClosure>,
    request: WriterEpochRequest,
) -> Result<WriterEpoch, WalStoreError> {
    if active_epoch.is_some() {
        return Err(WalStoreError::WriterEpochAlreadyActive);
    }
    match request.previous_epoch_id {
        Some(previous_epoch_id) => {
            let previous_epoch = closed_epochs
                .iter()
                .find(|epoch| epoch.epoch_id == previous_epoch_id)
                .ok_or(WalStoreError::UnknownPreviousWriterEpoch)?;
            let previous_closure = epoch_closures
                .get(&previous_epoch_id)
                .copied()
                .unwrap_or_default();
            if request.previous_epoch_final_commit_digest != previous_closure.final_commit_digest {
                return Err(WalStoreError::WriterEpochFinalCommitDigestMismatch);
            }
            if let Some(final_lsn) = previous_closure.final_lsn {
                if request.started_at_lsn <= final_lsn {
                    return Err(WalStoreError::WriterEpochLsnRegression);
                }
            }
            if request.storage_fencing_token == previous_epoch.storage_fencing_token
                || request.lease_or_lock_evidence == previous_epoch.lease_or_lock_evidence
            {
                return Err(WalStoreError::WriterEpochFencingMismatch);
            }
        }
        None => {
            if !closed_epochs.is_empty() {
                return Err(WalStoreError::WriterEpochChainGap);
            }
        }
    }
    Ok(WriterEpoch {
        epoch_id: request.epoch_id,
        storage_fencing_token: request.storage_fencing_token,
        process_identity: request.process_identity,
        host_identity: request.host_identity,
        started_at_lsn: request.started_at_lsn,
        previous_epoch_id: request.previous_epoch_id,
        previous_epoch_final_commit_digest: request.previous_epoch_final_commit_digest,
        lease_or_lock_evidence: request.lease_or_lock_evidence,
    })
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

/// Segment placement family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalSegmentPlacementKind {
    /// Placement is derived from logical segment id.
    CausalSegmentId,
    /// Placement is derived from wall-clock bucketing.
    WallClockPartition,
}

/// Segment placement policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalSegmentPlacementPolicy {
    /// Placement family.
    pub kind: WalSegmentPlacementKind,
    /// `true` if placement participates in authoritative WAL identity.
    pub authoritative: bool,
}

/// Segment placement policy error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WalSegmentPlacementPolicyError {
    /// Wall-clock placement cannot define authoritative WAL identity.
    #[error("wall-clock WAL segment placement cannot be authoritative")]
    WallClockPlacementCannotBeAuthoritative,
}

/// Validates a segment placement policy.
pub fn validate_segment_placement_policy(
    policy: WalSegmentPlacementPolicy,
) -> Result<(), WalSegmentPlacementPolicyError> {
    if policy.kind == WalSegmentPlacementKind::WallClockPartition && policy.authoritative {
        return Err(WalSegmentPlacementPolicyError::WallClockPlacementCannotBeAuthoritative);
    }
    Ok(())
}

/// Segment id arithmetic error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WalSegmentIdError {
    /// Segment id overflowed.
    #[error("WAL segment id overflow")]
    Overflow,
}

/// Returns the next logical segment id.
pub fn next_segment_id(previous: Option<WalSegmentId>) -> Result<WalSegmentId, WalSegmentIdError> {
    let Some(previous) = previous else {
        return Ok(WalSegmentId::from_raw(1));
    };
    previous
        .as_u64()
        .checked_add(1)
        .map(WalSegmentId::from_raw)
        .ok_or(WalSegmentIdError::Overflow)
}

/// Segment manifest entry derived from logical identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalSegmentManifestEntry {
    /// Logical segment id.
    pub segment_id: WalSegmentId,
    /// Canonical relative path.
    pub relative_path: PathBuf,
    /// Segment digest.
    pub segment_digest: Hash,
    /// First LSN included in the segment.
    pub first_lsn: Option<Lsn>,
    /// Last LSN included in the segment.
    pub last_lsn: Option<Lsn>,
}

/// Filesystem manifest validation report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalManifestValidationReport {
    /// Manifest loaded from disk.
    pub manifest: WalManifest,
    /// Number of segment files observed.
    pub segment_count: u64,
    /// Last committed LSN observed in segments.
    pub last_committed_lsn: Option<Lsn>,
    /// Last commit digest observed in segments.
    pub last_commit_digest: Option<Hash>,
}

/// Builds a segment manifest entry from segment frames.
#[must_use]
pub fn segment_manifest_entry(
    segment_id: WalSegmentId,
    frames: &[WalFrame],
) -> WalSegmentManifestEntry {
    let frame_refs = frames.iter().collect::<Vec<_>>();
    WalSegmentManifestEntry {
        segment_id,
        relative_path: canonical_segment_relative_path(segment_id),
        segment_digest: segment_digest(segment_id, &frame_refs),
        first_lsn: frames.iter().map(|frame| frame.header.lsn).min(),
        last_lsn: frames.iter().map(|frame| frame.header.lsn).max(),
    }
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
    /// Number of segment files covered by the manifest.
    pub sealed_segment_count: u64,
}

/// Canonical crashpoint execution posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalCrashpointExecution {
    /// Crashpoint is simulated in-process by Rust fixtures.
    SimulatedInProcess,
    /// Crashpoint is reserved for a future process-kill runner.
    ProcessKillFuture,
}

/// Canonical WAL crashpoint boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalCrashpointBoundary {
    /// Submission intake boundary.
    Submission,
    /// Scheduler-owned tick boundary.
    Tick,
    /// Checkpoint publication boundary.
    Checkpoint,
    /// Retained material or side-effect materialization boundary.
    Material,
    /// Index publication boundary.
    Index,
    /// Future process-kill runner boundary.
    Process,
}

/// Canonical crashpoint descriptor used by tests and future CLI/BATS runners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalCrashpointDescriptor {
    /// Stable crashpoint name.
    pub name: &'static str,
    /// Boundary family.
    pub boundary: WalCrashpointBoundary,
    /// Execution posture.
    pub execution: WalCrashpointExecution,
}

const WAL_CRASHPOINT_MANIFEST: &[WalCrashpointDescriptor] = &[
    WalCrashpointDescriptor {
        name: "submission.before_commit",
        boundary: WalCrashpointBoundary::Submission,
        execution: WalCrashpointExecution::SimulatedInProcess,
    },
    WalCrashpointDescriptor {
        name: "submission.after_commit_before_ack",
        boundary: WalCrashpointBoundary::Submission,
        execution: WalCrashpointExecution::SimulatedInProcess,
    },
    WalCrashpointDescriptor {
        name: "tick.before_commit",
        boundary: WalCrashpointBoundary::Tick,
        execution: WalCrashpointExecution::SimulatedInProcess,
    },
    WalCrashpointDescriptor {
        name: "tick.after_commit_before_publish",
        boundary: WalCrashpointBoundary::Tick,
        execution: WalCrashpointExecution::SimulatedInProcess,
    },
    WalCrashpointDescriptor {
        name: "checkpoint.before_rename",
        boundary: WalCrashpointBoundary::Checkpoint,
        execution: WalCrashpointExecution::SimulatedInProcess,
    },
    WalCrashpointDescriptor {
        name: "checkpoint.after_rename_before_publication",
        boundary: WalCrashpointBoundary::Checkpoint,
        execution: WalCrashpointExecution::SimulatedInProcess,
    },
    WalCrashpointDescriptor {
        name: "material.before_wal_reference",
        boundary: WalCrashpointBoundary::Material,
        execution: WalCrashpointExecution::SimulatedInProcess,
    },
    WalCrashpointDescriptor {
        name: "material.after_effect_before_observation",
        boundary: WalCrashpointBoundary::Material,
        execution: WalCrashpointExecution::SimulatedInProcess,
    },
    WalCrashpointDescriptor {
        name: "index.before_publish",
        boundary: WalCrashpointBoundary::Index,
        execution: WalCrashpointExecution::SimulatedInProcess,
    },
    WalCrashpointDescriptor {
        name: "process.kill.after_wal_commit",
        boundary: WalCrashpointBoundary::Process,
        execution: WalCrashpointExecution::ProcessKillFuture,
    },
];

/// Returns the canonical WAL crashpoint manifest.
#[must_use]
pub const fn wal_crashpoint_manifest() -> &'static [WalCrashpointDescriptor] {
    WAL_CRASHPOINT_MANIFEST
}

/// Filesystem sync boundary evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FilesystemSyncBoundary {
    /// WAL root directory was synced after creating the segment namespace.
    SegmentNamespaceDirectorySynced,
    /// Segment file was created and synced.
    SegmentFileCreated,
    /// Containing directory was synced after segment creation.
    SegmentDirectorySynced,
    /// Commit marker append synced the WAL segment file.
    CommitFileSynced,
    /// Manifest temp file was synced before rename.
    ManifestTempFileSynced,
    /// Containing directory was synced after manifest rename.
    ManifestRenamedDirectorySynced,
    /// Checkpoint temp file was synced before rename.
    CheckpointTempFileSynced,
    /// Containing directory was synced after checkpoint rename.
    CheckpointRenamedDirectorySynced,
}

/// Filesystem sync evidence emitted by strict filesystem helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FilesystemSyncEvidence {
    /// Sync boundary.
    pub boundary: FilesystemSyncBoundary,
    /// Segment id, when the boundary is segment-scoped.
    pub segment_id: Option<WalSegmentId>,
    /// Transaction id, when the boundary is commit-scoped.
    pub transaction_id: Option<WalTransactionId>,
}

impl FilesystemSyncEvidence {
    const fn segment(boundary: FilesystemSyncBoundary, segment_id: WalSegmentId) -> Self {
        Self {
            boundary,
            segment_id: Some(segment_id),
            transaction_id: None,
        }
    }

    const fn transaction(
        boundary: FilesystemSyncBoundary,
        transaction_id: WalTransactionId,
    ) -> Self {
        Self {
            boundary,
            segment_id: None,
            transaction_id: Some(transaction_id),
        }
    }

    const fn unscoped(boundary: FilesystemSyncBoundary) -> Self {
        Self {
            boundary,
            segment_id: None,
            transaction_id: None,
        }
    }
}

/// Filesystem sync evidence validation error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FilesystemSyncEvidenceError {
    /// Required sync evidence was missing.
    #[error("strict filesystem WAL missing sync evidence for {0:?}")]
    Missing(FilesystemSyncBoundary),
}

/// Validates strict filesystem sync evidence for required boundaries.
pub fn validate_filesystem_strict_sync_evidence(
    evidence: &[FilesystemSyncEvidence],
    required: &[FilesystemSyncBoundary],
) -> Result<(), FilesystemSyncEvidenceError> {
    let present = evidence
        .iter()
        .map(|entry| entry.boundary)
        .collect::<BTreeSet<_>>();
    for boundary in required {
        if !present.contains(boundary) {
            return Err(FilesystemSyncEvidenceError::Missing(*boundary));
        }
    }
    Ok(())
}

/// Deterministic in-memory WAL store.
#[derive(Clone, Debug, Default)]
pub struct InMemoryWalStore {
    active_epoch: Option<WriterEpoch>,
    closed_epochs: Vec<WriterEpoch>,
    epoch_closures: BTreeMap<WriterEpochId, WriterEpochClosure>,
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
        let epoch = validate_writer_epoch_request(
            self.active_epoch.as_ref(),
            &self.closed_epochs,
            &self.epoch_closures,
            request,
        )?;
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
        self.epoch_closures.insert(
            epoch_id,
            WriterEpochClosure {
                final_lsn: Some(commit.last_lsn),
                final_commit_digest: Some(commit.commit_digest),
            },
        );
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
        self.epoch_closures.entry(epoch_id).or_default();
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

impl RecoveryScanReport {
    /// Returns the first committed LSN in the scan.
    #[must_use]
    pub fn first_committed_lsn(&self) -> Option<Lsn> {
        self.transactions
            .iter()
            .map(|transaction| transaction.commit.first_lsn)
            .min()
    }

    /// Returns the last committed LSN in the scan.
    #[must_use]
    pub fn last_committed_lsn(&self) -> Option<Lsn> {
        self.transactions
            .iter()
            .map(|transaction| transaction.commit.last_lsn)
            .max()
    }

    /// Returns the last committed transaction digest in replay order.
    #[must_use]
    pub fn last_commit_digest(&self) -> Option<Hash> {
        self.transactions
            .last()
            .map(|transaction| transaction.commit.commit_digest)
    }
}

/// Recovered committed transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalRecoveredTransaction {
    /// Commit marker.
    pub commit: WalTransactionCommit,
    /// Transaction frames.
    pub frames: Vec<WalFrame>,
}

/// Recovery evidence decoded from embedded WAL segment bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalSegmentBytesRecovery {
    /// Logical segment id.
    pub segment_id: WalSegmentId,
    /// Digest computed from recovered segment frames.
    pub segment_digest: Hash,
    /// Recovery report built from segment frames and commit markers.
    pub report: RecoveryScanReport,
}

/// WAL submission acceptance record payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubmissionAcceptanceRecord {
    /// Stable submission id.
    pub submission_id: Hash,
    /// Canonical envelope digest accepted by Echo.
    pub canonical_envelope_digest: Hash,
    /// Optional explicit idempotency/dedupe key.
    pub idempotency_key_digest: Option<Hash>,
    /// Acceptance evidence digest returned to the caller after durable commit.
    pub acceptance_evidence_digest: Hash,
}

impl SubmissionAcceptanceRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.submission_id);
        push_hash(&mut out, &self.canonical_envelope_digest);
        push_optional_hash(&mut out, self.idempotency_key_digest);
        push_hash(&mut out, &self.acceptance_evidence_digest);
        out
    }

    /// Decodes a deterministic submission acceptance payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let submission_id = cursor.read_hash()?;
        let canonical_envelope_digest = cursor.read_hash()?;
        let idempotency_key_digest = cursor.read_optional_hash()?;
        let acceptance_evidence_digest = cursor.read_hash()?;
        cursor.finish()?;
        Ok(Self {
            submission_id,
            canonical_envelope_digest,
            idempotency_key_digest,
            acceptance_evidence_digest,
        })
    }
}

/// Scheduler-owned tick decision captured by a WAL receipt record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalTickDecision {
    /// The scheduler applied the work.
    Applied,
    /// The scheduler lawfully rejected the work for a footprint conflict.
    RejectedFootprintConflict,
    /// The work is obstructed by retained material or runtime support posture.
    Obstructed,
}

impl WalTickDecision {
    fn code(self) -> u8 {
        match self {
            Self::Applied => 1,
            Self::RejectedFootprintConflict => 2,
            Self::Obstructed => 3,
        }
    }

    fn from_code(code: u8) -> Result<Self, WalDecodeError> {
        match code {
            1 => Ok(Self::Applied),
            2 => Ok(Self::RejectedFootprintConflict),
            3 => Ok(Self::Obstructed),
            _ => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "WalTickDecision",
                code,
            }),
        }
    }

    /// Returns `true` when this decision is a lawful rejection, not a fault.
    #[must_use]
    pub const fn is_lawful_rejection(self) -> bool {
        matches!(self, Self::RejectedFootprintConflict)
    }
}

/// WAL tick receipt record payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickReceiptRecord {
    /// Submission decided by the receipt.
    pub submission_id: Hash,
    /// Admission ticket digest.
    pub ticket_digest: Hash,
    /// Tick receipt digest.
    pub receipt_digest: Hash,
    /// Scheduler decision.
    pub decision: WalTickDecision,
}

impl TickReceiptRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.submission_id);
        push_hash(&mut out, &self.ticket_digest);
        push_hash(&mut out, &self.receipt_digest);
        out.push(self.decision.code());
        out
    }

    /// Decodes a deterministic tick receipt payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let submission_id = cursor.read_hash()?;
        let ticket_digest = cursor.read_hash()?;
        let receipt_digest = cursor.read_hash()?;
        let decision = WalTickDecision::from_code(cursor.read_u8()?)?;
        cursor.finish()?;
        Ok(Self {
            submission_id,
            ticket_digest,
            receipt_digest,
            decision,
        })
    }
}

/// WAL receipt correlation record payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalReceiptCorrelationRecord {
    /// Submission correlated to the receipt.
    pub submission_id: Hash,
    /// Admission ticket digest.
    pub ticket_digest: Hash,
    /// Tick receipt digest.
    pub receipt_digest: Hash,
}

impl WalReceiptCorrelationRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.submission_id);
        push_hash(&mut out, &self.ticket_digest);
        push_hash(&mut out, &self.receipt_digest);
        out
    }

    /// Decodes a deterministic receipt correlation payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let submission_id = cursor.read_hash()?;
        let ticket_digest = cursor.read_hash()?;
        let receipt_digest = cursor.read_hash()?;
        cursor.finish()?;
        Ok(Self {
            submission_id,
            ticket_digest,
            receipt_digest,
        })
    }
}

/// Retained material family referenced by committed WAL history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RetainedMaterialKind {
    /// Canonical submission payload material.
    SubmissionPayload,
    /// Tick receipt material.
    TickReceipt,
    /// Runtime state delta material.
    RuntimeStateDelta,
    /// Runtime control posture material.
    RuntimeControl,
    /// Reading payload material.
    ReadingPayload,
    /// Reading envelope material.
    ReadingEnvelope,
    /// Diagnostic-only material.
    Diagnostic,
}

impl RetainedMaterialKind {
    fn code(self) -> u8 {
        match self {
            Self::SubmissionPayload => 1,
            Self::TickReceipt => 2,
            Self::RuntimeStateDelta => 3,
            Self::RuntimeControl => 4,
            Self::ReadingPayload => 5,
            Self::ReadingEnvelope => 6,
            Self::Diagnostic => 7,
        }
    }

    fn from_code(code: u8) -> Result<Self, WalDecodeError> {
        match code {
            1 => Ok(Self::SubmissionPayload),
            2 => Ok(Self::TickReceipt),
            3 => Ok(Self::RuntimeStateDelta),
            4 => Ok(Self::RuntimeControl),
            5 => Ok(Self::ReadingPayload),
            6 => Ok(Self::ReadingEnvelope),
            7 => Ok(Self::Diagnostic),
            _ => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "RetainedMaterialKind",
                code,
            }),
        }
    }
}

/// Evidence posture for retained material and readings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceMaterialPosture {
    /// Material is present.
    Present,
    /// Material is hidden by revelation policy.
    RedactedByPolicy,
    /// Material is encrypted and the key is unavailable.
    EncryptedKeyUnavailable,
    /// Material is missing.
    Missing,
    /// Material is corrupt.
    Corrupt,
    /// Material is obstructed by causal/runtime posture.
    Obstructed,
}

impl EvidenceMaterialPosture {
    fn code(self) -> u8 {
        match self {
            Self::Present => 1,
            Self::RedactedByPolicy => 2,
            Self::EncryptedKeyUnavailable => 3,
            Self::Missing => 4,
            Self::Corrupt => 5,
            Self::Obstructed => 6,
        }
    }

    fn from_code(code: u8) -> Result<Self, WalDecodeError> {
        match code {
            1 => Ok(Self::Present),
            2 => Ok(Self::RedactedByPolicy),
            3 => Ok(Self::EncryptedKeyUnavailable),
            4 => Ok(Self::Missing),
            5 => Ok(Self::Corrupt),
            6 => Ok(Self::Obstructed),
            _ => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "EvidenceMaterialPosture",
                code,
            }),
        }
    }
}

/// Inspector-facing material posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialInspectionPosture {
    /// Material is available.
    Present,
    /// Material is intentionally hidden by policy.
    PolicyHidden,
    /// Material is encrypted and the key is unavailable.
    EncryptedKeyUnavailable,
    /// Material is missing from storage.
    Missing,
    /// Material bytes are corrupt.
    Corrupt,
    /// Material is blocked by runtime or causal posture.
    Obstructed,
}

/// Converts retained material posture into an explicit inspector posture.
#[must_use]
pub const fn inspect_evidence_material_posture(
    posture: EvidenceMaterialPosture,
) -> MaterialInspectionPosture {
    match posture {
        EvidenceMaterialPosture::Present => MaterialInspectionPosture::Present,
        EvidenceMaterialPosture::RedactedByPolicy => MaterialInspectionPosture::PolicyHidden,
        EvidenceMaterialPosture::EncryptedKeyUnavailable => {
            MaterialInspectionPosture::EncryptedKeyUnavailable
        }
        EvidenceMaterialPosture::Missing => MaterialInspectionPosture::Missing,
        EvidenceMaterialPosture::Corrupt => MaterialInspectionPosture::Corrupt,
        EvidenceMaterialPosture::Obstructed => MaterialInspectionPosture::Obstructed,
    }
}

/// WAL retained material reference payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedMaterialRecord {
    /// Material digest.
    pub material_digest: Hash,
    /// Semantic coordinate digest naming why the material matters.
    pub semantic_coordinate_digest: Hash,
    /// Material family.
    pub kind: RetainedMaterialKind,
    /// Material posture.
    pub posture: EvidenceMaterialPosture,
}

impl RetainedMaterialRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.material_digest);
        push_hash(&mut out, &self.semantic_coordinate_digest);
        out.push(self.kind.code());
        out.push(self.posture.code());
        out
    }

    /// Decodes a deterministic retained material payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let material_digest = cursor.read_hash()?;
        let semantic_coordinate_digest = cursor.read_hash()?;
        let kind = RetainedMaterialKind::from_code(cursor.read_u8()?)?;
        let posture = EvidenceMaterialPosture::from_code(cursor.read_u8()?)?;
        cursor.finish()?;
        Ok(Self {
            material_digest,
            semantic_coordinate_digest,
            kind,
            posture,
        })
    }
}

/// WAL reading reference payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadingRefRecord {
    /// Reading identity digest.
    pub reading_id: Hash,
    /// Query or reading semantic coordinate digest.
    pub semantic_coordinate_digest: Hash,
    /// Retained payload digest.
    pub payload_digest: Hash,
    /// Retained envelope digest.
    pub envelope_digest: Hash,
    /// Reading evidence posture.
    pub posture: EvidenceMaterialPosture,
}

impl ReadingRefRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.reading_id);
        push_hash(&mut out, &self.semantic_coordinate_digest);
        push_hash(&mut out, &self.payload_digest);
        push_hash(&mut out, &self.envelope_digest);
        out.push(self.posture.code());
        out
    }

    /// Decodes a deterministic retained reading payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let reading_id = cursor.read_hash()?;
        let semantic_coordinate_digest = cursor.read_hash()?;
        let payload_digest = cursor.read_hash()?;
        let envelope_digest = cursor.read_hash()?;
        let posture = EvidenceMaterialPosture::from_code(cursor.read_u8()?)?;
        cursor.finish()?;
        Ok(Self {
            reading_id,
            semantic_coordinate_digest,
            payload_digest,
            envelope_digest,
            posture,
        })
    }
}

/// Scope of a retained-material recovery obstruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingMaterialScope {
    /// One recovered submission is obstructed.
    Submission,
    /// A receipt or ticket correlation is obstructed.
    ReceiptOrTicket,
    /// Runtime recovery must fault globally.
    RuntimeGlobal,
    /// A retained reading is obstructed.
    Reading,
    /// Diagnostic material loss does not block causal recovery.
    DiagnosticLoss,
}

/// Classifies the recovery scope for missing retained material.
#[must_use]
pub const fn missing_material_scope(kind: RetainedMaterialKind) -> MissingMaterialScope {
    match kind {
        RetainedMaterialKind::SubmissionPayload => MissingMaterialScope::Submission,
        RetainedMaterialKind::TickReceipt => MissingMaterialScope::ReceiptOrTicket,
        RetainedMaterialKind::RuntimeStateDelta | RetainedMaterialKind::RuntimeControl => {
            MissingMaterialScope::RuntimeGlobal
        }
        RetainedMaterialKind::ReadingPayload | RetainedMaterialKind::ReadingEnvelope => {
            MissingMaterialScope::Reading
        }
        RetainedMaterialKind::Diagnostic => MissingMaterialScope::DiagnosticLoss,
    }
}

/// Recovered submission posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveredSubmissionPosture {
    /// Submission was accepted and has no committed decision yet.
    AcceptedPending,
    /// Submission was decided as applied.
    DecidedApplied,
    /// Submission was lawfully rejected.
    DecidedRejected,
    /// Submission is obstructed.
    Obstructed,
    /// Recovery found a fault for this submission.
    RecoveryFaulted,
}

/// Retry posture for a submitted id/envelope pair after recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionRetryPosture {
    /// No recovered acceptance exists for this submission id.
    NotAccepted,
    /// Same id and envelope recovered as accepted pending.
    AlreadyAcceptedPending,
    /// Same id and envelope recovered as decided applied.
    AlreadyDecidedApplied,
    /// Same id and envelope recovered as decided rejected.
    AlreadyDecidedRejected,
    /// Same id and envelope recovered as obstructed.
    AlreadyObstructed,
    /// Same id recovered with a different canonical envelope digest.
    ConflictSameIdDifferentEnvelope,
    /// New id with same envelope should be treated as a new submission unless
    /// an explicit dedupe policy says otherwise.
    NewSubmissionWithoutPolicyDedupe,
}

/// Recovered submission entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveredSubmissionEntry {
    /// Submission acceptance record.
    pub acceptance: SubmissionAcceptanceRecord,
    /// Current recovered posture.
    pub posture: RecoveredSubmissionPosture,
    /// Deciding receipt digest, if any.
    pub receipt_digest: Option<Hash>,
}

/// Recovered submission index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveredSubmissionIndex {
    submissions: BTreeMap<Hash, RecoveredSubmissionEntry>,
    envelope_to_submissions: BTreeMap<Hash, BTreeSet<Hash>>,
}

impl RecoveredSubmissionIndex {
    /// Builds a recovered index from accepted submission records.
    ///
    /// Every recovered record starts as accepted pending because no scheduler
    /// decision material is supplied by this constructor.
    ///
    /// # Errors
    ///
    /// Returns [`WalRecoveryIndexError::SubmissionEnvelopeConflict`] when one
    /// submission id is associated with conflicting canonical envelope digests.
    pub fn from_acceptance_records<I>(records: I) -> Result<Self, WalRecoveryIndexError>
    where
        I: IntoIterator<Item = SubmissionAcceptanceRecord>,
    {
        let mut index = Self::default();
        for record in records {
            index.insert_acceptance_record(record)?;
        }
        Ok(index)
    }

    /// Builds a recovered index from accepted submission and tick receipt records.
    ///
    /// Accepted submissions with no matching receipt remain accepted pending.
    /// Matching receipt records move accepted submissions to their decided or
    /// obstructed posture without inventing retries.
    ///
    /// # Errors
    ///
    /// Returns [`WalRecoveryIndexError::SubmissionEnvelopeConflict`] when one
    /// submission id is associated with conflicting canonical envelope digests.
    pub fn from_acceptance_and_receipt_records<I, J>(
        acceptances: I,
        receipts: J,
    ) -> Result<Self, WalRecoveryIndexError>
    where
        I: IntoIterator<Item = SubmissionAcceptanceRecord>,
        J: IntoIterator<Item = TickReceiptRecord>,
    {
        let mut index = Self::from_acceptance_records(acceptances)?;
        for receipt in receipts {
            index.apply_tick_receipt_record(receipt);
        }
        Ok(index)
    }

    fn insert_acceptance_record(
        &mut self,
        record: SubmissionAcceptanceRecord,
    ) -> Result<(), WalRecoveryIndexError> {
        if let Some(existing) = self.submissions.get(&record.submission_id) {
            if existing.acceptance.canonical_envelope_digest != record.canonical_envelope_digest {
                return Err(WalRecoveryIndexError::SubmissionEnvelopeConflict {
                    submission_id: record.submission_id,
                });
            }
        }
        self.envelope_to_submissions
            .entry(record.canonical_envelope_digest)
            .or_default()
            .insert(record.submission_id);
        self.submissions
            .entry(record.submission_id)
            .or_insert(RecoveredSubmissionEntry {
                acceptance: record,
                posture: RecoveredSubmissionPosture::AcceptedPending,
                receipt_digest: None,
            });
        Ok(())
    }

    fn apply_tick_receipt_record(&mut self, receipt: TickReceiptRecord) {
        if let Some(entry) = self.submissions.get_mut(&receipt.submission_id) {
            entry.posture = match receipt.decision {
                WalTickDecision::Applied => RecoveredSubmissionPosture::DecidedApplied,
                WalTickDecision::RejectedFootprintConflict => {
                    RecoveredSubmissionPosture::DecidedRejected
                }
                WalTickDecision::Obstructed => RecoveredSubmissionPosture::Obstructed,
            };
            entry.receipt_digest = Some(receipt.receipt_digest);
        }
    }

    /// Returns a recovered submission entry.
    #[must_use]
    pub fn get(&self, submission_id: &Hash) -> Option<&RecoveredSubmissionEntry> {
        self.submissions.get(submission_id)
    }

    /// Returns the number of recovered submissions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.submissions.len()
    }

    /// Returns `true` when the index contains no submissions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.submissions.is_empty()
    }

    /// Classifies retry posture for a submission id and canonical envelope.
    #[must_use]
    pub fn retry_posture(
        &self,
        submission_id: Hash,
        canonical_envelope_digest: Hash,
    ) -> SubmissionRetryPosture {
        let Some(entry) = self.submissions.get(&submission_id) else {
            if self
                .envelope_to_submissions
                .get(&canonical_envelope_digest)
                .is_some_and(|ids| !ids.is_empty())
            {
                return SubmissionRetryPosture::NewSubmissionWithoutPolicyDedupe;
            }
            return SubmissionRetryPosture::NotAccepted;
        };
        if entry.acceptance.canonical_envelope_digest != canonical_envelope_digest {
            return SubmissionRetryPosture::ConflictSameIdDifferentEnvelope;
        }
        match entry.posture {
            RecoveredSubmissionPosture::AcceptedPending => {
                SubmissionRetryPosture::AlreadyAcceptedPending
            }
            RecoveredSubmissionPosture::DecidedApplied => {
                SubmissionRetryPosture::AlreadyDecidedApplied
            }
            RecoveredSubmissionPosture::DecidedRejected => {
                SubmissionRetryPosture::AlreadyDecidedRejected
            }
            RecoveredSubmissionPosture::Obstructed
            | RecoveredSubmissionPosture::RecoveryFaulted => {
                SubmissionRetryPosture::AlreadyObstructed
            }
        }
    }
}

/// Builds a stable root over recovered submission and receipt indexes.
#[must_use]
pub fn recovered_submission_receipt_index_root(
    submissions: &RecoveredSubmissionIndex,
    receipts: &RecoveredReceiptIndex,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(WAL_RECOVERED_INDEX_ROOT_DOMAIN);
    hasher.update(&len_u64(submissions.submissions.len()).to_le_bytes());
    for (submission_id, entry) in &submissions.submissions {
        hasher.update(b"submission");
        hasher.update(submission_id);
        hasher.update(&entry.acceptance.submission_id);
        hasher.update(&entry.acceptance.canonical_envelope_digest);
        match entry.acceptance.idempotency_key_digest {
            Some(digest) => {
                hasher.update(&[1]);
                hasher.update(&digest);
            }
            None => {
                hasher.update(&[0]);
            }
        }
        hasher.update(&entry.acceptance.acceptance_evidence_digest);
        hasher.update(&[recovered_submission_posture_code(entry.posture)]);
        match entry.receipt_digest {
            Some(digest) => {
                hasher.update(&[1]);
                hasher.update(&digest);
            }
            None => {
                hasher.update(&[0]);
            }
        }
    }
    hasher.update(&len_u64(submissions.envelope_to_submissions.len()).to_le_bytes());
    for (envelope_digest, submission_ids) in &submissions.envelope_to_submissions {
        hasher.update(b"envelope");
        hasher.update(envelope_digest);
        hasher.update(&len_u64(submission_ids.len()).to_le_bytes());
        for submission_id in submission_ids {
            hasher.update(submission_id);
        }
    }
    hasher.update(&len_u64(receipts.receipt_by_submission.len()).to_le_bytes());
    for (submission_id, receipt_digest) in &receipts.receipt_by_submission {
        hasher.update(b"receipt-by-submission");
        hasher.update(submission_id);
        hasher.update(receipt_digest);
    }
    hasher.update(&len_u64(receipts.receipt_by_ticket.len()).to_le_bytes());
    for (ticket_digest, receipt_digest) in &receipts.receipt_by_ticket {
        hasher.update(b"receipt-by-ticket");
        hasher.update(ticket_digest);
        hasher.update(receipt_digest);
    }
    hasher.update(&len_u64(receipts.ticket_by_submission.len()).to_le_bytes());
    for (submission_id, ticket_digest) in &receipts.ticket_by_submission {
        hasher.update(b"ticket-by-submission");
        hasher.update(submission_id);
        hasher.update(ticket_digest);
    }
    hasher.update(&len_u64(receipts.decisions_by_receipt.len()).to_le_bytes());
    for (receipt_digest, decision) in &receipts.decisions_by_receipt {
        hasher.update(b"decision-by-receipt");
        hasher.update(receipt_digest);
        hasher.update(&[decision.code()]);
    }
    hasher.finalize().into()
}

fn recovered_submission_posture_code(posture: RecoveredSubmissionPosture) -> u8 {
    match posture {
        RecoveredSubmissionPosture::AcceptedPending => 1,
        RecoveredSubmissionPosture::DecidedApplied => 2,
        RecoveredSubmissionPosture::DecidedRejected => 3,
        RecoveredSubmissionPosture::Obstructed => 4,
        RecoveredSubmissionPosture::RecoveryFaulted => 5,
    }
}

/// Recovered receipt correlation index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveredReceiptIndex {
    /// Receipt by submission id.
    pub receipt_by_submission: BTreeMap<Hash, Hash>,
    /// Receipt by admission ticket digest.
    pub receipt_by_ticket: BTreeMap<Hash, Hash>,
    /// Ticket by submission id.
    pub ticket_by_submission: BTreeMap<Hash, Hash>,
    /// Decisions by receipt digest.
    pub decisions_by_receipt: BTreeMap<Hash, WalTickDecision>,
}

impl RecoveredReceiptIndex {
    /// Builds a recovered receipt index from receipt and correlation records.
    ///
    /// Tick receipt records carry decision posture. Correlation records can
    /// restore ticket/submission/receipt lookup handles when decision material
    /// is not present in the same source.
    #[must_use]
    pub fn from_receipt_correlation_records<I, J>(receipts: I, correlations: J) -> Self
    where
        I: IntoIterator<Item = TickReceiptRecord>,
        J: IntoIterator<Item = WalReceiptCorrelationRecord>,
    {
        let mut index = Self::default();
        for receipt in receipts {
            index
                .receipt_by_submission
                .insert(receipt.submission_id, receipt.receipt_digest);
            index
                .receipt_by_ticket
                .insert(receipt.ticket_digest, receipt.receipt_digest);
            index
                .ticket_by_submission
                .insert(receipt.submission_id, receipt.ticket_digest);
            index
                .decisions_by_receipt
                .insert(receipt.receipt_digest, receipt.decision);
        }
        for correlation in correlations {
            index
                .receipt_by_submission
                .insert(correlation.submission_id, correlation.receipt_digest);
            index
                .receipt_by_ticket
                .insert(correlation.ticket_digest, correlation.receipt_digest);
            index
                .ticket_by_submission
                .insert(correlation.submission_id, correlation.ticket_digest);
        }
        index
    }
}

/// Recovered retained material and reading index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveredRetentionIndex {
    /// Retained material by digest.
    pub material_by_digest: BTreeMap<Hash, RetainedMaterialRecord>,
    /// Retained material by semantic coordinate.
    pub material_by_semantic_coordinate: BTreeMap<Hash, BTreeSet<Hash>>,
    /// Retained reading by reading id.
    pub reading_by_id: BTreeMap<Hash, ReadingRefRecord>,
    /// Reading ids by semantic coordinate.
    pub readings_by_semantic_coordinate: BTreeMap<Hash, BTreeSet<Hash>>,
}

/// Error returned while building a recovered retention index.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum RecoveredRetentionIndexError {
    /// A material digest mapped to conflicting retained material records.
    #[error("retained material digest maps to conflicting records")]
    ConflictingMaterialDigest {
        /// Conflicting retained material digest.
        material_digest: Hash,
    },
    /// A reading id mapped to conflicting reading reference records.
    #[error("retained reading id maps to conflicting records")]
    ConflictingReadingId {
        /// Conflicting reading id.
        reading_id: Hash,
    },
}

impl RecoveredRetentionIndex {
    /// Builds a recovered retention index from retained material and reading records.
    ///
    /// # Errors
    ///
    /// Returns an error when retained evidence records carry conflicting
    /// material digests or reading ids.
    pub fn from_retention_records<I, J>(
        materials: I,
        readings: J,
    ) -> Result<Self, RecoveredRetentionIndexError>
    where
        I: IntoIterator<Item = RetainedMaterialRecord>,
        J: IntoIterator<Item = ReadingRefRecord>,
    {
        let mut index = Self::default();
        for record in materials {
            if let Some(existing) = index.material_by_digest.get(&record.material_digest) {
                if existing != &record {
                    return Err(RecoveredRetentionIndexError::ConflictingMaterialDigest {
                        material_digest: record.material_digest,
                    });
                }
            } else {
                index
                    .material_by_digest
                    .insert(record.material_digest, record);
            }
            index
                .material_by_semantic_coordinate
                .entry(record.semantic_coordinate_digest)
                .or_default()
                .insert(record.material_digest);
        }
        for record in readings {
            if let Some(existing) = index.reading_by_id.get(&record.reading_id) {
                if existing != &record {
                    return Err(RecoveredRetentionIndexError::ConflictingReadingId {
                        reading_id: record.reading_id,
                    });
                }
            } else {
                index.reading_by_id.insert(record.reading_id, record);
            }
            index
                .readings_by_semantic_coordinate
                .entry(record.semantic_coordinate_digest)
                .or_default()
                .insert(record.reading_id);
        }
        Ok(index)
    }
}

/// Retained material obstruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedMaterialObstruction {
    /// Missing or obstructed material digest.
    pub material_digest: Hash,
    /// Material kind.
    pub kind: RetainedMaterialKind,
    /// Recovery scope.
    pub scope: MissingMaterialScope,
    /// Evidence posture.
    pub posture: EvidenceMaterialPosture,
}

/// WAL checkpoint record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointRecord {
    /// Stable checkpoint id.
    pub checkpoint_id: Hash,
    /// Last included LSN.
    pub last_included_lsn: Lsn,
    /// Last included commit digest.
    pub last_included_commit_digest: Hash,
    /// Runtime state root.
    pub state_root: Hash,
    /// Rebuilt index root.
    pub index_root: Hash,
    /// Retained material root.
    pub retained_material_root: Hash,
    /// Checkpoint schema version.
    pub schema_version: u16,
    /// Digest of the WAL chain used to create the checkpoint.
    pub created_from_wal_digest: Hash,
}

impl CheckpointRecord {
    /// Computes the checkpoint digest.
    #[must_use]
    pub fn checkpoint_digest(&self) -> Hash {
        let mut h = blake3::Hasher::new();
        h.update(b"echo:causal_wal:checkpoint:v1\0");
        h.update(&self.checkpoint_id);
        h.update(&self.last_included_lsn.as_u64().to_le_bytes());
        h.update(&self.last_included_commit_digest);
        h.update(&self.state_root);
        h.update(&self.index_root);
        h.update(&self.retained_material_root);
        h.update(&self.schema_version.to_le_bytes());
        h.update(&self.created_from_wal_digest);
        h.finalize().into()
    }

    /// Encodes the checkpoint as deterministic payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.checkpoint_id);
        out.extend_from_slice(&self.last_included_lsn.as_u64().to_le_bytes());
        push_hash(&mut out, &self.last_included_commit_digest);
        push_hash(&mut out, &self.state_root);
        push_hash(&mut out, &self.index_root);
        push_hash(&mut out, &self.retained_material_root);
        out.extend_from_slice(&self.schema_version.to_le_bytes());
        push_hash(&mut out, &self.created_from_wal_digest);
        out
    }

    /// Decodes a deterministic checkpoint payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let checkpoint_id = cursor.read_hash()?;
        let last_included_lsn = Lsn::from_raw(cursor.read_u64()?);
        let last_included_commit_digest = cursor.read_hash()?;
        let state_root = cursor.read_hash()?;
        let index_root = cursor.read_hash()?;
        let retained_material_root = cursor.read_hash()?;
        let schema_version = cursor.read_u16()?;
        let created_from_wal_digest = cursor.read_hash()?;
        cursor.finish()?;
        Ok(Self {
            checkpoint_id,
            last_included_lsn,
            last_included_commit_digest,
            state_root,
            index_root,
            retained_material_root,
            schema_version,
            created_from_wal_digest,
        })
    }
}

/// WAL checkpoint publication record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointPublicationRecord {
    /// Published checkpoint id.
    pub checkpoint_id: Hash,
    /// Published checkpoint digest.
    pub checkpoint_digest: Hash,
}

impl CheckpointPublicationRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.checkpoint_id);
        push_hash(&mut out, &self.checkpoint_digest);
        out
    }

    /// Decodes a deterministic checkpoint publication payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let checkpoint_id = cursor.read_hash()?;
        let checkpoint_digest = cursor.read_hash()?;
        cursor.finish()?;
        Ok(Self {
            checkpoint_id,
            checkpoint_digest,
        })
    }
}

/// Checkpoint validation posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointValidationPosture {
    /// Checkpoint validates and no publication record is required for use.
    UsableWithoutPublicationRecord,
    /// Checkpoint validates and publication evidence matches.
    PublishedAndUsable,
    /// Publication evidence exists but checkpoint material is missing.
    PublishedCheckpointMaterialMissing,
    /// Checkpoint does not validate against recovered WAL.
    Invalid,
}

/// Recovery certificate produced after WAL recovery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryCertificate {
    /// Checkpoint digest used as replay base, if any.
    pub checkpoint_used: Option<Hash>,
    /// First scanned committed LSN.
    pub first_lsn: Option<Lsn>,
    /// Last scanned committed LSN.
    pub last_lsn: Option<Lsn>,
    /// Number of committed transactions replayed.
    pub committed_transactions_replayed: u64,
    /// Tail posture.
    pub tail_posture: RecoveryTailPosture,
    /// Obstruction count.
    pub obstruction_count: u64,
    /// Final frontier root.
    pub recovered_frontier_root: Hash,
    /// Final index root.
    pub recovered_indexes_root: Hash,
}

/// Read-model projection of a WAL root.
///
/// This record is evidence about committed WAL history. It is not a
/// [`WalStorePort`], does not grant storage authority, and intentionally carries
/// only typed references suitable for graph projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalRoot {
    /// Projection root digest.
    pub root_digest: Hash,
    /// Writer epochs covered by this projection root.
    pub writer_epochs: Vec<WalWriterEpoch>,
    /// Segment references covered by this projection root.
    pub segments: Vec<WalSegmentRef>,
    /// Recovery certificate reference, if recovery produced one.
    pub recovery_certificate: Option<RecoveryCertificateRef>,
}

impl WalRoot {
    /// Computes a stable read-model identity digest for the projected WAL root.
    ///
    /// Segment storage locators are excluded from this digest by delegating to
    /// [`WalSegmentRef::identity_digest`].
    #[must_use]
    pub fn identity_digest(&self) -> Hash {
        let mut h = blake3::Hasher::new();
        h.update(WAL_PROJECTION_ROOT_DOMAIN);
        h.update(&self.root_digest);
        let mut writer_epoch_digests = self
            .writer_epochs
            .iter()
            .map(WalWriterEpoch::identity_digest)
            .collect::<Vec<_>>();
        writer_epoch_digests.sort_unstable();
        h.update(&len_u64(writer_epoch_digests.len()).to_le_bytes());
        for writer_epoch_digest in &writer_epoch_digests {
            h.update(writer_epoch_digest);
        }

        let mut segment_digests = self
            .segments
            .iter()
            .map(WalSegmentRef::identity_digest)
            .collect::<Vec<_>>();
        segment_digests.sort_unstable();
        h.update(&len_u64(segment_digests.len()).to_le_bytes());
        for segment_digest in &segment_digests {
            h.update(segment_digest);
        }
        update_optional_projection_digest(
            &mut h,
            self.recovery_certificate
                .as_ref()
                .map(RecoveryCertificateRef::identity_digest),
        );
        h.finalize().into()
    }
}

/// Read-model projection of WAL writer epoch metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalWriterEpoch {
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

impl WalWriterEpoch {
    /// Builds a projection record from writer-epoch evidence.
    #[must_use]
    pub fn from_writer_epoch(epoch: &WriterEpoch) -> Self {
        Self {
            epoch_id: epoch.epoch_id,
            storage_fencing_token: epoch.storage_fencing_token,
            process_identity: epoch.process_identity,
            host_identity: epoch.host_identity,
            started_at_lsn: epoch.started_at_lsn,
            previous_epoch_id: epoch.previous_epoch_id,
            previous_epoch_final_commit_digest: epoch.previous_epoch_final_commit_digest,
            lease_or_lock_evidence: epoch.lease_or_lock_evidence,
        }
    }

    /// Computes a stable read-model identity digest for this writer epoch.
    #[must_use]
    pub fn identity_digest(&self) -> Hash {
        let mut h = blake3::Hasher::new();
        h.update(WAL_PROJECTION_WRITER_EPOCH_DOMAIN);
        h.update(&self.epoch_id.as_hash());
        h.update(&self.storage_fencing_token);
        h.update(&self.process_identity);
        h.update(&self.host_identity);
        h.update(&self.started_at_lsn.as_u64().to_le_bytes());
        update_optional_writer_epoch_id(&mut h, self.previous_epoch_id);
        update_optional_projection_digest(&mut h, self.previous_epoch_final_commit_digest);
        h.update(&self.lease_or_lock_evidence);
        h.finalize().into()
    }
}

/// Commit anchor retained by a WAL segment projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalCommitAnchor {
    /// Transaction id covered by the anchor.
    pub transaction_id: WalTransactionId,
    /// Commit digest covered by the anchor.
    pub commit_digest: Hash,
    /// First LSN covered by the committed transaction.
    pub first_lsn: Lsn,
    /// Last LSN covered by the committed transaction.
    pub last_lsn: Lsn,
    /// Number of records covered by the committed transaction.
    pub record_count: u64,
}

impl WalCommitAnchor {
    /// Builds a commit anchor from a committed transaction marker.
    #[must_use]
    pub fn from_commit(commit: &WalTransactionCommit) -> Self {
        Self {
            transaction_id: commit.transaction_id,
            commit_digest: commit.commit_digest,
            first_lsn: commit.first_lsn,
            last_lsn: commit.last_lsn,
            record_count: commit.record_count,
        }
    }

    /// Computes a stable identity digest for this commit anchor.
    #[must_use]
    pub fn identity_digest(&self) -> Hash {
        let mut h = blake3::Hasher::new();
        h.update(WAL_PROJECTION_COMMIT_ANCHOR_DOMAIN);
        h.update(&self.transaction_id.as_hash());
        h.update(&self.commit_digest);
        h.update(&self.first_lsn.as_u64().to_le_bytes());
        h.update(&self.last_lsn.as_u64().to_le_bytes());
        h.update(&self.record_count.to_le_bytes());
        h.finalize().into()
    }
}

/// WAL segment sealing posture projected into the read model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalSegmentSealPosture {
    /// Segment remains open and must not be treated as sealed evidence.
    Open,
    /// Segment was sealed through WAL store evidence.
    Sealed {
        /// Last LSN included in the sealed segment, if any.
        sealed_lsn: Option<Lsn>,
    },
}

impl WalSegmentSealPosture {
    fn code(&self) -> u8 {
        match self {
            Self::Open => 0,
            Self::Sealed { .. } => 1,
        }
    }
}

/// Storage locator metadata for a projected WAL segment.
///
/// Locators are transport and operator metadata. They are intentionally excluded
/// from [`WalSegmentRef::identity_digest`] so moving a segment between absolute
/// filesystem roots cannot alter causal projection identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalSegmentStorageLocator {
    /// Repository- or root-relative locator.
    RelativePath(PathBuf),
    /// Absolute local filesystem locator.
    AbsolutePath(PathBuf),
}

/// Read-model projection reference to one WAL segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalSegmentRef {
    /// Writer epoch that owns this segment.
    pub writer_epoch: WriterEpochId,
    /// Logical WAL segment id.
    pub segment_id: WalSegmentId,
    /// First LSN covered by the segment.
    pub first_lsn: Lsn,
    /// Last LSN covered by the segment.
    pub last_lsn: Lsn,
    /// Commit digest preceding the segment's committed chain.
    pub previous_commit_digest: Hash,
    /// Final commit digest observed for the segment.
    pub final_commit_digest: Hash,
    /// Digest of the segment contents.
    pub segment_digest: Hash,
    /// Commit anchors covered by the segment.
    pub commit_anchors: Vec<WalCommitAnchor>,
    /// Segment sealing posture.
    pub seal_posture: WalSegmentSealPosture,
    /// Optional storage locator metadata, excluded from identity.
    pub storage_locator: Option<WalSegmentStorageLocator>,
}

impl WalSegmentRef {
    /// Computes a stable read-model identity digest for the segment reference.
    ///
    /// The digest includes writer epoch, LSN range, commit-digest chain, segment
    /// digest, commit anchors, and sealing posture. It deliberately excludes
    /// [`Self::storage_locator`].
    #[must_use]
    pub fn identity_digest(&self) -> Hash {
        let mut h = blake3::Hasher::new();
        h.update(WAL_PROJECTION_SEGMENT_DOMAIN);
        h.update(&self.writer_epoch.as_hash());
        h.update(&self.segment_id.as_u64().to_le_bytes());
        h.update(&self.first_lsn.as_u64().to_le_bytes());
        h.update(&self.last_lsn.as_u64().to_le_bytes());
        h.update(&self.previous_commit_digest);
        h.update(&self.final_commit_digest);
        h.update(&self.segment_digest);

        let mut anchors = self.commit_anchors.clone();
        anchors.sort_by_key(|anchor| {
            (
                anchor.first_lsn,
                anchor.last_lsn,
                anchor.transaction_id,
                anchor.commit_digest,
            )
        });
        h.update(&len_u64(anchors.len()).to_le_bytes());
        for anchor in &anchors {
            h.update(&anchor.identity_digest());
        }

        h.update(&[self.seal_posture.code()]);
        match &self.seal_posture {
            WalSegmentSealPosture::Open => {}
            WalSegmentSealPosture::Sealed { sealed_lsn } => {
                update_optional_lsn(&mut h, *sealed_lsn);
            }
        }
        h.finalize().into()
    }
}

/// Read-model projection reference to a recovery certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryCertificateRef {
    /// Recovery certificate digest.
    pub certificate_digest: Hash,
    /// Checkpoint digest used as replay base, if any.
    pub checkpoint_used: Option<Hash>,
    /// First scanned committed LSN.
    pub first_lsn: Option<Lsn>,
    /// Last scanned committed LSN.
    pub last_lsn: Option<Lsn>,
    /// Tail posture observed during recovery.
    pub tail_posture: RecoveryTailPosture,
    /// Final frontier root.
    pub recovered_frontier_root: Hash,
    /// Final index root.
    pub recovered_indexes_root: Hash,
}

impl RecoveryCertificateRef {
    /// Builds a projection reference from a recovery certificate.
    #[must_use]
    pub fn from_certificate(certificate: &RecoveryCertificate) -> Self {
        Self {
            certificate_digest: recovery_certificate_digest(certificate),
            checkpoint_used: certificate.checkpoint_used,
            first_lsn: certificate.first_lsn,
            last_lsn: certificate.last_lsn,
            tail_posture: certificate.tail_posture,
            recovered_frontier_root: certificate.recovered_frontier_root,
            recovered_indexes_root: certificate.recovered_indexes_root,
        }
    }

    /// Computes a stable identity digest for this recovery-certificate reference.
    #[must_use]
    pub fn identity_digest(&self) -> Hash {
        let mut h = blake3::Hasher::new();
        h.update(WAL_PROJECTION_RECOVERY_CERTIFICATE_DOMAIN);
        h.update(&self.certificate_digest);
        update_optional_projection_digest(&mut h, self.checkpoint_used);
        update_optional_lsn(&mut h, self.first_lsn);
        update_optional_lsn(&mut h, self.last_lsn);
        update_recovery_tail_posture(&mut h, self.tail_posture);
        h.update(&self.recovered_frontier_root);
        h.update(&self.recovered_indexes_root);
        h.finalize().into()
    }
}

/// Segment evidence available when projecting a recovered WAL into graph facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalRecoverySegmentEvidence {
    /// Logical WAL segment id.
    pub segment_id: WalSegmentId,
    /// Digest of the segment contents.
    pub segment_digest: Hash,
    /// Segment sealing posture.
    pub seal_posture: WalSegmentSealPosture,
    /// Segment locator metadata. Projection from recovery treats missing locators
    /// as an obstruction rather than fabricating storage evidence.
    pub storage_locator: Option<WalSegmentStorageLocator>,
}

/// WAL recovery projection posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalRecoveryProjectionPosture {
    /// No WAL evidence was present to project.
    Absent,
    /// A projection root was built from explicit recovery evidence.
    Present,
    /// Recovery evidence existed, but was insufficient or obstructed.
    Obstructed,
}

/// Typed reason a recovered WAL could not be projected into graph facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalRecoveryProjectionObstruction {
    /// Recovery saw a non-clean tail posture, so the projection cannot claim a
    /// sealed history root.
    TailPostureObstructed {
        /// Observed tail posture.
        posture: RecoveryTailPosture,
    },
    /// A published manifest was required but absent.
    MissingManifest,
    /// The manifest could not be validated from filesystem evidence.
    ManifestValidationUnavailable,
    /// Segment evidence could not be read from filesystem storage.
    SegmentEvidenceUnavailable,
    /// The manifest's last committed LSN did not match recovered commits.
    ManifestLastCommittedLsnMismatch {
        /// Last committed LSN from recovery.
        expected: Option<Lsn>,
        /// Last committed LSN from the manifest.
        actual: Option<Lsn>,
    },
    /// The manifest's last commit digest did not match recovered commits.
    ManifestLastCommitDigestMismatch {
        /// Last commit digest from recovery.
        expected: Option<Hash>,
        /// Last commit digest from the manifest.
        actual: Option<Hash>,
    },
    /// The manifest's sealed segment count did not match segment evidence.
    ManifestSegmentCountMismatch {
        /// Sealed segment count from the manifest.
        expected: u64,
        /// Segment evidence count supplied for projection.
        actual: u64,
    },
    /// Duplicate segment evidence was supplied for the same logical segment.
    DuplicateSegmentEvidence {
        /// Duplicated segment id.
        segment_id: WalSegmentId,
    },
    /// Writer epoch projection evidence was missing for a recovered commit.
    MissingWriterEpochEvidence {
        /// Missing writer epoch id.
        writer_epoch: WriterEpochId,
    },
    /// Segment evidence was missing for recovered committed frames.
    MissingSegmentEvidence {
        /// Missing segment id.
        segment_id: WalSegmentId,
    },
    /// Segment locator metadata was unavailable.
    SegmentLocatorUnavailable {
        /// Segment id with no locator.
        segment_id: WalSegmentId,
    },
    /// Segment evidence did not prove the segment sealed.
    SegmentNotSealed {
        /// Unsealed segment id.
        segment_id: WalSegmentId,
    },
    /// Segment seal evidence did not cover the recovered committed range.
    SegmentSealDoesNotCoverRecoveredCommit {
        /// Segment id.
        segment_id: WalSegmentId,
        /// Sealed LSN from segment evidence.
        sealed_lsn: Option<Lsn>,
        /// Last recovered committed LSN for this segment.
        recovered_last_lsn: Lsn,
    },
    /// Segment digest evidence did not match recovered frames.
    SegmentDigestMismatch {
        /// Segment id.
        segment_id: WalSegmentId,
        /// Digest recomputed from recovered segment frames.
        expected: Hash,
        /// Digest supplied as projection evidence.
        actual: Hash,
    },
    /// Recovery certificate scan fields did not match the recovery report.
    RecoveryCertificateScanMismatch {
        /// Expected first committed LSN from the recovery report.
        expected_first_lsn: Option<Lsn>,
        /// Certificate first committed LSN.
        actual_first_lsn: Option<Lsn>,
        /// Expected last committed LSN from the recovery report.
        expected_last_lsn: Option<Lsn>,
        /// Certificate last committed LSN.
        actual_last_lsn: Option<Lsn>,
        /// Expected committed transaction replay count.
        expected_committed_transactions_replayed: u64,
        /// Certificate committed transaction replay count.
        actual_committed_transactions_replayed: u64,
        /// Expected recovery tail posture.
        expected_tail_posture: RecoveryTailPosture,
        /// Certificate recovery tail posture.
        actual_tail_posture: RecoveryTailPosture,
    },
    /// Recovered frame evidence did not identify a transaction segment.
    TransactionSegmentEvidenceUnavailable {
        /// Transaction id.
        transaction_id: WalTransactionId,
    },
    /// A recovered transaction spans multiple segment ids and cannot be projected
    /// as a single segment anchor.
    TransactionSpansSegments {
        /// Transaction id.
        transaction_id: WalTransactionId,
    },
    /// A projected segment contains commits from multiple writer epochs.
    MixedWriterEpochSegment {
        /// Segment id.
        segment_id: WalSegmentId,
    },
}

/// Projection result for recovered WAL evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalRecoveryProjection {
    /// Projection posture.
    pub posture: WalRecoveryProjectionPosture,
    /// Projected WAL root, if evidence was complete.
    pub root: Option<WalRoot>,
    /// Typed obstructions that prevented projection.
    pub obstructions: Vec<WalRecoveryProjectionObstruction>,
}

impl WalRecoveryProjection {
    fn absent() -> Self {
        Self {
            posture: WalRecoveryProjectionPosture::Absent,
            root: None,
            obstructions: Vec::new(),
        }
    }

    fn present(root: WalRoot) -> Self {
        Self {
            posture: WalRecoveryProjectionPosture::Present,
            root: Some(root),
            obstructions: Vec::new(),
        }
    }

    fn obstructed(obstructions: Vec<WalRecoveryProjectionObstruction>) -> Self {
        Self {
            posture: WalRecoveryProjectionPosture::Obstructed,
            root: None,
            obstructions,
        }
    }
}

/// Bootstrap evidence used to start a recovery plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalRecoveryBootstrapSource {
    /// Recovery started from an already projected WAL root.
    WalRoot {
        /// Projection root digest.
        root_digest: Hash,
        /// Stable identity digest of the projected root.
        root_identity_digest: Hash,
    },
    /// Recovery started from storage manifest evidence.
    StorageManifest {
        /// Published manifest digest.
        manifest_digest: Hash,
    },
}

impl WalRecoveryBootstrapSource {
    /// Builds bootstrap evidence from a projected WAL root.
    #[must_use]
    pub fn from_wal_root(root: &WalRoot) -> Self {
        Self::WalRoot {
            root_digest: root.root_digest,
            root_identity_digest: root.identity_digest(),
        }
    }

    /// Builds bootstrap evidence from a storage manifest.
    #[must_use]
    pub fn from_manifest(manifest: &WalManifest) -> Self {
        Self::StorageManifest {
            manifest_digest: manifest.manifest_digest,
        }
    }
}

/// Checkpoint selection posture for a recovery plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalRecoveryCheckpointPosture {
    /// Recovery did not use a checkpoint base.
    NotSelected,
    /// Recovery selected a checkpoint and records its validation posture.
    Selected {
        /// Selected checkpoint digest.
        checkpoint_digest: Hash,
        /// Validation posture for the selected checkpoint.
        validation: CheckpointValidationPosture,
    },
}

/// Committed WAL suffix replayed by a recovery plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalRecoveryReplaySuffix {
    /// First committed LSN replayed.
    pub first_lsn: Option<Lsn>,
    /// Last committed LSN replayed.
    pub last_lsn: Option<Lsn>,
    /// Number of committed transactions replayed.
    pub committed_transactions: u64,
    /// Final committed transaction digest.
    pub final_commit_digest: Option<Hash>,
}

impl WalRecoveryReplaySuffix {
    /// Builds replay suffix evidence from a recovery scan report.
    #[must_use]
    pub fn from_report(report: &RecoveryScanReport) -> Self {
        Self {
            first_lsn: report.first_committed_lsn(),
            last_lsn: report.last_committed_lsn(),
            committed_transactions: len_u64(report.transactions.len()),
            final_commit_digest: report.last_commit_digest(),
        }
    }
}

/// Index roots produced by recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalRecoveryIndexRoots {
    /// Recovered frontier root.
    pub recovered_frontier_root: Hash,
    /// Recovered index root.
    pub recovered_indexes_root: Hash,
}

impl WalRecoveryIndexRoots {
    /// Builds index-root evidence from a recovery certificate.
    #[must_use]
    pub fn from_certificate(certificate: &RecoveryCertificate) -> Self {
        Self {
            recovered_frontier_root: certificate.recovered_frontier_root,
            recovered_indexes_root: certificate.recovered_indexes_root,
        }
    }
}

/// Retained-material availability posture for recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalRecoveryRetainedMaterialAvailability {
    /// All retained material required by the recovered index is available.
    Available,
    /// At least one retained material reference is missing, corrupt, or obstructed.
    Obstructed,
}

/// Retained-material posture recorded by a recovery plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalRecoveryRetainedMaterialPosture {
    /// Overall retained-material availability.
    pub availability: WalRecoveryRetainedMaterialAvailability,
    /// Number of recovered retained material references.
    pub material_count: u64,
    /// Number of recovered reading references.
    pub reading_count: u64,
    /// Number of retained-material obstructions.
    pub obstruction_count: u64,
}

impl WalRecoveryRetainedMaterialPosture {
    /// Builds retained-material posture from recovered retained evidence.
    #[must_use]
    pub fn from_retention_index(
        index: &RecoveredRetentionIndex,
        available_material: &BTreeSet<Hash>,
    ) -> Self {
        let obstruction_count =
            len_u64(retained_material_obstructions(index, available_material).len());
        Self {
            availability: if obstruction_count == 0 {
                WalRecoveryRetainedMaterialAvailability::Available
            } else {
                WalRecoveryRetainedMaterialAvailability::Obstructed
            },
            material_count: len_u64(index.material_by_digest.len()),
            reading_count: len_u64(index.reading_by_id.len()),
            obstruction_count,
        }
    }
}

/// Recovery plan/report shape tying bootstrap, replay, retention, and projection evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalRecoveryPlan {
    /// Evidence used to bootstrap the recovery plan.
    pub bootstrap_source: WalRecoveryBootstrapSource,
    /// Checkpoint selection and validation posture.
    pub checkpoint_posture: WalRecoveryCheckpointPosture,
    /// Committed suffix replayed after the bootstrap source.
    pub replay_suffix: WalRecoveryReplaySuffix,
    /// Tail posture observed during recovery.
    pub tail_posture: RecoveryTailPosture,
    /// Recovered frontier and index roots.
    pub index_roots: WalRecoveryIndexRoots,
    /// Retained-material availability posture.
    pub retained_material_posture: WalRecoveryRetainedMaterialPosture,
    /// Projection posture for graph-ready WAL evidence.
    pub projected_evidence_posture: WalRecoveryProjectionPosture,
    /// Number of typed projection obstructions.
    pub projected_evidence_obstruction_count: u64,
}

impl WalRecoveryPlan {
    /// Builds a recovery plan that bootstraps from a projected WAL root.
    #[must_use]
    pub fn from_wal_root(
        root: &WalRoot,
        report: &RecoveryScanReport,
        checkpoint_posture: WalRecoveryCheckpointPosture,
        certificate: &RecoveryCertificate,
        retained_material_posture: WalRecoveryRetainedMaterialPosture,
        projection: &WalRecoveryProjection,
    ) -> Self {
        Self::from_bootstrap_source(
            WalRecoveryBootstrapSource::from_wal_root(root),
            report,
            checkpoint_posture,
            certificate,
            retained_material_posture,
            projection,
        )
    }

    /// Builds a recovery plan that bootstraps from storage manifest evidence.
    #[must_use]
    pub fn from_manifest(
        manifest: &WalManifest,
        report: &RecoveryScanReport,
        checkpoint_posture: WalRecoveryCheckpointPosture,
        certificate: &RecoveryCertificate,
        retained_material_posture: WalRecoveryRetainedMaterialPosture,
        projection: &WalRecoveryProjection,
    ) -> Self {
        Self::from_bootstrap_source(
            WalRecoveryBootstrapSource::from_manifest(manifest),
            report,
            checkpoint_posture,
            certificate,
            retained_material_posture,
            projection,
        )
    }

    fn from_bootstrap_source(
        bootstrap_source: WalRecoveryBootstrapSource,
        report: &RecoveryScanReport,
        checkpoint_posture: WalRecoveryCheckpointPosture,
        certificate: &RecoveryCertificate,
        retained_material_posture: WalRecoveryRetainedMaterialPosture,
        projection: &WalRecoveryProjection,
    ) -> Self {
        Self {
            bootstrap_source,
            checkpoint_posture,
            replay_suffix: WalRecoveryReplaySuffix::from_report(report),
            tail_posture: report.tail_posture,
            index_roots: WalRecoveryIndexRoots::from_certificate(certificate),
            retained_material_posture,
            projected_evidence_posture: projection.posture,
            projected_evidence_obstruction_count: len_u64(projection.obstructions.len()),
        }
    }
}

/// Graph/WSC projection index rebuilt after WAL recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveredWalProjectionIndex {
    /// Projection posture rebuilt from recovered evidence.
    pub posture: WalRecoveryProjectionPosture,
    /// Identity digest for the projected WAL root when projection succeeded.
    pub root_identity_digest: Option<Hash>,
    /// Number of projected writer epochs.
    pub writer_epoch_count: u64,
    /// Number of projected WAL segments.
    pub segment_count: u64,
    /// Number of typed projection obstructions.
    pub obstruction_count: u64,
}

impl RecoveredWalProjectionIndex {
    /// Builds projection index posture from a WAL recovery projection.
    #[must_use]
    pub fn from_projection(projection: &WalRecoveryProjection) -> Self {
        let (root_identity_digest, writer_epoch_count, segment_count) =
            projection.root.as_ref().map_or((None, 0, 0), |root| {
                (
                    Some(root.identity_digest()),
                    len_u64(root.writer_epochs.len()),
                    len_u64(root.segments.len()),
                )
            });
        Self {
            posture: projection.posture,
            root_identity_digest,
            writer_epoch_count,
            segment_count,
            obstruction_count: len_u64(projection.obstructions.len()),
        }
    }
}

/// Durability indexes rebuilt from committed WAL recovery evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveredDurabilityIndexes {
    /// Rebuilt submission posture index.
    pub submissions: RecoveredSubmissionIndex,
    /// Rebuilt receipt/correlation index.
    pub receipts: RecoveredReceiptIndex,
    /// Rebuilt retained-material and reading index.
    pub retention: RecoveredRetentionIndex,
    /// Rebuilt materialization outbox.
    pub materialization_outbox: BTreeMap<Hash, MaterializationOutboxEntry>,
    /// Rebuilt topology/graph index.
    pub topology: RecoveredTopologyIndex,
    /// Rebuilt graph/WSC projection posture.
    pub projection: RecoveredWalProjectionIndex,
    /// Stable root for submission and receipt indexes.
    pub submission_receipt_index_root: Hash,
    /// Stable root for topology indexes.
    pub topology_index_root: Hash,
}

/// Rebuilds all durability indexes from a recovered committed WAL scan.
///
/// This function only consumes [`RecoveryScanReport`] transactions and explicit
/// projection/materialization evidence supplied by the caller. It does not invoke
/// scheduler handlers, contract observers, wall-clock APIs, network APIs, or app
/// code.
///
/// # Errors
///
/// Returns [`WalRecoveryIndexError`] when any committed WAL payload cannot decode
/// or contains conflicting recovered index evidence.
pub fn rebuild_durability_indexes_after_recovery(
    report: &RecoveryScanReport,
    existing_artifacts: &BTreeMap<Hash, ExistingMaterializedArtifact>,
    projection: &WalRecoveryProjection,
) -> Result<RecoveredDurabilityIndexes, WalRecoveryIndexError> {
    let submissions = recover_submission_index(report)?;
    let receipts = recover_receipt_index(report)?;
    let retention = recover_retention_index(report)?;
    let materialization_outbox = recover_materialization_outbox(report, existing_artifacts)?;
    let topology = recover_topology_index(report)?;
    let submission_receipt_index_root =
        recovered_submission_receipt_index_root(&submissions, &receipts);
    let topology_index_root = recovered_topology_index_root(&topology);
    Ok(RecoveredDurabilityIndexes {
        submissions,
        receipts,
        retention,
        materialization_outbox,
        topology,
        projection: RecoveredWalProjectionIndex::from_projection(projection),
        submission_receipt_index_root,
        topology_index_root,
    })
}

/// Materialized WARP graph facts for a projected WAL root.
///
/// The graph is a read-only fact projection. It carries typed nodes, edges, and
/// atom attachments suitable for WSC serialization, but it is not a
/// [`WalStorePort`] and cannot append, truncate, recover, or publish WAL
/// storage material.
#[derive(Clone, Debug)]
pub struct WalProjectionGraph {
    /// Graph store containing materialized projection facts.
    pub store: GraphStore,
    /// Root node for the materialized WAL projection graph.
    pub root_node_id: NodeId,
}

/// Import posture for materialized WAL projection graph WSC bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalProjectionGraphObservationPosture {
    /// The import is observation evidence only and carries no WAL storage authority.
    ObservationOnly,
}

/// Observation evidence recovered from a materialized WAL projection graph WSC payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalProjectionGraphObservation {
    /// Import posture for this projection graph.
    pub posture: WalProjectionGraphObservationPosture,
    /// Observed WSC schema hash.
    pub schema_hash: Hash,
    /// Observed WARP id.
    pub warp_id: Hash,
    /// Observed projection root node id.
    pub root_node_id: NodeId,
    /// Number of observed graph fact nodes.
    pub node_count: u64,
    /// Number of observed graph fact edges.
    pub edge_count: u64,
    /// Number of observed node atom attachments.
    pub node_attachment_count: u64,
    /// Number of observed edge attachments.
    pub edge_attachment_count: u64,
    /// Observed WSC blob section byte length.
    pub blob_len: u64,
}

/// Error returned when observing materialized WAL projection graph WSC bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum WalProjectionGraphObservationError {
    /// The bytes are not valid WSC material.
    #[error("invalid WAL projection graph WSC payload")]
    InvalidWsc,
    /// The WSC payload uses a schema other than the WAL projection graph schema.
    #[error("WAL projection graph schema mismatch")]
    SchemaMismatch {
        /// Expected WAL projection graph schema hash.
        expected: Hash,
        /// Actual WSC schema hash.
        actual: Hash,
    },
}

/// Returns the WSC schema hash for materialized WAL projection graph facts.
#[must_use]
pub fn wal_projection_graph_schema_hash() -> Hash {
    crate::ident::make_type_id(WAL_PROJECTION_GRAPH_SCHEMA).0
}

/// Observes a materialized WAL projection graph WSC payload without recovering WAL authority.
///
/// This validates the generic WSC container and the WAL projection graph schema,
/// then returns graph-shape evidence only. It does not decode payloads into a
/// [`WalRoot`], does not validate WAL recovery, and does not expose a
/// [`WalStorePort`].
///
/// # Errors
///
/// Returns [`WalProjectionGraphObservationError::InvalidWsc`] for malformed WSC
/// bytes and [`WalProjectionGraphObservationError::SchemaMismatch`] when the
/// payload is valid WSC but not WAL projection graph material.
pub fn observe_wal_projection_graph_wsc(
    wsc_bytes: &[u8],
) -> Result<WalProjectionGraphObservation, WalProjectionGraphObservationError> {
    let file = WscFile::from_bytes(wsc_bytes.to_vec())
        .map_err(|_| WalProjectionGraphObservationError::InvalidWsc)?;
    validate_wsc(&file).map_err(|_| WalProjectionGraphObservationError::InvalidWsc)?;
    let expected = wal_projection_graph_schema_hash();
    let actual = *file.schema_hash();
    if actual != expected {
        return Err(WalProjectionGraphObservationError::SchemaMismatch { expected, actual });
    }
    let view = file
        .warp_view(0)
        .map_err(|_| WalProjectionGraphObservationError::InvalidWsc)?;
    let node_attachment_count = (0..view.nodes().len())
        .map(|index| view.node_attachments(index).len())
        .sum::<usize>();
    let edge_attachment_count = (0..view.edges().len())
        .map(|index| view.edge_attachments(index).len())
        .sum::<usize>();

    Ok(WalProjectionGraphObservation {
        posture: WalProjectionGraphObservationPosture::ObservationOnly,
        schema_hash: actual,
        warp_id: *view.warp_id(),
        root_node_id: NodeId(*view.root_node_id()),
        node_count: len_u64(view.nodes().len()),
        edge_count: len_u64(view.edges().len()),
        node_attachment_count: len_u64(node_attachment_count),
        edge_attachment_count: len_u64(edge_attachment_count),
        blob_len: len_u64(view.blobs().len()),
    })
}

/// Materializes a projected WAL root into a deterministic WARP graph.
///
/// The materialized graph uses one fact node for the WAL root, every writer
/// epoch, every segment, every commit anchor, and the optional recovery
/// certificate reference. Fact bytes are stored as typed atom attachments.
/// Segment storage locator metadata is deliberately omitted from the atom
/// payload because it is locator evidence, not graph identity or append
/// authority.
#[must_use]
pub fn materialize_wal_projection_graph(root: &WalRoot) -> WalProjectionGraph {
    let mut store = GraphStore::new(crate::ident::make_warp_id(WAL_PROJECTION_GRAPH_WARP));
    let root_node_id = wal_projection_node_id(b"root", root.identity_digest());
    insert_wal_projection_fact_node(
        &mut store,
        root_node_id,
        WAL_PROJECTION_GRAPH_ROOT_NODE_TYPE,
        WAL_PROJECTION_GRAPH_ROOT_ATTACHMENT_TYPE,
        wal_projection_root_payload(root),
    );

    let mut writer_epochs = root.writer_epochs.clone();
    writer_epochs.sort_by_key(WalWriterEpoch::identity_digest);
    for writer_epoch in &writer_epochs {
        let node_id = wal_projection_node_id(b"writer_epoch", writer_epoch.identity_digest());
        insert_wal_projection_fact_node(
            &mut store,
            node_id,
            WAL_PROJECTION_GRAPH_WRITER_EPOCH_NODE_TYPE,
            WAL_PROJECTION_GRAPH_WRITER_EPOCH_ATTACHMENT_TYPE,
            wal_projection_writer_epoch_payload(writer_epoch),
        );
        insert_wal_projection_edge(
            &mut store,
            root_node_id,
            node_id,
            b"root_writer_epoch",
            WAL_PROJECTION_GRAPH_WRITER_EPOCH_EDGE_TYPE,
        );
    }

    let mut segments = root.segments.clone();
    segments.sort_by_key(WalSegmentRef::identity_digest);
    for segment in &segments {
        let segment_node_id = wal_projection_node_id(b"segment", segment.identity_digest());
        insert_wal_projection_fact_node(
            &mut store,
            segment_node_id,
            WAL_PROJECTION_GRAPH_SEGMENT_NODE_TYPE,
            WAL_PROJECTION_GRAPH_SEGMENT_ATTACHMENT_TYPE,
            wal_projection_segment_payload(segment),
        );
        insert_wal_projection_edge(
            &mut store,
            root_node_id,
            segment_node_id,
            b"root_segment",
            WAL_PROJECTION_GRAPH_SEGMENT_EDGE_TYPE,
        );

        let mut commit_anchors = segment.commit_anchors.clone();
        commit_anchors.sort_by_key(WalCommitAnchor::identity_digest);
        for anchor in &commit_anchors {
            let anchor_node_id = wal_projection_node_id(b"commit_anchor", anchor.identity_digest());
            insert_wal_projection_fact_node(
                &mut store,
                anchor_node_id,
                WAL_PROJECTION_GRAPH_COMMIT_ANCHOR_NODE_TYPE,
                WAL_PROJECTION_GRAPH_COMMIT_ANCHOR_ATTACHMENT_TYPE,
                wal_projection_commit_anchor_payload(anchor),
            );
            insert_wal_projection_edge(
                &mut store,
                root_node_id,
                anchor_node_id,
                b"root_commit_anchor",
                WAL_PROJECTION_GRAPH_ROOT_COMMIT_ANCHOR_EDGE_TYPE,
            );
            insert_wal_projection_edge(
                &mut store,
                segment_node_id,
                anchor_node_id,
                b"segment_commit_anchor",
                WAL_PROJECTION_GRAPH_SEGMENT_COMMIT_ANCHOR_EDGE_TYPE,
            );
        }
    }

    if let Some(certificate) = &root.recovery_certificate {
        let node_id =
            wal_projection_node_id(b"recovery_certificate", certificate.identity_digest());
        insert_wal_projection_fact_node(
            &mut store,
            node_id,
            WAL_PROJECTION_GRAPH_RECOVERY_CERTIFICATE_NODE_TYPE,
            WAL_PROJECTION_GRAPH_RECOVERY_CERTIFICATE_ATTACHMENT_TYPE,
            wal_projection_recovery_certificate_payload(certificate),
        );
        insert_wal_projection_edge(
            &mut store,
            root_node_id,
            node_id,
            b"root_recovery_certificate",
            WAL_PROJECTION_GRAPH_RECOVERY_CERTIFICATE_EDGE_TYPE,
        );
    }

    WalProjectionGraph {
        store,
        root_node_id,
    }
}

/// Projects recovered WAL evidence into a graph-ready root record.
///
/// This function does not read or write storage. It only combines explicit
/// recovery, manifest, writer-epoch, segment, locator, seal, and certificate
/// evidence. Missing evidence produces typed obstructions instead of an empty
/// successful projection.
#[must_use]
pub fn project_wal_recovery(
    report: &RecoveryScanReport,
    manifest: Option<&WalManifest>,
    writer_epochs: &[WalWriterEpoch],
    segments: &[WalRecoverySegmentEvidence],
    recovery_certificate: Option<&RecoveryCertificate>,
) -> WalRecoveryProjection {
    let mut obstructions = Vec::new();
    if report.transactions.is_empty()
        && manifest.is_none()
        && writer_epochs.is_empty()
        && segments.is_empty()
        && recovery_certificate.is_none()
        && report.tail_posture == RecoveryTailPosture::Clean
    {
        return WalRecoveryProjection::absent();
    }
    if report.tail_posture != RecoveryTailPosture::Clean {
        obstructions.push(WalRecoveryProjectionObstruction::TailPostureObstructed {
            posture: report.tail_posture,
        });
    }

    let Some(manifest) = manifest else {
        obstructions.push(WalRecoveryProjectionObstruction::MissingManifest);
        return WalRecoveryProjection::obstructed(obstructions);
    };

    if manifest.last_committed_lsn != report.last_committed_lsn() {
        obstructions.push(
            WalRecoveryProjectionObstruction::ManifestLastCommittedLsnMismatch {
                expected: report.last_committed_lsn(),
                actual: manifest.last_committed_lsn,
            },
        );
    }
    if manifest.last_commit_digest != report.last_commit_digest() {
        obstructions.push(
            WalRecoveryProjectionObstruction::ManifestLastCommitDigestMismatch {
                expected: report.last_commit_digest(),
                actual: manifest.last_commit_digest,
            },
        );
    }
    if manifest.sealed_segment_count != len_u64(segments.len()) {
        obstructions.push(
            WalRecoveryProjectionObstruction::ManifestSegmentCountMismatch {
                expected: manifest.sealed_segment_count,
                actual: len_u64(segments.len()),
            },
        );
    }

    let mut seen_segments = BTreeSet::new();
    for segment in segments {
        if !seen_segments.insert(segment.segment_id) {
            obstructions.push(WalRecoveryProjectionObstruction::DuplicateSegmentEvidence {
                segment_id: segment.segment_id,
            });
        }
    }

    let writer_epoch_by_id = writer_epochs
        .iter()
        .map(|epoch| (epoch.epoch_id, epoch))
        .collect::<BTreeMap<_, _>>();
    let segment_by_id = segments
        .iter()
        .map(|segment| (segment.segment_id, segment))
        .collect::<BTreeMap<_, _>>();
    let mut transactions_by_segment: BTreeMap<WalSegmentId, Vec<&WalRecoveredTransaction>> =
        BTreeMap::new();
    let mut required_writer_epochs = BTreeSet::new();

    for transaction in &report.transactions {
        required_writer_epochs.insert(transaction.commit.writer_epoch);
        let frame_segments = transaction
            .frames
            .iter()
            .map(|frame| frame.header.segment_id)
            .collect::<BTreeSet<_>>();
        match frame_segments.len() {
            0 => obstructions.push(
                WalRecoveryProjectionObstruction::TransactionSegmentEvidenceUnavailable {
                    transaction_id: transaction.commit.transaction_id,
                },
            ),
            1 => {
                if let Some(segment_id) = frame_segments.iter().next().copied() {
                    transactions_by_segment
                        .entry(segment_id)
                        .or_default()
                        .push(transaction);
                } else {
                    obstructions.push(
                        WalRecoveryProjectionObstruction::TransactionSegmentEvidenceUnavailable {
                            transaction_id: transaction.commit.transaction_id,
                        },
                    );
                }
            }
            _ => obstructions.push(WalRecoveryProjectionObstruction::TransactionSpansSegments {
                transaction_id: transaction.commit.transaction_id,
            }),
        }
    }

    for writer_epoch in &required_writer_epochs {
        if !writer_epoch_by_id.contains_key(writer_epoch) {
            obstructions.push(
                WalRecoveryProjectionObstruction::MissingWriterEpochEvidence {
                    writer_epoch: *writer_epoch,
                },
            );
        }
    }

    for (segment_id, transactions) in &transactions_by_segment {
        let Some(segment) = segment_by_id.get(segment_id) else {
            obstructions.push(WalRecoveryProjectionObstruction::MissingSegmentEvidence {
                segment_id: *segment_id,
            });
            continue;
        };
        if segment.storage_locator.is_none() {
            obstructions.push(
                WalRecoveryProjectionObstruction::SegmentLocatorUnavailable {
                    segment_id: *segment_id,
                },
            );
        }
        if segment.seal_posture == WalSegmentSealPosture::Open {
            obstructions.push(WalRecoveryProjectionObstruction::SegmentNotSealed {
                segment_id: *segment_id,
            });
        }
        if let Some(recovered_last_lsn) = transactions
            .iter()
            .map(|transaction| transaction.commit.last_lsn)
            .max()
        {
            if let WalSegmentSealPosture::Sealed { sealed_lsn } = segment.seal_posture {
                if sealed_lsn.is_none_or(|lsn| lsn < recovered_last_lsn) {
                    obstructions.push(
                        WalRecoveryProjectionObstruction::SegmentSealDoesNotCoverRecoveredCommit {
                            segment_id: *segment_id,
                            sealed_lsn,
                            recovered_last_lsn,
                        },
                    );
                }
            }
        }
        let segment_frames = transactions
            .iter()
            .flat_map(|transaction| {
                transaction
                    .frames
                    .iter()
                    .filter(move |frame| frame.header.segment_id == *segment_id)
            })
            .collect::<Vec<_>>();
        let recovered_segment_digest = segment_digest(*segment_id, &segment_frames);
        if segment.segment_digest != recovered_segment_digest {
            obstructions.push(WalRecoveryProjectionObstruction::SegmentDigestMismatch {
                segment_id: *segment_id,
                expected: recovered_segment_digest,
                actual: segment.segment_digest,
            });
        }
        let segment_writer_epochs = transactions
            .iter()
            .map(|transaction| transaction.commit.writer_epoch)
            .collect::<BTreeSet<_>>();
        if segment_writer_epochs.len() > 1 {
            obstructions.push(WalRecoveryProjectionObstruction::MixedWriterEpochSegment {
                segment_id: *segment_id,
            });
        }
    }

    if let Some(certificate) = recovery_certificate {
        let committed_transactions_replayed = len_u64(report.transactions.len());
        if certificate.first_lsn != report.first_committed_lsn()
            || certificate.last_lsn != report.last_committed_lsn()
            || certificate.committed_transactions_replayed != committed_transactions_replayed
            || certificate.tail_posture != report.tail_posture
        {
            obstructions.push(
                WalRecoveryProjectionObstruction::RecoveryCertificateScanMismatch {
                    expected_first_lsn: report.first_committed_lsn(),
                    actual_first_lsn: certificate.first_lsn,
                    expected_last_lsn: report.last_committed_lsn(),
                    actual_last_lsn: certificate.last_lsn,
                    expected_committed_transactions_replayed: committed_transactions_replayed,
                    actual_committed_transactions_replayed: certificate
                        .committed_transactions_replayed,
                    expected_tail_posture: report.tail_posture,
                    actual_tail_posture: certificate.tail_posture,
                },
            );
        }
    }

    if !obstructions.is_empty() {
        return WalRecoveryProjection::obstructed(obstructions);
    }

    let projected_writer_epochs = required_writer_epochs
        .iter()
        .filter_map(|epoch_id| writer_epoch_by_id.get(epoch_id).copied())
        .cloned()
        .collect::<Vec<_>>();
    let projected_segments = transactions_by_segment
        .iter()
        .filter_map(|(segment_id, transactions)| {
            let segment = *segment_by_id.get(segment_id)?;
            let mut transactions = transactions.clone();
            transactions.sort_by_key(|transaction| transaction.commit.first_lsn);
            let first = transactions.first()?;
            let last = transactions.last()?;
            Some(WalSegmentRef {
                writer_epoch: first.commit.writer_epoch,
                segment_id: *segment_id,
                first_lsn: first.commit.first_lsn,
                last_lsn: last.commit.last_lsn,
                previous_commit_digest: first.commit.previous_committed_transaction_digest,
                final_commit_digest: last.commit.commit_digest,
                segment_digest: segment.segment_digest,
                commit_anchors: transactions
                    .iter()
                    .map(|transaction| WalCommitAnchor::from_commit(&transaction.commit))
                    .collect(),
                seal_posture: segment.seal_posture.clone(),
                storage_locator: segment.storage_locator.clone(),
            })
        })
        .collect::<Vec<_>>();
    let recovery_certificate = recovery_certificate.map(RecoveryCertificateRef::from_certificate);

    WalRecoveryProjection::present(WalRoot {
        root_digest: manifest.manifest_digest,
        writer_epochs: projected_writer_epochs,
        segments: projected_segments,
        recovery_certificate,
    })
}

/// Projects a filesystem WAL recovery scan using read-only filesystem evidence.
///
/// The adapter validates the published manifest and scans segment files, but does
/// not mutate WAL storage or graph storage.
#[must_use]
pub fn project_filesystem_wal_recovery(
    root: impl AsRef<Path>,
    report: &RecoveryScanReport,
    writer_epochs: &[WalWriterEpoch],
    recovery_certificate: Option<&RecoveryCertificate>,
) -> WalRecoveryProjection {
    let root = root.as_ref();
    let manifest = match validate_filesystem_manifest(root) {
        Ok(report) => report.manifest,
        Err(WalStoreError::MissingManifest) => {
            return project_wal_recovery(report, None, writer_epochs, &[], recovery_certificate);
        }
        Err(WalStoreError::ManifestLastCommittedLsnMismatch { expected, actual }) => {
            return WalRecoveryProjection::obstructed(vec![
                WalRecoveryProjectionObstruction::ManifestLastCommittedLsnMismatch {
                    expected,
                    actual,
                },
            ]);
        }
        Err(WalStoreError::ManifestLastCommitDigestMismatch { expected, actual }) => {
            return WalRecoveryProjection::obstructed(vec![
                WalRecoveryProjectionObstruction::ManifestLastCommitDigestMismatch {
                    expected,
                    actual,
                },
            ]);
        }
        Err(WalStoreError::ManifestSegmentCountMismatch { expected, actual }) => {
            return WalRecoveryProjection::obstructed(vec![
                WalRecoveryProjectionObstruction::ManifestSegmentCountMismatch {
                    expected: actual,
                    actual: expected,
                },
            ]);
        }
        Err(WalStoreError::ManifestCannotValidateUncommittedTail) => {
            return WalRecoveryProjection::obstructed(vec![
                WalRecoveryProjectionObstruction::TailPostureObstructed {
                    posture: report.tail_posture,
                },
            ]);
        }
        Err(_) => {
            return WalRecoveryProjection::obstructed(vec![
                WalRecoveryProjectionObstruction::ManifestValidationUnavailable,
            ]);
        }
    };
    let segments = match filesystem_wal_recovery_segment_evidence(root) {
        Ok(segments) => segments,
        Err(_) => {
            return WalRecoveryProjection::obstructed(vec![
                WalRecoveryProjectionObstruction::SegmentEvidenceUnavailable,
            ]);
        }
    };
    project_wal_recovery(
        report,
        Some(&manifest),
        writer_epochs,
        &segments,
        recovery_certificate,
    )
}

fn insert_wal_projection_fact_node(
    store: &mut GraphStore,
    node_id: NodeId,
    node_type: &str,
    attachment_type: &str,
    payload_bytes: Vec<u8>,
) {
    store.insert_node(
        node_id,
        NodeRecord {
            ty: crate::ident::make_type_id(node_type),
        },
    );
    store.set_node_attachment(
        node_id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            crate::ident::make_type_id(attachment_type),
            Bytes::from(payload_bytes),
        ))),
    );
}

fn insert_wal_projection_edge(
    store: &mut GraphStore,
    from: NodeId,
    to: NodeId,
    role: &[u8],
    edge_type: &str,
) {
    store.insert_edge(
        from,
        EdgeRecord {
            id: wal_projection_edge_id(role, from, to),
            from,
            to,
            ty: crate::ident::make_type_id(edge_type),
        },
    );
}

fn wal_projection_node_id(role: &[u8], identity_digest: Hash) -> NodeId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(WAL_PROJECTION_GRAPH_NODE_DOMAIN);
    update_len_prefixed(&mut hasher, role);
    hasher.update(&identity_digest);
    NodeId(hasher.finalize().into())
}

fn wal_projection_edge_id(role: &[u8], from: NodeId, to: NodeId) -> EdgeId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(WAL_PROJECTION_GRAPH_EDGE_DOMAIN);
    update_len_prefixed(&mut hasher, role);
    hasher.update(&from.0);
    hasher.update(&to.0);
    EdgeId(hasher.finalize().into())
}

fn wal_projection_root_payload(root: &WalRoot) -> Vec<u8> {
    let mut out = Vec::new();
    push_hash(&mut out, &root.root_digest);

    let mut writer_epoch_digests = root
        .writer_epochs
        .iter()
        .map(WalWriterEpoch::identity_digest)
        .collect::<Vec<_>>();
    writer_epoch_digests.sort_unstable();
    out.extend_from_slice(&len_u64(writer_epoch_digests.len()).to_le_bytes());
    for digest in &writer_epoch_digests {
        push_hash(&mut out, digest);
    }

    let mut segment_digests = root
        .segments
        .iter()
        .map(WalSegmentRef::identity_digest)
        .collect::<Vec<_>>();
    segment_digests.sort_unstable();
    out.extend_from_slice(&len_u64(segment_digests.len()).to_le_bytes());
    for digest in &segment_digests {
        push_hash(&mut out, digest);
    }

    match &root.recovery_certificate {
        Some(certificate) => {
            out.push(1);
            push_hash(&mut out, &certificate.identity_digest());
        }
        None => out.push(0),
    }
    out
}

fn wal_projection_writer_epoch_payload(epoch: &WalWriterEpoch) -> Vec<u8> {
    let mut out = Vec::new();
    push_hash(&mut out, &epoch.epoch_id.as_hash());
    push_hash(&mut out, &epoch.storage_fencing_token);
    push_hash(&mut out, &epoch.process_identity);
    push_hash(&mut out, &epoch.host_identity);
    out.extend_from_slice(&epoch.started_at_lsn.as_u64().to_le_bytes());
    match epoch.previous_epoch_id {
        Some(previous_epoch_id) => {
            out.push(1);
            push_hash(&mut out, &previous_epoch_id.as_hash());
        }
        None => out.push(0),
    }
    push_optional_hash(&mut out, epoch.previous_epoch_final_commit_digest);
    push_hash(&mut out, &epoch.lease_or_lock_evidence);
    out
}

fn wal_projection_segment_payload(segment: &WalSegmentRef) -> Vec<u8> {
    let mut out = Vec::new();
    push_hash(&mut out, &segment.writer_epoch.as_hash());
    out.extend_from_slice(&segment.segment_id.as_u64().to_le_bytes());
    out.extend_from_slice(&segment.first_lsn.as_u64().to_le_bytes());
    out.extend_from_slice(&segment.last_lsn.as_u64().to_le_bytes());
    push_hash(&mut out, &segment.previous_commit_digest);
    push_hash(&mut out, &segment.final_commit_digest);
    push_hash(&mut out, &segment.segment_digest);

    let mut commit_anchor_digests = segment
        .commit_anchors
        .iter()
        .map(WalCommitAnchor::identity_digest)
        .collect::<Vec<_>>();
    commit_anchor_digests.sort_unstable();
    out.extend_from_slice(&len_u64(commit_anchor_digests.len()).to_le_bytes());
    for digest in &commit_anchor_digests {
        push_hash(&mut out, digest);
    }

    push_wal_segment_seal_posture(&mut out, &segment.seal_posture);
    out
}

fn wal_projection_commit_anchor_payload(anchor: &WalCommitAnchor) -> Vec<u8> {
    let mut out = Vec::new();
    push_hash(&mut out, &anchor.transaction_id.as_hash());
    push_hash(&mut out, &anchor.commit_digest);
    out.extend_from_slice(&anchor.first_lsn.as_u64().to_le_bytes());
    out.extend_from_slice(&anchor.last_lsn.as_u64().to_le_bytes());
    out.extend_from_slice(&anchor.record_count.to_le_bytes());
    out
}

fn wal_projection_recovery_certificate_payload(certificate: &RecoveryCertificateRef) -> Vec<u8> {
    let mut out = Vec::new();
    push_hash(&mut out, &certificate.certificate_digest);
    push_optional_hash(&mut out, certificate.checkpoint_used);
    push_optional_lsn(&mut out, certificate.first_lsn);
    push_optional_lsn(&mut out, certificate.last_lsn);
    push_recovery_tail_posture(&mut out, &certificate.tail_posture);
    push_hash(&mut out, &certificate.recovered_frontier_root);
    push_hash(&mut out, &certificate.recovered_indexes_root);
    out
}

fn push_wal_segment_seal_posture(out: &mut Vec<u8>, posture: &WalSegmentSealPosture) {
    match posture {
        WalSegmentSealPosture::Open => out.push(0),
        WalSegmentSealPosture::Sealed { sealed_lsn } => {
            out.push(1);
            push_optional_lsn(out, *sealed_lsn);
        }
    }
}

fn push_recovery_tail_posture(out: &mut Vec<u8>, posture: &RecoveryTailPosture) {
    match posture {
        RecoveryTailPosture::Clean => out.push(0),
        RecoveryTailPosture::TruncatedAll => out.push(1),
        RecoveryTailPosture::TruncatedAfter(lsn) => {
            out.push(2);
            out.extend_from_slice(&lsn.as_u64().to_le_bytes());
        }
        RecoveryTailPosture::WouldTruncateAll => out.push(3),
        RecoveryTailPosture::WouldTruncateAfter(lsn) => {
            out.push(4);
            out.extend_from_slice(&lsn.as_u64().to_le_bytes());
        }
    }
}

/// Read-only WAL doctor posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalDoctorPosture {
    /// WAL can be recovered from committed history.
    Recoverable,
    /// WAL is inspectable, but read-only mode detected a tail that would be
    /// truncated by writable recovery.
    RecoverableWithUncommittedTail,
    /// WAL has a recovery obstruction.
    Obstructed,
}

/// Read-only WAL doctor report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalDoctorReport {
    /// Doctor posture.
    pub posture: WalDoctorPosture,
    /// Recovery certificate.
    pub recovery_certificate: RecoveryCertificate,
    /// Tail posture.
    pub tail_posture: RecoveryTailPosture,
}

impl WalDoctorReport {
    /// Stable report field names for CLI/MCP adapters.
    #[must_use]
    pub const fn stable_field_names() -> &'static [&'static str] {
        &[
            "posture",
            "tail_posture",
            "committed_transactions_replayed",
            "obstruction_count",
            "recovered_frontier_root",
            "recovered_indexes_root",
        ]
    }
}

/// Filesystem WAL operation that may be faulted by host-test fixtures.
#[cfg(any(test, feature = "host_test"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesystemWalFaultTarget {
    /// Fail the next frame append before writing segment bytes.
    AppendFrame,
    /// Fail the next commit flush before writing the commit marker.
    FlushCommit,
    /// Fail the next manifest publish before writing manifest material.
    PublishManifest,
}

/// Host-test-only filesystem WAL fault plan.
#[cfg(any(test, feature = "host_test"))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilesystemWalFaultPlan {
    append_frame: u32,
    flush_commit: u32,
    publish_manifest: u32,
}

#[cfg(any(test, feature = "host_test"))]
impl FilesystemWalFaultPlan {
    /// Builds a plan that fails the next operation for `target`.
    #[must_use]
    pub const fn fail_next(target: FilesystemWalFaultTarget) -> Self {
        match target {
            FilesystemWalFaultTarget::AppendFrame => Self {
                append_frame: 1,
                flush_commit: 0,
                publish_manifest: 0,
            },
            FilesystemWalFaultTarget::FlushCommit => Self {
                append_frame: 0,
                flush_commit: 1,
                publish_manifest: 0,
            },
            FilesystemWalFaultTarget::PublishManifest => Self {
                append_frame: 0,
                flush_commit: 0,
                publish_manifest: 1,
            },
        }
    }

    fn should_fail(&mut self, target: FilesystemWalFaultTarget) -> bool {
        let remaining = match target {
            FilesystemWalFaultTarget::AppendFrame => &mut self.append_frame,
            FilesystemWalFaultTarget::FlushCommit => &mut self.flush_commit,
            FilesystemWalFaultTarget::PublishManifest => &mut self.publish_manifest,
        };
        if *remaining == 0 {
            return false;
        }
        *remaining -= 1;
        true
    }
}

/// Local filesystem WAL store backed by segment files.
#[derive(Clone, Debug)]
pub struct FilesystemWalStore {
    root: PathBuf,
    segment_id: WalSegmentId,
    active_epoch: Option<WriterEpoch>,
    closed_epochs: Vec<WriterEpoch>,
    epoch_closures: BTreeMap<WriterEpochId, WriterEpochClosure>,
    manifests: Vec<WalManifest>,
    sync_evidence: Vec<FilesystemSyncEvidence>,
    #[cfg(any(test, feature = "host_test"))]
    fault_plan: FilesystemWalFaultPlan,
}

impl FilesystemWalStore {
    /// Opens a filesystem WAL store rooted at `root`.
    ///
    /// Segment creation fsyncs the containing directory. Frame appends are
    /// written to the active segment; commit flushing syncs the file and is the
    /// durable ACK boundary for strict filesystem mode.
    pub fn open(root: impl AsRef<Path>, segment_id: WalSegmentId) -> Result<Self, WalStoreError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let segments_dir = segments_dir(&root);
        let segments_dir_existed = segments_dir.exists();
        fs::create_dir_all(&segments_dir)?;
        let segment_path = segment_path(&root, segment_id);
        let mut sync_evidence = Vec::new();
        if !segments_dir_existed {
            sync_directory_store(&root)?;
            sync_evidence.push(FilesystemSyncEvidence::unscoped(
                FilesystemSyncBoundary::SegmentNamespaceDirectorySynced,
            ));
        }
        if !segment_path.exists() {
            File::create(&segment_path)?.sync_all()?;
            sync_evidence.push(FilesystemSyncEvidence::segment(
                FilesystemSyncBoundary::SegmentFileCreated,
                segment_id,
            ));
            sync_directory_store(&segments_dir)?;
            sync_evidence.push(FilesystemSyncEvidence::segment(
                FilesystemSyncBoundary::SegmentDirectorySynced,
                segment_id,
            ));
        }
        Ok(Self {
            root,
            segment_id,
            active_epoch: None,
            closed_epochs: Vec::new(),
            epoch_closures: BTreeMap::new(),
            manifests: Vec::new(),
            sync_evidence,
            #[cfg(any(test, feature = "host_test"))]
            fault_plan: FilesystemWalFaultPlan::default(),
        })
    }

    /// Opens a filesystem WAL store with an injected host-test fault plan.
    #[cfg(any(test, feature = "host_test"))]
    pub fn open_with_fault_plan_for_test(
        root: impl AsRef<Path>,
        segment_id: WalSegmentId,
        fault_plan: FilesystemWalFaultPlan,
    ) -> Result<Self, WalStoreError> {
        let mut store = Self::open(root, segment_id)?;
        store.fault_plan = fault_plan;
        Ok(store)
    }

    /// Replaces the host-test fault plan for this filesystem WAL store.
    #[cfg(any(test, feature = "host_test"))]
    pub fn replace_fault_plan_for_test(&mut self, fault_plan: FilesystemWalFaultPlan) {
        self.fault_plan = fault_plan;
    }

    /// Returns the active segment path.
    #[must_use]
    pub fn segment_path(&self) -> PathBuf {
        segment_path(&self.root, self.segment_id)
    }

    /// Returns the active logical segment id.
    #[must_use]
    pub const fn active_segment_id(&self) -> WalSegmentId {
        self.segment_id
    }

    /// Returns the WAL root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns strict filesystem sync evidence observed by this store.
    #[must_use]
    pub fn sync_evidence(&self) -> &[FilesystemSyncEvidence] {
        &self.sync_evidence
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

    /// Appends an uncommitted frame to simulate a torn transaction tail.
    pub fn append_uncommitted_frame(
        &mut self,
        epoch_id: WriterEpochId,
        frame: WalFrame,
    ) -> Result<(), WalStoreError> {
        self.append_frame(epoch_id, frame)
    }

    /// Seals the active segment and starts the next canonical segment.
    pub fn rotate_segment(
        &mut self,
        epoch_id: WriterEpochId,
    ) -> Result<WalSegmentSeal, WalStoreError> {
        let active_epoch = self
            .active_epoch
            .as_ref()
            .ok_or(WalStoreError::NoActiveWriterEpoch)?;
        if active_epoch.epoch_id != epoch_id {
            return Err(WalStoreError::WriterEpochMismatch);
        }
        let current_segment_id = self.segment_id;
        ensure_segment_has_no_uncommitted_tail(&self.root, current_segment_id)?;
        let seal = self.seal_segment(epoch_id, current_segment_id)?;
        let next_segment_id = next_segment_id(Some(current_segment_id))
            .map_err(|_| WalStoreError::SegmentIdOverflow)?;
        let next_segment_path = segment_path(&self.root, next_segment_id);
        if next_segment_path.exists() {
            return Err(WalStoreError::DuplicateSegment(next_segment_id));
        }
        File::create(&next_segment_path)?.sync_all()?;
        self.sync_evidence.push(FilesystemSyncEvidence::segment(
            FilesystemSyncBoundary::SegmentFileCreated,
            next_segment_id,
        ));
        sync_directory_store(&segments_dir(&self.root))?;
        self.sync_evidence.push(FilesystemSyncEvidence::segment(
            FilesystemSyncBoundary::SegmentDirectorySynced,
            next_segment_id,
        ));
        self.segment_id = next_segment_id;
        Ok(seal)
    }
}

impl WalStorePort for FilesystemWalStore {
    fn acquire_writer_epoch(
        &mut self,
        request: WriterEpochRequest,
    ) -> Result<WriterEpoch, WalStoreError> {
        let epoch = validate_writer_epoch_request(
            self.active_epoch.as_ref(),
            &self.closed_epochs,
            &self.epoch_closures,
            request,
        )?;
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
        if frame.header.segment_id != self.segment_id {
            return Err(WalStoreError::SegmentMismatch {
                expected: self.segment_id,
                actual: frame.header.segment_id,
            });
        }
        frame.validate_integrity()?;
        #[cfg(any(test, feature = "host_test"))]
        if self
            .fault_plan
            .should_fail(FilesystemWalFaultTarget::AppendFrame)
        {
            return Err(WalStoreError::Io(
                "injected filesystem WAL append_frame failure".to_owned(),
            ));
        }
        append_segment_record(&self.segment_path(), DiskWalRecord::Frame(&frame), false)
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
        #[cfg(any(test, feature = "host_test"))]
        if self
            .fault_plan
            .should_fail(FilesystemWalFaultTarget::FlushCommit)
        {
            return Err(WalStoreError::Io(
                "injected filesystem WAL flush_commit failure".to_owned(),
            ));
        }
        let transaction_id = commit.transaction_id;
        append_segment_record(&self.segment_path(), DiskWalRecord::Commit(&commit), true)?;
        self.epoch_closures.insert(
            epoch_id,
            WriterEpochClosure {
                final_lsn: Some(commit.last_lsn),
                final_commit_digest: Some(commit.commit_digest),
            },
        );
        self.sync_evidence.push(FilesystemSyncEvidence::transaction(
            FilesystemSyncBoundary::CommitFileSynced,
            transaction_id,
        ));
        Ok(())
    }

    fn read_frames(&self) -> Vec<WalFrame> {
        match read_filesystem_segments(&self.root) {
            Ok((frames, _, _)) => frames,
            Err(_) => Vec::new(),
        }
    }

    fn read_commits(&self) -> Vec<WalTransactionCommit> {
        match read_filesystem_segments(&self.root) {
            Ok((_, commits, _)) => commits,
            Err(_) => Vec::new(),
        }
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
        let (frames, _, _) = read_segment_file(&segment_path(&self.root, segment_id))?;
        let frame_refs = frames.iter().collect::<Vec<_>>();
        let seal = WalSegmentSeal {
            segment_id,
            sealed_lsn: frames.iter().map(|frame| frame.header.lsn).max(),
            segment_digest: segment_digest(segment_id, &frame_refs),
        };
        Ok(seal)
    }

    fn truncate_tail_after(&mut self, after_lsn: Lsn) -> Result<(), WalStoreError> {
        rewrite_filesystem_segments_after_truncation(&self.root, after_lsn)
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
        #[cfg(any(test, feature = "host_test"))]
        if self
            .fault_plan
            .should_fail(FilesystemWalFaultTarget::PublishManifest)
        {
            return Err(WalStoreError::Io(
                "injected filesystem WAL publish_manifest failure".to_owned(),
            ));
        }
        write_manifest_atomic(&self.root, &manifest)?;
        self.sync_evidence.push(FilesystemSyncEvidence::unscoped(
            FilesystemSyncBoundary::ManifestTempFileSynced,
        ));
        self.sync_evidence.push(FilesystemSyncEvidence::unscoped(
            FilesystemSyncBoundary::ManifestRenamedDirectorySynced,
        ));
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
        self.epoch_closures.entry(epoch_id).or_default();
        self.closed_epochs.push(epoch);
        Ok(())
    }
}

/// Recovers committed transactions from filesystem WAL segments.
pub fn recover_filesystem_store(
    root: impl AsRef<Path>,
    mode: RecoveryAccessMode,
) -> Result<RecoveryScanReport, WalRecoveryError> {
    let root = root.as_ref();
    let (frames, commits, torn_tail) = read_filesystem_segments(root)?;
    let mut report = recover_from_frames_and_commits(&frames, &commits, mode)?;
    if torn_tail && matches!(report.tail_posture, RecoveryTailPosture::Clean) {
        report.tail_posture = match mode {
            RecoveryAccessMode::Writable => {
                if let Some(lsn) = report.last_committed_lsn() {
                    RecoveryTailPosture::TruncatedAfter(lsn)
                } else {
                    RecoveryTailPosture::TruncatedAll
                }
            }
            RecoveryAccessMode::ReadOnly => {
                if let Some(lsn) = report.last_committed_lsn() {
                    RecoveryTailPosture::WouldTruncateAfter(lsn)
                } else {
                    RecoveryTailPosture::WouldTruncateAll
                }
            }
        };
    }
    match (mode, report.tail_posture) {
        (RecoveryAccessMode::Writable, RecoveryTailPosture::TruncatedAfter(lsn)) => {
            rewrite_filesystem_segments_after_truncation(root, lsn)?;
        }
        (RecoveryAccessMode::Writable, RecoveryTailPosture::TruncatedAll) => {
            clear_filesystem_segments(root)?;
        }
        _ => {}
    }
    Ok(report)
}

/// Recovers committed transactions from embedded bytes for one WAL segment.
///
/// This validates the same disk record checksums, frame integrity, transaction
/// commit chain, and read-only tail posture used by filesystem WAL recovery,
/// but it does not require access to the original filesystem root.
pub fn recover_wal_segment_bytes(
    segment_id: WalSegmentId,
    bytes: &[u8],
    mode: RecoveryAccessMode,
) -> Result<WalSegmentBytesRecovery, WalRecoveryError> {
    let (frames, commits, torn_tail) = read_segment_bytes(bytes)?;
    for frame in &frames {
        if frame.header.segment_id != segment_id {
            return Err(WalStoreError::SegmentMismatch {
                expected: segment_id,
                actual: frame.header.segment_id,
            }
            .into());
        }
    }
    let frame_refs = frames.iter().collect::<Vec<_>>();
    let segment_digest = segment_digest(segment_id, &frame_refs);
    let mut report = recover_from_frames_and_commits(&frames, &commits, mode)?;
    if torn_tail && matches!(report.tail_posture, RecoveryTailPosture::Clean) {
        report.tail_posture = match mode {
            RecoveryAccessMode::Writable => report.last_committed_lsn().map_or(
                RecoveryTailPosture::TruncatedAll,
                RecoveryTailPosture::TruncatedAfter,
            ),
            RecoveryAccessMode::ReadOnly => report.last_committed_lsn().map_or(
                RecoveryTailPosture::WouldTruncateAll,
                RecoveryTailPosture::WouldTruncateAfter,
            ),
        };
    }
    Ok(WalSegmentBytesRecovery {
        segment_id,
        segment_digest,
        report,
    })
}

/// Runs a read-only WAL doctor over filesystem WAL segments.
pub fn doctor_filesystem_store(
    root: impl AsRef<Path>,
) -> Result<WalDoctorReport, WalRecoveryError> {
    let report = match recover_filesystem_store(root, RecoveryAccessMode::ReadOnly) {
        Ok(report) => report,
        Err(_) => return Ok(obstructed_doctor_report()),
    };
    Ok(doctor_report_from_scan(report, 0, [0; 32], [0; 32]))
}

/// Reads the published filesystem WAL manifest, if present.
pub fn read_filesystem_manifest(
    root: impl AsRef<Path>,
) -> Result<Option<WalManifest>, WalStoreError> {
    let path = root.as_ref().join("manifest.ecwal");
    if !path.exists() {
        return Ok(None);
    }
    read_manifest_file(&path).map(Some)
}

/// Validates the published filesystem WAL manifest against segment contents.
pub fn validate_filesystem_manifest(
    root: impl AsRef<Path>,
) -> Result<WalManifestValidationReport, WalStoreError> {
    let root = root.as_ref();
    let manifest = read_filesystem_manifest(root)?.ok_or(WalStoreError::MissingManifest)?;
    let segment_count = len_u64(segment_paths(root)?.len());
    let (frames, commits, torn_tail) = read_filesystem_segments(root)?;
    if torn_tail {
        return Err(WalStoreError::ManifestCannotValidateUncommittedTail);
    }
    let last_committed_lsn = commits.iter().map(|commit| commit.last_lsn).max();
    if frames
        .iter()
        .any(|frame| last_committed_lsn.is_none_or(|lsn| frame.header.lsn > lsn))
    {
        return Err(WalStoreError::ManifestCannotValidateUncommittedTail);
    }
    let last_commit_digest = commits.last().map(|commit| commit.commit_digest);
    if manifest.sealed_segment_count != segment_count {
        return Err(WalStoreError::ManifestSegmentCountMismatch {
            expected: segment_count,
            actual: manifest.sealed_segment_count,
        });
    }
    if manifest.last_committed_lsn != last_committed_lsn {
        return Err(WalStoreError::ManifestLastCommittedLsnMismatch {
            expected: last_committed_lsn,
            actual: manifest.last_committed_lsn,
        });
    }
    if manifest.last_commit_digest != last_commit_digest {
        return Err(WalStoreError::ManifestLastCommitDigestMismatch {
            expected: last_commit_digest,
            actual: manifest.last_commit_digest,
        });
    }
    Ok(WalManifestValidationReport {
        manifest,
        segment_count,
        last_committed_lsn,
        last_commit_digest,
    })
}

fn filesystem_wal_recovery_segment_evidence(
    root: &Path,
) -> Result<Vec<WalRecoverySegmentEvidence>, WalStoreError> {
    let mut evidence = Vec::new();
    for path in segment_paths(root)? {
        let segment_id = parse_segment_id(&path)?;
        let (frames, _, torn_tail) = read_segment_file(&path)?;
        if torn_tail {
            return Err(WalStoreError::ManifestCannotValidateUncommittedTail);
        }
        let frame_refs = frames.iter().collect::<Vec<_>>();
        let storage_locator = match path.strip_prefix(root) {
            Ok(relative) => WalSegmentStorageLocator::RelativePath(relative.to_path_buf()),
            Err(_) => WalSegmentStorageLocator::AbsolutePath(path.clone()),
        };
        evidence.push(WalRecoverySegmentEvidence {
            segment_id,
            segment_digest: segment_digest(segment_id, &frame_refs),
            seal_posture: WalSegmentSealPosture::Sealed {
                sealed_lsn: frames.iter().map(|frame| frame.header.lsn).max(),
            },
            storage_locator: Some(storage_locator),
        });
    }
    Ok(evidence)
}

/// Object-store read-after-write posture required by strict object storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectStoreReadAfterWritePosture {
    /// The adapter proves read-after-write behavior for committed manifests.
    Verified,
    /// The adapter cannot prove read-after-write behavior.
    Unverified,
}

/// Capability evidence for a strict object-store WAL adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectStoreWalCapabilities {
    /// Content-addressed object writes are used for segment/checkpoint bodies.
    pub content_addressed_object_write: bool,
    /// Object version or ETag evidence is verified.
    pub verify_object_version: bool,
    /// Manifest publication is conditional/compare-and-swap.
    pub conditional_manifest_commit: bool,
    /// Read-after-write posture.
    pub read_after_write: ObjectStoreReadAfterWritePosture,
}

/// Object-store capability validation error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ObjectStoreCapabilityError {
    /// Strict object stores must write content-addressed objects.
    #[error("strict object store requires content-addressed object writes")]
    MissingContentAddressedObjectWrite,
    /// Strict object stores must verify object version/ETag evidence.
    #[error("strict object store requires object version verification")]
    MissingObjectVersionVerification,
    /// Strict object stores must publish manifests conditionally.
    #[error("strict object store requires conditional manifest commit")]
    MissingConditionalManifestCommit,
    /// Strict object stores must prove read-after-write behavior.
    #[error("strict object store requires verified read-after-write posture")]
    MissingReadAfterWrite,
    /// Strict object stores must not overwrite manifests unconditionally.
    #[error("strict object store manifest commit must be conditional compare-and-swap")]
    UnconditionalManifestOverwrite,
}

/// Validates strict object-store WAL adapter capabilities.
pub fn validate_strict_object_store_capabilities(
    capabilities: ObjectStoreWalCapabilities,
) -> Result<(), ObjectStoreCapabilityError> {
    if !capabilities.content_addressed_object_write {
        return Err(ObjectStoreCapabilityError::MissingContentAddressedObjectWrite);
    }
    if !capabilities.verify_object_version {
        return Err(ObjectStoreCapabilityError::MissingObjectVersionVerification);
    }
    if !capabilities.conditional_manifest_commit {
        return Err(ObjectStoreCapabilityError::MissingConditionalManifestCommit);
    }
    if capabilities.read_after_write != ObjectStoreReadAfterWritePosture::Verified {
        return Err(ObjectStoreCapabilityError::MissingReadAfterWrite);
    }
    Ok(())
}

/// Object-store manifest commit mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectStoreManifestCommitMode {
    /// Manifest publication uses compare-and-swap against an expected previous digest.
    ConditionalCompareAndSwap,
    /// Manifest publication overwrites without an expected previous digest.
    UnconditionalOverwrite,
}

/// Object-store manifest commit shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectStoreManifestCommitShape {
    /// Commit mode.
    pub mode: ObjectStoreManifestCommitMode,
    /// Expected previous manifest digest for compare-and-swap.
    pub expected_previous_manifest_digest: Option<Hash>,
    /// New manifest digest.
    pub new_manifest_digest: Hash,
}

/// Validates the strict object-store manifest commit shape.
pub fn validate_strict_object_store_manifest_commit(
    shape: ObjectStoreManifestCommitShape,
) -> Result<(), ObjectStoreCapabilityError> {
    if shape.mode != ObjectStoreManifestCommitMode::ConditionalCompareAndSwap
        || shape.expected_previous_manifest_digest.is_none()
    {
        return Err(ObjectStoreCapabilityError::UnconditionalManifestOverwrite);
    }
    Ok(())
}

/// Side-effect materialization intent recorded after committed authorization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterializationIntentRecord {
    /// Stable effect id.
    pub effect_id: Hash,
    /// Expected external artifact digest.
    pub expected_artifact_digest: Hash,
    /// Materialization intent digest.
    pub materialization_intent_digest: Hash,
    /// Idempotency token for replay.
    pub idempotency_token: Hash,
    /// Target metadata digest.
    pub target_metadata_digest: Hash,
}

impl MaterializationIntentRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.effect_id);
        push_hash(&mut out, &self.expected_artifact_digest);
        push_hash(&mut out, &self.materialization_intent_digest);
        push_hash(&mut out, &self.idempotency_token);
        push_hash(&mut out, &self.target_metadata_digest);
        out
    }

    /// Decodes a deterministic materialization intent payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let effect_id = cursor.read_hash()?;
        let expected_artifact_digest = cursor.read_hash()?;
        let materialization_intent_digest = cursor.read_hash()?;
        let idempotency_token = cursor.read_hash()?;
        let target_metadata_digest = cursor.read_hash()?;
        cursor.finish()?;
        Ok(Self {
            effect_id,
            expected_artifact_digest,
            materialization_intent_digest,
            idempotency_token,
            target_metadata_digest,
        })
    }
}

/// Side-effect materialization observation recorded after external effect verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterializationObservationRecord {
    /// Stable effect id.
    pub effect_id: Hash,
    /// Observed external artifact digest.
    pub observed_artifact_digest: Hash,
    /// Observed metadata digest.
    pub observed_metadata_digest: Hash,
}

impl MaterializationObservationRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.effect_id);
        push_hash(&mut out, &self.observed_artifact_digest);
        push_hash(&mut out, &self.observed_metadata_digest);
        out
    }

    /// Decodes a deterministic materialization observation payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let effect_id = cursor.read_hash()?;
        let observed_artifact_digest = cursor.read_hash()?;
        let observed_metadata_digest = cursor.read_hash()?;
        cursor.finish()?;
        Ok(Self {
            effect_id,
            observed_artifact_digest,
            observed_metadata_digest,
        })
    }
}

/// Existing external artifact evidence used for idempotent outbox replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExistingMaterializedArtifact {
    /// Stable effect id.
    pub effect_id: Hash,
    /// Existing artifact digest.
    pub artifact_digest: Hash,
    /// Existing metadata digest.
    pub metadata_digest: Hash,
}

/// Materialization replay posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterializationReplayPosture {
    /// Effect is authorized but not observed.
    Pending,
    /// Effect has a committed observation.
    AlreadyObserved,
    /// External artifact already matches the authorized effect.
    ExistingArtifactMatches,
    /// External artifact exists but does not match authorization.
    Obstructed,
}

/// Typed materialization recovery posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterializationRecoveryPosture {
    /// Effect has a committed observation that matches authorization.
    AlreadyObserved {
        /// Observed artifact digest.
        observed_artifact_digest: Hash,
        /// Observed metadata digest.
        observed_metadata_digest: Hash,
    },
    /// External artifact already matches the authorized effect.
    ExistingArtifactMatches {
        /// Existing artifact digest.
        artifact_digest: Hash,
        /// Existing metadata digest.
        metadata_digest: Hash,
    },
    /// No committed observation or existing artifact evidence was available.
    MissingArtifact {
        /// Stable effect id.
        effect_id: Hash,
        /// Expected external artifact digest.
        expected_artifact_digest: Hash,
        /// Expected external metadata digest.
        expected_metadata_digest: Hash,
    },
    /// External artifact digest mismatched the authorized effect.
    ArtifactDigestMismatch {
        /// Expected external artifact digest.
        expected_artifact_digest: Hash,
        /// Actual external artifact digest.
        actual_artifact_digest: Hash,
    },
    /// External metadata digest mismatched the authorized effect.
    MetadataDigestMismatch {
        /// Expected external metadata digest.
        expected_metadata_digest: Hash,
        /// Actual external metadata digest.
        actual_metadata_digest: Hash,
    },
    /// External artifact and metadata digests both mismatched authorization.
    ArtifactAndMetadataDigestMismatch {
        /// Expected external artifact digest.
        expected_artifact_digest: Hash,
        /// Actual external artifact digest.
        actual_artifact_digest: Hash,
        /// Expected external metadata digest.
        expected_metadata_digest: Hash,
        /// Actual external metadata digest.
        actual_metadata_digest: Hash,
    },
    /// A committed observation conflicted with the authorized intent.
    ObservationDigestMismatch {
        /// Expected external artifact digest.
        expected_artifact_digest: Hash,
        /// Observed external artifact digest.
        observed_artifact_digest: Hash,
        /// Expected external metadata digest.
        expected_metadata_digest: Hash,
        /// Observed external metadata digest.
        observed_metadata_digest: Hash,
    },
    /// Retained material required to verify recovery was unavailable.
    RetainedMaterialUnavailable {
        /// Unavailable material digest.
        material_digest: Hash,
        /// Material kind.
        kind: RetainedMaterialKind,
        /// Recovery scope.
        scope: MissingMaterialScope,
        /// Evidence posture.
        posture: EvidenceMaterialPosture,
    },
}

impl MaterializationRecoveryPosture {
    /// Returns the coarse replay posture for compatibility with existing callers.
    #[must_use]
    pub const fn replay_posture(self) -> MaterializationReplayPosture {
        match self {
            Self::AlreadyObserved { .. } => MaterializationReplayPosture::AlreadyObserved,
            Self::ExistingArtifactMatches { .. } => {
                MaterializationReplayPosture::ExistingArtifactMatches
            }
            Self::MissingArtifact { .. } => MaterializationReplayPosture::Pending,
            Self::ArtifactDigestMismatch { .. }
            | Self::MetadataDigestMismatch { .. }
            | Self::ArtifactAndMetadataDigestMismatch { .. }
            | Self::ObservationDigestMismatch { .. }
            | Self::RetainedMaterialUnavailable { .. } => MaterializationReplayPosture::Obstructed,
        }
    }
}

/// Recovered materialization outbox entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterializationOutboxEntry {
    /// Intent record.
    pub intent: MaterializationIntentRecord,
    /// Observation record, if committed.
    pub observation: Option<MaterializationObservationRecord>,
    /// Replay posture.
    pub posture: MaterializationReplayPosture,
    /// Typed recovery posture.
    pub recovery_posture: MaterializationRecoveryPosture,
}

/// Builds a side-effect outbox transaction.
pub fn build_materialization_outbox_transaction(
    mut builder: WalTransactionBuilder,
    intent: MaterializationIntentRecord,
    observation: Option<MaterializationObservationRecord>,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, WalBuildError> {
    builder.push_record(
        WalRecordKind::MaterializationIntentRecorded,
        intent.to_payload_bytes(),
    )?;
    if let Some(observation) = observation {
        builder.push_record(
            WalRecordKind::MaterializationEffectObserved,
            observation.to_payload_bytes(),
        )?;
    }
    builder.commit(affected_frontiers)
}

/// Recovers materialization outbox posture from committed WAL transactions.
pub fn recover_materialization_outbox(
    report: &RecoveryScanReport,
    existing_artifacts: &BTreeMap<Hash, ExistingMaterializedArtifact>,
) -> Result<BTreeMap<Hash, MaterializationOutboxEntry>, WalRecoveryIndexError> {
    recover_materialization_outbox_with_retained_material(
        report,
        existing_artifacts,
        &RecoveredRetentionIndex::default(),
        &BTreeSet::new(),
    )
}

/// Recovers materialization outbox posture with retained-material availability.
pub fn recover_materialization_outbox_with_retained_material(
    report: &RecoveryScanReport,
    existing_artifacts: &BTreeMap<Hash, ExistingMaterializedArtifact>,
    retention_index: &RecoveredRetentionIndex,
    available_material: &BTreeSet<Hash>,
) -> Result<BTreeMap<Hash, MaterializationOutboxEntry>, WalRecoveryIndexError> {
    let mut intents = BTreeMap::new();
    let mut observations = BTreeMap::new();
    for transaction in &report.transactions {
        for frame in &transaction.frames {
            match frame.header.record_kind {
                WalRecordKind::MaterializationIntentRecorded => {
                    let intent = MaterializationIntentRecord::from_payload_bytes(
                        &frame.payload.canonical_bytes,
                    )?;
                    intents.insert(intent.effect_id, intent);
                }
                WalRecordKind::MaterializationEffectObserved => {
                    let observation = MaterializationObservationRecord::from_payload_bytes(
                        &frame.payload.canonical_bytes,
                    )?;
                    observations.insert(observation.effect_id, observation);
                }
                _ => {}
            }
        }
    }
    let retained_obstructions: BTreeMap<_, _> =
        retained_material_obstructions(retention_index, available_material)
            .into_iter()
            .map(|obstruction| (obstruction.material_digest, obstruction))
            .collect();
    let mut outbox = BTreeMap::new();
    for (effect_id, intent) in intents {
        let observation = observations.get(&effect_id).copied();
        let recovery_posture = materialization_recovery_posture(
            intent,
            observation,
            existing_artifacts.get(&effect_id).copied(),
            retained_obstructions
                .get(&intent.expected_artifact_digest)
                .copied(),
        );
        let posture = recovery_posture.replay_posture();
        outbox.insert(
            effect_id,
            MaterializationOutboxEntry {
                intent,
                observation,
                posture,
                recovery_posture,
            },
        );
    }
    Ok(outbox)
}

fn materialization_recovery_posture(
    intent: MaterializationIntentRecord,
    observation: Option<MaterializationObservationRecord>,
    existing: Option<ExistingMaterializedArtifact>,
    retained_obstruction: Option<RetainedMaterialObstruction>,
) -> MaterializationRecoveryPosture {
    if let Some(observation) = observation {
        return if observation.observed_artifact_digest == intent.expected_artifact_digest
            && observation.observed_metadata_digest == intent.target_metadata_digest
        {
            MaterializationRecoveryPosture::AlreadyObserved {
                observed_artifact_digest: observation.observed_artifact_digest,
                observed_metadata_digest: observation.observed_metadata_digest,
            }
        } else {
            MaterializationRecoveryPosture::ObservationDigestMismatch {
                expected_artifact_digest: intent.expected_artifact_digest,
                observed_artifact_digest: observation.observed_artifact_digest,
                expected_metadata_digest: intent.target_metadata_digest,
                observed_metadata_digest: observation.observed_metadata_digest,
            }
        };
    }
    if let Some(obstruction) = retained_obstruction {
        return MaterializationRecoveryPosture::RetainedMaterialUnavailable {
            material_digest: obstruction.material_digest,
            kind: obstruction.kind,
            scope: obstruction.scope,
            posture: obstruction.posture,
        };
    }
    if let Some(existing) = existing {
        let artifact_matches = existing.artifact_digest == intent.expected_artifact_digest;
        let metadata_matches = existing.metadata_digest == intent.target_metadata_digest;
        return match (artifact_matches, metadata_matches) {
            (true, true) => MaterializationRecoveryPosture::ExistingArtifactMatches {
                artifact_digest: existing.artifact_digest,
                metadata_digest: existing.metadata_digest,
            },
            (false, true) => MaterializationRecoveryPosture::ArtifactDigestMismatch {
                expected_artifact_digest: intent.expected_artifact_digest,
                actual_artifact_digest: existing.artifact_digest,
            },
            (true, false) => MaterializationRecoveryPosture::MetadataDigestMismatch {
                expected_metadata_digest: intent.target_metadata_digest,
                actual_metadata_digest: existing.metadata_digest,
            },
            (false, false) => MaterializationRecoveryPosture::ArtifactAndMetadataDigestMismatch {
                expected_artifact_digest: intent.expected_artifact_digest,
                actual_artifact_digest: existing.artifact_digest,
                expected_metadata_digest: intent.target_metadata_digest,
                actual_metadata_digest: existing.metadata_digest,
            },
        };
    }
    MaterializationRecoveryPosture::MissingArtifact {
        effect_id: intent.effect_id,
        expected_artifact_digest: intent.expected_artifact_digest,
        expected_metadata_digest: intent.target_metadata_digest,
    }
}

/// Imported suffix admission outcome stored in topology recovery evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TopologyImportOutcomeKind {
    /// The import lowered to one derived result.
    Derived,
    /// The import retained lawful plurality.
    Plural,
    /// The import produced conflict residue.
    Conflict,
    /// The import was obstructed before admission completed.
    Obstruction,
}

impl TopologyImportOutcomeKind {
    fn code(self) -> u8 {
        match self {
            Self::Derived => 1,
            Self::Plural => 2,
            Self::Conflict => 3,
            Self::Obstruction => 4,
        }
    }

    fn from_code(code: u8) -> Result<Self, WalDecodeError> {
        match code {
            1 => Ok(Self::Derived),
            2 => Ok(Self::Plural),
            3 => Ok(Self::Conflict),
            4 => Ok(Self::Obstruction),
            _ => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "TopologyImportOutcomeKind",
                code,
            }),
        }
    }
}

/// WAL evidence that one strand fork intent was accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrandForkRecord {
    /// Stable topology intent id.
    pub topology_intent_id: Hash,
    /// Forked strand identity.
    pub strand_id: StrandId,
    /// Source worldline the strand was forked from.
    pub source_worldline_id: WorldlineId,
    /// Last included source tick in the copied prefix.
    pub fork_tick: WorldlineTick,
    /// Source commit hash at `fork_tick`.
    pub source_commit_hash: Hash,
    /// Source boundary hash at `fork_tick`.
    pub source_boundary_hash: Hash,
    /// Child worldline created for the strand.
    pub child_worldline_id: WorldlineId,
    /// Writer heads created for the child worldline.
    pub writer_heads: Vec<WriterHeadKey>,
    /// Digest of the accepted retention-posture bundle.
    pub retention_posture_digest: Hash,
    /// Issuer or session evidence digest.
    pub issuer_evidence_digest: Hash,
    /// Optional idempotency key supplied with the fork intent.
    pub idempotency_key_digest: Option<Hash>,
}

impl StrandForkRecord {
    /// Returns a copy with writer heads in canonical `(worldline_id, head_id)` order.
    #[must_use]
    pub fn canonicalized(&self) -> Self {
        let mut record = self.clone();
        record.writer_heads = canonical_writer_heads(&record.writer_heads);
        record
    }

    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let writer_heads = canonical_writer_heads(&self.writer_heads);
        let mut out = Vec::new();
        push_hash(&mut out, &self.topology_intent_id);
        push_strand_id(&mut out, self.strand_id);
        push_worldline_id(&mut out, self.source_worldline_id);
        push_worldline_tick(&mut out, self.fork_tick);
        push_hash(&mut out, &self.source_commit_hash);
        push_hash(&mut out, &self.source_boundary_hash);
        push_worldline_id(&mut out, self.child_worldline_id);
        out.extend_from_slice(&len_u64(writer_heads.len()).to_le_bytes());
        for head in &writer_heads {
            push_writer_head_key(&mut out, *head);
        }
        push_hash(&mut out, &self.retention_posture_digest);
        push_hash(&mut out, &self.issuer_evidence_digest);
        push_optional_hash(&mut out, self.idempotency_key_digest);
        out
    }

    /// Decodes a deterministic strand fork payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let topology_intent_id = cursor.read_hash()?;
        let strand_id = cursor.read_strand_id()?;
        let source_worldline_id = cursor.read_worldline_id()?;
        let fork_tick = cursor.read_worldline_tick()?;
        let source_commit_hash = cursor.read_hash()?;
        let source_boundary_hash = cursor.read_hash()?;
        let child_worldline_id = cursor.read_worldline_id()?;
        let writer_count =
            usize::try_from(cursor.read_u64()?).map_err(|_| WalDecodeError::UnexpectedEof)?;
        let writer_bytes = writer_count
            .checked_mul(WRITER_HEAD_KEY_PAYLOAD_LEN)
            .ok_or(WalDecodeError::UnexpectedEof)?;
        if writer_bytes > cursor.remaining_len() {
            return Err(WalDecodeError::UnexpectedEof);
        }
        let mut writer_heads = Vec::with_capacity(writer_count);
        for _ in 0..writer_count {
            writer_heads.push(cursor.read_writer_head_key()?);
        }
        let writer_heads = canonical_writer_heads(&writer_heads);
        let retention_posture_digest = cursor.read_hash()?;
        let issuer_evidence_digest = cursor.read_hash()?;
        let idempotency_key_digest = cursor.read_optional_hash()?;
        cursor.finish()?;
        Ok(Self {
            topology_intent_id,
            strand_id,
            source_worldline_id,
            fork_tick,
            source_commit_hash,
            source_boundary_hash,
            child_worldline_id,
            writer_heads,
            retention_posture_digest,
            issuer_evidence_digest,
            idempotency_key_digest,
        })
    }
}

/// WAL evidence that one strand drop intent was accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrandDropRecord {
    /// Stable topology intent id.
    pub topology_intent_id: Hash,
    /// Dropped strand identity.
    pub strand_id: StrandId,
    /// Child worldline removed from the live strand registry.
    pub child_worldline_id: WorldlineId,
    /// Final child-worldline tick at drop time.
    pub final_tick: WorldlineTick,
    /// Digest of the drop receipt returned after durable acceptance.
    pub drop_receipt_digest: Hash,
    /// Issuer or session evidence digest.
    pub issuer_evidence_digest: Hash,
    /// Optional idempotency key supplied with the drop intent.
    pub idempotency_key_digest: Option<Hash>,
}

impl StrandDropRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.topology_intent_id);
        push_strand_id(&mut out, self.strand_id);
        push_worldline_id(&mut out, self.child_worldline_id);
        push_worldline_tick(&mut out, self.final_tick);
        push_hash(&mut out, &self.drop_receipt_digest);
        push_hash(&mut out, &self.issuer_evidence_digest);
        push_optional_hash(&mut out, self.idempotency_key_digest);
        out
    }

    /// Decodes a deterministic strand drop payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let topology_intent_id = cursor.read_hash()?;
        let strand_id = cursor.read_strand_id()?;
        let child_worldline_id = cursor.read_worldline_id()?;
        let final_tick = cursor.read_worldline_tick()?;
        let drop_receipt_digest = cursor.read_hash()?;
        let issuer_evidence_digest = cursor.read_hash()?;
        let idempotency_key_digest = cursor.read_optional_hash()?;
        cursor.finish()?;
        Ok(Self {
            topology_intent_id,
            strand_id,
            child_worldline_id,
            final_tick,
            drop_receipt_digest,
            issuer_evidence_digest,
            idempotency_key_digest,
        })
    }
}

/// WAL evidence that one braid lifecycle event was accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyBraidEventRecord {
    /// Stable topology intent id.
    pub topology_intent_id: Hash,
    /// Braid whose event log accepted the event.
    pub braid_id: Hash,
    /// Sequence number in the durable braid event log.
    pub event_index: u64,
    /// Accepted braid event.
    pub event: BraidEvent,
    /// Folded braid status after accepting the event.
    pub status_after: BraidStatus,
    /// Digest of the accepted event payload.
    pub event_digest: Hash,
    /// Issuer or session evidence digest.
    pub issuer_evidence_digest: Hash,
    /// Optional idempotency key supplied with the braid event intent.
    pub idempotency_key_digest: Option<Hash>,
}

impl TopologyBraidEventRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.topology_intent_id);
        push_hash(&mut out, &self.braid_id);
        out.extend_from_slice(&self.event_index.to_le_bytes());
        push_braid_event(&mut out, &self.event);
        out.push(braid_status_code(self.status_after));
        push_hash(&mut out, &self.event_digest);
        push_hash(&mut out, &self.issuer_evidence_digest);
        push_optional_hash(&mut out, self.idempotency_key_digest);
        out
    }

    /// Decodes a deterministic braid event payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let topology_intent_id = cursor.read_hash()?;
        let braid_id = cursor.read_hash()?;
        let event_index = cursor.read_u64()?;
        let event = cursor.read_braid_event()?;
        let status_after = braid_status_from_code(cursor.read_u8()?)?;
        let event_digest = cursor.read_hash()?;
        let issuer_evidence_digest = cursor.read_hash()?;
        let idempotency_key_digest = cursor.read_optional_hash()?;
        cursor.finish()?;
        Ok(Self {
            topology_intent_id,
            braid_id,
            event_index,
            event,
            status_after,
            event_digest,
            issuer_evidence_digest,
            idempotency_key_digest,
        })
    }
}

/// WAL evidence that a braid shell was retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BraidShellRetentionRecord {
    /// Stable topology intent id.
    pub topology_intent_id: Hash,
    /// Braid associated with the shell.
    pub braid_id: Hash,
    /// Retained shell digest.
    pub shell_digest: Hash,
    /// Digest of retained shell material.
    pub material_digest: Hash,
    /// Digest of the shell coordinate or basis.
    pub basis_digest: Hash,
    /// Admission outcome family carried by the shell.
    pub outcome_kind: TopologyImportOutcomeKind,
    /// Retention posture digest for the shell material.
    pub retention_posture_digest: Hash,
    /// Witness digest binding the retained hologram.
    pub witness_digest: Hash,
    /// Optional idempotency key supplied with the retention intent.
    pub idempotency_key_digest: Option<Hash>,
}

impl BraidShellRetentionRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.topology_intent_id);
        push_hash(&mut out, &self.braid_id);
        push_hash(&mut out, &self.shell_digest);
        push_hash(&mut out, &self.material_digest);
        push_hash(&mut out, &self.basis_digest);
        out.push(self.outcome_kind.code());
        push_hash(&mut out, &self.retention_posture_digest);
        push_hash(&mut out, &self.witness_digest);
        push_optional_hash(&mut out, self.idempotency_key_digest);
        out
    }

    /// Decodes a deterministic retained braid-shell payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let topology_intent_id = cursor.read_hash()?;
        let braid_id = cursor.read_hash()?;
        let shell_digest = cursor.read_hash()?;
        let material_digest = cursor.read_hash()?;
        let basis_digest = cursor.read_hash()?;
        let outcome_kind = TopologyImportOutcomeKind::from_code(cursor.read_u8()?)?;
        let retention_posture_digest = cursor.read_hash()?;
        let witness_digest = cursor.read_hash()?;
        let idempotency_key_digest = cursor.read_optional_hash()?;
        cursor.finish()?;
        Ok(Self {
            topology_intent_id,
            braid_id,
            shell_digest,
            material_digest,
            basis_digest,
            outcome_kind,
            retention_posture_digest,
            witness_digest,
            idempotency_key_digest,
        })
    }
}

/// WAL evidence that one remote suffix import intent was accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuffixImportRecord {
    /// Stable import intent id.
    pub import_id: Hash,
    /// Digest naming the remote suffix family.
    pub remote_suffix_family_digest: Hash,
    /// Digest of remote authorship evidence.
    pub authorship_evidence_digest: Hash,
    /// Digest of comparable local/remote basis anchors.
    pub basis_anchor_digest: Hash,
    /// Digest of the transported causal suffix bundle.
    pub bundle_digest: Hash,
    /// Digest of the source suffix shell.
    pub source_shell_digest: Hash,
    /// Digest of the target basis used for judgment.
    pub target_basis_digest: Hash,
    /// Top-level import admission outcome.
    pub outcome_kind: TopologyImportOutcomeKind,
    /// Digest of the retained import shell or obstruction shell.
    pub import_shell_digest: Hash,
    /// Retention posture digest for import evidence.
    pub retention_posture_digest: Hash,
    /// Idempotency key for duplicate transport delivery.
    pub idempotency_key_digest: Hash,
}

impl SuffixImportRecord {
    /// Encodes the record as deterministic WAL payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_hash(&mut out, &self.import_id);
        push_hash(&mut out, &self.remote_suffix_family_digest);
        push_hash(&mut out, &self.authorship_evidence_digest);
        push_hash(&mut out, &self.basis_anchor_digest);
        push_hash(&mut out, &self.bundle_digest);
        push_hash(&mut out, &self.source_shell_digest);
        push_hash(&mut out, &self.target_basis_digest);
        out.push(self.outcome_kind.code());
        push_hash(&mut out, &self.import_shell_digest);
        push_hash(&mut out, &self.retention_posture_digest);
        push_hash(&mut out, &self.idempotency_key_digest);
        out
    }

    /// Decodes a deterministic suffix import payload.
    pub fn from_payload_bytes(bytes: &[u8]) -> Result<Self, WalDecodeError> {
        let mut cursor = WalPayloadCursor::new(bytes);
        let import_id = cursor.read_hash()?;
        let remote_suffix_family_digest = cursor.read_hash()?;
        let authorship_evidence_digest = cursor.read_hash()?;
        let basis_anchor_digest = cursor.read_hash()?;
        let bundle_digest = cursor.read_hash()?;
        let source_shell_digest = cursor.read_hash()?;
        let target_basis_digest = cursor.read_hash()?;
        let outcome_kind = TopologyImportOutcomeKind::from_code(cursor.read_u8()?)?;
        let import_shell_digest = cursor.read_hash()?;
        let retention_posture_digest = cursor.read_hash()?;
        let idempotency_key_digest = cursor.read_hash()?;
        cursor.finish()?;
        Ok(Self {
            import_id,
            remote_suffix_family_digest,
            authorship_evidence_digest,
            basis_anchor_digest,
            bundle_digest,
            source_shell_digest,
            target_basis_digest,
            outcome_kind,
            import_shell_digest,
            retention_posture_digest,
            idempotency_key_digest,
        })
    }
}

/// One topology evidence record carried by a topology-intent transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TopologyIntentRecord {
    /// Strand fork evidence.
    StrandFork(StrandForkRecord),
    /// Strand drop evidence.
    StrandDrop(StrandDropRecord),
    /// Braid event evidence.
    BraidEvent(TopologyBraidEventRecord),
    /// Braid shell retention evidence.
    BraidShell(BraidShellRetentionRecord),
    /// Replica suffix import evidence.
    SuffixImport(SuffixImportRecord),
}

impl TopologyIntentRecord {
    /// Returns the WAL record kind used by this topology evidence.
    #[must_use]
    pub const fn record_kind(&self) -> WalRecordKind {
        match self {
            Self::StrandFork(_) => WalRecordKind::TopologyStrandForkRecorded,
            Self::StrandDrop(_) => WalRecordKind::TopologyStrandDropRecorded,
            Self::BraidEvent(_) => WalRecordKind::TopologyBraidEventRecorded,
            Self::BraidShell(_) => WalRecordKind::TopologyBraidShellRetained,
            Self::SuffixImport(_) => WalRecordKind::TopologySuffixImportRecorded,
        }
    }

    /// Encodes this topology record to deterministic payload bytes.
    #[must_use]
    pub fn to_payload_bytes(&self) -> Vec<u8> {
        match self {
            Self::StrandFork(record) => record.to_payload_bytes(),
            Self::StrandDrop(record) => record.to_payload_bytes(),
            Self::BraidEvent(record) => record.to_payload_bytes(),
            Self::BraidShell(record) => record.to_payload_bytes(),
            Self::SuffixImport(record) => record.to_payload_bytes(),
        }
    }
}

/// Recovered topology indexes rebuilt from committed WAL/WSC evidence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveredTopologyIndex {
    /// Accepted strand forks by strand id.
    pub strand_forks: BTreeMap<StrandId, StrandForkRecord>,
    /// Accepted strand drops by strand id.
    pub strand_drops: BTreeMap<StrandId, StrandDropRecord>,
    /// Child-worldline ancestry index.
    pub child_worldlines: BTreeMap<WorldlineId, StrandId>,
    /// Accepted braid event logs by braid id.
    pub braid_events: BTreeMap<Hash, Vec<TopologyBraidEventRecord>>,
    /// Retained braid-shell records by shell digest.
    pub braid_shells: BTreeMap<Hash, BraidShellRetentionRecord>,
    /// Suffix imports by explicit import id.
    pub suffix_imports: BTreeMap<Hash, SuffixImportRecord>,
    /// Suffix imports by transport idempotency key.
    pub suffix_imports_by_idempotency_key: BTreeMap<Hash, Hash>,
    /// Suffix imports by bundle digest.
    pub suffix_imports_by_bundle_digest: BTreeMap<Hash, Hash>,
}

impl RecoveredTopologyIndex {
    /// Builds topology indexes from recovered topology evidence.
    ///
    /// Duplicate identical evidence is idempotent. Duplicate identities with
    /// divergent material are recovery obstructions.
    pub fn from_topology_records<I>(records: I) -> Result<Self, WalRecoveryIndexError>
    where
        I: IntoIterator<Item = TopologyIntentRecord>,
    {
        let mut index = Self::default();
        let mut braid_event_maps: BTreeMap<Hash, BTreeMap<u64, TopologyBraidEventRecord>> =
            BTreeMap::new();

        for record in records {
            match record {
                TopologyIntentRecord::StrandFork(record) => {
                    let record = record.canonicalized();
                    if let Some(drop) = index.strand_drops.get(&record.strand_id) {
                        if drop.child_worldline_id != record.child_worldline_id {
                            return Err(WalRecoveryIndexError::ConflictingStrandFork {
                                strand_id: record.strand_id,
                            });
                        }
                    }
                    insert_unique(
                        &mut index.strand_forks,
                        record.strand_id,
                        record.clone(),
                        WalRecoveryIndexError::ConflictingStrandFork {
                            strand_id: record.strand_id,
                        },
                    )?;
                    insert_unique(
                        &mut index.child_worldlines,
                        record.child_worldline_id,
                        record.strand_id,
                        WalRecoveryIndexError::ConflictingChildWorldline {
                            child_worldline_id: record.child_worldline_id,
                        },
                    )?;
                }
                TopologyIntentRecord::StrandDrop(record) => {
                    if let Some(fork) = index.strand_forks.get(&record.strand_id) {
                        if fork.child_worldline_id != record.child_worldline_id {
                            return Err(WalRecoveryIndexError::ConflictingStrandDrop {
                                strand_id: record.strand_id,
                            });
                        }
                    }
                    insert_unique(
                        &mut index.strand_drops,
                        record.strand_id,
                        record.clone(),
                        WalRecoveryIndexError::ConflictingStrandDrop {
                            strand_id: record.strand_id,
                        },
                    )?;
                }
                TopologyIntentRecord::BraidEvent(record) => {
                    validate_topology_braid_event(&record)?;
                    let per_braid = braid_event_maps.entry(record.braid_id).or_default();
                    insert_unique(
                        per_braid,
                        record.event_index,
                        record.clone(),
                        WalRecoveryIndexError::ConflictingBraidEvent {
                            braid_id: record.braid_id,
                            event_index: record.event_index,
                        },
                    )?;
                }
                TopologyIntentRecord::BraidShell(record) => {
                    insert_unique(
                        &mut index.braid_shells,
                        record.shell_digest,
                        record.clone(),
                        WalRecoveryIndexError::ConflictingBraidShell {
                            shell_digest: record.shell_digest,
                        },
                    )?;
                }
                TopologyIntentRecord::SuffixImport(record) => {
                    insert_unique(
                        &mut index.suffix_imports,
                        record.import_id,
                        record.clone(),
                        WalRecoveryIndexError::ConflictingSuffixImport {
                            import_id: record.import_id,
                        },
                    )?;
                    insert_unique(
                        &mut index.suffix_imports_by_idempotency_key,
                        record.idempotency_key_digest,
                        record.import_id,
                        WalRecoveryIndexError::ConflictingSuffixImportIdempotency {
                            idempotency_key_digest: record.idempotency_key_digest,
                        },
                    )?;
                    insert_unique(
                        &mut index.suffix_imports_by_bundle_digest,
                        record.bundle_digest,
                        record.import_id,
                        WalRecoveryIndexError::ConflictingSuffixImportBundle {
                            bundle_digest: record.bundle_digest,
                        },
                    )?;
                }
            }
        }

        index.braid_events = braid_event_maps
            .into_iter()
            .map(|(braid_id, events)| (braid_id, events.into_values().collect()))
            .collect();
        Ok(index)
    }

    /// Returns the number of accepted topology evidence records in the index.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strand_forks.len()
            + self.strand_drops.len()
            + self
                .braid_events
                .values()
                .map(std::vec::Vec::len)
                .sum::<usize>()
            + self.braid_shells.len()
            + self.suffix_imports.len()
    }

    /// Returns `true` when the topology index contains no recovered evidence.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Builds a stable root over recovered topology indexes.
#[must_use]
pub fn recovered_topology_index_root(index: &RecoveredTopologyIndex) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(WAL_RECOVERED_INDEX_ROOT_DOMAIN);
    hasher.update(b"topology");
    for (strand_id, record) in &index.strand_forks {
        hasher.update(b"strand-fork");
        hasher.update(strand_id.as_bytes());
        hasher.update(&record.to_payload_bytes());
    }
    for (strand_id, record) in &index.strand_drops {
        hasher.update(b"strand-drop");
        hasher.update(strand_id.as_bytes());
        hasher.update(&record.to_payload_bytes());
    }
    for (child_worldline_id, strand_id) in &index.child_worldlines {
        hasher.update(b"child-worldline");
        hasher.update(child_worldline_id.as_bytes());
        hasher.update(strand_id.as_bytes());
    }
    for (braid_id, events) in &index.braid_events {
        hasher.update(b"braid-events");
        hasher.update(braid_id);
        for event in events {
            hasher.update(&event.to_payload_bytes());
        }
    }
    for (shell_digest, record) in &index.braid_shells {
        hasher.update(b"braid-shell");
        hasher.update(shell_digest);
        hasher.update(&record.to_payload_bytes());
    }
    for (import_id, record) in &index.suffix_imports {
        hasher.update(b"suffix-import");
        hasher.update(import_id);
        hasher.update(&record.to_payload_bytes());
    }
    hasher.finalize().into()
}

/// Causal commit evidence source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CausalCommitEvidenceSource {
    /// Evidence comes from Echo's WAL.
    EchoWal,
    /// Evidence comes from a validated Echo checkpoint.
    EchoCheckpoint,
    /// Evidence comes from an Echo recovery certificate.
    EchoRecoveryCertificate,
}

/// Causal commit evidence posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CausalCommitEvidencePosture {
    /// Commit evidence is present.
    Present,
    /// Commit evidence is unavailable.
    Absent,
    /// Commit evidence is obstructed.
    Obstructed,
}

/// Host-neutral causal commit evidence projected for debuggers and operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalCommitEvidence {
    /// Stable evidence id.
    pub evidence_id: Hash,
    /// Evidence posture.
    pub posture: CausalCommitEvidencePosture,
    /// Evidence source.
    pub source: CausalCommitEvidenceSource,
    /// Durability mode.
    pub durability_mode: WalDurabilityMode,
    /// Writer epoch.
    pub writer_epoch: WriterEpochId,
    /// Last LSN for the committed transaction.
    pub lsn: Lsn,
    /// Transaction id.
    pub transaction_id: WalTransactionId,
    /// Commit digest.
    pub commit_digest: Hash,
    /// Checkpoint digest, if one anchors the evidence.
    pub checkpoint_digest: Option<Hash>,
    /// Recovery certificate digest, if one anchors the evidence.
    pub recovery_certificate_digest: Option<Hash>,
    /// Obstruction digest, if obstructed.
    pub obstruction_digest: Option<Hash>,
}

/// Projects host-neutral causal commit evidence from recovered WAL history.
#[must_use]
pub fn project_causal_commit_evidence(report: &RecoveryScanReport) -> Vec<CausalCommitEvidence> {
    report
        .transactions
        .iter()
        .map(|transaction| {
            let evidence_id = causal_commit_evidence_id(&transaction.commit);
            CausalCommitEvidence {
                evidence_id,
                posture: CausalCommitEvidencePosture::Present,
                source: CausalCommitEvidenceSource::EchoWal,
                durability_mode: transaction.commit.durability_mode,
                writer_epoch: transaction.commit.writer_epoch,
                lsn: transaction.commit.last_lsn,
                transaction_id: transaction.commit.transaction_id,
                commit_digest: transaction.commit.commit_digest,
                checkpoint_digest: None,
                recovery_certificate_digest: None,
                obstruction_digest: None,
            }
        })
        .collect()
}

/// Projects explicit absent causal commit evidence.
#[must_use]
pub fn project_absent_causal_commit_evidence(
    evidence_id: Hash,
    durability_mode: WalDurabilityMode,
) -> CausalCommitEvidence {
    CausalCommitEvidence {
        evidence_id,
        posture: CausalCommitEvidencePosture::Absent,
        source: CausalCommitEvidenceSource::EchoWal,
        durability_mode,
        writer_epoch: WriterEpochId::from_hash([0; 32]),
        lsn: Lsn::from_raw(0),
        transaction_id: WalTransactionId::from_hash([0; 32]),
        commit_digest: [0; 32],
        checkpoint_digest: None,
        recovery_certificate_digest: None,
        obstruction_digest: None,
    }
}

/// Projects obstructed causal commit evidence for a known commit anchor.
#[must_use]
pub fn project_obstructed_causal_commit_evidence(
    commit: &WalTransactionCommit,
    obstruction_digest: Hash,
) -> CausalCommitEvidence {
    CausalCommitEvidence {
        evidence_id: causal_commit_evidence_id(commit),
        posture: CausalCommitEvidencePosture::Obstructed,
        source: CausalCommitEvidenceSource::EchoWal,
        durability_mode: commit.durability_mode,
        writer_epoch: commit.writer_epoch,
        lsn: commit.last_lsn,
        transaction_id: commit.transaction_id,
        commit_digest: commit.commit_digest,
        checkpoint_digest: None,
        recovery_certificate_digest: None,
        obstruction_digest: Some(obstruction_digest),
    }
}

/// Shadow replay mismatch classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowReplayMismatch {
    /// Applied transaction count differs.
    AppliedTransactionCount {
        /// Live applied transaction count.
        live: usize,
        /// Replayed applied transaction count.
        replayed: usize,
    },
    /// Applied transaction id differs at the first mismatching index.
    AppliedTransaction {
        /// First mismatching index.
        index: usize,
        /// Live transaction id.
        live: WalTransactionId,
        /// Replayed transaction id.
        replayed: WalTransactionId,
    },
    /// Live state is missing a frontier present in replay.
    MissingLiveFrontier {
        /// Frontier kind.
        kind: AffectedFrontierKind,
    },
    /// Replay state is missing a frontier present in live state.
    MissingReplayedFrontier {
        /// Frontier kind.
        kind: AffectedFrontierKind,
    },
    /// Frontier digest differs.
    FrontierDigest {
        /// Frontier kind.
        kind: AffectedFrontierKind,
        /// Live frontier digest.
        live: Hash,
        /// Replayed frontier digest.
        replayed: Hash,
    },
}

/// Shadow replay comparison report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShadowReplayReport {
    /// `true` when live and replayed state match exactly.
    pub matches: bool,
    /// First mismatch, when present.
    pub first_mismatch: Option<ShadowReplayMismatch>,
    /// Replayed state produced from committed transactions.
    pub replayed_state: RecoveredState,
}

/// Checks that live state and WAL replay produce the same recovered state.
pub fn shadow_replay_matches(
    live_state: &RecoveredState,
    transactions: &[WalCommittedTransaction],
) -> Result<bool, WalReplayError> {
    Ok(shadow_replay_report(live_state, transactions)?.matches)
}

/// Reports whether live state and WAL replay produce the same recovered state.
pub fn shadow_replay_report(
    live_state: &RecoveredState,
    transactions: &[WalCommittedTransaction],
) -> Result<ShadowReplayReport, WalReplayError> {
    let mut replayed = RecoveredState::default();
    for transaction in transactions {
        replayed = apply_committed_transaction(replayed, transaction)?;
    }
    let first_mismatch = first_shadow_replay_mismatch(live_state, &replayed);
    Ok(ShadowReplayReport {
        matches: first_mismatch.is_none(),
        first_mismatch,
        replayed_state: replayed,
    })
}

fn first_shadow_replay_mismatch(
    live_state: &RecoveredState,
    replayed_state: &RecoveredState,
) -> Option<ShadowReplayMismatch> {
    if live_state.applied_transactions.len() != replayed_state.applied_transactions.len() {
        return Some(ShadowReplayMismatch::AppliedTransactionCount {
            live: live_state.applied_transactions.len(),
            replayed: replayed_state.applied_transactions.len(),
        });
    }
    for (index, (live, replayed)) in live_state
        .applied_transactions
        .iter()
        .zip(&replayed_state.applied_transactions)
        .enumerate()
    {
        if live != replayed {
            return Some(ShadowReplayMismatch::AppliedTransaction {
                index,
                live: *live,
                replayed: *replayed,
            });
        }
    }
    let frontier_kinds = live_state
        .frontiers
        .keys()
        .chain(replayed_state.frontiers.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    for kind in frontier_kinds {
        match (
            live_state.frontiers.get(&kind),
            replayed_state.frontiers.get(&kind),
        ) {
            (Some(live), Some(replayed)) if live != replayed => {
                return Some(ShadowReplayMismatch::FrontierDigest {
                    kind,
                    live: *live,
                    replayed: *replayed,
                });
            }
            (None, Some(_)) => return Some(ShadowReplayMismatch::MissingLiveFrontier { kind }),
            (Some(_), None) => return Some(ShadowReplayMismatch::MissingReplayedFrontier { kind }),
            _ => {}
        }
    }
    None
}

/// WAL release readiness audit result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalReleaseReadinessReport {
    /// `true` when all release-readiness gates are satisfied.
    pub ready: bool,
    /// Gate names that passed.
    pub passed_gates: Vec<&'static str>,
    /// Gate names that remain blocked.
    pub blocked_gates: Vec<&'static str>,
}

/// Audits WAL release readiness from known gate booleans.
#[must_use]
pub fn audit_wal_release_readiness(gates: WalReleaseReadinessGates) -> WalReleaseReadinessReport {
    let mut passed_gates = Vec::new();
    let mut blocked_gates = Vec::new();
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "filesystem_adapter",
        gates.filesystem_adapter,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "object_store_capability_gate",
        gates.object_store_capability_gate,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "segment_repair",
        gates.segment_repair,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "segment_layout_policy",
        gates.segment_layout_policy,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "segment_manifest_validation",
        gates.segment_manifest_validation,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "crash_matrix",
        gates.crash_matrix,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "crashpoint_manifest",
        gates.crashpoint_manifest,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "shadow_replay",
        gates.shadow_replay,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "outbox",
        gates.outbox,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "commit_evidence",
        gates.commit_evidence,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "wal_doctor",
        gates.wal_doctor,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "semantic_validator",
        gates.semantic_validator,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "topology_recovery",
        gates.topology_recovery,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "filesystem_sync_evidence",
        gates.filesystem_sync_evidence,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "object_store_manifest_negatives",
        gates.object_store_manifest_negatives,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "security_redaction",
        gates.security_redaction,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "app_noun_guard",
        gates.app_noun_guard,
    );
    push_gate(
        &mut passed_gates,
        &mut blocked_gates,
        "external_consumer_gate",
        gates.external_consumer_gate,
    );
    let ready = blocked_gates.is_empty();
    WalReleaseReadinessReport {
        ready,
        passed_gates,
        blocked_gates,
    }
}

/// WAL release readiness gate input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalReleaseReadinessGates {
    /// Filesystem WAL adapter gate.
    pub filesystem_adapter: bool,
    /// Object-store capability gate.
    pub object_store_capability_gate: bool,
    /// Segment repair/truncation gate.
    pub segment_repair: bool,
    /// Segment layout policy gate.
    pub segment_layout_policy: bool,
    /// Segment manifest validation gate.
    pub segment_manifest_validation: bool,
    /// Crash matrix gate.
    pub crash_matrix: bool,
    /// Crashpoint manifest gate.
    pub crashpoint_manifest: bool,
    /// Shadow replay gate.
    pub shadow_replay: bool,
    /// Side-effect outbox gate.
    pub outbox: bool,
    /// Causal commit evidence projection gate.
    pub commit_evidence: bool,
    /// WAL doctor/inspector gate.
    pub wal_doctor: bool,
    /// Semantic validator gate.
    pub semantic_validator: bool,
    /// Topology recovery index gate.
    pub topology_recovery: bool,
    /// Strict filesystem sync evidence gate.
    pub filesystem_sync_evidence: bool,
    /// Object-store manifest negative matrix gate.
    pub object_store_manifest_negatives: bool,
    /// Security and redaction posture gate.
    pub security_redaction: bool,
    /// No-app-nouns guard gate.
    pub app_noun_guard: bool,
    /// External consumer recovery gate.
    pub external_consumer_gate: bool,
}

enum DiskWalRecord<'a> {
    Frame(&'a WalFrame),
    Commit(&'a WalTransactionCommit),
}

fn append_segment_record(
    path: &Path,
    record: DiskWalRecord<'_>,
    sync: bool,
) -> Result<(), WalStoreError> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let (kind, payload) = match record {
        DiskWalRecord::Frame(frame) => (1u8, encode_frame(frame)),
        DiskWalRecord::Commit(commit) => (2u8, encode_commit(commit)),
    };
    let digest = disk_record_digest(kind, &payload);
    file.write_all(WAL_SEGMENT_RECORD_MAGIC)?;
    file.write_all(&[kind])?;
    file.write_all(&len_u64(payload.len()).to_le_bytes())?;
    file.write_all(&payload)?;
    file.write_all(&digest)?;
    if sync {
        file.sync_all()?;
    }
    Ok(())
}

fn read_filesystem_segments(
    root: &Path,
) -> Result<(Vec<WalFrame>, Vec<WalTransactionCommit>, bool), WalStoreError> {
    if !root.exists() {
        return Ok((Vec::new(), Vec::new(), false));
    }
    let mut all_frames = Vec::new();
    let mut all_commits = Vec::new();
    let mut any_torn_tail = false;
    for path in segment_paths(root)? {
        let (frames, commits, torn_tail) = read_segment_file(&path)?;
        all_frames.extend(frames);
        all_commits.extend(commits);
        any_torn_tail |= torn_tail;
    }
    all_frames.sort_by_key(|frame| frame.header.lsn);
    all_commits.sort_by_key(|commit| commit.last_lsn);
    Ok((all_frames, all_commits, any_torn_tail))
}

fn read_segment_file(
    path: &Path,
) -> Result<(Vec<WalFrame>, Vec<WalTransactionCommit>, bool), WalStoreError> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    read_segment_bytes(&bytes)
}

fn read_segment_bytes(
    bytes: &[u8],
) -> Result<(Vec<WalFrame>, Vec<WalTransactionCommit>, bool), WalStoreError> {
    let mut offset = 0usize;
    let mut frames = Vec::new();
    let mut commits = Vec::new();
    let mut torn_tail = false;
    while offset < bytes.len() {
        let header_len = WAL_SEGMENT_RECORD_MAGIC.len() + 1 + 8;
        let Some(header_end) = offset.checked_add(header_len) else {
            torn_tail = true;
            break;
        };
        if header_end > bytes.len() {
            torn_tail = true;
            break;
        }
        if bytes.get(offset..offset + WAL_SEGMENT_RECORD_MAGIC.len())
            != Some(WAL_SEGMENT_RECORD_MAGIC.as_slice())
        {
            return Err(WalStoreError::SegmentRecordDigestMismatch);
        }
        offset += WAL_SEGMENT_RECORD_MAGIC.len();
        let kind = bytes[offset];
        offset += 1;
        let mut len = [0; 8];
        len.copy_from_slice(&bytes[offset..offset + 8]);
        offset += 8;
        let payload_len = match usize::try_from(u64::from_le_bytes(len)) {
            Ok(value) => value,
            Err(_) => return Err(WalStoreError::Decode(WalDecodeError::UnexpectedEof)),
        };
        let Some(payload_end) = offset.checked_add(payload_len) else {
            torn_tail = true;
            break;
        };
        let Some(digest_end) = payload_end.checked_add(32) else {
            torn_tail = true;
            break;
        };
        if digest_end > bytes.len() {
            torn_tail = true;
            break;
        }
        let payload = &bytes[offset..payload_end];
        let digest = &bytes[payload_end..digest_end];
        if digest != disk_record_digest(kind, payload) {
            return Err(WalStoreError::SegmentRecordDigestMismatch);
        }
        match kind {
            1 => frames.push(decode_frame(payload)?),
            2 => commits.push(decode_commit(payload)?),
            other => return Err(WalStoreError::UnknownDiskRecordKind(other)),
        }
        offset = digest_end;
    }
    Ok((frames, commits, torn_tail))
}

fn ensure_segment_has_no_uncommitted_tail(
    root: &Path,
    segment_id: WalSegmentId,
) -> Result<(), WalStoreError> {
    let (frames, commits, torn_tail) = read_segment_file(&segment_path(root, segment_id))?;
    if torn_tail {
        return Err(WalStoreError::SegmentHasUncommittedTail(segment_id));
    }
    let last_committed_lsn = commits.iter().map(|commit| commit.last_lsn).max();
    if frames
        .iter()
        .any(|frame| last_committed_lsn.is_none_or(|lsn| frame.header.lsn > lsn))
    {
        return Err(WalStoreError::SegmentHasUncommittedTail(segment_id));
    }
    Ok(())
}

fn rewrite_filesystem_segments_after_truncation(
    root: &Path,
    after_lsn: Lsn,
) -> Result<(), WalStoreError> {
    let (frames, commits, _) = read_filesystem_segments(root)?;
    let kept_frames = frames
        .into_iter()
        .filter(|frame| frame.header.lsn <= after_lsn)
        .collect::<Vec<_>>();
    let kept_commits = commits
        .into_iter()
        .filter(|commit| commit.last_lsn <= after_lsn)
        .collect::<Vec<_>>();
    rewrite_segment_records(root, &kept_frames, &kept_commits)
}

fn clear_filesystem_segments(root: &Path) -> Result<(), WalStoreError> {
    rewrite_segment_records(root, &[], &[])
}

fn rewrite_segment_records(
    root: &Path,
    frames: &[WalFrame],
    commits: &[WalTransactionCommit],
) -> Result<(), WalStoreError> {
    fs::create_dir_all(root)?;
    fs::create_dir_all(segments_dir(root))?;
    for path in segment_paths(root)? {
        fs::remove_file(path)?;
    }
    let path = segment_path(root, WalSegmentId::from_raw(1));
    File::create(&path)?.sync_all()?;
    for frame in frames {
        append_segment_record(&path, DiskWalRecord::Frame(frame), false)?;
    }
    for commit in commits {
        append_segment_record(&path, DiskWalRecord::Commit(commit), false)?;
    }
    File::options().append(true).open(&path)?.sync_all()?;
    sync_directory_store(root)?;
    Ok(())
}

fn segment_paths(root: &Path) -> Result<Vec<PathBuf>, WalStoreError> {
    let mut paths = Vec::new();
    for scan_root in segment_scan_roots(root) {
        if !scan_root.exists() {
            continue;
        }
        for entry in fs::read_dir(scan_root)? {
            let entry = entry?;
            let path = entry.path();
            if path
                .file_stem()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("segment-"))
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("ecwal"))
            {
                let segment_id = parse_segment_id(&path)?;
                paths.push((segment_id, path));
            }
        }
    }
    paths.sort_by_key(|(segment_id, path)| (*segment_id, path.clone()));
    if let Some((first_segment_id, _)) = paths.first() {
        let first_expected = WalSegmentId::from_raw(1);
        if *first_segment_id != first_expected {
            return Err(WalStoreError::SegmentGap {
                expected: first_expected,
                actual: *first_segment_id,
            });
        }
    }
    let mut previous_segment_id: Option<WalSegmentId> = None;
    for (segment_id, _) in &paths {
        if let Some(previous) = previous_segment_id {
            let expected = WalSegmentId::from_raw(previous.as_u64() + 1);
            if *segment_id == previous {
                return Err(WalStoreError::DuplicateSegment(*segment_id));
            }
            if *segment_id != expected {
                return Err(WalStoreError::SegmentGap {
                    expected,
                    actual: *segment_id,
                });
            }
        }
        previous_segment_id = Some(*segment_id);
    }
    Ok(paths.into_iter().map(|(_, path)| path).collect())
}

/// Returns the canonical segment path relative to the WAL root.
#[must_use]
pub fn canonical_segment_relative_path(segment_id: WalSegmentId) -> PathBuf {
    PathBuf::from(WAL_SEGMENTS_DIR).join(segment_file_name(segment_id))
}

/// Returns the canonical segment path for a WAL root.
#[must_use]
pub fn canonical_segment_path(root: impl AsRef<Path>, segment_id: WalSegmentId) -> PathBuf {
    root.as_ref()
        .join(canonical_segment_relative_path(segment_id))
}

fn segment_path(root: &Path, segment_id: WalSegmentId) -> PathBuf {
    canonical_segment_path(root, segment_id)
}

fn segments_dir(root: &Path) -> PathBuf {
    root.join(WAL_SEGMENTS_DIR)
}

fn segment_scan_roots(root: &Path) -> Vec<PathBuf> {
    let canonical = segments_dir(root);
    if canonical.exists() {
        vec![canonical, root.to_path_buf()]
    } else {
        vec![root.to_path_buf()]
    }
}

fn segment_file_name(segment_id: WalSegmentId) -> String {
    format!("segment-{:020}.ecwal", segment_id.as_u64())
}

fn parse_segment_id(path: &Path) -> Result<WalSegmentId, WalStoreError> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| WalStoreError::InvalidSegmentFileName(path.display().to_string()))?;
    let Some(after_prefix) = name.strip_prefix("segment-") else {
        return Err(WalStoreError::InvalidSegmentFileName(name.to_string()));
    };
    let digits = after_prefix
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return Err(WalStoreError::InvalidSegmentFileName(name.to_string()));
    }
    let raw = digits
        .parse::<u64>()
        .map_err(|_| WalStoreError::InvalidSegmentFileName(name.to_string()))?;
    Ok(WalSegmentId::from_raw(raw))
}

fn encode_manifest(manifest: &WalManifest) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_hash(&mut bytes, &manifest.manifest_digest);
    push_optional_lsn(&mut bytes, manifest.last_committed_lsn);
    push_optional_hash(&mut bytes, manifest.last_commit_digest);
    bytes.extend_from_slice(&manifest.sealed_segment_count.to_le_bytes());
    bytes
}

fn decode_manifest(bytes: &[u8]) -> Result<WalManifest, WalDecodeError> {
    let mut cursor = WalPayloadCursor::new(bytes);
    let manifest_digest = cursor.read_hash()?;
    let last_committed_lsn = cursor.read_optional_lsn()?;
    let last_commit_digest = cursor.read_optional_hash()?;
    let sealed_segment_count = cursor.read_u64()?;
    cursor.finish()?;
    Ok(WalManifest {
        manifest_digest,
        last_committed_lsn,
        last_commit_digest,
        sealed_segment_count,
    })
}

fn read_manifest_file(path: &Path) -> Result<WalManifest, WalStoreError> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    decode_manifest(&bytes).map_err(WalStoreError::Decode)
}

fn write_manifest_atomic(root: &Path, manifest: &WalManifest) -> Result<(), WalStoreError> {
    let path = root.join("manifest.ecwal");
    let temp = root.join(".manifest.ecwal.tmp");
    let bytes = encode_manifest(manifest);
    {
        let mut file = File::create(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    fs::rename(temp, path)?;
    sync_directory_store(root)
}

fn sync_directory_store(path: &Path) -> Result<(), WalStoreError> {
    File::open(path)?.sync_all()?;
    Ok(())
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
    validate_recovery_frame_order(frames)?;
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

fn validate_recovery_frame_order(frames: &[WalFrame]) -> Result<(), WalValidationError> {
    let mut sorted = frames.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|frame| frame.header.lsn);
    let mut previous_lsn: Option<Lsn> = None;
    for frame in sorted {
        frame.validate_integrity()?;
        if let Some(previous) = previous_lsn {
            let expected = previous
                .checked_next()
                .ok_or(WalValidationError::LsnContinuityMismatch)?;
            if frame.header.lsn != expected {
                return Err(WalValidationError::LsnContinuityMismatch);
            }
        }
        previous_lsn = Some(frame.header.lsn);
    }
    Ok(())
}

/// Builds a submission acceptance transaction.
pub fn build_submission_acceptance_transaction(
    mut builder: WalTransactionBuilder,
    record: SubmissionAcceptanceRecord,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, WalBuildError> {
    builder.push_record(
        WalRecordKind::SubmissionAcceptedRecorded,
        record.to_payload_bytes(),
    )?;
    builder.push_record(
        WalRecordKind::SubmissionAcceptanceEvidenceRecorded,
        record.acceptance_evidence_digest.to_vec(),
    )?;
    builder.commit(affected_frontiers)
}

/// Builds a scheduler-owned tick transaction.
pub fn build_tick_transaction(
    mut builder: WalTransactionBuilder,
    receipt: TickReceiptRecord,
    correlation: WalReceiptCorrelationRecord,
    state_delta_digest: Hash,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, WalBuildError> {
    builder.push_record(
        WalRecordKind::TickReceiptRecorded,
        receipt.to_payload_bytes(),
    )?;
    builder.push_record(
        WalRecordKind::ReceiptCorrelationRecorded,
        correlation.to_payload_bytes(),
    )?;
    builder.push_record(
        WalRecordKind::RuntimeStateDeltaRecorded,
        state_delta_digest.to_vec(),
    )?;
    builder.commit(affected_frontiers)
}

/// Builds a retained reading transaction.
pub fn build_retained_reading_transaction(
    mut builder: WalTransactionBuilder,
    material: &[RetainedMaterialRecord],
    reading: ReadingRefRecord,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, WalBuildError> {
    for record in material {
        builder.push_record(
            WalRecordKind::RetainedMaterialRefRecorded,
            record.to_payload_bytes(),
        )?;
    }
    builder.push_record(
        WalRecordKind::ReadingEnvelopeRetained,
        reading.to_payload_bytes(),
    )?;
    builder.commit(affected_frontiers)
}

/// Builds a topology-intent transaction from typed topology evidence records.
pub fn build_topology_intent_transaction(
    mut builder: WalTransactionBuilder,
    records: &[TopologyIntentRecord],
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, WalBuildError> {
    for record in records {
        builder.push_record(record.record_kind(), record.to_payload_bytes())?;
    }
    builder.commit(affected_frontiers)
}

/// Builds a checkpoint publication transaction.
pub fn build_checkpoint_publication_transaction(
    mut builder: WalTransactionBuilder,
    publication: CheckpointPublicationRecord,
    affected_frontiers: Vec<AffectedFrontier>,
) -> Result<WalCommittedTransaction, WalBuildError> {
    builder.push_record(
        WalRecordKind::CheckpointPublicationRecorded,
        publication.to_payload_bytes(),
    )?;
    builder.commit(affected_frontiers)
}

/// Writes a checkpoint file through an atomic temp-file + rename protocol.
///
/// This is a local filesystem helper for checkpoint material, not causal
/// history authority. A checkpoint remains a replay accelerator and must still
/// validate against committed WAL history before recovery uses it.
pub fn write_checkpoint_record_atomic(
    path: impl AsRef<Path>,
    checkpoint: &CheckpointRecord,
) -> Result<(), WalCheckpointIoError> {
    write_checkpoint_record_atomic_with_evidence(path, checkpoint).map(|_| ())
}

/// Writes a checkpoint file and returns strict filesystem sync evidence.
pub fn write_checkpoint_record_atomic_with_evidence(
    path: impl AsRef<Path>,
    checkpoint: &CheckpointRecord,
) -> Result<Vec<FilesystemSyncEvidence>, WalCheckpointIoError> {
    let path = path.as_ref();
    let parent = path
        .parent()
        .ok_or(WalCheckpointIoError::MissingParentDirectory)?;
    fs::create_dir_all(parent)?;
    let temp_path = checkpoint_temp_path(path)?;
    let bytes = checkpoint_file_bytes(checkpoint);
    {
        let mut file = File::create(&temp_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    fs::rename(&temp_path, path)?;
    sync_directory(parent)?;
    Ok(vec![
        FilesystemSyncEvidence::unscoped(FilesystemSyncBoundary::CheckpointTempFileSynced),
        FilesystemSyncEvidence::unscoped(FilesystemSyncBoundary::CheckpointRenamedDirectorySynced),
    ])
}

/// Reads a checkpoint file written by [`write_checkpoint_record_atomic`].
pub fn read_checkpoint_record(
    path: impl AsRef<Path>,
) -> Result<CheckpointRecord, WalCheckpointIoError> {
    let mut bytes = Vec::new();
    File::open(path.as_ref())?.read_to_end(&mut bytes)?;
    parse_checkpoint_file_bytes(&bytes)
}

/// Recovers submission posture from committed WAL transactions.
pub fn recover_submission_index(
    report: &RecoveryScanReport,
) -> Result<RecoveredSubmissionIndex, WalRecoveryIndexError> {
    let mut index = RecoveredSubmissionIndex::default();
    for transaction in &report.transactions {
        for frame in &transaction.frames {
            match frame.header.record_kind {
                WalRecordKind::SubmissionAcceptedRecorded => {
                    let record = SubmissionAcceptanceRecord::from_payload_bytes(
                        &frame.payload.canonical_bytes,
                    )?;
                    index.insert_acceptance_record(record)?;
                }
                WalRecordKind::TickReceiptRecorded => {
                    let receipt =
                        TickReceiptRecord::from_payload_bytes(&frame.payload.canonical_bytes)?;
                    index.apply_tick_receipt_record(receipt);
                }
                _ => {}
            }
        }
    }
    Ok(index)
}

/// Recovers receipt correlations from committed WAL transactions.
pub fn recover_receipt_index(
    report: &RecoveryScanReport,
) -> Result<RecoveredReceiptIndex, WalRecoveryIndexError> {
    let mut index = RecoveredReceiptIndex::default();
    for transaction in &report.transactions {
        for frame in &transaction.frames {
            match frame.header.record_kind {
                WalRecordKind::TickReceiptRecorded => {
                    let receipt =
                        TickReceiptRecord::from_payload_bytes(&frame.payload.canonical_bytes)?;
                    index
                        .receipt_by_submission
                        .insert(receipt.submission_id, receipt.receipt_digest);
                    index
                        .receipt_by_ticket
                        .insert(receipt.ticket_digest, receipt.receipt_digest);
                    index
                        .ticket_by_submission
                        .insert(receipt.submission_id, receipt.ticket_digest);
                    index
                        .decisions_by_receipt
                        .insert(receipt.receipt_digest, receipt.decision);
                }
                WalRecordKind::ReceiptCorrelationRecorded => {
                    let correlation = WalReceiptCorrelationRecord::from_payload_bytes(
                        &frame.payload.canonical_bytes,
                    )?;
                    index
                        .receipt_by_submission
                        .insert(correlation.submission_id, correlation.receipt_digest);
                    index
                        .receipt_by_ticket
                        .insert(correlation.ticket_digest, correlation.receipt_digest);
                    index
                        .ticket_by_submission
                        .insert(correlation.submission_id, correlation.ticket_digest);
                }
                _ => {}
            }
        }
    }
    Ok(index)
}

/// Recovers retained material and reading indexes from committed WAL transactions.
pub fn recover_retention_index(
    report: &RecoveryScanReport,
) -> Result<RecoveredRetentionIndex, WalRecoveryIndexError> {
    let mut index = RecoveredRetentionIndex::default();
    for transaction in &report.transactions {
        for frame in &transaction.frames {
            match frame.header.record_kind {
                WalRecordKind::RetainedMaterialRefRecorded => {
                    let record =
                        RetainedMaterialRecord::from_payload_bytes(&frame.payload.canonical_bytes)?;
                    index
                        .material_by_semantic_coordinate
                        .entry(record.semantic_coordinate_digest)
                        .or_default()
                        .insert(record.material_digest);
                    index
                        .material_by_digest
                        .insert(record.material_digest, record);
                }
                WalRecordKind::ReadingEnvelopeRetained => {
                    let record =
                        ReadingRefRecord::from_payload_bytes(&frame.payload.canonical_bytes)?;
                    index
                        .readings_by_semantic_coordinate
                        .entry(record.semantic_coordinate_digest)
                        .or_default()
                        .insert(record.reading_id);
                    index.reading_by_id.insert(record.reading_id, record);
                }
                _ => {}
            }
        }
    }
    Ok(index)
}

/// Recovers topology indexes from committed WAL transactions.
pub fn recover_topology_index(
    report: &RecoveryScanReport,
) -> Result<RecoveredTopologyIndex, WalRecoveryIndexError> {
    let mut records = Vec::new();
    for transaction in &report.transactions {
        for frame in &transaction.frames {
            match frame.header.record_kind {
                WalRecordKind::TopologyStrandForkRecorded => {
                    records.push(TopologyIntentRecord::StrandFork(
                        StrandForkRecord::from_payload_bytes(&frame.payload.canonical_bytes)?,
                    ));
                }
                WalRecordKind::TopologyStrandDropRecorded => {
                    records.push(TopologyIntentRecord::StrandDrop(
                        StrandDropRecord::from_payload_bytes(&frame.payload.canonical_bytes)?,
                    ));
                }
                WalRecordKind::TopologyBraidEventRecorded => {
                    records.push(TopologyIntentRecord::BraidEvent(
                        TopologyBraidEventRecord::from_payload_bytes(
                            &frame.payload.canonical_bytes,
                        )?,
                    ));
                }
                WalRecordKind::TopologyBraidShellRetained => {
                    records.push(TopologyIntentRecord::BraidShell(
                        BraidShellRetentionRecord::from_payload_bytes(
                            &frame.payload.canonical_bytes,
                        )?,
                    ));
                }
                WalRecordKind::TopologySuffixImportRecorded => {
                    records.push(TopologyIntentRecord::SuffixImport(
                        SuffixImportRecord::from_payload_bytes(&frame.payload.canonical_bytes)?,
                    ));
                }
                _ => {}
            }
        }
    }
    RecoveredTopologyIndex::from_topology_records(records)
}

/// Validates retained material references against an available material set.
#[must_use]
pub fn retained_material_obstructions(
    index: &RecoveredRetentionIndex,
    available_material: &BTreeSet<Hash>,
) -> Vec<RetainedMaterialObstruction> {
    let mut obstructions = Vec::new();
    for record in index.material_by_digest.values() {
        if record.posture != EvidenceMaterialPosture::Present
            || !available_material.contains(&record.material_digest)
        {
            let posture = if record.posture == EvidenceMaterialPosture::Present {
                EvidenceMaterialPosture::Missing
            } else {
                record.posture
            };
            obstructions.push(RetainedMaterialObstruction {
                material_digest: record.material_digest,
                kind: record.kind,
                scope: missing_material_scope(record.kind),
                posture,
            });
        }
    }
    obstructions
}

/// Recovers checkpoint publication records from committed WAL transactions.
pub fn recover_checkpoint_publications(
    report: &RecoveryScanReport,
) -> Result<Vec<CheckpointPublicationRecord>, WalRecoveryIndexError> {
    let mut publications = Vec::new();
    for transaction in &report.transactions {
        for frame in &transaction.frames {
            if frame.header.record_kind == WalRecordKind::CheckpointPublicationRecorded {
                publications.push(CheckpointPublicationRecord::from_payload_bytes(
                    &frame.payload.canonical_bytes,
                )?);
            }
        }
    }
    Ok(publications)
}

/// Evaluates whether a checkpoint may be used as a replay accelerator.
#[must_use]
pub fn validate_checkpoint_record(
    checkpoint: &CheckpointRecord,
    report: &RecoveryScanReport,
    publications: &[CheckpointPublicationRecord],
) -> CheckpointValidationPosture {
    let Some(last_lsn) = report.last_committed_lsn() else {
        return CheckpointValidationPosture::Invalid;
    };
    let Some(last_commit_digest) = report.last_commit_digest() else {
        return CheckpointValidationPosture::Invalid;
    };
    if checkpoint.last_included_lsn > last_lsn
        || checkpoint.last_included_commit_digest != last_commit_digest
    {
        return CheckpointValidationPosture::Invalid;
    }
    let checkpoint_digest = checkpoint.checkpoint_digest();
    if publications.iter().any(|publication| {
        publication.checkpoint_id == checkpoint.checkpoint_id
            && publication.checkpoint_digest == checkpoint_digest
    }) {
        CheckpointValidationPosture::PublishedAndUsable
    } else {
        CheckpointValidationPosture::UsableWithoutPublicationRecord
    }
}

/// Evaluates checkpoint publication evidence when checkpoint material may be absent.
#[must_use]
pub fn evaluate_checkpoint_publication(
    publication: &CheckpointPublicationRecord,
    checkpoint: Option<&CheckpointRecord>,
    report: &RecoveryScanReport,
) -> CheckpointValidationPosture {
    let Some(checkpoint) = checkpoint else {
        return CheckpointValidationPosture::PublishedCheckpointMaterialMissing;
    };
    validate_checkpoint_record(checkpoint, report, &[*publication])
}

/// Builds a recovery certificate from recovered committed history.
#[must_use]
pub fn build_recovery_certificate(
    report: &RecoveryScanReport,
    checkpoint_used: Option<Hash>,
    obstruction_count: u64,
    recovered_frontier_root: Hash,
    recovered_indexes_root: Hash,
) -> RecoveryCertificate {
    RecoveryCertificate {
        checkpoint_used,
        first_lsn: report.first_committed_lsn(),
        last_lsn: report.last_committed_lsn(),
        committed_transactions_replayed: len_u64(report.transactions.len()),
        tail_posture: report.tail_posture,
        obstruction_count,
        recovered_frontier_root,
        recovered_indexes_root,
    }
}

/// Computes the stable digest for a recovery certificate.
#[must_use]
pub fn recovery_certificate_digest(certificate: &RecoveryCertificate) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(WAL_RECOVERY_CERTIFICATE_DOMAIN);
    update_optional_projection_digest(&mut h, certificate.checkpoint_used);
    update_optional_lsn(&mut h, certificate.first_lsn);
    update_optional_lsn(&mut h, certificate.last_lsn);
    h.update(&certificate.committed_transactions_replayed.to_le_bytes());
    update_recovery_tail_posture(&mut h, certificate.tail_posture);
    h.update(&certificate.obstruction_count.to_le_bytes());
    h.update(&certificate.recovered_frontier_root);
    h.update(&certificate.recovered_indexes_root);
    h.finalize().into()
}

/// Runs a read-only WAL doctor over an in-memory store.
pub fn doctor_in_memory_store(
    store: &InMemoryWalStore,
) -> Result<WalDoctorReport, WalRecoveryError> {
    let frames = store.read_frames();
    let commits = store.read_commits();
    let report =
        match recover_from_frames_and_commits(&frames, &commits, RecoveryAccessMode::ReadOnly) {
            Ok(report) => report,
            Err(_) => return Ok(obstructed_doctor_report()),
        };
    Ok(doctor_report_from_scan(report, 0, [0; 32], [0; 32]))
}

/// Runs a read-only WAL doctor over an in-memory store and available material set.
pub fn doctor_in_memory_store_with_materials(
    store: &InMemoryWalStore,
    available_material: &BTreeSet<Hash>,
) -> Result<WalDoctorReport, WalRecoveryError> {
    let frames = store.read_frames();
    let commits = store.read_commits();
    let report =
        match recover_from_frames_and_commits(&frames, &commits, RecoveryAccessMode::ReadOnly) {
            Ok(report) => report,
            Err(_) => return Ok(obstructed_doctor_report()),
        };
    let retention = recover_retention_index(&report)?;
    let obstruction_count = retained_material_obstructions(&retention, available_material)
        .into_iter()
        .filter(|obstruction| obstruction.scope != MissingMaterialScope::DiagnosticLoss)
        .count();
    Ok(doctor_report_from_scan(
        report,
        len_u64(obstruction_count),
        [0; 32],
        [0; 32],
    ))
}

fn obstructed_doctor_report() -> WalDoctorReport {
    WalDoctorReport {
        posture: WalDoctorPosture::Obstructed,
        recovery_certificate: RecoveryCertificate {
            checkpoint_used: None,
            first_lsn: None,
            last_lsn: None,
            committed_transactions_replayed: 0,
            tail_posture: RecoveryTailPosture::Clean,
            obstruction_count: 1,
            recovered_frontier_root: [0; 32],
            recovered_indexes_root: [0; 32],
        },
        tail_posture: RecoveryTailPosture::Clean,
    }
}

fn doctor_report_from_scan(
    report: RecoveryScanReport,
    obstruction_count: u64,
    recovered_frontier_root: Hash,
    recovered_indexes_root: Hash,
) -> WalDoctorReport {
    let posture = if obstruction_count > 0 {
        WalDoctorPosture::Obstructed
    } else {
        match report.tail_posture {
            RecoveryTailPosture::Clean => WalDoctorPosture::Recoverable,
            RecoveryTailPosture::WouldTruncateAll | RecoveryTailPosture::WouldTruncateAfter(_) => {
                WalDoctorPosture::RecoverableWithUncommittedTail
            }
            RecoveryTailPosture::TruncatedAll | RecoveryTailPosture::TruncatedAfter(_) => {
                WalDoctorPosture::Obstructed
            }
        }
    };
    let tail_posture = report.tail_posture;
    WalDoctorReport {
        posture,
        recovery_certificate: build_recovery_certificate(
            &report,
            None,
            obstruction_count,
            recovered_frontier_root,
            recovered_indexes_root,
        ),
        tail_posture,
    }
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
    /// Affected frontier kind does not match transaction kind.
    #[error("WAL transaction affected frontier kind mismatch")]
    FrontierTransitionKindMismatch,
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
    /// A new epoch omitted the previous closed epoch.
    #[error("WAL writer epoch chain gap")]
    WriterEpochChainGap,
    /// Previous epoch final commit digest does not match the closed epoch.
    #[error("WAL writer epoch final commit digest mismatch")]
    WriterEpochFinalCommitDigestMismatch,
    /// New epoch reuses or mismatches storage fencing evidence.
    #[error("WAL writer epoch fencing mismatch")]
    WriterEpochFencingMismatch,
    /// New epoch starts at or before the previous closed epoch final LSN.
    #[error("WAL writer epoch LSN regression")]
    WriterEpochLsnRegression,
    /// Validation failed.
    #[error(transparent)]
    Validation(#[from] WalValidationError),
    /// Payload decode failed.
    #[error(transparent)]
    Decode(#[from] WalDecodeError),
    /// Filesystem I/O failed.
    #[error("WAL filesystem I/O failed: {0}")]
    Io(String),
    /// Segment record digest mismatch.
    #[error("WAL segment record digest mismatch")]
    SegmentRecordDigestMismatch,
    /// Segment record kind was not recognized.
    #[error("unknown WAL disk record kind {0}")]
    UnknownDiskRecordKind(u8),
    /// Segment filename could not be parsed.
    #[error("invalid WAL segment filename {0}")]
    InvalidSegmentFileName(String),
    /// Segment ids are not contiguous.
    #[error("WAL segment gap: expected {expected:?}, found {actual:?}")]
    SegmentGap {
        /// Expected segment id.
        expected: WalSegmentId,
        /// Actual segment id.
        actual: WalSegmentId,
    },
    /// Multiple segment files claim the same segment id.
    #[error("duplicate WAL segment id {0:?}")]
    DuplicateSegment(WalSegmentId),
    /// Frame segment id does not match the active segment.
    #[error("WAL segment mismatch: expected {expected:?}, found {actual:?}")]
    SegmentMismatch {
        /// Expected active segment id.
        expected: WalSegmentId,
        /// Actual frame segment id.
        actual: WalSegmentId,
    },
    /// Segment rotation would overflow logical segment identity.
    #[error("WAL segment id overflow")]
    SegmentIdOverflow,
    /// Segment contains an uncommitted tail and cannot be sealed.
    #[error("WAL segment {0:?} has an uncommitted tail")]
    SegmentHasUncommittedTail(WalSegmentId),
    /// Published filesystem manifest is missing.
    #[error("filesystem WAL manifest is missing")]
    MissingManifest,
    /// Published filesystem manifest segment count does not match segments.
    #[error("filesystem WAL manifest segment count mismatch: expected {expected}, found {actual}")]
    ManifestSegmentCountMismatch {
        /// Expected scanned segment count.
        expected: u64,
        /// Actual manifest segment count.
        actual: u64,
    },
    /// Published filesystem manifest last committed LSN does not match segments.
    #[error("filesystem WAL manifest last committed LSN mismatch")]
    ManifestLastCommittedLsnMismatch {
        /// Expected scanned last committed LSN.
        expected: Option<Lsn>,
        /// Actual manifest last committed LSN.
        actual: Option<Lsn>,
    },
    /// Published filesystem manifest last commit digest does not match segments.
    #[error("filesystem WAL manifest last commit digest mismatch")]
    ManifestLastCommitDigestMismatch {
        /// Expected scanned last commit digest.
        expected: Option<Hash>,
        /// Actual manifest last commit digest.
        actual: Option<Hash>,
    },
    /// Manifest cannot validate while segments contain an uncommitted tail.
    #[error("filesystem WAL manifest cannot validate an uncommitted tail")]
    ManifestCannotValidateUncommittedTail,
}

impl From<std::io::Error> for WalStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
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
    /// Recovered index construction failed.
    #[error(transparent)]
    Index(#[from] WalRecoveryIndexError),
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

/// WAL payload decode errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WalDecodeError {
    /// Payload ended before the expected field could be decoded.
    #[error("WAL payload ended early")]
    UnexpectedEof,
    /// Payload contained trailing bytes.
    #[error("WAL payload contained trailing bytes")]
    TrailingBytes,
    /// Payload contained an unknown enum code.
    #[error("unknown WAL enum code {code} for {enum_name}")]
    UnknownEnumCode {
        /// Enum name.
        enum_name: &'static str,
        /// Unknown code.
        code: u8,
    },
    /// Encoded frame failed embedded integrity validation.
    #[error("encoded WAL frame failed embedded integrity validation")]
    InvalidEmbeddedFrame,
}

/// WAL recovered index errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WalRecoveryIndexError {
    /// Payload decode failed.
    #[error(transparent)]
    Decode(#[from] WalDecodeError),
    /// Submission id was reused with a different canonical envelope digest.
    #[error("submission id was reused with a different canonical envelope digest")]
    SubmissionEnvelopeConflict {
        /// Conflicting submission id.
        submission_id: Hash,
    },
    /// Strand fork evidence conflicted for one strand id.
    #[error("strand fork evidence conflicted for strand {strand_id:?}")]
    ConflictingStrandFork {
        /// Conflicting strand id.
        strand_id: StrandId,
    },
    /// Strand drop evidence conflicted for one strand id.
    #[error("strand drop evidence conflicted for strand {strand_id:?}")]
    ConflictingStrandDrop {
        /// Conflicting strand id.
        strand_id: StrandId,
    },
    /// Child-worldline ancestry conflicted.
    #[error("child worldline evidence conflicted for {child_worldline_id:?}")]
    ConflictingChildWorldline {
        /// Conflicting child worldline id.
        child_worldline_id: WorldlineId,
    },
    /// Braid event evidence conflicted at one event index.
    #[error("braid event evidence conflicted for braid {braid_id:?} at event {event_index}")]
    ConflictingBraidEvent {
        /// Conflicting braid id.
        braid_id: Hash,
        /// Conflicting event index.
        event_index: u64,
    },
    /// Braid shell retention evidence conflicted for one digest.
    #[error("braid shell retention evidence conflicted for shell {shell_digest:?}")]
    ConflictingBraidShell {
        /// Conflicting shell digest.
        shell_digest: Hash,
    },
    /// Suffix import evidence conflicted for one import id.
    #[error("suffix import evidence conflicted for import {import_id:?}")]
    ConflictingSuffixImport {
        /// Conflicting import id.
        import_id: Hash,
    },
    /// Suffix import idempotency key mapped to conflicting imports.
    #[error("suffix import idempotency key conflicted for {idempotency_key_digest:?}")]
    ConflictingSuffixImportIdempotency {
        /// Conflicting idempotency key digest.
        idempotency_key_digest: Hash,
    },
    /// Suffix import bundle digest mapped to conflicting imports.
    #[error("suffix import bundle digest conflicted for {bundle_digest:?}")]
    ConflictingSuffixImportBundle {
        /// Conflicting bundle digest.
        bundle_digest: Hash,
    },
}

/// Checkpoint file I/O errors.
#[derive(Debug, Error)]
pub enum WalCheckpointIoError {
    /// Checkpoint path has no parent directory.
    #[error("checkpoint path has no parent directory")]
    MissingParentDirectory,
    /// Checkpoint file magic is invalid.
    #[error("invalid checkpoint file magic")]
    InvalidMagic,
    /// Checkpoint file digest does not match the checkpoint payload.
    #[error("checkpoint file digest mismatch")]
    DigestMismatch,
    /// Checkpoint payload decode failed.
    #[error(transparent)]
    Decode(#[from] WalDecodeError),
    /// Filesystem I/O failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
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

fn validate_transaction_frontiers(
    frontiers: &[AffectedFrontier],
    transaction_kind: WalTransactionKind,
) -> Result<(), WalValidationError> {
    for frontier in frontiers {
        if !frontier_kind_allowed_for_transaction(transaction_kind, frontier.kind) {
            return Err(WalValidationError::FrontierTransitionKindMismatch);
        }
    }
    Ok(())
}

fn frontier_kind_allowed_for_transaction(
    transaction_kind: WalTransactionKind,
    frontier_kind: AffectedFrontierKind,
) -> bool {
    match transaction_kind {
        WalTransactionKind::SubmissionIntake => {
            matches!(frontier_kind, AffectedFrontierKind::SubmissionQueue)
        }
        WalTransactionKind::SchedulerTick => matches!(
            frontier_kind,
            AffectedFrontierKind::RuntimeState
                | AffectedFrontierKind::ReceiptIndex
                | AffectedFrontierKind::ReadingIndex
        ),
        WalTransactionKind::RuntimePosture => {
            matches!(frontier_kind, AffectedFrontierKind::RuntimeControl)
        }
        WalTransactionKind::Checkpoint => {
            matches!(frontier_kind, AffectedFrontierKind::CheckpointIndex)
        }
        WalTransactionKind::MaterializationOutbox => {
            matches!(frontier_kind, AffectedFrontierKind::ReceiptIndex)
        }
        WalTransactionKind::TopologyIntent => {
            matches!(frontier_kind, AffectedFrontierKind::TopologyIndex)
        }
    }
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

fn update_optional_projection_digest(hasher: &mut blake3::Hasher, digest: Option<Hash>) {
    match digest {
        Some(digest) => {
            hasher.update(&[1]);
            hasher.update(&digest);
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn update_optional_writer_epoch_id(hasher: &mut blake3::Hasher, epoch_id: Option<WriterEpochId>) {
    match epoch_id {
        Some(epoch_id) => {
            hasher.update(&[1]);
            hasher.update(&epoch_id.as_hash());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn update_optional_lsn(hasher: &mut blake3::Hasher, lsn: Option<Lsn>) {
    match lsn {
        Some(lsn) => {
            hasher.update(&[1]);
            hasher.update(&lsn.as_u64().to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn update_recovery_tail_posture(hasher: &mut blake3::Hasher, posture: RecoveryTailPosture) {
    match posture {
        RecoveryTailPosture::Clean => {
            hasher.update(&[0]);
        }
        RecoveryTailPosture::TruncatedAll => {
            hasher.update(&[1]);
        }
        RecoveryTailPosture::TruncatedAfter(lsn) => {
            hasher.update(&[2]);
            hasher.update(&lsn.as_u64().to_le_bytes());
        }
        RecoveryTailPosture::WouldTruncateAll => {
            hasher.update(&[3]);
        }
        RecoveryTailPosture::WouldTruncateAfter(lsn) => {
            hasher.update(&[4]);
            hasher.update(&lsn.as_u64().to_le_bytes());
        }
    }
}

fn push_hash(out: &mut Vec<u8>, hash: &Hash) {
    out.extend_from_slice(hash);
}

fn push_optional_hash(out: &mut Vec<u8>, hash: Option<Hash>) {
    match hash {
        Some(hash) => {
            out.push(1);
            push_hash(out, &hash);
        }
        None => out.push(0),
    }
}

fn push_optional_lsn(out: &mut Vec<u8>, lsn: Option<Lsn>) {
    match lsn {
        Some(lsn) => {
            out.push(1);
            out.extend_from_slice(&lsn.as_u64().to_le_bytes());
        }
        None => out.push(0),
    }
}

fn push_worldline_id(out: &mut Vec<u8>, worldline_id: WorldlineId) {
    out.extend_from_slice(worldline_id.as_bytes());
}

fn push_worldline_tick(out: &mut Vec<u8>, tick: WorldlineTick) {
    out.extend_from_slice(&tick.as_u64().to_le_bytes());
}

fn push_strand_id(out: &mut Vec<u8>, strand_id: StrandId) {
    out.extend_from_slice(strand_id.as_bytes());
}

fn push_writer_head_key(out: &mut Vec<u8>, head: WriterHeadKey) {
    out.extend_from_slice(head.worldline_id.as_bytes());
    out.extend_from_slice(head.head_id.as_bytes());
}

fn push_authority_domain_ref(out: &mut Vec<u8>, authority: AuthorityDomainRef) {
    out.extend_from_slice(authority.origin_id.as_bytes());
    out.extend_from_slice(authority.domain_id.as_bytes());
}

fn push_braid_member_ref(out: &mut Vec<u8>, member_ref: BraidMemberRef) {
    match member_ref {
        BraidMemberRef::Revealed(strand_id) => {
            out.push(1);
            push_strand_id(out, strand_id);
        }
        BraidMemberRef::Sealed {
            blinded_commitment,
            authority,
        } => {
            out.push(2);
            push_hash(out, &blinded_commitment);
            push_authority_domain_ref(out, authority);
        }
    }
}

fn push_braid_event(out: &mut Vec<u8>, event: &BraidEvent) {
    match event {
        BraidEvent::BraidCreated {
            braid_id,
            creator_domain,
        } => {
            out.push(1);
            push_hash(out, braid_id);
            push_authority_domain_ref(out, *creator_domain);
        }
        BraidEvent::MemberWoven {
            member_ref,
            sequence_num,
        } => {
            out.push(2);
            push_braid_member_ref(out, *member_ref);
            out.extend_from_slice(&sequence_num.to_le_bytes());
        }
        BraidEvent::SettlementFinalized { settlement_digest } => {
            out.push(3);
            push_hash(out, settlement_digest);
        }
        BraidEvent::BraidCollapsed {
            collapse_witness,
            outcome_digest,
        } => {
            out.push(4);
            push_hash(out, collapse_witness);
            push_hash(out, outcome_digest);
        }
    }
}

fn braid_status_code(status: BraidStatus) -> u8 {
    match status {
        BraidStatus::Active => 1,
        BraidStatus::Finalized => 2,
        BraidStatus::Collapsed => 3,
    }
}

fn braid_status_from_code(code: u8) -> Result<BraidStatus, WalDecodeError> {
    match code {
        1 => Ok(BraidStatus::Active),
        2 => Ok(BraidStatus::Finalized),
        3 => Ok(BraidStatus::Collapsed),
        _ => Err(WalDecodeError::UnknownEnumCode {
            enum_name: "BraidStatus",
            code,
        }),
    }
}

fn canonical_writer_heads(heads: &[WriterHeadKey]) -> Vec<WriterHeadKey> {
    let mut sorted = heads.to_vec();
    sorted.sort_by(|left, right| {
        left.worldline_id
            .as_bytes()
            .cmp(right.worldline_id.as_bytes())
            .then_with(|| left.head_id.as_bytes().cmp(right.head_id.as_bytes()))
    });
    sorted
}

fn validate_topology_braid_event(
    record: &TopologyBraidEventRecord,
) -> Result<(), WalRecoveryIndexError> {
    if let BraidEvent::BraidCreated { braid_id, .. } = &record.event {
        if braid_id != &record.braid_id {
            return Err(WalRecoveryIndexError::ConflictingBraidEvent {
                braid_id: record.braid_id,
                event_index: record.event_index,
            });
        }
    }
    let expected_status = match &record.event {
        BraidEvent::BraidCreated { .. } | BraidEvent::MemberWoven { .. } => BraidStatus::Active,
        BraidEvent::SettlementFinalized { .. } => BraidStatus::Finalized,
        BraidEvent::BraidCollapsed { .. } => BraidStatus::Collapsed,
    };
    if record.status_after != expected_status {
        return Err(WalRecoveryIndexError::ConflictingBraidEvent {
            braid_id: record.braid_id,
            event_index: record.event_index,
        });
    }
    Ok(())
}

fn insert_unique<K, V>(
    map: &mut BTreeMap<K, V>,
    key: K,
    value: V,
    error: WalRecoveryIndexError,
) -> Result<(), WalRecoveryIndexError>
where
    K: Ord,
    V: PartialEq,
{
    if let Some(existing) = map.get(&key) {
        if existing != &value {
            return Err(error);
        }
        return Ok(());
    }
    map.insert(key, value);
    Ok(())
}

fn encode_frame(frame: &WalFrame) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&frame.header.wal_version.to_le_bytes());
    push_hash(&mut out, &frame.header.writer_epoch.as_hash());
    out.extend_from_slice(&frame.header.segment_id.as_u64().to_le_bytes());
    out.extend_from_slice(&frame.header.lsn.as_u64().to_le_bytes());
    push_hash(&mut out, &frame.header.transaction_id.as_hash());
    out.extend_from_slice(&frame.header.transaction_local_index.as_u32().to_le_bytes());
    out.push(frame.header.record_kind.code());
    out.extend_from_slice(&frame.header.payload_len.to_le_bytes());
    push_hash(&mut out, &frame.header.payload_digest);
    push_hash(&mut out, &frame.header.payload_codec_id.as_hash());
    push_hash(&mut out, &frame.header.payload_schema_id.as_hash());
    out.extend_from_slice(&frame.header.payload_schema_version.to_le_bytes());
    out.extend_from_slice(&frame.header.canonical_encoding_version.to_le_bytes());
    push_hash(&mut out, &frame.header.digest_domain);
    out.push(frame.header.compression_kind.code());
    out.push(frame.header.encryption_or_redaction_posture.code());
    push_hash(&mut out, &frame.header.previous_frame_digest);
    out.extend_from_slice(&frame.header.header_checksum.to_le_bytes());
    out.extend_from_slice(&frame.payload.schema_version.to_le_bytes());
    out.extend_from_slice(&len_u64(frame.payload.canonical_bytes.len()).to_le_bytes());
    out.extend_from_slice(&frame.payload.canonical_bytes);
    out.extend_from_slice(&frame.trailer.frame_checksum.to_le_bytes());
    out
}

fn decode_frame(bytes: &[u8]) -> Result<WalFrame, WalDecodeError> {
    let mut cursor = WalPayloadCursor::new(bytes);
    let wal_version = cursor.read_u16()?;
    let writer_epoch = WriterEpochId::from_hash(cursor.read_hash()?);
    let segment_id = WalSegmentId::from_raw(cursor.read_u64()?);
    let lsn = Lsn::from_raw(cursor.read_u64()?);
    let transaction_id = WalTransactionId::from_hash(cursor.read_hash()?);
    let transaction_local_index = TransactionLocalIndex::from_raw(cursor.read_u32()?);
    let record_kind = WalRecordKind::from_code(cursor.read_u8()?)?;
    let payload_len = cursor.read_u64()?;
    let payload_digest = cursor.read_hash()?;
    let payload_codec_id = PayloadCodecId::from_hash(cursor.read_hash()?);
    let payload_schema_id = PayloadSchemaId::from_hash(cursor.read_hash()?);
    let payload_schema_version = cursor.read_u16()?;
    let canonical_encoding_version = cursor.read_u16()?;
    let digest_domain = cursor.read_hash()?;
    let compression_kind = WalCompressionKind::from_code(cursor.read_u8()?)?;
    let encryption_or_redaction_posture = WalRedactionPosture::from_code(cursor.read_u8()?)?;
    let previous_frame_digest = cursor.read_hash()?;
    let header_checksum = cursor.read_u32()?;
    let payload_schema_version_recorded = cursor.read_u16()?;
    let canonical_bytes = cursor.read_vec()?;
    let frame_checksum = cursor.read_u32()?;
    cursor.finish()?;
    let frame = WalFrame {
        header: WalFrameHeader {
            wal_version,
            writer_epoch,
            segment_id,
            lsn,
            transaction_id,
            transaction_local_index,
            record_kind,
            payload_len,
            payload_digest,
            payload_codec_id,
            payload_schema_id,
            payload_schema_version,
            canonical_encoding_version,
            digest_domain,
            compression_kind,
            encryption_or_redaction_posture,
            previous_frame_digest,
            header_checksum,
        },
        payload: WalRecordPayload::new(
            record_kind,
            payload_schema_version_recorded,
            canonical_bytes,
        ),
        trailer: WalFrameTrailer { frame_checksum },
    };
    frame
        .validate_integrity()
        .map_err(|_| WalDecodeError::InvalidEmbeddedFrame)?;
    Ok(frame)
}

fn encode_commit(commit: &WalTransactionCommit) -> Vec<u8> {
    let mut out = Vec::new();
    push_hash(&mut out, &commit.writer_epoch.as_hash());
    push_hash(&mut out, &commit.transaction_id.as_hash());
    out.push(commit.transaction_kind.code());
    out.extend_from_slice(&commit.first_lsn.as_u64().to_le_bytes());
    out.extend_from_slice(&commit.last_lsn.as_u64().to_le_bytes());
    out.extend_from_slice(&commit.record_count.to_le_bytes());
    push_hash(&mut out, &commit.records_root);
    push_hash(&mut out, &commit.affected_frontiers_root);
    push_hash(&mut out, &commit.previous_committed_transaction_digest);
    out.push(commit.durability_mode.code());
    out.extend_from_slice(&commit.schema_version.to_le_bytes());
    push_hash(&mut out, &commit.commit_digest);
    out
}

fn decode_commit(bytes: &[u8]) -> Result<WalTransactionCommit, WalDecodeError> {
    let mut cursor = WalPayloadCursor::new(bytes);
    let writer_epoch = WriterEpochId::from_hash(cursor.read_hash()?);
    let transaction_id = WalTransactionId::from_hash(cursor.read_hash()?);
    let transaction_kind = WalTransactionKind::from_code(cursor.read_u8()?)?;
    let first_lsn = Lsn::from_raw(cursor.read_u64()?);
    let last_lsn = Lsn::from_raw(cursor.read_u64()?);
    let record_count = cursor.read_u64()?;
    let records_root = cursor.read_hash()?;
    let affected_frontiers_root = cursor.read_hash()?;
    let previous_committed_transaction_digest = cursor.read_hash()?;
    let durability_mode = WalDurabilityMode::from_code(cursor.read_u8()?)?;
    let schema_version = cursor.read_u16()?;
    let commit_digest = cursor.read_hash()?;
    cursor.finish()?;
    Ok(WalTransactionCommit {
        writer_epoch,
        transaction_id,
        transaction_kind,
        first_lsn,
        last_lsn,
        record_count,
        records_root,
        affected_frontiers_root,
        previous_committed_transaction_digest,
        durability_mode,
        schema_version,
        commit_digest,
    })
}

fn disk_record_digest(kind: u8, payload: &[u8]) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(WAL_DISK_RECORD_DOMAIN);
    h.update(&[kind]);
    h.update(&len_u64(payload.len()).to_le_bytes());
    h.update(payload);
    h.finalize().into()
}

fn causal_commit_evidence_id(commit: &WalTransactionCommit) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(b"echo:causal_wal:commit_evidence:v1\0");
    h.update(&commit.transaction_id.as_hash());
    h.update(&commit.commit_digest);
    h.finalize().into()
}

fn push_gate(
    passed: &mut Vec<&'static str>,
    blocked: &mut Vec<&'static str>,
    name: &'static str,
    ok: bool,
) {
    if ok {
        passed.push(name);
    } else {
        blocked.push(name);
    }
}

struct WalPayloadCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> WalPayloadCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn read_u8(&mut self) -> Result<u8, WalDecodeError> {
        let Some(value) = self.bytes.get(self.offset).copied() else {
            return Err(WalDecodeError::UnexpectedEof);
        };
        self.offset += 1;
        Ok(value)
    }

    fn read_u16(&mut self) -> Result<u16, WalDecodeError> {
        let end = self
            .offset
            .checked_add(2)
            .ok_or(WalDecodeError::UnexpectedEof)?;
        let Some(bytes) = self.bytes.get(self.offset..end) else {
            return Err(WalDecodeError::UnexpectedEof);
        };
        let mut out = [0; 2];
        out.copy_from_slice(bytes);
        self.offset = end;
        Ok(u16::from_le_bytes(out))
    }

    fn read_u32(&mut self) -> Result<u32, WalDecodeError> {
        let end = self
            .offset
            .checked_add(4)
            .ok_or(WalDecodeError::UnexpectedEof)?;
        let Some(bytes) = self.bytes.get(self.offset..end) else {
            return Err(WalDecodeError::UnexpectedEof);
        };
        let mut out = [0; 4];
        out.copy_from_slice(bytes);
        self.offset = end;
        Ok(u32::from_le_bytes(out))
    }

    fn read_u64(&mut self) -> Result<u64, WalDecodeError> {
        let end = self
            .offset
            .checked_add(8)
            .ok_or(WalDecodeError::UnexpectedEof)?;
        let Some(bytes) = self.bytes.get(self.offset..end) else {
            return Err(WalDecodeError::UnexpectedEof);
        };
        let mut out = [0; 8];
        out.copy_from_slice(bytes);
        self.offset = end;
        Ok(u64::from_le_bytes(out))
    }

    fn read_hash(&mut self) -> Result<Hash, WalDecodeError> {
        let end = self
            .offset
            .checked_add(32)
            .ok_or(WalDecodeError::UnexpectedEof)?;
        let Some(bytes) = self.bytes.get(self.offset..end) else {
            return Err(WalDecodeError::UnexpectedEof);
        };
        let mut out = [0; 32];
        out.copy_from_slice(bytes);
        self.offset = end;
        Ok(out)
    }

    fn read_optional_hash(&mut self) -> Result<Option<Hash>, WalDecodeError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_hash().map(Some),
            code => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "Option<Hash>",
                code,
            }),
        }
    }

    fn read_optional_lsn(&mut self) -> Result<Option<Lsn>, WalDecodeError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_u64().map(|raw| Some(Lsn::from_raw(raw))),
            code => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "Option<Lsn>",
                code,
            }),
        }
    }

    fn read_worldline_id(&mut self) -> Result<WorldlineId, WalDecodeError> {
        self.read_hash().map(WorldlineId::from_bytes)
    }

    fn read_worldline_tick(&mut self) -> Result<WorldlineTick, WalDecodeError> {
        self.read_u64().map(WorldlineTick::from_raw)
    }

    fn read_strand_id(&mut self) -> Result<StrandId, WalDecodeError> {
        self.read_hash().map(StrandId::from_bytes)
    }

    fn read_writer_head_key(&mut self) -> Result<WriterHeadKey, WalDecodeError> {
        let worldline_id = self.read_worldline_id()?;
        let head_id = HeadId::from_bytes(self.read_hash()?);
        Ok(WriterHeadKey {
            worldline_id,
            head_id,
        })
    }

    fn read_authority_domain_ref(&mut self) -> Result<AuthorityDomainRef, WalDecodeError> {
        let origin_id = OriginId::from_bytes(self.read_hash()?);
        let domain_id = AuthorityDomainId::from_bytes(self.read_hash()?);
        Ok(AuthorityDomainRef::new(origin_id, domain_id))
    }

    fn read_braid_member_ref(&mut self) -> Result<BraidMemberRef, WalDecodeError> {
        match self.read_u8()? {
            1 => self.read_strand_id().map(BraidMemberRef::Revealed),
            2 => {
                let blinded_commitment = self.read_hash()?;
                let authority = self.read_authority_domain_ref()?;
                Ok(BraidMemberRef::Sealed {
                    blinded_commitment,
                    authority,
                })
            }
            code => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "BraidMemberRef",
                code,
            }),
        }
    }

    fn read_braid_event(&mut self) -> Result<BraidEvent, WalDecodeError> {
        match self.read_u8()? {
            1 => {
                let braid_id = self.read_hash()?;
                let creator_domain = self.read_authority_domain_ref()?;
                Ok(BraidEvent::BraidCreated {
                    braid_id,
                    creator_domain,
                })
            }
            2 => {
                let member_ref = self.read_braid_member_ref()?;
                let sequence_num = self.read_u64()?;
                Ok(BraidEvent::MemberWoven {
                    member_ref,
                    sequence_num,
                })
            }
            3 => {
                let settlement_digest = self.read_hash()?;
                Ok(BraidEvent::SettlementFinalized { settlement_digest })
            }
            4 => {
                let collapse_witness = self.read_hash()?;
                let outcome_digest = self.read_hash()?;
                Ok(BraidEvent::BraidCollapsed {
                    collapse_witness,
                    outcome_digest,
                })
            }
            code => Err(WalDecodeError::UnknownEnumCode {
                enum_name: "BraidEvent",
                code,
            }),
        }
    }

    fn read_vec(&mut self) -> Result<Vec<u8>, WalDecodeError> {
        let len = usize::try_from(self.read_u64()?).map_err(|_| WalDecodeError::UnexpectedEof)?;
        let end = self
            .offset
            .checked_add(len)
            .ok_or(WalDecodeError::UnexpectedEof)?;
        let Some(bytes) = self.bytes.get(self.offset..end) else {
            return Err(WalDecodeError::UnexpectedEof);
        };
        self.offset = end;
        Ok(bytes.to_vec())
    }

    fn finish(&self) -> Result<(), WalDecodeError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(WalDecodeError::TrailingBytes)
        }
    }
}

fn checkpoint_temp_path(path: &Path) -> Result<PathBuf, WalCheckpointIoError> {
    let file_name = path
        .file_name()
        .ok_or(WalCheckpointIoError::MissingParentDirectory)?;
    let temp_name = format!(".{}.tmp", file_name.to_string_lossy());
    Ok(path.with_file_name(temp_name))
}

fn checkpoint_file_bytes(checkpoint: &CheckpointRecord) -> Vec<u8> {
    let payload = checkpoint.to_payload_bytes();
    let digest = checkpoint.checkpoint_digest();
    let mut out = Vec::new();
    out.extend_from_slice(CHECKPOINT_FILE_MAGIC);
    out.extend_from_slice(&len_u64(payload.len()).to_le_bytes());
    out.extend_from_slice(&payload);
    out.extend_from_slice(&digest);
    out
}

fn parse_checkpoint_file_bytes(bytes: &[u8]) -> Result<CheckpointRecord, WalCheckpointIoError> {
    if bytes.get(..CHECKPOINT_FILE_MAGIC.len()) != Some(CHECKPOINT_FILE_MAGIC.as_slice()) {
        return Err(WalCheckpointIoError::InvalidMagic);
    }
    let mut offset = CHECKPOINT_FILE_MAGIC.len();
    let len_end = offset.checked_add(8).ok_or(WalDecodeError::UnexpectedEof)?;
    let Some(len_bytes) = bytes.get(offset..len_end) else {
        return Err(WalCheckpointIoError::Decode(WalDecodeError::UnexpectedEof));
    };
    let mut payload_len = [0; 8];
    payload_len.copy_from_slice(len_bytes);
    offset = len_end;
    let payload_len = usize::try_from(u64::from_le_bytes(payload_len))
        .map_err(|_| WalDecodeError::UnexpectedEof)?;
    let payload_end = offset
        .checked_add(payload_len)
        .ok_or(WalDecodeError::UnexpectedEof)?;
    let Some(payload) = bytes.get(offset..payload_end) else {
        return Err(WalCheckpointIoError::Decode(WalDecodeError::UnexpectedEof));
    };
    let digest_end = payload_end
        .checked_add(32)
        .ok_or(WalDecodeError::UnexpectedEof)?;
    let Some(digest_bytes) = bytes.get(payload_end..digest_end) else {
        return Err(WalCheckpointIoError::Decode(WalDecodeError::UnexpectedEof));
    };
    if digest_end != bytes.len() {
        return Err(WalCheckpointIoError::Decode(WalDecodeError::TrailingBytes));
    }
    let checkpoint = CheckpointRecord::from_payload_bytes(payload)?;
    if digest_bytes != checkpoint.checkpoint_digest() {
        return Err(WalCheckpointIoError::DigestMismatch);
    }
    Ok(checkpoint)
}

fn sync_directory(path: &Path) -> Result<(), WalCheckpointIoError> {
    File::open(path)?.sync_all()?;
    Ok(())
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
