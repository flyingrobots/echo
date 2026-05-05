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
//! The current ABI version is [`ABI_VERSION`] (9). All response types are
//! CBOR-encoded using the canonical rules defined in `docs/spec/js-cbor-mapping.md`.
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

use alloc::boxed::Box;
use alloc::format;
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
pub const ABI_VERSION: u32 = 9;

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

opaque_id!(
    /// Opaque stable identifier for an Echo optic descriptor.
    OpticId
);

opaque_id!(
    /// Opaque stable identifier for a generic braid.
    BraidId
);

opaque_id!(
    /// Opaque stable identifier for a retained reading key.
    RetainedReadingKey
);

opaque_id!(
    /// Opaque stable identifier for the encoding used by a retained reading payload.
    RetainedReadingCodecId
);

opaque_id!(
    /// Opaque stable identifier for an intent family allowed through an optic.
    IntentFamilyId
);

opaque_id!(
    /// Opaque stable identifier for an admission law used by optic dispatch.
    AdmissionLawId
);

opaque_id!(
    /// Opaque stable identifier for an optic capability basis.
    OpticCapabilityId
);

opaque_id!(
    /// Opaque stable identifier for an actor opening or using an optic.
    OpticActorId
);

opaque_id!(
    /// Opaque stable identifier for an authored or kernel observer plan.
    ObserverPlanId
);

opaque_id!(
    /// Opaque stable identifier for a hosted observer instance.
    ObserverInstanceId
);

opaque_id!(
    /// Opaque stable identifier for a WARP instance.
    WarpId
);

opaque_id!(
    /// Opaque stable identifier for a node within a WARP instance.
    NodeId
);

opaque_id!(
    /// Opaque stable identifier for an edge within a WARP instance.
    EdgeId
);

opaque_id!(
    /// Opaque stable identifier for a materialization channel.
    ChannelId
);

/// Version of the projection law used by an optic read.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct ProjectionVersion(pub u32);

/// Version of the reducer law used by an optic read, when a reducer is present.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct ReducerVersion(pub u32);

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
    /// The requested observer plan is not available in this kernel.
    pub const UNSUPPORTED_OBSERVER_PLAN: u32 = 15;
    /// The requested observer instance is not available in this kernel.
    pub const UNSUPPORTED_OBSERVER_INSTANCE: u32 = 16;
    /// The requested observation rights posture is not available in this kernel.
    pub const UNSUPPORTED_OBSERVATION_RIGHTS: u32 = 17;
    /// The requested observation exceeded its explicit read budget.
    pub const OBSERVATION_BUDGET_EXCEEDED: u32 = 18;
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

/// Attachment plane selector for optic boundary reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentPlane {
    /// Vertex/node attachment plane.
    Alpha,
    /// Edge attachment plane.
    Beta,
}

/// Attachment owner reference for optic boundary reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AttachmentOwnerRef {
    /// Node-owned attachment.
    Node {
        /// WARP instance containing the node.
        warp_id: WarpId,
        /// Node identity within that WARP instance.
        node_id: NodeId,
    },
    /// Edge-owned attachment.
    Edge {
        /// WARP instance containing the edge.
        warp_id: WarpId,
        /// Edge identity within that WARP instance.
        edge_id: EdgeId,
    },
}

/// First-class reference to an attachment boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentKey {
    /// Owner of the attachment slot.
    pub owner: AttachmentOwnerRef,
    /// Attachment plane selector.
    pub plane: AttachmentPlane,
}

/// Lawful subject named by an optic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpticFocus {
    /// A whole worldline.
    Worldline {
        /// Target worldline.
        worldline_id: WorldlineId,
    },
    /// A live or retained strand.
    Strand {
        /// Target strand.
        strand_id: StrandId,
    },
    /// A generic braid projection.
    Braid {
        /// Target braid.
        braid_id: BraidId,
    },
    /// A previously retained reading.
    RetainedReading {
        /// Retained reading key.
        key: RetainedReadingKey,
    },
    /// An explicit attachment boundary.
    AttachmentBoundary {
        /// Attachment boundary key.
        key: AttachmentKey,
    },
}

/// Coordinate selector used by generic optics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CoordinateAt {
    /// Current frontier at observation or dispatch time.
    Frontier,
    /// Specific committed tick.
    Tick {
        /// Per-worldline append identity.
        worldline_tick: WorldlineTick,
    },
    /// Full provenance coordinate.
    Provenance {
        /// Provenance coordinate reference.
        reference: ProvenanceRef,
    },
}

/// Causal coordinate named by an optic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EchoCoordinate {
    /// Coordinate on a worldline.
    Worldline {
        /// Target worldline.
        worldline_id: WorldlineId,
        /// Requested position.
        at: CoordinateAt,
    },
    /// Coordinate on a strand.
    Strand {
        /// Target strand.
        strand_id: StrandId,
        /// Requested position.
        at: CoordinateAt,
        /// Optional parent basis that makes the strand read honest.
        parent_basis: Option<ProvenanceRef>,
    },
    /// Coordinate on a braid projection.
    Braid {
        /// Target braid.
        braid_id: BraidId,
        /// Projection digest at the named member frontier.
        projection_digest: Vec<u8>,
        /// Number of members included in the projection.
        member_count: u64,
    },
    /// Coordinate of a retained reading.
    RetainedReading {
        /// Retained reading key.
        key: RetainedReadingKey,
    },
}

/// Attachment recursion policy for an optic aperture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentDescentPolicy {
    /// Stop at the attachment boundary and expose only the boundary reference.
    BoundaryOnly,
    /// Recursive descent was explicitly requested and remains budget/capability checked.
    Explicit,
}

/// Budget bound for an optic read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OpticReadBudget {
    /// Maximum payload bytes to produce.
    pub max_bytes: Option<u64>,
    /// Maximum graph nodes or entities to visit.
    pub max_nodes: Option<u64>,
    /// Maximum causal ticks to reduce.
    pub max_ticks: Option<u64>,
    /// Maximum attachment boundaries to descend through.
    pub max_attachments: Option<u64>,
}

/// Bounded aperture shape selected by an optic read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpticApertureShape {
    /// Head/frontier metadata only.
    Head,
    /// Snapshot metadata only.
    SnapshotMetadata,
    /// Recorded truth channels.
    TruthChannels {
        /// Optional channel filter. `None` means all recorded channels within budget.
        channels: Option<Vec<ChannelId>>,
    },
    /// Contract query bytes identified by query id and vars digest.
    QueryBytes {
        /// Stable query identifier.
        query_id: u32,
        /// Hash of canonical query variables.
        vars_digest: Vec<u8>,
    },
    /// Bounded byte range aperture.
    ByteRange {
        /// Start byte offset.
        start: u64,
        /// Maximum byte length to return.
        len: u64,
    },
    /// Explicit attachment boundary.
    AttachmentBoundary,
}

/// Complete aperture for one optic read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpticAperture {
    /// Shape of the read aperture.
    pub shape: OpticApertureShape,
    /// Read budget.
    pub budget: OpticReadBudget,
    /// Attachment recursion policy.
    pub attachment_descent: AttachmentDescentPolicy,
}

/// Opened optic descriptor. This is not a mutable handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EchoOptic {
    /// Stable optic identity derived by the core host.
    pub optic_id: OpticId,
    /// Lawful subject being observed or targeted by intent dispatch.
    pub focus: OpticFocus,
    /// Explicit causal coordinate.
    pub coordinate: EchoCoordinate,
    /// Projection law version.
    pub projection_version: ProjectionVersion,
    /// Reducer law version, if a reducer participates.
    pub reducer_version: Option<ReducerVersion>,
    /// Intent family allowed through this optic.
    pub intent_family: IntentFamilyId,
    /// Capability basis under which the optic was opened.
    pub capability: OpticCapabilityId,
}

/// Reason an optic read identity cannot name a complete witness basis yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingWitnessBasisReason {
    /// Required witness evidence is unavailable.
    EvidenceUnavailable,
    /// The requested read exceeded its declared budget.
    BudgetLimited,
    /// The current capability does not permit revealing the basis.
    RightsLimited,
    /// The requested basis posture is not supported by this projection law.
    UnsupportedBasis,
}

/// Witness basis named by a read identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WitnessBasis {
    /// One resolved provenance commit witnesses the reading.
    ResolvedCommit {
        /// Provenance coordinate that witnesses the reading.
        reference: ProvenanceRef,
        /// State root at the witness coordinate.
        state_root: Vec<u8>,
        /// Commit hash at the witness coordinate.
        commit_hash: Vec<u8>,
    },
    /// A checkpoint plus explicit live-tail witness set witnesses the reading.
    CheckpointPlusTail {
        /// Checkpoint coordinate used as the cold basis.
        checkpoint_ref: ProvenanceRef,
        /// Checkpoint content hash.
        checkpoint_hash: Vec<u8>,
        /// Live-tail provenance refs reduced after the checkpoint.
        tail_witness_refs: Vec<ProvenanceRef>,
        /// Digest of the live-tail witness set.
        tail_digest: Vec<u8>,
    },
    /// A witness set whose exact semantics are named by the contained refs and digest.
    WitnessSet {
        /// Witness refs supporting the read.
        refs: Vec<ReadingWitnessRef>,
        /// Digest over the witness set.
        witness_set_hash: Vec<u8>,
    },
    /// The basis is missing; callers must treat the read as obstructed or incomplete.
    Missing {
        /// Deterministic reason the basis is missing.
        reason: MissingWitnessBasisReason,
    },
}

/// Stable identity of the question an optic read answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadIdentity {
    /// Stable hash over all identity fields.
    pub read_identity_hash: Vec<u8>,
    /// Optic being observed.
    pub optic_id: OpticId,
    /// Digest of the focus named by the read.
    pub focus_digest: Vec<u8>,
    /// Coordinate named by the read.
    pub coordinate: EchoCoordinate,
    /// Digest of the aperture named by the read.
    pub aperture_digest: Vec<u8>,
    /// Projection law version.
    pub projection_version: ProjectionVersion,
    /// Reducer law version, if present.
    pub reducer_version: Option<ReducerVersion>,
    /// Witness basis used by the read.
    pub witness_basis: WitnessBasis,
    /// Rights posture of the emitted reading.
    pub rights_posture: ReadingRightsPosture,
    /// Budget posture of the emitted reading.
    pub budget_posture: ReadingBudgetPosture,
    /// Residual posture of the emitted reading.
    pub residual_posture: ReadingResidualPosture,
}

/// Existing reading envelope plus first-class optic read identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpticReadingEnvelope {
    /// Existing observation reading envelope.
    pub reading: ReadingEnvelope,
    /// Stable read identity for the question this reading answered.
    pub read_identity: ReadIdentity,
}

/// Descriptor for a retained reading payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedReadingDescriptor {
    /// Stable key derived from semantic read identity and byte identity.
    pub key: RetainedReadingKey,
    /// Semantic identity of the question answered by the retained payload.
    pub read_identity: ReadIdentity,
    /// Content hash of the retained payload bytes.
    pub content_hash: Vec<u8>,
    /// Codec used for the retained payload bytes.
    pub codec_id: RetainedReadingCodecId,
    /// Retained payload byte length.
    pub byte_len: u64,
}

/// Bounded read request through an Echo optic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserveOpticRequest {
    /// Optic being observed.
    pub optic_id: OpticId,
    /// Focus being observed.
    pub focus: OpticFocus,
    /// Explicit causal coordinate for the read.
    pub coordinate: EchoCoordinate,
    /// Bounded aperture selected by the read.
    pub aperture: OpticAperture,
    /// Projection law version requested by the read.
    pub projection_version: ProjectionVersion,
    /// Reducer law version requested by the read, when present.
    pub reducer_version: Option<ReducerVersion>,
    /// Capability basis for the read.
    pub capability: OpticCapabilityId,
}

/// Intent payload dispatched through an optic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OpticIntentPayload {
    /// Canonical Echo intent v1 bytes.
    EintV1 {
        /// Complete EINT v1 envelope bytes.
        bytes: Vec<u8>,
    },
}

/// Write-side proposal request through an Echo optic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchOpticIntentRequest {
    /// Optic being used as the proposal boundary.
    pub optic_id: OpticId,
    /// Explicit causal basis for the proposal.
    pub base_coordinate: EchoCoordinate,
    /// Intent family being proposed.
    pub intent_family: IntentFamilyId,
    /// Focus targeted by the proposal.
    pub focus: OpticFocus,
    /// Actor/cause associated with the proposal.
    pub cause: OpticCause,
    /// Capability basis for the proposal.
    pub capability: OpticCapability,
    /// Admission law requested for the proposal.
    pub admission_law: AdmissionLawId,
    /// Intent payload carried by the proposal.
    pub payload: OpticIntentPayload,
}

/// Deterministic reason an optic read or dispatch could not lawfully proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpticObstructionKind {
    /// Required witness evidence is unavailable.
    MissingWitness,
    /// A retained reading named by the optic cannot be found or revealed.
    MissingRetainedReading,
    /// The dispatch named a base coordinate that is no longer the admitted basis.
    StaleBasis,
    /// The capability basis does not authorize the requested read or dispatch.
    CapabilityDenied,
    /// The declared read or dispatch budget was exceeded.
    BudgetExceeded,
    /// The requested aperture is not supported by this optic or projection law.
    UnsupportedAperture,
    /// The requested projection law/version is not available.
    UnsupportedProjectionLaw,
    /// The requested intent family is not available through this optic.
    UnsupportedIntentFamily,
    /// The read reached an attachment boundary and explicit descent is required.
    AttachmentDescentRequired,
    /// The requested attachment descent is not authorized.
    AttachmentDescentDenied,
    /// A live-tail read requires additional bounded reduction before it is honest.
    LiveTailRequiresReduction,
    /// The requested coordinate names an incompatible frontier.
    ConflictingFrontier,
    /// The request would collapse plurality without an explicit policy.
    PluralityRequiresExplicitPolicy,
}

/// Typed obstruction returned instead of a hidden fallback or fake success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpticObstruction {
    /// Deterministic obstruction kind.
    pub kind: OpticObstructionKind,
    /// Optic implicated by the obstruction, when known.
    pub optic_id: Option<OpticId>,
    /// Focus implicated by the obstruction, when known.
    pub focus: Option<OpticFocus>,
    /// Coordinate implicated by the obstruction, when known.
    pub coordinate: Option<EchoCoordinate>,
    /// Witness basis posture that explains evidence availability, when known.
    pub witness_basis: Option<WitnessBasis>,
    /// Human-readable diagnostic text.
    pub message: String,
}

/// Admission result for an optic intent that Echo accepted into witnessed history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmittedIntent {
    /// Optic through which the intent was dispatched.
    pub optic_id: OpticId,
    /// Explicit causal basis named by the dispatch.
    pub base_coordinate: EchoCoordinate,
    /// Intent family admitted through the optic.
    pub intent_family: IntentFamilyId,
    /// Provenance coordinate produced or identified by admission.
    pub admitted_ref: ProvenanceRef,
    /// Receipt digest witnessing the admission outcome.
    pub receipt_hash: Vec<u8>,
}

/// Reason an optic intent is staged instead of admitted immediately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagedIntentReason {
    /// The proposal needs an explicit rebase before admission can proceed.
    RebaseRequired,
    /// The proposal is waiting for additional capability evidence.
    AwaitingCapability,
    /// The proposal is waiting for additional witness evidence.
    AwaitingWitness,
    /// The proposal was deliberately staged for later explicit admission.
    AwaitingExplicitAdmission,
}

/// Admission result for an optic intent retained without mutating the frontier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedIntent {
    /// Optic through which the intent was dispatched.
    pub optic_id: OpticId,
    /// Explicit causal basis named by the dispatch.
    pub base_coordinate: EchoCoordinate,
    /// Intent family proposed through the optic.
    pub intent_family: IntentFamilyId,
    /// Stable digest or storage key for the staged proposal.
    pub stage_ref: Vec<u8>,
    /// Deterministic reason the proposal is staged.
    pub reason: StagedIntentReason,
}

/// Admission result that preserves lawful plurality instead of selecting one winner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluralIntent {
    /// Optic through which the intent was dispatched.
    pub optic_id: OpticId,
    /// Explicit causal basis named by the dispatch.
    pub base_coordinate: EchoCoordinate,
    /// Intent family proposed through the optic.
    pub intent_family: IntentFamilyId,
    /// Candidate coordinates that remain lawful plural outcomes.
    pub candidate_refs: Vec<ProvenanceRef>,
    /// Residual posture associated with the preserved plurality.
    pub residual_posture: ReadingResidualPosture,
}

/// Deterministic conflict reason for an optic intent dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentConflictReason {
    /// The named base coordinate is no longer the applicable basis.
    StaleBasis,
    /// The request conflicts with the named or observed frontier.
    ConflictingFrontier,
    /// Capability evidence conflicts with the requested operation.
    CapabilityConflict,
    /// The verified footprint conflicts with concurrent causal claims.
    FootprintConflict,
    /// The requested admission law conflicts with the available host law.
    AdmissionLawConflict,
    /// The request needs an explicit plurality policy before admission.
    UnsupportedPluralityPolicy,
}

/// Admission result for incompatible causal claims under an optic dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentConflict {
    /// Optic through which the intent was dispatched.
    pub optic_id: OpticId,
    /// Explicit causal basis named by the dispatch.
    pub base_coordinate: EchoCoordinate,
    /// Intent family proposed through the optic.
    pub intent_family: IntentFamilyId,
    /// Deterministic conflict reason.
    pub reason: IntentConflictReason,
    /// Provenance coordinate implicated by the conflict, when known.
    pub conflict_ref: Option<ProvenanceRef>,
    /// Digest of compact conflict evidence.
    pub evidence_digest: Vec<u8>,
    /// Human-readable diagnostic text.
    pub message: String,
}

/// Typed top-level result for dispatching an intent through an optic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "outcome", rename_all = "snake_case")]
pub enum IntentDispatchResult {
    /// Echo accepted the intent into witnessed history.
    Admitted(AdmittedIntent),
    /// Echo retained the proposal without mutating the named frontier.
    Staged(StagedIntent),
    /// Echo preserved lawful plurality instead of selecting a single result.
    Plural(PluralIntent),
    /// Echo found incompatible causal claims under the named admission law.
    Conflict(IntentConflict),
    /// Echo could not lawfully proceed because basis, evidence, rights, or law is missing.
    Obstructed(OpticObstruction),
}

/// Auditable cause for opening, closing, observing, or dispatching through an optic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpticCause {
    /// Actor associated with the cause.
    pub actor: OpticActorId,
    /// Stable digest of the host-level cause or request.
    pub cause_hash: Vec<u8>,
    /// Optional diagnostic label for humans.
    pub label: Option<String>,
}

/// Capability grant used while validating an optic descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpticCapability {
    /// Stable capability identity retained in opened optic descriptors.
    pub capability_id: OpticCapabilityId,
    /// Actor to which the capability was issued.
    pub actor: OpticActorId,
    /// Provenance ref for the issuer or policy source, when available.
    pub issuer_ref: Option<ProvenanceRef>,
    /// Stable digest of the capability policy.
    pub policy_hash: Vec<u8>,
    /// Focus this minimal capability authorizes.
    pub allowed_focus: OpticFocus,
    /// Projection law version this capability authorizes.
    pub projection_version: ProjectionVersion,
    /// Reducer law version this capability authorizes, when required.
    pub reducer_version: Option<ReducerVersion>,
    /// Intent family this capability authorizes.
    pub allowed_intent_family: IntentFamilyId,
    /// Maximum read budget authorized by this capability.
    pub max_budget: OpticReadBudget,
}

/// Capability posture returned after successfully validating an optic descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CapabilityPosture {
    /// The descriptor is authorized by the named capability grant.
    Granted {
        /// Capability identity retained in the opened descriptor.
        capability_id: OpticCapabilityId,
        /// Actor to which the capability was issued.
        actor: OpticActorId,
        /// Provenance ref for the issuer or policy source, when available.
        issuer_ref: Option<ProvenanceRef>,
        /// Stable digest of the capability policy.
        policy_hash: Vec<u8>,
    },
}

/// Descriptor-validation request for opening a session-local optic resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenOpticRequest {
    /// Lawful subject being observed or targeted by intent dispatch.
    pub focus: OpticFocus,
    /// Explicit causal coordinate for the optic descriptor.
    pub coordinate: EchoCoordinate,
    /// Projection law version requested by the descriptor.
    pub projection_version: ProjectionVersion,
    /// Reducer law version requested by the descriptor, when present.
    pub reducer_version: Option<ReducerVersion>,
    /// Intent family allowed through the opened optic.
    pub intent_family: IntentFamilyId,
    /// Capability grant used to validate this descriptor.
    pub capability: OpticCapability,
    /// Auditable cause for opening the descriptor.
    pub cause: OpticCause,
}

/// Successful descriptor-validation result for opening an optic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenOpticResult {
    /// Opened optic descriptor. This is not a mutable subject handle.
    pub optic: EchoOptic,
    /// Capability posture that authorized the descriptor.
    pub capability_posture: CapabilityPosture,
}

/// Error returned while opening an optic descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "obstruction", rename_all = "snake_case")]
pub enum OpticOpenError {
    /// Opening failed with a typed obstruction.
    Obstructed(OpticObstruction),
}

/// Request for releasing a session-local optic descriptor resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseOpticRequest {
    /// Optic descriptor to release from the session.
    pub optic_id: OpticId,
    /// Auditable cause for closing the descriptor.
    pub cause: OpticCause,
}

/// Result for releasing a session-local optic descriptor resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseOpticResult {
    /// Optic descriptor released from the session.
    pub optic_id: OpticId,
}

/// Error returned while closing an optic descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "obstruction", rename_all = "snake_case")]
pub enum OpticCloseError {
    /// Closing failed with a typed obstruction.
    Obstructed(OpticObstruction),
}

/// Successful bounded reading returned through an optic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpticReading {
    /// Reading-envelope metadata.
    pub envelope: ReadingEnvelope,
    /// Stable read identity for the question this reading answered.
    pub read_identity: ReadIdentity,
    /// Observation payload emitted by the observer.
    pub payload: ObservationPayload,
    /// Retained reading key, when the payload was retained.
    pub retained: Option<RetainedReadingKey>,
}

/// Result of observing an optic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ObserveOpticResult {
    /// The optic emitted a bounded reading.
    Reading(Box<OpticReading>),
    /// The optic could not lawfully emit a reading.
    Obstructed(Box<OpticObstruction>),
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
    /// Observer plan the caller is explicitly invoking.
    pub observer_plan: ReadingObserverPlan,
    /// Hosted observer instance state, when this is not a one-shot read.
    pub observer_instance: Option<ObserverInstanceRef>,
    /// Declared read budget.
    pub budget: ObservationReadBudget,
    /// Declared rights posture for the read.
    pub rights: ObservationRights,
}

impl ObservationRequest {
    /// Builds a one-shot built-in observation request for the frame/projection pair.
    #[must_use]
    pub fn builtin_one_shot(
        coordinate: ObservationCoordinate,
        frame: ObservationFrame,
        projection: ObservationProjection,
    ) -> Self {
        let observer_plan = ReadingObserverPlan::Builtin {
            plan: builtin_observer_plan_for(&frame, &projection),
        };
        Self {
            coordinate,
            frame,
            projection,
            observer_plan,
            observer_instance: None,
            budget: ObservationReadBudget::UnboundedOneShot,
            rights: ObservationRights::KernelPublic,
        }
    }
}

fn builtin_observer_plan_for(
    frame: &ObservationFrame,
    projection: &ObservationProjection,
) -> BuiltinObserverPlan {
    match (frame, projection) {
        (&ObservationFrame::CommitBoundary, ObservationProjection::Head) => {
            BuiltinObserverPlan::CommitBoundaryHead
        }
        (&ObservationFrame::CommitBoundary, ObservationProjection::Snapshot) => {
            BuiltinObserverPlan::CommitBoundarySnapshot
        }
        (&ObservationFrame::RecordedTruth, ObservationProjection::TruthChannels { .. }) => {
            BuiltinObserverPlan::RecordedTruthChannels
        }
        (&ObservationFrame::QueryView, ObservationProjection::Query { .. }) => {
            BuiltinObserverPlan::QueryBytes
        }
        _ => BuiltinObserverPlan::QueryBytes,
    }
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

/// Read-side basis posture carried by every observation artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservationBasisPosture {
    /// Ordinary worldline read with no live-strand basis relation.
    Worldline,
    /// Historical coordinate on a live strand's child worldline.
    StrandHistorical {
        /// Live strand whose child worldline was read.
        strand_id: StrandId,
    },
    /// Live strand frontier read while parent remains at the fork anchor.
    StrandAtAnchor {
        /// Live strand whose child worldline was read.
        strand_id: StrandId,
    },
    /// Live strand frontier read after parent movement outside the owned footprint.
    StrandParentAdvancedDisjoint {
        /// Live strand whose child worldline was read.
        strand_id: StrandId,
        /// Anchor coordinate from which the strand diverged.
        parent_from: ProvenanceRef,
        /// Current parent basis used for the read.
        parent_to: ProvenanceRef,
    },
    /// Live strand frontier read after parent movement inside the owned footprint.
    StrandRevalidationRequired {
        /// Live strand whose child worldline was read.
        strand_id: StrandId,
        /// Anchor coordinate from which the strand diverged.
        parent_from: ProvenanceRef,
        /// Current parent basis that must be revalidated.
        parent_to: ProvenanceRef,
        /// Number of overlapping slots in the core artifact.
        overlapping_slot_count: u64,
        /// Deterministic digest of the core overlapping slot list.
        overlapping_slots_digest: Vec<u8>,
    },
}

/// Built-in observer plans provided by the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinObserverPlan {
    /// Commit-boundary head metadata reading.
    CommitBoundaryHead,
    /// Commit-boundary snapshot metadata reading.
    CommitBoundarySnapshot,
    /// Recorded-truth channel payload reading.
    RecordedTruthChannels,
    /// Query-byte reading placeholder.
    QueryBytes,
}

/// Authored observer plan identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoredObserverPlan {
    /// Stable plan identity.
    pub plan_id: ObserverPlanId,
    /// Hash of the generated or installed observer artifact.
    pub artifact_hash: Vec<u8>,
    /// Hash of the authored schema or contract family.
    pub schema_hash: Vec<u8>,
    /// Hash of the observer state schema.
    pub state_schema_hash: Vec<u8>,
    /// Hash of the observer update law.
    pub update_law_hash: Vec<u8>,
    /// Hash of the observer emission law.
    pub emission_law_hash: Vec<u8>,
}

/// Observer plan identity for a reading artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReadingObserverPlan {
    /// Kernel-provided observer plan.
    Builtin {
        /// Built-in plan selected by the observation frame/projection pair.
        plan: BuiltinObserverPlan,
    },
    /// Authored/generated observer plan.
    Authored {
        /// Authored plan identity and law hashes.
        plan: Box<AuthoredObserverPlan>,
    },
}

/// Hosted observer instance identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverInstanceRef {
    /// Runtime instance identity.
    pub instance_id: ObserverInstanceId,
    /// Plan that owns this instance.
    pub plan_id: ObserverPlanId,
    /// Hash of the accumulated observer state.
    pub state_hash: Vec<u8>,
}

/// Native observer basis used by the emitted reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingObserverBasis {
    /// Commit-boundary observer basis.
    CommitBoundary,
    /// Recorded-truth observer basis.
    RecordedTruth,
    /// Query-view observer basis.
    QueryView,
}

/// Read budget requested by an observation caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservationReadBudget {
    /// One-shot built-in observer with no caller-specified slice budget.
    UnboundedOneShot,
    /// Caller-bounded read budget.
    Bounded {
        /// Maximum encoded payload bytes the caller is willing to receive.
        max_payload_bytes: u64,
        /// Maximum witness references the caller is willing to accept.
        max_witness_refs: u64,
    },
}

/// Rights posture requested by an observation caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservationRights {
    /// Kernel-public read.
    KernelPublic,
    /// Capability-scoped read. Echo carries this now but does not execute it
    /// until a capability checker is installed for the observer family.
    CapabilityScoped {
        /// Capability basis named by the caller.
        capability: OpticCapabilityId,
    },
}

/// Witness reference carried by a reading artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReadingWitnessRef {
    /// The reading is witnessed by a retained provenance commit.
    ResolvedCommit {
        /// Provenance coordinate that witnesses the reading.
        reference: ProvenanceRef,
    },
    /// The reading is the deterministic empty frontier before any commit exists.
    EmptyFrontier {
        /// Worldline observed at its empty frontier.
        worldline_id: WorldlineId,
        /// Deterministic empty-frontier state root.
        state_root: Vec<u8>,
        /// Deterministic empty-frontier commit/frontier hash.
        commit_hash: Vec<u8>,
    },
}

/// Budget posture for a reading artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingBudgetPosture {
    /// One-shot built-in observer with no caller-specified slice budget.
    UnboundedOneShot,
    /// Caller-bounded reading that remained within budget.
    Bounded {
        /// Requested encoded payload byte limit.
        max_payload_bytes: u64,
        /// Encoded payload bytes emitted.
        payload_bytes: u64,
        /// Requested witness-reference limit.
        max_witness_refs: u64,
        /// Witness references emitted.
        witness_refs: u64,
    },
}

/// Rights posture for a reading artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingRightsPosture {
    /// Kernel-public reading with no app-specific authorization layer.
    KernelPublic,
}

/// Residual posture for a reading artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingResidualPosture {
    /// The observer emitted a clean, complete reading for the requested projection.
    Complete,
    /// The observer emitted a bounded reading with explicit residual outside the payload.
    Residual,
    /// The observer preserved lawful plurality instead of collapsing to one reading.
    PluralityPreserved,
    /// The observer surfaced a lawful obstruction instead of a derived reading.
    Obstructed,
}

/// Reading-envelope metadata carried by every observation artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadingEnvelope {
    /// Observer plan identity.
    pub observer_plan: ReadingObserverPlan,
    /// Hosted observer instance, when the reading used accumulated observer state.
    pub observer_instance: Option<ObserverInstanceRef>,
    /// Native observer basis used by the reading.
    pub observer_basis: ReadingObserverBasis,
    /// Witnesses or shell references that support the reading.
    pub witness_refs: Vec<ReadingWitnessRef>,
    /// Read-side parent/strand basis posture.
    pub parent_basis_posture: ObservationBasisPosture,
    /// Budget posture for the reading.
    pub budget_posture: ReadingBudgetPosture,
    /// Rights or revelation posture for the reading.
    pub rights_posture: ReadingRightsPosture,
    /// Residual, obstruction, or plurality posture for the reading.
    pub residual_posture: ReadingResidualPosture,
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
/// the resolved coordinate, reading envelope, frame, projection, and canonical
/// payload bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationHashInput {
    /// Resolved coordinate metadata.
    pub resolved: ResolvedObservationCoordinate,
    /// Reading-envelope metadata.
    pub reading: ReadingEnvelope,
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
    /// Reading-envelope metadata.
    pub reading: ReadingEnvelope,
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
    /// The parent advanced into the strand-owned closed footprint.
    ParentFootprintOverlap,
    /// The source and target lanes disagree on time-quantum assumptions.
    QuantumMismatch,
}

/// Parent-basis posture used while comparing or planning settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SettlementParentRevalidation {
    /// The parent remains at the strand's anchor coordinate.
    AtAnchor,
    /// The parent advanced outside the strand-owned closed footprint.
    ParentAdvancedDisjoint {
        /// Anchor coordinate from which the strand diverged.
        parent_from: ProvenanceRef,
        /// Current parent coordinate used as the settlement basis.
        parent_to: ProvenanceRef,
    },
    /// The parent advanced into the strand-owned closed footprint.
    RevalidationRequired {
        /// Anchor coordinate from which the strand diverged.
        parent_from: ProvenanceRef,
        /// Current parent coordinate that requires overlap revalidation.
        parent_to: ProvenanceRef,
        /// Number of overlapping slots in the core artifact.
        overlapping_slot_count: u64,
        /// Deterministic digest of the core overlapping slot list.
        overlapping_slots_digest: Vec<u8>,
    },
}

/// Compact basis-relative settlement evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementBasisReport {
    /// Recorded parent anchor for the strand.
    pub parent_anchor: BaseRef,
    /// Child worldline carrying speculative suffix history.
    pub child_worldline_id: WorldlineId,
    /// First suffix tick eligible for settlement consideration.
    pub source_suffix_start_tick: WorldlineTick,
    /// Last suffix tick currently present on the source worldline, if any.
    pub source_suffix_end_tick: Option<WorldlineTick>,
    /// Current parent basis used while producing this report.
    pub realized_parent_ref: ProvenanceRef,
    /// Number of unique slots in the strand-owned closed footprint.
    pub owned_closed_slot_count: u64,
    /// Number of unique parent-written slots after the strand anchor.
    pub parent_written_slot_count: u64,
    /// Parent movement posture relative to the strand-owned footprint.
    pub parent_revalidation: SettlementParentRevalidation,
}

/// Outcome of explicitly revalidating an overlapped settlement patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SettlementOverlapRevalidation {
    /// Replaying the source patch left the overlapped parent slots unchanged.
    Clean {
        /// Number of overlapping slots checked by the core artifact.
        overlapping_slot_count: u64,
        /// Deterministic digest of the core overlapping slot list.
        overlapping_slots_digest: Vec<u8>,
    },
    /// Replaying the source patch failed on the current parent basis.
    Obstructed {
        /// Number of overlapping slots checked by the core artifact.
        overlapping_slot_count: u64,
        /// Deterministic digest of the core overlapping slot list.
        overlapping_slots_digest: Vec<u8>,
    },
    /// Replaying the source patch would mutate overlapped parent slots.
    Conflict {
        /// Number of overlapping slots checked by the core artifact.
        overlapping_slot_count: u64,
        /// Deterministic digest of the core overlapping slot list.
        overlapping_slots_digest: Vec<u8>,
    },
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
    /// Compact basis-relative evidence used for this comparison.
    pub basis_report: SettlementBasisReport,
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
    /// Explicit overlap revalidation evidence when this import crossed overlap.
    pub overlap_revalidation: Option<SettlementOverlapRevalidation>,
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
    /// Explicit overlap revalidation evidence when overlap caused this residue.
    pub overlap_revalidation: Option<SettlementOverlapRevalidation>,
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
    /// Compact basis-relative evidence used while producing this plan.
    pub basis_report: SettlementBasisReport,
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

/// Compact shell for judging a witnessed suffix without transport or sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessedSuffixShell {
    /// Worldline carrying the proposed suffix.
    pub source_worldline_id: WorldlineId,
    /// First source tick included in the proposed suffix.
    pub source_suffix_start_tick: WorldlineTick,
    /// Last source tick included in the proposed suffix, if any.
    pub source_suffix_end_tick: Option<WorldlineTick>,
    /// Ordered source provenance coordinates covered by the shell.
    pub source_entries: Vec<ProvenanceRef>,
    /// Boundary witness used when the shell has no importable entries yet.
    pub boundary_witness: Option<ProvenanceRef>,
    /// Deterministic digest identifying the compact shell evidence.
    pub witness_digest: Vec<u8>,
    /// Optional basis-relative settlement evidence reused by the shell.
    pub basis_report: Option<SettlementBasisReport>,
}

/// Request to judge a witnessed suffix against a target basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessedSuffixAdmissionRequest {
    /// Source suffix and compact witness material.
    pub source_suffix: WitnessedSuffixShell,
    /// Worldline receiving the proposed admission.
    pub target_worldline_id: WorldlineId,
    /// Target basis used while judging admission.
    pub target_basis: ProvenanceRef,
    /// Optional target-basis evidence for strand/parent realization cases.
    pub basis_report: Option<SettlementBasisReport>,
}

/// Response to one witnessed suffix admission request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessedSuffixAdmissionResponse {
    /// Deterministic digest of the source shell being judged.
    pub source_shell_digest: Vec<u8>,
    /// Resolved target basis used for the response.
    pub target_basis: ProvenanceRef,
    /// Exactly one top-level admission outcome.
    pub outcome: WitnessedSuffixAdmissionOutcome,
}

/// Request to export a witnessed causal suffix rooted at a known source frontier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportSuffixRequest {
    /// Source worldline carrying the suffix.
    pub source_worldline_id: WorldlineId,
    /// Known source basis before the suffix begins.
    pub base_frontier: ProvenanceRef,
    /// Optional requested source frontier to export through.
    pub target_frontier: Option<ProvenanceRef>,
    /// Optional basis-relative settlement evidence reused by the exported shell.
    pub basis_report: Option<SettlementBasisReport>,
}

/// Witnessed suffix bundle exchanged across a hot/cold runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CausalSuffixBundle {
    /// Known source basis before the suffix begins.
    pub base_frontier: ProvenanceRef,
    /// Source frontier reached by this exported suffix shell.
    pub target_frontier: ProvenanceRef,
    /// Compact source suffix and its witness digest.
    pub source_suffix: WitnessedSuffixShell,
    /// Deterministic digest of the bundle identity.
    pub bundle_digest: Vec<u8>,
}

/// Obstruction returned when Echo cannot produce a witnessed suffix bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportSuffixObstruction {
    /// Source coordinate implicated in the obstruction.
    pub source_ref: ProvenanceRef,
    /// Read-side residual posture associated with the obstruction.
    pub residual_posture: ReadingResidualPosture,
    /// Deterministic digest of compact obstruction evidence.
    pub evidence_digest: Vec<u8>,
}

/// Request to import one witnessed causal suffix bundle by classifying it
/// against a target basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportSuffixRequest {
    /// Source bundle being judged.
    pub bundle: CausalSuffixBundle,
    /// Worldline receiving the proposed admission.
    pub target_worldline_id: WorldlineId,
    /// Target basis used while judging admission.
    pub target_basis: ProvenanceRef,
    /// Optional target-basis evidence for strand/parent realization cases.
    pub basis_report: Option<SettlementBasisReport>,
}

/// Result of importing one witnessed causal suffix bundle into local admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportSuffixResult {
    /// Bundle identity retained for shell-equivalence and loop-prevention checks.
    pub bundle_digest: Vec<u8>,
    /// Admission classifier response for the bundle's source suffix.
    pub admission: WitnessedSuffixAdmissionResponse,
}

/// Top-level witnessed suffix admission posture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WitnessedSuffixAdmissionOutcome {
    /// The suffix is admissible on the named target basis.
    Admitted {
        /// Target worldline receiving the admissible suffix.
        target_worldline_id: WorldlineId,
        /// Target-local provenance coordinates produced or expected by admission.
        admitted_refs: Vec<ProvenanceRef>,
        /// Basis evidence used to classify the suffix as admitted.
        basis_report: Option<SettlementBasisReport>,
    },
    /// The suffix is well-formed but retained for later judgment.
    Staged {
        /// Source or target coordinates retained while staged.
        staged_refs: Vec<ProvenanceRef>,
        /// Basis evidence used to classify the suffix as staged.
        basis_report: Option<SettlementBasisReport>,
    },
    /// The suffix preserves lawful plurality instead of one admitted result.
    Plural {
        /// Candidate coordinates that remain lawful plural outcomes.
        candidate_refs: Vec<ProvenanceRef>,
        /// Read-side residual posture associated with preserved plurality.
        residual_posture: ReadingResidualPosture,
        /// Basis evidence used to classify the suffix as plural.
        basis_report: Option<SettlementBasisReport>,
    },
    /// The suffix conflicts with the target basis under current admission law.
    Conflict {
        /// Deterministic reason the suffix conflicts.
        reason: ConflictReason,
        /// Source coordinate implicated in the conflict.
        source_ref: ProvenanceRef,
        /// Deterministic digest of compact conflict evidence.
        evidence_digest: Vec<u8>,
        /// Optional overlap revalidation evidence when footprint overlap caused the conflict.
        overlap_revalidation: Option<SettlementOverlapRevalidation>,
    },
    /// The suffix cannot currently be judged or admitted.
    Obstructed {
        /// Source coordinate implicated in the obstruction.
        source_ref: ProvenanceRef,
        /// Read-side residual posture associated with the obstruction.
        residual_posture: ReadingResidualPosture,
        /// Deterministic digest of compact obstruction evidence.
        evidence_digest: Vec<u8>,
    },
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

fn optic_focus_matches_coordinate(focus: &OpticFocus, coordinate: &EchoCoordinate) -> bool {
    match (focus, coordinate) {
        (
            OpticFocus::Worldline { worldline_id },
            EchoCoordinate::Worldline {
                worldline_id: coordinate_worldline,
                ..
            },
        ) => worldline_id == coordinate_worldline,
        (
            OpticFocus::Strand { strand_id },
            EchoCoordinate::Strand {
                strand_id: coordinate_strand,
                ..
            },
        ) => strand_id == coordinate_strand,
        (
            OpticFocus::Braid { braid_id },
            EchoCoordinate::Braid {
                braid_id: coordinate_braid,
                ..
            },
        ) => braid_id == coordinate_braid,
        (
            OpticFocus::RetainedReading { key },
            EchoCoordinate::RetainedReading {
                key: coordinate_key,
            },
        ) => key == coordinate_key,
        (OpticFocus::AttachmentBoundary { .. }, _) => true,
        _ => false,
    }
}

fn optic_dispatch_obstruction(
    request: &DispatchOpticIntentRequest,
    kind: OpticObstructionKind,
    message: impl Into<String>,
) -> IntentDispatchResult {
    IntentDispatchResult::Obstructed(OpticObstruction {
        kind,
        optic_id: Some(request.optic_id),
        focus: Some(request.focus.clone()),
        coordinate: Some(request.base_coordinate.clone()),
        witness_basis: None,
        message: message.into(),
    })
}

fn validate_optic_dispatch_request(
    request: &DispatchOpticIntentRequest,
    current_coordinate: Option<&EchoCoordinate>,
) -> Option<IntentDispatchResult> {
    if !optic_focus_matches_coordinate(&request.focus, &request.base_coordinate) {
        return Some(optic_dispatch_obstruction(
            request,
            OpticObstructionKind::ConflictingFrontier,
            "optic dispatch focus and base coordinate name different subjects",
        ));
    }

    if request.capability.actor != request.cause.actor {
        return Some(optic_dispatch_obstruction(
            request,
            OpticObstructionKind::CapabilityDenied,
            "optic dispatch capability actor does not match cause actor",
        ));
    }

    if request.capability.allowed_focus != request.focus {
        return Some(optic_dispatch_obstruction(
            request,
            OpticObstructionKind::CapabilityDenied,
            "optic dispatch capability does not authorize focus",
        ));
    }

    if request.capability.allowed_intent_family != request.intent_family {
        return Some(optic_dispatch_obstruction(
            request,
            OpticObstructionKind::UnsupportedIntentFamily,
            "optic dispatch capability does not authorize intent family",
        ));
    }

    if let Some(current_coordinate) = current_coordinate {
        if !coordinates_name_same_subject(&request.base_coordinate, current_coordinate) {
            return Some(optic_dispatch_obstruction(
                request,
                OpticObstructionKind::ConflictingFrontier,
                "optic dispatch current coordinate names a different subject",
            ));
        }

        if base_coordinate_is_stale(&request.base_coordinate, current_coordinate) {
            return Some(optic_dispatch_obstruction(
                request,
                OpticObstructionKind::StaleBasis,
                "optic dispatch base coordinate is stale relative to current frontier",
            ));
        }
    }

    None
}

fn coordinates_name_same_subject(base: &EchoCoordinate, current: &EchoCoordinate) -> bool {
    match (base, current) {
        (
            EchoCoordinate::Worldline { worldline_id, .. },
            EchoCoordinate::Worldline {
                worldline_id: current_worldline,
                ..
            },
        ) => worldline_id == current_worldline,
        (
            EchoCoordinate::Strand { strand_id, .. },
            EchoCoordinate::Strand {
                strand_id: current_strand,
                ..
            },
        ) => strand_id == current_strand,
        (
            EchoCoordinate::Braid { braid_id, .. },
            EchoCoordinate::Braid {
                braid_id: current_braid,
                ..
            },
        ) => braid_id == current_braid,
        (
            EchoCoordinate::RetainedReading { key },
            EchoCoordinate::RetainedReading { key: current_key },
        ) => key == current_key,
        _ => false,
    }
}

fn base_coordinate_is_stale(base: &EchoCoordinate, current: &EchoCoordinate) -> bool {
    match (base, current) {
        (
            EchoCoordinate::Worldline { at, .. } | EchoCoordinate::Strand { at, .. },
            EchoCoordinate::Worldline { at: current_at, .. }
            | EchoCoordinate::Strand { at: current_at, .. },
        ) => coordinate_at_tick(at).is_some_and(|base_tick| {
            coordinate_at_tick(current_at).is_some_and(|current_tick| base_tick < current_tick)
        }),
        (
            EchoCoordinate::Braid { member_count, .. },
            EchoCoordinate::Braid {
                member_count: current_member_count,
                ..
            },
        ) => member_count < current_member_count,
        _ => false,
    }
}

fn coordinate_at_tick(at: &CoordinateAt) -> Option<u64> {
    match at {
        CoordinateAt::Frontier => None,
        CoordinateAt::Tick { worldline_tick } => Some(worldline_tick.0),
        CoordinateAt::Provenance { reference } => Some(reference.worldline_tick.0),
    }
}

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

    /// Returns the current coordinate for an optic focus when the implementation
    /// can resolve it cheaply enough to validate stale bases.
    fn current_optic_coordinate(
        &self,
        _focus: &OpticFocus,
    ) -> Result<Option<EchoCoordinate>, AbiError> {
        Ok(None)
    }

    /// Propose an intent through an explicit optic dispatch request.
    ///
    /// The default implementation validates the generic optic/capability
    /// request and routes `EintV1` payloads into [`KernelPort::dispatch_intent`].
    /// Because that existing path only ingests an intent into the runtime inbox,
    /// the resulting optic outcome is `Staged`, not a fabricated admitted tick.
    fn dispatch_optic_intent(
        &mut self,
        request: DispatchOpticIntentRequest,
    ) -> Result<IntentDispatchResult, AbiError> {
        if let Some(obstruction) = validate_optic_dispatch_request(&request, None) {
            return Ok(obstruction);
        }

        let current_coordinate = self.current_optic_coordinate(&request.focus)?;
        if let Some(obstruction) =
            validate_optic_dispatch_request(&request, current_coordinate.as_ref())
        {
            return Ok(obstruction);
        }

        match &request.payload {
            OpticIntentPayload::EintV1 { bytes } => {
                if let Err(error) = crate::unpack_intent_v1(bytes) {
                    return Ok(optic_dispatch_obstruction(
                        &request,
                        OpticObstructionKind::UnsupportedIntentFamily,
                        format!("optic dispatch EINT v1 payload is malformed: {error}"),
                    ));
                }

                let dispatch = self.dispatch_intent(bytes)?;
                Ok(IntentDispatchResult::Staged(StagedIntent {
                    optic_id: request.optic_id,
                    base_coordinate: request.base_coordinate,
                    intent_family: request.intent_family,
                    stage_ref: dispatch.intent_id,
                    reason: StagedIntentReason::AwaitingExplicitAdmission,
                }))
            }
        }
    }

    /// Observe through an explicit optic request.
    ///
    /// The default implementation reports that optic reads are not supported by
    /// this kernel implementation.
    fn observe_optic(&self, _request: ObserveOpticRequest) -> Result<ObserveOpticResult, AbiError> {
        Err(AbiError {
            code: error_codes::NOT_SUPPORTED,
            message: "observe_optic is not supported by this kernel".into(),
        })
    }

    /// Observe a worldline at an explicit coordinate and frame.
    ///
    /// This is the canonical world-state read entrypoint. The
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
