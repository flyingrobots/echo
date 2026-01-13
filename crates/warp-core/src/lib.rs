// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! warp-core: typed deterministic graph rewriting engine.
//!
//! The current implementation executes queued rewrites deterministically via the
//! motion-rule spike utilities. Broader storage and scheduling features will
//! continue to land over subsequent phases.
//!
//! # Protocol Determinism
//!
//! `warp-core` enforces strict determinism for all protocol artifacts (snapshots, patches, receipts).
//!
//! - **Wire Format:** Canonical CBOR via [`echo_wasm_abi`].
//!   - Maps must have sorted keys.
//!   - Floats are forbidden or strictly canonicalized (see `math` module).
//! - **JSON:** Forbidden for protocol/hashing. Allowed ONLY for debug/view layers (e.g. telemetry).
//! - **Float Math:** The default `F32Scalar` backend is optimistic (assumes IEEE 754).
//!   For strict cross-platform consensus, use the `det_fixed` feature.
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

mod attachment;
mod cmd;
mod constants;
/// Demo implementations showcasing engine capabilities (e.g., motion rule).
pub mod demo;
mod engine_impl;
mod footprint;
mod graph;
mod ident;
pub mod inbox;
mod payload;
mod receipt;
mod record;
mod rule;
mod sandbox;
mod scheduler;
#[cfg(feature = "serde")]
mod serializable;
mod snapshot;
mod tick_patch;
mod tx;
mod warp_state;

// Re-exports for stable public API
/// Attachment-plane atoms and codec boundaries.
pub use attachment::{
    AtomPayload, AttachmentKey, AttachmentOwner, AttachmentPlane, AttachmentValue, Codec,
    CodecRegistry, DecodeError, ErasedCodec, RegistryError,
};
/// Canonical digests (e.g., empty inputs, empty length-prefixed lists).
pub use constants::{BLAKE3_EMPTY, DIGEST_LEN0_U64, POLICY_ID_NO_POLICY_V0};
/// Demo helpers and constants for the motion rule.
pub use demo::motion::{build_motion_demo_engine, motion_rule, MOTION_RULE_NAME};
/// Rewrite engine and canonical hashing helpers.
pub use engine_impl::{scope_hash, ApplyResult, Engine, EngineError};
/// Footprint utilities for MWMR independence checks.
pub use footprint::{pack_port_key, AttachmentSet, Footprint, IdSet, PortKey, PortSet};
/// In-memory graph store used by the engine spike.
pub use graph::GraphStore;
/// Core identifier types and constructors for nodes, types, and edges.
pub use ident::{
    make_edge_id, make_node_id, make_type_id, make_warp_id, EdgeId, EdgeKey, Hash, NodeId, NodeKey,
    TypeId, WarpId,
};
/// Motion payload encoding/decoding helpers.
pub use payload::{
    decode_motion_atom_payload, decode_motion_payload, encode_motion_atom_payload,
    encode_motion_atom_payload_v0, encode_motion_payload, encode_motion_payload_q32_32,
    encode_motion_payload_v0, motion_payload_type_id, motion_payload_type_id_v0,
};
/// Tick receipts for deterministic commits (accepted vs rejected rewrites).
pub use receipt::{TickReceipt, TickReceiptDisposition, TickReceiptEntry, TickReceiptRejection};
/// Graph node and edge record types.
pub use record::{EdgeRecord, NodeRecord};
/// Rule primitives for pattern/match/execute.
pub use rule::{ConflictPolicy, ExecuteFn, MatchFn, PatternGraph, RewriteRule};
/// Sandbox helpers for constructing and comparing isolated Echo instances.
pub use sandbox::{build_engine, run_pair_determinism, DeterminismError, EchoConfig};
/// Scheduler selection (Radix vs Legacy) for sandbox/engine builders.
pub use scheduler::SchedulerKind;
/// UI-friendly serializable wrappers for ledger artifacts.
#[cfg(feature = "serde")]
pub use serializable::{
    SerializableReceipt, SerializableReceiptEntry, SerializableSnapshot, SerializableTick,
};
/// Immutable deterministic snapshot.
pub use snapshot::Snapshot;
/// Tick patch boundary artifacts (Paper III): replayable delta ops + slot sets.
pub use tick_patch::{
    slice_worldline_indices, PortalInit, SlotId, TickCommitStatus, TickPatchError, WarpOp,
    WarpTickPatchV1,
};
/// Transaction identifier type.
pub use tx::TxId;
/// Stage B1 multi-instance state types (`WarpInstances`).
pub use warp_state::{WarpInstance, WarpState};
