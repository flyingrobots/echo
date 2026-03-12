// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Engine-backed [`KernelPort`] implementation.
//!
//! [`WarpKernel`] wraps a `warp-core::Engine` and translates its typed API
//! into the byte-level contract expected by the WASM boundary. This module
//! is gated behind the `engine` feature.

use echo_wasm_abi::kernel_port::{
    error_codes, AbiError, ChannelData, DispatchResponse, DrainResponse, HeadInfo, KernelPort,
    RegistryInfo, StepResponse, ABI_VERSION,
};
use echo_wasm_abi::unpack_intent_v1;
use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, Engine, EngineBuilder, GraphStore,
    IngressDisposition, IngressEnvelope, IngressTarget, NodeRecord, PlaybackMode,
    SchedulerCoordinator, SchedulerKind, WorldlineId, WorldlineRuntime, WorldlineState, WriterHead,
    WriterHeadKey,
};

/// App-agnostic kernel wrapping a `warp-core::Engine`.
///
/// Constructed via [`WarpKernel::new`] (default empty engine) or
/// [`WarpKernel::with_engine`] (pre-configured engine with rules).
pub struct WarpKernel {
    engine: Engine,
    runtime: WorldlineRuntime,
    default_worldline: WorldlineId,
    /// Whether materialization output has been drained since the last step.
    /// Prevents returning stale data on consecutive drain calls.
    drained: bool,
    /// Registry metadata (injected at construction, immutable after).
    registry: RegistryInfo,
}

impl WarpKernel {
    /// Create a new kernel with a minimal empty engine.
    ///
    /// The engine has a single root node and no rewrite rules.
    /// Useful for testing the boundary or as a starting point.
    pub fn new() -> Self {
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
    pub fn with_engine(engine: Engine, registry: RegistryInfo) -> Self {
        let root = engine.root_key();
        let default_worldline = WorldlineId(root.warp_id.0);
        let mut runtime = WorldlineRuntime::new();
        #[allow(clippy::unwrap_used)]
        runtime
            .register_worldline(
                default_worldline,
                WorldlineState::from(engine.state().clone()),
            )
            .unwrap();
        #[allow(clippy::unwrap_used)]
        runtime
            .register_writer_head(WriterHead::with_routing(
                WriterHeadKey {
                    worldline_id: default_worldline,
                    head_id: make_head_id("default"),
                },
                PlaybackMode::Play,
                warp_core::InboxPolicy::AcceptAll,
                None,
                true,
            ))
            .unwrap();

        Self {
            engine,
            runtime,
            default_worldline,
            drained: true,
            registry,
        }
    }

    /// Build a [`HeadInfo`] from the current engine snapshot.
    fn head_info(&self) -> HeadInfo {
        let frontier = self
            .runtime
            .worldlines
            .get(&self.default_worldline)
            .expect("default worldline must exist");
        let snap = frontier
            .state()
            .last_snapshot()
            .cloned()
            .unwrap_or_else(|| self.engine.snapshot_for_state(frontier.state()));
        HeadInfo {
            tick: frontier.frontier_tick(),
            state_root: snap.state_root.to_vec(),
            commit_id: snap.hash.to_vec(),
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
                head: self.head_info(),
            });
        }

        let mut ticks_executed: u32 = 0;

        for _ in 0..budget {
            let records = SchedulerCoordinator::super_tick(&mut self.runtime, &mut self.engine)
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
            self.drained = false;
        }

        Ok(StepResponse {
            ticks_executed,
            head: self.head_info(),
        })
    }

    fn drain_view_ops(&mut self) -> Result<DrainResponse, AbiError> {
        // If already drained since the last step, return empty to avoid
        // returning stale data (the engine doesn't clear last_materialization).
        if self.drained {
            return Ok(DrainResponse {
                channels: Vec::new(),
            });
        }
        self.drained = true;

        let finalized = self
            .runtime
            .worldlines
            .get(&self.default_worldline)
            .expect("default worldline must exist")
            .state()
            .last_materialization();
        let channels: Vec<ChannelData> = finalized
            .iter()
            .map(|ch| ChannelData {
                channel_id: ch.channel.0.to_vec(),
                data: ch.data.clone(),
            })
            .collect();

        Ok(DrainResponse { channels })
    }

    fn get_head(&self) -> Result<HeadInfo, AbiError> {
        Ok(self.head_info())
    }

    fn snapshot_at(&mut self, tick: u64) -> Result<Vec<u8>, AbiError> {
        let tick_index = usize::try_from(tick).map_err(|_| AbiError {
            code: error_codes::INVALID_TICK,
            message: format!("tick {tick} exceeds addressable range"),
        })?;
        let frontier = self
            .runtime
            .worldlines
            .get(&self.default_worldline)
            .expect("default worldline must exist");
        let snap = self
            .engine
            .snapshot_at_state(frontier.state(), tick_index)
            .map_err(|e| AbiError {
                code: error_codes::INVALID_TICK,
                message: e.to_string(),
            })?;
        let head = HeadInfo {
            tick,
            state_root: snap.state_root.to_vec(),
            commit_id: snap.hash.to_vec(),
        };

        echo_wasm_abi::encode_cbor(&head).map_err(|e| AbiError {
            code: error_codes::CODEC_ERROR,
            message: e.to_string(),
        })
    }

    fn registry_info(&self) -> RegistryInfo {
        self.registry.clone()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use echo_wasm_abi::pack_intent_v1;

    #[test]
    fn new_kernel_has_zero_tick() {
        let kernel = WarpKernel::new();
        let head = kernel.get_head().unwrap();
        assert_eq!(head.tick, 0);
        assert_eq!(head.state_root.len(), 32);
        assert_eq!(head.commit_id.len(), 32);
    }

    /// Regression: init() must return real 32-byte hashes, not empty vecs.
    /// The init() WASM export calls get_head() before boxing the kernel.
    /// This test verifies the contract that get_head() upholds on a fresh kernel.
    #[test]
    fn fresh_kernel_head_has_real_hashes() {
        let kernel = WarpKernel::new();
        let head = kernel.get_head().unwrap();
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
        let mut kernel = WarpKernel::new();
        let result = kernel.step(0).unwrap();
        assert_eq!(result.ticks_executed, 0);
        assert_eq!(result.head.tick, 0);
    }

    #[test]
    fn step_executes_ticks() {
        let mut kernel = WarpKernel::new();
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
        let mut kernel = WarpKernel::new();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        let resp = kernel.dispatch_intent(&intent).unwrap();
        assert!(resp.accepted);
        assert_eq!(resp.intent_id.len(), 32);
    }

    #[test]
    fn dispatch_intent_duplicate() {
        let mut kernel = WarpKernel::new();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        let r1 = kernel.dispatch_intent(&intent).unwrap();
        let r2 = kernel.dispatch_intent(&intent).unwrap();
        assert!(r1.accepted);
        assert!(!r2.accepted);
        assert_eq!(r1.intent_id, r2.intent_id);
    }

    #[test]
    fn dispatch_then_step_changes_state() {
        let mut kernel = WarpKernel::new();
        let head_before = kernel.get_head().unwrap();

        let intent = pack_intent_v1(1, b"test-intent").unwrap();
        kernel.dispatch_intent(&intent).unwrap();

        let result = kernel.step(1).unwrap();
        assert_eq!(result.ticks_executed, 1);
        assert_eq!(result.head.tick, 1);
        assert_ne!(result.head.tick, head_before.tick);
    }

    #[test]
    fn drain_empty_on_fresh_kernel() {
        let mut kernel = WarpKernel::new();
        let drain = kernel.drain_view_ops().unwrap();
        assert!(drain.channels.is_empty());
    }

    #[test]
    fn execute_query_returns_not_supported() {
        let kernel = WarpKernel::new();
        let err = kernel.execute_query(0, &[]).unwrap_err();
        assert_eq!(err.code, error_codes::NOT_SUPPORTED);
    }

    #[test]
    fn snapshot_at_invalid_tick_returns_error() {
        let mut kernel = WarpKernel::new();
        let err = kernel.snapshot_at(999).unwrap_err();
        assert_eq!(err.code, error_codes::INVALID_TICK);
    }

    #[test]
    fn snapshot_at_valid_tick() {
        let mut kernel = WarpKernel::new();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        kernel.step(1).unwrap();
        let bytes = kernel.snapshot_at(0).unwrap();
        assert!(!bytes.is_empty());
        // Decode and verify it's valid CBOR with a HeadInfo
        let head: HeadInfo = echo_wasm_abi::decode_cbor(&bytes).unwrap();
        assert_eq!(head.tick, 0);
        assert_eq!(head.state_root.len(), 32);
    }

    #[test]
    fn snapshot_at_does_not_corrupt_live_state() {
        let mut kernel = WarpKernel::new();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        kernel.step(1).unwrap();

        // Capture live head before snapshot_at
        let head_before = kernel.get_head().unwrap();
        assert_eq!(head_before.tick, 1);

        // Replay to tick 0 — must NOT corrupt live state
        kernel.snapshot_at(0).unwrap();

        // Live head must be unchanged
        let head_after = kernel.get_head().unwrap();
        assert_eq!(head_after.tick, 1);
        assert_eq!(head_after.state_root, head_before.state_root);
        assert_eq!(head_after.commit_id, head_before.commit_id);

        // Subsequent step must work correctly on live state
        let intent2 = pack_intent_v1(2, b"second").unwrap();
        kernel.dispatch_intent(&intent2).unwrap();
        let result = kernel.step(1).unwrap();
        assert_eq!(result.ticks_executed, 1);
        assert_eq!(result.head.tick, 2);
    }

    #[test]
    fn drain_returns_empty_on_second_call() {
        let mut kernel = WarpKernel::new();
        let intent = pack_intent_v1(1, b"hello").unwrap();
        kernel.dispatch_intent(&intent).unwrap();
        kernel.step(1).unwrap();

        // First drain returns data (even if empty channels, the flag is set)
        let _d1 = kernel.drain_view_ops().unwrap();

        // Second drain without intervening step must return empty
        let d2 = kernel.drain_view_ops().unwrap();
        assert!(d2.channels.is_empty());
    }

    #[test]
    fn render_snapshot_returns_not_supported() {
        let kernel = WarpKernel::new();
        let err = kernel.render_snapshot(&[]).unwrap_err();
        assert_eq!(err.code, error_codes::NOT_SUPPORTED);
    }

    #[test]
    fn registry_info_has_abi_version() {
        let kernel = WarpKernel::new();
        let info = kernel.registry_info();
        assert_eq!(info.abi_version, ABI_VERSION);
        assert_eq!(info.codec_id.as_deref(), Some("cbor-canonical-v1"));
    }

    #[test]
    fn head_state_root_is_deterministic() {
        // Two fresh kernels should produce identical state roots
        let k1 = WarpKernel::new();
        let k2 = WarpKernel::new();
        let h1 = k1.get_head().unwrap();
        let h2 = k2.get_head().unwrap();
        assert_eq!(h1.state_root, h2.state_root);
        assert_eq!(h1.commit_id, h2.commit_id);
    }

    #[test]
    fn dispatch_invalid_intent_returns_invalid_intent_error() {
        let mut kernel = WarpKernel::new();

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
        );

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
        );
        assert_eq!(kernel.get_head().unwrap().tick, 0);
    }

    #[test]
    fn step_ignores_legacy_engine_inbox_state() {
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

        let mut kernel = WarpKernel::with_engine(
            engine,
            RegistryInfo {
                codec_id: None,
                registry_version: None,
                schema_sha256_hex: None,
                abi_version: ABI_VERSION,
            },
        );

        let result = kernel.step(1).unwrap();
        assert_eq!(result.ticks_executed, 0);
        assert_eq!(result.head.tick, 0);
    }

    #[test]
    fn step_produces_deterministic_commits() {
        let mut k1 = WarpKernel::new();
        let mut k2 = WarpKernel::new();

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
