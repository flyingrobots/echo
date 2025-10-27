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
    clippy::redundant_pub_crate
)]

pub mod math;

mod ident;
mod record;
mod graph;
mod rule;
mod tx;
mod scheduler;
mod snapshot;
mod payload;
mod engine_impl;
pub mod demo;

// Re-exports for stable public API
pub use ident::{make_node_id, make_type_id, EdgeId, Hash, NodeId, TypeId};
pub use record::{EdgeRecord, NodeRecord};
pub use graph::GraphStore;
pub use rule::{ExecuteFn, MatchFn, PatternGraph, RewriteRule};
pub use tx::TxId;
pub use scheduler::{DeterministicScheduler, PendingRewrite};
pub use snapshot::Snapshot;
pub use payload::{decode_motion_payload, encode_motion_payload};
pub use engine_impl::{ApplyResult, Engine, EngineError};
pub use demo::motion::{build_motion_demo_engine, motion_rule, MOTION_RULE_NAME};
