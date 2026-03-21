// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! App-agnostic kernel boundary trait and ABI response types.
//!
//! This module defines the contract between a WASM host adapter and a
//! deterministic simulation kernel. The [`KernelPort`] trait is byte-oriented
//! and app-agnostic: any engine that can ingest intents, advance logical
//! scheduler cycles, and serve observation-backed reads can implement it.
//!
//! # ABI Version
//!
//! The current ABI version is [`ABI_VERSION`] (3). All response types are
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
pub const ABI_VERSION: u32 = 3;

macro_rules! logical_counter {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(pub u64);
    };
}

logical_counter!(
    /// Per-worldline logical coordinate in host-visible metadata.
    ///
    /// The meaning of `0` depends on the surface carrying it:
    ///
    /// - In historical coordinates such as [`ObservationAt::Tick`], `0` names
    ///   the first committed append.
    /// - In frontier/head metadata such as [`HeadInfo`] and
    ///   [`HeadObservation`], `0` paired with `commit_global_tick = None`
    ///   means the worldline is still at `U0` and has not committed anything.
    WorldlineTick
);

logical_counter!(
    /// Runtime-cycle correlation stamp in host-visible metadata.
    GlobalTick
);

logical_counter!(
    /// Control-plane run generation token.
    RunId
);

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
    /// Reserved for the removed v1 snapshot adapter.
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
    /// The provided control intent payload was invalid.
    pub const INVALID_CONTROL: u32 = 13;
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
    /// Scheduler status after the intent is ingested or applied.
    pub scheduler_status: SchedulerStatus,
}

/// Current head state of the kernel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadInfo {
    /// Current committed frontier position for the worldline.
    ///
    /// `worldline_tick == WorldlineTick(0)` together with
    /// `commit_global_tick == None` represents the empty `U0` frontier with no
    /// committed appends yet.
    pub worldline_tick: WorldlineTick,
    /// Runtime cycle stamp for the current committed head, if any.
    ///
    /// `None` means the worldline has not committed anything yet.
    pub commit_global_tick: Option<GlobalTick>,
    /// Canonical full-state root hash (32 bytes).
    pub state_root: Vec<u8>,
    /// Canonical commit hash (32 bytes).
    pub commit_id: Vec<u8>,
}

/// Declared scheduler mode visible to hosts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SchedulerMode {
    /// Run until no runnable work remains, optionally bounded by cycle count.
    UntilIdle {
        /// Maximum cycles to run before yielding.
        ///
        /// When present, this value must be non-zero. `Some(0)` is rejected as
        /// [`error_codes::INVALID_CONTROL`].
        cycle_limit: Option<u32>,
    },
}

/// Scheduler lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerState {
    /// Scheduler is inactive and no run is currently executing.
    Inactive,
    /// Scheduler is actively executing runtime cycles.
    Running,
    /// Scheduler will stop after the current cycle boundary.
    Stopping,
}

/// Runtime work availability at the current scheduler boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkState {
    /// No runnable work remains at the current boundary.
    Quiescent,
    /// Runnable work exists and can be scheduled immediately.
    RunnablePending,
    /// Work exists, but all current heads are blocked or dormant.
    BlockedOnly,
}

/// Completion reason for the most recent bounded or active run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunCompletion {
    /// The most recent run ended because no runnable work remained.
    Quiesced,
    /// The most recent run ended because work remained but all heads were blocked or dormant.
    BlockedOnly,
    /// The most recent run ended because its cycle bound was reached.
    CycleLimitReached,
    /// The most recent run ended because stop was requested.
    Stopped,
}

/// Declarative host intent for head admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadEligibility {
    /// Head is intentionally excluded from scheduling.
    Dormant,
    /// Head is admitted and may participate if otherwise runnable.
    Admitted,
}

/// Runtime truth about a head's scheduler disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadDisposition {
    /// Head is intentionally excluded from scheduling.
    Dormant,
    /// Head is currently runnable by the scheduler.
    Runnable,
    /// Head is admitted but cannot currently run.
    Blocked,
    /// Head has been retired and cannot be reactivated.
    Retired,
}

/// Current scheduler metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerStatus {
    /// Current scheduler lifecycle state.
    pub state: SchedulerState,
    /// Active scheduler mode, if any run is configured.
    pub active_mode: Option<SchedulerMode>,
    /// Runtime work availability at the current scheduler boundary.
    pub work_state: WorkState,
    /// Current run generation token, if a run is active or recently completed.
    pub run_id: Option<RunId>,
    /// Latest completed runtime cycle, even if no commit occurred.
    pub latest_cycle_global_tick: Option<GlobalTick>,
    /// Latest runtime cycle that produced a commit.
    pub latest_commit_global_tick: Option<GlobalTick>,
    /// Most recent cycle that transitioned the runtime into quiescence.
    pub last_quiescent_global_tick: Option<GlobalTick>,
    /// Completion reason for the most recent run, if one has completed.
    pub last_run_completion: Option<RunCompletion>,
}

/// Stable head key used by control intents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadKey {
    /// Worldline that owns the head.
    pub worldline_id: Vec<u8>,
    /// Stable head identifier within that worldline.
    pub head_id: Vec<u8>,
}

/// Privileged control intents routed through the same intent intake surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ControlIntentV1 {
    /// Request a bounded or unbounded runtime run.
    Start {
        /// Requested scheduler mode.
        mode: SchedulerMode,
    },
    /// Request scheduler stop for implementations with persistent runs.
    ///
    /// The current engine-backed `warp-wasm` kernel executes `Start` runs
    /// synchronously, so hosts normally observe the completed run via
    /// `last_run_completion` instead of an intermediate stopping state.
    Stop,
    /// Change declarative head admission.
    SetHeadEligibility {
        /// Target head whose eligibility should change.
        head: HeadKey,
        /// New declarative eligibility for that head.
        eligibility: HeadEligibility,
    },
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
        /// Zero-based committed append index.
        ///
        /// `WorldlineTick(0)` means "the first committed append", not `U0`.
        worldline_tick: WorldlineTick,
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
    /// Concrete resolved worldline coordinate.
    ///
    /// For historical requests this is a zero-based committed append index. For
    /// empty-frontier observations it may be `WorldlineTick(0)` paired with
    /// `commit_global_tick == None` to represent `U0`.
    pub resolved_worldline_tick: WorldlineTick,
    /// Commit cycle stamp for the resolved commit, if any.
    ///
    /// `None` indicates that the resolved coordinate is the empty `U0`
    /// frontier rather than a committed append.
    pub commit_global_tick: Option<GlobalTick>,
    /// Observation freshness watermark after resolving this artifact.
    pub observed_after_global_tick: Option<GlobalTick>,
    /// Canonical state root at the resolved coordinate.
    pub state_root: Vec<u8>,
    /// Canonical commit hash at the resolved coordinate.
    pub commit_hash: Vec<u8>,
}

/// Minimal head observation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadObservation {
    /// Current committed frontier position at the observed frontier.
    ///
    /// `worldline_tick == WorldlineTick(0)` together with
    /// `commit_global_tick == None` means the observed frontier is still `U0`
    /// with no committed appends.
    pub worldline_tick: WorldlineTick,
    /// Commit cycle stamp for the observed head, if any.
    ///
    /// `None` means the observed frontier has not committed anything yet.
    pub commit_global_tick: Option<GlobalTick>,
    /// Canonical full-state root hash (32 bytes).
    pub state_root: Vec<u8>,
    /// Canonical commit hash (32 bytes).
    pub commit_id: Vec<u8>,
}

/// Minimal historical snapshot payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotObservation {
    /// Historical committed append index being observed.
    ///
    /// `WorldlineTick(0)` names the first committed append.
    pub worldline_tick: WorldlineTick,
    /// Commit cycle stamp for the observed historical commit, if any.
    ///
    /// Historical snapshots are expected to carry `Some(_)`; `None` is
    /// reserved for empty-frontier metadata surfaces such as [`HeadInfo`].
    pub commit_global_tick: Option<GlobalTick>,
    /// Canonical full-state root hash (32 bytes).
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
/// purely on canonical intent bytes, scheduler status inspection, and explicit observation
/// requests. App-specific behavior is injected by the kernel implementation,
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

    /// Observe a worldline at an explicit coordinate and frame.
    ///
    /// This is the only canonical public read entrypoint in ABI v3. The
    /// default implementation reports that the observation contract is not
    /// supported by this kernel implementation.
    fn observe(&self, _request: ObservationRequest) -> Result<ObservationArtifact, AbiError> {
        Err(AbiError {
            code: error_codes::NOT_SUPPORTED,
            message: "observe is not supported by this kernel".into(),
        })
    }

    /// Return registry and handshake metadata.
    fn registry_info(&self) -> RegistryInfo;

    /// Return read-only scheduler status metadata.
    fn scheduler_status(&self) -> Result<SchedulerStatus, AbiError>;
}
