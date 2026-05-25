// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Reference trusted runtime host loop.
//!
//! This module names the local host role for the v0.1.0 contract path. It is a
//! convenience wrapper around existing Echo runtime pieces; it does not create a
//! daemon, does not make wall-clock cadence semantic, and does not give
//! application code tick authority.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::{
    causal_wal::{
        build_submission_acceptance_transaction, build_tick_transaction, AffectedFrontier,
        AffectedFrontierKind, InMemoryWalStore, Lsn, PayloadCodecId, PayloadSchemaId,
        SubmissionAcceptanceRecord, TickReceiptRecord, WalAppendAuthority, WalBuildError,
        WalCommittedTransaction, WalDurabilityMode, WalReceiptCorrelationRecord, WalRecordKind,
        WalSegmentId, WalStoreError, WalStorePort, WalTickDecision, WalTransactionBuilder,
        WalTransactionCommit, WalTransactionId, WalTransactionKind, WriterEpochId,
        WriterEpochRequest,
    },
    Engine, IngressEnvelope, InstalledContractPackage, InstalledContractPackageError,
    InstalledContractPackageRecord, IntentOutcome, IntentOutcomeDecision, IntentOutcomeObservation,
    IntentSubmissionHandle, ObservationArtifact, ObservationError, ObservationRequest,
    ObservationService, OpticAdmissionTicket, ProvenanceService, ReceiptCorrelationRecord,
    RuntimeError, SchedulerCoordinator, StepRecord, TickReceiptRejection,
    TicketedRuntimeIngressAuthority, TicketedRuntimeIngressDisposition, WorldlineRuntime,
};
use crate::{Hash, HistoryError};

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
}

/// Summary returned after a trusted host runs the scheduler until idle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrustedRuntimeHostRunReport {
    /// Scheduler passes attempted, including the final idle pass.
    pub scheduler_passes: u64,
    /// Scheduler-owned step records committed across non-idle passes.
    pub committed_steps: usize,
}

/// Local trusted runtime host for the app-safe contract-host path.
///
/// Application code should receive [`TrustedRuntimeApp`], not this type. This
/// host owns package installation, ticketed runtime ingress, scheduler passes,
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
        self.runtime_wal = Some(TrustedRuntimeWal::new_in_memory()?);
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

    /// Returns the app-facing surface. This surface can submit and observe, but
    /// it cannot tick, stage ticketed ingress, install packages, or recover
    /// scheduler faults.
    pub fn app(&mut self) -> TrustedRuntimeApp<'_> {
        TrustedRuntimeApp { host: self }
    }

    /// Installs a generated contract package through the trusted host boundary.
    ///
    /// # Errors
    ///
    /// Returns an installed-package error when registry verification fails or
    /// any handler/observer conflicts with existing runtime state.
    pub fn install_contract_package<'a>(
        &mut self,
        package: InstalledContractPackage<'a>,
    ) -> Result<InstalledContractPackageRecord, InstalledContractPackageError<'a>> {
        self.engine.install_contract_package(package)
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
        let tick_wal_records = self
            .runtime
            .receipt_correlations()
            .filter(|correlation| !existing_correlations.contains(&correlation.ticketed_ingress_id))
            .map(|correlation| {
                (
                    correlation.clone(),
                    wal_tick_decision_from_observation(
                        self.runtime
                            .observe_intent_outcome(&correlation.submission_id),
                        correlation.tick_receipt_digest,
                    ),
                    tick_state_delta_digest(correlation),
                )
            })
            .collect::<Vec<_>>();
        if let Some(runtime_wal) = self.runtime_wal.as_mut() {
            for (correlation, decision, state_delta_digest) in &tick_wal_records {
                if let Err(error) =
                    runtime_wal.record_tick_receipt(correlation, *decision, *state_delta_digest)
                {
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

/// Minimal trusted-runtime WAL adapter for ACK-boundary integration tests.
#[derive(Clone, Debug)]
pub struct TrustedRuntimeWal {
    store: InMemoryWalStore,
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
        Self::new_in_memory_at_lsn(Lsn::from_raw(0))
    }

    fn new_in_memory_at_lsn(next_lsn: Lsn) -> Result<Self, TrustedRuntimeWalError> {
        let mut store = InMemoryWalStore::new();
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
        Ok(Self {
            store,
            writer_epoch,
            segment_id: WalSegmentId::from_raw(1),
            next_lsn,
            previous_frame_digest: trusted_runtime_wal_digest("previous-frame:genesis"),
            previous_committed_transaction_digest: trusted_runtime_wal_digest(
                "previous-commit:genesis",
            ),
            durability_mode: WalDurabilityMode::Buffered,
            payload_codec_id: PayloadCodecId::from_hash(trusted_runtime_wal_digest(
                "payload-codec",
            )),
            payload_schema_id: PayloadSchemaId::from_hash(trusted_runtime_wal_digest(
                "payload-schema",
            )),
            digest_domain: trusted_runtime_wal_digest("digest-domain"),
            submission_frontier_digest: trusted_runtime_wal_digest("submission-frontier:genesis"),
            receipt_frontier_digest: trusted_runtime_wal_digest("receipt-frontier:genesis"),
            runtime_state_frontier_digest: trusted_runtime_wal_digest("runtime-frontier:genesis"),
        })
    }

    /// Returns committed WAL markers recorded by the adapter.
    #[must_use]
    pub fn commits(&self) -> Vec<WalTransactionCommit> {
        self.store.read_commits()
    }

    /// Returns committed WAL frames recorded by the adapter.
    #[must_use]
    pub fn frames(&self) -> Vec<crate::causal_wal::WalFrame> {
        self.store.read_frames()
    }

    /// Returns a clone of the underlying in-memory store for recovery tests.
    #[must_use]
    pub fn cloned_store(&self) -> InMemoryWalStore {
        self.store.clone()
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
    ) -> bool {
        self.store.read_frames().into_iter().any(|frame| {
            if frame.header.record_kind != WalRecordKind::SubmissionAcceptedRecorded {
                return false;
            }
            SubmissionAcceptanceRecord::from_payload_bytes(&frame.payload.canonical_bytes)
                .is_ok_and(|record| {
                    record.submission_id == submission_id
                        && record.canonical_envelope_digest == canonical_envelope_digest
                })
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
        let transaction = build_submission_acceptance_transaction(
            self.builder(
                WalTransactionKind::SubmissionIntake,
                WalAppendAuthority::SubmissionIntake,
                WalTransactionId::from_hash(submission_transaction_digest(handle, record)),
            ),
            record,
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
        };
        let next_receipt_frontier =
            receipt_frontier_digest(self.receipt_frontier_digest, receipt, wal_correlation);
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
        let commit = transaction.commit.clone();
        self.store.append_transaction(transaction)?;
        self.next_lsn = next_lsn;
        self.previous_frame_digest = last_frame_digest;
        self.previous_committed_transaction_digest = commit.commit_digest;
        Ok(commit)
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

#[cfg(any(test, feature = "host_test"))]
impl TrustedRuntimeWal {
    /// Builds an in-memory WAL at a caller-supplied LSN for overflow tests.
    pub fn new_in_memory_at_lsn_for_test(next_lsn: Lsn) -> Result<Self, TrustedRuntimeWalError> {
        Self::new_in_memory_at_lsn(next_lsn)
    }
}

/// App-facing handle for a trusted local runtime host.
///
/// This type intentionally exposes no scheduler control, package installation,
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
    /// WAL. It does not tick, stage ticketed ingress, install packages, or
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
        if handle.duplicate
            && runtime_wal.has_submission_acceptance(handle.submission_id, envelope.ingress_id())
        {
            return Ok(handle);
        }
        if let Err(error) = runtime_wal.record_submission_acceptance(&envelope, handle) {
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
) -> WalTickDecision {
    let IntentOutcomeObservation::Decided {
        correlation,
        decision,
    } = observation
    else {
        return WalTickDecision::Obstructed;
    };
    if correlation.tick_receipt_digest != expected_receipt_digest {
        return WalTickDecision::Obstructed;
    }
    match decision {
        IntentOutcomeDecision::Applied { .. } => WalTickDecision::Applied,
        IntentOutcomeDecision::Rejected {
            reason: TickReceiptRejection::FootprintConflict,
            ..
        } => WalTickDecision::RejectedFootprintConflict,
        IntentOutcomeDecision::NoMatchingReceiptEntry { .. } => WalTickDecision::Obstructed,
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
    correlation: WalReceiptCorrelationRecord,
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
    hasher.finalize().into()
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
