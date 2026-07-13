// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Installed-contract inverse intent boundary.
//!
//! Echo resolves retained causal evidence and admits the resulting intent. The
//! installed contract owns inverse semantics and receives only read-only
//! runtime/provenance access through this boundary.

use std::sync::Arc;

use thiserror::Error;

use crate::{
    CausalTickReceiptRef, ContractEvidenceIdentity, ContractOperationKind, Hash, IngressTarget,
    InstalledContractPackageId, IntentKind, ProvenanceService, WorldlineId, WorldlineRuntime,
    WorldlineTick,
};

/// App request to derive and durably witness one contract-defined inverse intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractInverseAdmissionRequest {
    /// Exact retained receipt coordinate of the transition to invert.
    pub target_receipt_ref: CausalTickReceiptRef,
    /// Current routing coordinate where the inverse intent must be admitted.
    pub current_target: IngressTarget,
    /// Current frontier the application observed when requesting the inverse.
    pub expected_current_frontier_tick: WorldlineTick,
    /// Canonical application-defined inverse policy bytes.
    pub policy_bytes: Vec<u8>,
}

/// Canonical mutation intent produced by an installed contract inverse law.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractInverseIntent {
    /// Generated mutation operation id to admit.
    pub op_id: u32,
    /// Canonical generated variables for the mutation operation.
    pub vars_bytes: Vec<u8>,
}

impl ContractInverseIntent {
    /// Builds one canonical contract mutation intent.
    #[must_use]
    pub fn new(op_id: u32, vars_bytes: Vec<u8>) -> Self {
        Self { op_id, vars_bytes }
    }
}

/// Installed read-only inverse law for one generated mutation operation.
#[derive(Clone)]
pub struct ContractInverseHandler {
    /// Original generated mutation operation id this law can invert.
    pub target_op_id: u32,
    /// Read-only inverse resolver.
    pub resolve: ContractInverseResolveFn,
}

impl ContractInverseHandler {
    /// Builds an installed inverse law from a read-only host closure.
    pub fn new<F>(target_op_id: u32, resolve: F) -> Self
    where
        F: for<'a> Fn(
                ContractInverseContext<'a>,
            ) -> Result<ContractInverseIntent, ContractInverseHandlerError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            target_op_id,
            resolve: Arc::new(resolve),
        }
    }
}

/// Read-only installed contract inverse resolver function.
pub type ContractInverseResolveFn = Arc<
    dyn for<'a> Fn(
            ContractInverseContext<'a>,
        ) -> Result<ContractInverseIntent, ContractInverseHandlerError>
        + Send
        + Sync
        + 'static,
>;

/// Durable target evidence and current basis passed to a contract inverse law.
pub struct ContractInverseContext<'a> {
    /// Exact causal receipt coordinate selected by the application.
    pub target_receipt_ref: CausalTickReceiptRef,
    /// Witnessed submission whose admitted transition is being inverted.
    pub target_submission_id: Hash,
    /// Installed contract evidence retained on the target transition.
    pub target_contract: &'a ContractEvidenceIdentity,
    /// Original retained ingress kind.
    pub target_intent_kind: IntentKind,
    /// Original generated mutation operation id.
    pub target_op_id: u32,
    /// Original canonical generated variables.
    pub target_vars_bytes: &'a [u8],
    /// Original routing coordinate retained with the target submission.
    pub target_ingress_target: &'a IngressTarget,
    /// Current routing coordinate selected for the inverse.
    pub current_target: &'a IngressTarget,
    /// Validated current worldline frontier tick.
    pub current_frontier_tick: WorldlineTick,
    /// Exact receipt set that constitutes the current frontier commit basis.
    pub current_basis_receipt_refs: &'a [CausalTickReceiptRef],
    /// Canonical application-defined inverse policy bytes.
    pub policy_bytes: &'a [u8],
    /// Read-only runtime evidence available to the contract law.
    pub runtime: &'a WorldlineRuntime,
    /// Read-only provenance evidence available to the contract law.
    pub provenance: &'a ProvenanceService,
}

/// Typed refusal emitted by an installed contract inverse law.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ContractInverseHandlerError {
    /// Retained material required to construct the inverse is unavailable.
    #[error("contract inverse fragment unavailable: {message}")]
    InverseFragmentUnavailable {
        /// Deterministic contract-provided explanation.
        message: String,
    },
    /// The original causal span cannot be mapped to the current frontier.
    #[error("contract inverse causal span unmappable: {message}")]
    CausalSpanUnmappable {
        /// Deterministic contract-provided explanation.
        message: String,
    },
    /// Retained compressed history must be rehydrated before inversion.
    #[error("contract inverse requires retained history rehydration: {message}")]
    HistoryRehydrationRequired {
        /// Deterministic contract-provided explanation.
        message: String,
    },
    /// The contract rejected the request for another typed application reason.
    #[error("contract inverse obstructed ({code}): {message}")]
    ContractDefined {
        /// Stable application-defined obstruction code.
        code: String,
        /// Deterministic contract-provided explanation.
        message: String,
    },
}

impl ContractInverseHandlerError {
    /// Builds a typed unavailable-fragment obstruction.
    #[must_use]
    pub fn inverse_fragment_unavailable(message: impl Into<String>) -> Self {
        Self::InverseFragmentUnavailable {
            message: message.into(),
        }
    }

    /// Builds a typed unmappable-causal-span obstruction.
    #[must_use]
    pub fn causal_span_unmappable(message: impl Into<String>) -> Self {
        Self::CausalSpanUnmappable {
            message: message.into(),
        }
    }

    /// Builds a typed rehydration-required obstruction.
    #[must_use]
    pub fn history_rehydration_required(message: impl Into<String>) -> Self {
        Self::HistoryRehydrationRequired {
            message: message.into(),
        }
    }

    /// Builds a stable application-defined inverse obstruction.
    #[must_use]
    pub fn contract_defined(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ContractDefined {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Echo-side obstruction while resolving or admitting a contract inverse.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ContractInverseObstruction {
    /// No retained transition matched the exact causal receipt coordinate.
    #[error("contract inverse target receipt is unavailable: {target_receipt_ref:?}")]
    TargetReceiptUnavailable {
        /// Requested exact target receipt coordinate.
        target_receipt_ref: Box<CausalTickReceiptRef>,
    },
    /// The target transition names submission material that is not retained.
    #[error("contract inverse target submission is unavailable: {submission_id:?}")]
    TargetSubmissionUnavailable {
        /// Missing witnessed submission id.
        submission_id: Hash,
    },
    /// The retained target submission is not a canonical generated mutation intent.
    #[error("contract inverse target intent is malformed: {submission_id:?}")]
    TargetIntentMalformed {
        /// Malformed witnessed submission id.
        submission_id: Hash,
    },
    /// The target transition lacks installed contract evidence.
    #[error("contract inverse target lacks contract evidence: {target_receipt_ref:?}")]
    TargetContractEvidenceUnavailable {
        /// Requested exact target receipt coordinate.
        target_receipt_ref: Box<CausalTickReceiptRef>,
    },
    /// Retained contract evidence names a different operation than the target intent.
    #[error(
        "contract inverse target operation mismatch: retained {retained_op_id}, envelope {envelope_op_id}"
    )]
    TargetOperationMismatch {
        /// Operation id retained with receipt evidence.
        retained_op_id: u32,
        /// Operation id decoded from the witnessed envelope.
        envelope_op_id: u32,
    },
    /// Retained evidence does not describe a mutation operation.
    #[error("contract inverse target is not a mutation operation: {actual:?}")]
    TargetOperationKindMismatch {
        /// Retained operation kind.
        actual: ContractOperationKind,
    },
    /// The currently installed host has no mutation package for the target operation.
    #[error("contract inverse target operation is not installed: {target_op_id}")]
    InstalledContractUnavailable {
        /// Original mutation operation id.
        target_op_id: u32,
    },
    /// The installed artifact does not exactly match target transition evidence.
    #[error(
        "contract inverse artifact mismatch: retained {retained_package_id:?}, installed {installed_package_id:?}"
    )]
    ContractVersionMismatch {
        /// Package id retained on the original transition.
        retained_package_id: InstalledContractPackageId,
        /// Package id currently installed for the operation.
        installed_package_id: InstalledContractPackageId,
    },
    /// The matching installed contract has no inverse law for the operation.
    #[error("contract inverse handler is unavailable for operation {target_op_id}")]
    InverseHandlerUnavailable {
        /// Original mutation operation id.
        target_op_id: u32,
    },
    /// The current target worldline does not exist.
    #[error("contract inverse current worldline is unavailable: {worldline_id:?}")]
    CurrentWorldlineUnavailable {
        /// Requested current target worldline.
        worldline_id: WorldlineId,
    },
    /// The request was based on a stale current frontier.
    #[error(
        "contract inverse current basis mismatch: expected {expected:?}, observed {observed:?}"
    )]
    CurrentBasisMismatch {
        /// Frontier tick observed by the application.
        expected: WorldlineTick,
        /// Frontier tick owned by Echo when resolving the request.
        observed: WorldlineTick,
    },
    /// Echo could not resolve provenance for the requested current frontier.
    #[error(
        "contract inverse current provenance unavailable for worldline {worldline_id:?} frontier {frontier_tick:?}"
    )]
    CurrentBasisProvenanceUnavailable {
        /// Current target worldline.
        worldline_id: WorldlineId,
        /// Current frontier tick whose provenance was unavailable.
        frontier_tick: WorldlineTick,
    },
    /// The current commit has no retained receipt coordinate that can be cited.
    #[error(
        "contract inverse current receipt basis unavailable for worldline {worldline_id:?} frontier {frontier_tick:?} commit {commit_hash:?}"
    )]
    CurrentBasisReceiptUnavailable {
        /// Current target worldline.
        worldline_id: WorldlineId,
        /// Current frontier tick whose receipt basis was unavailable.
        frontier_tick: WorldlineTick,
        /// Provenance commit that could not be joined to retained receipts.
        commit_hash: Hash,
    },
    /// The inverse law emitted an operation that is not installed as a mutation.
    #[error("contract inverse emitted uninstalled mutation operation {op_id}")]
    ProducedMutationUnavailable {
        /// Emitted operation id.
        op_id: u32,
    },
    /// The inverse law emitted an operation owned by another contract package.
    #[error(
        "contract inverse emitted cross-package mutation {op_id}: target {target_package_id:?}, emitted {emitted_package_id:?}"
    )]
    ProducedMutationContractMismatch {
        /// Emitted operation id.
        op_id: u32,
        /// Package that owns the target transition.
        target_package_id: InstalledContractPackageId,
        /// Package that owns the emitted operation.
        emitted_package_id: InstalledContractPackageId,
    },
    /// The emitted canonical mutation envelope could not be encoded.
    #[error("contract inverse emitted invalid mutation variables for operation {op_id}")]
    ProducedIntentEncodingFailed {
        /// Emitted operation id.
        op_id: u32,
    },
    /// The installed contract law returned a typed domain obstruction.
    #[error(transparent)]
    Contract(#[from] ContractInverseHandlerError),
}
