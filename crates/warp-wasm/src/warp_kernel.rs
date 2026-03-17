// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Engine-backed [`KernelPort`] implementation.
//!
//! [`WarpKernel`] wraps a `warp-core::Engine` and translates its typed API
//! into the byte-level contract expected by the WASM boundary. This module
//! is gated behind the `engine` feature.

use std::fmt;

use echo_wasm_abi::kernel_port::{
    error_codes, AbiError, DispatchResponse, HeadInfo, KernelPort,
    ObservationArtifact as AbiObservationArtifact, ObservationFrame as AbiObservationFrame,
    ObservationProjection as AbiObservationProjection, ObservationRequest as AbiObservationRequest,
    RegistryInfo, StepResponse, ABI_VERSION,
};
use echo_wasm_abi::unpack_intent_v1;
use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, Engine, EngineBuilder, GraphStore,
    HistoryError, IngressDisposition, IngressEnvelope, IngressTarget, NodeRecord, ObservationAt,
    ObservationCoordinate, ObservationError, ObservationFrame, ObservationPayload,
    ObservationProjection, ObservationRequest, ObservationService, PlaybackMode, ProvenanceService,
    RuntimeError, SchedulerCoordinator, SchedulerKind, WorldlineId, WorldlineRuntime,
    WorldlineState, WorldlineStateError, WriterHead, WriterHeadKey,
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
            echo_wasm_abi::kernel_port::ObservationAt::Tick { tick } => ObservationAt::Tick(tick),
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
                tick: head.tick,
                state_root: head.state_root.to_vec(),
                commit_id: head.commit_hash.to_vec(),
            }),
            _ => Err(AbiError {
                code: error_codes::ENGINE_ERROR,
                message: "observe returned non-head payload for head adapter".into(),
            }),
        }
    }
}

impl KernelPort for WarpKernel {
    fn dispatch_intent(&mut self, intent_bytes: &[u8]) -> Result<DispatchResponse, AbiError> {
        // Validate the EINT envelope before passing to the engine.
        if let Err(e) = unpack_intent_v1(intent_bytes) {
            return Err(AbiError {
                code: error_codes::INVALID_INTENT,
                message: format!(
                    "malformed EINT envelope ({} bytes): {e}",
                    intent_bytes.len()
                ),
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
                let (accepted, ingress_id) = match disposition {
                    IngressDisposition::Accepted { ingress_id, .. } => (true, ingress_id),
                    IngressDisposition::Duplicate { ingress_id, .. } => (false, ingress_id),
                };
                Ok(DispatchResponse {
                    accepted,
                    intent_id: ingress_id.to_vec(),
                })
            }
            Err(e) => Err(AbiError {
                code: error_codes::ENGINE_ERROR,
                message: e.to_string(),
            }),
        }
    }

    fn step(&mut self, budget: u32) -> Result<StepResponse, AbiError> {
        if budget == 0 {
            return Ok(StepResponse {
                ticks_executed: 0,
                head: self.current_head()?,
            });
        }

        let mut ticks_executed: u32 = 0;

        for _ in 0..budget {
            // Phase 3 exposes only the default worldline/default writer through
            // the WASM ABI, so one coordinator pass can produce at most one
            // committed head step here.
            let records = SchedulerCoordinator::super_tick(
                &mut self.runtime,
                &mut self.provenance,
                &mut self.engine,
            )
            .map_err(|e| AbiError {
                code: error_codes::ENGINE_ERROR,
                message: e.to_string(),
            })?;
            if records.is_empty() {
                break;
            }
            #[allow(clippy::cast_possible_truncation)]
            {
                ticks_executed += records.len() as u32;
            }
        }

        Ok(StepResponse {
            ticks_executed,
            head: self.current_head()?,
        })
    }

    fn observe(&self, request: AbiObservationRequest) -> Result<AbiObservationArtifact, AbiError> {
        let request = Self::to_core_request(request)?;
        Ok(self.observe_core(request)?.to_abi())
    }

    fn registry_info(&self) -> RegistryInfo {
        self.registry.clone()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use echo_wasm_abi::{
        kernel_port::{
            ObservationAt as AbiObservationAt, ObservationCoordinate as AbiObservationCoordinate,
            ObservationFrame as AbiObservationFrame, ObservationPayload as AbiObservationPayload,
            ObservationProjection as AbiObservationProjection,
            ObservationRequest as AbiObservationRequest,
        },
        pack_intent_v1,
    };
    use warp_core::{materialization::make_channel_id, ProvenanceStore};

    #[test]
    fn new_kernel_has_zero_tick() {
        let kernel = WarpKernel::new().unwrap();
        let head = kernel.current_head().unwrap();
        assert_eq!(head.tick, 0);
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
    fn step_zero_is_noop() {
        let mut kernel = WarpKernel::new().unwrap();
        let result = kernel.step(0).unwrap();
        assert_eq!(result.ticks_executed, 0);
        assert_eq!(result.head.tick, 0);
    }

    #[test]
    fn step_executes_ticks() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        let result = kernel.step(3).unwrap();
        assert_eq!(result.ticks_executed, 1);
        assert_eq!(result.head.tick, 1);
        // State root should be non-zero (deterministic hash of root node)
        assert_ne!(result.head.state_root, vec![0u8; 32]);
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
    fn dispatch_then_step_changes_state() {
        let mut kernel = WarpKernel::new().unwrap();
        let head_before = kernel.current_head().unwrap();

        let intent = pack_intent_v1(1, b"test-intent").unwrap();
        kernel.dispatch_intent(&intent).unwrap();

        let result = kernel.step(1).unwrap();
        assert_eq!(result.ticks_executed, 1);
        assert_eq!(result.head.tick, 1);
        assert_ne!(result.head.tick, head_before.tick);
    }

    #[test]
    fn observe_invalid_tick_returns_observation_error_code() {
        let kernel = WarpKernel::new().unwrap();
        let err = kernel
            .observe(AbiObservationRequest {
                coordinate: AbiObservationCoordinate {
                    worldline_id: kernel.default_worldline.0.to_vec(),
                    at: AbiObservationAt::Tick { tick: 999 },
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
        kernel.step(1).unwrap();
        let artifact = kernel
            .observe(AbiObservationRequest {
                coordinate: AbiObservationCoordinate {
                    worldline_id: kernel.default_worldline.0.to_vec(),
                    at: AbiObservationAt::Tick { tick: 0 },
                },
                frame: AbiObservationFrame::CommitBoundary,
                projection: AbiObservationProjection::Snapshot,
            })
            .unwrap();

        let AbiObservationPayload::Snapshot { snapshot } = artifact.payload else {
            panic!("expected snapshot observation payload");
        };
        assert_eq!(snapshot.tick, 0);
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
        assert_eq!(observed.tick, head.tick);
        assert_eq!(observed.state_root, head.state_root);
        assert_eq!(observed.commit_id, head.commit_id);
    }

    #[test]
    fn observe_recorded_truth_is_read_only() {
        let mut kernel = WarpKernel::new().unwrap();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        kernel.step(1).unwrap();

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
        kernel.step(1).unwrap();
        kernel.dispatch_intent(&intent_b).unwrap();
        kernel.step(1).unwrap();

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
            let mut entry = kernel.provenance.entry(worldline_id, tick).unwrap();
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
                    at: AbiObservationAt::Tick { tick: 1 },
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
        let result = kernel.step(1).unwrap();
        assert_eq!(result.ticks_executed, 1);
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
        assert_eq!(kernel.current_head().unwrap().tick, 0);
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
    fn step_produces_deterministic_commits() {
        let mut k1 = WarpKernel::new().unwrap();
        let mut k2 = WarpKernel::new().unwrap();

        // Same operations should produce identical state
        let intent = pack_intent_v1(42, b"determinism-test").unwrap();
        k1.dispatch_intent(&intent).unwrap();
        k2.dispatch_intent(&intent).unwrap();

        let r1 = k1.step(1).unwrap();
        let r2 = k2.step(1).unwrap();

        assert_eq!(r1.head.state_root, r2.head.state_root);
        assert_eq!(r1.head.commit_id, r2.head.commit_id);
    }
}
