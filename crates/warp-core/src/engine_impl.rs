// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Core rewrite engine implementation.
use std::collections::{BTreeMap, HashMap, HashSet};

use blake3::Hasher;
use thiserror::Error;

use crate::attachment::{AttachmentKey, AttachmentValue};
#[cfg(any(test, feature = "delta_validate"))]
use crate::boaw::merge_deltas;
use crate::boaw::{build_work_units, execute_work_queue, ExecItem, NUM_SHARDS};
use crate::graph::GraphStore;
use crate::graph_view::GraphView;
use crate::ident::{
    make_edge_id, make_node_id, make_type_id, CompactRuleId, Hash, NodeId, NodeKey, WarpId,
};
use crate::inbox::{INBOX_EVENT_TYPE, INBOX_PATH, INTENT_ATTACHMENT_TYPE, PENDING_EDGE_TYPE};
use crate::materialization::{ChannelConflict, FinalizedChannel, MaterializationBus};
use crate::receipt::{TickReceipt, TickReceiptDisposition, TickReceiptEntry, TickReceiptRejection};
use crate::record::NodeRecord;
use crate::rule::{ConflictPolicy, RewriteRule};
use crate::scheduler::{DeterministicScheduler, PendingRewrite, RewritePhase, SchedulerKind};
use crate::snapshot::{compute_commit_hash_v2, compute_state_root, Snapshot};
use crate::telemetry::{NullTelemetrySink, TelemetrySink};
use crate::tick_delta::OpOrigin;
use crate::tick_patch::{diff_state, SlotId, TickCommitStatus, WarpOp, WarpTickPatchV1};
use crate::tx::TxId;
use crate::warp_state::{WarpInstance, WarpState};
use std::sync::Arc;

/// Outcome of calling [`Engine::apply`].
///
/// This is a *match-status* indicator, not a `Result<_, ApplyError>` type alias.
/// `ApplyResult` tells the caller whether the rewrite rule's pattern matched the
/// given scope, independent of any storage-level errors (which are reported via
/// [`EngineError`]).
#[derive(Debug)]
pub enum ApplyResult {
    /// The rewrite matched and was enqueued for execution.
    Applied,
    /// The rewrite did not match the provided scope.
    NoMatch,
}

/// Result of calling [`Engine::ingest_intent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestDisposition {
    /// The intent was already present in the ledger (idempotent retry).
    Duplicate {
        /// Content hash of the canonical intent bytes (`intent_id = H(intent_bytes)`).
        intent_id: Hash,
    },
    /// The intent was accepted and added to the pending inbox set.
    Accepted {
        /// Content hash of the canonical intent bytes (`intent_id = H(intent_bytes)`).
        intent_id: Hash,
    },
}

/// Result of calling [`Engine::dispatch_next_intent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchDisposition {
    /// No pending intent was present in the inbox.
    NoPending,
    /// A pending intent was consumed (pending edge removed).
    ///
    /// `handler_matched` indicates whether a `cmd/*` rule matched the intent in this tick.
    Consumed {
        /// Content hash of the canonical intent bytes (`intent_id = H(intent_bytes)`).
        intent_id: Hash,
        /// Whether a command handler (`cmd/*`) matched and was enqueued.
        handler_matched: bool,
    },
}

/// Errors emitted by the engine.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The supplied transaction identifier did not exist or was already closed.
    #[error("transaction not active")]
    UnknownTx,
    /// A rule was requested that has not been registered with the engine.
    #[error("rule not registered: {0}")]
    UnknownRule(String),
    /// Attempted to register a rule with a duplicate name.
    #[error("duplicate rule name: {0}")]
    DuplicateRuleName(&'static str),
    /// Attempted to register a rule with a duplicate ID.
    #[error("duplicate rule id: {0:?}")]
    DuplicateRuleId(Hash),
    /// Conflict policy Join requires a join function.
    #[error("missing join function for ConflictPolicy::Join")]
    MissingJoinFn,
    /// Internal invariant violated (engine state corruption).
    #[error("internal invariant violated: {0}")]
    InternalCorruption(&'static str),
    /// Attempted to access a warp instance that does not exist.
    #[error("unknown warp instance: {0:?}")]
    UnknownWarp(WarpId),
    /// Tick index is out of bounds (exceeds ledger length).
    #[error("tick index {0} exceeds ledger length {1}")]
    InvalidTickIndex(usize, usize),
}

// ============================================================================
// Engine Builder
// ============================================================================

/// Source for building an engine from a fresh [`GraphStore`].
pub struct FreshStore {
    store: GraphStore,
    root: NodeId,
}

/// Source for building an engine from an existing [`WarpState`].
pub struct ExistingState {
    state: WarpState,
    root: NodeKey,
}

/// Returns the default worker count for parallel execution.
///
/// Precedence:
/// 1. `ECHO_WORKERS` environment variable (if set and valid)
/// 2. `available_parallelism().min(NUM_SHARDS)` (capped at shard count)
///
/// # Environment Variable
///
/// Set `ECHO_WORKERS=N` to override the default. Useful for:
/// - CI environments (deterministic worker count)
/// - Debugging (force serial with `ECHO_WORKERS=1`)
/// - Benchmarking (compare different parallelism levels)
fn default_worker_count() -> usize {
    if let Ok(val) = std::env::var("ECHO_WORKERS") {
        if let Ok(n) = val.parse::<usize>() {
            return n.max(1);
        }
    }
    std::thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(1)
        .min(NUM_SHARDS)
}

/// Fluent builder for constructing [`Engine`] instances.
///
/// Use [`EngineBuilder::new`] to start from a fresh [`GraphStore`], or
/// [`EngineBuilder::from_state`] to start from an existing [`WarpState`].
///
/// # Example
///
/// ```rust
/// use warp_core::{
///     make_node_id, make_type_id, EngineBuilder, GraphStore, NodeRecord, SchedulerKind,
/// };
///
/// let mut store = GraphStore::default();
/// let root = make_node_id("root");
/// store.insert_node(root, NodeRecord { ty: make_type_id("world") });
///
/// let _engine = EngineBuilder::new(store, root)
///     .scheduler(SchedulerKind::Radix)
///     .policy_id(42)
///     .build();
/// ```
///
/// # Dependency Injection
///
/// For testing or custom configurations, you can inject a pre-configured
/// [`MaterializationBus`]:
///
/// ```rust
/// use warp_core::{make_node_id, make_type_id, EngineBuilder, GraphStore, NodeRecord};
/// use warp_core::materialization::{make_channel_id, ChannelPolicy, MaterializationBus};
///
/// let mut store = GraphStore::default();
/// let root = make_node_id("root");
/// store.insert_node(root, NodeRecord { ty: make_type_id("world") });
///
/// let ch = make_channel_id("demo:ch");
/// let mut bus = MaterializationBus::new();
/// bus.register_channel(ch, ChannelPolicy::StrictSingle);
///
/// let _engine = EngineBuilder::new(store, root)
///     .with_materialization_bus(bus)
///     .build();
/// ```
pub struct EngineBuilder<Source> {
    source: Source,
    scheduler: SchedulerKind,
    policy_id: u32,
    worker_count: usize,
    telemetry: Option<Arc<dyn TelemetrySink>>,
    bus: Option<MaterializationBus>,
}

impl EngineBuilder<FreshStore> {
    /// Creates a builder for a new engine with the given store and root node.
    ///
    /// Defaults:
    /// - Scheduler: [`SchedulerKind::Radix`]
    /// - Policy ID: [`crate::POLICY_ID_NO_POLICY_V0`]
    /// - Worker count: `default_worker_count()` (env `ECHO_WORKERS` or `available_parallelism`)
    /// - Telemetry: [`NullTelemetrySink`]
    /// - `MaterializationBus`: A fresh bus with no pre-registered channels
    pub fn new(store: GraphStore, root: NodeId) -> Self {
        Self {
            source: FreshStore { store, root },
            scheduler: SchedulerKind::Radix,
            policy_id: crate::POLICY_ID_NO_POLICY_V0,
            worker_count: default_worker_count(),
            telemetry: None,
            bus: None,
        }
    }

    /// Builds the engine. This operation is infallible for fresh stores.
    #[must_use]
    pub fn build(self) -> Engine {
        let telemetry = self
            .telemetry
            .unwrap_or_else(|| Arc::new(NullTelemetrySink));
        let bus = self.bus.unwrap_or_default();
        Engine::with_telemetry_bus_and_workers(
            self.source.store,
            self.source.root,
            self.scheduler,
            self.policy_id,
            telemetry,
            bus,
            self.worker_count,
        )
    }
}

impl EngineBuilder<ExistingState> {
    /// Creates a builder for an engine from an existing [`WarpState`].
    ///
    /// Defaults:
    /// - Scheduler: [`SchedulerKind::Radix`]
    /// - Policy ID: [`crate::POLICY_ID_NO_POLICY_V0`]
    /// - Worker count: `default_worker_count()` (env `ECHO_WORKERS` or `available_parallelism`)
    /// - Telemetry: [`NullTelemetrySink`]
    /// - `MaterializationBus`: A fresh bus with no pre-registered channels
    pub fn from_state(state: WarpState, root: NodeKey) -> Self {
        Self {
            source: ExistingState { state, root },
            scheduler: SchedulerKind::Radix,
            policy_id: crate::POLICY_ID_NO_POLICY_V0,
            worker_count: default_worker_count(),
            telemetry: None,
            bus: None,
        }
    }

    /// Builds the engine.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownWarp`] if the root warp instance is missing.
    /// Returns [`EngineError::InternalCorruption`] if the root is invalid.
    pub fn build(self) -> Result<Engine, EngineError> {
        let telemetry = self
            .telemetry
            .unwrap_or_else(|| Arc::new(NullTelemetrySink));
        let bus = self.bus.unwrap_or_default();
        Engine::with_state_telemetry_bus_and_workers(
            self.source.state,
            self.source.root,
            self.scheduler,
            self.policy_id,
            telemetry,
            bus,
            self.worker_count,
        )
    }
}

impl<S> EngineBuilder<S> {
    /// Sets the scheduler variant.
    #[must_use]
    pub fn scheduler(mut self, kind: SchedulerKind) -> Self {
        self.scheduler = kind;
        self
    }

    /// Sets the policy identifier.
    ///
    /// The policy ID is committed into `patch_digest` and `commit_id` v2.
    #[must_use]
    pub fn policy_id(mut self, id: u32) -> Self {
        self.policy_id = id;
        self
    }

    /// Sets the worker count for parallel execution.
    ///
    /// Default: `default_worker_count()` (env `ECHO_WORKERS` or `available_parallelism`).
    ///
    /// # Notes
    ///
    /// - Workers are capped at `NUM_SHARDS` (256) internally
    /// - Values less than 1 are treated as 1
    /// - Use `ECHO_WORKERS=1` environment variable to force serial execution for debugging
    #[must_use]
    pub fn workers(mut self, n: usize) -> Self {
        self.worker_count = n.max(1);
        self
    }

    /// Sets the telemetry sink for observability events.
    #[must_use]
    pub fn telemetry(mut self, sink: Arc<dyn TelemetrySink>) -> Self {
        self.telemetry = Some(sink);
        self
    }

    /// Injects a custom [`MaterializationBus`] for dependency injection.
    ///
    /// This is useful for:
    /// - Testing: Pre-registering channels with specific policies
    /// - Custom configurations: Setting up channel policies before engine construction
    ///
    /// If not called, a fresh bus with no pre-registered channels is created.
    ///
    /// # Example
    ///
    /// ```rust
    /// use warp_core::{make_node_id, make_type_id, EngineBuilder, GraphStore, NodeRecord};
    /// use warp_core::materialization::{make_channel_id, ChannelPolicy, MaterializationBus};
    ///
    /// let mut store = GraphStore::default();
    /// let root = make_node_id("root");
    /// store.insert_node(root, NodeRecord { ty: make_type_id("world") });
    ///
    /// let ch = make_channel_id("demo:ch");
    /// let mut bus = MaterializationBus::new();
    /// bus.register_channel(ch, ChannelPolicy::StrictSingle);
    ///
    /// let _engine = EngineBuilder::new(store, root)
    ///     .with_materialization_bus(bus)
    ///     .build();
    /// ```
    #[must_use]
    pub fn with_materialization_bus(mut self, bus: MaterializationBus) -> Self {
        self.bus = Some(bus);
        self
    }
}

// ============================================================================
// Engine
// ============================================================================

/// Core rewrite engine used by the spike.
///
/// It owns a `GraphStore`, the registered rules, and the deterministic
/// scheduler. Snapshot determinism is provided by the snapshot hashing routine:
/// includes the root id, all nodes in ascending `NodeId` order, and all
/// outbound edges per node sorted by `EdgeId`. All length prefixes are 8-byte
/// little-endian and ids are raw 32-byte values. Changing any of these rules is
/// a breaking change to snapshot identity and must be recorded in the
/// determinism spec and tests.
///
/// # Construction
///
/// Use [`EngineBuilder`] for fluent configuration:
///
/// ```rust
/// use warp_core::{
///     make_node_id, make_type_id, EngineBuilder, GraphStore, NodeRecord, SchedulerKind,
/// };
///
/// let mut store = GraphStore::default();
/// let root = make_node_id("root");
/// store.insert_node(root, NodeRecord { ty: make_type_id("world") });
///
/// let _engine = EngineBuilder::new(store, root)
///     .scheduler(SchedulerKind::Radix)
///     .policy_id(42)
///     .build();
/// ```
///
/// For testing or custom configurations, inject a pre-configured
/// [`MaterializationBus`]:
///
/// ```rust
/// use warp_core::{make_node_id, make_type_id, EngineBuilder, GraphStore, NodeRecord};
/// use warp_core::materialization::{make_channel_id, ChannelPolicy, MaterializationBus};
///
/// let mut store = GraphStore::default();
/// let root = make_node_id("root");
/// store.insert_node(root, NodeRecord { ty: make_type_id("world") });
///
/// let ch = make_channel_id("demo:ch");
/// let mut bus = MaterializationBus::new();
/// bus.register_channel(ch, ChannelPolicy::StrictSingle);
///
/// let _engine = EngineBuilder::new(store, root)
///     .with_materialization_bus(bus)
///     .build();
/// ```
///
/// Legacy constructors are also available for backward compatibility.
pub struct Engine {
    state: WarpState,
    rules: HashMap<&'static str, RewriteRule>,
    rules_by_id: HashMap<Hash, &'static str>,
    compact_rule_ids: HashMap<Hash, CompactRuleId>,
    rules_by_compact: HashMap<CompactRuleId, &'static str>,
    scheduler: DeterministicScheduler,
    /// Policy identifier committed into `patch_digest` (tick patches) and
    /// `commit_id` (commit hash v2).
    ///
    /// This is part of the deterministic boundary. Callers select it explicitly
    /// via constructors like [`Engine::with_policy_id`].
    policy_id: u32,
    /// Worker count for parallel execution.
    ///
    /// Capped at `NUM_SHARDS` internally. Use [`EngineBuilder::workers`] to override
    /// the default (which respects `ECHO_WORKERS` env var).
    worker_count: usize,
    tx_counter: u64,
    live_txs: HashSet<u64>,
    current_root: NodeKey,
    last_snapshot: Option<Snapshot>,
    /// Sequential history of all committed ticks (Snapshot, Receipt, Patch).
    tick_history: Vec<(Snapshot, TickReceipt, WarpTickPatchV1)>,
    intent_log: Vec<(u64, crate::attachment::AtomPayload)>,
    /// Initial state (U0) snapshot preserved for replay via `jump_to_tick`.
    initial_state: WarpState,
    /// Materialization bus for tick-scoped channel emissions.
    ///
    /// Rules emit to channels via [`ScopedEmitter`](crate::materialization::ScopedEmitter) during execution. The bus
    /// collects emissions and finalizes them post-commit according to each
    /// channel's policy.
    bus: MaterializationBus,
    /// Last finalized materialization channels (populated by commit, cleared by abort).
    last_materialization: Vec<FinalizedChannel>,
    /// Materialization errors from the last commit (e.g., `StrictSingle` conflicts).
    ///
    /// This is populated alongside `last_materialization` by [`Engine::commit_with_receipt`].
    /// A non-empty list indicates boundary errors: state committed successfully, but
    /// some materialization channels failed to finalize.
    last_materialization_errors: Vec<ChannelConflict>,
}

struct ReserveOutcome {
    receipt: TickReceipt,
    reserved: Vec<PendingRewrite>,
    in_slots: std::collections::BTreeSet<SlotId>,
    out_slots: std::collections::BTreeSet<SlotId>,
}

impl Engine {
    /// Constructs a new engine with the supplied backing store and root node id.
    ///
    /// Uses the default scheduler (Radix) and the default policy id
    /// [`crate::POLICY_ID_NO_POLICY_V0`].
    pub fn new(store: GraphStore, root: NodeId) -> Self {
        Self::with_scheduler_and_policy_id(
            store,
            root,
            SchedulerKind::Radix,
            crate::POLICY_ID_NO_POLICY_V0,
        )
    }

    /// Constructs a new engine with an explicit scheduler kind (radix vs. legacy).
    ///
    /// Uses the default policy id [`crate::POLICY_ID_NO_POLICY_V0`].
    pub fn with_scheduler(store: GraphStore, root: NodeId, kind: SchedulerKind) -> Self {
        Self::with_scheduler_and_policy_id(store, root, kind, crate::POLICY_ID_NO_POLICY_V0)
    }

    /// Constructs a new engine with an explicit policy identifier.
    ///
    /// `policy_id` is committed into both `patch_digest` (tick patches) and
    /// `commit_id` (commit hash v2). Callers must treat it as part of the
    /// deterministic boundary.
    pub fn with_policy_id(store: GraphStore, root: NodeId, policy_id: u32) -> Self {
        Self::with_scheduler_and_policy_id(store, root, SchedulerKind::Radix, policy_id)
    }

    /// Constructs a new engine with explicit scheduler kind and policy identifier.
    ///
    /// Uses a null telemetry sink (events are discarded).
    ///
    /// # Parameters
    /// - `store`: Backing graph store.
    ///   The supplied store is assigned to the canonical root warp instance; any pre-existing
    ///   `warp_id` on the store is overwritten.
    /// - `root`: Root node id for snapshot hashing.
    /// - `kind`: Scheduler variant (Radix vs Legacy).
    /// - `policy_id`: Policy identifier committed into `patch_digest` and `commit_id` v2.
    pub fn with_scheduler_and_policy_id(
        store: GraphStore,
        root: NodeId,
        kind: SchedulerKind,
        policy_id: u32,
    ) -> Self {
        Self::with_telemetry(store, root, kind, policy_id, Arc::new(NullTelemetrySink))
    }

    /// Constructs a new engine with explicit telemetry sink.
    ///
    /// This constructor delegates to [`Engine::with_telemetry_and_bus`] with a fresh bus.
    ///
    /// # Parameters
    /// - `store`: Backing graph store.
    ///   The supplied store is assigned to the canonical root warp instance; any pre-existing
    ///   `warp_id` on the store is overwritten.
    /// - `root`: Root node id for snapshot hashing.
    /// - `kind`: Scheduler variant (Radix vs Legacy).
    /// - `policy_id`: Policy identifier committed into `patch_digest` and `commit_id` v2.
    /// - `telemetry`: Telemetry sink for observability events.
    pub fn with_telemetry(
        store: GraphStore,
        root: NodeId,
        kind: SchedulerKind,
        policy_id: u32,
        telemetry: Arc<dyn TelemetrySink>,
    ) -> Self {
        Self::with_telemetry_and_bus(
            store,
            root,
            kind,
            policy_id,
            telemetry,
            MaterializationBus::new(),
        )
    }

    /// Constructs a new engine with explicit telemetry sink and materialization bus.
    ///
    /// This constructor delegates to [`Engine::with_telemetry_bus_and_workers`] with
    /// the default worker count.
    ///
    /// # Parameters
    /// - `store`: Backing graph store.
    ///   The supplied store is assigned to the canonical root warp instance; any pre-existing
    ///   `warp_id` on the store is overwritten.
    /// - `root`: Root node id for snapshot hashing.
    /// - `kind`: Scheduler variant (Radix vs Legacy).
    /// - `policy_id`: Policy identifier committed into `patch_digest` and `commit_id` v2.
    /// - `telemetry`: Telemetry sink for observability events.
    /// - `bus`: Pre-configured materialization bus for dependency injection.
    pub fn with_telemetry_and_bus(
        store: GraphStore,
        root: NodeId,
        kind: SchedulerKind,
        policy_id: u32,
        telemetry: Arc<dyn TelemetrySink>,
        bus: MaterializationBus,
    ) -> Self {
        Self::with_telemetry_bus_and_workers(
            store,
            root,
            kind,
            policy_id,
            telemetry,
            bus,
            default_worker_count(),
        )
    }

    /// Constructs a new engine with explicit telemetry sink, materialization bus, and worker count.
    ///
    /// This is the canonical constructor; all other constructors delegate here.
    ///
    /// # Parameters
    /// - `store`: Backing graph store.
    ///   The supplied store is assigned to the canonical root warp instance; any pre-existing
    ///   `warp_id` on the store is overwritten.
    /// - `root`: Root node id for snapshot hashing.
    /// - `kind`: Scheduler variant (Radix vs Legacy).
    /// - `policy_id`: Policy identifier committed into `patch_digest` and `commit_id` v2.
    /// - `telemetry`: Telemetry sink for observability events.
    /// - `bus`: Pre-configured materialization bus for dependency injection.
    /// - `worker_count`: Number of workers for parallel execution (capped at `NUM_SHARDS`).
    pub fn with_telemetry_bus_and_workers(
        store: GraphStore,
        root: NodeId,
        kind: SchedulerKind,
        policy_id: u32,
        telemetry: Arc<dyn TelemetrySink>,
        bus: MaterializationBus,
        worker_count: usize,
    ) -> Self {
        // NOTE: The supplied `GraphStore` is assigned to the canonical root warp instance.
        // Any pre-existing `warp_id` on the store is overwritten.
        let root_warp = crate::ident::make_warp_id("root");
        let mut state = WarpState::new();
        let mut store = store;
        store.warp_id = root_warp;
        state.upsert_instance(
            WarpInstance {
                warp_id: root_warp,
                root_node: root,
                parent: None,
            },
            store,
        );
        // Preserve the initial state (U0) for replay via `jump_to_tick`.
        let initial_state = state.clone();
        Self {
            state,
            rules: HashMap::new(),
            rules_by_id: HashMap::new(),
            compact_rule_ids: HashMap::new(),
            rules_by_compact: HashMap::new(),
            scheduler: DeterministicScheduler::new(kind, telemetry),
            policy_id,
            worker_count: worker_count.clamp(1, NUM_SHARDS),
            tx_counter: 0,
            live_txs: HashSet::new(),
            current_root: NodeKey {
                warp_id: root_warp,
                local_id: root,
            },
            last_snapshot: None,
            tick_history: Vec::new(),
            intent_log: Vec::new(),
            initial_state,
            bus,
            last_materialization: Vec::new(),
            last_materialization_errors: Vec::new(),
        }
    }

    /// Constructs an engine from an existing multi-instance [`WarpState`] (Stage B1).
    ///
    /// This constructor is primarily intended for:
    /// - replaying a sequence of tick patches into a `WarpState`, then continuing execution,
    /// - building multi-instance fixtures for tests/tools without exposing `WarpState` internals, and
    /// - running rules against an externally-authored state (imported or synthesized).
    ///
    /// **Important**: `Engine::with_state` initializes a clean execution environment:
    /// the scheduler starts empty (no pending rewrites), there are no live transactions,
    /// and `tx_counter` is reset to `0`. Transaction/scheduler state from any original
    /// execution is intentionally not preserved.
    ///
    /// # Parameters
    /// - `state`: pre-constructed multi-instance state.
    /// - `root`: the root node for snapshot hashing and commits. This must refer to the root instance.
    /// - `kind`: scheduler variant (Radix vs Legacy).
    /// - `policy_id`: policy identifier committed into `patch_digest` and `commit_id` v2.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownWarp`] if the root warp instance is not present
    /// (missing store or missing instance metadata).
    ///
    /// Returns [`EngineError::InternalCorruption`] if the supplied `root` does not
    /// match the root instance metadata (`WarpInstance.root_node`), or if the root
    /// instance declares a `parent` (root instances must have `parent = None`).
    pub fn with_state(
        state: WarpState,
        root: NodeKey,
        kind: SchedulerKind,
        policy_id: u32,
    ) -> Result<Self, EngineError> {
        let Some(root_instance) = state.instance(&root.warp_id) else {
            return Err(EngineError::UnknownWarp(root.warp_id));
        };
        if root_instance.parent.is_some() {
            return Err(EngineError::InternalCorruption(
                "root warp instance must not declare a parent",
            ));
        }
        if root_instance.root_node != root.local_id {
            return Err(EngineError::InternalCorruption(
                "Engine root must match WarpInstance.root_node",
            ));
        }
        if state.store(&root.warp_id).is_none() {
            return Err(EngineError::UnknownWarp(root.warp_id));
        }

        Self::with_state_and_telemetry(state, root, kind, policy_id, Arc::new(NullTelemetrySink))
    }

    /// Constructs an engine from an existing multi-instance [`WarpState`] with telemetry.
    ///
    /// This constructor delegates to [`Engine::with_state_telemetry_and_bus`] with a fresh bus.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownWarp`] if the root warp ID is not present in the state.
    /// Returns [`EngineError::InternalCorruption`] if the root instance declares a parent
    /// or if the root node does not match the instance's `root_node`.
    pub fn with_state_and_telemetry(
        state: WarpState,
        root: NodeKey,
        kind: SchedulerKind,
        policy_id: u32,
        telemetry: Arc<dyn TelemetrySink>,
    ) -> Result<Self, EngineError> {
        Self::with_state_telemetry_and_bus(
            state,
            root,
            kind,
            policy_id,
            telemetry,
            MaterializationBus::new(),
        )
    }

    /// Constructs an engine from an existing multi-instance [`WarpState`] with telemetry and bus.
    ///
    /// This constructor delegates to [`Engine::with_state_telemetry_bus_and_workers`] with
    /// the default worker count.
    ///
    /// # Parameters
    /// - `state`: Pre-constructed multi-instance state.
    /// - `root`: The root node for snapshot hashing and commits.
    /// - `kind`: Scheduler variant (Radix vs Legacy).
    /// - `policy_id`: Policy identifier committed into `patch_digest` and `commit_id` v2.
    /// - `telemetry`: Telemetry sink for observability events.
    /// - `bus`: Pre-configured materialization bus for dependency injection.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownWarp`] if the root warp ID is not present in the state.
    /// Returns [`EngineError::InternalCorruption`] if the root instance declares a parent
    /// or if the root node does not match the instance's `root_node`.
    pub fn with_state_telemetry_and_bus(
        state: WarpState,
        root: NodeKey,
        kind: SchedulerKind,
        policy_id: u32,
        telemetry: Arc<dyn TelemetrySink>,
        bus: MaterializationBus,
    ) -> Result<Self, EngineError> {
        Self::with_state_telemetry_bus_and_workers(
            state,
            root,
            kind,
            policy_id,
            telemetry,
            bus,
            default_worker_count(),
        )
    }

    /// Constructs an engine from an existing multi-instance [`WarpState`] with telemetry, bus, and worker count.
    ///
    /// This is the canonical constructor for existing state; [`Engine::with_state`] and
    /// [`Engine::with_state_and_telemetry`] delegate here.
    ///
    /// # Parameters
    /// - `state`: Pre-constructed multi-instance state.
    /// - `root`: The root node for snapshot hashing and commits.
    /// - `kind`: Scheduler variant (Radix vs Legacy).
    /// - `policy_id`: Policy identifier committed into `patch_digest` and `commit_id` v2.
    /// - `telemetry`: Telemetry sink for observability events.
    /// - `bus`: Pre-configured materialization bus for dependency injection.
    /// - `worker_count`: Number of workers for parallel execution (capped at `NUM_SHARDS`).
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownWarp`] if the root warp ID is not present in the state.
    /// Returns [`EngineError::InternalCorruption`] if the root instance declares a parent
    /// or if the root node does not match the instance's `root_node`.
    pub fn with_state_telemetry_bus_and_workers(
        state: WarpState,
        root: NodeKey,
        kind: SchedulerKind,
        policy_id: u32,
        telemetry: Arc<dyn TelemetrySink>,
        bus: MaterializationBus,
        worker_count: usize,
    ) -> Result<Self, EngineError> {
        let Some(root_instance) = state.instance(&root.warp_id) else {
            return Err(EngineError::UnknownWarp(root.warp_id));
        };
        if root_instance.parent.is_some() {
            return Err(EngineError::InternalCorruption(
                "root warp instance must not declare a parent",
            ));
        }
        if root_instance.root_node != root.local_id {
            return Err(EngineError::InternalCorruption(
                "Engine root must match WarpInstance.root_node",
            ));
        }
        if state.store(&root.warp_id).is_none() {
            return Err(EngineError::UnknownWarp(root.warp_id));
        }

        // Preserve the initial state (U0) for replay via `jump_to_tick`.
        let initial_state = state.clone();
        Ok(Self {
            state,
            rules: HashMap::new(),
            rules_by_id: HashMap::new(),
            compact_rule_ids: HashMap::new(),
            rules_by_compact: HashMap::new(),
            scheduler: DeterministicScheduler::new(kind, telemetry),
            policy_id,
            worker_count: worker_count.clamp(1, NUM_SHARDS),
            tx_counter: 0,
            live_txs: HashSet::new(),
            current_root: root,
            last_snapshot: None,
            tick_history: Vec::new(),
            intent_log: Vec::new(),
            initial_state,
            bus,
            last_materialization: Vec::new(),
            last_materialization_errors: Vec::new(),
        })
    }

    /// Registers a rewrite rule so it can be referenced by name.
    ///
    /// # Errors
    /// Returns [`EngineError::DuplicateRuleName`] if a rule with the same
    /// name has already been registered, or [`EngineError::DuplicateRuleId`]
    /// if a rule with the same id was previously registered.
    pub fn register_rule(&mut self, rule: RewriteRule) -> Result<(), EngineError> {
        if self.rules.contains_key(rule.name) {
            return Err(EngineError::DuplicateRuleName(rule.name));
        }
        if self.rules_by_id.contains_key(&rule.id) {
            return Err(EngineError::DuplicateRuleId(rule.id));
        }
        if matches!(rule.conflict_policy, ConflictPolicy::Join) && rule.join_fn.is_none() {
            return Err(EngineError::MissingJoinFn);
        }
        self.rules_by_id.insert(rule.id, rule.name);
        debug_assert!(
            self.compact_rule_ids.len() < u32::MAX as usize,
            "too many rules to assign a compact id"
        );
        #[allow(clippy::cast_possible_truncation)]
        let next = CompactRuleId(self.compact_rule_ids.len() as u32);
        let compact = *self.compact_rule_ids.entry(rule.id).or_insert(next);
        self.rules_by_compact.insert(compact, rule.name);
        self.rules.insert(rule.name, rule);
        Ok(())
    }

    /// Begins a new transaction and returns its identifier.
    #[must_use]
    pub fn begin(&mut self) -> TxId {
        // Increment with wrap and ensure we never produce 0 (reserved invalid).
        self.tx_counter = self.tx_counter.wrapping_add(1);
        if self.tx_counter == 0 {
            self.tx_counter = 1;
        }
        self.live_txs.insert(self.tx_counter);
        TxId::from_raw(self.tx_counter)
    }

    /// Queues a rewrite for execution if it matches the provided scope.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownTx`] if the transaction is invalid, or
    /// [`EngineError::UnknownRule`] if the named rule is not registered.
    ///
    /// # Panics
    /// Panics only if internal rule tables are corrupted (should not happen
    /// when rules are registered via `register_rule`).
    pub fn apply(
        &mut self,
        tx: TxId,
        rule_name: &str,
        scope: &NodeId,
    ) -> Result<ApplyResult, EngineError> {
        self.apply_in_warp(tx, self.current_root.warp_id, rule_name, scope, &[])
    }

    /// Queues a rewrite for execution within a specific warp instance.
    ///
    /// `descent_stack` is the chain of attachment slots (root → … → current)
    /// that establishes reachability for this instance. For Stage B1
    /// determinism, any match/exec inside a descended instance must record
    /// READs of every `AttachmentKey` in this stack so that changing a descent
    /// pointer deterministically invalidates the match.
    ///
    /// # Errors
    /// Returns:
    /// - [`EngineError::UnknownTx`] if `tx` is invalid or already closed.
    /// - [`EngineError::UnknownRule`] if `rule_name` was not registered.
    /// - [`EngineError::UnknownWarp`] if `warp_id` does not exist.
    /// - [`EngineError::InternalCorruption`] if an internal rule table invariant is violated
    ///   (should not occur when using the public registration APIs).
    pub fn apply_in_warp(
        &mut self,
        tx: TxId,
        warp_id: WarpId,
        rule_name: &str,
        scope: &NodeId,
        descent_stack: &[AttachmentKey],
    ) -> Result<ApplyResult, EngineError> {
        if tx.value() == 0 || !self.live_txs.contains(&tx.value()) {
            return Err(EngineError::UnknownTx);
        }
        let Some(rule) = self.rules.get(rule_name) else {
            return Err(EngineError::UnknownRule(rule_name.to_owned()));
        };
        let Some(store) = self.state.store(&warp_id) else {
            return Err(EngineError::UnknownWarp(warp_id));
        };
        let view = GraphView::new(store);
        let matches = (rule.matcher)(view, scope);
        if !matches {
            return Ok(ApplyResult::NoMatch);
        }

        let scope_key = NodeKey {
            warp_id,
            local_id: *scope,
        };
        let scope_fp = scope_hash(&rule.id, &scope_key);
        let mut footprint = (rule.compute_footprint)(view, scope);
        // Stage B1 law: any match/exec inside a descended instance must READ
        // every attachment slot in the descent chain.
        for key in descent_stack {
            footprint.a_read.insert(*key);
        }
        let Some(&compact_rule) = self.compact_rule_ids.get(&rule.id) else {
            return Err(EngineError::InternalCorruption(
                "missing compact rule id for a registered rule",
            ));
        };
        self.scheduler.enqueue(
            tx,
            PendingRewrite {
                rule_id: rule.id,
                compact_rule,
                scope_hash: scope_fp,
                scope: scope_key,
                footprint,
                phase: RewritePhase::Matched,
            },
        );

        Ok(ApplyResult::Applied)
    }

    /// Executes all pending rewrites for the transaction and produces a snapshot.
    ///
    /// # Errors
    /// - Returns [`EngineError::UnknownTx`] if `tx` does not refer to a live transaction.
    /// - Returns [`EngineError::InternalCorruption`] if internal rule tables are
    ///   corrupted (e.g., a reserved rewrite references a missing rule).
    pub fn commit(&mut self, tx: TxId) -> Result<Snapshot, EngineError> {
        let (snapshot, _receipt, _patch) = self.commit_with_receipt(tx)?;
        Ok(snapshot)
    }

    /// Executes all pending rewrites for the transaction, producing both a snapshot and a tick receipt.
    ///
    /// The receipt records (in canonical plan order) which candidates were accepted vs rejected.
    /// For rejected candidates, it also records which earlier applied candidates blocked them
    /// (a minimal blocking-causality witness / poset edge list, per Paper II).
    ///
    /// This method also produces a delta tick patch (Paper III): a replayable boundary artifact
    /// whose digest is committed into the v2 commit hash.
    ///
    /// # Errors
    /// - Returns [`EngineError::UnknownTx`] if `tx` does not refer to a live transaction.
    /// - Returns [`EngineError::InternalCorruption`] if internal rule tables are
    ///   corrupted (e.g., a reserved rewrite references a missing rule).
    ///
    /// # Panics
    ///
    /// Panics if delta validation is enabled and the `SnapshotAccumulator` produces
    /// a different `state_root` than the legacy computation.
    pub fn commit_with_receipt(
        &mut self,
        tx: TxId,
    ) -> Result<(Snapshot, TickReceipt, WarpTickPatchV1), EngineError> {
        if tx.value() == 0 || !self.live_txs.contains(&tx.value()) {
            return Err(EngineError::UnknownTx);
        }
        let policy_id = self.policy_id;
        let rule_pack_id = self.compute_rule_pack_id();
        // Drain pending to form the ready set and compute a plan digest over its canonical order.
        let drained = self.scheduler.drain_for_tx(tx);
        let plan_digest = compute_plan_digest(&drained);

        let ReserveOutcome {
            receipt,
            reserved: reserved_rewrites,
            in_slots,
            out_slots,
        } = self.reserve_for_receipt(tx, drained)?;

        // Deterministic digest of the ordered rewrites we will apply.
        let rewrites_digest = compute_rewrites_digest(&reserved_rewrites);

        // Capture pre-state for delta patch construction.
        // PERF: Full state clone; consider COW or incremental tracking for large graphs.
        let state_before = self.state.clone();

        #[cfg(feature = "delta_validate")]
        let delta_ops = self.apply_reserved_rewrites(reserved_rewrites, &state_before)?;

        #[cfg(all(any(test, feature = "delta_validate"), not(feature = "delta_validate")))]
        let _ = self.apply_reserved_rewrites(reserved_rewrites, &state_before)?;

        #[cfg(not(any(test, feature = "delta_validate")))]
        self.apply_reserved_rewrites(reserved_rewrites)?;

        // Finalize materialization bus and store results.
        // Note: Rules don't emit yet (requires executor signature change), but the
        // bus is wired in and ready. When rules do emit, this will capture their output.
        //
        // The FinalizeReport partitions channels into successes and errors:
        // - `channels`: Successfully finalized outputs
        // - `errors`: Channels that failed (e.g., StrictSingle conflicts)
        //
        // We store both. A non-empty `errors` list indicates boundary errors:
        // state committed successfully, but some materialization channels failed.
        // Callers can inspect `last_materialization_errors()` to handle this.
        let mat_report = self.bus.finalize();
        self.last_materialization = mat_report.channels;
        self.last_materialization_errors = mat_report.errors;

        // Delta tick patch (Paper III boundary artifact).
        let ops = diff_state(&state_before, &self.state);
        let patch = WarpTickPatchV1::new(
            policy_id,
            rule_pack_id,
            TickCommitStatus::Committed,
            in_slots.into_iter().collect(),
            out_slots.into_iter().collect(),
            ops,
        );
        let patch_digest = patch.digest();

        let state_root = crate::snapshot::compute_state_root(&self.state, &self.current_root);

        #[cfg(feature = "delta_validate")]
        {
            use crate::snapshot_accum::SnapshotAccumulator;

            let mut accumulator = SnapshotAccumulator::from_warp_state(&state_before);
            accumulator.apply_ops(delta_ops);

            // Use placeholder values for schema_hash and tick since WSC bytes aren't used yet
            let schema_hash = [0u8; 32];
            let tick = tx.value();

            let accum_output = accumulator.build(&self.current_root, schema_hash, tick);

            assert_eq!(
                state_root, accum_output.state_root,
                "SnapshotAccumulator state_root mismatch: legacy={:?} vs accumulator={:?}",
                state_root, accum_output.state_root
            );
        }

        let parents: Vec<Hash> = self
            .last_snapshot
            .as_ref()
            .map(|s| vec![s.hash])
            .unwrap_or_default();
        // `decision_digest` is reserved for Aion tie-breaks; in the spike we use the tick receipt digest
        // to commit to accepted/rejected decisions in a deterministic way.
        let decision_digest: Hash = receipt.digest();
        let hash = crate::snapshot::compute_commit_hash_v2(
            &state_root,
            &parents,
            &patch_digest,
            policy_id,
        );
        let snapshot = Snapshot {
            root: self.current_root,
            hash,
            state_root,
            parents,
            plan_digest,
            decision_digest,
            rewrites_digest,
            patch_digest,
            policy_id,
            tx,
        };
        self.last_snapshot = Some(snapshot.clone());
        self.tick_history
            .push((snapshot.clone(), receipt.clone(), patch.clone()));
        // Mark transaction as closed/inactive and finalize scheduler accounting.
        self.live_txs.remove(&tx.value());
        self.scheduler.finalize_tx(tx);
        Ok((snapshot, receipt, patch))
    }

    /// Aborts a transaction without committing a tick.
    ///
    /// This closes the transaction and releases any resources reserved in the scheduler.
    /// Also clears pending materialization emissions (`bus`), as well as the cached results
    /// from the previous successful commit (`last_materialization` and `last_materialization_errors`).
    /// This invalidation ensures that stale materialization state is not observed after an abort.
    pub fn abort(&mut self, tx: TxId) {
        self.live_txs.remove(&tx.value());
        self.scheduler.finalize_tx(tx);
        self.bus.clear();
        self.last_materialization.clear();
        self.last_materialization_errors.clear();
    }

    fn reserve_for_receipt(
        &mut self,
        tx: TxId,
        drained: Vec<PendingRewrite>,
    ) -> Result<ReserveOutcome, EngineError> {
        // Reserve phase: enforce independence against active frontier.
        let mut receipt_entries: Vec<TickReceiptEntry> = Vec::with_capacity(drained.len());
        let mut in_slots: std::collections::BTreeSet<SlotId> = std::collections::BTreeSet::new();
        let mut out_slots: std::collections::BTreeSet<SlotId> = std::collections::BTreeSet::new();
        let mut blocked_by: Vec<Vec<u32>> = Vec::with_capacity(drained.len());
        let mut reserved: Vec<PendingRewrite> = Vec::new();
        let mut reserved_entry_indices: Vec<u32> = Vec::new();

        for (entry_idx, mut rewrite) in drained.into_iter().enumerate() {
            let entry_idx_u32 = u32::try_from(entry_idx).map_err(|_| {
                EngineError::InternalCorruption("too many receipt entries to index")
            })?;
            let accepted = self.scheduler.reserve(tx, &mut rewrite);
            let blockers = if accepted {
                Vec::new()
            } else {
                // O(n) scan over reserved rewrites. Acceptable for typical tick sizes;
                // consider spatial indexing if tick candidate counts grow large.
                let mut blockers: Vec<u32> = Vec::new();
                for (k, prior) in reserved.iter().enumerate() {
                    if footprints_conflict(&rewrite.footprint, &prior.footprint) {
                        blockers.push(reserved_entry_indices[k]);
                    }
                }
                if blockers.is_empty() {
                    // `reserve()` currently returns `false` exclusively on footprint
                    // conflicts (see scheduler reserve rustdoc). If additional rejection
                    // reasons are added, update the scheduler contract and this attribution
                    // logic accordingly.
                    return Err(EngineError::InternalCorruption(
                        "scheduler rejected rewrite but no blockers were found",
                    ));
                }
                blockers
            };
            receipt_entries.push(TickReceiptEntry {
                rule_id: rewrite.rule_id,
                scope_hash: rewrite.scope_hash,
                scope: rewrite.scope,
                disposition: if accepted {
                    TickReceiptDisposition::Applied
                } else {
                    // NOTE: reserve() currently returns `false` exclusively on
                    // footprint conflicts (see scheduler reserve rustdoc).
                    // If additional rejection reasons are added, update this mapping.
                    TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict)
                },
            });
            if accepted {
                extend_slots_from_footprint(
                    &mut in_slots,
                    &mut out_slots,
                    &rewrite.scope.warp_id,
                    &rewrite.footprint,
                );
                reserved.push(rewrite);
                reserved_entry_indices.push(entry_idx_u32);
            }
            blocked_by.push(blockers);
        }

        Ok(ReserveOutcome {
            receipt: TickReceipt::new(tx, receipt_entries, blocked_by),
            reserved,
            in_slots,
            out_slots,
        })
    }

    fn apply_reserved_rewrites(
        &mut self,
        rewrites: Vec<PendingRewrite>,
        #[cfg(any(test, feature = "delta_validate"))] state_before: &WarpState,
    ) -> Result<Vec<WarpOp>, EngineError> {
        use crate::tick_patch::WarpTickPatchV1;

        // Defensive guardrail: clamp workers to valid range
        let workers = self.worker_count.clamp(1, NUM_SHARDS);

        // Phase 6 BOAW: Group by warp_id and execute in parallel per warp.
        // BTreeMap ensures deterministic iteration order (WarpId: Ord from [u8; 32]).

        // 1. Pre-validate all rewrites and group by warp_id
        let mut by_warp: BTreeMap<WarpId, Vec<(PendingRewrite, crate::rule::ExecuteFn)>> =
            BTreeMap::new();
        for rewrite in rewrites {
            let id = rewrite.compact_rule;
            let executor = {
                let Some(rule) = self.rule_by_compact(id) else {
                    debug_assert!(false, "missing rule for compact id: {id:?}");
                    return Err(EngineError::InternalCorruption(
                        "missing rule for compact id during commit",
                    ));
                };
                rule.executor
            };
            // Validate store exists for this warp
            if self.state.store(&rewrite.scope.warp_id).is_none() {
                debug_assert!(
                    false,
                    "missing store for warp id: {:?}",
                    rewrite.scope.warp_id
                );
                return Err(EngineError::UnknownWarp(rewrite.scope.warp_id));
            }
            by_warp
                .entry(rewrite.scope.warp_id)
                .or_default()
                .push((rewrite, executor));
        }

        // 2. Convert to ExecItems and build work units (cross-warp parallelism)
        let items_by_warp = by_warp.into_iter().map(|(warp_id, warp_rewrites)| {
            let items: Vec<ExecItem> = warp_rewrites
                .into_iter()
                .map(|(rw, exec)| ExecItem {
                    exec,
                    scope: rw.scope.local_id,
                    origin: OpOrigin::default(),
                })
                .collect();
            (warp_id, items)
        });

        // Build (warp, shard) work units - canonical ordering preserved
        let units = build_work_units(items_by_warp);

        // Cap workers at unit count (no point spawning more threads than work)
        let capped_workers = workers.min(units.len().max(1));

        // Execute all units in parallel across warps (single spawn site)
        // Views resolved per-unit inside threads, dropped before next unit
        let all_deltas =
            execute_work_queue(&units, capped_workers, |warp_id| self.state.store(warp_id))
                .map_err(|_| {
                    EngineError::InternalCorruption(
                        "execute_work_queue: missing store for warp during execution",
                    )
                })?;

        // 3. Merge deltas - use merge_deltas for conflict detection under delta_validate
        #[cfg(any(test, feature = "delta_validate"))]
        let ops = {
            merge_deltas(all_deltas).map_err(|conflict| {
                debug_assert!(false, "merge conflict: {conflict:?}");
                EngineError::InternalCorruption("apply_reserved_rewrites: merge conflict")
            })?
        };

        #[cfg(not(any(test, feature = "delta_validate")))]
        let ops = {
            // Without delta_validate, flatten and sort by sort_key for determinism.
            // Ops with the same sort_key are deduplicated (footprint ensures they're identical).
            let mut flat: Vec<_> = all_deltas
                .into_iter()
                .flat_map(crate::TickDelta::into_ops_unsorted)
                .map(|op| (op.sort_key(), op))
                .collect();

            // Sort by sort_key for canonical order.
            // Use unstable sort for efficiency; equal keys become consecutive for dedup.
            // Unstable sort doesn't preserve input order for equal elements, but since
            // we deduplicate afterwards and the footprint invariant guarantees identical
            // content for ops with the same key, the final output is deterministic.
            flat.sort_unstable_by(|a, b| a.0.cmp(&b.0));

            // Reject conflicting ops with same sort_key in all builds.
            for w in flat.windows(2) {
                if w[0].0 == w[1].0 && w[0].1 != w[1].1 {
                    return Err(EngineError::InternalCorruption(
                        "apply_reserved_rewrites: conflicting ops share sort_key",
                    ));
                }
            }

            flat.dedup_by(|a, b| a.0 == b.0);

            flat.into_iter().map(|(_, op)| op).collect::<Vec<_>>()
        };

        // 4. Apply the merged ops to the state
        let patch = WarpTickPatchV1::new(
            self.policy_id,
            self.compute_rule_pack_id(),
            crate::tick_patch::TickCommitStatus::Committed,
            Vec::new(), // in_slots
            Vec::new(), // out_slots
            ops.clone(),
        );
        patch.apply_to_state(&mut self.state).map_err(|_| {
            EngineError::InternalCorruption("apply_reserved_rewrites: failed to apply ops")
        })?;

        #[cfg(any(test, feature = "delta_validate"))]
        {
            use crate::tick_delta::assert_delta_matches_diff;
            use crate::tick_patch::diff_state;

            let diff_ops = diff_state(state_before, &self.state);
            assert_delta_matches_diff(&ops, &diff_ops);
        }

        Ok(ops)
    }

    /// Returns a snapshot for the current graph state without executing rewrites.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        // Build a lightweight snapshot view of the current state using the
        // same v2 commit hash shape (parents + state_root + patch_digest) but
        // with empty diagnostic digests. This makes it explicit that no rewrites
        // were applied while keeping the structure stable for callers/tools.
        let state_root = compute_state_root(&self.state, &self.current_root);
        let parents: Vec<Hash> = self
            .last_snapshot
            .as_ref()
            .map(|s| vec![s.hash])
            .unwrap_or_default();
        // Canonical empty digests match commit() behaviour when no rewrites are pending.
        let zero_digest: Hash = crate::constants::digest_len0_u64();
        let empty_digest: Hash = zero_digest;
        let decision_empty: Hash = zero_digest;
        let policy_id = self.policy_id;
        let rule_pack_id = self.compute_rule_pack_id();
        let patch_digest = WarpTickPatchV1::new(
            policy_id,
            rule_pack_id,
            TickCommitStatus::Committed,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .digest();
        let hash = compute_commit_hash_v2(&state_root, &parents, &patch_digest, policy_id);
        Snapshot {
            root: self.current_root,
            hash,
            state_root,
            parents,
            plan_digest: empty_digest,
            decision_digest: decision_empty,
            rewrites_digest: empty_digest,
            patch_digest,
            policy_id,
            tx: TxId::from_raw(self.tx_counter),
        }
    }

    /// Returns a cloned view of the current warp's graph store (for tests/tools).
    ///
    /// This is a snapshot-only view; mutations must go through engine APIs.
    ///
    /// # Panics
    ///
    /// Panics if the root warp store doesn't exist, which indicates a bug in
    /// engine construction (the root store should always be present).
    #[must_use]
    #[allow(clippy::expect_used)] // Documented panic: root store missing is a construction bug
    pub fn store_clone(&self) -> GraphStore {
        let warp_id = self.current_root.warp_id;
        self.state
            .store(&warp_id)
            .cloned()
            .expect("root warp store missing - engine construction bug")
    }

    /// Legacy ingest helper: ingests an inbox event from a [`crate::attachment::AtomPayload`].
    ///
    /// This method exists for older call sites that pre-wrap intent bytes in an
    /// atom payload and/or provide an arrival `seq`.
    ///
    /// Canonical semantics:
    /// - `payload.bytes` are treated as `intent_bytes` and forwarded to [`Engine::ingest_intent`].
    /// - `seq` is ignored for identity; event nodes are content-addressed by `intent_id`.
    /// - Invalid intent envelopes are ignored deterministically (no graph mutation).
    ///
    /// For debugging only, the provided `(seq, payload)` is recorded in an in-memory
    /// log when the intent is newly accepted.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownWarp`] if the current warp store is missing.
    pub fn ingest_inbox_event(
        &mut self,
        seq: u64,
        payload: &crate::attachment::AtomPayload,
    ) -> Result<(), EngineError> {
        // Legacy API retained for compatibility with older call sites. The new
        // canonical ingress is content-addressed (`intent_id = H(intent_bytes)`)
        // via [`Engine::ingest_intent`]; the `seq` input is ignored for identity.
        //
        // We still record the provided `seq` in the in-memory intent log for
        // debugging, but it has no effect on graph identity or hashing.
        let intent_bytes = payload.bytes.as_ref();
        let disposition = self.ingest_intent(intent_bytes)?;
        if let IngestDisposition::Accepted { .. } = disposition {
            self.intent_log.push((seq, payload.clone()));
        }
        Ok(())
    }

    /// Ingest a canonical intent envelope (`intent_bytes`) into the runtime inbox.
    ///
    /// This is the causality-first boundary for writes:
    /// - `intent_id = H(intent_bytes)` is computed immediately (domain-separated).
    /// - The event node id is derived from `intent_id` (content-addressed), not arrival order.
    /// - Ingress is idempotent: re-ingesting identical `intent_bytes` returns `Duplicate` and
    ///   does not create additional ledger entries or pending edges.
    ///
    /// Inbox mechanics (pending vs. applied) are tracked via edges:
    /// - While pending, an edge of type `edge:pending` exists from `sim/inbox` to the event node.
    /// - When consumed, the pending edge is deleted as queue maintenance; the event node remains
    ///   as an append-only ledger entry.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownWarp`] if the current warp store is missing.
    pub fn ingest_intent(&mut self, intent_bytes: &[u8]) -> Result<IngestDisposition, EngineError> {
        let intent_id = crate::inbox::compute_intent_id(intent_bytes);
        let event_id = NodeId(intent_id);

        let warp_id = self.current_root.warp_id;
        let store = self
            .state
            .store_mut(&warp_id)
            .ok_or(EngineError::UnknownWarp(warp_id))?;

        let root_id = self.current_root.local_id;

        let sim_id = make_node_id("sim");
        let inbox_id = make_node_id(INBOX_PATH);

        let sim_ty = make_type_id("sim");
        let inbox_ty = make_type_id(INBOX_PATH);
        let event_ty = make_type_id(INBOX_EVENT_TYPE);

        // Structural nodes/edges (idempotent).
        store.insert_node(sim_id, NodeRecord { ty: sim_ty });
        store.insert_node(inbox_id, NodeRecord { ty: inbox_ty });

        store.insert_edge(
            root_id,
            crate::record::EdgeRecord {
                id: make_edge_id("edge:root/sim"),
                from: root_id,
                to: sim_id,
                ty: make_type_id("edge:sim"),
            },
        );
        store.insert_edge(
            sim_id,
            crate::record::EdgeRecord {
                id: make_edge_id("edge:sim/inbox"),
                from: sim_id,
                to: inbox_id,
                ty: make_type_id("edge:inbox"),
            },
        );

        if store.node(&event_id).is_some() {
            return Ok(IngestDisposition::Duplicate { intent_id });
        }

        // Ledger entry: immutable event node keyed by content hash.
        store.insert_node(event_id, NodeRecord { ty: event_ty });
        let payload = crate::attachment::AtomPayload::new(
            make_type_id(INTENT_ATTACHMENT_TYPE),
            bytes::Bytes::copy_from_slice(intent_bytes),
        );
        store.set_node_attachment(event_id, Some(AttachmentValue::Atom(payload)));

        // Pending queue membership (edge id derived from inbox_id + intent_id).
        store.insert_edge(
            inbox_id,
            crate::record::EdgeRecord {
                id: crate::inbox::pending_edge_id(&inbox_id, &intent_id),
                from: inbox_id,
                to: event_id,
                ty: make_type_id(PENDING_EDGE_TYPE),
            },
        );

        Ok(IngestDisposition::Accepted { intent_id })
    }

    /// Returns the number of currently pending intents in `sim/inbox`.
    ///
    /// This counts `edge:pending` edges from the inbox node; ledger nodes are
    /// append-only and are not required to remain reachable once their pending
    /// edge is removed.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownWarp`] if the current warp store is missing.
    pub fn pending_intent_count(&self) -> Result<usize, EngineError> {
        let warp_id = self.current_root.warp_id;
        let store = self
            .state
            .store(&warp_id)
            .ok_or(EngineError::UnknownWarp(warp_id))?;
        let inbox_id = make_node_id(INBOX_PATH);
        let pending_ty = make_type_id(PENDING_EDGE_TYPE);
        Ok(store
            .edges_from(&inbox_id)
            .filter(|e| e.ty == pending_ty)
            .count())
    }

    /// Dispatches exactly one pending intent (if any) in canonical `intent_id` order.
    ///
    /// Canonical ordering is defined by ascending byte order over `intent_id`.
    ///
    /// Mechanically:
    /// - Select the pending event node with the smallest `intent_id`.
    /// - Attempt to enqueue exactly one `cmd/*` rule for that node, using stable
    ///   rule id order as the tie-break when multiple handlers exist.
    /// - Enqueue `sys/ack_pending` to delete the pending edge (queue maintenance).
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownTx`] if `tx` is invalid, or
    /// [`EngineError::UnknownWarp`] if the current warp store is missing.
    pub fn dispatch_next_intent(&mut self, tx: TxId) -> Result<DispatchDisposition, EngineError> {
        let warp_id = self.current_root.warp_id;
        let store = self
            .state
            .store(&warp_id)
            .ok_or(EngineError::UnknownWarp(warp_id))?;
        let inbox_id = make_node_id(INBOX_PATH);
        let pending_ty = make_type_id(PENDING_EDGE_TYPE);

        let mut next: Option<NodeId> = None;
        for edge in store.edges_from(&inbox_id) {
            if edge.ty != pending_ty {
                continue;
            }
            let cand = edge.to;
            next = Some(next.map_or(cand, |current| current.min(cand)));
        }

        let Some(event_id) = next else {
            return Ok(DispatchDisposition::NoPending);
        };

        // Deterministic handler order: rule_id ascending over cmd/* rules.
        let mut cmd_rules: Vec<(Hash, &'static str)> = self
            .rules
            .values()
            .filter(|r| r.name.starts_with("cmd/"))
            .map(|r| (r.id, r.name))
            .collect();
        cmd_rules.sort_unstable_by(|(a_id, a_name), (b_id, b_name)| {
            a_id.cmp(b_id).then_with(|| a_name.cmp(b_name))
        });

        let mut handler_matched = false;
        for (_id, name) in cmd_rules {
            if matches!(self.apply(tx, name, &event_id)?, ApplyResult::Applied) {
                handler_matched = true;
                break;
            }
        }

        // Always consume one pending intent per tick (queue maintenance).
        let _ = self.apply(tx, crate::inbox::ACK_PENDING_RULE_NAME, &event_id)?;

        Ok(DispatchDisposition::Consumed {
            intent_id: event_id.0,
            handler_matched,
        })
    }

    /// Returns the sequence of all committed ticks (Snapshot, Receipt, Patch).
    #[must_use]
    pub fn get_ledger(&self) -> &[(Snapshot, TickReceipt, WarpTickPatchV1)] {
        &self.tick_history
    }

    /// Resets the engine state to the beginning of time (U0) and re-applies all patches
    /// up to and including the specified tick index.
    ///
    /// # Errors
    /// - Returns [`EngineError::InvalidTickIndex`] if `tick_index` exceeds ledger length.
    /// - Returns [`EngineError::InternalCorruption`] if a patch fails to apply.
    pub fn jump_to_tick(&mut self, tick_index: usize) -> Result<(), EngineError> {
        let ledger_len = self.tick_history.len();
        if tick_index >= ledger_len {
            return Err(EngineError::InvalidTickIndex(tick_index, ledger_len));
        }

        // 1. Restore state to the preserved initial state (U0).
        // This ensures patches are replayed against the exact original base,
        // rather than a fresh WarpState which would discard the original U0.
        self.state = self.initial_state.clone();

        // 2. Re-apply patches from index 0 to tick_index
        for i in 0..=tick_index {
            let (_, _, patch) = &self.tick_history[i];
            patch.apply_to_state(&mut self.state).map_err(|_| {
                EngineError::InternalCorruption("failed to replay patch during jump")
            })?;
        }

        Ok(())
    }

    /// Returns a shared view of the current warp state.
    #[must_use]
    pub fn state(&self) -> &WarpState {
        &self.state
    }

    /// Returns a mutable view of the current warp state.
    pub fn state_mut(&mut self) -> &mut WarpState {
        &mut self.state
    }

    /// Returns a shared reference to the materialization bus.
    ///
    /// The bus collects emissions from rewrite rules during a tick. Rules emit
    /// via [`ScopedEmitter`](crate::materialization::ScopedEmitter) adapters that auto-construct [`EmitKey`](crate::materialization::EmitKey)s from
    /// execution context.
    #[must_use]
    pub fn materialization_bus(&self) -> &MaterializationBus {
        &self.bus
    }

    /// Returns a mutable reference to the materialization bus.
    ///
    /// Use this to register channels with custom policies before commit:
    /// ```rust
    /// use warp_core::{make_node_id, make_type_id, EngineBuilder, GraphStore, NodeRecord};
    /// use warp_core::materialization::{make_channel_id, ChannelPolicy};
    ///
    /// let mut store = GraphStore::default();
    /// let root = make_node_id("root");
    /// store.insert_node(root, NodeRecord { ty: make_type_id("world") });
    ///
    /// let mut engine = EngineBuilder::new(store, root).build();
    /// let ch = make_channel_id("demo:ch");
    ///
    /// engine
    ///     .materialization_bus_mut()
    ///     .register_channel(ch, ChannelPolicy::StrictSingle);
    /// ```
    pub fn materialization_bus_mut(&mut self) -> &mut MaterializationBus {
        &mut self.bus
    }

    /// Returns the finalized materialization channels from the last commit.
    ///
    /// This is populated by [`Engine::commit_with_receipt`] and cleared by
    /// [`Engine::abort`]. Returns an empty slice before the first commit.
    #[must_use]
    pub fn last_materialization(&self) -> &[FinalizedChannel] {
        &self.last_materialization
    }

    /// Returns materialization errors from the last commit.
    ///
    /// # WARNING: Callers MUST check this after every commit!
    ///
    /// **Ignoring errors can lead to silent data loss.** When materialization
    /// channels fail (e.g., `StrictSingle` conflicts), the intended output is
    /// discarded. If you don't check for errors, your application may appear
    /// to work while silently dropping critical data.
    ///
    /// A non-empty list indicates boundary errors: the tick committed successfully
    /// (graph state updated, receipt generated), but one or more materialization
    /// channels failed to finalize. Common causes:
    ///
    /// - `StrictSingle` channel received multiple emissions (rule authoring bug)
    ///
    /// This is populated by [`Engine::commit_with_receipt`] and cleared by
    /// [`Engine::abort`]. Returns an empty slice if the last commit had no errors.
    #[must_use]
    pub fn last_materialization_errors(&self) -> &[ChannelConflict] {
        &self.last_materialization_errors
    }

    /// Returns `true` if the last commit had materialization boundary errors.
    ///
    /// # WARNING: Callers MUST check this after every commit!
    ///
    /// **Ignoring errors can lead to silent data loss.** When this returns `true`,
    /// one or more materialization channels failed to produce output. The graph
    /// state committed successfully, but the boundary output is incomplete.
    ///
    /// Typical usage:
    /// ```rust
    /// use warp_core::{make_node_id, make_type_id, EngineBuilder, GraphStore, NodeRecord};
    /// use warp_core::materialization::{make_channel_id, ChannelPolicy};
    ///
    /// # fn main() -> Result<(), warp_core::EngineError> {
    /// let mut store = GraphStore::default();
    /// let root = make_node_id("root");
    /// store.insert_node(root, NodeRecord { ty: make_type_id("world") });
    ///
    /// let mut engine = EngineBuilder::new(store, root).build();
    /// let ch = make_channel_id("demo:ch");
    /// engine
    ///     .materialization_bus_mut()
    ///     .register_channel(ch, ChannelPolicy::StrictSingle);
    ///
    /// let tx = engine.begin();
    /// let _ = engine.commit_with_receipt(tx)?;
    ///
    /// if engine.has_materialization_errors() {
    ///     // Handle or log errors - do NOT ignore!
    ///     for err in engine.last_materialization_errors() {
    ///         let _ = err;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// This is a convenience method equivalent to `!last_materialization_errors().is_empty()`.
    #[must_use]
    pub fn has_materialization_errors(&self) -> bool {
        !self.last_materialization_errors.is_empty()
    }

    /// Returns a shared view of a node when it exists.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownWarp`] if the root warp store is missing.
    pub fn node(&self, id: &NodeId) -> Result<Option<&NodeRecord>, EngineError> {
        let Some(store) = self.state.store(&self.current_root.warp_id) else {
            return Err(EngineError::UnknownWarp(self.current_root.warp_id));
        };
        Ok(store.node(id))
    }

    /// Returns the node's attachment value (if any) in the root instance.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownWarp`] if the root warp store is missing.
    pub fn node_attachment(&self, id: &NodeId) -> Result<Option<&AttachmentValue>, EngineError> {
        let Some(store) = self.state.store(&self.current_root.warp_id) else {
            return Err(EngineError::UnknownWarp(self.current_root.warp_id));
        };
        Ok(store.node_attachment(id))
    }

    /// Inserts or replaces a node directly inside the store.
    ///
    /// The spike uses this to create motion entities prior to executing rewrites.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownWarp`] if the root warp store is missing.
    /// This indicates internal state corruption: the root warp store is expected
    /// to exist after engine construction.
    pub fn insert_node(&mut self, id: NodeId, record: NodeRecord) -> Result<(), EngineError> {
        let Some(store) = self.state.store_mut(&self.current_root.warp_id) else {
            return Err(EngineError::UnknownWarp(self.current_root.warp_id));
        };
        store.insert_node(id, record);
        Ok(())
    }

    /// Sets the node's attachment value (if any) in the root instance.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownWarp`] if the root warp store is missing.
    /// This indicates internal state corruption: the root warp store is expected
    /// to exist after engine construction.
    pub fn set_node_attachment(
        &mut self,
        id: NodeId,
        value: Option<AttachmentValue>,
    ) -> Result<(), EngineError> {
        let Some(store) = self.state.store_mut(&self.current_root.warp_id) else {
            return Err(EngineError::UnknownWarp(self.current_root.warp_id));
        };
        store.set_node_attachment(id, value);
        Ok(())
    }

    /// Inserts or replaces a node and sets its attachment value (if any) in the root instance.
    ///
    /// This is a convenience for bootstrapping/demo callers that want to avoid
    /// partially-initialized nodes (node record inserted without its attachment)
    /// if an engine invariant is violated.
    ///
    /// # Errors
    /// Returns [`EngineError::UnknownWarp`] if the root warp store is missing.
    /// This indicates internal state corruption: the root warp store is expected
    /// to exist after engine construction.
    pub fn insert_node_with_attachment(
        &mut self,
        id: NodeId,
        record: NodeRecord,
        attachment: Option<AttachmentValue>,
    ) -> Result<(), EngineError> {
        let Some(store) = self.state.store_mut(&self.current_root.warp_id) else {
            return Err(EngineError::UnknownWarp(self.current_root.warp_id));
        };
        store.insert_node(id, record);
        store.set_node_attachment(id, attachment);
        Ok(())
    }
}

fn footprints_conflict(a: &crate::footprint::Footprint, b: &crate::footprint::Footprint) -> bool {
    // IMPORTANT: do not use `Footprint::independent` here yet.
    //
    // This logic MUST remain consistent with the scheduler’s footprint conflict
    // predicate (`RadixScheduler::has_conflict` in `scheduler.rs`). If one
    // changes, the other must change too, or receipts will attribute blockers
    // differently than the scheduler rejects candidates.
    //
    // `Footprint::independent` includes a `factor_mask` fast-path that assumes
    // masks are correctly populated as a conservative superset. Many current
    // footprints in the engine spike use `factor_mask = 0` as a placeholder,
    // which would incorrectly classify conflicting rewrites as independent.
    //
    // The scheduler’s conflict logic is defined by explicit overlap checks on
    // nodes/edges/ports; this mirrors that behavior exactly and stays correct
    // while factor masks are still being wired through.
    if a.b_in.intersects(&b.b_in)
        || a.b_in.intersects(&b.b_out)
        || a.b_out.intersects(&b.b_in)
        || a.b_out.intersects(&b.b_out)
    {
        return true;
    }
    if a.e_write.intersects(&b.e_write)
        || a.e_write.intersects(&b.e_read)
        || b.e_write.intersects(&a.e_read)
    {
        return true;
    }
    if a.a_write.intersects(&b.a_write)
        || a.a_write.intersects(&b.a_read)
        || b.a_write.intersects(&a.a_read)
    {
        return true;
    }
    a.n_write.intersects(&b.n_write)
        || a.n_write.intersects(&b.n_read)
        || b.n_write.intersects(&a.n_read)
}

fn compute_plan_digest(plan: &[PendingRewrite]) -> Hash {
    if plan.is_empty() {
        return crate::constants::digest_len0_u64();
    }
    let mut hasher = Hasher::new();
    hasher.update(&(plan.len() as u64).to_le_bytes());
    for pr in plan {
        hasher.update(&pr.scope_hash);
        hasher.update(&pr.rule_id);
    }
    hasher.finalize().into()
}

fn compute_rewrites_digest(rewrites: &[PendingRewrite]) -> Hash {
    if rewrites.is_empty() {
        return crate::constants::digest_len0_u64();
    }
    let mut hasher = Hasher::new();
    hasher.update(&(rewrites.len() as u64).to_le_bytes());
    for r in rewrites {
        hasher.update(&r.rule_id);
        hasher.update(&r.scope_hash);
        hasher.update(r.scope.warp_id.as_bytes());
        hasher.update(r.scope.local_id.as_bytes());
    }
    hasher.finalize().into()
}

impl Engine {
    fn rule_by_compact(&self, id: CompactRuleId) -> Option<&RewriteRule> {
        let name = self.rules_by_compact.get(&id)?;
        self.rules.get(name)
    }

    fn compute_rule_pack_id(&self) -> Hash {
        let mut ids: Vec<Hash> = self.rules.values().map(|r| r.id).collect();
        ids.sort_unstable();
        ids.dedup();

        let mut h = Hasher::new();
        // Version tag for future evolution.
        h.update(&1u16.to_le_bytes());
        h.update(&(ids.len() as u64).to_le_bytes());
        for id in ids {
            h.update(&id);
        }
        h.finalize().into()
    }
}

impl Engine {
    /// Returns a reference to the intent log.
    ///
    /// Each entry is a `(seq, crate::attachment::AtomPayload)` pair where:
    /// - `seq` is the ingest sequence number provided to [`Engine::ingest_inbox_event`].
    /// - `AtomPayload` is the stored payload associated with the ingested intent.
    ///
    /// This log is populated by [`Engine::ingest_inbox_event`] for debugging purposes;
    /// it does not affect graph identity or deterministic hashing.
    pub fn get_intent_log(&self) -> &[(u64, crate::attachment::AtomPayload)] {
        &self.intent_log
    }
}
/// Computes the canonical scope hash used for deterministic scheduler ordering.
///
/// This value is the first component of the scheduler’s canonical ordering key
/// (`scope_hash`, then `rule_id`, then nonce), and is used to domain-separate
/// candidates by both the producing rule and the scoped node.
///
/// Stable definition (v0):
/// - `scope_hash := blake3(rule_id || warp_id || scope_node_id)`
pub fn scope_hash(rule_id: &Hash, scope: &NodeKey) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(rule_id);
    hasher.update(scope.warp_id.as_bytes());
    hasher.update(scope.local_id.as_bytes());
    hasher.finalize().into()
}

/// Extends the slot sets with resources from a warp-scoped footprint.
///
/// Footprint sets are now warp-scoped: they contain full `NodeKey`, `EdgeKey`,
/// and `WarpScopedPortKey` values directly. The `_warp_id` parameter is kept
/// for call-site compatibility but is no longer used (the `warp_id` is embedded
/// in the footprint keys).
fn extend_slots_from_footprint(
    in_slots: &mut std::collections::BTreeSet<SlotId>,
    out_slots: &mut std::collections::BTreeSet<SlotId>,
    _warp_id: &WarpId, // Kept for API compat; warp_id is now in the footprint keys
    fp: &crate::footprint::Footprint,
) {
    // Nodes (warp-scoped NodeKey)
    for key in fp.n_read.iter() {
        in_slots.insert(SlotId::Node(*key));
    }
    for key in fp.n_write.iter() {
        in_slots.insert(SlotId::Node(*key));
        out_slots.insert(SlotId::Node(*key));
    }

    // Edges (warp-scoped EdgeKey)
    for key in fp.e_read.iter() {
        in_slots.insert(SlotId::Edge(*key));
    }
    for key in fp.e_write.iter() {
        in_slots.insert(SlotId::Edge(*key));
        out_slots.insert(SlotId::Edge(*key));
    }

    // Attachments (already warp-scoped via AttachmentKey)
    for key in fp.a_read.iter() {
        in_slots.insert(SlotId::Attachment(*key));
    }
    for key in fp.a_write.iter() {
        in_slots.insert(SlotId::Attachment(*key));
        out_slots.insert(SlotId::Attachment(*key));
    }

    // Ports (warp-scoped: (WarpId, PortKey))
    for port_key in fp.b_in.keys() {
        in_slots.insert(SlotId::Port(*port_key));
    }
    for port_key in fp.b_out.keys() {
        in_slots.insert(SlotId::Port(*port_key));
        out_slots.insert(SlotId::Port(*port_key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::{AtomPayload, AttachmentKey, AttachmentValue};
    use crate::ident::{make_node_id, make_type_id};
    use crate::payload::encode_motion_atom_payload;
    use crate::record::NodeRecord;
    use crate::tick_patch::WarpOp;

    const TEST_RULE_NAME: &str = "test/motion";

    fn test_motion_rule() -> RewriteRule {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"rule:");
        hasher.update(TEST_RULE_NAME.as_bytes());
        let id: Hash = hasher.finalize().into();
        RewriteRule {
            id,
            name: TEST_RULE_NAME,
            left: crate::rule::PatternGraph { nodes: vec![] },
            matcher: |view: GraphView<'_>, scope| {
                matches!(
                    view.node_attachment(scope),
                    Some(AttachmentValue::Atom(payload)) if crate::payload::decode_motion_atom_payload(payload).is_some()
                )
            },
            executor: |view: GraphView<'_>, scope, delta| {
                // Phase 5 BOAW: read from view, emit ops to delta (no direct mutation).
                let warp_id = view.warp_id();

                let Some(AttachmentValue::Atom(payload)) = view.node_attachment(scope) else {
                    return;
                };
                let Some((pos_raw, vel_raw)) =
                    crate::payload::decode_motion_atom_payload_q32_32(payload)
                else {
                    return;
                };

                // Compute the new position
                let new_pos_raw = [
                    pos_raw[0].saturating_add(vel_raw[0]),
                    pos_raw[1].saturating_add(vel_raw[1]),
                    pos_raw[2].saturating_add(vel_raw[2]),
                ];

                // Build new bytes
                let new_bytes = crate::payload::encode_motion_payload_q32_32(new_pos_raw, vel_raw);

                // Only emit if bytes actually changed
                if payload.bytes != new_bytes {
                    let key = AttachmentKey::node_alpha(NodeKey {
                        warp_id,
                        local_id: *scope,
                    });
                    delta.push(WarpOp::SetAttachment {
                        key,
                        value: Some(AttachmentValue::Atom(AtomPayload {
                            type_id: crate::payload::motion_payload_type_id(),
                            bytes: new_bytes,
                        })),
                    });
                }
            },
            compute_footprint: |view: GraphView<'_>, scope| {
                let mut a_write = crate::AttachmentSet::default();
                if view.node(scope).is_some() {
                    a_write.insert(AttachmentKey::node_alpha(NodeKey {
                        warp_id: view.warp_id(),
                        local_id: *scope,
                    }));
                }
                crate::Footprint {
                    n_read: crate::NodeSet::default(),
                    n_write: crate::NodeSet::default(),
                    e_read: crate::EdgeSet::default(),
                    e_write: crate::EdgeSet::default(),
                    a_read: crate::AttachmentSet::default(),
                    a_write,
                    b_in: crate::PortSet::default(),
                    b_out: crate::PortSet::default(),
                    factor_mask: 0,
                }
            },
            factor_mask: 0,
            conflict_policy: crate::rule::ConflictPolicy::Abort,
            join_fn: None,
        }
    }

    #[test]
    fn scope_hash_stable_for_rule_and_scope() {
        let rule = test_motion_rule();
        let warp_id = crate::ident::make_warp_id("scope-hash-test-warp");
        let scope_node = make_node_id("scope-hash-entity");
        let scope = NodeKey {
            warp_id,
            local_id: scope_node,
        };
        let h1 = super::scope_hash(&rule.id, &scope);
        // Recompute expected value manually using the same inputs.
        let mut hasher = blake3::Hasher::new();
        hasher.update(&rule.id);
        hasher.update(warp_id.as_bytes());
        hasher.update(scope_node.as_bytes());
        let expected: Hash = hasher.finalize().into();
        assert_eq!(h1, expected);
    }

    #[test]
    fn register_rule_join_requires_join_fn() {
        // Build a rule that declares Join but provides no join_fn.
        let bad = RewriteRule {
            id: [0u8; 32],
            name: "bad/join",
            left: crate::rule::PatternGraph { nodes: vec![] },
            matcher: |_s: GraphView<'_>, _n| true,
            executor: |_s: GraphView<'_>, _n, _delta| {},
            compute_footprint: |_s: GraphView<'_>, _n| crate::footprint::Footprint::default(),
            factor_mask: 0,
            conflict_policy: crate::rule::ConflictPolicy::Join,
            join_fn: None,
        };
        let mut engine = Engine::new(GraphStore::default(), make_node_id("r"));
        let res = engine.register_rule(bad);
        assert!(
            matches!(res, Err(EngineError::MissingJoinFn)),
            "expected MissingJoinFn, got {res:?}"
        );
    }

    #[test]
    fn tick_patch_replay_matches_post_state() {
        let entity = make_node_id("tick-patch-entity");
        let entity_type = make_type_id("entity");
        let payload = encode_motion_atom_payload([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);

        let mut store = GraphStore::default();
        store.insert_node(entity, NodeRecord { ty: entity_type });
        store.set_node_attachment(entity, Some(AttachmentValue::Atom(payload)));

        let mut engine = Engine::new(store, entity);
        let register = engine.register_rule(test_motion_rule());
        assert!(register.is_ok(), "rule registration failed: {register:?}");

        let tx = engine.begin();
        let applied = engine.apply(tx, TEST_RULE_NAME, &entity);
        assert!(
            matches!(applied, Ok(ApplyResult::Applied)),
            "expected ApplyResult::Applied, got {applied:?}"
        );

        let state_before = engine.state.clone();
        let committed = engine.commit_with_receipt(tx);
        assert!(
            committed.is_ok(),
            "commit_with_receipt failed: {committed:?}"
        );
        let Ok((snapshot, _receipt, patch)) = committed else {
            return;
        };
        let state_after = engine.state.clone();

        // Replay patch delta from the captured pre-state and compare the resulting state root.
        let mut state_replay = state_before;
        let replay = patch.apply_to_state(&mut state_replay);
        assert!(replay.is_ok(), "patch replay failed: {replay:?}");

        let root = engine.current_root;
        let state_after = compute_state_root(&state_after, &root);
        let state_replay = compute_state_root(&state_replay, &root);
        assert_eq!(
            state_after, state_replay,
            "patch replay must match post-state"
        );

        // Patch digest is the committed boundary artifact in commit hash v2.
        assert_eq!(snapshot.patch_digest, patch.digest());
        assert_eq!(
            snapshot.hash,
            compute_commit_hash_v2(
                &state_after,
                &snapshot.parents,
                &snapshot.patch_digest,
                snapshot.policy_id
            ),
            "commit hash v2 must commit to state_root + patch_digest (+ parents/policy)"
        );

        // Conservative slots from footprint: motion writes the scoped node attachment (α plane).
        let slot = SlotId::Attachment(AttachmentKey::node_alpha(NodeKey {
            warp_id: root.warp_id,
            local_id: entity,
        }));
        assert!(patch.in_slots().contains(&slot));
        assert!(patch.out_slots().contains(&slot));
    }
}
