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
use crate::head_inbox::{InboxAddress, InboxIngestResult, IngressEnvelope, IngressTarget};
use crate::ident::Hash;
use crate::optic_artifact::OpticAdmissionTicket;
use crate::provenance_store::{
    HistoryError, ProvenanceCheckpoint, ProvenanceEntry, ProvenanceService, ProvenanceStore,
    ReplayError,
};
use crate::strand::{ForkBasisRef, Strand, StrandError, StrandId, StrandRegistry, SupportPin};
use crate::worldline::WorldlineId;
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
    /// Registry of live speculative strands attached to the runtime.
    strands: StrandRegistry,
}

#[derive(Clone, Debug)]
struct RuntimeCheckpoint {
    global_tick: GlobalTick,
    heads: BTreeMap<WriterHeadKey, WriterHead>,
    frontiers: BTreeMap<WorldlineId, WorldlineFrontier>,
    receipt_correlations_by_ticketed_ingress: BTreeMap<Hash, ReceiptCorrelationRecord>,
    receipt_correlation_by_submission: BTreeMap<Hash, Hash>,
    receipt_correlation_by_ticket: BTreeMap<Hash, Hash>,
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
            receipt_correlations_by_ticketed_ingress: self
                .receipt_correlations_by_ticketed_ingress
                .clone(),
            receipt_correlation_by_submission: self.receipt_correlation_by_submission.clone(),
            receipt_correlation_by_ticket: self.receipt_correlation_by_ticket.clone(),
        })
    }

    fn restore(&mut self, checkpoint: RuntimeCheckpoint) {
        self.global_tick = checkpoint.global_tick;
        self.receipt_correlations_by_ticketed_ingress =
            checkpoint.receipt_correlations_by_ticketed_ingress;
        self.receipt_correlation_by_submission = checkpoint.receipt_correlation_by_submission;
        self.receipt_correlation_by_ticket = checkpoint.receipt_correlation_by_ticket;
        for head in checkpoint.heads.into_values() {
            self.heads.insert(head);
        }
        for frontier in checkpoint.frontiers.into_values() {
            self.worldlines.replace_frontier(frontier);
        }
        self.refresh_runnable();
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
    /// match the witnessed submission, the target rejects the envelope, or a
    /// different ticket has already staged the same submission.
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

    fn record_receipt_correlations(
        &mut self,
        head_key: WriterHeadKey,
        admitted: &[IngressEnvelope],
        commit_global_tick: GlobalTick,
        worldline_tick_after: WorldlineTick,
        tick_receipt_digest: Hash,
        commit_hash: Hash,
    ) {
        for envelope in admitted {
            let ingress_id = envelope.ingress_id();
            let Some(ticketed_ingress_id) = self
                .ticketed_runtime_ingress_by_target
                .get(&(head_key, ingress_id))
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
                head_key,
                commit_global_tick,
                worldline_tick_after,
                tick_receipt_digest,
                commit_hash,
            };
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
        let next_global_tick = runtime
            .global_tick
            .checked_increment()
            .ok_or(RuntimeError::GlobalTickOverflow)?;
        runtime.refresh_runnable();

        let mut records = Vec::new();
        let keys: Vec<WriterHeadKey> = runtime.runnable.iter().copied().collect();

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
                return Err(RuntimeError::FrontierTickOverflow(key.worldline_id));
            }
        }

        let runtime_before = runtime.checkpoint_for(&keys)?;
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
                    *key,
                    &admitted,
                    next_global_tick,
                    worldline_tick_after,
                    tick_receipt_digest,
                    snapshot.hash,
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
                    runtime.restore(runtime_before);
                    provenance.restore(&provenance_before);
                    return Err(err);
                }
                Err(payload) => {
                    runtime.restore(runtime_before);
                    provenance.restore(&provenance_before);
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
            .filter_map(|(key, head)| (head.is_admitted() && !head.is_paused()).then_some(*key))
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
    fn frontier_tick_overflow_preflight_preserves_runtime_state() {
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
    }

    #[test]
    fn global_tick_overflow_preflight_preserves_runtime_state() {
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
    }
}
