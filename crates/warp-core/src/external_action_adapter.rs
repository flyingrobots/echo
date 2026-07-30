// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Compiler-authored external requests and capability-rooted observation.
//!
//! This module is a trusted-host boundary. Edict contributes canonical request
//! data but receives no filesystem authority. The adapter accepts work only
//! through an [`ExternalActionClaimGrantV1`], which is constructible only after
//! Echo durably commits the corresponding request and claim.

use std::path::Path;

use cap_std::fs::Dir;
use echo_edict_canonical::CanonicalValueErrorKind;
use thiserror::Error;

use crate::causal_wal::WalStorePort;
use crate::external_action::{
    AdmittedExternalActionSettlementV1, ExternalActionAdapterBindingV1, ExternalActionAdapterIdV1,
    ExternalActionClaimGrantV1, ExternalActionCoordinatorV1, ExternalActionOperationIdV1,
    ExternalActionProtocolErrorV1, ExternalActionRequestV1, ExternalActionSettlementCandidateV1,
    ExternalActionTransactionContextV1,
};
use crate::{Hash, WorldlineId};

/// One canonical Edict request admitted from exact Core and Target IR bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedEdictExternalActionRequestV1 {
    request: ExternalActionRequestV1,
    canonical_operation_input: Vec<u8>,
    source_core_digest: String,
    target_ir_digest: String,
    operation_coordinate: String,
}

impl AdmittedEdictExternalActionRequestV1 {
    /// Returns the generic Echo request derived from compiler-owned data.
    #[must_use]
    pub const fn request(&self) -> ExternalActionRequestV1 {
        self.request
    }

    /// Returns the exact canonical operation input resolved from application input.
    #[must_use]
    pub fn canonical_operation_input(&self) -> &[u8] {
        &self.canonical_operation_input
    }

    /// Returns the independently recomputed reviewed Core digest.
    #[must_use]
    pub fn source_core_digest(&self) -> &str {
        &self.source_core_digest
    }

    /// Returns the independently recomputed reviewed Target IR digest.
    #[must_use]
    pub fn target_ir_digest(&self) -> &str {
        &self.target_ir_digest
    }

    /// Returns the compiler-declared operation coordinate for diagnostics.
    #[must_use]
    pub fn operation_coordinate(&self) -> &str {
        &self.operation_coordinate
    }
}

/// Independent compiler-artifact admission failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EdictExternalActionAdmissionErrorV1 {
    /// The canonical decoder rejected one compiler artifact or runtime value.
    #[error("canonical Edict value was rejected: {0:?}")]
    Canonical(CanonicalValueErrorKind),
    /// The Target IR shape is not the exact supported request-only contract.
    #[error("Edict Target IR artifact shape is unsupported")]
    ArtifactShape,
    /// The requested intent is absent.
    #[error("Edict Target IR intent is absent")]
    MissingIntent,
    /// The intent does not contain exactly one external request.
    #[error("Edict Target IR intent must contain exactly one external request")]
    RequestCardinality,
    /// A request-only artifact also contains callable target steps.
    #[error("Edict external-request Target IR contains callable target steps")]
    CallableStepsPresent,
    /// The Target IR omitted its exact source/capability closure.
    #[error("Edict Target IR semantic closure is absent")]
    MissingSemanticClosure,
    /// The supplied Core bytes do not match the Target IR source-Core binding.
    #[error("Edict source Core digest does not match Target IR")]
    CoreDigestMismatch,
    /// The request operation is absent or substituted in the capability closure.
    #[error("Edict external request is outside the exact capability closure")]
    CapabilityClosureMismatch,
    /// The supported runtime expression subset could not resolve a request field.
    #[error("Edict external request uses an unsupported runtime expression")]
    UnsupportedExpression,
    /// A runtime value has the wrong type or bounds.
    #[error("Edict external request runtime value is invalid")]
    InvalidRuntimeValue,
    /// A reviewed digest does not use the required lowercase SHA-256 rendering.
    #[error("Edict external request contains an invalid reviewed digest")]
    InvalidDigest,
    /// The generic request protocol rejected the compiler-derived request.
    #[error(transparent)]
    Protocol(#[from] ExternalActionProtocolErrorV1),
    /// RED boundary until the independent admission implementation lands.
    #[error("compiler-authored external-request admission is not implemented")]
    Unavailable,
}

/// Admits one request-only Edict Target IR artifact without invoking a provider.
#[allow(clippy::too_many_arguments)]
pub fn admit_edict_external_action_request_v1(
    _worldline_id: WorldlineId,
    _canonical_core_bytes: &[u8],
    _canonical_target_ir_bytes: &[u8],
    _intent_name: &str,
    _canonical_application_input_bytes: &[u8],
) -> Result<AdmittedEdictExternalActionRequestV1, EdictExternalActionAdmissionErrorV1> {
    Err(EdictExternalActionAdmissionErrorV1::Unavailable)
}

/// Runtime-owner profile binding a validator and adapter to compiler identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundedWorkspaceObservationProfileV1 {
    /// Exact compiler-declared operation family.
    pub operation_id: ExternalActionOperationIdV1,
    /// Exact compiler-declared input schema.
    pub input_schema_digest: Hash,
    /// Exact compiler-declared settlement schema.
    pub settlement_schema_digest: Hash,
    /// Exact compiler-declared reconciliation law.
    pub reconciliation_law_digest: Hash,
    /// Maximum authority scope delegated by the runtime owner.
    pub authority_scope_digest: Hash,
    /// Runtime-owned adapter identity.
    pub adapter_id: ExternalActionAdapterIdV1,
}

/// Bounded, read-only host adapter rooted in one capability directory.
pub struct BoundedWorkspaceObservationAdapterV1 {
    _root: Dir,
    permitted_paths: Vec<String>,
    profile: BoundedWorkspaceObservationProfileV1,
}

impl BoundedWorkspaceObservationAdapterV1 {
    /// Opens the configured root once and retains only directory-relative authority.
    pub fn open(
        _root: &Path,
        _permitted_paths: impl IntoIterator<Item = String>,
        _profile: BoundedWorkspaceObservationProfileV1,
    ) -> Result<Self, BoundedWorkspaceObservationErrorV1> {
        Err(BoundedWorkspaceObservationErrorV1::Unavailable)
    }

    /// Returns the runtime registry binding for this attenuated adapter.
    #[must_use]
    pub const fn adapter_binding(&self) -> ExternalActionAdapterBindingV1 {
        ExternalActionAdapterBindingV1 {
            adapter_id: self.profile.adapter_id,
            operation_id: self.profile.operation_id,
            authority_scope_digest: self.profile.authority_scope_digest,
        }
    }

    /// Performs one bounded observation after request and claim durability.
    pub fn observe(
        &self,
        _grant: &ExternalActionClaimGrantV1,
        _admitted: &AdmittedEdictExternalActionRequestV1,
    ) -> Result<ExternalActionSettlementCandidateV1, BoundedWorkspaceObservationErrorV1> {
        let _ = &self.permitted_paths;
        Err(BoundedWorkspaceObservationErrorV1::Unavailable)
    }

    /// Produces an explicit ambiguous settlement during reconciliation.
    pub fn outcome_unknown(
        &self,
        _grant: &ExternalActionClaimGrantV1,
        _admitted: &AdmittedEdictExternalActionRequestV1,
        _external_evidence_digest: Hash,
    ) -> Result<ExternalActionSettlementCandidateV1, BoundedWorkspaceObservationErrorV1> {
        Err(BoundedWorkspaceObservationErrorV1::Unavailable)
    }

    /// Validates the operation-specific schema and durably admits the settlement.
    pub fn admit_settlement(
        &self,
        _store: &mut impl WalStorePort,
        _coordinator: &mut ExternalActionCoordinatorV1,
        _context: ExternalActionTransactionContextV1,
        _grant: ExternalActionClaimGrantV1,
        _candidate: ExternalActionSettlementCandidateV1,
    ) -> Result<AdmittedExternalActionSettlementV1, BoundedWorkspaceObservationErrorV1> {
        Err(BoundedWorkspaceObservationErrorV1::Unavailable)
    }
}

/// Encodes the operation-specific path request as canonical CBOR.
pub fn encode_bounded_workspace_observation_input_v1(
    _paths: impl IntoIterator<Item = String>,
) -> Result<Vec<u8>, BoundedWorkspaceObservationErrorV1> {
    Err(BoundedWorkspaceObservationErrorV1::Unavailable)
}

/// Computes the basis committed by a sorted path/content snapshot.
pub fn bounded_workspace_observation_basis_v1<'a>(
    _entries: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> Hash {
    [0; 32]
}

/// Stable adapter/profile failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BoundedWorkspaceObservationErrorV1 {
    /// The canonical input or settlement encoding is invalid.
    #[error("bounded observation canonical value is invalid")]
    Canonical,
    /// The claim, compiler request, and adapter profile do not identify one request.
    #[error("bounded observation grant does not match the admitted compiler request")]
    GrantMismatch,
    /// The compiler request is outside the runtime-owned adapter profile.
    #[error("bounded observation request does not match adapter policy")]
    ProfileMismatch,
    /// An input path is empty, absolute, non-normalized, or traverses a parent.
    #[error("bounded observation path is invalid")]
    InvalidPath,
    /// The requested relative path is outside the attenuated path set.
    #[error("bounded observation path is unauthorized")]
    UnauthorizedPath,
    /// A requested path or any traversed component is a symlink.
    #[error("bounded observation refuses symlinks")]
    SymlinkRefused,
    /// A requested path is not one regular file.
    #[error("bounded observation requires regular files")]
    NotRegularFile,
    /// The observed bytes do not match the request's exact basis.
    #[error("bounded observation basis is stale")]
    StaleBasis,
    /// The canonical settlement exceeds the compiler-delegated bound.
    #[error("bounded observation settlement exceeds its byte budget")]
    SettlementBudgetExceeded,
    /// The candidate did not pass the operation-specific settlement validator.
    #[error("bounded observation settlement schema admission failed")]
    SchemaAdmissionFailed,
    /// Capability-rooted filesystem access failed.
    #[error("bounded observation filesystem access failed")]
    Io,
    /// The generic durable protocol refused the transition.
    #[error(transparent)]
    Protocol(#[from] ExternalActionProtocolErrorV1),
    /// RED boundary until the capability-rooted adapter implementation lands.
    #[error("bounded observation adapter is not implemented")]
    Unavailable,
}
