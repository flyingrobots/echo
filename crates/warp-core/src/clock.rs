// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Re-export of shared logical clock and generation identifiers for runtime metadata.
//!
//! Echo's internal clocks are logical monotone counters only. They carry no
//! wall-clock or elapsed-time semantics.

pub use echo_runtime_schema::{GlobalTick, RunId, WorldlineTick};
