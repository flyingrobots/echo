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

use crate::{
    causal_wal::{
        build_recovery_certificate, build_replayable_tick_transaction,
        build_submission_acceptance_with_material_transaction, recover_filesystem_store,
        recover_from_frames_and_commits, recover_receipt_index, recover_submission_index,
        recovered_submission_receipt_index_root, AffectedFrontier, AffectedFrontierKind,
        FilesystemWalStore, InMemoryWalStore, Lsn, PayloadCodecId, PayloadSchemaId,
        RecoveredReceiptIndex, RecoveredSubmissionIndex, RecoveryAccessMode, RecoveryCertificate,
        RecoveryScanReport, SubmissionAcceptanceRecord, SubmissionRetryPosture, TickReceiptRecord,
        WalAppendAuthority, WalBuildError, WalCommittedTransaction, WalDecodeError,
        WalDurabilityMode, WalReceiptCorrelationRecord, WalRecordKind, WalRecoveryError,
        WalRecoveryIndexError, WalRuntimeStateDeltaRecord, WalSegmentId, WalStoreError,
        WalStorePort, WalSubmissionEnvelopeRecord, WalTickDecision, WalTransactionBuilder,
        WalTransactionCommit, WalTransactionId, WalTransactionKind, WriterEpochId,
        WriterEpochRequest,
    },
    contract_host::{decode_canonical_eint, encode_canonical_eint},
    ContractInverseAdmissionRequest, ContractInverseContext, ContractInverseDerivation,
    ContractInverseHistoryObstruction, ContractInverseObstruction, ContractOperationKind, Engine,
    IngressCausalParent, IngressEnvelope, IngressEnvelopeDecodeError, IngressPayload,
    IngressSubmissionGeneration, InstalledContractPackage, InstalledContractPackageError,
    InstalledContractPackageRecord, IntentOutcome, IntentOutcomeDecision, IntentOutcomeObservation,
    IntentSubmissionHandle, IntentSubmissionRecord, ObservationArtifact, ObservationError,
    ObservationRequest, ObservationService, OpticAdmissionTicket, ProvenanceEntry,
    ProvenanceService, ProvenanceStore, ReceiptCorrelationPersistenceRecord,
    ReceiptCorrelationRecord, RetainedProvenanceError, RuntimeError, SchedulerCoordinator,
    StepRecord, TickReceiptRejection, TicketedRuntimeIngressAuthority,
    TicketedRuntimeIngressDisposition, WitnessedSubmissionPersistenceRecord,
    WitnessedSubmissionPersistenceSnapshot, WorldlineRuntime,
};
use crate::{Hash, HistoryError};

#[cfg(any(test, feature = "host_test"))]
use crate::causal_wal::FilesystemWalFaultPlan;

const TRUSTED_RUNTIME_WAL_DOMAIN: &[u8] = b"echo:trusted-runtime-wal:v1\0";

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
}

/// Summary returned after a trusted host runs the scheduler until idle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrustedRuntimeHostRunReport {
    /// Scheduler passes attempted, including the final idle pass.
    pub scheduler_passes: u64,
    /// Scheduler-owned step records committed across non-idle passes.
    pub committed_steps: usize,
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
        Ok(Self {
            runtime,
            provenance,
            engine,
            runtime_wal: None,
        })
    }

    /// Builds a trusted host from already-initialized parts.
    #[must_use]
    pub fn from_parts(
        runtime: WorldlineRuntime,
        provenance: ProvenanceService,
        engine: Engine,
    ) -> Self {
        Self {
            runtime,
            provenance,
            engine,
            runtime_wal: None,
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
        ensure_runtime_authority_is_durable(&self.runtime, &self.provenance, &recovery)?;

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

        self.runtime = restored_runtime;
        self.provenance = restored_provenance;
        self.runtime_wal = Some(runtime_wal);
        Ok(())
    }

    /// Returns the configured runtime WAL adapter, if any, as read-only
    /// evidence.
    #[must_use]
    pub fn runtime_wal(&self) -> Option<&TrustedRuntimeWal> {
        self.runtime_wal.as_ref()
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

    /// Returns the app-facing surface. This surface can submit and observe, but
    /// it cannot tick, stage ticketed ingress, register packages, or recover
    /// scheduler faults.
    pub fn app(&mut self) -> TrustedRuntimeApp<'_> {
        TrustedRuntimeApp { host: self }
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
        let target_contract = correlation.contract.as_ref().ok_or_else(|| {
            ContractInverseObstruction::TargetContractEvidenceUnavailable {
                target_receipt_ref: Box::new(request.target_receipt_ref),
            }
        })?;
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
                    .receipt_correlations()
                    .filter(|candidate| {
                        candidate.causal_receipt_ref.worldline_id == current_worldline_id
                            && candidate.worldline_tick_after == current_frontier_tick
                            && candidate.commit_hash == current_tip.commit_hash
                    })
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

    /// Runs one scheduler-owned pass.
    ///
    /// # Errors
    ///
    /// Returns a runtime error if the scheduler pass fails.
    pub fn tick_once(&mut self) -> Result<Vec<StepRecord>, TrustedRuntimeHostError> {
        let existing_correlations = self
            .runtime
            .receipt_correlations()
            .map(|correlation| correlation.ticketed_ingress_id)
            .collect::<BTreeSet<_>>();
        let runtime_before = self.runtime.clone();
        let provenance_before = self.provenance.clone();
        let records = SchedulerCoordinator::super_tick(
            &mut self.runtime,
            &mut self.provenance,
            &mut self.engine,
        )?;
        let mut tick_wal_records = Vec::new();
        if self.runtime_wal.is_some() {
            let new_correlations = self
                .runtime
                .receipt_correlations()
                .filter(|correlation| {
                    !existing_correlations.contains(&correlation.ticketed_ingress_id)
                })
                .cloned()
                .collect::<Vec<_>>();
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
                        return Err(error.into());
                    }
                };
                let (state_delta, state_delta_digest) =
                    match retained_state_delta_for_correlation(&self.provenance, &correlation) {
                        Ok(state_delta) => state_delta,
                        Err(error) => {
                            self.runtime = runtime_before;
                            self.provenance = provenance_before;
                            return Err(error);
                        }
                    };
                tick_wal_records.push((correlation, decision, state_delta, state_delta_digest));
            }
        }
        if let Some(runtime_wal) = self.runtime_wal.as_mut() {
            if runtime_wal.uses_filesystem_store() && tick_wal_records.len() > 1 {
                self.runtime = runtime_before;
                self.provenance = provenance_before;
                return Err(TrustedRuntimeWalError::FilesystemAtomicBatchUnsupported {
                    transaction_kind: WalTransactionKind::SchedulerTick,
                    transaction_count: tick_wal_records.len(),
                }
                .into());
            }
            let runtime_wal_before = runtime_wal.clone();
            for (correlation, decision, state_delta, state_delta_digest) in &tick_wal_records {
                if let Err(error) = runtime_wal.record_tick_receipt(
                    correlation,
                    *decision,
                    state_delta,
                    *state_delta_digest,
                ) {
                    if runtime_wal.recover_filesystem_tick_commit_after_error(correlation) {
                        continue;
                    }
                    if !runtime_wal.uses_filesystem_store() {
                        *runtime_wal = runtime_wal_before;
                    }
                    self.runtime = runtime_before;
                    self.provenance = provenance_before;
                    return Err(error.into());
                }
            }
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

fn ensure_runtime_authority_is_durable(
    runtime: &WorldlineRuntime,
    provenance: &ProvenanceService,
    recovery: &TrustedRuntimeWalRecovery,
) -> Result<(), TrustedRuntimeWalError> {
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

/// Minimal trusted-runtime WAL adapter for ACK-boundary integration tests.
#[derive(Clone, Debug)]
pub struct TrustedRuntimeWal {
    store: TrustedRuntimeWalStore,
    evidence_catalog: Option<crate::evidence::CausalSegmentCatalog>,
    evidence_catalog_posture: EvidenceCatalogPosture,
    #[cfg(any(test, feature = "host_test"))]
    fail_next_evidence_catalog_update: bool,
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
}

impl TrustedRuntimeWal {
    /// Builds a WAL adapter backed by an in-memory store.
    pub fn new_in_memory() -> Result<Self, TrustedRuntimeWalError> {
        Self::from_config(TrustedRuntimeWalConfig::in_memory())
    }

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
            evidence_catalog: Some(evidence_catalog),
            evidence_catalog_posture: EvidenceCatalogPosture::Fresh,
            #[cfg(any(test, feature = "host_test"))]
            fail_next_evidence_catalog_update: false,
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
        let report = self.store.recover_read_only()?;
        let submissions = recover_submission_index(&report).map_err(WalRecoveryError::from)?;
        let receipts = recover_receipt_index(&report).map_err(WalRecoveryError::from)?;
        let (witnessed_submissions, missing_submission_envelopes) =
            recover_witnessed_submission_material(&report, &submissions)?;
        let runtime_state = recover_runtime_state_delta_material(&report)?;
        let provenance_entries = runtime_state.provenance_entries;
        let receipt_correlations = runtime_state.receipt_correlations;
        let missing_runtime_state_deltas = runtime_state.missing_runtime_state_deltas;
        Ok(TrustedRuntimeWalRecovery {
            certificate: runtime_wal_recovery_certificate(
                &report,
                &submissions,
                &receipts,
                &witnessed_submissions,
                &missing_submission_envelopes,
                &provenance_entries,
                &missing_runtime_state_deltas,
            )?,
            submissions,
            receipts,
            witnessed_submissions,
            missing_submission_envelopes,
            provenance_entries,
            missing_runtime_state_deltas,
            receipt_correlations,
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

    fn append_transaction(
        &mut self,
        transaction: WalCommittedTransaction,
    ) -> Result<WalTransactionCommit, TrustedRuntimeWalError> {
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

#[derive(Clone, Copy, Debug)]
struct TrustedRuntimeWalCursor {
    has_committed_history: bool,
    next_lsn: Lsn,
    previous_frame_digest: Hash,
    previous_committed_transaction_digest: Hash,
    submission_frontier_digest: Hash,
    receipt_frontier_digest: Hash,
    runtime_state_frontier_digest: Hash,
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
        }
    }

    fn from_recovery(report: &RecoveryScanReport) -> Result<Self, TrustedRuntimeWalError> {
        let mut cursor = Self::genesis();
        for transaction in &report.transactions {
            cursor.has_committed_history = true;
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
                    let (receipt, correlation, state_delta_digest, provenance_entry) =
                        tick_records_from_transaction(transaction)?;
                    cursor.receipt_frontier_digest = receipt_frontier_digest(
                        cursor.receipt_frontier_digest,
                        receipt,
                        &correlation,
                    );
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
                            correlation,
                            state_delta_digest,
                        ),
                    };
                }
                _ => {}
            }
        }
        Ok(cursor)
    }
}

fn submission_acceptance_record_from_transaction(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
) -> Result<SubmissionAcceptanceRecord, TrustedRuntimeWalError> {
    let frame = transaction
        .frames
        .iter()
        .find(|frame| frame.header.record_kind == WalRecordKind::SubmissionAcceptedRecorded)
        .ok_or_else(missing_trusted_runtime_record)?;
    SubmissionAcceptanceRecord::from_payload_bytes(&frame.payload.canonical_bytes)
        .map_err(decode_trusted_runtime_wal_payload)
}

fn recover_witnessed_submission_material(
    report: &RecoveryScanReport,
    submissions: &RecoveredSubmissionIndex,
) -> Result<(WitnessedSubmissionPersistenceSnapshot, Vec<Hash>), TrustedRuntimeWalError> {
    let mut material_by_submission = BTreeMap::new();
    for transaction in &report.transactions {
        for frame in &transaction.frames {
            if frame.header.record_kind != WalRecordKind::SubmissionEnvelopeRetained {
                continue;
            }
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

struct RecoveredRuntimeStateMaterial {
    provenance_entries: Vec<ProvenanceEntry>,
    receipt_correlations: Vec<ReceiptCorrelationPersistenceRecord>,
    missing_runtime_state_deltas: Vec<Hash>,
}

fn recover_runtime_state_delta_material(
    report: &RecoveryScanReport,
) -> Result<RecoveredRuntimeStateMaterial, TrustedRuntimeWalError> {
    let mut entries_by_coordinate = BTreeMap::new();
    let mut correlations_by_submission = BTreeMap::new();
    let mut submission_by_ticket = BTreeMap::new();
    let mut missing = Vec::new();
    for transaction in &report.transactions {
        if transaction.commit.transaction_kind != WalTransactionKind::SchedulerTick {
            continue;
        }
        let receipt = tick_receipt_from_transaction(transaction)?;
        let wal_correlation = tick_correlation_from_transaction(transaction)?;
        if wal_correlation.receipt_ref != receipt.receipt_ref {
            return Err(decode_trusted_runtime_wal_payload(
                WalDecodeError::InvalidEmbeddedFrame,
            ));
        }
        let mut state_delta_frames = transaction
            .frames
            .iter()
            .filter(|frame| frame.header.record_kind == WalRecordKind::RuntimeStateDeltaRecorded);
        let state_delta_frame = state_delta_frames
            .next()
            .ok_or_else(missing_trusted_runtime_record)?;
        if state_delta_frames.next().is_some() {
            return Err(decode_trusted_runtime_wal_payload(
                WalDecodeError::InvalidEmbeddedFrame,
            ));
        }
        if state_delta_frame.payload.canonical_bytes.len() == core::mem::size_of::<Hash>() {
            missing.push(receipt.receipt_ref.identity_digest());
            continue;
        }
        let state_delta = WalRuntimeStateDeltaRecord::from_payload_bytes(
            &state_delta_frame.payload.canonical_bytes,
        )?;
        if state_delta.receipt_digest() != receipt.receipt_ref.receipt_content_digest {
            return Err(RetainedProvenanceError::Inconsistent("state-delta receipt").into());
        }
        let entry = state_delta.provenance_entry().clone();
        let head_key = entry
            .head_key
            .ok_or(RetainedProvenanceError::MissingHeadKey)?;
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
    Ok(RecoveredRuntimeStateMaterial {
        provenance_entries: entries,
        receipt_correlations: correlations,
        missing_runtime_state_deltas: missing,
    })
}

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
    let receipt = tick_receipt_from_transaction(transaction)?;
    let correlation = tick_correlation_from_transaction(transaction)?;
    if correlation.receipt_ref != receipt.receipt_ref {
        return Err(decode_trusted_runtime_wal_payload(
            WalDecodeError::InvalidEmbeddedFrame,
        ));
    }
    let state_delta_frame = transaction
        .frames
        .iter()
        .find(|frame| frame.header.record_kind == WalRecordKind::RuntimeStateDeltaRecorded)
        .ok_or_else(missing_trusted_runtime_record)?;
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
        if state_delta.receipt_digest() != receipt.receipt_ref.receipt_content_digest {
            return Err(RetainedProvenanceError::Inconsistent("state-delta receipt").into());
        }
        (
            state_delta.digest()?,
            Some(state_delta.provenance_entry().clone()),
        )
    };
    Ok((receipt, correlation, state_delta_digest, provenance_entry))
}

fn tick_receipt_from_transaction(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
) -> Result<TickReceiptRecord, TrustedRuntimeWalError> {
    let receipt_frame = transaction
        .frames
        .iter()
        .find(|frame| frame.header.record_kind == WalRecordKind::TickReceiptRecorded)
        .ok_or_else(missing_trusted_runtime_record)?;
    TickReceiptRecord::from_payload_bytes(&receipt_frame.payload.canonical_bytes)
        .map_err(decode_trusted_runtime_wal_payload)
}

fn tick_correlation_from_transaction(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
) -> Result<WalReceiptCorrelationRecord, TrustedRuntimeWalError> {
    let correlation_frame = transaction
        .frames
        .iter()
        .find(|frame| frame.header.record_kind == WalRecordKind::ReceiptCorrelationRecorded)
        .ok_or_else(missing_trusted_runtime_record)?;
    WalReceiptCorrelationRecord::from_payload_bytes(&correlation_frame.payload.canonical_bytes)
        .map_err(decode_trusted_runtime_wal_payload)
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

impl TrustedRuntimeApp<'_> {
    /// Submits canonical intent material as witnessed ingress history.
    ///
    /// # Errors
    ///
    /// Returns a runtime error if the target cannot accept the submission.
    pub fn submit_intent(
        &mut self,
        envelope: IngressEnvelope,
    ) -> Result<IntentSubmissionHandle, RuntimeError> {
        self.host.runtime.submit_app_intent(envelope)
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
        if self.host.runtime_wal.is_none() {
            return Err(TrustedRuntimeHostError::RuntimeWalUnavailable);
        }

        let before_runtime = self.host.runtime.clone();
        let handle = self.host.runtime.submit_app_intent(envelope.clone())?;
        let Some(runtime_wal) = self.host.runtime_wal.as_mut() else {
            self.host.runtime = before_runtime;
            return Err(TrustedRuntimeHostError::RuntimeWalUnavailable);
        };
        if handle.duplicate {
            match runtime_wal.has_submission_acceptance(handle.submission_id, envelope.ingress_id())
            {
                Ok(true) => return Ok(handle),
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
                return Ok(handle);
            }
            self.host.runtime = before_runtime;
            return Err(error.into());
        }
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
        self.submit_intent_with_runtime_wal_ack(envelope)
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

fn trusted_runtime_wal_digest(label: &str) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(label.as_bytes());
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
        CausalTickReceiptRef, GlobalTick, IngressSubmissionGeneration, WorldlineId, WorldlineTick,
        WriterHeadKey,
    };

    fn test_head_key() -> WriterHeadKey {
        WriterHeadKey {
            worldline_id: WorldlineId::from_bytes([9; 32]),
            head_id: crate::make_head_id("runtime-wal-test"),
        }
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

fn runtime_wal_recovery_certificate(
    report: &RecoveryScanReport,
    submissions: &RecoveredSubmissionIndex,
    receipts: &RecoveredReceiptIndex,
    witnessed_submissions: &WitnessedSubmissionPersistenceSnapshot,
    missing_submission_envelopes: &[Hash],
    provenance_entries: &[ProvenanceEntry],
    missing_runtime_state_deltas: &[Hash],
) -> Result<RecoveryCertificate, TrustedRuntimeWalError> {
    let recovered_frontier_root = report
        .last_commit_digest()
        .unwrap_or_else(|| trusted_runtime_wal_digest("recovery-frontier:empty"));
    let recovered_indexes_root = recovered_submission_material_index_root(
        recovered_submission_receipt_index_root(submissions, receipts),
        witnessed_submissions,
        missing_submission_envelopes,
    );
    let recovered_indexes_root = recovered_runtime_state_delta_index_root(
        recovered_indexes_root,
        provenance_entries,
        missing_runtime_state_deltas,
    )?;
    Ok(build_recovery_certificate(
        report,
        None,
        (missing_submission_envelopes.len() + missing_runtime_state_deltas.len()) as u64,
        recovered_frontier_root,
        recovered_indexes_root,
    ))
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
