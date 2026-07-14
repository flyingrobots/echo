// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generic WSC storage port and deterministic envelope format.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use blake3::Hasher;
use bytes::Bytes;
use thiserror::Error;

use crate::attachment::{AtomPayload, AttachmentValue};
use crate::causal_wal::{
    materialize_wal_projection_graph, observe_wal_projection_graph_wsc, recover_wal_segment_bytes,
    wal_projection_graph_schema_hash, BraidShellRetentionRecord, Lsn, ReadingRefRecord,
    RecoveredReceiptIndex, RecoveredSubmissionIndex, RecoveryAccessMode, RecoveryTailPosture,
    RetainedMaterialKind, RetainedMaterialRecord, StrandDropRecord, StrandForkRecord,
    SubmissionAcceptanceRecord, SuffixImportRecord, TickReceiptRecord, TopologyBraidEventRecord,
    TopologyIntentRecord, WalCommitAnchor, WalProjectionGraphObservation,
    WalProjectionGraphObservationError, WalReceiptCorrelationRecord, WalRecoveryError,
    WalRecoveryIndexError, WalRoot, WalSegmentBytesRecovery, WalSegmentId, WalSegmentRef,
    WalSegmentStorageLocator,
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
const WSC_STORE_FILESYSTEM_PATH_DOMAIN: &[u8] = b"echo:wsc_store:filesystem_path:v1\0";
const WSC_STORE_COMMIT_MARKER_MAGIC: &[u8; 8] = b"ECWSCMK1";
const WSC_STORE_COMMIT_MARKER_VERSION: u16 = 1;
const COMMIT_MARKER_LEN: usize = 188;
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
const WSC_SELF_CONTAINED_WAL_SEGMENT_BASIS_DOMAIN: &[u8] =
    b"echo:wsc_store:self_contained_wal_segment_basis:v1\0";
const WSC_SELF_CONTAINED_WAL_SEGMENT_NODE_DOMAIN: &[u8] =
    b"echo:wsc_store:self_contained_wal_segment_node:v1\0";
const WSC_SELF_CONTAINED_WAL_SEGMENT_EDGE_DOMAIN: &[u8] =
    b"echo:wsc_store:self_contained_wal_segment_edge:v1\0";
const WSC_SELF_CONTAINED_WAL_SEGMENT_SCHEMA: &str = "echo/wsc-store/wal-self-contained-segments/v1";
const WSC_SELF_CONTAINED_WAL_SEGMENT_WARP: &str = "echo/wsc-store/wal-self-contained-segments";
const WSC_SELF_CONTAINED_WAL_SEGMENT_ROOT: &str = "echo/wsc-store/wal-self-contained-segments/root";
const WSC_SELF_CONTAINED_WAL_SEGMENT_NODE_TYPE: &str =
    "echo/wsc-store/wal-self-contained-segments/node/v1";
const WSC_SELF_CONTAINED_WAL_SEGMENT_EDGE_TYPE: &str =
    "echo/wsc-store/wal-self-contained-segments/member/v1";
const WSC_SELF_CONTAINED_WAL_SEGMENT_ATTACHMENT_TYPE: &str =
    "echo/wsc-store/wal-self-contained-segments/segment-bytes/v1";
const WSC_SELF_CONTAINED_RETAINED_BASIS_DOMAIN: &[u8] =
    b"echo:wsc_store:self_contained_retained_basis:v1\0";
const WSC_SELF_CONTAINED_RETAINED_NODE_DOMAIN: &[u8] =
    b"echo:wsc_store:self_contained_retained_node:v1\0";
const WSC_SELF_CONTAINED_RETAINED_EDGE_DOMAIN: &[u8] =
    b"echo:wsc_store:self_contained_retained_edge:v1\0";
const WSC_SELF_CONTAINED_RETAINED_SCHEMA: &str = "echo/wsc-store/self-contained-retained/v1";
const WSC_SELF_CONTAINED_RETAINED_WARP: &str = "echo/wsc-store/self-contained-retained";
const WSC_SELF_CONTAINED_RETAINED_ROOT: &str = "echo/wsc-store/self-contained-retained/root";
const WSC_SELF_CONTAINED_RETAINED_NODE_TYPE: &str =
    "echo/wsc-store/self-contained-retained/node/v1";
const WSC_SELF_CONTAINED_RETAINED_EDGE_TYPE: &str =
    "echo/wsc-store/self-contained-retained/member/v1";
const WSC_SELF_CONTAINED_RETAINED_ATTACHMENT_TYPE: &str =
    "echo/wsc-store/self-contained-retained/material-bytes/v1";
const WSC_CAS_ADDRESSED_WAL_REF_BASIS_DOMAIN: &[u8] =
    b"echo:wsc_store:cas_addressed_wal_ref_basis:v1\0";
const WSC_CAS_ADDRESSED_WAL_REF_NODE_DOMAIN: &[u8] =
    b"echo:wsc_store:cas_addressed_wal_ref_node:v1\0";
const WSC_CAS_ADDRESSED_WAL_REF_EDGE_DOMAIN: &[u8] =
    b"echo:wsc_store:cas_addressed_wal_ref_edge:v1\0";
const WSC_CAS_ADDRESSED_WAL_REF_SCHEMA: &str = "echo/wsc-store/wal-cas-addressed-refs/v1";
const WSC_CAS_ADDRESSED_WAL_REF_WARP: &str = "echo/wsc-store/wal-cas-addressed-refs";
const WSC_CAS_ADDRESSED_WAL_REF_ROOT: &str = "echo/wsc-store/wal-cas-addressed-refs/root";
const WSC_CAS_ADDRESSED_WAL_REF_NODE_TYPE: &str = "echo/wsc-store/wal-cas-addressed-refs/node/v1";
const WSC_CAS_ADDRESSED_WAL_REF_EDGE_TYPE: &str = "echo/wsc-store/wal-cas-addressed-refs/member/v1";
const WSC_CAS_ADDRESSED_WAL_SEGMENT_ATTACHMENT_TYPE: &str =
    "echo/wsc-store/wal-cas-addressed-refs/segment/v1";
const WSC_CAS_ADDRESSED_RETAINED_ATTACHMENT_TYPE: &str =
    "echo/wsc-store/wal-cas-addressed-refs/retained-material/v1";
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

/// Record slices carried by WAL causal-history WSC exports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WscWalCausalHistoryRecords<'a> {
    /// Retained material records recovered from WAL.
    pub retained_materials: &'a [RetainedMaterialRecord],
    /// Retained reading references recovered from WAL.
    pub reading_refs: &'a [ReadingRefRecord],
    /// Accepted submission records recovered from WAL.
    pub accepted_submissions: &'a [SubmissionAcceptanceRecord],
    /// Tick receipt records recovered from WAL.
    pub receipts: &'a [TickReceiptRecord],
    /// Receipt-correlation records recovered from WAL.
    pub correlations: &'a [WalReceiptCorrelationRecord],
}

impl WscWalCausalHistoryRecords<'_> {
    /// Empty causal-history records for dependency-only export construction.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            retained_materials: &[],
            reading_refs: &[],
            accepted_submissions: &[],
            receipts: &[],
            correlations: &[],
        }
    }
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
    /// Retained material and reading reference WSC envelope.
    pub retention_envelope: WscStoreEnvelope,
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
    /// Retained material and reading references recovered from WSC.
    pub retention: WscRetentionRecords,
    /// External segment byte dependencies for ref-only validation.
    pub segment_dependencies: Vec<WscRefOnlyWalSegmentDependency>,
}

/// Embedded WAL segment material carried by a self-contained WSC export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscSelfContainedWalSegmentMaterial {
    /// Logical WAL segment id.
    pub segment_id: WalSegmentId,
    /// Raw encoded WAL segment bytes.
    pub segment_bytes: Vec<u8>,
}

/// Embedded retained payload material carried by a self-contained WSC export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscSelfContainedRetainedMaterial {
    /// Retained material record recovered from WAL.
    pub material: RetainedMaterialRecord,
    /// Raw retained payload bytes.
    pub material_bytes: Vec<u8>,
}

/// Self-contained WAL causal-history WSC export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscSelfContainedWalExport {
    /// Export profile.
    pub profile: WscCausalHistoryExportProfileKind,
    /// WAL projection graph WSC envelope.
    pub projection_envelope: WscStoreEnvelope,
    /// Embedded WAL segment material WSC envelope.
    pub segment_material_envelope: WscStoreEnvelope,
    /// Embedded retained payload material WSC envelope.
    pub retained_material_envelope: WscStoreEnvelope,
    /// Accepted submission evidence WSC envelope.
    pub accepted_submission_envelope: WscStoreEnvelope,
    /// Tick receipt and receipt-correlation WSC envelope.
    pub receipt_correlation_envelope: WscStoreEnvelope,
    /// Retained material and reading reference WSC envelope.
    pub retention_envelope: WscStoreEnvelope,
}

/// Imported and validated self-contained WAL causal-history WSC evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscSelfContainedWalImport {
    /// Export profile.
    pub profile: WscCausalHistoryExportProfileKind,
    /// Observed projection graph WSC shape.
    pub projection: WalProjectionGraphObservation,
    /// Expected WAL root identity digest validated against the projection WSC.
    pub root_identity_digest: Hash,
    /// Embedded segment recoveries validated against the projected WAL root.
    pub segment_recoveries: Vec<WalSegmentBytesRecovery>,
    /// Retained payload bytes recovered from embedded WSC material.
    pub retained_payloads: Vec<WscSelfContainedRetainedMaterial>,
    /// Accepted submission records recovered from WSC.
    pub accepted_submissions: Vec<SubmissionAcceptanceRecord>,
    /// Tick receipt records recovered from WSC.
    pub receipts: Vec<TickReceiptRecord>,
    /// Receipt-correlation records recovered from WSC.
    pub correlations: Vec<WalReceiptCorrelationRecord>,
    /// Submission retry index rebuilt from imported WSC evidence.
    pub submission_index: RecoveredSubmissionIndex,
    /// Receipt correlation index rebuilt from imported WSC evidence.
    pub receipt_index: RecoveredReceiptIndex,
    /// Retained material and reading references recovered from WSC.
    pub retention: WscRetentionRecords,
}

/// CAS material for one WAL segment supplied by an exporter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WscCasAddressedWalSegmentMaterial {
    /// Logical WAL segment id.
    pub segment_id: WalSegmentId,
    /// Content-only CAS hash for the retained segment bytes.
    pub content_hash: Hash,
    /// Semantic coordinate digest naming why this blob matters.
    pub semantic_coordinate_digest: Hash,
    /// Retained byte length.
    pub byte_len: u64,
}

/// CAS reference for one projected WAL segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscCasAddressedWalSegmentReference {
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
    /// Content-only CAS hash for the retained segment bytes.
    pub content_hash: Hash,
    /// Semantic coordinate digest naming why this blob matters.
    pub semantic_coordinate_digest: Hash,
    /// Retained byte length.
    pub byte_len: u64,
}

/// CAS reference for retained material named by semantic coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WscCasAddressedRetainedMaterialReference {
    /// Retained material family.
    pub material_kind: RetainedMaterialKind,
    /// Content-only CAS hash for the retained bytes.
    pub content_hash: Hash,
    /// Semantic coordinate digest naming why this blob matters.
    pub semantic_coordinate_digest: Hash,
    /// Retained byte length.
    pub byte_len: u64,
}

/// CAS-addressed material references recovered from a WSC export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscCasAddressedWalReferences {
    /// WAL segment byte CAS references.
    pub segments: Vec<WscCasAddressedWalSegmentReference>,
    /// Retained material CAS references.
    pub retained_materials: Vec<WscCasAddressedRetainedMaterialReference>,
}

/// CAS-addressed WAL causal-history WSC export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscCasAddressedWalExport {
    /// Export profile.
    pub profile: WscCausalHistoryExportProfileKind,
    /// WAL projection graph WSC envelope.
    pub projection_envelope: WscStoreEnvelope,
    /// CAS material reference WSC envelope.
    pub cas_reference_envelope: WscStoreEnvelope,
    /// Accepted submission evidence WSC envelope.
    pub accepted_submission_envelope: WscStoreEnvelope,
    /// Tick receipt and receipt-correlation WSC envelope.
    pub receipt_correlation_envelope: WscStoreEnvelope,
    /// Retained material and reading reference WSC envelope.
    pub retention_envelope: WscStoreEnvelope,
}

/// Imported and validated CAS-addressed WAL causal-history WSC evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WscCasAddressedWalImport {
    /// Export profile.
    pub profile: WscCausalHistoryExportProfileKind,
    /// Observed projection graph WSC shape.
    pub projection: WalProjectionGraphObservation,
    /// Expected WAL root identity digest validated against the projection WSC.
    pub root_identity_digest: Hash,
    /// CAS references validated for projection agreement and blob availability.
    pub cas_references: WscCasAddressedWalReferences,
    /// Accepted submission records recovered from WSC.
    pub accepted_submissions: Vec<SubmissionAcceptanceRecord>,
    /// Tick receipt records recovered from WSC.
    pub receipts: Vec<TickReceiptRecord>,
    /// Receipt-correlation records recovered from WSC.
    pub correlations: Vec<WalReceiptCorrelationRecord>,
    /// Retained material and reading references recovered from WSC.
    pub retention: WscRetentionRecords,
}

/// Minimal CAS blob lookup port used by CAS-addressed WSC validation.
pub trait WscCasBlobStorePort {
    /// Returns retained bytes for a content-only CAS hash, if present.
    fn cas_blob_bytes(&self, content_hash: &Hash) -> Option<Vec<u8>>;
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
    /// Retained evidence WSC material was invalid.
    #[error("invalid retained evidence WSC material")]
    Retention(WscStoreObstruction),
    /// Causal-history WSC evidence was incomplete.
    #[error("incomplete causal-history WSC material")]
    IncompleteCausalHistory(WscStoreObstruction),
}

/// Error returned when building a self-contained WAL WSC export.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WscSelfContainedWalExportError {
    /// Projection graph WSC serialization failed.
    #[error("invalid self-contained WAL projection WSC material")]
    Projection(WscRefOnlyWalExportError),
    /// The expected WAL root named a segment absent from embedded material.
    #[error("self-contained WAL WSC export is missing embedded segment material")]
    MissingSegmentMaterial {
        /// Missing segment id.
        segment_id: WalSegmentId,
    },
    /// Embedded material carried a segment absent from the expected WAL root.
    #[error("self-contained WAL WSC export has extra embedded segment material")]
    ExtraSegmentMaterial {
        /// Extra segment id.
        segment_id: WalSegmentId,
    },
    /// A present retained material record had no embedded payload bytes.
    #[error("self-contained WAL WSC export is missing embedded retained material")]
    MissingRetainedMaterial {
        /// Missing retained material digest.
        material_digest: Hash,
    },
    /// Embedded retained material carried a digest absent from retained records.
    #[error("self-contained WAL WSC export has extra embedded retained material")]
    ExtraRetainedMaterial {
        /// Extra retained material digest.
        material_digest: Hash,
    },
    /// Embedded retained material bytes did not hash to the retained digest.
    #[error("self-contained WAL WSC retained material digest mismatch")]
    RetainedMaterialDigestMismatch {
        /// Expected retained material digest.
        expected: Hash,
        /// Actual retained material byte hash.
        actual: Hash,
    },
    /// Embedded segment material WSC envelope was invalid.
    #[error("invalid self-contained WAL segment material WSC")]
    SegmentMaterial(WscStoreObstruction),
    /// Embedded retained material WSC envelope was invalid.
    #[error("invalid self-contained retained material WSC")]
    RetainedMaterial(WscStoreObstruction),
    /// Accepted submission WSC envelope was invalid.
    #[error("invalid accepted submission WSC material")]
    AcceptedSubmissions(WscStoreObstruction),
    /// Receipt correlation WSC envelope was invalid.
    #[error("invalid receipt correlation WSC material")]
    ReceiptCorrelations(WscStoreObstruction),
    /// Retained evidence WSC envelope was invalid.
    #[error("invalid retained evidence WSC material")]
    Retention(WscStoreObstruction),
}

/// Error returned when importing a self-contained WAL WSC export.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WscSelfContainedWalImportError {
    /// The export profile was not self-contained.
    #[error("WAL WSC export profile is not self-contained")]
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
    /// Embedded segment material WSC material was invalid.
    #[error("invalid self-contained WAL segment material WSC")]
    SegmentMaterial(WscStoreObstruction),
    /// The expected WAL root named a segment absent from embedded material.
    #[error("self-contained WAL WSC export is missing embedded segment material")]
    MissingSegmentMaterial {
        /// Missing segment id.
        segment_id: WalSegmentId,
    },
    /// Embedded material carried a segment absent from the expected WAL root.
    #[error("self-contained WAL WSC export has extra embedded segment material")]
    ExtraSegmentMaterial {
        /// Extra segment id.
        segment_id: WalSegmentId,
    },
    /// Embedded WAL segment recovery failed.
    #[error("embedded WAL segment recovery failed")]
    SegmentRecovery {
        /// Segment id being recovered.
        segment_id: WalSegmentId,
        /// WAL recovery error.
        error: WalRecoveryError,
    },
    /// Embedded WAL segment digest did not match projected root evidence.
    #[error("embedded WAL segment digest mismatch")]
    SegmentDigestMismatch {
        /// Segment id being checked.
        segment_id: WalSegmentId,
        /// Expected segment digest from the projected root.
        expected: Hash,
        /// Actual digest recovered from embedded bytes.
        actual: Hash,
    },
    /// Embedded WAL segment LSN range did not match projected root evidence.
    #[error("embedded WAL segment LSN range mismatch")]
    SegmentLsnRangeMismatch {
        /// Segment id being checked.
        segment_id: WalSegmentId,
        /// Expected first LSN from the projected root.
        expected_first_lsn: Option<Lsn>,
        /// Actual first recovered LSN.
        actual_first_lsn: Option<Lsn>,
        /// Expected last LSN from the projected root.
        expected_last_lsn: Option<Lsn>,
        /// Actual last recovered LSN.
        actual_last_lsn: Option<Lsn>,
    },
    /// Embedded WAL segment commit chain did not match projected root evidence.
    #[error("embedded WAL segment commit chain mismatch")]
    SegmentCommitChainMismatch {
        /// Segment id being checked.
        segment_id: WalSegmentId,
        /// Expected final commit digest from the projected root.
        expected_final_commit_digest: Hash,
        /// Actual final commit digest recovered from embedded bytes.
        actual_final_commit_digest: Option<Hash>,
    },
    /// Embedded WAL segment commit anchors did not match projected root evidence.
    #[error("embedded WAL segment commit anchors mismatch")]
    SegmentCommitAnchorMismatch {
        /// Segment id being checked.
        segment_id: WalSegmentId,
        /// Expected commit anchor identity digests from the projected root.
        expected: Vec<Hash>,
        /// Actual commit anchor identity digests recovered from embedded bytes.
        actual: Vec<Hash>,
    },
    /// Embedded WAL segment recovery found an unclean tail posture.
    #[error("embedded WAL segment tail posture mismatch")]
    SegmentTailPostureMismatch {
        /// Segment id being checked.
        segment_id: WalSegmentId,
        /// Actual tail posture recovered from embedded bytes.
        actual: RecoveryTailPosture,
    },
    /// Embedded retained material WSC material was invalid.
    #[error("invalid self-contained retained material WSC")]
    RetainedMaterial(WscStoreObstruction),
    /// A present retained material record had no embedded payload bytes.
    #[error("self-contained WAL WSC import is missing embedded retained material")]
    MissingRetainedMaterial {
        /// Missing retained material digest.
        material_digest: Hash,
    },
    /// Embedded retained material carried a digest absent from retained records.
    #[error("self-contained WAL WSC import has extra embedded retained material")]
    ExtraRetainedMaterial {
        /// Extra retained material digest.
        material_digest: Hash,
    },
    /// Embedded retained material bytes did not hash to the retained digest.
    #[error("self-contained WAL WSC retained material digest mismatch")]
    RetainedMaterialDigestMismatch {
        /// Expected retained material digest.
        expected: Hash,
        /// Actual retained material byte hash.
        actual: Hash,
    },
    /// Accepted submission WSC material was invalid.
    #[error("invalid accepted submission WSC material")]
    AcceptedSubmissions(WscStoreObstruction),
    /// Receipt correlation WSC material was invalid.
    #[error("invalid receipt correlation WSC material")]
    ReceiptCorrelations(WscStoreObstruction),
    /// Retained evidence WSC material was invalid.
    #[error("invalid retained evidence WSC material")]
    Retention(WscStoreObstruction),
    /// Causal-history WSC evidence was incomplete.
    #[error("incomplete causal-history WSC material")]
    IncompleteCausalHistory(WscStoreObstruction),
    /// Recovered submission index construction failed.
    #[error("recovered submission index construction failed")]
    Index(WalRecoveryIndexError),
}

/// Error returned when building a CAS-addressed WAL WSC export.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WscCasAddressedWalExportError {
    /// Projection graph WSC serialization failed.
    #[error("invalid CAS-addressed WAL projection WSC material")]
    Projection(WscRefOnlyWalExportError),
    /// The expected WAL root named a segment absent from CAS references.
    #[error("CAS-addressed WAL WSC export is missing a segment CAS reference")]
    MissingSegmentCasReference {
        /// Missing segment id.
        segment_id: WalSegmentId,
    },
    /// CAS references carried a segment absent from the expected WAL root.
    #[error("CAS-addressed WAL WSC export has an extra segment CAS reference")]
    ExtraSegmentCasReference {
        /// Extra segment id.
        segment_id: WalSegmentId,
    },
    /// CAS reference WSC envelope was invalid.
    #[error("invalid CAS-addressed WAL reference WSC")]
    CasReferences(WscStoreObstruction),
    /// Accepted submission WSC envelope was invalid.
    #[error("invalid accepted submission WSC material")]
    AcceptedSubmissions(WscStoreObstruction),
    /// Receipt correlation WSC envelope was invalid.
    #[error("invalid receipt correlation WSC material")]
    ReceiptCorrelations(WscStoreObstruction),
    /// Retained evidence WSC envelope was invalid.
    #[error("invalid retained evidence WSC material")]
    Retention(WscStoreObstruction),
}

/// Error returned when importing a CAS-addressed WAL WSC export.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WscCasAddressedWalImportError {
    /// The export profile was not CAS-addressed.
    #[error("WAL WSC export profile is not CAS-addressed")]
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
    /// CAS reference WSC material was invalid.
    #[error("invalid CAS-addressed WAL reference WSC")]
    CasReferences(WscStoreObstruction),
    /// Segment CAS references did not match the recovered WAL root.
    #[error("CAS-addressed WAL segment references mismatch")]
    SegmentCasReferenceMismatch {
        /// Segment id being checked.
        segment_id: WalSegmentId,
    },
    /// A required CAS blob was missing.
    #[error("missing CAS blob for WAL WSC material")]
    MissingCasBlob {
        /// Missing content hash.
        content_hash: Hash,
        /// Semantic coordinate digest for the missing reference.
        semantic_coordinate_digest: Hash,
    },
    /// CAS blob bytes did not hash to the referenced content hash.
    #[error("CAS blob content hash mismatch")]
    CasBlobHashMismatch {
        /// Expected content hash.
        expected: Hash,
        /// Actual content hash computed from stored bytes.
        actual: Hash,
    },
    /// CAS blob byte length did not match the reference.
    #[error("CAS blob byte length mismatch")]
    CasBlobLengthMismatch {
        /// Expected byte length.
        expected: u64,
        /// Actual byte length.
        actual: u64,
    },
    /// Accepted submission WSC material was invalid.
    #[error("invalid accepted submission WSC material")]
    AcceptedSubmissions(WscStoreObstruction),
    /// Receipt correlation WSC material was invalid.
    #[error("invalid receipt correlation WSC material")]
    ReceiptCorrelations(WscStoreObstruction),
    /// Retained evidence WSC material was invalid.
    #[error("invalid retained evidence WSC material")]
    Retention(WscStoreObstruction),
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
    /// Filesystem path was implicated.
    FilesystemPath {
        /// Digest of the filesystem path, without making the host path a
        /// portable semantic identifier.
        path_digest: Hash,
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
    /// Commit marker bytes were malformed.
    InvalidCommitMarker,
    /// Commit marker does not match the envelope material.
    CommitMarkerMismatch,
    /// Filesystem I/O obstructed store access.
    FilesystemIo,
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

    fn invalid_commit_marker(envelope_id: WscStoreEnvelopeId) -> Self {
        Self {
            kind: WscStoreObstructionKind::InvalidCommitMarker,
            subject: WscStoreSubject::Envelope { envelope_id },
        }
    }

    fn commit_marker_mismatch(envelope_id: WscStoreEnvelopeId) -> Self {
        Self {
            kind: WscStoreObstructionKind::CommitMarkerMismatch,
            subject: WscStoreSubject::Envelope { envelope_id },
        }
    }

    fn filesystem_io(path: &Path) -> Self {
        Self {
            kind: WscStoreObstructionKind::FilesystemIo,
            subject: WscStoreSubject::FilesystemPath {
                path_digest: digest_filesystem_path(path),
            },
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

    fn decode(envelope_id: WscStoreEnvelopeId, bytes: &[u8]) -> Result<Self, WscStoreObstruction> {
        if bytes.len() != COMMIT_MARKER_LEN {
            return Err(WscStoreObstruction::invalid_commit_marker(envelope_id));
        }
        let magic = read_marker_array::<8>(bytes, 0, envelope_id)?;
        if &magic != WSC_STORE_COMMIT_MARKER_MAGIC {
            return Err(WscStoreObstruction::invalid_commit_marker(envelope_id));
        }
        let version = u16::from_le_bytes(read_marker_array::<2>(bytes, 8, envelope_id)?);
        if version != WSC_STORE_COMMIT_MARKER_VERSION {
            return Err(WscStoreObstruction::invalid_commit_marker(envelope_id));
        }
        let record_kind_code = u16::from_le_bytes(read_marker_array::<2>(bytes, 10, envelope_id)?);
        let record_kind = WscStoreRecordKind::from_code(record_kind_code)
            .ok_or_else(|| WscStoreObstruction::invalid_commit_marker(envelope_id))?;
        let decoded_envelope_id =
            WscStoreEnvelopeId::from_hash(read_marker_array::<32>(bytes, 12, envelope_id)?);
        if decoded_envelope_id != envelope_id {
            return Err(WscStoreObstruction::commit_marker_mismatch(envelope_id));
        }
        let schema_hash = read_marker_array::<32>(bytes, 44, envelope_id)?;
        let basis_digest = read_marker_array::<32>(bytes, 76, envelope_id)?;
        let wsc_digest = read_marker_array::<32>(bytes, 108, envelope_id)?;
        let tick = u64::from_le_bytes(read_marker_array::<8>(bytes, 140, envelope_id)?);
        let encoded_len = u64::from_le_bytes(read_marker_array::<8>(bytes, 148, envelope_id)?);
        let marker_digest = read_marker_array::<32>(bytes, 156, envelope_id)?;
        let expected_marker_digest = derive_commit_marker_digest_from_parts(
            envelope_id,
            record_kind,
            &basis_digest,
            &schema_hash,
            tick,
            &wsc_digest,
            encoded_len,
        );
        if marker_digest != expected_marker_digest {
            return Err(WscStoreObstruction::commit_marker_mismatch(envelope_id));
        }
        Ok(Self {
            envelope_id,
            record_kind,
            basis_digest,
            schema_hash,
            tick,
            wsc_digest,
            encoded_len,
            marker_digest,
        })
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(COMMIT_MARKER_LEN);
        bytes.extend_from_slice(WSC_STORE_COMMIT_MARKER_MAGIC);
        bytes.extend_from_slice(&WSC_STORE_COMMIT_MARKER_VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.record_kind.code().to_le_bytes());
        bytes.extend_from_slice(&self.envelope_id.as_hash());
        bytes.extend_from_slice(&self.schema_hash);
        bytes.extend_from_slice(&self.basis_digest);
        bytes.extend_from_slice(&self.wsc_digest);
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        bytes.extend_from_slice(&self.encoded_len.to_le_bytes());
        bytes.extend_from_slice(&self.marker_digest);
        bytes
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
    records: WscWalCausalHistoryRecords<'_>,
) -> Result<WscRefOnlyWalExport, WscRefOnlyWalExportError> {
    Ok(WscRefOnlyWalExport {
        profile: WscCausalHistoryExportProfileKind::RefOnly,
        projection_envelope: wsc_ref_only_wal_projection_envelope(root)?,
        accepted_submission_envelope: accepted_submission_records_to_wsc_envelope(
            records.accepted_submissions,
        )
        .map_err(WscRefOnlyWalExportError::Envelope)?,
        receipt_correlation_envelope: receipt_correlation_records_to_wsc_envelope(
            records.receipts,
            records.correlations,
        )
        .map_err(WscRefOnlyWalExportError::Envelope)?,
        retention_envelope: retention_records_to_wsc_envelope(
            records.retained_materials,
            records.reading_refs,
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
    let retention = retention_records_from_wsc_envelope(&export.retention_envelope)
        .map_err(WscRefOnlyWalImportError::Retention)?;

    Ok(WscRefOnlyWalImport {
        profile: export.profile,
        projection,
        root_identity_digest: expected_root_identity,
        accepted_submissions,
        receipts: receipt_records.receipts,
        correlations: receipt_records.correlations,
        retention,
        segment_dependencies: expected_dependencies,
    })
}

/// Builds a self-contained WAL causal-history WSC export.
///
/// The projection envelope carries read-only WAL graph facts. Segment bytes are
/// embedded as WSC material so importers can validate segment digests, LSN
/// ranges, and commit-chain facts without access to the original WAL root.
///
/// # Errors
///
/// Returns a typed export error when generated WSC material cannot be written or
/// one of the generated envelopes fails validation.
pub fn wsc_self_contained_wal_export(
    root: &WalRoot,
    segment_materials: &[WscSelfContainedWalSegmentMaterial],
    retained_payloads: &[WscSelfContainedRetainedMaterial],
    records: WscWalCausalHistoryRecords<'_>,
) -> Result<WscSelfContainedWalExport, WscSelfContainedWalExportError> {
    let segment_materials = canonical_self_contained_segment_materials(segment_materials)
        .map_err(WscSelfContainedWalExportError::SegmentMaterial)?;
    validate_self_contained_export_segment_material_ids(root, &segment_materials)?;
    let retained_payloads = canonical_self_contained_retained_materials(retained_payloads)
        .map_err(WscSelfContainedWalExportError::RetainedMaterial)?;
    validate_self_contained_export_retained_payloads(
        records.retained_materials,
        &retained_payloads,
    )?;
    Ok(WscSelfContainedWalExport {
        profile: WscCausalHistoryExportProfileKind::SelfContained,
        projection_envelope: wsc_ref_only_wal_projection_envelope(root)
            .map_err(WscSelfContainedWalExportError::Projection)?,
        segment_material_envelope: self_contained_segment_materials_to_wsc_envelope(
            &segment_materials,
        )
        .map_err(WscSelfContainedWalExportError::SegmentMaterial)?,
        retained_material_envelope: self_contained_retained_materials_to_wsc_envelope(
            &retained_payloads,
        )
        .map_err(WscSelfContainedWalExportError::RetainedMaterial)?,
        accepted_submission_envelope: accepted_submission_records_to_wsc_envelope(
            records.accepted_submissions,
        )
        .map_err(WscSelfContainedWalExportError::AcceptedSubmissions)?,
        receipt_correlation_envelope: receipt_correlation_records_to_wsc_envelope(
            records.receipts,
            records.correlations,
        )
        .map_err(WscSelfContainedWalExportError::ReceiptCorrelations)?,
        retention_envelope: retention_records_to_wsc_envelope(
            records.retained_materials,
            records.reading_refs,
        )
        .map_err(WscSelfContainedWalExportError::Retention)?,
    })
}

/// Validates and imports a self-contained WAL causal-history WSC export.
///
/// This compares the imported projection graph with `expected_root`, recovers
/// embedded segment bytes through WAL recovery, compares recovered segment
/// evidence back to projected root facts, and rebuilds accepted-submission and
/// receipt indexes from WSC material.
///
/// # Errors
///
/// Returns a typed import error when WSC material is malformed, embedded segment
/// bytes fail WAL recovery, or recovered evidence does not match the projected
/// WAL root.
pub fn validate_wsc_self_contained_wal_export(
    export: &WscSelfContainedWalExport,
    expected_root: &WalRoot,
) -> Result<WscSelfContainedWalImport, WscSelfContainedWalImportError> {
    if export.profile != WscCausalHistoryExportProfileKind::SelfContained {
        return Err(WscSelfContainedWalImportError::ProfileMismatch {
            actual: export.profile,
        });
    }
    if export.projection_envelope.record_kind() != WscStoreRecordKind::CausalHistory {
        return Err(WscSelfContainedWalImportError::InvalidProjectionEnvelopeKind);
    }

    let projection = observe_wal_projection_graph_wsc(export.projection_envelope.wsc_bytes())
        .map_err(WscSelfContainedWalImportError::ProjectionObservation)?;
    let expected_projection = wsc_ref_only_wal_projection_envelope(expected_root)
        .map_err(WscSelfContainedWalImportError::ExpectedProjection)?;
    let expected_root_identity = expected_root.identity_digest();
    if export.projection_envelope.basis_digest() != &expected_root_identity {
        return Err(WscSelfContainedWalImportError::ProjectionBasisMismatch {
            expected: expected_root_identity,
            actual: *export.projection_envelope.basis_digest(),
        });
    }
    if export.projection_envelope.wsc_bytes() != expected_projection.wsc_bytes() {
        return Err(WscSelfContainedWalImportError::ProjectionPayloadMismatch {
            expected: *expected_projection.wsc_digest(),
            actual: *export.projection_envelope.wsc_digest(),
        });
    }

    let segment_materials =
        self_contained_segment_materials_from_wsc_envelope(&export.segment_material_envelope)
            .map_err(WscSelfContainedWalImportError::SegmentMaterial)?;
    let segment_recoveries =
        validate_self_contained_segment_recoveries(&segment_materials, expected_root)?;
    let retained_payloads =
        self_contained_retained_materials_from_wsc_envelope(&export.retained_material_envelope)
            .map_err(WscSelfContainedWalImportError::RetainedMaterial)?;
    let retention = retention_records_from_wsc_envelope(&export.retention_envelope)
        .map_err(WscSelfContainedWalImportError::Retention)?;
    validate_self_contained_import_retained_payloads(&retention.materials, &retained_payloads)?;

    let accepted_submissions =
        accepted_submission_records_from_wsc_envelope(&export.accepted_submission_envelope)
            .map_err(WscSelfContainedWalImportError::AcceptedSubmissions)?;
    let receipt_records =
        receipt_correlation_records_from_wsc_envelope(&export.receipt_correlation_envelope)
            .map_err(WscSelfContainedWalImportError::ReceiptCorrelations)?;
    validate_wsc_causal_history_records(
        &accepted_submissions,
        &receipt_records.receipts,
        &receipt_records.correlations,
    )
    .map_err(WscSelfContainedWalImportError::IncompleteCausalHistory)?;
    let submission_index = RecoveredSubmissionIndex::from_acceptance_and_receipt_records(
        accepted_submissions.iter().copied(),
        receipt_records.receipts.iter().copied(),
    )
    .map_err(WscSelfContainedWalImportError::Index)?;
    let receipt_index = RecoveredReceiptIndex::from_receipt_correlation_records(
        receipt_records.receipts.iter().copied(),
        receipt_records.correlations.iter().cloned(),
    )
    .map_err(WscSelfContainedWalImportError::Index)?;

    Ok(WscSelfContainedWalImport {
        profile: export.profile,
        projection,
        root_identity_digest: expected_root_identity,
        segment_recoveries,
        retained_payloads,
        accepted_submissions,
        receipts: receipt_records.receipts,
        correlations: receipt_records.correlations,
        submission_index,
        receipt_index,
        retention,
    })
}

/// Builds a CAS-addressed WAL causal-history WSC export.
///
/// The projection envelope remains the causal graph fact source. CAS references
/// name retained bytes by content hash plus semantic coordinate digest, but
/// those references are byte availability evidence, not causal authority.
///
/// # Errors
///
/// Returns a typed export error when generated WSC material cannot be written,
/// segment CAS references do not cover the projected root, or generated
/// envelopes fail validation.
pub fn wsc_cas_addressed_wal_export(
    root: &WalRoot,
    segment_materials: &[WscCasAddressedWalSegmentMaterial],
    retained_material_references: &[WscCasAddressedRetainedMaterialReference],
    records: WscWalCausalHistoryRecords<'_>,
) -> Result<WscCasAddressedWalExport, WscCasAddressedWalExportError> {
    let references =
        wsc_cas_addressed_wal_references(root, segment_materials, retained_material_references)?;
    Ok(WscCasAddressedWalExport {
        profile: WscCausalHistoryExportProfileKind::CasAddressed,
        projection_envelope: wsc_ref_only_wal_projection_envelope(root)
            .map_err(WscCasAddressedWalExportError::Projection)?,
        cas_reference_envelope: cas_addressed_wal_references_to_wsc_envelope(&references)
            .map_err(WscCasAddressedWalExportError::CasReferences)?,
        accepted_submission_envelope: accepted_submission_records_to_wsc_envelope(
            records.accepted_submissions,
        )
        .map_err(WscCasAddressedWalExportError::AcceptedSubmissions)?,
        receipt_correlation_envelope: receipt_correlation_records_to_wsc_envelope(
            records.receipts,
            records.correlations,
        )
        .map_err(WscCasAddressedWalExportError::ReceiptCorrelations)?,
        retention_envelope: retention_records_to_wsc_envelope(
            records.retained_materials,
            records.reading_refs,
        )
        .map_err(WscCasAddressedWalExportError::Retention)?,
    })
}

/// Validates and imports a CAS-addressed WAL causal-history WSC export.
///
/// This verifies projection WSC bytes against `expected_root`, checks that
/// segment CAS references match projected segment facts, and verifies each
/// referenced CAS blob is present and hashes to its content-only reference. It
/// does not recover WAL authority from CAS bytes.
///
/// # Errors
///
/// Returns a typed import error when WSC material is malformed, CAS references
/// do not match projected root facts, required blobs are missing, or stored bytes
/// do not match their content hash references.
pub fn validate_wsc_cas_addressed_wal_export<P>(
    export: &WscCasAddressedWalExport,
    expected_root: &WalRoot,
    cas_store: &P,
) -> Result<WscCasAddressedWalImport, WscCasAddressedWalImportError>
where
    P: WscCasBlobStorePort + ?Sized,
{
    if export.profile != WscCausalHistoryExportProfileKind::CasAddressed {
        return Err(WscCasAddressedWalImportError::ProfileMismatch {
            actual: export.profile,
        });
    }
    if export.projection_envelope.record_kind() != WscStoreRecordKind::CausalHistory {
        return Err(WscCasAddressedWalImportError::InvalidProjectionEnvelopeKind);
    }

    let projection = observe_wal_projection_graph_wsc(export.projection_envelope.wsc_bytes())
        .map_err(WscCasAddressedWalImportError::ProjectionObservation)?;
    let expected_projection = wsc_ref_only_wal_projection_envelope(expected_root)
        .map_err(WscCasAddressedWalImportError::ExpectedProjection)?;
    let expected_root_identity = expected_root.identity_digest();
    if export.projection_envelope.basis_digest() != &expected_root_identity {
        return Err(WscCasAddressedWalImportError::ProjectionBasisMismatch {
            expected: expected_root_identity,
            actual: *export.projection_envelope.basis_digest(),
        });
    }
    if export.projection_envelope.wsc_bytes() != expected_projection.wsc_bytes() {
        return Err(WscCasAddressedWalImportError::ProjectionPayloadMismatch {
            expected: *expected_projection.wsc_digest(),
            actual: *export.projection_envelope.wsc_digest(),
        });
    }

    let cas_references =
        cas_addressed_wal_references_from_wsc_envelope(&export.cas_reference_envelope)
            .map_err(WscCasAddressedWalImportError::CasReferences)?;
    validate_cas_addressed_segment_references(&cas_references.segments, expected_root)?;
    validate_cas_addressed_blob_availability(&cas_references, cas_store)?;

    let accepted_submissions =
        accepted_submission_records_from_wsc_envelope(&export.accepted_submission_envelope)
            .map_err(WscCasAddressedWalImportError::AcceptedSubmissions)?;
    let receipt_records =
        receipt_correlation_records_from_wsc_envelope(&export.receipt_correlation_envelope)
            .map_err(WscCasAddressedWalImportError::ReceiptCorrelations)?;
    validate_wsc_causal_history_records(
        &accepted_submissions,
        &receipt_records.receipts,
        &receipt_records.correlations,
    )
    .map_err(WscCasAddressedWalImportError::IncompleteCausalHistory)?;
    let retention = retention_records_from_wsc_envelope(&export.retention_envelope)
        .map_err(WscCasAddressedWalImportError::Retention)?;

    Ok(WscCasAddressedWalImport {
        profile: export.profile,
        projection,
        root_identity_digest: expected_root_identity,
        cas_references,
        accepted_submissions,
        receipts: receipt_records.receipts,
        correlations: receipt_records.correlations,
        retention,
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

/// Filesystem-backed WSC store implementation.
#[derive(Debug, Clone)]
pub struct FilesystemWscStore {
    root: PathBuf,
    envelopes_dir: PathBuf,
    commit_markers_dir: PathBuf,
}

impl FilesystemWscStore {
    /// Opens or creates a filesystem-backed WSC store rooted at `root`.
    ///
    /// # Errors
    ///
    /// Returns a typed filesystem obstruction when the store directories cannot
    /// be created.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, WscStoreObstruction> {
        let root = root.as_ref().to_path_buf();
        let envelopes_dir = root.join("envelopes");
        let commit_markers_dir = root.join("commit-markers");
        fs::create_dir_all(&root).map_err(|_| WscStoreObstruction::filesystem_io(&root))?;
        if let Some(parent) = root
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            sync_directory(parent)?;
        }
        fs::create_dir_all(&envelopes_dir)
            .map_err(|_| WscStoreObstruction::filesystem_io(&envelopes_dir))?;
        fs::create_dir_all(&commit_markers_dir)
            .map_err(|_| WscStoreObstruction::filesystem_io(&commit_markers_dir))?;
        sync_directory(&root)?;
        Ok(Self {
            root,
            envelopes_dir,
            commit_markers_dir,
        })
    }

    /// Returns the store root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the deterministic envelope file path for `envelope_id`.
    #[must_use]
    pub fn envelope_path(&self, envelope_id: WscStoreEnvelopeId) -> PathBuf {
        self.envelopes_dir
            .join(format!("{}.ecwsc", hash_hex(&envelope_id.as_hash())))
    }

    /// Returns the deterministic commit-marker file path for `envelope_id`.
    #[must_use]
    pub fn commit_marker_path(&self, envelope_id: WscStoreEnvelopeId) -> PathBuf {
        self.commit_markers_dir
            .join(format!("{}.ecwsc.commit", hash_hex(&envelope_id.as_hash())))
    }

    /// Stages an envelope without publishing its commit marker.
    ///
    /// # Errors
    ///
    /// Returns a typed obstruction when existing staged material or marker
    /// material conflicts with the supplied envelope, or when filesystem I/O
    /// fails.
    pub fn stage_envelope_without_commit_marker(
        &mut self,
        envelope: WscStoreEnvelope,
    ) -> Result<WscStoreEnvelopeId, WscStoreObstruction> {
        let envelope_id = envelope.id();
        let existing = self.read_envelope_material(envelope_id)?;
        let marker = self.read_commit_marker_material(envelope_id)?;
        if let Some(existing) = existing.as_ref() {
            if existing != &envelope {
                return Err(WscStoreObstruction::duplicate_mismatch(envelope_id));
            }
            if let Some(marker) = marker {
                if !marker.matches_envelope(existing) {
                    return Err(WscStoreObstruction::commit_marker_mismatch(envelope_id));
                }
            }
            return Ok(envelope_id);
        }
        if let Some(marker) = marker {
            if !marker.matches_envelope(&envelope) {
                return Err(WscStoreObstruction::commit_marker_mismatch(envelope_id));
            }
        }
        write_atomic(&self.envelope_path(envelope_id), &envelope.encode())?;
        Ok(envelope_id)
    }

    /// Publishes the commit marker for a staged envelope.
    ///
    /// # Errors
    ///
    /// Returns a typed obstruction when the staged envelope is absent, malformed,
    /// or mismatched with an existing commit marker.
    pub fn commit_staged_envelope(
        &mut self,
        envelope_id: WscStoreEnvelopeId,
    ) -> Result<WscStoreWriteReceipt, WscStoreObstruction> {
        let Some(envelope) = self.read_envelope_material(envelope_id)? else {
            return Err(WscStoreObstruction::incomplete_write(envelope_id));
        };
        let marker = WscStoreCommitMarker::from_envelope(&envelope)?;
        if let Some(existing) = self.read_commit_marker_material(envelope_id)? {
            if existing != marker {
                return Err(WscStoreObstruction::commit_marker_mismatch(envelope_id));
            }
            return Ok(existing.write_receipt());
        }
        write_atomic(&self.commit_marker_path(envelope_id), &marker.encode())?;
        Ok(marker.write_receipt())
    }

    fn read_envelope_material(
        &self,
        envelope_id: WscStoreEnvelopeId,
    ) -> Result<Option<WscStoreEnvelope>, WscStoreObstruction> {
        let path = self.envelope_path(envelope_id);
        match fs::read(&path) {
            Ok(bytes) => {
                let envelope = WscStoreEnvelope::decode(&bytes)?;
                if envelope.id() != envelope_id {
                    return Err(WscStoreObstruction::duplicate_mismatch(envelope_id));
                }
                Ok(Some(envelope))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(WscStoreObstruction::filesystem_io(&path)),
        }
    }

    fn read_commit_marker_material(
        &self,
        envelope_id: WscStoreEnvelopeId,
    ) -> Result<Option<WscStoreCommitMarker>, WscStoreObstruction> {
        let path = self.commit_marker_path(envelope_id);
        match fs::read(&path) {
            Ok(bytes) => WscStoreCommitMarker::decode(envelope_id, &bytes).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(WscStoreObstruction::filesystem_io(&path)),
        }
    }
}

impl WscStorePort for FilesystemWscStore {
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
            self.read_envelope_material(envelope_id)?,
            self.read_commit_marker_material(envelope_id)?,
        ) {
            (Some(envelope), Some(marker)) if marker.matches_envelope(&envelope) => Ok(envelope),
            (Some(_), Some(_)) => Err(WscStoreObstruction::commit_marker_mismatch(envelope_id)),
            (Some(_), None) | (None, Some(_)) => {
                Err(WscStoreObstruction::incomplete_write(envelope_id))
            }
            (None, None) => Err(WscStoreObstruction::missing_envelope(envelope_id)),
        }
    }

    fn list_envelopes(&self) -> Vec<WscStoreEnvelopeId> {
        let Ok(entries) = fs::read_dir(&self.commit_markers_dir) else {
            return Vec::new();
        };
        let mut envelope_ids = BTreeSet::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(envelope_id) = envelope_id_from_commit_marker_path(&path) else {
                continue;
            };
            envelope_ids.insert(envelope_id);
        }
        envelope_ids.into_iter().collect()
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
            receipt_node_id(&receipt.receipt_ref.identity_digest()),
            WSC_TICK_RECEIPT_ATTACHMENT_TYPE,
            receipt.to_payload_bytes(),
        );
    }
    for correlation in &correlations {
        insert_receipt_material_node(
            &mut store,
            root,
            correlation_node_id(&correlation.receipt_ref.identity_digest()),
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

fn validate_self_contained_export_segment_material_ids(
    root: &WalRoot,
    materials: &[WscSelfContainedWalSegmentMaterial],
) -> Result<(), WscSelfContainedWalExportError> {
    let material_ids = materials
        .iter()
        .map(|material| material.segment_id)
        .collect::<BTreeSet<_>>();
    for segment in &root.segments {
        if !material_ids.contains(&segment.segment_id) {
            return Err(WscSelfContainedWalExportError::MissingSegmentMaterial {
                segment_id: segment.segment_id,
            });
        }
    }
    let root_segment_ids = root
        .segments
        .iter()
        .map(|segment| segment.segment_id)
        .collect::<BTreeSet<_>>();
    if let Some(segment_id) = material_ids
        .iter()
        .find(|segment_id| !root_segment_ids.contains(segment_id))
        .copied()
    {
        return Err(WscSelfContainedWalExportError::ExtraSegmentMaterial { segment_id });
    }
    Ok(())
}

fn self_contained_segment_materials_to_wsc_envelope(
    materials: &[WscSelfContainedWalSegmentMaterial],
) -> Result<WscStoreEnvelope, WscStoreObstruction> {
    let materials = canonical_self_contained_segment_materials(materials)?;
    let mut store = GraphStore::new(make_warp_id(WSC_SELF_CONTAINED_WAL_SEGMENT_WARP));
    let root = make_node_id(WSC_SELF_CONTAINED_WAL_SEGMENT_ROOT);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id(WSC_SELF_CONTAINED_WAL_SEGMENT_NODE_TYPE),
        },
    );
    for material in &materials {
        let payload = self_contained_segment_material_payload(material);
        let node = self_contained_segment_material_node_id(&payload);
        store.insert_node(
            node,
            NodeRecord {
                ty: make_type_id(WSC_SELF_CONTAINED_WAL_SEGMENT_NODE_TYPE),
            },
        );
        store.insert_edge(
            root,
            EdgeRecord {
                id: self_contained_segment_material_edge_id(material.segment_id),
                from: root,
                to: node,
                ty: make_type_id(WSC_SELF_CONTAINED_WAL_SEGMENT_EDGE_TYPE),
            },
        );
        store.set_node_attachment(
            node,
            Some(AttachmentValue::Atom(AtomPayload::new(
                make_type_id(WSC_SELF_CONTAINED_WAL_SEGMENT_ATTACHMENT_TYPE),
                Bytes::from(payload),
            ))),
        );
    }
    let basis_digest = self_contained_segment_material_basis_digest(&materials);
    let input = build_one_warp_input(&store, root);
    let wsc_bytes = write_wsc_one_warp(
        &input,
        make_type_id(WSC_SELF_CONTAINED_WAL_SEGMENT_SCHEMA).0,
        0,
    )
    .map_err(|_| WscStoreObstruction::invalid_wsc(basis_digest))?;
    WscStoreEnvelope::validated(WscStoreRecordKind::CausalHistory, basis_digest, wsc_bytes)
}

fn self_contained_segment_materials_from_wsc_envelope(
    envelope: &WscStoreEnvelope,
) -> Result<Vec<WscSelfContainedWalSegmentMaterial>, WscStoreObstruction> {
    if envelope.record_kind() != WscStoreRecordKind::CausalHistory {
        return Err(WscStoreObstruction::invalid_envelope(0));
    }
    let wsc_digest = *envelope.wsc_digest();
    let file = WscFile::from_bytes(envelope.wsc_bytes().to_vec())
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    if file.schema_hash() != &make_type_id(WSC_SELF_CONTAINED_WAL_SEGMENT_SCHEMA).0 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let view = file
        .warp_view(0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let mut materials = Vec::new();
    for node_index in 0..view.nodes().len() {
        for attachment in view.node_attachments(node_index) {
            if attachment.type_or_warp
                != make_type_id(WSC_SELF_CONTAINED_WAL_SEGMENT_ATTACHMENT_TYPE).0
            {
                return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
            }
            let payload = atom_payload_bytes(&view, attachment, wsc_digest)?;
            materials.push(self_contained_segment_material_from_payload(
                payload, wsc_digest,
            )?);
        }
    }
    let materials = canonical_self_contained_segment_materials(&materials)?;
    let basis_digest = self_contained_segment_material_basis_digest(&materials);
    if envelope.basis_digest() != &basis_digest {
        return Err(WscStoreObstruction::basis_digest_mismatch(
            *envelope.basis_digest(),
            basis_digest,
        ));
    }
    Ok(materials)
}

fn self_contained_retained_materials_to_wsc_envelope(
    materials: &[WscSelfContainedRetainedMaterial],
) -> Result<WscStoreEnvelope, WscStoreObstruction> {
    let materials = canonical_self_contained_retained_materials(materials)?;
    let mut store = GraphStore::new(make_warp_id(WSC_SELF_CONTAINED_RETAINED_WARP));
    let root = make_node_id(WSC_SELF_CONTAINED_RETAINED_ROOT);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id(WSC_SELF_CONTAINED_RETAINED_NODE_TYPE),
        },
    );
    for material in &materials {
        let payload = self_contained_retained_material_payload(material);
        let node = self_contained_retained_material_node_id(&payload);
        store.insert_node(
            node,
            NodeRecord {
                ty: make_type_id(WSC_SELF_CONTAINED_RETAINED_NODE_TYPE),
            },
        );
        store.insert_edge(
            root,
            EdgeRecord {
                id: self_contained_retained_material_edge_id(material.material.material_digest),
                from: root,
                to: node,
                ty: make_type_id(WSC_SELF_CONTAINED_RETAINED_EDGE_TYPE),
            },
        );
        store.set_node_attachment(
            node,
            Some(AttachmentValue::Atom(AtomPayload::new(
                make_type_id(WSC_SELF_CONTAINED_RETAINED_ATTACHMENT_TYPE),
                Bytes::from(payload),
            ))),
        );
    }
    let basis_digest = self_contained_retained_material_basis_digest(&materials);
    let input = build_one_warp_input(&store, root);
    let wsc_bytes = write_wsc_one_warp(
        &input,
        make_type_id(WSC_SELF_CONTAINED_RETAINED_SCHEMA).0,
        0,
    )
    .map_err(|_| WscStoreObstruction::invalid_wsc(basis_digest))?;
    WscStoreEnvelope::validated(
        WscStoreRecordKind::RetainedEvidence,
        basis_digest,
        wsc_bytes,
    )
}

fn self_contained_retained_materials_from_wsc_envelope(
    envelope: &WscStoreEnvelope,
) -> Result<Vec<WscSelfContainedRetainedMaterial>, WscStoreObstruction> {
    if envelope.record_kind() != WscStoreRecordKind::RetainedEvidence {
        return Err(WscStoreObstruction::invalid_envelope(0));
    }
    let wsc_digest = *envelope.wsc_digest();
    let file = WscFile::from_bytes(envelope.wsc_bytes().to_vec())
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    if file.schema_hash() != &make_type_id(WSC_SELF_CONTAINED_RETAINED_SCHEMA).0 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let view = file
        .warp_view(0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let mut materials = Vec::new();
    for node_index in 0..view.nodes().len() {
        for attachment in view.node_attachments(node_index) {
            if attachment.type_or_warp
                != make_type_id(WSC_SELF_CONTAINED_RETAINED_ATTACHMENT_TYPE).0
            {
                return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
            }
            let payload = atom_payload_bytes(&view, attachment, wsc_digest)?;
            materials.push(self_contained_retained_material_from_payload(
                payload, wsc_digest,
            )?);
        }
    }
    let materials = canonical_self_contained_retained_materials(&materials)?;
    let basis_digest = self_contained_retained_material_basis_digest(&materials);
    if envelope.basis_digest() != &basis_digest {
        return Err(WscStoreObstruction::basis_digest_mismatch(
            *envelope.basis_digest(),
            basis_digest,
        ));
    }
    Ok(materials)
}

fn validate_self_contained_export_retained_payloads(
    retained_materials: &[RetainedMaterialRecord],
    retained_payloads: &[WscSelfContainedRetainedMaterial],
) -> Result<(), WscSelfContainedWalExportError> {
    let material_by_digest = retained_materials
        .iter()
        .map(|material| (material.material_digest, *material))
        .collect::<BTreeMap<_, _>>();
    validate_self_contained_retained_hashes(retained_payloads).map_err(|(expected, actual)| {
        WscSelfContainedWalExportError::RetainedMaterialDigestMismatch { expected, actual }
    })?;
    for material in retained_materials
        .iter()
        .filter(|material| material.posture == crate::causal_wal::EvidenceMaterialPosture::Present)
    {
        if !retained_payloads
            .iter()
            .any(|payload| payload.material.material_digest == material.material_digest)
        {
            return Err(WscSelfContainedWalExportError::MissingRetainedMaterial {
                material_digest: material.material_digest,
            });
        }
    }
    if let Some(extra) = retained_payloads
        .iter()
        .find(|payload| !material_by_digest.contains_key(&payload.material.material_digest))
    {
        return Err(WscSelfContainedWalExportError::ExtraRetainedMaterial {
            material_digest: extra.material.material_digest,
        });
    }
    Ok(())
}

fn validate_self_contained_import_retained_payloads(
    retained_materials: &[RetainedMaterialRecord],
    retained_payloads: &[WscSelfContainedRetainedMaterial],
) -> Result<(), WscSelfContainedWalImportError> {
    let material_by_digest = retained_materials
        .iter()
        .map(|material| (material.material_digest, *material))
        .collect::<BTreeMap<_, _>>();
    validate_self_contained_retained_hashes(retained_payloads).map_err(|(expected, actual)| {
        WscSelfContainedWalImportError::RetainedMaterialDigestMismatch { expected, actual }
    })?;
    for material in retained_materials
        .iter()
        .filter(|material| material.posture == crate::causal_wal::EvidenceMaterialPosture::Present)
    {
        if !retained_payloads
            .iter()
            .any(|payload| payload.material.material_digest == material.material_digest)
        {
            return Err(WscSelfContainedWalImportError::MissingRetainedMaterial {
                material_digest: material.material_digest,
            });
        }
    }
    if let Some(extra) = retained_payloads
        .iter()
        .find(|payload| !material_by_digest.contains_key(&payload.material.material_digest))
    {
        return Err(WscSelfContainedWalImportError::ExtraRetainedMaterial {
            material_digest: extra.material.material_digest,
        });
    }
    Ok(())
}

fn validate_self_contained_segment_recoveries(
    materials: &[WscSelfContainedWalSegmentMaterial],
    expected_root: &WalRoot,
) -> Result<Vec<WalSegmentBytesRecovery>, WscSelfContainedWalImportError> {
    let mut material_by_segment = materials
        .iter()
        .map(|material| (material.segment_id, material))
        .collect::<BTreeMap<_, _>>();
    let mut recoveries = Vec::new();
    for segment in &expected_root.segments {
        let Some(material) = material_by_segment.remove(&segment.segment_id) else {
            return Err(WscSelfContainedWalImportError::MissingSegmentMaterial {
                segment_id: segment.segment_id,
            });
        };
        let recovery = recover_wal_segment_bytes(
            segment.segment_id,
            &material.segment_bytes,
            RecoveryAccessMode::ReadOnly,
        )
        .map_err(|error| WscSelfContainedWalImportError::SegmentRecovery {
            segment_id: segment.segment_id,
            error,
        })?;
        validate_self_contained_segment_recovery(segment, &recovery)?;
        recoveries.push(recovery);
    }
    if let Some(segment_id) = material_by_segment.keys().next().copied() {
        return Err(WscSelfContainedWalImportError::ExtraSegmentMaterial { segment_id });
    }
    recoveries.sort_by_key(|recovery| recovery.segment_id);
    Ok(recoveries)
}

fn validate_self_contained_segment_recovery(
    segment: &WalSegmentRef,
    recovery: &WalSegmentBytesRecovery,
) -> Result<(), WscSelfContainedWalImportError> {
    if recovery.report.tail_posture != RecoveryTailPosture::Clean {
        return Err(WscSelfContainedWalImportError::SegmentTailPostureMismatch {
            segment_id: segment.segment_id,
            actual: recovery.report.tail_posture,
        });
    }
    if recovery.segment_digest != segment.segment_digest {
        return Err(WscSelfContainedWalImportError::SegmentDigestMismatch {
            segment_id: segment.segment_id,
            expected: segment.segment_digest,
            actual: recovery.segment_digest,
        });
    }
    let actual_first_lsn = recovery.report.first_committed_lsn();
    let actual_last_lsn = recovery.report.last_committed_lsn();
    if actual_first_lsn != Some(segment.first_lsn) || actual_last_lsn != Some(segment.last_lsn) {
        return Err(WscSelfContainedWalImportError::SegmentLsnRangeMismatch {
            segment_id: segment.segment_id,
            expected_first_lsn: Some(segment.first_lsn),
            actual_first_lsn,
            expected_last_lsn: Some(segment.last_lsn),
            actual_last_lsn,
        });
    }
    let actual_previous_commit_digest = recovery
        .report
        .transactions
        .first()
        .map(|transaction| transaction.commit.previous_committed_transaction_digest);
    let actual_final_commit_digest = recovery.report.last_commit_digest();
    if actual_previous_commit_digest != Some(segment.previous_commit_digest)
        || actual_final_commit_digest != Some(segment.final_commit_digest)
    {
        return Err(WscSelfContainedWalImportError::SegmentCommitChainMismatch {
            segment_id: segment.segment_id,
            expected_final_commit_digest: segment.final_commit_digest,
            actual_final_commit_digest,
        });
    }
    let mut expected_anchor_digests = segment
        .commit_anchors
        .iter()
        .map(WalCommitAnchor::identity_digest)
        .collect::<Vec<_>>();
    expected_anchor_digests.sort_unstable();
    let mut actual_anchor_digests = recovery
        .report
        .transactions
        .iter()
        .map(|transaction| WalCommitAnchor::from_commit(&transaction.commit).identity_digest())
        .collect::<Vec<_>>();
    actual_anchor_digests.sort_unstable();
    if actual_anchor_digests != expected_anchor_digests {
        return Err(
            WscSelfContainedWalImportError::SegmentCommitAnchorMismatch {
                segment_id: segment.segment_id,
                expected: expected_anchor_digests,
                actual: actual_anchor_digests,
            },
        );
    }
    Ok(())
}

fn canonical_self_contained_segment_materials(
    materials: &[WscSelfContainedWalSegmentMaterial],
) -> Result<Vec<WscSelfContainedWalSegmentMaterial>, WscStoreObstruction> {
    let mut by_segment = BTreeMap::new();
    for material in materials {
        if let Some(existing) = by_segment.get(&material.segment_id) {
            if existing != material {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    self_contained_segment_material_duplicate_id(material.segment_id),
                ));
            }
        }
        by_segment.insert(material.segment_id, material.clone());
    }
    Ok(by_segment.into_values().collect())
}

fn canonical_self_contained_retained_materials(
    materials: &[WscSelfContainedRetainedMaterial],
) -> Result<Vec<WscSelfContainedRetainedMaterial>, WscStoreObstruction> {
    let mut by_digest = BTreeMap::new();
    for material in materials {
        if let Some(existing) = by_digest.get(&material.material.material_digest) {
            if existing != material {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    self_contained_retained_material_duplicate_id(
                        material.material.material_digest,
                    ),
                ));
            }
        }
        by_digest.insert(material.material.material_digest, material.clone());
    }
    Ok(by_digest.into_values().collect())
}

fn self_contained_segment_material_payload(
    material: &WscSelfContainedWalSegmentMaterial,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&material.segment_id.as_u64().to_le_bytes());
    out.extend_from_slice(&len_u64(material.segment_bytes.len()).to_le_bytes());
    out.extend_from_slice(&material.segment_bytes);
    out
}

fn self_contained_segment_material_from_payload(
    bytes: &[u8],
    wsc_digest: Hash,
) -> Result<WscSelfContainedWalSegmentMaterial, WscStoreObstruction> {
    if bytes.len() < 16 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let segment_id = WalSegmentId::from_raw(u64::from_le_bytes(
        bytes[0..8]
            .try_into()
            .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?,
    ));
    let segment_len_u64 = u64::from_le_bytes(
        bytes[8..16]
            .try_into()
            .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?,
    );
    let segment_len = usize::try_from(segment_len_u64)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let segment_end = 16usize
        .checked_add(segment_len)
        .ok_or_else(|| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let segment_bytes = bytes
        .get(16..segment_end)
        .ok_or_else(|| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    if segment_end != bytes.len() {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    Ok(WscSelfContainedWalSegmentMaterial {
        segment_id,
        segment_bytes: segment_bytes.to_vec(),
    })
}

fn self_contained_retained_material_payload(
    material: &WscSelfContainedRetainedMaterial,
) -> Vec<u8> {
    let material_record = material.material.to_payload_bytes();
    let mut out = Vec::new();
    out.extend_from_slice(&len_u64(material_record.len()).to_le_bytes());
    out.extend_from_slice(&material_record);
    out.extend_from_slice(&len_u64(material.material_bytes.len()).to_le_bytes());
    out.extend_from_slice(&material.material_bytes);
    out
}

fn self_contained_retained_material_from_payload(
    bytes: &[u8],
    wsc_digest: Hash,
) -> Result<WscSelfContainedRetainedMaterial, WscStoreObstruction> {
    let mut cursor = WscPayloadCursor::new(bytes, wsc_digest);
    let record_len = cursor.read_usize()?;
    let record_bytes = cursor.read_bytes(record_len)?;
    let material = RetainedMaterialRecord::from_payload_bytes(record_bytes)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let material_len = cursor.read_usize()?;
    let material_bytes = cursor.read_bytes(material_len)?.to_vec();
    cursor.finish()?;
    Ok(WscSelfContainedRetainedMaterial {
        material,
        material_bytes,
    })
}

fn self_contained_segment_material_basis_digest(
    materials: &[WscSelfContainedWalSegmentMaterial],
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_SELF_CONTAINED_WAL_SEGMENT_BASIS_DOMAIN);
    for material in materials {
        hasher.update(&self_contained_segment_material_payload(material));
    }
    hasher.finalize().into()
}

fn self_contained_retained_material_basis_digest(
    materials: &[WscSelfContainedRetainedMaterial],
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_SELF_CONTAINED_RETAINED_BASIS_DOMAIN);
    for material in materials {
        hasher.update(&self_contained_retained_material_payload(material));
    }
    hasher.finalize().into()
}

fn self_contained_segment_material_node_id(payload: &[u8]) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_SELF_CONTAINED_WAL_SEGMENT_NODE_DOMAIN);
    hasher.update(payload);
    NodeId(hasher.finalize().into())
}

fn self_contained_retained_material_node_id(payload: &[u8]) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_SELF_CONTAINED_RETAINED_NODE_DOMAIN);
    hasher.update(payload);
    NodeId(hasher.finalize().into())
}

fn self_contained_segment_material_edge_id(segment_id: WalSegmentId) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_SELF_CONTAINED_WAL_SEGMENT_EDGE_DOMAIN);
    hasher.update(&segment_id.as_u64().to_le_bytes());
    EdgeId(hasher.finalize().into())
}

fn self_contained_retained_material_edge_id(material_digest: Hash) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_SELF_CONTAINED_RETAINED_EDGE_DOMAIN);
    hasher.update(&material_digest);
    EdgeId(hasher.finalize().into())
}

fn self_contained_segment_material_duplicate_id(segment_id: WalSegmentId) -> WscStoreEnvelopeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_SELF_CONTAINED_WAL_SEGMENT_NODE_DOMAIN);
    hasher.update(b"duplicate");
    hasher.update(&segment_id.as_u64().to_le_bytes());
    WscStoreEnvelopeId::from_hash(hasher.finalize().into())
}

fn self_contained_retained_material_duplicate_id(material_digest: Hash) -> WscStoreEnvelopeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_SELF_CONTAINED_RETAINED_NODE_DOMAIN);
    hasher.update(b"duplicate");
    hasher.update(&material_digest);
    WscStoreEnvelopeId::from_hash(hasher.finalize().into())
}

fn validate_self_contained_retained_hashes(
    materials: &[WscSelfContainedRetainedMaterial],
) -> Result<(), (Hash, Hash)> {
    for material in materials {
        let actual = cas_content_hash(&material.material_bytes);
        if actual != material.material.material_digest {
            return Err((material.material.material_digest, actual));
        }
    }
    Ok(())
}

fn len_u64(len: usize) -> u64 {
    match u64::try_from(len) {
        Ok(value) => value,
        Err(_) => u64::MAX,
    }
}

fn wsc_cas_addressed_wal_references(
    root: &WalRoot,
    segment_materials: &[WscCasAddressedWalSegmentMaterial],
    retained_materials: &[WscCasAddressedRetainedMaterialReference],
) -> Result<WscCasAddressedWalReferences, WscCasAddressedWalExportError> {
    let mut material_by_segment = BTreeMap::new();
    for material in segment_materials {
        if let Some(existing) = material_by_segment.get(&material.segment_id) {
            if existing != material {
                return Err(WscCasAddressedWalExportError::CasReferences(
                    WscStoreObstruction::duplicate_mismatch(cas_addressed_segment_duplicate_id(
                        material.segment_id,
                    )),
                ));
            }
        }
        material_by_segment.insert(material.segment_id, *material);
    }

    let mut segment_references = Vec::new();
    for segment in &root.segments {
        let Some(material) = material_by_segment.remove(&segment.segment_id) else {
            return Err(WscCasAddressedWalExportError::MissingSegmentCasReference {
                segment_id: segment.segment_id,
            });
        };
        segment_references.push(cas_addressed_segment_reference(segment, material));
    }
    if let Some(segment_id) = material_by_segment.keys().next().copied() {
        return Err(WscCasAddressedWalExportError::ExtraSegmentCasReference { segment_id });
    }

    Ok(WscCasAddressedWalReferences {
        segments: canonical_cas_addressed_segment_references(&segment_references)
            .map_err(WscCasAddressedWalExportError::CasReferences)?,
        retained_materials: canonical_cas_addressed_retained_references(retained_materials)
            .map_err(WscCasAddressedWalExportError::CasReferences)?,
    })
}

fn cas_addressed_segment_reference(
    segment: &WalSegmentRef,
    material: WscCasAddressedWalSegmentMaterial,
) -> WscCasAddressedWalSegmentReference {
    let mut commit_anchor_digests = segment
        .commit_anchors
        .iter()
        .map(WalCommitAnchor::identity_digest)
        .collect::<Vec<_>>();
    commit_anchor_digests.sort_unstable();
    WscCasAddressedWalSegmentReference {
        segment_id: segment.segment_id,
        segment_identity_digest: segment.identity_digest(),
        segment_digest: segment.segment_digest,
        first_lsn: segment.first_lsn,
        last_lsn: segment.last_lsn,
        commit_anchor_digests,
        content_hash: material.content_hash,
        semantic_coordinate_digest: material.semantic_coordinate_digest,
        byte_len: material.byte_len,
    }
}

fn cas_addressed_wal_references_to_wsc_envelope(
    references: &WscCasAddressedWalReferences,
) -> Result<WscStoreEnvelope, WscStoreObstruction> {
    let segments = canonical_cas_addressed_segment_references(&references.segments)?;
    let retained_materials =
        canonical_cas_addressed_retained_references(&references.retained_materials)?;
    let references = WscCasAddressedWalReferences {
        segments,
        retained_materials,
    };
    let mut store = GraphStore::new(make_warp_id(WSC_CAS_ADDRESSED_WAL_REF_WARP));
    let root = make_node_id(WSC_CAS_ADDRESSED_WAL_REF_ROOT);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id(WSC_CAS_ADDRESSED_WAL_REF_NODE_TYPE),
        },
    );
    for segment in &references.segments {
        let payload = cas_addressed_segment_reference_payload(segment);
        insert_cas_addressed_reference_node(
            &mut store,
            root,
            cas_addressed_reference_node_id(b"segment", &payload),
            WSC_CAS_ADDRESSED_WAL_SEGMENT_ATTACHMENT_TYPE,
            payload,
        );
    }
    for retained in &references.retained_materials {
        let payload = cas_addressed_retained_reference_payload(retained);
        insert_cas_addressed_reference_node(
            &mut store,
            root,
            cas_addressed_reference_node_id(b"retained", &payload),
            WSC_CAS_ADDRESSED_RETAINED_ATTACHMENT_TYPE,
            payload,
        );
    }
    let basis_digest = cas_addressed_reference_basis_digest(&references);
    let input = build_one_warp_input(&store, root);
    let wsc_bytes = write_wsc_one_warp(&input, make_type_id(WSC_CAS_ADDRESSED_WAL_REF_SCHEMA).0, 0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(basis_digest))?;
    WscStoreEnvelope::validated(WscStoreRecordKind::CausalHistory, basis_digest, wsc_bytes)
}

fn cas_addressed_wal_references_from_wsc_envelope(
    envelope: &WscStoreEnvelope,
) -> Result<WscCasAddressedWalReferences, WscStoreObstruction> {
    if envelope.record_kind() != WscStoreRecordKind::CausalHistory {
        return Err(WscStoreObstruction::invalid_envelope(0));
    }
    let wsc_digest = *envelope.wsc_digest();
    let file = WscFile::from_bytes(envelope.wsc_bytes().to_vec())
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    validate_wsc(&file).map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    if file.schema_hash() != &make_type_id(WSC_CAS_ADDRESSED_WAL_REF_SCHEMA).0 {
        return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
    }
    let view = file
        .warp_view(0)
        .map_err(|_| WscStoreObstruction::invalid_wsc(wsc_digest))?;
    let mut segments = Vec::new();
    let mut retained_materials = Vec::new();
    for node_index in 0..view.nodes().len() {
        for attachment in view.node_attachments(node_index) {
            let payload = atom_payload_bytes(&view, attachment, wsc_digest)?;
            if attachment.type_or_warp
                == make_type_id(WSC_CAS_ADDRESSED_WAL_SEGMENT_ATTACHMENT_TYPE).0
            {
                segments.push(cas_addressed_segment_reference_from_payload(
                    payload, wsc_digest,
                )?);
            } else if attachment.type_or_warp
                == make_type_id(WSC_CAS_ADDRESSED_RETAINED_ATTACHMENT_TYPE).0
            {
                retained_materials.push(cas_addressed_retained_reference_from_payload(
                    payload, wsc_digest,
                )?);
            } else {
                return Err(WscStoreObstruction::invalid_wsc(wsc_digest));
            }
        }
    }
    let references = WscCasAddressedWalReferences {
        segments: canonical_cas_addressed_segment_references(&segments)?,
        retained_materials: canonical_cas_addressed_retained_references(&retained_materials)?,
    };
    let basis_digest = cas_addressed_reference_basis_digest(&references);
    if envelope.basis_digest() != &basis_digest {
        return Err(WscStoreObstruction::basis_digest_mismatch(
            *envelope.basis_digest(),
            basis_digest,
        ));
    }
    Ok(references)
}

fn validate_cas_addressed_segment_references(
    references: &[WscCasAddressedWalSegmentReference],
    expected_root: &WalRoot,
) -> Result<(), WscCasAddressedWalImportError> {
    let references = references
        .iter()
        .map(|reference| (reference.segment_id, reference))
        .collect::<BTreeMap<_, _>>();
    for segment in &expected_root.segments {
        let Some(reference) = references.get(&segment.segment_id) else {
            return Err(WscCasAddressedWalImportError::SegmentCasReferenceMismatch {
                segment_id: segment.segment_id,
            });
        };
        let expected = cas_addressed_segment_reference(
            segment,
            WscCasAddressedWalSegmentMaterial {
                segment_id: reference.segment_id,
                content_hash: reference.content_hash,
                semantic_coordinate_digest: reference.semantic_coordinate_digest,
                byte_len: reference.byte_len,
            },
        );
        if *reference != &expected {
            return Err(WscCasAddressedWalImportError::SegmentCasReferenceMismatch {
                segment_id: segment.segment_id,
            });
        }
    }
    if let Some(segment_id) = references
        .keys()
        .find(|segment_id| {
            !expected_root
                .segments
                .iter()
                .any(|segment| &segment.segment_id == *segment_id)
        })
        .copied()
    {
        return Err(WscCasAddressedWalImportError::SegmentCasReferenceMismatch { segment_id });
    }
    Ok(())
}

fn validate_cas_addressed_blob_availability<P>(
    references: &WscCasAddressedWalReferences,
    cas_store: &P,
) -> Result<(), WscCasAddressedWalImportError>
where
    P: WscCasBlobStorePort + ?Sized,
{
    for segment in &references.segments {
        validate_cas_blob_reference(
            cas_store,
            segment.content_hash,
            segment.semantic_coordinate_digest,
            segment.byte_len,
        )?;
    }
    for retained in &references.retained_materials {
        validate_cas_blob_reference(
            cas_store,
            retained.content_hash,
            retained.semantic_coordinate_digest,
            retained.byte_len,
        )?;
    }
    Ok(())
}

fn validate_cas_blob_reference<P>(
    cas_store: &P,
    content_hash: Hash,
    semantic_coordinate_digest: Hash,
    byte_len: u64,
) -> Result<(), WscCasAddressedWalImportError>
where
    P: WscCasBlobStorePort + ?Sized,
{
    let Some(bytes) = cas_store.cas_blob_bytes(&content_hash) else {
        return Err(WscCasAddressedWalImportError::MissingCasBlob {
            content_hash,
            semantic_coordinate_digest,
        });
    };
    let actual_hash = cas_content_hash(&bytes);
    if actual_hash != content_hash {
        return Err(WscCasAddressedWalImportError::CasBlobHashMismatch {
            expected: content_hash,
            actual: actual_hash,
        });
    }
    let actual_len = len_u64(bytes.len());
    if actual_len != byte_len {
        return Err(WscCasAddressedWalImportError::CasBlobLengthMismatch {
            expected: byte_len,
            actual: actual_len,
        });
    }
    Ok(())
}

fn canonical_cas_addressed_segment_references(
    references: &[WscCasAddressedWalSegmentReference],
) -> Result<Vec<WscCasAddressedWalSegmentReference>, WscStoreObstruction> {
    let mut by_segment = BTreeMap::new();
    for reference in references {
        if let Some(existing) = by_segment.get(&reference.segment_id) {
            if existing != reference {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    cas_addressed_segment_duplicate_id(reference.segment_id),
                ));
            }
        }
        by_segment.insert(reference.segment_id, reference.clone());
    }
    Ok(by_segment.into_values().collect())
}

fn canonical_cas_addressed_retained_references(
    references: &[WscCasAddressedRetainedMaterialReference],
) -> Result<Vec<WscCasAddressedRetainedMaterialReference>, WscStoreObstruction> {
    let mut by_semantic = BTreeMap::new();
    for reference in references {
        let key = (
            reference.material_kind,
            reference.semantic_coordinate_digest,
        );
        if let Some(existing) = by_semantic.get(&key) {
            if existing != reference {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(reference.semantic_coordinate_digest),
                ));
            }
        }
        by_semantic.insert(key, *reference);
    }
    Ok(by_semantic.into_values().collect())
}

fn insert_cas_addressed_reference_node(
    store: &mut GraphStore,
    root: NodeId,
    node: NodeId,
    attachment_type: &str,
    payload_bytes: Vec<u8>,
) {
    store.insert_node(
        node,
        NodeRecord {
            ty: make_type_id(WSC_CAS_ADDRESSED_WAL_REF_NODE_TYPE),
        },
    );
    store.insert_edge(
        root,
        EdgeRecord {
            id: cas_addressed_reference_edge_id(&node.0),
            from: root,
            to: node,
            ty: make_type_id(WSC_CAS_ADDRESSED_WAL_REF_EDGE_TYPE),
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

fn cas_addressed_segment_reference_payload(
    reference: &WscCasAddressedWalSegmentReference,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&reference.segment_id.as_u64().to_le_bytes());
    out.extend_from_slice(&reference.segment_identity_digest);
    out.extend_from_slice(&reference.segment_digest);
    out.extend_from_slice(&reference.first_lsn.as_u64().to_le_bytes());
    out.extend_from_slice(&reference.last_lsn.as_u64().to_le_bytes());
    out.extend_from_slice(&len_u64(reference.commit_anchor_digests.len()).to_le_bytes());
    for digest in &reference.commit_anchor_digests {
        out.extend_from_slice(digest);
    }
    out.extend_from_slice(&reference.content_hash);
    out.extend_from_slice(&reference.semantic_coordinate_digest);
    out.extend_from_slice(&reference.byte_len.to_le_bytes());
    out
}

fn cas_addressed_segment_reference_from_payload(
    bytes: &[u8],
    wsc_digest: Hash,
) -> Result<WscCasAddressedWalSegmentReference, WscStoreObstruction> {
    let mut cursor = WscPayloadCursor::new(bytes, wsc_digest);
    let segment_id = WalSegmentId::from_raw(cursor.read_u64()?);
    let segment_identity_digest = cursor.read_hash()?;
    let segment_digest = cursor.read_hash()?;
    let first_lsn = Lsn::from_raw(cursor.read_u64()?);
    let last_lsn = Lsn::from_raw(cursor.read_u64()?);
    let anchor_count = cursor.read_usize()?;
    let mut commit_anchor_digests = Vec::with_capacity(anchor_count);
    for _ in 0..anchor_count {
        commit_anchor_digests.push(cursor.read_hash()?);
    }
    let content_hash = cursor.read_hash()?;
    let semantic_coordinate_digest = cursor.read_hash()?;
    let byte_len = cursor.read_u64()?;
    cursor.finish()?;
    commit_anchor_digests.sort_unstable();
    Ok(WscCasAddressedWalSegmentReference {
        segment_id,
        segment_identity_digest,
        segment_digest,
        first_lsn,
        last_lsn,
        commit_anchor_digests,
        content_hash,
        semantic_coordinate_digest,
        byte_len,
    })
}

fn cas_addressed_retained_reference_payload(
    reference: &WscCasAddressedRetainedMaterialReference,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(retained_material_kind_code(reference.material_kind));
    out.extend_from_slice(&reference.content_hash);
    out.extend_from_slice(&reference.semantic_coordinate_digest);
    out.extend_from_slice(&reference.byte_len.to_le_bytes());
    out
}

fn cas_addressed_retained_reference_from_payload(
    bytes: &[u8],
    wsc_digest: Hash,
) -> Result<WscCasAddressedRetainedMaterialReference, WscStoreObstruction> {
    let mut cursor = WscPayloadCursor::new(bytes, wsc_digest);
    let material_kind = retained_material_kind_from_code(cursor.read_u8()?, wsc_digest)?;
    let content_hash = cursor.read_hash()?;
    let semantic_coordinate_digest = cursor.read_hash()?;
    let byte_len = cursor.read_u64()?;
    cursor.finish()?;
    Ok(WscCasAddressedRetainedMaterialReference {
        material_kind,
        content_hash,
        semantic_coordinate_digest,
        byte_len,
    })
}

fn cas_addressed_reference_basis_digest(references: &WscCasAddressedWalReferences) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_CAS_ADDRESSED_WAL_REF_BASIS_DOMAIN);
    for segment in &references.segments {
        hasher.update(b"segment");
        hasher.update(&cas_addressed_segment_reference_payload(segment));
    }
    for retained in &references.retained_materials {
        hasher.update(b"retained");
        hasher.update(&cas_addressed_retained_reference_payload(retained));
    }
    hasher.finalize().into()
}

fn cas_addressed_reference_node_id(role: &[u8], payload_bytes: &[u8]) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_CAS_ADDRESSED_WAL_REF_NODE_DOMAIN);
    hasher.update(role);
    hasher.update(payload_bytes);
    NodeId(hasher.finalize().into())
}

fn cas_addressed_reference_edge_id(node_id: &Hash) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_CAS_ADDRESSED_WAL_REF_EDGE_DOMAIN);
    hasher.update(node_id);
    EdgeId(hasher.finalize().into())
}

fn cas_addressed_segment_duplicate_id(segment_id: WalSegmentId) -> WscStoreEnvelopeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_CAS_ADDRESSED_WAL_REF_NODE_DOMAIN);
    hasher.update(b"duplicate-segment");
    hasher.update(&segment_id.as_u64().to_le_bytes());
    WscStoreEnvelopeId::from_hash(hasher.finalize().into())
}

fn retained_material_kind_code(kind: RetainedMaterialKind) -> u8 {
    match kind {
        RetainedMaterialKind::SubmissionPayload => 1,
        RetainedMaterialKind::TickReceipt => 2,
        RetainedMaterialKind::RuntimeStateDelta => 3,
        RetainedMaterialKind::RuntimeControl => 4,
        RetainedMaterialKind::ReadingPayload => 5,
        RetainedMaterialKind::ReadingEnvelope => 6,
        RetainedMaterialKind::Diagnostic => 7,
    }
}

fn retained_material_kind_from_code(
    code: u8,
    wsc_digest: Hash,
) -> Result<RetainedMaterialKind, WscStoreObstruction> {
    match code {
        1 => Ok(RetainedMaterialKind::SubmissionPayload),
        2 => Ok(RetainedMaterialKind::TickReceipt),
        3 => Ok(RetainedMaterialKind::RuntimeStateDelta),
        4 => Ok(RetainedMaterialKind::RuntimeControl),
        5 => Ok(RetainedMaterialKind::ReadingPayload),
        6 => Ok(RetainedMaterialKind::ReadingEnvelope),
        7 => Ok(RetainedMaterialKind::Diagnostic),
        _ => Err(WscStoreObstruction::invalid_wsc(wsc_digest)),
    }
}

fn cas_content_hash(bytes: &[u8]) -> Hash {
    blake3::hash(bytes).into()
}

struct WscPayloadCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
    wsc_digest: Hash,
}

impl<'a> WscPayloadCursor<'a> {
    const fn new(bytes: &'a [u8], wsc_digest: Hash) -> Self {
        Self {
            bytes,
            offset: 0,
            wsc_digest,
        }
    }

    fn read_u8(&mut self) -> Result<u8, WscStoreObstruction> {
        let Some(value) = self.bytes.get(self.offset).copied() else {
            return Err(WscStoreObstruction::invalid_wsc(self.wsc_digest));
        };
        self.offset += 1;
        Ok(value)
    }

    fn read_u64(&mut self) -> Result<u64, WscStoreObstruction> {
        let bytes = self.read_array::<8>()?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_usize(&mut self) -> Result<usize, WscStoreObstruction> {
        usize::try_from(self.read_u64()?)
            .map_err(|_| WscStoreObstruction::invalid_wsc(self.wsc_digest))
    }

    fn read_hash(&mut self) -> Result<Hash, WscStoreObstruction> {
        self.read_array::<32>()
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], WscStoreObstruction> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| WscStoreObstruction::invalid_wsc(self.wsc_digest))?;
        let slice = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| WscStoreObstruction::invalid_wsc(self.wsc_digest))?;
        self.offset = end;
        Ok(slice)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], WscStoreObstruction> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or_else(|| WscStoreObstruction::invalid_wsc(self.wsc_digest))?;
        let slice = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| WscStoreObstruction::invalid_wsc(self.wsc_digest))?;
        let mut out = [0; N];
        out.copy_from_slice(slice);
        self.offset = end;
        Ok(out)
    }

    fn finish(self) -> Result<(), WscStoreObstruction> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(WscStoreObstruction::invalid_wsc(self.wsc_digest))
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

fn read_marker_array<const N: usize>(
    bytes: &[u8],
    offset: usize,
    envelope_id: WscStoreEnvelopeId,
) -> Result<[u8; N], WscStoreObstruction> {
    read_array(bytes, offset).map_err(|_| WscStoreObstruction::invalid_commit_marker(envelope_id))
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
    derive_commit_marker_digest_from_parts(
        envelope.id(),
        envelope.record_kind(),
        envelope.basis_digest(),
        envelope.schema_hash(),
        envelope.tick(),
        envelope.wsc_digest(),
        encoded_len,
    )
}

fn derive_commit_marker_digest_from_parts(
    envelope_id: WscStoreEnvelopeId,
    record_kind: WscStoreRecordKind,
    basis_digest: &Hash,
    schema_hash: &Hash,
    tick: u64,
    wsc_digest: &Hash,
    encoded_len: u64,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_STORE_COMMIT_MARKER_DOMAIN);
    hasher.update(&envelope_id.as_hash());
    hasher.update(&record_kind.code().to_le_bytes());
    hasher.update(basis_digest);
    hasher.update(schema_hash);
    hasher.update(&tick.to_le_bytes());
    hasher.update(wsc_digest);
    hasher.update(&encoded_len.to_le_bytes());
    hasher.finalize().into()
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), WscStoreObstruction> {
    let parent = path
        .parent()
        .ok_or_else(|| WscStoreObstruction::filesystem_io(path))?;
    fs::create_dir_all(parent).map_err(|_| WscStoreObstruction::filesystem_io(parent))?;
    let temp_path = path.with_extension("tmp");
    {
        let mut file =
            File::create(&temp_path).map_err(|_| WscStoreObstruction::filesystem_io(&temp_path))?;
        file.write_all(bytes)
            .map_err(|_| WscStoreObstruction::filesystem_io(&temp_path))?;
        file.sync_all()
            .map_err(|_| WscStoreObstruction::filesystem_io(&temp_path))?;
    }
    fs::rename(&temp_path, path).map_err(|_| WscStoreObstruction::filesystem_io(path))?;
    sync_directory(parent)
}

fn sync_directory(path: &Path) -> Result<(), WscStoreObstruction> {
    #[cfg(unix)]
    {
        let dir = File::open(path).map_err(|_| WscStoreObstruction::filesystem_io(path))?;
        dir.sync_all()
            .map_err(|_| WscStoreObstruction::filesystem_io(path))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

fn envelope_id_from_commit_marker_path(path: &Path) -> Option<WscStoreEnvelopeId> {
    let name = path.file_name()?.to_str()?;
    let hex = name.strip_suffix(".ecwsc.commit")?;
    parse_hash_hex(hex).map(WscStoreEnvelopeId::from_hash)
}

fn hash_hex(hash: &Hash) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(hash.len() * 2);
    for byte in hash {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

fn parse_hash_hex(hex: &str) -> Option<Hash> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0; 32];
    for (index, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(chunk[0])?;
        let low = hex_nibble(chunk[1])?;
        out[index] = (high << 4) | low;
    }
    Some(out)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn digest_filesystem_path(path: &Path) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(WSC_STORE_FILESYSTEM_PATH_DOMAIN);
    hasher.update(path.to_string_lossy().as_bytes());
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
        if let Some(existing) = by_receipt.get(&record.receipt_ref) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.receipt_ref.identity_digest()),
                ));
            }
        }
        if let Some(existing) = by_submission.get(&record.receipt_ref.submission_id) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.receipt_ref.submission_id),
                ));
            }
        }
        if let Some(existing) = by_ticket.get(&record.receipt_ref.ticket_digest) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.receipt_ref.ticket_digest),
                ));
            }
        }
        by_receipt.insert(record.receipt_ref, *record);
        by_submission.insert(record.receipt_ref.submission_id, *record);
        by_ticket.insert(record.receipt_ref.ticket_digest, *record);
    }
    Ok(by_receipt.into_values().collect())
}

fn canonical_receipt_correlations(
    records: &[WalReceiptCorrelationRecord],
) -> Result<Vec<WalReceiptCorrelationRecord>, WscStoreObstruction> {
    let mut by_correlation = BTreeMap::new();
    let mut by_submission = BTreeMap::new();
    let mut by_ticket = BTreeMap::new();
    for record in records {
        if let Some(existing) = by_submission.get(&record.receipt_ref.submission_id) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.receipt_ref.submission_id),
                ));
            }
        }
        if let Some(existing) = by_ticket.get(&record.receipt_ref.ticket_digest) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.receipt_ref.ticket_digest),
                ));
            }
        }
        if let Some(existing) = by_correlation.get(&record.receipt_ref) {
            if existing != record {
                return Err(WscStoreObstruction::duplicate_mismatch(
                    WscStoreEnvelopeId::from_hash(record.receipt_ref.identity_digest()),
                ));
            }
        }
        by_correlation.insert(record.receipt_ref, record.clone());
        by_submission.insert(record.receipt_ref.submission_id, record.clone());
        by_ticket.insert(record.receipt_ref.ticket_digest, record.clone());
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
    let receipt_keys = receipts
        .iter()
        .map(|record| record.receipt_ref)
        .collect::<BTreeSet<_>>();
    let correlation_keys = correlations
        .iter()
        .map(|record| record.receipt_ref)
        .collect::<BTreeSet<_>>();
    for receipt in receipts {
        if !accepted_submissions.contains(&receipt.receipt_ref.submission_id) {
            return Err(WscStoreObstruction::incomplete_causal_history(
                receipt.receipt_ref.identity_digest(),
            ));
        }
        if !correlation_keys.contains(&receipt.receipt_ref) {
            return Err(WscStoreObstruction::incomplete_causal_history(
                receipt.receipt_ref.identity_digest(),
            ));
        }
    }
    for correlation in correlations {
        if !accepted_submissions.contains(&correlation.receipt_ref.submission_id) {
            return Err(WscStoreObstruction::incomplete_causal_history(
                correlation.receipt_ref.identity_digest(),
            ));
        }
        if !receipt_keys.contains(&correlation.receipt_ref) {
            return Err(WscStoreObstruction::incomplete_causal_history(
                correlation.receipt_ref.identity_digest(),
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

fn receipt_node_id(receipt_ref_digest: &Hash) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RECEIPT_CORRELATION_NODE_DOMAIN);
    hasher.update(b"receipt");
    hasher.update(receipt_ref_digest);
    NodeId(hasher.finalize().into())
}

fn correlation_node_id(receipt_ref_digest: &Hash) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(WSC_RECEIPT_CORRELATION_NODE_DOMAIN);
    hasher.update(b"correlation");
    hasher.update(receipt_ref_digest);
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
