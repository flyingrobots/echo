//! rmg-core: typed deterministic graph rewriting engine.
//!
//! The current implementation executes queued rewrites deterministically via the
//! motion-rule spike utilities. Broader storage and scheduling features will
//! continue to land over subsequent phases.
#![forbid(unsafe_code)]
#![deny(missing_docs, rust_2018_idioms, unused_must_use)]
#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro,
    clippy::print_stdout,
    clippy::print_stderr
)]
#![allow(
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::unreadable_literal,
    clippy::missing_const_for_fn,
    clippy::suboptimal_flops,
    clippy::redundant_pub_crate,
    clippy::many_single_char_names
)]

/// Deterministic math subsystem (Vec3, Mat4, Quat, PRNG).
pub mod math;

/// Demo implementations showcasing engine capabilities (e.g., motion rule).
pub mod demo;
mod engine_impl;
mod footprint;
mod graph;
mod ident;
mod payload;
mod record;
mod rule;
mod scheduler;
mod snapshot;
mod tx;

// Re-exports for stable public API
/// Demo helpers and constants for the motion rule.
pub use demo::motion::{build_motion_demo_engine, motion_rule, MOTION_RULE_NAME};
/// Rewrite engine and error types.
pub use engine_impl::{ApplyResult, Engine, EngineError};
/// Footprint utilities for MWMR independence checks.
pub use footprint::{pack_port_key, Footprint, PortKey};
/// In-memory graph store used by the engine spike.
pub use graph::GraphStore;
/// Core identifier types and constructors for nodes, types, and edges.
pub use ident::{make_edge_id, make_node_id, make_type_id, EdgeId, Hash, NodeId, TypeId};
/// Motion payload encoding/decoding helpers.
pub use payload::{decode_motion_payload, encode_motion_payload};
/// Graph node and edge record types.
pub use record::{EdgeRecord, NodeRecord};
/// Rule primitives for pattern/match/execute.
pub use rule::{ConflictPolicy, ExecuteFn, MatchFn, PatternGraph, RewriteRule};
/// Immutable deterministic snapshot.
pub use snapshot::Snapshot;
/// Transaction identifier type.
pub use tx::TxId;
