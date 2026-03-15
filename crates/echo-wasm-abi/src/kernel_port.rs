// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! App-agnostic kernel boundary trait and ABI response types.
//!
//! This module defines the contract between a WASM host adapter and a
//! deterministic simulation kernel. The [`KernelPort`] trait is byte-oriented
//! and app-agnostic: any engine that can ingest intents, execute ticks, and
//! drain materialized output can implement it.
//!
//! # ABI Version
//!
//! The current ABI version is [`ABI_VERSION`] (1). All response types are
//! CBOR-encoded using the canonical rules defined in `docs/js-cbor-mapping.md`.
//! Breaking changes to response shapes or error codes require a bump to the
//! ABI version.
//!
//! # Error Protocol
//!
//! Methods return `Result<T, AbiError>`. The WASM boundary layer encodes:
//! - `Ok(value)` → CBOR of `{ "ok": true, ...value_fields }`
//! - `Err(error)` → CBOR of `{ "ok": false, "code": u32, "message": string }`
//!
//! This envelope allows JS callers to distinguish success from failure by
//! checking the `ok` field before further decoding.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Current ABI version for the kernel port contract.
///
/// Increment when response types, error codes, or method signatures change
/// in a backward-incompatible way.
pub const ABI_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Error codes
// ---------------------------------------------------------------------------

/// Machine-readable error codes for ABI errors.
pub mod error_codes {
    /// Kernel has not been initialized (call `init()` first).
    pub const NOT_INITIALIZED: u32 = 1;
    /// The intent payload was malformed or rejected by the engine.
    pub const INVALID_INTENT: u32 = 2;
    /// An internal engine error occurred during processing.
    pub const ENGINE_ERROR: u32 = 3;
    /// Legacy snapshot/history tick index is out of bounds.
    pub const LEGACY_INVALID_TICK: u32 = 4;
    /// The requested operation is not yet supported by this kernel.
    pub const NOT_SUPPORTED: u32 = 5;
    /// CBOR encoding or decoding failed.
    pub const CODEC_ERROR: u32 = 6;
    /// The provided payload bytes were invalid or corrupted.
    pub const INVALID_PAYLOAD: u32 = 7;
    /// The requested worldline is not registered.
    pub const INVALID_WORLDLINE: u32 = 8;
    /// The requested observation tick is not available.
    pub const INVALID_TICK: u32 = 9;
    /// The requested frame/projection pairing is invalid.
    pub const UNSUPPORTED_FRAME_PROJECTION: u32 = 10;
    /// Query observation is not implemented yet.
    pub const UNSUPPORTED_QUERY: u32 = 11;
    /// The requested observation cannot be produced at this coordinate.
    pub const OBSERVATION_UNAVAILABLE: u32 = 12;
}

// ---------------------------------------------------------------------------
// ABI error type
// ---------------------------------------------------------------------------

/// Structured error returned by kernel port operations.
///
/// Serialized to CBOR with `{ "ok": false, "code": u32, "message": string }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiError {
    /// Machine-readable error code (see [`error_codes`]).
    pub code: u32,
    /// Human-readable error description.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Response DTOs
// ---------------------------------------------------------------------------

/// Response from [`KernelPort::dispatch_intent`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchResponse {
    /// Whether the intent was newly accepted (false if duplicate).
    pub accepted: bool,
    /// Content-addressed intent identifier (BLAKE3 hash, 32 bytes).
    pub intent_id: Vec<u8>,
}

/// Current head state of the kernel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadInfo {
    /// Current tick count (number of committed ticks).
    pub tick: u64,
    /// Graph-only state hash (32 bytes).
    pub state_root: Vec<u8>,
    /// Canonical commit hash (32 bytes).
    pub commit_id: Vec<u8>,
}

/// Response from [`KernelPort::step`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepResponse {
    /// Number of ticks actually executed (may be less than budget).
    pub ticks_executed: u32,
    /// Head state after stepping.
    pub head: HeadInfo,
}

/// A single materialized channel output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelData {
    /// Channel identifier (32 bytes).
    pub channel_id: Vec<u8>,
    /// Raw finalized data for this channel.
    pub data: Vec<u8>,
}

/// Coordinate selector for an observation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationCoordinate {
    /// Worldline to observe.
    pub worldline_id: Vec<u8>,
    /// Requested coordinate within the worldline.
    pub at: ObservationAt,
}

/// Requested position within a worldline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservationAt {
    /// Observe the current frontier.
    Frontier,
    /// Observe a specific committed historical tick.
    Tick {
        /// Zero-based historical tick index.
        tick: u64,
    },
}

/// Declared semantic frame for an observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationFrame {
    /// Commit-boundary metadata and snapshots.
    CommitBoundary,
    /// Recorded truth emitted by committed history.
    RecordedTruth,
    /// Query-shaped observation frame.
    QueryView,
}

/// Requested observation projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservationProjection {
    /// Head metadata at the resolved coordinate.
    Head,
    /// Snapshot metadata at the resolved coordinate.
    Snapshot,
    /// Recorded truth channel payloads.
    TruthChannels {
        /// Optional channel filter. `None` means all recorded channels.
        channels: Option<Vec<Vec<u8>>>,
    },
    /// Query payload placeholder.
    Query {
        /// Stable query identifier.
        query_id: u32,
        /// Canonical vars payload bytes.
        vars_bytes: Vec<u8>,
    },
}

/// Canonical observation request DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationRequest {
    /// Requested worldline coordinate.
    pub coordinate: ObservationCoordinate,
    /// Declared read frame.
    pub frame: ObservationFrame,
    /// Requested projection within that frame.
    pub projection: ObservationProjection,
}

/// Resolved coordinate returned with every observation artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedObservationCoordinate {
    /// Observation contract version.
    pub observation_version: u32,
    /// Worldline actually observed.
    pub worldline_id: Vec<u8>,
    /// Original coordinate selector from the request.
    pub requested_at: ObservationAt,
    /// Concrete resolved committed tick.
    pub resolved_tick: u64,
    /// Canonical state root at the resolved coordinate.
    pub state_root: Vec<u8>,
    /// Canonical commit hash at the resolved coordinate.
    pub commit_hash: Vec<u8>,
}

/// Minimal head observation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadObservation {
    /// Current committed tick count at the observed frontier.
    pub tick: u64,
    /// Graph-only state hash (32 bytes).
    pub state_root: Vec<u8>,
    /// Canonical commit hash (32 bytes).
    pub commit_id: Vec<u8>,
}

/// Minimal historical snapshot payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotObservation {
    /// Historical tick index being observed.
    pub tick: u64,
    /// Graph-only state hash (32 bytes).
    pub state_root: Vec<u8>,
    /// Canonical commit hash (32 bytes).
    pub commit_id: Vec<u8>,
}

/// Observation payload variants returned by the kernel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservationPayload {
    /// Head payload.
    Head {
        /// Head observation.
        head: HeadObservation,
    },
    /// Snapshot payload.
    Snapshot {
        /// Snapshot observation.
        snapshot: SnapshotObservation,
    },
    /// Recorded truth payload.
    TruthChannels {
        /// Recorded channel payloads.
        channels: Vec<ChannelData>,
    },
    /// Query payload.
    QueryBytes {
        /// Raw query result bytes.
        data: Vec<u8>,
    },
}

/// Canonical hash input for an observation artifact.
///
/// This excludes `artifact_hash` itself so kernels can compute the hash over
/// the resolved coordinate, frame, projection, and canonical payload bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationHashInput {
    /// Resolved coordinate metadata.
    pub resolved: ResolvedObservationCoordinate,
    /// Declared semantic frame.
    pub frame: ObservationFrame,
    /// Declared projection.
    pub projection: ObservationProjection,
    /// Observation payload.
    pub payload: ObservationPayload,
}

/// Full observation artifact returned by `observe(...)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationArtifact {
    /// Resolved coordinate metadata.
    pub resolved: ResolvedObservationCoordinate,
    /// Declared semantic frame.
    pub frame: ObservationFrame,
    /// Declared projection.
    pub projection: ObservationProjection,
    /// Canonical artifact hash.
    pub artifact_hash: Vec<u8>,
    /// Observation payload.
    pub payload: ObservationPayload,
}

/// Response from [`KernelPort::drain_view_ops`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrainResponse {
    /// Finalized channel outputs since the last drain.
    pub channels: Vec<ChannelData>,
}

/// Registry and handshake metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryInfo {
    /// Codec identifier for the installed schema (if any).
    pub codec_id: Option<String>,
    /// Registry version string (if any).
    pub registry_version: Option<String>,
    /// SHA-256 hex digest of the schema (if any).
    pub schema_sha256_hex: Option<String>,
    /// ABI version of the kernel port contract.
    pub abi_version: u32,
}

// ---------------------------------------------------------------------------
// CBOR wire envelope
// ---------------------------------------------------------------------------

/// Success envelope wrapping a response value for CBOR encoding.
///
/// The `ok: true` field allows JS callers to distinguish success from error
/// without inspecting the inner type.
///
/// Construct via [`OkEnvelope::new`] to guarantee `ok` is always `true`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkEnvelope<T> {
    /// Always `true` for success responses.
    ok: bool,
    /// The response payload.
    #[serde(flatten)]
    pub data: T,
}

impl<T> OkEnvelope<T> {
    /// Create a success envelope. Sets `ok` to `true` automatically.
    ///
    /// `T` may be a reference (e.g., `&impl Serialize`) when the envelope
    /// is used for immediate serialization and not stored.
    pub fn new(data: T) -> Self {
        Self { ok: true, data }
    }
}

/// Wrapper for raw CBOR byte payloads in success envelopes.
///
/// Used by endpoints that return pre-encoded CBOR bytes (e.g., `snapshot_at`,
/// `execute_query`). Unlike struct responses that flatten into the envelope,
/// raw bytes are placed in a `data` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawBytesResponse {
    /// The raw CBOR-encoded payload.
    pub data: Vec<u8>,
}

/// Error envelope for CBOR encoding.
///
/// Construct via [`ErrEnvelope::new`] to guarantee `ok` is always `false`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrEnvelope {
    /// Always `false` for error responses.
    ok: bool,
    /// Machine-readable error code.
    pub code: u32,
    /// Human-readable error description.
    pub message: String,
}

impl ErrEnvelope {
    /// Create an error envelope. Sets `ok` to `false` automatically.
    pub fn new(code: u32, message: String) -> Self {
        Self {
            ok: false,
            code,
            message,
        }
    }
}

// ---------------------------------------------------------------------------
// KernelPort trait
// ---------------------------------------------------------------------------

/// App-agnostic kernel boundary for WASM host adapters.
///
/// Implementors wrap a specific simulation engine and expose the byte-level
/// contract expected by WASM exports. All response data is returned as typed
/// Rust structs; the WASM boundary layer handles CBOR encoding.
///
/// # App-Agnostic Design
///
/// The trait makes no assumptions about what rules the engine runs, what
/// schema is installed, or what domain the simulation models. It operates
/// purely on canonical intent bytes, tick budgets, and materialized channel
/// outputs. App-specific behavior is injected by the kernel implementation,
/// not by the boundary.
///
/// # Thread Safety
///
/// WASM is single-threaded, so `KernelPort` does not require `Send` or `Sync`.
/// Native test harnesses should use appropriate synchronization if needed.
pub trait KernelPort {
    /// Ingest a canonical intent envelope into the kernel inbox.
    ///
    /// The kernel content-addresses the intent and returns whether it was
    /// newly accepted or a duplicate.
    fn dispatch_intent(&mut self, intent_bytes: &[u8]) -> Result<DispatchResponse, AbiError>;

    /// Execute deterministic ticks up to the given budget.
    ///
    /// Returns the number of ticks actually executed and the head state
    /// after stepping. A budget of 0 is a no-op that returns the current head.
    fn step(&mut self, budget: u32) -> Result<StepResponse, AbiError>;

    /// Observe a worldline at an explicit coordinate and frame.
    ///
    /// The default implementation reports that the observation contract is not
    /// supported by this kernel implementation.
    fn observe(&self, _request: ObservationRequest) -> Result<ObservationArtifact, AbiError> {
        Err(AbiError {
            code: error_codes::NOT_SUPPORTED,
            message: "observe is not supported by this kernel".into(),
        })
    }

    /// Drain materialized ViewOps channels since the last drain.
    ///
    /// Returns finalized channel data. Calling drain twice without an
    /// intervening step returns empty channels.
    fn drain_view_ops(&mut self) -> Result<DrainResponse, AbiError>;

    /// Get the current head state (tick, state_root, commit_id).
    fn get_head(&self) -> Result<HeadInfo, AbiError>;

    /// Execute a read-only query against the current state.
    ///
    /// Returns CBOR-encoded query results. The default implementation returns
    /// `NOT_SUPPORTED`; override when the engine has a query dispatcher.
    fn execute_query(&self, _query_id: u32, _vars_bytes: &[u8]) -> Result<Vec<u8>, AbiError> {
        Err(AbiError {
            code: error_codes::NOT_SUPPORTED,
            message: "execute_query is not supported by this kernel".into(),
        })
    }

    /// Replay to a specific tick and return the snapshot as CBOR bytes.
    fn snapshot_at(&mut self, tick: u64) -> Result<Vec<u8>, AbiError>;

    /// Render a snapshot into ViewOps for visualization.
    ///
    /// The default implementation returns `NOT_SUPPORTED`; override when the
    /// engine has snapshot rendering.
    fn render_snapshot(&self, _snapshot_bytes: &[u8]) -> Result<Vec<u8>, AbiError> {
        Err(AbiError {
            code: error_codes::NOT_SUPPORTED,
            message: "render_snapshot is not supported by this kernel".into(),
        })
    }

    /// Return registry and handshake metadata.
    fn registry_info(&self) -> RegistryInfo;
}
