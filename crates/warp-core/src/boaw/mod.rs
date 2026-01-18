// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! BOAW Phase 6A: Parallel Execution with Canonical Merge.

mod exec;
mod merge;

pub use exec::{execute_parallel, execute_serial, ExecItem};
#[cfg(any(test, feature = "delta_validate"))]
pub use merge::merge_deltas;
pub use merge::MergeConflict;
