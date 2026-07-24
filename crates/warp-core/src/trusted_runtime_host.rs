// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Reference trusted runtime host loop.
//!
//! This module names the local host role for the v0.1.0 contract path. It is a
//! convenience wrapper around existing Echo runtime pieces; it does not create a
//! daemon, does not make wall-clock cadence semantic, and does not give
//! application code tick authority.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::causal_anchor::prepare_causal_anchor_admission;

use crate::{
    causal_wal::{
        affected_frontiers_root, build_causal_anchor_admission_transaction,
        build_executable_operation_installation_transaction,
        build_executable_operation_tick_transaction, build_recovery_certificate,
        build_replayable_tick_batch_transaction, build_replayable_tick_transaction,
        build_submission_acceptance_with_material_transaction,
        causal_anchor_frontier_digest_from_evidence, causal_anchor_genesis_frontier_digest,
        causal_history_genesis_frontier_digest, decode_tick_receipt_records,
        logical_causal_history_frontier_digest, recover_filesystem_store,
        recover_from_frames_and_commits, recover_receipt_index, recover_submission_index,
        recovered_submission_receipt_index_root, tick_receipt_payload_is_batch,
        trusted_runtime_wal_digest, validate_recovered_causal_anchor_history, AffectedFrontier,
        AffectedFrontierKind, FilesystemWalStore, InMemoryWalStore, Lsn, PayloadCodecId,
        PayloadSchemaId, RecoveredCausalAnchorAdmission, RecoveredReceiptIndex,
        RecoveredSubmissionIndex, RecoveryAccessMode, RecoveryCertificate, RecoveryScanReport,
        SubmissionAcceptanceRecord, SubmissionRetryPosture, TickReceiptRecord, WalAppendAuthority,
        WalBuildError, WalCommittedTransaction, WalDecodeError, WalDurabilityMode,
        WalReceiptCorrelationRecord, WalRecordKind, WalRecoveryError, WalRecoveryIndexError,
        WalRuntimeStateDeltaRecord, WalSegmentId, WalStoreError, WalStorePort,
        WalSubmissionEnvelopeRecord, WalTickDecision, WalTransactionBuilder, WalTransactionCommit,
        WalTransactionId, WalTransactionKind, WriterEpochId, WriterEpochRequest,
        TRUSTED_RUNTIME_WAL_DOMAIN,
    },
    contract_host::{decode_canonical_eint, encode_canonical_eint},
    echo_operation::{
        action_admission_evidence_matches_v1, action_application_basis_matches_state_v1,
        action_batch_composition_digest_from_receipts_v1, action_preparation_identity_matches_v1,
        admit_action_invocation_v1, admit_invocation_v1, admit_package_v1,
        commit_prepared_to_state, decode_invocation_route_v1,
        echo_operation_action_invocation_bytes_v1, inspect_action_invocation_v1,
        install_recovered_v1, installed_from_admitted, not_committed_basis_changed,
        not_committed_evaluation_authority_mismatch, not_committed_installation_unavailable,
        operation_descent_stack, prepare_operation_v1, reconstruct_action_preparation_v1,
        recover_action_outcome_v1, recover_committed_execution_receipt_v1, recover_installation_v1,
        retain_action_outcome_v1, retain_committed_execution_v1, retain_installation_v1,
        validate_receipt_installation_v1, EchoOperationEvaluationAuthorityV1,
    },
    provider_contract::admit_provider_contract_package_v1,
    AdmittedEchoOperationInvocationV1, AdmittedExecutableOperationPackageV1,
    AdmittedProviderContractPackageV1, CausalAnchorAdmissionRequest, CausalAnchorClaim,
    CausalAnchorError, CausalAnchorId, CausalAnchorRootSupportPolicy, CausalAnchorSupportError,
    CausalFrontierRef, ContractInverseAdmissionRequest, ContractInverseContext,
    ContractInverseDerivation, ContractInverseHistoryObstruction, ContractInverseObstruction,
    ContractOperationKind, EchoOperationActionOutcomeV1, EchoOperationAdmissionErrorV1,
    EchoOperationAdmissionPolicyV1, EchoOperationApplicationBasisV1, EchoOperationArtifactErrorV1,
    EchoOperationCommitErrorV1, EchoOperationEvaluationBasisV1, EchoOperationExecutionEvidenceV1,
    EchoOperationInstallationErrorV1, EchoOperationInvocationAdmissionErrorKindV1,
    EchoOperationInvocationAdmissionErrorV1, EchoOperationInvocationAdmissionPolicyV1,
    EchoOperationPreparationV1, EchoOperationReceiptV1, EchoOperationTerminalPostureV1, Engine,
    IngressCausalParent, IngressEnvelope, IngressEnvelopeDecodeError, IngressPayload,
    IngressSubmissionGeneration, InstalledContractPackage, InstalledContractPackageError,
    InstalledContractPackageRecord, InstalledEchoOperationV1,
    InstalledProviderContractPackageRecordV1, IntentOutcome, IntentOutcomeDecision,
    IntentOutcomeObservation, IntentSubmissionHandle, IntentSubmissionRecord, ObservationArtifact,
    ObservationError, ObservationRequest, ObservationService, OpticAdmissionTicket,
    PreparedEchoOperationV1, ProvenanceEntry, ProvenanceService, ProvenanceStore,
    ProviderContractAdmissionError, ProviderContractAdmissionPolicyV1,
    ProviderContractInstallationError, ProviderContractPackageInstallerV1,
    ProviderContractPackageProposalV1, ProviderPackageReferenceV1,
    ReceiptCorrelationPersistenceRecord, ReceiptCorrelationRecord, RetainedProvenanceError,
    RuntimeError, SchedulerCoordinator, StepRecord, TickReceiptDisposition, TickReceiptRejection,
    TicketedRuntimeIngressAuthority, TicketedRuntimeIngressDisposition,
    WitnessedSubmissionPersistenceRecord, WitnessedSubmissionPersistenceSnapshot, WorldlineRuntime,
};
use crate::{Hash, HistoryError};

#[cfg(any(test, feature = "host_test"))]
use crate::causal_wal::FilesystemWalFaultPlan;

const INSTALLED_CONTRACT_HOST_ADMISSION_DIGEST_DOMAIN: &[u8] =
    b"echo:trusted-host-installed-contract-admission:v1\0";
const PROVIDER_CONTRACT_HOST_ADMISSION_DIGEST_DOMAIN: &[u8] =
    b"echo:trusted-host-provider-contract-admission:v1\0";
const ECHO_OPERATION_ACTION_ADMISSION_DIGEST_DOMAIN: &[u8] =
    b"echo:trusted-host-executable-operation-action-admission:v1\0";

/// Error returned by the reference trusted host loop.
#[derive(Debug, Error)]
pub enum TrustedRuntimeHostError {
    /// Provenance initialization failed.
    #[error("trusted runtime host provenance error: {0}")]
    Provenance(#[from] HistoryError),
    /// Scheduler/runtime work failed.
    #[error("trusted runtime host runtime error: {0}")]
    Runtime(Box<RuntimeError>),
    /// The app used the WAL-backed ACK path before a runtime WAL was configured.
    #[error("trusted runtime host runtime WAL is unavailable")]
    RuntimeWalUnavailable,
    /// No host-owned generic root-support policy is installed for anchor admission.
    #[error("trusted runtime host causal-anchor support policy is unavailable")]
    CausalAnchorSupportPolicyUnavailable,
    /// The application supplied a malformed causal-anchor claim.
    #[error("trusted runtime host causal-anchor claim error: {0}")]
    CausalAnchorClaim(#[from] CausalAnchorError),
    /// The host-owned generic root-support policy refused the anchor claim.
    #[error("trusted runtime host causal-anchor support error: {0}")]
    CausalAnchorSupport(#[from] CausalAnchorSupportError),
    /// The requested anchor basis is not the current durable causal frontier.
    #[error("trusted runtime host causal-anchor basis is stale")]
    CausalAnchorBasisStale {
        /// Basis named by the application request.
        requested: Hash,
        /// Current logical frontier derived from durable runtime history.
        current: Hash,
    },
    /// Contract-defined inverse resolution was causally obstructed.
    #[error("trusted runtime host contract inverse obstruction: {0}")]
    ContractInverse(#[from] ContractInverseObstruction),
    /// Runtime WAL append or build failed.
    #[error("trusted runtime host WAL error: {0}")]
    Wal(#[from] TrustedRuntimeWalError),
    /// The host reached its caller-supplied scheduler-pass bound before idling.
    #[error("trusted runtime host exceeded scheduler pass limit: {max_scheduler_passes}")]
    SchedulerPassLimitExceeded {
        /// Maximum scheduler passes the caller allowed.
        max_scheduler_passes: u64,
    },
    /// A privately evaluated operation could not produce complete commit material.
    #[error("trusted runtime host executable-operation commit error: {0}")]
    EchoOperationCommit(#[from] EchoOperationCommitErrorV1),
    /// Exact executable-operation retained material was malformed.
    #[error("trusted runtime host executable-operation artifact error: {0}")]
    EchoOperationArtifact(#[from] EchoOperationArtifactErrorV1),
    /// Runtime-owned Action admission policy is not installed.
    #[error("trusted runtime host executable-operation Action admission policy is unavailable")]
    EchoOperationActionAdmissionPolicyUnavailable,
    /// Runtime-owned admission refused a durably accepted executable Action.
    #[error("trusted runtime host executable-operation Action admission error: {0}")]
    EchoOperationActionAdmission(#[from] EchoOperationInvocationAdmissionErrorV1),
    /// Executable-operation installation conflicted with installed authority.
    #[error("trusted runtime host executable-operation installation error: {0}")]
    EchoOperationInstallation(#[from] EchoOperationInstallationErrorV1),
}

impl From<RuntimeError> for TrustedRuntimeHostError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(Box::new(error))
    }
}

/// Error returned by the trusted runtime WAL adapter.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrustedRuntimeWalError {
    /// WAL transaction construction failed before storage append.
    #[error("trusted runtime WAL transaction build error: {0}")]
    Build(#[from] WalBuildError),
    /// WAL storage failed before durable acknowledgement.
    #[error("trusted runtime WAL store error: {0}")]
    Store(#[from] WalStoreError),
    /// WAL recovery failed while rebuilding runtime evidence.
    #[error("trusted runtime WAL recovery error: {0}")]
    Recovery(#[from] WalRecoveryError),
    /// A retained submission envelope could not be decoded.
    #[error("trusted runtime WAL retained envelope decode failed for submission {submission_id:?}: {error}")]
    SubmissionEnvelopeDecode {
        /// Submission whose retained envelope was malformed.
        submission_id: Hash,
        /// Typed retained-envelope codec error.
        error: IngressEnvelopeDecodeError,
    },
    /// Retained submission material disagreed with acceptance evidence.
    #[error("trusted runtime WAL retained envelope mismatched submission {submission_id:?}")]
    SubmissionEnvelopeMismatch {
        /// Submission whose retained material was inconsistent.
        submission_id: Hash,
    },
    /// Multiple retained records disagreed for one submission.
    #[error("trusted runtime WAL retained envelope conflicted for submission {submission_id:?}")]
    SubmissionEnvelopeConflict {
        /// Submission whose retained records conflicted.
        submission_id: Hash,
    },
    /// Recovery cannot restore an accepted submission without envelope material.
    #[error("trusted runtime WAL is missing retained envelope material for submission {submission_id:?}")]
    SubmissionEnvelopeMissing {
        /// Accepted submission whose canonical envelope was unavailable.
        submission_id: Hash,
    },
    /// Receipt correlation parents disagreed with the retained ingress envelope.
    #[error(
        "trusted runtime WAL causal parents mismatched submission {submission_id:?} receipt {receipt_ref_digest:?}"
    )]
    ReceiptCorrelationCausalParentsMismatch {
        /// Submission whose retained causal-parent claims disagreed.
        submission_id: Hash,
        /// Identity digest of the receipt carrying the conflicting correlation.
        receipt_ref_digest: Hash,
    },
    /// A replayable runtime state delta could not be encoded or decoded.
    #[error("trusted runtime WAL retained state-delta codec error: {0}")]
    RuntimeStateDeltaCodec(#[from] RetainedProvenanceError),
    /// Recovery cannot restore a decided receipt without replayable state material.
    #[error(
        "trusted runtime WAL is missing replayable state material for receipt {receipt_digest:?}"
    )]
    RuntimeStateDeltaMissing {
        /// Receipt whose committed state transition lacks replayable material.
        receipt_digest: Hash,
    },
    /// Two retained state deltas disagreed at one worldline tick.
    #[error("trusted runtime WAL retained state delta conflicted for worldline {worldline_id:?} tick {worldline_tick:?}")]
    RuntimeStateDeltaConflict {
        /// Worldline whose retained transition conflicted.
        worldline_id: crate::WorldlineId,
        /// Tick whose retained transition conflicted.
        worldline_tick: crate::WorldlineTick,
    },
    /// Retained executable-operation material failed canonical self-validation.
    #[error("trusted runtime WAL executable-operation artifact error: {0}")]
    EchoOperationArtifact(#[from] EchoOperationArtifactErrorV1),
    /// Recovered executable-operation installations conflicted.
    #[error("trusted runtime WAL executable-operation installation error: {0}")]
    EchoOperationInstallation(#[from] EchoOperationInstallationErrorV1),
    /// More than one committed transaction retained the same typed operation receipt.
    #[error(
        "trusted runtime WAL recovered duplicate executable-operation receipt {receipt_digest:?}"
    )]
    EchoOperationReceiptConflict {
        /// Content identity claimed by more than one operation-tick transaction.
        receipt_digest: Hash,
    },
    /// A committed operation receipt disagreed with its retained state transition.
    #[error("trusted runtime WAL executable-operation receipt/state mismatch: {detail}")]
    EchoOperationExecutionMismatch {
        /// Stable explanation of the mismatched commitments.
        detail: &'static str,
    },
    /// A committed operation transaction carried the wrong exact frontier transition.
    #[error(
        "trusted runtime WAL executable-operation frontier mismatched transaction {transaction_id:?}: expected {expected:?}, actual {actual:?}"
    )]
    EchoOperationFrontierMismatch {
        /// Operation transaction whose frontier commitment was inconsistent.
        transaction_id: Hash,
        /// Root recomputed from the exact retained operation material.
        expected: Hash,
        /// Affected-frontier root carried by the committed transaction.
        actual: Hash,
    },
    /// Causal-anchor recovery omitted evidence for an anchor transaction.
    #[error("trusted runtime WAL omitted recovered causal-anchor admission for transaction {transaction_id:?}")]
    CausalAnchorAdmissionMissing {
        /// Transaction whose canonical anchor evidence was unavailable.
        transaction_id: Hash,
    },
    /// A recovered anchor claim named a basis other than its transaction basis.
    #[error(
        "trusted runtime WAL causal-anchor basis mismatched transaction {transaction_id:?}: claimed {claimed:?}, recovered {recovered:?}"
    )]
    CausalAnchorBasisMismatch {
        /// Transaction carrying the inconsistent anchor admission.
        transaction_id: Hash,
        /// Basis encoded in the recovered anchor claim and receipt.
        claimed: Hash,
        /// Logical causal-history frontier immediately before the transaction.
        recovered: Hash,
    },
    /// Recovered anchor evidence was not bound to its exact frontier transition.
    #[error(
        "trusted runtime WAL causal-anchor frontier mismatched transaction {transaction_id:?}: expected {expected:?}, actual {actual:?}"
    )]
    CausalAnchorFrontierMismatch {
        /// Transaction carrying the unattested anchor-index transition.
        transaction_id: Hash,
        /// Root of the exact reconstructed causal-anchor frontier transition.
        expected: Hash,
        /// Affected-frontier root carried by the recovered commit marker.
        actual: Hash,
    },
    /// Distinct recovered admissions claimed the same stable anchor identity.
    #[error("trusted runtime WAL recovered duplicate causal-anchor id {anchor_id:?}")]
    CausalAnchorIdConflict {
        /// Anchor identity claimed by more than one admission.
        anchor_id: CausalAnchorId,
    },
    /// More than one recovered admission matched one canonical claim digest.
    #[error("trusted runtime WAL recovered ambiguous causal-anchor claim {claim_digest:?}")]
    CausalAnchorClaimConflict {
        /// Canonical claim digest with ambiguous recovered evidence.
        claim_digest: Hash,
    },
    /// A basis-pinned observation named a frontier outside recovered history.
    #[error("trusted runtime WAL causal-history basis is unavailable: {requested:?}")]
    CausalHistoryBasisUnavailable {
        /// Requested logical causal-history frontier digest.
        requested: Hash,
    },
    /// Live runtime authority contains state absent from the recovered WAL.
    #[error("trusted runtime WAL activation would forget process-only {gap:?} authority")]
    RuntimeAuthorityNotDurable {
        /// Category of live authority missing from durable recovery evidence.
        gap: RuntimeWalActivationGap,
    },
    /// Evidence catalog operations failed.
    #[error("trusted runtime evidence catalog error: {0}")]
    EvidenceCatalog(#[from] crate::evidence::EvidenceCatalogError),
    /// Runtime outcome evidence could not be matched to the receipt correlation.
    #[error(
        "trusted runtime WAL tick outcome unavailable for submission {submission_id:?} receipt {receipt_digest:?}"
    )]
    TickOutcomeUnavailable {
        /// Submission whose scheduler tick outcome should have been decided.
        submission_id: Hash,
        /// Receipt digest expected by the new correlation.
        receipt_digest: Hash,
    },
    /// Runtime outcome evidence did not match the receipt correlation.
    #[error(
        "trusted runtime WAL receipt digest mismatch: expected {expected_receipt_digest:?}, observed {observed_receipt_digest:?}"
    )]
    TickReceiptDigestMismatch {
        /// Receipt digest expected by the new correlation.
        expected_receipt_digest: Hash,
        /// Receipt digest observed through the outcome surface.
        observed_receipt_digest: Hash,
    },
    /// Filesystem WAL cannot safely roll back multiple durable tick commits as
    /// separate transactions.
    #[error(
        "trusted runtime WAL filesystem adapter cannot atomically commit {transaction_count} {transaction_kind:?} transactions"
    )]
    FilesystemAtomicBatchUnsupported {
        /// Transaction kind that would require atomic multi-transaction append.
        transaction_kind: WalTransactionKind,
        /// Number of transactions in the attempted durable batch.
        transaction_count: usize,
    },
    /// Per-Action receipt records do not describe one exact scheduler Tick.
    #[error("trusted runtime WAL scheduler Tick batch is internally inconsistent")]
    SchedulerTickBatchMismatch,
}

/// Live runtime authority category that a WAL activation cannot safely recover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeWalActivationGap {
    /// A witnessed submission has no durable acceptance and envelope evidence.
    WitnessedSubmission,
    /// A staged ticketed ingress has no durable decision correlation.
    TicketedIngress,
    /// A receipt correlation has no durable tick transaction.
    ReceiptCorrelation,
    /// A provenance entry has no replayable durable state delta.
    Provenance,
    /// A writer-head inbox contains pending process-only ingress.
    PendingIngress,
    /// The runtime cycle has advanced beyond durable tick evidence.
    GlobalTick,
    /// A live worldline frontier differs from its retained provenance history.
    WorldlineState,
    /// Installed executable meaning has no exact durable installation record.
    ExecutableOperationInstallation,
}

/// Summary returned after a trusted host runs the scheduler until idle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrustedRuntimeHostRunReport {
    /// Scheduler passes attempted, including the final idle pass.
    pub scheduler_passes: u64,
    /// Scheduler-owned step records committed across non-idle passes.
    pub committed_steps: usize,
}

/// One Echo causal-anchor admission observed in committed control history.
///
/// The enclosed fact and receipt are the semantic evidence. WAL coordinates on
/// the recovered admission prove how that transition was made durable, while
/// the before/after bases place it in Echo's ordered causal history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessedCausalAnchorAdmission {
    admission: RecoveredCausalAnchorAdmission,
    basis_before: CausalFrontierRef,
    basis_after: CausalFrontierRef,
    transition_ordinal: usize,
}

impl WitnessedCausalAnchorAdmission {
    fn new(
        admission: RecoveredCausalAnchorAdmission,
        basis_before: CausalFrontierRef,
        basis_after: CausalFrontierRef,
        transition_ordinal: usize,
    ) -> Self {
        Self {
            admission,
            basis_before,
            basis_after,
            transition_ordinal,
        }
    }

    /// Returns the admitted fact, receipt, and durable carrier evidence.
    #[must_use]
    pub const fn admission(&self) -> &RecoveredCausalAnchorAdmission {
        &self.admission
    }

    /// Returns the causal frontier immediately before admission.
    #[must_use]
    pub const fn basis_before(&self) -> &CausalFrontierRef {
        &self.basis_before
    }

    /// Returns the causal frontier produced by admission.
    #[must_use]
    pub const fn basis_after(&self) -> &CausalFrontierRef {
        &self.basis_after
    }
}

/// Read-only runtime WAL recovery report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedRuntimeWalRecovery {
    /// Recovery certificate summarizing committed history replay.
    pub certificate: RecoveryCertificate,
    /// Rebuilt submission posture index.
    pub submissions: RecoveredSubmissionIndex,
    /// Rebuilt receipt/correlation index.
    pub receipts: RecoveredReceiptIndex,
    /// Replayable witnessed submission ledger reconstructed from retained envelopes.
    pub witnessed_submissions: WitnessedSubmissionPersistenceSnapshot,
    /// Accepted submissions whose canonical envelope material was unavailable.
    pub missing_submission_envelopes: Vec<Hash>,
    /// Replayable scheduler local commits reconstructed from retained state deltas.
    pub provenance_entries: Vec<ProvenanceEntry>,
    /// Decided receipts whose legacy state-delta record retained only a digest.
    pub missing_runtime_state_deltas: Vec<Hash>,
    /// Receipt correlations reconstructed from atomic tick transactions.
    pub receipt_correlations: Vec<ReceiptCorrelationPersistenceRecord>,
    /// Ordered Echo control history for committed causal-anchor admissions.
    pub causal_anchor_history: Vec<WitnessedCausalAnchorAdmission>,
    /// Exact executable meaning reconstructed from retained installation records.
    pub installed_echo_operations: Vec<InstalledEchoOperationV1>,
    /// Typed receipts reconstructed from executable-operation tick records.
    pub echo_operation_receipts: Vec<EchoOperationReceiptV1>,
    /// Typed executable-operation Action outcomes reconstructed from
    /// scheduler-owned Tick records, keyed by witnessed submission.
    pub echo_operation_action_outcomes: Vec<(Hash, Hash, EchoOperationActionOutcomeV1)>,
    echo_operation_action_decisions: BTreeMap<Hash, WalTickDecision>,
    echo_operation_action_installations_before_tick:
        BTreeMap<Hash, BTreeSet<crate::EchoOperationPackageIdV1>>,
    causal_history_frontiers: Vec<CausalFrontierRef>,
}

impl TrustedRuntimeWalRecovery {
    /// Re-runs executable Action/Tick correspondence checks after a targeted
    /// test mutates recovered evidence.
    #[cfg(any(test, feature = "host_test"))]
    pub fn validate_echo_operation_action_outcomes_for_test(
        &self,
    ) -> Result<(), TrustedRuntimeWalError> {
        validate_recovered_echo_operation_action_outcomes(
            &self.witnessed_submissions,
            &self.receipt_correlations,
            &self.provenance_entries,
            &self.installed_echo_operations,
            &self.echo_operation_action_outcomes,
            &self.echo_operation_action_decisions,
            &self.echo_operation_action_installations_before_tick,
        )
    }

    /// Replaces one recovered scheduler decision for adversarial recovery tests.
    #[cfg(any(test, feature = "host_test"))]
    pub fn replace_echo_operation_action_decision_for_test(
        &mut self,
        submission_id: Hash,
        decision: WalTickDecision,
    ) {
        self.echo_operation_action_decisions
            .insert(submission_id, decision);
    }

    /// Removes the installation-order witness for one Action in adversarial tests.
    #[cfg(any(test, feature = "host_test"))]
    pub fn clear_echo_operation_action_installations_before_tick_for_test(
        &mut self,
        submission_id: Hash,
    ) {
        self.echo_operation_action_installations_before_tick
            .remove(&submission_id);
    }

    /// Replaces one rejected preparation identity for adversarial tests.
    #[cfg(any(test, feature = "host_test"))]
    pub fn replace_echo_operation_action_conflict_preparation_for_test(
        &mut self,
        submission_id: Hash,
        digest: Hash,
    ) {
        if let Some((_, _, outcome)) = self
            .echo_operation_action_outcomes
            .iter_mut()
            .find(|(candidate, _, _)| *candidate == submission_id)
        {
            outcome.replace_conflict_preparation_id_for_test(digest);
        }
    }

    /// Replaces one retained conflict's blockers for adversarial recovery tests.
    #[cfg(any(test, feature = "host_test"))]
    pub fn replace_echo_operation_action_conflict_blockers_for_test(
        &mut self,
        submission_id: Hash,
        replacement: Vec<u32>,
    ) {
        if let Some((_, _, outcome)) = self
            .echo_operation_action_outcomes
            .iter_mut()
            .find(|(retained_submission_id, _, _)| *retained_submission_id == submission_id)
        {
            outcome.replace_conflict_blockers_for_test(replacement);
        }
    }

    /// Re-runs activation-time Action parent-state checks for adversarial tests.
    #[cfg(any(test, feature = "host_test"))]
    pub fn validate_echo_operation_parent_states_for_test(
        &self,
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
    ) -> Result<(), TrustedRuntimeWalError> {
        validate_recovered_echo_operation_parent_states(runtime, provenance, self)
    }

    /// Recomputes the certificate's canonical index root from recovered evidence.
    ///
    /// # Errors
    ///
    /// Returns a retained-provenance codec error when a recovered local commit
    /// cannot be canonically encoded.
    pub fn recomputed_indexes_root(&self) -> Result<Hash, TrustedRuntimeWalError> {
        recovered_runtime_wal_indexes_root(&RecoveredRuntimeWalIndexEvidence {
            submissions: &self.submissions,
            receipts: &self.receipts,
            witnessed_submissions: &self.witnessed_submissions,
            missing_submission_envelopes: &self.missing_submission_envelopes,
            provenance_entries: &self.provenance_entries,
            missing_runtime_state_deltas: &self.missing_runtime_state_deltas,
            causal_anchor_history: &self.causal_anchor_history,
            installed_echo_operations: &self.installed_echo_operations,
            echo_operation_receipts: &self.echo_operation_receipts,
            echo_operation_action_outcomes: &self.echo_operation_action_outcomes,
        })
    }

    /// Returns one recovered causal-anchor admission by stable anchor identity.
    #[must_use]
    pub fn causal_anchor_by_id(
        &self,
        anchor_id: &CausalAnchorId,
    ) -> Option<&RecoveredCausalAnchorAdmission> {
        self.causal_anchor_history
            .iter()
            .find(|entry| entry.admission().fact().anchor_id() == anchor_id)
            .map(WitnessedCausalAnchorAdmission::admission)
    }

    /// Observes one anchor at an exact committed causal-history basis.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the requested basis is not part of the
    /// recovered history represented by this report.
    pub fn causal_anchor_by_id_at_basis(
        &self,
        anchor_id: &CausalAnchorId,
        basis: &CausalFrontierRef,
    ) -> Result<Option<&RecoveredCausalAnchorAdmission>, TrustedRuntimeWalError> {
        let basis_ordinal = self
            .causal_history_frontiers
            .iter()
            .position(|candidate| candidate == basis)
            .ok_or(TrustedRuntimeWalError::CausalHistoryBasisUnavailable {
                requested: basis.frontier_digest,
            })?;
        Ok(self
            .causal_anchor_history
            .iter()
            .find(|entry| {
                entry.transition_ordinal <= basis_ordinal
                    && entry.admission().fact().anchor_id() == anchor_id
            })
            .map(WitnessedCausalAnchorAdmission::admission))
    }
}

/// Store kind configured for the trusted runtime WAL adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrustedRuntimeWalStoreKind {
    /// Deterministic process-local store used by fast tests.
    InMemory,
    /// Strict filesystem store rooted in host-owned storage.
    Filesystem,
}

/// Host-owned runtime WAL adapter configuration.
#[derive(Clone, Debug)]
pub struct TrustedRuntimeWalConfig {
    store: TrustedRuntimeWalStoreConfig,
    next_lsn: Lsn,
}

impl TrustedRuntimeWalConfig {
    /// Builds the deterministic in-memory runtime WAL configuration.
    #[must_use]
    pub fn in_memory() -> Self {
        Self {
            store: TrustedRuntimeWalStoreConfig::InMemory,
            next_lsn: Lsn::from_raw(0),
        }
    }

    /// Builds a filesystem-backed runtime WAL configuration rooted at `root`.
    ///
    /// The configured root is host-owned authority. Application code receives
    /// only submission handles and observations through [`TrustedRuntimeApp`].
    #[must_use]
    pub fn filesystem(root: impl AsRef<Path>) -> Self {
        Self {
            store: TrustedRuntimeWalStoreConfig::Filesystem {
                root: root.as_ref().to_path_buf(),
                segment_id: WalSegmentId::from_raw(1),
                #[cfg(any(test, feature = "host_test"))]
                fault_plan: None,
            },
            next_lsn: Lsn::from_raw(0),
        }
    }

    /// Builds a filesystem-backed runtime WAL configuration with a host-test
    /// fault plan.
    #[cfg(any(test, feature = "host_test"))]
    #[must_use]
    pub fn filesystem_with_fault_plan_for_test(
        root: impl AsRef<Path>,
        fault_plan: FilesystemWalFaultPlan,
    ) -> Self {
        Self {
            store: TrustedRuntimeWalStoreConfig::Filesystem {
                root: root.as_ref().to_path_buf(),
                segment_id: WalSegmentId::from_raw(1),
                fault_plan: Some(fault_plan),
            },
            next_lsn: Lsn::from_raw(0),
        }
    }

    /// Returns the configured store kind as read-only evidence.
    #[must_use]
    pub fn store_kind(&self) -> TrustedRuntimeWalStoreKind {
        self.store.kind()
    }

    /// Returns this configuration with a caller-supplied next LSN.
    #[must_use]
    pub fn with_next_lsn(mut self, next_lsn: Lsn) -> Self {
        self.next_lsn = next_lsn;
        self
    }
}

#[derive(Clone, Debug)]
enum TrustedRuntimeWalStoreConfig {
    InMemory,
    Filesystem {
        root: PathBuf,
        segment_id: WalSegmentId,
        #[cfg(any(test, feature = "host_test"))]
        fault_plan: Option<FilesystemWalFaultPlan>,
    },
}

impl TrustedRuntimeWalStoreConfig {
    fn kind(&self) -> TrustedRuntimeWalStoreKind {
        match self {
            Self::InMemory => TrustedRuntimeWalStoreKind::InMemory,
            Self::Filesystem { .. } => TrustedRuntimeWalStoreKind::Filesystem,
        }
    }
}

/// Local trusted runtime host for the app-safe contract-host path.
///
/// Application code should receive [`TrustedRuntimeApp`], not this type. This
/// host owns package registration, ticketed runtime ingress, scheduler passes,
/// and read-only observation service access.
pub struct TrustedRuntimeHost {
    runtime: WorldlineRuntime,
    provenance: ProvenanceService,
    engine: Engine,
    runtime_wal: Option<TrustedRuntimeWal>,
    causal_anchor_support_policy: Option<CausalAnchorRootSupportPolicy>,
    echo_operation_evaluation_authority: EchoOperationEvaluationAuthorityV1,
    echo_operation_action_admission_policy: Option<EchoOperationInvocationAdmissionPolicyV1>,
    echo_operation_action_outcomes: BTreeMap<Hash, EchoOperationActionOutcomeV1>,
    pending_echo_operation_actions: BTreeSet<Hash>,
    admitted_echo_operation_actions: BTreeMap<Hash, AdmittedEchoOperationInvocationV1>,
    echo_operation_action_admission_obstructions:
        BTreeMap<Hash, EchoOperationInvocationAdmissionErrorKindV1>,
    #[cfg(any(test, feature = "host_test"))]
    echo_operation_action_admission_attempts: BTreeMap<Hash, u64>,
}

fn pending_echo_operation_action_ids_v1(
    runtime: &WorldlineRuntime,
    decided: &BTreeMap<Hash, EchoOperationActionOutcomeV1>,
) -> BTreeSet<Hash> {
    runtime
        .pending_witnessed_submissions()
        .filter_map(|submission| {
            let submission_id = submission.submission_id;
            (!decided.contains_key(&submission_id)
                && runtime
                    .witnessed_submission_envelope(&submission_id)
                    .is_some_and(|envelope| {
                        echo_operation_action_invocation_bytes_v1(envelope).is_some()
                    }))
            .then_some(submission_id)
        })
        .collect()
}

impl TrustedRuntimeHost {
    /// Builds a trusted host and initializes provenance from registered runtime
    /// worldlines.
    ///
    /// # Errors
    ///
    /// Returns a provenance error if any runtime worldline cannot be registered.
    pub fn new(runtime: WorldlineRuntime, engine: Engine) -> Result<Self, TrustedRuntimeHostError> {
        let provenance = provenance_from_runtime(&runtime)?;
        let pending_echo_operation_actions =
            pending_echo_operation_action_ids_v1(&runtime, &BTreeMap::new());
        Ok(Self {
            runtime,
            provenance,
            engine,
            runtime_wal: None,
            causal_anchor_support_policy: None,
            echo_operation_evaluation_authority: EchoOperationEvaluationAuthorityV1::new(),
            echo_operation_action_admission_policy: None,
            echo_operation_action_outcomes: BTreeMap::new(),
            pending_echo_operation_actions,
            admitted_echo_operation_actions: BTreeMap::new(),
            echo_operation_action_admission_obstructions: BTreeMap::new(),
            #[cfg(any(test, feature = "host_test"))]
            echo_operation_action_admission_attempts: BTreeMap::new(),
        })
    }

    /// Builds a trusted host from already-initialized parts.
    #[must_use]
    pub fn from_parts(
        runtime: WorldlineRuntime,
        provenance: ProvenanceService,
        engine: Engine,
    ) -> Self {
        let pending_echo_operation_actions =
            pending_echo_operation_action_ids_v1(&runtime, &BTreeMap::new());
        Self {
            runtime,
            provenance,
            engine,
            runtime_wal: None,
            causal_anchor_support_policy: None,
            echo_operation_evaluation_authority: EchoOperationEvaluationAuthorityV1::new(),
            echo_operation_action_admission_policy: None,
            echo_operation_action_outcomes: BTreeMap::new(),
            pending_echo_operation_actions,
            admitted_echo_operation_actions: BTreeMap::new(),
            echo_operation_action_admission_obstructions: BTreeMap::new(),
            #[cfg(any(test, feature = "host_test"))]
            echo_operation_action_admission_attempts: BTreeMap::new(),
        }
    }

    /// Consumes the host and returns owned runtime parts.
    #[must_use]
    pub fn into_parts(self) -> (WorldlineRuntime, ProvenanceService, Engine) {
        (self.runtime, self.provenance, self.engine)
    }

    /// Returns the host-owned runtime as read-only evidence.
    #[must_use]
    pub fn runtime(&self) -> &WorldlineRuntime {
        &self.runtime
    }

    /// Returns the host-owned provenance service as read-only evidence.
    #[must_use]
    pub fn provenance(&self) -> &ProvenanceService {
        &self.provenance
    }

    /// Returns the host-owned engine as read-only evidence.
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Installs the runtime-owned policy used to admit durably accepted
    /// executable-operation Actions before scheduler selection.
    pub fn install_echo_operation_action_admission_policy_v1(
        &mut self,
        policy: EchoOperationInvocationAdmissionPolicyV1,
    ) {
        self.echo_operation_action_admission_policy = Some(policy);
        self.requeue_obstructed_echo_operation_actions_v1();
    }

    /// Returns the typed scheduler-owned outcome for one executable-operation
    /// Action submission.
    #[must_use]
    pub fn echo_operation_action_outcome_v1(
        &self,
        submission_id: &Hash,
    ) -> Option<&EchoOperationActionOutcomeV1> {
        self.echo_operation_action_outcomes.get(submission_id)
    }

    /// Returns the typed pending posture for an executable-operation Action
    /// which runtime-owned admission cannot currently authorize.
    #[must_use]
    pub fn echo_operation_action_admission_obstruction_v1(
        &self,
        submission_id: &Hash,
    ) -> Option<EchoOperationInvocationAdmissionErrorKindV1> {
        self.echo_operation_action_admission_obstructions
            .get(submission_id)
            .copied()
    }

    /// Returns how often runtime-owned admission evaluated one Action in
    /// scheduler tests.
    #[cfg(any(test, feature = "host_test"))]
    #[must_use]
    pub fn echo_operation_action_admission_attempts_for_test(&self, submission_id: &Hash) -> u64 {
        self.echo_operation_action_admission_attempts
            .get(submission_id)
            .copied()
            .unwrap_or(0)
    }

    /// Admits exact executable-operation package bytes under independent policy.
    ///
    /// A successful result is an opaque admission token. It does not install
    /// the package and the program digest inside the package does not confer an
    /// operation coordinate, invocability, or authority.
    pub fn admit_echo_operation_package_v1(
        &self,
        policy: &EchoOperationAdmissionPolicyV1,
        canonical_package_bytes: Vec<u8>,
    ) -> Result<AdmittedExecutableOperationPackageV1, EchoOperationAdmissionErrorV1> {
        admit_package_v1(policy, canonical_package_bytes)
    }

    /// Installs exact executable meaning carried by an admitted operation package.
    ///
    /// Installation stores no native callback and still does not authorize an
    /// invocation. Application-facing [`TrustedRuntimeApp`] handles cannot call
    /// this runtime-owner method.
    pub fn install_admitted_echo_operation_package_v1(
        &mut self,
        admitted: AdmittedExecutableOperationPackageV1,
    ) -> Result<InstalledEchoOperationV1, TrustedRuntimeHostError> {
        let installed = installed_from_admitted(admitted)?;
        self.engine
            .preflight_recovered_echo_operation_packages_v1(core::slice::from_ref(&installed))?;
        if self
            .engine
            .installed_echo_operation_package_v1(installed.package_id())
            == Some(&installed)
        {
            return Ok(installed);
        }

        if let Some(runtime_wal) = self.runtime_wal.as_mut() {
            let retained = retain_installation_v1(&installed)?;
            if let Err(error) = runtime_wal.record_executable_operation_installation(&retained) {
                if !runtime_wal.recover_filesystem_executable_operation_installation_after_error(
                    installed.package_id(),
                ) {
                    return Err(error.into());
                }
            }
        }
        self.engine
            .restore_recovered_echo_operation_packages_v1(core::slice::from_ref(&installed))?;
        self.requeue_obstructed_echo_operation_actions_v1();
        Ok(installed)
    }

    /// Resolves the exact current parent basis for an operation invocation.
    ///
    /// The returned basis binds Echo's writer head, worldline tick, committing
    /// global tick when one exists, graph state root, commit identity, and the
    /// separately typed application basis proposition.
    pub fn echo_operation_evaluation_basis_v1(
        &self,
        writer_head: crate::WriterHeadKey,
        application_basis: EchoOperationApplicationBasisV1,
    ) -> Result<EchoOperationEvaluationBasisV1, TrustedRuntimeHostError> {
        crate::coordinator::resolve_echo_operation_evaluation_basis_v1(
            &self.runtime,
            &self.provenance,
            writer_head,
            application_basis,
        )
        .map_err(TrustedRuntimeHostError::from)
    }

    /// Independently admits a canonical installed-operation invocation.
    ///
    /// Admission checks exact installed package identity, public operation
    /// coordinate, authority profile and grant, delegated budget, and current
    /// parent basis. It does not evaluate or mutate the graph.
    pub fn admit_echo_operation_invocation_v1(
        &self,
        policy: &EchoOperationInvocationAdmissionPolicyV1,
        canonical_invocation_bytes: &[u8],
    ) -> Result<AdmittedEchoOperationInvocationV1, EchoOperationInvocationAdmissionErrorV1> {
        let (package_id, claimed_basis) = decode_invocation_route_v1(canonical_invocation_bytes)?;
        let current_basis = self
            .echo_operation_evaluation_basis_v1(
                claimed_basis.writer_head(),
                claimed_basis.application_basis(),
            )
            .map_err(|error| crate::echo_operation::invocation_runtime_error(error.to_string()))?;
        let current_state = self
            .runtime
            .worldlines()
            .get(&claimed_basis.writer_head().worldline_id)
            .map(crate::WorldlineFrontier::state)
            .ok_or_else(|| {
                crate::echo_operation::invocation_runtime_error(
                    "operation invocation worldline is unavailable",
                )
            })?;
        admit_invocation_v1(
            self.engine.installed_echo_operation_package_v1(package_id),
            *policy,
            canonical_invocation_bytes,
            current_basis,
            current_state,
            self.echo_operation_evaluation_authority.clone(),
        )
    }

    /// Transitional host-only seam for evaluating one admitted operation
    /// without mutating the parent state.
    ///
    /// Application execution must submit an executable-operation Action and
    /// let the scheduler invoke private evaluation while constructing a Tick.
    /// This direct seam remains hidden for compatibility and focused tests.
    #[must_use]
    #[doc(hidden)]
    pub fn prepare_echo_operation_v1(
        &self,
        admitted: AdmittedEchoOperationInvocationV1,
    ) -> EchoOperationPreparationV1 {
        let current_basis = self.echo_operation_evaluation_basis_v1(
            admitted.evaluation_basis().writer_head(),
            admitted.evaluation_basis().application_basis(),
        );
        let Ok(current_basis) = current_basis else {
            return crate::echo_operation::runtime_basis_obstruction(admitted);
        };
        let state = self
            .runtime
            .worldlines()
            .get(&current_basis.writer_head().worldline_id)
            .map(crate::WorldlineFrontier::state);
        let Some(state) = state else {
            return crate::echo_operation::runtime_basis_obstruction(admitted);
        };
        prepare_operation_v1(
            self.engine
                .installed_echo_operation_package_v1(admitted.package_id()),
            admitted,
            current_basis,
            state,
            self.engine.echo_operation_policy_id(),
            &self.echo_operation_evaluation_authority,
        )
    }

    /// Transitional host-only seam for directly committing one privately
    /// prepared consequence against its exact evaluation basis.
    ///
    /// Application execution must use scheduler-owned executable-operation
    /// Actions. This direct seam remains hidden for compatibility and focused
    /// tests while that older lifecycle is retired.
    #[doc(hidden)]
    pub fn commit_prepared_echo_operation_v1(
        &mut self,
        prepared: Box<PreparedEchoOperationV1>,
    ) -> Result<EchoOperationExecutionEvidenceV1, TrustedRuntimeHostError> {
        let prepared = *prepared;
        let current_basis = self.echo_operation_evaluation_basis_v1(
            prepared.evaluation_basis().writer_head(),
            prepared.evaluation_basis().application_basis(),
        )?;
        if &current_basis != prepared.evaluation_basis() {
            let frontier = self
                .runtime
                .worldlines()
                .get(&current_basis.writer_head().worldline_id)
                .ok_or_else(|| {
                    RuntimeError::UnknownWorldline(current_basis.writer_head().worldline_id)
                })?;
            return Ok(not_committed_basis_changed(
                &prepared,
                frontier.state().state_root(),
                frontier.frontier_tick(),
            ));
        }
        if self
            .engine
            .installed_echo_operation_package_v1(prepared.package_id())
            .is_none_or(|installed| {
                installed.installed_operation_id() != prepared.installed_operation_id()
            })
        {
            return Ok(not_committed_installation_unavailable(
                &prepared,
                current_basis.state_root(),
                current_basis.worldline_tick(),
            ));
        }
        if !prepared.is_owned_by(&self.echo_operation_evaluation_authority) {
            return Ok(not_committed_evaluation_authority_mismatch(
                &prepared,
                current_basis.state_root(),
                current_basis.worldline_tick(),
            ));
        }

        let mut next_runtime = self.runtime.clone();
        let mut next_provenance = self.provenance.clone();
        let commit_global_tick = next_runtime.advance_global_tick()?;
        let worldline_id = current_basis.writer_head().worldline_id;
        let parents = next_provenance.tip_ref(worldline_id)?.into_iter().collect();
        let worldline_tick = current_basis.worldline_tick();
        let (material, provenance_entry) = {
            let frontier = next_runtime.frontier_mut(&worldline_id)?;
            let material =
                commit_prepared_to_state(&prepared, frontier.state_mut(), commit_global_tick)?;
            let worldline_patch = crate::WorldlineTickPatchV1 {
                header: crate::WorldlineTickHeaderV1 {
                    commit_global_tick,
                    policy_id: material.patch.policy_id(),
                    rule_pack_id: material.patch.rule_pack_id(),
                    plan_digest: material.snapshot.plan_digest,
                    decision_digest: material.snapshot.decision_digest,
                    rewrites_digest: material.snapshot.rewrites_digest,
                },
                warp_id: material.snapshot.root.warp_id,
                ops: material.patch.ops().to_vec(),
                in_slots: material.patch.in_slots().to_vec(),
                out_slots: material.patch.out_slots().to_vec(),
                patch_digest: material.patch.digest(),
            };
            let entry = ProvenanceEntry::local_commit(
                worldline_id,
                worldline_tick,
                commit_global_tick,
                current_basis.writer_head(),
                parents,
                crate::HashTriplet {
                    state_root: material.snapshot.state_root,
                    patch_digest: material.snapshot.patch_digest,
                    commit_hash: material.snapshot.hash,
                },
                worldline_patch,
                Vec::new(),
                Vec::new(),
            )
            .with_tick_receipt(material.tick_receipt.clone());
            next_provenance.append_local_commit(entry.clone())?;
            frontier
                .advance_tick()
                .ok_or(RuntimeError::FrontierTickOverflow(worldline_id))?;
            (material, entry)
        };

        let installed = self
            .engine
            .installed_echo_operation_package_v1(prepared.package_id());
        if let Some(runtime_wal) = self.runtime_wal.as_mut() {
            let installed =
                installed.ok_or(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                    detail: "committed operation installation is unavailable for WAL validation",
                })?;
            let state_delta = WalRuntimeStateDeltaRecord::from_provenance_entry(
                material.tick_receipt.digest(),
                None,
                provenance_entry,
            )
            .map_err(TrustedRuntimeWalError::from)?;
            let state_delta_digest = state_delta.digest().map_err(TrustedRuntimeWalError::from)?;
            let retained_execution = retain_committed_execution_v1(&material.evidence)?;
            if let Err(error) = runtime_wal.record_executable_operation_tick(
                material.evidence.receipt(),
                retained_execution,
                &state_delta,
                state_delta_digest,
                installed,
            ) {
                if !runtime_wal.recover_filesystem_executable_operation_tick_after_error(
                    material.evidence.receipt().digest(),
                ) {
                    return Err(error.into());
                }
            }
        }

        self.runtime = next_runtime;
        self.provenance = next_provenance;
        Ok(material.evidence)
    }

    /// Enables the in-memory WAL adapter used by the reference host tests.
    ///
    /// This adapter proves the ACK ordering contract and recovery indexes. It
    /// is not a strict filesystem durability adapter.
    ///
    /// # Errors
    ///
    /// Returns a WAL error when the writer epoch cannot be acquired.
    pub fn enable_in_memory_runtime_wal(&mut self) -> Result<(), TrustedRuntimeHostError> {
        self.enable_runtime_wal(TrustedRuntimeWalConfig::in_memory())
    }

    /// Enables a host-owned runtime WAL adapter configuration.
    ///
    /// # Errors
    ///
    /// Returns a WAL error when the configured store cannot acquire the runtime
    /// writer epoch.
    pub fn enable_runtime_wal(
        &mut self,
        config: TrustedRuntimeWalConfig,
    ) -> Result<(), TrustedRuntimeHostError> {
        let runtime_wal = TrustedRuntimeWal::from_config(config)?;
        let recovery = runtime_wal.recover_read_only()?;
        if let Some(submission_id) = recovery.missing_submission_envelopes.first().copied() {
            return Err(TrustedRuntimeWalError::SubmissionEnvelopeMissing { submission_id }.into());
        }
        if let Some(receipt_digest) = recovery.missing_runtime_state_deltas.first().copied() {
            return Err(TrustedRuntimeWalError::RuntimeStateDeltaMissing { receipt_digest }.into());
        }
        ensure_runtime_authority_is_durable(
            &self.runtime,
            &self.provenance,
            &self.engine,
            &recovery,
        )?;
        self.engine
            .preflight_recovered_echo_operation_packages_v1(&recovery.installed_echo_operations)?;

        let mut restored_runtime = self.runtime.clone();
        let mut restored_provenance = self.provenance.clone();
        restored_runtime
            .restore_witnessed_submission_persistence(recovery.witnessed_submissions)?;
        restore_provenance_entries(&mut restored_provenance, &recovery.provenance_entries)?;
        restored_runtime.restore_causal_runtime_history(
            &restored_provenance,
            &recovery.provenance_entries,
            &recovery.receipt_correlations,
        )?;

        self.engine
            .restore_recovered_echo_operation_packages_v1(&recovery.installed_echo_operations)?;
        self.echo_operation_action_outcomes = recovery
            .echo_operation_action_outcomes
            .iter()
            .map(|(submission_id, _, outcome)| (*submission_id, outcome.clone()))
            .collect();
        self.runtime = restored_runtime;
        self.provenance = restored_provenance;
        self.pending_echo_operation_actions = pending_echo_operation_action_ids_v1(
            &self.runtime,
            &self.echo_operation_action_outcomes,
        );
        self.admitted_echo_operation_actions.clear();
        self.echo_operation_action_admission_obstructions.clear();
        #[cfg(any(test, feature = "host_test"))]
        self.echo_operation_action_admission_attempts.clear();
        self.runtime_wal = Some(runtime_wal);
        Ok(())
    }

    /// Returns the configured runtime WAL adapter, if any, as read-only
    /// evidence.
    #[must_use]
    pub fn runtime_wal(&self) -> Option<&TrustedRuntimeWal> {
        self.runtime_wal.as_ref()
    }

    /// Installs the host-owned generic root-support policy used for causal anchors.
    ///
    /// Application-facing handles cannot install or replace this policy.
    pub fn install_causal_anchor_root_support_policy(
        &mut self,
        policy: CausalAnchorRootSupportPolicy,
    ) {
        self.causal_anchor_support_policy = Some(policy);
    }

    /// Replaces the runtime WAL adapter for targeted host tests.
    #[cfg(any(test, feature = "host_test"))]
    pub fn replace_runtime_wal_for_test(&mut self, runtime_wal: TrustedRuntimeWal) {
        self.runtime_wal = Some(runtime_wal);
    }

    /// Replaces the filesystem WAL fault plan for targeted host tests.
    #[cfg(any(test, feature = "host_test"))]
    pub fn inject_runtime_wal_filesystem_fault_for_test(
        &mut self,
        fault_plan: FilesystemWalFaultPlan,
    ) -> Result<(), TrustedRuntimeHostError> {
        let runtime_wal = self
            .runtime_wal
            .as_mut()
            .ok_or(TrustedRuntimeHostError::RuntimeWalUnavailable)?;
        runtime_wal.replace_filesystem_fault_plan_for_test(fault_plan)?;
        Ok(())
    }

    /// Injects one scheduler Action Tick construction failure for a targeted
    /// rollback test.
    #[cfg(all(
        feature = "native_rule_bootstrap",
        feature = "trusted_runtime",
        any(test, feature = "host_test")
    ))]
    pub fn inject_echo_operation_action_tick_construction_failure_for_test(&mut self) {
        self.runtime
            .inject_echo_operation_action_tick_construction_failure_for_test();
    }

    /// Returns the app-facing surface. This surface can submit and observe, but
    /// it cannot tick, stage ticketed ingress, register packages, or recover
    /// scheduler faults.
    pub fn app(&mut self) -> TrustedRuntimeApp<'_> {
        TrustedRuntimeApp { host: self }
    }

    /// Admits an exact provider proposal under independently pinned host policy.
    ///
    /// This crossing retains an opaque admitted token only. It does not install
    /// handlers, mutate the engine registry, schedule work, or invoke callbacks.
    ///
    /// # Errors
    ///
    /// Returns a structured admission error when any package occurrence or
    /// provider-registry proposition differs from policy.
    pub fn admit_provider_contract_package_v1<'a>(
        &self,
        policy: &ProviderContractAdmissionPolicyV1<'_>,
        proposal: ProviderContractPackageProposalV1<'a>,
    ) -> Result<AdmittedProviderContractPackageV1<'a>, ProviderContractAdmissionError> {
        admit_provider_contract_package_v1(policy, proposal)
    }

    /// Installs an admitted provider package under a trusted caller's package-root claim.
    ///
    /// This is a runtime-owner lower primitive. It requires a nonempty
    /// caller-asserted coordinate and verifies strict digest rendering and
    /// equality with the admitted occurrence hash before performing atomic
    /// Engine registration. It does not authenticate or compare the provider
    /// coordinate, inspect, load, or hash package bytes, so calling it alone is
    /// not package-byte admission evidence. The normal path is the
    /// proof-consuming `echo-wesley-gen` adapter.
    ///
    /// Application-facing [`TrustedRuntimeApp`] handles cannot call this method.
    ///
    /// # Errors
    ///
    /// Returns a structured provider installation failure when the claim is
    /// malformed or conflicts with admitted or already installed state.
    #[doc(hidden)]
    pub fn install_admitted_provider_contract_package_v1_trusted(
        &mut self,
        package_reference: ProviderPackageReferenceV1,
        admitted: AdmittedProviderContractPackageV1<'_>,
    ) -> Result<InstalledProviderContractPackageRecordV1, ProviderContractInstallationError> {
        self.engine
            .install_admitted_provider_contract_package_v1_trusted(package_reference, admitted)
    }

    /// Registers a generated contract package through the trusted host boundary.
    ///
    /// # Errors
    ///
    /// Returns an installed-package error when registry verification fails or
    /// any handler/observer conflicts with existing runtime state.
    pub fn register_contract_package<'a>(
        &mut self,
        package: InstalledContractPackage<'a>,
    ) -> Result<InstalledContractPackageRecord, InstalledContractPackageError<'a>> {
        self.engine.register_contract_package(package)
    }

    fn resolve_contract_inverse_envelope(
        &self,
        request: &ContractInverseAdmissionRequest,
    ) -> Result<IngressEnvelope, ContractInverseObstruction> {
        let correlation = self
            .runtime
            .receipt_correlation_for_receipt_ref(&request.target_receipt_ref)
            .ok_or_else(|| ContractInverseObstruction::TargetReceiptUnavailable {
                target_receipt_ref: Box::new(request.target_receipt_ref),
            })?;
        if !matches!(
            self.runtime
                .observe_app_intent_outcome(&correlation.submission_id),
            IntentOutcome::Applied { receipt, .. }
                if receipt.causal_receipt_ref == request.target_receipt_ref
        ) {
            return Err(ContractInverseObstruction::TargetReceiptNotApplied {
                target_receipt_ref: Box::new(request.target_receipt_ref),
            });
        }
        let target_envelope = self
            .runtime
            .witnessed_submission_envelope(&correlation.submission_id)
            .ok_or(ContractInverseObstruction::TargetSubmissionUnavailable {
                submission_id: correlation.submission_id,
            })?;
        let IngressPayload::LocalIntent {
            intent_kind,
            intent_bytes,
        } = target_envelope.payload();
        let (target_op_id, target_vars_bytes) = decode_canonical_eint(intent_bytes).ok_or(
            ContractInverseObstruction::TargetIntentMalformed {
                submission_id: correlation.submission_id,
            },
        )?;
        let target_evidence = correlation.contract.as_ref().ok_or_else(|| {
            ContractInverseObstruction::TargetContractEvidenceUnavailable {
                target_receipt_ref: Box::new(request.target_receipt_ref),
            }
        })?;
        let crate::InstalledInvocationEvidence::LegacyContract(target_contract) = target_evidence
        else {
            return Err(ContractInverseObstruction::ProviderTargetUnsupported {
                target_receipt_ref: Box::new(request.target_receipt_ref),
            });
        };
        if target_contract.op_kind != ContractOperationKind::Mutation {
            return Err(ContractInverseObstruction::TargetOperationKindMismatch {
                actual: target_contract.op_kind,
            });
        }
        if target_contract.op_id != target_op_id {
            return Err(ContractInverseObstruction::TargetOperationMismatch {
                retained_op_id: target_contract.op_id,
                envelope_op_id: target_op_id,
            });
        }

        let installed_contract = self
            .engine
            .installed_contract_mutation_evidence(target_op_id)
            .ok_or(ContractInverseObstruction::InstalledContractUnavailable { target_op_id })?;
        if installed_contract != *target_contract {
            return Err(ContractInverseObstruction::ContractVersionMismatch {
                retained_package_id: target_contract.package_id,
                installed_package_id: installed_contract.package_id,
            });
        }
        let inverse_handler = self
            .engine
            .installed_contract_inverse_handler(target_op_id)
            .cloned()
            .ok_or(ContractInverseObstruction::InverseHandlerUnavailable { target_op_id })?;

        let current_worldline_id = request.current_target.worldline_id();
        let current_frontier_tick = self
            .runtime
            .worldlines()
            .get(&current_worldline_id)
            .ok_or(ContractInverseObstruction::CurrentWorldlineUnavailable {
                worldline_id: current_worldline_id,
            })?
            .frontier_tick();
        if current_frontier_tick != request.expected_current_frontier_tick {
            return Err(ContractInverseObstruction::CurrentBasisMismatch {
                expected: request.expected_current_frontier_tick,
                observed: current_frontier_tick,
            });
        }
        let mut current_basis_receipt_refs = Vec::new();
        if current_frontier_tick != crate::WorldlineTick::ZERO {
            let current_tip = self
                .provenance
                .tip_ref(current_worldline_id)
                .ok()
                .flatten()
                .filter(|tip| tip.worldline_tick.checked_increment() == Some(current_frontier_tick))
                .ok_or(
                    ContractInverseObstruction::CurrentBasisProvenanceUnavailable {
                        worldline_id: current_worldline_id,
                        frontier_tick: current_frontier_tick,
                    },
                )?;
            current_basis_receipt_refs.extend(
                self.runtime
                    .receipt_correlations_for_current_basis(
                        current_worldline_id,
                        current_frontier_tick,
                        current_tip.commit_hash,
                    )
                    .map(|candidate| candidate.causal_receipt_ref),
            );
            current_basis_receipt_refs.sort_unstable();
            current_basis_receipt_refs.dedup();
            if current_basis_receipt_refs.is_empty() {
                return Err(ContractInverseObstruction::CurrentBasisReceiptUnavailable {
                    worldline_id: current_worldline_id,
                    frontier_tick: current_frontier_tick,
                    commit_hash: current_tip.commit_hash,
                });
            }
        }

        let inverse = (inverse_handler.resolve)(ContractInverseContext {
            target_receipt_ref: request.target_receipt_ref,
            target_submission_id: correlation.submission_id,
            target_contract,
            target_intent_kind: *intent_kind,
            target_op_id,
            target_vars_bytes,
            target_ingress_target: target_envelope.target(),
            current_target: &request.current_target,
            current_frontier_tick,
            current_basis_receipt_refs: &current_basis_receipt_refs,
            policy_bytes: &request.policy_bytes,
            runtime: &self.runtime,
            provenance: &self.provenance,
        })?;
        let emitted_package_id = self
            .engine
            .installed_contract_mutation_package_id(inverse.op_id)
            .copied()
            .ok_or(ContractInverseObstruction::ProducedMutationUnavailable {
                op_id: inverse.op_id,
            })?;
        if emitted_package_id != target_contract.package_id {
            return Err(
                ContractInverseObstruction::ProducedMutationContractMismatch {
                    op_id: inverse.op_id,
                    target_package_id: target_contract.package_id,
                    emitted_package_id,
                },
            );
        }
        let intent_bytes = encode_canonical_eint(inverse.op_id, &inverse.vars_bytes).ok_or(
            ContractInverseObstruction::ProducedIntentEncodingFailed {
                op_id: inverse.op_id,
            },
        )?;
        let mut causal_parents = current_basis_receipt_refs
            .into_iter()
            .map(|receipt_ref| IngressCausalParent::TickReceipt { receipt_ref })
            .collect::<Vec<_>>();
        causal_parents.push(IngressCausalParent::ContractInverseTarget {
            receipt_ref: request.target_receipt_ref,
        });
        Ok(IngressEnvelope::local_intent_with_causal_parents(
            request.current_target.clone(),
            *intent_kind,
            intent_bytes,
            causal_parents,
        ))
    }

    fn contract_inverse_derivation(
        &self,
        inverse_receipt_ref: &crate::CausalTickReceiptRef,
    ) -> Result<Option<ContractInverseDerivation>, ContractInverseHistoryObstruction> {
        let correlation = self
            .runtime
            .receipt_correlation_for_receipt_ref(inverse_receipt_ref)
            .ok_or_else(
                || ContractInverseHistoryObstruction::InverseReceiptUnavailable {
                    inverse_receipt_ref: Box::new(*inverse_receipt_ref),
                },
            )?;
        let envelope = self
            .runtime
            .witnessed_submission_envelope(&correlation.submission_id)
            .ok_or_else(
                || ContractInverseHistoryObstruction::InverseSubmissionUnavailable {
                    inverse_receipt_ref: Box::new(*inverse_receipt_ref),
                    submission_id: correlation.submission_id,
                },
            )?;
        let mut target_receipt_refs = Vec::new();
        let mut current_basis_receipt_refs = Vec::new();
        for parent in envelope.causal_parents() {
            match *parent {
                IngressCausalParent::TickReceipt { receipt_ref } => {
                    current_basis_receipt_refs.push(receipt_ref);
                }
                IngressCausalParent::ContractInverseTarget { receipt_ref } => {
                    target_receipt_refs.push(receipt_ref);
                }
            }
        }
        target_receipt_refs.sort_unstable();
        target_receipt_refs.dedup();
        let Some(target_receipt_ref) = target_receipt_refs.first().copied() else {
            return Ok(None);
        };
        if target_receipt_refs.len() != 1 {
            return Err(ContractInverseHistoryObstruction::AmbiguousInverseTarget {
                inverse_receipt_ref: Box::new(*inverse_receipt_ref),
            });
        }
        if self
            .runtime
            .receipt_correlation_for_receipt_ref(&target_receipt_ref)
            .is_none()
        {
            return Err(
                ContractInverseHistoryObstruction::TargetReceiptUnavailable {
                    inverse_receipt_ref: Box::new(*inverse_receipt_ref),
                    target_receipt_ref: Box::new(target_receipt_ref),
                },
            );
        }
        current_basis_receipt_refs.sort_unstable();
        current_basis_receipt_refs.dedup();
        for basis_receipt_ref in &current_basis_receipt_refs {
            if self
                .runtime
                .receipt_correlation_for_receipt_ref(basis_receipt_ref)
                .is_none()
            {
                return Err(
                    ContractInverseHistoryObstruction::CurrentBasisReceiptUnavailable {
                        inverse_receipt_ref: Box::new(*inverse_receipt_ref),
                        basis_receipt_ref: Box::new(*basis_receipt_ref),
                    },
                );
            }
        }
        Ok(Some(ContractInverseDerivation {
            inverse_receipt_ref: *inverse_receipt_ref,
            target_receipt_ref,
            current_basis_receipt_refs,
        }))
    }

    /// Stages one witnessed installed-contract submission into runtime ingress.
    ///
    /// The host uses retained canonical envelope material and a supplied
    /// admission ticket. This method does not tick or execute.
    ///
    /// # Errors
    ///
    /// Returns a runtime error if the submission is unknown, unsupported by an
    /// installed package, or rejected by the ticketed ingress boundary.
    pub fn stage_installed_contract_submission(
        &mut self,
        submission_id: Hash,
        ticket: &OpticAdmissionTicket,
    ) -> Result<TicketedRuntimeIngressDisposition, RuntimeError> {
        let envelope = self
            .runtime
            .witnessed_submission_envelope(&submission_id)
            .cloned()
            .ok_or(RuntimeError::UnknownIntentSubmission(submission_id))?;
        self.runtime.ingest_installed_contract_invocation(
            &TicketedRuntimeIngressAuthority::assume_runtime_owner(),
            &self.engine,
            submission_id,
            ticket,
            envelope,
        )
    }

    /// Stages one witnessed provider-native submission with a supplied ticket.
    ///
    /// This boundary retains provider-native installed-operation evidence and
    /// does not tick or execute the operation.
    ///
    /// # Errors
    ///
    /// Returns a runtime error if the submission is unknown, malformed,
    /// unsupported by an installed provider package, or rejected by ingress.
    pub fn stage_provider_contract_submission_v1(
        &mut self,
        submission_id: Hash,
        ticket: &OpticAdmissionTicket,
    ) -> Result<TicketedRuntimeIngressDisposition, RuntimeError> {
        let envelope = self
            .runtime
            .witnessed_submission_envelope(&submission_id)
            .cloned()
            .ok_or(RuntimeError::UnknownIntentSubmission(submission_id))?;
        self.runtime.ingest_provider_contract_invocation_v1(
            &TicketedRuntimeIngressAuthority::assume_runtime_owner(),
            &self.engine,
            submission_id,
            ticket,
            envelope,
        )
    }

    /// Admits and stages one witnessed generated-contract submission under
    /// trusted-host policy.
    ///
    /// Echo derives the admission digest from its witnessed submission record
    /// and verified installed-package evidence. Application code neither
    /// supplies nor observes an authority-bearing admission ticket. This method
    /// does not tick or execute the installed operation.
    ///
    /// # Errors
    ///
    /// Returns a runtime error if the submission is unknown, malformed, not
    /// backed by an installed mutation package, or rejected by runtime ingress.
    pub fn admit_installed_contract_submission(
        &mut self,
        submission_id: Hash,
    ) -> Result<TicketedRuntimeIngressDisposition, RuntimeError> {
        let submission = self
            .runtime
            .witnessed_submission(&submission_id)
            .cloned()
            .ok_or(RuntimeError::UnknownIntentSubmission(submission_id))?;
        let envelope = self
            .runtime
            .witnessed_submission_envelope(&submission_id)
            .cloned()
            .ok_or(RuntimeError::UnknownIntentSubmission(submission_id))?;
        let op_id = crate::coordinator::installed_contract_mutation_op_id(&envelope)?;
        let contract = self
            .engine
            .installed_contract_mutation_evidence(op_id)
            .ok_or(RuntimeError::UnsupportedInstalledContractMutation { op_id })?;
        let admission_digest = installed_contract_host_admission_digest(&submission, &contract);

        self.runtime
            .ingest_host_admitted_installed_contract_invocation(
                &TicketedRuntimeIngressAuthority::assume_runtime_owner(),
                &self.engine,
                submission_id,
                admission_digest,
                envelope,
            )
    }

    /// Admits and stages one witnessed provider-native mutation under host policy.
    ///
    /// Echo derives the admission digest from its witnessed submission and the
    /// exact installed provider evidence. Application code supplies no ticket.
    /// This method does not itself tick or execute the operation.
    ///
    /// # Errors
    ///
    /// Returns a runtime error if the submission is unknown, malformed, not
    /// backed by an installed provider mutation, or rejected by ingress.
    pub fn admit_provider_contract_submission_v1(
        &mut self,
        submission_id: Hash,
    ) -> Result<TicketedRuntimeIngressDisposition, RuntimeError> {
        let submission = self
            .runtime
            .witnessed_submission(&submission_id)
            .cloned()
            .ok_or(RuntimeError::UnknownIntentSubmission(submission_id))?;
        let envelope = self
            .runtime
            .witnessed_submission_envelope(&submission_id)
            .cloned()
            .ok_or(RuntimeError::UnknownIntentSubmission(submission_id))?;
        let op_id = crate::coordinator::installed_contract_mutation_op_id(&envelope)?;
        let contract = self
            .engine
            .installed_provider_contract_mutation_evidence_v1(op_id)
            .ok_or(RuntimeError::UnsupportedInstalledProviderContractMutation { op_id })?;
        let admission_digest = provider_contract_host_admission_digest(&submission, &contract);

        self.runtime
            .ingest_host_admitted_provider_contract_invocation_v1(
                &TicketedRuntimeIngressAuthority::assume_runtime_owner(),
                &self.engine,
                submission_id,
                admission_digest,
                envelope,
            )
    }

    fn track_pending_echo_operation_action_v1(
        &mut self,
        submission_id: Hash,
        is_echo_operation_action: bool,
    ) {
        if is_echo_operation_action
            && !self
                .echo_operation_action_outcomes
                .contains_key(&submission_id)
        {
            self.pending_echo_operation_actions.insert(submission_id);
        }
    }

    fn requeue_obstructed_echo_operation_actions_v1(&mut self) {
        self.pending_echo_operation_actions.extend(
            self.echo_operation_action_admission_obstructions
                .keys()
                .copied(),
        );
        self.echo_operation_action_admission_obstructions.clear();
    }

    fn admit_pending_echo_operation_actions_v1(
        &mut self,
    ) -> Result<BTreeMap<Hash, AdmittedEchoOperationInvocationV1>, TrustedRuntimeHostError> {
        if self.pending_echo_operation_actions.is_empty() {
            return Ok(self.admitted_echo_operation_actions.clone());
        }
        let Some(policy) = self.echo_operation_action_admission_policy else {
            return Ok(self.admitted_echo_operation_actions.clone());
        };
        let available = crate::echo_operation::ACTION_BATCH_CANDIDATE_LIMIT_V1
            .saturating_sub(self.admitted_echo_operation_actions.len());
        let pending = self
            .pending_echo_operation_actions
            .iter()
            .filter(|submission_id| {
                !self
                    .admitted_echo_operation_actions
                    .contains_key(*submission_id)
            })
            .take(available)
            .copied()
            .collect::<Vec<_>>();
        for submission_id in pending {
            let submission = self
                .runtime
                .witnessed_submission(&submission_id)
                .cloned()
                .ok_or(RuntimeError::UnknownIntentSubmission(submission_id))?;
            let envelope = self
                .runtime
                .witnessed_submission_envelope(&submission_id)
                .cloned()
                .ok_or(RuntimeError::UnknownIntentSubmission(submission_id))?;
            let invocation_bytes = echo_operation_action_invocation_bytes_v1(&envelope)
                .ok_or(RuntimeError::UnknownIntentSubmission(submission_id))?
                .to_vec();
            if let Some(runtime_wal) = self.runtime_wal.as_ref() {
                let durably_accepted = runtime_wal
                    .has_submission_acceptance(submission.submission_id, submission.ingress_id)?;
                if !durably_accepted {
                    continue;
                }
            }
            #[cfg(any(test, feature = "host_test"))]
            self.echo_operation_action_admission_attempts
                .entry(submission_id)
                .and_modify(|attempts| *attempts += 1)
                .or_insert(1);
            let package_id = match decode_invocation_route_v1(&invocation_bytes) {
                Ok((package_id, _)) => package_id,
                Err(error) => {
                    self.echo_operation_action_admission_obstructions
                        .insert(submission.submission_id, error.kind());
                    self.pending_echo_operation_actions
                        .remove(&submission.submission_id);
                    continue;
                }
            };
            let admitted = match admit_action_invocation_v1(
                self.engine.installed_echo_operation_package_v1(package_id),
                policy,
                &invocation_bytes,
                self.echo_operation_evaluation_authority.clone(),
            ) {
                Ok(admitted) => admitted,
                Err(error) => {
                    self.echo_operation_action_admission_obstructions
                        .insert(submission.submission_id, error.kind());
                    self.pending_echo_operation_actions
                        .remove(&submission.submission_id);
                    continue;
                }
            };
            let admission_digest = echo_operation_action_admission_digest(&submission, &admitted);
            self.runtime.ingest_echo_operation_action_v1(
                &TicketedRuntimeIngressAuthority::assume_runtime_owner(),
                submission.submission_id,
                admission_digest,
                envelope,
            )?;
            self.echo_operation_action_admission_obstructions
                .remove(&submission.submission_id);
            self.admitted_echo_operation_actions
                .insert(submission.submission_id, admitted);
        }
        Ok(self.admitted_echo_operation_actions.clone())
    }

    /// Runs one scheduler-owned pass.
    ///
    /// # Errors
    ///
    /// Returns a runtime error if the scheduler pass fails.
    pub fn tick_once(&mut self) -> Result<Vec<StepRecord>, TrustedRuntimeHostError> {
        let runtime_before = self.runtime.clone();
        let provenance_before = self.provenance.clone();
        let action_outcomes_before = self.echo_operation_action_outcomes.clone();
        let admitted_actions = match self.admit_pending_echo_operation_actions_v1() {
            Ok(admitted) => admitted,
            Err(error) => {
                self.runtime = runtime_before;
                self.provenance = provenance_before;
                return Err(error);
            }
        };
        let existing_correlations = self
            .runtime
            .receipt_correlations()
            .map(|correlation| correlation.ticketed_ingress_id)
            .collect::<BTreeSet<_>>();
        let (records, action_outcomes) =
            SchedulerCoordinator::super_tick_with_echo_operation_actions_v1(
                &mut self.runtime,
                &mut self.provenance,
                &mut self.engine,
                &admitted_actions,
                &self.echo_operation_evaluation_authority,
            )?;
        let new_correlations = self
            .runtime
            .receipt_correlations()
            .filter(|correlation| !existing_correlations.contains(&correlation.ticketed_ingress_id))
            .cloned()
            .collect::<Vec<_>>();
        let mut tick_wal_records = Vec::new();
        if self.runtime_wal.is_some() {
            for correlation in new_correlations {
                let decision = match wal_tick_decision_from_observation(
                    self.runtime
                        .observe_intent_outcome(&correlation.submission_id),
                    correlation.tick_receipt_digest,
                ) {
                    Ok(decision) => decision,
                    Err(error) => {
                        self.runtime = runtime_before;
                        self.provenance = provenance_before;
                        self.echo_operation_action_outcomes = action_outcomes_before;
                        return Err(error.into());
                    }
                };
                let (state_delta, state_delta_digest) =
                    match retained_state_delta_for_correlation(&self.provenance, &correlation) {
                        Ok(state_delta) => state_delta,
                        Err(error) => {
                            self.runtime = runtime_before;
                            self.provenance = provenance_before;
                            self.echo_operation_action_outcomes = action_outcomes_before;
                            return Err(error);
                        }
                    };
                tick_wal_records.push((correlation, decision, state_delta, state_delta_digest));
            }
        }
        let action_outcomes_by_submission =
            action_outcomes.iter().cloned().collect::<BTreeMap<_, _>>();
        let mut tick_wal_groups = BTreeMap::new();
        for (correlation, decision, state_delta, state_delta_digest) in tick_wal_records {
            let key = (
                correlation.commit_hash,
                correlation.tick_receipt_digest,
                state_delta_digest,
            );
            tick_wal_groups.entry(key).or_insert_with(Vec::new).push((
                correlation,
                decision,
                state_delta,
                state_delta_digest,
            ));
        }
        if let Some(runtime_wal) = self.runtime_wal.as_mut() {
            if runtime_wal.uses_filesystem_store() && tick_wal_groups.len() > 1 {
                self.runtime = runtime_before;
                self.provenance = provenance_before;
                self.echo_operation_action_outcomes = action_outcomes_before;
                return Err(TrustedRuntimeWalError::FilesystemAtomicBatchUnsupported {
                    transaction_kind: WalTransactionKind::SchedulerTick,
                    transaction_count: tick_wal_groups.len(),
                }
                .into());
            }
            let runtime_wal_before = runtime_wal.clone();
            for group in tick_wal_groups.values() {
                let Some((first_correlation, _, state_delta, state_delta_digest)) = group.first()
                else {
                    continue;
                };
                let group_has_action_outcomes = group.iter().any(|(correlation, _, _, _)| {
                    action_outcomes_by_submission.contains_key(&correlation.submission_id)
                });
                let result = if group.len() == 1 && !group_has_action_outcomes {
                    runtime_wal.record_tick_receipt(
                        first_correlation,
                        group[0].1,
                        state_delta,
                        *state_delta_digest,
                    )
                } else {
                    let correlations = group
                        .iter()
                        .map(|(correlation, decision, _, _)| (correlation.clone(), *decision))
                        .collect::<Vec<_>>();
                    runtime_wal.record_tick_receipt_batch(
                        &correlations,
                        &action_outcomes_by_submission,
                        state_delta,
                        *state_delta_digest,
                    )
                };
                if let Err(error) = result {
                    if runtime_wal.recover_filesystem_tick_commit_after_error(first_correlation) {
                        continue;
                    }
                    if !runtime_wal.uses_filesystem_store() {
                        *runtime_wal = runtime_wal_before;
                    }
                    self.runtime = runtime_before;
                    self.provenance = provenance_before;
                    self.echo_operation_action_outcomes = action_outcomes_before;
                    return Err(error.into());
                }
            }
        }
        for (submission_id, outcome) in action_outcomes {
            self.pending_echo_operation_actions.remove(&submission_id);
            self.admitted_echo_operation_actions.remove(&submission_id);
            self.echo_operation_action_admission_obstructions
                .remove(&submission_id);
            self.echo_operation_action_outcomes
                .insert(submission_id, outcome);
        }
        Ok(records)
    }

    /// Runs scheduler-owned passes until an idle pass occurs.
    ///
    /// # Errors
    ///
    /// Returns an error if the scheduler fails or if the caller-supplied pass
    /// limit is reached before an idle pass.
    pub fn run_until_idle(
        &mut self,
        max_scheduler_passes: u64,
    ) -> Result<TrustedRuntimeHostRunReport, TrustedRuntimeHostError> {
        if max_scheduler_passes == 0 {
            return Err(TrustedRuntimeHostError::SchedulerPassLimitExceeded {
                max_scheduler_passes,
            });
        }

        let mut report = TrustedRuntimeHostRunReport::default();
        loop {
            if report.scheduler_passes == max_scheduler_passes {
                return Err(TrustedRuntimeHostError::SchedulerPassLimitExceeded {
                    max_scheduler_passes,
                });
            }
            let steps = self.tick_once()?;
            report.scheduler_passes += 1;
            if steps.is_empty() {
                return Ok(report);
            }
            report.committed_steps += steps.len();
        }
    }
}

impl crate::provider_contract::SealedProviderContractPackageInstallerV1 for TrustedRuntimeHost {}

impl ProviderContractPackageInstallerV1 for TrustedRuntimeHost {
    fn install_admitted_provider_contract_package_v1_trusted(
        &mut self,
        package_reference: ProviderPackageReferenceV1,
        admitted: AdmittedProviderContractPackageV1<'_>,
    ) -> Result<InstalledProviderContractPackageRecordV1, ProviderContractInstallationError> {
        TrustedRuntimeHost::install_admitted_provider_contract_package_v1_trusted(
            self,
            package_reference,
            admitted,
        )
    }
}

fn ensure_runtime_authority_is_durable(
    runtime: &WorldlineRuntime,
    provenance: &ProvenanceService,
    engine: &Engine,
    recovery: &TrustedRuntimeWalRecovery,
) -> Result<(), TrustedRuntimeWalError> {
    let recovered_operations = recovery
        .installed_echo_operations
        .iter()
        .map(|installed| (installed.package_id(), installed))
        .collect::<BTreeMap<_, _>>();
    if engine
        .installed_echo_operation_packages_v1()
        .any(|installed| {
            recovered_operations
                .get(&installed.package_id())
                .is_none_or(|recovered| **recovered != *installed)
        })
    {
        return Err(TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
            gap: RuntimeWalActivationGap::ExecutableOperationInstallation,
        });
    }

    let mut recovered_provenance = provenance.clone();
    restore_provenance_entries(&mut recovered_provenance, &recovery.provenance_entries).map_err(
        |_| TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
            gap: RuntimeWalActivationGap::Provenance,
        },
    )?;
    validate_recovered_echo_operation_parent_states(runtime, &recovered_provenance, recovery)?;

    let recovered_submissions = recovery
        .witnessed_submissions
        .records()
        .iter()
        .map(|record| (record.submission.submission_id, record))
        .collect::<BTreeMap<_, _>>();
    if runtime.witnessed_submissions().any(|submission| {
        recovered_submissions
            .get(&submission.submission_id)
            .is_none_or(|record| record.submission != *submission)
    }) {
        return Err(TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
            gap: RuntimeWalActivationGap::WitnessedSubmission,
        });
    }

    let recovered_correlations = recovery
        .receipt_correlations
        .iter()
        .map(|correlation| (correlation.submission_id, correlation))
        .collect::<BTreeMap<_, _>>();
    if runtime.receipt_correlations().any(|correlation| {
        let persisted = ReceiptCorrelationPersistenceRecord::from(correlation);
        recovered_correlations
            .get(&persisted.submission_id)
            .is_none_or(|recovered| **recovered != persisted)
    }) {
        return Err(TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
            gap: RuntimeWalActivationGap::ReceiptCorrelation,
        });
    }

    if runtime.ticketed_runtime_ingress_records().any(|ticketed| {
        let Some(correlation) = recovered_correlations.get(&ticketed.submission_id) else {
            return true;
        };
        let Some(submission) = recovered_submissions.get(&ticketed.submission_id) else {
            return true;
        };
        correlation.ticket_digest != ticketed.ticket_digest
            || correlation.head_key != ticketed.head_key
            || correlation.contract != ticketed.contract
            || submission.submission.ingress_id != ticketed.ingress_id
    }) {
        return Err(TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
            gap: RuntimeWalActivationGap::TicketedIngress,
        });
    }

    if runtime
        .heads()
        .iter()
        .any(|(_, head)| !head.inbox().is_empty())
    {
        return Err(TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
            gap: RuntimeWalActivationGap::PendingIngress,
        });
    }

    let recovered_entries = recovery
        .provenance_entries
        .iter()
        .map(|entry| ((entry.worldline_id, entry.worldline_tick), entry))
        .collect::<BTreeMap<_, _>>();
    for (worldline_id, frontier) in runtime.worldlines().iter() {
        let retained_len = provenance.len(*worldline_id).map_err(|_| {
            TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
                gap: RuntimeWalActivationGap::Provenance,
            }
        })?;
        for raw_tick in 0..retained_len {
            let tick = crate::WorldlineTick::from_raw(raw_tick);
            let current = provenance.entry(*worldline_id, tick).map_err(|_| {
                TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
                    gap: RuntimeWalActivationGap::Provenance,
                }
            })?;
            if recovered_entries
                .get(&(*worldline_id, tick))
                .is_none_or(|recovered| **recovered != current)
            {
                return Err(TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
                    gap: RuntimeWalActivationGap::Provenance,
                });
            }
        }

        let replayed = provenance
            .replay_worldline_state(*worldline_id, frontier.state())
            .map_err(|_| TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
                gap: RuntimeWalActivationGap::WorldlineState,
            })?;
        if frontier.frontier_tick().as_u64() != retained_len
            || replayed.state_root() != frontier.state().state_root()
        {
            return Err(TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
                gap: RuntimeWalActivationGap::WorldlineState,
            });
        }
    }

    let recovered_global_tick = recovery
        .provenance_entries
        .iter()
        .map(|entry| entry.commit_global_tick)
        .chain(
            recovery
                .receipt_correlations
                .iter()
                .map(|correlation| correlation.commit_global_tick),
        )
        .max()
        .unwrap_or_default();
    if runtime.global_tick() > recovered_global_tick {
        return Err(TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
            gap: RuntimeWalActivationGap::GlobalTick,
        });
    }

    Ok(())
}

/// Represents the live cache posture of the derived evidence catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceCatalogPosture {
    /// The live catalog is synchronized with committed history.
    Fresh,
    /// The live catalog failed to update and requires recovery rebuild.
    NeedsRebuild {
        /// A digest indicating the reason for the rebuild.
        reason: Hash,
        /// The last transaction digest where the catalog was known to be fresh.
        last_good_commit: Hash,
    },
}

/// Disposable claim lookup rebuilt from witnessed causal-anchor history.
#[derive(Clone, Debug, Default)]
struct CausalAnchorClaimProjection {
    admissions_by_claim: BTreeMap<Hash, Vec<RecoveredCausalAnchorAdmission>>,
}

impl CausalAnchorClaimProjection {
    fn from_history(history: &[WitnessedCausalAnchorAdmission]) -> Self {
        let mut projection = Self::default();
        for entry in history {
            projection.insert(entry.admission().clone());
        }
        projection
    }

    fn insert(&mut self, admission: RecoveredCausalAnchorAdmission) {
        self.admissions_by_claim
            .entry(*admission.fact().claim().claim_digest())
            .or_default()
            .push(admission);
    }

    fn find(
        &self,
        claim_digest: &Hash,
        support_policy_digest: Option<&Hash>,
    ) -> Result<Option<RecoveredCausalAnchorAdmission>, TrustedRuntimeWalError> {
        let Some(admissions) = self.admissions_by_claim.get(claim_digest) else {
            return Ok(None);
        };
        let mut matching = admissions.iter().filter(|admission| {
            support_policy_digest
                .is_none_or(|digest| admission.receipt().support_policy_digest() == digest)
        });
        let first = matching.next().cloned();
        if matching.next().is_some() {
            return Err(TrustedRuntimeWalError::CausalAnchorClaimConflict {
                claim_digest: *claim_digest,
            });
        }
        Ok(first)
    }
}

/// Minimal trusted-runtime WAL adapter for ACK-boundary integration tests.
#[derive(Clone, Debug)]
pub struct TrustedRuntimeWal {
    store: TrustedRuntimeWalStore,
    evidence_catalog: Option<crate::evidence::CausalSegmentCatalog>,
    evidence_catalog_posture: EvidenceCatalogPosture,
    #[cfg(any(test, feature = "host_test"))]
    fail_next_evidence_catalog_update: bool,
    #[cfg(test)]
    recover_read_only_call_count: std::cell::Cell<usize>,
    writer_epoch: WriterEpochId,
    segment_id: WalSegmentId,
    next_lsn: Lsn,
    previous_frame_digest: Hash,
    previous_committed_transaction_digest: Hash,
    durability_mode: WalDurabilityMode,
    payload_codec_id: PayloadCodecId,
    payload_schema_id: PayloadSchemaId,
    digest_domain: Hash,
    submission_frontier_digest: Hash,
    receipt_frontier_digest: Hash,
    runtime_state_frontier_digest: Hash,
    executable_operation_catalog_frontier_digest: Hash,
    executable_operation_receipt_frontier_digest: Hash,
    causal_anchor_frontier_digest: Hash,
    causal_history_frontier_digest: Hash,
    causal_anchor_claim_projection: CausalAnchorClaimProjection,
}

impl TrustedRuntimeWal {
    /// Builds a WAL adapter backed by an in-memory store.
    pub fn new_in_memory() -> Result<Self, TrustedRuntimeWalError> {
        Self::from_config(TrustedRuntimeWalConfig::in_memory())
    }

    #[cfg(any(test, feature = "host_test"))]
    fn new_in_memory_at_lsn(next_lsn: Lsn) -> Result<Self, TrustedRuntimeWalError> {
        Self::from_config(TrustedRuntimeWalConfig::in_memory().with_next_lsn(next_lsn))
    }

    /// Builds a WAL adapter from host-owned configuration.
    pub fn from_config(config: TrustedRuntimeWalConfig) -> Result<Self, TrustedRuntimeWalError> {
        let TrustedRuntimeWalConfig { store, next_lsn } = config;
        let mut store = TrustedRuntimeWalStore::open(store)?;
        let recovery_report = store.recover_for_writer()?;
        let recovered_cursor = TrustedRuntimeWalCursor::from_recovery(&recovery_report)?;
        let evidence_catalog =
            crate::evidence::CausalSegmentCatalog::from_recovery_scan(&recovery_report)?;
        let next_lsn = if recovered_cursor.has_committed_history {
            recovered_cursor.next_lsn
        } else {
            next_lsn
        };
        let writer_epoch = WriterEpochId::from_hash(trusted_runtime_wal_digest("writer-epoch"));
        store.acquire_writer_epoch(WriterEpochRequest {
            epoch_id: writer_epoch,
            storage_fencing_token: trusted_runtime_wal_digest("fencing-token"),
            process_identity: trusted_runtime_wal_digest("process"),
            host_identity: trusted_runtime_wal_digest("host"),
            started_at_lsn: next_lsn,
            previous_epoch_id: None,
            previous_epoch_final_commit_digest: None,
            lease_or_lock_evidence: trusted_runtime_wal_digest("lease"),
        })?;
        let durability_mode = store.durability_mode();
        Ok(Self {
            store,
            writer_epoch,
            segment_id: WalSegmentId::from_raw(1),
            next_lsn,
            previous_frame_digest: recovered_cursor.previous_frame_digest,
            previous_committed_transaction_digest: recovered_cursor
                .previous_committed_transaction_digest,
            durability_mode,
            payload_codec_id: PayloadCodecId::from_hash(trusted_runtime_wal_digest(
                "payload-codec",
            )),
            payload_schema_id: PayloadSchemaId::from_hash(trusted_runtime_wal_digest(
                "payload-schema",
            )),
            digest_domain: trusted_runtime_wal_digest("digest-domain"),
            submission_frontier_digest: recovered_cursor.submission_frontier_digest,
            receipt_frontier_digest: recovered_cursor.receipt_frontier_digest,
            runtime_state_frontier_digest: recovered_cursor.runtime_state_frontier_digest,
            executable_operation_catalog_frontier_digest: recovered_cursor
                .executable_operation_catalog_frontier_digest,
            executable_operation_receipt_frontier_digest: recovered_cursor
                .executable_operation_receipt_frontier_digest,
            causal_anchor_frontier_digest: recovered_cursor.causal_anchor_frontier_digest,
            causal_history_frontier_digest: recovered_cursor.causal_history_frontier_digest,
            causal_anchor_claim_projection: recovered_cursor.causal_anchor_claim_projection,
            evidence_catalog: Some(evidence_catalog),
            evidence_catalog_posture: EvidenceCatalogPosture::Fresh,
            #[cfg(any(test, feature = "host_test"))]
            fail_next_evidence_catalog_update: false,
            #[cfg(test)]
            recover_read_only_call_count: std::cell::Cell::new(0),
        })
    }

    /// Returns the configured store kind as read-only evidence.
    #[must_use]
    pub fn store_kind(&self) -> TrustedRuntimeWalStoreKind {
        self.store.kind()
    }

    /// Returns committed WAL markers recorded by the adapter.
    #[must_use]
    pub fn commits(&self) -> Vec<WalTransactionCommit> {
        self.store.read_commits()
    }

    /// Returns WAL frames recorded by the adapter.
    #[must_use]
    pub fn frames(&self) -> Vec<crate::causal_wal::WalFrame> {
        self.store.read_frames()
    }

    /// Returns a clone of the underlying in-memory store for recovery tests,
    /// if this adapter is backed by the in-memory store kind.
    #[must_use]
    pub fn cloned_store(&self) -> Option<InMemoryWalStore> {
        self.store.cloned_in_memory_store()
    }

    /// Recovers submission and receipt indexes from committed WAL transactions
    /// without scheduler callbacks.
    pub fn recover_read_only(&self) -> Result<TrustedRuntimeWalRecovery, TrustedRuntimeWalError> {
        #[cfg(test)]
        self.recover_read_only_call_count
            .set(self.recover_read_only_call_count.get() + 1);
        let report = self.store.recover_read_only()?;
        let submissions = recover_submission_index(&report).map_err(WalRecoveryError::from)?;
        let receipts = recover_receipt_index(&report).map_err(WalRecoveryError::from)?;
        let (causal_anchor_history, causal_history_frontiers) =
            recover_witnessed_causal_anchor_history(&report)?;
        let (witnessed_submissions, missing_submission_envelopes) =
            recover_witnessed_submission_material(&report, &submissions)?;
        let runtime_state = recover_runtime_state_delta_material(&report)?;
        let operation_material =
            recover_echo_operation_material(&report, &runtime_state.provenance_entries)?;
        let provenance_entries = runtime_state.provenance_entries;
        let receipt_correlations = runtime_state.receipt_correlations;
        let echo_operation_action_outcomes = runtime_state.echo_operation_action_outcomes;
        let echo_operation_action_decisions = runtime_state.echo_operation_action_decisions;
        let echo_operation_action_installations_before_tick =
            runtime_state.echo_operation_action_installations_before_tick;
        let missing_runtime_state_deltas = runtime_state.missing_runtime_state_deltas;
        let installed_echo_operations = operation_material.installations;
        let echo_operation_receipts = operation_material.receipts;
        validate_recovered_causal_parent_evidence(&witnessed_submissions, &receipt_correlations)?;
        validate_recovered_echo_operation_action_outcomes(
            &witnessed_submissions,
            &receipt_correlations,
            &provenance_entries,
            &installed_echo_operations,
            &echo_operation_action_outcomes,
            &echo_operation_action_decisions,
            &echo_operation_action_installations_before_tick,
        )?;
        let certificate = runtime_wal_recovery_certificate(
            &report,
            &RecoveredRuntimeWalIndexEvidence {
                submissions: &submissions,
                receipts: &receipts,
                witnessed_submissions: &witnessed_submissions,
                missing_submission_envelopes: &missing_submission_envelopes,
                provenance_entries: &provenance_entries,
                missing_runtime_state_deltas: &missing_runtime_state_deltas,
                causal_anchor_history: &causal_anchor_history,
                installed_echo_operations: &installed_echo_operations,
                echo_operation_receipts: &echo_operation_receipts,
                echo_operation_action_outcomes: &echo_operation_action_outcomes,
            },
        )?;
        Ok(TrustedRuntimeWalRecovery {
            certificate,
            submissions,
            receipts,
            witnessed_submissions,
            missing_submission_envelopes,
            provenance_entries,
            missing_runtime_state_deltas,
            receipt_correlations,
            causal_anchor_history,
            installed_echo_operations,
            echo_operation_receipts,
            echo_operation_action_outcomes,
            echo_operation_action_decisions,
            echo_operation_action_installations_before_tick,
            causal_history_frontiers,
        })
    }

    /// Recovers the causal segment catalog from committed WAL transactions.
    pub fn recover_evidence_catalog_read_only(
        &self,
    ) -> Result<crate::evidence::CausalSegmentCatalog, TrustedRuntimeWalError> {
        let report = self.store.recover_read_only()?;
        crate::evidence::CausalSegmentCatalog::from_recovery_scan(&report)
            .map_err(TrustedRuntimeWalError::EvidenceCatalog)
    }

    /// Returns the live evidence catalog if it exists.
    #[must_use]
    pub fn evidence_catalog(&self) -> Option<&crate::evidence::CausalSegmentCatalog> {
        self.evidence_catalog.as_ref()
    }

    /// Returns the current posture of the live evidence catalog cache.
    #[must_use]
    pub fn evidence_catalog_posture(&self) -> &EvidenceCatalogPosture {
        &self.evidence_catalog_posture
    }

    /// Returns the number of committed submission-intake transactions.
    #[must_use]
    pub fn submission_acceptance_count(&self) -> usize {
        self.store
            .read_commits()
            .into_iter()
            .filter(|commit| commit.transaction_kind == WalTransactionKind::SubmissionIntake)
            .count()
    }

    /// Returns the number of committed scheduler-tick transactions.
    #[must_use]
    pub fn scheduler_tick_count(&self) -> usize {
        self.store
            .read_commits()
            .into_iter()
            .filter(|commit| commit.transaction_kind == WalTransactionKind::SchedulerTick)
            .count()
    }

    fn current_causal_anchor_basis(&self) -> CausalFrontierRef {
        CausalFrontierRef::from_digest(self.causal_history_frontier_digest)
    }

    fn causal_anchor_by_id(
        &self,
        anchor_id: &CausalAnchorId,
    ) -> Result<Option<RecoveredCausalAnchorAdmission>, TrustedRuntimeWalError> {
        Ok(self
            .recover_read_only()?
            .causal_anchor_by_id(anchor_id)
            .cloned())
    }

    fn causal_anchor_by_id_at_basis(
        &self,
        anchor_id: &CausalAnchorId,
        basis: &CausalFrontierRef,
    ) -> Result<Option<RecoveredCausalAnchorAdmission>, TrustedRuntimeWalError> {
        Ok(self
            .recover_read_only()?
            .causal_anchor_by_id_at_basis(anchor_id, basis)?
            .cloned())
    }

    fn causal_anchor_by_claim(
        &self,
        claim_digest: &Hash,
        support_policy_digest: Option<&Hash>,
    ) -> Result<Option<RecoveredCausalAnchorAdmission>, TrustedRuntimeWalError> {
        self.causal_anchor_claim_projection
            .find(claim_digest, support_policy_digest)
    }

    fn has_submission_acceptance(
        &self,
        submission_id: Hash,
        canonical_envelope_digest: Hash,
    ) -> Result<bool, TrustedRuntimeWalError> {
        let recovery = self.recover_read_only()?;
        Ok(matches!(
            recovery
                .submissions
                .retry_posture(submission_id, canonical_envelope_digest),
            SubmissionRetryPosture::AlreadyAcceptedPending
                | SubmissionRetryPosture::AlreadyDecidedApplied
                | SubmissionRetryPosture::AlreadyDecidedRejected
                | SubmissionRetryPosture::AlreadyObstructed
        ))
    }

    fn uses_filesystem_store(&self) -> bool {
        self.store.is_filesystem()
    }

    #[cfg(any(test, feature = "host_test"))]
    fn replace_filesystem_fault_plan_for_test(
        &mut self,
        fault_plan: FilesystemWalFaultPlan,
    ) -> Result<(), TrustedRuntimeWalError> {
        self.store
            .replace_filesystem_fault_plan_for_test(fault_plan)?;
        Ok(())
    }

    fn refresh_cursor_from_store_for_writer(&mut self) -> Result<(), TrustedRuntimeWalError> {
        let report = self.store.recover_for_writer()?;
        let cursor = TrustedRuntimeWalCursor::from_recovery(&report)?;
        match crate::evidence::CausalSegmentCatalog::from_recovery_scan(&report) {
            Ok(catalog) => {
                self.evidence_catalog = Some(catalog);
                self.evidence_catalog_posture = EvidenceCatalogPosture::Fresh;
            }
            Err(_) => {
                self.evidence_catalog_posture = EvidenceCatalogPosture::NeedsRebuild {
                    reason: *blake3::hash(b"catalog_recovery_rebuild_error").as_bytes(),
                    last_good_commit: self.previous_committed_transaction_digest,
                };
            }
        }
        self.next_lsn = cursor.next_lsn;
        self.previous_frame_digest = cursor.previous_frame_digest;
        self.previous_committed_transaction_digest = cursor.previous_committed_transaction_digest;
        self.submission_frontier_digest = cursor.submission_frontier_digest;
        self.receipt_frontier_digest = cursor.receipt_frontier_digest;
        self.runtime_state_frontier_digest = cursor.runtime_state_frontier_digest;
        self.executable_operation_catalog_frontier_digest =
            cursor.executable_operation_catalog_frontier_digest;
        self.executable_operation_receipt_frontier_digest =
            cursor.executable_operation_receipt_frontier_digest;
        self.causal_anchor_frontier_digest = cursor.causal_anchor_frontier_digest;
        self.causal_history_frontier_digest = cursor.causal_history_frontier_digest;
        self.causal_anchor_claim_projection = cursor.causal_anchor_claim_projection;
        Ok(())
    }

    fn recover_filesystem_submission_acceptance_after_error(
        &mut self,
        submission_id: Hash,
        canonical_envelope_digest: Hash,
    ) -> bool {
        if !self.uses_filesystem_store() {
            return false;
        }
        if self.refresh_cursor_from_store_for_writer().is_err() {
            return false;
        }
        self.has_submission_acceptance(submission_id, canonical_envelope_digest)
            .unwrap_or(false)
    }

    fn recover_filesystem_tick_commit_after_error(
        &mut self,
        correlation: &ReceiptCorrelationRecord,
    ) -> bool {
        if !self.uses_filesystem_store() {
            return false;
        }
        if self.refresh_cursor_from_store_for_writer().is_err() {
            return false;
        }
        let Ok(recovery) = self.recover_read_only() else {
            return false;
        };
        recovery
            .receipts
            .receipt_by_submission
            .get(&correlation.submission_id)
            == Some(&correlation.causal_receipt_ref)
    }

    fn recover_filesystem_causal_anchor_after_error(
        &mut self,
        claim_digest: &Hash,
        support_policy_digest: &Hash,
    ) -> Option<RecoveredCausalAnchorAdmission> {
        if !self.uses_filesystem_store() {
            return None;
        }
        if self.refresh_cursor_from_store_for_writer().is_err() {
            return None;
        }
        self.causal_anchor_by_claim(claim_digest, Some(support_policy_digest))
            .ok()
            .flatten()
    }

    fn recover_filesystem_executable_operation_installation_after_error(
        &mut self,
        package_id: crate::EchoOperationPackageIdV1,
    ) -> bool {
        if !self.uses_filesystem_store() {
            return false;
        }
        if self.refresh_cursor_from_store_for_writer().is_err() {
            return false;
        }
        self.recover_read_only().is_ok_and(|recovery| {
            recovery
                .installed_echo_operations
                .iter()
                .any(|installed| installed.package_id() == package_id)
        })
    }

    fn recover_filesystem_executable_operation_tick_after_error(
        &mut self,
        receipt_digest: Hash,
    ) -> bool {
        if !self.uses_filesystem_store() {
            return false;
        }
        if self.refresh_cursor_from_store_for_writer().is_err() {
            return false;
        }
        self.recover_read_only().is_ok_and(|recovery| {
            recovery
                .echo_operation_receipts
                .iter()
                .any(|receipt| receipt.digest() == receipt_digest)
        })
    }

    fn record_submission_acceptance(
        &mut self,
        envelope: &IngressEnvelope,
        handle: IntentSubmissionHandle,
    ) -> Result<WalTransactionCommit, TrustedRuntimeWalError> {
        let record = SubmissionAcceptanceRecord {
            submission_id: handle.submission_id,
            canonical_envelope_digest: envelope.ingress_id(),
            idempotency_key_digest: None,
            acceptance_evidence_digest: acceptance_evidence_digest(handle),
        };
        let next_submission_frontier =
            submission_frontier_digest(self.submission_frontier_digest, record);
        let transaction = build_submission_acceptance_with_material_transaction(
            self.builder(
                WalTransactionKind::SubmissionIntake,
                WalAppendAuthority::SubmissionIntake,
                WalTransactionId::from_hash(submission_transaction_digest(handle, record)),
            ),
            record,
            submission_envelope_record(envelope, handle),
            vec![AffectedFrontier {
                kind: AffectedFrontierKind::SubmissionQueue,
                before_digest: self.submission_frontier_digest,
                after_digest: next_submission_frontier,
            }],
        )?;
        let commit = self.append_transaction(transaction)?;
        self.submission_frontier_digest = next_submission_frontier;
        Ok(commit)
    }

    fn record_tick_receipt(
        &mut self,
        correlation: &ReceiptCorrelationRecord,
        decision: WalTickDecision,
        state_delta: &WalRuntimeStateDeltaRecord,
        state_delta_digest: Hash,
    ) -> Result<WalTransactionCommit, TrustedRuntimeWalError> {
        let receipt = TickReceiptRecord {
            receipt_ref: correlation.causal_receipt_ref,
            decision,
        };
        let wal_correlation = WalReceiptCorrelationRecord {
            receipt_ref: correlation.causal_receipt_ref,
            causal_parent_receipts: correlation.causal_parent_receipts.clone(),
        };
        let next_receipt_frontier =
            receipt_frontier_digest(self.receipt_frontier_digest, receipt, &wal_correlation);
        let next_runtime_frontier = runtime_state_frontier_digest(
            self.runtime_state_frontier_digest,
            correlation,
            state_delta_digest,
        );
        let transaction = build_replayable_tick_transaction(
            self.builder(
                WalTransactionKind::SchedulerTick,
                WalAppendAuthority::TrustedScheduler,
                WalTransactionId::from_hash(tick_transaction_digest(
                    correlation,
                    decision,
                    state_delta_digest,
                )),
            ),
            receipt,
            wal_correlation,
            state_delta.to_payload_bytes()?,
            vec![
                AffectedFrontier {
                    kind: AffectedFrontierKind::ReceiptIndex,
                    before_digest: self.receipt_frontier_digest,
                    after_digest: next_receipt_frontier,
                },
                AffectedFrontier {
                    kind: AffectedFrontierKind::RuntimeState,
                    before_digest: self.runtime_state_frontier_digest,
                    after_digest: next_runtime_frontier,
                },
            ],
        )?;
        let commit = self.append_transaction(transaction)?;
        self.receipt_frontier_digest = next_receipt_frontier;
        self.runtime_state_frontier_digest = next_runtime_frontier;
        Ok(commit)
    }

    fn record_tick_receipt_batch(
        &mut self,
        correlations: &[(ReceiptCorrelationRecord, WalTickDecision)],
        action_outcomes_by_submission: &BTreeMap<Hash, EchoOperationActionOutcomeV1>,
        state_delta: &WalRuntimeStateDeltaRecord,
        state_delta_digest: Hash,
    ) -> Result<WalTransactionCommit, TrustedRuntimeWalError> {
        let mut correlations = correlations.to_vec();
        correlations.sort_by_key(|(correlation, _)| correlation.ingress_id);
        let Some((first_correlation, _)) = correlations.first() else {
            return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
        };
        if correlations
            .windows(2)
            .any(|pair| pair[0].0.ingress_id == pair[1].0.ingress_id)
        {
            return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
        }
        let same_tick = correlations.iter().all(|(correlation, _)| {
            correlation.head_key == first_correlation.head_key
                && correlation.commit_global_tick == first_correlation.commit_global_tick
                && correlation.worldline_tick_after == first_correlation.worldline_tick_after
                && correlation.tick_receipt_digest == first_correlation.tick_receipt_digest
                && correlation.commit_hash == first_correlation.commit_hash
        });
        if !same_tick {
            return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
        }
        let first_has_action_outcome =
            action_outcomes_by_submission.contains_key(&first_correlation.submission_id);
        if correlations.iter().any(|(correlation, _)| {
            action_outcomes_by_submission.contains_key(&correlation.submission_id)
                != first_has_action_outcome
        }) {
            return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
        }

        let mut next_receipt_frontier = self.receipt_frontier_digest;
        let mut records = Vec::with_capacity(correlations.len());
        for (correlation, decision) in &correlations {
            let receipt = TickReceiptRecord {
                receipt_ref: correlation.causal_receipt_ref,
                decision: *decision,
            };
            let wal_correlation = WalReceiptCorrelationRecord {
                receipt_ref: correlation.causal_receipt_ref,
                causal_parent_receipts: correlation.causal_parent_receipts.clone(),
            };
            next_receipt_frontier =
                receipt_frontier_digest(next_receipt_frontier, receipt, &wal_correlation);
            let action_outcome = action_outcomes_by_submission
                .get(&correlation.submission_id)
                .map(|outcome| {
                    if wal_tick_decision_for_action_outcome(outcome) != *decision {
                        return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
                    }
                    retain_action_outcome_v1(
                        correlation.submission_id,
                        correlation.ingress_id,
                        outcome,
                    )
                    .map_err(TrustedRuntimeWalError::from)
                })
                .transpose()?;
            records.push((receipt, wal_correlation, action_outcome));
        }
        let next_runtime_frontier = runtime_state_frontier_digest(
            self.runtime_state_frontier_digest,
            first_correlation,
            state_delta_digest,
        );
        let transaction = build_replayable_tick_batch_transaction(
            self.builder(
                WalTransactionKind::SchedulerTick,
                WalAppendAuthority::TrustedScheduler,
                WalTransactionId::from_hash(tick_batch_transaction_digest(
                    &correlations,
                    state_delta_digest,
                )),
            ),
            records,
            state_delta.to_payload_bytes()?,
            vec![
                AffectedFrontier {
                    kind: AffectedFrontierKind::ReceiptIndex,
                    before_digest: self.receipt_frontier_digest,
                    after_digest: next_receipt_frontier,
                },
                AffectedFrontier {
                    kind: AffectedFrontierKind::RuntimeState,
                    before_digest: self.runtime_state_frontier_digest,
                    after_digest: next_runtime_frontier,
                },
            ],
        )?;
        let commit = self.append_transaction(transaction)?;
        self.receipt_frontier_digest = next_receipt_frontier;
        self.runtime_state_frontier_digest = next_runtime_frontier;
        Ok(commit)
    }

    fn record_executable_operation_installation(
        &mut self,
        retained_installation_bytes: &[u8],
    ) -> Result<WalTransactionCommit, TrustedRuntimeWalError> {
        let installed = recover_installation_v1(retained_installation_bytes)?;
        let next_catalog_frontier = executable_operation_catalog_frontier_digest(
            self.executable_operation_catalog_frontier_digest,
            installed.package_id(),
            retained_installation_bytes,
        );
        let transaction = build_executable_operation_installation_transaction(
            self.builder(
                WalTransactionKind::ExecutableOperationInstallation,
                WalAppendAuthority::RuntimeControl,
                WalTransactionId::from_hash(executable_operation_installation_transaction_digest(
                    self.executable_operation_catalog_frontier_digest,
                    installed.package_id(),
                    retained_installation_bytes,
                )),
            ),
            retained_installation_bytes.to_vec(),
            vec![AffectedFrontier {
                kind: AffectedFrontierKind::ExecutableOperationCatalog,
                before_digest: self.executable_operation_catalog_frontier_digest,
                after_digest: next_catalog_frontier,
            }],
        )?;
        let commit = self.append_transaction(transaction)?;
        self.executable_operation_catalog_frontier_digest = next_catalog_frontier;
        Ok(commit)
    }

    fn record_executable_operation_tick(
        &mut self,
        receipt: &EchoOperationReceiptV1,
        retained_execution_bytes: Vec<u8>,
        state_delta: &WalRuntimeStateDeltaRecord,
        state_delta_digest: Hash,
        installed: &InstalledEchoOperationV1,
    ) -> Result<WalTransactionCommit, TrustedRuntimeWalError> {
        let retained_receipt = recover_committed_execution_receipt_v1(&retained_execution_bytes)?;
        if &retained_receipt != receipt || state_delta.digest()? != state_delta_digest {
            return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "live operation material disagrees with retained bytes",
            });
        }
        validate_receipt_installation_v1(receipt, installed)?;
        validate_operation_receipt_state_delta(receipt, state_delta, installed)?;
        let next_receipt_frontier = executable_operation_receipt_frontier_digest(
            self.executable_operation_receipt_frontier_digest,
            receipt.digest(),
        );
        let entry = state_delta.provenance_entry();
        let worldline_tick_after = entry
            .worldline_tick
            .checked_add(1)
            .ok_or(RetainedProvenanceError::Inconsistent("worldline tick"))?;
        let next_runtime_frontier = runtime_state_frontier_digest_from_fields(
            self.runtime_state_frontier_digest,
            entry.expected.commit_hash,
            state_delta_digest,
            entry.commit_global_tick,
            worldline_tick_after,
        );
        let transaction = build_executable_operation_tick_transaction(
            self.builder(
                WalTransactionKind::ExecutableOperationTick,
                WalAppendAuthority::ExecutionKernel,
                WalTransactionId::from_hash(executable_operation_tick_transaction_digest(
                    self.executable_operation_receipt_frontier_digest,
                    self.runtime_state_frontier_digest,
                    receipt.digest(),
                    state_delta_digest,
                )),
            ),
            retained_execution_bytes,
            state_delta.to_payload_bytes()?,
            vec![
                AffectedFrontier {
                    kind: AffectedFrontierKind::ExecutableOperationReceiptIndex,
                    before_digest: self.executable_operation_receipt_frontier_digest,
                    after_digest: next_receipt_frontier,
                },
                AffectedFrontier {
                    kind: AffectedFrontierKind::RuntimeState,
                    before_digest: self.runtime_state_frontier_digest,
                    after_digest: next_runtime_frontier,
                },
            ],
        )?;
        let commit = self.append_transaction(transaction)?;
        self.executable_operation_receipt_frontier_digest = next_receipt_frontier;
        self.runtime_state_frontier_digest = next_runtime_frontier;
        Ok(commit)
    }

    fn record_causal_anchor_admission(
        &mut self,
        claim: CausalAnchorClaim,
        support_policy_digest: Hash,
    ) -> Result<RecoveredCausalAnchorAdmission, TrustedRuntimeWalError> {
        let transaction_id = WalTransactionId::from_hash(causal_anchor_transaction_digest(
            self.causal_anchor_frontier_digest,
            claim.claim_digest(),
            &support_policy_digest,
        ));
        let (fact, receipt) = prepare_causal_anchor_admission(
            claim.clone(),
            support_policy_digest,
            self.writer_epoch.as_hash(),
            transaction_id.as_hash(),
            self.next_lsn.as_u64(),
        );
        let next_causal_anchor_frontier = causal_anchor_frontier_digest_from_evidence(
            self.causal_anchor_frontier_digest,
            &fact,
            &receipt,
        );
        let transaction = build_causal_anchor_admission_transaction(
            self.causal_anchor_builder(transaction_id),
            claim,
            support_policy_digest,
            vec![AffectedFrontier {
                kind: AffectedFrontierKind::CausalAnchorIndex,
                before_digest: self.causal_anchor_frontier_digest,
                after_digest: next_causal_anchor_frontier,
            }],
        )?;
        let commit = self.append_transaction(transaction)?;
        self.causal_anchor_frontier_digest = next_causal_anchor_frontier;
        let admission = RecoveredCausalAnchorAdmission::from_committed_wal_evidence(
            fact,
            receipt,
            commit.transaction_id,
            commit.last_lsn,
            commit.commit_digest,
        );
        self.causal_anchor_claim_projection
            .insert(admission.clone());
        Ok(admission)
    }

    fn append_transaction(
        &mut self,
        transaction: WalCommittedTransaction,
    ) -> Result<WalTransactionCommit, TrustedRuntimeWalError> {
        let next_causal_history_frontier = logical_causal_history_frontier_digest(
            self.causal_history_frontier_digest,
            transaction.commit.transaction_kind,
            &transaction.frames,
        );
        let last_frame_digest = transaction.frames.last().map_or(
            self.previous_frame_digest,
            crate::causal_wal::WalFrame::digest,
        );
        let next_lsn = transaction
            .commit
            .last_lsn
            .checked_next()
            .ok_or(WalBuildError::LsnOverflow)?;
        let last_good_commit = self.previous_committed_transaction_digest;
        let commit = transaction.commit.clone();
        let frames = transaction.frames.clone();
        self.store.append_transaction(transaction)?;
        self.next_lsn = next_lsn;
        self.previous_frame_digest = last_frame_digest;
        self.previous_committed_transaction_digest = commit.commit_digest;
        self.causal_history_frontier_digest = next_causal_history_frontier;
        self.try_update_evidence_catalog_after_commit(&commit, &frames, last_good_commit);
        Ok(commit)
    }

    fn try_update_evidence_catalog_after_commit(
        &mut self,
        commit: &WalTransactionCommit,
        frames: &[crate::causal_wal::WalFrame],
        last_good_commit: Hash,
    ) {
        use crate::evidence::CommittedWalObserver;
        #[cfg(any(test, feature = "host_test"))]
        if self.fail_next_evidence_catalog_update {
            self.fail_next_evidence_catalog_update = false;
            self.evidence_catalog_posture = EvidenceCatalogPosture::NeedsRebuild {
                reason: *blake3::hash(b"catalog_update_error").as_bytes(),
                last_good_commit,
            };
            return;
        }
        if let Some(catalog) = self.evidence_catalog.as_mut() {
            let view = crate::evidence::CommittedWalView { commit, frames };
            if catalog.observe_committed_wal(view).is_err() {
                self.evidence_catalog_posture = EvidenceCatalogPosture::NeedsRebuild {
                    reason: *blake3::hash(b"catalog_update_error").as_bytes(),
                    last_good_commit,
                };
            }
        }
    }

    fn builder(
        &self,
        kind: WalTransactionKind,
        authority: WalAppendAuthority,
        transaction_id: WalTransactionId,
    ) -> WalTransactionBuilder {
        WalTransactionBuilder::new(
            self.writer_epoch,
            self.segment_id,
            transaction_id,
            kind,
            authority,
            self.next_lsn,
            self.previous_frame_digest,
            self.previous_committed_transaction_digest,
            self.durability_mode,
            self.payload_codec_id,
            self.payload_schema_id,
            1,
            1,
            self.digest_domain,
        )
    }

    fn causal_anchor_builder(&self, transaction_id: WalTransactionId) -> WalTransactionBuilder {
        WalTransactionBuilder::new_causal_anchor_admission(
            self.writer_epoch,
            self.segment_id,
            transaction_id,
            self.next_lsn,
            self.previous_frame_digest,
            self.previous_committed_transaction_digest,
            self.durability_mode,
            self.payload_codec_id,
            self.payload_schema_id,
            1,
            1,
            self.digest_domain,
        )
    }
}

#[derive(Clone, Debug)]
enum TrustedRuntimeWalStore {
    InMemory(InMemoryWalStore),
    Filesystem(FilesystemWalStore),
}

impl TrustedRuntimeWalStore {
    fn open(config: TrustedRuntimeWalStoreConfig) -> Result<Self, TrustedRuntimeWalError> {
        match config {
            TrustedRuntimeWalStoreConfig::InMemory => Ok(Self::InMemory(InMemoryWalStore::new())),
            TrustedRuntimeWalStoreConfig::Filesystem {
                root,
                segment_id,
                #[cfg(any(test, feature = "host_test"))]
                fault_plan,
            } => {
                #[cfg(any(test, feature = "host_test"))]
                let store = match fault_plan {
                    Some(plan) => {
                        FilesystemWalStore::open_with_fault_plan_for_test(root, segment_id, plan)?
                    }
                    None => FilesystemWalStore::open(root, segment_id)?,
                };
                #[cfg(not(any(test, feature = "host_test")))]
                let store = FilesystemWalStore::open(root, segment_id)?;
                Ok(Self::Filesystem(store))
            }
        }
    }

    fn kind(&self) -> TrustedRuntimeWalStoreKind {
        match self {
            Self::InMemory(_) => TrustedRuntimeWalStoreKind::InMemory,
            Self::Filesystem(_) => TrustedRuntimeWalStoreKind::Filesystem,
        }
    }

    fn is_filesystem(&self) -> bool {
        matches!(self, Self::Filesystem(_))
    }

    fn durability_mode(&self) -> WalDurabilityMode {
        match self {
            Self::InMemory(_) => WalDurabilityMode::Buffered,
            Self::Filesystem(_) => WalDurabilityMode::StrictFilesystem,
        }
    }

    fn recover_for_writer(&self) -> Result<RecoveryScanReport, WalRecoveryError> {
        match self {
            Self::InMemory(store) => recover_runtime_wal_store_read_only(store),
            Self::Filesystem(store) => {
                recover_filesystem_store(store.root(), RecoveryAccessMode::Writable)
            }
        }
    }

    fn recover_read_only(&self) -> Result<RecoveryScanReport, WalRecoveryError> {
        match self {
            Self::InMemory(store) => recover_runtime_wal_store_read_only(store),
            Self::Filesystem(store) => {
                recover_filesystem_store(store.root(), RecoveryAccessMode::ReadOnly)
            }
        }
    }

    #[cfg(any(test, feature = "host_test"))]
    fn replace_filesystem_fault_plan_for_test(
        &mut self,
        fault_plan: FilesystemWalFaultPlan,
    ) -> Result<(), WalStoreError> {
        match self {
            Self::InMemory(_) => Err(WalStoreError::Io(
                "cannot inject filesystem WAL fault plan into in-memory store".to_owned(),
            )),
            Self::Filesystem(store) => {
                store.replace_fault_plan_for_test(fault_plan);
                Ok(())
            }
        }
    }

    fn append_transaction(
        &mut self,
        transaction: WalCommittedTransaction,
    ) -> Result<(), WalStoreError> {
        match self {
            Self::InMemory(store) => store.append_transaction(transaction),
            Self::Filesystem(store) => store.append_transaction(transaction),
        }
    }

    #[cfg(any(test, feature = "host_test"))]
    fn append_uncommitted_frame(
        &mut self,
        epoch_id: WriterEpochId,
        frame: crate::causal_wal::WalFrame,
    ) -> Result<(), WalStoreError> {
        match self {
            Self::InMemory(store) => store.append_uncommitted_frame(epoch_id, frame),
            Self::Filesystem(store) => store.append_uncommitted_frame(epoch_id, frame),
        }
    }

    fn cloned_in_memory_store(&self) -> Option<InMemoryWalStore> {
        match self {
            Self::InMemory(store) => Some(store.clone()),
            Self::Filesystem(_) => None,
        }
    }
}

impl WalStorePort for TrustedRuntimeWalStore {
    fn acquire_writer_epoch(
        &mut self,
        request: WriterEpochRequest,
    ) -> Result<crate::causal_wal::WriterEpoch, WalStoreError> {
        match self {
            Self::InMemory(store) => store.acquire_writer_epoch(request),
            Self::Filesystem(store) => store.acquire_writer_epoch(request),
        }
    }

    fn append_frame(
        &mut self,
        epoch_id: WriterEpochId,
        frame: crate::causal_wal::WalFrame,
    ) -> Result<(), WalStoreError> {
        match self {
            Self::InMemory(store) => store.append_frame(epoch_id, frame),
            Self::Filesystem(store) => store.append_frame(epoch_id, frame),
        }
    }

    fn flush_commit(
        &mut self,
        epoch_id: WriterEpochId,
        commit: WalTransactionCommit,
    ) -> Result<(), WalStoreError> {
        match self {
            Self::InMemory(store) => store.flush_commit(epoch_id, commit),
            Self::Filesystem(store) => store.flush_commit(epoch_id, commit),
        }
    }

    fn read_frames(&self) -> Vec<crate::causal_wal::WalFrame> {
        match self {
            Self::InMemory(store) => store.read_frames(),
            Self::Filesystem(store) => store.read_frames(),
        }
    }

    fn read_commits(&self) -> Vec<WalTransactionCommit> {
        match self {
            Self::InMemory(store) => store.read_commits(),
            Self::Filesystem(store) => store.read_commits(),
        }
    }

    fn seal_segment(
        &mut self,
        epoch_id: WriterEpochId,
        segment_id: WalSegmentId,
    ) -> Result<crate::causal_wal::WalSegmentSeal, WalStoreError> {
        match self {
            Self::InMemory(store) => store.seal_segment(epoch_id, segment_id),
            Self::Filesystem(store) => store.seal_segment(epoch_id, segment_id),
        }
    }

    fn truncate_tail_after(&mut self, after_lsn: Lsn) -> Result<(), WalStoreError> {
        match self {
            Self::InMemory(store) => store.truncate_tail_after(after_lsn),
            Self::Filesystem(store) => store.truncate_tail_after(after_lsn),
        }
    }

    fn publish_manifest(
        &mut self,
        epoch_id: WriterEpochId,
        manifest: crate::causal_wal::WalManifest,
    ) -> Result<(), WalStoreError> {
        match self {
            Self::InMemory(store) => store.publish_manifest(epoch_id, manifest),
            Self::Filesystem(store) => store.publish_manifest(epoch_id, manifest),
        }
    }

    fn close_epoch(&mut self, epoch_id: WriterEpochId) -> Result<(), WalStoreError> {
        match self {
            Self::InMemory(store) => store.close_epoch(epoch_id),
            Self::Filesystem(store) => store.close_epoch(epoch_id),
        }
    }
}

#[derive(Clone, Debug)]
struct TrustedRuntimeWalCursor {
    has_committed_history: bool,
    next_lsn: Lsn,
    previous_frame_digest: Hash,
    previous_committed_transaction_digest: Hash,
    submission_frontier_digest: Hash,
    receipt_frontier_digest: Hash,
    runtime_state_frontier_digest: Hash,
    executable_operation_catalog_frontier_digest: Hash,
    executable_operation_receipt_frontier_digest: Hash,
    causal_anchor_frontier_digest: Hash,
    causal_history_frontier_digest: Hash,
    causal_anchor_claim_projection: CausalAnchorClaimProjection,
}

struct RecoveredCausalAnchorTraversal {
    history: Vec<WitnessedCausalAnchorAdmission>,
    causal_history_frontiers: Vec<CausalFrontierRef>,
    causal_anchor_frontier_digest: Hash,
}

impl TrustedRuntimeWalCursor {
    fn genesis() -> Self {
        Self {
            has_committed_history: false,
            next_lsn: Lsn::from_raw(0),
            previous_frame_digest: trusted_runtime_wal_digest("previous-frame:genesis"),
            previous_committed_transaction_digest: trusted_runtime_wal_digest(
                "previous-commit:genesis",
            ),
            submission_frontier_digest: trusted_runtime_wal_digest("submission-frontier:genesis"),
            receipt_frontier_digest: trusted_runtime_wal_digest("receipt-frontier:genesis"),
            runtime_state_frontier_digest: trusted_runtime_wal_digest("runtime-frontier:genesis"),
            executable_operation_catalog_frontier_digest: trusted_runtime_wal_digest(
                "executable-operation-catalog-frontier:genesis",
            ),
            executable_operation_receipt_frontier_digest: trusted_runtime_wal_digest(
                "executable-operation-receipt-frontier:genesis",
            ),
            causal_anchor_frontier_digest: causal_anchor_genesis_frontier_digest(),
            causal_history_frontier_digest: causal_history_genesis_frontier_digest(),
            causal_anchor_claim_projection: CausalAnchorClaimProjection::default(),
        }
    }

    fn from_recovery(report: &RecoveryScanReport) -> Result<Self, TrustedRuntimeWalError> {
        let causal_anchor_traversal = traverse_recovered_causal_anchors(report)?;
        let mut cursor = Self::genesis();
        for (index, transaction) in report.transactions.iter().enumerate() {
            cursor.has_committed_history = true;
            cursor.causal_history_frontier_digest =
                causal_anchor_traversal.causal_history_frontiers[index + 1].frontier_digest;
            cursor.next_lsn = transaction
                .commit
                .last_lsn
                .checked_next()
                .ok_or(WalBuildError::LsnOverflow)?;
            cursor.previous_committed_transaction_digest = transaction.commit.commit_digest;
            if let Some(frame) = transaction.frames.last() {
                cursor.previous_frame_digest = frame.digest();
            }
            match transaction.commit.transaction_kind {
                WalTransactionKind::SubmissionIntake => {
                    let record = submission_acceptance_record_from_transaction(transaction)?;
                    cursor.submission_frontier_digest =
                        submission_frontier_digest(cursor.submission_frontier_digest, record);
                }
                WalTransactionKind::SchedulerTick => {
                    let (records, state_delta_digest, provenance_entry) =
                        tick_record_batch_from_transaction(transaction)?;
                    let Some((_, first_correlation, _)) = records.first() else {
                        return Err(decode_trusted_runtime_wal_payload(
                            WalDecodeError::InvalidEmbeddedFrame,
                        ));
                    };
                    for (receipt, correlation, _) in &records {
                        cursor.receipt_frontier_digest = receipt_frontier_digest(
                            cursor.receipt_frontier_digest,
                            *receipt,
                            correlation,
                        );
                    }
                    cursor.runtime_state_frontier_digest = match provenance_entry {
                        Some(entry) => runtime_state_frontier_digest_from_fields(
                            cursor.runtime_state_frontier_digest,
                            entry.expected.commit_hash,
                            state_delta_digest,
                            entry.commit_global_tick,
                            entry
                                .worldline_tick
                                .checked_add(1)
                                .ok_or(RetainedProvenanceError::Inconsistent("worldline tick"))?,
                        ),
                        None => recovered_legacy_runtime_state_frontier_digest(
                            cursor.runtime_state_frontier_digest,
                            first_correlation.clone(),
                            state_delta_digest,
                        ),
                    };
                }
                WalTransactionKind::ExecutableOperationInstallation => {
                    let (installed, retained_bytes) =
                        operation_installation_from_transaction(transaction)?;
                    let catalog_before = cursor.executable_operation_catalog_frontier_digest;
                    let catalog_after = executable_operation_catalog_frontier_digest(
                        catalog_before,
                        installed.package_id(),
                        &retained_bytes,
                    );
                    validate_echo_operation_frontier_root(
                        transaction,
                        &[AffectedFrontier {
                            kind: AffectedFrontierKind::ExecutableOperationCatalog,
                            before_digest: catalog_before,
                            after_digest: catalog_after,
                        }],
                    )?;
                    cursor.executable_operation_catalog_frontier_digest = catalog_after;
                }
                WalTransactionKind::ExecutableOperationTick => {
                    let (receipt, state_delta, state_delta_digest) =
                        operation_tick_records_from_transaction(transaction)?;
                    let receipt_before = cursor.executable_operation_receipt_frontier_digest;
                    let receipt_after = executable_operation_receipt_frontier_digest(
                        receipt_before,
                        receipt.digest(),
                    );
                    let entry = state_delta.provenance_entry();
                    let runtime_before = cursor.runtime_state_frontier_digest;
                    let runtime_after = runtime_state_frontier_digest_from_fields(
                        runtime_before,
                        entry.expected.commit_hash,
                        state_delta_digest,
                        entry.commit_global_tick,
                        entry
                            .worldline_tick
                            .checked_add(1)
                            .ok_or(RetainedProvenanceError::Inconsistent("worldline tick"))?,
                    );
                    validate_echo_operation_frontier_root(
                        transaction,
                        &[
                            AffectedFrontier {
                                kind: AffectedFrontierKind::ExecutableOperationReceiptIndex,
                                before_digest: receipt_before,
                                after_digest: receipt_after,
                            },
                            AffectedFrontier {
                                kind: AffectedFrontierKind::RuntimeState,
                                before_digest: runtime_before,
                                after_digest: runtime_after,
                            },
                        ],
                    )?;
                    cursor.executable_operation_receipt_frontier_digest = receipt_after;
                    cursor.runtime_state_frontier_digest = runtime_after;
                }
                _ => {}
            }
        }
        cursor.causal_anchor_frontier_digest =
            causal_anchor_traversal.causal_anchor_frontier_digest;
        cursor.causal_anchor_claim_projection =
            CausalAnchorClaimProjection::from_history(&causal_anchor_traversal.history);
        Ok(cursor)
    }
}

fn recover_witnessed_causal_anchor_history(
    report: &RecoveryScanReport,
) -> Result<(Vec<WitnessedCausalAnchorAdmission>, Vec<CausalFrontierRef>), TrustedRuntimeWalError> {
    let traversal = traverse_recovered_causal_anchors(report)?;
    Ok((traversal.history, traversal.causal_history_frontiers))
}

fn traverse_recovered_causal_anchors(
    report: &RecoveryScanReport,
) -> Result<RecoveredCausalAnchorTraversal, TrustedRuntimeWalError> {
    let validated = validate_recovered_causal_anchor_history(report)
        .map_err(trusted_runtime_causal_anchor_recovery_error)?;
    let frontiers = validated.causal_history_frontiers;
    let history = validated
        .admissions
        .into_iter()
        .map(|(admission, transaction_index)| {
            WitnessedCausalAnchorAdmission::new(
                RecoveredCausalAnchorAdmission::from_observation(admission),
                frontiers[transaction_index],
                frontiers[transaction_index + 1],
                transaction_index + 1,
            )
        })
        .collect();

    Ok(RecoveredCausalAnchorTraversal {
        history,
        causal_history_frontiers: frontiers,
        causal_anchor_frontier_digest: validated.causal_anchor_frontier_digest,
    })
}

fn trusted_runtime_causal_anchor_recovery_error(
    error: WalRecoveryIndexError,
) -> TrustedRuntimeWalError {
    match error {
        WalRecoveryIndexError::CausalAnchorAdmissionMissing { transaction_id } => {
            TrustedRuntimeWalError::CausalAnchorAdmissionMissing {
                transaction_id: transaction_id.as_hash(),
            }
        }
        WalRecoveryIndexError::CausalAnchorBasisMismatch {
            transaction_id,
            claimed,
            recovered,
        } => TrustedRuntimeWalError::CausalAnchorBasisMismatch {
            transaction_id: transaction_id.as_hash(),
            claimed,
            recovered,
        },
        WalRecoveryIndexError::CausalAnchorFrontierMismatch {
            transaction_id,
            expected,
            actual,
        } => TrustedRuntimeWalError::CausalAnchorFrontierMismatch {
            transaction_id: transaction_id.as_hash(),
            expected,
            actual,
        },
        WalRecoveryIndexError::CausalAnchorIdConflict { anchor_id } => {
            TrustedRuntimeWalError::CausalAnchorIdConflict { anchor_id }
        }
        error => TrustedRuntimeWalError::Recovery(WalRecoveryError::from(error)),
    }
}

fn submission_acceptance_record_from_transaction(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
) -> Result<SubmissionAcceptanceRecord, TrustedRuntimeWalError> {
    let frame =
        required_unique_transaction_frame(transaction, WalRecordKind::SubmissionAcceptedRecorded)?;
    SubmissionAcceptanceRecord::from_payload_bytes(&frame.payload.canonical_bytes)
        .map_err(decode_trusted_runtime_wal_payload)
}

fn recover_witnessed_submission_material(
    report: &RecoveryScanReport,
    submissions: &RecoveredSubmissionIndex,
) -> Result<(WitnessedSubmissionPersistenceSnapshot, Vec<Hash>), TrustedRuntimeWalError> {
    let mut material_by_submission = BTreeMap::new();
    for transaction in &report.transactions {
        if let Some(frame) =
            unique_transaction_frame(transaction, WalRecordKind::SubmissionEnvelopeRetained)?
        {
            let material =
                WalSubmissionEnvelopeRecord::from_payload_bytes(&frame.payload.canonical_bytes)
                    .map_err(decode_trusted_runtime_wal_payload)?;
            if material_by_submission
                .get(&material.submission_id)
                .is_some_and(|existing| existing != &material)
            {
                return Err(TrustedRuntimeWalError::SubmissionEnvelopeConflict {
                    submission_id: material.submission_id,
                });
            }
            material_by_submission.insert(material.submission_id, material);
        }
    }

    let mut records = Vec::new();
    let mut missing = Vec::new();
    for (submission_id, entry) in submissions.entries() {
        let Some(material) = material_by_submission.remove(submission_id) else {
            missing.push(*submission_id);
            continue;
        };
        if material.canonical_envelope_digest != entry.acceptance.canonical_envelope_digest
            || material.submission_generation == 0
        {
            return Err(TrustedRuntimeWalError::SubmissionEnvelopeMismatch {
                submission_id: *submission_id,
            });
        }
        let envelope = IngressEnvelope::from_retained_bytes(&material.retained_envelope_bytes)
            .map_err(|error| TrustedRuntimeWalError::SubmissionEnvelopeDecode {
                submission_id: *submission_id,
                error,
            })?;
        if envelope.ingress_id() != entry.acceptance.canonical_envelope_digest {
            return Err(TrustedRuntimeWalError::SubmissionEnvelopeMismatch {
                submission_id: *submission_id,
            });
        }
        records.push(WitnessedSubmissionPersistenceRecord {
            submission: IntentSubmissionRecord {
                submission_id: *submission_id,
                ingress_id: entry.acceptance.canonical_envelope_digest,
                head_key: material.head_key,
                submission_generation: IngressSubmissionGeneration::from_raw(
                    material.submission_generation,
                ),
            },
            envelope,
        });
    }
    if let Some((submission_id, _)) = material_by_submission.first_key_value() {
        return Err(TrustedRuntimeWalError::SubmissionEnvelopeMismatch {
            submission_id: *submission_id,
        });
    }
    Ok((
        WitnessedSubmissionPersistenceSnapshot::new(records),
        missing,
    ))
}

fn restore_provenance_entries(
    provenance: &mut ProvenanceService,
    entries: &[ProvenanceEntry],
) -> Result<(), TrustedRuntimeHostError> {
    for entry in entries {
        let retained_len = provenance.len(entry.worldline_id)?;
        if entry.worldline_tick.as_u64() < retained_len {
            let existing = provenance.entry(entry.worldline_id, entry.worldline_tick)?;
            if existing != *entry {
                return Err(TrustedRuntimeWalError::RuntimeStateDeltaConflict {
                    worldline_id: entry.worldline_id,
                    worldline_tick: entry.worldline_tick,
                }
                .into());
            }
            continue;
        }
        provenance.append_local_commit(entry.clone())?;
    }
    Ok(())
}

struct RecoveredEchoOperationMaterial {
    installations: Vec<InstalledEchoOperationV1>,
    receipts: Vec<EchoOperationReceiptV1>,
}

fn recover_echo_operation_material(
    report: &RecoveryScanReport,
    provenance_entries: &[ProvenanceEntry],
) -> Result<RecoveredEchoOperationMaterial, TrustedRuntimeWalError> {
    let mut installations = BTreeMap::new();
    let mut operations = BTreeMap::new();
    let mut receipts_by_digest = BTreeMap::new();
    let mut receipts = Vec::new();
    let provenance_by_coordinate = provenance_entries
        .iter()
        .map(|entry| ((entry.worldline_id, entry.worldline_tick), entry))
        .collect::<BTreeMap<_, _>>();

    for transaction in &report.transactions {
        match transaction.commit.transaction_kind {
            WalTransactionKind::ExecutableOperationInstallation => {
                let (installed, _) = operation_installation_from_transaction(transaction)?;
                install_recovered_v1(&mut installations, &mut operations, installed)?;
            }
            WalTransactionKind::ExecutableOperationTick => {
                let (receipt, state_delta, _) =
                    operation_tick_records_from_transaction(transaction)?;
                let installed = installations.get(&receipt.package_id()).ok_or(
                    TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                        detail: "operation receipt precedes its exact package installation",
                    },
                )?;
                validate_receipt_installation_v1(&receipt, installed)?;
                validate_operation_receipt_state_delta(&receipt, &state_delta, installed)?;
                validate_operation_receipt_parent_material(
                    receipt.evaluation_basis(),
                    state_delta.provenance_entry(),
                    &provenance_by_coordinate,
                )?;
                if receipts_by_digest
                    .insert(receipt.digest(), transaction.commit.transaction_id)
                    .is_some()
                {
                    return Err(TrustedRuntimeWalError::EchoOperationReceiptConflict {
                        receipt_digest: receipt.digest(),
                    });
                }
                receipts.push(receipt);
            }
            _ => {}
        }
    }

    Ok(RecoveredEchoOperationMaterial {
        installations: installations.into_values().collect(),
        receipts,
    })
}

/// Recognizes only the canonical patch consequence selected by the exact
/// installed program. Update and create-if-absent are separate executable
/// programs; recovery must not infer meaning from an arbitrary patch that
/// merely resembles one program's operation and slot silhouette.
///
/// `patch.warp_id` is the parent worldline root, while a scoped operation may
/// target a node in a descended WARP instance. The exact `NodeKey` carried by
/// the operation and slots therefore owns operation scope; root validation is
/// performed independently by the surrounding provenance checks.
fn operation_patch_scope_v1(
    patch: &crate::WorldlineTickPatchV1,
    program: &crate::EchoOperationProgramV1,
) -> Option<crate::NodeKey> {
    match program {
        crate::EchoOperationProgramV1::AnchoredNodeAttachmentCompareAndSet {
            required_attachment_type,
            max_replacement_bytes,
            ..
        } => match patch.ops.as_slice() {
            [crate::WarpOp::SetAttachment {
                key,
                value: Some(crate::AttachmentValue::Atom(atom)),
            }] => {
                let crate::AttachmentOwner::Node(node) = key.owner else {
                    return None;
                };
                let attachment_slot = crate::AttachmentKey::node_alpha(node);
                if *key != attachment_slot
                    || atom.type_id != *required_attachment_type
                    || u64::try_from(atom.bytes.len())
                        .map_or(true, |len| len > *max_replacement_bytes)
                    || !operation_patch_inputs_match_v1(patch, node, attachment_slot)
                    || patch.out_slots.as_slice() != [crate::SlotId::Attachment(attachment_slot)]
                {
                    return None;
                }
                Some(node)
            }
            _ => None,
        },
        crate::EchoOperationProgramV1::AnchoredNodeAttachmentCreateIfAbsent {
            required_node_type,
            required_attachment_type,
            max_replacement_bytes,
        } => match patch.ops.as_slice() {
            [crate::WarpOp::UpsertNode { node, record }, crate::WarpOp::SetAttachment {
                key,
                value: Some(crate::AttachmentValue::Atom(atom)),
            }] => {
                let node = *node;
                let crate::AttachmentOwner::Node(attachment_node) = key.owner else {
                    return None;
                };
                let attachment_slot = crate::AttachmentKey::node_alpha(node);
                if attachment_node != node
                    || *key != attachment_slot
                    || record.ty != *required_node_type
                    || atom.type_id != *required_attachment_type
                    || u64::try_from(atom.bytes.len())
                        .map_or(true, |len| len > *max_replacement_bytes)
                    || !operation_patch_inputs_match_v1(patch, node, attachment_slot)
                    || patch.out_slots.as_slice()
                        != [
                            crate::SlotId::Node(node),
                            crate::SlotId::Attachment(attachment_slot),
                        ]
                {
                    return None;
                }
                Some(node)
            }
            _ => None,
        },
    }
}

fn operation_patch_inputs_match_v1(
    patch: &crate::WorldlineTickPatchV1,
    node: crate::NodeKey,
    attachment_slot: crate::AttachmentKey,
) -> bool {
    let mut has_node = false;
    let mut has_attachment = false;
    let mut seen_portals = std::collections::BTreeSet::new();
    let mut has_root_portal = false;

    for slot in &patch.in_slots {
        match *slot {
            crate::SlotId::Node(candidate) if candidate == node && !has_node => {
                has_node = true;
            }
            crate::SlotId::Attachment(candidate)
                if candidate == attachment_slot && !has_attachment =>
            {
                has_attachment = true;
            }
            crate::SlotId::Attachment(portal)
                if operation_attachment_owner_warp_v1(portal) != node.warp_id =>
            {
                if !seen_portals.insert(portal) {
                    return false;
                }
                has_root_portal |= operation_attachment_owner_warp_v1(portal) == patch.warp_id;
            }
            _ => return false,
        }
    }

    has_node
        && has_attachment
        && if node.warp_id == patch.warp_id {
            seen_portals.is_empty()
        } else {
            !seen_portals.is_empty() && has_root_portal
        }
}

fn operation_patch_scope_in_parent_state_v1(
    patch: &crate::WorldlineTickPatchV1,
    program: &crate::EchoOperationProgramV1,
    parent_state: &crate::WorldlineState,
) -> Option<crate::NodeKey> {
    let node = operation_patch_scope_v1(patch, program)?;
    if patch.warp_id != parent_state.root().warp_id {
        return None;
    }
    let descent_stack = operation_descent_stack(parent_state, node.warp_id)?;
    if matches!(
        program,
        crate::EchoOperationProgramV1::AnchoredNodeAttachmentCreateIfAbsent { .. }
    ) {
        let store = parent_state.store(&node.warp_id)?;
        if store.node(&node.local_id).is_some() || store.node_attachment(&node.local_id).is_some() {
            return None;
        }
    }
    let attachment_slot = crate::AttachmentKey::node_alpha(node);
    let expected_inputs = std::iter::once(crate::SlotId::Node(node))
        .chain(std::iter::once(crate::SlotId::Attachment(attachment_slot)))
        .chain(descent_stack.into_iter().map(crate::SlotId::Attachment))
        .collect::<BTreeSet<_>>();
    let actual_inputs = patch.in_slots.iter().copied().collect::<BTreeSet<_>>();
    (patch.in_slots.len() == expected_inputs.len() && actual_inputs == expected_inputs)
        .then_some(node)
}

fn operation_attachment_owner_warp_v1(key: crate::AttachmentKey) -> crate::WarpId {
    match key.owner {
        crate::AttachmentOwner::Node(node) => node.warp_id,
        crate::AttachmentOwner::Edge(edge) => edge.warp_id,
    }
}

fn operation_tick_binds_patch_v1(
    tick_receipt: &crate::TickReceipt,
    patch: &crate::WorldlineTickPatchV1,
    expected_rule_id: Hash,
    program: &crate::EchoOperationProgramV1,
) -> bool {
    let Some(expected_scope) = operation_patch_scope_v1(patch, program) else {
        return false;
    };
    matches!(
        tick_receipt.entries(),
        [tick_entry]
            if patch.rule_pack_id() == expected_rule_id
                && tick_entry.rule_id == expected_rule_id
                && tick_entry.scope == expected_scope
                && tick_entry.scope_hash == crate::scope_hash(&expected_rule_id, &expected_scope)
                && tick_entry.disposition == crate::TickReceiptDisposition::Applied
                && tick_receipt.blocked_by(0).is_empty()
    )
}

fn operation_tick_scope_in_parent_state_v1(
    tick_receipt: &crate::TickReceipt,
    patch: &crate::WorldlineTickPatchV1,
    expected_rule_id: Hash,
    program: &crate::EchoOperationProgramV1,
    parent_state: &crate::WorldlineState,
) -> Option<crate::NodeKey> {
    let expected_scope = operation_patch_scope_in_parent_state_v1(patch, program, parent_state)?;
    matches!(
        tick_receipt.entries(),
        [tick_entry]
            if patch.rule_pack_id() == expected_rule_id
                && tick_entry.rule_id == expected_rule_id
                && tick_entry.scope == expected_scope
                && tick_entry.scope_hash == crate::scope_hash(&expected_rule_id, &expected_scope)
                && tick_entry.disposition == crate::TickReceiptDisposition::Applied
                && tick_receipt.blocked_by(0).is_empty()
    )
    .then_some(expected_scope)
}

fn operation_application_basis_matches_scope_v1(
    program: &crate::EchoOperationProgramV1,
    node: crate::NodeKey,
    application_basis: EchoOperationApplicationBasisV1,
) -> bool {
    match program {
        crate::EchoOperationProgramV1::AnchoredNodeAttachmentCompareAndSet { .. } => true,
        crate::EchoOperationProgramV1::AnchoredNodeAttachmentCreateIfAbsent { .. } => {
            application_basis
                == crate::echo_operation_anchored_node_absent_application_basis_v1(node)
        }
    }
}

fn validate_recovered_echo_operation_parent_states(
    runtime: &WorldlineRuntime,
    recovered_provenance: &ProvenanceService,
    recovery: &TrustedRuntimeWalRecovery,
) -> Result<(), TrustedRuntimeWalError> {
    let installations = recovery
        .installed_echo_operations
        .iter()
        .map(|installed| (installed.package_id(), installed))
        .collect::<BTreeMap<_, _>>();
    let entries = recovery
        .provenance_entries
        .iter()
        .map(|entry| ((entry.worldline_id, entry.worldline_tick), entry))
        .collect::<BTreeMap<_, _>>();

    for receipt in &recovery.echo_operation_receipts {
        let basis = receipt.evaluation_basis();
        let worldline_id = basis.writer_head().worldline_id;
        let frontier = runtime.worldlines().get(&worldline_id).ok_or(
            TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "operation receipt names an unavailable recovery worldline",
            },
        )?;
        let parent_state = recovered_provenance
            .replay_worldline_state_at(worldline_id, frontier.state(), basis.worldline_tick())
            .map_err(|_| TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "operation receipt parent state cannot be reconstructed",
            })?;
        let entry = entries.get(&(worldline_id, basis.worldline_tick())).ok_or(
            TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "operation receipt has no recovered state transition",
            },
        )?;
        let installed = installations.get(&receipt.package_id()).ok_or(
            TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "operation receipt has no recovered installation",
            },
        )?;
        let exact_parent_state_binding = entry
            .patch
            .as_ref()
            .zip(entry.tick_receipt.as_ref())
            .is_some_and(|(patch, tick_receipt)| {
                let exact_scope = operation_tick_scope_in_parent_state_v1(
                    tick_receipt,
                    patch,
                    receipt.installed_operation_id().as_hash(),
                    installed.program(),
                    &parent_state,
                );
                let application_basis_matches = exact_scope.is_some_and(|node| {
                    operation_application_basis_matches_scope_v1(
                        installed.program(),
                        node,
                        basis.application_basis(),
                    )
                });
                parent_state.state_root() == basis.state_root() && application_basis_matches
            });
        if !exact_parent_state_binding {
            return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "operation receipt patch disagrees with its reconstructed parent state",
            });
        }
    }

    let submissions = recovery
        .witnessed_submissions
        .records()
        .iter()
        .map(|record| (record.submission.submission_id, record))
        .collect::<BTreeMap<_, _>>();
    let correlations = recovery
        .receipt_correlations
        .iter()
        .map(|correlation| (correlation.submission_id, correlation))
        .collect::<BTreeMap<_, _>>();
    let outcomes = recovery
        .echo_operation_action_outcomes
        .iter()
        .map(|(submission_id, _, outcome)| (*submission_id, outcome))
        .collect::<BTreeMap<_, _>>();
    let mut reconstructed_preparations = BTreeMap::new();
    let mut action_tick_members = BTreeMap::<
        (
            crate::WriterHeadKey,
            crate::WorldlineTick,
            crate::GlobalTick,
            Hash,
        ),
        Vec<(Hash, Hash)>,
    >::new();
    for (submission_id, ingress_id, outcome) in &recovery.echo_operation_action_outcomes {
        let submission = submissions.get(submission_id).ok_or(
            TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action outcome has no retained submission",
            },
        )?;
        let correlation = correlations.get(submission_id).ok_or(
            TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action outcome has no retained receipt correlation",
            },
        )?;
        let invocation_bytes = echo_operation_action_invocation_bytes_v1(&submission.envelope)
            .ok_or(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action outcome has no canonical invocation",
            })?;
        let invocation = inspect_action_invocation_v1(invocation_bytes).map_err(|_| {
            TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action outcome invocation cannot be inspected",
            }
        })?;
        let installed = installations.get(&invocation.package_id).ok_or(
            TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action outcome has no recovered installation",
            },
        )?;
        let basis = invocation.evaluation_basis;
        let worldline_id = basis.writer_head().worldline_id;
        let frontier = runtime.worldlines().get(&worldline_id).ok_or(
            TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action basis names an unavailable recovery worldline",
            },
        )?;
        let basis_state = recovered_provenance
            .replay_worldline_state_at(worldline_id, frontier.state(), basis.worldline_tick())
            .map_err(|_| TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action basis state cannot be reconstructed",
            })?;
        if basis_state.state_root() != basis.state_root()
            || !action_application_basis_matches_state_v1(installed, invocation_bytes, &basis_state)
                .unwrap_or(false)
            || !evaluation_basis_matches_recovered_coordinate(basis, &entries)
        {
            return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action basis disagrees with reconstructed causal state",
            });
        }
        let tick_before = correlation
            .worldline_tick_after
            .as_u64()
            .checked_sub(1)
            .map(crate::WorldlineTick::from_raw)
            .ok_or(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action outcome has an impossible Tick coordinate",
            })?;
        let basis_is_current =
            basis.writer_head() == correlation.head_key && basis.worldline_tick() == tick_before;
        let basis_changed = matches!(
            outcome,
            EchoOperationActionOutcomeV1::Obstructed(obstruction)
                if obstruction.kind() == crate::EchoOperationObstructionKindV1::BasisChanged
        );
        if basis_changed == basis_is_current {
            return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action basis posture disagrees with the scheduler Tick parent",
            });
        }
        let transition = entries.get(&(worldline_id, tick_before)).ok_or(
            TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action outcome has no recovered scheduler transition",
            },
        )?;
        let policy_id = transition
            .patch
            .as_ref()
            .map(crate::WorldlineTickPatchV1::policy_id)
            .ok_or(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "Action outcome scheduler transition has no retained patch",
            })?;
        let reconstructed = match outcome {
            EchoOperationActionOutcomeV1::Committed(receipt) => {
                let prepared = reconstruct_action_preparation_v1(
                    installed,
                    invocation_bytes,
                    receipt.retained_invocation_admission_maximum_budget(),
                    receipt.retained_invocation_admission_policy_id(),
                    receipt.invocation_admission_id(),
                    &basis_state,
                    policy_id,
                )
                .ok_or(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                    detail: "committed Action preparation cannot be reconstructed",
                })?;
                if prepared.package_id() != receipt.package_id()
                    || prepared.installed_operation_id() != receipt.installed_operation_id()
                    || prepared.invocation_id() != receipt.invocation_id()
                    || prepared.evaluation_basis().identity() != receipt.evaluation_basis_id()
                    || prepared.private_evaluation_id() != receipt.private_evaluation_id()
                    || prepared.prepared_patch_digest() != receipt.prepared_patch_digest()
                    || prepared.result_id() != receipt.prepared_result_id()
                    || prepared.actual_footprint_digest() != receipt.actual_footprint_digest()
                    || prepared.preparation_id() != receipt.preparation_id()
                {
                    return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                        detail:
                            "committed Action evidence disagrees with reconstructed preparation",
                    });
                }
                Some(prepared)
            }
            EchoOperationActionOutcomeV1::RejectedFootprintConflict(conflict) => {
                let prepared = reconstruct_action_preparation_v1(
                    installed,
                    invocation_bytes,
                    conflict.invocation_admission_maximum_budget,
                    conflict.invocation_admission_policy_id,
                    conflict.invocation_admission_id,
                    &basis_state,
                    policy_id,
                )
                .ok_or(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                    detail: "conflicting Action preparation cannot be reconstructed",
                })?;
                if prepared.package_id() != conflict.package_id
                    || prepared.installed_operation_id() != conflict.installed_operation_id
                    || prepared.invocation_id() != conflict.invocation_id
                    || prepared.evaluation_basis().identity() != conflict.evaluation_basis_id
                    || prepared.private_evaluation_id() != conflict.private_evaluation_id
                    || prepared.prepared_patch_digest() != conflict.prepared_patch_digest
                    || prepared.result_id() != conflict.prepared_result_id
                    || prepared.actual_footprint_digest() != conflict.actual_footprint_digest
                    || prepared.preparation_id() != conflict.preparation_id
                {
                    return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                        detail: "conflict evidence disagrees with reconstructed preparation",
                    });
                }
                Some(prepared)
            }
            EchoOperationActionOutcomeV1::Obstructed(_) => None,
        };
        if let Some(prepared) = reconstructed {
            reconstructed_preparations.insert(*submission_id, prepared);
        }
        action_tick_members
            .entry((
                correlation.head_key,
                correlation.worldline_tick_after,
                correlation.commit_global_tick,
                correlation.commit_hash,
            ))
            .or_default()
            .push((*ingress_id, *submission_id));
    }
    for (_, mut members) in action_tick_members {
        members.sort_by_key(|(ingress_id, _)| *ingress_id);
        for (index, (_, submission_id)) in members.iter().enumerate() {
            let Some(EchoOperationActionOutcomeV1::RejectedFootprintConflict(conflict)) =
                outcomes.get(submission_id)
            else {
                continue;
            };
            let candidate = reconstructed_preparations.get(submission_id).ok_or(
                TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                    detail: "conflicting Action has no reconstructed preparation",
                },
            )?;
            let mut expected_blockers = Vec::new();
            for (earlier_index, (_, earlier_submission_id)) in members[..index].iter().enumerate() {
                if !matches!(
                    outcomes.get(earlier_submission_id),
                    Some(EchoOperationActionOutcomeV1::Committed(_))
                ) {
                    continue;
                }
                let earlier = reconstructed_preparations
                    .get(earlier_submission_id)
                    .ok_or(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                        detail: "committed Action has no reconstructed preparation",
                    })?;
                if crate::engine_impl::footprints_conflict(
                    candidate.actual_footprint(),
                    earlier.actual_footprint(),
                ) {
                    expected_blockers.push(u32::try_from(earlier_index).map_err(|_| {
                        TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                            detail: "Action Tick has too many members to index",
                        }
                    })?);
                }
            }
            if expected_blockers != conflict.blocked_by {
                return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                    detail: "conflicting Action blockers disagree with reconstructed footprints",
                });
            }
        }
    }
    Ok(())
}

fn evaluation_basis_matches_recovered_coordinate(
    basis: EchoOperationEvaluationBasisV1,
    entries: &BTreeMap<(crate::WorldlineId, crate::WorldlineTick), &ProvenanceEntry>,
) -> bool {
    if basis.worldline_tick() == crate::WorldlineTick::ZERO {
        return basis.commit_global_tick().is_none()
            && basis.commit_id()
                == crate::echo_operation::genesis_commit_id(
                    basis.writer_head(),
                    basis.state_root(),
                );
    }
    let Some(parent_tick) = basis.worldline_tick().as_u64().checked_sub(1) else {
        return false;
    };
    entries
        .get(&(
            basis.writer_head().worldline_id,
            crate::WorldlineTick::from_raw(parent_tick),
        ))
        .is_some_and(|parent| {
            parent.head_key == Some(basis.writer_head())
                && parent.expected.state_root == basis.state_root()
                && parent.expected.commit_hash == basis.commit_id()
                && basis.commit_global_tick() == Some(parent.commit_global_tick)
        })
}

fn validate_operation_receipt_state_delta(
    receipt: &EchoOperationReceiptV1,
    state_delta: &WalRuntimeStateDeltaRecord,
    installed: &InstalledEchoOperationV1,
) -> Result<(), TrustedRuntimeWalError> {
    let entry = state_delta.provenance_entry();
    let basis = receipt.evaluation_basis();
    let expected_rule_id = receipt.installed_operation_id().as_hash();
    let patch_binds_installation = entry.patch.as_ref().is_some_and(|patch| {
        patch.rule_pack_id() == expected_rule_id
            && receipt.committed_patch_digest() == Some(patch.patch_digest)
    });
    let tick_binds_installation = entry
        .patch
        .as_ref()
        .zip(entry.tick_receipt.as_ref())
        .is_some_and(|(patch, tick_receipt)| {
            operation_tick_binds_patch_v1(
                tick_receipt,
                patch,
                expected_rule_id,
                installed.program(),
            )
        });
    let worldline_tick_after = entry
        .worldline_tick
        .checked_add(1)
        .ok_or(RetainedProvenanceError::Inconsistent("worldline tick"))?;
    if state_delta.contract().is_some()
        || entry.event_kind != crate::ProvenanceEventKind::LocalCommit
        || entry.worldline_id != basis.writer_head().worldline_id
        || entry.head_key != Some(basis.writer_head())
        || entry.worldline_tick != basis.worldline_tick()
        || receipt.tick_receipt_digest() != state_delta.receipt_digest()
        || entry
            .tick_receipt
            .as_ref()
            .is_none_or(|tick_receipt| tick_receipt.digest() != state_delta.receipt_digest())
        || !patch_binds_installation
        || !tick_binds_installation
        || receipt.commit_id() != entry.expected.commit_hash
        || receipt.state_root_after() != entry.expected.state_root
        || receipt.committed_patch_digest() != Some(entry.expected.patch_digest)
        || receipt.commit_global_tick() != Some(entry.commit_global_tick)
        || receipt.worldline_tick_after() != worldline_tick_after
    {
        return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
            detail: "typed operation receipt disagrees with replayable provenance",
        });
    }
    let parent_matches_basis = if basis.worldline_tick() == crate::WorldlineTick::ZERO {
        entry.parents.is_empty()
            && basis.commit_global_tick().is_none()
            && basis.commit_id()
                == crate::echo_operation::genesis_commit_id(basis.writer_head(), basis.state_root())
    } else {
        let Some(parent_tick) = basis.worldline_tick().as_u64().checked_sub(1) else {
            return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
                detail: "operation receipt has an impossible non-genesis basis tick",
            });
        };
        matches!(
            entry.parents.as_slice(),
            [parent]
                if parent.worldline_id == basis.writer_head().worldline_id
                    && parent.worldline_tick == crate::WorldlineTick::from_raw(parent_tick)
                    && parent.commit_hash == basis.commit_id()
        )
    };
    if !parent_matches_basis {
        return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
            detail: "typed operation receipt basis disagrees with provenance parents",
        });
    }
    Ok(())
}

fn validate_operation_receipt_parent_material(
    basis: EchoOperationEvaluationBasisV1,
    entry: &ProvenanceEntry,
    provenance_by_coordinate: &BTreeMap<
        (crate::WorldlineId, crate::WorldlineTick),
        &ProvenanceEntry,
    >,
) -> Result<(), TrustedRuntimeWalError> {
    if basis.worldline_tick() == crate::WorldlineTick::ZERO {
        return Ok(());
    }
    let parent_tick = basis.worldline_tick().as_u64().checked_sub(1).ok_or(
        TrustedRuntimeWalError::EchoOperationExecutionMismatch {
            detail: "operation receipt has an impossible non-genesis basis tick",
        },
    )?;
    let parent_coordinate = (
        basis.writer_head().worldline_id,
        crate::WorldlineTick::from_raw(parent_tick),
    );
    let parent = provenance_by_coordinate.get(&parent_coordinate).ok_or(
        TrustedRuntimeWalError::EchoOperationExecutionMismatch {
            detail: "operation receipt basis has no retained parent provenance",
        },
    )?;
    let parent_matches_basis = entry.parents.as_slice() == [parent.as_ref()]
        && parent.expected.state_root == basis.state_root()
        && parent.expected.commit_hash == basis.commit_id()
        && basis.commit_global_tick() == Some(parent.commit_global_tick);
    if !parent_matches_basis {
        return Err(TrustedRuntimeWalError::EchoOperationExecutionMismatch {
            detail: "operation receipt basis disagrees with retained parent provenance",
        });
    }
    Ok(())
}

struct RecoveredRuntimeStateMaterial {
    provenance_entries: Vec<ProvenanceEntry>,
    receipt_correlations: Vec<ReceiptCorrelationPersistenceRecord>,
    echo_operation_action_outcomes: Vec<(Hash, Hash, EchoOperationActionOutcomeV1)>,
    echo_operation_action_decisions: BTreeMap<Hash, WalTickDecision>,
    echo_operation_action_installations_before_tick:
        BTreeMap<Hash, BTreeSet<crate::EchoOperationPackageIdV1>>,
    missing_runtime_state_deltas: Vec<Hash>,
}

fn recover_runtime_state_delta_material(
    report: &RecoveryScanReport,
) -> Result<RecoveredRuntimeStateMaterial, TrustedRuntimeWalError> {
    let mut entries_by_coordinate = BTreeMap::new();
    let mut correlations_by_submission = BTreeMap::new();
    let mut action_outcomes_by_submission = BTreeMap::new();
    let mut action_decisions_by_submission = BTreeMap::new();
    let mut action_installations_before_tick = BTreeMap::new();
    let mut installed_packages = BTreeSet::new();
    let mut submission_by_ticket = BTreeMap::new();
    let mut missing = Vec::new();
    for transaction in &report.transactions {
        if transaction.commit.transaction_kind
            == WalTransactionKind::ExecutableOperationInstallation
        {
            let (installed, _) = operation_installation_from_transaction(transaction)?;
            installed_packages.insert(installed.package_id());
            continue;
        }
        if transaction.commit.transaction_kind == WalTransactionKind::ExecutableOperationTick {
            let (_, state_delta, _) = operation_tick_records_from_transaction(transaction)?;
            let entry = state_delta.provenance_entry().clone();
            let coordinate = (entry.worldline_id, entry.worldline_tick);
            if entries_by_coordinate
                .get(&coordinate)
                .is_some_and(|existing| existing != &entry)
            {
                return Err(TrustedRuntimeWalError::RuntimeStateDeltaConflict {
                    worldline_id: entry.worldline_id,
                    worldline_tick: entry.worldline_tick,
                });
            }
            entries_by_coordinate.insert(coordinate, entry);
            continue;
        }
        if transaction.commit.transaction_kind != WalTransactionKind::SchedulerTick {
            continue;
        }
        let (records, _, _) = tick_record_batch_from_transaction(transaction)?;
        let state_delta_frame = required_unique_transaction_frame(
            transaction,
            WalRecordKind::RuntimeStateDeltaRecorded,
        )?;
        if state_delta_frame.payload.canonical_bytes.len() == core::mem::size_of::<Hash>() {
            missing.extend(
                records
                    .iter()
                    .map(|(receipt, _, _)| receipt.receipt_ref.identity_digest()),
            );
            continue;
        }
        let state_delta = WalRuntimeStateDeltaRecord::from_payload_bytes(
            &state_delta_frame.payload.canonical_bytes,
        )?;
        if records.iter().any(|(receipt, _, _)| {
            state_delta.receipt_digest() != receipt.receipt_ref.receipt_content_digest
        }) {
            return Err(RetainedProvenanceError::Inconsistent("state-delta receipt").into());
        }
        let entry = state_delta.provenance_entry().clone();
        let head_key = entry
            .head_key
            .ok_or(RetainedProvenanceError::MissingHeadKey)?;
        for (receipt, wal_correlation, action_outcome) in records {
            let expected_receipt_ref = crate::CausalTickReceiptRef {
                worldline_id: entry.worldline_id,
                worldline_tick_after: entry
                    .worldline_tick
                    .checked_add(1)
                    .ok_or(RetainedProvenanceError::Inconsistent("worldline tick"))?,
                commit_global_tick: entry.commit_global_tick,
                commit_hash: entry.expected.commit_hash,
                submission_id: receipt.receipt_ref.submission_id,
                ticket_digest: receipt.receipt_ref.ticket_digest,
                receipt_content_digest: state_delta.receipt_digest(),
            };
            if receipt.receipt_ref != expected_receipt_ref {
                return Err(RetainedProvenanceError::Inconsistent("causal receipt ref").into());
            }
            let persistence = ReceiptCorrelationPersistenceRecord {
                submission_id: receipt.receipt_ref.submission_id,
                ticket_digest: receipt.receipt_ref.ticket_digest,
                causal_receipt_ref: receipt.receipt_ref,
                head_key,
                commit_global_tick: entry.commit_global_tick,
                worldline_tick_after: receipt.receipt_ref.worldline_tick_after,
                tick_receipt_digest: receipt.receipt_ref.receipt_content_digest,
                commit_hash: entry.expected.commit_hash,
                contract: state_delta.contract().cloned(),
                causal_parent_receipts: wal_correlation.causal_parent_receipts,
            };
            if correlations_by_submission
                .get(&receipt.receipt_ref.submission_id)
                .is_some_and(|existing| existing != &persistence)
            {
                return Err(TrustedRuntimeWalError::RuntimeStateDeltaConflict {
                    worldline_id: entry.worldline_id,
                    worldline_tick: entry.worldline_tick,
                });
            }
            if submission_by_ticket
                .insert(
                    receipt.receipt_ref.ticket_digest,
                    receipt.receipt_ref.submission_id,
                )
                .is_some_and(|existing| existing != receipt.receipt_ref.submission_id)
            {
                return Err(TrustedRuntimeWalError::RuntimeStateDeltaConflict {
                    worldline_id: entry.worldline_id,
                    worldline_tick: entry.worldline_tick,
                });
            }
            correlations_by_submission.insert(receipt.receipt_ref.submission_id, persistence);
            if let Some((submission_id, ingress_id, outcome)) = action_outcome {
                if submission_id != receipt.receipt_ref.submission_id
                    || action_outcomes_by_submission
                        .get(&submission_id)
                        .is_some_and(|existing| existing != &(ingress_id, outcome.clone()))
                    || action_decisions_by_submission
                        .insert(submission_id, receipt.decision)
                        .is_some_and(|existing| existing != receipt.decision)
                {
                    return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
                }
                action_outcomes_by_submission.insert(submission_id, (ingress_id, outcome));
                action_installations_before_tick.insert(submission_id, installed_packages.clone());
            }
        }
        let coordinate = (entry.worldline_id, entry.worldline_tick);
        if entries_by_coordinate
            .get(&coordinate)
            .is_some_and(|existing| existing != &entry)
        {
            return Err(TrustedRuntimeWalError::RuntimeStateDeltaConflict {
                worldline_id: entry.worldline_id,
                worldline_tick: entry.worldline_tick,
            });
        }
        entries_by_coordinate.insert(coordinate, entry);
    }
    let mut entries = entries_by_coordinate.into_values().collect::<Vec<_>>();
    entries.sort_by_key(|entry| {
        (
            entry.commit_global_tick,
            entry.worldline_id,
            entry.worldline_tick,
        )
    });
    let mut correlations = correlations_by_submission.into_values().collect::<Vec<_>>();
    correlations.sort_by_key(|correlation| {
        (
            correlation.commit_global_tick,
            correlation.head_key,
            correlation.worldline_tick_after,
            correlation.submission_id,
        )
    });
    missing.sort_unstable();
    missing.dedup();
    let echo_operation_action_outcomes = action_outcomes_by_submission
        .into_iter()
        .map(|(submission_id, (ingress_id, outcome))| (submission_id, ingress_id, outcome))
        .collect();
    Ok(RecoveredRuntimeStateMaterial {
        provenance_entries: entries,
        receipt_correlations: correlations,
        echo_operation_action_outcomes,
        echo_operation_action_decisions: action_decisions_by_submission,
        echo_operation_action_installations_before_tick: action_installations_before_tick,
        missing_runtime_state_deltas: missing,
    })
}

fn validate_recovered_causal_parent_evidence(
    witnessed_submissions: &WitnessedSubmissionPersistenceSnapshot,
    receipt_correlations: &[ReceiptCorrelationPersistenceRecord],
) -> Result<(), TrustedRuntimeWalError> {
    let envelopes_by_submission = witnessed_submissions
        .records()
        .iter()
        .map(|record| (record.submission.submission_id, &record.envelope))
        .collect::<BTreeMap<_, _>>();
    for correlation in receipt_correlations {
        let Some(envelope) = envelopes_by_submission.get(&correlation.submission_id) else {
            continue;
        };
        if correlation.causal_parent_receipts != envelope.canonical_causal_parent_receipt_refs() {
            return Err(
                TrustedRuntimeWalError::ReceiptCorrelationCausalParentsMismatch {
                    submission_id: correlation.submission_id,
                    receipt_ref_digest: correlation.causal_receipt_ref.identity_digest(),
                },
            );
        }
    }
    Ok(())
}

fn validate_recovered_echo_operation_action_outcomes(
    witnessed_submissions: &WitnessedSubmissionPersistenceSnapshot,
    receipt_correlations: &[ReceiptCorrelationPersistenceRecord],
    provenance_entries: &[ProvenanceEntry],
    installed_echo_operations: &[InstalledEchoOperationV1],
    outcomes: &[(Hash, Hash, EchoOperationActionOutcomeV1)],
    decisions: &BTreeMap<Hash, WalTickDecision>,
    installations_before_tick: &BTreeMap<Hash, BTreeSet<crate::EchoOperationPackageIdV1>>,
) -> Result<(), TrustedRuntimeWalError> {
    let envelopes = witnessed_submissions
        .records()
        .iter()
        .map(|record| (record.submission.submission_id, &record.envelope))
        .collect::<BTreeMap<_, _>>();
    let submissions = witnessed_submissions
        .records()
        .iter()
        .map(|record| (record.submission.submission_id, &record.submission))
        .collect::<BTreeMap<_, _>>();
    let correlations = receipt_correlations
        .iter()
        .map(|correlation| (correlation.submission_id, correlation))
        .collect::<BTreeMap<_, _>>();
    let provenance = provenance_entries
        .iter()
        .filter_map(|entry| {
            Some((
                (
                    entry.head_key?,
                    entry.worldline_tick.checked_add(1)?,
                    entry.commit_global_tick,
                    entry.expected.commit_hash,
                ),
                entry,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut outcomes_by_tick = BTreeMap::<
        (
            crate::WriterHeadKey,
            crate::WorldlineTick,
            crate::GlobalTick,
            Hash,
        ),
        Vec<(
            Hash,
            &EchoOperationActionOutcomeV1,
            &ReceiptCorrelationPersistenceRecord,
            crate::echo_operation::EchoOperationActionInvocationEvidenceV1,
        )>,
    >::new();
    for (submission_id, ingress_id, outcome) in outcomes {
        if decisions.get(submission_id) != Some(&wal_tick_decision_for_action_outcome(outcome)) {
            return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
        }
        let envelope = envelopes
            .get(submission_id)
            .ok_or(TrustedRuntimeWalError::SchedulerTickBatchMismatch)?;
        let submission = submissions
            .get(submission_id)
            .ok_or(TrustedRuntimeWalError::SchedulerTickBatchMismatch)?;
        let correlation = correlations
            .get(submission_id)
            .ok_or(TrustedRuntimeWalError::SchedulerTickBatchMismatch)?;
        let invocation_bytes = echo_operation_action_invocation_bytes_v1(envelope)
            .ok_or(TrustedRuntimeWalError::SchedulerTickBatchMismatch)?;
        let invocation = inspect_action_invocation_v1(invocation_bytes)
            .map_err(|_| TrustedRuntimeWalError::SchedulerTickBatchMismatch)?;
        let installed = installed_echo_operations
            .iter()
            .find(|installed| installed.package_id() == invocation.package_id)
            .ok_or(TrustedRuntimeWalError::SchedulerTickBatchMismatch)?;
        if installations_before_tick
            .get(submission_id)
            .is_none_or(|packages| !packages.contains(&invocation.package_id))
        {
            return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
        }
        if envelope.ingress_id() != *ingress_id {
            return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
        }
        let invocation_admission_id = match outcome {
            EchoOperationActionOutcomeV1::Committed(receipt) => {
                Some(receipt.invocation_admission_id())
            }
            EchoOperationActionOutcomeV1::Obstructed(obstruction) => {
                Some(obstruction.invocation_admission_id())
            }
            EchoOperationActionOutcomeV1::RejectedFootprintConflict(conflict) => {
                Some(conflict.invocation_admission_id)
            }
        };
        if correlation.ticket_digest != correlation.causal_receipt_ref.ticket_digest
            || invocation_admission_id.is_some_and(|admission_id| {
                correlation.ticket_digest
                    != echo_operation_action_admission_digest_from_parts(
                        submission,
                        invocation.package_id,
                        installed.installed_operation_id(),
                        admission_id,
                    )
            })
        {
            return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
        }
        match outcome {
            EchoOperationActionOutcomeV1::Committed(receipt) => {
                if validate_receipt_installation_v1(receipt, installed).is_err()
                    || receipt.tick_receipt_digest() != correlation.tick_receipt_digest
                    || receipt.commit_id() != correlation.commit_hash
                    || receipt.commit_global_tick() != Some(correlation.commit_global_tick)
                    || receipt.worldline_tick_after() != correlation.worldline_tick_after
                    || receipt.evaluation_basis().writer_head() != correlation.head_key
                    || receipt.package_id() != invocation.package_id
                    || receipt.invocation_id() != invocation.invocation_id
                    || receipt.invocation_bytes_digest() != invocation.invocation_bytes_digest
                    || receipt.evaluation_basis() != invocation.evaluation_basis
                    || receipt.terminal_posture() != EchoOperationTerminalPostureV1::Committed
                {
                    return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
                }
            }
            EchoOperationActionOutcomeV1::Obstructed(obstruction) => {
                if obstruction.installed_operation_id() != installed.installed_operation_id()
                    || obstruction.package_id() != invocation.package_id
                    || obstruction.invocation_id() != invocation.invocation_id
                    || obstruction.evaluation_basis_id() != invocation.evaluation_basis.identity()
                {
                    return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
                }
            }
            EchoOperationActionOutcomeV1::RejectedFootprintConflict(conflict) => {
                if conflict.package_id != invocation.package_id
                    || conflict.installed_operation_id != installed.installed_operation_id()
                    || !action_admission_evidence_matches_v1(
                        installed,
                        invocation_bytes,
                        conflict.invocation_admission_maximum_budget,
                        conflict.invocation_admission_policy_id,
                        conflict.invocation_admission_id,
                    )
                    || conflict.evaluation_basis_id != invocation.evaluation_basis.identity()
                    || !action_preparation_identity_matches_v1(
                        conflict.private_evaluation_id,
                        conflict.prepared_patch_digest,
                        conflict.prepared_result_id,
                        conflict.preparation_id,
                    )
                    || conflict.blocked_by.is_empty()
                    || conflict
                        .blocked_by
                        .windows(2)
                        .any(|pair| pair[0] >= pair[1])
                    || conflict.invocation_id != invocation.invocation_id
                {
                    return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
                }
            }
        }
        outcomes_by_tick
            .entry((
                correlation.head_key,
                correlation.worldline_tick_after,
                correlation.commit_global_tick,
                correlation.commit_hash,
            ))
            .or_default()
            .push((*ingress_id, outcome, correlation, invocation));
    }
    for (tick_coordinate, mut group) in outcomes_by_tick {
        let entry = provenance
            .get(&tick_coordinate)
            .ok_or(TrustedRuntimeWalError::SchedulerTickBatchMismatch)?;
        let tick_receipt = entry
            .tick_receipt
            .as_ref()
            .ok_or(TrustedRuntimeWalError::SchedulerTickBatchMismatch)?;
        group.sort_by_key(|(ingress_id, _, _, _)| *ingress_id);
        if group.len() != tick_receipt.entries().len() {
            return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
        }
        let committed_receipts = group
            .iter()
            .filter_map(|(_, outcome, _, _)| match outcome {
                EchoOperationActionOutcomeV1::Committed(receipt) => Some(receipt.as_ref()),
                EchoOperationActionOutcomeV1::Obstructed(_)
                | EchoOperationActionOutcomeV1::RejectedFootprintConflict(_) => None,
            })
            .collect::<Vec<_>>();
        if !committed_receipts.is_empty() {
            let expected_composition_digest = action_batch_composition_digest_from_receipts_v1(
                &committed_receipts,
                entry.expected.patch_digest,
            );
            if committed_receipts
                .iter()
                .any(|receipt| receipt.composition_digest() != Some(expected_composition_digest))
            {
                return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
            }
        }
        for (index, (_, outcome, correlation, invocation)) in group.into_iter().enumerate() {
            if entry.commit_global_tick != correlation.commit_global_tick
                || entry.head_key != Some(correlation.head_key)
                || entry.worldline_id != correlation.head_key.worldline_id
                || tick_receipt.digest() != correlation.tick_receipt_digest
            {
                return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
            }
            let tick_entry = tick_receipt
                .entries()
                .get(index)
                .ok_or(TrustedRuntimeWalError::SchedulerTickBatchMismatch)?;
            let blockers = tick_receipt.blocked_by(index);
            if tick_entry.scope != invocation.scope
                || tick_entry.scope_hash
                    != crate::scope_hash(&tick_entry.rule_id, &tick_entry.scope)
            {
                return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
            }
            match outcome {
                EchoOperationActionOutcomeV1::Committed(receipt) => {
                    if tick_entry.disposition != TickReceiptDisposition::Applied
                        || !blockers.is_empty()
                        || tick_entry.rule_id != receipt.installed_operation_id().as_hash()
                        || receipt.state_root_after() != entry.expected.state_root
                        || receipt.committed_patch_digest() != Some(entry.expected.patch_digest)
                    {
                        return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
                    }
                }
                EchoOperationActionOutcomeV1::Obstructed(obstruction) => {
                    if tick_entry.disposition
                        != TickReceiptDisposition::Rejected(
                            TickReceiptRejection::ExecutableOperationObstruction,
                        )
                        || !blockers.is_empty()
                        || tick_entry.rule_id != obstruction.installed_operation_id().as_hash()
                    {
                        return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
                    }
                }
                EchoOperationActionOutcomeV1::RejectedFootprintConflict(conflict) => {
                    if tick_entry.disposition
                        != TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict)
                        || tick_entry.rule_id != conflict.installed_operation_id.as_hash()
                        || blockers != conflict.blocked_by
                    {
                        return Err(TrustedRuntimeWalError::SchedulerTickBatchMismatch);
                    }
                }
            }
        }
    }
    Ok(())
}

fn wal_tick_decision_for_action_outcome(outcome: &EchoOperationActionOutcomeV1) -> WalTickDecision {
    match outcome {
        EchoOperationActionOutcomeV1::Committed(_) => WalTickDecision::Applied,
        EchoOperationActionOutcomeV1::Obstructed(_) => WalTickDecision::Obstructed,
        EchoOperationActionOutcomeV1::RejectedFootprintConflict(_) => {
            WalTickDecision::RejectedFootprintConflict
        }
    }
}

fn operation_installation_from_transaction(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
) -> Result<(InstalledEchoOperationV1, Vec<u8>), TrustedRuntimeWalError> {
    let frame = required_unique_transaction_frame(
        transaction,
        WalRecordKind::ExecutableOperationPackageInstalled,
    )?;
    let retained_bytes = frame.payload.canonical_bytes.clone();
    let installed = recover_installation_v1(&retained_bytes)?;
    Ok((installed, retained_bytes))
}

fn validate_echo_operation_frontier_root(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
    expected_frontiers: &[AffectedFrontier],
) -> Result<(), TrustedRuntimeWalError> {
    let expected = affected_frontiers_root(expected_frontiers);
    let actual = transaction.commit.affected_frontiers_root;
    if actual != expected {
        return Err(TrustedRuntimeWalError::EchoOperationFrontierMismatch {
            transaction_id: transaction.commit.transaction_id.as_hash(),
            expected,
            actual,
        });
    }
    Ok(())
}

fn operation_tick_records_from_transaction(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
) -> Result<(EchoOperationReceiptV1, WalRuntimeStateDeltaRecord, Hash), TrustedRuntimeWalError> {
    let receipt_frame = required_unique_transaction_frame(
        transaction,
        WalRecordKind::ExecutableOperationExecutionRecorded,
    )?;
    let receipt = recover_committed_execution_receipt_v1(&receipt_frame.payload.canonical_bytes)?;
    let state_delta_frame = required_unique_transaction_frame(
        transaction,
        WalRecordKind::ExecutableOperationStateDeltaRecorded,
    )?;
    let state_delta =
        WalRuntimeStateDeltaRecord::from_payload_bytes(&state_delta_frame.payload.canonical_bytes)?;
    let state_delta_digest = state_delta.digest()?;
    Ok((receipt, state_delta, state_delta_digest))
}

#[cfg(test)]
fn tick_records_from_transaction(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
) -> Result<
    (
        TickReceiptRecord,
        WalReceiptCorrelationRecord,
        Hash,
        Option<ProvenanceEntry>,
    ),
    TrustedRuntimeWalError,
> {
    let (mut records, state_delta_digest, provenance_entry) =
        tick_record_batch_from_transaction(transaction)?;
    if records.len() != 1 {
        return Err(decode_trusted_runtime_wal_payload(
            WalDecodeError::InvalidEmbeddedFrame,
        ));
    }
    let (receipt, correlation, action_outcome) = records.remove(0);
    if action_outcome.is_some() {
        return Err(decode_trusted_runtime_wal_payload(
            WalDecodeError::InvalidEmbeddedFrame,
        ));
    }
    Ok((receipt, correlation, state_delta_digest, provenance_entry))
}

type RecoveredSchedulerTickMember = (
    TickReceiptRecord,
    WalReceiptCorrelationRecord,
    Option<(Hash, Hash, EchoOperationActionOutcomeV1)>,
);
type RecoveredSchedulerTickBatch = (
    Vec<RecoveredSchedulerTickMember>,
    Hash,
    Option<ProvenanceEntry>,
);

fn tick_record_batch_from_transaction(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
) -> Result<RecoveredSchedulerTickBatch, TrustedRuntimeWalError> {
    let state_delta_index = transaction
        .frames
        .iter()
        .position(|frame| frame.header.record_kind == WalRecordKind::RuntimeStateDeltaRecorded)
        .ok_or_else(missing_trusted_runtime_record)?;
    if state_delta_index == 0 || state_delta_index + 1 != transaction.frames.len() {
        return Err(decode_trusted_runtime_wal_payload(
            WalDecodeError::InvalidEmbeddedFrame,
        ));
    }
    let mut records = Vec::new();
    let first_frame = &transaction.frames[0];
    if first_frame.header.record_kind != WalRecordKind::TickReceiptRecorded {
        return Err(decode_trusted_runtime_wal_payload(
            WalDecodeError::InvalidEmbeddedFrame,
        ));
    }
    if tick_receipt_payload_is_batch(&first_frame.payload.canonical_bytes) {
        let receipts = decode_tick_receipt_records(&first_frame.payload.canonical_bytes)
            .map_err(decode_trusted_runtime_wal_payload)?;
        let mut index = 1;
        for receipt in receipts {
            let Some(correlation_frame) = transaction.frames.get(index) else {
                return Err(decode_trusted_runtime_wal_payload(
                    WalDecodeError::InvalidEmbeddedFrame,
                ));
            };
            let Some(outcome_frame) = transaction.frames.get(index + 1) else {
                return Err(decode_trusted_runtime_wal_payload(
                    WalDecodeError::InvalidEmbeddedFrame,
                ));
            };
            if correlation_frame.header.record_kind != WalRecordKind::ReceiptCorrelationRecorded
                || outcome_frame.header.record_kind
                    != WalRecordKind::ExecutableOperationActionOutcomeRecorded
            {
                return Err(decode_trusted_runtime_wal_payload(
                    WalDecodeError::InvalidEmbeddedFrame,
                ));
            }
            let correlation = WalReceiptCorrelationRecord::from_payload_bytes(
                &correlation_frame.payload.canonical_bytes,
            )
            .map_err(decode_trusted_runtime_wal_payload)?;
            let action_outcome = recover_action_outcome_v1(&outcome_frame.payload.canonical_bytes)?;
            if correlation.receipt_ref != receipt.receipt_ref
                || action_outcome.0 != receipt.receipt_ref.submission_id
            {
                return Err(decode_trusted_runtime_wal_payload(
                    WalDecodeError::InvalidEmbeddedFrame,
                ));
            }
            records.push((receipt, correlation, Some(action_outcome)));
            index += 2;
        }
        if index != state_delta_index {
            return Err(decode_trusted_runtime_wal_payload(
                WalDecodeError::InvalidEmbeddedFrame,
            ));
        }
    } else {
        let mut index = 0;
        while index < state_delta_index {
            let Some(receipt_frame) = transaction.frames.get(index) else {
                return Err(decode_trusted_runtime_wal_payload(
                    WalDecodeError::InvalidEmbeddedFrame,
                ));
            };
            let Some(correlation_frame) = transaction.frames.get(index + 1) else {
                return Err(decode_trusted_runtime_wal_payload(
                    WalDecodeError::InvalidEmbeddedFrame,
                ));
            };
            if receipt_frame.header.record_kind != WalRecordKind::TickReceiptRecorded
                || correlation_frame.header.record_kind != WalRecordKind::ReceiptCorrelationRecorded
            {
                return Err(decode_trusted_runtime_wal_payload(
                    WalDecodeError::InvalidEmbeddedFrame,
                ));
            }
            let receipt =
                TickReceiptRecord::from_payload_bytes(&receipt_frame.payload.canonical_bytes)
                    .map_err(decode_trusted_runtime_wal_payload)?;
            let correlation = WalReceiptCorrelationRecord::from_payload_bytes(
                &correlation_frame.payload.canonical_bytes,
            )
            .map_err(decode_trusted_runtime_wal_payload)?;
            if correlation.receipt_ref != receipt.receipt_ref {
                return Err(decode_trusted_runtime_wal_payload(
                    WalDecodeError::InvalidEmbeddedFrame,
                ));
            }
            index += 2;
            if transaction.frames.get(index).is_some_and(|frame| {
                frame.header.record_kind == WalRecordKind::ExecutableOperationActionOutcomeRecorded
            }) {
                return Err(decode_trusted_runtime_wal_payload(
                    WalDecodeError::InvalidEmbeddedFrame,
                ));
            }
            records.push((receipt, correlation, None));
        }
    }
    let action_outcome_count = records
        .iter()
        .filter(|(_, _, action_outcome)| action_outcome.is_some())
        .count();
    if action_outcome_count != 0 && action_outcome_count != records.len() {
        return Err(decode_trusted_runtime_wal_payload(
            WalDecodeError::InvalidEmbeddedFrame,
        ));
    }
    if action_outcome_count != 0 {
        let mut previous_ingress_id = None;
        for (_, _, action_outcome) in &records {
            let (_, ingress_id, _) = action_outcome.as_ref().ok_or_else(|| {
                decode_trusted_runtime_wal_payload(WalDecodeError::InvalidEmbeddedFrame)
            })?;
            if previous_ingress_id.is_some_and(|previous| previous >= *ingress_id) {
                return Err(decode_trusted_runtime_wal_payload(
                    WalDecodeError::InvalidEmbeddedFrame,
                ));
            }
            previous_ingress_id = Some(*ingress_id);
        }
    }
    let state_delta_frame = &transaction.frames[state_delta_index];
    let (state_delta_digest, provenance_entry) = if state_delta_frame.payload.canonical_bytes.len()
        == core::mem::size_of::<Hash>()
    {
        (
            state_delta_frame
                .payload
                .canonical_bytes
                .as_slice()
                .try_into()
                .map_err(|_| decode_trusted_runtime_wal_payload(WalDecodeError::UnexpectedEof))?,
            None,
        )
    } else {
        let state_delta = WalRuntimeStateDeltaRecord::from_payload_bytes(
            &state_delta_frame.payload.canonical_bytes,
        )?;
        if records.iter().any(|(receipt, _, _)| {
            state_delta.receipt_digest() != receipt.receipt_ref.receipt_content_digest
        }) {
            return Err(RetainedProvenanceError::Inconsistent("state-delta receipt").into());
        }
        (
            state_delta.digest()?,
            Some(state_delta.provenance_entry().clone()),
        )
    };
    Ok((records, state_delta_digest, provenance_entry))
}

fn required_unique_transaction_frame(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
    record_kind: WalRecordKind,
) -> Result<&crate::causal_wal::WalFrame, TrustedRuntimeWalError> {
    unique_transaction_frame(transaction, record_kind)?.ok_or_else(missing_trusted_runtime_record)
}

fn unique_transaction_frame(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
    record_kind: WalRecordKind,
) -> Result<Option<&crate::causal_wal::WalFrame>, TrustedRuntimeWalError> {
    let mut matching = transaction
        .frames
        .iter()
        .filter(|frame| frame.header.record_kind == record_kind);
    let record = matching.next();
    if matching.next().is_some() {
        return Err(decode_trusted_runtime_wal_payload(
            WalDecodeError::InvalidEmbeddedFrame,
        ));
    }
    Ok(record)
}

fn missing_trusted_runtime_record() -> TrustedRuntimeWalError {
    decode_trusted_runtime_wal_payload(WalDecodeError::UnexpectedEof)
}

fn decode_trusted_runtime_wal_payload(error: WalDecodeError) -> TrustedRuntimeWalError {
    TrustedRuntimeWalError::Recovery(WalRecoveryError::Index(WalRecoveryIndexError::Decode(
        error,
    )))
}

fn recover_runtime_wal_store_read_only(
    store: &impl WalStorePort,
) -> Result<RecoveryScanReport, WalRecoveryError> {
    let frames = store.read_frames();
    let commits = store.read_commits();
    recover_from_frames_and_commits(&frames, &commits, RecoveryAccessMode::ReadOnly)
}

#[cfg(any(test, feature = "host_test"))]
impl TrustedRuntimeWal {
    /// Builds an in-memory WAL at a caller-supplied LSN for overflow tests.
    pub fn new_in_memory_at_lsn_for_test(next_lsn: Lsn) -> Result<Self, TrustedRuntimeWalError> {
        Self::new_in_memory_at_lsn(next_lsn)
    }

    /// Appends submission acceptance frames without a transaction commit marker.
    pub fn append_uncommitted_submission_acceptance_for_test(
        &mut self,
        envelope: &IngressEnvelope,
        handle: IntentSubmissionHandle,
    ) -> Result<(), TrustedRuntimeWalError> {
        let record = SubmissionAcceptanceRecord {
            submission_id: handle.submission_id,
            canonical_envelope_digest: envelope.ingress_id(),
            idempotency_key_digest: None,
            acceptance_evidence_digest: acceptance_evidence_digest(handle),
        };
        let next_submission_frontier =
            submission_frontier_digest(self.submission_frontier_digest, record);
        let transaction = build_submission_acceptance_with_material_transaction(
            self.builder(
                WalTransactionKind::SubmissionIntake,
                WalAppendAuthority::SubmissionIntake,
                WalTransactionId::from_hash(submission_transaction_digest(handle, record)),
            ),
            record,
            submission_envelope_record(envelope, handle),
            vec![AffectedFrontier {
                kind: AffectedFrontierKind::SubmissionQueue,
                before_digest: self.submission_frontier_digest,
                after_digest: next_submission_frontier,
            }],
        )?;
        let last_frame_digest = transaction.frames.last().map_or(
            self.previous_frame_digest,
            crate::causal_wal::WalFrame::digest,
        );
        let next_lsn = transaction
            .commit
            .last_lsn
            .checked_next()
            .ok_or(WalBuildError::LsnOverflow)?;
        for frame in transaction.frames {
            self.store
                .append_uncommitted_frame(self.writer_epoch, frame)?;
        }
        self.next_lsn = next_lsn;
        self.previous_frame_digest = last_frame_digest;
        Ok(())
    }

    /// Forces the next live evidence catalog update to fail after WAL commit.
    pub fn fail_next_evidence_catalog_update_for_test(&mut self) {
        self.fail_next_evidence_catalog_update = true;
    }
}

/// App-facing handle for a trusted local runtime host.
///
/// This type intentionally exposes no scheduler control, package registration,
/// ticketed ingress staging, or fault recovery authority.
pub struct TrustedRuntimeApp<'a> {
    host: &'a mut TrustedRuntimeHost,
}

#[derive(Clone, Copy)]
enum AppIntentAdmission {
    Ordinary,
    ContractInverse,
}

impl TrustedRuntimeApp<'_> {
    /// Returns the current logical causal frontier derived from durable runtime history.
    ///
    /// Applications place this explicit basis in a causal-anchor admission request.
    ///
    /// # Errors
    ///
    /// Returns [`TrustedRuntimeHostError::RuntimeWalUnavailable`] when the trusted
    /// host has no durable runtime WAL authority configured.
    pub fn current_causal_anchor_basis(
        &self,
    ) -> Result<CausalFrontierRef, TrustedRuntimeHostError> {
        let runtime_wal = self
            .host
            .runtime_wal
            .as_ref()
            .ok_or(TrustedRuntimeHostError::RuntimeWalUnavailable)?;
        Ok(runtime_wal.current_causal_anchor_basis())
    }

    /// Requests Echo-owned admission of one application causal-anchor claim.
    ///
    /// The trusted host canonicalizes the claim, requires its exact current
    /// durable basis, applies the host-installed generic root-support policy,
    /// and returns fact/receipt evidence only after WAL commit. The application
    /// cannot supply receipt identity or install support through this handle.
    ///
    /// # Errors
    ///
    /// Returns a typed host error for missing WAL or support policy, malformed
    /// claims, stale bases, unsupported roots, and failed durable append.
    pub fn admit_causal_anchor(
        &mut self,
        request: CausalAnchorAdmissionRequest,
    ) -> Result<RecoveredCausalAnchorAdmission, TrustedRuntimeHostError> {
        if self.host.runtime_wal.is_none() {
            return Err(TrustedRuntimeHostError::RuntimeWalUnavailable);
        }
        let claim = CausalAnchorClaim::from_admission_request(request)?;
        if let Some(existing) = self
            .host
            .runtime_wal
            .as_ref()
            .ok_or(TrustedRuntimeHostError::RuntimeWalUnavailable)?
            .causal_anchor_by_claim(claim.claim_digest(), None)?
        {
            return Ok(existing);
        }
        let policy = self
            .host
            .causal_anchor_support_policy
            .clone()
            .ok_or(TrustedRuntimeHostError::CausalAnchorSupportPolicyUnavailable)?;
        let runtime_wal = self
            .host
            .runtime_wal
            .as_mut()
            .ok_or(TrustedRuntimeHostError::RuntimeWalUnavailable)?;
        let current_basis = runtime_wal.current_causal_anchor_basis();
        if claim.basis_frontier() != &current_basis {
            return Err(TrustedRuntimeHostError::CausalAnchorBasisStale {
                requested: claim.basis_frontier().frontier_digest,
                current: current_basis.frontier_digest,
            });
        }
        policy.validate_claim(&claim)?;
        let support_policy_digest = *policy.policy_digest();
        match runtime_wal.record_causal_anchor_admission(claim.clone(), support_policy_digest) {
            Ok(admission) => Ok(admission),
            Err(error) => {
                if let Some(admission) = runtime_wal.recover_filesystem_causal_anchor_after_error(
                    claim.claim_digest(),
                    &support_policy_digest,
                ) {
                    return Ok(admission);
                }
                Err(error.into())
            }
        }
    }

    /// Finds one Echo-admitted causal anchor by stable identity after recovery.
    ///
    /// # Errors
    ///
    /// Returns a typed host error when no runtime WAL is configured or recovery
    /// evidence is malformed.
    pub fn causal_anchor_by_id(
        &self,
        anchor_id: &CausalAnchorId,
    ) -> Result<Option<RecoveredCausalAnchorAdmission>, TrustedRuntimeHostError> {
        let runtime_wal = self
            .host
            .runtime_wal
            .as_ref()
            .ok_or(TrustedRuntimeHostError::RuntimeWalUnavailable)?;
        runtime_wal
            .causal_anchor_by_id(anchor_id)
            .map_err(TrustedRuntimeHostError::from)
    }

    /// Observes one Echo-admitted causal anchor at an exact historical basis.
    ///
    /// This is a bounded point reading over witnessed control history. The WAL
    /// adapter reconstructs that history, but neither its transient lookup nor
    /// the returned projection becomes a second source of authority.
    ///
    /// # Errors
    ///
    /// Returns a typed host error when no runtime WAL is configured, recovery
    /// evidence is malformed, or the requested basis is not in local history.
    pub fn causal_anchor_by_id_at_basis(
        &self,
        anchor_id: &CausalAnchorId,
        basis: &CausalFrontierRef,
    ) -> Result<Option<RecoveredCausalAnchorAdmission>, TrustedRuntimeHostError> {
        let runtime_wal = self
            .host
            .runtime_wal
            .as_ref()
            .ok_or(TrustedRuntimeHostError::RuntimeWalUnavailable)?;
        runtime_wal
            .causal_anchor_by_id_at_basis(anchor_id, basis)
            .map_err(TrustedRuntimeHostError::from)
    }

    /// Submits canonical intent material as witnessed ingress history.
    ///
    /// # Errors
    ///
    /// Returns a runtime error if the target cannot accept the submission.
    pub fn submit_intent(
        &mut self,
        envelope: IngressEnvelope,
    ) -> Result<IntentSubmissionHandle, RuntimeError> {
        let is_echo_operation_action =
            echo_operation_action_invocation_bytes_v1(&envelope).is_some();
        let handle = self.host.runtime.submit_app_intent(envelope)?;
        if self.host.runtime_wal.is_none() {
            self.host.track_pending_echo_operation_action_v1(
                handle.submission_id,
                is_echo_operation_action,
            );
        }
        Ok(handle)
    }

    /// Submits canonical intent material and returns only after the configured
    /// runtime WAL has committed the acceptance transaction.
    ///
    /// This is the ACK-boundary path for hosts that have configured a runtime
    /// WAL. It does not tick, stage ticketed ingress, register packages, or
    /// expose WAL append authority to the application.
    ///
    /// # Errors
    ///
    /// Returns an explicit host error if no WAL is configured, if runtime intake
    /// rejects the submission, or if WAL commit fails. On WAL failure, the
    /// in-memory runtime intake mutation is rolled back before the error is
    /// returned.
    pub fn submit_intent_with_runtime_wal_ack(
        &mut self,
        envelope: IngressEnvelope,
    ) -> Result<IntentSubmissionHandle, TrustedRuntimeHostError> {
        self.submit_intent_with_runtime_wal_ack_inner(envelope, AppIntentAdmission::Ordinary)
    }

    fn submit_intent_with_runtime_wal_ack_inner(
        &mut self,
        envelope: IngressEnvelope,
        admission: AppIntentAdmission,
    ) -> Result<IntentSubmissionHandle, TrustedRuntimeHostError> {
        if self.host.runtime_wal.is_none() {
            return Err(TrustedRuntimeHostError::RuntimeWalUnavailable);
        }

        let is_echo_operation_action =
            echo_operation_action_invocation_bytes_v1(&envelope).is_some();
        let before_runtime = self.host.runtime.clone();
        let handle = match admission {
            AppIntentAdmission::Ordinary => {
                self.host.runtime.submit_app_intent(envelope.clone())?
            }
            AppIntentAdmission::ContractInverse => self
                .host
                .runtime
                .submit_contract_inverse_intent(envelope.clone())?,
        };
        let Some(runtime_wal) = self.host.runtime_wal.as_mut() else {
            self.host.runtime = before_runtime;
            return Err(TrustedRuntimeHostError::RuntimeWalUnavailable);
        };
        if handle.duplicate {
            match runtime_wal.has_submission_acceptance(handle.submission_id, envelope.ingress_id())
            {
                Ok(true) => {
                    self.host.track_pending_echo_operation_action_v1(
                        handle.submission_id,
                        is_echo_operation_action,
                    );
                    return Ok(handle);
                }
                Ok(false) => {}
                Err(error) => {
                    self.host.runtime = before_runtime;
                    return Err(error.into());
                }
            }
        }
        if let Err(error) = runtime_wal.record_submission_acceptance(&envelope, handle) {
            if runtime_wal.recover_filesystem_submission_acceptance_after_error(
                handle.submission_id,
                envelope.ingress_id(),
            ) {
                self.host.track_pending_echo_operation_action_v1(
                    handle.submission_id,
                    is_echo_operation_action,
                );
                return Ok(handle);
            }
            self.host.runtime = before_runtime;
            return Err(error.into());
        }
        self.host
            .track_pending_echo_operation_action_v1(handle.submission_id, is_echo_operation_action);
        Ok(handle)
    }

    /// Resolves one installed contract inverse from durable causal evidence and
    /// submits it through the normal WAL-backed ingress boundary.
    ///
    /// The installed contract defines inverse semantics. Echo validates the
    /// target receipt, canonical submission, exact contract artifact, and
    /// current frontier before admitting the produced intent with the target
    /// receipt and current frontier receipts as causal parents. This method does
    /// not stage or tick.
    ///
    /// # Errors
    ///
    /// Returns a typed inverse obstruction when required causal evidence or the
    /// matching contract law is unavailable. Returns the usual trusted-host
    /// error when WAL-backed submission cannot be durably acknowledged.
    pub fn submit_contract_inverse_with_runtime_wal_ack(
        &mut self,
        request: ContractInverseAdmissionRequest,
    ) -> Result<IntentSubmissionHandle, TrustedRuntimeHostError> {
        if self.host.runtime_wal.is_none() {
            return Err(TrustedRuntimeHostError::RuntimeWalUnavailable);
        }
        let envelope = self.host.resolve_contract_inverse_envelope(&request)?;
        self.submit_intent_with_runtime_wal_ack_inner(envelope, AppIntentAdmission::ContractInverse)
    }

    /// Recovers the typed causal derivation for one admitted inverse receipt.
    ///
    /// Ordinary non-inverse receipts return `Ok(None)`. The returned derivation
    /// is reconstructed from retained receipt correlation and witnessed ingress
    /// material, so callers do not need process-local request maps.
    ///
    /// # Errors
    ///
    /// Returns a typed obstruction when the requested receipt, its witnessed
    /// submission, inverse target, or current-basis receipt evidence is missing
    /// or internally ambiguous.
    pub fn contract_inverse_derivation(
        &self,
        inverse_receipt_ref: &crate::CausalTickReceiptRef,
    ) -> Result<Option<ContractInverseDerivation>, ContractInverseHistoryObstruction> {
        self.host.contract_inverse_derivation(inverse_receipt_ref)
    }

    /// Observes the product-facing outcome for one witnessed submission.
    #[must_use]
    pub fn observe_intent_outcome(&self, submission_id: &Hash) -> IntentOutcome {
        self.host.runtime.observe_app_intent_outcome(submission_id)
    }

    /// Runs a read-only observation through the current installed-query path.
    ///
    /// # Errors
    ///
    /// Returns an observation error when the query is unsupported, malformed, or
    /// obstructed by the requested basis/aperture/budget.
    pub fn observe(
        &self,
        request: ObservationRequest,
    ) -> Result<ObservationArtifact, ObservationError> {
        ObservationService::observe(
            &self.host.runtime,
            &self.host.provenance,
            &self.host.engine,
            request,
        )
    }
}

fn provenance_from_runtime(
    runtime: &WorldlineRuntime,
) -> Result<ProvenanceService, TrustedRuntimeHostError> {
    let mut provenance = ProvenanceService::new();
    for (worldline_id, frontier) in runtime.worldlines().iter() {
        provenance.register_worldline(*worldline_id, frontier.state())?;
    }
    Ok(provenance)
}

fn retained_state_delta_for_correlation(
    provenance: &ProvenanceService,
    correlation: &ReceiptCorrelationRecord,
) -> Result<(WalRuntimeStateDeltaRecord, Hash), TrustedRuntimeHostError> {
    let provenance_tick = correlation
        .worldline_tick_after
        .checked_sub(1)
        .ok_or(RetainedProvenanceError::Inconsistent(
            "worldline tick after",
        ))
        .map_err(TrustedRuntimeWalError::from)?;
    let entry = provenance.entry(correlation.head_key.worldline_id, provenance_tick)?;
    if entry.head_key != Some(correlation.head_key)
        || entry.commit_global_tick != correlation.commit_global_tick
        || entry.expected.commit_hash != correlation.commit_hash
    {
        return Err(
            TrustedRuntimeWalError::from(RetainedProvenanceError::Inconsistent(
                "receipt-correlated provenance",
            ))
            .into(),
        );
    }
    let state_delta = WalRuntimeStateDeltaRecord::from_provenance_entry(
        correlation.tick_receipt_digest,
        correlation.contract.clone(),
        entry,
    )
    .map_err(TrustedRuntimeWalError::from)?;
    let digest = state_delta.digest().map_err(TrustedRuntimeWalError::from)?;
    Ok((state_delta, digest))
}

fn causal_anchor_transaction_digest(
    causal_anchor_frontier: Hash,
    claim_digest: &Hash,
    support_policy_digest: &Hash,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo:trusted-runtime:causal-anchor-transaction:v1\0");
    hasher.update(&causal_anchor_frontier);
    hasher.update(claim_digest);
    hasher.update(support_policy_digest);
    hasher.finalize().into()
}

fn acceptance_evidence_digest(handle: IntentSubmissionHandle) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"acceptance-evidence");
    hasher.update(&handle.ingress_id);
    hasher.update(handle.head_key.worldline_id.as_bytes());
    hasher.update(handle.head_key.head_id.as_bytes());
    hasher.update(&handle.submission_id);
    hasher.update(&handle.submission_generation.as_u64().to_le_bytes());
    hasher.update(&[u8::from(handle.duplicate)]);
    hasher.finalize().into()
}

fn submission_envelope_record(
    envelope: &IngressEnvelope,
    handle: IntentSubmissionHandle,
) -> WalSubmissionEnvelopeRecord {
    WalSubmissionEnvelopeRecord {
        submission_id: handle.submission_id,
        canonical_envelope_digest: envelope.ingress_id(),
        submission_generation: handle.submission_generation.as_u64(),
        head_key: handle.head_key,
        retained_envelope_bytes: envelope.to_retained_bytes_v2(),
    }
}

fn submission_transaction_digest(
    handle: IntentSubmissionHandle,
    record: SubmissionAcceptanceRecord,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"submission-transaction");
    hasher.update(&handle.submission_id);
    hasher.update(&handle.ingress_id);
    hasher.update(&handle.submission_generation.as_u64().to_le_bytes());
    hasher.update(&record.canonical_envelope_digest);
    hasher.update(&record.acceptance_evidence_digest);
    hasher.finalize().into()
}

fn installed_contract_host_admission_digest(
    submission: &IntentSubmissionRecord,
    contract: &crate::ContractEvidenceIdentity,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(INSTALLED_CONTRACT_HOST_ADMISSION_DIGEST_DOMAIN);
    hasher.update(&submission.submission_id);
    hasher.update(&submission.ingress_id);
    hasher.update(submission.head_key.worldline_id.as_bytes());
    hasher.update(submission.head_key.head_id.as_bytes());
    hasher.update(contract.package_id.as_bytes());
    hasher.update(&contract.op_id.to_le_bytes());
    hasher.finalize().into()
}

fn provider_contract_host_admission_digest(
    submission: &IntentSubmissionRecord,
    contract: &crate::ProviderContractEvidenceIdentityV1,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(PROVIDER_CONTRACT_HOST_ADMISSION_DIGEST_DOMAIN);
    hasher.update(&submission.submission_id);
    hasher.update(&submission.ingress_id);
    hasher.update(submission.head_key.worldline_id.as_bytes());
    hasher.update(submission.head_key.head_id.as_bytes());
    hasher.update(contract.package_id().as_bytes());
    hasher.update(&(contract.package_reference().coordinate().len() as u64).to_le_bytes());
    hasher.update(contract.package_reference().coordinate().as_bytes());
    hasher.update(&(contract.package_reference().digest().len() as u64).to_le_bytes());
    hasher.update(contract.package_reference().digest().as_bytes());
    hasher.update(&contract.operation_id().to_le_bytes());
    hasher.update(contract.rule_id());
    hasher.finalize().into()
}

fn echo_operation_action_admission_digest(
    submission: &IntentSubmissionRecord,
    admitted: &AdmittedEchoOperationInvocationV1,
) -> Hash {
    echo_operation_action_admission_digest_from_parts(
        submission,
        admitted.package_id(),
        admitted.installed_operation_id(),
        admitted.admission_id(),
    )
}

fn echo_operation_action_admission_digest_from_parts(
    submission: &IntentSubmissionRecord,
    package_id: crate::EchoOperationPackageIdV1,
    installed_operation_id: crate::InstalledEchoOperationIdV1,
    invocation_admission_id: crate::EchoOperationInvocationAdmissionIdV1,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(ECHO_OPERATION_ACTION_ADMISSION_DIGEST_DOMAIN);
    hasher.update(&submission.submission_id);
    hasher.update(&submission.ingress_id);
    hasher.update(submission.head_key.worldline_id.as_bytes());
    hasher.update(submission.head_key.head_id.as_bytes());
    hasher.update(&package_id.as_hash());
    hasher.update(&installed_operation_id.as_hash());
    hasher.update(&invocation_admission_id.as_hash());
    hasher.finalize().into()
}

fn submission_frontier_digest(previous: Hash, record: SubmissionAcceptanceRecord) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"submission-frontier");
    hasher.update(&previous);
    hasher.update(&record.submission_id);
    hasher.update(&record.canonical_envelope_digest);
    hasher.update(&record.acceptance_evidence_digest);
    hasher.finalize().into()
}

fn wal_tick_decision_from_observation(
    observation: IntentOutcomeObservation,
    expected_receipt_digest: Hash,
) -> Result<WalTickDecision, TrustedRuntimeWalError> {
    let (correlation, decision) = match observation {
        IntentOutcomeObservation::Decided {
            correlation,
            decision,
        } => (correlation, decision),
        IntentOutcomeObservation::UnknownSubmission { submission_id }
        | IntentOutcomeObservation::Pending { submission_id, .. } => {
            return Err(TrustedRuntimeWalError::TickOutcomeUnavailable {
                submission_id,
                receipt_digest: expected_receipt_digest,
            });
        }
    };
    if correlation.tick_receipt_digest != expected_receipt_digest {
        return Err(TrustedRuntimeWalError::TickReceiptDigestMismatch {
            expected_receipt_digest,
            observed_receipt_digest: correlation.tick_receipt_digest,
        });
    }
    Ok(match decision {
        IntentOutcomeDecision::Applied { .. } => WalTickDecision::Applied,
        IntentOutcomeDecision::Rejected {
            reason: TickReceiptRejection::FootprintConflict,
            ..
        } => WalTickDecision::RejectedFootprintConflict,
        IntentOutcomeDecision::Rejected {
            reason: TickReceiptRejection::ExecutableOperationObstruction,
            ..
        } => WalTickDecision::Obstructed,
        IntentOutcomeDecision::NoMatchingReceiptEntry { .. } => {
            return Err(TrustedRuntimeWalError::TickOutcomeUnavailable {
                submission_id: correlation.submission_id,
                receipt_digest: expected_receipt_digest,
            });
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CausalTickReceiptRef, GlobalTick, IngressSubmissionGeneration, IngressTarget, WorldlineId,
        WorldlineTick, WriterHeadKey,
    };
    use bytes::Bytes;

    fn test_head_key() -> WriterHeadKey {
        WriterHeadKey {
            worldline_id: WorldlineId::from_bytes([9; 32]),
            head_id: crate::make_head_id("runtime-wal-test"),
        }
    }

    fn creation_scope_patch(node: crate::NodeKey) -> crate::WorldlineTickPatchV1 {
        let attachment = crate::AttachmentKey::node_alpha(node);
        let parent_warp = crate::make_warp_id("operation-wal-parent-root");
        let portal = crate::AttachmentKey::node_alpha(crate::NodeKey {
            warp_id: parent_warp,
            local_id: crate::make_node_id("operation-wal-parent-portal"),
        });
        let middle_warp = crate::make_warp_id("operation-wal-middle");
        let middle_portal = crate::AttachmentKey::node_alpha(crate::NodeKey {
            warp_id: middle_warp,
            local_id: crate::make_node_id("operation-wal-middle-portal"),
        });
        crate::WorldlineTickPatchV1 {
            header: crate::WorldlineTickHeaderV1 {
                commit_global_tick: GlobalTick::from_raw(1),
                policy_id: 7,
                rule_pack_id: [3; 32],
                plan_digest: [4; 32],
                decision_digest: [5; 32],
                rewrites_digest: [6; 32],
            },
            // The parent worldline root is intentionally different from the
            // descendant node's WARP id.
            warp_id: parent_warp,
            ops: vec![
                crate::WarpOp::UpsertNode {
                    node,
                    record: crate::NodeRecord {
                        ty: crate::make_type_id("operation-wal-created-node"),
                    },
                },
                crate::WarpOp::SetAttachment {
                    key: attachment,
                    value: Some(crate::AttachmentValue::Atom(crate::AtomPayload::new(
                        crate::make_type_id("operation-wal-created-attachment"),
                        Bytes::from_static(b"created"),
                    ))),
                },
            ],
            in_slots: vec![
                crate::SlotId::Node(node),
                crate::SlotId::Attachment(attachment),
                crate::SlotId::Attachment(portal),
                crate::SlotId::Attachment(middle_portal),
            ],
            out_slots: vec![
                crate::SlotId::Node(node),
                crate::SlotId::Attachment(attachment),
            ],
            patch_digest: [7; 32],
        }
    }

    fn creation_scope_parent_state(node: crate::NodeKey) -> crate::WorldlineState {
        let parent_warp = crate::make_warp_id("operation-wal-parent-root");
        let parent_node = crate::make_node_id("operation-wal-parent-portal");
        let portal = crate::AttachmentKey::node_alpha(crate::NodeKey {
            warp_id: parent_warp,
            local_id: parent_node,
        });
        let middle_warp = crate::make_warp_id("operation-wal-middle");
        let middle_node = crate::make_node_id("operation-wal-middle-portal");
        let middle_portal = crate::AttachmentKey::node_alpha(crate::NodeKey {
            warp_id: middle_warp,
            local_id: middle_node,
        });
        let child_root = crate::make_node_id("operation-wal-descendant-root");

        let mut parent_store = crate::GraphStore::new(parent_warp);
        parent_store.insert_node(
            parent_node,
            crate::NodeRecord {
                ty: crate::make_type_id("operation-wal-parent-node"),
            },
        );
        parent_store.set_node_attachment(
            parent_node,
            Some(crate::AttachmentValue::Descend(middle_warp)),
        );
        let mut middle_store = crate::GraphStore::new(middle_warp);
        middle_store.insert_node(
            middle_node,
            crate::NodeRecord {
                ty: crate::make_type_id("operation-wal-middle-node"),
            },
        );
        middle_store.set_node_attachment(
            middle_node,
            Some(crate::AttachmentValue::Descend(node.warp_id)),
        );
        let mut child_store = crate::GraphStore::new(node.warp_id);
        child_store.insert_node(
            child_root,
            crate::NodeRecord {
                ty: crate::make_type_id("operation-wal-descendant-root-node"),
            },
        );

        let mut warp_state = crate::WarpState::new();
        warp_state.upsert_instance(
            crate::WarpInstance {
                warp_id: parent_warp,
                root_node: parent_node,
                parent: None,
            },
            parent_store,
        );
        warp_state.upsert_instance(
            crate::WarpInstance {
                warp_id: middle_warp,
                root_node: middle_node,
                parent: Some(portal),
            },
            middle_store,
        );
        warp_state.upsert_instance(
            crate::WarpInstance {
                warp_id: node.warp_id,
                root_node: child_root,
                parent: Some(middle_portal),
            },
            child_store,
        );
        crate::WorldlineState::new(
            warp_state,
            crate::NodeKey {
                warp_id: parent_warp,
                local_id: parent_node,
            },
        )
        .expect("the recovery parent-state fixture is lawful")
    }

    #[test]
    fn executable_operation_index_preserves_legacy_root_without_action_outcomes() {
        let operation_coordinate = "echo.test.LegacyRecoveryIndex.v1";
        let authority_profile_identity = [0x17; 32];
        let budget = crate::EchoOperationBudgetV1::new(7, 1_024, 1_024);
        let package = crate::ExecutableOperationPackageV1::new(
            operation_coordinate,
            crate::EchoOperationSemanticClosureV1::new(
                [0x10; 32],
                [0x11; 32],
                [0x12; 32],
                [0x13; 32],
                "echo.test.legacy-index-schema/v1",
                [0x14; 32],
                "echo.test.legacy-index-lawpack/v1",
                [0x15; 32],
            ),
            crate::echo_operation_target_profile_identity_v1(),
            authority_profile_identity,
            budget,
            crate::EchoOperationProgramV1::anchored_node_attachment_compare_and_set(
                crate::make_type_id("legacy-index-node"),
                crate::make_type_id("legacy-index-attachment"),
                128,
            ),
        );
        let package_bytes = package
            .to_canonical_bytes()
            .expect("the legacy-index package is canonical");
        let package_id = crate::echo_operation_package_id_v1(&package_bytes);
        let admitted = admit_package_v1(
            &crate::EchoOperationAdmissionPolicyV1::exact(
                package_id,
                operation_coordinate,
                authority_profile_identity,
                budget,
            ),
            package_bytes,
        )
        .expect("the legacy-index package is admitted");
        let installed =
            installed_from_admitted(admitted).expect("the legacy-index package installs");

        assert_eq!(
            recovered_echo_operation_index_root([0x20; 32], &[installed], &[], &[])
                .expect("the legacy index root is computable"),
            [
                0xff, 0x77, 0x8e, 0x79, 0x1c, 0x4b, 0x7f, 0x99, 0xb7, 0xa5, 0x4c, 0x4b, 0x8d, 0xdb,
                0x36, 0x91, 0x62, 0x17, 0x8a, 0x22, 0xe2, 0xbe, 0xde, 0xc6, 0x54, 0x4c, 0x8d, 0x25,
                0xa1, 0xa0, 0x69, 0x04,
            ]
        );
    }

    #[test]
    fn creation_wal_scope_accepts_descendants_and_rejects_mutated_shapes() {
        let node = crate::NodeKey {
            warp_id: crate::make_warp_id("operation-wal-descendant"),
            local_id: crate::make_node_id("operation-wal-created-node"),
        };
        let installed_program =
            crate::EchoOperationProgramV1::anchored_node_attachment_create_if_absent(
                crate::make_type_id("operation-wal-created-node"),
                crate::make_type_id("operation-wal-created-attachment"),
                7,
            );
        let update_program =
            crate::EchoOperationProgramV1::anchored_node_attachment_compare_and_set(
                crate::make_type_id("operation-wal-created-node"),
                crate::make_type_id("operation-wal-created-attachment"),
                7,
            );
        let patch = creation_scope_patch(node);
        let parent_state = creation_scope_parent_state(node);
        assert_eq!(
            operation_patch_scope_v1(&patch, &installed_program),
            Some(node),
            "the parent worldline root must not erase descendant operation scope"
        );
        assert_eq!(
            operation_patch_scope_in_parent_state_v1(&patch, &installed_program, &parent_state),
            Some(node),
            "activation recovery must corroborate the exact retained portal chain"
        );
        assert!(operation_application_basis_matches_scope_v1(
            &installed_program,
            node,
            crate::echo_operation_anchored_node_absent_application_basis_v1(node),
        ));
        assert!(
            !operation_application_basis_matches_scope_v1(
                &installed_program,
                node,
                EchoOperationApplicationBasisV1::new([0x91; 32], [0x92; 32]),
            ),
            "creation recovery must bind the receipt to the canonical absence proposition"
        );

        let mut node_occupied_parent = parent_state.clone();
        node_occupied_parent
            .warp_state
            .store_mut(&node.warp_id)
            .expect("the fixture retains its descendant store")
            .insert_node(
                node.local_id,
                crate::NodeRecord {
                    ty: crate::make_type_id("operation-wal-existing-node"),
                },
            );
        assert_eq!(
            operation_patch_scope_in_parent_state_v1(
                &patch,
                &installed_program,
                &node_occupied_parent
            ),
            None,
            "creation recovery must reject an occupied node even when its attachment is absent"
        );

        let mut attachment_occupied_parent = parent_state.clone();
        attachment_occupied_parent
            .warp_state
            .store_mut(&node.warp_id)
            .expect("the fixture retains its descendant store")
            .set_node_attachment(
                node.local_id,
                Some(crate::AttachmentValue::Atom(crate::AtomPayload::new(
                    crate::make_type_id("operation-wal-existing-attachment"),
                    Bytes::from_static(b"occupied"),
                ))),
            );
        assert_eq!(
            operation_patch_scope_in_parent_state_v1(
                &patch,
                &installed_program,
                &attachment_occupied_parent
            ),
            None,
            "creation recovery must reject an orphan attachment even when its node is absent"
        );

        let mut fully_occupied_parent = node_occupied_parent;
        fully_occupied_parent
            .warp_state
            .store_mut(&node.warp_id)
            .expect("the fixture retains its descendant store")
            .set_node_attachment(
                node.local_id,
                Some(crate::AttachmentValue::Atom(crate::AtomPayload::new(
                    crate::make_type_id("operation-wal-existing-attachment"),
                    Bytes::from_static(b"occupied"),
                ))),
            );
        assert_eq!(
            operation_patch_scope_in_parent_state_v1(
                &patch,
                &installed_program,
                &fully_occupied_parent
            ),
            None,
            "creation recovery must reject a fully occupied target"
        );

        assert_eq!(
            operation_patch_scope_v1(&patch, &update_program),
            None,
            "the update profile must reject the creation program's two-op patch"
        );

        let mut missing_portal_read = patch.clone();
        missing_portal_read.in_slots.retain(|slot| {
            matches!(
                slot,
                crate::SlotId::Node(candidate) if candidate == &node
            ) || matches!(
                slot,
                crate::SlotId::Attachment(candidate)
                    if candidate == &crate::AttachmentKey::node_alpha(node)
            )
        });
        assert_eq!(
            operation_patch_scope_v1(&missing_portal_read, &installed_program),
            None,
            "a descendant operation must retain the root portal dependency"
        );

        let mut duplicate_portal_read = patch.clone();
        duplicate_portal_read
            .in_slots
            .push(duplicate_portal_read.in_slots[2]);
        assert_eq!(
            operation_patch_scope_v1(&duplicate_portal_read, &installed_program),
            None,
            "a descendant operation must reject duplicate portal dependencies"
        );

        let mut substituted_root_portal = patch.clone();
        substituted_root_portal.in_slots[2] =
            crate::SlotId::Attachment(crate::AttachmentKey::node_alpha(crate::NodeKey {
                warp_id: substituted_root_portal.warp_id,
                local_id: crate::make_node_id("operation-wal-unrelated-root-portal"),
            }));
        assert_eq!(
            operation_patch_scope_in_parent_state_v1(
                &substituted_root_portal,
                &installed_program,
                &parent_state
            ),
            None,
            "recovery must reject an unrelated root-owned portal substituted for the exact descent chain"
        );

        let mut missing_middle_portal = patch.clone();
        missing_middle_portal.in_slots.remove(3);
        assert_eq!(
            operation_patch_scope_v1(&missing_middle_portal, &installed_program),
            Some(node),
            "the byte-shape layer alone cannot infer a portal's Descend target"
        );
        assert_eq!(
            operation_patch_scope_in_parent_state_v1(
                &missing_middle_portal,
                &installed_program,
                &parent_state
            ),
            None,
            "activation recovery must reject a patch that omits an intermediate portal"
        );

        let mut reversed = patch.clone();
        reversed.ops.reverse();
        assert_eq!(
            operation_patch_scope_v1(&reversed, &installed_program),
            None
        );

        let mut missing_node_write = patch.clone();
        missing_node_write.ops.remove(0);
        assert_eq!(
            operation_patch_scope_v1(&missing_node_write, &installed_program),
            None
        );

        let mut attachment_only_output = patch.clone();
        attachment_only_output.out_slots.remove(0);
        assert_eq!(
            operation_patch_scope_v1(&attachment_only_output, &installed_program),
            None
        );

        let mut mismatched_attachment = patch;
        let other_node = crate::NodeKey {
            warp_id: node.warp_id,
            local_id: crate::make_node_id("operation-wal-other-node"),
        };
        let crate::WarpOp::SetAttachment { key, .. } = &mut mismatched_attachment.ops[1] else {
            panic!("fixture has the canonical attachment operation");
        };
        *key = crate::AttachmentKey::node_alpha(other_node);
        assert_eq!(
            operation_patch_scope_v1(&mismatched_attachment, &installed_program),
            None
        );

        let mut wrong_node_type = creation_scope_patch(node);
        let crate::WarpOp::UpsertNode { record, .. } = &mut wrong_node_type.ops[0] else {
            panic!("fixture has the canonical node operation");
        };
        record.ty = crate::make_type_id("operation-wal-wrong-node-type");
        assert_eq!(
            operation_patch_scope_v1(&wrong_node_type, &installed_program),
            None,
            "recovery must reject a node type the installed program cannot emit"
        );

        let mut wrong_attachment_type = creation_scope_patch(node);
        let crate::WarpOp::SetAttachment { value, .. } = &mut wrong_attachment_type.ops[1] else {
            panic!("fixture has the canonical attachment operation");
        };
        *value = Some(crate::AttachmentValue::Atom(crate::AtomPayload::new(
            crate::make_type_id("operation-wal-wrong-attachment-type"),
            Bytes::from_static(b"created"),
        )));
        assert_eq!(
            operation_patch_scope_v1(&wrong_attachment_type, &installed_program),
            None,
            "recovery must reject an attachment type the installed program cannot emit"
        );

        let mut descended_attachment = creation_scope_patch(node);
        let crate::WarpOp::SetAttachment { value, .. } = &mut descended_attachment.ops[1] else {
            panic!("fixture has the canonical attachment operation");
        };
        *value = Some(crate::AttachmentValue::Descend(crate::make_warp_id(
            "operation-wal-hidden-descendant",
        )));
        assert_eq!(
            operation_patch_scope_v1(&descended_attachment, &installed_program),
            None,
            "recovery must reject attachment algebras the installed program cannot emit"
        );

        let mut oversized_attachment = creation_scope_patch(node);
        let crate::WarpOp::SetAttachment { value, .. } = &mut oversized_attachment.ops[1] else {
            panic!("fixture has the canonical attachment operation");
        };
        *value = Some(crate::AttachmentValue::Atom(crate::AtomPayload::new(
            crate::make_type_id("operation-wal-created-attachment"),
            Bytes::from_static(b"too-long"),
        )));
        assert_eq!(
            operation_patch_scope_v1(&oversized_attachment, &installed_program),
            None,
            "recovery must enforce the installed program's replacement bound"
        );
    }

    #[test]
    fn operation_recovery_rejects_uncorroborated_frontier_root() {
        let expected_frontiers = [AffectedFrontier {
            kind: AffectedFrontierKind::ExecutableOperationCatalog,
            before_digest: [1; 32],
            after_digest: [2; 32],
        }];
        let expected = affected_frontiers_root(&expected_frontiers);
        let actual = [3; 32];
        let transaction_id = WalTransactionId::from_hash([4; 32]);
        let transaction = crate::causal_wal::WalRecoveredTransaction {
            commit: WalTransactionCommit {
                writer_epoch: WriterEpochId::from_hash([5; 32]),
                transaction_id,
                transaction_kind: WalTransactionKind::ExecutableOperationInstallation,
                first_lsn: Lsn::from_raw(0),
                last_lsn: Lsn::from_raw(0),
                record_count: 1,
                records_root: [6; 32],
                affected_frontiers_root: actual,
                previous_committed_transaction_digest: [7; 32],
                durability_mode: WalDurabilityMode::StrictFilesystem,
                schema_version: 1,
                commit_digest: [8; 32],
            },
            frames: Vec::new(),
        };

        assert_eq!(
            validate_echo_operation_frontier_root(&transaction, &expected_frontiers),
            Err(TrustedRuntimeWalError::EchoOperationFrontierMismatch {
                transaction_id: transaction_id.as_hash(),
                expected,
                actual,
            })
        );
    }

    #[test]
    fn operation_recovery_requires_tick_scope_to_bind_the_patch_target() {
        let node = crate::NodeKey {
            warp_id: crate::make_warp_id("operation-recovery-scope"),
            local_id: crate::make_node_id("operation-recovery-target"),
        };
        let attachment_slot = crate::AttachmentKey::node_alpha(node);
        let rule_id = [21; 32];
        let attachment_type = crate::make_type_id("operation-recovery-attachment");
        let program = crate::EchoOperationProgramV1::anchored_node_attachment_compare_and_set(
            crate::make_type_id("operation-recovery-node"),
            attachment_type,
            7,
        );
        let runtime_patch = crate::WarpTickPatchV1::new(
            7,
            rule_id,
            crate::TickCommitStatus::Committed,
            vec![
                crate::SlotId::Node(node),
                crate::SlotId::Attachment(attachment_slot),
            ],
            vec![crate::SlotId::Attachment(attachment_slot)],
            vec![crate::WarpOp::SetAttachment {
                key: attachment_slot,
                value: Some(crate::AttachmentValue::Atom(crate::AtomPayload::new(
                    attachment_type,
                    Bytes::from_static(b"updated"),
                ))),
            }],
        );
        let patch = crate::WorldlineTickPatchV1 {
            header: crate::WorldlineTickHeaderV1 {
                commit_global_tick: GlobalTick::from_raw(1),
                policy_id: runtime_patch.policy_id(),
                rule_pack_id: runtime_patch.rule_pack_id(),
                plan_digest: [22; 32],
                decision_digest: [23; 32],
                rewrites_digest: [24; 32],
            },
            warp_id: node.warp_id,
            ops: runtime_patch.ops().to_vec(),
            in_slots: runtime_patch.in_slots().to_vec(),
            out_slots: runtime_patch.out_slots().to_vec(),
            patch_digest: runtime_patch.digest(),
        };
        let receipt = |scope, scope_hash| {
            crate::TickReceipt::new(
                crate::TxId::from_raw(1),
                vec![crate::TickReceiptEntry {
                    rule_id,
                    scope_hash,
                    scope,
                    disposition: crate::TickReceiptDisposition::Applied,
                }],
                vec![Vec::new()],
            )
        };
        let valid = receipt(node, crate::scope_hash(&rule_id, &node));
        assert!(operation_tick_binds_patch_v1(
            &valid, &patch, rule_id, &program,
        ));

        let wrong_scope = crate::NodeKey {
            warp_id: node.warp_id,
            local_id: crate::make_node_id("operation-recovery-wrong-scope"),
        };
        let self_consistent_wrong_scope =
            receipt(wrong_scope, crate::scope_hash(&rule_id, &wrong_scope));
        assert!(!operation_tick_binds_patch_v1(
            &self_consistent_wrong_scope,
            &patch,
            rule_id,
            &program,
        ));

        let forged_scope_hash = receipt(node, [25; 32]);
        assert!(!operation_tick_binds_patch_v1(
            &forged_scope_hash,
            &patch,
            rule_id,
            &program,
        ));
    }

    #[test]
    fn operation_recovery_corroborates_non_genesis_parent_basis_material() {
        let head_key = test_head_key();
        let parent_state_root = [31; 32];
        let parent_commit = [32; 32];
        let parent_global_tick = GlobalTick::from_raw(7);
        let parent = ProvenanceEntry {
            worldline_id: head_key.worldline_id,
            worldline_tick: WorldlineTick::ZERO,
            commit_global_tick: parent_global_tick,
            head_key: Some(head_key),
            parents: Vec::new(),
            event_kind: crate::ProvenanceEventKind::LocalCommit,
            expected: crate::HashTriplet {
                state_root: parent_state_root,
                patch_digest: [33; 32],
                commit_hash: parent_commit,
            },
            patch: None,
            tick_receipt: None,
            outputs: Vec::new(),
            atom_writes: Vec::new(),
        };
        let child = ProvenanceEntry {
            worldline_id: head_key.worldline_id,
            worldline_tick: WorldlineTick::from_raw(1),
            commit_global_tick: GlobalTick::from_raw(8),
            head_key: Some(head_key),
            parents: vec![parent.as_ref()],
            event_kind: crate::ProvenanceEventKind::LocalCommit,
            expected: crate::HashTriplet {
                state_root: [34; 32],
                patch_digest: [35; 32],
                commit_hash: [36; 32],
            },
            patch: None,
            tick_receipt: None,
            outputs: Vec::new(),
            atom_writes: Vec::new(),
        };
        let basis = EchoOperationEvaluationBasisV1::new(
            head_key,
            WorldlineTick::from_raw(1),
            Some(parent_global_tick),
            parent_state_root,
            parent_commit,
            EchoOperationApplicationBasisV1::new([37; 32], [38; 32]),
        );
        let parent_coordinate = (parent.worldline_id, parent.worldline_tick);
        assert!(
            validate_operation_receipt_parent_material(basis, &child, &BTreeMap::new()).is_err()
        );
        let provenance = BTreeMap::from([(parent_coordinate, &parent)]);
        validate_operation_receipt_parent_material(basis, &child, &provenance)
            .expect("the exact retained parent corroborates every causal basis field");
        assert!(evaluation_basis_matches_recovered_coordinate(
            basis,
            &provenance
        ));

        let mut wrong_root = parent.clone();
        wrong_root.expected.state_root = [39; 32];
        let provenance = BTreeMap::from([(parent_coordinate, &wrong_root)]);
        assert!(validate_operation_receipt_parent_material(basis, &child, &provenance).is_err());
        assert!(!evaluation_basis_matches_recovered_coordinate(
            basis,
            &provenance
        ));

        let mut wrong_global_tick = parent.clone();
        wrong_global_tick.commit_global_tick = GlobalTick::from_raw(9);
        let provenance = BTreeMap::from([(parent_coordinate, &wrong_global_tick)]);
        assert!(validate_operation_receipt_parent_material(basis, &child, &provenance).is_err());
        assert!(!evaluation_basis_matches_recovered_coordinate(
            basis,
            &provenance
        ));
    }

    fn test_correlation(receipt_digest: Hash) -> ReceiptCorrelationRecord {
        let head_key = test_head_key();
        ReceiptCorrelationRecord {
            ticketed_ingress_id: [1; 32],
            submission_id: [2; 32],
            ticket_digest: [3; 32],
            ingress_id: [4; 32],
            head_key,
            contract: None,
            commit_global_tick: GlobalTick::from_raw(1),
            worldline_tick_after: WorldlineTick::from_raw(1),
            tick_receipt_digest: receipt_digest,
            commit_hash: [5; 32],
            causal_receipt_ref: CausalTickReceiptRef {
                worldline_id: head_key.worldline_id,
                worldline_tick_after: WorldlineTick::from_raw(1),
                commit_global_tick: GlobalTick::from_raw(1),
                commit_hash: [5; 32],
                submission_id: [2; 32],
                ticket_digest: [3; 32],
                receipt_content_digest: receipt_digest,
            },
            causal_parent_receipts: Vec::new(),
        }
    }

    fn test_causal_anchor_claim(basis_frontier: CausalFrontierRef) -> CausalAnchorClaim {
        CausalAnchorClaim::from_admission_request(CausalAnchorAdmissionRequest {
            schema_version: crate::CAUSAL_ANCHOR_SCHEMA_VERSION,
            subject: crate::CausalAnchorSubject::new(
                "jedit",
                "BufferWorldline",
                "worldline:recovery-basis",
            ),
            basis_frontier,
            retained_roots: vec![crate::CausalAnchorRoot::AppSubjectRoot {
                app_id: "jedit".to_owned(),
                subject_kind: "RopeHead".to_owned(),
                id: "head:recovery-basis".to_owned(),
                role: crate::CausalAnchorAppRootRole::Authority,
            }],
            materialization_roots: Vec::new(),
            purpose: crate::CausalAnchorPurpose::Recovery,
        })
        .expect("test causal-anchor claim should be valid")
    }

    #[test]
    fn runtime_wal_recovery_rejects_anchor_claimed_at_unrelated_basis() {
        let mut wal = TrustedRuntimeWal::new_in_memory().expect("test WAL should initialize");
        let recovered_basis = wal.current_causal_anchor_basis();
        let claimed_basis = CausalFrontierRef::from_digest([0x5a; 32]);
        assert_ne!(claimed_basis, recovered_basis);
        wal.record_causal_anchor_admission(test_causal_anchor_claim(claimed_basis), [0x6b; 32])
            .expect("internal test setup should append malformed historical evidence");

        let report = wal
            .store
            .recover_read_only()
            .expect("malformed committed transaction should remain physically readable");
        let cursor_error = TrustedRuntimeWalCursor::from_recovery(&report)
            .expect_err("writer recovery must reject the unrelated anchor basis");
        assert!(matches!(
            cursor_error,
            TrustedRuntimeWalError::CausalAnchorBasisMismatch {
                claimed,
                recovered,
                ..
            } if claimed == claimed_basis.frontier_digest
                && recovered == recovered_basis.frontier_digest
        ));

        let reading_error = wal
            .recover_read_only()
            .expect_err("recovery must reject an anchor claim at an unrelated basis");
        assert!(matches!(
            reading_error,
            TrustedRuntimeWalError::CausalAnchorBasisMismatch {
                claimed,
                recovered,
                ..
            } if claimed == claimed_basis.frontier_digest
                && recovered == recovered_basis.frontier_digest
        ));
    }

    #[test]
    fn runtime_wal_recovery_rejects_unattested_anchor_frontier_transition() {
        let mut wal = TrustedRuntimeWal::new_in_memory().expect("test WAL should initialize");
        let claim = test_causal_anchor_claim(wal.current_causal_anchor_basis());
        let support_policy_digest = [0x7c; 32];
        let transaction_id = WalTransactionId::from_hash(causal_anchor_transaction_digest(
            wal.causal_anchor_frontier_digest,
            claim.claim_digest(),
            &support_policy_digest,
        ));
        let transaction = build_causal_anchor_admission_transaction(
            wal.causal_anchor_builder(transaction_id),
            claim,
            support_policy_digest,
            vec![AffectedFrontier {
                kind: AffectedFrontierKind::CausalAnchorIndex,
                before_digest: [0x8d; 32],
                after_digest: [0x9e; 32],
            }],
        )
        .expect("internal test setup should build malformed frontier evidence");
        wal.append_transaction(transaction)
            .expect("malformed historical evidence should remain physically appendable");

        let report = wal
            .store
            .recover_read_only()
            .expect("malformed committed transaction should remain physically readable");
        let cursor_error = TrustedRuntimeWalCursor::from_recovery(&report)
            .expect_err("writer recovery must reject an unattested anchor frontier transition");
        assert!(matches!(
            cursor_error,
            TrustedRuntimeWalError::CausalAnchorFrontierMismatch { .. }
        ));

        let reading_error = wal
            .recover_read_only()
            .expect_err("read-only recovery must reject an unattested anchor frontier transition");
        assert!(matches!(
            reading_error,
            TrustedRuntimeWalError::CausalAnchorFrontierMismatch { .. }
        ));
    }

    #[test]
    fn causal_anchor_recovery_traversal_drives_cursor_and_witnessed_history() {
        let mut wal = TrustedRuntimeWal::new_in_memory().expect("test WAL should initialize");
        let claim = test_causal_anchor_claim(wal.current_causal_anchor_basis());
        wal.record_causal_anchor_admission(claim, [0x4f; 32])
            .expect("test causal-anchor admission should commit");
        let report = wal
            .store
            .recover_read_only()
            .expect("committed causal-anchor admission should recover");

        let traversal = traverse_recovered_causal_anchors(&report)
            .expect("shared causal-anchor traversal should validate recovery");
        let cursor = TrustedRuntimeWalCursor::from_recovery(&report)
            .expect("writer cursor should consume the shared traversal");
        let (history, frontiers) = recover_witnessed_causal_anchor_history(&report)
            .expect("read-only history should consume the shared traversal");

        assert_eq!(history.len(), 1);
        assert_eq!(history.len(), traversal.history.len());
        assert_eq!(frontiers, traversal.causal_history_frontiers);
        assert_eq!(
            cursor.causal_history_frontier_digest,
            traversal
                .causal_history_frontiers
                .last()
                .expect("traversal must retain its terminal frontier")
                .frontier_digest
        );
        assert_eq!(
            cursor.causal_anchor_frontier_digest,
            traversal.causal_anchor_frontier_digest
        );
    }

    #[test]
    fn causal_anchor_claim_lookup_uses_projection_without_wal_replay() {
        let mut wal = TrustedRuntimeWal::new_in_memory().expect("test WAL should initialize");
        let claim = test_causal_anchor_claim(wal.current_causal_anchor_basis());
        let claim_digest = *claim.claim_digest();
        let admitted = wal
            .record_causal_anchor_admission(claim, [0x5f; 32])
            .expect("test causal-anchor admission should commit");
        wal.recover_read_only_call_count.set(0);

        let recovered = wal
            .causal_anchor_by_claim(&claim_digest, None)
            .expect("claim lookup should use a validated projection");

        assert_eq!(recovered, Some(admitted));
        assert_eq!(
            wal.recover_read_only_call_count.get(),
            0,
            "idempotency lookup must not replay committed WAL history"
        );
    }

    #[test]
    fn runtime_wal_tick_decision_rejects_pending_observation_as_invariant() {
        let err = wal_tick_decision_from_observation(
            IntentOutcomeObservation::Pending {
                submission_id: [2; 32],
                submission_generation: IngressSubmissionGeneration::from_raw(1),
                ticketed_ingress_id: Some([6; 32]),
            },
            [7; 32],
        )
        .expect_err("pending outcome cannot produce scheduler tick WAL evidence");

        assert!(matches!(
            err,
            TrustedRuntimeWalError::TickOutcomeUnavailable {
                submission_id,
                receipt_digest,
            } if submission_id == [2; 32] && receipt_digest == [7; 32]
        ));
    }

    #[test]
    fn runtime_wal_tick_decision_rejects_receipt_digest_mismatch_as_invariant() {
        let err = wal_tick_decision_from_observation(
            IntentOutcomeObservation::Decided {
                correlation: Box::new(test_correlation([8; 32])),
                decision: IntentOutcomeDecision::Applied {
                    receipt_entry_index: 0,
                    rule_id: [9; 32],
                },
            },
            [7; 32],
        )
        .expect_err("mismatched receipt digest cannot produce scheduler tick WAL evidence");

        assert!(matches!(
            err,
            TrustedRuntimeWalError::TickReceiptDigestMismatch {
                expected_receipt_digest,
                observed_receipt_digest,
            } if expected_receipt_digest == [7; 32] && observed_receipt_digest == [8; 32]
        ));
    }

    #[test]
    fn runtime_wal_tick_decision_rejects_missing_receipt_entry_as_invariant() {
        let err = wal_tick_decision_from_observation(
            IntentOutcomeObservation::Decided {
                correlation: Box::new(test_correlation([7; 32])),
                decision: IntentOutcomeDecision::NoMatchingReceiptEntry {
                    tick_receipt_digest: [7; 32],
                },
            },
            [7; 32],
        )
        .expect_err("missing receipt entries cannot produce scheduler tick WAL evidence");

        assert!(matches!(
            err,
            TrustedRuntimeWalError::TickOutcomeUnavailable {
                submission_id,
                receipt_digest,
            } if submission_id == [2; 32] && receipt_digest == [7; 32]
        ));
    }

    #[test]
    fn runtime_wal_tick_decision_maps_matching_outcome() {
        let decision = wal_tick_decision_from_observation(
            IntentOutcomeObservation::Decided {
                correlation: Box::new(test_correlation([7; 32])),
                decision: IntentOutcomeDecision::Applied {
                    receipt_entry_index: 0,
                    rule_id: [9; 32],
                },
            },
            [7; 32],
        )
        .expect("matching outcome should map to a WAL tick decision");

        assert_eq!(decision, WalTickDecision::Applied);
    }

    #[test]
    fn runtime_wal_recovery_marks_legacy_acceptance_without_envelope_material() {
        let mut wal = TrustedRuntimeWal::new_in_memory().expect("test WAL should initialize");
        let handle = IntentSubmissionHandle {
            ingress_id: [11; 32],
            head_key: test_head_key(),
            submission_id: [12; 32],
            submission_generation: IngressSubmissionGeneration::from_raw(1),
            duplicate: false,
        };
        let record = SubmissionAcceptanceRecord {
            submission_id: handle.submission_id,
            canonical_envelope_digest: handle.ingress_id,
            idempotency_key_digest: None,
            acceptance_evidence_digest: acceptance_evidence_digest(handle),
        };
        let transaction = crate::causal_wal::build_submission_acceptance_transaction(
            wal.builder(
                WalTransactionKind::SubmissionIntake,
                WalAppendAuthority::SubmissionIntake,
                WalTransactionId::from_hash(submission_transaction_digest(handle, record)),
            ),
            record,
            vec![AffectedFrontier {
                kind: AffectedFrontierKind::SubmissionQueue,
                before_digest: wal.submission_frontier_digest,
                after_digest: submission_frontier_digest(wal.submission_frontier_digest, record),
            }],
        )
        .expect("legacy acceptance transaction should build");
        wal.append_transaction(transaction)
            .expect("legacy acceptance transaction should commit");

        let recovery = wal
            .recover_read_only()
            .expect("legacy acceptance should remain inspectable");

        assert!(recovery.witnessed_submissions.is_empty());
        assert_eq!(
            recovery.missing_submission_envelopes,
            vec![handle.submission_id]
        );
        assert_eq!(recovery.certificate.obstruction_count, 1);
    }

    #[test]
    fn runtime_wal_recovery_marks_legacy_tick_without_replayable_state_material() {
        let mut wal = TrustedRuntimeWal::new_in_memory().expect("test WAL should initialize");
        let receipt = TickReceiptRecord {
            receipt_ref: CausalTickReceiptRef {
                worldline_id: WorldlineId::from_bytes([20; 32]),
                worldline_tick_after: WorldlineTick::from_raw(1),
                commit_global_tick: GlobalTick::from_raw(1),
                commit_hash: [26; 32],
                submission_id: [21; 32],
                ticket_digest: [22; 32],
                receipt_content_digest: [23; 32],
            },
            decision: WalTickDecision::Applied,
        };
        let correlation = WalReceiptCorrelationRecord {
            receipt_ref: receipt.receipt_ref,
            causal_parent_receipts: Vec::new(),
        };
        let legacy_state_delta_digest = [24; 32];
        let transaction = crate::causal_wal::build_tick_transaction(
            wal.builder(
                WalTransactionKind::SchedulerTick,
                WalAppendAuthority::TrustedScheduler,
                WalTransactionId::from_hash([25; 32]),
            ),
            receipt,
            correlation.clone(),
            legacy_state_delta_digest,
            vec![
                AffectedFrontier {
                    kind: AffectedFrontierKind::ReceiptIndex,
                    before_digest: wal.receipt_frontier_digest,
                    after_digest: receipt_frontier_digest(
                        wal.receipt_frontier_digest,
                        receipt,
                        &correlation,
                    ),
                },
                AffectedFrontier {
                    kind: AffectedFrontierKind::RuntimeState,
                    before_digest: wal.runtime_state_frontier_digest,
                    after_digest: trusted_runtime_wal_digest("legacy-runtime-frontier"),
                },
            ],
        )
        .expect("legacy tick transaction should build");
        wal.append_transaction(transaction)
            .expect("legacy tick transaction should commit");

        let recovery = wal
            .recover_read_only()
            .expect("legacy tick should remain inspectable");

        assert!(recovery.provenance_entries.is_empty());
        assert_eq!(
            recovery.missing_runtime_state_deltas,
            vec![receipt.receipt_ref.identity_digest()]
        );
        assert_eq!(recovery.certificate.obstruction_count, 1);
    }

    #[test]
    fn runtime_wal_submission_record_selection_rejects_duplicate_singular_evidence() {
        let wal = TrustedRuntimeWal::new_in_memory().expect("test WAL should initialize");
        let head_key = test_head_key();
        let envelope = IngressEnvelope::local_intent(
            IngressTarget::ExactHead { key: head_key },
            crate::make_intent_kind("runtime-wal-duplicate-submission-record"),
            b"duplicate-submission-record".to_vec(),
        );
        let handle = IntentSubmissionHandle {
            ingress_id: envelope.ingress_id(),
            head_key,
            submission_id: [21; 32],
            submission_generation: IngressSubmissionGeneration::from_raw(1),
            duplicate: false,
        };
        let acceptance = SubmissionAcceptanceRecord {
            submission_id: handle.submission_id,
            canonical_envelope_digest: handle.ingress_id,
            idempotency_key_digest: None,
            acceptance_evidence_digest: acceptance_evidence_digest(handle),
        };
        let retained_envelope = submission_envelope_record(&envelope, handle);

        for (index, duplicate_kind) in [
            WalRecordKind::SubmissionAcceptedRecorded,
            WalRecordKind::SubmissionEnvelopeRetained,
        ]
        .into_iter()
        .enumerate()
        {
            let mut transaction_id = [22; 32];
            transaction_id[0] = u8::try_from(index).expect("fixture index must fit in u8");
            let mut builder = wal.builder(
                WalTransactionKind::SubmissionIntake,
                WalAppendAuthority::SubmissionIntake,
                WalTransactionId::from_hash(transaction_id),
            );
            builder
                .push_record(
                    WalRecordKind::SubmissionAcceptedRecorded,
                    acceptance.to_payload_bytes(),
                )
                .expect("fixture acceptance must append");
            if duplicate_kind == WalRecordKind::SubmissionAcceptedRecorded {
                builder
                    .push_record(
                        WalRecordKind::SubmissionAcceptedRecorded,
                        acceptance.to_payload_bytes(),
                    )
                    .expect("duplicate acceptance must append");
            }
            builder
                .push_record(
                    WalRecordKind::SubmissionEnvelopeRetained,
                    retained_envelope.to_payload_bytes(),
                )
                .expect("fixture retained envelope must append");
            if duplicate_kind == WalRecordKind::SubmissionEnvelopeRetained {
                builder
                    .push_record(
                        WalRecordKind::SubmissionEnvelopeRetained,
                        retained_envelope.to_payload_bytes(),
                    )
                    .expect("duplicate retained envelope must append");
            }
            let transaction = builder
                .commit(Vec::new())
                .expect("duplicate-kind transaction must commit structurally");
            transaction
                .validate()
                .expect("duplicate-kind transaction must remain structurally valid");
            let recovered = crate::causal_wal::WalRecoveredTransaction {
                commit: transaction.commit,
                frames: transaction.frames,
            };

            let result = if duplicate_kind == WalRecordKind::SubmissionAcceptedRecorded {
                submission_acceptance_record_from_transaction(&recovered).map(|_| ())
            } else {
                let report = RecoveryScanReport {
                    transactions: vec![recovered],
                    tail_posture: crate::causal_wal::RecoveryTailPosture::Clean,
                };
                let submissions =
                    recover_submission_index(&report).expect("canonical acceptance should recover");
                recover_witnessed_submission_material(&report, &submissions).map(|_| ())
            };

            assert!(matches!(
                result,
                Err(TrustedRuntimeWalError::Recovery(WalRecoveryError::Index(
                    WalRecoveryIndexError::Decode(WalDecodeError::InvalidEmbeddedFrame)
                )))
            ));
        }
    }

    #[test]
    fn runtime_wal_tick_record_selection_rejects_duplicate_singular_evidence() {
        let wal = TrustedRuntimeWal::new_in_memory().expect("test WAL should initialize");
        let receipt = TickReceiptRecord {
            receipt_ref: CausalTickReceiptRef {
                worldline_id: WorldlineId::from_bytes([30; 32]),
                worldline_tick_after: WorldlineTick::from_raw(1),
                commit_global_tick: GlobalTick::from_raw(1),
                commit_hash: [31; 32],
                submission_id: [32; 32],
                ticket_digest: [33; 32],
                receipt_content_digest: [34; 32],
            },
            decision: WalTickDecision::Applied,
        };
        let correlation = WalReceiptCorrelationRecord {
            receipt_ref: receipt.receipt_ref,
            causal_parent_receipts: Vec::new(),
        };
        let receipt_bytes = receipt.to_payload_bytes();
        let correlation_bytes = correlation.to_payload_bytes();

        for (index, duplicate_kind) in [
            WalRecordKind::TickReceiptRecorded,
            WalRecordKind::ReceiptCorrelationRecorded,
            WalRecordKind::RuntimeStateDeltaRecorded,
        ]
        .into_iter()
        .enumerate()
        {
            let mut transaction_id = [35; 32];
            transaction_id[0] = u8::try_from(index).expect("fixture index must fit in u8");
            let mut builder = wal.builder(
                WalTransactionKind::SchedulerTick,
                WalAppendAuthority::TrustedScheduler,
                WalTransactionId::from_hash(transaction_id),
            );
            builder
                .push_record(WalRecordKind::TickReceiptRecorded, receipt_bytes.clone())
                .expect("fixture tick receipt must append");
            if duplicate_kind == WalRecordKind::TickReceiptRecorded {
                builder
                    .push_record(WalRecordKind::TickReceiptRecorded, receipt_bytes.clone())
                    .expect("duplicate tick receipt must append");
            }
            builder
                .push_record(
                    WalRecordKind::ReceiptCorrelationRecorded,
                    correlation_bytes.clone(),
                )
                .expect("fixture correlation must append");
            if duplicate_kind == WalRecordKind::ReceiptCorrelationRecorded {
                builder
                    .push_record(
                        WalRecordKind::ReceiptCorrelationRecorded,
                        correlation_bytes.clone(),
                    )
                    .expect("duplicate correlation must append");
            }
            builder
                .push_record(WalRecordKind::RuntimeStateDeltaRecorded, [36; 32].to_vec())
                .expect("fixture state delta must append");
            if duplicate_kind == WalRecordKind::RuntimeStateDeltaRecorded {
                builder
                    .push_record(WalRecordKind::RuntimeStateDeltaRecorded, [36; 32].to_vec())
                    .expect("duplicate state delta must append");
            }
            let transaction = builder
                .commit(Vec::new())
                .expect("duplicate-kind transaction must commit structurally");
            transaction
                .validate()
                .expect("duplicate-kind transaction must remain structurally valid");
            let recovered = crate::causal_wal::WalRecoveredTransaction {
                commit: transaction.commit,
                frames: transaction.frames,
            };

            assert!(matches!(
                tick_records_from_transaction(&recovered),
                Err(TrustedRuntimeWalError::Recovery(WalRecoveryError::Index(
                    WalRecoveryIndexError::Decode(WalDecodeError::InvalidEmbeddedFrame)
                )))
            ));
        }
    }

    #[test]
    fn runtime_wal_legacy_tick_rejects_action_outcome_frame() {
        let wal = TrustedRuntimeWal::new_in_memory().expect("test WAL should initialize");
        let receipt = TickReceiptRecord {
            receipt_ref: CausalTickReceiptRef {
                worldline_id: WorldlineId::from_bytes([40; 32]),
                worldline_tick_after: WorldlineTick::from_raw(1),
                commit_global_tick: GlobalTick::from_raw(1),
                commit_hash: [41; 32],
                submission_id: [42; 32],
                ticket_digest: [43; 32],
                receipt_content_digest: [44; 32],
            },
            decision: WalTickDecision::Obstructed,
        };
        let correlation = WalReceiptCorrelationRecord {
            receipt_ref: receipt.receipt_ref,
            causal_parent_receipts: Vec::new(),
        };
        let mut action_outcome = b"EOACT002".to_vec();
        action_outcome.extend_from_slice(&receipt.receipt_ref.submission_id);
        action_outcome.extend_from_slice(&[45; 32]);
        action_outcome.extend_from_slice(&[2, 1]);
        for byte in 46..=50 {
            action_outcome.extend_from_slice(&[byte; 32]);
        }

        let mut builder = wal.builder(
            WalTransactionKind::SchedulerTick,
            WalAppendAuthority::TrustedScheduler,
            WalTransactionId::from_hash([51; 32]),
        );
        builder
            .push_record(
                WalRecordKind::TickReceiptRecorded,
                receipt.to_payload_bytes(),
            )
            .expect("fixture legacy receipt must append");
        builder
            .push_record(
                WalRecordKind::ReceiptCorrelationRecorded,
                correlation.to_payload_bytes(),
            )
            .expect("fixture legacy correlation must append");
        builder
            .push_record(
                WalRecordKind::ExecutableOperationActionOutcomeRecorded,
                action_outcome,
            )
            .expect("adversarial Action outcome must append structurally");
        builder
            .push_record(WalRecordKind::RuntimeStateDeltaRecorded, [52; 32].to_vec())
            .expect("fixture state delta must append");
        let transaction = builder
            .commit(Vec::new())
            .expect("adversarial legacy transaction must commit structurally");
        let recovered = crate::causal_wal::WalRecoveredTransaction {
            commit: transaction.commit,
            frames: transaction.frames,
        };

        assert!(matches!(
            tick_record_batch_from_transaction(&recovered),
            Err(TrustedRuntimeWalError::Recovery(WalRecoveryError::Index(
                WalRecoveryIndexError::Decode(WalDecodeError::InvalidEmbeddedFrame)
            )))
        ));
    }

    #[test]
    fn recovered_correlation_rejects_parents_not_bound_by_envelope() {
        let head_key = test_head_key();
        let envelope_parent = CausalTickReceiptRef {
            worldline_id: head_key.worldline_id,
            worldline_tick_after: WorldlineTick::from_raw(2),
            commit_global_tick: GlobalTick::from_raw(2),
            commit_hash: [31; 32],
            submission_id: [32; 32],
            ticket_digest: [33; 32],
            receipt_content_digest: [34; 32],
        };
        let envelope = IngressEnvelope::local_intent_with_causal_parents(
            IngressTarget::ExactHead { key: head_key },
            crate::make_intent_kind("runtime-wal-parent-validation"),
            b"parent-validation".to_vec(),
            vec![IngressCausalParent::TickReceipt {
                receipt_ref: envelope_parent,
            }],
        );
        let witnessed = WitnessedSubmissionPersistenceSnapshot::new(vec![
            WitnessedSubmissionPersistenceRecord {
                submission: IntentSubmissionRecord {
                    submission_id: [2; 32],
                    ingress_id: envelope.ingress_id(),
                    head_key,
                    submission_generation: IngressSubmissionGeneration::from_raw(1),
                },
                envelope,
            },
        ]);
        let mut correlation = ReceiptCorrelationPersistenceRecord::from(&test_correlation([7; 32]));
        correlation.causal_parent_receipts = vec![CausalTickReceiptRef {
            ticket_digest: [35; 32],
            ..envelope_parent
        }];

        assert_eq!(
            validate_recovered_causal_parent_evidence(&witnessed, &[correlation.clone()]),
            Err(
                TrustedRuntimeWalError::ReceiptCorrelationCausalParentsMismatch {
                    submission_id: correlation.submission_id,
                    receipt_ref_digest: correlation.causal_receipt_ref.identity_digest(),
                }
            )
        );
    }
}

fn tick_transaction_digest(
    correlation: &ReceiptCorrelationRecord,
    decision: WalTickDecision,
    state_delta_digest: Hash,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"tick-transaction");
    hasher.update(&correlation.ticketed_ingress_id);
    hasher.update(&correlation.causal_receipt_ref.to_canonical_bytes());
    hasher.update(&correlation.ingress_id);
    hash_causal_parent_receipts(&mut hasher, &correlation.causal_parent_receipts);
    hasher.update(&[wal_tick_decision_code(decision)]);
    hasher.update(&state_delta_digest);
    hasher.finalize().into()
}

fn tick_batch_transaction_digest(
    correlations: &[(ReceiptCorrelationRecord, WalTickDecision)],
    state_delta_digest: Hash,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"tick-batch-transaction:v1\0");
    hasher.update(&(correlations.len() as u64).to_le_bytes());
    for (correlation, decision) in correlations {
        hasher.update(&correlation.ticketed_ingress_id);
        hasher.update(&correlation.causal_receipt_ref.to_canonical_bytes());
        hasher.update(&correlation.ingress_id);
        hash_causal_parent_receipts(&mut hasher, &correlation.causal_parent_receipts);
        hasher.update(&[wal_tick_decision_code(*decision)]);
    }
    hasher.update(&state_delta_digest);
    hasher.finalize().into()
}

fn receipt_frontier_digest(
    previous: Hash,
    receipt: TickReceiptRecord,
    correlation: &WalReceiptCorrelationRecord,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"receipt-frontier");
    hasher.update(&previous);
    hasher.update(&receipt.receipt_ref.to_canonical_bytes());
    hasher.update(&[wal_tick_decision_code(receipt.decision)]);
    hasher.update(&correlation.receipt_ref.to_canonical_bytes());
    hash_causal_parent_receipts(&mut hasher, &correlation.causal_parent_receipts);
    hasher.finalize().into()
}

fn hash_causal_parent_receipts(
    hasher: &mut blake3::Hasher,
    parents: &[crate::CausalTickReceiptRef],
) {
    if parents.is_empty() {
        return;
    }
    hasher.update(b"causal-parent-tick-receipts:v2\0");
    hasher.update(&(parents.len() as u64).to_le_bytes());
    for parent in parents {
        hasher.update(&parent.to_canonical_bytes());
    }
}

fn wal_tick_decision_code(decision: WalTickDecision) -> u8 {
    match decision {
        WalTickDecision::Applied => 1,
        WalTickDecision::RejectedFootprintConflict => 2,
        WalTickDecision::Obstructed => 3,
    }
}

fn executable_operation_catalog_frontier_digest(
    previous: Hash,
    package_id: crate::EchoOperationPackageIdV1,
    retained_installation_bytes: &[u8],
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"executable-operation-catalog-frontier");
    hasher.update(&previous);
    hasher.update(&package_id.as_hash());
    hasher.update(&(retained_installation_bytes.len() as u64).to_le_bytes());
    hasher.update(retained_installation_bytes);
    hasher.finalize().into()
}

fn executable_operation_receipt_frontier_digest(previous: Hash, receipt_digest: Hash) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"executable-operation-receipt-frontier");
    hasher.update(&previous);
    hasher.update(&receipt_digest);
    hasher.finalize().into()
}

fn executable_operation_installation_transaction_digest(
    catalog_frontier: Hash,
    package_id: crate::EchoOperationPackageIdV1,
    retained_installation_bytes: &[u8],
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo:trusted-runtime:executable-operation-installation-transaction:v1\0");
    hasher.update(&catalog_frontier);
    hasher.update(&package_id.as_hash());
    hasher.update(&(retained_installation_bytes.len() as u64).to_le_bytes());
    hasher.update(retained_installation_bytes);
    hasher.finalize().into()
}

fn executable_operation_tick_transaction_digest(
    receipt_frontier: Hash,
    runtime_state_frontier: Hash,
    receipt_digest: Hash,
    state_delta_digest: Hash,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo:trusted-runtime:executable-operation-tick-transaction:v1\0");
    hasher.update(&receipt_frontier);
    hasher.update(&runtime_state_frontier);
    hasher.update(&receipt_digest);
    hasher.update(&state_delta_digest);
    hasher.finalize().into()
}

fn runtime_state_frontier_digest(
    previous: Hash,
    correlation: &ReceiptCorrelationRecord,
    state_delta_digest: Hash,
) -> Hash {
    runtime_state_frontier_digest_from_fields(
        previous,
        correlation.commit_hash,
        state_delta_digest,
        correlation.commit_global_tick,
        correlation.worldline_tick_after,
    )
}

fn runtime_state_frontier_digest_from_fields(
    previous: Hash,
    commit_hash: Hash,
    state_delta_digest: Hash,
    commit_global_tick: crate::GlobalTick,
    worldline_tick_after: crate::WorldlineTick,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"runtime-state-frontier");
    hasher.update(&previous);
    hasher.update(&commit_hash);
    hasher.update(&state_delta_digest);
    hasher.update(&commit_global_tick.as_u64().to_le_bytes());
    hasher.update(&worldline_tick_after.as_u64().to_le_bytes());
    hasher.finalize().into()
}

fn recovered_legacy_runtime_state_frontier_digest(
    previous: Hash,
    correlation: WalReceiptCorrelationRecord,
    state_delta_digest: Hash,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"runtime-state-frontier:recovered");
    hasher.update(&previous);
    hasher.update(&correlation.receipt_ref.to_canonical_bytes());
    hasher.update(&state_delta_digest);
    hasher.finalize().into()
}

struct RecoveredRuntimeWalIndexEvidence<'a> {
    submissions: &'a RecoveredSubmissionIndex,
    receipts: &'a RecoveredReceiptIndex,
    witnessed_submissions: &'a WitnessedSubmissionPersistenceSnapshot,
    missing_submission_envelopes: &'a [Hash],
    provenance_entries: &'a [ProvenanceEntry],
    missing_runtime_state_deltas: &'a [Hash],
    causal_anchor_history: &'a [WitnessedCausalAnchorAdmission],
    installed_echo_operations: &'a [InstalledEchoOperationV1],
    echo_operation_receipts: &'a [EchoOperationReceiptV1],
    echo_operation_action_outcomes: &'a [(Hash, Hash, EchoOperationActionOutcomeV1)],
}

fn runtime_wal_recovery_certificate(
    report: &RecoveryScanReport,
    indexes: &RecoveredRuntimeWalIndexEvidence<'_>,
) -> Result<RecoveryCertificate, TrustedRuntimeWalError> {
    let recovered_frontier_root = report
        .last_commit_digest()
        .unwrap_or_else(|| trusted_runtime_wal_digest("recovery-frontier:empty"));
    let recovered_indexes_root = recovered_runtime_wal_indexes_root(indexes)?;
    Ok(build_recovery_certificate(
        report,
        None,
        (indexes.missing_submission_envelopes.len() + indexes.missing_runtime_state_deltas.len())
            as u64,
        recovered_frontier_root,
        recovered_indexes_root,
    ))
}

fn recovered_runtime_wal_indexes_root(
    indexes: &RecoveredRuntimeWalIndexEvidence<'_>,
) -> Result<Hash, TrustedRuntimeWalError> {
    let recovered_indexes_root = recovered_submission_material_index_root(
        recovered_submission_receipt_index_root(indexes.submissions, indexes.receipts),
        indexes.witnessed_submissions,
        indexes.missing_submission_envelopes,
    );
    let runtime_root = recovered_runtime_state_delta_index_root(
        recovered_indexes_root,
        indexes.provenance_entries,
        indexes.missing_runtime_state_deltas,
    )?;
    let causal_anchor_root =
        recovered_causal_anchor_index_root(runtime_root, indexes.causal_anchor_history);
    recovered_echo_operation_index_root(
        causal_anchor_root,
        indexes.installed_echo_operations,
        indexes.echo_operation_receipts,
        indexes.echo_operation_action_outcomes,
    )
}

fn recovered_echo_operation_index_root(
    base_root: Hash,
    installations: &[InstalledEchoOperationV1],
    receipts: &[EchoOperationReceiptV1],
    action_outcomes: &[(Hash, Hash, EchoOperationActionOutcomeV1)],
) -> Result<Hash, TrustedRuntimeWalError> {
    let legacy_root =
        recovered_echo_operation_legacy_index_root(base_root, installations, receipts)?;
    if action_outcomes.is_empty() {
        return Ok(legacy_root);
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo:trusted-runtime-wal:executable-operation-index:v2\0");
    hasher.update(&legacy_root);
    hasher.update(&(action_outcomes.len() as u64).to_le_bytes());
    for (submission_id, ingress_id, outcome) in action_outcomes {
        let bytes = retain_action_outcome_v1(*submission_id, *ingress_id, outcome)?;
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(hasher.finalize().into())
}

fn recovered_echo_operation_legacy_index_root(
    base_root: Hash,
    installations: &[InstalledEchoOperationV1],
    receipts: &[EchoOperationReceiptV1],
) -> Result<Hash, TrustedRuntimeWalError> {
    if installations.is_empty() && receipts.is_empty() {
        return Ok(base_root);
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo:trusted-runtime-wal:executable-operation-index:v1\0");
    hasher.update(&base_root);
    hasher.update(&(installations.len() as u64).to_le_bytes());
    for installed in installations {
        let bytes = retain_installation_v1(installed)?;
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    hasher.update(&(receipts.len() as u64).to_le_bytes());
    for receipt in receipts {
        let bytes = receipt.to_canonical_bytes()?;
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(hasher.finalize().into())
}

fn recovered_causal_anchor_index_root(
    base_root: Hash,
    causal_anchor_history: &[WitnessedCausalAnchorAdmission],
) -> Hash {
    if causal_anchor_history.is_empty() {
        return base_root;
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo:trusted-runtime-wal:causal-anchor-history-index:v1\0");
    hasher.update(&base_root);
    hasher.update(&(causal_anchor_history.len() as u64).to_le_bytes());
    for entry in causal_anchor_history {
        let admission = entry.admission();
        hasher.update(admission.fact().anchor_id().as_bytes());
        hasher.update(&entry.basis_before().frontier_digest);
        hasher.update(&entry.basis_after().frontier_digest);
        let fact_bytes = admission.fact().to_payload_bytes();
        hasher.update(&(fact_bytes.len() as u64).to_le_bytes());
        hasher.update(&fact_bytes);
        let receipt_bytes = admission.receipt().to_payload_bytes();
        hasher.update(&(receipt_bytes.len() as u64).to_le_bytes());
        hasher.update(&receipt_bytes);
        hasher.update(&admission.transaction_id().as_hash());
        hasher.update(&admission.committed_lsn().as_u64().to_le_bytes());
        hasher.update(admission.commit_digest());
    }
    hasher.finalize().into()
}

fn recovered_submission_material_index_root(
    base_root: Hash,
    witnessed_submissions: &WitnessedSubmissionPersistenceSnapshot,
    missing_submission_envelopes: &[Hash],
) -> Hash {
    if witnessed_submissions.is_empty() && missing_submission_envelopes.is_empty() {
        return base_root;
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo:trusted-runtime-wal:submission-material-index:v1\0");
    hasher.update(&base_root);
    hasher.update(&(witnessed_submissions.len() as u64).to_le_bytes());
    for record in witnessed_submissions.records() {
        hasher.update(&record.submission.submission_id);
        hasher.update(&record.submission.ingress_id);
        hasher.update(record.submission.head_key.worldline_id.as_bytes());
        hasher.update(record.submission.head_key.head_id.as_bytes());
        hasher.update(
            &record
                .submission
                .submission_generation
                .as_u64()
                .to_le_bytes(),
        );
        let retained_bytes = record.envelope.to_retained_bytes_v2();
        hasher.update(&(retained_bytes.len() as u64).to_le_bytes());
        hasher.update(&retained_bytes);
    }
    hasher.update(&(missing_submission_envelopes.len() as u64).to_le_bytes());
    for submission_id in missing_submission_envelopes {
        hasher.update(submission_id);
    }
    hasher.finalize().into()
}

fn recovered_runtime_state_delta_index_root(
    base_root: Hash,
    provenance_entries: &[ProvenanceEntry],
    missing_runtime_state_deltas: &[Hash],
) -> Result<Hash, TrustedRuntimeWalError> {
    if provenance_entries.is_empty() && missing_runtime_state_deltas.is_empty() {
        return Ok(base_root);
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo:trusted-runtime-wal:runtime-state-delta-index:v1\0");
    hasher.update(&base_root);
    hasher.update(&(provenance_entries.len() as u64).to_le_bytes());
    for entry in provenance_entries {
        let retained_bytes = crate::provenance_codec::encode_local_commit_v1(entry)?;
        hasher.update(&(retained_bytes.len() as u64).to_le_bytes());
        hasher.update(&retained_bytes);
    }
    hasher.update(&(missing_runtime_state_deltas.len() as u64).to_le_bytes());
    for receipt_digest in missing_runtime_state_deltas {
        hasher.update(receipt_digest);
    }
    Ok(hasher.finalize().into())
}
