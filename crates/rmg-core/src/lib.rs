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

pub mod math;

pub mod demo;
mod engine_impl;
mod graph;
mod ident;
mod payload;
mod record;
mod rule;
mod scheduler;
mod snapshot;
mod tx;

// Re-exports for stable public API
pub use demo::motion::{build_motion_demo_engine, motion_rule, MOTION_RULE_NAME};
pub use engine_impl::{ApplyResult, Engine, EngineError};
pub use graph::GraphStore;
pub use ident::{make_node_id, make_type_id, EdgeId, Hash, NodeId, TypeId};
pub use payload::{decode_motion_payload, encode_motion_payload};
pub use record::{EdgeRecord, NodeRecord};
pub use rule::{ExecuteFn, MatchFn, PatternGraph, RewriteRule};
pub use scheduler::{DeterministicScheduler, PendingRewrite};
pub use snapshot::Snapshot;
pub use tx::TxId;
