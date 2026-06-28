// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generic WSC storage port and deterministic envelope format.

use std::collections::{BTreeMap, BTreeSet};

use blake3::Hasher;
use bytes::Bytes;
use thiserror::Error;

use crate::attachment::{AtomPayload, AttachmentValue};
use crate::causal_wal::{
    materialize_wal_projection_graph, observe_wal_projection_graph_wsc,
    wal_projection_graph_schema_hash, BraidShellRetentionRecord, Lsn, ReadingRefRecord,
    RetainedMaterialRecord, StrandDropRecord, StrandForkRecord, SubmissionAcceptanceRecord,
    SuffixImportRecord, TickReceiptRecord, TopologyBraidEventRecord, TopologyIntentRecord,
    WalProjectionGraphObservation, WalProjectionGraphObservationError, WalReceiptCorrelationRecord,
    WalRoot, WalSegmentId, WalSegmentRef, WalSegmentStorageLocator,
};
use crate::graph::GraphStore;
use crate::ident::{make_node_id, make_type_id, make_warp_id, EdgeId, Hash, NodeId};
use crate::record::{EdgeRecord, NodeRecord};

use super::build::build_one_warp_input;
use super::types::AttRow;
use super::validate::validate_wsc;
use super::view::WscFile;
use super::write::write_wsc_one_warp;

const WSC_STORE_ENVELOPE_MAGIC: &[u8; 8] = b"ECWSCST1";
const WSC_STORE_ENVELOPE_VERSION: u16 = 1;
const WSC_STORE_ENVELOPE_ID_DOMAIN: &[u8] = b"echo:wsc_store:envelope_id:v1\0";
const WSC_STORE_BYTES_DOMAIN: &[u8] = b"echo:wsc_store:wsc_bytes:v1\0";
const WSC_STORE_COMMIT_MARKER_DOMAIN: &[u8] = b"echo:wsc_store:commit_marker:v1\0";
const WSC_ACCEPTED_SUBMISSION_BASIS_DOMAIN: &[u8] =
    b"echo:wsc_store:accepted_submission_basis:v1\0";
const WSC_ACCEPTED_SUBMISSION_NODE_DOMAIN: &[u8] = b"echo:wsc_store:accepted_submission_node:v1\0";
const WSC_ACCEPTED_SUBMISSION_EDGE_DOMAIN: &[u8] = b"echo:wsc_store:accepted_submission_edge:v1\0";
const WSC_ACCEPTED_SUBMISSION_SCHEMA: &str = "echo/wsc-store/accepted-submissions/v1";
const WSC_ACCEPTED_SUBMISSION_WARP: &str = "echo/wsc-store/accepted-submissions";
const WSC_ACCEPTED_SUBMISSION_ROOT: &str = "echo/wsc-store/accepted-submissions/root";
const WSC_ACCEPTED_SUBMISSION_NODE_TYPE: &str = "echo/wsc-store/accepted-submissions/node/v1";
const WSC_ACCEPTED_SUBMISSION_EDGE_TYPE: &str = "echo/wsc-store/accepted-submissions/member/v1";
const WSC_ACCEPTED_SUBMISSION_ATTACHMENT_TYPE: &str =
    "echo/wsc-store/accepted-submissions/record/v1";
const WSC_RECEIPT_CORRELATION_BASIS_DOMAIN: &[u8] =
    b"echo:wsc_store:receipt_correlation_basis:v1\0";
const WSC_RECEIPT_CORRELATION_NODE_DOMAIN: &[u8] = b"echo:wsc_store:receipt_correlation_node:v1\0";
const WSC_RECEIPT_CORRELATION_EDGE_DOMAIN: &[u8] = b"echo:wsc_store:receipt_correlation_edge:v1\0";
const WSC_RECEIPT_CORRELATION_SCHEMA: &str = "echo/wsc-store/receipt-correlations/v1";
const WSC_RECEIPT_CORRELATION_WARP: &str = "echo/wsc-store/receipt-correlations";
const WSC_RECEIPT_CORRELATION_ROOT: &str = "echo/wsc-store/receipt-correlations/root";
const WSC_RECEIPT_CORRELATION_NODE_TYPE: &str = "echo/wsc-store/receipt-correlations/node/v1";
const WSC_RECEIPT_CORRELATION_EDGE_TYPE: &str = "echo/wsc-store/receipt-correlations/member/v1";
const WSC_TICK_RECEIPT_ATTACHMENT_TYPE: &str = "echo/wsc-store/receipt-correlations/receipt/v1";
const WSC_RECEIPT_CORRELATION_ATTACHMENT_TYPE: &str =
    "echo/wsc-store/receipt-correlations/correlation/v1";
const WSC_RETENTION_BASIS_DOMAIN: &[u8] = b"echo:wsc_store:retention_basis:v1\0";
const WSC_RETENTION_NODE_DOMAIN: &[u8] = b"echo:wsc_store:retention_node:v1\0";
const WSC_RETENTION_EDGE_DOMAIN: &[u8] = b"echo:wsc_store:retention_edge:v1\0";
const WSC_RETENTION_SCHEMA: &str = "echo/wsc-store/retention/v1";
const WSC_RETENTION_WARP: &str = "echo/wsc-store/retention";
const WSC_RETENTION_ROOT: &str = "echo/wsc-store/retention/root";
const WSC_RETENTION_NODE_TYPE: &str = "echo/wsc-store/retention/node/v1";
const WSC_RETENTION_EDGE_TYPE: &str = "echo/wsc-store/retention/member/v1";
const WSC_RETAINED_MATERIAL_ATTACHMENT_TYPE: &str = "echo/wsc-store/retention/material/v1";
const WSC_READING_REF_ATTACHMENT_TYPE: &str = "echo/wsc-store/retention/reading/v1";
const WSC_TOPOLOGY_BASIS_DOMAIN: &[u8] = b"echo:wsc_store:topology_basis:v1\0";
const WSC_TOPOLOGY_NODE_DOMAIN: &[u8] = b"echo:wsc_store:topology_node:v1\0";
const WSC_TOPOLOGY_EDGE_DOMAIN: &[u8] = b"echo:wsc_store:topology_edge:v1\0";
const WSC_TOPOLOGY_SCHEMA: &str = "echo/wsc-store/topology/v1";
const WSC_TOPOLOGY_WARP: &str = "echo/wsc-store/topology";
const WSC_TOPOLOGY_ROOT: &str = "echo/wsc-store/topology/root";
const WSC_TOPOLOGY_NODE_TYPE: &str = "echo/wsc-store/topology/node/v1";
const WSC_TOPOLOGY_EDGE_TYPE: &str = "echo/wsc-store/topology/member/v1";
const WSC_TOPOLOGY_STRAND_FORK_ATTACHMENT_TYPE: &str = "echo/wsc-store/topology/strand-fork/v1";
const WSC_TOPOLOGY_STRAND_DROP_ATTACHMENT_TYPE: &str = "echo/wsc-store/topology/strand-drop/v1";
const WSC_TOPOLOGY_BRAID_EVENT_ATTACHMENT_TYPE: &str = "echo/wsc-store/topology/braid-event/v1";
const WSC_TOPOLOGY_BRAID_SHELL_ATTACHMENT_TYPE: &str = "echo/wsc-store/topology/braid-shell/v1";
const WSC_TOPOLOGY_SUFFIX_IMPORT_ATTACHMENT_TYPE: &str = "echo/wsc-store/topology/suffix-import/v1";
const HEADER_LEN: usize = 124;

/// Stable identifier for a WSC store envelope.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WscStoreEnvelopeId(Hash);

impl WscStoreEnvelopeId {
    /// Builds an envelope id from a canonical digest.
    #[must_use]
    pub const fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Returns the canonical digest bytes.
    #[must_use]
    pub const fn as_hash(self) -> Hash {
        self.0
    }
}

/// Generic kind of WSC material stored by Echo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WscStoreRecordKind {
    /// Materialized causal snapshot.
    Snapshot,
    /// Causal-history material.
    CausalHistory,
    /// Retained evidence material.
    RetainedEvidence,
}

impl WscStoreRecordKind {
    const fn code(self) -> u16 {
        match self {
            Self::Snapshot => 1,
            Self::CausalHistory => 2,
            Self::RetainedEvidence => 3,
        }
    }

    const fn from_code(code: u16) -> Option<Self> {
        match code {
            1 => Some(Self::Snapshot),
            2 => Some(Self::CausalHistory),
            3 => Some(Self::RetainedEvidence),
            _ => None,
        }
    }
}

/// Current version of the causal-history WSC export profile model.
pub const WSC_CAUSAL_HISTORY_EXPORT_PROFILE_VERSION: u16 = 1;

/// Versioned causal-history WSC export profile kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WscCausalHistoryExportProfileKind {
    /// Reference-only export carrying graph facts and storage references.
    RefOnly,
    /// Self-contained export carrying embedded validation material.
    SelfContained,
    /// CAS-addressed export carrying byte hashes plus semantic references.
    CasAddressed,
}

impl WscCausalHistoryExportProfileKind {
    /// Returns the stable profile label used in manifests and tests.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::RefOnly => "ref-only",
            Self::SelfContained => "self-contained",
            Self::CasAddressed => "CAS-addressed",
        }
    }
}

/// Evidence requirement for a causal-history WSC export profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WscCausalHistoryExportEvidence {
    /// The profile must carry this evidence.
    Required,
    /// The profile may carry this evidence, but consumers must not rely on it.
    Optional,
    /// The profile must not carry this evidence.
    Forbidden,
}

/// Validation material mode for a causal-history WSC export profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WscCausalHistoryExportValidationMaterial {
    /// Validation depends on external WAL storage named by locators.
    ExternalWalStorageRefs,
    /// Validation can use embedded segment bytes or retained material.
    EmbeddedSegmentBytesOrRetainedMaterial,
    /// Validation names CAS bytes through content hashes and semantic refs.
    CasHashesWithSemanticRefs,
}

/// CAS authority posture for a causal-history WSC export profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WscCausalHistoryCasAuthority {
    /// CAS names retained bytes only and is not causal authority.
    ByteRetentionOnly,
}

/// Versioned causal-history WSC export profile evidence requirements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WscCausalHistoryExportProfile {
    /// Profile model version.
    pub version: u16,
    /// Export profile kind.
    pub kind: WscCausalHistoryExportProfileKind,
    /// Required validation material mode.
    pub validation_material: WscCausalHistoryExportValidationMaterial,
    /// Projected graph facts requirement.
    pub projected_graph_facts: WscCausalHistoryExportEvidence,
    /// Segment locator requirement.
    pub segment_locators: WscCausalHistoryExportEvidence,
    /// Segment digest requirement.
    pub segment_digests: WscCausalHistoryExportEvidence,
    /// LSN range requirement.
    pub lsn_ranges: WscCausalHistoryExportEvidence,
    /// Commit anchor requirement.
    pub commit_anchors: WscCausalHistoryExportEvidence,
    /// Embedded segment bytes or retained material requirement.
    pub embedded_segment_bytes_or_retained_material: WscCausalHistoryExportEvidence,
    /// CAS content hash requirement.
    pub cas_content_hashes: WscCausalHistoryExportEvidence,
    /// Semantic reference requirement for CAS-addressed material.
    pub semantic_refs: WscCausalHistoryExportEvidence,
    /// CAS authority posture, when the profile uses CAS-addressed material.
    pub cas_authority: Option<WscCausalHistoryCasAuthority>,
}

/// Returns the evidence requirements for one causal-history WSC export profile.
#[must_use]
pub const fn wsc_causal_history_export_profile(
    kind: WscCausalHistoryExportProfileKind,
) -> WscCausalHistoryExportProfile {
    match kind {
        WscCausalHistoryExportProfileKind::RefOnly => WscCausalHistoryExportProfile {
            version: WSC_CAUSAL_HISTORY_EXPORT_PROFILE_VERSION,
            kind,
            validation_material: WscCausalHistoryExportValidationMaterial::ExternalWalStorageRefs,
            projected_graph_facts: WscCausalHistoryExportEvidence::Required,
            segment_locators: WscCausalHistoryExportEvidence::Required,
            segment_digests: WscCausalHistoryExportEvidence::Required,
            lsn_ranges: WscCausalHistoryExportEvidence::Required,
            commit_anchors: WscCausalHistoryExportEvidence::Required,
            embedded_segment_bytes_or_retained_material: WscCausalHistoryExportEvidence::Forbidden,
            cas_content_hashes: WscCausalHistoryExportEvidence::Forbidden,
            semantic_refs: WscCausalHistoryExportEvidence::Forbidden,
            cas_authority: None,
        },
        WscCausalHistoryExportProfileKind::SelfContained => WscCausalHistoryExportProfile {
            version: WSC_CAUSAL_HISTORY_EXPORT_PROFILE_VERSION,
            kind,
            validation_material:
                WscCausalHistoryExportValidationMaterial::EmbeddedSegmentBytesOrRetainedMaterial,
            projected_graph_facts: WscCausalHistoryExportEvidence::Required,
            segment_locators: WscCausalHistoryExportEvidence::Optional,
            segment_digests: WscCausalHistoryExportEvidence::Required,
            lsn_ranges: WscCausalHistoryExportEvidence::Required,
            commit_anchors: WscCausalHistoryExportEvidence::Required,
            embedded_segment_bytes_or_retained_material: WscCausalHistoryExportEvidence::Required,
            cas_content_hashes: WscCausalHistoryExportEvidence::Forbidden,
            semantic_refs: WscCausalHistoryExportEvidence::Forbidden,
            cas_authority: None,
        },
        WscCausalHistoryExportProfileKind::CasAddressed => WscCausalHistoryExportProfile {
            version: WSC_CAUSAL_HISTORY_EXPORT_PROFILE_VERSION,
            kind,
            validation_material:
                WscCausalHistoryExportValidationMaterial::CasHashesWithSemanticRefs,
            projected_graph_facts: WscCausalHistoryExportEvidence::Required,
            segment_locators: WscCausalHistoryExportEvidence::Forbidden,
            segment_digests: WscCausalHistoryExportEvidence::Required,
            lsn_ranges: WscCausalHistoryExportEvidence::Required,
            commit_anchors: WscCausalHistoryExportEvidence::Required,
            embedded_segment_bytes_or_retained_material: WscCausalHistoryExportEvidence::Forbidden,
            cas_content_hashes: WscCausalHistoryExportEvidence::Required,
            semantic_refs: WscCausalHistoryExportEvidence::Required,
            cas_authority: Some(WscCausalHistoryCasAuthority::ByteRetentionOnly),
        },
    }
}

/// Returns all supported causal-history WSC export profiles.
#[must_use]
pub const fn wsc_causal_history_export_profiles() -> [WscCausalHistoryExportProfile; 3] {
    [
        wsc_causal_history_export_profile(WscCausalHistoryExportProfileKind::RefOnly),
        wsc_causal_history_export_profile(WscCausalHistoryExportProfileKind::SelfContained),
        wsc_causal_history_export_profile(WscCausalHistoryExportProfileKind::CasAddressed),
    ]
}

/// Ref-only segment material dependency represented by a WSC export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WscRefOnlyWalMaterialDependency {
    /// Segment bytes must be supplied from external WAL storage.
    ExternalSegmentBytes,
}

/// Segment locator posture carried by a ref-only WSC import report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WscRefOnlyWalLocatorPosture {
    /// A relative locator was present, but remains non-authoritative metadata.
    RelativePath,
    /// An absolute host locator was present and normalized out of causal identity.
    AbsolutePathNormalized,
}

/// Segment dependency recovered for a ref-only WAL WSC export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscRefOnlyWalSegmentDependency {
    /// Logical WAL segment id.
    pub segment_id: WalSegmentId,
    /// Segment projection identity digest, excluding locator metadata.
    pub segment_identity_digest: Hash,
    /// Segment byte digest from WAL recovery evidence.
    pub segment_digest: Hash,
    /// First LSN covered by the segment projection.
    pub first_lsn: Lsn,
    /// Last LSN covered by the segment projection.
    pub last_lsn: Lsn,
    /// Commit anchor identity digests covered by the segment.
    pub commit_anchor_digests: Vec<Hash>,
    /// External material required to validate the ref-only segment.
    pub material_dependency: WscRefOnlyWalMaterialDependency,
    /// Locator posture, with host path strings normalized out.
    pub locator_posture: WscRefOnlyWalLocatorPosture,
}

/// Ref-only WAL causal-history WSC export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscRefOnlyWalExport {
    /// Export profile.
    pub profile: WscCausalHistoryExportProfileKind,
    /// WAL projection graph WSC envelope.
    pub projection_envelope: WscStoreEnvelope,
    /// Accepted submission evidence WSC envelope.
    pub accepted_submission_envelope: WscStoreEnvelope,
    /// Tick receipt and receipt-correlation WSC envelope.
    pub receipt_correlation_envelope: WscStoreEnvelope,
    /// External segment byte dependencies for ref-only validation.
    pub segment_dependencies: Vec<WscRefOnlyWalSegmentDependency>,
}

/// Imported and validated ref-only WAL causal-history WSC evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscRefOnlyWalImport {
    /// Export profile.
    pub profile: WscCausalHistoryExportProfileKind,
    /// Observed projection graph WSC shape.
    pub projection: WalProjectionGraphObservation,
    /// Expected WAL root identity digest validated against the projection WSC.
    pub root_identity_digest: Hash,
    /// Accepted submission records recovered from WSC.
    pub accepted_submissions: Vec<SubmissionAcceptanceRecord>,
    /// Tick receipt records recovered from WSC.
    pub receipts: Vec<TickReceiptRecord>,
    /// Receipt-correlation records recovered from WSC.
    pub correlations: Vec<WalReceiptCorrelationRecord>,
    /// External segment byte dependencies for ref-only validation.
    pub segment_dependencies: Vec<WscRefOnlyWalSegmentDependency>,
}

/// Error returned when building a ref-only WAL WSC export.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WscRefOnlyWalExportError {
    /// Projection graph WSC serialization failed.
    #[error("failed to write WAL projection graph WSC payload")]
    ProjectionWriteFailed,
    /// Ref-only exports require segment locator metadata.
    #[error("ref-only WAL WSC export is missing segment locator metadata")]
    MissingSegmentLocator {
        /// Segment id missing locator metadata.
        segment_id: WalSegmentId,
    },
    /// A generated WSC store envelope was invalid.
    #[error("invalid ref-only WAL WSC envelope")]
    Envelope(WscStoreObstruction),
}

/// Error returned when importing a ref-only WAL WSC export.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WscRefOnlyWalImportError {
    /// The export profile was not ref-only.
    #[error("WAL WSC export profile is not ref-only")]
    ProfileMismatch {
        /// Actual profile kind.
        actual: WscCausalHistoryExportProfileKind,
    },
    /// The projection envelope was not causal-history WSC material.
    #[error("WAL projection envelope is not causal-history material")]
    InvalidProjectionEnvelopeKind,
    /// The projection WSC payload could not be observed as WAL projection graph material.
    #[error("invalid WAL projection graph WSC payload")]
    ProjectionObservation(WalProjectionGraphObservationError),
    /// Expected projection WSC material could not be rebuilt from the recovered root.
    #[error("failed to rebuild expected WAL projection WSC material")]
    ExpectedProjection(WscRefOnlyWalExportError),
    /// The projection envelope basis digest did not match the recovered WAL root.
    #[error("WAL projection basis digest mismatch")]
    ProjectionBasisMismatch {
        /// Expected root identity digest.
        expected: Hash,
        /// Actual envelope basis digest.
        actual: Hash,
    },
    /// The projection WSC bytes did not match the recovered WAL root projection.
    #[error("WAL projection payload digest mismatch")]
    ProjectionPayloadMismatch {
        /// Expected WSC payload digest.
        expected: Hash,
        /// Actual WSC payload digest.
        actual: Hash,
    },
    /// Segment dependency sidecar did not match recovered WAL root evidence.
    #[error("WAL ref-only segment dependency mismatch")]
    SegmentDependencyMismatch,
    /// Accepted submission WSC material was invalid.
    #[error("invalid accepted submission WSC material")]
    AcceptedSubmissions(WscStoreObstruction),
    /// Receipt correlation WSC material was invalid.
    #[error("invalid receipt correlation WSC material")]
    ReceiptCorrelations(WscStoreObstruction),
    /// Causal-history WSC evidence was incomplete.
    #[error("incomplete causal-history WSC material")]
    IncompleteCausalHistory(WscStoreObstruction),
}

/// Subject named by a WSC store obstruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WscStoreSubject {
    /// Envelope identity was implicated.
    Envelope {
        /// Envelope id.
        envelope_id: WscStoreEnvelopeId,
    },
    /// Encoded bytes were malformed near an offset.
    EnvelopeBytes {
        /// Byte offset implicated by the obstruction.
        offset: usize,
    },
    /// Envelope digest evidence mismatched.
    EnvelopeDigest {
        /// Expected digest recorded by the envelope.
        expected: Hash,
        /// Actual digest computed from the payload.
        actual: Hash,
    },
    /// WSC payload was invalid.
    WscPayload {
        /// Digest of the invalid WSC payload.
        digest: Hash,
    },
    /// Causal-history material was inconsistent.
    CausalHistory {
        /// Digest naming the inconsistent causal-history subject.
        subject_digest: Hash,
    },
}

/// Generic WSC store obstruction kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WscStoreObstructionKind {
    /// Requested envelope was absent.
    MissingEnvelope,
    /// Envelope header or structural fields were malformed.
    InvalidEnvelope,
    /// WSC payload failed WSC parsing or validation.
    InvalidWsc,
    /// Encoded envelope digest did not match its payload.
    DigestMismatch,
    /// Envelope basis digest did not match recovered canonical records.
    BasisDigestMismatch,
    /// Existing envelope id maps to different material.
    DuplicateEnvelopeMismatch,
    /// Envelope material exists without a matching commit marker, or vice versa.
    IncompleteEnvelopeWrite,
    /// Commit marker does not match the envelope material.
    CommitMarkerMismatch,
    /// Committed causal-history records are missing required partner material.
    IncompleteCausalHistory,
}

/// Typed obstruction returned instead of hidden fallback or invented success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WscStoreObstruction {
    /// Obstruction kind.
    pub kind: WscStoreObstructionKind,
    /// Obstruction subject.
    pub subject: WscStoreSubject,
}

impl WscStoreObstruction {
    fn invalid_envelope(offset: usize) -> Self {
        Self {
            kind: WscStoreObstructionKind::InvalidEnvelope,
            subject: WscStoreSubject::EnvelopeBytes { offset },
        }
    }

    fn invalid_wsc(wsc_digest: Hash) -> Self {
        Self {
            kind: WscStoreObstructionKind::InvalidWsc,
            subject: WscStoreSubject::WscPayload { digest: wsc_digest },
        }
    }

    fn digest_mismatch(expected: Hash, actual: Hash) -> Self {
        Self {
            kind: WscStoreObstructionKind::DigestMismatch,
            subject: WscStoreSubject::EnvelopeDigest { expected, actual },
        }
    }

    fn basis_digest_mismatch(expected: Hash, actual: Hash) -> Self {
        Self {
            kind: WscStoreObstructionKind::BasisDigestMismatch,
            subject: WscStoreSubject::EnvelopeDigest { expected, actual },
        }
    }

    fn missing_envelope(envelope_id: WscStoreEnvelopeId) -> Self {
        Self {
            kind: WscStoreObstructionKind::MissingEnvelope,
            subject: WscStoreSubject::Envelope { envelope_id },
        }
    }

    fn duplicate_mismatch(envelope_id: WscStoreEnvelopeId) -> Self {
        Self {
            kind: WscStoreObstructionKind::DuplicateEnvelopeMismatch,
            subject: WscStoreSubject::Envelope { envelope_id },
        }
    }

    fn incomplete_write(envelope_id: WscStoreEnvelopeId) -> Self {
        Self {
            kind: WscStoreObstructionKind::IncompleteEnvelopeWrite,
            subject: WscStoreSubject::Envelope { envelope_id },
        }
    }

    fn commit_marker_mismatch(envelope_id: WscStoreEnvelopeId) -> Self {
        Self {
            kind: WscStoreObstructionKind::CommitMarkerMismatch,
            subject: WscStoreSubject::Envelope { envelope_id },
        }
    }

    fn incomplete_causal_history(subject_digest: Hash) -> Self {
        Self {
            kind: WscStoreObstructionKind::IncompleteCausalHistory,
            subject: WscStoreSubject::CausalHistory { subject_digest },
        }
    }
}

/// Deterministic WSC store envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscStoreEnvelope {
    id: WscStoreEnvelopeId,
    record_kind: WscStoreRecordKind,
    basis_digest: Hash,
    schema_hash: Hash,
    tick: u64,
    wsc_digest: Hash,
    wsc_len: u64,
    wsc_bytes: Vec<u8>,
}

impl WscStoreEnvelope {
    /// Builds and validates a WSC store envelope.
    ///
    /// # Errors
    ///
    /// Returns [`WscStoreObstructionKind::InvalidWsc`] when the payload is not
    /// valid WSC material.
    pub fn validated(
        record_kind: WscStoreRecordKind,
        basis_digest: Hash,
        wsc_bytes: Vec<u8>,
    ) -> Result<Self, WscStoreObstruction> {
        let wsc_digest = digest_wsc_bytes(&wsc_bytes);
        let file = WscFile::from_bytes(wsc_bytes.clone())
            .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
        validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
        let schema_hash = *file.schema_hash();
        let tick = file.tick();
        let wsc_len = u64::try_from(wsc_bytes.len())
            .map_err(|_| WscStoreObstruction::invalid_envelope(HEADER_LEN))?;
        let id = derive_envelope_id(
            record_kind,
            &basis_digest,
            &schema_hash,
            tick,
            &wsc_digest,
            wsc_len,
        );
        Ok(Self {
            id,
            record_kind,
            basis_digest,
            schema_hash,
            tick,
            wsc_digest,
            wsc_len,
            wsc_bytes,
        })
    }

    /// Decodes and validates a deterministic WSC store envelope.
    ///
    /// # Errors
    ///
    /// Returns a typed WSC store obstruction for malformed envelopes, digest
    /// mismatch, or invalid WSC payloads.
    pub fn decode(bytes: &[u8]) -> Result<Self, WscStoreObstruction> {
        let magic = read_array::<8>(bytes, 0)?;
        if &magic != WSC_STORE_ENVELOPE_MAGIC {
            return Err(WscStoreObstruction::invalid_envelope(0));
        }
        let version = u16::from_le_bytes(read_array::<2>(bytes, 8)?);
        if version != WSC_STORE_ENVELOPE_VERSION {
            return Err(WscStoreObstruction::invalid_envelope(8));
        }
        let record_kind_code = u16::from_le_bytes(read_array::<2>(bytes, 10)?);
        let record_kind = WscStoreRecordKind::from_code(record_kind_code)
            .ok_or_else(|| WscStoreObstruction::invalid_envelope(10))?;
        let schema_hash = read_array::<32>(bytes, 12)?;
        let basis_digest = read_array::<32>(bytes, 44)?;
        let wsc_digest = read_array::<32>(bytes, 76)?;
        let tick = u64::from_le_bytes(read_array::<8>(bytes, 108)?);
        let wsc_len = u64::from_le_bytes(read_array::<8>(bytes, 116)?);
        let payload_start = 124usize;
        let payload_len =
            usize::try_from(wsc_len).map_err(|_| WscStoreObstruction::invalid_envelope(116))?;
        let payload_end = payload_start
            .checked_add(payload_len)
            .ok_or_else(|| WscStoreObstruction::invalid_envelope(payload_start))?;
        let payload = bytes
            .get(payload_start..payload_end)
            .ok_or_else(|| WscStoreObstruction::invalid_envelope(payload_start))?;
        if payload_end != bytes.len() {
            return Err(WscStoreObstruction::invalid_envelope(payload_end));
        }
        let actual_digest = digest_wsc_bytes(payload);
        if actual_digest != wsc_digest {
            return Err(WscStoreObstruction::digest_mismatch(
                wsc_digest,
                actual_digest,
            ));
        }
        let envelope = Self::validated(record_kind, basis_digest, payload.to_vec())?;
        if envelope.schema_hash != schema_hash
            || envelope.tick != tick
            || envelope.wsc_len != wsc_len
        {
            return Err(WscStoreObstruction::invalid_envelope(12));
        }
        Ok(envelope)
    }

    /// Encodes this envelope into deterministic bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.wsc_bytes.len());
        bytes.extend_from_slice(WSC_STORE_ENVELOPE_MAGIC);
        bytes.extend_from_slice(&WSC_STORE_ENVELOPE_VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.record_kind.code().to_le_bytes());
        bytes.extend_from_slice(&self.schema_hash);
        bytes.extend_from_slice(&self.basis_digest);
        bytes.extend_from_slice(&self.wsc_digest);
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        bytes.extend_from_slice(&self.wsc_len.to_le_bytes());
        bytes.extend_from_slice(&self.wsc_bytes);
        bytes
    }

    /// Returns the envelope id.
    #[must_use]
    pub const fn id(&self) -> WscStoreEnvelopeId {
        self.id
    }

    /// Returns the generic record kind.
    #[must_use]
    pub const fn record_kind(&self) -> WscStoreRecordKind {
        self.record_kind
    }

    /// Returns the basis digest.
    #[must_use]
    pub const fn basis_digest(&self) -> &Hash {
        &self.basis_digest
    }

    /// Returns the WSC schema hash.
    #[must_use]
    pub const fn schema_hash(&self) -> &Hash {
        &self.schema_hash
    }

    /// Returns the WSC tick.
    #[must_use]
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Returns the WSC payload digest.
    #[must_use]
    pub const fn wsc_digest(&self) -> &Hash {
        &self.wsc_digest
    }

    /// Returns the WSC bytes.
    #[must_use]
    pub fn wsc_bytes(&self) -> &[u8] {
        &self.wsc_bytes
    }
}

/// Receipt returned after a WSC envelope write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WscStoreWriteReceipt {
    /// Written envelope id.
    pub envelope_id: WscStoreEnvelopeId,
    /// Commit marker digest proving the envelope was published.
    pub commit_marker_digest: Hash,
    /// WSC payload digest.
    pub wsc_digest: Hash,
    /// Encoded envelope byte length.
    pub encoded_len: u64,
}

/// Commit marker for a completed WSC envelope write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WscStoreCommitMarker {
    envelope_id: WscStoreEnvelopeId,
    record_kind: WscStoreRecordKind,
    basis_digest: Hash,
    schema_hash: Hash,
    tick: u64,
    wsc_digest: Hash,
    encoded_len: u64,
    marker_digest: Hash,
}

impl WscStoreCommitMarker {
    fn from_envelope(envelope: &WscStoreEnvelope) -> Result<Self, WscStoreObstruction> {
        let encoded_len = u64::try_from(envelope.encode().len())
            .map_err(|_| WscStoreObstruction::invalid_envelope(HEADER_LEN))?;
        let marker_digest = derive_commit_marker_digest(envelope, encoded_len);
        Ok(Self {
            envelope_id: envelope.id(),
            record_kind: envelope.record_kind(),
            basis_digest: *envelope.basis_digest(),
            schema_hash: *envelope.schema_hash(),
            tick: envelope.tick(),
            wsc_digest: *envelope.wsc_digest(),
            encoded_len,
            marker_digest,
        })
    }

    /// Returns the envelope id committed by this marker.
    #[must_use]
    pub const fn envelope_id(&self) -> WscStoreEnvelopeId {
        self.envelope_id
    }

    /// Returns the marker digest.
    #[must_use]
    pub const fn marker_digest(&self) -> Hash {
        self.marker_digest
    }

    fn write_receipt(self) -> WscStoreWriteReceipt {
        WscStoreWriteReceipt {
            envelope_id: self.envelope_id,
            commit_marker_digest: self.marker_digest,
            wsc_digest: self.wsc_digest,
            encoded_len: self.encoded_len,
        }
    }

    fn matches_envelope(self, envelope: &WscStoreEnvelope) -> bool {
        self.envelope_id == envelope.id()
            && self.record_kind == envelope.record_kind()
            && self.basis_digest == *envelope.basis_digest()
            && self.schema_hash == *envelope.schema_hash()
            && self.tick == envelope.tick()
            && self.wsc_digest == *envelope.wsc_digest()
            && self.encoded_len == u64::try_from(envelope.encode().len()).unwrap_or(u64::MAX)
            && self.marker_digest == derive_commit_marker_digest(envelope, self.encoded_len)
    }
}

/// Receipt and correlation records recovered from WSC material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscReceiptCorrelationRecords {
    /// Tick receipt records with decision posture.
    pub receipts: Vec<TickReceiptRecord>,
    /// Ticket/submission/receipt correlation records.
    pub correlations: Vec<WalReceiptCorrelationRecord>,
}

/// Retained material and reading records recovered from WSC material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscRetentionRecords {
    /// Retained material references with evidence posture.
    pub materials: Vec<RetainedMaterialRecord>,
    /// Retained reading references with semantic coordinates.
    pub readings: Vec<ReadingRefRecord>,
}

/// Topology records recovered from WSC material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscTopologyRecords {
    /// Strand fork evidence records.
    pub strand_forks: Vec<StrandForkRecord>,
    /// Strand drop evidence records.
    pub strand_drops: Vec<StrandDropRecord>,
    /// Braid lifecycle event records.
    pub braid_events: Vec<TopologyBraidEventRecord>,
    /// Retained braid shell records.
    pub braid_shells: Vec<BraidShellRetentionRecord>,
    /// Witnessed suffix import records.
    pub suffix_imports: Vec<SuffixImportRecord>,
}

impl WscTopologyRecords {
    /// Returns all topology records in deterministic typed order.
    #[must_use]
    pub fn into_topology_records(self) -> Vec<TopologyIntentRecord> {
        let mut records = Vec::new();
        records.extend(
            self.strand_forks
                .into_iter()
                .map(TopologyIntentRecord::StrandFork),
        );
        records.extend(
            self.strand_drops
                .into_iter()
                .map(TopologyIntentRecord::StrandDrop),
        );
        records.extend(
            self.braid_events
                .into_iter()
                .map(TopologyIntentRecord::BraidEvent),
        );
        records.extend(
            self.braid_shells
                .into_iter()
                .map(TopologyIntentRecord::BraidShell),
        );
        records.extend(
            self.suffix_imports
                .into_iter()
                .map(TopologyIntentRecord::SuffixImport),
        );
        records
    }
}

/// Builds a ref-only WAL causal-history WSC export.
///
/// The projection envelope carries read-only WAL graph facts. Segment bytes are
/// not embedded; each segment is reported as an explicit external dependency.
/// Locator strings remain outside the projection identity.
///
/// # Errors
///
/// Returns a typed export error when generated WSC material cannot be written or
/// one of the generated envelopes fails validation.
pub fn wsc_ref_only_wal_export(
    root: &WalRoot,
    accepted_submissions: &[SubmissionAcceptanceRecord],
    receipts: &[TickReceiptRecord],
    correlations: &[WalReceiptCorrelationRecord],
) -> Result<WscRefOnlyWalExport, WscRefOnlyWalExportError> {
    Ok(WscRefOnlyWalExport {
        profile: WscCausalHistoryExportProfileKind::RefOnly,
        projection_envelope: wsc_ref_only_wal_projection_envelope(root)?,
        accepted_submission_envelope: accepted_submission_records_to_wsc_envelope(
            accepted_submissions,
        )
        .map_err(WscRefOnlyWalExportError::Envelope)?,
        receipt_correlation_envelope: receipt_correlation_records_to_wsc_envelope(
            receipts,
            correlations,
        )
        .map_err(WscRefOnlyWalExportError::Envelope)?,
        segment_dependencies: wsc_ref_only_wal_segment_dependencies(root)?,
    })
}

/// Validates and imports a ref-only WAL causal-history WSC export.
///
/// Projection validation compares the imported WSC graph with the graph
/// deterministically rebuilt from `expected_root`. This validates projected
/// identities, segment digest, LSN range, and commit-anchor facts without
/// promoting locators or external segment bytes to causal authority.
///
/// # Errors
///
/// Returns a typed import error when WSC material is malformed, does not match
/// the recovered root, or lacks required accepted-submission/receipt partners.
pub fn validate_wsc_ref_only_wal_export(
    export: &WscRefOnlyWalExport,
    expected_root: &WalRoot,
) -> Result<WscRefOnlyWalImport, WscRefOnlyWalImportError> {
    if export.profile != WscCausalHistoryExportProfileKind::RefOnly {
        return Err(WscRefOnlyWalImportError::ProfileMismatch {
            actual: export.profile,
        });
    }
    if export.projection_envelope.record_kind() != WscStoreRecordKind::CausalHistory {
        return Err(WscRefOnlyWalImportError::InvalidProjectionEnvelopeKind);
    }

    let projection = observe_wal_projection_graph_wsc(export.projection_envelope.wsc_bytes())
        .map_err(WscRefOnlyWalImportError::ProjectionObservation)?;
    let expected_projection = wsc_ref_only_wal_projection_envelope(expected_root)
        .map_err(WscRefOnlyWalImportError::ExpectedProjection)?;
    let expected_root_identity = expected_root.identity_digest();
    if export.projection_envelope.basis_digest() != &expected_root_identity {
        return Err(WscRefOnlyWalImportError::ProjectionBasisMismatch {
            expected: expected_root_identity,
            actual: *export.projection_envelope.basis_digest(),
        });
    }
    if export.projection_envelope.wsc_bytes() != expected_projection.wsc_bytes() {
        return Err(WscRefOnlyWalImportError::ProjectionPayloadMismatch {
            expected: *expected_projection.wsc_digest(),
            actual: *export.projection_envelope.wsc_digest(),
        });
    }

    let expected_dependencies = wsc_ref_only_wal_segment_dependencies(expected_root)
        .map_err(WscRefOnlyWalImportError::ExpectedProjection)?;
    if export.segment_dependencies != expected_dependencies {
        return Err(WscRefOnlyWalImportError::SegmentDependencyMismatch);
    }

    let accepted_submissions =
        accepted_submission_records_from_wsc_envelope(&export.accepted_submission_envelope)
            .map_err(WscRefOnlyWalImportError::AcceptedSubmissions)?;
    let receipt_records =
        receipt_correlation_records_from_wsc_envelope(&export.receipt_correlation_envelope)
            .map_err(WscRefOnlyWalImportError::ReceiptCorrelations)?;
    validate_wsc_causal_history_records(
        &accepted_submissions,
        &receipt_records.receipts,
        &receipt_records.correlations,
    )
    .map_err(WscRefOnlyWalImportError::IncompleteCausalHistory)?;

    Ok(WscRefOnlyWalImport {
        profile: export.profile,
        projection,
        root_identity_digest: expected_root_identity,
        accepted_submissions,
        receipts: receipt_records.receipts,
        correlations: receipt_records.correlations,
        segment_dependencies: expected_dependencies,
    })
}

/// Generic WSC store port.
pub trait WscStorePort {
    /// Writes a validated WSC envelope.
    fn write_envelope(
        &mut self,
        envelope: WscStoreEnvelope,
    ) -> Result<WscStoreWriteReceipt, WscStoreObstruction>;

    /// Reads a WSC envelope by id.
    fn read_envelope(
        &self,
        envelope_id: WscStoreEnvelopeId,
    ) -> Result<WscStoreEnvelope, WscStoreObstruction>;

    /// Lists known envelope ids in deterministic order.
    fn list_envelopes(&self) -> Vec<WscStoreEnvelopeId>;
}

/// In-memory WSC store implementation for tests and adapters.
#[derive(Debug, Default)]
pub struct InMemoryWscStore {
    staged_envelopes: BTreeMap<WscStoreEnvelopeId, WscStoreEnvelope>,
    commit_markers: BTreeMap<WscStoreEnvelopeId, WscStoreCommitMarker>,
}

impl InMemoryWscStore {
    /// Stages an envelope without publishing its commit marker.
    ///
    /// This models the pre-commit phase of an atomic write. Callers that read
    /// through [`WscStorePort`] will not observe the staged envelope until
    /// [`Self::commit_staged_envelope`] publishes the matching marker.
    ///
    /// # Errors
    ///
    /// Returns a typed obstruction when the same envelope id already maps to
    /// different staged material.
    pub fn stage_envelope_without_commit_marker(
        &mut self,
        envelope: WscStoreEnvelope,
    ) -> Result<WscStoreEnvelopeId, WscStoreObstruction> {
        let envelope_id = envelope.id();
        if let Some(existing) = self.staged_envelopes.get(&envelope_id) {
            if existing != &envelope {
                return Err(WscStoreObstruction::duplicate_mismatch(envelope_id));
            }
        }
        if let Some(marker) = self.commit_markers.get(&envelope_id) {
            if !marker.matches_envelope(&envelope) {
                return Err(WscStoreObstruction::commit_marker_mismatch(envelope_id));
            }
        }
        self.staged_envelopes.insert(envelope_id, envelope);
        Ok(envelope_id)
    }

    /// Publishes the commit marker for a staged envelope.
    ///
    /// # Errors
    ///
    /// Returns a typed obstruction when the staged envelope is absent or when
    /// an existing marker does not match the staged material.
    pub fn commit_staged_envelope(
        &mut self,
        envelope_id: WscStoreEnvelopeId,
    ) -> Result<WscStoreWriteReceipt, WscStoreObstruction> {
        let envelope = self
            .staged_envelopes
            .get(&envelope_id)
            .ok_or_else(|| WscStoreObstruction::incomplete_write(envelope_id))?;
        let marker = WscStoreCommitMarker::from_envelope(envelope)?;
        if let Some(existing) = self.commit_markers.get(&envelope_id) {
            if existing != &marker {
                return Err(WscStoreObstruction::commit_marker_mismatch(envelope_id));
            }
            return Ok(existing.write_receipt());
        }
        self.commit_markers.insert(envelope_id, marker);
        Ok(marker.write_receipt())
    }
}

impl WscStorePort for InMemoryWscStore {
    fn write_envelope(
        &mut self,
        envelope: WscStoreEnvelope,
    ) -> Result<WscStoreWriteReceipt, WscStoreObstruction> {
        let envelope_id = envelope.id();
        self.stage_envelope_without_commit_marker(envelope)?;
        self.commit_staged_envelope(envelope_id)
    }

    fn read_envelope(
        &self,
        envelope_id: WscStoreEnvelopeId,
    ) -> Result<WscStoreEnvelope, WscStoreObstruction> {
        match (
            self.staged_envelopes.get(&envelope_id),
            self.commit_markers.get(&envelope_id),
        ) {
            (Some(envelope), Some(marker)) if marker.matches_envelope(envelope) => {
                Ok(envelope.clone())
            }
            (Some(_), Some(_)) => Err(WscStoreObstruction::commit_marker_mismatch(envelope_id)),
            (Some(_), None) | (None, Some(_)) => {
                Err(WscStoreObstruction::incomplete_write(envelope_id))
            }
            (None, None) => Err(WscStoreObstruction::missing_envelope(envelope_id)),
        }
    }

    fn list_envelopes(&self) -> Vec<WscStoreEnvelopeId> {
        self.commit_markers
            .iter()
            .filter_map(|(envelope_id, marker)| {
                self.staged_envelopes
                    .get(envelope_id)
                    .filter(|envelope| marker.matches_envelope(envelope))
                    .map(|_| *envelope_id)
            })
            .collect()
    }
}

/// Builds a generic WSC envelope for accepted submission records.
///
/// Duplicate identical records are represented once. A duplicate submission id
/// with different material is a typed obstruction.
///
/// # Errors
///
/// Returns [`WscStoreObstructionKind::DuplicateEnvelopeMismatch`] for
/// conflicting duplicate submission ids or [`WscStoreObstructionKind::InvalidWsc`]
/// when generated WSC material fails validation.
pub fn accepted_submission_records_to_wsc_envelope(
    records: &[SubmissionAcceptanceRecord],
) -> Result<WscStoreEnvelope, WscStoreObstruction> {
    let records = canonical_accepted_submission_records(records)?;
    let mut store = GraphStore::new(make_warp_id(WSC_ACCEPTED_SUBMISSION_WARP));
    let root = make_node_id(WSC_ACCEPTED_SUBMISSION_ROOT);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id(WSC_ACCEPTED_SUBMISSION_NODE_TYPE),
        },
    );
    for record in &records {
        let node = accepted_submission_node_id(&record.submission_id);
        store.insert_node(
            node,
            NodeRecord {
                ty: make_type_id(WSC_ACCEPTED_SUBMISSION_NODE_TYPE),
            },
        );
        store.insert_edge(
            root,
            EdgeRecord {
                id: accepted_submission_edge_id(&record.submission_id),
                from: root,
                to: node,
                ty: make_type_id(WSC_ACCEPTED_SUBMISSION_EDGE_TYPE),
            },
        );
        store.set_node_attachment(
            node,
            Some(AttachmentValue::Atom(AtomPayload::new(
                make_type_id(WSC_ACCEPTED_SUBMISSION_ATTACHMENT_TYPE),
                Bytes::from(record.to_payload_bytes()),
            ))),
        );
    }
    let basis_digest = accepted_submission_basis_digest(&records);
    let input = build_one_warp_input(&store, root);
    let wsc_bytes = write_wsc_one_warp(&input, make_type_id(WSC_ACCEPTED_SUBMISSION_SCHEMA).0, 0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(basis_digest))?;
    WscStoreEnvelope::validated(WscStoreRecordKind::CausalHistory, basis_digest, wsc_bytes)
}

/// Recovers accepted submission records from a generic WSC envelope.
///
/// # Errors
///
/// Returns a typed WSC store obstruction when the envelope is not accepted
/// submission causal-history material, when record payloads are malformed, or
/// when the envelope basis digest does not match recovered canonical records.
pub fn accepted_submission_records_from_wsc_envelope(
    envelope: &WscStoreEnvelope,
) -> Result<Vec<SubmissionAcceptanceRecord>, WscStoreObstruction> {
    if envelope.record_kind() != WscStoreRecordKind::CausalHistory {
        return Err(WscStoreObstruction::invalid_envelope(0));
    }
    let wsc_digest = *envelope.wsc_digest();
    let file = WscFile::from_bytes(envelope.wsc_bytes().to_vec())
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    if file.schema_hash() != &make_type_id(WSC_ACCEPTED_SUBMISSION_SCHEMA).0 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let view = file
        .warp_view(0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let mut records = Vec::new();
    for node_index in 0..view.nodes().len() {
        for attachment in view.node_attachments(node_index) {
            if attachment.type_or_warp != make_type_id(WSC_ACCEPTED_SUBMISSION_ATTACHMENT_TYPE).0 {
                return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
            }
            let payload = atom_payload_bytes(&view, attachment, wsc_digest)?;
            let record = SubmissionAcceptanceRecord::from_payload_bytes(payload)
                .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
            records.push(record);
        }
    }
    let records = canonical_accepted_submission_records(&records)?;
    let basis_digest = accepted_submission_basis_digest(&records);
    if envelope.basis_digest() != &basis_digest {
        return Err(WscStoreObstruction::basis_digest_mismatch(
            *envelope.basis_digest(),
            basis_digest,
        ));
    }
    Ok(records)
}

/// Recovers accepted submission records from committed WSC store envelopes.
///
/// Incomplete staged writes are not visible through [`WscStorePort::list_envelopes`],
/// and any incomplete envelope read returns a typed obstruction.
///
/// # Errors
///
/// Returns a typed WSC store obstruction when a committed accepted-submission
/// envelope is malformed, basis-mismatched, or conflicting duplicate submission
/// material is found.
pub fn accepted_submission_records_from_wsc_store<P>(
    store: &P,
) -> Result<Vec<SubmissionAcceptanceRecord>, WscStoreObstruction>
where
    P: WscStorePort + ?Sized,
{
    let mut records = Vec::new();
    for envelope_id in store.list_envelopes() {
        let envelope = store.read_envelope(envelope_id)?;
        if envelope.record_kind() != WscStoreRecordKind::CausalHistory
            || !envelope_has_schema(&envelope, WSC_ACCEPTED_SUBMISSION_SCHEMA)?
        {
            continue;
        }
        records.extend(accepted_submission_records_from_wsc_envelope(&envelope)?);
    }
    canonical_accepted_submission_records(&records)
}

/// Builds a generic WSC envelope for receipt and ticket correlation records.
///
/// # Errors
///
/// Returns a typed obstruction when generated WSC material fails validation or
/// when duplicate receipt/correlation keys map to conflicting material.
pub fn receipt_correlation_records_to_wsc_envelope(
    receipts: &[TickReceiptRecord],
    correlations: &[WalReceiptCorrelationRecord],
) -> Result<WscStoreEnvelope, WscStoreObstruction> {
    let receipts = canonical_tick_receipts(receipts)?;
    let correlations = canonical_receipt_correlations(correlations)?;
    let mut store = GraphStore::new(make_warp_id(WSC_RECEIPT_CORRELATION_WARP));
    let root = make_node_id(WSC_RECEIPT_CORRELATION_ROOT);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id(WSC_RECEIPT_CORRELATION_NODE_TYPE),
        },
    );
    for receipt in &receipts {
        insert_receipt_material_node(
            &mut store,
            root,
            receipt_node_id(&receipt.receipt_digest),
            WSC_TICK_RECEIPT_ATTACHMENT_TYPE,
            receipt.to_payload_bytes(),
        );
    }
    for correlation in &correlations {
        insert_receipt_material_node(
            &mut store,
            root,
            correlation_node_id(&correlation.submission_id, &correlation.ticket_digest),
            WSC_RECEIPT_CORRELATION_ATTACHMENT_TYPE,
            correlation.to_payload_bytes(),
        );
    }
    let basis_digest = receipt_correlation_basis_digest(&receipts, &correlations);
    let input = build_one_warp_input(&store, root);
    let wsc_bytes = write_wsc_one_warp(&input, make_type_id(WSC_RECEIPT_CORRELATION_SCHEMA).0, 0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(basis_digest))?;
    WscStoreEnvelope::validated(WscStoreRecordKind::CausalHistory, basis_digest, wsc_bytes)
}

/// Recovers receipt and ticket correlation records from a generic WSC envelope.
///
/// # Errors
///
/// Returns a typed WSC store obstruction when the envelope is not receipt
/// correlation material, when record payloads are malformed, or when the
/// envelope basis digest does not match recovered canonical records.
pub fn receipt_correlation_records_from_wsc_envelope(
    envelope: &WscStoreEnvelope,
) -> Result<WscReceiptCorrelationRecords, WscStoreObstruction> {
    if envelope.record_kind() != WscStoreRecordKind::CausalHistory {
        return Err(WscStoreObstruction::invalid_envelope(0));
    }
    let wsc_digest = *envelope.wsc_digest();
    let file = WscFile::from_bytes(envelope.wsc_bytes().to_vec())
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    if file.schema_hash() != &make_type_id(WSC_RECEIPT_CORRELATION_SCHEMA).0 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let view = file
        .warp_view(0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let mut receipts = Vec::new();
    let mut correlations = Vec::new();
    for node_index in 0..view.nodes().len() {
        for attachment in view.node_attachments(node_index) {
            let payload = atom_payload_bytes(&view, attachment, wsc_digest)?;
            if attachment.type_or_warp == make_type_id(WSC_TICK_RECEIPT_ATTACHMENT_TYPE).0 {
                let receipt = TickReceiptRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
                receipts.push(receipt);
            } else if attachment.type_or_warp
                == make_type_id(WSC_RECEIPT_CORRELATION_ATTACHMENT_TYPE).0
            {
                let correlation = WalReceiptCorrelationRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
                correlations.push(correlation);
            } else {
                return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
            }
        }
    }
    let receipts = canonical_tick_receipts(&receipts)?;
    let correlations = canonical_receipt_correlations(&correlations)?;
    let basis_digest = receipt_correlation_basis_digest(&receipts, &correlations);
    if envelope.basis_digest() != &basis_digest {
        return Err(WscStoreObstruction::basis_digest_mismatch(
            *envelope.basis_digest(),
            basis_digest,
        ));
    }
    Ok(WscReceiptCorrelationRecords {
        receipts,
        correlations,
    })
}

/// Recovers receipt and ticket correlation records from committed WSC store envelopes.
///
/// # Errors
///
/// Returns a typed WSC store obstruction when a committed receipt-correlation
/// envelope is malformed or basis-mismatched.
pub fn receipt_correlation_records_from_wsc_store<P>(
    store: &P,
) -> Result<WscReceiptCorrelationRecords, WscStoreObstruction>
where
    P: WscStorePort + ?Sized,
{
    let mut receipts = Vec::new();
    let mut correlations = Vec::new();
    for envelope_id in store.list_envelopes() {
        let envelope = store.read_envelope(envelope_id)?;
        if envelope.record_kind() != WscStoreRecordKind::CausalHistory
            || !envelope_has_schema(&envelope, WSC_RECEIPT_CORRELATION_SCHEMA)?
        {
            continue;
        }
        let recovered = receipt_correlation_records_from_wsc_envelope(&envelope)?;
        receipts.extend(recovered.receipts);
        correlations.extend(recovered.correlations);
    }
    Ok(WscReceiptCorrelationRecords {
        receipts: canonical_tick_receipts(&receipts)?,
        correlations: canonical_receipt_correlations(&correlations)?,
    })
}

/// Validates committed WSC causal-history records for required partner material.
///
/// Accepted submissions may remain pending without receipts. Receipt records
/// and receipt-correlation records, however, require a committed accepted
/// submission and a matching receipt/correlation pair.
///
/// # Errors
///
/// Returns [`WscStoreObstructionKind::IncompleteCausalHistory`] when committed
/// records reference missing partner material.
pub fn validate_wsc_causal_history_store<P>(store: &P) -> Result<(), WscStoreObstruction>
where
    P: WscStorePort + ?Sized,
{
    let acceptances = accepted_submission_records_from_wsc_store(store)?;
    let receipt_records = receipt_correlation_records_from_wsc_store(store)?;
    validate_wsc_causal_history_records(
        &acceptances,
        &receipt_records.receipts,
        &receipt_records.correlations,
    )
}

/// Builds a generic WSC envelope for retained material and reading records.
///
/// Duplicate identical records are represented once.
///
/// # Errors
///
/// Returns a typed obstruction when retained evidence identities conflict or
/// generated WSC material fails validation.
pub fn retention_records_to_wsc_envelope(
    materials: &[RetainedMaterialRecord],
    readings: &[ReadingRefRecord],
) -> Result<WscStoreEnvelope, WscStoreObstruction> {
    let materials = canonical_retained_material_records(materials)?;
    let readings = canonical_reading_ref_records(readings)?;
    let mut store = GraphStore::new(make_warp_id(WSC_RETENTION_WARP));
    let root = make_node_id(WSC_RETENTION_ROOT);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id(WSC_RETENTION_NODE_TYPE),
        },
    );
    for material in &materials {
        insert_retention_record_node(
            &mut store,
            root,
            retention_node_id(b"material", &material.to_payload_bytes()),
            WSC_RETAINED_MATERIAL_ATTACHMENT_TYPE,
            material.to_payload_bytes(),
        );
    }
    for reading in &readings {
        insert_retention_record_node(
            &mut store,
            root,
            retention_node_id(b"reading", &reading.to_payload_bytes()),
            WSC_READING_REF_ATTACHMENT_TYPE,
            reading.to_payload_bytes(),
        );
    }
    let basis_digest = retention_basis_digest(&materials, &readings);
    let input = build_one_warp_input(&store, root);
    let wsc_bytes = write_wsc_one_warp(&input, make_type_id(WSC_RETENTION_SCHEMA).0, 0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(basis_digest))?;
    WscStoreEnvelope::validated(
        WscStoreRecordKind::RetainedEvidence,
        basis_digest,
        wsc_bytes,
    )
}

/// Recovers retained material and reading records from a generic WSC envelope.
///
/// # Errors
///
/// Returns a typed WSC store obstruction when the envelope is not retained
/// evidence material, when record payloads are malformed, or when retained
/// evidence identities conflict, or when the envelope basis digest does not
/// match recovered canonical records.
pub fn retention_records_from_wsc_envelope(
    envelope: &WscStoreEnvelope,
) -> Result<WscRetentionRecords, WscStoreObstruction> {
    if envelope.record_kind() != WscStoreRecordKind::RetainedEvidence {
        return Err(WscStoreObstruction::invalid_envelope(0));
    }
    let wsc_digest = *envelope.wsc_digest();
    let file = WscFile::from_bytes(envelope.wsc_bytes().to_vec())
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    if file.schema_hash() != &make_type_id(WSC_RETENTION_SCHEMA).0 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let view = file
        .warp_view(0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let mut materials = Vec::new();
    let mut readings = Vec::new();
    for node_index in 0..view.nodes().len() {
        for attachment in view.node_attachments(node_index) {
            let payload = atom_payload_bytes(&view, attachment, wsc_digest)?;
            if attachment.type_or_warp == make_type_id(WSC_RETAINED_MATERIAL_ATTACHMENT_TYPE).0 {
                let material = RetainedMaterialRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
                materials.push(material);
            } else if attachment.type_or_warp == make_type_id(WSC_READING_REF_ATTACHMENT_TYPE).0 {
                let reading = ReadingRefRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
                readings.push(reading);
            } else {
                return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
            }
        }
    }
    let materials = canonical_retained_material_records(&materials)?;
    let readings = canonical_reading_ref_records(&readings)?;
    let basis_digest = retention_basis_digest(&materials, &readings);
    if envelope.basis_digest() != &basis_digest {
        return Err(WscStoreObstruction::basis_digest_mismatch(
            *envelope.basis_digest(),
            basis_digest,
        ));
    }
    Ok(WscRetentionRecords {
        materials,
        readings,
    })
}

/// Recovers retained material and reading records from committed WSC store envelopes.
///
/// # Errors
///
/// Returns a typed WSC store obstruction when a committed retention envelope is
/// malformed, basis-mismatched, or when retained evidence identities conflict.
pub fn retention_records_from_wsc_store<P>(
    store: &P,
) -> Result<WscRetentionRecords, WscStoreObstruction>
where
    P: WscStorePort + ?Sized,
{
    let mut materials = Vec::new();
    let mut readings = Vec::new();
    for envelope_id in store.list_envelopes() {
        let envelope = store.read_envelope(envelope_id)?;
        if envelope.record_kind() != WscStoreRecordKind::RetainedEvidence
            || !envelope_has_schema(&envelope, WSC_RETENTION_SCHEMA)?
        {
            continue;
        }
        let recovered = retention_records_from_wsc_envelope(&envelope)?;
        materials.extend(recovered.materials);
        readings.extend(recovered.readings);
    }
    Ok(WscRetentionRecords {
        materials: canonical_retained_material_records(&materials)?,
        readings: canonical_reading_ref_records(&readings)?,
    })
}

/// Builds a generic WSC envelope for topology evidence records.
///
/// Duplicate identical records are represented once. Divergent duplicate
/// topology identities return a typed obstruction.
pub fn topology_records_to_wsc_envelope(
    records: &[TopologyIntentRecord],
) -> Result<WscStoreEnvelope, WscStoreObstruction> {
    let records = canonical_topology_records(records)?;
    let mut store = GraphStore::new(make_warp_id(WSC_TOPOLOGY_WARP));
    let root = make_node_id(WSC_TOPOLOGY_ROOT);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id(WSC_TOPOLOGY_NODE_TYPE),
        },
    );
    for record in &records {
        let (role, attachment_type, payload_bytes) = topology_record_payload(record);
        insert_topology_record_node(
            &mut store,
            root,
            topology_node_id(role, &payload_bytes),
            attachment_type,
            payload_bytes,
        );
    }
    let basis_digest = topology_basis_digest(&records);
    let input = build_one_warp_input(&store, root);
    let wsc_bytes = write_wsc_one_warp(&input, make_type_id(WSC_TOPOLOGY_SCHEMA).0, 0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(basis_digest))?;
    WscStoreEnvelope::validated(WscStoreRecordKind::CausalHistory, basis_digest, wsc_bytes)
}

/// Recovers topology records from a generic WSC envelope.
pub fn topology_records_from_wsc_envelope(
    envelope: &WscStoreEnvelope,
) -> Result<WscTopologyRecords, WscStoreObstruction> {
    if envelope.record_kind() != WscStoreRecordKind::CausalHistory {
        return Err(WscStoreObstruction::invalid_envelope(0));
    }
    let wsc_digest = *envelope.wsc_digest();
    let file = WscFile::from_bytes(envelope.wsc_bytes().to_vec())
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    if file.schema_hash() != &make_type_id(WSC_TOPOLOGY_SCHEMA).0 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let view = file
        .warp_view(0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let records = topology_records_from_wsc_view(&view, wsc_digest)?;
    let records = canonical_topology_records(&records)?;
    let basis_digest = topology_basis_digest(&records);
    if envelope.basis_digest() != &basis_digest {
        return Err(WscStoreObstruction::basis_digest_mismatch(
            *envelope.basis_digest(),
            basis_digest,
        ));
    }
    Ok(split_topology_records(records))
}

fn topology_records_from_wsc_view(
    view: &super::view::WarpView<'_>,
    wsc_digest: Hash,
) -> Result<Vec<TopologyIntentRecord>, WscStoreObstruction> {
    let expected_warp = make_warp_id(WSC_TOPOLOGY_WARP).0;
    let root = make_node_id(WSC_TOPOLOGY_ROOT);
    let node_type = make_type_id(WSC_TOPOLOGY_NODE_TYPE).0;
    let edge_type = make_type_id(WSC_TOPOLOGY_EDGE_TYPE).0;

    if view.warp_id() != &expected_warp || view.root_node_id() != &root.0 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let Some(root_ix) = view.node_ix(&root.0) else {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    };
    if view.nodes()[root_ix].node_type != node_type || !view.node_attachments(root_ix).is_empty() {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }

    let mut records = Vec::new();
    let mut record_node_ids = BTreeSet::new();
    for (node_ix, node) in view.nodes().iter().enumerate() {
        if node.node_type != node_type {
            return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
        }
        if node.node_id == root.0 {
            continue;
        }
        let attachments = view.node_attachments(node_ix);
        if attachments.len() != 1 {
            return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
        }
        let attachment = &attachments[0];
        let payload = atom_payload_bytes(view, attachment, wsc_digest)?;
        let (role, record) =
            topology_record_from_attachment(attachment.type_or_warp, payload, wsc_digest)?;
        if node.node_id != topology_node_id(role, payload).0
            || !record_node_ids.insert(node.node_id)
        {
            return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
        }
        records.push(record);
    }

    let mut edge_targets = BTreeSet::new();
    for (edge_ix, edge) in view.edges().iter().enumerate() {
        if !view.edge_attachments(edge_ix).is_empty()
            || edge.edge_type != edge_type
            || edge.from_node_id != root.0
            || edge.edge_id != topology_edge_id(&edge.to_node_id).0
            || !record_node_ids.contains(&edge.to_node_id)
            || !edge_targets.insert(edge.to_node_id)
        {
            return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
        }
    }
    if edge_targets != record_node_ids {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }

    Ok(records)
}

fn topology_record_from_attachment(
    attachment_type: Hash,
    payload: &[u8],
    wsc_digest: Hash,
) -> Result<(&'static [u8], TopologyIntentRecord), WscStoreObstruction> {
    if attachment_type == make_type_id(WSC_TOPOLOGY_STRAND_FORK_ATTACHMENT_TYPE).0 {
        Ok((
            b"strand-fork",
            TopologyIntentRecord::StrandFork(
                StrandForkRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?,
            ),
        ))
    } else if attachment_type == make_type_id(WSC_TOPOLOGY_STRAND_DROP_ATTACHMENT_TYPE).0 {
        Ok((
            b"strand-drop",
            TopologyIntentRecord::StrandDrop(
                StrandDropRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?,
            ),
        ))
    } else if attachment_type == make_type_id(WSC_TOPOLOGY_BRAID_EVENT_ATTACHMENT_TYPE).0 {
        Ok((
            b"braid-event",
            TopologyIntentRecord::BraidEvent(
                TopologyBraidEventRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?,
            ),
        ))
    } else if attachment_type == make_type_id(WSC_TOPOLOGY_BRAID_SHELL_ATTACHMENT_TYPE).0 {
        Ok((
            b"braid-shell",
            TopologyIntentRecord::BraidShell(
                BraidShellRetentionRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?,
            ),
        ))
    } else if attachment_type == make_type_id(WSC_TOPOLOGY_SUFFIX_IMPORT_ATTACHMENT_TYPE).0 {
        Ok((
            b"suffix-import",
            TopologyIntentRecord::SuffixImport(
                SuffixImportRecord::from_payload_bytes(payload)
                    .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?,
            ),
        ))
    } else {
        Err(WscStoreObstruction::invalid_wsc(wsc_digest))
    }
}

/// Recovers topology records from committed WSC store envelopes.
pub fn topology_records_from_wsc_store<P>(
    store: &P,
) -> Result<WscTopologyRecords, WscStoreObstruction>
where
    P: WscStorePort + ?Sized,
{
    let mut records = Vec::new();
    for envelope_id in store.list_envelopes() {
        let envelope = store.read_envelope(envelope_id)?;
        if envelope.record_kind() != WscStoreRecordKind::CausalHistory
            || !envelope_has_schema(&envelope, WSC_TOPOLOGY_SCHEMA)?
        {
            continue;
        }
        records.extend(topology_records_from_wsc_envelope(&envelope)?.into_topology_records());
    }
    Ok(split_topology_records(canonical_topology_records(
        &records,
    )?))
}

fn canonical_topology_records(
    records: &[TopologyIntentRecord],
) -> Result<Vec<TopologyIntentRecord>, WscStoreObstruction> {
    let mut by_payload = BTreeMap::new();
    let mut strand_forks = BTreeMap::new();
    let mut strand_forks_by_idempotency = BTreeMap::new();
    let mut strand_drops = BTreeMap::new();
    let mut strand_drops_by_idempotency = BTreeMap::new();
    let mut braid_events = BTreeMap::new();
    let mut braid_events_by_idempotency = BTreeMap::new();
    let mut braid_shells = BTreeMap::new();
    let mut braid_shells_by_idempotency = BTreeMap::new();
    let mut suffix_imports = BTreeMap::new();
    let mut suffix_imports_by_idempotency = BTreeMap::new();
    let mut suffix_imports_by_bundle = BTreeMap::new();

    for record in records {
        let record = canonical_topology_record(record);
        match &record {
            TopologyIntentRecord::StrandFork(record) => {
                insert_wsc_unique(
                    &mut strand_forks,
                    *record.strand_id.as_bytes(),
                    record.clone(),
                    record.strand_id.as_bytes(),
                )?;
                if let Some(idempotency_key_digest) = record.idempotency_key_digest {
                    insert_wsc_unique(
                        &mut strand_forks_by_idempotency,
                        idempotency_key_digest,
                        record.clone(),
                        &idempotency_key_digest,
                    )?;
                }
            }
            TopologyIntentRecord::StrandDrop(record) => {
                insert_wsc_unique(
                    &mut strand_drops,
                    *record.strand_id.as_bytes(),
                    record.clone(),
                    record.strand_id.as_bytes(),
                )?;
                if let Some(idempotency_key_digest) = record.idempotency_key_digest {
                    insert_wsc_unique(
                        &mut strand_drops_by_idempotency,
                        idempotency_key_digest,
                        record.clone(),
                        &idempotency_key_digest,
                    )?;
                }
            }
            TopologyIntentRecord::BraidEvent(record) => {
                insert_wsc_unique(
                    &mut braid_events,
                    (record.braid_id, record.event_index),
                    record.clone(),
                    &record.braid_id,
                )?;
                if let Some(idempotency_key_digest) = record.idempotency_key_digest {
                    insert_wsc_unique(
                        &mut braid_events_by_idempotency,
                        idempotency_key_digest,
                        record.clone(),
                        &idempotency_key_digest,
                    )?;
                }
            }
            TopologyIntentRecord::BraidShell(record) => {
                insert_wsc_unique(
                    &mut braid_shells,
                    record.shell_digest,
                    record.clone(),
                    &record.shell_digest,
                )?;
                if let Some(idempotency_key_digest) = record.idempotency_key_digest {
                    insert_wsc_unique(
                        &mut braid_shells_by_idempotency,
                        idempotency_key_digest,
                        record.clone(),
                        &idempotency_key_digest,
                    )?;
                }
            }
            TopologyIntentRecord::SuffixImport(record) => {
                insert_wsc_unique(
                    &mut suffix_imports,
                    record.import_id,
                    record.clone(),
                    &record.import_id,
                )?;
                insert_wsc_unique(
                    &mut suffix_imports_by_idempotency,
                    record.idempotency_key_digest,
                    record.import_id,
                    &record.idempotency_key_digest,
                )?;
                insert_wsc_unique(
                    &mut suffix_imports_by_bundle,
                    record.bundle_digest,
                    record.import_id,
                    &record.bundle_digest,
                )?;
            }
        }
        by_payload.insert(topology_record_sort_key(&record), record);
    }
    Ok(by_payload.into_values().collect())
}

fn canonical_topology_record(record: &TopologyIntentRecord) -> TopologyIntentRecord {
    match record {
        TopologyIntentRecord::StrandFork(record) => {
            TopologyIntentRecord::StrandFork(record.canonicalized())
        }
        _ => record.clone(),
    }
}

/// Returns [`WscStoreObstructionKind::DuplicateEnvelopeMismatch`] for both
/// committed-store duplicate mismatches and canonical topology payload conflicts.
fn insert_wsc_unique<K, V>(
    map: &mut BTreeMap<K, V>,
    key: K,
    value: V,
    envelope_hash: &Hash,
) -> Result<(), WscStoreObstruction>
where
    K: Ord,
    V: PartialEq,
{
    if let Some(existing) = map.get(&key) {
        if existing != &value {
            return Err(WscStoreObstruction::duplicate_mismatch(
                WscStoreEnvelopeId::from_hash(*envelope_hash),
            ));
        }
        return Ok(());
    }
    map.insert(key, value);
    Ok(())
}

fn split_topology_records(records: Vec<TopologyIntentRecord>) -> WscTopologyRecords {
    let mut topology = WscTopologyRecords {
        strand_forks: Vec::new(),
        strand_drops: Vec::new(),
        braid_events: Vec::new(),
        braid_shells: Vec::new(),
        suffix_imports: Vec::new(),
    };
    for record in records {
        match record {
            TopologyIntentRecord::StrandFork(record) => topology.strand_forks.push(record),
            TopologyIntentRecord::StrandDrop(record) => topology.strand_drops.push(record),
            TopologyIntentRecord::BraidEvent(record) => topology.braid_events.push(record),
            TopologyIntentRecord::BraidShell(record) => topology.braid_shells.push(record),
            TopologyIntentRecord::SuffixImport(record) => topology.suffix_imports.push(record),
        }
    }
    topology
}

fn topology_record_payload(
    record: &TopologyIntentRecord,
) -> (&'static [u8], &'static str, Vec<u8>) {
    match record {
        TopologyIntentRecord::StrandFork(record) => (
            b"strand-fork",
            WSC_TOPOLOGY_STRAND_FORK_ATTACHMENT_TYPE,
            record.to_payload_bytes(),
        ),
        TopologyIntentRecord::StrandDrop(record) => (
            b"strand-drop",
            WSC_TOPOLOGY_STRAND_DROP_ATTACHMENT_TYPE,
            record.to_payload_bytes(),
        ),
        TopologyIntentRecord::BraidEvent(record) => (
            b"braid-event",
            WSC_TOPOLOGY_BRAID_EVENT_ATTACHMENT_TYPE,
            record.to_payload_bytes(),
        ),
        TopologyIntentRecord::BraidShell(record) => (
            b"braid-shell",
            WSC_TOPOLOGY_BRAID_SHELL_ATTACHMENT_TYPE,
            record.to_payload_bytes(),
        ),
        TopologyIntentRecord::SuffixImport(record) => (
            b"suffix-import",
            WSC_TOPOLOGY_SUFFIX_IMPORT_ATTACHMENT_TYPE,
            record.to_payload_bytes(),
        ),
    }
}

fn topology_record_sort_key(record: &TopologyIntentRecord) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(record.record_kind().label().as_bytes());
    key.push(0);
    key.extend_from_slice(&record.to_payload_bytes());
    key
}

fn topology_basis_digest(records: &[TopologyIntentRecord]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_TOPOLOGY_BASIS_DOMAIN);
    for record in records {
        hasher.update(record.record_kind().label().as_bytes());
        hasher.update(&record.to_payload_bytes());
    }
    hasher.finalize().into()
}

fn insert_topology_record_node(
    store: &mut GraphStore,
    root: NodeId,
    node: NodeId,
    attachment_type: &str,
    payload_bytes: Vec<u8>,
) {
    store.insert_node(
        node,
        NodeRecord {
            ty: make_type_id(WSC_TOPOLOGY_NODE_TYPE),
        },
    );
    store.insert_edge(
        root,
        EdgeRecord {
            id: topology_edge_id(&node.0),
            from: root,
            to: node,
            ty: make_type_id(WSC_TOPOLOGY_EDGE_TYPE),
        },
    );
    store.set_node_attachment(
        node,
        Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id(attachment_type),
            Bytes::from(payload_bytes),
        ))),
    );
}

fn topology_node_id(role: &[u8], payload_bytes: &[u8]) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_TOPOLOGY_NODE_DOMAIN);
    hasher.update(role);
    hasher.update(payload_bytes);
    NodeId(hasher.finalize().into())
}

fn topology_edge_id(node_id: &Hash) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_TOPOLOGY_EDGE_DOMAIN);
    hasher.update(node_id);
    EdgeId(hasher.finalize().into())
}

fn wsc_ref_only_wal_projection_envelope(
    root: &WalRoot,
) -> Result<WscStoreEnvelope, WscRefOnlyWalExportError> {
    let graph = materialize_wal_projection_graph(root);
    let input = build_one_warp_input(&graph.store, graph.root_node_id);
    let wsc_bytes = write_wsc_one_warp(&input, wal_projection_graph_schema_hash(), 0)
        .map_err(|_| WscRefOnlyWalExportError::ProjectionWriteFailed)?;
    WscStoreEnvelope::validated(
        WscStoreRecordKind::CausalHistory,
        root.identity_digest(),
        wsc_bytes,
    )
    .map_err(WscRefOnlyWalExportError::Envelope)
}

fn wsc_ref_only_wal_segment_dependencies(
    root: &WalRoot,
) -> Result<Vec<WscRefOnlyWalSegmentDependency>, WscRefOnlyWalExportError> {
    let mut dependencies = root
        .segments
        .iter()
        .map(wsc_ref_only_wal_segment_dependency)
        .collect::<Result<Vec<_>, _>>()?;
    dependencies.sort_by_key(|dependency| {
        (
            dependency.segment_id,
            dependency.segment_identity_digest,
            dependency.segment_digest,
            dependency.first_lsn,
            dependency.last_lsn,
        )
    });
    Ok(dependencies)
}

fn wsc_ref_only_wal_segment_dependency(
    segment: &WalSegmentRef,
) -> Result<WscRefOnlyWalSegmentDependency, WscRefOnlyWalExportError> {
    let mut commit_anchor_digests = segment
        .commit_anchors
        .iter()
        .map(crate::causal_wal::WalCommitAnchor::identity_digest)
        .collect::<Vec<_>>();
    commit_anchor_digests.sort_unstable();
    let Some(locator) = segment.storage_locator.as_ref() else {
        return Err(WscRefOnlyWalExportError::MissingSegmentLocator {
            segment_id: segment.segment_id,
        });
    };
    Ok(WscRefOnlyWalSegmentDependency {
        segment_id: segment.segment_id,
        segment_identity_digest: segment.identity_digest(),
        segment_digest: segment.segment_digest,
        first_lsn: segment.first_lsn,
        last_lsn: segment.last_lsn,
        commit_anchor_digests,
        material_dependency: WscRefOnlyWalMaterialDependency::ExternalSegmentBytes,
        locator_posture: wsc_ref_only_wal_locator_posture(locator),
    })
}

fn wsc_ref_only_wal_locator_posture(
    locator: &WalSegmentStorageLocator,
) -> WscRefOnlyWalLocatorPosture {
    match locator {
        WalSegmentStorageLocator::RelativePath(_) => WscRefOnlyWalLocatorPosture::RelativePath,
        WalSegmentStorageLocator::AbsolutePath(_) => {
            WscRefOnlyWalLocatorPosture::AbsolutePathNormalized
        }
    }
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], WscStoreObstruction> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| WscStoreObstruction::invalid_envelope(offset))?;
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| WscStoreObstruction::invalid_envelope(offset))?;
    let mut out = [0; N];
    out.copy_from_slice(slice);
    Ok(out)
}

fn derive_envelope_id(
    record_kind: WscStoreRecordKind,
    basis_digest: &Hash,
    schema_hash: &Hash,
    tick: u64,
    wsc_digest: &Hash,
    wsc_len: u64,
) -> WscStoreEnvelopeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_STORE_ENVELOPE_ID_DOMAIN);
    hasher.update(&record_kind.code().to_le_bytes());
    hasher.update(basis_digest);
    hasher.update(schema_hash);
    hasher.update(&tick.to_le_bytes());
    hasher.update(wsc_digest);
    hasher.update(&wsc_len.to_le_bytes());
    WscStoreEnvelopeId(hasher.finalize().into())
}

fn derive_commit_marker_digest(envelope: &WscStoreEnvelope, encoded_len: u64) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_STORE_COMMIT_MARKER_DOMAIN);
    hasher.update(&envelope.id().as_hash());
    hasher.update(&envelope.record_kind().code().to_le_bytes());
    hasher.update(envelope.basis_digest());
    hasher.update(envelope.schema_hash());
    hasher.update(&envelope.tick().to_le_bytes());
    hasher.update(envelope.wsc_digest());
    hasher.update(&encoded_len.to_le_bytes());
    hasher.finalize().into()
}

fn digest_wsc_bytes(bytes: &[u8]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_STORE_BYTES_DOMAIN);
    hasher.update(bytes);
    hasher.finalize().into()
}

fn canonical_accepted_submission_records(
    records: &[SubmissionAcceptanceRecord],
) -> Result<Vec<SubmissionAcceptanceRecord>, WscStoreObstruction> {
    let mut by_submission = BTreeMap::new();
    for record in records {
        if let Some(existing) = by_submission.get(&record.submission_id) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.submission_id),
                ));
            }
        }
        by_submission.insert(record.submission_id, *record);
    }
    Ok(by_submission.into_values().collect())
}

fn accepted_submission_basis_digest(records: &[SubmissionAcceptanceRecord]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_ACCEPTED_SUBMISSION_BASIS_DOMAIN);
    for record in records {
        hasher.update(&record.to_payload_bytes());
    }
    hasher.finalize().into()
}

fn accepted_submission_node_id(submission_id: &Hash) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_ACCEPTED_SUBMISSION_NODE_DOMAIN);
    hasher.update(submission_id);
    NodeId(hasher.finalize().into())
}

fn accepted_submission_edge_id(submission_id: &Hash) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_ACCEPTED_SUBMISSION_EDGE_DOMAIN);
    hasher.update(submission_id);
    EdgeId(hasher.finalize().into())
}

fn atom_payload_bytes<'a>(
    view: &'a super::view::WarpView<'a>,
    attachment: &AttRow,
    wsc_digest: Hash,
) -> Result<&'a [u8], WscStoreObstruction> {
    if !attachment.is_atom() {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    view.blob_for_attachment(attachment)
        .ok_or_else(|| WscStoreObstruction::invalid_wsc(wsc_digest))
}

fn envelope_has_schema(
    envelope: &WscStoreEnvelope,
    schema: &str,
) -> Result<bool, WscStoreObstruction> {
    let wsc_digest = *envelope.wsc_digest();
    let file = WscFile::from_bytes(envelope.wsc_bytes().to_vec())
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    Ok(file.schema_hash() == &make_type_id(schema).0)
}

fn canonical_tick_receipts(
    records: &[TickReceiptRecord],
) -> Result<Vec<TickReceiptRecord>, WscStoreObstruction> {
    let mut by_receipt = BTreeMap::new();
    let mut by_submission = BTreeMap::new();
    let mut by_ticket = BTreeMap::new();
    for record in records {
        if let Some(existing) = by_receipt.get(&record.receipt_digest) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.receipt_digest),
                ));
            }
        }
        if let Some(existing) = by_submission.get(&record.submission_id) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.submission_id),
                ));
            }
        }
        if let Some(existing) = by_ticket.get(&record.ticket_digest) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.ticket_digest),
                ));
            }
        }
        by_receipt.insert(record.receipt_digest, *record);
        by_submission.insert(record.submission_id, *record);
        by_ticket.insert(record.ticket_digest, *record);
    }
    Ok(by_receipt.into_values().collect())
}

fn canonical_receipt_correlations(
    records: &[WalReceiptCorrelationRecord],
) -> Result<Vec<WalReceiptCorrelationRecord>, WscStoreObstruction> {
    let mut by_correlation = BTreeMap::new();
    let mut by_submission = BTreeMap::new();
    let mut by_ticket = BTreeMap::new();
    let mut by_receipt = BTreeMap::new();
    for record in records {
        if let Some(existing) = by_submission.get(&record.submission_id) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.submission_id),
                ));
            }
        }
        if let Some(existing) = by_ticket.get(&record.ticket_digest) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.ticket_digest),
                ));
            }
        }
        if let Some(existing) = by_receipt.get(&record.receipt_digest) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.receipt_digest),
                ));
            }
        }
        by_correlation.insert(
            (
                record.submission_id,
                record.ticket_digest,
                record.receipt_digest,
            ),
            *record,
        );
        by_submission.insert(record.submission_id, *record);
        by_ticket.insert(record.ticket_digest, *record);
        by_receipt.insert(record.receipt_digest, *record);
    }
    Ok(by_correlation.into_values().collect())
}

fn validate_wsc_causal_history_records(
    acceptances: &[SubmissionAcceptanceRecord],
    receipts: &[TickReceiptRecord],
    correlations: &[WalReceiptCorrelationRecord],
) -> Result<(), WscStoreObstruction> {
    let accepted_submissions: BTreeSet<Hash> = acceptances
        .iter()
        .map(|record| record.submission_id)
        .collect();
    let receipt_keys: BTreeSet<(Hash, Hash, Hash)> = receipts
        .iter()
        .map(|record| {
            (
                record.submission_id,
                record.ticket_digest,
                record.receipt_digest,
            )
        })
        .collect();
    let correlation_keys: BTreeSet<(Hash, Hash, Hash)> = correlations
        .iter()
        .map(|record| {
            (
                record.submission_id,
                record.ticket_digest,
                record.receipt_digest,
            )
        })
        .collect();
    for receipt in receipts {
        if !accepted_submissions.contains(&receipt.submission_id) {
            return Err(WscStoreObstruction::incomplete_causal_history(
                receipt.receipt_digest,
            ));
        }
        if !correlation_keys.contains(&(
            receipt.submission_id,
            receipt.ticket_digest,
            receipt.receipt_digest,
        )) {
            return Err(WscStoreObstruction::incomplete_causal_history(
                receipt.receipt_digest,
            ));
        }
    }
    for correlation in correlations {
        if !accepted_submissions.contains(&correlation.submission_id) {
            return Err(WscStoreObstruction::incomplete_causal_history(
                correlation.receipt_digest,
            ));
        }
        if !receipt_keys.contains(&(
            correlation.submission_id,
            correlation.ticket_digest,
            correlation.receipt_digest,
        )) {
            return Err(WscStoreObstruction::incomplete_causal_history(
                correlation.receipt_digest,
            ));
        }
    }
    Ok(())
}

fn insert_receipt_material_node(
    store: &mut GraphStore,
    root: NodeId,
    node: NodeId,
    attachment_type: &str,
    payload_bytes: Vec<u8>,
) {
    store.insert_node(
        node,
        NodeRecord {
            ty: make_type_id(WSC_RECEIPT_CORRELATION_NODE_TYPE),
        },
    );
    store.insert_edge(
        root,
        EdgeRecord {
            id: receipt_material_edge_id(&node.0),
            from: root,
            to: node,
            ty: make_type_id(WSC_RECEIPT_CORRELATION_EDGE_TYPE),
        },
    );
    store.set_node_attachment(
        node,
        Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id(attachment_type),
            Bytes::from(payload_bytes),
        ))),
    );
}

fn receipt_correlation_basis_digest(
    receipts: &[TickReceiptRecord],
    correlations: &[WalReceiptCorrelationRecord],
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RECEIPT_CORRELATION_BASIS_DOMAIN);
    for receipt in receipts {
        hasher.update(b"receipt");
        hasher.update(&receipt.to_payload_bytes());
    }
    for correlation in correlations {
        hasher.update(b"correlation");
        hasher.update(&correlation.to_payload_bytes());
    }
    hasher.finalize().into()
}

fn receipt_node_id(receipt_digest: &Hash) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RECEIPT_CORRELATION_NODE_DOMAIN);
    hasher.update(b"receipt");
    hasher.update(receipt_digest);
    NodeId(hasher.finalize().into())
}

fn correlation_node_id(submission_id: &Hash, ticket_digest: &Hash) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RECEIPT_CORRELATION_NODE_DOMAIN);
    hasher.update(b"correlation");
    hasher.update(submission_id);
    hasher.update(ticket_digest);
    NodeId(hasher.finalize().into())
}

fn receipt_material_edge_id(node_id: &Hash) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RECEIPT_CORRELATION_EDGE_DOMAIN);
    hasher.update(node_id);
    EdgeId(hasher.finalize().into())
}

fn canonical_retained_material_records(
    records: &[RetainedMaterialRecord],
) -> Result<Vec<RetainedMaterialRecord>, WscStoreObstruction> {
    let mut by_payload = BTreeMap::new();
    let mut by_material_digest = BTreeMap::new();
    for record in records {
        if let Some(existing) = by_material_digest.get(&record.material_digest) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.material_digest),
                ));
            }
        }
        by_payload.insert(record.to_payload_bytes(), *record);
        by_material_digest.insert(record.material_digest, *record);
    }
    Ok(by_payload.into_values().collect())
}

fn canonical_reading_ref_records(
    records: &[ReadingRefRecord],
) -> Result<Vec<ReadingRefRecord>, WscStoreObstruction> {
    let mut by_payload = BTreeMap::new();
    let mut by_reading_id = BTreeMap::new();
    for record in records {
        if let Some(existing) = by_reading_id.get(&record.reading_id) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.reading_id),
                ));
            }
        }
        by_payload.insert(record.to_payload_bytes(), *record);
        by_reading_id.insert(record.reading_id, *record);
    }
    Ok(by_payload.into_values().collect())
}

fn insert_retention_record_node(
    store: &mut GraphStore,
    root: NodeId,
    node: NodeId,
    attachment_type: &str,
    payload_bytes: Vec<u8>,
) {
    store.insert_node(
        node,
        NodeRecord {
            ty: make_type_id(WSC_RETENTION_NODE_TYPE),
        },
    );
    store.insert_edge(
        root,
        EdgeRecord {
            id: retention_edge_id(&node.0),
            from: root,
            to: node,
            ty: make_type_id(WSC_RETENTION_EDGE_TYPE),
        },
    );
    store.set_node_attachment(
        node,
        Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id(attachment_type),
            Bytes::from(payload_bytes),
        ))),
    );
}

fn retention_basis_digest(
    materials: &[RetainedMaterialRecord],
    readings: &[ReadingRefRecord],
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RETENTION_BASIS_DOMAIN);
    for material in materials {
        hasher.update(b"material");
        hasher.update(&material.to_payload_bytes());
    }
    for reading in readings {
        hasher.update(b"reading");
        hasher.update(&reading.to_payload_bytes());
    }
    hasher.finalize().into()
}

fn retention_node_id(role: &[u8], payload_bytes: &[u8]) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RETENTION_NODE_DOMAIN);
    hasher.update(role);
    hasher.update(payload_bytes);
    NodeId(hasher.finalize().into())
}

fn retention_edge_id(node_id: &Hash) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RETENTION_EDGE_DOMAIN);
    hasher.update(node_id);
    EdgeId(hasher.finalize().into())
}
