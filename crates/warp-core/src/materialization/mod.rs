// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Materialization subsystem for deterministic channel-based output.
//!
//! The materialization bus enables rewrite rules to emit derived updates during ticks.
//! These emissions are collected in an **order-independent** manner and finalized
//! post-commit according to each channel's policy.
//!
//! # Order Independence (Confluence Safety)
//!
//! Emissions are keyed by [`EmitKey`], which is derived from the scheduler's canonical
//! ordering (scope_hash, rule_id, nonce). This ensures that the finalized output is
//! deterministic regardless of the order in which rules execute — a critical property
//! for confluence-safe parallel rewriting.
//!
//! # Channel Policies
//!
//! Each channel can be configured with a [`ChannelPolicy`]:
//!
//! - [`Snapshot`](ChannelPolicy::Snapshot): Single value per tick (max EmitKey wins)
//! - [`SnapshotStrict`](ChannelPolicy::SnapshotStrict): Same, but error on conflict
//! - [`Log`](ChannelPolicy::Log): All emissions in EmitKey order
//! - [`Reduce`](ChannelPolicy::Reduce): Fold via join function
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │              External World              │
//! └─────────────────────────────────────────┘
//!                       ▲
//!                       │ ViewOps frames
//!              ┌────────┴────────┐
//!              │ MaterializationPort │  ← Boundary API
//!              └────────┬────────┘
//!                       │
//!              ┌────────┴────────┐
//!              │ MaterializationBus │   ← Internal Runtime
//!              └────────┬────────┘
//!                       │
//!              ┌────────┴────────┐
//!              │      Engine      │
//!              │  commit → finalize │
//!              └─────────────────┘
//! ```

mod bus;
mod channel;
mod emission_port;
mod emit_key;
mod frame;
mod port;
mod reduce_op;
mod scoped_emitter;

pub use bus::{DuplicateEmission, FinalizeReport, FinalizedChannel, MaterializationBus};
pub use channel::{
    make_channel_id, ChannelConflict, ChannelId, ChannelPolicy, MaterializationErrorKind,
};
pub use emission_port::EmissionPort;
pub use emit_key::EmitKey;
pub use frame::{decode_frames, encode_frames, MaterializationFrame, FRAME_MAGIC, FRAME_VERSION};
pub use port::MaterializationPort;
pub use reduce_op::ReduceOp;
pub use scoped_emitter::ScopedEmitter;
