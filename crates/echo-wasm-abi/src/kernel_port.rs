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
use core::fmt;
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};

/// Current ABI version for the kernel port contract.
///
/// Increment when response types, error codes, or method signatures change
/// in a backward-incompatible way.
pub const ABI_VERSION: u32 = 3;

fn deserialize_opaque_id<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct OpaqueIdVisitor;

    impl Visitor<'_> for OpaqueIdVisitor {
        type Value = [u8; 32];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("exactly 32 bytes")
        }

        fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value
                .try_into()
                .map_err(|_| E::invalid_length(value.len(), &self))
        }

        fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_bytes(&value)
        }
    }

    deserializer.deserialize_bytes(OpaqueIdVisitor)
}

macro_rules! opaque_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Reconstructs an id from its canonical 32-byte representation.
            #[must_use]
            pub fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Returns the canonical 32-byte representation.
            #[must_use]
            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_bytes(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserialize_opaque_id(deserializer).map(Self)
            }
        }
    };
}

macro_rules! logical_counter {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u64);

        impl $name {
            /// Zero value for this logical counter.
            pub const ZERO: Self = Self(0);
            /// Largest representable value for this logical counter.
            pub const MAX: Self = Self(u64::MAX);

            /// Builds the counter from its raw logical value.
            #[must_use]
            pub const fn from_raw(raw: u64) -> Self {
                Self(raw)
            }

            /// Returns the raw logical value.
            #[must_use]
            pub const fn as_u64(self) -> u64 {
                self.0
            }
        }
    };
}

opaque_id!(
    /// Opaque stable identifier for a worldline.
    ///
    /// This is the canonical 32-byte worldline-id hash, carried as typed
    /// metadata rather than a generic byte vector.
    WorldlineId
);

opaque_id!(
    /// Opaque stable identifier for a strand.
    StrandId
);

opaque_id!(
    /// Opaque stable identifier for a head within a worldline.
    ///
    /// This is the canonical 32-byte head-id hash, carried as typed metadata
    /// rather than a generic byte vector.
    HeadId
);

opaque_id!(
    /// Opaque stable identifier for a published local site.
    NeighborhoodSiteId
);

logical_counter!(
    /// Per-worldline append identity for committed history.
    WorldlineTick
);

logical_counter!(
    /// Runtime-cycle correlation stamp. No wall-clock semantics.
    GlobalTick
);

logical_counter!(
    /// Control-plane generation token for scheduler runs.
    ///
    /// This value is not provenance, replay state, or hash input.
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
    /// The requested strand is not registered.
    pub const INVALID_STRAND: u32 = 14;
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
    ///
    /// For the empty `U0` frontier this is still populated: it is the
    /// deterministic root hash of the current `U0` materialization.
    pub state_root: Vec<u8>,
    /// Canonical frontier hash (32 bytes).
    ///
    /// For the empty `U0` frontier this is the deterministic frontier snapshot
    /// hash over that `U0` materialization, not an absent sentinel.
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
///
/// ABI invariants:
///
/// - `run_id` is `Some(...)` once a run starts and remains `Some(...)` after
///   that run completes until a later `Start` replaces it or `init()` resets
///   the kernel.
/// - `active_mode` is `Some(...)` only while the scheduler is active
///   (`state = running` or `state = stopping`). The current engine-backed
///   implementation runs `Start` synchronously, so hosts normally observe
///   `active_mode = None` together with the completed
///   `last_run_completion`.
/// - `latest_commit_global_tick <= latest_cycle_global_tick` whenever both are
///   present.
/// - `last_quiescent_global_tick` is monotonic for the lifetime of one
///   initialized kernel. It records the most recent transition into
///   quiescence and does not regress when work later becomes runnable again.
/// - `latest_cycle_global_tick = None` means the kernel has not completed a
///   runtime cycle yet. `last_run_completion = None` means no run has
///   completed since initialization or the most recent accepted `Start` has
///   not finished yet.
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

/// Stable writer-head key used by control intents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriterHeadKey {
    /// Worldline that owns the head.
    pub worldline_id: WorldlineId,
    /// Stable head identifier within that worldline.
    pub head_id: HeadId,
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
        head: WriterHeadKey,
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
    pub worldline_id: WorldlineId,
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
    pub worldline_id: WorldlineId,
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
    ///
    /// Empty-frontier `U0` observations still carry the deterministic `U0`
    /// materialization root here.
    pub state_root: Vec<u8>,
    /// Canonical frontier/commit hash at the resolved coordinate.
    ///
    /// Empty-frontier `U0` observations still carry the deterministic frontier
    /// snapshot hash for that `U0` materialization here.
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
    ///
    /// Empty-frontier `U0` observations still carry the deterministic `U0`
    /// materialization root here.
    pub state_root: Vec<u8>,
    /// Canonical frontier hash (32 bytes).
    ///
    /// Empty-frontier `U0` observations still carry the deterministic frontier
    /// snapshot hash for that `U0` materialization here.
    pub commit_id: Vec<u8>,
}

/// Minimal snapshot payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotObservation {
    /// Snapshot coordinate being observed.
    ///
    /// Historical observations use zero-based committed append indices.
    /// `ObservationAt::Frontier + Snapshot` may also resolve to
    /// `WorldlineTick(0)` plus `commit_global_tick = None` to describe the
    /// empty `U0` frontier snapshot.
    pub worldline_tick: WorldlineTick,
    /// Commit cycle stamp for the observed historical commit, if any.
    ///
    /// Historical snapshots carry `Some(_)`; `None` is reserved for an
    /// empty-frontier `U0` snapshot resolved from `ObservationAt::Frontier`.
    pub commit_global_tick: Option<GlobalTick>,
    /// Canonical full-state root hash (32 bytes).
    ///
    /// Empty-frontier `U0` snapshots still carry the deterministic `U0`
    /// materialization root here.
    pub state_root: Vec<u8>,
    /// Canonical snapshot hash (32 bytes).
    ///
    /// Empty-frontier `U0` snapshots still carry the deterministic frontier
    /// snapshot hash for that `U0` materialization here.
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

/// Whether a published local site is singleton or plural.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SitePlurality {
    /// Only the primary lane participates.
    Singleton,
    /// One or more additional participants are present.
    Braided,
}

/// Role a participant plays in a local site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantRole {
    /// The directly observed lane.
    Primary,
    /// The base coordinate from which the primary strand forked.
    BaseAnchor,
    /// A read-only support-pinned lane.
    Support,
}

/// One participating lane in a published local site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteParticipant {
    /// The participant's worldline.
    pub worldline_id: WorldlineId,
    /// Strand identity when the participant is a strand.
    pub strand_id: Option<StrandId>,
    /// Participant role within the site.
    pub role: ParticipantRole,
    /// Exact participant tick.
    pub tick: WorldlineTick,
    /// Canonical state hash for the participant at that tick.
    pub state_hash: Vec<u8>,
}

/// Published local-site object for one observation anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeighborhoodSite {
    /// Stable identity for this local site.
    pub site_id: NeighborhoodSiteId,
    /// Anchor coordinate for the site.
    pub anchor: ResolvedObservationCoordinate,
    /// Singleton or plural site truth.
    pub plurality: SitePlurality,
    /// Participating lanes for the site.
    pub participants: Vec<SiteParticipant>,
}

/// Shared lawful outcome kind for observer/debugger publication families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdmissionOutcomeKind {
    /// One lawful derived result exists.
    Derived,
    /// Multiple lawful results coexist.
    Plural,
    /// A lawful conflict artifact was produced.
    Conflict,
    /// Lawful admission was obstructed.
    Obstruction,
}

/// Shared plurality for one published neighborhood core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NeighborhoodPlurality {
    /// Only the primary lane participates.
    Singleton,
    /// Multiple lanes participate in the local site.
    Plural,
}

/// Shared participant role for one published neighborhood core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NeighborhoodParticipantRole {
    /// The directly observed lane.
    Primary,
    /// The fork/source lane anchoring the primary strand.
    BasisAnchor,
    /// A read-only support lane.
    Support,
}

/// Shared participant for one published neighborhood core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeighborhoodParticipant {
    /// Stable participant identity within the published site.
    pub participant_id: String,
    /// Stable lane identity for the participant carrier.
    pub lane_id: String,
    /// Optional strand identity when the participant is strand-backed.
    pub strand_id: Option<String>,
    /// Participant role within the site.
    pub role: NeighborhoodParticipantRole,
    /// Exact participant frame index.
    pub frame_index: u64,
    /// Canonical state hash for the participant at that frame.
    pub state_hash: String,
}

/// Shared observer/debugger publication for one local neighborhood core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeighborhoodCore {
    /// Stable identity for this published neighborhood core.
    pub site_id: String,
    /// Stable lane identity for the anchor worldline.
    pub anchor_lane_id: String,
    /// Exact resolved anchor frame index.
    pub anchor_frame_index: u64,
    /// Optional anchor head identity, when the kernel publishes one truthfully.
    pub anchor_head_id: Option<String>,
    /// Top-level lawful outcome kind for the site.
    pub outcome_kind: AdmissionOutcomeKind,
    /// Shared singleton-vs-plural truth.
    pub plurality: NeighborhoodPlurality,
    /// Participating lanes for the published site.
    pub participants: Vec<NeighborhoodParticipant>,
    /// Narrow human-readable summary for debugger surfaces.
    pub summary: String,
}

/// Request payload for strand settlement publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementRequest {
    /// Strand being compared, planned, or settled.
    pub strand_id: StrandId,
}

/// One stable provenance coordinate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRef {
    /// Worldline that owns this coordinate.
    pub worldline_id: WorldlineId,
    /// Tick within that worldline.
    pub worldline_tick: WorldlineTick,
    /// Canonical commit hash for the coordinate.
    pub commit_hash: Vec<u8>,
}

/// The recorded base coordinate for a strand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseRef {
    /// Source worldline the strand forked from.
    pub source_worldline_id: WorldlineId,
    /// Last included tick in the copied prefix.
    pub fork_tick: WorldlineTick,
    /// Commit hash at the fork tick.
    pub commit_hash: Vec<u8>,
    /// Boundary state hash at the fork tick.
    pub boundary_hash: Vec<u8>,
    /// Full provenance coordinate for the fork boundary.
    pub provenance_ref: ProvenanceRef,
}

/// Deterministic reasons a settlement step could not be imported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictReason {
    /// The source step depends on unsupported channel policy detail.
    ChannelPolicyConflict,
    /// The source step is not replayable under current settlement law.
    UnsupportedImport,
    /// The base worldline advanced away from the strand's fork boundary.
    BaseDivergence,
    /// The source and target lanes disagree on time-quantum assumptions.
    QuantumMismatch,
}

/// Compare output for one strand suffix relative to its recorded base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementDelta {
    /// Strand being compared.
    pub strand_id: StrandId,
    /// Recorded base coordinate for the strand.
    pub base_ref: BaseRef,
    /// Child worldline carrying speculative suffix history.
    pub source_worldline_id: WorldlineId,
    /// First suffix tick eligible for settlement consideration.
    pub source_suffix_start_tick: WorldlineTick,
    /// Last suffix tick currently present on the source worldline.
    pub source_suffix_end_tick: WorldlineTick,
    /// Authoritative source provenance refs in settlement order.
    pub source_entries: Vec<ProvenanceRef>,
}

/// One accepted unit of source provenance eligible for import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportCandidate {
    /// Source provenance coordinate being imported.
    pub source_ref: ProvenanceRef,
    /// Source writer head when the imported entry was a local commit.
    pub source_head_key: Option<WriterHeadKey>,
    /// Stable imported operation identifier.
    pub imported_op_id: Vec<u8>,
}

/// One unresolved settlement residue draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictArtifactDraft {
    /// Stable artifact identifier for this residue record.
    pub artifact_id: Vec<u8>,
    /// Source provenance coordinate that could not be imported.
    pub source_ref: ProvenanceRef,
    /// Channels implicated by the unresolved source entry.
    pub channel_ids: Vec<Vec<u8>>,
    /// Deterministic reason the source entry was rejected.
    pub reason: ConflictReason,
}

/// One deterministic settlement decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SettlementDecision {
    /// Source history that can be imported into the base worldline.
    ImportCandidate {
        /// Accepted import detail.
        candidate: ImportCandidate,
    },
    /// Source history that must remain explicit residue.
    ConflictArtifact {
        /// Residue detail.
        artifact: ConflictArtifactDraft,
    },
}

/// Deterministic settlement evaluation for one strand against its base worldline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementPlan {
    /// Strand being settled.
    pub strand_id: StrandId,
    /// Base worldline receiving settlement output.
    pub target_worldline: WorldlineId,
    /// Provenance coordinate the strand claims as its base.
    pub target_base_ref: ProvenanceRef,
    /// Ordered import or conflict decisions for the suffix.
    pub decisions: Vec<SettlementDecision>,
}

/// Runtime result of executing one settlement plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementResult {
    /// Deterministic plan that was executed.
    pub plan: SettlementPlan,
    /// Target-worldline refs appended as `MergeImport`.
    pub appended_imports: Vec<ProvenanceRef>,
    /// Target-worldline refs appended as `ConflictArtifact`.
    pub appended_conflicts: Vec<ProvenanceRef>,
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

    /// Publish the local neighborhood site for an explicit observation request.
    ///
    /// This is the canonical public read for plural local-site inspection.
    /// The default implementation reports that neighborhood publication is not
    /// supported by this kernel implementation.
    fn observe_neighborhood_site(
        &self,
        _request: ObservationRequest,
    ) -> Result<NeighborhoodSite, AbiError> {
        Err(AbiError {
            code: error_codes::NOT_SUPPORTED,
            message: "observe_neighborhood_site is not supported by this kernel".into(),
        })
    }

    /// Publish the shared neighborhood-core family projection for an explicit
    /// observation request.
    ///
    /// This is the canonical shared observer/debugger read for the first
    /// neighborhood-core family slice. The default implementation reports that
    /// this projection is not supported by the kernel implementation.
    fn observe_neighborhood_core(
        &self,
        _request: ObservationRequest,
    ) -> Result<NeighborhoodCore, AbiError> {
        Err(AbiError {
            code: error_codes::NOT_SUPPORTED,
            message: "observe_neighborhood_core is not supported by this kernel".into(),
        })
    }

    /// Compare a strand suffix against its recorded base coordinate.
    ///
    /// The default implementation reports that settlement publication is not
    /// supported by this kernel implementation.
    fn compare_settlement(&self, _request: SettlementRequest) -> Result<SettlementDelta, AbiError> {
        Err(AbiError {
            code: error_codes::NOT_SUPPORTED,
            message: "compare_settlement is not supported by this kernel".into(),
        })
    }

    /// Produce a deterministic settlement plan for one strand.
    ///
    /// The default implementation reports that settlement publication is not
    /// supported by this kernel implementation.
    fn plan_settlement(&self, _request: SettlementRequest) -> Result<SettlementPlan, AbiError> {
        Err(AbiError {
            code: error_codes::NOT_SUPPORTED,
            message: "plan_settlement is not supported by this kernel".into(),
        })
    }

    /// Execute the deterministic settlement plan for one strand.
    ///
    /// The default implementation reports that settlement execution is not
    /// supported by this kernel implementation.
    fn settle_strand(&mut self, _request: SettlementRequest) -> Result<SettlementResult, AbiError> {
        Err(AbiError {
            code: error_codes::NOT_SUPPORTED,
            message: "settle_strand is not supported by this kernel".into(),
        })
    }

    /// Return registry and handshake metadata.
    fn registry_info(&self) -> RegistryInfo;

    /// Return read-only scheduler status metadata.
    ///
    /// This call is side-effect free. Implementations may report a run that has
    /// already completed by the time the host polls here; for example, a
    /// synchronous `Start` can return from `dispatch_intent(...)` with
    /// `state = Inactive` and `last_run_completion` populated immediately.
    fn scheduler_status(&self) -> Result<SchedulerStatus, AbiError>;
}
