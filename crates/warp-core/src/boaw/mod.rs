// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! BOAW Phase 6: Parallel Execution with Canonical Merge.
//!
//! - **Phase 6A**: Stride partitioning + canonical merge (proven)
//! - **Phase 6B**: Virtual shard partitioning for cache locality

mod exec;
mod merge;
pub mod shard;

#[cfg(any(test, feature = "parallel-stride-fallback"))]
pub use exec::execute_parallel_stride;
pub use exec::{execute_parallel, execute_parallel_sharded, execute_serial, ExecItem};
#[cfg(any(test, feature = "delta_validate"))]
pub use merge::merge_deltas;
pub use merge::MergeConflict;
pub use shard::{shard_of, NUM_SHARDS};
