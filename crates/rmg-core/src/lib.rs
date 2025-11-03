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
    clippy::many_single_char_names,
    clippy::module_name_repetitions,
    clippy::use_self
)]
// Permit intentional name repetition for public API clarity (e.g., FooFoo types) and
// functions named after their module for discoverability (e.g., `motion_rule`).

/// Deterministic math subsystem (Vec3, Mat4, Quat, PRNG).
pub mod math;

mod constants;
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
pub mod math{scalar};

// Re-exports for stable public API
/// Canonical digests (e.g., empty inputs, empty length-prefixed lists).
pub use constants::{BLAKE3_EMPTY, DIGEST_LEN0_U64};
/// Demo helpers and constants for the motion rule.
pub use demo::motion::{build_motion_demo_engine, motion_rule, MOTION_RULE_NAME};
/// Rewrite engine and error types.
pub use engine_impl::{ApplyResult, Engine, EngineError};
/// Footprint utilities for MWMR independence checks.
/// `pack_port_key(node, port_id, dir_in)` packs a 64‑bit key as:
/// - upper 32 bits: low 32 bits of the `NodeId` (LE)
/// - bits 31..2: `port_id` (must be < 2^30)
/// - bit 1: reserved (0)
/// - bit 0: direction flag (`1` = input, `0` = output)
///
/// Collisions are possible across nodes that share the same low 32‑bit
/// fingerprint; choose ids/ports accordingly.
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

