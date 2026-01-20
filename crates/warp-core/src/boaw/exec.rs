// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Parallel and serial execution for BOAW Phase 6.
//!
//! - **Phase 6A**: Stride partitioning (`execute_parallel_stride`)
//! - **Phase 6B**: Virtual shard partitioning (`execute_parallel_sharded`)
//!
//! Default is sharded (Phase 6B). Stride fallback requires feature flag.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::graph_view::GraphView;
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
/// Falls back to stride partitioning if `parallel-stride-fallback` feature
/// is enabled AND `ECHO_PARALLEL_STRIDE=1` environment variable is set.
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

    #[cfg(feature = "parallel-stride-fallback")]
    {
        if std::env::var("ECHO_PARALLEL_STRIDE")
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            // LOUD WARNING: This is a fallback mode for benchmarking only
            eprintln!(
                "\n\
                ╔══════════════════════════════════════════════════════════════╗\n\
                ║  WARNING: STRIDE FALLBACK ENABLED (ECHO_PARALLEL_STRIDE=1)   ║\n\
                ║  This is for A/B benchmarking only. Do NOT ship in prod.     ║\n\
                ╚══════════════════════════════════════════════════════════════╝\n"
            );
            return execute_parallel_stride(view, items, capped_workers);
        }
    }

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

/// Parallel execution with stride partitioning (Phase 6A legacy).
///
/// Each worker processes indices: `w, w + workers, w + 2*workers, ...`
/// This is the original Phase 6A implementation, kept for A/B benchmarking.
///
/// # Feature Gate
///
/// Only available when `parallel-stride-fallback` feature is enabled.
/// Activated at runtime by setting `ECHO_PARALLEL_STRIDE=1` env var.
///
/// # Panics
///
/// Panics if `workers` is 0.
#[cfg(any(test, feature = "parallel-stride-fallback"))]
pub fn execute_parallel_stride(
    view: GraphView<'_>,
    items: &[ExecItem],
    workers: usize,
) -> Vec<TickDelta> {
    assert!(workers > 0, "workers must be > 0");

    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(workers);
        for w in 0..workers {
            handles.push(s.spawn(move || {
                let mut delta = TickDelta::new();
                let mut i = w;
                while i < items.len() {
                    let item = &items[i];
                    let mut scoped = delta.scoped(item.origin);
                    (item.exec)(view, &item.scope, scoped.inner_mut());
                    i += workers;
                }
                delta
            }));
        }
        handles
            .into_iter()
            .map(|h| match h.join() {
                Ok(delta) => delta,
                Err(e) => std::panic::resume_unwind(e),
            })
            .collect()
    })
}
