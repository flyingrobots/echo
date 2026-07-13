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
        build_recovery_certificate, build_submission_acceptance_with_material_transaction,
        build_tick_transaction, recover_filesystem_store, recover_from_frames_and_commits,
        recover_receipt_index, recover_submission_index, recovered_submission_receipt_index_root,
        AffectedFrontier, AffectedFrontierKind, FilesystemWalStore, InMemoryWalStore, Lsn,
        PayloadCodecId, PayloadSchemaId, RecoveredReceiptIndex, RecoveredSubmissionIndex,
        RecoveryAccessMode, RecoveryCertificate, RecoveryScanReport, SubmissionAcceptanceRecord,
        SubmissionRetryPosture, TickReceiptRecord, WalAppendAuthority, WalBuildError,
        WalCommittedTransaction, WalDecodeError, WalDurabilityMode, WalReceiptCorrelationRecord,
        WalRecordKind, WalRecoveryError, WalRecoveryIndexError, WalSegmentId, WalStoreError,
        WalStorePort, WalSubmissionEnvelopeRecord, WalTickDecision, WalTransactionBuilder,
        WalTransactionCommit, WalTransactionId, WalTransactionKind, WriterEpochId,
        WriterEpochRequest,
    },
    Engine, IngressEnvelope, IngressEnvelopeDecodeError, IngressSubmissionGeneration,
    InstalledContractPackage, InstalledContractPackageError, InstalledContractPackageRecord,
    IntentOutcome, IntentOutcomeDecision, IntentOutcomeObservation, IntentSubmissionHandle,
    IntentSubmissionRecord, ObservationArtifact, ObservationError, ObservationRequest,
    ObservationService, OpticAdmissionTicket, ProvenanceService, ReceiptCorrelationRecord,
    RuntimeError, SchedulerCoordinator, StepRecord, TickReceiptRejection,
    TicketedRuntimeIngressAuthority, TicketedRuntimeIngressDisposition,
    WitnessedSubmissionPersistenceRecord, WitnessedSubmissionPersistenceSnapshot, WorldlineRuntime,
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
        self.runtime
            .restore_witnessed_submission_persistence(recovery.witnessed_submissions)?;
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
                let state_delta_digest = tick_state_delta_digest(&correlation);
                tick_wal_records.push((correlation, decision, state_delta_digest));
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
            for (correlation, decision, state_delta_digest) in &tick_wal_records {
                if let Err(error) =
                    runtime_wal.record_tick_receipt(correlation, *decision, *state_delta_digest)
                {
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
        Ok(TrustedRuntimeWalRecovery {
            certificate: runtime_wal_recovery_certificate(
                &report,
                &submissions,
                &receipts,
                &witnessed_submissions,
                &missing_submission_envelopes,
            ),
            submissions,
            receipts,
            witnessed_submissions,
            missing_submission_envelopes,
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
            == Some(&correlation.tick_receipt_digest)
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
        state_delta_digest: Hash,
    ) -> Result<WalTransactionCommit, TrustedRuntimeWalError> {
        let receipt = TickReceiptRecord {
            submission_id: correlation.submission_id,
            ticket_digest: correlation.ticket_digest,
            receipt_digest: correlation.tick_receipt_digest,
            decision,
        };
        let wal_correlation = WalReceiptCorrelationRecord {
            submission_id: correlation.submission_id,
            ticket_digest: correlation.ticket_digest,
            receipt_digest: correlation.tick_receipt_digest,
            causal_parent_receipts: correlation.causal_parent_receipts.clone(),
        };
        let next_receipt_frontier =
            receipt_frontier_digest(self.receipt_frontier_digest, receipt, &wal_correlation);
        let next_runtime_frontier = runtime_state_frontier_digest(
            self.runtime_state_frontier_digest,
            correlation,
            state_delta_digest,
        );
        let transaction = build_tick_transaction(
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
            state_delta_digest,
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
                    let (receipt, correlation, state_delta_digest) =
                        tick_records_from_transaction(transaction)?;
                    cursor.receipt_frontier_digest = receipt_frontier_digest(
                        cursor.receipt_frontier_digest,
                        receipt,
                        &correlation,
                    );
                    cursor.runtime_state_frontier_digest = recovered_runtime_state_frontier_digest(
                        cursor.runtime_state_frontier_digest,
                        correlation,
                        state_delta_digest,
                    );
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
        let envelope = IngressEnvelope::from_retained_bytes_v1(&material.retained_envelope_bytes)
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

fn tick_records_from_transaction(
    transaction: &crate::causal_wal::WalRecoveredTransaction,
) -> Result<(TickReceiptRecord, WalReceiptCorrelationRecord, Hash), TrustedRuntimeWalError> {
    let receipt_frame = transaction
        .frames
        .iter()
        .find(|frame| frame.header.record_kind == WalRecordKind::TickReceiptRecorded)
        .ok_or_else(missing_trusted_runtime_record)?;
    let receipt = TickReceiptRecord::from_payload_bytes(&receipt_frame.payload.canonical_bytes)
        .map_err(decode_trusted_runtime_wal_payload)?;
    let correlation_frame = transaction
        .frames
        .iter()
        .find(|frame| frame.header.record_kind == WalRecordKind::ReceiptCorrelationRecorded)
        .ok_or_else(missing_trusted_runtime_record)?;
    let correlation =
        WalReceiptCorrelationRecord::from_payload_bytes(&correlation_frame.payload.canonical_bytes)
            .map_err(decode_trusted_runtime_wal_payload)?;
    let state_delta_frame = transaction
        .frames
        .iter()
        .find(|frame| frame.header.record_kind == WalRecordKind::RuntimeStateDeltaRecorded)
        .ok_or_else(missing_trusted_runtime_record)?;
    let state_delta_digest = state_delta_frame
        .payload
        .canonical_bytes
        .as_slice()
        .try_into()
        .map_err(|_| decode_trusted_runtime_wal_payload(WalDecodeError::UnexpectedEof))?;
    Ok((receipt, correlation, state_delta_digest))
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

    /// Observes the product-facing outcome for one witnessed submission.
    #[must_use]
    pub fn observe_intent_outcome(&self, submission_id: &Hash) -> IntentOutcome {
        self.host.runtime.observe_app_intent_outcome(submission_id)
    }

    /// Runs a read-only observation through the host-owned query service.
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
        retained_envelope_bytes: envelope.to_retained_bytes_v1(),
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
        GlobalTick, IngressSubmissionGeneration, WorldlineId, WorldlineTick, WriterHeadKey,
    };

    fn test_head_key() -> WriterHeadKey {
        WriterHeadKey {
            worldline_id: WorldlineId::from_bytes([9; 32]),
            head_id: crate::make_head_id("runtime-wal-test"),
        }
    }

    fn test_correlation(receipt_digest: Hash) -> ReceiptCorrelationRecord {
        ReceiptCorrelationRecord {
            ticketed_ingress_id: [1; 32],
            submission_id: [2; 32],
            ticket_digest: [3; 32],
            ingress_id: [4; 32],
            head_key: test_head_key(),
            contract: None,
            commit_global_tick: GlobalTick::from_raw(1),
            worldline_tick_after: WorldlineTick::from_raw(1),
            tick_receipt_digest: receipt_digest,
            commit_hash: [5; 32],
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
    hasher.update(&correlation.submission_id);
    hasher.update(&correlation.ticket_digest);
    hasher.update(&correlation.ingress_id);
    hasher.update(&correlation.tick_receipt_digest);
    hasher.update(&correlation.commit_hash);
    hasher.update(&correlation.commit_global_tick.as_u64().to_le_bytes());
    hasher.update(&correlation.worldline_tick_after.as_u64().to_le_bytes());
    hash_causal_parent_receipts(&mut hasher, &correlation.causal_parent_receipts);
    hasher.update(&[wal_tick_decision_code(decision)]);
    hasher.update(&state_delta_digest);
    hasher.finalize().into()
}

fn tick_state_delta_digest(correlation: &ReceiptCorrelationRecord) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"tick-state-delta");
    hasher.update(&correlation.commit_hash);
    hasher.update(correlation.head_key.worldline_id.as_bytes());
    hasher.update(correlation.head_key.head_id.as_bytes());
    hasher.update(&correlation.commit_global_tick.as_u64().to_le_bytes());
    hasher.update(&correlation.worldline_tick_after.as_u64().to_le_bytes());
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
    hasher.update(&receipt.submission_id);
    hasher.update(&receipt.ticket_digest);
    hasher.update(&receipt.receipt_digest);
    hasher.update(&[wal_tick_decision_code(receipt.decision)]);
    hasher.update(&correlation.submission_id);
    hasher.update(&correlation.ticket_digest);
    hasher.update(&correlation.receipt_digest);
    hash_causal_parent_receipts(&mut hasher, &correlation.causal_parent_receipts);
    hasher.finalize().into()
}

fn hash_causal_parent_receipts(hasher: &mut blake3::Hasher, parents: &[Hash]) {
    if parents.is_empty() {
        return;
    }
    hasher.update(b"causal-parent-tick-receipts:v1\0");
    hasher.update(&(parents.len() as u64).to_le_bytes());
    for parent in parents {
        hasher.update(parent);
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
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"runtime-state-frontier");
    hasher.update(&previous);
    hasher.update(&correlation.commit_hash);
    hasher.update(&state_delta_digest);
    hasher.update(&correlation.commit_global_tick.as_u64().to_le_bytes());
    hasher.update(&correlation.worldline_tick_after.as_u64().to_le_bytes());
    hasher.finalize().into()
}

fn recovered_runtime_state_frontier_digest(
    previous: Hash,
    correlation: WalReceiptCorrelationRecord,
    state_delta_digest: Hash,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TRUSTED_RUNTIME_WAL_DOMAIN);
    hasher.update(b"runtime-state-frontier:recovered");
    hasher.update(&previous);
    hasher.update(&correlation.submission_id);
    hasher.update(&correlation.ticket_digest);
    hasher.update(&correlation.receipt_digest);
    hasher.update(&state_delta_digest);
    hasher.finalize().into()
}

fn runtime_wal_recovery_certificate(
    report: &RecoveryScanReport,
    submissions: &RecoveredSubmissionIndex,
    receipts: &RecoveredReceiptIndex,
    witnessed_submissions: &WitnessedSubmissionPersistenceSnapshot,
    missing_submission_envelopes: &[Hash],
) -> RecoveryCertificate {
    let recovered_frontier_root = report
        .last_commit_digest()
        .unwrap_or_else(|| trusted_runtime_wal_digest("recovery-frontier:empty"));
    let recovered_indexes_root = recovered_submission_material_index_root(
        recovered_submission_receipt_index_root(submissions, receipts),
        witnessed_submissions,
        missing_submission_envelopes,
    );
    build_recovery_certificate(
        report,
        None,
        missing_submission_envelopes.len() as u64,
        recovered_frontier_root,
        recovered_indexes_root,
    )
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
        let retained_bytes = record.envelope.to_retained_bytes_v1();
        hasher.update(&(retained_bytes.len() as u64).to_le_bytes());
        hasher.update(&retained_bytes);
    }
    hasher.update(&(missing_submission_envelopes.len() as u64).to_le_bytes());
    for submission_id in missing_submission_envelopes {
        hasher.update(submission_id);
    }
    hasher.finalize().into()
}
