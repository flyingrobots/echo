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
// Escalate workspace `deny(unsafe_code)` to `forbid` — no exceptions in the engine.
#![forbid(unsafe_code)]

#[cfg(all(feature = "footprint_enforce_release", feature = "unsafe_graph"))]
compile_error!(
    "features `footprint_enforce_release` and `unsafe_graph` are mutually exclusive: \
     unsafe_graph disables enforcement"
);

/// Deterministic fixed-point helpers (Q32.32).
pub mod fixed;
/// Deterministic math subsystem (Vec3, Mat4, Quat, PRNG).
pub mod math;
/// WSC (Write-Streaming Columnar) snapshot format for deterministic serialization.
pub mod wsc;

mod admission;
mod attachment;
mod clock;
mod cmd;
mod constants;
/// Domain separation prefixes for hashing.
pub mod domain;
mod dynamic_binding;
mod engine_impl;
mod footprint;
/// Footprint enforcement guard for parallel execution.
///
/// # Intent
///
/// Validates that execute functions stay within their declared footprints.
/// Every read and write is checked against the `Footprint` declared by the rule.
///
/// # Gating
///
/// - **Debug builds**: enforcement enabled by default (`debug_assertions`)
/// - **Release builds**: enforcement disabled unless `footprint_enforce_release` feature is enabled
/// - **`unsafe_graph` feature**: mutually exclusive with `footprint_enforce_release` at
///   compile time (enabling both is a compile error). Use `unsafe_graph` as an escape
///   hatch for benchmarks/fuzzing where safety checks are deliberately bypassed
///
/// # Invariants
///
/// - Each `ExecItem` is paired with a `FootprintGuard` aligned by index in the `WorkUnit`
/// - Reads via `GraphView::new_guarded()` are intercepted and validated inline
/// - Writes are validated post-hoc via `check_op()` after the executor completes or unwinds
///   (panics); validation runs even when the executor panics to catch violations on emitted ops
///
/// # Violation Surfacing
///
/// Violations produce panic payloads:
/// - [`FootprintViolation`]: emitted when an illegal op is detected (undeclared read/write,
///   cross-warp emission, unauthorized instance op)
/// - [`FootprintViolationWithPanic`]: wraps both a `FootprintViolation` and an executor panic
///   when both occur
///
/// Downstream effects: a violation causes the `TickDelta` to become a `PoisonedDelta`,
/// preventing merge. At the engine layer, poisoned deltas trigger `MergeError::PoisonedDelta`.
///
/// # Recommended Usage
///
/// - **Tests (debug)**: enforcement is active by default (`debug_assertions`); tests
///   should exercise both valid and intentionally-violating footprints
/// - **Tests (release)**: enforcement is disabled unless `footprint_enforce_release`
///   feature is enabled (e.g., `cargo test --release --features footprint_enforce_release`)
/// - **Production**: leave enforcement off (default) for maximum throughput, or enable
///   `footprint_enforce_release` during validation/staging
/// - **Opting out**: `unsafe_graph` feature disables enforcement unconditionally, even
///   in debug builds; use for benchmarks or fuzzing where safety checks are bypassed
pub mod footprint_guard;
mod graph;
mod graph_view;
mod ident;
/// Legacy graph-backed inbox helpers for compatibility and older tests.
///
/// New runtime-owned ingress code should prefer [`WorldlineRuntime`],
/// [`IngressEnvelope`], and [`HeadInbox`]. This module remains available for
/// legacy tests and transitional callers, but it is no longer the primary live
/// ingress path in Phase 3.
pub mod inbox;
/// Materialization subsystem for deterministic channel-based output.
pub mod materialization;
mod neighborhood;
mod observation;
mod optic;
/// Parallel execution module.
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
pub mod parallel;
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
mod settlement;
mod snapshot;
mod snapshot_accum;
mod telemetry;
mod tick_delta;
mod tick_patch;
mod tx;
mod warp_state;
mod witnessed_suffix;
#[cfg(test)]
mod witnessed_suffix_tests;
mod worldline;

// ADR-0008 runtime primitives (Phases 1–3)
mod coordinator;
mod head;
mod head_inbox;
/// Strand contract for speculative execution lanes.
pub mod strand;
mod worldline_registry;
mod worldline_state;

// Re-exports for stable public API
pub use admission::{
    AdmissionOutcome, AdmissionOutcomeKind, AdmissionPolicyRef, AffectedRegion, BoundedSite,
    PluralArtifact, ReintegrationBoundary,
};
pub use attachment::{
    AtomPayload, AttachmentKey, AttachmentOwner, AttachmentPlane, AttachmentValue, Codec,
    CodecRegistry, DecodeError, ErasedCodec, RegistryError,
};
pub use clock::{GlobalTick, RunId, WorldlineTick};
pub use cmd::{
    import_suffix_intent_rule, import_suffix_result_edge_id, import_suffix_result_node_id,
    IMPORT_SUFFIX_INTENT_RULE_NAME, IMPORT_SUFFIX_RESULT_ATTACHMENT_TYPE,
    IMPORT_SUFFIX_RESULT_EDGE_TYPE, IMPORT_SUFFIX_RESULT_NODE_TYPE,
};
pub use constants::{blake3_empty, digest_len0_u64, POLICY_ID_NO_POLICY_V0};
pub use dynamic_binding::{
    BoundNodeRef, ClosureMemberBinding, DirectSlotBinding, DynamicBindingError,
    DynamicBindingRuntimeError, RangeClosureBindingRequest, RelationSlotBinding,
    ResolvedClosureBinding, ResolvedSlotBinding, StructuredBindingResolver,
    StructuredBindingRuntime, StructuredRuntimeBindings,
};
pub use engine_impl::{
    scope_hash, ApplyResult, CommitOutcome, DispatchDisposition, Engine, EngineBuilder,
    EngineError, ExistingState, FreshStore, IngestDisposition,
};
pub use footprint::{
    pack_port_key, AttachmentSet, EdgeSet, Footprint, NodeSet, PortKey, PortSet, WarpScopedPortKey,
};
pub use footprint_guard::{FootprintViolation, FootprintViolationWithPanic, ViolationKind};
pub use graph::{DeleteNodeError, GraphStore};
pub use graph_view::GraphView;
pub use ident::{
    make_edge_id, make_node_id, make_type_id, make_warp_id, EdgeId, EdgeKey, Hash, NodeId, NodeKey,
    TypeId, WarpId,
};
pub use parallel::{
    execute_parallel, execute_parallel_sharded, execute_parallel_sharded_with_policy,
    execute_parallel_with_policy, execute_serial, shard_of, DeltaAccumulationPolicy, ExecItem,
    MergeConflict, ParallelExecutionPolicy, PoisonedDelta, ShardAssignmentPolicy, NUM_SHARDS,
};
/// Delta merging functions, only available with `delta_validate` feature.
///
/// These functions are feature-gated because they are primarily used for testing
/// and validation. `merge_deltas` accepts `Vec<Result<TickDelta, PoisonedDelta>>`
/// and performs poisoned-delta rejection; `merge_deltas_ok` is a convenience wrapper
/// that maps `Vec<TickDelta>` into `Ok` variants and delegates to `merge_deltas`.
/// Enable `delta_validate` to access them.
#[cfg(any(test, feature = "delta_validate"))]
#[cfg_attr(docsrs, doc(cfg(feature = "delta_validate")))]
pub use parallel::{merge_deltas, merge_deltas_ok, MergeError};
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
pub use neighborhood::{
    NeighborhoodCore, NeighborhoodError, NeighborhoodParticipant, NeighborhoodParticipantRole,
    NeighborhoodPlurality, NeighborhoodSite, NeighborhoodSiteId, NeighborhoodSiteService,
    ParticipantRole, SiteParticipant, SitePlurality,
};
pub use observation::{
    AuthoredObserverPlan, BuiltinObserverPlan, HeadObservation, ObservationArtifact, ObservationAt,
    ObservationBasisPosture, ObservationCoordinate, ObservationError, ObservationFrame,
    ObservationPayload, ObservationProjection, ObservationProjectionKind, ObservationReadBudget,
    ObservationRequest, ObservationRights, ObservationService, ObserverInstanceId,
    ObserverInstanceRef, ObserverPlanId, ReadingBudgetPosture, ReadingEnvelope,
    ReadingObserverBasis, ReadingObserverPlan, ReadingResidualPosture, ReadingRightsPosture,
    ReadingWitnessRef, ResolvedObservationCoordinate, WorldlineSnapshot,
};
pub use optic::{
    AdmissionLawId, AdmittedIntent, AttachmentDescentPolicy, BraidId, CapabilityPosture,
    CloseOpticRequest, CloseOpticResult, CoordinateAt, DispatchOpticIntentRequest, EchoCoordinate,
    EchoOptic, IntentConflict, IntentConflictReason, IntentDispatchResult, IntentFamilyId,
    MissingWitnessBasisReason, ObserveOpticRequest, ObserveOpticResult, OpenOpticRequest,
    OpenOpticResult, OpticActorId, OpticAperture, OpticApertureShape, OpticCapability,
    OpticCapabilityId, OpticCause, OpticCloseError, OpticFocus, OpticId, OpticIntentPayload,
    OpticObstruction, OpticObstructionKind, OpticOpenError, OpticReadBudget, OpticReading,
    OpticReadingEnvelope, PluralIntent, ProjectionVersion, ReadIdentity, ReducerVersion,
    RetainReadingRequest, RetainReadingResult, RetainedReadingCache, RetainedReadingCodecId,
    RetainedReadingDescriptor, RetainedReadingKey, RevealReadingRequest, RevealReadingResult,
    StagedIntent, StagedIntentReason, WitnessBasis, WorldlineHeadOptic,
};
pub use playback::{CursorReceipt, TruthFrame, TruthSink};
pub use provenance_store::{
    BoundaryTransitionRecord, BtrError, BtrPayload, CheckpointRef, HistoryError,
    LocalProvenanceStore, ProvenanceEntry, ProvenanceEventKind, ProvenanceRef, ProvenanceService,
    ProvenanceStore, ReplayCheckpoint, ReplayError,
};
pub use receipt::{TickReceipt, TickReceiptDisposition, TickReceiptEntry, TickReceiptRejection};
pub use record::{EdgeRecord, NodeRecord};
#[cfg(feature = "native_rule_bootstrap")]
pub use rule::{ConflictPolicy, ExecuteFn, MatchFn, PatternGraph, RewriteRule};
pub use sandbox::DeterminismError;
#[cfg(feature = "native_rule_bootstrap")]
pub use sandbox::{build_engine, run_pair_determinism, EchoConfig};
pub use scheduler::SchedulerKind;
#[cfg(feature = "serde")]
pub use serializable::{
    SerializableReceipt, SerializableReceiptEntry, SerializableSnapshot, SerializableTick,
};
pub use settlement::{
    ConflictArtifactDraft, ConflictReason, ImportCandidate, SettlementDecision, SettlementDelta,
    SettlementError, SettlementPlan, SettlementResult, SettlementService,
};
pub use snapshot::{
    compute_commit_hash_v2, compute_emissions_digest, compute_op_emission_index_digest,
    compute_state_root_for_warp_store, compute_tick_commit_hash_v2, OpEmissionEntry, Snapshot,
};
pub use strand::{
    make_strand_id, DropReceipt, ForkBasisRef, ParentMovementFootprint, Strand, StrandBasisReport,
    StrandDivergenceFootprint, StrandError, StrandId, StrandOverlapRevalidation, StrandRegistry,
    StrandRevalidationState, SupportPin,
};
pub use telemetry::{NullTelemetrySink, TelemetrySink};
pub use tick_delta::{DeltaStats, OpOrigin, ScopedDelta, TickDelta};
pub use tick_patch::{
    slice_worldline_indices, PortalInit, SlotId, TickCommitStatus, TickPatchError, WarpOp,
    WarpOpKey, WarpTickPatchV1,
};
pub use tx::TxId;
pub use warp_state::{WarpInstance, WarpState};
pub use witnessed_suffix::{
    derive_witnessed_suffix_shell_digest, evaluate_witnessed_suffix_admission, export_suffix,
    import_suffix, CausalSuffixBundle, ExportSuffixObstruction, ExportSuffixRequest,
    ImportSuffixRequest, ImportSuffixResult, WitnessedSuffixAdmissionContext,
    WitnessedSuffixAdmissionOutcome, WitnessedSuffixAdmissionRequest,
    WitnessedSuffixAdmissionResponse, WitnessedSuffixExportContext,
    WitnessedSuffixLocalAdmissionPosture, WitnessedSuffixLocalAdmissionPostureError,
    WitnessedSuffixShell,
};
pub use worldline::{
    ApplyError, AtomWrite, AtomWriteSet, HashTriplet, OutputFrameSet, WorldlineId,
    WorldlineTickHeaderV1, WorldlineTickPatchV1,
};

/// Phase 3 runtime-owned scheduler and ingress surface.
///
/// Prefer this coordinator/runtime API for new stepping and routing code.
pub use coordinator::{
    ForkStrandReceipt, ForkStrandRequest, IngressDisposition, RuntimeError, SchedulerCoordinator,
    StepRecord, WorldlineRuntime,
};
/// Writer-head registry and routing primitives used by the runtime-owned ingress path.
pub use head::{
    make_head_id, HeadEligibility, HeadId, PlaybackHeadRegistry, RunnableWriterSet, WriterHead,
    WriterHeadKey,
};
/// Primary ingress-envelope and per-head inbox types for the live runtime path.
///
/// Compatibility note: [`crate::inbox`] remains available for legacy tests and
/// transitional callers, but new code should route ingress via
/// [`WorldlineRuntime::ingest`] with these types.
pub use head_inbox::{
    make_intent_kind, HeadInbox, InboxAddress, InboxPolicy, IngressEnvelope, IngressPayload,
    IngressTarget, IntentKind,
};
pub use worldline_registry::WorldlineRegistry;
pub use worldline_state::{WorldlineFrontier, WorldlineState, WorldlineStateError};

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
