// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Worldline-aware runtime coordinator and deterministic ingress routing.
//!
//! The [`WorldlineRuntime`] owns the live ingress path for ADR-0008 Phase 3:
//! per-head inboxes, deterministic routing, and canonical SuperTick stepping.

use std::collections::BTreeMap;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use thiserror::Error;

use crate::clock::{GlobalTick, WorldlineTick};
use crate::engine_impl::{CommitOutcome, Engine, EngineError};
use crate::head::{
    HeadEligibility, PlaybackHeadRegistry, RunnableWriterSet, WriterHead, WriterHeadKey,
};
#[cfg(feature = "native_rule_bootstrap")]
use crate::head_inbox::IngressPayload;
use crate::head_inbox::{InboxAddress, InboxIngestResult, IngressEnvelope, IngressTarget};
use crate::ident::Hash;
use crate::optic_artifact::OpticAdmissionTicket;
use crate::provenance_store::{
    HistoryError, ProvenanceCheckpoint, ProvenanceEntry, ProvenanceEventKind, ProvenanceRef,
    ProvenanceService, ProvenanceStore, ReplayError,
};
use crate::strand::{ForkBasisRef, Strand, StrandError, StrandId, StrandRegistry, SupportPin};
use crate::worldline::{ApplyError, WorldlineId};
use crate::worldline_registry::WorldlineRegistry;
use crate::worldline_state::{WorldlineFrontier, WorldlineState};

// =============================================================================
// Runtime Errors and Ingress Disposition
// =============================================================================

/// Runtime-level errors for worldline registration, routing, and stepping.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Attempted to register a worldline twice.
    #[error("worldline already registered: {0:?}")]
    DuplicateWorldline(WorldlineId),
    /// Attempted to register a writer head twice.
    #[error("writer head already registered: {0:?}")]
    DuplicateHead(WriterHeadKey),
    /// Attempted to use a worldline that is not registered.
    #[error("unknown worldline: {0:?}")]
    UnknownWorldline(WorldlineId),
    /// Attempted to route to a head that is not registered.
    #[error("unknown writer head: {0:?}")]
    UnknownHead(WriterHeadKey),
    /// Attempted to register more than one default writer for a worldline.
    #[error("duplicate default writer for worldline: {0:?}")]
    DuplicateDefaultWriter(WorldlineId),
    /// Attempted to reuse a public inbox address within the same worldline.
    #[error("duplicate public inbox {inbox:?} for worldline {worldline_id:?}")]
    DuplicateInboxAddress {
        /// The worldline with the conflicting address.
        worldline_id: WorldlineId,
        /// The conflicting public inbox address.
        inbox: InboxAddress,
    },
    /// No default writer has been registered for the target worldline.
    #[error("no default writer registered for worldline: {0:?}")]
    MissingDefaultWriter(WorldlineId),
    /// No named inbox route exists for the target worldline.
    #[error("no public inbox {inbox:?} registered for worldline {worldline_id:?}")]
    MissingInboxAddress {
        /// The worldline that was targeted.
        worldline_id: WorldlineId,
        /// The missing inbox address.
        inbox: InboxAddress,
    },
    /// The resolved head rejected the envelope under its inbox policy.
    #[error("writer head rejected ingress by policy: {0:?}")]
    RejectedByPolicy(WriterHeadKey),
    /// A commit against a worldline frontier failed.
    #[error(transparent)]
    Engine(#[from] EngineError),
    /// Provenance append or lookup failed during a runtime step.
    #[error(transparent)]
    Provenance(#[from] HistoryError),
    /// Historical replay or materialization failed.
    #[error(transparent)]
    Replay(#[from] ReplayError),
    /// Strand registry or support-pin operation failed.
    #[error(transparent)]
    Strand(#[from] StrandError),
    /// Attempted to advance a frontier tick past `u64::MAX`.
    #[error("frontier tick overflow for worldline: {0:?}")]
    FrontierTickOverflow(WorldlineId),
    /// Attempted to advance the global tick past `u64::MAX`.
    #[error("global tick overflow")]
    GlobalTickOverflow,
    /// Attempted to allocate more witnessed intent submission generations than
    /// the runtime counter can represent.
    #[error("intent submission generation overflow")]
    IntentSubmissionGenerationOverflow,
    /// Ticketed runtime ingress referenced a submission Echo has not witnessed.
    #[error("unknown intent submission: {0:?}")]
    UnknownIntentSubmission(Hash),
    /// Ticketed runtime ingress envelope did not match the witnessed submission.
    #[error("ticketed runtime ingress does not match witnessed submission: {0:?}")]
    TicketedIngressSubmissionMismatch(Hash),
    /// A different admission ticket already staged this witnessed submission.
    #[error("witnessed submission already has ticketed runtime ingress: {0:?}")]
    TicketedIngressAlreadyStaged(Hash),
    /// Installed contract runtime ingress received malformed canonical intent bytes.
    #[error("installed contract invocation is not a canonical EINT intent")]
    MalformedInstalledContractIntent,
    /// Installed contract runtime ingress named a mutation operation id that no
    /// installed package supports.
    #[error("unsupported installed contract mutation op id: {op_id}")]
    UnsupportedInstalledContractMutation {
        /// The unsupported canonical EINT mutation operation id.
        op_id: u32,
    },
    /// Ticketed runtime ingress attempted to claim an envelope that was already
    /// pending or committed through another ingress path.
    #[error("ticketed runtime ingress cannot claim duplicate runtime ingress {ingress_id:?} for head {head_key:?}")]
    TicketedIngressDuplicateRuntimeIngress {
        /// The resolved writer head containing the duplicate ingress.
        head_key: WriterHeadKey,
        /// The content-addressed ingress id that was already known to runtime.
        ingress_id: Hash,
    },
    /// Scheduler work is blocked by an active runtime-wide fault.
    #[error("scheduler runtime fault is active: {0:?}")]
    SchedulerRuntimeFaultActive(SchedulerFaultId),
    /// Attempted to resolve a scheduler fault that is not recorded.
    #[error("unknown scheduler fault: {0:?}")]
    UnknownSchedulerFault(SchedulerFaultId),
    /// Attempted to resolve a scheduler fault that is no longer active.
    #[error("scheduler fault is already resolved: {0:?}")]
    SchedulerFaultAlreadyResolved(SchedulerFaultId),
    /// Attempted to allocate more scheduler fault generations than the runtime
    /// counter can represent.
    #[error("scheduler fault generation overflow")]
    SchedulerFaultGenerationOverflow,
}

/// Echo-owned intake/correlation generation for witnessed intent submissions.
///
/// This is audit metadata for accepted ingress history. It is not scheduler
/// order, not worldline tick identity, and not wall-clock time.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IngressSubmissionGeneration(u64);

impl IngressSubmissionGeneration {
    /// Zero value used when Echo can derive a duplicate submission identity but
    /// no local generation record is present.
    pub const ZERO: Self = Self(0);
    /// Largest representable submission generation.
    pub const MAX: Self = Self(u64::MAX);

    /// Builds a submission generation from its raw value.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw logical value.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Increments by one, returning `None` on overflow.
    #[must_use]
    pub fn checked_increment(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Witnessed Echo ingress submission accepted by the runtime.
///
/// This record says Echo accepted a canonical ingress claim into its witnessed
/// ingress history. It is not execution, not application state mutation, and not
/// a scheduler-owned tick decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntentSubmissionRecord {
    /// Content-addressed submission event id.
    pub submission_id: Hash,
    /// Content-addressed canonical intent ingress id.
    pub ingress_id: Hash,
    /// Resolved semantic writer-head target.
    pub head_key: WriterHeadKey,
    /// Echo-owned intake/correlation generation.
    pub submission_generation: IngressSubmissionGeneration,
}

/// Explicit authority token for staging ticketed runtime ingress.
///
/// Application-facing code should not hold this token. An admission ticket is
/// evidence, but ticketed runtime ingress is an Echo runtime-owner action:
/// handing this token to application/plugin/browser code would let that code
/// choose which witnessed submissions enter scheduler-visible runtime ingress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TicketedRuntimeIngressAuthority {
    _private: (),
}

impl TicketedRuntimeIngressAuthority {
    /// Assumes trusted runtime-owner authority for staging ticketed ingress.
    ///
    /// The caller must prove it is executing inside Echo's trusted runtime
    /// owner, test harness, or equivalent host-controlled boundary.
    #[cfg(feature = "host_test")]
    #[doc(hidden)]
    #[must_use]
    pub fn assume_runtime_owner() -> Self {
        Self { _private: () }
    }
}

/// Content-addressed identifier for a scheduler run attempt.
///
/// A run id names a scheduler-owned attempt. It is not a global tick, not a
/// worldline tick, and not wall-clock time.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchedulerRunId(Hash);

impl SchedulerRunId {
    /// Builds a scheduler run id from canonical bytes.
    #[must_use]
    pub const fn from_bytes(bytes: Hash) -> Self {
        Self(bytes)
    }

    /// Returns the raw scheduler run id bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Echo-owned generation assigned to scheduler fault records.
///
/// This keeps repeated, same-cause faults distinct when a trusted runtime owner
/// resolves a fault and the same head or runtime fails again before any tick
/// advances.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchedulerFaultGeneration(u64);

impl SchedulerFaultGeneration {
    /// Zero value used before any scheduler fault has been recorded.
    pub const ZERO: Self = Self(0);
    /// Largest representable scheduler fault generation.
    pub const MAX: Self = Self(u64::MAX);

    /// Builds a scheduler fault generation from its raw value.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw logical value.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Increments by one, returning `None` on overflow.
    #[must_use]
    pub fn checked_increment(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Content-addressed identifier for scheduler fault evidence.
///
/// Fault ids name runtime safety evidence. They are not application intent ids,
/// admission tickets, or tick receipts.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchedulerFaultId(Hash);

impl SchedulerFaultId {
    /// Builds a scheduler fault id from canonical bytes.
    #[must_use]
    pub const fn from_bytes(bytes: Hash) -> Self {
        Self(bytes)
    }

    /// Returns the raw scheduler fault id bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Scope quarantined by scheduler fault evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulerFaultScope {
    /// One writer head is the scoped fault culprit.
    Head(WriterHeadKey),
    /// The scheduler/runtime as a whole is unsafe to advance.
    Runtime,
}

/// Lifecycle status for scheduler fault evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulerFaultStatus {
    /// The fault is actively blocking its scope.
    Active,
    /// Trusted runtime recovery resolved the fault.
    Resolved {
        /// Runtime-owner recovery event or control digest.
        recovery_id: Hash,
    },
}

/// Runtime-local safety evidence recorded after an internal scheduler fault.
///
/// This is control-plane posture. It is not application history, not an undo,
/// not a tick receipt, not a lawful domain rejection, and not yet published as
/// durable provenance/control-plane history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulerFaultRecord {
    /// Content-addressed fault id.
    pub fault_id: SchedulerFaultId,
    /// Echo-owned generation assigned when the fault evidence was recorded.
    pub fault_generation: SchedulerFaultGeneration,
    /// Scheduler-owned run attempt that produced the fault.
    pub run_id: SchedulerRunId,
    /// Minimal safely isolated fault scope.
    pub scope: SchedulerFaultScope,
    /// Deterministic digest of the internal fault cause.
    pub cause_digest: Hash,
    /// Active/resolved fault lifecycle posture.
    pub status: SchedulerFaultStatus,
}

/// Trusted authority token for resolving scheduler fault quarantine.
///
/// Application-facing code should not hold this token. Fault recovery is a
/// runtime-owner action because it changes scheduler safety posture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchedulerFaultRecoveryAuthority {
    _private: (),
}

impl SchedulerFaultRecoveryAuthority {
    /// Assumes trusted runtime-owner authority for resolving scheduler faults.
    ///
    /// The caller must prove it is executing inside Echo's trusted runtime
    /// owner, test harness, or equivalent host-controlled boundary.
    #[cfg(any(test, feature = "host_test", feature = "trusted_runtime"))]
    #[doc(hidden)]
    #[must_use]
    pub fn assume_runtime_owner() -> Self {
        Self { _private: () }
    }
}

/// Result of accepting an intent into witnessed ingress history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentSubmissionDisposition {
    /// Echo recorded a new witnessed submission without entering runtime ingress.
    Accepted {
        /// Content-addressed ingress id.
        ingress_id: Hash,
        /// Resolved semantic writer-head target.
        head_key: WriterHeadKey,
        /// Witnessed submission event id.
        submission_id: Hash,
        /// Echo-owned intake/correlation generation.
        submission_generation: IngressSubmissionGeneration,
    },
    /// Echo had already witnessed this semantic submission.
    Duplicate {
        /// Content-addressed ingress id.
        ingress_id: Hash,
        /// Resolved semantic writer-head target.
        head_key: WriterHeadKey,
        /// Existing witnessed submission event id.
        submission_id: Hash,
        /// Existing submission generation, or zero when only the duplicate
        /// identity can be derived from replayed committed state.
        submission_generation: IngressSubmissionGeneration,
    },
}

/// Result of ingesting an envelope into the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IngressDisposition {
    /// The envelope was admitted to the resolved head inbox.
    Accepted {
        /// Content-addressed ingress id.
        ingress_id: Hash,
        /// The head that accepted the ingress.
        head_key: WriterHeadKey,
        /// Witnessed submission event id.
        submission_id: Hash,
        /// Echo-owned intake/correlation generation.
        submission_generation: IngressSubmissionGeneration,
    },
    /// The envelope was already pending or already committed.
    Duplicate {
        /// Content-addressed ingress id.
        ingress_id: Hash,
        /// The head that owns the duplicate route target.
        head_key: WriterHeadKey,
        /// Existing or deterministically derived witnessed submission event id.
        submission_id: Hash,
        /// Existing submission generation, or zero when only the duplicate
        /// identity can be derived from replayed committed state.
        submission_generation: IngressSubmissionGeneration,
    },
}

/// Runtime ingress staged from a witnessed submission and admission ticket.
///
/// This record is correlation material only. It is not a tick receipt, not
/// handler dispatch, and not execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TicketedRuntimeIngressRecord {
    /// Content-addressed ticketed-ingress event id.
    pub ticketed_ingress_id: Hash,
    /// Witnessed Echo submission being staged.
    pub submission_id: Hash,
    /// Admission ticket digest that authorizes runtime ingress.
    pub ticket_digest: Hash,
    /// Content-addressed canonical ingress id.
    pub ingress_id: Hash,
    /// Resolved semantic writer-head target.
    pub head_key: WriterHeadKey,
}

/// Result of staging a ticketed invocation into runtime ingress.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TicketedRuntimeIngressDisposition {
    /// The ticketed invocation entered the runtime inbox.
    Staged {
        /// Ticketed ingress correlation record.
        record: TicketedRuntimeIngressRecord,
        /// Underlying inbox disposition.
        ingress: IngressDisposition,
    },
    /// The same ticketed invocation had already been staged.
    Duplicate {
        /// Existing ticketed ingress correlation record.
        record: TicketedRuntimeIngressRecord,
    },
}

/// Correlation between a ticketed runtime ingress record and a scheduler tick receipt.
///
/// This is an observation/correlation index only. It does not interpret the
/// receipt into an application outcome and does not dispatch handlers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptCorrelationRecord {
    /// Ticketed runtime ingress event that reached scheduler-owned execution.
    pub ticketed_ingress_id: Hash,
    /// Witnessed Echo submission that produced the runtime ingress.
    pub submission_id: Hash,
    /// Admission ticket digest bound to the runtime ingress.
    pub ticket_digest: Hash,
    /// Content-addressed canonical ingress id decided by the tick.
    pub ingress_id: Hash,
    /// Writer head that committed the ingress batch.
    pub head_key: WriterHeadKey,
    /// Runtime cycle stamp that produced the receipt.
    pub commit_global_tick: GlobalTick,
    /// Worldline frontier tick after the scheduler-owned commit.
    pub worldline_tick_after: WorldlineTick,
    /// Digest of the scheduler-owned tick receipt.
    pub tick_receipt_digest: Hash,
    /// Commit hash emitted by the scheduler-owned tick.
    pub commit_hash: Hash,
}

/// Polling observation for a witnessed intent submission.
///
/// This is intentionally narrower than a final applied/rejected application
/// outcome. Until receipt entries are bound to intent-level semantics, Echo can
/// report whether the submission is unknown, still pending, or decided by a
/// scheduler-owned tick receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntentOutcomeObservation {
    /// Echo has no witnessed submission for the supplied id.
    UnknownSubmission {
        /// Submission id the caller asked about.
        submission_id: Hash,
    },
    /// Echo has witnessed the submission, but no receipt correlation exists yet.
    Pending {
        /// Witnessed Echo submission id.
        submission_id: Hash,
        /// Echo-owned intake/correlation generation.
        submission_generation: IngressSubmissionGeneration,
        /// Ticketed runtime ingress id, if the submission has reached runtime ingress.
        ticketed_ingress_id: Option<Hash>,
    },
    /// Echo has correlated the submission to a scheduler-owned tick receipt.
    Decided {
        /// Scheduler-owned receipt correlation.
        correlation: ReceiptCorrelationRecord,
    },
}

/// Request to fork a strand from one precise source-lane coordinate.
#[derive(Clone, Debug)]
pub struct ForkStrandRequest {
    /// Stable strand identity to register.
    pub strand_id: StrandId,
    /// Source lane carrying the fork basis in v1.
    pub source_lane_id: WorldlineId,
    /// Last included tick in the copied prefix.
    pub fork_tick: WorldlineTick,
    /// Child worldline that will carry the speculative frontier.
    pub child_worldline_id: WorldlineId,
    /// Writer heads to register for the child worldline.
    pub writer_heads: Vec<WriterHead>,
}

/// Receipt returned after a strand fork succeeds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForkStrandReceipt {
    /// Stable strand identity.
    pub strand_id: StrandId,
    /// Immutable fork basis recorded for the new strand.
    pub fork_basis_ref: ForkBasisRef,
    /// Child worldline carrying the strand's live state.
    pub child_worldline_id: WorldlineId,
    /// Writer heads authorized for the child worldline.
    pub writer_heads: Vec<WriterHeadKey>,
}

// =============================================================================
// WorldlineRuntime
// =============================================================================

/// Top-level runtime state for the worldline model.
///
/// Bundles worldline frontiers, writer heads, routing tables, and the global
/// SuperTick counter into a single deterministic runtime object.
#[derive(Clone, Debug, Default)]
pub struct WorldlineRuntime {
    /// Registry of all worldline frontiers.
    worldlines: WorldlineRegistry,
    /// Registry of all writer heads.
    heads: PlaybackHeadRegistry,
    /// Ordered set of currently runnable (non-paused) writer heads.
    runnable: RunnableWriterSet,
    /// Global tick counter (metadata only; not per-worldline identity).
    global_tick: GlobalTick,
    /// Deterministic route table for default writers.
    default_writers: BTreeMap<WorldlineId, WriterHeadKey>,
    /// Deterministic route table for named public inboxes.
    public_inboxes: BTreeMap<WorldlineId, BTreeMap<InboxAddress, WriterHeadKey>>,
    /// Next Echo-owned submission generation to assign.
    next_submission_generation: IngressSubmissionGeneration,
    /// Witnessed submission records keyed by content-addressed submission id.
    witnessed_submissions: BTreeMap<Hash, IntentSubmissionRecord>,
    /// Deterministic lookup from resolved semantic target and ingress id to
    /// witnessed submission id.
    submission_by_target: BTreeMap<(WriterHeadKey, Hash), Hash>,
    /// Ticketed runtime ingress records keyed by deterministic event id.
    ticketed_runtime_ingress: BTreeMap<Hash, TicketedRuntimeIngressRecord>,
    /// Deterministic lookup from witnessed submission id to ticketed ingress.
    ticketed_runtime_ingress_by_submission: BTreeMap<Hash, Hash>,
    /// Deterministic lookup from resolved semantic target and ingress id to
    /// ticketed ingress id.
    ticketed_runtime_ingress_by_target: BTreeMap<(WriterHeadKey, Hash), Hash>,
    /// Receipt correlations keyed by ticketed ingress id.
    receipt_correlations_by_ticketed_ingress: BTreeMap<Hash, ReceiptCorrelationRecord>,
    /// Deterministic lookup from witnessed submission id to receipt correlation.
    receipt_correlation_by_submission: BTreeMap<Hash, Hash>,
    /// Deterministic lookup from admission ticket digest to receipt correlation.
    receipt_correlation_by_ticket: BTreeMap<Hash, Hash>,
    /// Scheduler fault evidence keyed by content-addressed fault id.
    scheduler_faults: BTreeMap<SchedulerFaultId, SchedulerFaultRecord>,
    /// Active scoped scheduler faults by writer head.
    faulted_heads: BTreeMap<WriterHeadKey, SchedulerFaultId>,
    /// Active runtime-wide scheduler fault, if the scheduler cannot safely
    /// isolate the fault to one head.
    runtime_fault: Option<SchedulerFaultId>,
    /// Next Echo-owned scheduler fault generation to assign.
    next_scheduler_fault_generation: SchedulerFaultGeneration,
    /// Registry of live speculative strands attached to the runtime.
    strands: StrandRegistry,
}

#[derive(Clone, Debug)]
struct RuntimeCheckpoint {
    global_tick: GlobalTick,
    heads: BTreeMap<WriterHeadKey, WriterHead>,
    frontiers: BTreeMap<WorldlineId, WorldlineFrontier>,
}

#[derive(Clone, Debug)]
struct ReceiptCorrelationRollbackEntry {
    ticketed_ingress_id: Hash,
    previous_record: Option<ReceiptCorrelationRecord>,
    submission_id: Hash,
    previous_submission_ticketed_ingress: Option<Hash>,
    ticket_digest: Hash,
    previous_ticket_ticketed_ingress: Option<Hash>,
}

#[derive(Clone, Debug, Default)]
struct ReceiptCorrelationRollback {
    entries: Vec<ReceiptCorrelationRollbackEntry>,
}

#[derive(Clone, Copy, Debug)]
struct ReceiptCorrelationCommitContext {
    head_key: WriterHeadKey,
    commit_global_tick: GlobalTick,
    worldline_tick_after: WorldlineTick,
    tick_receipt_digest: Hash,
    commit_hash: Hash,
}

impl WorldlineRuntime {
    /// Creates an empty runtime.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds the runnable set from the current head registry.
    pub fn refresh_runnable(&mut self) {
        self.runnable.rebuild(&self.heads);
        if self.runtime_fault.is_some() {
            self.runnable.clear();
        } else {
            self.runnable
                .retain(|key| !self.faulted_heads.contains_key(key));
        }
    }

    fn checkpoint_for(&self, keys: &[WriterHeadKey]) -> Result<RuntimeCheckpoint, RuntimeError> {
        let mut heads = BTreeMap::new();
        let mut frontiers = BTreeMap::new();

        for key in keys {
            let head = self.heads.get(key).ok_or(RuntimeError::UnknownHead(*key))?;
            heads.insert(*key, head.clone());
            if let std::collections::btree_map::Entry::Vacant(slot) =
                frontiers.entry(key.worldline_id)
            {
                let frontier = self
                    .worldlines
                    .get(&key.worldline_id)
                    .ok_or(RuntimeError::UnknownWorldline(key.worldline_id))?;
                slot.insert(frontier.clone());
            }
        }

        Ok(RuntimeCheckpoint {
            global_tick: self.global_tick,
            heads,
            frontiers,
        })
    }

    fn restore(&mut self, checkpoint: RuntimeCheckpoint) {
        self.global_tick = checkpoint.global_tick;
        for head in checkpoint.heads.into_values() {
            self.heads.insert(head);
        }
        for frontier in checkpoint.frontiers.into_values() {
            self.worldlines.replace_frontier(frontier);
        }
        self.refresh_runnable();
    }

    fn rollback_receipt_correlations(&mut self, rollback: &mut ReceiptCorrelationRollback) {
        for entry in rollback.entries.drain(..).rev() {
            match entry.previous_record {
                Some(previous) => {
                    self.receipt_correlations_by_ticketed_ingress
                        .insert(entry.ticketed_ingress_id, previous);
                }
                None => {
                    self.receipt_correlations_by_ticketed_ingress
                        .remove(&entry.ticketed_ingress_id);
                }
            }
            match entry.previous_submission_ticketed_ingress {
                Some(previous) => {
                    self.receipt_correlation_by_submission
                        .insert(entry.submission_id, previous);
                }
                None => {
                    self.receipt_correlation_by_submission
                        .remove(&entry.submission_id);
                }
            }
            match entry.previous_ticket_ticketed_ingress {
                Some(previous) => {
                    self.receipt_correlation_by_ticket
                        .insert(entry.ticket_digest, previous);
                }
                None => {
                    self.receipt_correlation_by_ticket
                        .remove(&entry.ticket_digest);
                }
            }
        }
    }

    /// Returns the registered worldline frontiers.
    #[must_use]
    pub fn worldlines(&self) -> &WorldlineRegistry {
        &self.worldlines
    }

    /// Returns the registered writer heads.
    #[must_use]
    pub fn heads(&self) -> &PlaybackHeadRegistry {
        &self.heads
    }

    /// Returns a witnessed submission by content-addressed submission id.
    #[must_use]
    pub fn witnessed_submission(&self, submission_id: &Hash) -> Option<&IntentSubmissionRecord> {
        self.witnessed_submissions.get(submission_id)
    }

    /// Iterates witnessed submissions in deterministic submission-id order.
    pub fn witnessed_submissions(&self) -> impl Iterator<Item = &IntentSubmissionRecord> {
        self.witnessed_submissions.values()
    }

    /// Returns the number of witnessed submission records.
    #[must_use]
    pub fn witnessed_submission_count(&self) -> usize {
        self.witnessed_submissions.len()
    }

    /// Returns a ticketed runtime ingress record by deterministic event id.
    #[must_use]
    pub fn ticketed_runtime_ingress(
        &self,
        ticketed_ingress_id: &Hash,
    ) -> Option<&TicketedRuntimeIngressRecord> {
        self.ticketed_runtime_ingress.get(ticketed_ingress_id)
    }

    /// Iterates ticketed runtime ingress records in deterministic id order.
    pub fn ticketed_runtime_ingress_records(
        &self,
    ) -> impl Iterator<Item = &TicketedRuntimeIngressRecord> {
        self.ticketed_runtime_ingress.values()
    }

    /// Returns the number of staged ticketed runtime ingress records.
    #[must_use]
    pub fn ticketed_runtime_ingress_count(&self) -> usize {
        self.ticketed_runtime_ingress.len()
    }

    /// Returns a receipt correlation by ticketed runtime ingress id.
    #[must_use]
    pub fn receipt_correlation_for_ticketed_ingress(
        &self,
        ticketed_ingress_id: &Hash,
    ) -> Option<&ReceiptCorrelationRecord> {
        self.receipt_correlations_by_ticketed_ingress
            .get(ticketed_ingress_id)
    }

    /// Returns a receipt correlation by witnessed submission id.
    #[must_use]
    pub fn receipt_correlation_for_submission(
        &self,
        submission_id: &Hash,
    ) -> Option<&ReceiptCorrelationRecord> {
        self.receipt_correlation_by_submission
            .get(submission_id)
            .and_then(|ticketed_ingress_id| {
                self.receipt_correlations_by_ticketed_ingress
                    .get(ticketed_ingress_id)
            })
    }

    /// Returns a receipt correlation by admission ticket digest.
    #[must_use]
    pub fn receipt_correlation_for_ticket(
        &self,
        ticket_digest: &Hash,
    ) -> Option<&ReceiptCorrelationRecord> {
        self.receipt_correlation_by_ticket
            .get(ticket_digest)
            .and_then(|ticketed_ingress_id| {
                self.receipt_correlations_by_ticketed_ingress
                    .get(ticketed_ingress_id)
            })
    }

    /// Iterates receipt correlations in deterministic ticketed-ingress id order.
    pub fn receipt_correlations(&self) -> impl Iterator<Item = &ReceiptCorrelationRecord> {
        self.receipt_correlations_by_ticketed_ingress.values()
    }

    /// Returns the number of scheduler-owned receipt correlations.
    #[must_use]
    pub fn receipt_correlation_count(&self) -> usize {
        self.receipt_correlations_by_ticketed_ingress.len()
    }

    /// Returns scheduler fault evidence by fault id.
    #[must_use]
    pub fn scheduler_fault(&self, fault_id: &SchedulerFaultId) -> Option<&SchedulerFaultRecord> {
        self.scheduler_faults.get(fault_id)
    }

    /// Returns active scheduler fault evidence for a writer head.
    #[must_use]
    pub fn scheduler_fault_for_head(
        &self,
        head_key: &WriterHeadKey,
    ) -> Option<&SchedulerFaultRecord> {
        self.faulted_heads
            .get(head_key)
            .and_then(|fault_id| self.scheduler_faults.get(fault_id))
    }

    /// Returns the active runtime-wide scheduler fault, if one exists.
    #[must_use]
    pub fn scheduler_runtime_fault(&self) -> Option<&SchedulerFaultRecord> {
        self.runtime_fault
            .as_ref()
            .and_then(|fault_id| self.scheduler_faults.get(fault_id))
    }

    /// Iterates scheduler fault evidence in deterministic fault-id order.
    pub fn scheduler_faults(&self) -> impl Iterator<Item = &SchedulerFaultRecord> {
        self.scheduler_faults.values()
    }

    /// Returns the number of scheduler fault evidence records.
    #[must_use]
    pub fn scheduler_fault_count(&self) -> usize {
        self.scheduler_faults.len()
    }

    /// Returns `true` when a writer head is actively fault-quarantined.
    #[must_use]
    pub fn is_head_faulted(&self, head_key: &WriterHeadKey) -> bool {
        self.faulted_heads.contains_key(head_key)
    }

    /// Returns `true` when the runtime is globally faulted.
    #[must_use]
    pub fn is_runtime_faulted(&self) -> bool {
        self.runtime_fault.is_some()
    }

    /// Resolves active scheduler fault quarantine through trusted runtime authority.
    ///
    /// Generic head eligibility changes do not clear fault quarantine. Recovery
    /// must cite the fault being resolved and pass through this runtime-owner
    /// boundary.
    ///
    /// # Errors
    ///
    /// Returns an error when the fault id is unknown or already resolved.
    pub fn resolve_scheduler_fault(
        &mut self,
        _authority: &SchedulerFaultRecoveryAuthority,
        fault_id: SchedulerFaultId,
        recovery_id: Hash,
    ) -> Result<(), RuntimeError> {
        let record = self
            .scheduler_faults
            .get_mut(&fault_id)
            .ok_or(RuntimeError::UnknownSchedulerFault(fault_id))?;
        if !matches!(record.status, SchedulerFaultStatus::Active) {
            return Err(RuntimeError::SchedulerFaultAlreadyResolved(fault_id));
        }

        match record.scope {
            SchedulerFaultScope::Head(head_key) => {
                if self.faulted_heads.get(&head_key) == Some(&fault_id) {
                    self.faulted_heads.remove(&head_key);
                }
            }
            SchedulerFaultScope::Runtime => {
                if self.runtime_fault == Some(fault_id) {
                    self.runtime_fault = None;
                }
            }
        }
        record.status = SchedulerFaultStatus::Resolved { recovery_id };
        self.refresh_runnable();
        Ok(())
    }

    /// Observes the current scheduler-owned outcome posture for a submission.
    ///
    /// This is a zero-write polling surface. It does not tick, dispatch
    /// handlers, subscribe to streams, or infer applied/rejected semantics from
    /// receipt entries.
    #[must_use]
    pub fn observe_intent_outcome(&self, submission_id: &Hash) -> IntentOutcomeObservation {
        let Some(submission) = self.witnessed_submissions.get(submission_id) else {
            return IntentOutcomeObservation::UnknownSubmission {
                submission_id: *submission_id,
            };
        };
        if let Some(correlation) = self.receipt_correlation_for_submission(submission_id) {
            return IntentOutcomeObservation::Decided {
                correlation: correlation.clone(),
            };
        }
        IntentOutcomeObservation::Pending {
            submission_id: *submission_id,
            submission_generation: submission.submission_generation,
            ticketed_ingress_id: self
                .ticketed_runtime_ingress_by_submission
                .get(submission_id)
                .copied(),
        }
    }

    /// Returns the current correlation tick.
    #[must_use]
    pub fn global_tick(&self) -> GlobalTick {
        self.global_tick
    }

    pub(crate) fn advance_global_tick(&mut self) -> Result<GlobalTick, RuntimeError> {
        self.global_tick = self
            .global_tick
            .checked_increment()
            .ok_or(RuntimeError::GlobalTickOverflow)?;
        Ok(self.global_tick)
    }

    /// Returns the live strand registry.
    #[must_use]
    pub fn strands(&self) -> &StrandRegistry {
        &self.strands
    }

    pub(crate) fn frontier_mut(
        &mut self,
        worldline_id: &WorldlineId,
    ) -> Result<&mut WorldlineFrontier, RuntimeError> {
        self.worldlines
            .frontier_mut(worldline_id)
            .ok_or(RuntimeError::UnknownWorldline(*worldline_id))
    }

    #[cfg(test)]
    pub(crate) fn strands_mut_for_tests(&mut self) -> &mut StrandRegistry {
        &mut self.strands
    }

    /// Registers a worldline frontier with the runtime.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::DuplicateWorldline`] if the worldline already exists.
    pub fn register_worldline(
        &mut self,
        worldline_id: WorldlineId,
        state: WorldlineState,
    ) -> Result<(), RuntimeError> {
        self.worldlines
            .register(worldline_id, state)
            .map_err(|_| RuntimeError::DuplicateWorldline(worldline_id))
    }

    /// Registers a strand whose worldlines and heads are already live in the runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if the strand references missing worldlines/heads or
    /// violates strand-registry invariants.
    pub fn register_strand(&mut self, strand: Strand) -> Result<(), RuntimeError> {
        if !self
            .worldlines
            .contains(&strand.fork_basis_ref.source_lane_id)
        {
            return Err(RuntimeError::UnknownWorldline(
                strand.fork_basis_ref.source_lane_id,
            ));
        }
        if !self.worldlines.contains(&strand.child_worldline_id) {
            return Err(RuntimeError::UnknownWorldline(strand.child_worldline_id));
        }
        for head_key in &strand.writer_heads {
            if self.heads.get(head_key).is_none() {
                return Err(RuntimeError::UnknownHead(*head_key));
            }
        }
        self.strands.insert(strand).map_err(RuntimeError::Strand)
    }

    /// Forks a new strand from one precise source-lane coordinate.
    ///
    /// This copies provenance history, materializes the child frontier at the
    /// requested fork basis, registers the child worldline and writer heads,
    /// then inserts the strand relation object.
    ///
    /// On failure, both runtime and provenance are restored to their pre-fork
    /// state so forking does not leave partial truth behind.
    ///
    /// # Errors
    ///
    /// Returns an error if the source lane is missing, provenance cannot fork
    /// or replay the child history, writer-head registration fails, or strand
    /// invariants are violated.
    pub fn fork_strand(
        &mut self,
        provenance: &mut ProvenanceService,
        request: ForkStrandRequest,
    ) -> Result<ForkStrandReceipt, RuntimeError> {
        let runtime_before = self.clone();
        let provenance_before = provenance.clone();

        let outcome =
            (|| {
                let source_state = self
                    .worldlines
                    .get(&request.source_lane_id)
                    .ok_or(RuntimeError::UnknownWorldline(request.source_lane_id))?
                    .state()
                    .clone();
                let historical_source_state = provenance.replay_worldline_state_at(
                    request.source_lane_id,
                    &source_state,
                    request.fork_tick,
                )?;

                provenance.fork(
                    request.source_lane_id,
                    request.fork_tick,
                    request.child_worldline_id,
                )?;

                let child_target_tick = request.fork_tick.checked_increment().ok_or(
                    RuntimeError::FrontierTickOverflow(request.child_worldline_id),
                )?;
                let child_state = provenance.replay_worldline_state_at(
                    request.child_worldline_id,
                    &historical_source_state,
                    child_target_tick,
                )?;

                let source_entry = provenance.entry(request.source_lane_id, request.fork_tick)?;
                let fork_basis_ref = ForkBasisRef {
                    source_lane_id: request.source_lane_id,
                    fork_tick: request.fork_tick,
                    commit_hash: source_entry.expected.commit_hash,
                    boundary_hash: source_entry.expected.state_root,
                    provenance_ref: source_entry.as_ref(),
                };
                let writer_heads = request
                    .writer_heads
                    .iter()
                    .map(|head| *head.key())
                    .collect::<Vec<_>>();

                self.register_worldline(request.child_worldline_id, child_state)?;
                for head in request.writer_heads {
                    self.register_writer_head(head)?;
                }
                self.register_strand(Strand {
                    strand_id: request.strand_id,
                    fork_basis_ref,
                    child_worldline_id: request.child_worldline_id,
                    writer_heads: writer_heads.clone(),
                    support_pins: Vec::new(),
                })?;

                Ok(ForkStrandReceipt {
                    strand_id: request.strand_id,
                    fork_basis_ref,
                    child_worldline_id: request.child_worldline_id,
                    writer_heads,
                })
            })();

        match outcome {
            Ok(receipt) => Ok(receipt),
            Err(err) => {
                *self = runtime_before;
                *provenance = provenance_before;
                Err(err)
            }
        }
    }

    /// Adds a validated support pin between two live strands.
    ///
    /// # Errors
    ///
    /// Returns an error if the pin would violate strand invariants or if the
    /// pinned coordinate is not available in provenance.
    pub fn pin_support(
        &mut self,
        provenance: &ProvenanceService,
        strand_id: StrandId,
        support_strand_id: StrandId,
        pinned_tick: WorldlineTick,
    ) -> Result<SupportPin, RuntimeError> {
        self.strands
            .pin_support(provenance, strand_id, support_strand_id, pinned_tick)
            .map_err(RuntimeError::Strand)
    }

    /// Removes one support pin from a live strand.
    ///
    /// # Errors
    ///
    /// Returns an error if the owner strand or support target is not found.
    pub fn unpin_support(
        &mut self,
        strand_id: StrandId,
        support_strand_id: StrandId,
    ) -> Result<SupportPin, RuntimeError> {
        self.strands
            .unpin_support(strand_id, support_strand_id)
            .map_err(RuntimeError::Strand)
    }

    /// Registers a writer head and its routing metadata with the runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if the worldline is missing, if the head key already
    /// exists, if a default writer already exists for the worldline, or if a
    /// public inbox address is reused within the worldline.
    pub fn register_writer_head(&mut self, head: WriterHead) -> Result<(), RuntimeError> {
        let key = *head.key();
        if !self.worldlines.contains(&key.worldline_id) {
            return Err(RuntimeError::UnknownWorldline(key.worldline_id));
        }
        if self.heads.get(&key).is_some() {
            return Err(RuntimeError::DuplicateHead(key));
        }
        if head.is_default_writer() && self.default_writers.contains_key(&key.worldline_id) {
            return Err(RuntimeError::DuplicateDefaultWriter(key.worldline_id));
        }
        if let Some(inbox) = head.public_inbox() {
            if self
                .public_inboxes
                .get(&key.worldline_id)
                .is_some_and(|routes| routes.contains_key(inbox))
            {
                return Err(RuntimeError::DuplicateInboxAddress {
                    worldline_id: key.worldline_id,
                    inbox: inbox.clone(),
                });
            }
        }

        if head.is_default_writer() {
            self.default_writers.insert(key.worldline_id, key);
        }
        if let Some(inbox) = head.public_inbox().cloned() {
            self.public_inboxes
                .entry(key.worldline_id)
                .or_default()
                .insert(inbox, key);
        }
        self.heads.insert(head);
        self.refresh_runnable();
        Ok(())
    }

    /// Sets declarative scheduler eligibility for a specific head.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::UnknownHead`] if the head is not registered.
    pub fn set_head_eligibility(
        &mut self,
        key: WriterHeadKey,
        eligibility: HeadEligibility,
    ) -> Result<(), RuntimeError> {
        self.heads
            .get_mut(&key)
            .ok_or(RuntimeError::UnknownHead(key))?
            .set_eligibility(eligibility);
        self.refresh_runnable();
        Ok(())
    }

    /// Records an accepted intent submission without entering runtime ingress.
    ///
    /// This is witnessed Echo ingress history only. It does not store the
    /// envelope in a head inbox, does not advance ticks, and does not dispatch
    /// handlers. A later ticketed runtime ingress step must stage the envelope
    /// before scheduler-owned execution can consider it.
    ///
    /// # Errors
    ///
    /// Returns an error if the routing target does not resolve or if the target
    /// head would reject the envelope under its inbox policy.
    pub fn submit_intent(
        &mut self,
        envelope: IngressEnvelope,
    ) -> Result<IntentSubmissionDisposition, RuntimeError> {
        let ingress_id = envelope.ingress_id();
        let head_key = self.resolve_target(envelope.target())?;

        if self
            .worldlines
            .get(&head_key.worldline_id)
            .is_some_and(|frontier| {
                frontier
                    .state()
                    .contains_committed_ingress(&head_key, &ingress_id)
            })
        {
            let record = self.duplicate_submission_record(head_key, ingress_id);
            return Ok(IntentSubmissionDisposition::Duplicate {
                ingress_id,
                head_key,
                submission_id: record.submission_id,
                submission_generation: record.submission_generation,
            });
        }

        let head = self
            .heads
            .get(&head_key)
            .ok_or(RuntimeError::UnknownHead(head_key))?;
        if !head.inbox().would_accept(&envelope) {
            return Err(RuntimeError::RejectedByPolicy(head_key));
        }

        if let Some(record) = self
            .submission_by_target
            .get(&(head_key, ingress_id))
            .and_then(|submission_id| self.witnessed_submissions.get(submission_id))
        {
            return Ok(IntentSubmissionDisposition::Duplicate {
                ingress_id,
                head_key,
                submission_id: record.submission_id,
                submission_generation: record.submission_generation,
            });
        }

        let record = self.record_witnessed_submission(head_key, ingress_id)?;
        Ok(IntentSubmissionDisposition::Accepted {
            ingress_id,
            head_key,
            submission_id: record.submission_id,
            submission_generation: record.submission_generation,
        })
    }

    /// Stages a witnessed submission into runtime ingress using an admission ticket.
    ///
    /// The ticket opens the runtime ingress boundary only. This method does not
    /// tick, dispatch handlers, execute contracts, correlate receipts, or
    /// observe outcomes.
    ///
    /// # Errors
    ///
    /// Returns an error when the submission is unknown, the envelope does not
    /// match the witnessed submission, the target rejects the envelope, the
    /// runtime ingress already exists through another path, or a different
    /// ticket has already staged the same submission.
    pub fn ingest_ticketed_invocation(
        &mut self,
        _authority: &TicketedRuntimeIngressAuthority,
        submission_id: Hash,
        ticket: &OpticAdmissionTicket,
        envelope: IngressEnvelope,
    ) -> Result<TicketedRuntimeIngressDisposition, RuntimeError> {
        let Some(submission) = self.witnessed_submissions.get(&submission_id) else {
            return Err(RuntimeError::UnknownIntentSubmission(submission_id));
        };
        let ingress_id = envelope.ingress_id();
        let head_key = self.resolve_target(envelope.target())?;
        if submission.ingress_id != ingress_id || submission.head_key != head_key {
            return Err(RuntimeError::TicketedIngressSubmissionMismatch(
                submission_id,
            ));
        }

        let ticketed_ingress_id = derive_ticketed_runtime_ingress_id(
            submission_id,
            ticket.ticket_digest,
            ingress_id,
            head_key,
        );
        if let Some(existing_id) = self
            .ticketed_runtime_ingress_by_submission
            .get(&submission_id)
            .copied()
        {
            let Some(record) = self.ticketed_runtime_ingress.get(&existing_id).cloned() else {
                return Err(RuntimeError::TicketedIngressAlreadyStaged(submission_id));
            };
            if existing_id == ticketed_ingress_id {
                return Ok(TicketedRuntimeIngressDisposition::Duplicate { record });
            }
            return Err(RuntimeError::TicketedIngressAlreadyStaged(submission_id));
        }

        let ingress = self.ingest(envelope)?;
        if !matches!(ingress, IngressDisposition::Accepted { .. }) {
            return Err(RuntimeError::TicketedIngressDuplicateRuntimeIngress {
                head_key,
                ingress_id,
            });
        }
        let record = TicketedRuntimeIngressRecord {
            ticketed_ingress_id,
            submission_id,
            ticket_digest: ticket.ticket_digest,
            ingress_id,
            head_key,
        };
        self.ticketed_runtime_ingress
            .insert(ticketed_ingress_id, record.clone());
        self.ticketed_runtime_ingress_by_submission
            .insert(submission_id, ticketed_ingress_id);
        self.ticketed_runtime_ingress_by_target
            .insert((head_key, ingress_id), ticketed_ingress_id);
        Ok(TicketedRuntimeIngressDisposition::Staged { record, ingress })
    }

    /// Stages a witnessed installed-contract mutation invocation into runtime ingress.
    ///
    /// This is the package boundary between lawful admission evidence and the
    /// scheduler-owned runtime path. It verifies that the canonical EINT mutation
    /// operation id is supported by an installed contract package before the work
    /// becomes runtime-visible. The method does not tick, dispatch handlers, or
    /// execute contracts.
    ///
    /// # Errors
    ///
    /// Returns an error when the envelope is not a canonical EINT local intent,
    /// no installed contract package supports its mutation operation id, or the
    /// underlying ticketed ingress boundary rejects the submission.
    #[cfg(feature = "native_rule_bootstrap")]
    pub fn ingest_installed_contract_invocation(
        &mut self,
        authority: &TicketedRuntimeIngressAuthority,
        engine: &Engine,
        submission_id: Hash,
        ticket: &OpticAdmissionTicket,
        envelope: IngressEnvelope,
    ) -> Result<TicketedRuntimeIngressDisposition, RuntimeError> {
        let op_id = installed_contract_mutation_op_id(&envelope)?;
        if engine
            .installed_contract_mutation_package_id(op_id)
            .is_none()
        {
            return Err(RuntimeError::UnsupportedInstalledContractMutation { op_id });
        }

        self.ingest_ticketed_invocation(authority, submission_id, ticket, envelope)
    }

    /// Resolves an ingress envelope to a specific writer head and stores it in that inbox.
    ///
    /// # Errors
    ///
    /// Returns an error if the routing target does not resolve or if the target
    /// head rejects the envelope under its inbox policy.
    pub fn ingest(
        &mut self,
        envelope: IngressEnvelope,
    ) -> Result<IngressDisposition, RuntimeError> {
        let ingress_id = envelope.ingress_id();
        let head_key = self.resolve_target(envelope.target())?;

        if self
            .worldlines
            .get(&head_key.worldline_id)
            .is_some_and(|frontier| {
                frontier
                    .state()
                    .contains_committed_ingress(&head_key, &ingress_id)
            })
        {
            let record = self.duplicate_submission_record(head_key, ingress_id);
            return Ok(IngressDisposition::Duplicate {
                ingress_id,
                head_key,
                submission_id: record.submission_id,
                submission_generation: record.submission_generation,
            });
        }

        let outcome = self
            .heads
            .inbox_mut(&head_key)
            .ok_or(RuntimeError::UnknownHead(head_key))?
            .ingest(envelope);

        match outcome {
            InboxIngestResult::Accepted => {
                let record = self.record_witnessed_submission(head_key, ingress_id)?;
                Ok(IngressDisposition::Accepted {
                    ingress_id,
                    head_key,
                    submission_id: record.submission_id,
                    submission_generation: record.submission_generation,
                })
            }
            InboxIngestResult::Duplicate => {
                let record = self.duplicate_submission_record(head_key, ingress_id);
                Ok(IngressDisposition::Duplicate {
                    ingress_id,
                    head_key,
                    submission_id: record.submission_id,
                    submission_generation: record.submission_generation,
                })
            }
            InboxIngestResult::Rejected => Err(RuntimeError::RejectedByPolicy(head_key)),
        }
    }

    fn record_witnessed_submission(
        &mut self,
        head_key: WriterHeadKey,
        ingress_id: Hash,
    ) -> Result<IntentSubmissionRecord, RuntimeError> {
        if let Some(record) = self
            .submission_by_target
            .get(&(head_key, ingress_id))
            .and_then(|submission_id| self.witnessed_submissions.get(submission_id))
        {
            return Ok(record.clone());
        }

        let generation = self
            .next_submission_generation
            .checked_increment()
            .ok_or(RuntimeError::IntentSubmissionGenerationOverflow)?;
        let submission_id = derive_intent_submission_id(head_key, ingress_id);
        let record = IntentSubmissionRecord {
            submission_id,
            ingress_id,
            head_key,
            submission_generation: generation,
        };
        self.next_submission_generation = generation;
        self.submission_by_target
            .insert((head_key, ingress_id), submission_id);
        self.witnessed_submissions
            .insert(submission_id, record.clone());
        Ok(record)
    }

    fn duplicate_submission_record(
        &self,
        head_key: WriterHeadKey,
        ingress_id: Hash,
    ) -> IntentSubmissionRecord {
        if let Some(record) = self
            .submission_by_target
            .get(&(head_key, ingress_id))
            .and_then(|submission_id| self.witnessed_submissions.get(submission_id))
        {
            return record.clone();
        }

        IntentSubmissionRecord {
            submission_id: derive_intent_submission_id(head_key, ingress_id),
            ingress_id,
            head_key,
            submission_generation: IngressSubmissionGeneration::ZERO,
        }
    }

    fn record_scheduler_head_fault(
        &mut self,
        run_id: SchedulerRunId,
        head_key: WriterHeadKey,
        cause_digest: Hash,
    ) -> Result<SchedulerFaultId, RuntimeError> {
        if let Some(existing) = self.faulted_heads.get(&head_key).copied() {
            return Ok(existing);
        }
        let scope = SchedulerFaultScope::Head(head_key);
        let (fault_generation, fault_id) =
            self.allocate_scheduler_fault_identity(run_id, scope, cause_digest)?;
        let record = SchedulerFaultRecord {
            fault_id,
            fault_generation,
            run_id,
            scope,
            cause_digest,
            status: SchedulerFaultStatus::Active,
        };
        self.scheduler_faults.insert(fault_id, record);
        self.faulted_heads.insert(head_key, fault_id);
        self.refresh_runnable();
        Ok(fault_id)
    }

    fn record_scheduler_runtime_fault(
        &mut self,
        run_id: SchedulerRunId,
        cause_digest: Hash,
    ) -> Result<SchedulerFaultId, RuntimeError> {
        if let Some(existing) = self.runtime_fault {
            return Ok(existing);
        }
        let scope = SchedulerFaultScope::Runtime;
        let (fault_generation, fault_id) =
            self.allocate_scheduler_fault_identity(run_id, scope, cause_digest)?;
        let record = SchedulerFaultRecord {
            fault_id,
            fault_generation,
            run_id,
            scope,
            cause_digest,
            status: SchedulerFaultStatus::Active,
        };
        self.scheduler_faults.insert(fault_id, record);
        self.runtime_fault = Some(fault_id);
        self.refresh_runnable();
        Ok(fault_id)
    }

    fn allocate_scheduler_fault_identity(
        &mut self,
        run_id: SchedulerRunId,
        scope: SchedulerFaultScope,
        cause_digest: Hash,
    ) -> Result<(SchedulerFaultGeneration, SchedulerFaultId), RuntimeError> {
        loop {
            let fault_generation = self
                .next_scheduler_fault_generation
                .checked_increment()
                .ok_or(RuntimeError::SchedulerFaultGenerationOverflow)?;
            self.next_scheduler_fault_generation = fault_generation;
            let fault_id = derive_scheduler_fault_id(fault_generation, run_id, scope, cause_digest);
            if !self.scheduler_faults.contains_key(&fault_id) {
                return Ok((fault_generation, fault_id));
            }
        }
    }

    fn record_receipt_correlations(
        &mut self,
        admitted: &[IngressEnvelope],
        context: ReceiptCorrelationCommitContext,
        rollback: &mut ReceiptCorrelationRollback,
    ) {
        for envelope in admitted {
            let ingress_id = envelope.ingress_id();
            let Some(ticketed_ingress_id) = self
                .ticketed_runtime_ingress_by_target
                .get(&(context.head_key, ingress_id))
                .copied()
            else {
                continue;
            };
            if self
                .receipt_correlations_by_ticketed_ingress
                .contains_key(&ticketed_ingress_id)
            {
                continue;
            }
            let Some(ticketed_ingress) = self
                .ticketed_runtime_ingress
                .get(&ticketed_ingress_id)
                .cloned()
            else {
                continue;
            };
            let record = ReceiptCorrelationRecord {
                ticketed_ingress_id,
                submission_id: ticketed_ingress.submission_id,
                ticket_digest: ticketed_ingress.ticket_digest,
                ingress_id,
                head_key: context.head_key,
                commit_global_tick: context.commit_global_tick,
                worldline_tick_after: context.worldline_tick_after,
                tick_receipt_digest: context.tick_receipt_digest,
                commit_hash: context.commit_hash,
            };
            rollback.entries.push(ReceiptCorrelationRollbackEntry {
                ticketed_ingress_id,
                previous_record: self
                    .receipt_correlations_by_ticketed_ingress
                    .get(&ticketed_ingress_id)
                    .cloned(),
                submission_id: ticketed_ingress.submission_id,
                previous_submission_ticketed_ingress: self
                    .receipt_correlation_by_submission
                    .get(&ticketed_ingress.submission_id)
                    .copied(),
                ticket_digest: ticketed_ingress.ticket_digest,
                previous_ticket_ticketed_ingress: self
                    .receipt_correlation_by_ticket
                    .get(&ticketed_ingress.ticket_digest)
                    .copied(),
            });
            self.receipt_correlations_by_ticketed_ingress
                .insert(ticketed_ingress_id, record);
            self.receipt_correlation_by_submission
                .insert(ticketed_ingress.submission_id, ticketed_ingress_id);
            self.receipt_correlation_by_ticket
                .insert(ticketed_ingress.ticket_digest, ticketed_ingress_id);
        }
    }

    fn resolve_target(&self, target: &IngressTarget) -> Result<WriterHeadKey, RuntimeError> {
        match target {
            IngressTarget::DefaultWriter { worldline_id } => self
                .default_writers
                .get(worldline_id)
                .copied()
                .ok_or(RuntimeError::MissingDefaultWriter(*worldline_id)),
            IngressTarget::InboxAddress {
                worldline_id,
                inbox,
            } => self
                .public_inboxes
                .get(worldline_id)
                .and_then(|routes| routes.get(inbox))
                .copied()
                .ok_or_else(|| RuntimeError::MissingInboxAddress {
                    worldline_id: *worldline_id,
                    inbox: inbox.clone(),
                }),
            IngressTarget::ExactHead { key } => self
                .heads
                .get(key)
                .map(|_| *key)
                .ok_or(RuntimeError::UnknownHead(*key)),
        }
    }
}

fn derive_intent_submission_id(head_key: WriterHeadKey, ingress_id: Hash) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo.intent-submission.v0");
    hasher.update(head_key.worldline_id.as_bytes());
    hasher.update(head_key.head_id.as_bytes());
    hasher.update(&ingress_id);
    hasher.finalize().into()
}

fn derive_ticketed_runtime_ingress_id(
    submission_id: Hash,
    ticket_digest: Hash,
    ingress_id: Hash,
    head_key: WriterHeadKey,
) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo.ticketed-runtime-ingress");
    hasher.update(&submission_id);
    hasher.update(&ticket_digest);
    hasher.update(head_key.worldline_id.as_bytes());
    hasher.update(head_key.head_id.as_bytes());
    hasher.update(&ingress_id);
    hasher.finalize().into()
}

#[cfg(feature = "native_rule_bootstrap")]
fn installed_contract_mutation_op_id(envelope: &IngressEnvelope) -> Result<u32, RuntimeError> {
    let IngressPayload::LocalIntent { intent_bytes, .. } = envelope.payload();
    echo_wasm_abi::unpack_intent_v1(intent_bytes)
        .map(|(op_id, _vars)| op_id)
        .map_err(|_error| RuntimeError::MalformedInstalledContractIntent)
}

fn derive_scheduler_run_id(next_global_tick: GlobalTick, keys: &[WriterHeadKey]) -> SchedulerRunId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo.scheduler-run");
    hasher.update(&next_global_tick.as_u64().to_le_bytes());
    hasher.update(&(keys.len() as u64).to_le_bytes());
    for key in keys {
        hasher.update(key.worldline_id.as_bytes());
        hasher.update(key.head_id.as_bytes());
    }
    SchedulerRunId::from_bytes(hasher.finalize().into())
}

fn derive_scheduler_fault_id(
    fault_generation: SchedulerFaultGeneration,
    run_id: SchedulerRunId,
    scope: SchedulerFaultScope,
    cause_digest: Hash,
) -> SchedulerFaultId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo.scheduler-fault");
    hasher.update(&fault_generation.as_u64().to_le_bytes());
    hasher.update(run_id.as_bytes());
    match scope {
        SchedulerFaultScope::Head(head_key) => {
            hasher.update(b"head");
            hasher.update(head_key.worldline_id.as_bytes());
            hasher.update(head_key.head_id.as_bytes());
        }
        SchedulerFaultScope::Runtime => {
            hasher.update(b"runtime");
        }
    }
    hasher.update(&cause_digest);
    SchedulerFaultId::from_bytes(hasher.finalize().into())
}

fn scheduler_fault_scope_for_error(
    head_key: WriterHeadKey,
    err: &RuntimeError,
) -> SchedulerFaultScope {
    match err {
        RuntimeError::Engine(_) | RuntimeError::FrontierTickOverflow(_) => {
            SchedulerFaultScope::Head(head_key)
        }
        RuntimeError::Provenance(_)
        | RuntimeError::UnknownHead(_)
        | RuntimeError::UnknownWorldline(_)
        | RuntimeError::GlobalTickOverflow
        | RuntimeError::SchedulerRuntimeFaultActive(_)
        | RuntimeError::UnknownSchedulerFault(_)
        | RuntimeError::SchedulerFaultAlreadyResolved(_)
        | RuntimeError::SchedulerFaultGenerationOverflow => SchedulerFaultScope::Runtime,
        RuntimeError::DuplicateWorldline(_)
        | RuntimeError::DuplicateHead(_)
        | RuntimeError::DuplicateDefaultWriter(_)
        | RuntimeError::DuplicateInboxAddress { .. }
        | RuntimeError::MissingDefaultWriter(_)
        | RuntimeError::MissingInboxAddress { .. }
        | RuntimeError::RejectedByPolicy(_)
        | RuntimeError::Replay(_)
        | RuntimeError::Strand(_)
        | RuntimeError::IntentSubmissionGenerationOverflow
        | RuntimeError::UnknownIntentSubmission(_)
        | RuntimeError::TicketedIngressSubmissionMismatch(_)
        | RuntimeError::TicketedIngressAlreadyStaged(_)
        | RuntimeError::MalformedInstalledContractIntent
        | RuntimeError::UnsupportedInstalledContractMutation { .. }
        | RuntimeError::TicketedIngressDuplicateRuntimeIngress { .. } => {
            SchedulerFaultScope::Runtime
        }
    }
}

fn hash_worldline_tick(hasher: &mut blake3::Hasher, tick: WorldlineTick) {
    hasher.update(&tick.as_u64().to_le_bytes());
}

fn hash_worldline_id(hasher: &mut blake3::Hasher, worldline_id: &WorldlineId) {
    hasher.update(worldline_id.as_bytes());
}

fn hash_writer_head_key(hasher: &mut blake3::Hasher, head_key: &WriterHeadKey) {
    hasher.update(head_key.worldline_id.as_bytes());
    hasher.update(head_key.head_id.as_bytes());
}

fn hash_node_key(hasher: &mut blake3::Hasher, node: &crate::NodeKey) {
    hasher.update(node.warp_id.as_bytes());
    hasher.update(node.local_id.as_bytes());
}

fn hash_edge_key(hasher: &mut blake3::Hasher, edge: &crate::EdgeKey) {
    hasher.update(edge.warp_id.as_bytes());
    hasher.update(edge.local_id.as_bytes());
}

fn hash_provenance_ref(hasher: &mut blake3::Hasher, reference: &ProvenanceRef) {
    hash_worldline_id(hasher, &reference.worldline_id);
    hash_worldline_tick(hasher, reference.worldline_tick);
    hasher.update(&reference.commit_hash);
}

fn hash_provenance_event_kind(hasher: &mut blake3::Hasher, event_kind: &ProvenanceEventKind) {
    match event_kind {
        ProvenanceEventKind::LocalCommit => {
            hasher.update(b"local-commit");
        }
        ProvenanceEventKind::CrossWorldlineMessage {
            source_worldline,
            source_worldline_tick,
            message_id,
        } => {
            hasher.update(b"cross-worldline-message");
            hash_worldline_id(hasher, source_worldline);
            hash_worldline_tick(hasher, *source_worldline_tick);
            hasher.update(message_id);
        }
        ProvenanceEventKind::MergeImport {
            source_worldline,
            source_worldline_tick,
            op_id,
        } => {
            hasher.update(b"merge-import");
            hash_worldline_id(hasher, source_worldline);
            hash_worldline_tick(hasher, *source_worldline_tick);
            hasher.update(op_id);
        }
        ProvenanceEventKind::ConflictArtifact { artifact_id } => {
            hasher.update(b"conflict-artifact");
            hasher.update(artifact_id);
        }
    }
}

fn hash_history_error(hasher: &mut blake3::Hasher, err: &HistoryError) {
    match err {
        HistoryError::HistoryUnavailable { tick } => {
            hasher.update(b"history-unavailable");
            hash_worldline_tick(hasher, *tick);
        }
        HistoryError::WorldlineNotFound(worldline_id) => {
            hasher.update(b"worldline-not-found");
            hash_worldline_id(hasher, worldline_id);
        }
        HistoryError::WorldlineAlreadyExists(worldline_id) => {
            hasher.update(b"worldline-already-exists");
            hash_worldline_id(hasher, worldline_id);
        }
        HistoryError::TickGap { expected, got } => {
            hasher.update(b"tick-gap");
            hash_worldline_tick(hasher, *expected);
            hash_worldline_tick(hasher, *got);
        }
        HistoryError::EntryWorldlineMismatch { expected, got } => {
            hasher.update(b"entry-worldline-mismatch");
            hash_worldline_id(hasher, expected);
            hash_worldline_id(hasher, got);
        }
        HistoryError::LocalCommitMissingHeadKey { tick } => {
            hasher.update(b"local-commit-missing-head-key");
            hash_worldline_tick(hasher, *tick);
        }
        HistoryError::LocalCommitMissingPatch { tick } => {
            hasher.update(b"local-commit-missing-patch");
            hash_worldline_tick(hasher, *tick);
        }
        HistoryError::HeadWorldlineMismatch {
            entry_worldline,
            head_key,
        } => {
            hasher.update(b"head-worldline-mismatch");
            hash_worldline_id(hasher, entry_worldline);
            hash_writer_head_key(hasher, head_key);
        }
        HistoryError::InvalidLocalCommitEventKind { tick, got } => {
            hasher.update(b"invalid-local-commit-event-kind");
            hash_worldline_tick(hasher, *tick);
            hash_provenance_event_kind(hasher, got);
        }
        HistoryError::RecordedEventUnexpectedHeadKey { tick } => {
            hasher.update(b"recorded-event-unexpected-head-key");
            hash_worldline_tick(hasher, *tick);
        }
        HistoryError::RecordedEventMissingPatch { tick } => {
            hasher.update(b"recorded-event-missing-patch");
            hash_worldline_tick(hasher, *tick);
        }
        HistoryError::InvalidRecordedEventKind { tick, got } => {
            hasher.update(b"invalid-recorded-event-kind");
            hash_worldline_tick(hasher, *tick);
            hash_provenance_event_kind(hasher, got);
        }
        HistoryError::NonCanonicalParents { tick } => {
            hasher.update(b"non-canonical-parents");
            hash_worldline_tick(hasher, *tick);
        }
        HistoryError::MissingParentRef { tick, parent } => {
            hasher.update(b"missing-parent-ref");
            hash_worldline_tick(hasher, *tick);
            hash_provenance_ref(hasher, parent);
        }
        HistoryError::ParentCommitHashMismatch {
            tick,
            parent,
            stored_commit_hash,
        } => {
            hasher.update(b"parent-commit-hash-mismatch");
            hash_worldline_tick(hasher, *tick);
            hash_provenance_ref(hasher, parent);
            hasher.update(stored_commit_hash);
        }
        HistoryError::CheckpointRootWarpMismatch {
            worldline_id,
            expected,
            actual,
        } => {
            hasher.update(b"checkpoint-root-warp-mismatch");
            hash_worldline_id(hasher, worldline_id);
            hasher.update(expected.as_bytes());
            hasher.update(actual.as_bytes());
        }
        HistoryError::CheckpointInitialBoundaryHashMismatch {
            worldline_id,
            expected,
            actual,
        } => {
            hasher.update(b"checkpoint-initial-boundary-hash-mismatch");
            hash_worldline_id(hasher, worldline_id);
            hasher.update(expected);
            hasher.update(actual);
        }
        HistoryError::CheckpointStateRootMismatch {
            tick,
            expected,
            actual,
        } => {
            hasher.update(b"checkpoint-state-root-mismatch");
            hash_worldline_tick(hasher, *tick);
            hasher.update(expected);
            hasher.update(actual);
        }
        HistoryError::CheckpointReplayMetadataMismatch { tick, field } => {
            hasher.update(b"checkpoint-replay-metadata-mismatch");
            hash_worldline_tick(hasher, *tick);
            hasher.update(field.as_bytes());
        }
    }
}

fn hash_attachment_key(hasher: &mut blake3::Hasher, key: &crate::attachment::AttachmentKey) {
    match key.owner {
        crate::attachment::AttachmentOwner::Node(node) => {
            hasher.update(b"node");
            hash_node_key(hasher, &node);
        }
        crate::attachment::AttachmentOwner::Edge(edge) => {
            hasher.update(b"edge");
            hash_edge_key(hasher, &edge);
        }
    }
    match key.plane {
        crate::attachment::AttachmentPlane::Alpha => hasher.update(b"alpha"),
        crate::attachment::AttachmentPlane::Beta => hasher.update(b"beta"),
    };
}

fn hash_tick_patch_error(hasher: &mut blake3::Hasher, err: &crate::tick_patch::TickPatchError) {
    match err {
        crate::tick_patch::TickPatchError::MissingWarp(warp_id) => {
            hasher.update(b"missing-warp");
            hasher.update(warp_id.as_bytes());
        }
        crate::tick_patch::TickPatchError::MissingNode(node) => {
            hasher.update(b"missing-node");
            hash_node_key(hasher, node);
        }
        crate::tick_patch::TickPatchError::MissingEdge(edge) => {
            hasher.update(b"missing-edge");
            hash_edge_key(hasher, edge);
        }
        crate::tick_patch::TickPatchError::NodeNotIsolated(node) => {
            hasher.update(b"node-not-isolated");
            hash_node_key(hasher, node);
        }
        crate::tick_patch::TickPatchError::InvalidAttachmentKey(key) => {
            hasher.update(b"invalid-attachment-key");
            hash_attachment_key(hasher, key);
        }
        crate::tick_patch::TickPatchError::PortalInitRequired => {
            hasher.update(b"portal-init-required");
        }
        crate::tick_patch::TickPatchError::PortalInvariantViolation => {
            hasher.update(b"portal-invariant-violation");
        }
        crate::tick_patch::TickPatchError::DigestMismatch => {
            hasher.update(b"digest-mismatch");
        }
    }
}

fn hash_apply_error(hasher: &mut blake3::Hasher, err: &ApplyError) {
    match err {
        ApplyError::MissingNode(node) => {
            hasher.update(b"missing-node");
            hash_node_key(hasher, node);
        }
        ApplyError::MissingEdge(edge) => {
            hasher.update(b"missing-edge");
            hash_edge_key(hasher, edge);
        }
        ApplyError::WarpMismatch { expected, actual } => {
            hasher.update(b"warp-mismatch");
            hasher.update(expected.as_bytes());
            hasher.update(actual.as_bytes());
        }
        ApplyError::InvalidAttachmentKey => {
            hasher.update(b"invalid-attachment-key");
        }
        ApplyError::NodeNotIsolated(node) => {
            hasher.update(b"node-not-isolated");
            hash_node_key(hasher, node);
        }
        ApplyError::TickPatch(err) => {
            hasher.update(b"tick-patch");
            hash_tick_patch_error(hasher, err);
        }
    }
}

fn hash_replay_error(hasher: &mut blake3::Hasher, err: &ReplayError) {
    match err {
        ReplayError::History(err) => {
            hasher.update(b"history");
            hash_history_error(hasher, err);
        }
        ReplayError::Apply { tick, source } => {
            hasher.update(b"apply");
            hash_worldline_tick(hasher, *tick);
            hash_apply_error(hasher, source);
        }
        ReplayError::CheckpointStateRootMismatch {
            tick,
            expected,
            actual,
        } => {
            hasher.update(b"checkpoint-state-root-mismatch");
            hash_worldline_tick(hasher, *tick);
            hasher.update(expected);
            hasher.update(actual);
        }
        ReplayError::ReplayBaseWarpMismatch { expected, actual } => {
            hasher.update(b"replay-base-warp-mismatch");
            hasher.update(expected.as_bytes());
            hasher.update(actual.as_bytes());
        }
        ReplayError::InitialBoundaryHashMismatch { expected, actual } => {
            hasher.update(b"initial-boundary-hash-mismatch");
            hasher.update(expected);
            hasher.update(actual);
        }
        ReplayError::MissingPatch { tick } => {
            hasher.update(b"missing-patch");
            hash_worldline_tick(hasher, *tick);
        }
        ReplayError::TickOverflow { tick } => {
            hasher.update(b"tick-overflow");
            hash_worldline_tick(hasher, *tick);
        }
        ReplayError::PatchDigestMismatch {
            tick,
            expected,
            actual,
        } => {
            hasher.update(b"patch-digest-mismatch");
            hash_worldline_tick(hasher, *tick);
            hasher.update(expected);
            hasher.update(actual);
        }
        ReplayError::StateRootMismatch {
            tick,
            expected,
            actual,
        } => {
            hasher.update(b"state-root-mismatch");
            hash_worldline_tick(hasher, *tick);
            hasher.update(expected);
            hasher.update(actual);
        }
        ReplayError::CommitHashMismatch {
            tick,
            expected,
            actual,
        } => {
            hasher.update(b"commit-hash-mismatch");
            hash_worldline_tick(hasher, *tick);
            hasher.update(expected);
            hasher.update(actual);
        }
    }
}

fn hash_strand_error(hasher: &mut blake3::Hasher, err: &StrandError) {
    match err {
        StrandError::AlreadyExists(strand_id) => {
            hasher.update(b"already-exists");
            hasher.update(strand_id.as_bytes());
        }
        StrandError::NotFound(strand_id) => {
            hasher.update(b"not-found");
            hasher.update(strand_id.as_bytes());
        }
        StrandError::ForkTickUnavailable { worldline, tick } => {
            hasher.update(b"fork-tick-unavailable");
            hash_worldline_id(hasher, worldline);
            hash_worldline_tick(hasher, *tick);
        }
        StrandError::SourceWorldlineNotFound(worldline_id) => {
            hasher.update(b"source-worldline-not-found");
            hash_worldline_id(hasher, worldline_id);
        }
        StrandError::Provenance(message) => {
            hasher.update(b"provenance");
            hasher.update(message.as_bytes());
        }
        StrandError::ForkTickOverflow(strand_id) => {
            hasher.update(b"fork-tick-overflow");
            hasher.update(strand_id.as_bytes());
        }
        StrandError::MissingSupportTarget(strand_id) => {
            hasher.update(b"missing-support-target");
            hasher.update(strand_id.as_bytes());
        }
        StrandError::SupportWorldlineMismatch {
            target,
            expected,
            got,
        } => {
            hasher.update(b"support-worldline-mismatch");
            hasher.update(target.as_bytes());
            hash_worldline_id(hasher, expected);
            hash_worldline_id(hasher, got);
        }
        StrandError::SelfSupportPin(strand_id) => {
            hasher.update(b"self-support-pin");
            hasher.update(strand_id.as_bytes());
        }
        StrandError::DuplicateSupportTarget { owner, target } => {
            hasher.update(b"duplicate-support-target");
            hasher.update(owner.as_bytes());
            hasher.update(target.as_bytes());
        }
        StrandError::SupportPinUnavailable { target, tick } => {
            hasher.update(b"support-pin-unavailable");
            hasher.update(target.as_bytes());
            hash_worldline_tick(hasher, *tick);
        }
        StrandError::PinnedByLiveStrand { strand, pinned_by } => {
            hasher.update(b"pinned-by-live-strand");
            hasher.update(strand.as_bytes());
            hasher.update(pinned_by.as_bytes());
        }
        StrandError::InvariantViolation(message) => {
            hasher.update(b"invariant-violation");
            hasher.update(message.as_bytes());
        }
    }
}

fn scheduler_error_cause_digest(err: &RuntimeError) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo.scheduler-fault-cause.error");
    match err {
        RuntimeError::Engine(engine_err) => {
            hasher.update(b"engine");
            match engine_err {
                EngineError::UnknownTx => {
                    hasher.update(b"unknown-tx");
                }
                EngineError::UnknownRule(rule) => {
                    hasher.update(b"unknown-rule");
                    hasher.update(rule.as_bytes());
                }
                EngineError::DuplicateRuleName(rule) => {
                    hasher.update(b"duplicate-rule-name");
                    hasher.update(rule.as_bytes());
                }
                EngineError::DuplicateRuleId(rule_id) => {
                    hasher.update(b"duplicate-rule-id");
                    hasher.update(rule_id);
                }
                EngineError::DuplicateContractQueryObserver(query_id) => {
                    hasher.update(b"duplicate-contract-query-observer");
                    hasher.update(&query_id.to_le_bytes());
                }
                EngineError::MissingJoinFn => {
                    hasher.update(b"missing-join-fn");
                }
                EngineError::InternalCorruption(message) => {
                    hasher.update(b"internal-corruption");
                    hasher.update(message.as_bytes());
                }
                EngineError::UnknownWarp(warp_id) => {
                    hasher.update(b"unknown-warp");
                    hasher.update(&warp_id.0);
                }
                EngineError::InvalidTickIndex(index, len) => {
                    hasher.update(b"invalid-tick-index");
                    hasher.update(&(*index as u64).to_le_bytes());
                    hasher.update(&(*len as u64).to_le_bytes());
                }
            }
        }
        RuntimeError::FrontierTickOverflow(worldline_id) => {
            hasher.update(b"frontier-tick-overflow");
            hasher.update(worldline_id.as_bytes());
        }
        RuntimeError::GlobalTickOverflow => {
            hasher.update(b"global-tick-overflow");
        }
        RuntimeError::DuplicateWorldline(worldline_id) => {
            hasher.update(b"duplicate-worldline");
            hash_worldline_id(&mut hasher, worldline_id);
        }
        RuntimeError::DuplicateHead(head_key) => {
            hasher.update(b"duplicate-head");
            hash_writer_head_key(&mut hasher, head_key);
        }
        RuntimeError::UnknownWorldline(worldline_id) => {
            hasher.update(b"unknown-worldline");
            hash_worldline_id(&mut hasher, worldline_id);
        }
        RuntimeError::UnknownHead(head_key) => {
            hasher.update(b"unknown-head");
            hash_writer_head_key(&mut hasher, head_key);
        }
        RuntimeError::DuplicateDefaultWriter(worldline_id) => {
            hasher.update(b"duplicate-default-writer");
            hash_worldline_id(&mut hasher, worldline_id);
        }
        RuntimeError::DuplicateInboxAddress {
            worldline_id,
            inbox,
        } => {
            hasher.update(b"duplicate-inbox-address");
            hash_worldline_id(&mut hasher, worldline_id);
            hasher.update(inbox.0.as_bytes());
        }
        RuntimeError::MissingDefaultWriter(worldline_id) => {
            hasher.update(b"missing-default-writer");
            hash_worldline_id(&mut hasher, worldline_id);
        }
        RuntimeError::MissingInboxAddress {
            worldline_id,
            inbox,
        } => {
            hasher.update(b"missing-inbox-address");
            hash_worldline_id(&mut hasher, worldline_id);
            hasher.update(inbox.0.as_bytes());
        }
        RuntimeError::RejectedByPolicy(head_key) => {
            hasher.update(b"rejected-by-policy");
            hash_writer_head_key(&mut hasher, head_key);
        }
        RuntimeError::Provenance(err) => {
            hasher.update(b"provenance");
            hash_history_error(&mut hasher, err);
        }
        RuntimeError::Replay(err) => {
            hasher.update(b"replay");
            hash_replay_error(&mut hasher, err);
        }
        RuntimeError::Strand(err) => {
            hasher.update(b"strand");
            hash_strand_error(&mut hasher, err);
        }
        RuntimeError::IntentSubmissionGenerationOverflow => {
            hasher.update(b"intent-submission-generation-overflow");
        }
        RuntimeError::UnknownIntentSubmission(submission_id) => {
            hasher.update(b"unknown-intent-submission");
            hasher.update(submission_id);
        }
        RuntimeError::TicketedIngressSubmissionMismatch(submission_id) => {
            hasher.update(b"ticketed-ingress-submission-mismatch");
            hasher.update(submission_id);
        }
        RuntimeError::TicketedIngressAlreadyStaged(submission_id) => {
            hasher.update(b"ticketed-ingress-already-staged");
            hasher.update(submission_id);
        }
        RuntimeError::MalformedInstalledContractIntent => {
            hasher.update(b"malformed-installed-contract-intent");
        }
        RuntimeError::UnsupportedInstalledContractMutation { op_id } => {
            hasher.update(b"unsupported-installed-contract-mutation");
            hasher.update(&op_id.to_le_bytes());
        }
        RuntimeError::TicketedIngressDuplicateRuntimeIngress {
            head_key,
            ingress_id,
        } => {
            hasher.update(b"ticketed-ingress-duplicate-runtime-ingress");
            hash_writer_head_key(&mut hasher, head_key);
            hasher.update(ingress_id);
        }
        RuntimeError::SchedulerRuntimeFaultActive(fault_id) => {
            hasher.update(b"scheduler-runtime-fault-active");
            hasher.update(fault_id.as_bytes());
        }
        RuntimeError::UnknownSchedulerFault(fault_id) => {
            hasher.update(b"unknown-scheduler-fault");
            hasher.update(fault_id.as_bytes());
        }
        RuntimeError::SchedulerFaultAlreadyResolved(fault_id) => {
            hasher.update(b"scheduler-fault-already-resolved");
            hasher.update(fault_id.as_bytes());
        }
        RuntimeError::SchedulerFaultGenerationOverflow => {
            hasher.update(b"scheduler-fault-generation-overflow");
        }
    }
    hasher.finalize().into()
}

fn scheduler_panic_cause_digest(payload: &(dyn std::any::Any + Send)) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo.scheduler-fault-cause.panic");
    if let Some(message) = payload.downcast_ref::<&str>() {
        hasher.update(b"str");
        hasher.update(message.as_bytes());
    } else if let Some(message) = payload.downcast_ref::<String>() {
        hasher.update(b"string");
        hasher.update(message.as_bytes());
    } else {
        hasher.update(b"opaque");
    }
    hasher.finalize().into()
}

// =============================================================================
// StepRecord
// =============================================================================

/// Record of a single head commit during a SuperTick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepRecord {
    /// The head that was stepped.
    pub head_key: WriterHeadKey,
    /// Number of ingress envelopes admitted for this commit.
    pub admitted_count: usize,
    /// The worldline tick after this step.
    pub worldline_tick_after: WorldlineTick,
    /// Runtime cycle stamp that produced this commit.
    pub commit_global_tick: GlobalTick,
    /// Resulting graph state root after the commit.
    pub state_root: Hash,
    /// Resulting commit hash after the commit.
    pub commit_hash: Hash,
}

// =============================================================================
// SchedulerCoordinator
// =============================================================================

/// Coordinator for worldline-aware serial canonical scheduling.
pub struct SchedulerCoordinator;

impl SchedulerCoordinator {
    /// Executes one SuperTick: admits inbox work in canonical head order and
    /// commits each non-empty head against its worldline frontier.
    ///
    /// In the admission-law vocabulary, `super_tick()` is the single place
    /// where Echo judges ingress claims at bounded sites under engine-defined
    /// deterministic policy. The current runtime still derives bounded sites
    /// from rule footprints and commits policy identity through the emitted
    /// patch/shell family, but it does not introduce a second execution model.
    ///
    /// The SuperTick is failure-atomic with respect to runtime state: if any
    /// head commit fails, all prior runtime and provenance mutations from this
    /// pass are discarded and both subsystems are restored to their
    /// pre-SuperTick state.
    ///
    /// # Panics
    ///
    /// Re-raises any panic from rule execution after restoring the runtime to
    /// its pre-SuperTick state.
    pub fn super_tick(
        runtime: &mut WorldlineRuntime,
        provenance: &mut ProvenanceService,
        engine: &mut Engine,
    ) -> Result<Vec<StepRecord>, RuntimeError> {
        if let Some(fault_id) = runtime.runtime_fault {
            return Err(RuntimeError::SchedulerRuntimeFaultActive(fault_id));
        }
        runtime.refresh_runnable();

        let mut records = Vec::new();
        let keys: Vec<WriterHeadKey> = runtime.runnable.iter().copied().collect();
        let next_global_tick = if let Some(next) = runtime.global_tick.checked_increment() {
            next
        } else {
            let run_id = derive_scheduler_run_id(runtime.global_tick, &keys);
            let cause_digest = scheduler_error_cause_digest(&RuntimeError::GlobalTickOverflow);
            runtime.record_scheduler_runtime_fault(run_id, cause_digest)?;
            return Err(RuntimeError::GlobalTickOverflow);
        };
        let run_id = derive_scheduler_run_id(next_global_tick, &keys);

        for key in &keys {
            let head = runtime
                .heads
                .get(key)
                .ok_or(RuntimeError::UnknownHead(*key))?;
            if !head.inbox().can_admit() {
                continue;
            }

            let frontier = runtime
                .worldlines
                .get(&key.worldline_id)
                .ok_or(RuntimeError::UnknownWorldline(key.worldline_id))?;
            if frontier.frontier_tick() == WorldlineTick::MAX {
                let err = RuntimeError::FrontierTickOverflow(key.worldline_id);
                let cause_digest = scheduler_error_cause_digest(&err);
                runtime.record_scheduler_head_fault(run_id, *key, cause_digest)?;
                return Err(err);
            }
        }

        let runtime_before = runtime.checkpoint_for(&keys)?;
        let mut receipt_correlation_rollback = ReceiptCorrelationRollback::default();
        let provenance_before: ProvenanceCheckpoint =
            provenance.checkpoint_for(keys.iter().map(|key| key.worldline_id))?;

        for key in &keys {
            let admitted = runtime
                .heads
                .inbox_mut(key)
                .ok_or(RuntimeError::UnknownHead(*key))?
                .admit();

            if admitted.is_empty() {
                continue;
            }

            let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<StepRecord, RuntimeError> {
                let worldline_tick = runtime
                    .worldlines
                    .get(&key.worldline_id)
                    .ok_or(RuntimeError::UnknownWorldline(key.worldline_id))?
                    .frontier_tick();
                let parents = provenance.tip_ref(key.worldline_id)?.into_iter().collect();

                let CommitOutcome {
                    snapshot,
                    patch,
                    receipt,
                } = {
                    let frontier = runtime
                        .worldlines
                        .frontier_mut(&key.worldline_id)
                        .ok_or(RuntimeError::UnknownWorldline(key.worldline_id))?;
                    engine
                        .commit_with_state(frontier.state_mut(), &admitted)
                        .map_err(RuntimeError::from)?
                };
                let tick_receipt_digest = receipt.digest();

                let (state_root, worldline_tick_after) = {
                    let frontier = runtime
                        .worldlines
                        .frontier_mut(&key.worldline_id)
                        .ok_or(RuntimeError::UnknownWorldline(key.worldline_id))?;
                    let outputs = frontier
                        .state()
                        .last_materialization()
                        .iter()
                        .map(|channel| (channel.channel, channel.data.clone()))
                        .collect();
                    let worldline_patch = crate::worldline::WorldlineTickPatchV1 {
                        header: crate::worldline::WorldlineTickHeaderV1 {
                            commit_global_tick: next_global_tick,
                            policy_id: patch.policy_id(),
                            rule_pack_id: patch.rule_pack_id(),
                            plan_digest: snapshot.plan_digest,
                            decision_digest: snapshot.decision_digest,
                            rewrites_digest: snapshot.rewrites_digest,
                        },
                        warp_id: snapshot.root.warp_id,
                        ops: patch.ops().to_vec(),
                        in_slots: patch.in_slots().to_vec(),
                        out_slots: patch.out_slots().to_vec(),
                        patch_digest: patch.digest(),
                    };
                    let entry = ProvenanceEntry::local_commit(
                        key.worldline_id,
                        worldline_tick,
                        next_global_tick,
                        *key,
                        parents,
                        crate::worldline::HashTriplet {
                            state_root: snapshot.state_root,
                            patch_digest: snapshot.patch_digest,
                            commit_hash: snapshot.hash,
                        },
                        worldline_patch,
                        outputs,
                        Vec::new(),
                    );
                    provenance.append_local_commit(entry)?;
                    frontier.state_mut().record_committed_ingress(
                        *key,
                        admitted.iter().map(IngressEnvelope::ingress_id),
                    );
                    let worldline_tick_after = frontier
                        .advance_tick()
                        .ok_or(RuntimeError::FrontierTickOverflow(key.worldline_id))?;
                    (snapshot.state_root, worldline_tick_after)
                };
                runtime.record_receipt_correlations(
                    &admitted,
                    ReceiptCorrelationCommitContext {
                        head_key: *key,
                        commit_global_tick: next_global_tick,
                        worldline_tick_after,
                        tick_receipt_digest,
                        commit_hash: snapshot.hash,
                    },
                    &mut receipt_correlation_rollback,
                );

                Ok(StepRecord {
                    head_key: *key,
                    admitted_count: admitted.len(),
                    worldline_tick_after,
                    commit_global_tick: next_global_tick,
                    state_root,
                    commit_hash: snapshot.hash,
                })
            }));

            let record = match outcome {
                Ok(Ok(record)) => record,
                Ok(Err(err)) => {
                    let scope = scheduler_fault_scope_for_error(*key, &err);
                    let cause_digest = scheduler_error_cause_digest(&err);
                    runtime.rollback_receipt_correlations(&mut receipt_correlation_rollback);
                    runtime.restore(runtime_before);
                    provenance.restore(&provenance_before);
                    match scope {
                        SchedulerFaultScope::Head(head_key) => {
                            runtime.record_scheduler_head_fault(run_id, head_key, cause_digest)?;
                        }
                        SchedulerFaultScope::Runtime => {
                            runtime.record_scheduler_runtime_fault(run_id, cause_digest)?;
                        }
                    }
                    return Err(err);
                }
                Err(payload) => {
                    let cause_digest = scheduler_panic_cause_digest(payload.as_ref());
                    runtime.rollback_receipt_correlations(&mut receipt_correlation_rollback);
                    runtime.restore(runtime_before);
                    provenance.restore(&provenance_before);
                    let _ = runtime.record_scheduler_runtime_fault(run_id, cause_digest);
                    resume_unwind(payload);
                }
            };

            records.push(record);
        }

        runtime.global_tick = next_global_tick;
        Ok(records)
    }

    /// Returns the canonical ordering of runnable heads without mutating state.
    #[must_use]
    pub fn peek_order(runtime: &WorldlineRuntime) -> Vec<WriterHeadKey> {
        runtime
            .heads
            .iter()
            .filter_map(|(key, head)| {
                (head.is_admitted()
                    && !head.is_paused()
                    && !runtime.is_runtime_faulted()
                    && !runtime.is_head_faulted(key))
                .then_some(*key)
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::head::{make_head_id, WriterHead};
    use crate::head_inbox::{make_intent_kind, InboxPolicy};
    use crate::playback::PlaybackMode;
    use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};
    use crate::strand::make_strand_id;
    use crate::worldline::WorldlineId;
    use crate::{
        make_node_id, make_type_id, EngineBuilder, GraphStore, GraphView, NodeId, NodeRecord,
    };

    fn wl(n: u8) -> WorldlineId {
        WorldlineId::from_bytes([n; 32])
    }

    fn wt(raw: u64) -> WorldlineTick {
        WorldlineTick::from_raw(raw)
    }

    fn gt(raw: u64) -> GlobalTick {
        GlobalTick::from_raw(raw)
    }

    fn empty_engine() -> Engine {
        let mut store = GraphStore::default();
        let root = make_node_id("root");
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );
        EngineBuilder::new(store, root).build()
    }

    fn register_head(
        runtime: &mut WorldlineRuntime,
        worldline_id: WorldlineId,
        label: &str,
        public_inbox: Option<&str>,
        is_default_writer: bool,
        policy: InboxPolicy,
    ) -> WriterHeadKey {
        let key = WriterHeadKey {
            worldline_id,
            head_id: make_head_id(label),
        };
        runtime
            .register_writer_head(WriterHead::with_routing(
                key,
                PlaybackMode::Play,
                policy,
                public_inbox.map(|name| InboxAddress(name.to_owned())),
                is_default_writer,
            ))
            .unwrap();
        key
    }

    fn runtime_store(runtime: &WorldlineRuntime, worldline_id: WorldlineId) -> &crate::GraphStore {
        let frontier = runtime.worldlines.get(&worldline_id).unwrap();
        frontier
            .state()
            .warp_state()
            .store(&frontier.state().root().warp_id)
            .unwrap()
    }

    fn mirrored_provenance(runtime: &WorldlineRuntime) -> ProvenanceService {
        let mut provenance = ProvenanceService::new();
        for (worldline_id, frontier) in runtime.worldlines().iter() {
            provenance
                .register_worldline(*worldline_id, frontier.state())
                .unwrap();
        }
        provenance
    }

    fn commit_one_tick(
        runtime: &mut WorldlineRuntime,
        provenance: &mut ProvenanceService,
        engine: &mut Engine,
        worldline_id: WorldlineId,
        label: &str,
    ) {
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                make_intent_kind(label),
                label.as_bytes().to_vec(),
            ))
            .unwrap();
        SchedulerCoordinator::super_tick(runtime, provenance, engine).unwrap();
    }

    fn runtime_marker_matches(view: GraphView<'_>, scope: &NodeId) -> bool {
        matches!(
            view.node_attachment(scope),
            Some(crate::AttachmentValue::Atom(payload)) if payload.bytes.as_ref() == b"commit-a"
        )
    }

    fn runtime_panic_matches(view: GraphView<'_>, scope: &NodeId) -> bool {
        matches!(
            view.node_attachment(scope),
            Some(crate::AttachmentValue::Atom(payload)) if payload.bytes.as_ref() == b"panic-b"
        )
    }

    const TOY_INCREMENT_OP_ID: u32 = 1001;
    const TOY_INCREMENT_VARS: &[u8] = b"amount=42";
    const TOY_INCREMENT_RESULT_BYTES: &[u8] = b"value=42";
    const TOY_INCREMENT_RESULT_TYPE: &str = "test/toy-counter/increment-result";
    const TOY_INCREMENT_RESULT_EDGE_TYPE: &str = "test/toy-counter/increment-result-edge";

    fn toy_increment_result_node_id(scope: &NodeId) -> NodeId {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"test.toy-counter.increment.result.node");
        hasher.update(scope.as_bytes());
        NodeId(hasher.finalize().into())
    }

    fn toy_increment_result_edge_id(scope: &NodeId, result: &NodeId) -> crate::EdgeId {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"test.toy-counter.increment.result.edge");
        hasher.update(scope.as_bytes());
        hasher.update(result.as_bytes());
        crate::EdgeId(hasher.finalize().into())
    }

    fn toy_increment_matches(view: GraphView<'_>, scope: &NodeId) -> bool {
        crate::contract_host::eint_vars_for_op(view, scope, TOY_INCREMENT_OP_ID)
            == Some(TOY_INCREMENT_VARS)
    }

    fn toy_increment_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut crate::TickDelta) {
        let Some(vars) = crate::contract_host::eint_vars_for_op(view, scope, TOY_INCREMENT_OP_ID)
        else {
            return;
        };
        if vars != TOY_INCREMENT_VARS {
            return;
        }

        let warp_id = view.warp_id();
        let result = toy_increment_result_node_id(scope);
        let result_edge = toy_increment_result_edge_id(scope, &result);
        delta.push(crate::tick_patch::WarpOp::UpsertNode {
            node: crate::NodeKey {
                warp_id,
                local_id: result,
            },
            record: NodeRecord {
                ty: make_type_id(TOY_INCREMENT_RESULT_TYPE),
            },
        });
        delta.push(crate::tick_patch::WarpOp::UpsertEdge {
            warp_id,
            record: crate::record::EdgeRecord {
                id: result_edge,
                from: *scope,
                to: result,
                ty: make_type_id(TOY_INCREMENT_RESULT_EDGE_TYPE),
            },
        });
        delta.push(crate::tick_patch::WarpOp::SetAttachment {
            key: crate::AttachmentKey::node_alpha(crate::NodeKey {
                warp_id,
                local_id: result,
            }),
            value: Some(crate::AttachmentValue::Atom(crate::AtomPayload::new(
                make_type_id(TOY_INCREMENT_RESULT_TYPE),
                bytes::Bytes::copy_from_slice(TOY_INCREMENT_RESULT_BYTES),
            ))),
        });
    }

    fn toy_increment_footprint(view: GraphView<'_>, scope: &NodeId) -> crate::Footprint {
        let mut footprint = crate::contract_host::runtime_ingress_eint_read_footprint(view, scope);
        let warp_id = view.warp_id();
        let result = toy_increment_result_node_id(scope);
        let result_edge = toy_increment_result_edge_id(scope, &result);
        footprint.n_write.insert_with_warp(warp_id, *scope);
        footprint.n_write.insert_with_warp(warp_id, result);
        footprint.e_write.insert_with_warp(warp_id, result_edge);
        footprint
            .a_write
            .insert(crate::AttachmentKey::node_alpha(crate::NodeKey {
                warp_id,
                local_id: result,
            }));
        footprint
    }

    fn toy_increment_contract_rule() -> RewriteRule {
        RewriteRule {
            id: make_type_id("rule:cmd/contract/toy-counter/increment").0,
            name: "cmd/contract/toy-counter/increment",
            left: PatternGraph { nodes: vec![] },
            matcher: toy_increment_matches,
            executor: toy_increment_executor,
            compute_footprint: toy_increment_footprint,
            factor_mask: 0,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        }
    }

    fn toy_increment_intent(vars: &[u8]) -> Vec<u8> {
        echo_wasm_abi::pack_intent_v1(TOY_INCREMENT_OP_ID, vars).unwrap()
    }

    fn toy_contract_runtime() -> (WorldlineRuntime, Engine, WorldlineId) {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        engine.register_rule(toy_increment_contract_rule()).unwrap();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        (runtime, engine, worldline_id)
    }

    #[test]
    fn installed_contract_handler_runs_only_during_scheduler_owned_tick() {
        let (mut runtime, mut engine, worldline_id) = toy_contract_runtime();

        let envelope = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("echo.intent/eint-v1"),
            toy_increment_intent(TOY_INCREMENT_VARS),
        );
        let event_id = NodeId(envelope.ingress_id());
        let result_id = toy_increment_result_node_id(&event_id);
        let dispatch = runtime.ingest(envelope).unwrap();
        assert!(matches!(dispatch, IngressDisposition::Accepted { .. }));
        assert!(
            runtime_store(&runtime, worldline_id)
                .node(&result_id)
                .is_none(),
            "application dispatch must not call installed contract handlers synchronously"
        );

        let mut provenance = mirrored_provenance(&runtime);
        let records =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].admitted_count, 1);

        let store = runtime_store(&runtime, worldline_id);
        assert!(store.node(&event_id).is_some());
        assert_eq!(
            store.node(&result_id).map(|node| node.ty),
            Some(make_type_id(TOY_INCREMENT_RESULT_TYPE))
        );
        assert!(matches!(
            store.node_attachment(&result_id),
            Some(crate::AttachmentValue::Atom(payload))
                if payload.type_id == make_type_id(TOY_INCREMENT_RESULT_TYPE)
                    && payload.bytes.as_ref() == TOY_INCREMENT_RESULT_BYTES
        ));
        assert_eq!(provenance.len(worldline_id).unwrap(), 1);
    }

    #[test]
    fn installed_contract_handler_ignores_nonmatching_eint_operation() {
        let (mut runtime, mut engine, worldline_id) = toy_contract_runtime();
        let other_intent =
            echo_wasm_abi::pack_intent_v1(TOY_INCREMENT_OP_ID + 1, TOY_INCREMENT_VARS).unwrap();
        let envelope = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("echo.intent/eint-v1"),
            other_intent,
        );
        let event_id = NodeId(envelope.ingress_id());
        let result_id = toy_increment_result_node_id(&event_id);
        runtime.ingest(envelope).unwrap();

        let mut provenance = mirrored_provenance(&runtime);
        let records =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert_eq!(records.len(), 1);

        let store = runtime_store(&runtime, worldline_id);
        assert!(store.node(&event_id).is_some());
        assert!(
            store.node(&result_id).is_none(),
            "installed contract handlers must only run for their generated op id"
        );
        assert_eq!(provenance.len(worldline_id).unwrap(), 1);
    }

    #[test]
    fn fork_strand_registers_child_frontier_and_strand_relation() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let source_lane_id = wl(1);
        let child_worldline_id = wl(2);
        let strand_id = make_strand_id("fork-runtime");

        runtime
            .register_worldline(source_lane_id, WorldlineState::empty())
            .unwrap();
        register_head(
            &mut runtime,
            source_lane_id,
            "source-default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );

        let mut provenance = mirrored_provenance(&runtime);
        commit_one_tick(
            &mut runtime,
            &mut provenance,
            &mut engine,
            source_lane_id,
            "fork-source-commit",
        );

        let child_head_key = WriterHeadKey {
            worldline_id: child_worldline_id,
            head_id: make_head_id("child-default"),
        };
        let receipt = runtime
            .fork_strand(
                &mut provenance,
                ForkStrandRequest {
                    strand_id,
                    source_lane_id,
                    fork_tick: wt(0),
                    child_worldline_id,
                    writer_heads: vec![WriterHead::with_routing(
                        child_head_key,
                        PlaybackMode::Play,
                        InboxPolicy::AcceptAll,
                        None,
                        true,
                    )],
                },
            )
            .unwrap();

        let child_frontier = runtime.worldlines().get(&child_worldline_id).unwrap();
        assert_eq!(child_frontier.frontier_tick(), wt(1));
        assert_eq!(child_frontier.state().current_tick(), wt(1));
        assert_eq!(
            child_frontier.state().state_root(),
            runtime
                .worldlines()
                .get(&source_lane_id)
                .unwrap()
                .state()
                .state_root()
        );

        let strand = runtime.strands().get(&strand_id).unwrap();
        assert_eq!(strand.fork_basis_ref, receipt.fork_basis_ref);
        assert_eq!(receipt.fork_basis_ref.source_lane_id, source_lane_id);
        assert_eq!(receipt.fork_basis_ref.fork_tick, wt(0));
        assert_eq!(receipt.child_worldline_id, child_worldline_id);
        assert_eq!(receipt.writer_heads, vec![child_head_key]);
        assert!(runtime.heads().get(&child_head_key).is_some());
        assert_eq!(provenance.len(child_worldline_id).unwrap(), 1);
    }

    #[test]
    fn fork_strand_from_non_tip_tick_materializes_historical_basis() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let source_lane_id = wl(1);
        let child_worldline_id = wl(2);
        let strand_id = make_strand_id("fork-non-tip");

        runtime
            .register_worldline(source_lane_id, WorldlineState::empty())
            .unwrap();
        register_head(
            &mut runtime,
            source_lane_id,
            "source-default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );

        let mut provenance = mirrored_provenance(&runtime);
        commit_one_tick(
            &mut runtime,
            &mut provenance,
            &mut engine,
            source_lane_id,
            "fork-source-commit-a",
        );
        let historical_state = provenance
            .replay_worldline_state_at(
                source_lane_id,
                runtime.worldlines().get(&source_lane_id).unwrap().state(),
                wt(1),
            )
            .unwrap();
        commit_one_tick(
            &mut runtime,
            &mut provenance,
            &mut engine,
            source_lane_id,
            "fork-source-commit-b",
        );

        let child_head_key = WriterHeadKey {
            worldline_id: child_worldline_id,
            head_id: make_head_id("child-default"),
        };
        runtime
            .fork_strand(
                &mut provenance,
                ForkStrandRequest {
                    strand_id,
                    source_lane_id,
                    fork_tick: wt(0),
                    child_worldline_id,
                    writer_heads: vec![WriterHead::with_routing(
                        child_head_key,
                        PlaybackMode::Play,
                        InboxPolicy::AcceptAll,
                        None,
                        true,
                    )],
                },
            )
            .unwrap();

        let child_frontier = runtime.worldlines().get(&child_worldline_id).unwrap();
        assert_eq!(child_frontier.frontier_tick(), wt(1));
        assert_eq!(child_frontier.state().current_tick(), wt(1));
        assert_eq!(
            child_frontier.state().state_root(),
            historical_state.state_root()
        );
        assert_eq!(provenance.len(child_worldline_id).unwrap(), 1);
    }

    #[test]
    fn fork_strand_rolls_back_runtime_and_provenance_on_error() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let source_lane_id = wl(1);
        let child_worldline_id = wl(2);
        let strand_id = make_strand_id("fork-rollback");

        runtime
            .register_worldline(source_lane_id, WorldlineState::empty())
            .unwrap();
        register_head(
            &mut runtime,
            source_lane_id,
            "source-default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );

        let mut provenance = mirrored_provenance(&runtime);
        commit_one_tick(
            &mut runtime,
            &mut provenance,
            &mut engine,
            source_lane_id,
            "fork-rollback-source",
        );

        let wrong_head_key = WriterHeadKey {
            worldline_id: source_lane_id,
            head_id: make_head_id("wrong-worldline"),
        };
        let err = runtime
            .fork_strand(
                &mut provenance,
                ForkStrandRequest {
                    strand_id,
                    source_lane_id,
                    fork_tick: wt(0),
                    child_worldline_id,
                    writer_heads: vec![WriterHead::with_routing(
                        wrong_head_key,
                        PlaybackMode::Play,
                        InboxPolicy::AcceptAll,
                        Some(InboxAddress("wrong-worldline".to_owned())),
                        false,
                    )],
                },
            )
            .unwrap_err();

        assert!(matches!(
            err,
            RuntimeError::Strand(StrandError::InvariantViolation(_))
        ));
        assert!(runtime.worldlines().get(&child_worldline_id).is_none());
        assert!(runtime.strands().get(&strand_id).is_none());
        assert!(runtime.heads().get(&wrong_head_key).is_none());
        assert!(provenance.len(child_worldline_id).is_err());
        assert_eq!(provenance.len(source_lane_id).unwrap(), 1);
    }

    fn noop_runtime_rule(rule_name: &'static str) -> RewriteRule {
        RewriteRule {
            id: [1; 32],
            name: rule_name,
            left: PatternGraph { nodes: vec![] },
            matcher: runtime_marker_matches,
            executor: |_view, _scope, _delta| {},
            compute_footprint: |_view, _scope| crate::Footprint::default(),
            factor_mask: 0,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        }
    }

    fn test_rule_id(rule_name: &str) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"echo.coordinator.test-rule");
        hasher.update(rule_name.as_bytes());
        hasher.finalize().into()
    }

    fn shared_footprint_runtime_rule(rule_name: &'static str) -> RewriteRule {
        RewriteRule {
            id: test_rule_id(rule_name),
            name: rule_name,
            left: PatternGraph { nodes: vec![] },
            matcher: |_view, _scope| true,
            executor: |_view, _scope, _delta| {},
            compute_footprint: |view, _scope| {
                let mut footprint = crate::Footprint::default();
                footprint
                    .n_write
                    .insert_with_warp(view.warp_id(), make_node_id("shared-footprint"));
                footprint.factor_mask = 1;
                footprint
            },
            factor_mask: 0,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        }
    }

    #[allow(clippy::panic)]
    fn panic_runtime_rule(rule_name: &'static str) -> RewriteRule {
        RewriteRule {
            id: [2; 32],
            name: rule_name,
            left: PatternGraph { nodes: vec![] },
            matcher: runtime_panic_matches,
            executor: |_view, _scope, _delta| std::panic::panic_any("runtime-commit-panic"),
            compute_footprint: |_view, _scope| crate::Footprint::default(),
            factor_mask: 0,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        }
    }

    #[test]
    fn lawful_rejection_does_not_fault_head() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        engine
            .register_rule(shared_footprint_runtime_rule("cmd/shared-footprint"))
            .unwrap();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let head_key = register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let kind = make_intent_kind("test");
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                kind,
                b"conflict-a".to_vec(),
            ))
            .unwrap();
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                kind,
                b"conflict-b".to_vec(),
            ))
            .unwrap();

        let mut provenance = mirrored_provenance(&runtime);
        let records =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].head_key, head_key);
        assert_eq!(runtime.scheduler_fault_count(), 0);
        assert!(runtime.scheduler_fault_for_head(&head_key).is_none());
        assert!(runtime.scheduler_runtime_fault().is_none());
        let (_, receipt, _) = runtime
            .worldlines
            .get(&worldline_id)
            .unwrap()
            .state()
            .tick_history()
            .last()
            .unwrap();
        assert!(receipt.entries().iter().any(|entry| {
            entry.disposition
                == crate::receipt::TickReceiptDisposition::Rejected(
                    crate::receipt::TickReceiptRejection::FootprintConflict,
                )
        }));
    }

    #[test]
    fn default_and_named_routes_are_deterministic() {
        let mut runtime = WorldlineRuntime::new();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();

        let default_key = register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let named_key = register_head(
            &mut runtime,
            worldline_id,
            "orders",
            Some("orders"),
            false,
            InboxPolicy::AcceptAll,
        );

        let kind = make_intent_kind("test");
        let default_env = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            kind,
            b"default".to_vec(),
        );
        let named_env = IngressEnvelope::local_intent(
            IngressTarget::InboxAddress {
                worldline_id,
                inbox: InboxAddress("orders".to_string()),
            },
            kind,
            b"named".to_vec(),
        );

        let default_result = runtime.ingest(default_env).unwrap();
        let named_result = runtime.ingest(named_env).unwrap();
        let default_id = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            kind,
            b"default".to_vec(),
        )
        .ingress_id();
        let named_id = IngressEnvelope::local_intent(
            IngressTarget::InboxAddress {
                worldline_id,
                inbox: InboxAddress("orders".to_string()),
            },
            kind,
            b"named".to_vec(),
        )
        .ingress_id();

        assert!(matches!(
            default_result,
            IngressDisposition::Accepted { ingress_id, head_key, .. }
                if ingress_id == default_id && head_key == default_key
        ));
        assert!(matches!(
            named_result,
            IngressDisposition::Accepted { ingress_id, head_key, .. }
                if ingress_id == named_id && head_key == named_key
        ));
    }

    #[test]
    fn duplicate_public_inbox_is_rejected() {
        let mut runtime = WorldlineRuntime::new();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();

        let head_a = WriterHead::with_routing(
            WriterHeadKey {
                worldline_id,
                head_id: make_head_id("a"),
            },
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            Some(InboxAddress("orders".to_string())),
            true,
        );
        let head_b = WriterHead::with_routing(
            WriterHeadKey {
                worldline_id,
                head_id: make_head_id("b"),
            },
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            Some(InboxAddress("orders".to_string())),
            false,
        );

        runtime.register_writer_head(head_a).unwrap();
        let err = runtime.register_writer_head(head_b).unwrap_err();
        assert!(matches!(err, RuntimeError::DuplicateInboxAddress { .. }));
    }

    #[test]
    fn duplicate_ingress_is_scoped_to_the_resolved_head() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();

        let default_key = register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let named_key = register_head(
            &mut runtime,
            worldline_id,
            "orders",
            Some("orders"),
            false,
            InboxPolicy::AcceptAll,
        );

        let kind = make_intent_kind("test");
        let default_env = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            kind,
            b"same-payload".to_vec(),
        );
        let named_env = IngressEnvelope::local_intent(
            IngressTarget::InboxAddress {
                worldline_id,
                inbox: InboxAddress("orders".to_owned()),
            },
            kind,
            b"same-payload".to_vec(),
        );

        assert!(matches!(
            runtime.ingest(default_env.clone()).unwrap(),
            IngressDisposition::Accepted { ingress_id, head_key, .. }
                if ingress_id == default_env.ingress_id() && head_key == default_key
        ));
        assert!(matches!(
            runtime.ingest(default_env.clone()).unwrap(),
            IngressDisposition::Duplicate { ingress_id, head_key, .. }
                if ingress_id == default_env.ingress_id() && head_key == default_key
        ));

        let mut provenance = mirrored_provenance(&runtime);
        let records =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert_eq!(records.len(), 1);

        assert!(matches!(
            runtime.ingest(named_env.clone()).unwrap(),
            IngressDisposition::Accepted { ingress_id, head_key, .. }
                if ingress_id == named_env.ingress_id() && head_key == named_key
        ));
        assert!(matches!(
            runtime.ingest(named_env).unwrap(),
            IngressDisposition::Duplicate { ingress_id, head_key, .. }
                if ingress_id == default_env.ingress_id() && head_key == named_key
        ));
    }

    #[test]
    fn submission_event_is_content_addressed_by_intent_bytes_and_target() {
        let mut runtime = WorldlineRuntime::new();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let default_key = register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let named_key = register_head(
            &mut runtime,
            worldline_id,
            "orders",
            Some("orders"),
            false,
            InboxPolicy::AcceptAll,
        );

        let kind = make_intent_kind("test");
        let default_env = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            kind,
            b"same-payload".to_vec(),
        );
        let named_env = IngressEnvelope::local_intent(
            IngressTarget::InboxAddress {
                worldline_id,
                inbox: InboxAddress("orders".to_owned()),
            },
            kind,
            b"same-payload".to_vec(),
        );

        let default_accepted = match runtime.ingest(default_env.clone()).unwrap() {
            IngressDisposition::Accepted {
                ingress_id,
                head_key,
                submission_id,
                submission_generation,
            } => Some((ingress_id, head_key, submission_id, submission_generation)),
            IngressDisposition::Duplicate { .. } => None,
        };
        assert!(
            default_accepted.is_some(),
            "default submission should be accepted"
        );
        let Some((default_ingress, default_head, default_submission, default_generation)) =
            default_accepted
        else {
            return;
        };

        let named_accepted = match runtime.ingest(named_env.clone()).unwrap() {
            IngressDisposition::Accepted {
                ingress_id,
                head_key,
                submission_id,
                submission_generation,
            } => Some((ingress_id, head_key, submission_id, submission_generation)),
            IngressDisposition::Duplicate { .. } => None,
        };
        assert!(
            named_accepted.is_some(),
            "named submission should be accepted"
        );
        let Some((named_ingress, named_head, named_submission, named_generation)) = named_accepted
        else {
            return;
        };

        assert_eq!(default_ingress, default_env.ingress_id());
        assert_eq!(named_ingress, named_env.ingress_id());
        assert_eq!(default_ingress, named_ingress);
        assert_eq!(default_head, default_key);
        assert_eq!(named_head, named_key);
        assert_ne!(
            default_submission, named_submission,
            "same canonical intent bytes routed to different heads must have distinct submission ids"
        );
        assert_eq!(default_generation.as_u64(), 1);
        assert_eq!(named_generation.as_u64(), 2);

        let default_record = runtime.witnessed_submission(&default_submission);
        assert!(
            default_record.is_some(),
            "default submission should be recorded"
        );
        let Some(default_record) = default_record else {
            return;
        };
        assert_eq!(default_record.ingress_id, default_env.ingress_id());
        assert_eq!(default_record.head_key, default_key);
        assert_eq!(default_record.submission_generation, default_generation);

        let named_record = runtime.witnessed_submission(&named_submission);
        assert!(
            named_record.is_some(),
            "named submission should be recorded"
        );
        let Some(named_record) = named_record else {
            return;
        };
        assert_eq!(named_record.ingress_id, named_env.ingress_id());
        assert_eq!(named_record.head_key, named_key);
        assert_eq!(named_record.submission_generation, named_generation);
    }

    #[test]
    fn duplicate_submission_returns_same_submission_identity_without_duplicate_history() {
        let mut runtime = WorldlineRuntime::new();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let env = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test"),
            b"duplicate".to_vec(),
        );

        let first = runtime.ingest(env.clone()).unwrap();
        let duplicate = runtime.ingest(env).unwrap();

        let first_accepted = match first {
            IngressDisposition::Accepted {
                submission_id,
                submission_generation,
                ..
            } => Some((submission_id, submission_generation)),
            IngressDisposition::Duplicate { .. } => None,
        };
        assert!(
            first_accepted.is_some(),
            "first submission should be accepted"
        );
        let Some((first_submission, first_generation)) = first_accepted else {
            return;
        };

        let duplicate_posture = match duplicate {
            IngressDisposition::Duplicate {
                submission_id,
                submission_generation,
                ..
            } => Some((submission_id, submission_generation)),
            IngressDisposition::Accepted { .. } => None,
        };
        assert!(
            duplicate_posture.is_some(),
            "second submission should be duplicate"
        );
        let Some((duplicate_submission, duplicate_generation)) = duplicate_posture else {
            return;
        };

        assert_eq!(first_submission, duplicate_submission);
        assert_eq!(first_generation, duplicate_generation);
        assert_eq!(runtime.witnessed_submission_count(), 1);
    }

    #[test]
    fn submission_history_does_not_advance_worldline_tick() {
        let mut runtime = WorldlineRuntime::new();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let head_key = register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let env = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test"),
            b"pending".to_vec(),
        );
        let ingress_id = env.ingress_id();

        assert!(matches!(
            runtime.ingest(env).unwrap(),
            IngressDisposition::Accepted { .. }
        ));

        assert_eq!(runtime.global_tick(), gt(0));
        let frontier = runtime.worldlines().get(&worldline_id).unwrap();
        assert_eq!(frontier.frontier_tick(), wt(0));
        assert!(!frontier
            .state()
            .contains_committed_ingress(&head_key, &ingress_id));
        assert_eq!(
            runtime
                .heads
                .get(&head_key)
                .unwrap()
                .inbox()
                .pending_count(),
            1
        );
        assert_eq!(runtime.witnessed_submission_count(), 1);
    }

    #[test]
    fn submission_order_does_not_define_scheduler_order() {
        let worldline_id = wl(1);
        let kind = make_intent_kind("test");
        let first = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            kind,
            b"first-submitted".to_vec(),
        );
        let second = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            kind,
            b"second-submitted".to_vec(),
        );

        let mut runtime_ab = WorldlineRuntime::new();
        runtime_ab
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let head_ab = register_head(
            &mut runtime_ab,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        runtime_ab.ingest(first.clone()).unwrap();
        runtime_ab.ingest(second.clone()).unwrap();
        let admitted_ab = runtime_ab
            .heads
            .inbox_mut(&head_ab)
            .unwrap()
            .admit()
            .into_iter()
            .map(|env| env.ingress_id())
            .collect::<Vec<_>>();

        let mut runtime_ba = WorldlineRuntime::new();
        runtime_ba
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let head_ba = register_head(
            &mut runtime_ba,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        runtime_ba.ingest(second).unwrap();
        runtime_ba.ingest(first).unwrap();
        let admitted_ba = runtime_ba
            .heads
            .inbox_mut(&head_ba)
            .unwrap()
            .admit()
            .into_iter()
            .map(|env| env.ingress_id())
            .collect::<Vec<_>>();

        assert_eq!(
            admitted_ab, admitted_ba,
            "scheduler admission order must follow canonical ingress ids, not submission order"
        );
    }

    #[test]
    fn submission_history_has_deterministic_replay_shape() {
        fn record_shape(runtime: &WorldlineRuntime) -> Vec<(Hash, Hash, WriterHeadKey, u64)> {
            runtime
                .witnessed_submissions()
                .map(|record| {
                    (
                        record.submission_id,
                        record.ingress_id,
                        record.head_key,
                        record.submission_generation.as_u64(),
                    )
                })
                .collect()
        }

        fn build_runtime_with_submissions() -> WorldlineRuntime {
            let mut runtime = WorldlineRuntime::new();
            let worldline_id = wl(1);
            runtime
                .register_worldline(worldline_id, WorldlineState::empty())
                .unwrap();
            register_head(
                &mut runtime,
                worldline_id,
                "default",
                None,
                true,
                InboxPolicy::AcceptAll,
            );
            for bytes in [b"alpha".as_slice(), b"beta".as_slice(), b"gamma".as_slice()] {
                runtime
                    .ingest(IngressEnvelope::local_intent(
                        IngressTarget::DefaultWriter { worldline_id },
                        make_intent_kind("test"),
                        bytes.to_vec(),
                    ))
                    .unwrap();
            }
            runtime
        }

        let first = build_runtime_with_submissions();
        let second = build_runtime_with_submissions();

        assert_eq!(record_shape(&first), record_shape(&second));
    }

    #[test]
    fn exact_head_route_is_deterministic() {
        let mut runtime = WorldlineRuntime::new();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();

        let exact_key = register_head(
            &mut runtime,
            worldline_id,
            "control",
            None,
            true,
            InboxPolicy::AcceptAll,
        );

        let envelope = IngressEnvelope::local_intent(
            IngressTarget::ExactHead { key: exact_key },
            make_intent_kind("test"),
            b"exact".to_vec(),
        );

        assert!(matches!(
            runtime.ingest(envelope.clone()).unwrap(),
            IngressDisposition::Accepted { ingress_id, head_key, .. }
                if ingress_id == envelope.ingress_id() && head_key == exact_key
        ));
        assert!(matches!(
            runtime.ingest(envelope.clone()).unwrap(),
            IngressDisposition::Duplicate { ingress_id, head_key, .. }
                if ingress_id == envelope.ingress_id() && head_key == exact_key
        ));
    }

    #[test]
    fn missing_default_writer_returns_error() {
        let mut runtime = WorldlineRuntime::new();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();

        let env = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test"),
            b"hello".to_vec(),
        );
        let err = runtime.ingest(env).unwrap_err();
        assert!(matches!(err, RuntimeError::MissingDefaultWriter(id) if id == worldline_id));
    }

    #[test]
    fn missing_named_inbox_returns_error() {
        let mut runtime = WorldlineRuntime::new();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );

        let env = IngressEnvelope::local_intent(
            IngressTarget::InboxAddress {
                worldline_id,
                inbox: InboxAddress("missing".to_owned()),
            },
            make_intent_kind("test"),
            b"hello".to_vec(),
        );
        let err = runtime.ingest(env).unwrap_err();
        assert!(matches!(
            err,
            RuntimeError::MissingInboxAddress {
                worldline_id: id,
                inbox
            } if id == worldline_id && inbox == InboxAddress("missing".to_owned())
        ));
    }

    #[test]
    fn super_tick_commits_heads_in_canonical_order() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();

        let first = register_head(
            &mut runtime,
            worldline_id,
            "alpha",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let second = register_head(
            &mut runtime,
            worldline_id,
            "beta",
            Some("beta"),
            false,
            InboxPolicy::AcceptAll,
        );

        let kind = make_intent_kind("test");
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::ExactHead { key: second },
                kind,
                b"second".to_vec(),
            ))
            .unwrap();
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::ExactHead { key: first },
                kind,
                b"first".to_vec(),
            ))
            .unwrap();

        let expected_order = SchedulerCoordinator::peek_order(&runtime);
        let mut provenance = mirrored_provenance(&runtime);
        let records =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();

        assert_eq!(
            records
                .iter()
                .map(|record| record.head_key)
                .collect::<Vec<_>>(),
            expected_order
        );
        assert!(records.iter().all(|record| record.admitted_count == 1));
        assert_eq!(provenance.len(worldline_id).unwrap(), 2);
        assert_eq!(
            provenance.entry(worldline_id, wt(0)).unwrap().head_key,
            Some(first)
        );
        assert_eq!(
            provenance.entry(worldline_id, wt(1)).unwrap().head_key,
            Some(second)
        );
    }

    #[test]
    fn super_tick_keeps_worldlines_isolated() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_a = wl(1);
        let worldline_b = wl(2);
        runtime
            .register_worldline(worldline_a, WorldlineState::empty())
            .unwrap();
        runtime
            .register_worldline(worldline_b, WorldlineState::empty())
            .unwrap();

        let head_a = register_head(
            &mut runtime,
            worldline_a,
            "default-a",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let head_b = register_head(
            &mut runtime,
            worldline_b,
            "default-b",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let env_a = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: worldline_a,
            },
            make_intent_kind("test"),
            b"alpha".to_vec(),
        );
        let env_b = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: worldline_b,
            },
            make_intent_kind("test"),
            b"beta".to_vec(),
        );

        runtime.ingest(env_a.clone()).unwrap();
        runtime.ingest(env_b.clone()).unwrap();

        let mut provenance = mirrored_provenance(&runtime);
        let records =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(provenance.len(worldline_a).unwrap(), 1);
        assert_eq!(provenance.len(worldline_b).unwrap(), 1);
        assert_eq!(
            provenance.entry(worldline_a, wt(0)).unwrap().head_key,
            Some(head_a)
        );
        assert_eq!(
            provenance.entry(worldline_b, wt(0)).unwrap().head_key,
            Some(head_b)
        );
        assert_eq!(
            runtime
                .worldlines
                .get(&worldline_a)
                .unwrap()
                .frontier_tick(),
            wt(1)
        );
        assert_eq!(
            runtime
                .worldlines
                .get(&worldline_b)
                .unwrap()
                .frontier_tick(),
            wt(1)
        );
        assert!(runtime
            .worldlines
            .get(&worldline_a)
            .unwrap()
            .state()
            .contains_committed_ingress(&head_a, &env_a.ingress_id()));
        assert!(runtime
            .worldlines
            .get(&worldline_b)
            .unwrap()
            .state()
            .contains_committed_ingress(&head_b, &env_b.ingress_id()));
        assert!(runtime_store(&runtime, worldline_a)
            .node(&crate::NodeId(env_a.ingress_id()))
            .is_some());
        assert!(runtime_store(&runtime, worldline_b)
            .node(&crate::NodeId(env_b.ingress_id()))
            .is_some());
        assert!(runtime_store(&runtime, worldline_a)
            .node(&crate::NodeId(env_b.ingress_id()))
            .is_none());
        assert!(runtime_store(&runtime, worldline_b)
            .node(&crate::NodeId(env_a.ingress_id()))
            .is_none());
    }

    #[test]
    fn empty_super_tick_does_not_advance_frontier_ticks() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );

        let mut provenance = mirrored_provenance(&runtime);
        let records =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert!(records.is_empty());
        assert_eq!(provenance.len(worldline_id).unwrap(), 0);
        assert_eq!(
            runtime
                .worldlines
                .get(&worldline_id)
                .unwrap()
                .frontier_tick(),
            wt(0)
        );
    }

    #[test]
    fn dormant_head_is_skipped_by_super_tick_until_readmitted() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let head_key = register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );

        let envelope = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test"),
            b"dormant".to_vec(),
        );
        let ingress_id = envelope.ingress_id();
        runtime.ingest(envelope).unwrap();
        runtime
            .set_head_eligibility(head_key, HeadEligibility::Dormant)
            .unwrap();

        let mut provenance = mirrored_provenance(&runtime);
        let skipped =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert!(skipped.is_empty(), "dormant head should not be executed");
        assert_eq!(provenance.len(worldline_id).unwrap(), 0);
        assert_eq!(
            runtime
                .heads
                .get(&head_key)
                .unwrap()
                .inbox()
                .pending_count(),
            1,
            "dormant head should retain its pending ingress"
        );
        assert!(
            runtime_store(&runtime, worldline_id)
                .node(&crate::NodeId(ingress_id))
                .is_none(),
            "dormant head should not mutate worldline state"
        );

        runtime
            .set_head_eligibility(head_key, HeadEligibility::Admitted)
            .unwrap();
        let resumed =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert_eq!(resumed.len(), 1, "readmitted head should execute");
        assert_eq!(resumed[0].head_key, head_key);
        assert_eq!(provenance.len(worldline_id).unwrap(), 1);
    }

    #[test]
    fn frontier_tick_overflow_preflight_preserves_uncommitted_state_and_faults_head() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        let head_key = register_head(
            {
                runtime
                    .register_worldline(worldline_id, WorldlineState::empty())
                    .unwrap();
                &mut runtime
            },
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let envelope = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test"),
            b"overflow-frontier".to_vec(),
        );
        runtime.ingest(envelope.clone()).unwrap();
        runtime
            .worldlines
            .frontier_mut(&worldline_id)
            .unwrap()
            .frontier_tick = WorldlineTick::MAX;

        let mut provenance = mirrored_provenance(&runtime);
        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(err, RuntimeError::FrontierTickOverflow(id) if id == worldline_id));
        assert_eq!(
            runtime
                .worldlines
                .get(&worldline_id)
                .unwrap()
                .frontier_tick(),
            WorldlineTick::MAX
        );
        assert_eq!(
            runtime
                .heads
                .get(&head_key)
                .unwrap()
                .inbox()
                .pending_count(),
            1,
            "overflow must leave the admitted envelope pending"
        );
        assert!(
            runtime
                .worldlines
                .get(&worldline_id)
                .unwrap()
                .state()
                .last_snapshot()
                .is_none(),
            "overflow must not record a committed snapshot"
        );
        assert!(
            runtime_store(&runtime, worldline_id)
                .node(&crate::NodeId(envelope.ingress_id()))
                .is_none(),
            "overflow must not mutate the worldline state"
        );
        let fault = runtime
            .scheduler_fault_for_head(&head_key)
            .expect("frontier overflow should fault the scoped head");
        assert_eq!(fault.scope, SchedulerFaultScope::Head(head_key));
        assert_eq!(fault.status, SchedulerFaultStatus::Active);
        assert!(runtime.scheduler_runtime_fault().is_none());
        assert!(
            SchedulerCoordinator::peek_order(&runtime).is_empty(),
            "faulted head must not remain scheduler-runnable"
        );
    }

    #[test]
    fn global_tick_overflow_preflight_preserves_uncommitted_state_and_faults_runtime() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        let head_key = register_head(
            {
                runtime
                    .register_worldline(worldline_id, WorldlineState::empty())
                    .unwrap();
                &mut runtime
            },
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let envelope = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test"),
            b"overflow-global".to_vec(),
        );
        runtime.ingest(envelope.clone()).unwrap();
        runtime.global_tick = GlobalTick::MAX;

        let mut provenance = mirrored_provenance(&runtime);
        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(err, RuntimeError::GlobalTickOverflow));
        assert_eq!(
            runtime
                .heads
                .get(&head_key)
                .unwrap()
                .inbox()
                .pending_count(),
            1,
            "global-tick overflow must leave the envelope pending"
        );
        assert!(
            runtime
                .worldlines
                .get(&worldline_id)
                .unwrap()
                .state()
                .last_snapshot()
                .is_none(),
            "global-tick overflow must not record a committed snapshot"
        );
        assert!(
            runtime_store(&runtime, worldline_id)
                .node(&crate::NodeId(envelope.ingress_id()))
                .is_none(),
            "global-tick overflow must not mutate the worldline state"
        );
        let fault = runtime
            .scheduler_runtime_fault()
            .expect("global tick overflow should fault the runtime");
        assert_eq!(fault.scope, SchedulerFaultScope::Runtime);
        assert_eq!(fault.status, SchedulerFaultStatus::Active);
        let fault_id = fault.fault_id;
        assert!(runtime.scheduler_fault_for_head(&head_key).is_none());
        let blocked = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(
            blocked,
            RuntimeError::SchedulerRuntimeFaultActive(id) if id == fault_id
        ));
    }

    #[test]
    fn super_tick_rolls_back_earlier_head_commits_when_a_later_head_fails() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_a = wl(1);
        let worldline_b = wl(2);
        runtime
            .register_worldline(worldline_a, WorldlineState::empty())
            .unwrap();
        runtime
            .register_worldline(worldline_b, WorldlineState::empty())
            .unwrap();
        let head_a = register_head(
            &mut runtime,
            worldline_a,
            "default-a",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let head_b = register_head(
            &mut runtime,
            worldline_b,
            "default-b",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let env_a = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: worldline_a,
            },
            make_intent_kind("test"),
            b"commit-a".to_vec(),
        );
        let env_b = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: worldline_b,
            },
            make_intent_kind("test"),
            b"commit-b".to_vec(),
        );
        let env_a_ingress_id = env_a.ingress_id();
        runtime.ingest(env_a).unwrap();
        runtime.ingest(env_b).unwrap();

        {
            let frontier = runtime.worldlines.frontier_mut(&worldline_b).unwrap();
            let broken_root = frontier.state.root.warp_id;
            assert!(frontier.state.warp_state.delete_instance(&broken_root));
        }

        let mut provenance = mirrored_provenance(&runtime);
        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(
            err,
            RuntimeError::Engine(EngineError::UnknownWarp(warp_id))
                if warp_id == runtime
                    .worldlines
                    .get(&worldline_b)
                    .unwrap()
                    .state()
                    .root()
                    .warp_id
        ));

        assert_eq!(
            runtime.global_tick(),
            gt(0),
            "failed SuperTick must not advance global tick"
        );
        assert_eq!(
            runtime.heads.get(&head_a).unwrap().inbox().pending_count(),
            1,
            "rollback must restore the earlier head inbox"
        );
        assert_eq!(
            runtime.heads.get(&head_b).unwrap().inbox().pending_count(),
            1,
            "rollback must preserve the failing head inbox contents"
        );
        assert!(
            runtime
                .worldlines
                .get(&worldline_a)
                .unwrap()
                .state()
                .last_snapshot()
                .is_none(),
            "rollback must discard snapshots from earlier successful heads"
        );
        assert!(
            runtime
                .worldlines
                .get(&worldline_b)
                .unwrap()
                .state()
                .last_snapshot()
                .is_none(),
            "the failing head must not record a committed snapshot"
        );
        assert!(
            runtime_store(&runtime, worldline_a)
                .node(&crate::NodeId(env_a_ingress_id))
                .is_none(),
            "rollback must discard earlier runtime ingress materialization"
        );
        assert_eq!(provenance.len(worldline_a).unwrap(), 0);
        assert_eq!(provenance.len(worldline_b).unwrap(), 0);
        assert!(
            runtime
                .worldlines
                .get(&worldline_b)
                .unwrap()
                .state()
                .warp_state()
                .store(
                    &runtime
                        .worldlines
                        .get(&worldline_b)
                        .unwrap()
                        .state()
                        .root()
                        .warp_id
                )
                .is_none(),
            "rollback must restore the failing worldline to its pre-SuperTick state"
        );
    }

    #[test]
    fn super_tick_failure_rolls_back_ticketed_receipt_correlations() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_a = wl(1);
        let worldline_b = wl(2);
        runtime
            .register_worldline(worldline_a, WorldlineState::empty())
            .unwrap();
        runtime
            .register_worldline(worldline_b, WorldlineState::empty())
            .unwrap();
        let head_a = register_head(
            &mut runtime,
            worldline_a,
            "default-a",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let head_b = register_head(
            &mut runtime,
            worldline_b,
            "default-b",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let env_a = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: worldline_a,
            },
            make_intent_kind("test"),
            b"commit-a".to_vec(),
        );
        let env_b = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: worldline_b,
            },
            make_intent_kind("test"),
            b"commit-b".to_vec(),
        );
        let ingress_id = env_a.ingress_id();
        let submission_id = match runtime.ingest(env_a).unwrap() {
            IngressDisposition::Accepted { submission_id, .. } => submission_id,
            IngressDisposition::Duplicate { .. } => {
                unreachable!("first runtime ingress must be accepted")
            }
        };
        runtime.ingest(env_b).unwrap();

        let ticket_digest = [7; 32];
        let ticketed_ingress_id =
            derive_ticketed_runtime_ingress_id(submission_id, ticket_digest, ingress_id, head_a);
        let ticketed_record = TicketedRuntimeIngressRecord {
            ticketed_ingress_id,
            submission_id,
            ticket_digest,
            ingress_id,
            head_key: head_a,
        };
        runtime
            .ticketed_runtime_ingress
            .insert(ticketed_ingress_id, ticketed_record);
        runtime
            .ticketed_runtime_ingress_by_submission
            .insert(submission_id, ticketed_ingress_id);
        runtime
            .ticketed_runtime_ingress_by_target
            .insert((head_a, ingress_id), ticketed_ingress_id);

        {
            let frontier = runtime.worldlines.frontier_mut(&worldline_b).unwrap();
            let broken_root = frontier.state.root.warp_id;
            assert!(frontier.state.warp_state.delete_instance(&broken_root));
        }

        let mut provenance = mirrored_provenance(&runtime);
        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(
            err,
            RuntimeError::Engine(EngineError::UnknownWarp(warp_id))
                if warp_id == runtime
                    .worldlines
                    .get(&worldline_b)
                    .unwrap()
                    .state()
                    .root()
                    .warp_id
        ));
        assert_eq!(
            runtime.receipt_correlation_count(),
            0,
            "failed SuperTick must roll back receipt correlations from earlier heads"
        );
        assert!(runtime
            .receipt_correlation_for_ticketed_ingress(&ticketed_ingress_id)
            .is_none());
        assert!(runtime
            .receipt_correlation_for_submission(&submission_id)
            .is_none());
        assert!(runtime
            .receipt_correlation_for_ticket(&ticket_digest)
            .is_none());
        assert_eq!(
            runtime.heads.get(&head_b).unwrap().inbox().pending_count(),
            1,
            "rollback must preserve the failing head inbox contents"
        );
    }

    #[test]
    fn failed_later_head_rolls_back_attempt_and_faults_only_failing_head() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_a = wl(1);
        let worldline_b = wl(2);
        runtime
            .register_worldline(worldline_a, WorldlineState::empty())
            .unwrap();
        runtime
            .register_worldline(worldline_b, WorldlineState::empty())
            .unwrap();
        let head_a = register_head(
            &mut runtime,
            worldline_a,
            "default-a",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let head_b = register_head(
            &mut runtime,
            worldline_b,
            "default-b",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let env_a = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: worldline_a,
            },
            make_intent_kind("test"),
            b"commit-a".to_vec(),
        );
        let env_b = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: worldline_b,
            },
            make_intent_kind("test"),
            b"commit-b".to_vec(),
        );
        let env_a_ingress_id = env_a.ingress_id();
        runtime.ingest(env_a).unwrap();
        runtime.ingest(env_b).unwrap();

        {
            let frontier = runtime.worldlines.frontier_mut(&worldline_b).unwrap();
            let broken_root = frontier.state.root.warp_id;
            assert!(frontier.state.warp_state.delete_instance(&broken_root));
        }

        let mut provenance = mirrored_provenance(&runtime);
        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(
            err,
            RuntimeError::Engine(EngineError::UnknownWarp(_))
        ));

        assert_eq!(runtime.global_tick(), gt(0));
        assert_eq!(provenance.len(worldline_a).unwrap(), 0);
        assert_eq!(provenance.len(worldline_b).unwrap(), 0);
        assert!(runtime_store(&runtime, worldline_a)
            .node(&crate::NodeId(env_a_ingress_id))
            .is_none());
        assert!(
            runtime.scheduler_fault_for_head(&head_a).is_none(),
            "rollback collateral must not fault the earlier successful head"
        );
        let fault = runtime
            .scheduler_fault_for_head(&head_b)
            .expect("failing head should be quarantined");
        assert_eq!(fault.scope, SchedulerFaultScope::Head(head_b));
        assert_eq!(fault.status, SchedulerFaultStatus::Active);
        assert!(runtime.scheduler_runtime_fault().is_none());
    }

    #[test]
    fn faulted_head_is_skipped_and_unrelated_head_continues_on_next_tick() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_a = wl(1);
        let worldline_b = wl(2);
        runtime
            .register_worldline(worldline_a, WorldlineState::empty())
            .unwrap();
        runtime
            .register_worldline(worldline_b, WorldlineState::empty())
            .unwrap();
        let head_a = register_head(
            &mut runtime,
            worldline_a,
            "default-a",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let head_b = register_head(
            &mut runtime,
            worldline_b,
            "default-b",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter {
                    worldline_id: worldline_a,
                },
                make_intent_kind("test"),
                b"commit-a".to_vec(),
            ))
            .unwrap();
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter {
                    worldline_id: worldline_b,
                },
                make_intent_kind("test"),
                b"commit-b".to_vec(),
            ))
            .unwrap();

        {
            let frontier = runtime.worldlines.frontier_mut(&worldline_b).unwrap();
            let broken_root = frontier.state.root.warp_id;
            assert!(frontier.state.warp_state.delete_instance(&broken_root));
        }

        let mut provenance = mirrored_provenance(&runtime);
        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(
            err,
            RuntimeError::Engine(EngineError::UnknownWarp(_))
        ));
        assert!(runtime.scheduler_fault_for_head(&head_b).is_some());

        let records =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].head_key, head_a);
        assert_eq!(runtime.global_tick(), gt(1));
        assert_eq!(provenance.len(worldline_a).unwrap(), 1);
        assert_eq!(provenance.len(worldline_b).unwrap(), 0);
        assert_eq!(
            runtime.heads.get(&head_b).unwrap().inbox().pending_count(),
            1,
            "faulted head keeps pending ingress but is not retried"
        );
        assert!(runtime.scheduler_fault_for_head(&head_b).is_some());
    }

    #[test]
    fn trusted_recovery_is_required_to_resume_faulted_head() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let head_key = register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                make_intent_kind("test"),
                b"commit".to_vec(),
            ))
            .unwrap();
        runtime
            .worldlines
            .frontier_mut(&worldline_id)
            .unwrap()
            .frontier_tick = WorldlineTick::MAX;

        let mut provenance = mirrored_provenance(&runtime);
        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(err, RuntimeError::FrontierTickOverflow(id) if id == worldline_id));
        let fault_id = runtime
            .scheduler_fault_for_head(&head_key)
            .expect("overflowing head should be faulted")
            .fault_id;

        runtime
            .set_head_eligibility(head_key, HeadEligibility::Admitted)
            .unwrap();
        assert!(
            runtime.scheduler_fault_for_head(&head_key).is_some(),
            "generic eligibility changes must not clear fault quarantine"
        );

        let authority = SchedulerFaultRecoveryAuthority::assume_runtime_owner();
        runtime
            .resolve_scheduler_fault(&authority, fault_id, [9; 32])
            .unwrap();
        assert!(runtime.scheduler_fault_for_head(&head_key).is_none());
    }

    #[test]
    fn retried_head_fault_after_recovery_keeps_resolved_fault_evidence() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let head_key = register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                make_intent_kind("test"),
                b"commit".to_vec(),
            ))
            .unwrap();
        runtime
            .worldlines
            .frontier_mut(&worldline_id)
            .unwrap()
            .frontier_tick = WorldlineTick::MAX;

        let mut provenance = mirrored_provenance(&runtime);
        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(err, RuntimeError::FrontierTickOverflow(id) if id == worldline_id));
        let first_fault_id = runtime
            .scheduler_fault_for_head(&head_key)
            .expect("overflowing head should be faulted")
            .fault_id;

        let authority = SchedulerFaultRecoveryAuthority::assume_runtime_owner();
        runtime
            .resolve_scheduler_fault(&authority, first_fault_id, [7; 32])
            .unwrap();
        assert_eq!(
            runtime.scheduler_fault(&first_fault_id).unwrap().status,
            SchedulerFaultStatus::Resolved {
                recovery_id: [7; 32]
            }
        );

        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(err, RuntimeError::FrontierTickOverflow(id) if id == worldline_id));
        let second_fault = runtime
            .scheduler_fault_for_head(&head_key)
            .expect("retried overflowing head should be faulted again");

        assert_ne!(
            first_fault_id, second_fault.fault_id,
            "retry after trusted recovery must not overwrite resolved fault evidence"
        );
        assert_eq!(
            runtime.scheduler_fault(&first_fault_id).unwrap().status,
            SchedulerFaultStatus::Resolved {
                recovery_id: [7; 32]
            }
        );
        assert_eq!(second_fault.status, SchedulerFaultStatus::Active);
        assert_eq!(runtime.scheduler_fault_count(), 2);
    }

    #[test]
    fn retried_runtime_fault_after_recovery_keeps_resolved_fault_evidence() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        engine
            .register_rule(panic_runtime_rule("cmd/runtime-panic"))
            .unwrap();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                make_intent_kind("test"),
                b"panic-b".to_vec(),
            ))
            .unwrap();
        let mut provenance = mirrored_provenance(&runtime);

        let first_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine);
        }));
        assert!(first_panic.is_err());
        let first_fault_id = runtime
            .scheduler_runtime_fault()
            .expect("panic should fault the runtime")
            .fault_id;

        let authority = SchedulerFaultRecoveryAuthority::assume_runtime_owner();
        runtime
            .resolve_scheduler_fault(&authority, first_fault_id, [8; 32])
            .unwrap();
        assert_eq!(
            runtime.scheduler_fault(&first_fault_id).unwrap().status,
            SchedulerFaultStatus::Resolved {
                recovery_id: [8; 32]
            }
        );

        let second_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine);
        }));
        assert!(second_panic.is_err());
        let second_fault = runtime
            .scheduler_runtime_fault()
            .expect("retried panic should fault the runtime again");

        assert_ne!(
            first_fault_id, second_fault.fault_id,
            "retry after trusted recovery must not overwrite resolved runtime fault evidence"
        );
        assert_eq!(
            runtime.scheduler_fault(&first_fault_id).unwrap().status,
            SchedulerFaultStatus::Resolved {
                recovery_id: [8; 32]
            }
        );
        assert_eq!(second_fault.status, SchedulerFaultStatus::Active);
        assert_eq!(runtime.scheduler_fault_count(), 2);
    }

    #[test]
    fn runtime_error_fault_digest_uses_canonical_variant_tags() {
        let fault_id = SchedulerFaultId::from_bytes([7; 32]);
        let digest = scheduler_error_cause_digest(&RuntimeError::UnknownSchedulerFault(fault_id));

        let mut expected = blake3::Hasher::new();
        expected.update(b"echo.scheduler-fault-cause.error");
        expected.update(b"unknown-scheduler-fault");
        expected.update(fault_id.as_bytes());

        let expected: Hash = expected.finalize().into();
        assert_eq!(digest, expected);
    }

    #[test]
    fn super_tick_restores_runtime_before_resuming_a_later_head_panic() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_a = wl(1);
        let worldline_b = wl(2);
        runtime
            .register_worldline(worldline_a, WorldlineState::empty())
            .unwrap();
        runtime
            .register_worldline(worldline_b, WorldlineState::empty())
            .unwrap();
        let register_ok = engine.register_rule(noop_runtime_rule("cmd/runtime-ok"));
        assert!(register_ok.is_ok(), "runtime ok rule should register");
        let register_panic = engine.register_rule(panic_runtime_rule("cmd/runtime-panic"));
        assert!(register_panic.is_ok(), "runtime panic rule should register");
        let head_a = register_head(
            &mut runtime,
            worldline_a,
            "default-a",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let head_b = register_head(
            &mut runtime,
            worldline_b,
            "default-b",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let env_a = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: worldline_a,
            },
            make_intent_kind("test"),
            b"commit-a".to_vec(),
        );
        let env_b = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: worldline_b,
            },
            make_intent_kind("test"),
            b"panic-b".to_vec(),
        );
        let env_a_ingress_id = env_a.ingress_id();
        runtime.ingest(env_a).unwrap();
        runtime.ingest(env_b).unwrap();
        let mut provenance = mirrored_provenance(&runtime);

        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine);
        }));
        let Err(payload) = panic_result else {
            unreachable!("later head panic should resume through coordinator");
        };
        let panic_message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
        assert_eq!(panic_message, Some("runtime-commit-panic"));

        assert_eq!(
            runtime.global_tick(),
            gt(0),
            "panic unwind must not advance global tick"
        );
        assert_eq!(
            runtime.heads.get(&head_a).unwrap().inbox().pending_count(),
            1,
            "panic rollback must restore the earlier head inbox"
        );
        assert_eq!(
            runtime.heads.get(&head_b).unwrap().inbox().pending_count(),
            1,
            "panic rollback must preserve the failing head inbox"
        );
        assert!(
            runtime
                .worldlines
                .get(&worldline_a)
                .unwrap()
                .state()
                .last_snapshot()
                .is_none(),
            "panic rollback must discard earlier committed snapshots"
        );
        assert!(
            runtime_store(&runtime, worldline_a)
                .node(&crate::NodeId(env_a_ingress_id))
                .is_none(),
            "panic rollback must discard earlier runtime ingress materialization"
        );
        assert_eq!(provenance.len(worldline_a).unwrap(), 0);
        assert_eq!(provenance.len(worldline_b).unwrap(), 0);
    }

    #[test]
    fn runtime_fault_blocks_all_scheduler_work_when_fault_is_unscoped() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_a = wl(1);
        let worldline_b = wl(2);
        runtime
            .register_worldline(worldline_a, WorldlineState::empty())
            .unwrap();
        runtime
            .register_worldline(worldline_b, WorldlineState::empty())
            .unwrap();
        let register_ok = engine.register_rule(noop_runtime_rule("cmd/runtime-ok"));
        assert!(register_ok.is_ok(), "runtime ok rule should register");
        let register_panic = engine.register_rule(panic_runtime_rule("cmd/runtime-panic"));
        assert!(register_panic.is_ok(), "runtime panic rule should register");
        let head_a = register_head(
            &mut runtime,
            worldline_a,
            "default-a",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        let head_b = register_head(
            &mut runtime,
            worldline_b,
            "default-b",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter {
                    worldline_id: worldline_a,
                },
                make_intent_kind("test"),
                b"commit-a".to_vec(),
            ))
            .unwrap();
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter {
                    worldline_id: worldline_b,
                },
                make_intent_kind("test"),
                b"panic-b".to_vec(),
            ))
            .unwrap();
        let mut provenance = mirrored_provenance(&runtime);

        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine);
        }));
        assert!(panic_result.is_err());
        let fault = runtime
            .scheduler_runtime_fault()
            .expect("unscoped panic should fault the runtime");
        assert_eq!(fault.scope, SchedulerFaultScope::Runtime);
        assert_eq!(fault.status, SchedulerFaultStatus::Active);
        let fault_id = fault.fault_id;

        let blocked = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(
            blocked,
            RuntimeError::SchedulerRuntimeFaultActive(id) if id == fault_id
        ));
        assert_eq!(
            runtime.heads.get(&head_a).unwrap().inbox().pending_count(),
            1,
            "runtime fault must block otherwise healthy pending work"
        );
        assert_eq!(
            runtime.heads.get(&head_b).unwrap().inbox().pending_count(),
            1,
            "runtime fault must block the panicking pending work"
        );
        assert_eq!(provenance.len(worldline_a).unwrap(), 0);
        assert_eq!(provenance.len(worldline_b).unwrap(), 0);
    }

    #[test]
    fn budgeted_inbox_admits_up_to_its_limit() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let budget_key = register_head(
            &mut runtime,
            worldline_id,
            "budgeted",
            None,
            true,
            InboxPolicy::Budgeted { max_per_tick: 2 },
        );
        let kind = make_intent_kind("test");

        for payload in [b"a".as_slice(), b"b".as_slice(), b"c".as_slice()] {
            runtime
                .ingest(IngressEnvelope::local_intent(
                    IngressTarget::ExactHead { key: budget_key },
                    kind,
                    payload.to_vec(),
                ))
                .unwrap();
        }

        let mut provenance = mirrored_provenance(&runtime);
        let first =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].admitted_count, 2);
        assert_eq!(provenance.len(worldline_id).unwrap(), 1);
        assert_eq!(
            runtime
                .heads
                .get(&budget_key)
                .unwrap()
                .inbox()
                .pending_count(),
            1
        );

        let second =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].admitted_count, 1);
        assert_eq!(provenance.len(worldline_id).unwrap(), 2);
        assert!(runtime.heads.get(&budget_key).unwrap().inbox().is_empty());
    }

    #[test]
    fn runtime_commit_path_does_not_create_legacy_graph_inbox_nodes() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );

        let envelope = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test"),
            b"runtime".to_vec(),
        );
        runtime.ingest(envelope.clone()).unwrap();

        let mut provenance = mirrored_provenance(&runtime);
        let records =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert_eq!(records.len(), 1);

        let store = runtime_store(&runtime, worldline_id);
        assert!(store.node(&make_node_id("sim")).is_none());
        assert!(store.node(&make_node_id("sim/inbox")).is_none());
        assert!(store.node(&crate::NodeId(envelope.ingress_id())).is_some());
    }

    #[test]
    fn peek_order_rebuilds_from_heads_when_cache_is_stale() {
        let mut runtime = WorldlineRuntime::new();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let head_key = register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );

        runtime.runnable = crate::head::RunnableWriterSet::new();

        assert_eq!(SchedulerCoordinator::peek_order(&runtime), vec![head_key]);
    }

    #[test]
    fn super_tick_returns_frontier_tick_overflow_error() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        let head_key = register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                make_intent_kind("test"),
                b"runtime".to_vec(),
            ))
            .unwrap();
        runtime
            .worldlines
            .frontier_mut(&worldline_id)
            .unwrap()
            .frontier_tick = WorldlineTick::MAX;

        let mut provenance = mirrored_provenance(&runtime);
        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(err, RuntimeError::FrontierTickOverflow(id) if id == worldline_id));
        let fault = runtime
            .scheduler_fault_for_head(&head_key)
            .expect("frontier overflow should fault the scoped head");
        assert_eq!(fault.scope, SchedulerFaultScope::Head(head_key));
        assert_eq!(fault.status, SchedulerFaultStatus::Active);
        assert!(runtime.scheduler_runtime_fault().is_none());
    }

    #[test]
    fn super_tick_returns_global_tick_overflow_error() {
        let mut runtime = WorldlineRuntime::new();
        let mut engine = empty_engine();
        let worldline_id = wl(1);
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .unwrap();
        register_head(
            &mut runtime,
            worldline_id,
            "default",
            None,
            true,
            InboxPolicy::AcceptAll,
        );
        runtime.global_tick = GlobalTick::MAX;

        let mut provenance = mirrored_provenance(&runtime);
        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
            .unwrap_err();
        assert!(matches!(err, RuntimeError::GlobalTickOverflow));
        let fault = runtime
            .scheduler_runtime_fault()
            .expect("global overflow should fault the runtime");
        assert_eq!(fault.scope, SchedulerFaultScope::Runtime);
        assert_eq!(fault.status, SchedulerFaultStatus::Active);
    }
}
