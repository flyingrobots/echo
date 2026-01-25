// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! BOAW Phase 6B: Parallel Execution with Canonical Merge.
//!
//! Virtual shard partitioning for cache locality + canonical merge ordering.

mod exec;
mod merge;
pub mod shard;

#[cfg(any(debug_assertions, feature = "footprint_enforce_release"))]
#[cfg(not(feature = "unsafe_graph"))]
pub(crate) use exec::ExecItemKind;
pub use exec::{
    build_work_units, execute_parallel, execute_parallel_sharded, execute_serial,
    execute_work_queue, ExecItem, PoisonedDelta, WorkUnit, WorkerResult,
};
pub use merge::MergeConflict;
#[cfg(any(test, feature = "delta_validate"))]
pub use merge::{merge_deltas, merge_deltas_ok, MergeError};
pub use shard::{shard_of, NUM_SHARDS};
