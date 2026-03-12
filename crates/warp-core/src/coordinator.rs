// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Worldline-aware runtime coordinator and deterministic ingress routing.
//!
//! The [`WorldlineRuntime`] owns the live ingress path for ADR-0008 Phase 3:
//! per-head inboxes, deterministic routing, and canonical SuperTick stepping.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::engine_impl::{CommitOutcome, Engine, EngineError};
use crate::head::{PlaybackHeadRegistry, RunnableWriterSet, WriterHead, WriterHeadKey};
use crate::head_inbox::{InboxAddress, InboxIngestResult, IngressEnvelope, IngressTarget};
use crate::ident::Hash;
use crate::worldline::WorldlineId;
use crate::worldline_registry::WorldlineRegistry;
use crate::worldline_state::WorldlineState;

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
    pub worldlines: WorldlineRegistry,
    /// Registry of all writer heads.
    pub heads: PlaybackHeadRegistry,
    /// Ordered set of currently runnable (non-paused) writer heads.
    pub runnable: RunnableWriterSet,
    /// Global tick counter (metadata only; not per-worldline identity).
    pub global_tick: u64,
    /// Deterministic route table for default writers.
    default_writers: BTreeMap<WorldlineId, WriterHeadKey>,
    /// Deterministic route table for named public inboxes.
    public_inboxes: BTreeMap<(WorldlineId, InboxAddress), WriterHeadKey>,
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
        if !self.worldlines.register(worldline_id, state) {
            return Err(RuntimeError::DuplicateWorldline(worldline_id));
        }
        Ok(())
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
            let route_key = (key.worldline_id, inbox.clone());
            if self.public_inboxes.contains_key(&route_key) {
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
            self.public_inboxes.insert((key.worldline_id, inbox), key);
        }
        self.heads.insert(head);
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
                .get(&(*worldline_id, inbox.clone()))
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
    pub frontier_tick_after: u64,
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
    pub fn super_tick(
        runtime: &mut WorldlineRuntime,
        engine: &mut Engine,
    ) -> Result<Vec<StepRecord>, RuntimeError> {
        runtime.refresh_runnable();

        let mut records = Vec::new();
        let keys: Vec<WriterHeadKey> = runtime.runnable.iter().copied().collect();

        for key in &keys {
            let admitted = runtime
                .heads
                .inbox_mut(key)
                .ok_or(RuntimeError::UnknownHead(*key))?
                .admit();

            if admitted.is_empty() {
                continue;
            }

            let outcome = if let Some(frontier) = runtime.worldlines.frontier_mut(&key.worldline_id)
            {
                engine.commit_with_state(frontier.state_mut(), &admitted)
            } else {
                runtime
                    .heads
                    .inbox_mut(key)
                    .ok_or(RuntimeError::UnknownHead(*key))?
                    .requeue(admitted);
                return Err(RuntimeError::UnknownWorldline(key.worldline_id));
            };

            let CommitOutcome { snapshot, .. } = match outcome {
                Ok(outcome) => outcome,
                Err(err) => {
                    runtime
                        .heads
                        .inbox_mut(key)
                        .ok_or(RuntimeError::UnknownHead(*key))?
                        .requeue(admitted);
                    return Err(err.into());
                }
            };

            let frontier_tick_after = {
                let frontier = runtime
                    .worldlines
                    .frontier_mut(&key.worldline_id)
                    .ok_or(RuntimeError::UnknownWorldline(key.worldline_id))?;
                frontier.state_mut().record_committed_ingress(
                    *key,
                    admitted.iter().map(IngressEnvelope::ingress_id),
                );
                frontier
                    .advance_tick()
                    .ok_or(RuntimeError::FrontierTickOverflow(key.worldline_id))?
            };

            records.push(StepRecord {
                head_key: *key,
                admitted_count: admitted.len(),
                frontier_tick_after,
                state_root: snapshot.state_root,
                commit_hash: snapshot.hash,
            });
        }

        runtime.global_tick = runtime
            .global_tick
            .checked_add(1)
            .ok_or(RuntimeError::GlobalTickOverflow)?;
        Ok(records)
    }

    /// Returns the canonical ordering of runnable heads without mutating state.
    #[must_use]
    pub fn peek_order(runtime: &WorldlineRuntime) -> Vec<WriterHeadKey> {
        runtime
            .heads
            .iter()
            .filter_map(|(key, head)| (!head.is_paused()).then_some(*key))
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
    use crate::worldline::WorldlineId;
    use crate::{make_node_id, make_type_id, EngineBuilder, GraphStore, NodeRecord};

    fn wl(n: u8) -> WorldlineId {
        WorldlineId([n; 32])
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

        let records = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap();
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
        let records = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap();

        assert_eq!(
            records
                .iter()
                .map(|record| record.head_key)
                .collect::<Vec<_>>(),
            expected_order
        );
        assert!(records.iter().all(|record| record.admitted_count == 1));
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

        let records = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(
            runtime
                .worldlines
                .get(&worldline_a)
                .unwrap()
                .frontier_tick(),
            1
        );
        assert_eq!(
            runtime
                .worldlines
                .get(&worldline_b)
                .unwrap()
                .frontier_tick(),
            1
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

        let records = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap();
        assert!(records.is_empty());
        assert_eq!(
            runtime
                .worldlines
                .get(&worldline_id)
                .unwrap()
                .frontier_tick(),
            0
        );
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

        let first = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].admitted_count, 2);
        assert_eq!(
            runtime
                .heads
                .get(&budget_key)
                .unwrap()
                .inbox()
                .pending_count(),
            1
        );

        let second = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].admitted_count, 1);
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

        let records = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap();
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
            .frontier_tick = u64::MAX;

        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap_err();
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
        runtime.global_tick = u64::MAX;

        let err = SchedulerCoordinator::super_tick(&mut runtime, &mut engine).unwrap_err();
        assert!(matches!(err, RuntimeError::GlobalTickOverflow));
    }
}
