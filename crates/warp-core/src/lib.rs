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
//! - **Wire Format:** Canonical CBOR via `echo_wasm_abi`.
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

/// Deterministic fixed-point helpers (Q32.32).
pub mod fixed;
/// Deterministic math subsystem (Vec3, Mat4, Quat, PRNG).
pub mod math;
/// WSC (Write-Streaming Columnar) snapshot format for deterministic serialization.
pub mod wsc;

mod attachment;
/// BOAW (Best Of All Worlds) parallel execution module.
///
/// Provides both serial and parallel execution strategies for rewrite rules,
/// with deterministic results guaranteed through canonical merge sorting.
///
/// # Key Types
///
/// - [`ExecItem`]: Encapsulates a single rewrite ready for execution
/// - [`MergeConflict`]: Error type for footprint model violations
///
/// # Key Functions
///
/// - [`execute_serial`]: Baseline serial execution
/// - [`execute_parallel`]: Parallel execution with shard partitioning
/// - [`shard_of`]: Compute shard ID from a scope `NodeId`
///
/// # Determinism Guarantee
///
/// Execution order across workers is non-deterministic, but the final merged
/// output is always canonical regardless of worker count or thread scheduling.
pub mod boaw;
mod cmd;
mod constants;
mod engine_impl;
mod footprint;
/// Footprint enforcement guard for BOAW Phase 6B.
///
/// Validates that execute functions stay within their declared footprints.
/// Active in debug builds; opt-in for release via `footprint_enforce_release` feature.
pub mod footprint_guard;
mod graph;
mod graph_view;
mod ident;
/// Canonical inbox management for deterministic intent sequencing.
pub mod inbox;
/// Materialization subsystem for deterministic channel-based output.
pub mod materialization;
mod payload;
mod playback;
mod provenance_store;
mod receipt;
mod record;
mod retention;
mod rule;
mod sandbox;
mod scheduler;
#[cfg(feature = "serde")]
mod serializable;
mod snapshot;
mod snapshot_accum;
mod telemetry;
mod tick_delta;
mod tick_patch;
mod tx;
mod warp_state;
mod worldline;

// Re-exports for stable public API
pub use attachment::{
    AtomPayload, AttachmentKey, AttachmentOwner, AttachmentPlane, AttachmentValue, Codec,
    CodecRegistry, DecodeError, ErasedCodec, RegistryError,
};
pub use boaw::{
    execute_parallel, execute_parallel_sharded, execute_serial, shard_of, ExecItem, MergeConflict,
    PoisonedDelta, NUM_SHARDS,
};
#[cfg(any(test, feature = "delta_validate"))]
pub use boaw::{merge_deltas, merge_deltas_ok, MergeError};
pub use constants::{blake3_empty, digest_len0_u64, POLICY_ID_NO_POLICY_V0};
pub use engine_impl::{
    scope_hash, ApplyResult, DispatchDisposition, Engine, EngineBuilder, EngineError,
    ExistingState, FreshStore, IngestDisposition,
};
pub use footprint::{
    pack_port_key, AttachmentSet, EdgeSet, Footprint, NodeSet, PortKey, PortSet, WarpScopedPortKey,
};
pub use footprint_guard::{FootprintViolation, FootprintViolationWithPanic, ViolationKind};
pub use graph::GraphStore;
pub use graph_view::GraphView;
pub use ident::{
    make_edge_id, make_node_id, make_type_id, make_warp_id, EdgeId, EdgeKey, Hash, NodeId, NodeKey,
    TypeId, WarpId,
};
pub use payload::{
    decode_motion_atom_payload, decode_motion_atom_payload_q32_32, decode_motion_payload,
    encode_motion_atom_payload, encode_motion_atom_payload_v0, encode_motion_payload,
    encode_motion_payload_q32_32, encode_motion_payload_v0, motion_payload_type_id,
    motion_payload_type_id_v0,
};
// --- Cursor types ---
pub use playback::{
    CursorId, CursorRole, PlaybackCursor, PlaybackMode, SeekError, SeekThen, StepResult,
};
// --- Session types ---
pub use playback::{SessionId, ViewSession};
// --- Truth delivery ---
pub use playback::{CursorReceipt, TruthFrame, TruthSink};
pub use provenance_store::{CheckpointRef, HistoryError, LocalProvenanceStore, ProvenanceStore};
pub use receipt::{TickReceipt, TickReceiptDisposition, TickReceiptEntry, TickReceiptRejection};
pub use record::{EdgeRecord, NodeRecord};
pub use rule::{ConflictPolicy, ExecuteFn, MatchFn, PatternGraph, RewriteRule};
pub use sandbox::{build_engine, run_pair_determinism, DeterminismError, EchoConfig};
pub use scheduler::SchedulerKind;
#[cfg(feature = "serde")]
pub use serializable::{
    SerializableReceipt, SerializableReceiptEntry, SerializableSnapshot, SerializableTick,
};
pub use snapshot::{compute_commit_hash_v2, compute_state_root_for_warp_store, Snapshot};
pub use telemetry::{NullTelemetrySink, TelemetrySink};
pub use tick_delta::{DeltaStats, OpOrigin, ScopedDelta, TickDelta};
pub use tick_patch::{
    slice_worldline_indices, PortalInit, SlotId, TickCommitStatus, TickPatchError, WarpOp,
    WarpOpKey, WarpTickPatchV1,
};
pub use tx::TxId;
pub use warp_state::{WarpInstance, WarpState};
pub use worldline::{
    ApplyError, HashTriplet, OutputFrameSet, WorldlineId, WorldlineTickHeaderV1,
    WorldlineTickPatchV1,
};

/// Zero-copy typed view over an atom payload.
pub trait AtomView<'a>: Sized {
    /// Generated constant identifying the type.
    const TYPE_ID: TypeId;
    /// Required exact byte length for the payload.
    const BYTE_LEN: usize;

    /// Parse a raw byte slice into the typed view.
    fn parse(bytes: &'a [u8]) -> Option<Self>;

    /// Safe downcast from a generic `AtomPayload`.
    #[inline]
    fn try_from_payload(payload: &'a AtomPayload) -> Option<Self> {
        if payload.type_id != Self::TYPE_ID {
            return None;
        }
        if payload.bytes.len() != Self::BYTE_LEN {
            return None;
        }
        Self::parse(payload.bytes.as_ref())
    }
}
