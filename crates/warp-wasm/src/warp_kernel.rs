// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Engine-backed [`KernelPort`] implementation.
//!
//! [`WarpKernel`] wraps a `warp-core::Engine` and translates its typed API
//! into the byte-level contract expected by the WASM boundary. This module
//! is gated behind the `engine` feature.

use std::fmt;

use echo_wasm_abi::kernel_port::{
    error_codes, AbiError, AttachmentDescentPolicy as AbiAttachmentDescentPolicy,
    AttachmentKey as AbiAttachmentKey, AttachmentOwnerRef as AbiAttachmentOwnerRef,
    AttachmentPlane as AbiAttachmentPlane, AuthoredObserverPlan as AbiAuthoredObserverPlan,
    BraidId as AbiBraidId, ControlIntentV1, CoordinateAt as AbiCoordinateAt, DispatchResponse,
    EchoCoordinate as AbiEchoCoordinate, GlobalTick as AbiGlobalTick,
    HeadEligibility as AbiHeadEligibility, HeadId as AbiHeadId, HeadInfo, KernelPort,
    NeighborhoodCore as AbiNeighborhoodCore, NeighborhoodSite as AbiNeighborhoodSite,
    ObservationArtifact as AbiObservationArtifact, ObservationFrame as AbiObservationFrame,
    ObservationProjection as AbiObservationProjection,
    ObservationReadBudget as AbiObservationReadBudget, ObservationRequest as AbiObservationRequest,
    ObservationRights as AbiObservationRights, ObserveOpticRequest as AbiObserveOpticRequest,
    ObserveOpticResult as AbiObserveOpticResult, ObserverInstanceRef as AbiObserverInstanceRef,
    OpticAperture as AbiOpticAperture, OpticApertureShape as AbiOpticApertureShape,
    OpticFocus as AbiOpticFocus, ProjectionVersion as AbiProjectionVersion,
    ReadingObserverPlan as AbiReadingObserverPlan, ReducerVersion as AbiReducerVersion,
    RegistryInfo, RetainedReadingKey as AbiRetainedReadingKey, RunCompletion, RunId as AbiRunId,
    SchedulerMode, SchedulerState, SchedulerStatus, SettlementDelta as AbiSettlementDelta,
    SettlementPlan as AbiSettlementPlan, SettlementRequest as AbiSettlementRequest,
    SettlementResult as AbiSettlementResult, WorkState, WorldlineId as AbiWorldlineId,
    WorldlineTick as AbiWorldlineTick, WriterHeadKey as AbiWriterHeadKey, ABI_VERSION,
};
use echo_wasm_abi::{
    unpack_control_intent_v1, unpack_import_suffix_intent_v1, unpack_intent_v1,
    CONTROL_INTENT_V1_OP_ID, IMPORT_SUFFIX_INTENT_V1_OP_ID,
};
use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, AttachmentDescentPolicy,
    AttachmentKey, AttachmentOwner, AttachmentPlane, AuthoredObserverPlan, BraidId, CoordinateAt,
    EchoCoordinate, EdgeKey, Engine, EngineBuilder, EngineError, GlobalTick, GraphStore,
    HeadEligibility, HeadId, HistoryError, IngressDisposition, IngressEnvelope, IngressTarget,
    NeighborhoodError, NeighborhoodSiteService, NodeKey, NodeRecord, ObservationAt,
    ObservationCoordinate, ObservationError, ObservationFrame, ObservationPayload,
    ObservationProjection, ObservationReadBudget, ObservationRequest, ObservationRights,
    ObservationService, ObserveOpticRequest, ObserverInstanceId, ObserverInstanceRef,
    ObserverPlanId, OpticAperture, OpticApertureShape, OpticCapabilityId, OpticFocus,
    OpticReadBudget, PlaybackMode, ProjectionVersion, ProvenanceRef, ProvenanceService,
    ReadingObserverPlan, ReducerVersion, RetainedReadingKey, RunId, RuntimeError,
    SchedulerCoordinator, SchedulerKind, SettlementError, SettlementService, StrandId, TypeId,
    WorldlineId, WorldlineRuntime, WorldlineState, WorldlineStateError, WorldlineTick, WriterHead,
    WriterHeadKey,
};

/// Error returned when a [`WarpKernel`] cannot be initialized from a caller-supplied engine.
#[derive(Debug)]
pub enum KernelInitError {
    /// The supplied engine has already advanced and cannot seed a fresh runtime.
    NonFreshEngine,
    /// The engine's backing state does not satisfy [`WorldlineState`] invariants.
    WorldlineState(WorldlineStateError),
    /// Provenance registration failed while installing the default worldline.
    Provenance(HistoryError),
    /// Runtime registration failed while installing the default worldline/head.
    Runtime(RuntimeError),
    /// Kernel-owned command rule registration failed.
    Engine(EngineError),
}

impl fmt::Display for KernelInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFreshEngine => write!(f, "WarpKernel::with_engine requires a fresh engine"),
            Self::WorldlineState(err) => err.fmt(f),
            Self::Provenance(err) => err.fmt(f),
            Self::Runtime(err) => err.fmt(f),
            Self::Engine(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for KernelInitError {}

impl From<WorldlineStateError> for KernelInitError {
    fn from(value: WorldlineStateError) -> Self {
        Self::WorldlineState(value)
    }
}

impl From<RuntimeError> for KernelInitError {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

impl From<HistoryError> for KernelInitError {
    fn from(value: HistoryError) -> Self {
        Self::Provenance(value)
    }
}

impl From<EngineError> for KernelInitError {
    fn from(value: EngineError) -> Self {
        Self::Engine(value)
    }
}

/// App-agnostic kernel wrapping a `warp-core::Engine`.
///
/// Constructed via [`WarpKernel::new`] (default empty engine) or
/// [`WarpKernel::with_engine`] (pre-configured engine with rules).
pub struct WarpKernel {
    engine: Engine,
    runtime: WorldlineRuntime,
    provenance: ProvenanceService,
    default_worldline: WorldlineId,
    scheduler_status: SchedulerStatus,
    next_run_id: RunId,
    /// Registry metadata (injected at construction, immutable after).
    registry: RegistryInfo,
}

impl WarpKernel {
    /// Create a new kernel with a minimal empty engine.
    ///
    /// The engine starts with a single root node; [`Self::with_engine`]
    /// installs the generic Echo command rules used by the boundary.
    pub fn new() -> Result<Self, KernelInitError> {
        let mut store = GraphStore::default();
        let root = make_node_id("root");
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );

        let engine = EngineBuilder::new(store, root)
            .scheduler(SchedulerKind::Radix)
            .workers(1) // WASM is single-threaded
            .build();
        Self::with_engine(
            engine,
            RegistryInfo {
                codec_id: Some("cbor-canonical-v1".into()),
                registry_version: None,
                schema_sha256_hex: None,
                abi_version: ABI_VERSION,
            },
        )
    }

    /// Create a kernel with a pre-configured engine and registry metadata.
    ///
    /// Use this to inject app-specific rewrite rules and schema metadata.
    ///
    /// The engine must be fresh: `WarpKernel` can mirror graph state into the
    /// default worldline runtime, but it cannot reconstruct prior tick history
    /// or materialization state from an already-advanced engine.
    pub fn with_engine(
        mut engine: Engine,
        registry: RegistryInfo,
    ) -> Result<Self, KernelInitError> {
        if !engine.is_fresh_runtime_state() {
            return Err(KernelInitError::NonFreshEngine);
        }
        engine.register_rule(warp_core::import_suffix_intent_rule())?;
        let root = engine.root_key();
        let default_worldline = WorldlineId::from_bytes(root.warp_id.0);
        let mut runtime = WorldlineRuntime::new();
        let default_state = WorldlineState::try_from(engine.state().clone())?;
        let mut provenance = ProvenanceService::new();
        provenance.register_worldline(default_worldline, &default_state)?;
        runtime.register_worldline(default_worldline, default_state)?;
        runtime.register_writer_head(WriterHead::with_routing(
            WriterHeadKey {
                worldline_id: default_worldline,
                head_id: make_head_id("default"),
            },
            PlaybackMode::Play,
            warp_core::InboxPolicy::AcceptAll,
            None,
            true,
        ))?;

        Ok(Self {
            engine,
            runtime,
            provenance,
            default_worldline,
            scheduler_status: SchedulerStatus {
                state: SchedulerState::Inactive,
                active_mode: None,
                work_state: WorkState::Quiescent,
                run_id: None,
                latest_cycle_global_tick: None,
                latest_commit_global_tick: None,
                last_quiescent_global_tick: None,
                last_run_completion: None,
            },
            next_run_id: RunId::from_raw(1),
            registry,
        })
    }

    fn to_core_worldline_id(worldline_id: &AbiWorldlineId) -> WorldlineId {
        WorldlineId::from_bytes(*worldline_id.as_bytes())
    }

    fn to_core_head_id(head_id: &AbiHeadId) -> HeadId {
        HeadId::from_bytes(*head_id.as_bytes())
    }

    fn to_core_strand_id(strand_id: &echo_wasm_abi::kernel_port::StrandId) -> StrandId {
        StrandId::from_bytes(*strand_id.as_bytes())
    }

    fn parse_channel_ids(
        channels: Option<&Vec<Vec<u8>>>,
    ) -> Result<Option<Vec<warp_core::TypeId>>, AbiError> {
        channels
            .map(|ids| {
                ids.iter()
                    .map(|bytes| {
                        let hash: [u8; 32] = bytes.as_slice().try_into().map_err(|_| AbiError {
                            code: error_codes::INVALID_PAYLOAD,
                            message: format!(
                                "channel id must be exactly 32 bytes, got {}",
                                bytes.len()
                            ),
                        })?;
                        Ok(warp_core::TypeId(hash))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
    }

    fn map_observation_error(err: ObservationError) -> AbiError {
        match err {
            ObservationError::InvalidWorldline(worldline_id) => AbiError {
                code: error_codes::INVALID_WORLDLINE,
                message: format!("invalid worldline: {worldline_id:?}"),
            },
            ObservationError::InvalidTick { worldline_id, tick } => AbiError {
                code: error_codes::INVALID_TICK,
                message: format!("invalid tick {tick} for worldline {worldline_id:?}"),
            },
            ObservationError::UnsupportedFrameProjection { frame, projection } => AbiError {
                code: error_codes::UNSUPPORTED_FRAME_PROJECTION,
                message: format!(
                    "unsupported frame/projection pairing: {frame:?} + {projection:?}"
                ),
            },
            ObservationError::UnsupportedQuery => AbiError {
                code: error_codes::UNSUPPORTED_QUERY,
                message: "query observation is not supported by this kernel".into(),
            },
            ObservationError::UnsupportedObserverPlan(plan) => AbiError {
                code: error_codes::UNSUPPORTED_OBSERVER_PLAN,
                message: format!("unsupported observer plan: {plan:?}"),
            },
            ObservationError::UnsupportedObserverInstance(instance) => AbiError {
                code: error_codes::UNSUPPORTED_OBSERVER_INSTANCE,
                message: format!("unsupported observer instance: {instance:?}"),
            },
            ObservationError::UnsupportedRights(rights) => AbiError {
                code: error_codes::UNSUPPORTED_OBSERVATION_RIGHTS,
                message: format!("unsupported observation rights posture: {rights:?}"),
            },
            ObservationError::BudgetExceeded {
                max_payload_bytes,
                payload_bytes,
                max_witness_refs,
                witness_refs,
            } => AbiError {
                code: error_codes::OBSERVATION_BUDGET_EXCEEDED,
                message: format!(
                    "observation budget exceeded: payload {payload_bytes}/{max_payload_bytes} bytes, witness refs {witness_refs}/{max_witness_refs}"
                ),
            },
            ObservationError::ObservationUnavailable { worldline_id, at } => AbiError {
                code: error_codes::OBSERVATION_UNAVAILABLE,
                message: format!(
                    "observation unavailable for worldline {worldline_id:?} at {at:?}"
                ),
            },
            ObservationError::CodecFailure(message) => AbiError {
                code: error_codes::CODEC_ERROR,
                message,
            },
        }
    }

    fn map_neighborhood_error(err: NeighborhoodError) -> AbiError {
        match err {
            NeighborhoodError::Observation(observation_error) => {
                Self::map_observation_error(observation_error)
            }
            NeighborhoodError::InvalidSupportPin { .. }
            | NeighborhoodError::MissingSupportStrand { .. } => AbiError {
                code: error_codes::OBSERVATION_UNAVAILABLE,
                message: err.to_string(),
            },
        }
    }

    fn map_settlement_error(err: SettlementError) -> AbiError {
        match err {
            SettlementError::StrandNotFound(strand_id) => AbiError {
                code: error_codes::INVALID_STRAND,
                message: format!("invalid strand: {strand_id:?}"),
            },
            _ => AbiError {
                code: error_codes::ENGINE_ERROR,
                message: err.to_string(),
            },
        }
    }

    fn to_core_request(request: AbiObservationRequest) -> Result<ObservationRequest, AbiError> {
        let worldline_id = Self::to_core_worldline_id(&request.coordinate.worldline_id);
        let at = match request.coordinate.at {
            echo_wasm_abi::kernel_port::ObservationAt::Frontier => ObservationAt::Frontier,
            echo_wasm_abi::kernel_port::ObservationAt::Tick { worldline_tick } => {
                ObservationAt::Tick(WorldlineTick::from_raw(worldline_tick.0))
            }
        };
        let frame = match request.frame {
            AbiObservationFrame::CommitBoundary => ObservationFrame::CommitBoundary,
            AbiObservationFrame::RecordedTruth => ObservationFrame::RecordedTruth,
            AbiObservationFrame::QueryView => ObservationFrame::QueryView,
        };
        let projection = match request.projection {
            AbiObservationProjection::Head => ObservationProjection::Head,
            AbiObservationProjection::Snapshot => ObservationProjection::Snapshot,
            AbiObservationProjection::TruthChannels { channels } => {
                ObservationProjection::TruthChannels {
                    channels: Self::parse_channel_ids(channels.as_ref())?,
                }
            }
            AbiObservationProjection::Query {
                query_id,
                vars_bytes,
            } => ObservationProjection::Query {
                query_id,
                vars_bytes,
            },
        };
        let observer_plan = Self::to_core_observer_plan(request.observer_plan)?;
        let observer_instance = request
            .observer_instance
            .map(Self::to_core_observer_instance)
            .transpose()?;
        let budget = Self::to_core_observation_budget(request.budget);
        let rights = Self::to_core_observation_rights(request.rights);
        Ok(ObservationRequest {
            coordinate: ObservationCoordinate { worldline_id, at },
            frame,
            projection,
            observer_plan,
            observer_instance,
            budget,
            rights,
        })
    }

    fn to_core_observe_optic_request(
        request: AbiObserveOpticRequest,
    ) -> Result<ObserveOpticRequest, AbiError> {
        Ok(ObserveOpticRequest {
            optic_id: warp_core::OpticId::from_bytes(*request.optic_id.as_bytes()),
            focus: Self::to_core_optic_focus(request.focus)?,
            coordinate: Self::to_core_echo_coordinate(request.coordinate)?,
            aperture: Self::to_core_optic_aperture(request.aperture)?,
            projection_version: Self::to_core_projection_version(request.projection_version),
            reducer_version: request.reducer_version.map(Self::to_core_reducer_version),
            capability: OpticCapabilityId::from_bytes(*request.capability.as_bytes()),
        })
    }

    fn to_core_optic_focus(focus: AbiOpticFocus) -> Result<OpticFocus, AbiError> {
        Ok(match focus {
            AbiOpticFocus::Worldline { worldline_id } => OpticFocus::Worldline {
                worldline_id: Self::to_core_worldline_id(&worldline_id),
            },
            AbiOpticFocus::Strand { strand_id } => OpticFocus::Strand {
                strand_id: Self::to_core_strand_id(&strand_id),
            },
            AbiOpticFocus::Braid { braid_id } => OpticFocus::Braid {
                braid_id: Self::to_core_braid_id(&braid_id),
            },
            AbiOpticFocus::RetainedReading { key } => OpticFocus::RetainedReading {
                key: Self::to_core_retained_reading_key(&key),
            },
            AbiOpticFocus::AttachmentBoundary { key } => OpticFocus::AttachmentBoundary {
                key: Self::to_core_attachment_key(key)?,
            },
        })
    }

    fn to_core_echo_coordinate(coordinate: AbiEchoCoordinate) -> Result<EchoCoordinate, AbiError> {
        Ok(match coordinate {
            AbiEchoCoordinate::Worldline { worldline_id, at } => EchoCoordinate::Worldline {
                worldline_id: Self::to_core_worldline_id(&worldline_id),
                at: Self::to_core_coordinate_at(at)?,
            },
            AbiEchoCoordinate::Strand {
                strand_id,
                at,
                parent_basis,
            } => EchoCoordinate::Strand {
                strand_id: Self::to_core_strand_id(&strand_id),
                at: Self::to_core_coordinate_at(at)?,
                parent_basis: parent_basis.map(Self::to_core_provenance_ref).transpose()?,
            },
            AbiEchoCoordinate::Braid {
                braid_id,
                projection_digest,
                member_count,
            } => EchoCoordinate::Braid {
                braid_id: Self::to_core_braid_id(&braid_id),
                projection_digest: Self::hash_from_vec(
                    "braid projection digest",
                    projection_digest,
                )?,
                member_count,
            },
            AbiEchoCoordinate::RetainedReading { key } => EchoCoordinate::RetainedReading {
                key: Self::to_core_retained_reading_key(&key),
            },
        })
    }

    fn to_core_coordinate_at(at: AbiCoordinateAt) -> Result<CoordinateAt, AbiError> {
        Ok(match at {
            AbiCoordinateAt::Frontier => CoordinateAt::Frontier,
            AbiCoordinateAt::Tick { worldline_tick } => {
                CoordinateAt::Tick(WorldlineTick::from_raw(worldline_tick.0))
            }
            AbiCoordinateAt::Provenance { reference } => {
                CoordinateAt::Provenance(Self::to_core_provenance_ref(reference)?)
            }
        })
    }

    fn to_core_provenance_ref(
        reference: echo_wasm_abi::kernel_port::ProvenanceRef,
    ) -> Result<ProvenanceRef, AbiError> {
        Ok(ProvenanceRef {
            worldline_id: Self::to_core_worldline_id(&reference.worldline_id),
            worldline_tick: WorldlineTick::from_raw(reference.worldline_tick.0),
            commit_hash: Self::hash_from_vec("provenance commit hash", reference.commit_hash)?,
        })
    }

    fn to_core_optic_aperture(aperture: AbiOpticAperture) -> Result<OpticAperture, AbiError> {
        Ok(OpticAperture {
            shape: Self::to_core_optic_aperture_shape(aperture.shape)?,
            budget: OpticReadBudget {
                max_bytes: aperture.budget.max_bytes,
                max_nodes: aperture.budget.max_nodes,
                max_ticks: aperture.budget.max_ticks,
                max_attachments: aperture.budget.max_attachments,
            },
            attachment_descent: match aperture.attachment_descent {
                AbiAttachmentDescentPolicy::BoundaryOnly => AttachmentDescentPolicy::BoundaryOnly,
                AbiAttachmentDescentPolicy::Explicit => AttachmentDescentPolicy::Explicit,
            },
        })
    }

    fn to_core_optic_aperture_shape(
        shape: AbiOpticApertureShape,
    ) -> Result<OpticApertureShape, AbiError> {
        Ok(match shape {
            AbiOpticApertureShape::Head => OpticApertureShape::Head,
            AbiOpticApertureShape::SnapshotMetadata => OpticApertureShape::SnapshotMetadata,
            AbiOpticApertureShape::TruthChannels { channels } => {
                OpticApertureShape::TruthChannels {
                    channels: channels.map(|ids| {
                        ids.into_iter()
                            .map(|id| TypeId(*id.as_bytes()))
                            .collect::<Vec<_>>()
                    }),
                }
            }
            AbiOpticApertureShape::QueryBytes {
                query_id,
                vars_digest,
            } => OpticApertureShape::QueryBytes {
                query_id,
                vars_digest: Self::hash_from_vec("optic query vars digest", vars_digest)?,
            },
            AbiOpticApertureShape::ByteRange { start, len } => {
                OpticApertureShape::ByteRange { start, len }
            }
            AbiOpticApertureShape::AttachmentBoundary => OpticApertureShape::AttachmentBoundary,
        })
    }

    fn to_core_projection_version(version: AbiProjectionVersion) -> ProjectionVersion {
        ProjectionVersion::from_raw(version.0)
    }

    fn to_core_reducer_version(version: AbiReducerVersion) -> ReducerVersion {
        ReducerVersion::from_raw(version.0)
    }

    fn to_core_braid_id(id: &AbiBraidId) -> BraidId {
        BraidId::from_bytes(*id.as_bytes())
    }

    fn to_core_retained_reading_key(id: &AbiRetainedReadingKey) -> RetainedReadingKey {
        RetainedReadingKey::from_bytes(*id.as_bytes())
    }

    fn to_core_attachment_key(key: AbiAttachmentKey) -> Result<AttachmentKey, AbiError> {
        let owner = match key.owner {
            AbiAttachmentOwnerRef::Node { warp_id, node_id } => AttachmentOwner::Node(NodeKey {
                warp_id: warp_core::WarpId(*warp_id.as_bytes()),
                local_id: warp_core::NodeId(*node_id.as_bytes()),
            }),
            AbiAttachmentOwnerRef::Edge { warp_id, edge_id } => AttachmentOwner::Edge(EdgeKey {
                warp_id: warp_core::WarpId(*warp_id.as_bytes()),
                local_id: warp_core::EdgeId(*edge_id.as_bytes()),
            }),
        };
        let plane = match key.plane {
            AbiAttachmentPlane::Alpha => AttachmentPlane::Alpha,
            AbiAttachmentPlane::Beta => AttachmentPlane::Beta,
        };
        let key = AttachmentKey { owner, plane };
        if !key.is_plane_valid() {
            return Err(AbiError {
                code: error_codes::INVALID_PAYLOAD,
                message: "attachment key plane does not match owner kind".into(),
            });
        }
        Ok(key)
    }

    fn to_core_observer_plan(
        plan: AbiReadingObserverPlan,
    ) -> Result<ReadingObserverPlan, AbiError> {
        match plan {
            AbiReadingObserverPlan::Builtin { plan } => Ok(ReadingObserverPlan::Builtin {
                plan: match plan {
                    echo_wasm_abi::kernel_port::BuiltinObserverPlan::CommitBoundaryHead => {
                        warp_core::BuiltinObserverPlan::CommitBoundaryHead
                    }
                    echo_wasm_abi::kernel_port::BuiltinObserverPlan::CommitBoundarySnapshot => {
                        warp_core::BuiltinObserverPlan::CommitBoundarySnapshot
                    }
                    echo_wasm_abi::kernel_port::BuiltinObserverPlan::RecordedTruthChannels => {
                        warp_core::BuiltinObserverPlan::RecordedTruthChannels
                    }
                    echo_wasm_abi::kernel_port::BuiltinObserverPlan::QueryBytes => {
                        warp_core::BuiltinObserverPlan::QueryBytes
                    }
                },
            }),
            AbiReadingObserverPlan::Authored { plan } => Ok(ReadingObserverPlan::Authored {
                plan: Box::new(Self::to_core_authored_observer_plan(*plan)?),
            }),
        }
    }

    fn to_core_authored_observer_plan(
        plan: AbiAuthoredObserverPlan,
    ) -> Result<AuthoredObserverPlan, AbiError> {
        Ok(AuthoredObserverPlan {
            plan_id: ObserverPlanId::from_bytes(*plan.plan_id.as_bytes()),
            artifact_hash: Self::hash_from_vec("observer artifact hash", plan.artifact_hash)?,
            schema_hash: Self::hash_from_vec("observer schema hash", plan.schema_hash)?,
            state_schema_hash: Self::hash_from_vec(
                "observer state schema hash",
                plan.state_schema_hash,
            )?,
            update_law_hash: Self::hash_from_vec("observer update law hash", plan.update_law_hash)?,
            emission_law_hash: Self::hash_from_vec(
                "observer emission law hash",
                plan.emission_law_hash,
            )?,
        })
    }

    fn to_core_observer_instance(
        instance: AbiObserverInstanceRef,
    ) -> Result<ObserverInstanceRef, AbiError> {
        Ok(ObserverInstanceRef {
            instance_id: ObserverInstanceId::from_bytes(*instance.instance_id.as_bytes()),
            plan_id: ObserverPlanId::from_bytes(*instance.plan_id.as_bytes()),
            state_hash: Self::hash_from_vec("observer state hash", instance.state_hash)?,
        })
    }

    fn to_core_observation_budget(budget: AbiObservationReadBudget) -> ObservationReadBudget {
        match budget {
            AbiObservationReadBudget::UnboundedOneShot => ObservationReadBudget::UnboundedOneShot,
            AbiObservationReadBudget::Bounded {
                max_payload_bytes,
                max_witness_refs,
            } => ObservationReadBudget::Bounded {
                max_payload_bytes,
                max_witness_refs,
            },
        }
    }

    fn to_core_observation_rights(rights: AbiObservationRights) -> ObservationRights {
        match rights {
            AbiObservationRights::KernelPublic => ObservationRights::KernelPublic,
            AbiObservationRights::CapabilityScoped { capability } => {
                ObservationRights::CapabilityScoped {
                    capability: OpticCapabilityId::from_bytes(*capability.as_bytes()),
                }
            }
        }
    }

    fn hash_from_vec(label: &str, bytes: Vec<u8>) -> Result<warp_core::Hash, AbiError> {
        bytes.try_into().map_err(|bytes: Vec<u8>| AbiError {
            code: error_codes::INVALID_PAYLOAD,
            message: format!("{label} must be exactly 32 bytes, got {}", bytes.len()),
        })
    }

    fn observe_core(
        &self,
        request: ObservationRequest,
    ) -> Result<warp_core::ObservationArtifact, AbiError> {
        ObservationService::observe(&self.runtime, &self.provenance, &self.engine, request)
            .map_err(Self::map_observation_error)
    }

    pub(crate) fn current_head(&self) -> Result<HeadInfo, AbiError> {
        let request = ObservationRequest::builtin_one_shot(
            ObservationCoordinate {
                worldline_id: self.default_worldline,
                at: ObservationAt::Frontier,
            },
            ObservationFrame::CommitBoundary,
            ObservationProjection::Head,
        )
        .map_err(Self::map_observation_error)?;
        Self::head_info_from_observation(self.observe_core(request)?)
    }

    fn head_info_from_observation(
        artifact: warp_core::ObservationArtifact,
    ) -> Result<HeadInfo, AbiError> {
        match artifact.payload {
            ObservationPayload::Head(head) => Ok(HeadInfo {
                worldline_tick: AbiWorldlineTick(head.worldline_tick.as_u64()),
                commit_global_tick: head
                    .commit_global_tick
                    .map(|tick| AbiGlobalTick(tick.as_u64())),
                state_root: head.state_root.to_vec(),
                commit_id: head.commit_hash.to_vec(),
            }),
            _ => Err(AbiError {
                code: error_codes::ENGINE_ERROR,
                message: "observe returned non-head payload for head adapter".into(),
            }),
        }
    }

    fn option_abi_global_tick(tick: GlobalTick) -> Option<AbiGlobalTick> {
        (tick != GlobalTick::ZERO).then_some(AbiGlobalTick(tick.as_u64()))
    }

    fn current_work_state(&self) -> WorkState {
        let mut has_pending = false;
        let mut has_runnable = false;

        for (_, head) in self.runtime.heads().iter() {
            if !head.inbox().is_empty() {
                has_pending = true;
            }
            if head.is_admitted() && !head.is_paused() && head.inbox().can_admit() {
                has_runnable = true;
            }
        }

        if has_runnable {
            WorkState::RunnablePending
        } else if has_pending {
            WorkState::BlockedOnly
        } else {
            WorkState::Quiescent
        }
    }

    fn refresh_scheduler_status(&mut self) {
        self.scheduler_status.work_state = self.current_work_state();
        self.scheduler_status.latest_cycle_global_tick =
            Self::option_abi_global_tick(self.runtime.global_tick());
    }

    fn clear_active_run_state(&mut self, clear_run_id: bool) {
        self.scheduler_status.state = SchedulerState::Inactive;
        self.scheduler_status.active_mode = None;
        if clear_run_id {
            self.scheduler_status.run_id = None;
        }
        self.refresh_scheduler_status();
    }

    fn apply_control_intent(&mut self, intent: ControlIntentV1) -> Result<(), AbiError> {
        match intent {
            ControlIntentV1::Start {
                mode: SchedulerMode::UntilIdle { cycle_limit },
            } => {
                if matches!(
                    self.scheduler_status.state,
                    SchedulerState::Running | SchedulerState::Stopping
                ) {
                    return Err(AbiError {
                        code: error_codes::INVALID_CONTROL,
                        message: "scheduler is already active".into(),
                    });
                }
                if matches!(cycle_limit, Some(0)) {
                    return Err(AbiError {
                        code: error_codes::INVALID_CONTROL,
                        message: "cycle_limit must be non-zero when present".into(),
                    });
                }

                let run_id = self.next_run_id;
                self.next_run_id =
                    self.next_run_id
                        .checked_increment()
                        .ok_or_else(|| AbiError {
                            code: error_codes::ENGINE_ERROR,
                            message: "run id overflow".into(),
                        })?;
                self.scheduler_status.state = SchedulerState::Running;
                self.scheduler_status.active_mode = Some(SchedulerMode::UntilIdle { cycle_limit });
                self.scheduler_status.run_id = Some(AbiRunId(run_id.as_u64()));
                self.scheduler_status.last_run_completion = None;
                self.refresh_scheduler_status();

                let mut cycles_executed = 0u32;
                loop {
                    let records = match SchedulerCoordinator::super_tick(
                        &mut self.runtime,
                        &mut self.provenance,
                        &mut self.engine,
                    ) {
                        Ok(records) => records,
                        Err(error) => {
                            self.clear_active_run_state(true);
                            return Err(AbiError {
                                code: error_codes::ENGINE_ERROR,
                                message: error.to_string(),
                            });
                        }
                    };

                    self.refresh_scheduler_status();
                    if !records.is_empty() {
                        self.scheduler_status.latest_commit_global_tick =
                            Self::option_abi_global_tick(self.runtime.global_tick());
                    }
                    cycles_executed = cycles_executed.saturating_add(1);

                    if self.scheduler_status.work_state == WorkState::Quiescent {
                        self.scheduler_status.last_quiescent_global_tick =
                            Self::option_abi_global_tick(self.runtime.global_tick());
                        self.scheduler_status.last_run_completion = Some(RunCompletion::Quiesced);
                        break;
                    }

                    if records.is_empty()
                        && self.scheduler_status.work_state == WorkState::BlockedOnly
                    {
                        self.scheduler_status.last_run_completion =
                            Some(RunCompletion::BlockedOnly);
                        break;
                    }

                    if cycle_limit.is_some_and(|limit| cycles_executed >= limit) {
                        self.scheduler_status.last_run_completion =
                            Some(RunCompletion::CycleLimitReached);
                        break;
                    }
                }

                self.clear_active_run_state(false);
            }
            ControlIntentV1::Stop => {
                if self.scheduler_status.state == SchedulerState::Inactive {
                    self.refresh_scheduler_status();
                    return Ok(());
                }
                self.scheduler_status.last_run_completion = Some(RunCompletion::Stopped);
                self.clear_active_run_state(false);
            }
            ControlIntentV1::SetHeadEligibility { head, eligibility } => {
                let key = Self::to_core_head_key(&head);
                let eligibility = match eligibility {
                    AbiHeadEligibility::Dormant => HeadEligibility::Dormant,
                    AbiHeadEligibility::Admitted => HeadEligibility::Admitted,
                };
                self.runtime
                    .set_head_eligibility(key, eligibility)
                    .map_err(|e| AbiError {
                        code: match e {
                            RuntimeError::UnknownHead(_) => error_codes::INVALID_CONTROL,
                            _ => error_codes::ENGINE_ERROR,
                        },
                        message: e.to_string(),
                    })?;
                self.refresh_scheduler_status();
            }
        }

        Ok(())
    }

    fn to_core_head_key(head: &AbiWriterHeadKey) -> WriterHeadKey {
        WriterHeadKey {
            worldline_id: Self::to_core_worldline_id(&head.worldline_id),
            head_id: Self::to_core_head_id(&head.head_id),
        }
    }
}

impl KernelPort for WarpKernel {
    fn dispatch_intent(&mut self, intent_bytes: &[u8]) -> Result<DispatchResponse, AbiError> {
        let (op_id, _vars) = unpack_intent_v1(intent_bytes).map_err(|e| AbiError {
            code: error_codes::INVALID_INTENT,
            message: format!(
                "malformed EINT envelope ({} bytes): {e}",
                intent_bytes.len()
            ),
        })?;

        let intent_id = if op_id == CONTROL_INTENT_V1_OP_ID {
            IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter {
                    worldline_id: self.default_worldline,
                },
                make_intent_kind("echo.control/eint-v1"),
                intent_bytes.to_vec(),
            )
            .ingress_id()
            .to_vec()
        } else {
            IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter {
                    worldline_id: self.default_worldline,
                },
                make_intent_kind("echo.intent/eint-v1"),
                intent_bytes.to_vec(),
            )
            .ingress_id()
            .to_vec()
        };

        if op_id == CONTROL_INTENT_V1_OP_ID {
            let control = unpack_control_intent_v1(intent_bytes).map_err(|_| AbiError {
                code: error_codes::INVALID_CONTROL,
                message: "invalid control intent envelope".into(),
            })?;
            self.apply_control_intent(control)?;
            return Ok(DispatchResponse {
                accepted: true,
                intent_id,
                scheduler_status: self.scheduler_status()?,
            });
        }

        if op_id == IMPORT_SUFFIX_INTENT_V1_OP_ID {
            unpack_import_suffix_intent_v1(intent_bytes).map_err(|_| AbiError {
                code: error_codes::INVALID_INTENT,
                message: "invalid import suffix intent envelope".into(),
            })?;
        }

        let envelope = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter {
                worldline_id: self.default_worldline,
            },
            make_intent_kind("echo.intent/eint-v1"),
            intent_bytes.to_vec(),
        );

        match self.runtime.ingest(envelope) {
            Ok(disposition) => {
                let accepted = matches!(disposition, IngressDisposition::Accepted { .. });
                self.refresh_scheduler_status();
                Ok(DispatchResponse {
                    accepted,
                    intent_id,
                    scheduler_status: self.scheduler_status()?,
                })
            }
            Err(e) => Err(AbiError {
                code: error_codes::ENGINE_ERROR,
                message: e.to_string(),
            }),
        }
    }

    fn current_optic_coordinate(
        &self,
        focus: &AbiOpticFocus,
    ) -> Result<Option<AbiEchoCoordinate>, AbiError> {
        match focus {
            AbiOpticFocus::Worldline { worldline_id } => {
                if Self::to_core_worldline_id(worldline_id) != self.default_worldline {
                    return Ok(None);
                }

                let head = self.current_head()?;
                Ok(Some(AbiEchoCoordinate::Worldline {
                    worldline_id: *worldline_id,
                    at: AbiCoordinateAt::Tick {
                        worldline_tick: head.worldline_tick,
                    },
                }))
            }
            _ => Ok(None),
        }
    }

    fn observe_optic(
        &self,
        request: AbiObserveOpticRequest,
    ) -> Result<AbiObserveOpticResult, AbiError> {
        let request = Self::to_core_observe_optic_request(request)?;
        Ok(ObservationService::observe_optic(
            &self.runtime,
            &self.provenance,
            &self.engine,
            request,
        )
        .to_abi())
    }

    fn observe(&self, request: AbiObservationRequest) -> Result<AbiObservationArtifact, AbiError> {
        let request = Self::to_core_request(request)?;
        Ok(self.observe_core(request)?.to_abi())
    }

    fn observe_neighborhood_site(
        &self,
        request: AbiObservationRequest,
    ) -> Result<AbiNeighborhoodSite, AbiError> {
        let request = Self::to_core_request(request)?;
        NeighborhoodSiteService::observe(&self.runtime, &self.provenance, &self.engine, request)
            .map(|site| site.to_abi())
            .map_err(Self::map_neighborhood_error)
    }

    fn observe_neighborhood_core(
        &self,
        request: AbiObservationRequest,
    ) -> Result<AbiNeighborhoodCore, AbiError> {
        let request = Self::to_core_request(request)?;
        NeighborhoodSiteService::observe(&self.runtime, &self.provenance, &self.engine, request)
            .map(|site| site.to_core().to_abi())
            .map_err(Self::map_neighborhood_error)
    }

    fn compare_settlement(
        &self,
        request: AbiSettlementRequest,
    ) -> Result<AbiSettlementDelta, AbiError> {
        let strand_id = Self::to_core_strand_id(&request.strand_id);
        SettlementService::compare(&self.runtime, &self.provenance, strand_id)
            .map(|delta| delta.to_abi())
            .map_err(Self::map_settlement_error)
    }

    fn plan_settlement(
        &self,
        request: AbiSettlementRequest,
    ) -> Result<AbiSettlementPlan, AbiError> {
        let strand_id = Self::to_core_strand_id(&request.strand_id);
        SettlementService::plan(&self.runtime, &self.provenance, strand_id)
            .map(|plan| plan.to_abi())
            .map_err(Self::map_settlement_error)
    }

    fn settle_strand(
        &mut self,
        request: AbiSettlementRequest,
    ) -> Result<AbiSettlementResult, AbiError> {
        let strand_id = Self::to_core_strand_id(&request.strand_id);
        SettlementService::settle(&mut self.runtime, &mut self.provenance, strand_id)
            .map(|result| result.to_abi())
            .map_err(Self::map_settlement_error)
    }

    fn registry_info(&self) -> RegistryInfo {
        self.registry.clone()
    }

    fn scheduler_status(&self) -> Result<SchedulerStatus, AbiError> {
        Ok(self.scheduler_status.clone())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use echo_wasm_abi::{
        decode_cbor,
        kernel_port::{
            BuiltinObserverPlan as AbiBuiltinObserverPlan,
            CausalSuffixBundle as AbiCausalSuffixBundle, ControlIntentV1,
            GlobalTick as AbiGlobalTick, HeadEligibility as AbiHeadEligibility,
            HeadId as AbiHeadId, ImportSuffixRequest as AbiImportSuffixRequest,
            ImportSuffixResult as AbiImportSuffixResult, ObservationAt as AbiObservationAt,
            ObservationBasisPosture as AbiObservationBasisPosture,
            ObservationCoordinate as AbiObservationCoordinate,
            ObservationFrame as AbiObservationFrame, ObservationPayload as AbiObservationPayload,
            ObservationProjection as AbiObservationProjection,
            ObservationRequest as AbiObservationRequest, ProvenanceRef as AbiProvenanceRef,
            ReadingBudgetPosture as AbiReadingBudgetPosture,
            ReadingObserverBasis as AbiReadingObserverBasis,
            ReadingObserverPlan as AbiReadingObserverPlan,
            ReadingResidualPosture as AbiReadingResidualPosture,
            ReadingRightsPosture as AbiReadingRightsPosture, RunCompletion, SchedulerMode,
            SchedulerState, SettlementDecision as AbiSettlementDecision,
            SettlementOverlapRevalidation as AbiSettlementOverlapRevalidation,
            SettlementParentRevalidation as AbiSettlementParentRevalidation,
            SettlementRequest as AbiSettlementRequest,
            WitnessedSuffixAdmissionOutcome as AbiWitnessedSuffixAdmissionOutcome,
            WitnessedSuffixShell as AbiWitnessedSuffixShell, WorkState,
            WorldlineId as AbiWorldlineId, WorldlineTick as AbiWorldlineTick,
            WriterHeadKey as AbiWriterHeadKey,
        },
        pack_control_intent_v1, pack_import_suffix_intent_v1, pack_intent_v1,
        IMPORT_SUFFIX_INTENT_V1_OP_ID,
    };
    use warp_core::{
        compute_commit_hash_v2, make_edge_id, make_head_id, make_node_id, make_strand_id,
        make_type_id, make_warp_id, materialization::make_channel_id, AdmissionLawId, CoordinateAt,
        EchoCoordinate, EdgeRecord, ForkBasisRef, GlobalTick, GraphStore, HashTriplet, InboxPolicy,
        IntentFamilyId, NodeId, NodeKey, NodeRecord, OpticActorId, OpticCapabilityId, OpticCause,
        OpticReadBudget, PlaybackMode, ProvenanceEntry, ProvenanceService, ProvenanceStore, SlotId,
        Strand, StrandId, TickCommitStatus, WarpOp, WarpTickPatchV1, WorldlineHeadOptic,
        WorldlineRuntime, WorldlineState, WorldlineTick, WorldlineTickHeaderV1,
        WorldlineTickPatchV1, WriterHead, WriterHeadKey,
    };

    fn start_until_idle(kernel: &mut WarpKernel, cycle_limit: Option<u32>) -> DispatchResponse {
        start_until_idle_result(kernel, cycle_limit).unwrap()
    }

    fn abi_builtin_one_shot(
        coordinate: AbiObservationCoordinate,
        frame: AbiObservationFrame,
        projection: AbiObservationProjection,
    ) -> AbiObservationRequest {
        AbiObservationRequest::builtin_one_shot(coordinate, frame, projection).unwrap()
    }

    fn start_until_idle_result(
        kernel: &mut WarpKernel,
        cycle_limit: Option<u32>,
    ) -> Result<DispatchResponse, AbiError> {
        let control = pack_control_intent_v1(&ControlIntentV1::Start {
            mode: SchedulerMode::UntilIdle { cycle_limit },
        })
        .unwrap();
        kernel.dispatch_intent(&control)
    }

    fn abi_worldline_id(worldline_id: WorldlineId) -> AbiWorldlineId {
        AbiWorldlineId::from_bytes(*worldline_id.as_bytes())
    }

    fn abi_head_id(head_id: HeadId) -> AbiHeadId {
        AbiHeadId::from_bytes(*head_id.as_bytes())
    }

    fn abi_provenance_ref(worldline_id: WorldlineId, tick: u64, seed: u8) -> AbiProvenanceRef {
        AbiProvenanceRef {
            worldline_id: abi_worldline_id(worldline_id),
            worldline_tick: AbiWorldlineTick(tick),
            commit_hash: vec![seed; 32],
        }
    }

    fn sample_import_suffix_request(kernel: &WarpKernel) -> AbiImportSuffixRequest {
        let worldline_id = kernel.default_worldline;
        let base_frontier = abi_provenance_ref(worldline_id, 0, 1);
        let target_frontier = abi_provenance_ref(worldline_id, 1, 2);
        let source_suffix = AbiWitnessedSuffixShell {
            source_worldline_id: abi_worldline_id(worldline_id),
            source_suffix_start_tick: AbiWorldlineTick(1),
            source_suffix_end_tick: Some(AbiWorldlineTick(1)),
            source_entries: vec![target_frontier.clone()],
            boundary_witness: Some(base_frontier.clone()),
            witness_digest: vec![3; 32],
            basis_report: None,
        };

        AbiImportSuffixRequest {
            bundle: AbiCausalSuffixBundle {
                base_frontier,
                target_frontier,
                source_suffix,
                bundle_digest: vec![4; 32],
            },
            target_worldline_id: abi_worldline_id(worldline_id),
            target_basis: abi_provenance_ref(worldline_id, 0, 1),
            basis_report: None,
        }
    }

    fn wl(n: u8) -> WorldlineId {
        WorldlineId::from_bytes([n; 32])
    }

    fn wt(raw: u64) -> WorldlineTick {
        WorldlineTick::from_raw(raw)
    }

    fn gt(raw: u64) -> GlobalTick {
        GlobalTick::from_raw(raw)
    }

    fn register_head(
        runtime: &mut WorldlineRuntime,
        worldline_id: WorldlineId,
        label: &str,
    ) -> WriterHeadKey {
        let key = WriterHeadKey {
            worldline_id,
            head_id: make_head_id(label),
        };
        runtime
            .register_writer_head(WriterHead::with_routing(
                key,
                PlaybackMode::Play,
                InboxPolicy::AcceptAll,
                None,
                true,
            ))
            .unwrap();
        key
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
        entry
    }

    fn node_upsert_patch(
        state: &WorldlineState,
        label: &str,
        commit_global_tick: GlobalTick,
    ) -> WorldlineTickPatchV1 {
        let root = *state.root();
        let node = NodeKey {
            warp_id: root.warp_id,
            local_id: make_node_id(label),
        };
        let edge_id = make_edge_id(&format!("root-to-{label}"));
        let edge = warp_core::EdgeKey {
            warp_id: root.warp_id,
            local_id: edge_id,
        };
        let replay_patch = WarpTickPatchV1::new(
            warp_core::POLICY_ID_NO_POLICY_V0,
            warp_core::blake3_empty(),
            TickCommitStatus::Committed,
            vec![SlotId::Node(root)],
            vec![SlotId::Node(node), SlotId::Edge(edge)],
            vec![
                WarpOp::UpsertNode {
                    node,
                    record: NodeRecord {
                        ty: make_type_id("settlement-node"),
                    },
                },
                WarpOp::UpsertEdge {
                    warp_id: root.warp_id,
                    record: EdgeRecord {
                        id: edge_id,
                        from: root.local_id,
                        to: node.local_id,
                        ty: make_type_id("settlement-edge"),
                    },
                },
            ],
        );
        WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                commit_global_tick,
                policy_id: replay_patch.policy_id(),
                rule_pack_id: replay_patch.rule_pack_id(),
                plan_digest: warp_core::blake3_empty(),
                decision_digest: warp_core::blake3_empty(),
                rewrites_digest: warp_core::blake3_empty(),
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
        OverlapSame,
    }

    fn setup_runtime_with_strand(
        parent_drift: ParentDrift,
    ) -> (
        WorldlineRuntime,
        ProvenanceService,
        StrandId,
        WorldlineId,
        WorldlineId,
    ) {
        let base_worldline = wl(1);
        let child_worldline = wl(2);
        let warp_id = make_warp_id("settlement-root");
        let mut base_store = GraphStore::new(warp_id);
        let root_node = make_node_id("root");
        base_store.insert_node(
            root_node,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );
        let engine = warp_core::EngineBuilder::new(base_store, root_node)
            .workers(1)
            .build();
        let mut base_state = WorldlineState::try_from(engine.state().clone()).unwrap();
        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(base_worldline, &base_state)
            .unwrap();

        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(base_worldline, base_state.clone())
            .unwrap();
        let base_head = register_head(&mut runtime, base_worldline, "base-head");
        let base_patch = node_upsert_patch(&base_state, "base-node", gt(1));
        let base_entry = append_local_patch(
            &mut base_state,
            &mut provenance,
            base_worldline,
            base_head,
            gt(1),
            base_patch,
        );
        base_state = provenance
            .replay_worldline_state(base_worldline, &base_state)
            .unwrap();
        runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(base_worldline, base_state.clone())
            .unwrap();
        register_head(&mut runtime, base_worldline, "base-head");

        provenance
            .fork(base_worldline, wt(0), child_worldline)
            .unwrap();
        let mut child_state = provenance
            .replay_worldline_state(base_worldline, &base_state)
            .unwrap();
        runtime
            .register_worldline(child_worldline, child_state.clone())
            .unwrap();
        let child_head = register_head(&mut runtime, child_worldline, "child-head");
        let strand_id = make_strand_id("kernel-settlement-strand");
        runtime
            .register_strand(Strand {
                strand_id,
                fork_basis_ref: ForkBasisRef {
                    source_lane_id: base_worldline,
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

        let parent_drift_patch = match parent_drift {
            ParentDrift::None => None,
            ParentDrift::Disjoint => Some(node_upsert_patch(&base_state, "base-diverged", gt(2))),
            ParentDrift::OverlapSame => Some(node_upsert_patch(&base_state, "child-node", gt(2))),
        };
        if let Some(diverged_patch) = parent_drift_patch {
            let diverged_head = WriterHeadKey {
                worldline_id: base_worldline,
                head_id: make_head_id("base-head"),
            };
            append_local_patch(
                &mut base_state,
                &mut provenance,
                base_worldline,
                diverged_head,
                gt(2),
                diverged_patch,
            );
            base_state = provenance
                .replay_worldline_state(base_worldline, &base_state)
                .unwrap();
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
        child_state = provenance
            .replay_worldline_state(child_worldline, &child_state)
            .unwrap();

        runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(base_worldline, base_state)
            .unwrap();
        register_head(&mut runtime, base_worldline, "base-head");
        runtime
            .register_worldline(child_worldline, child_state)
            .unwrap();
        register_head(&mut runtime, child_worldline, "child-head");
        runtime
            .register_strand(Strand {
                strand_id,
                fork_basis_ref: ForkBasisRef {
                    source_lane_id: base_worldline,
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
        (
            runtime,
            provenance,
            strand_id,
            base_worldline,
            child_worldline,
        )
    }

    #[test]
    fn new_kernel_has_zero_tick() {
        let kernel = WarpKernel::new().unwrap();
        let head = kernel.current_head().unwrap();
        assert_eq!(head.worldline_tick, AbiWorldlineTick(0));
        assert_eq!(head.commit_global_tick, None);
        assert_eq!(head.state_root.len(), 32);
        assert_eq!(head.commit_id.len(), 32);
    }

    #[test]
    fn worldline_head_optic_example_reads_and_dispatches_through_kernel() {
        let mut kernel = WarpKernel::new().unwrap();
        let worldline_id = kernel.default_worldline;
        let actor = OpticActorId::from_bytes([41; 32]);
        let optic = WorldlineHeadOptic::open(
            worldline_id,
            CoordinateAt::Frontier,
            actor,
            OpticCapabilityId::from_bytes([42; 32]),
            IntentFamilyId::from_bytes([43; 32]),
            [44; 32],
        )
        .unwrap();

        let observe = optic
            .observe_head_request(OpticReadBudget {
                max_bytes: Some(1024),
                max_nodes: Some(8),
                max_ticks: Some(4),
                max_attachments: Some(0),
            })
            .to_abi();
        let reading = kernel.observe_optic(observe).unwrap();
        match reading {
            AbiObserveOpticResult::Reading(reading) => {
                assert_eq!(
                    reading.read_identity.optic_id,
                    echo_wasm_abi::kernel_port::OpticId::from_bytes(
                        *optic.optic.optic_id.as_bytes()
                    )
                );
                assert!(matches!(
                    reading.payload,
                    AbiObservationPayload::Head { .. }
                ));
            }
            AbiObserveOpticResult::Obstructed(obstruction) => {
                panic!("worldline head optic should read through kernel, got {obstruction:?}");
            }
        }

        let base_coordinate = EchoCoordinate::Worldline {
            worldline_id,
            at: CoordinateAt::Tick(WorldlineTick::from_raw(0)),
        };
        let dispatch = optic
            .dispatch_eint_v1_request(
                base_coordinate,
                OpticCause {
                    actor,
                    cause_hash: [45; 32],
                    label: Some("kernel optic example".to_owned()),
                },
                AdmissionLawId::from_bytes([46; 32]),
                pack_intent_v1(88, b"kernel-optic-example").unwrap(),
            )
            .to_abi();

        let dispatch = kernel.dispatch_optic_intent(dispatch).unwrap();
        assert!(matches!(
            dispatch,
            echo_wasm_abi::kernel_port::IntentDispatchResult::Staged(_)
        ));
    }

    /// Regression: init() must return real 32-byte hashes, not empty vecs.
    /// The init() WASM export reads the initial frontier head before boxing the
    /// kernel. This test verifies that the observation-backed head helper
    /// upholds that contract on a fresh kernel.
    #[test]
    fn fresh_kernel_head_has_real_hashes() {
        let kernel = WarpKernel::new().unwrap();
        let head = kernel.current_head().unwrap();
        // Must be 32 bytes (BLAKE3 hash), not empty
        assert_eq!(head.state_root.len(), 32, "state_root must be 32 bytes");
        assert_eq!(head.commit_id.len(), 32, "commit_id must be 32 bytes");
        // Must not be all zeros (a real hash of graph state)
        assert_ne!(
            head.state_root,
            vec![0u8; 32],
            "state_root must not be zero"
        );
        assert_ne!(head.commit_id, vec![0u8; 32], "commit_id must not be zero");
    }

    #[test]
    fn start_without_work_keeps_worldline_at_zero() {
        let mut kernel = WarpKernel::new().unwrap();
        let response = start_until_idle(&mut kernel, Some(1));
        assert_eq!(
            response.scheduler_status.last_run_completion,
            Some(RunCompletion::Quiesced)
        );
        assert_eq!(
            kernel.current_head().unwrap().worldline_tick,
            AbiWorldlineTick(0)
        );
    }

    #[test]
    fn start_rejects_zero_cycle_limit_as_invalid_control() {
        let mut kernel = WarpKernel::new().unwrap();
        let error = start_until_idle_result(&mut kernel, Some(0)).unwrap_err();
        assert_eq!(error.code, error_codes::INVALID_CONTROL);
        assert_eq!(error.message, "cycle_limit must be non-zero when present");
    }

    #[test]
    fn stop_while_inactive_preserves_last_run_completion() {
        let mut kernel = WarpKernel::new().unwrap();
        let response = start_until_idle(&mut kernel, Some(1));
        assert_eq!(
            response.scheduler_status.last_run_completion,
            Some(RunCompletion::Quiesced)
        );
        let status_before = kernel.scheduler_status().unwrap();
        assert_eq!(status_before.state, SchedulerState::Inactive);

        let stop = pack_control_intent_v1(&ControlIntentV1::Stop).unwrap();
        let stop_response = kernel.dispatch_intent(&stop).unwrap();

        assert_eq!(
            stop_response.scheduler_status.state,
            SchedulerState::Inactive
        );
        assert_eq!(
            stop_response.scheduler_status.last_run_completion,
            Some(RunCompletion::Quiesced)
        );
        assert_eq!(stop_response.scheduler_status.run_id, status_before.run_id);
        assert_eq!(
            kernel.scheduler_status().unwrap().last_run_completion,
            Some(RunCompletion::Quiesced)
        );
    }

    #[test]
    fn start_executes_commits_until_idle() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        let response = start_until_idle(&mut kernel, Some(3));
        let head = kernel.current_head().unwrap();
        assert_eq!(
            response.scheduler_status.last_run_completion,
            Some(RunCompletion::Quiesced)
        );
        assert_eq!(head.worldline_tick, AbiWorldlineTick(1));
        assert_eq!(
            head.commit_global_tick,
            Some(AbiGlobalTick(
                response
                    .scheduler_status
                    .latest_commit_global_tick
                    .expect("run should record a commit")
                    .0
            ))
        );
        // State root should be non-zero (deterministic hash of root node)
        assert_ne!(head.state_root, vec![0u8; 32]);
    }

    #[test]
    fn set_head_eligibility_rejects_unknown_head_as_invalid_control() {
        let mut kernel = WarpKernel::new().unwrap();
        let control = pack_control_intent_v1(&ControlIntentV1::SetHeadEligibility {
            head: AbiWriterHeadKey {
                worldline_id: abi_worldline_id(kernel.default_worldline),
                head_id: abi_head_id(make_head_id("missing")),
            },
            eligibility: AbiHeadEligibility::Dormant,
        })
        .unwrap();

        let error = kernel.dispatch_intent(&control).unwrap_err();
        assert_eq!(error.code, error_codes::INVALID_CONTROL);
        assert!(error.message.contains("unknown writer head"));
    }

    #[test]
    fn dispatch_intent_accepted() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        let resp = kernel.dispatch_intent(&intent).unwrap();
        assert!(resp.accepted);
        assert_eq!(resp.intent_id.len(), 32);
    }

    #[test]
    fn dispatch_intent_duplicate() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        let r1 = kernel.dispatch_intent(&intent).unwrap();
        let r2 = kernel.dispatch_intent(&intent).unwrap();
        assert!(r1.accepted);
        assert!(!r2.accepted);
        assert_eq!(r1.intent_id, r2.intent_id);
    }

    #[test]
    fn import_suffix_intent_rejects_malformed_payload_without_ingress() {
        let mut kernel = WarpKernel::new().unwrap();
        let head_before = kernel.current_head().unwrap();
        let provenance_len_before = kernel.provenance.len(kernel.default_worldline).unwrap();
        let intent = pack_intent_v1(IMPORT_SUFFIX_INTENT_V1_OP_ID, &[0xff]).unwrap();

        let error = kernel.dispatch_intent(&intent).unwrap_err();

        assert_eq!(error.code, error_codes::INVALID_INTENT);
        assert!(error.message.contains("invalid import suffix intent"));
        assert_eq!(kernel.current_head().unwrap(), head_before);
        assert_eq!(
            kernel.provenance.len(kernel.default_worldline).unwrap(),
            provenance_len_before
        );
    }

    #[test]
    fn import_suffix_intent_enters_ingress_and_scheduler() {
        let mut kernel = WarpKernel::new().unwrap();
        let request = sample_import_suffix_request(&kernel);
        let intent = pack_import_suffix_intent_v1(&request).unwrap();

        let dispatch = kernel.dispatch_intent(&intent).unwrap();
        assert!(dispatch.accepted);
        assert_eq!(dispatch.intent_id.len(), 32);

        let response = start_until_idle(&mut kernel, Some(1));
        let head = kernel.current_head().unwrap();
        assert_eq!(
            response.scheduler_status.last_run_completion,
            Some(RunCompletion::Quiesced)
        );
        assert_eq!(head.worldline_tick, AbiWorldlineTick(1));

        let event_id = NodeId(dispatch.intent_id.as_slice().try_into().unwrap());
        let result_id = warp_core::import_suffix_result_node_id(&event_id);
        let frontier = kernel
            .runtime
            .worldlines()
            .get(&kernel.default_worldline)
            .unwrap();
        let frontier_state = frontier.state();
        let root_warp = frontier_state.root().warp_id;
        let store = frontier_state.store(&root_warp).unwrap();
        let result_node = store.node(&result_id);
        assert!(result_node.is_some());

        let result_attachment = store
            .node_attachment(&result_id)
            .expect("import suffix result attachment should be recorded");
        let warp_core::AttachmentValue::Atom(atom) = result_attachment else {
            panic!("import suffix result must be a typed atom");
        };
        assert_eq!(
            atom.type_id,
            make_type_id(warp_core::IMPORT_SUFFIX_RESULT_ATTACHMENT_TYPE)
        );

        let result: AbiImportSuffixResult = decode_cbor(atom.bytes.as_ref()).unwrap();
        assert_eq!(result.bundle_digest, request.bundle.bundle_digest);
        assert_eq!(
            result.admission.source_shell_digest,
            request.bundle.source_suffix.witness_digest
        );
        assert_eq!(result.admission.target_basis, request.target_basis);
        match result.admission.outcome {
            AbiWitnessedSuffixAdmissionOutcome::Staged { staged_refs, .. } => {
                assert_eq!(staged_refs, request.bundle.source_suffix.source_entries);
            }
            other => panic!("expected staged import result, got {other:?}"),
        }

        let entry = kernel
            .provenance
            .entry(kernel.default_worldline, WorldlineTick::ZERO)
            .unwrap();
        let patch = entry.patch.expect("import tick should record a patch");
        assert!(patch.ops.iter().any(|op| {
            matches!(
                op,
                WarpOp::UpsertNode { node, .. } if node.local_id == result_id
            )
        }));
        assert!(patch.ops.iter().any(|op| {
            matches!(
                op,
                WarpOp::SetAttachment { key, .. }
                    if *key == AttachmentKey::node_alpha(NodeKey {
                        warp_id: root_warp,
                        local_id: result_id,
                    })
            )
        }));
    }

    #[test]
    fn stale_optic_dispatch_obstructs_without_advancing_head_or_provenance() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent = pack_intent_v1(1, b"advance").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        start_until_idle(&mut kernel, Some(1));

        let head_before = kernel.current_head().unwrap();
        let provenance_len_before = kernel.provenance.len(kernel.default_worldline).unwrap();
        assert_eq!(head_before.worldline_tick, AbiWorldlineTick(1));

        let actor = echo_wasm_abi::kernel_port::OpticActorId::from_bytes([4; 32]);
        let intent_family = echo_wasm_abi::kernel_port::IntentFamilyId::from_bytes([5; 32]);
        let worldline_id = abi_worldline_id(kernel.default_worldline);
        let focus = echo_wasm_abi::kernel_port::OpticFocus::Worldline { worldline_id };
        let stale_base = echo_wasm_abi::kernel_port::EchoCoordinate::Worldline {
            worldline_id,
            at: echo_wasm_abi::kernel_port::CoordinateAt::Tick {
                worldline_tick: AbiWorldlineTick(0),
            },
        };
        let request = echo_wasm_abi::kernel_port::DispatchOpticIntentRequest {
            optic_id: echo_wasm_abi::kernel_port::OpticId::from_bytes([1; 32]),
            base_coordinate: stale_base.clone(),
            intent_family,
            focus: focus.clone(),
            cause: echo_wasm_abi::kernel_port::OpticCause {
                actor,
                cause_hash: vec![6; 32],
                label: Some("stale optic dispatch".into()),
            },
            capability: echo_wasm_abi::kernel_port::OpticCapability {
                capability_id: echo_wasm_abi::kernel_port::OpticCapabilityId::from_bytes([7; 32]),
                actor,
                issuer_ref: None,
                policy_hash: vec![8; 32],
                allowed_focus: focus,
                projection_version: echo_wasm_abi::kernel_port::ProjectionVersion(1),
                reducer_version: None,
                allowed_intent_family: intent_family,
                max_budget: echo_wasm_abi::kernel_port::OpticReadBudget {
                    max_bytes: Some(4096),
                    max_nodes: Some(64),
                    max_ticks: Some(8),
                    max_attachments: Some(0),
                },
            },
            admission_law: echo_wasm_abi::kernel_port::AdmissionLawId::from_bytes([9; 32]),
            payload: echo_wasm_abi::kernel_port::OpticIntentPayload::EintV1 {
                bytes: pack_intent_v1(2, b"stale-write").unwrap(),
            },
        };

        let result = kernel.dispatch_optic_intent(request).unwrap();
        assert!(matches!(
            result,
            echo_wasm_abi::kernel_port::IntentDispatchResult::Obstructed(obstruction)
                if obstruction.kind == echo_wasm_abi::kernel_port::OpticObstructionKind::StaleBasis
                    && obstruction.coordinate == Some(stale_base)
        ));

        let head_after = kernel.current_head().unwrap();
        assert_eq!(head_after.worldline_tick, head_before.worldline_tick);
        assert_eq!(head_after.commit_id, head_before.commit_id);
        assert_eq!(
            kernel.provenance.len(kernel.default_worldline).unwrap(),
            provenance_len_before
        );
    }

    #[test]
    fn dispatch_then_start_changes_state() {
        let mut kernel = WarpKernel::new().unwrap();
        let head_before = kernel.current_head().unwrap();

        let intent = pack_intent_v1(1, b"test-intent").unwrap();
        kernel.dispatch_intent(&intent).unwrap();

        let response = start_until_idle(&mut kernel, Some(1));
        let head_after = kernel.current_head().unwrap();
        assert_eq!(
            response.scheduler_status.last_run_completion,
            Some(RunCompletion::Quiesced)
        );
        assert_eq!(head_after.worldline_tick, AbiWorldlineTick(1));
        assert_ne!(head_after.worldline_tick, head_before.worldline_tick);
    }

    #[test]
    fn start_until_idle_exits_when_work_is_blocked_only() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent = pack_intent_v1(7, b"blocked").unwrap();
        kernel.dispatch_intent(&intent).unwrap();

        let dormancy = pack_control_intent_v1(&ControlIntentV1::SetHeadEligibility {
            head: AbiWriterHeadKey {
                worldline_id: abi_worldline_id(kernel.default_worldline),
                head_id: abi_head_id(make_head_id("default")),
            },
            eligibility: AbiHeadEligibility::Dormant,
        })
        .unwrap();
        kernel.dispatch_intent(&dormancy).unwrap();

        let response = start_until_idle(&mut kernel, None);
        assert_eq!(
            response.scheduler_status.last_run_completion,
            Some(RunCompletion::BlockedOnly)
        );
        assert_eq!(response.scheduler_status.state, SchedulerState::Inactive);
        assert_eq!(response.scheduler_status.work_state, WorkState::BlockedOnly);
        assert_eq!(
            kernel.current_head().unwrap().worldline_tick,
            AbiWorldlineTick(0)
        );
    }

    #[test]
    fn scheduler_error_clears_active_run_state() {
        let mut kernel = WarpKernel::new().unwrap();
        let root_warp = kernel
            .runtime
            .worldlines()
            .get(&kernel.default_worldline)
            .expect("default frontier should exist")
            .state()
            .root()
            .warp_id;
        kernel
            .provenance
            .append_local_commit(ProvenanceEntry::local_commit(
                kernel.default_worldline,
                WorldlineTick::ZERO,
                GlobalTick::from_raw(1),
                WriterHeadKey {
                    worldline_id: kernel.default_worldline,
                    head_id: make_head_id("default"),
                },
                Vec::new(),
                HashTriplet {
                    state_root: [0u8; 32],
                    patch_digest: [0u8; 32],
                    commit_hash: [1u8; 32],
                },
                WorldlineTickPatchV1 {
                    header: WorldlineTickHeaderV1 {
                        commit_global_tick: GlobalTick::from_raw(1),
                        policy_id: 0,
                        rule_pack_id: [0u8; 32],
                        plan_digest: [0u8; 32],
                        decision_digest: [0u8; 32],
                        rewrites_digest: [0u8; 32],
                    },
                    warp_id: root_warp,
                    ops: Vec::new(),
                    in_slots: Vec::new(),
                    out_slots: Vec::new(),
                    patch_digest: [0u8; 32],
                },
                Vec::new(),
                Vec::new(),
            ))
            .unwrap();

        let intent = pack_intent_v1(8, b"error").unwrap();
        kernel.dispatch_intent(&intent).unwrap();

        let err = start_until_idle_result(&mut kernel, Some(1)).unwrap_err();
        assert_eq!(err.code, error_codes::ENGINE_ERROR);

        let status = kernel.scheduler_status().unwrap();
        assert_eq!(status.state, SchedulerState::Inactive);
        assert_eq!(status.active_mode, None);
        assert_eq!(status.run_id, None);
        assert_eq!(status.last_run_completion, None);
        assert_eq!(status.work_state, WorkState::RunnablePending);
    }

    #[test]
    fn observe_invalid_tick_returns_observation_error_code() {
        let kernel = WarpKernel::new().unwrap();
        let err = kernel
            .observe(abi_builtin_one_shot(
                AbiObservationCoordinate {
                    worldline_id: abi_worldline_id(kernel.default_worldline),
                    at: AbiObservationAt::Tick {
                        worldline_tick: AbiWorldlineTick(999),
                    },
                },
                AbiObservationFrame::CommitBoundary,
                AbiObservationProjection::Snapshot,
            ))
            .unwrap_err();
        assert_eq!(err.code, error_codes::INVALID_TICK);
    }

    #[test]
    fn observe_snapshot_returns_historical_commit_boundary() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        start_until_idle(&mut kernel, Some(1));
        let artifact = kernel
            .observe(abi_builtin_one_shot(
                AbiObservationCoordinate {
                    worldline_id: abi_worldline_id(kernel.default_worldline),
                    at: AbiObservationAt::Tick {
                        worldline_tick: AbiWorldlineTick(0),
                    },
                },
                AbiObservationFrame::CommitBoundary,
                AbiObservationProjection::Snapshot,
            ))
            .unwrap();

        let AbiObservationPayload::Snapshot { snapshot } = artifact.payload else {
            panic!("expected snapshot observation payload");
        };
        assert_eq!(snapshot.worldline_tick, AbiWorldlineTick(0));
        assert_eq!(snapshot.commit_global_tick, Some(AbiGlobalTick(1)));
        assert_eq!(snapshot.state_root.len(), 32);
        assert_eq!(snapshot.commit_id.len(), 32);
    }

    #[test]
    fn observe_frontier_head_matches_current_head() {
        let kernel = WarpKernel::new().unwrap();
        let artifact = kernel
            .observe(abi_builtin_one_shot(
                AbiObservationCoordinate {
                    worldline_id: abi_worldline_id(kernel.default_worldline),
                    at: AbiObservationAt::Frontier,
                },
                AbiObservationFrame::CommitBoundary,
                AbiObservationProjection::Head,
            ))
            .unwrap();
        let head = kernel.current_head().unwrap();

        assert_eq!(
            artifact.reading.observer_plan,
            AbiReadingObserverPlan::Builtin {
                plan: AbiBuiltinObserverPlan::CommitBoundaryHead,
            }
        );
        assert_eq!(
            artifact.reading.parent_basis_posture,
            AbiObservationBasisPosture::Worldline
        );
        assert_eq!(
            artifact.reading.observer_basis,
            AbiReadingObserverBasis::CommitBoundary
        );
        assert_eq!(
            artifact.reading.budget_posture,
            AbiReadingBudgetPosture::UnboundedOneShot
        );
        assert_eq!(
            artifact.reading.rights_posture,
            AbiReadingRightsPosture::KernelPublic
        );
        assert_eq!(
            artifact.reading.residual_posture,
            AbiReadingResidualPosture::Complete
        );

        let AbiObservationPayload::Head { head: observed } = artifact.payload else {
            panic!("expected head observation payload");
        };
        assert_eq!(observed.worldline_tick, head.worldline_tick);
        assert_eq!(observed.commit_global_tick, head.commit_global_tick);
        assert_eq!(observed.state_root, head.state_root);
        assert_eq!(observed.commit_id, head.commit_id);
    }

    #[test]
    fn observe_neighborhood_site_returns_singleton_site_for_default_worldline() {
        let kernel = WarpKernel::new().unwrap();
        let site = kernel
            .observe_neighborhood_site(abi_builtin_one_shot(
                AbiObservationCoordinate {
                    worldline_id: abi_worldline_id(kernel.default_worldline),
                    at: AbiObservationAt::Frontier,
                },
                AbiObservationFrame::CommitBoundary,
                AbiObservationProjection::Head,
            ))
            .unwrap();

        assert_eq!(
            site.plurality,
            echo_wasm_abi::kernel_port::SitePlurality::Singleton
        );
        assert_eq!(site.participants.len(), 1);
        assert_eq!(
            site.participants[0].role,
            echo_wasm_abi::kernel_port::ParticipantRole::Primary
        );
        assert_eq!(
            site.participants[0].worldline_id,
            abi_worldline_id(kernel.default_worldline)
        );
    }

    #[test]
    fn observe_neighborhood_core_returns_shared_projection_for_default_worldline() {
        let kernel = WarpKernel::new().unwrap();
        let core = kernel
            .observe_neighborhood_core(AbiObservationRequest {
                coordinate: AbiObservationCoordinate {
                    worldline_id: abi_worldline_id(kernel.default_worldline),
                    at: AbiObservationAt::Frontier,
                },
                frame: AbiObservationFrame::CommitBoundary,
                projection: AbiObservationProjection::Head,
            })
            .unwrap();

        assert_eq!(
            core.outcome_kind,
            echo_wasm_abi::kernel_port::AdmissionOutcomeKind::Derived
        );
        assert_eq!(
            core.plurality,
            echo_wasm_abi::kernel_port::NeighborhoodPlurality::Singleton
        );
        assert_eq!(core.anchor_frame_index, 0);
        assert_eq!(core.anchor_head_id, None);
        assert_eq!(core.participants.len(), 1);
        assert_eq!(
            core.participants[0].role,
            echo_wasm_abi::kernel_port::NeighborhoodParticipantRole::Primary
        );
        assert!(core.anchor_lane_id.starts_with("wl:"));
        assert!(core.site_id.starts_with("site:"));
    }

    #[test]
    fn settlement_publication_returns_import_plan_for_child_suffix() {
        let mut kernel = WarpKernel::new().unwrap();
        let (runtime, provenance, strand_id, base_worldline, child_worldline) =
            setup_runtime_with_strand(ParentDrift::None);
        kernel.runtime = runtime;
        kernel.provenance = provenance;
        kernel.default_worldline = base_worldline;

        let request = AbiSettlementRequest {
            strand_id: echo_wasm_abi::kernel_port::StrandId::from_bytes(*strand_id.as_bytes()),
        };
        let delta = kernel.compare_settlement(request.clone()).unwrap();
        assert_eq!(delta.source_worldline_id, abi_worldline_id(child_worldline));
        assert_eq!(delta.source_entries.len(), 1);

        let plan = kernel.plan_settlement(request.clone()).unwrap();
        assert_eq!(plan.strand_id, request.strand_id);
        assert_eq!(plan.target_worldline, abi_worldline_id(base_worldline));
        assert!(matches!(
            &plan.decisions[0],
            AbiSettlementDecision::ImportCandidate { .. }
        ));

        let result = kernel.settle_strand(request).unwrap();
        assert_eq!(result.appended_imports.len(), 1);
        assert!(result.appended_conflicts.is_empty());
    }

    #[test]
    fn settlement_publication_imports_when_parent_advanced_disjoint() {
        let mut kernel = WarpKernel::new().unwrap();
        let (runtime, provenance, strand_id, base_worldline, _) =
            setup_runtime_with_strand(ParentDrift::Disjoint);
        kernel.runtime = runtime;
        kernel.provenance = provenance;
        kernel.default_worldline = base_worldline;

        let request = AbiSettlementRequest {
            strand_id: echo_wasm_abi::kernel_port::StrandId::from_bytes(*strand_id.as_bytes()),
        };
        let delta = kernel.compare_settlement(request.clone()).unwrap();
        assert!(matches!(
            &delta.basis_report.parent_revalidation,
            AbiSettlementParentRevalidation::ParentAdvancedDisjoint { .. }
        ));

        let plan = kernel.plan_settlement(request.clone()).unwrap();
        assert!(matches!(
            &plan.basis_report.parent_revalidation,
            AbiSettlementParentRevalidation::ParentAdvancedDisjoint { .. }
        ));
        assert!(matches!(
            &plan.decisions[0],
            AbiSettlementDecision::ImportCandidate { .. }
        ));

        let result = kernel.settle_strand(request).unwrap();
        assert_eq!(result.appended_imports.len(), 1);
        assert!(result.appended_conflicts.is_empty());
    }

    #[test]
    fn settlement_publication_exposes_overlap_revalidation_evidence() {
        let mut kernel = WarpKernel::new().unwrap();
        let (runtime, provenance, strand_id, base_worldline, _) =
            setup_runtime_with_strand(ParentDrift::OverlapSame);
        kernel.runtime = runtime;
        kernel.provenance = provenance;
        kernel.default_worldline = base_worldline;

        let request = AbiSettlementRequest {
            strand_id: echo_wasm_abi::kernel_port::StrandId::from_bytes(*strand_id.as_bytes()),
        };
        let delta = kernel.compare_settlement(request.clone()).unwrap();
        let AbiSettlementParentRevalidation::RevalidationRequired {
            overlapping_slot_count,
            overlapping_slots_digest,
            ..
        } = &delta.basis_report.parent_revalidation
        else {
            panic!("expected overlap posture in compare_settlement");
        };
        assert!(*overlapping_slot_count > 0);
        assert_eq!(overlapping_slots_digest.len(), 32);

        let plan = kernel.plan_settlement(request).unwrap();
        let AbiSettlementParentRevalidation::RevalidationRequired {
            overlapping_slot_count: plan_overlap_count,
            overlapping_slots_digest: plan_overlap_digest,
            ..
        } = &plan.basis_report.parent_revalidation
        else {
            panic!("expected overlap posture in plan_settlement");
        };
        assert_eq!(plan_overlap_count, overlapping_slot_count);
        assert_eq!(plan_overlap_digest, overlapping_slots_digest);

        let AbiSettlementDecision::ImportCandidate { candidate } = &plan.decisions[0] else {
            panic!("expected overlap-clean import candidate");
        };
        let Some(AbiSettlementOverlapRevalidation::Clean {
            overlapping_slot_count: decision_overlap_count,
            overlapping_slots_digest: decision_overlap_digest,
        }) = &candidate.overlap_revalidation
        else {
            panic!("expected clean overlap revalidation evidence");
        };
        assert_eq!(decision_overlap_count, overlapping_slot_count);
        assert_eq!(decision_overlap_digest, overlapping_slots_digest);
    }

    #[test]
    fn observe_frontier_snapshot_reports_u0_without_fake_sentinels() {
        let kernel = WarpKernel::new().unwrap();
        let artifact = kernel
            .observe(abi_builtin_one_shot(
                AbiObservationCoordinate {
                    worldline_id: abi_worldline_id(kernel.default_worldline),
                    at: AbiObservationAt::Frontier,
                },
                AbiObservationFrame::CommitBoundary,
                AbiObservationProjection::Snapshot,
            ))
            .unwrap();

        let AbiObservationPayload::Snapshot { snapshot } = artifact.payload else {
            panic!("expected snapshot observation payload");
        };
        assert_eq!(snapshot.worldline_tick, AbiWorldlineTick(0));
        assert_eq!(snapshot.commit_global_tick, None);
        assert_eq!(snapshot.state_root.len(), 32);
        assert_eq!(snapshot.commit_id.len(), 32);
        assert_ne!(snapshot.state_root, vec![0u8; 32]);
        assert_ne!(snapshot.commit_id, vec![0u8; 32]);
    }

    #[test]
    fn observe_recorded_truth_is_read_only() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        start_until_idle(&mut kernel, Some(1));

        let head_before = kernel.current_head().unwrap();
        let _ = kernel
            .observe(abi_builtin_one_shot(
                AbiObservationCoordinate {
                    worldline_id: abi_worldline_id(kernel.default_worldline),
                    at: AbiObservationAt::Frontier,
                },
                AbiObservationFrame::RecordedTruth,
                AbiObservationProjection::TruthChannels { channels: None },
            ))
            .unwrap();
        let head_after = kernel.current_head().unwrap();

        assert_eq!(head_before, head_after);
    }

    #[test]
    fn observe_recorded_truth_returns_committed_outputs_for_tick() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent_a = pack_intent_v1(1, b"hello").unwrap();
        let intent_b = pack_intent_v1(2, b"world").unwrap();
        kernel.dispatch_intent(&intent_a).unwrap();
        start_until_idle(&mut kernel, Some(1));
        kernel.dispatch_intent(&intent_b).unwrap();
        start_until_idle(&mut kernel, Some(1));

        let worldline_id = kernel.default_worldline;
        let frontier_state = kernel
            .runtime
            .worldlines()
            .get(&worldline_id)
            .unwrap()
            .state();
        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(worldline_id, frontier_state)
            .unwrap();

        for tick in 0..kernel.provenance.len(worldline_id).unwrap() {
            let mut entry = kernel
                .provenance
                .entry(worldline_id, WorldlineTick::from_raw(tick))
                .unwrap();
            entry.outputs = vec![(
                make_channel_id("test:truth"),
                format!("tick-{tick}").into_bytes(),
            )];
            provenance.append_local_commit(entry).unwrap();
        }
        kernel.provenance = provenance;

        let artifact = kernel
            .observe(abi_builtin_one_shot(
                AbiObservationCoordinate {
                    worldline_id: abi_worldline_id(kernel.default_worldline),
                    at: AbiObservationAt::Tick {
                        worldline_tick: AbiWorldlineTick(1),
                    },
                },
                AbiObservationFrame::RecordedTruth,
                AbiObservationProjection::TruthChannels { channels: None },
            ))
            .unwrap();
        let AbiObservationPayload::TruthChannels { channels } = artifact.payload else {
            panic!("expected recorded-truth payload");
        };
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].data, b"tick-1".to_vec());
    }

    #[test]
    fn registry_info_has_abi_version() {
        let kernel = WarpKernel::new().unwrap();
        let info = kernel.registry_info();
        assert_eq!(info.abi_version, ABI_VERSION);
        assert_eq!(info.codec_id.as_deref(), Some("cbor-canonical-v1"));
    }

    #[test]
    fn head_state_root_is_deterministic() {
        // Two fresh kernels should produce identical state roots
        let k1 = WarpKernel::new().unwrap();
        let k2 = WarpKernel::new().unwrap();
        let h1 = k1.current_head().unwrap();
        let h2 = k2.current_head().unwrap();
        assert_eq!(h1.state_root, h2.state_root);
        assert_eq!(h1.commit_id, h2.commit_id);
    }

    #[test]
    fn dispatch_invalid_intent_returns_invalid_intent_error() {
        let mut kernel = WarpKernel::new().unwrap();

        // Garbage bytes (no EINT magic)
        let err = kernel.dispatch_intent(b"not-an-envelope").unwrap_err();
        assert_eq!(err.code, error_codes::INVALID_INTENT);

        // Truncated envelope (valid magic but too short for full header)
        let err = kernel.dispatch_intent(b"EINT\x00\x00").unwrap_err();
        assert_eq!(err.code, error_codes::INVALID_INTENT);

        // Empty bytes
        let err = kernel.dispatch_intent(b"").unwrap_err();
        assert_eq!(err.code, error_codes::INVALID_INTENT);
    }

    #[test]
    fn with_engine_installs_default_runtime_worldline() {
        let mut store = GraphStore::default();
        let root = make_node_id("root");
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );

        let engine = EngineBuilder::new(store, root)
            .scheduler(SchedulerKind::Radix)
            .workers(1)
            .build();
        let mut kernel = WarpKernel::with_engine(
            engine,
            RegistryInfo {
                codec_id: None,
                registry_version: None,
                schema_sha256_hex: None,
                abi_version: ABI_VERSION,
            },
        )
        .unwrap();

        let intent = pack_intent_v1(1, b"test").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        let response = start_until_idle(&mut kernel, Some(1));
        assert_eq!(
            response.scheduler_status.last_run_completion,
            Some(RunCompletion::Quiesced)
        );
    }

    #[test]
    fn with_engine_preserves_zero_tick_without_ingress() {
        let mut store = GraphStore::default();
        let root = make_node_id("root");
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );

        let engine = EngineBuilder::new(store, root)
            .scheduler(SchedulerKind::Radix)
            .workers(1)
            .build();
        let kernel = WarpKernel::with_engine(
            engine,
            RegistryInfo {
                codec_id: None,
                registry_version: None,
                schema_sha256_hex: None,
                abi_version: ABI_VERSION,
            },
        )
        .unwrap();
        assert_eq!(
            kernel.current_head().unwrap().worldline_tick,
            AbiWorldlineTick(0)
        );
    }

    #[test]
    fn with_engine_rejects_non_fresh_engine_state() {
        let mut store = GraphStore::default();
        let root = make_node_id("root");
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );

        let mut engine = EngineBuilder::new(store, root)
            .scheduler(SchedulerKind::Radix)
            .workers(1)
            .build();
        engine.ingest_intent(b"already-committed").unwrap();
        let tx = engine.begin();
        let _ = engine.commit(tx).unwrap();

        let kernel = WarpKernel::with_engine(
            engine,
            RegistryInfo {
                codec_id: None,
                registry_version: None,
                schema_sha256_hex: None,
                abi_version: ABI_VERSION,
            },
        );

        assert!(matches!(kernel, Err(KernelInitError::NonFreshEngine)));
    }

    #[test]
    fn with_engine_rejects_legacy_engine_inbox_state() {
        let mut store = GraphStore::default();
        let root = make_node_id("root");
        store.insert_node(
            root,
            NodeRecord {
                ty: make_type_id("world"),
            },
        );

        let mut engine = EngineBuilder::new(store, root)
            .scheduler(SchedulerKind::Radix)
            .workers(1)
            .build();
        let _ = engine.ingest_intent(b"legacy-only").unwrap();

        let kernel = WarpKernel::with_engine(
            engine,
            RegistryInfo {
                codec_id: None,
                registry_version: None,
                schema_sha256_hex: None,
                abi_version: ABI_VERSION,
            },
        );

        assert!(matches!(kernel, Err(KernelInitError::NonFreshEngine)));
    }

    #[test]
    fn start_produces_deterministic_commits() {
        let mut k1 = WarpKernel::new().unwrap();
        let mut k2 = WarpKernel::new().unwrap();

        // Same operations should produce identical state
        let intent = pack_intent_v1(42, b"determinism-test").unwrap();
        k1.dispatch_intent(&intent).unwrap();
        k2.dispatch_intent(&intent).unwrap();

        start_until_idle(&mut k1, Some(1));
        start_until_idle(&mut k2, Some(1));
        let r1 = k1.current_head().unwrap();
        let r2 = k2.current_head().unwrap();

        assert_eq!(r1.state_root, r2.state_root);
        assert_eq!(r1.commit_id, r2.commit_id);
    }
}
