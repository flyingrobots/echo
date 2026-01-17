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
mod cmd;
mod constants;
mod engine_impl;
mod footprint;
mod graph;
mod ident;
/// Canonical inbox management for deterministic intent sequencing.
pub mod inbox;
/// Materialization subsystem for deterministic channel-based output.
pub mod materialization;
mod payload;
mod receipt;
mod record;
mod rule;
mod sandbox;
mod scheduler;
#[cfg(feature = "serde")]
mod serializable;
mod snapshot;
mod telemetry;
mod tick_patch;
mod tx;
mod warp_state;

// Re-exports for stable public API
pub use attachment::{
    AtomPayload, AttachmentKey, AttachmentOwner, AttachmentPlane, AttachmentValue, Codec,
    CodecRegistry, DecodeError, ErasedCodec, RegistryError,
};
pub use constants::{blake3_empty, digest_len0_u64, POLICY_ID_NO_POLICY_V0};
pub use engine_impl::{
    scope_hash, ApplyResult, DispatchDisposition, Engine, EngineBuilder, EngineError,
    ExistingState, FreshStore, IngestDisposition,
};
pub use footprint::{pack_port_key, AttachmentSet, Footprint, IdSet, PortKey, PortSet};
pub use graph::GraphStore;
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
pub use receipt::{TickReceipt, TickReceiptDisposition, TickReceiptEntry, TickReceiptRejection};
pub use record::{EdgeRecord, NodeRecord};
pub use rule::{ConflictPolicy, ExecuteFn, MatchFn, PatternGraph, RewriteRule};
pub use sandbox::{build_engine, run_pair_determinism, DeterminismError, EchoConfig};
pub use scheduler::SchedulerKind;
#[cfg(feature = "serde")]
pub use serializable::{
    SerializableReceipt, SerializableReceiptEntry, SerializableSnapshot, SerializableTick,
};
pub use snapshot::Snapshot;
pub use telemetry::{NullTelemetrySink, TelemetrySink};
pub use tick_patch::{
    slice_worldline_indices, PortalInit, SlotId, TickCommitStatus, TickPatchError, WarpOp,
    WarpTickPatchV1,
};
pub use tx::TxId;
pub use warp_state::{WarpInstance, WarpState};

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
