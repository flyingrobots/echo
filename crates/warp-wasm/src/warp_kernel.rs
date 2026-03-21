// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Engine-backed [`KernelPort`] implementation.
//!
//! [`WarpKernel`] wraps a `warp-core::Engine` and translates its typed API
//! into the byte-level contract expected by the WASM boundary. This module
//! is gated behind the `engine` feature.

use std::fmt;

use echo_wasm_abi::kernel_port::{
    error_codes, AbiError, ControlIntentV1, DispatchResponse, GlobalTick as AbiGlobalTick,
    HeadEligibility as AbiHeadEligibility, HeadInfo, HeadKey as AbiHeadKey, KernelPort,
    ObservationArtifact as AbiObservationArtifact, ObservationFrame as AbiObservationFrame,
    ObservationProjection as AbiObservationProjection, ObservationRequest as AbiObservationRequest,
    RegistryInfo, RunCompletion, RunId as AbiRunId, SchedulerMode, SchedulerState, SchedulerStatus,
    WorkState, WorldlineTick as AbiWorldlineTick, ABI_VERSION,
};
use echo_wasm_abi::{unpack_control_intent_v1, unpack_intent_v1, CONTROL_INTENT_V1_OP_ID};
use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, Engine, EngineBuilder, GlobalTick,
    GraphStore, HeadEligibility, HeadId, HistoryError, IngressDisposition, IngressEnvelope,
    IngressTarget, NodeRecord, ObservationAt, ObservationCoordinate, ObservationError,
    ObservationFrame, ObservationPayload, ObservationProjection, ObservationRequest,
    ObservationService, PlaybackMode, ProvenanceService, RunId, RuntimeError, SchedulerCoordinator,
    SchedulerKind, WorldlineId, WorldlineRuntime, WorldlineState, WorldlineStateError,
    WorldlineTick, WriterHead, WriterHeadKey,
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
}

impl fmt::Display for KernelInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFreshEngine => write!(f, "WarpKernel::with_engine requires a fresh engine"),
            Self::WorldlineState(err) => err.fmt(f),
            Self::Provenance(err) => err.fmt(f),
            Self::Runtime(err) => err.fmt(f),
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
    /// The engine has a single root node and no rewrite rules.
    /// Useful for testing the boundary or as a starting point.
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
    pub fn with_engine(engine: Engine, registry: RegistryInfo) -> Result<Self, KernelInitError> {
        if !engine.is_fresh_runtime_state() {
            return Err(KernelInitError::NonFreshEngine);
        }
        let root = engine.root_key();
        let default_worldline = WorldlineId(root.warp_id.0);
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

    fn parse_worldline_id(bytes: &[u8]) -> Result<WorldlineId, AbiError> {
        let hash: [u8; 32] = bytes.try_into().map_err(|_| AbiError {
            code: error_codes::INVALID_WORLDLINE,
            message: format!("worldline id must be exactly 32 bytes, got {}", bytes.len()),
        })?;
        Ok(WorldlineId(hash))
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

    fn to_core_request(request: AbiObservationRequest) -> Result<ObservationRequest, AbiError> {
        let worldline_id = Self::parse_worldline_id(&request.coordinate.worldline_id)?;
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
        Ok(ObservationRequest {
            coordinate: ObservationCoordinate { worldline_id, at },
            frame,
            projection,
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
        Self::head_info_from_observation(self.observe_core(ObservationRequest {
            coordinate: ObservationCoordinate {
                worldline_id: self.default_worldline,
                at: ObservationAt::Frontier,
            },
            frame: ObservationFrame::CommitBoundary,
            projection: ObservationProjection::Head,
        })?)
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
                let key = Self::parse_head_key(&head)?;
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

    fn parse_head_key(head: &AbiHeadKey) -> Result<WriterHeadKey, AbiError> {
        let worldline_id = Self::parse_worldline_id(&head.worldline_id)?;
        let head_id_bytes: [u8; 32] = head.head_id.as_slice().try_into().map_err(|_| AbiError {
            code: error_codes::INVALID_CONTROL,
            message: format!(
                "head id must be exactly 32 bytes, got {}",
                head.head_id.len()
            ),
        })?;
        Ok(WriterHeadKey {
            worldline_id,
            head_id: HeadId::from_bytes(head_id_bytes),
        })
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

    fn observe(&self, request: AbiObservationRequest) -> Result<AbiObservationArtifact, AbiError> {
        let request = Self::to_core_request(request)?;
        Ok(self.observe_core(request)?.to_abi())
    }

    fn registry_info(&self) -> RegistryInfo {
        self.registry.clone()
    }

    fn scheduler_status(&self) -> Result<SchedulerStatus, AbiError> {
        Ok(self.scheduler_status.clone())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use echo_wasm_abi::{
        kernel_port::{
            ControlIntentV1, GlobalTick as AbiGlobalTick, HeadEligibility as AbiHeadEligibility,
            HeadKey as AbiHeadKey, ObservationAt as AbiObservationAt,
            ObservationCoordinate as AbiObservationCoordinate,
            ObservationFrame as AbiObservationFrame, ObservationPayload as AbiObservationPayload,
            ObservationProjection as AbiObservationProjection,
            ObservationRequest as AbiObservationRequest, RunCompletion, SchedulerMode,
            SchedulerState, WorkState, WorldlineTick as AbiWorldlineTick,
        },
        pack_control_intent_v1, pack_intent_v1,
    };
    use warp_core::{
        make_head_id, materialization::make_channel_id, GlobalTick, HashTriplet, ProvenanceEntry,
        ProvenanceStore, WorldlineTick, WorldlineTickHeaderV1, WorldlineTickPatchV1, WriterHeadKey,
    };

    fn start_until_idle(kernel: &mut WarpKernel, cycle_limit: Option<u32>) -> DispatchResponse {
        start_until_idle_result(kernel, cycle_limit).unwrap()
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

    #[test]
    fn new_kernel_has_zero_tick() {
        let kernel = WarpKernel::new().unwrap();
        let head = kernel.current_head().unwrap();
        assert_eq!(head.worldline_tick, AbiWorldlineTick(0));
        assert_eq!(head.commit_global_tick, None);
        assert_eq!(head.state_root.len(), 32);
        assert_eq!(head.commit_id.len(), 32);
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
            head: AbiHeadKey {
                worldline_id: kernel.default_worldline.0.to_vec(),
                head_id: make_head_id("missing").as_bytes().to_vec(),
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
            head: AbiHeadKey {
                worldline_id: kernel.default_worldline.0.to_vec(),
                head_id: make_head_id("default").as_bytes().to_vec(),
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
            .observe(AbiObservationRequest {
                coordinate: AbiObservationCoordinate {
                    worldline_id: kernel.default_worldline.0.to_vec(),
                    at: AbiObservationAt::Tick {
                        worldline_tick: AbiWorldlineTick(999),
                    },
                },
                frame: AbiObservationFrame::CommitBoundary,
                projection: AbiObservationProjection::Snapshot,
            })
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
            .observe(AbiObservationRequest {
                coordinate: AbiObservationCoordinate {
                    worldline_id: kernel.default_worldline.0.to_vec(),
                    at: AbiObservationAt::Tick {
                        worldline_tick: AbiWorldlineTick(0),
                    },
                },
                frame: AbiObservationFrame::CommitBoundary,
                projection: AbiObservationProjection::Snapshot,
            })
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
            .observe(AbiObservationRequest {
                coordinate: AbiObservationCoordinate {
                    worldline_id: kernel.default_worldline.0.to_vec(),
                    at: AbiObservationAt::Frontier,
                },
                frame: AbiObservationFrame::CommitBoundary,
                projection: AbiObservationProjection::Head,
            })
            .unwrap();
        let head = kernel.current_head().unwrap();

        let AbiObservationPayload::Head { head: observed } = artifact.payload else {
            panic!("expected head observation payload");
        };
        assert_eq!(observed.worldline_tick, head.worldline_tick);
        assert_eq!(observed.commit_global_tick, head.commit_global_tick);
        assert_eq!(observed.state_root, head.state_root);
        assert_eq!(observed.commit_id, head.commit_id);
    }

    #[test]
    fn observe_recorded_truth_is_read_only() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        start_until_idle(&mut kernel, Some(1));

        let head_before = kernel.current_head().unwrap();
        let _ = kernel
            .observe(AbiObservationRequest {
                coordinate: AbiObservationCoordinate {
                    worldline_id: kernel.default_worldline.0.to_vec(),
                    at: AbiObservationAt::Frontier,
                },
                frame: AbiObservationFrame::RecordedTruth,
                projection: AbiObservationProjection::TruthChannels { channels: None },
            })
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
            .observe(AbiObservationRequest {
                coordinate: AbiObservationCoordinate {
                    worldline_id: kernel.default_worldline.0.to_vec(),
                    at: AbiObservationAt::Tick {
                        worldline_tick: AbiWorldlineTick(1),
                    },
                },
                frame: AbiObservationFrame::RecordedTruth,
                projection: AbiObservationProjection::TruthChannels { channels: None },
            })
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
