// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Parallel and serial execution for BOAW Phase 6B.
//!
//! Uses virtual shard partitioning (`execute_parallel_sharded`) for cache locality.
//! Workers dynamically claim shards via atomic counter (work-stealing).

use std::any::Any;
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
    let capped_workers = workers.min(NUM_SHARDS);

    execute_parallel_sharded(view, items, capped_workers)
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
    assert!(workers > 0, "workers must be > 0");

    if items.is_empty() {
        // Can't use vec![TickDelta::new(); workers] because TickDelta doesn't impl Clone
        return (0..workers).map(|_| TickDelta::new()).collect();
    }

    // Partition into virtual shards by scope
    let shards = partition_into_shards(items);
    let next_shard = AtomicUsize::new(0);

    std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let view_copy = view;
                let shards = &shards;
                let next_shard = &next_shard;

                s.spawn(move || {
                    let mut delta = TickDelta::new();

                    // Work-stealing loop: claim shards until none remain
                    loop {
                        let shard_id = next_shard.fetch_add(1, Ordering::Relaxed);
                        if shard_id >= NUM_SHARDS {
                            break;
                        }

                        // Execute all items in this shard (cache locality)
                        for item in &shards[shard_id].items {
                            let mut scoped = delta.scoped(item.origin);
                            (item.exec)(view_copy, &item.scope, scoped.inner_mut());
                        }
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
