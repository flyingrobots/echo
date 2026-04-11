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
use crate::provenance_store::{
    HistoryError, ProvenanceCheckpoint, ProvenanceEntry, ProvenanceService, ProvenanceStore,
};
use crate::strand::{Strand, StrandError, StrandId, StrandRegistry, SupportPin};
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
    /// Strand registry or support-pin operation failed.
    #[error(transparent)]
    Strand(#[from] StrandError),
    /// Attempted to advance a frontier tick past `u64::MAX`.
    #[error("frontier tick overflow for worldline: {0:?}")]
    FrontierTickOverflow(WorldlineId),
    /// Attempted to advance the global tick past `u64::MAX`.
    #[error("global tick overflow")]
    GlobalTickOverflow,
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
    },
    /// The envelope was already pending or already committed.
    Duplicate {
        /// Content-addressed ingress id.
        ingress_id: Hash,
        /// The head that owns the duplicate route target.
        head_key: WriterHeadKey,
    },
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
    /// Registry of live speculative strands attached to the runtime.
    strands: StrandRegistry,
}

#[derive(Clone, Debug)]
struct RuntimeCheckpoint {
    global_tick: GlobalTick,
    heads: BTreeMap<WriterHeadKey, WriterHead>,
    frontiers: BTreeMap<WorldlineId, WorldlineFrontier>,
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

    /// Returns the current correlation tick.
    #[must_use]
    pub fn global_tick(&self) -> GlobalTick {
        self.global_tick
    }

    /// Returns the live strand registry.
    #[must_use]
    pub fn strands(&self) -> &StrandRegistry {
        &self.strands
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
            .contains(&strand.base_ref.source_worldline_id)
        {
            return Err(RuntimeError::UnknownWorldline(
                strand.base_ref.source_worldline_id,
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
            return Ok(IngressDisposition::Duplicate {
                ingress_id,
                head_key,
            });
        }

        let outcome = self
            .heads
            .inbox_mut(&head_key)
            .ok_or(RuntimeError::UnknownHead(head_key))?
            .ingest(envelope);

        match outcome {
            InboxIngestResult::Accepted => Ok(IngressDisposition::Accepted {
                ingress_id,
                head_key,
            }),
            InboxIngestResult::Duplicate => Ok(IngressDisposition::Duplicate {
                ingress_id,
                head_key,
            }),
            InboxIngestResult::Rejected => Err(RuntimeError::RejectedByPolicy(head_key)),
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
                    receipt: _,
                } = {
                    let frontier = runtime
                        .worldlines
                        .frontier_mut(&key.worldline_id)
                        .ok_or(RuntimeError::UnknownWorldline(key.worldline_id))?;
                    engine
                        .commit_with_state(frontier.state_mut(), &admitted)
                        .map_err(RuntimeError::from)?
                };

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

        assert_eq!(
            default_result,
            IngressDisposition::Accepted {
                ingress_id: default_id,
                head_key: default_key,
            }
        );
        assert_eq!(
            named_result,
            IngressDisposition::Accepted {
                ingress_id: named_id,
                head_key: named_key,
            }
        );
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

        assert_eq!(
            runtime.ingest(default_env.clone()).unwrap(),
            IngressDisposition::Accepted {
                ingress_id: default_env.ingress_id(),
                head_key: default_key,
            }
        );
        assert_eq!(
            runtime.ingest(default_env.clone()).unwrap(),
            IngressDisposition::Duplicate {
                ingress_id: default_env.ingress_id(),
                head_key: default_key,
            }
        );

        let mut provenance = mirrored_provenance(&runtime);
        let records =
            SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        assert_eq!(records.len(), 1);

        assert_eq!(
            runtime.ingest(named_env.clone()).unwrap(),
            IngressDisposition::Accepted {
                ingress_id: named_env.ingress_id(),
                head_key: named_key,
            }
        );
        assert_eq!(
            runtime.ingest(named_env).unwrap(),
            IngressDisposition::Duplicate {
                ingress_id: default_env.ingress_id(),
                head_key: named_key,
            }
        );
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

        assert_eq!(
            runtime.ingest(envelope.clone()).unwrap(),
            IngressDisposition::Accepted {
                ingress_id: envelope.ingress_id(),
                head_key: exact_key,
            }
        );
        assert_eq!(
            runtime.ingest(envelope.clone()).unwrap(),
            IngressDisposition::Duplicate {
                ingress_id: envelope.ingress_id(),
                head_key: exact_key,
            }
        );
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
