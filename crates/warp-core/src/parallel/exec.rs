// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Parallel and serial execution.
//!
//! Uses virtual shard partitioning (`execute_parallel_sharded`) for cache locality.
//! Workers dynamically claim shards via atomic counter (work-stealing).

use std::any::Any;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
#[cfg(not(feature = "unsafe_graph"))]
use crate::footprint_guard::FootprintGuard;
use crate::graph::GraphStore;
use crate::graph_view::GraphView;
use crate::ident::WarpId;
use crate::rule::ExecuteFn;
use crate::tick_delta::{OpOrigin, TickDelta};
use crate::NodeId;

use super::shard::{partition_into_shards, NUM_SHARDS};

/// How virtual shards are assigned to workers during parallel execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShardAssignmentPolicy {
    /// Workers claim shards dynamically via an atomic counter.
    DynamicSteal,
    /// Shards are assigned deterministically to workers by `shard_id % workers`.
    StaticRoundRobin,
    /// Each non-empty shard gets its own worker thread.
    ///
    /// This is primarily a benchmarking / comparison policy, not the default
    /// engine topology. It intentionally maximizes scheduling isolation at the
    /// cost of spawning up to one thread per non-empty shard.
    DedicatedPerShard,
}

/// How worker execution outputs are grouped into `TickDelta`s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeltaAccumulationPolicy {
    /// Each worker accumulates all claimed shards into one `TickDelta`.
    PerWorker,
    /// Each non-empty shard produces its own `TickDelta`.
    PerShard,
}

/// Execution policy for the shard-based parallel executor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParallelExecutionPolicy {
    /// How shards are assigned to workers.
    assignment: ShardAssignmentPolicy,
    /// How execution outputs are grouped into deltas.
    accumulation: DeltaAccumulationPolicy,
}

impl ParallelExecutionPolicy {
    /// Current default execution policy used by `execute_parallel()`.
    pub const DEFAULT: Self = Self {
        assignment: ShardAssignmentPolicy::DynamicSteal,
        accumulation: DeltaAccumulationPolicy::PerWorker,
    };

    /// Dynamic shard claiming with one output delta per worker.
    pub const DYNAMIC_PER_WORKER: Self = Self {
        assignment: ShardAssignmentPolicy::DynamicSteal,
        accumulation: DeltaAccumulationPolicy::PerWorker,
    };

    /// Dynamic shard claiming with one output delta per non-empty shard.
    pub const DYNAMIC_PER_SHARD: Self = Self {
        assignment: ShardAssignmentPolicy::DynamicSteal,
        accumulation: DeltaAccumulationPolicy::PerShard,
    };

    /// Deterministic round-robin shard assignment with one output delta per worker.
    pub const STATIC_PER_WORKER: Self = Self {
        assignment: ShardAssignmentPolicy::StaticRoundRobin,
        accumulation: DeltaAccumulationPolicy::PerWorker,
    };

    /// Deterministic round-robin shard assignment with one output delta per non-empty shard.
    pub const STATIC_PER_SHARD: Self = Self {
        assignment: ShardAssignmentPolicy::StaticRoundRobin,
        accumulation: DeltaAccumulationPolicy::PerShard,
    };

    /// One worker per non-empty shard with one output delta per shard.
    ///
    /// The `workers` argument is ignored for this policy. Empty input returns
    /// zero deltas because there are no non-empty shards to assign.
    pub const DEDICATED_PER_SHARD: Self = Self {
        assignment: ShardAssignmentPolicy::DedicatedPerShard,
        accumulation: DeltaAccumulationPolicy::PerShard,
    };

    /// Returns the shard-assignment mode for this execution policy.
    #[must_use]
    pub const fn assignment(self) -> ShardAssignmentPolicy {
        self.assignment
    }

    /// Returns the delta-grouping mode for this execution policy.
    #[must_use]
    pub const fn accumulation(self) -> DeltaAccumulationPolicy {
        self.accumulation
    }
}

/// Lightweight shape summary of a shard-partitioned parallel workload.
///
/// This profile is deterministic and cheap to derive once `partition_into_shards()`
/// has already grouped items by shard. Selectors can use it to choose a stable
/// execution plan without inspecting machine-local runtime state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParallelExecutionWorkloadProfile {
    total_items: usize,
    non_empty_shards: usize,
    max_shard_len: usize,
}

impl ParallelExecutionWorkloadProfile {
    /// Creates a workload profile from aggregate counts.
    #[must_use]
    pub const fn new(total_items: usize, non_empty_shards: usize, max_shard_len: usize) -> Self {
        Self {
            total_items,
            non_empty_shards,
            max_shard_len,
        }
    }

    /// Returns the total number of items in the workload.
    #[must_use]
    pub const fn total_items(self) -> usize {
        self.total_items
    }

    /// Returns the number of non-empty virtual shards in the workload.
    #[must_use]
    pub const fn non_empty_shards(self) -> usize {
        self.non_empty_shards
    }

    /// Returns the size of the largest non-empty virtual shard.
    #[must_use]
    pub const fn max_shard_len(self) -> usize {
        self.max_shard_len
    }

    fn from_shards(shards: &[super::shard::VirtualShard]) -> Self {
        let mut total_items = 0;
        let mut non_empty_shards = 0;
        let mut max_shard_len = 0;

        for shard in shards {
            let shard_len = shard.items.len();
            total_items += shard_len;
            if shard_len > 0 {
                non_empty_shards += 1;
                max_shard_len = max_shard_len.max(shard_len);
            }
        }

        Self::new(total_items, non_empty_shards, max_shard_len)
    }
}

/// Resolved plan for one parallel execution attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParallelExecutionPlan {
    workers: NonZeroUsize,
    policy: ParallelExecutionPolicy,
}

impl ParallelExecutionPlan {
    /// Creates a new plan from a worker count and fixed execution policy.
    #[must_use]
    pub const fn new(workers: NonZeroUsize, policy: ParallelExecutionPolicy) -> Self {
        Self { workers, policy }
    }

    /// Returns the worker count selected for this execution.
    #[must_use]
    pub const fn workers(self) -> NonZeroUsize {
        self.workers
    }

    /// Returns the fixed execution policy selected for this execution.
    #[must_use]
    pub const fn policy(self) -> ParallelExecutionPolicy {
        self.policy
    }
}

/// Selects a deterministic parallel execution plan from a worker hint and workload profile.
pub trait ParallelExecutionPlanSelector {
    /// Chooses the fixed execution plan to use for this workload.
    fn select_plan(
        &self,
        worker_hint: NonZeroUsize,
        workload: ParallelExecutionWorkloadProfile,
    ) -> ParallelExecutionPlan;
}

/// Workload-aware selector for shard-routing policy experiments.
///
/// The heuristic is intentionally conservative:
/// - very small or effectively serial workloads collapse to `STATIC_PER_WORKER` on `1w`
/// - medium workloads use `DYNAMIC_PER_WORKER` on `1w`
/// - large, well-distributed workloads switch to `DYNAMIC_PER_SHARD` with up to `4w`
///
/// This selector is meant for benchmarking and tuning. It does not observe
/// ambient machine state and remains a pure function of workload shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AdaptiveShardRoutingSelector;

const ADAPTIVE_SMALL_WORKLOAD_ITEMS: usize = 256;
const ADAPTIVE_LARGE_WORKLOAD_ITEMS: usize = 4_096;
const ADAPTIVE_MIN_PARALLEL_SHARDS: usize = 4;
const ADAPTIVE_DYNAMIC_PER_SHARD_WORKERS: usize = 4;

impl ParallelExecutionPlanSelector for AdaptiveShardRoutingSelector {
    fn select_plan(
        &self,
        worker_hint: NonZeroUsize,
        workload: ParallelExecutionWorkloadProfile,
    ) -> ParallelExecutionPlan {
        let effective_shard_parallelism = workload.non_empty_shards().min(worker_hint.get()).max(1);

        if workload.total_items() <= ADAPTIVE_SMALL_WORKLOAD_ITEMS
            || effective_shard_parallelism <= 1
        {
            return ParallelExecutionPlan::new(
                NonZeroUsize::MIN,
                ParallelExecutionPolicy::STATIC_PER_WORKER,
            );
        }

        if workload.total_items() >= ADAPTIVE_LARGE_WORKLOAD_ITEMS
            && effective_shard_parallelism >= ADAPTIVE_MIN_PARALLEL_SHARDS
            && workload.max_shard_len().saturating_mul(2) <= workload.total_items()
        {
            let workers = non_zero(
                worker_hint
                    .get()
                    .min(ADAPTIVE_DYNAMIC_PER_SHARD_WORKERS)
                    .min(workload.non_empty_shards())
                    .max(1),
            );
            return ParallelExecutionPlan::new(workers, ParallelExecutionPolicy::DYNAMIC_PER_SHARD);
        }

        ParallelExecutionPlan::new(
            NonZeroUsize::MIN,
            ParallelExecutionPolicy::DYNAMIC_PER_WORKER,
        )
    }
}

impl Default for ParallelExecutionPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Classification of an executor for footprint enforcement.
///
/// System items (engine-internal inbox rules) may emit instance-level ops
/// (`UpsertWarpInstance`, `DeleteWarpInstance`). User items cannot.
#[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
#[cfg(not(feature = "unsafe_graph"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExecItemKind {
    /// Normal user-registered rule — cannot emit instance ops.
    User,
    /// Engine-internal rule (inbox) — can emit instance-level ops.
    System,
}

/// A single rewrite ready for execution.
///
/// # Thread Safety
///
/// `ExecItem` is trivially `Sync + Send`:
/// - `exec`: plain function pointer (always Sync+Send)
/// - `scope`: `NodeId` is `[u8; 32]` newtype (Copy, no interior mutability)
/// - `origin`: `OpOrigin` contains only primitives (u64, u32, u32, u32)
#[derive(Clone, Copy, Debug)]
pub struct ExecItem {
    /// The execution function to run.
    pub exec: ExecuteFn,
    /// The scope node for this execution.
    pub scope: NodeId,
    /// Origin metadata for tracking.
    pub origin: OpOrigin,
    /// Classification for enforcement (user vs system).
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    pub(crate) kind: ExecItemKind,
}

impl ExecItem {
    /// Creates a new user-level `ExecItem`.
    ///
    /// This is the default constructor for all externally-registered rules.
    /// The cfg-gated `kind` field is set to `User` automatically.
    pub fn new(exec: ExecuteFn, scope: NodeId, origin: OpOrigin) -> Self {
        Self {
            exec,
            scope,
            origin,
            #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
            #[cfg(not(feature = "unsafe_graph"))]
            kind: ExecItemKind::User,
        }
    }

    /// Creates a new system-level `ExecItem`.
    ///
    /// System items are internal engine rules (e.g., inbox processing) that
    /// are allowed to emit instance-level ops under enforcement.
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    pub(crate) fn new_system(exec: ExecuteFn, scope: NodeId, origin: OpOrigin) -> Self {
        Self {
            exec,
            scope,
            origin,
            kind: ExecItemKind::System,
        }
    }
}

/// Marker type for deltas that must never be merged or committed.
///
/// Carries the delta for drop-only semantics and the panic payload that
/// triggered poisoning.
pub struct PoisonedDelta {
    _delta: TickDelta,
    panic: Box<dyn Any + Send + 'static>,
}

impl std::fmt::Debug for PoisonedDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PoisonedDelta")
            .field("panic", &"Box<dyn Any + Send>")
            .finish()
    }
}

impl PoisonedDelta {
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    pub(crate) fn new(delta: TickDelta, panic: Box<dyn Any + Send + 'static>) -> Self {
        Self {
            _delta: delta,
            panic,
        }
    }

    pub(crate) fn into_panic(self) -> Box<dyn Any + Send + 'static> {
        self.panic
    }
}

/// Result of a single worker's execution in `execute_work_queue`.
///
/// Flattens the nested `Result<Result<TickDelta, PoisonedDelta>, WarpId>` into
/// a single enum for clearer pattern matching.
pub enum WorkerResult {
    /// Worker completed successfully with a delta to merge.
    Success(TickDelta),
    /// Worker encountered a footprint violation or executor panic.
    Poisoned(PoisonedDelta),
    /// Worker failed to resolve a store for the given warp.
    MissingStore(WarpId),
}

impl std::fmt::Debug for WorkerResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success(_) => f.debug_tuple("Success").field(&"<TickDelta>").finish(),
            Self::Poisoned(p) => f.debug_tuple("Poisoned").field(p).finish(),
            Self::MissingStore(warp_id) => f.debug_tuple("MissingStore").field(warp_id).finish(),
        }
    }
}

/// Serial execution baseline.
pub fn execute_serial(view: GraphView<'_>, items: &[ExecItem]) -> TickDelta {
    let mut delta = TickDelta::new();
    for item in items {
        let mut scoped = delta.scoped(item.origin);
        (item.exec)(view, &item.scope, scoped.inner_mut());
    }
    delta
}

/// Parallel execution entry point.
///
/// Uses virtual shard partitioning by default (Phase 6B).
///
/// # Worker Count Cap
///
/// Workers are capped at `min(workers, NUM_SHARDS)` to avoid spawning
/// more threads than there are shards to process.
///
/// # Panics
///
/// Panics if `workers == 0` or if any worker thread panics during execution.
pub fn execute_parallel(view: GraphView<'_>, items: &[ExecItem], workers: usize) -> Vec<TickDelta> {
    assert!(workers >= 1, "need at least one worker");

    // Cap workers at NUM_SHARDS - no point spawning 512 threads for 256 shards
    let capped_workers = capped_workers(non_zero(workers));

    execute_parallel_sharded_with_policy(
        view,
        items,
        capped_workers,
        ParallelExecutionPolicy::DEFAULT,
    )
}

/// Parallel execution with virtual shard partitioning (Phase 6B).
///
/// Items are partitioned into 256 virtual shards by `shard_of(scope)`.
/// Workers dynamically claim shards via atomic counter (work-stealing).
/// Items in the same shard are processed together for cache locality.
///
/// # Determinism
///
/// Execution order is non-deterministic (workers race for shards).
/// Determinism is enforced by `merge_deltas()` which sorts canonically.
///
/// # Thread Safety
///
/// - `GraphView` is `Clone + Sync + Send` (read-only snapshot reference)
/// - `ExecItem` is `Sync + Send` (function pointer + primitives)
/// - Each worker gets its own `TickDelta` (no shared mutable state)
///
/// # Panics
///
/// Panics if `workers` is 0.
pub fn execute_parallel_sharded(
    view: GraphView<'_>,
    items: &[ExecItem],
    workers: usize,
) -> Vec<TickDelta> {
    assert!(workers >= 1, "need at least one worker");
    let workers = non_zero(workers);
    execute_parallel_sharded_with_policy(view, items, workers, ParallelExecutionPolicy::DEFAULT)
}

/// Parallel execution with an explicit shard assignment and delta accumulation policy.
///
/// This exposes the execution-policy matrix for benchmarking and experimentation
/// while preserving `execute_parallel()` as the stable default entrypoint.
///
/// `ParallelExecutionPolicy::DEDICATED_PER_SHARD` intentionally ignores the
/// `workers` argument and emits one delta per non-empty shard. All other
/// policies preserve the worker-shaped empty-result contract and return
/// `workers` empty deltas when `items` is empty.
///
pub fn execute_parallel_sharded_with_policy(
    view: GraphView<'_>,
    items: &[ExecItem],
    workers: NonZeroUsize,
    policy: ParallelExecutionPolicy,
) -> Vec<TickDelta> {
    let workers = capped_workers(workers);
    let shards = partition_into_shards(items);
    execute_partitioned_shards(view, &shards, ParallelExecutionPlan::new(workers, policy))
}

/// Parallel execution with a selector that can adapt the plan to workload shape.
pub fn execute_parallel_sharded_with_selector<S>(
    view: GraphView<'_>,
    items: &[ExecItem],
    workers: NonZeroUsize,
    selector: S,
) -> Vec<TickDelta>
where
    S: ParallelExecutionPlanSelector,
{
    let workers = capped_workers(workers);
    let shards = partition_into_shards(items);
    let workload = ParallelExecutionWorkloadProfile::from_shards(&shards);
    let plan = selector.select_plan(workers, workload);
    execute_partitioned_shards(view, &shards, plan)
}

fn execute_partitioned_shards(
    view: GraphView<'_>,
    shards: &[super::shard::VirtualShard],
    plan: ParallelExecutionPlan,
) -> Vec<TickDelta> {
    let workload = ParallelExecutionWorkloadProfile::from_shards(shards);
    if workload.total_items() == 0 {
        return if plan.policy().assignment == ShardAssignmentPolicy::DedicatedPerShard {
            Vec::new()
        } else {
            let workers = capped_workers(plan.workers()).get();
            (0..workers).map(|_| TickDelta::new()).collect()
        };
    }

    let workers = capped_workers(plan.workers()).get();
    match (plan.policy().assignment(), plan.policy().accumulation()) {
        (ShardAssignmentPolicy::DynamicSteal, DeltaAccumulationPolicy::PerWorker) => {
            execute_dynamic_per_worker(view, shards, workers)
        }
        (ShardAssignmentPolicy::DynamicSteal, DeltaAccumulationPolicy::PerShard) => {
            execute_dynamic_per_shard(view, shards, workers)
        }
        (ShardAssignmentPolicy::StaticRoundRobin, DeltaAccumulationPolicy::PerWorker) => {
            execute_static_per_worker(view, shards, workers)
        }
        (ShardAssignmentPolicy::StaticRoundRobin, DeltaAccumulationPolicy::PerShard) => {
            execute_static_per_shard(view, shards, workers)
        }
        (
            ShardAssignmentPolicy::DedicatedPerShard,
            DeltaAccumulationPolicy::PerWorker | DeltaAccumulationPolicy::PerShard,
        ) => {
            debug_assert_eq!(
                plan.policy().accumulation(),
                DeltaAccumulationPolicy::PerShard,
                "DedicatedPerShard is only exposed with PerShard accumulation"
            );
            execute_dedicated_per_shard(view, shards)
        }
    }
}

/// Parallel execution entry point with an explicit policy and worker cap.
///
/// This mirrors `execute_parallel()` but exposes the policy seam for benchmarks.
///
pub fn execute_parallel_with_policy(
    view: GraphView<'_>,
    items: &[ExecItem],
    workers: NonZeroUsize,
    policy: ParallelExecutionPolicy,
) -> Vec<TickDelta> {
    execute_parallel_sharded_with_policy(view, items, capped_workers(workers), policy)
}

/// Parallel execution entry point with a selector that may adapt policy and worker count.
pub fn execute_parallel_with_selector<S>(
    view: GraphView<'_>,
    items: &[ExecItem],
    workers: NonZeroUsize,
    selector: S,
) -> Vec<TickDelta>
where
    S: ParallelExecutionPlanSelector,
{
    execute_parallel_sharded_with_selector(view, items, capped_workers(workers), selector)
}

const fn non_zero(value: usize) -> NonZeroUsize {
    match NonZeroUsize::new(value) {
        Some(value) => value,
        None => NonZeroUsize::MIN,
    }
}

const fn capped_workers(workers: NonZeroUsize) -> NonZeroUsize {
    non_zero(if workers.get() < NUM_SHARDS {
        workers.get()
    } else {
        NUM_SHARDS
    })
}

fn execute_shard_into_delta(view: GraphView<'_>, items: &[ExecItem], delta: &mut TickDelta) {
    for item in items {
        let mut scoped = delta.scoped(item.origin);
        (item.exec)(view, &item.scope, scoped.inner_mut());
    }
}

fn execute_dynamic_per_worker(
    view: GraphView<'_>,
    shards: &[super::shard::VirtualShard],
    workers: usize,
) -> Vec<TickDelta> {
    let next_shard = AtomicUsize::new(0);

    std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let view_copy = view;
                let shards = &shards;
                let next_shard = &next_shard;

                s.spawn(move || {
                    let mut delta = TickDelta::new();
                    loop {
                        let shard_id = next_shard.fetch_add(1, Ordering::Relaxed);
                        if shard_id >= NUM_SHARDS {
                            break;
                        }
                        execute_shard_into_delta(view_copy, &shards[shard_id].items, &mut delta);
                    }
                    delta
                })
            })
            .collect();

        handles
            .into_iter()
            .map(|h| match h.join() {
                Ok(delta) => delta,
                Err(e) => std::panic::resume_unwind(e),
            })
            .collect()
    })
}

fn execute_dynamic_per_shard(
    view: GraphView<'_>,
    shards: &[super::shard::VirtualShard],
    workers: usize,
) -> Vec<TickDelta> {
    let next_shard = AtomicUsize::new(0);

    std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let view_copy = view;
                let shards = &shards;
                let next_shard = &next_shard;

                s.spawn(move || {
                    let mut deltas: Vec<(usize, TickDelta)> = Vec::new();
                    loop {
                        let shard_id = next_shard.fetch_add(1, Ordering::Relaxed);
                        if shard_id >= NUM_SHARDS {
                            break;
                        }
                        let items = &shards[shard_id].items;
                        if items.is_empty() {
                            continue;
                        }
                        let mut delta = TickDelta::new();
                        execute_shard_into_delta(view_copy, items, &mut delta);
                        deltas.push((shard_id, delta));
                    }
                    deltas
                })
            })
            .collect();

        let mut deltas: Vec<(usize, TickDelta)> = handles
            .into_iter()
            .flat_map(|h| match h.join() {
                Ok(worker_deltas) => worker_deltas,
                Err(e) => std::panic::resume_unwind(e),
            })
            .collect();
        deltas.sort_by_key(|(shard_id, _)| *shard_id);
        deltas.into_iter().map(|(_, delta)| delta).collect()
    })
}

fn execute_static_per_worker(
    view: GraphView<'_>,
    shards: &[super::shard::VirtualShard],
    workers: usize,
) -> Vec<TickDelta> {
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|worker_ix| {
                let view_copy = view;
                let shards = &shards;

                s.spawn(move || {
                    let mut delta = TickDelta::new();
                    for shard_id in (worker_ix..NUM_SHARDS).step_by(workers) {
                        execute_shard_into_delta(view_copy, &shards[shard_id].items, &mut delta);
                    }
                    delta
                })
            })
            .collect();

        handles
            .into_iter()
            .map(|h| match h.join() {
                Ok(delta) => delta,
                Err(e) => std::panic::resume_unwind(e),
            })
            .collect()
    })
}

fn execute_static_per_shard(
    view: GraphView<'_>,
    shards: &[super::shard::VirtualShard],
    workers: usize,
) -> Vec<TickDelta> {
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|worker_ix| {
                let view_copy = view;
                let shards = &shards;

                s.spawn(move || {
                    let mut deltas: Vec<(usize, TickDelta)> = Vec::new();
                    for shard_id in (worker_ix..NUM_SHARDS).step_by(workers) {
                        let items = &shards[shard_id].items;
                        if items.is_empty() {
                            continue;
                        }
                        let mut delta = TickDelta::new();
                        execute_shard_into_delta(view_copy, items, &mut delta);
                        deltas.push((shard_id, delta));
                    }
                    deltas
                })
            })
            .collect();

        let mut deltas: Vec<(usize, TickDelta)> = handles
            .into_iter()
            .flat_map(|h| match h.join() {
                Ok(worker_deltas) => worker_deltas,
                Err(e) => std::panic::resume_unwind(e),
            })
            .collect();
        deltas.sort_by_key(|(shard_id, _)| *shard_id);
        deltas.into_iter().map(|(_, delta)| delta).collect()
    })
}

fn execute_dedicated_per_shard(
    view: GraphView<'_>,
    shards: &[super::shard::VirtualShard],
) -> Vec<TickDelta> {
    std::thread::scope(|s| {
        let handles: Vec<_> = shards
            .iter()
            .enumerate()
            .filter(|(_, shard)| !shard.items.is_empty())
            .map(|(shard_id, shard)| {
                let view_copy = view;
                let items = &shard.items;
                s.spawn(move || {
                    let mut delta = TickDelta::new();
                    execute_shard_into_delta(view_copy, items, &mut delta);
                    (shard_id, delta)
                })
            })
            .collect();

        let mut deltas: Vec<(usize, TickDelta)> = handles
            .into_iter()
            .map(|h| match h.join() {
                Ok(delta) => delta,
                Err(e) => std::panic::resume_unwind(e),
            })
            .collect();
        deltas.sort_by_key(|(shard_id, _)| *shard_id);
        deltas.into_iter().map(|(_, delta)| delta).collect()
    })
}

// =============================================================================
// Cross-Warp Parallelism (Phase 6B+)
// =============================================================================

/// A unit of work: items from one shard within one warp.
///
/// The global work queue processes units in parallel across all warps,
/// eliminating the serial per-warp loop.
#[derive(Debug)]
pub struct WorkUnit {
    /// Which warp this unit belongs to.
    pub warp_id: WarpId,
    /// Items to execute (from one shard). Processed serially within the unit.
    pub items: Vec<ExecItem>,
    /// Precomputed footprint guards (1:1 with items).
    ///
    /// # Construction Contract
    ///
    /// This field is initialized empty by `build_work_units()`. The engine **MUST**
    /// call `attach_footprint_guards()` (or equivalent) to populate guards before
    /// any execution occurs. Runtime assertions in `execute_item_enforced()` verify
    /// this invariant—an empty `guards` vec when enforcement is active is a bug.
    ///
    /// # Invariants
    ///
    /// - `guards.len() == items.len()` before any item execution
    /// - Guards are indexed in parallel with items (guard[i] validates item[i])
    /// - Populated by engine after `build_work_units` when enforcement is active
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    pub(crate) guards: Vec<FootprintGuard>,
}

/// Builds work units from warp-partitioned items.
///
/// Creates `(warp, shard)` units by partitioning each warp's items into shards.
/// Only non-empty shards produce units. Units are ordered canonically:
/// `warp_id` (lexicographic via `BTreeMap`) then `shard_id` (ascending).
///
/// # Arguments
///
/// * `by_warp` - Any iterable of `(WarpId, Vec<ExecItem>)` pairs. Sorted by `WarpId` internally to guarantee deterministic output regardless of input order.
///
/// # Returns
///
/// Vector of work units in canonical order.
pub fn build_work_units(
    by_warp: impl IntoIterator<Item = (WarpId, Vec<ExecItem>)>,
) -> Vec<WorkUnit> {
    let mut sorted: Vec<_> = by_warp.into_iter().collect();
    sorted.sort_by_key(|(warp_id, _)| *warp_id);

    let mut units = Vec::new();

    for (warp_id, items) in sorted {
        let shards = partition_into_shards(&items);
        for shard in shards {
            if !shard.items.is_empty() {
                units.push(WorkUnit {
                    warp_id,
                    items: shard.items,
                    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
                    #[cfg(not(feature = "unsafe_graph"))]
                    guards: Vec::new(),
                });
            }
        }
    }

    units
}

/// Execute work queue with parallel workers (cross-warp parallelism).
///
/// This is the **only** spawn site for cross-warp execution. Workers claim
/// units atomically and execute items serially within each unit.
///
/// # Footprint Enforcement (cfg-gated)
///
/// When enforcement is active, the worker loop:
/// 1. Creates a guarded `GraphView` per item (read enforcement)
/// 2. Wraps execution in `catch_unwind` to ensure write validation runs
/// 3. Validates all emitted ops against the item's guard (write enforcement)
/// 4. Returns a poisoned delta carrying the panic payload
///
/// # Constraints (Non-Negotiable)
///
/// 1. **No nested threading**: Items within a unit are executed serially.
/// 2. **No long-lived borrows**: `GraphView` is resolved per-unit and dropped
///    before claiming the next unit.
/// 3. **`ExecItem` unchanged**: Work units carry items, items don't know their warp.
///
/// # Arguments
///
/// * `units` - Work units to execute (from `build_work_units`).
/// * `workers` - Number of parallel workers.
/// * `resolve_store` - Closure to resolve `&GraphStore` for a `WarpId`.
///
/// # Returns
///
/// A vector of [`WorkerResult`] entries, one per worker:
/// - [`WorkerResult::Success`]: delta ready to merge
/// - [`WorkerResult::Poisoned`]: executor or enforcement panic (must not be merged)
/// - [`WorkerResult::MissingStore`]: `resolve_store` returned `None` for a warp
///
/// # Panics
///
/// Panics if `workers == 0` or if any worker thread panics.
pub fn execute_work_queue<'state, F>(
    units: &[WorkUnit],
    workers: usize,
    resolve_store: F,
) -> Vec<WorkerResult>
where
    F: Fn(&WarpId) -> Option<&'state GraphStore> + Sync,
{
    assert!(workers > 0, "workers must be > 0");

    if units.is_empty() {
        return (0..workers)
            .map(|_| WorkerResult::Success(TickDelta::new()))
            .collect();
    }

    let next_unit = AtomicUsize::new(0);

    std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let units = &units;
                let next_unit = &next_unit;
                let resolve_store = &resolve_store;

                s.spawn(move || -> WorkerResult {
                    let mut delta = TickDelta::new();

                    // Work-stealing loop: claim units until none remain
                    loop {
                        let unit_idx = next_unit.fetch_add(1, Ordering::Relaxed);
                        if unit_idx >= units.len() {
                            break;
                        }

                        let unit = &units[unit_idx];

                        // Resolve view for this warp (per-unit, NOT cached across units)
                        let Some(store) = resolve_store(&unit.warp_id) else {
                            return WorkerResult::MissingStore(unit.warp_id);
                        };

                        // Execute items SERIALLY (no nested threading!)
                        for (idx, item) in unit.items.iter().enumerate() {
                            match execute_item_enforced(store, item, idx, unit, delta) {
                                Ok(next_delta) => {
                                    delta = next_delta;
                                }
                                Err(poisoned) => {
                                    return WorkerResult::Poisoned(poisoned);
                                }
                            }
                        }

                        // View dropped here - no long-lived borrows across warps
                    }

                    WorkerResult::Success(delta)
                })
            })
            .collect();

        handles
            .into_iter()
            .map(|h| match h.join() {
                Ok(result) => result,
                Err(e) => std::panic::resume_unwind(e),
            })
            .collect()
    })
}

/// Executes a single item with footprint enforcement (cfg-gated).
///
/// When enforcement is active:
/// 1. Creates a guarded `GraphView` (read enforcement via `new_guarded`)
/// 2. Wraps execution in `catch_unwind` to ensure write validation runs
/// 3. Validates all emitted ops via `check_op()` (write enforcement)
/// 4. Returns `Err(PoisonedDelta)` on executor panic or footprint violation
///
/// When enforcement is inactive (`unsafe_graph` feature or release without
/// `footprint_enforce_release`), executes directly without validation.
// Result is always Ok when enforcement is compiled out (unsafe_graph), but the
// signature must stay Result for the enforcement path.
#[allow(clippy::unnecessary_wraps)]
#[inline]
fn execute_item_enforced(
    store: &GraphStore,
    item: &ExecItem,
    idx: usize,
    unit: &WorkUnit,
    mut delta: TickDelta,
) -> Result<TickDelta, PoisonedDelta> {
    // Enforcement path: guarded view + catch_unwind + post-hoc write validation
    #[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
    #[cfg(not(feature = "unsafe_graph"))]
    {
        use std::panic::{catch_unwind, AssertUnwindSafe};

        // Hard invariant: guards must be populated and aligned with items.
        // This check runs in all builds (debug and release) when enforcement is active.
        // If guards are misaligned, it's a bug in the engine's guard construction.
        assert_eq!(
            unit.guards.len(),
            unit.items.len(),
            "guards must align with items before enforcement"
        );
        assert!(
            !unit.guards.is_empty(),
            "guards must be populated when enforcement is active"
        );

        let guard = &unit.guards[idx];
        let view = GraphView::new_guarded(store, guard);

        // Track delta growth for write validation
        let ops_before = delta.len();

        // Execute under catch_unwind to enforce writes even on panic
        let exec_result = catch_unwind(AssertUnwindSafe(|| {
            let mut scoped = delta.scoped(item.origin);
            (item.exec)(view, &item.scope, scoped.inner_mut());
        }));

        let exec_panic = exec_result.err();

        // Post-hoc write enforcement (runs whether exec succeeded or panicked)
        let check_result = catch_unwind(AssertUnwindSafe(|| {
            for op in &delta.ops_ref()[ops_before..] {
                guard.check_op(op);
            }
        }));

        match (exec_panic, check_result) {
            (None, Ok(())) => {
                return Ok(delta);
            }
            (Some(panic), Ok(())) | (None, Err(panic)) => {
                return Err(PoisonedDelta::new(delta, panic));
            }
            (Some(exec_panic), Err(guard_panic)) => {
                let payload = match guard_panic
                    .downcast::<crate::footprint_guard::FootprintViolation>()
                {
                    Ok(violation) => Box::new(crate::footprint_guard::FootprintViolationWithPanic {
                        violation: *violation,
                        exec_panic,
                    }) as Box<dyn Any + Send + 'static>,
                    Err(guard_panic) => {
                        Box::new((exec_panic, guard_panic)) as Box<dyn Any + Send + 'static>
                    }
                };
                return Err(PoisonedDelta::new(delta, payload));
            }
        }
    }

    // Non-enforced path: direct execution (unreachable when enforcement is active,
    // since all match arms in the cfg block above return).
    #[allow(unreachable_code)]
    {
        // Suppress unused variable warnings in non-enforced builds
        let _ = idx;
        let _ = unit;

        let view = GraphView::new(store);
        let mut scoped = delta.scoped(item.origin);
        (item.exec)(view, &item.scope, scoped.inner_mut());

        Ok(delta)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        execute_parallel_with_policy, execute_parallel_with_selector, AdaptiveShardRoutingSelector,
        ExecItem, ParallelExecutionPlan, ParallelExecutionPlanSelector, ParallelExecutionPolicy,
        ParallelExecutionWorkloadProfile,
    };
    use crate::{
        execute_serial, make_type_id, merge_deltas_ok, AtomPayload, AttachmentKey, AttachmentValue,
        GraphStore, GraphView, NodeId, NodeKey, NodeRecord, OpOrigin, TickDelta, WarpOp,
    };
    use std::num::NonZeroUsize;

    fn test_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
        let payload = AtomPayload::new(
            make_type_id("parallel/policy-test"),
            bytes::Bytes::from_static(b"ok"),
        );
        let key = AttachmentKey::node_alpha(NodeKey {
            warp_id: view.warp_id(),
            local_id: *scope,
        });
        delta.push(WarpOp::SetAttachment {
            key,
            value: Some(AttachmentValue::Atom(payload)),
        });
    }

    fn make_store_and_items(count: usize) -> (GraphStore, Vec<ExecItem>) {
        let mut store = GraphStore::default();
        let node_ty = make_type_id("parallel/policy-node");
        let mut items = Vec::with_capacity(count);
        for i in 0..count {
            let mut bytes = [0u8; 32];
            assert!(
                u8::try_from(i).is_ok(),
                "test fixture only supports up to 256 scopes"
            );
            bytes[0] = u8::try_from(i).unwrap_or(0);
            let scope = NodeId(bytes);
            store.insert_node(scope, NodeRecord { ty: node_ty });
            items.push(ExecItem::new(
                test_executor,
                scope,
                OpOrigin {
                    intent_id: i as u64,
                    rule_id: 1,
                    match_ix: 0,
                    op_ix: 0,
                },
            ));
        }
        (store, items)
    }

    fn worker_hint(workers: usize) -> NonZeroUsize {
        NonZeroUsize::new(workers.max(1)).map_or(NonZeroUsize::MIN, |w| w)
    }

    #[test]
    fn adaptive_selector_prefers_static_per_worker_for_small_workloads() {
        let selector = AdaptiveShardRoutingSelector;
        let plan = selector.select_plan(
            worker_hint(8),
            ParallelExecutionWorkloadProfile::new(100, 100, 1),
        );

        assert_eq!(
            plan,
            ParallelExecutionPlan::new(worker_hint(1), ParallelExecutionPolicy::STATIC_PER_WORKER)
        );
    }

    #[test]
    fn adaptive_selector_prefers_dynamic_per_worker_for_medium_workloads() {
        let selector = AdaptiveShardRoutingSelector;
        let plan = selector.select_plan(
            worker_hint(8),
            ParallelExecutionWorkloadProfile::new(1_000, 32, 48),
        );

        assert_eq!(
            plan,
            ParallelExecutionPlan::new(worker_hint(1), ParallelExecutionPolicy::DYNAMIC_PER_WORKER)
        );
    }

    #[test]
    fn adaptive_selector_prefers_dynamic_per_shard_for_large_wide_workloads() {
        let selector = AdaptiveShardRoutingSelector;
        let plan = selector.select_plan(
            worker_hint(8),
            ParallelExecutionWorkloadProfile::new(10_000, 64, 120),
        );

        assert_eq!(
            plan,
            ParallelExecutionPlan::new(worker_hint(4), ParallelExecutionPolicy::DYNAMIC_PER_SHARD)
        );
    }

    #[test]
    fn all_parallel_policies_match_serial_oracle() {
        let policies = [
            ParallelExecutionPolicy::DYNAMIC_PER_WORKER,
            ParallelExecutionPolicy::DYNAMIC_PER_SHARD,
            ParallelExecutionPolicy::STATIC_PER_WORKER,
            ParallelExecutionPolicy::STATIC_PER_SHARD,
            ParallelExecutionPolicy::DEDICATED_PER_SHARD,
        ];
        let (store, items) = make_store_and_items(32);
        let view = GraphView::new(&store);
        let serial_oracle_result = merge_deltas_ok(vec![execute_serial(view, &items)]);
        assert!(
            serial_oracle_result.is_ok(),
            "serial oracle merge failed: {serial_oracle_result:?}"
        );
        let Ok(serial_oracle) = serial_oracle_result else {
            unreachable!("assert above guarantees a valid serial oracle");
        };

        for policy in policies {
            for workers in [1_usize, 4, 8] {
                let deltas =
                    execute_parallel_with_policy(view, &items, worker_hint(workers), policy);
                let merged_result = merge_deltas_ok(deltas);
                assert!(
                    merged_result.is_ok(),
                    "policy merge failed for {policy:?} @ {workers}w: {merged_result:?}"
                );
                let Ok(merged) = merged_result else {
                    unreachable!("assert above guarantees a successful policy merge");
                };
                assert_eq!(
                    merged, serial_oracle,
                    "policy {policy:?} changed merged ops at {workers}w"
                );
            }
        }
    }

    #[test]
    fn per_shard_policy_emits_more_than_one_delta_when_one_worker_sees_many_shards() {
        let (store, items) = make_store_and_items(8);
        let view = GraphView::new(&store);

        let per_worker = execute_parallel_with_policy(
            view,
            &items,
            worker_hint(1),
            ParallelExecutionPolicy::DYNAMIC_PER_WORKER,
        );
        let per_shard = execute_parallel_with_policy(
            view,
            &items,
            worker_hint(1),
            ParallelExecutionPolicy::DYNAMIC_PER_SHARD,
        );
        let serial_oracle = merge_deltas_ok(vec![execute_serial(view, &items)]);
        assert!(
            serial_oracle.is_ok(),
            "serial oracle merge failed for per-shard count test: {serial_oracle:?}"
        );
        let Ok(serial_oracle) = serial_oracle else {
            unreachable!("assert above guarantees a valid serial oracle");
        };
        let per_worker_len = per_worker.len();
        let per_shard_len = per_shard.len();
        let per_worker_merged = merge_deltas_ok(per_worker);
        let per_shard_merged = merge_deltas_ok(per_shard);
        assert!(
            per_worker_merged.is_ok(),
            "per-worker merge failed: {per_worker_merged:?}"
        );
        assert!(
            per_shard_merged.is_ok(),
            "per-shard merge failed: {per_shard_merged:?}"
        );
        let Ok(per_worker_merged) = per_worker_merged else {
            unreachable!("assert above guarantees a merged per-worker result");
        };
        let Ok(per_shard_merged) = per_shard_merged else {
            unreachable!("assert above guarantees a merged per-shard result");
        };

        assert_eq!(per_worker_len, 1, "per-worker policy should emit one delta");
        assert!(
            per_shard_len > 1,
            "per-shard policy should emit multiple deltas when one worker processes multiple shards"
        );
        assert_eq!(
            per_worker_merged, serial_oracle,
            "per-worker 1w path changed merged ops"
        );
        assert_eq!(
            per_shard_merged, serial_oracle,
            "per-shard 1w path changed merged ops"
        );
    }

    #[test]
    fn dedicated_per_shard_ignores_worker_count() {
        let (store, items) = make_store_and_items(8);
        let view = GraphView::new(&store);

        let one_worker = execute_parallel_with_policy(
            view,
            &items,
            worker_hint(1),
            ParallelExecutionPolicy::DEDICATED_PER_SHARD,
        );
        let many_workers = execute_parallel_with_policy(
            view,
            &items,
            worker_hint(8),
            ParallelExecutionPolicy::DEDICATED_PER_SHARD,
        );
        let serial_oracle = merge_deltas_ok(vec![execute_serial(view, &items)]);
        assert!(
            serial_oracle.is_ok(),
            "serial oracle merge failed for dedicated-per-shard test: {serial_oracle:?}"
        );
        let Ok(serial_oracle) = serial_oracle else {
            unreachable!("assert above guarantees a valid serial oracle");
        };
        let one_worker_len = one_worker.len();
        let many_workers_len = many_workers.len();
        let one_worker_merged = merge_deltas_ok(one_worker);
        let many_workers_merged = merge_deltas_ok(many_workers);
        assert!(
            one_worker_merged.is_ok(),
            "dedicated-per-shard 1w merge failed: {one_worker_merged:?}"
        );
        assert!(
            many_workers_merged.is_ok(),
            "dedicated-per-shard 8w merge failed: {many_workers_merged:?}"
        );
        let Ok(one_worker_merged) = one_worker_merged else {
            unreachable!("assert above guarantees a merged dedicated-per-shard 1w result");
        };
        let Ok(many_workers_merged) = many_workers_merged else {
            unreachable!("assert above guarantees a merged dedicated-per-shard 8w result");
        };

        assert_eq!(
            one_worker_len, many_workers_len,
            "dedicated-per-shard ignores the worker-count hint"
        );
        assert_eq!(
            one_worker_merged, serial_oracle,
            "dedicated-per-shard 1w changed merged ops"
        );
        assert_eq!(
            many_workers_merged, serial_oracle,
            "dedicated-per-shard 8w changed merged ops"
        );
    }

    #[test]
    fn dedicated_per_shard_empty_workload_emits_no_deltas() {
        let store = GraphStore::default();
        let items: Vec<ExecItem> = Vec::new();
        let view = GraphView::new(&store);

        let deltas = execute_parallel_with_policy(
            view,
            &items,
            worker_hint(4),
            ParallelExecutionPolicy::DEDICATED_PER_SHARD,
        );

        assert!(
            deltas.is_empty(),
            "empty workload should produce no deltas for dedicated-per-shard"
        );
    }

    #[test]
    fn adaptive_selector_matches_serial_oracle() {
        let selector = AdaptiveShardRoutingSelector;
        let (store, items) = make_store_and_items(64);
        let view = GraphView::new(&store);
        let serial_oracle_result = merge_deltas_ok(vec![execute_serial(view, &items)]);
        assert!(
            serial_oracle_result.is_ok(),
            "serial oracle merge failed for adaptive selector: {serial_oracle_result:?}"
        );
        let Ok(serial_oracle) = serial_oracle_result else {
            unreachable!("assert above guarantees a valid serial oracle");
        };

        for workers in [1_usize, 4, 8] {
            let deltas =
                execute_parallel_with_selector(view, &items, worker_hint(workers), selector);
            let merged_result = merge_deltas_ok(deltas);
            assert!(
                merged_result.is_ok(),
                "adaptive selector merge failed at {workers}w: {merged_result:?}"
            );
            let Ok(merged) = merged_result else {
                unreachable!("assert above guarantees a valid adaptive merge");
            };
            assert_eq!(
                merged, serial_oracle,
                "adaptive selector changed merged ops at {workers}w"
            );
        }
    }
}
