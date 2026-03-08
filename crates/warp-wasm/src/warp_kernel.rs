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
    inbox, make_node_id, make_type_id, Engine, EngineBuilder, GraphStore, IngestDisposition,
    NodeRecord, SchedulerKind,
};

/// App-agnostic kernel wrapping a `warp-core::Engine`.
///
/// Constructed via [`WarpKernel::new`] (default empty engine) or
/// [`WarpKernel::with_engine`] (pre-configured engine with rules).
pub struct WarpKernel {
    engine: Engine,
    /// Tracks the number of committed ticks for the current head.
    tick_count: u64,
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

        let mut engine = EngineBuilder::new(store, root)
            .scheduler(SchedulerKind::Radix)
            .workers(1) // WASM is single-threaded
            .build();

        // Register system inbox rule (required for dispatch_next_intent).
        // This is safe to unwrap: fresh engine has no rules registered.
        #[allow(clippy::unwrap_used)]
        engine.register_rule(inbox::ack_pending_rule()).unwrap();

        Self {
            engine,
            tick_count: 0,
            drained: true,
            registry: RegistryInfo {
                codec_id: Some("cbor-canonical-v1".into()),
                registry_version: None,
                schema_sha256_hex: None,
                abi_version: ABI_VERSION,
            },
        }
    }

    /// Create a kernel with a pre-configured engine and registry metadata.
    ///
    /// Use this to inject app-specific rewrite rules and schema metadata.
    /// The `sys/ack_pending` system rule is registered automatically if absent
    /// (required by [`KernelPort::dispatch_intent`]).
    pub fn with_engine(mut engine: Engine, registry: RegistryInfo) -> Self {
        // Ensure the system inbox rule is present. If the caller already
        // registered it, register_rule returns DuplicateRuleName — ignore.
        let _ = engine.register_rule(inbox::ack_pending_rule());

        Self {
            engine,
            tick_count: 0,
            drained: true,
            registry,
        }
    }

    /// Build a [`HeadInfo`] from the current engine snapshot.
    fn head_info(&self) -> HeadInfo {
        let snap = self.engine.snapshot();
        HeadInfo {
            tick: self.tick_count,
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

        match self.engine.ingest_intent(intent_bytes) {
            Ok(disposition) => {
                let (accepted, intent_id) = match disposition {
                    IngestDisposition::Accepted { intent_id } => (true, intent_id),
                    IngestDisposition::Duplicate { intent_id } => (false, intent_id),
                };
                Ok(DispatchResponse {
                    accepted,
                    intent_id: intent_id.to_vec(),
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
            let tx = self.engine.begin();

            // Dispatch one pending intent for this tick (if any).
            // The ack_pending rewrite queued here executes during commit,
            // so we must NOT loop — the pending edge is still visible until
            // the transaction commits.
            match self.engine.dispatch_next_intent(tx) {
                Ok(_) => {}
                Err(e) => {
                    self.engine.abort(tx);
                    return Err(AbiError {
                        code: error_codes::ENGINE_ERROR,
                        message: format!("dispatch failed: {e}"),
                    });
                }
            }

            match self.engine.commit(tx) {
                Ok(_snapshot) => {
                    self.tick_count += 1;
                    ticks_executed += 1;
                    self.drained = false;
                }
                Err(e) => {
                    // abort is safe here: failed commit leaves the tx in
                    // live_txs, so abort cleans it up. If commit had
                    // succeeded the tx would already be removed, making
                    // abort a harmless no-op (TxId is Copy).
                    self.engine.abort(tx);
                    return Err(AbiError {
                        code: error_codes::ENGINE_ERROR,
                        message: format!("commit failed: {e}"),
                    });
                }
            }
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

        let finalized = self.engine.last_materialization();
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

        // Save current state — jump_to_tick overwrites engine state with a
        // replayed state. We must restore afterward to keep the live engine
        // consistent with tick_count and subsequent operations.
        let saved_state = self.engine.state().clone();

        self.engine.jump_to_tick(tick_index).map_err(|e| {
            // Restore even on error (jump_to_tick may have partially mutated).
            *self.engine.state_mut() = saved_state.clone();
            AbiError {
                code: error_codes::INVALID_TICK,
                message: e.to_string(),
            }
        })?;

        let snap = self.engine.snapshot();
        let head = HeadInfo {
            tick,
            state_root: snap.state_root.to_vec(),
            commit_id: snap.hash.to_vec(),
        };

        // Restore live state.
        *self.engine.state_mut() = saved_state;

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
        let result = kernel.step(3).unwrap();
        assert_eq!(result.ticks_executed, 3);
        assert_eq!(result.head.tick, 3);
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
        // State root changes after ingesting intent and stepping
        // (the intent creates inbox nodes in the graph)
        assert_ne!(result.head.state_root, head_before.state_root);
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
        // Step to create tick 0 in the ledger
        kernel.step(2).unwrap();
        // Now tick 0 exists in the ledger
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
        // Step without intents — intent ingestion modifies state outside the
        // patch system, so jump_to_tick cannot replay ticks that depend on
        // ingested intents. This is a known engine limitation.
        kernel.step(3).unwrap();

        // Capture live head before snapshot_at
        let head_before = kernel.get_head().unwrap();
        assert_eq!(head_before.tick, 3);

        // Replay to tick 0 — must NOT corrupt live state
        kernel.snapshot_at(0).unwrap();

        // Live head must be unchanged
        let head_after = kernel.get_head().unwrap();
        assert_eq!(head_after.tick, 3);
        assert_eq!(head_after.state_root, head_before.state_root);
        assert_eq!(head_after.commit_id, head_before.commit_id);

        // Subsequent step must work correctly on live state
        let result = kernel.step(1).unwrap();
        assert_eq!(result.ticks_executed, 1);
        assert_eq!(result.head.tick, 4);
    }

    #[test]
    fn drain_returns_empty_on_second_call() {
        let mut kernel = WarpKernel::new();
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
    fn with_engine_auto_registers_ack_pending() {
        // with_engine must register sys/ack_pending even if the caller omits it.
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
        // Engine has NO rules — with_engine should add ack_pending.
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
        // step would fail with ENGINE_ERROR if ack_pending wasn't registered.
        let result = kernel.step(1).unwrap();
        assert_eq!(result.ticks_executed, 1);
    }

    #[test]
    fn with_engine_tolerates_pre_registered_ack_pending() {
        // If the caller already registered ack_pending, with_engine must not fail.
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
        engine.register_rule(inbox::ack_pending_rule()).unwrap();

        // with_engine should silently ignore the duplicate.
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
