// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Parallel and serial execution for BOAW Phase 6B.
//!
//! Uses virtual shard partitioning (`execute_parallel_sharded`) for cache locality.
//! Workers dynamically claim shards via atomic counter (work-stealing).

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::graph::GraphStore;
use crate::graph_view::GraphView;
use crate::ident::WarpId;
use crate::rule::ExecuteFn;
use crate::tick_delta::{OpOrigin, TickDelta};
use crate::NodeId;

use super::shard::{partition_into_shards, NUM_SHARDS};

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
/// `Ok(deltas)` with one `TickDelta` per worker, to be merged by caller.
///
/// # Errors
///
/// Returns `Err(warp_id)` if `resolve_store` returned `None` for a unit's
/// warp, indicating the caller failed to validate store availability.
///
/// # Panics
///
/// Panics if `workers == 0` or if any worker thread panics.
pub fn execute_work_queue<'state, F>(
    units: &[WorkUnit],
    workers: usize,
    resolve_store: F,
) -> Result<Vec<TickDelta>, WarpId>
where
    F: Fn(&WarpId) -> Option<&'state GraphStore> + Sync,
{
    assert!(workers > 0, "workers must be > 0");

    if units.is_empty() {
        return Ok((0..workers).map(|_| TickDelta::new()).collect());
    }

    let next_unit = AtomicUsize::new(0);

    std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let units = &units;
                let next_unit = &next_unit;
                let resolve_store = &resolve_store;

                s.spawn(move || -> Result<TickDelta, WarpId> {
                    let mut delta = TickDelta::new();

                    // Work-stealing loop: claim units until none remain
                    loop {
                        let unit_idx = next_unit.fetch_add(1, Ordering::Relaxed);
                        if unit_idx >= units.len() {
                            break;
                        }

                        let unit = &units[unit_idx];

                        // Resolve view for this warp (per-unit, NOT cached across units)
                        let store = resolve_store(&unit.warp_id).ok_or(unit.warp_id)?;
                        let view = GraphView::new(store);

                        // Execute items SERIALLY (no nested threading!)
                        for item in &unit.items {
                            let mut scoped = delta.scoped(item.origin);
                            (item.exec)(view, &item.scope, scoped.inner_mut());
                        }

                        // View dropped here - no long-lived borrows across warps
                    }

                    Ok(delta)
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
