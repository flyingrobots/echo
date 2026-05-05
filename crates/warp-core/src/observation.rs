// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Explicit observation contract for worldline reads.
//!
//! Phase 5 makes observation the single canonical internal read path. Every
//! meaningful read names:
//!
//! - a worldline,
//! - a coordinate,
//! - a semantic frame,
//! - and a projection.
//!
//! Observation is strictly read-only. It never advances runtime state, drains
//! inboxes, rewrites provenance, or mutates compatibility mirrors.

use blake3::Hasher;
use echo_wasm_abi::kernel_port as abi;
use thiserror::Error;

use crate::attachment::{AttachmentOwner, AttachmentPlane};
use crate::clock::{GlobalTick, WorldlineTick};
use crate::coordinator::WorldlineRuntime;
use crate::engine_impl::Engine;
use crate::ident::Hash;
use crate::materialization::ChannelId;
use crate::optic::{
    AttachmentDescentPolicy, CoordinateAt, EchoCoordinate, MissingWitnessBasisReason,
    ObserveOpticRequest, ObserveOpticResult, OpticApertureShape, OpticCapabilityId, OpticFocus,
    OpticObstruction, OpticObstructionKind, OpticReading, ReadIdentity, WitnessBasis,
};
use crate::provenance_store::{ProvenanceRef, ProvenanceService, ProvenanceStore};
use crate::snapshot::Snapshot;
use crate::strand::{StrandId, StrandRevalidationState};
use crate::tick_patch::SlotId;
use crate::worldline::WorldlineId;

const OBSERVATION_VERSION: u32 = 2;
const OBSERVATION_ARTIFACT_DOMAIN: &[u8] = b"echo:observation-artifact:v2\0";
const OPTIC_OBSERVATION_WITNESS_SET_DOMAIN: &[u8] = b"echo:optic-observation-witness-set:v1\0";
const OPTIC_LIVE_TAIL_WITNESS_SET_DOMAIN: &[u8] = b"echo:optic-live-tail-witness-set:v1\0";
const OPTIC_METADATA_APERTURE_MIN_BYTES: u64 = 128;

macro_rules! opaque_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name([u8; 32]);

        impl $name {
            /// Reconstructs the id from canonical bytes.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Returns the canonical byte representation.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }
    };
}

opaque_id!(
    /// Stable identity for an authored or kernel observer plan.
    ObserverPlanId
);

opaque_id!(
    /// Stable identity for a hosted observer instance.
    ObserverInstanceId
);

/// Coordinate selector for an observation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationCoordinate {
    /// Worldline to observe.
    pub worldline_id: WorldlineId,
    /// Requested coordinate within the worldline.
    pub at: ObservationAt,
}

impl ObservationCoordinate {
    fn to_abi(&self) -> abi::ObservationCoordinate {
        abi::ObservationCoordinate {
            worldline_id: abi::WorldlineId::from_bytes(*self.worldline_id.as_bytes()),
            at: self.at.to_abi(),
        }
    }
}

/// Requested position within a worldline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationAt {
    /// Observe the current worldline frontier.
    Frontier,
    /// Observe a specific committed historical tick.
    Tick(WorldlineTick),
}

impl ObservationAt {
    fn to_abi(self) -> abi::ObservationAt {
        match self {
            Self::Frontier => abi::ObservationAt::Frontier,
            Self::Tick(worldline_tick) => abi::ObservationAt::Tick {
                worldline_tick: abi::WorldlineTick(worldline_tick.as_u64()),
            },
        }
    }
}

/// Semantic frame declared by an observation request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationFrame {
    /// Read commit-boundary state metadata.
    CommitBoundary,
    /// Read recorded truth from provenance outputs.
    RecordedTruth,
    /// Read query-shaped projections.
    QueryView,
}

impl ObservationFrame {
    fn to_abi(self) -> abi::ObservationFrame {
        match self {
            Self::CommitBoundary => abi::ObservationFrame::CommitBoundary,
            Self::RecordedTruth => abi::ObservationFrame::RecordedTruth,
            Self::QueryView => abi::ObservationFrame::QueryView,
        }
    }
}

/// Coarse projection kind used by the validity matrix and deterministic errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationProjectionKind {
    /// Head metadata projection.
    Head,
    /// Snapshot metadata projection.
    Snapshot,
    /// Recorded truth channels projection.
    TruthChannels,
    /// Query payload projection.
    Query,
}

impl ObservationProjectionKind {
    /// Converts a validated internal projection into the ABI form.
    ///
    /// This helper is only valid when `self` and `projection` are matching
    /// variants. Reaching the fallback arm is a programmer error in the
    /// observation service, not a recoverable runtime condition.
    fn to_abi(self, projection: &ObservationProjection) -> abi::ObservationProjection {
        match (self, projection) {
            (Self::Head, ObservationProjection::Head) => abi::ObservationProjection::Head,
            (Self::Snapshot, ObservationProjection::Snapshot) => {
                abi::ObservationProjection::Snapshot
            }
            (Self::TruthChannels, ObservationProjection::TruthChannels { channels }) => {
                abi::ObservationProjection::TruthChannels {
                    channels: channels.as_ref().map(|ids| {
                        ids.iter()
                            .map(|channel| channel.0.to_vec())
                            .collect::<Vec<_>>()
                    }),
                }
            }
            (
                Self::Query,
                ObservationProjection::Query {
                    query_id,
                    vars_bytes,
                },
            ) => abi::ObservationProjection::Query {
                query_id: *query_id,
                vars_bytes: vars_bytes.clone(),
            },
            _ => {
                debug_assert!(
                    false,
                    "ObservationProjectionKind::to_abi requires matching kind/projection variants"
                );
                unreachable!(
                    "ObservationProjectionKind::to_abi requires matching kind/projection variants"
                )
            }
        }
    }
}

/// Requested projection within a frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationProjection {
    /// Head metadata projection.
    Head,
    /// Snapshot metadata projection.
    Snapshot,
    /// Recorded truth channels projection.
    TruthChannels {
        /// Optional channel filter. `None` means all recorded channels.
        channels: Option<Vec<ChannelId>>,
    },
    /// Query payload placeholder.
    Query {
        /// Stable query identifier.
        query_id: u32,
        /// Canonical vars payload bytes.
        vars_bytes: Vec<u8>,
    },
}

impl ObservationProjection {
    /// Returns the coarse projection kind used for validation and error reporting.
    #[must_use]
    pub fn kind(&self) -> ObservationProjectionKind {
        match self {
            Self::Head => ObservationProjectionKind::Head,
            Self::Snapshot => ObservationProjectionKind::Snapshot,
            Self::TruthChannels { .. } => ObservationProjectionKind::TruthChannels,
            Self::Query { .. } => ObservationProjectionKind::Query,
        }
    }

    fn to_abi(&self) -> abi::ObservationProjection {
        self.kind().to_abi(self)
    }
}

/// Canonical observation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationRequest {
    /// Worldline coordinate being observed.
    pub coordinate: ObservationCoordinate,
    /// Declared semantic frame.
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
        let observer_plan = builtin_observer_plan_for(frame, projection.kind());
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

    /// Converts the request to the shared ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::ObservationRequest {
        abi::ObservationRequest {
            coordinate: self.coordinate.to_abi(),
            frame: self.frame.to_abi(),
            projection: self.projection.to_abi(),
            observer_plan: self.observer_plan.to_abi(),
            observer_instance: self
                .observer_instance
                .as_ref()
                .map(ObserverInstanceRef::to_abi),
            budget: self.budget.to_abi(),
            rights: self.rights.to_abi(),
        }
    }
}

/// Fully resolved coordinate returned with every observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedObservationCoordinate {
    /// Observation contract version.
    pub observation_version: u32,
    /// Worldline that was actually observed.
    pub worldline_id: WorldlineId,
    /// Original coordinate selector from the request.
    pub requested_at: ObservationAt,
    /// Concrete resolved tick.
    pub resolved_worldline_tick: WorldlineTick,
    /// Commit-cycle stamp for the resolved commit, if any.
    pub commit_global_tick: Option<GlobalTick>,
    /// Observation freshness watermark after resolving this artifact.
    pub observed_after_global_tick: Option<GlobalTick>,
    /// Canonical state root at the resolved coordinate.
    pub state_root: Hash,
    /// Canonical commit hash at the resolved coordinate.
    pub commit_hash: Hash,
}

impl ResolvedObservationCoordinate {
    pub(crate) fn to_abi(&self) -> abi::ResolvedObservationCoordinate {
        abi::ResolvedObservationCoordinate {
            observation_version: self.observation_version,
            worldline_id: abi::WorldlineId::from_bytes(*self.worldline_id.as_bytes()),
            requested_at: self.requested_at.to_abi(),
            resolved_worldline_tick: abi::WorldlineTick(self.resolved_worldline_tick.as_u64()),
            commit_global_tick: self
                .commit_global_tick
                .map(|tick| abi::GlobalTick(tick.as_u64())),
            observed_after_global_tick: self
                .observed_after_global_tick
                .map(|tick| abi::GlobalTick(tick.as_u64())),
            state_root: self.state_root.to_vec(),
            commit_hash: self.commit_hash.to_vec(),
        }
    }
}

/// Read-side basis posture carried by every observation artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
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
        parent_from: crate::provenance_store::ProvenanceRef,
        /// Current parent basis used for the read.
        parent_to: crate::provenance_store::ProvenanceRef,
    },
    /// Live strand frontier read after parent movement inside the owned footprint.
    StrandRevalidationRequired {
        /// Live strand whose child worldline was read.
        strand_id: StrandId,
        /// Anchor coordinate from which the strand diverged.
        parent_from: crate::provenance_store::ProvenanceRef,
        /// Current parent basis that must be revalidated.
        parent_to: crate::provenance_store::ProvenanceRef,
        /// Parent-written slots that overlap the strand-owned closed footprint.
        overlapping_slots: Vec<SlotId>,
    },
}

impl ObservationBasisPosture {
    fn to_abi(&self) -> abi::ObservationBasisPosture {
        match self {
            Self::Worldline => abi::ObservationBasisPosture::Worldline,
            Self::StrandHistorical { strand_id } => {
                abi::ObservationBasisPosture::StrandHistorical {
                    strand_id: abi::StrandId::from_bytes(*strand_id.as_bytes()),
                }
            }
            Self::StrandAtAnchor { strand_id } => abi::ObservationBasisPosture::StrandAtAnchor {
                strand_id: abi::StrandId::from_bytes(*strand_id.as_bytes()),
            },
            Self::StrandParentAdvancedDisjoint {
                strand_id,
                parent_from,
                parent_to,
            } => abi::ObservationBasisPosture::StrandParentAdvancedDisjoint {
                strand_id: abi::StrandId::from_bytes(*strand_id.as_bytes()),
                parent_from: provenance_ref_to_abi(*parent_from),
                parent_to: provenance_ref_to_abi(*parent_to),
            },
            Self::StrandRevalidationRequired {
                strand_id,
                parent_from,
                parent_to,
                overlapping_slots,
            } => abi::ObservationBasisPosture::StrandRevalidationRequired {
                strand_id: abi::StrandId::from_bytes(*strand_id.as_bytes()),
                parent_from: provenance_ref_to_abi(*parent_from),
                parent_to: provenance_ref_to_abi(*parent_to),
                overlapping_slot_count: overlapping_slots.len() as u64,
                overlapping_slots_digest: overlap_slots_digest(overlapping_slots).to_vec(),
            },
        }
    }
}

/// Built-in observer plans provided by the kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl BuiltinObserverPlan {
    fn to_abi(self) -> abi::BuiltinObserverPlan {
        match self {
            Self::CommitBoundaryHead => abi::BuiltinObserverPlan::CommitBoundaryHead,
            Self::CommitBoundarySnapshot => abi::BuiltinObserverPlan::CommitBoundarySnapshot,
            Self::RecordedTruthChannels => abi::BuiltinObserverPlan::RecordedTruthChannels,
            Self::QueryBytes => abi::BuiltinObserverPlan::QueryBytes,
        }
    }
}

/// Authored observer plan identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoredObserverPlan {
    /// Stable plan identity.
    pub plan_id: ObserverPlanId,
    /// Hash of the generated or installed observer artifact.
    pub artifact_hash: Hash,
    /// Hash of the authored schema or contract family.
    pub schema_hash: Hash,
    /// Hash of the observer state schema.
    pub state_schema_hash: Hash,
    /// Hash of the observer update law.
    pub update_law_hash: Hash,
    /// Hash of the observer emission law.
    pub emission_law_hash: Hash,
}

impl AuthoredObserverPlan {
    fn to_abi(&self) -> abi::AuthoredObserverPlan {
        abi::AuthoredObserverPlan {
            plan_id: observer_plan_id_to_abi(self.plan_id),
            artifact_hash: self.artifact_hash.to_vec(),
            schema_hash: self.schema_hash.to_vec(),
            state_schema_hash: self.state_schema_hash.to_vec(),
            update_law_hash: self.update_law_hash.to_vec(),
            emission_law_hash: self.emission_law_hash.to_vec(),
        }
    }
}

/// Observer plan identity for a reading artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl ReadingObserverPlan {
    fn to_abi(&self) -> abi::ReadingObserverPlan {
        match self {
            Self::Builtin { plan } => abi::ReadingObserverPlan::Builtin {
                plan: plan.to_abi(),
            },
            Self::Authored { plan } => abi::ReadingObserverPlan::Authored {
                plan: Box::new(plan.to_abi()),
            },
        }
    }
}

fn builtin_observer_plan_for(
    frame: ObservationFrame,
    projection: ObservationProjectionKind,
) -> ReadingObserverPlan {
    ReadingObserverPlan::Builtin {
        plan: ObservationService::builtin_observer_plan(frame, projection),
    }
}

/// Hosted observer instance identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverInstanceRef {
    /// Runtime instance identity.
    pub instance_id: ObserverInstanceId,
    /// Plan that owns this instance.
    pub plan_id: ObserverPlanId,
    /// Hash of the accumulated observer state.
    pub state_hash: Hash,
}

impl ObserverInstanceRef {
    fn to_abi(&self) -> abi::ObserverInstanceRef {
        abi::ObserverInstanceRef {
            instance_id: observer_instance_id_to_abi(self.instance_id),
            plan_id: observer_plan_id_to_abi(self.plan_id),
            state_hash: self.state_hash.to_vec(),
        }
    }
}

/// Native observer basis used by the emitted reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadingObserverBasis {
    /// Commit-boundary observer basis.
    CommitBoundary,
    /// Recorded-truth observer basis.
    RecordedTruth,
    /// Query-view observer basis.
    QueryView,
}

impl ReadingObserverBasis {
    fn to_abi(self) -> abi::ReadingObserverBasis {
        match self {
            Self::CommitBoundary => abi::ReadingObserverBasis::CommitBoundary,
            Self::RecordedTruth => abi::ReadingObserverBasis::RecordedTruth,
            Self::QueryView => abi::ReadingObserverBasis::QueryView,
        }
    }
}

/// Read budget requested by an observation caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl ObservationReadBudget {
    fn to_abi(self) -> abi::ObservationReadBudget {
        match self {
            Self::UnboundedOneShot => abi::ObservationReadBudget::UnboundedOneShot,
            Self::Bounded {
                max_payload_bytes,
                max_witness_refs,
            } => abi::ObservationReadBudget::Bounded {
                max_payload_bytes,
                max_witness_refs,
            },
        }
    }
}

/// Rights posture requested by an observation caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl ObservationRights {
    fn to_abi(self) -> abi::ObservationRights {
        match self {
            Self::KernelPublic => abi::ObservationRights::KernelPublic,
            Self::CapabilityScoped { capability } => abi::ObservationRights::CapabilityScoped {
                capability: abi::OpticCapabilityId::from_bytes(*capability.as_bytes()),
            },
        }
    }
}

/// Witness reference carried by a reading artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
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
        state_root: Hash,
        /// Deterministic empty-frontier commit/frontier hash.
        commit_hash: Hash,
    },
}

impl ReadingWitnessRef {
    fn to_abi(&self) -> abi::ReadingWitnessRef {
        match self {
            Self::ResolvedCommit { reference } => abi::ReadingWitnessRef::ResolvedCommit {
                reference: provenance_ref_to_abi(*reference),
            },
            Self::EmptyFrontier {
                worldline_id,
                state_root,
                commit_hash,
            } => abi::ReadingWitnessRef::EmptyFrontier {
                worldline_id: abi::WorldlineId::from_bytes(*worldline_id.as_bytes()),
                state_root: state_root.to_vec(),
                commit_hash: commit_hash.to_vec(),
            },
        }
    }
}

/// Budget posture for a reading artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl ReadingBudgetPosture {
    fn to_abi(self) -> abi::ReadingBudgetPosture {
        match self {
            Self::UnboundedOneShot => abi::ReadingBudgetPosture::UnboundedOneShot,
            Self::Bounded {
                max_payload_bytes,
                payload_bytes,
                max_witness_refs,
                witness_refs,
            } => abi::ReadingBudgetPosture::Bounded {
                max_payload_bytes,
                payload_bytes,
                max_witness_refs,
                witness_refs,
            },
        }
    }
}

/// Rights posture for a reading artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadingRightsPosture {
    /// Kernel-public reading with no app-specific authorization layer.
    KernelPublic,
}

impl ReadingRightsPosture {
    fn to_abi(self) -> abi::ReadingRightsPosture {
        match self {
            Self::KernelPublic => abi::ReadingRightsPosture::KernelPublic,
        }
    }
}

/// Residual posture for a reading artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl ReadingResidualPosture {
    fn to_abi(self) -> abi::ReadingResidualPosture {
        match self {
            Self::Complete => abi::ReadingResidualPosture::Complete,
            Self::Residual => abi::ReadingResidualPosture::Residual,
            Self::PluralityPreserved => abi::ReadingResidualPosture::PluralityPreserved,
            Self::Obstructed => abi::ReadingResidualPosture::Obstructed,
        }
    }
}

/// Reading-envelope metadata carried by every observation artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl ReadingEnvelope {
    pub(crate) fn to_abi(&self) -> abi::ReadingEnvelope {
        abi::ReadingEnvelope {
            observer_plan: self.observer_plan.to_abi(),
            observer_instance: self
                .observer_instance
                .as_ref()
                .map(ObserverInstanceRef::to_abi),
            observer_basis: self.observer_basis.to_abi(),
            witness_refs: self
                .witness_refs
                .iter()
                .map(ReadingWitnessRef::to_abi)
                .collect(),
            parent_basis_posture: self.parent_basis_posture.to_abi(),
            budget_posture: self.budget_posture.to_abi(),
            rights_posture: self.rights_posture.to_abi(),
            residual_posture: self.residual_posture.to_abi(),
        }
    }
}

/// Minimal frontier/head observation payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadObservation {
    /// Observed tick.
    pub worldline_tick: WorldlineTick,
    /// Commit-cycle stamp for the observed commit, if any.
    pub commit_global_tick: Option<GlobalTick>,
    /// Canonical state root at that tick.
    pub state_root: Hash,
    /// Canonical commit hash at that tick.
    pub commit_hash: Hash,
}

impl HeadObservation {
    fn to_abi(&self) -> abi::HeadObservation {
        abi::HeadObservation {
            worldline_tick: abi::WorldlineTick(self.worldline_tick.as_u64()),
            commit_global_tick: self
                .commit_global_tick
                .map(|tick| abi::GlobalTick(tick.as_u64())),
            state_root: self.state_root.to_vec(),
            commit_id: self.commit_hash.to_vec(),
        }
    }
}

/// Minimal historical snapshot observation payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldlineSnapshot {
    /// Observed historical tick.
    pub worldline_tick: WorldlineTick,
    /// Commit-cycle stamp for the observed commit, if any.
    pub commit_global_tick: Option<GlobalTick>,
    /// Canonical state root at that tick.
    pub state_root: Hash,
    /// Canonical commit hash at that tick.
    pub commit_hash: Hash,
}

impl WorldlineSnapshot {
    fn to_abi(&self) -> abi::SnapshotObservation {
        abi::SnapshotObservation {
            worldline_tick: abi::WorldlineTick(self.worldline_tick.as_u64()),
            commit_global_tick: self
                .commit_global_tick
                .map(|tick| abi::GlobalTick(tick.as_u64())),
            state_root: self.state_root.to_vec(),
            commit_id: self.commit_hash.to_vec(),
        }
    }
}

/// Observation payload variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationPayload {
    /// Head metadata.
    Head(HeadObservation),
    /// Historical snapshot metadata.
    Snapshot(WorldlineSnapshot),
    /// Recorded truth payloads in channel-id order.
    TruthChannels(Vec<(ChannelId, Vec<u8>)>),
    /// Query result bytes.
    QueryBytes(Vec<u8>),
}

impl ObservationPayload {
    pub(crate) fn to_abi(&self) -> abi::ObservationPayload {
        match self {
            Self::Head(head) => abi::ObservationPayload::Head {
                head: head.to_abi(),
            },
            Self::Snapshot(snapshot) => abi::ObservationPayload::Snapshot {
                snapshot: snapshot.to_abi(),
            },
            Self::TruthChannels(channels) => abi::ObservationPayload::TruthChannels {
                channels: channels
                    .iter()
                    .map(|(channel, data)| abi::ChannelData {
                        channel_id: channel.0.to_vec(),
                        data: data.clone(),
                    })
                    .collect(),
            },
            Self::QueryBytes(data) => abi::ObservationPayload::QueryBytes { data: data.clone() },
        }
    }
}

/// Full observation artifact with deterministic identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationArtifact {
    /// Resolved coordinate metadata.
    pub resolved: ResolvedObservationCoordinate,
    /// Reading-envelope metadata.
    pub reading: ReadingEnvelope,
    /// Declared semantic frame.
    pub frame: ObservationFrame,
    /// Declared projection.
    pub projection: ObservationProjection,
    /// Deterministic artifact hash.
    pub artifact_hash: Hash,
    /// Observation payload.
    pub payload: ObservationPayload,
}

impl ObservationArtifact {
    /// Converts this artifact into the shared ABI DTO shape.
    #[must_use]
    pub fn to_abi(&self) -> abi::ObservationArtifact {
        abi::ObservationArtifact {
            resolved: self.resolved.to_abi(),
            reading: self.reading.to_abi(),
            frame: self.frame.to_abi(),
            projection: self.projection.to_abi(),
            artifact_hash: self.artifact_hash.to_vec(),
            payload: self.payload.to_abi(),
        }
    }
}

/// Deterministic observation failures.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ObservationError {
    /// The requested worldline is not registered.
    #[error("invalid worldline: {0:?}")]
    InvalidWorldline(WorldlineId),
    /// The requested historical tick is not available.
    #[error("invalid tick {tick} for worldline {worldline_id:?}")]
    InvalidTick {
        /// Worldline that was targeted.
        worldline_id: WorldlineId,
        /// Requested tick.
        tick: WorldlineTick,
    },
    /// The frame/projection pairing is not valid in v1.
    #[error("unsupported frame/projection pairing: {frame:?} + {projection:?}")]
    UnsupportedFrameProjection {
        /// Declared frame.
        frame: ObservationFrame,
        /// Requested projection kind.
        projection: ObservationProjectionKind,
    },
    /// Query observation is not implemented yet.
    #[error("query observation is not supported in phase 5")]
    UnsupportedQuery,
    /// The requested observer plan is not installed or executable.
    #[error("unsupported observer plan: {0:?}")]
    UnsupportedObserverPlan(ReadingObserverPlan),
    /// The requested hosted observer instance is not installed or executable.
    #[error("unsupported observer instance: {0:?}")]
    UnsupportedObserverInstance(ObserverInstanceRef),
    /// The requested observation rights posture is not executable.
    #[error("unsupported observation rights posture: {0:?}")]
    UnsupportedRights(ObservationRights),
    /// The requested observation exceeded its declared budget.
    #[error(
        "observation budget exceeded: payload {payload_bytes}/{max_payload_bytes} bytes, witness refs {witness_refs}/{max_witness_refs}"
    )]
    BudgetExceeded {
        /// Declared payload byte limit.
        max_payload_bytes: u64,
        /// Encoded payload bytes produced by the observer.
        payload_bytes: u64,
        /// Declared witness-reference limit.
        max_witness_refs: u64,
        /// Witness references needed by the reading.
        witness_refs: u64,
    },
    /// The requested observation cannot be produced at this coordinate.
    #[error("observation unavailable for worldline {worldline_id:?} at {at:?}")]
    ObservationUnavailable {
        /// Worldline that was targeted.
        worldline_id: WorldlineId,
        /// Requested coordinate.
        at: ObservationAt,
    },
    /// Canonical artifact encoding failed.
    #[error("observation artifact encoding failed: {0}")]
    CodecFailure(String),
}

/// Immutable observation service.
pub struct ObservationService;

impl ObservationService {
    /// Observe a worldline at an explicit coordinate and frame.
    ///
    /// The runtime, provenance store, and engine are borrowed immutably. This
    /// method never mutates live frontier state or recorded history.
    ///
    /// # Errors
    ///
    /// Returns [`ObservationError`] for invalid worldlines/ticks, unsupported
    /// frame/projection pairings, unsupported query requests, or unavailable
    /// recorded truth.
    pub fn observe(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        engine: &Engine,
        request: ObservationRequest,
    ) -> Result<ObservationArtifact, ObservationError> {
        let worldline_id = request.coordinate.worldline_id;
        if runtime.worldlines().get(&worldline_id).is_none() {
            return Err(ObservationError::InvalidWorldline(worldline_id));
        }
        Self::validate_frame_projection(request.frame, &request.projection)?;
        Self::validate_observer_contract(&request)?;
        if matches!(request.frame, ObservationFrame::QueryView) {
            return Err(ObservationError::UnsupportedQuery);
        }

        let resolved = Self::resolve_coordinate(runtime, provenance, engine, &request)?;
        let parent_basis_posture =
            Self::basis_posture(runtime, provenance, worldline_id, request.coordinate.at)?;
        let payload = match (&request.frame, &request.projection) {
            (ObservationFrame::CommitBoundary, ObservationProjection::Head) => {
                ObservationPayload::Head(HeadObservation {
                    worldline_tick: resolved.resolved_worldline_tick,
                    commit_global_tick: resolved.commit_global_tick,
                    state_root: resolved.state_root,
                    commit_hash: resolved.commit_hash,
                })
            }
            (ObservationFrame::CommitBoundary, ObservationProjection::Snapshot) => {
                ObservationPayload::Snapshot(WorldlineSnapshot {
                    worldline_tick: resolved.resolved_worldline_tick,
                    commit_global_tick: resolved.commit_global_tick,
                    state_root: resolved.state_root,
                    commit_hash: resolved.commit_hash,
                })
            }
            (
                ObservationFrame::RecordedTruth,
                ObservationProjection::TruthChannels { channels },
            ) => {
                let entry = provenance
                    .entry(worldline_id, resolved.resolved_worldline_tick)
                    .map_err(|_| ObservationError::ObservationUnavailable {
                        worldline_id,
                        at: request.coordinate.at,
                    })?;
                let outputs = match channels {
                    Some(filter) => entry
                        .outputs
                        .into_iter()
                        .filter(|(channel, _)| filter.contains(channel))
                        .collect(),
                    None => entry.outputs,
                };
                ObservationPayload::TruthChannels(outputs)
            }
            (ObservationFrame::QueryView, ObservationProjection::Query { .. }) => {
                return Err(ObservationError::UnsupportedQuery);
            }
            _ => unreachable!("validity matrix must reject unsupported combinations"),
        };
        let reading = Self::reading_envelope(&resolved, parent_basis_posture, &request, &payload)?;

        let artifact_hash = Self::compute_artifact_hash(
            &resolved,
            &reading,
            request.frame,
            &request.projection,
            &payload,
        )?;
        Ok(ObservationArtifact {
            resolved,
            reading,
            frame: request.frame,
            projection: request.projection,
            artifact_hash,
            payload,
        })
    }

    /// Observe a worldline through a bounded optic request.
    ///
    /// This is the first narrow bridge from optics into the existing
    /// observation path. It supports commit-boundary head and snapshot
    /// apertures and returns typed obstructions for unsupported or unbounded
    /// reads instead of widening the read behind the caller's back.
    pub fn observe_optic(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        engine: &Engine,
        request: ObserveOpticRequest,
    ) -> ObserveOpticResult {
        match Self::observe_optic_inner(runtime, provenance, engine, &request) {
            Ok(reading) => ObserveOpticResult::Reading(Box::new(reading)),
            Err(obstruction) => ObserveOpticResult::Obstructed(obstruction),
        }
    }

    fn observe_optic_inner(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        engine: &Engine,
        request: &ObserveOpticRequest,
    ) -> Result<OpticReading, Box<OpticObstruction>> {
        Self::validate_optic_budget(request)?;
        if let Some(obstruction) = Self::attachment_boundary_obstruction(request) {
            return Err(obstruction);
        }
        let observation_request = Self::optic_observation_request(request)?;
        let artifact = Self::observe(runtime, provenance, engine, observation_request)
            .map_err(|err| Self::optic_observation_error(request, err))?;
        let witness_basis = Self::optic_witness_basis(provenance, request, &artifact)?;
        let read_identity = ReadIdentity::new(
            request.optic_id,
            &request.focus,
            request.coordinate.clone(),
            &request.aperture,
            request.projection_version,
            request.reducer_version,
            witness_basis,
            artifact.reading.rights_posture,
            artifact.reading.budget_posture,
            artifact.reading.residual_posture,
        );

        Ok(OpticReading {
            envelope: artifact.reading,
            read_identity,
            payload: artifact.payload,
            retained: None,
        })
    }

    fn validate_optic_budget(request: &ObserveOpticRequest) -> Result<(), Box<OpticObstruction>> {
        let Some(max_bytes) = request.aperture.budget.max_bytes else {
            return Err(Self::optic_obstruction(
                request,
                OpticObstructionKind::BudgetExceeded,
                Some(WitnessBasis::Missing {
                    reason: MissingWitnessBasisReason::BudgetLimited,
                }),
                "optic reads must declare a byte budget",
            ));
        };
        if max_bytes == 0 {
            return Err(Self::optic_obstruction(
                request,
                OpticObstructionKind::BudgetExceeded,
                Some(WitnessBasis::Missing {
                    reason: MissingWitnessBasisReason::BudgetLimited,
                }),
                "optic byte budget is zero",
            ));
        }

        match &request.aperture.shape {
            OpticApertureShape::Head | OpticApertureShape::SnapshotMetadata
                if max_bytes < OPTIC_METADATA_APERTURE_MIN_BYTES =>
            {
                Err(Self::optic_obstruction(
                    request,
                    OpticObstructionKind::BudgetExceeded,
                    Some(WitnessBasis::Missing {
                        reason: MissingWitnessBasisReason::BudgetLimited,
                    }),
                    "optic metadata aperture exceeds the declared byte budget",
                ))
            }
            OpticApertureShape::ByteRange { len, .. } if *len > max_bytes => {
                Err(Self::optic_obstruction(
                    request,
                    OpticObstructionKind::BudgetExceeded,
                    Some(WitnessBasis::Missing {
                        reason: MissingWitnessBasisReason::BudgetLimited,
                    }),
                    "optic byte-range aperture exceeds the declared byte budget",
                ))
            }
            _ => Ok(()),
        }
    }

    fn attachment_boundary_obstruction(
        request: &ObserveOpticRequest,
    ) -> Option<Box<OpticObstruction>> {
        if !matches!(request.focus, OpticFocus::AttachmentBoundary { .. }) {
            return None;
        }

        match (&request.aperture.shape, request.aperture.attachment_descent) {
            (OpticApertureShape::AttachmentBoundary, AttachmentDescentPolicy::BoundaryOnly) => {
                Some(Self::optic_obstruction(
                    request,
                    OpticObstructionKind::AttachmentDescentRequired,
                    Some(WitnessBasis::Missing {
                        reason: MissingWitnessBasisReason::UnsupportedBasis,
                    }),
                    "optic read reached an attachment boundary; recursive descent requires an explicit aperture, capability, budget, and law",
                ))
            }
            (OpticApertureShape::AttachmentBoundary, AttachmentDescentPolicy::Explicit)
                if request.aperture.budget.max_attachments.unwrap_or(0) == 0 =>
            {
                Some(Self::optic_obstruction(
                    request,
                    OpticObstructionKind::BudgetExceeded,
                    Some(WitnessBasis::Missing {
                        reason: MissingWitnessBasisReason::BudgetLimited,
                    }),
                    "explicit attachment descent requires a positive attachment budget",
                ))
            }
            (OpticApertureShape::AttachmentBoundary, AttachmentDescentPolicy::Explicit) => {
                Some(Self::optic_obstruction(
                    request,
                    OpticObstructionKind::AttachmentDescentDenied,
                    Some(WitnessBasis::Missing {
                        reason: MissingWitnessBasisReason::RightsLimited,
                    }),
                    "explicit attachment descent has no installed capability checker or projection law",
                ))
            }
            _ => Some(Self::optic_obstruction(
                request,
                OpticObstructionKind::UnsupportedAperture,
                Some(WitnessBasis::Missing {
                    reason: MissingWitnessBasisReason::UnsupportedBasis,
                }),
                "attachment boundary focus requires an attachment-boundary aperture",
            )),
        }
    }

    fn optic_observation_request(
        request: &ObserveOpticRequest,
    ) -> Result<ObservationRequest, Box<OpticObstruction>> {
        let (
            OpticFocus::Worldline {
                worldline_id: focus_worldline,
            },
            EchoCoordinate::Worldline { worldline_id, at },
        ) = (&request.focus, &request.coordinate)
        else {
            return Err(Self::optic_obstruction(
                request,
                OpticObstructionKind::UnsupportedProjectionLaw,
                None,
                "observe_optic currently supports worldline coordinates only",
            ));
        };

        if focus_worldline != worldline_id {
            return Err(Self::optic_obstruction(
                request,
                OpticObstructionKind::ConflictingFrontier,
                None,
                "optic focus and coordinate name different worldlines",
            ));
        }

        let at = Self::optic_coordinate_at(request, *worldline_id, *at)?;
        let (frame, projection) = match &request.aperture.shape {
            OpticApertureShape::Head => (
                ObservationFrame::CommitBoundary,
                ObservationProjection::Head,
            ),
            OpticApertureShape::SnapshotMetadata => (
                ObservationFrame::CommitBoundary,
                ObservationProjection::Snapshot,
            ),
            OpticApertureShape::QueryBytes { .. } => {
                return Err(Self::optic_obstruction(
                    request,
                    OpticObstructionKind::UnsupportedProjectionLaw,
                    None,
                    "contract QueryView optics are not installed yet",
                ));
            }
            _ => {
                return Err(Self::optic_obstruction(
                    request,
                    OpticObstructionKind::UnsupportedAperture,
                    None,
                    "this optic aperture is not supported by the observation bridge",
                ));
            }
        };

        let mut observation_request = ObservationRequest::builtin_one_shot(
            ObservationCoordinate {
                worldline_id: *worldline_id,
                at,
            },
            frame,
            projection,
        );
        if let Some(max_payload_bytes) = request.aperture.budget.max_bytes {
            observation_request.budget = ObservationReadBudget::Bounded {
                max_payload_bytes,
                max_witness_refs: request.aperture.budget.max_ticks.unwrap_or(u64::MAX),
            };
        }
        Ok(observation_request)
    }

    fn optic_coordinate_at(
        request: &ObserveOpticRequest,
        worldline_id: WorldlineId,
        at: CoordinateAt,
    ) -> Result<ObservationAt, Box<OpticObstruction>> {
        match at {
            CoordinateAt::Frontier => Ok(ObservationAt::Frontier),
            CoordinateAt::Tick(tick) => Ok(ObservationAt::Tick(tick)),
            CoordinateAt::Provenance(reference) if reference.worldline_id == worldline_id => {
                Ok(ObservationAt::Tick(reference.worldline_tick))
            }
            CoordinateAt::Provenance(_) => Err(Self::optic_obstruction(
                request,
                OpticObstructionKind::ConflictingFrontier,
                None,
                "provenance coordinate belongs to a different worldline",
            )),
        }
    }

    fn optic_observation_error(
        request: &ObserveOpticRequest,
        error: ObservationError,
    ) -> Box<OpticObstruction> {
        match error {
            ObservationError::InvalidWorldline(_)
            | ObservationError::InvalidTick { .. }
            | ObservationError::ObservationUnavailable { .. } => Self::optic_obstruction(
                request,
                OpticObstructionKind::MissingWitness,
                Some(WitnessBasis::Missing {
                    reason: MissingWitnessBasisReason::EvidenceUnavailable,
                }),
                "required observation witness evidence is unavailable",
            ),
            ObservationError::UnsupportedFrameProjection { .. } => Self::optic_obstruction(
                request,
                OpticObstructionKind::UnsupportedAperture,
                None,
                "unsupported optic frame/projection pairing",
            ),
            ObservationError::UnsupportedQuery
            | ObservationError::UnsupportedObserverPlan(_)
            | ObservationError::UnsupportedObserverInstance(_) => Self::optic_obstruction(
                request,
                OpticObstructionKind::UnsupportedProjectionLaw,
                None,
                "contract QueryView optics are not installed yet",
            ),
            ObservationError::UnsupportedRights(_) => Self::optic_obstruction(
                request,
                OpticObstructionKind::CapabilityDenied,
                None,
                "observation rights posture is not authorized",
            ),
            ObservationError::BudgetExceeded { .. } => Self::optic_obstruction(
                request,
                OpticObstructionKind::BudgetExceeded,
                Some(WitnessBasis::Missing {
                    reason: MissingWitnessBasisReason::BudgetLimited,
                }),
                "optic observation exceeded the declared read budget",
            ),
            ObservationError::CodecFailure(_) => Self::optic_obstruction(
                request,
                OpticObstructionKind::MissingWitness,
                Some(WitnessBasis::Missing {
                    reason: MissingWitnessBasisReason::EvidenceUnavailable,
                }),
                "observation artifact could not be encoded as witness evidence",
            ),
        }
    }

    fn optic_obstruction(
        request: &ObserveOpticRequest,
        kind: OpticObstructionKind,
        witness_basis: Option<WitnessBasis>,
        message: &str,
    ) -> Box<OpticObstruction> {
        Box::new(OpticObstruction {
            kind,
            optic_id: Some(request.optic_id),
            focus: Some(request.focus.clone()),
            coordinate: Some(request.coordinate.clone()),
            witness_basis,
            message: message.to_owned(),
        })
    }

    fn optic_witness_basis(
        provenance: &ProvenanceService,
        request: &ObserveOpticRequest,
        artifact: &ObservationArtifact,
    ) -> Result<WitnessBasis, Box<OpticObstruction>> {
        if let Some(witness_basis) =
            Self::checkpoint_plus_tail_witness_basis(provenance, request, artifact)?
        {
            return Ok(witness_basis);
        }

        Ok(Self::artifact_witness_basis(artifact))
    }

    fn artifact_witness_basis(artifact: &ObservationArtifact) -> WitnessBasis {
        match artifact.reading.witness_refs.as_slice() {
            [ReadingWitnessRef::ResolvedCommit { reference }] => WitnessBasis::ResolvedCommit {
                reference: *reference,
                state_root: artifact.resolved.state_root,
                commit_hash: artifact.resolved.commit_hash,
            },
            refs => WitnessBasis::WitnessSet {
                refs: refs.to_vec(),
                witness_set_hash: optic_witness_refs_hash(refs),
            },
        }
    }

    fn checkpoint_plus_tail_witness_basis(
        provenance: &ProvenanceService,
        request: &ObserveOpticRequest,
        artifact: &ObservationArtifact,
    ) -> Result<Option<WitnessBasis>, Box<OpticObstruction>> {
        let EchoCoordinate::Worldline { worldline_id, .. } = request.coordinate else {
            return Ok(None);
        };
        let [ReadingWitnessRef::ResolvedCommit { .. }] = artifact.reading.witness_refs.as_slice()
        else {
            return Ok(None);
        };
        let materialized_tick = artifact.resolved.resolved_worldline_tick;
        if materialized_tick == WorldlineTick::ZERO {
            return Ok(None);
        }
        let Some(checkpoint) = provenance.checkpoint_before(worldline_id, materialized_tick) else {
            return Ok(None);
        };
        if checkpoint.worldline_tick == WorldlineTick::ZERO {
            return Ok(None);
        }
        if checkpoint.worldline_tick >= materialized_tick {
            return Ok(None);
        }

        let tail_start = checkpoint.worldline_tick.as_u64();
        let tail_end = materialized_tick
            .checked_sub(1)
            .map(WorldlineTick::as_u64)
            .ok_or_else(|| {
                Self::optic_obstruction(
                    request,
                    OpticObstructionKind::LiveTailRequiresReduction,
                    Some(WitnessBasis::Missing {
                        reason: MissingWitnessBasisReason::EvidenceUnavailable,
                    }),
                    "live-tail witness span has no commit boundary",
                )
            })?;
        if tail_start > tail_end {
            return Ok(None);
        }
        let tail_len = tail_end
            .checked_sub(tail_start)
            .and_then(|len| len.checked_add(1))
            .ok_or_else(|| {
                Self::optic_obstruction(
                    request,
                    OpticObstructionKind::LiveTailRequiresReduction,
                    Some(WitnessBasis::Missing {
                        reason: MissingWitnessBasisReason::EvidenceUnavailable,
                    }),
                    "live-tail witness span overflowed",
                )
            })?;
        if tail_len > request.aperture.budget.max_ticks.unwrap_or(u64::MAX) {
            return Err(Self::optic_obstruction(
                request,
                OpticObstructionKind::LiveTailRequiresReduction,
                Some(WitnessBasis::Missing {
                    reason: MissingWitnessBasisReason::BudgetLimited,
                }),
                "live-tail witness set exceeds the declared tick budget",
            ));
        }

        let checkpoint_commit_tick = checkpoint.worldline_tick.checked_sub(1).ok_or_else(|| {
            Self::optic_obstruction(
                request,
                OpticObstructionKind::MissingWitness,
                Some(WitnessBasis::Missing {
                    reason: MissingWitnessBasisReason::EvidenceUnavailable,
                }),
                "checkpoint witness coordinate is unavailable",
            )
        })?;
        let checkpoint_entry = provenance
            .entry(worldline_id, checkpoint_commit_tick)
            .map_err(|_| {
                Self::optic_obstruction(
                    request,
                    OpticObstructionKind::MissingWitness,
                    Some(WitnessBasis::Missing {
                        reason: MissingWitnessBasisReason::EvidenceUnavailable,
                    }),
                    "checkpoint provenance witness entry is unavailable",
                )
            })?;
        let mut tail_witness_refs = Vec::new();
        for raw_tick in tail_start..=tail_end {
            let tick = WorldlineTick::from_raw(raw_tick);
            let entry = provenance.entry(worldline_id, tick).map_err(|_| {
                Self::optic_obstruction(
                    request,
                    OpticObstructionKind::MissingWitness,
                    Some(WitnessBasis::Missing {
                        reason: MissingWitnessBasisReason::EvidenceUnavailable,
                    }),
                    "live-tail provenance witness entry is unavailable",
                )
            })?;
            tail_witness_refs.push(ProvenanceRef {
                worldline_id,
                worldline_tick: tick,
                commit_hash: entry.expected.commit_hash,
            });
        }

        Ok(Some(WitnessBasis::CheckpointPlusTail {
            checkpoint_ref: ProvenanceRef {
                worldline_id,
                worldline_tick: checkpoint_commit_tick,
                commit_hash: checkpoint_entry.expected.commit_hash,
            },
            checkpoint_hash: checkpoint.state_hash,
            tail_digest: optic_tail_witness_refs_hash(&tail_witness_refs),
            tail_witness_refs,
        }))
    }

    fn validate_frame_projection(
        frame: ObservationFrame,
        projection: &ObservationProjection,
    ) -> Result<(), ObservationError> {
        let projection_kind = projection.kind();
        let valid = matches!(
            (frame, projection_kind),
            (
                ObservationFrame::CommitBoundary,
                ObservationProjectionKind::Head | ObservationProjectionKind::Snapshot
            ) | (
                ObservationFrame::RecordedTruth,
                ObservationProjectionKind::TruthChannels
            ) | (
                ObservationFrame::QueryView,
                ObservationProjectionKind::Query
            )
        );
        if valid {
            Ok(())
        } else {
            Err(ObservationError::UnsupportedFrameProjection {
                frame,
                projection: projection_kind,
            })
        }
    }

    fn validate_observer_contract(request: &ObservationRequest) -> Result<(), ObservationError> {
        let expected = Self::observer_plan(request.frame, request.projection.kind());
        match &request.observer_plan {
            ReadingObserverPlan::Builtin { .. } if request.observer_plan != expected => {
                return Err(ObservationError::UnsupportedObserverPlan(
                    request.observer_plan.clone(),
                ));
            }
            ReadingObserverPlan::Builtin { .. } => {}
            ReadingObserverPlan::Authored { .. } => {
                return Err(ObservationError::UnsupportedObserverPlan(
                    request.observer_plan.clone(),
                ));
            }
        }

        if let Some(instance) = &request.observer_instance {
            return Err(ObservationError::UnsupportedObserverInstance(
                instance.clone(),
            ));
        }

        if let ObservationRights::CapabilityScoped { .. } = request.rights {
            return Err(ObservationError::UnsupportedRights(request.rights));
        }

        Ok(())
    }

    fn resolve_coordinate(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        engine: &Engine,
        request: &ObservationRequest,
    ) -> Result<ResolvedObservationCoordinate, ObservationError> {
        let worldline_id = request.coordinate.worldline_id;
        let frontier = runtime
            .worldlines()
            .get(&worldline_id)
            .ok_or(ObservationError::InvalidWorldline(worldline_id))?;

        match (request.frame, request.coordinate.at) {
            (ObservationFrame::CommitBoundary, ObservationAt::Frontier) => {
                let snapshot = frontier
                    .state()
                    .last_snapshot()
                    .cloned()
                    .unwrap_or_else(|| engine.snapshot_for_state(frontier.state()));
                let commit_global_tick = frontier
                    .frontier_tick()
                    .checked_sub(1)
                    .map(|committed_tick| {
                        provenance
                            .entry(worldline_id, committed_tick)
                            .map(|entry| entry.commit_global_tick)
                            .map_err(|_| ObservationError::ObservationUnavailable {
                                worldline_id,
                                at: request.coordinate.at,
                            })
                    })
                    .transpose()?;
                Ok(Self::resolved_commit_boundary(
                    worldline_id,
                    request.coordinate.at,
                    frontier.frontier_tick(),
                    commit_global_tick,
                    runtime.global_tick(),
                    &snapshot,
                ))
            }
            (ObservationFrame::CommitBoundary, ObservationAt::Tick(tick)) => {
                let entry = provenance
                    .entry(worldline_id, tick)
                    .map_err(|_| ObservationError::InvalidTick { worldline_id, tick })?;
                Ok(ResolvedObservationCoordinate {
                    observation_version: OBSERVATION_VERSION,
                    worldline_id,
                    requested_at: request.coordinate.at,
                    resolved_worldline_tick: tick,
                    commit_global_tick: Some(entry.commit_global_tick),
                    observed_after_global_tick: current_cycle_tick(runtime),
                    state_root: entry.expected.state_root,
                    commit_hash: entry.expected.commit_hash,
                })
            }
            (ObservationFrame::RecordedTruth, ObservationAt::Frontier) => {
                let Some(resolved_worldline_tick) = frontier.frontier_tick().checked_sub(1) else {
                    return Err(ObservationError::ObservationUnavailable {
                        worldline_id,
                        at: request.coordinate.at,
                    });
                };
                let entry = provenance
                    .entry(worldline_id, resolved_worldline_tick)
                    .map_err(|_| ObservationError::ObservationUnavailable {
                        worldline_id,
                        at: request.coordinate.at,
                    })?;
                Ok(ResolvedObservationCoordinate {
                    observation_version: OBSERVATION_VERSION,
                    worldline_id,
                    requested_at: request.coordinate.at,
                    resolved_worldline_tick,
                    commit_global_tick: Some(entry.commit_global_tick),
                    observed_after_global_tick: current_cycle_tick(runtime),
                    state_root: entry.expected.state_root,
                    commit_hash: entry.expected.commit_hash,
                })
            }
            (ObservationFrame::RecordedTruth, ObservationAt::Tick(tick)) => {
                let entry = provenance
                    .entry(worldline_id, tick)
                    .map_err(|_| ObservationError::InvalidTick { worldline_id, tick })?;
                Ok(ResolvedObservationCoordinate {
                    observation_version: OBSERVATION_VERSION,
                    worldline_id,
                    requested_at: request.coordinate.at,
                    resolved_worldline_tick: tick,
                    commit_global_tick: Some(entry.commit_global_tick),
                    observed_after_global_tick: current_cycle_tick(runtime),
                    state_root: entry.expected.state_root,
                    commit_hash: entry.expected.commit_hash,
                })
            }
            (ObservationFrame::QueryView, _) => Err(ObservationError::UnsupportedQuery),
        }
    }

    fn resolved_commit_boundary(
        worldline_id: WorldlineId,
        requested_at: ObservationAt,
        resolved_worldline_tick: WorldlineTick,
        commit_global_tick: Option<GlobalTick>,
        latest_cycle_global_tick: GlobalTick,
        snapshot: &Snapshot,
    ) -> ResolvedObservationCoordinate {
        ResolvedObservationCoordinate {
            observation_version: OBSERVATION_VERSION,
            worldline_id,
            requested_at,
            resolved_worldline_tick,
            commit_global_tick,
            observed_after_global_tick: option_cycle_tick(latest_cycle_global_tick),
            state_root: snapshot.state_root,
            commit_hash: snapshot.hash,
        }
    }

    fn compute_artifact_hash(
        resolved: &ResolvedObservationCoordinate,
        reading: &ReadingEnvelope,
        frame: ObservationFrame,
        projection: &ObservationProjection,
        payload: &ObservationPayload,
    ) -> Result<Hash, ObservationError> {
        let input = abi::ObservationHashInput {
            resolved: resolved.to_abi(),
            reading: reading.to_abi(),
            frame: frame.to_abi(),
            projection: projection.to_abi(),
            payload: payload.to_abi(),
        };
        let bytes = echo_wasm_abi::encode_cbor(&input)
            .map_err(|err| ObservationError::CodecFailure(err.to_string()))?;
        let mut hasher = Hasher::new();
        hasher.update(OBSERVATION_ARTIFACT_DOMAIN);
        hasher.update(&bytes);
        Ok(hasher.finalize().into())
    }

    fn reading_envelope(
        resolved: &ResolvedObservationCoordinate,
        parent_basis_posture: ObservationBasisPosture,
        request: &ObservationRequest,
        payload: &ObservationPayload,
    ) -> Result<ReadingEnvelope, ObservationError> {
        let witness_refs = Self::witness_refs(resolved, request.frame);
        let budget_posture = Self::budget_posture(request.budget, payload, witness_refs.len())?;
        Ok(ReadingEnvelope {
            observer_plan: request.observer_plan.clone(),
            observer_instance: request.observer_instance.clone(),
            observer_basis: Self::observer_basis(request.frame),
            witness_refs,
            parent_basis_posture,
            budget_posture,
            rights_posture: Self::rights_posture(request.rights),
            residual_posture: ReadingResidualPosture::Complete,
        })
    }

    fn budget_posture(
        budget: ObservationReadBudget,
        payload: &ObservationPayload,
        witness_ref_count: usize,
    ) -> Result<ReadingBudgetPosture, ObservationError> {
        match budget {
            ObservationReadBudget::UnboundedOneShot => Ok(ReadingBudgetPosture::UnboundedOneShot),
            ObservationReadBudget::Bounded {
                max_payload_bytes,
                max_witness_refs,
            } => {
                let payload_bytes = Self::payload_wire_len(payload)?;
                let witness_refs = witness_ref_count as u64;
                if payload_bytes > max_payload_bytes || witness_refs > max_witness_refs {
                    return Err(ObservationError::BudgetExceeded {
                        max_payload_bytes,
                        payload_bytes,
                        max_witness_refs,
                        witness_refs,
                    });
                }
                Ok(ReadingBudgetPosture::Bounded {
                    max_payload_bytes,
                    payload_bytes,
                    max_witness_refs,
                    witness_refs,
                })
            }
        }
    }

    fn payload_wire_len(payload: &ObservationPayload) -> Result<u64, ObservationError> {
        let bytes = echo_wasm_abi::encode_cbor(&payload.to_abi())
            .map_err(|err| ObservationError::CodecFailure(err.to_string()))?;
        Ok(bytes.len() as u64)
    }

    fn rights_posture(rights: ObservationRights) -> ReadingRightsPosture {
        match rights {
            ObservationRights::KernelPublic => ReadingRightsPosture::KernelPublic,
            ObservationRights::CapabilityScoped { .. } => {
                debug_assert!(
                    false,
                    "capability-scoped observation rights must be rejected before reading"
                );
                ReadingRightsPosture::KernelPublic
            }
        }
    }

    fn observer_plan(
        frame: ObservationFrame,
        projection: ObservationProjectionKind,
    ) -> ReadingObserverPlan {
        builtin_observer_plan_for(frame, projection)
    }

    fn builtin_observer_plan(
        frame: ObservationFrame,
        projection: ObservationProjectionKind,
    ) -> BuiltinObserverPlan {
        match (frame, projection) {
            (ObservationFrame::CommitBoundary, ObservationProjectionKind::Head) => {
                BuiltinObserverPlan::CommitBoundaryHead
            }
            (ObservationFrame::CommitBoundary, ObservationProjectionKind::Snapshot) => {
                BuiltinObserverPlan::CommitBoundarySnapshot
            }
            (ObservationFrame::RecordedTruth, ObservationProjectionKind::TruthChannels) => {
                BuiltinObserverPlan::RecordedTruthChannels
            }
            (ObservationFrame::QueryView, ObservationProjectionKind::Query) => {
                BuiltinObserverPlan::QueryBytes
            }
            _ => {
                debug_assert!(
                    false,
                    "observer_plan requires a valid frame/projection pair"
                );
                BuiltinObserverPlan::QueryBytes
            }
        }
    }

    fn observer_basis(frame: ObservationFrame) -> ReadingObserverBasis {
        match frame {
            ObservationFrame::CommitBoundary => ReadingObserverBasis::CommitBoundary,
            ObservationFrame::RecordedTruth => ReadingObserverBasis::RecordedTruth,
            ObservationFrame::QueryView => ReadingObserverBasis::QueryView,
        }
    }

    fn witness_refs(
        resolved: &ResolvedObservationCoordinate,
        frame: ObservationFrame,
    ) -> Vec<ReadingWitnessRef> {
        let Some(commit_tick) = Self::witness_commit_tick(resolved, frame) else {
            return vec![ReadingWitnessRef::EmptyFrontier {
                worldline_id: resolved.worldline_id,
                state_root: resolved.state_root,
                commit_hash: resolved.commit_hash,
            }];
        };
        vec![ReadingWitnessRef::ResolvedCommit {
            reference: ProvenanceRef {
                worldline_id: resolved.worldline_id,
                worldline_tick: commit_tick,
                commit_hash: resolved.commit_hash,
            },
        }]
    }

    fn witness_commit_tick(
        resolved: &ResolvedObservationCoordinate,
        frame: ObservationFrame,
    ) -> Option<WorldlineTick> {
        resolved.commit_global_tick?;
        match (frame, resolved.requested_at) {
            (ObservationFrame::CommitBoundary, ObservationAt::Frontier) => {
                resolved.resolved_worldline_tick.checked_sub(1)
            }
            _ => Some(resolved.resolved_worldline_tick),
        }
    }

    fn basis_posture(
        runtime: &WorldlineRuntime,
        provenance: &ProvenanceService,
        worldline_id: WorldlineId,
        at: ObservationAt,
    ) -> Result<ObservationBasisPosture, ObservationError> {
        let Some(strand) = runtime.strands().find_by_child_worldline(&worldline_id) else {
            return Ok(ObservationBasisPosture::Worldline);
        };
        if !matches!(at, ObservationAt::Frontier) {
            return Ok(ObservationBasisPosture::StrandHistorical {
                strand_id: strand.strand_id,
            });
        }

        let report = strand
            .live_basis_report(provenance)
            .map_err(|_| ObservationError::ObservationUnavailable { worldline_id, at })?;
        Ok(match report.parent_revalidation {
            StrandRevalidationState::AtAnchor => ObservationBasisPosture::StrandAtAnchor {
                strand_id: strand.strand_id,
            },
            StrandRevalidationState::ParentAdvancedDisjoint {
                parent_from,
                parent_to,
            } => ObservationBasisPosture::StrandParentAdvancedDisjoint {
                strand_id: strand.strand_id,
                parent_from,
                parent_to,
            },
            StrandRevalidationState::RevalidationRequired {
                parent_from,
                parent_to,
                overlapping_slots,
            } => ObservationBasisPosture::StrandRevalidationRequired {
                strand_id: strand.strand_id,
                parent_from,
                parent_to,
                overlapping_slots,
            },
        })
    }
}

fn provenance_ref_to_abi(reference: crate::provenance_store::ProvenanceRef) -> abi::ProvenanceRef {
    abi::ProvenanceRef {
        worldline_id: abi::WorldlineId::from_bytes(*reference.worldline_id.as_bytes()),
        worldline_tick: abi::WorldlineTick(reference.worldline_tick.as_u64()),
        commit_hash: reference.commit_hash.to_vec(),
    }
}

fn observer_plan_id_to_abi(plan_id: ObserverPlanId) -> abi::ObserverPlanId {
    abi::ObserverPlanId::from_bytes(*plan_id.as_bytes())
}

fn observer_instance_id_to_abi(instance_id: ObserverInstanceId) -> abi::ObserverInstanceId {
    abi::ObserverInstanceId::from_bytes(*instance_id.as_bytes())
}

fn overlap_slots_digest(slots: &[SlotId]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(b"echo:observation-overlap-slots:v1\0");
    hasher.update(&(slots.len() as u64).to_le_bytes());
    for slot in slots {
        hash_slot(&mut hasher, slot);
    }
    hasher.finalize().into()
}

fn optic_witness_refs_hash(refs: &[ReadingWitnessRef]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(OPTIC_OBSERVATION_WITNESS_SET_DOMAIN);
    hasher.update(&(refs.len() as u64).to_le_bytes());
    for reference in refs {
        match reference {
            ReadingWitnessRef::ResolvedCommit { reference } => {
                hasher.update(&[1]);
                hash_provenance_ref(&mut hasher, *reference);
            }
            ReadingWitnessRef::EmptyFrontier {
                worldline_id,
                state_root,
                commit_hash,
            } => {
                hasher.update(&[2]);
                hasher.update(worldline_id.as_bytes());
                hasher.update(state_root);
                hasher.update(commit_hash);
            }
        }
    }
    hasher.finalize().into()
}

fn optic_tail_witness_refs_hash(refs: &[ProvenanceRef]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(OPTIC_LIVE_TAIL_WITNESS_SET_DOMAIN);
    hasher.update(&(refs.len() as u64).to_le_bytes());
    for reference in refs {
        hash_provenance_ref(&mut hasher, *reference);
    }
    hasher.finalize().into()
}

fn hash_provenance_ref(hasher: &mut Hasher, reference: ProvenanceRef) {
    hasher.update(reference.worldline_id.as_bytes());
    hasher.update(&reference.worldline_tick.as_u64().to_le_bytes());
    hasher.update(&reference.commit_hash);
}

fn hash_slot(hasher: &mut Hasher, slot: &SlotId) {
    match slot {
        SlotId::Node(node) => {
            hasher.update(&[1]);
            hasher.update(node.warp_id.as_bytes());
            hasher.update(node.local_id.as_bytes());
        }
        SlotId::Edge(edge) => {
            hasher.update(&[2]);
            hasher.update(edge.warp_id.as_bytes());
            hasher.update(edge.local_id.as_bytes());
        }
        SlotId::Attachment(attachment) => {
            hasher.update(&[3]);
            match attachment.owner {
                AttachmentOwner::Node(node) => {
                    hasher.update(&[1]);
                    hasher.update(node.warp_id.as_bytes());
                    hasher.update(node.local_id.as_bytes());
                }
                AttachmentOwner::Edge(edge) => {
                    hasher.update(&[2]);
                    hasher.update(edge.warp_id.as_bytes());
                    hasher.update(edge.local_id.as_bytes());
                }
            }
            match attachment.plane {
                AttachmentPlane::Alpha => hasher.update(&[1]),
                AttachmentPlane::Beta => hasher.update(&[2]),
            };
        }
        SlotId::Port((warp_id, port_key)) => {
            hasher.update(&[4]);
            hasher.update(warp_id.as_bytes());
            hasher.update(&port_key.to_le_bytes());
        }
    }
}

fn option_cycle_tick(tick: GlobalTick) -> Option<GlobalTick> {
    (tick != GlobalTick::ZERO).then_some(tick)
}

fn current_cycle_tick(runtime: &WorldlineRuntime) -> Option<GlobalTick> {
    option_cycle_tick(runtime.global_tick())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::coordinator::WorldlineRuntime;
    use crate::head::{make_head_id, WriterHead, WriterHeadKey};
    use crate::head_inbox::{make_intent_kind, InboxPolicy, IngressEnvelope, IngressTarget};
    use crate::ident::{make_edge_id, make_node_id, make_type_id, WarpId};
    use crate::materialization::make_channel_id;
    use crate::optic::{
        AttachmentDescentPolicy, CoordinateAt, EchoCoordinate, MissingWitnessBasisReason,
        ObserveOpticRequest, ObserveOpticResult, OpticAperture, OpticApertureShape,
        OpticCapabilityId, OpticFocus, OpticId, OpticObstructionKind, OpticReadBudget,
        ProjectionVersion, WitnessBasis,
    };
    use crate::provenance_store::replay_artifacts_for_entry;
    use crate::receipt::TickReceipt;
    use crate::record::{EdgeRecord, NodeRecord};
    use crate::snapshot::compute_commit_hash_v2;
    use crate::strand::{make_strand_id, BaseRef, Strand};
    use crate::tick_patch::{SlotId, TickCommitStatus, WarpOp, WarpTickPatchV1};
    use crate::worldline::{HashTriplet, WorldlineTickHeaderV1, WorldlineTickPatchV1};
    use crate::{
        EngineBuilder, GraphStore, PlaybackMode, ProvenanceEntry, SchedulerCoordinator,
        WorldlineState,
    };

    fn wl(n: u8) -> WorldlineId {
        WorldlineId::from_bytes([n; 32])
    }

    fn wt(raw: u64) -> WorldlineTick {
        WorldlineTick::from_raw(raw)
    }

    fn gt(raw: u64) -> GlobalTick {
        GlobalTick::from_raw(raw)
    }

    fn optic_request(
        worldline_id: WorldlineId,
        shape: OpticApertureShape,
        max_bytes: Option<u64>,
    ) -> ObserveOpticRequest {
        ObserveOpticRequest {
            optic_id: OpticId::from_bytes([70; 32]),
            focus: OpticFocus::Worldline { worldline_id },
            coordinate: EchoCoordinate::Worldline {
                worldline_id,
                at: CoordinateAt::Frontier,
            },
            aperture: OpticAperture {
                shape,
                budget: OpticReadBudget {
                    max_bytes,
                    max_nodes: Some(8),
                    max_ticks: Some(1),
                    max_attachments: Some(0),
                },
                attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
            },
            projection_version: ProjectionVersion::from_raw(1),
            reducer_version: None,
            capability: OpticCapabilityId::from_bytes([71; 32]),
        }
    }

    fn authored_observer_plan() -> ReadingObserverPlan {
        ReadingObserverPlan::Authored {
            plan: Box::new(AuthoredObserverPlan {
                plan_id: ObserverPlanId::from_bytes([80; 32]),
                artifact_hash: [81; 32],
                schema_hash: [82; 32],
                state_schema_hash: [83; 32],
                update_law_hash: [84; 32],
                emission_law_hash: [85; 32],
            }),
        }
    }

    fn empty_runtime_fixture() -> (Engine, WorldlineRuntime, ProvenanceService, WorldlineId) {
        let mut store = GraphStore::default();
        let root = make_node_id("root");
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );

        let engine = EngineBuilder::new(store, root).workers(1).build();
        let default_worldline = WorldlineId::from_bytes(engine.root_key().warp_id.0);
        let mut runtime = WorldlineRuntime::new();
        let default_state = WorldlineState::try_from(engine.state().clone()).unwrap();
        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(default_worldline, &default_state)
            .unwrap();
        runtime
            .register_worldline(default_worldline, default_state)
            .unwrap();
        runtime
            .register_writer_head(WriterHead::with_routing(
                WriterHeadKey {
                    worldline_id: default_worldline,
                    head_id: make_head_id("default"),
                },
                PlaybackMode::Play,
                InboxPolicy::AcceptAll,
                None,
                true,
            ))
            .unwrap();
        (engine, runtime, provenance, default_worldline)
    }

    fn one_commit_fixture() -> (Engine, WorldlineRuntime, ProvenanceService, WorldlineId) {
        let (mut engine, mut runtime, mut provenance, worldline_id) = empty_runtime_fixture();
        runtime
            .ingest(IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                make_intent_kind("echo.intent/test"),
                b"hello".to_vec(),
            ))
            .unwrap();
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
        (engine, runtime, provenance, worldline_id)
    }

    fn append_local_patch(
        state: &mut WorldlineState,
        provenance: &mut ProvenanceService,
        worldline_id: WorldlineId,
        head_key: WriterHeadKey,
        commit_global_tick: GlobalTick,
        patch: WorldlineTickPatchV1,
    ) -> ProvenanceEntry {
        let worldline_tick = state.current_tick();
        let parents = provenance
            .tip_ref(worldline_id)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        patch.apply_to_worldline_state(state).unwrap();
        let state_root = state.state_root();
        let parent_hashes = parents
            .iter()
            .map(|parent| parent.commit_hash)
            .collect::<Vec<_>>();
        let commit_hash = compute_commit_hash_v2(
            &state_root,
            &parent_hashes,
            &patch.patch_digest,
            patch.policy_id(),
        );
        let entry = ProvenanceEntry::local_commit(
            worldline_id,
            worldline_tick,
            commit_global_tick,
            head_key,
            parents,
            HashTriplet {
                state_root,
                patch_digest: patch.patch_digest,
                commit_hash,
            },
            patch,
            Vec::new(),
            Vec::new(),
        );
        provenance.append_local_commit(entry.clone()).unwrap();
        let patch_ref = entry.patch.as_ref().unwrap();
        let (snapshot, receipt, replay_patch) =
            replay_artifacts_for_entry(*state.root(), &entry, patch_ref).unwrap();
        state.record_replayed_tick(snapshot, receipt, replay_patch, Vec::new());
        entry
    }

    fn node_upsert_patch(
        state: &WorldlineState,
        label: &str,
        commit_global_tick: GlobalTick,
    ) -> WorldlineTickPatchV1 {
        let root = *state.root();
        let node = crate::ident::NodeKey {
            warp_id: root.warp_id,
            local_id: make_node_id(label),
        };
        let edge_id = make_edge_id(&format!("root-to-{label}"));
        let edge = crate::ident::EdgeKey {
            warp_id: root.warp_id,
            local_id: edge_id,
        };
        let replay_patch = WarpTickPatchV1::new(
            crate::POLICY_ID_NO_POLICY_V0,
            crate::blake3_empty(),
            TickCommitStatus::Committed,
            vec![SlotId::Node(root)],
            vec![SlotId::Node(node), SlotId::Edge(edge)],
            vec![
                WarpOp::UpsertNode {
                    node,
                    record: NodeRecord {
                        ty: make_type_id("observation-node"),
                    },
                },
                WarpOp::UpsertEdge {
                    warp_id: root.warp_id,
                    record: EdgeRecord {
                        id: edge_id,
                        from: root.local_id,
                        to: node.local_id,
                        ty: make_type_id("observation-edge"),
                    },
                },
            ],
        );
        WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                commit_global_tick,
                policy_id: replay_patch.policy_id(),
                rule_pack_id: replay_patch.rule_pack_id(),
                plan_digest: crate::blake3_empty(),
                decision_digest: crate::blake3_empty(),
                rewrites_digest: crate::blake3_empty(),
            },
            warp_id: root.warp_id,
            ops: replay_patch.ops().to_vec(),
            in_slots: replay_patch.in_slots().to_vec(),
            out_slots: replay_patch.out_slots().to_vec(),
            patch_digest: replay_patch.digest(),
        }
    }

    #[derive(Clone, Copy)]
    enum ParentDrift {
        None,
        Disjoint,
        Overlap,
    }

    fn strand_observation_fixture(
        parent_drift: ParentDrift,
    ) -> (
        Engine,
        WorldlineRuntime,
        ProvenanceService,
        crate::strand::StrandId,
        WorldlineId,
    ) {
        let base_worldline = wl(21);
        let child_worldline = wl(22);
        let mut base_store = GraphStore::new(crate::ident::make_warp_id("observation-root"));
        let root_node = make_node_id("root");
        base_store.insert_node(
            root_node,
            NodeRecord {
                ty: make_type_id("root"),
            },
        );
        let engine = EngineBuilder::new(base_store.clone(), root_node)
            .workers(1)
            .build();
        let mut base_state = WorldlineState::from_root_store(base_store, root_node).unwrap();
        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(base_worldline, &base_state)
            .unwrap();

        let base_head = WriterHeadKey {
            worldline_id: base_worldline,
            head_id: make_head_id("base-head"),
        };
        let child_head = WriterHeadKey {
            worldline_id: child_worldline,
            head_id: make_head_id("child-head"),
        };
        let base_patch = node_upsert_patch(&base_state, "base-node", gt(1));
        let base_entry = append_local_patch(
            &mut base_state,
            &mut provenance,
            base_worldline,
            base_head,
            gt(1),
            base_patch,
        );

        provenance
            .fork(base_worldline, wt(0), child_worldline)
            .unwrap();
        let mut child_state = provenance
            .replay_worldline_state(base_worldline, &base_state)
            .unwrap();

        match parent_drift {
            ParentDrift::None => {}
            ParentDrift::Disjoint => {
                let drift_patch = node_upsert_patch(&base_state, "parent-only", gt(2));
                append_local_patch(
                    &mut base_state,
                    &mut provenance,
                    base_worldline,
                    base_head,
                    gt(2),
                    drift_patch,
                );
            }
            ParentDrift::Overlap => {
                let drift_patch = node_upsert_patch(&base_state, "child-node", gt(2));
                append_local_patch(
                    &mut base_state,
                    &mut provenance,
                    base_worldline,
                    base_head,
                    gt(2),
                    drift_patch,
                );
            }
        }

        let child_patch = node_upsert_patch(&child_state, "child-node", gt(3));
        append_local_patch(
            &mut child_state,
            &mut provenance,
            child_worldline,
            child_head,
            gt(3),
            child_patch,
        );

        let strand_id = make_strand_id("observation-strand");
        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(base_worldline, base_state)
            .unwrap();
        runtime
            .register_worldline(child_worldline, child_state)
            .unwrap();
        for key in [base_head, child_head] {
            runtime
                .register_writer_head(WriterHead::with_routing(
                    key,
                    PlaybackMode::Play,
                    InboxPolicy::AcceptAll,
                    None,
                    true,
                ))
                .unwrap();
        }
        runtime
            .register_strand(Strand {
                strand_id,
                base_ref: BaseRef {
                    source_worldline_id: base_worldline,
                    fork_tick: wt(0),
                    commit_hash: base_entry.expected.commit_hash,
                    boundary_hash: base_entry.expected.state_root,
                    provenance_ref: base_entry.as_ref(),
                },
                child_worldline_id: child_worldline,
                writer_heads: vec![child_head],
                support_pins: Vec::new(),
            })
            .unwrap();

        (engine, runtime, provenance, strand_id, child_worldline)
    }

    fn recorded_truth_fixture() -> (Engine, WorldlineRuntime, ProvenanceService, WorldlineId) {
        let mut store = GraphStore::default();
        let root = make_node_id("root");
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );
        let engine = EngineBuilder::new(store, root).workers(1).build();
        let worldline_id = wl(7);
        let mut state = WorldlineState::empty();
        let snapshot = engine.snapshot_for_state(&state);
        state.tick_history.push((
            snapshot.clone(),
            TickReceipt::new(snapshot.tx, Vec::new(), Vec::new()),
            WarpTickPatchV1::new(
                crate::POLICY_ID_NO_POLICY_V0,
                crate::blake3_empty(),
                TickCommitStatus::Committed,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        ));
        state.last_snapshot = Some(snapshot.clone());
        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(worldline_id, state.clone())
            .unwrap();
        runtime
            .register_writer_head(WriterHead::with_routing(
                WriterHeadKey {
                    worldline_id,
                    head_id: make_head_id("default"),
                },
                PlaybackMode::Play,
                InboxPolicy::AcceptAll,
                None,
                true,
            ))
            .unwrap();
        let mut provenance = ProvenanceService::new();
        provenance.register_worldline(worldline_id, &state).unwrap();
        let channel = make_channel_id("test:truth");
        provenance
            .append_local_commit(ProvenanceEntry::local_commit(
                worldline_id,
                wt(0),
                gt(1),
                WriterHeadKey {
                    worldline_id,
                    head_id: make_head_id("default"),
                },
                Vec::new(),
                HashTriplet {
                    state_root: snapshot.state_root,
                    patch_digest: snapshot.patch_digest,
                    commit_hash: snapshot.hash,
                },
                WorldlineTickPatchV1 {
                    header: WorldlineTickHeaderV1 {
                        commit_global_tick: gt(1),
                        policy_id: crate::POLICY_ID_NO_POLICY_V0,
                        rule_pack_id: crate::blake3_empty(),
                        plan_digest: snapshot.plan_digest,
                        decision_digest: snapshot.decision_digest,
                        rewrites_digest: snapshot.rewrites_digest,
                    },
                    warp_id: WarpId(root.0),
                    ops: Vec::new(),
                    in_slots: Vec::new(),
                    out_slots: Vec::new(),
                    patch_digest: snapshot.patch_digest,
                },
                vec![(channel, b"truth".to_vec())],
                Vec::new(),
            ))
            .unwrap();
        (engine, runtime, provenance, worldline_id)
    }

    #[test]
    fn validity_matrix_accepts_only_centralized_pairs() {
        let truth = ObservationProjection::TruthChannels { channels: None };
        let query = ObservationProjection::Query {
            query_id: 7,
            vars_bytes: Vec::new(),
        };

        assert!(ObservationService::validate_frame_projection(
            ObservationFrame::CommitBoundary,
            &ObservationProjection::Head,
        )
        .is_ok());
        assert!(ObservationService::validate_frame_projection(
            ObservationFrame::CommitBoundary,
            &ObservationProjection::Snapshot,
        )
        .is_ok());
        assert!(ObservationService::validate_frame_projection(
            ObservationFrame::RecordedTruth,
            &truth,
        )
        .is_ok());
        assert!(
            ObservationService::validate_frame_projection(ObservationFrame::QueryView, &query,)
                .is_ok()
        );

        assert!(matches!(
            ObservationService::validate_frame_projection(
                ObservationFrame::RecordedTruth,
                &ObservationProjection::Head,
            ),
            Err(ObservationError::UnsupportedFrameProjection { .. })
        ));
        assert!(matches!(
            ObservationService::validate_frame_projection(ObservationFrame::CommitBoundary, &truth,),
            Err(ObservationError::UnsupportedFrameProjection { .. })
        ));
    }

    #[test]
    fn frontier_head_matches_live_frontier_snapshot() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let artifact = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest::builtin_one_shot(
                ObservationCoordinate {
                    worldline_id,
                    at: ObservationAt::Frontier,
                },
                ObservationFrame::CommitBoundary,
                ObservationProjection::Head,
            ),
        )
        .unwrap();

        let frontier = runtime.worldlines().get(&worldline_id).unwrap();
        let snapshot = frontier
            .state()
            .last_snapshot()
            .cloned()
            .unwrap_or_else(|| engine.snapshot_for_state(frontier.state()));
        assert_eq!(
            artifact.resolved.resolved_worldline_tick,
            frontier.frontier_tick()
        );
        assert_eq!(artifact.resolved.commit_global_tick, Some(gt(1)));
        assert_eq!(artifact.resolved.observed_after_global_tick, Some(gt(1)));
        assert_eq!(artifact.resolved.state_root, snapshot.state_root);
        assert_eq!(artifact.resolved.commit_hash, snapshot.hash);
    }

    #[test]
    fn recorded_truth_frontier_without_commits_is_unavailable() {
        let (engine, runtime, provenance, worldline_id) = empty_runtime_fixture();
        let err = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest::builtin_one_shot(
                ObservationCoordinate {
                    worldline_id,
                    at: ObservationAt::Frontier,
                },
                ObservationFrame::RecordedTruth,
                ObservationProjection::TruthChannels { channels: None },
            ),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ObservationError::ObservationUnavailable {
                worldline_id,
                at: ObservationAt::Frontier,
            }
        );
    }

    #[test]
    fn recorded_truth_reads_recorded_outputs_only() {
        let (engine, runtime, provenance, worldline_id) = recorded_truth_fixture();
        let artifact = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest::builtin_one_shot(
                ObservationCoordinate {
                    worldline_id,
                    at: ObservationAt::Frontier,
                },
                ObservationFrame::RecordedTruth,
                ObservationProjection::TruthChannels { channels: None },
            ),
        )
        .unwrap();
        let channels = if let ObservationPayload::TruthChannels(channels) = artifact.payload {
            channels
        } else {
            Vec::new()
        };
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].1, b"truth".to_vec());
    }

    #[test]
    fn identical_requests_produce_stable_artifact_hashes() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let request = ObservationRequest::builtin_one_shot(
            ObservationCoordinate {
                worldline_id,
                at: ObservationAt::Frontier,
            },
            ObservationFrame::CommitBoundary,
            ObservationProjection::Head,
        );
        let first =
            ObservationService::observe(&runtime, &provenance, &engine, request.clone()).unwrap();
        let second = ObservationService::observe(&runtime, &provenance, &engine, request).unwrap();
        assert_eq!(first.artifact_hash, second.artifact_hash);
        assert_eq!(first.to_abi(), second.to_abi());
    }

    #[test]
    fn ordinary_worldline_observation_reports_worldline_posture() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let artifact = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest::builtin_one_shot(
                ObservationCoordinate {
                    worldline_id,
                    at: ObservationAt::Frontier,
                },
                ObservationFrame::CommitBoundary,
                ObservationProjection::Head,
            ),
        )
        .unwrap();

        assert_eq!(
            artifact.reading.observer_plan,
            ReadingObserverPlan::Builtin {
                plan: BuiltinObserverPlan::CommitBoundaryHead
            }
        );
        assert_eq!(
            artifact.reading.observer_basis,
            ReadingObserverBasis::CommitBoundary
        );
        assert_eq!(
            artifact.reading.parent_basis_posture,
            ObservationBasisPosture::Worldline
        );
        assert_eq!(
            artifact.reading.budget_posture,
            ReadingBudgetPosture::UnboundedOneShot
        );
        assert_eq!(
            artifact.reading.rights_posture,
            ReadingRightsPosture::KernelPublic
        );
        assert_eq!(
            artifact.reading.residual_posture,
            ReadingResidualPosture::Complete
        );
        assert!(matches!(
            artifact.reading.witness_refs.as_slice(),
            [ReadingWitnessRef::ResolvedCommit { .. }]
        ));
        if let [ReadingWitnessRef::ResolvedCommit { reference }] =
            artifact.reading.witness_refs.as_slice()
        {
            assert_eq!(reference.worldline_id, worldline_id);
            assert_eq!(reference.worldline_tick, wt(0));
            assert_eq!(reference.commit_hash, artifact.resolved.commit_hash);
        }
        assert_eq!(
            artifact.to_abi().reading.parent_basis_posture,
            abi::ObservationBasisPosture::Worldline
        );
        assert_eq!(
            artifact.to_abi().reading.residual_posture,
            abi::ReadingResidualPosture::Complete
        );
    }

    #[test]
    fn explicit_bounded_observer_request_returns_bounded_reading_artifact() -> Result<(), String> {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let mut request = ObservationRequest::builtin_one_shot(
            ObservationCoordinate {
                worldline_id,
                at: ObservationAt::Frontier,
            },
            ObservationFrame::CommitBoundary,
            ObservationProjection::Head,
        );
        request.budget = ObservationReadBudget::Bounded {
            max_payload_bytes: 512,
            max_witness_refs: 1,
        };

        let artifact = ObservationService::observe(&runtime, &provenance, &engine, request.clone())
            .map_err(|err| err.to_string())?;

        assert_eq!(artifact.reading.observer_plan, request.observer_plan);
        assert_eq!(artifact.reading.observer_instance, None);
        assert!(matches!(
            artifact.reading.budget_posture,
            ReadingBudgetPosture::Bounded {
                max_payload_bytes: 512,
                payload_bytes: 1..=512,
                max_witness_refs: 1,
                witness_refs: 1,
            }
        ));
        assert!(matches!(
            request.to_abi().budget,
            abi::ObservationReadBudget::Bounded {
                max_payload_bytes: 512,
                max_witness_refs: 1,
            }
        ));

        Ok(())
    }

    #[test]
    fn authored_observer_plan_obstructs_without_hidden_builtin_fallback() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let mut request = ObservationRequest::builtin_one_shot(
            ObservationCoordinate {
                worldline_id,
                at: ObservationAt::Frontier,
            },
            ObservationFrame::CommitBoundary,
            ObservationProjection::Head,
        );
        let authored = authored_observer_plan();
        request.observer_plan = authored.clone();

        let err = ObservationService::observe(&runtime, &provenance, &engine, request).unwrap_err();

        assert_eq!(err, ObservationError::UnsupportedObserverPlan(authored));
    }

    #[test]
    fn hosted_observer_instance_obstructs_without_stateful_fallback() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let mut request = ObservationRequest::builtin_one_shot(
            ObservationCoordinate {
                worldline_id,
                at: ObservationAt::Frontier,
            },
            ObservationFrame::CommitBoundary,
            ObservationProjection::Head,
        );
        let instance = ObserverInstanceRef {
            instance_id: ObserverInstanceId::from_bytes([86; 32]),
            plan_id: ObserverPlanId::from_bytes([80; 32]),
            state_hash: [87; 32],
        };
        request.observer_instance = Some(instance.clone());

        let err = ObservationService::observe(&runtime, &provenance, &engine, request).unwrap_err();

        assert_eq!(err, ObservationError::UnsupportedObserverInstance(instance));
    }

    #[test]
    fn capability_scoped_observer_rights_obstruct_without_public_fallback() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let mut request = ObservationRequest::builtin_one_shot(
            ObservationCoordinate {
                worldline_id,
                at: ObservationAt::Frontier,
            },
            ObservationFrame::CommitBoundary,
            ObservationProjection::Head,
        );
        let rights = ObservationRights::CapabilityScoped {
            capability: OpticCapabilityId::from_bytes([88; 32]),
        };
        request.rights = rights;

        let err = ObservationService::observe(&runtime, &provenance, &engine, request).unwrap_err();

        assert_eq!(err, ObservationError::UnsupportedRights(rights));
    }

    #[test]
    fn observation_budget_obstructs_instead_of_emitting_oversized_reading() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let mut request = ObservationRequest::builtin_one_shot(
            ObservationCoordinate {
                worldline_id,
                at: ObservationAt::Frontier,
            },
            ObservationFrame::CommitBoundary,
            ObservationProjection::Head,
        );
        request.budget = ObservationReadBudget::Bounded {
            max_payload_bytes: 1,
            max_witness_refs: 1,
        };

        let err = ObservationService::observe(&runtime, &provenance, &engine, request).unwrap_err();

        assert!(matches!(
            err,
            ObservationError::BudgetExceeded {
                max_payload_bytes: 1,
                payload_bytes: 2..,
                max_witness_refs: 1,
                witness_refs: 1,
            }
        ));
    }

    #[test]
    fn bounded_head_optic_returns_read_identity() -> Result<(), String> {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let request = optic_request(worldline_id, OpticApertureShape::Head, Some(256));
        let reading = match ObservationService::observe_optic(
            &runtime,
            &provenance,
            &engine,
            request.clone(),
        ) {
            ObserveOpticResult::Reading(reading) => reading,
            ObserveOpticResult::Obstructed(obstruction) => {
                return Err(format!("expected optic reading, got {obstruction:?}"));
            }
        };

        assert_eq!(reading.read_identity.optic_id, request.optic_id);
        assert_eq!(reading.read_identity.coordinate, request.coordinate);
        assert_eq!(
            reading.read_identity.aperture_digest,
            request.aperture.digest()
        );
        assert!(matches!(
            reading.read_identity.witness_basis,
            WitnessBasis::ResolvedCommit { .. }
        ));
        assert!(matches!(reading.payload, ObservationPayload::Head(_)));
        assert_eq!(reading.retained, None);
        assert_eq!(
            reading.to_abi().read_identity.read_identity_hash,
            reading.read_identity.read_identity_hash.to_vec()
        );

        Ok(())
    }

    #[test]
    fn bounded_snapshot_optic_returns_read_identity() -> Result<(), String> {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let request = optic_request(
            worldline_id,
            OpticApertureShape::SnapshotMetadata,
            Some(256),
        );
        let reading = match ObservationService::observe_optic(
            &runtime,
            &provenance,
            &engine,
            request.clone(),
        ) {
            ObserveOpticResult::Reading(reading) => reading,
            ObserveOpticResult::Obstructed(obstruction) => {
                return Err(format!("expected optic reading, got {obstruction:?}"));
            }
        };

        assert_eq!(reading.read_identity.optic_id, request.optic_id);
        assert_eq!(reading.read_identity.focus_digest, request.focus.digest());
        assert!(matches!(reading.payload, ObservationPayload::Snapshot(_)));
        assert!(matches!(
            reading.read_identity.witness_basis,
            WitnessBasis::ResolvedCommit { .. }
        ));

        Ok(())
    }

    #[test]
    fn oversized_optic_aperture_returns_budget_obstruction() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let request = optic_request(
            worldline_id,
            OpticApertureShape::ByteRange {
                start: 0,
                len: 4096,
            },
            Some(1024),
        );

        let result = ObservationService::observe_optic(&runtime, &provenance, &engine, request);

        assert!(matches!(
            result,
            ObserveOpticResult::Obstructed(ref obstruction)
                if obstruction.kind == OpticObstructionKind::BudgetExceeded
                    && matches!(
                        obstruction.witness_basis,
                        Some(WitnessBasis::Missing {
                            reason: MissingWitnessBasisReason::BudgetLimited,
                        })
                    )
        ));
    }

    #[test]
    fn reading_residual_postures_convert_to_abi() {
        let cases = [
            (
                ReadingResidualPosture::Complete,
                abi::ReadingResidualPosture::Complete,
            ),
            (
                ReadingResidualPosture::Residual,
                abi::ReadingResidualPosture::Residual,
            ),
            (
                ReadingResidualPosture::PluralityPreserved,
                abi::ReadingResidualPosture::PluralityPreserved,
            ),
            (
                ReadingResidualPosture::Obstructed,
                abi::ReadingResidualPosture::Obstructed,
            ),
        ];

        for (posture, expected) in cases {
            assert_eq!(posture.to_abi(), expected);
        }
    }

    #[test]
    fn strand_frontier_observation_reports_anchor_posture() {
        let (engine, runtime, provenance, strand_id, child_worldline) =
            strand_observation_fixture(ParentDrift::None);
        let artifact = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest::builtin_one_shot(
                ObservationCoordinate {
                    worldline_id: child_worldline,
                    at: ObservationAt::Frontier,
                },
                ObservationFrame::CommitBoundary,
                ObservationProjection::Snapshot,
            ),
        )
        .unwrap();

        assert_eq!(
            artifact.reading.parent_basis_posture,
            ObservationBasisPosture::StrandAtAnchor { strand_id }
        );
    }

    #[test]
    fn strand_frontier_observation_reports_disjoint_live_basis_posture() {
        let (anchor_engine, anchor_runtime, anchor_provenance, _, anchor_child) =
            strand_observation_fixture(ParentDrift::None);
        let (engine, runtime, provenance, strand_id, child_worldline) =
            strand_observation_fixture(ParentDrift::Disjoint);

        let anchor_artifact = ObservationService::observe(
            &anchor_runtime,
            &anchor_provenance,
            &anchor_engine,
            ObservationRequest::builtin_one_shot(
                ObservationCoordinate {
                    worldline_id: anchor_child,
                    at: ObservationAt::Frontier,
                },
                ObservationFrame::CommitBoundary,
                ObservationProjection::Snapshot,
            ),
        )
        .unwrap();
        let artifact = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest::builtin_one_shot(
                ObservationCoordinate {
                    worldline_id: child_worldline,
                    at: ObservationAt::Frontier,
                },
                ObservationFrame::CommitBoundary,
                ObservationProjection::Snapshot,
            ),
        )
        .unwrap();

        assert!(matches!(
            artifact.reading.parent_basis_posture,
            ObservationBasisPosture::StrandParentAdvancedDisjoint {
                strand_id: observed_strand,
                ..
            } if observed_strand == strand_id
        ));
        assert_eq!(
            artifact.resolved.state_root,
            anchor_artifact.resolved.state_root
        );
        assert_ne!(artifact.artifact_hash, anchor_artifact.artifact_hash);
    }

    #[test]
    fn strand_frontier_observation_reports_overlap_revalidation_posture() {
        let (engine, runtime, provenance, strand_id, child_worldline) =
            strand_observation_fixture(ParentDrift::Overlap);
        let artifact = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest::builtin_one_shot(
                ObservationCoordinate {
                    worldline_id: child_worldline,
                    at: ObservationAt::Frontier,
                },
                ObservationFrame::CommitBoundary,
                ObservationProjection::Snapshot,
            ),
        )
        .unwrap();

        assert!(
            matches!(
                artifact.reading.parent_basis_posture,
                ObservationBasisPosture::StrandRevalidationRequired { .. }
            ),
            "expected revalidation-gated strand posture"
        );
        if let ObservationBasisPosture::StrandRevalidationRequired {
            strand_id: observed_strand,
            overlapping_slots,
            ..
        } = &artifact.reading.parent_basis_posture
        {
            assert_eq!(*observed_strand, strand_id);
            assert!(!overlapping_slots.is_empty());
        }

        assert!(matches!(
            artifact.to_abi().reading.parent_basis_posture,
            abi::ObservationBasisPosture::StrandRevalidationRequired {
                overlapping_slot_count,
                ref overlapping_slots_digest,
                ..
            } if overlapping_slot_count > 0 && overlapping_slots_digest.len() == 32
        ));
    }

    #[test]
    fn observation_is_zero_write_for_runtime_and_provenance() {
        let (engine, runtime, provenance, worldline_id) = one_commit_fixture();
        let runtime_before = runtime.clone();
        let provenance_before = provenance.clone();

        let artifact = ObservationService::observe(
            &runtime,
            &provenance,
            &engine,
            ObservationRequest::builtin_one_shot(
                ObservationCoordinate {
                    worldline_id,
                    at: ObservationAt::Tick(wt(0)),
                },
                ObservationFrame::CommitBoundary,
                ObservationProjection::Snapshot,
            ),
        )
        .unwrap();

        assert_eq!(artifact.resolved.resolved_worldline_tick, wt(0));
        assert_eq!(artifact.resolved.commit_global_tick, Some(gt(1)));
        assert_eq!(artifact.resolved.observed_after_global_tick, Some(gt(1)));
        assert_eq!(
            provenance
                .entry(worldline_id, wt(0))
                .unwrap()
                .commit_global_tick,
            gt(1)
        );
        let frontier_after = runtime.worldlines().get(&worldline_id).unwrap();
        let frontier_before = runtime_before.worldlines().get(&worldline_id).unwrap();
        assert_eq!(runtime.global_tick(), runtime_before.global_tick());
        assert_eq!(
            frontier_after.frontier_tick(),
            frontier_before.frontier_tick()
        );
        assert_eq!(
            frontier_after.state().current_tick(),
            frontier_before.state().current_tick()
        );
        assert_eq!(
            frontier_after
                .state()
                .last_snapshot()
                .map(|snapshot| snapshot.hash),
            frontier_before
                .state()
                .last_snapshot()
                .map(|snapshot| snapshot.hash)
        );
        assert_eq!(
            provenance.len(worldline_id).unwrap(),
            provenance_before.len(worldline_id).unwrap()
        );
        assert_eq!(
            provenance.entry(worldline_id, wt(0)).unwrap(),
            provenance_before.entry(worldline_id, wt(0)).unwrap()
        );
    }
}
