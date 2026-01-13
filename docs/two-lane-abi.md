<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Two-Lane ABI Design (Control Plane vs. Data Plane)

Status: **Phase 1 Complete**

The Echo WASM ABI is split into two distinct logical lanes to separate stable, 
schema-driven application logic from the low-level mechanical plumbing of the kernel.

## 1. Control Plane (The "Handshake" Lane)

The Control Plane is used at boot time and during structural transitions to ensure 
the host and the kernel are speaking the same language.

### Registry Handshake
- **`get_registry_info()`**: Returns canonical CBOR bytes containing `schema_sha256_hex`, 
  `codec_id`, and `registry_version`.
- **Purpose**: The host verifies these fields against its own generated manifest 
  (from `wesley-generator-vue`) before calling any other functions.
- **Fail-Fast**: If the hash or version mismatches, the host refuses to mount 
  to prevent undefined behavior and ledger corruption.

### Metadata Accessors
- **`get_codec_id()`**, **`get_registry_version()`**, **`get_schema_sha256_hex()`**: 
  Helper accessors for debugging and runtime inspection.

## 2. Data Plane (The "Execution" Lane)

The Data Plane handles the high-frequency flow of state changes and information retrieval.

### Input Lane (Intents)
- **`dispatch_intent(bytes)`**: Enqueues an opaque, pre-validated command payload 
  into the kernel's inbox.
- **Envelope**: The host uses `encode_command(op_id, payload)` to wrap app-specific 
  data in a canonical CBOR structure that the kernel knows how to route.

### Output Lane (Projection)
- **`step(budget)`**: Advances the causal clock. Returns a `StepResult`.
- **`drain_view_ops()`**: Returns an array of `ViewOp`s emitted during the 
  preceding steps. These drive the host UI (e.g., toasts, navigation).

### Query Lane (Read-Only)
- **`execute_query(id, vars)`**: Executes a schema-validated, side-effect-free 
  lookup against the current graph.
- **Purity Guard**: The ABI layer enforces that only operations marked as `Query` 
  in the registry can be invoked here.

## 3. Ledger Reconciliation

The separation of lanes allows the kernel to reconcile the **Intent Log** (Data Plane) 
against the **Schema Version** (Control Plane) recorded in the provenance layer. 
Future versions will include the `registry_version` in every tick header to allow 
for multi-version playback.
