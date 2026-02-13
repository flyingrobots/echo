<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WASM Runtime Integration

> **Milestone:** [First Light](README.md) | **Priority:** P1 | **Repo:** Echo

Wire warp-wasm exports to real engine logic. The warp-wasm crate currently ships placeholder stubs (`dispatch_intent`, `step`, `drain_view_ops`, `get_head`, `snapshot_at`, `render_snapshot`) that return empty bytes. This feature replaces those stubs with live Engine plumbing so the browser can drive a full tick loop.

## T-4-1-1: Wire Engine lifecycle behind wasm-bindgen exports

**User Story:** As a web developer, I want the WASM module to expose a real Engine instance so that I can drive tick execution from JavaScript.

**Requirements:**

- R1: Instantiate an `Engine` (via `EngineBuilder`) in a module-scoped `RefCell` or `OnceCell` inside warp-wasm, gated behind an `init()` export.
- R2: `dispatch_intent(bytes)` must call `engine.ingest_intent()` and return the `IngestDisposition` encoded as CBOR via `echo-wasm-abi`.
- R3: `step(budget)` must call `engine.tick()` up to `budget` times, returning a CBOR-encoded `StepResult` containing tick count and commit status.
- R4: `get_head()` must return the latest `Snapshot` fields (tick, state_root, commit_id) as CBOR bytes.

**Acceptance Criteria:**

- [ ] AC1: `init()` export constructs an Engine with at least one demo rewrite rule (motion rule).
- [ ] AC2: Round-trip test: JS calls `dispatch_intent` then `step(1)` and receives a non-empty `StepResult`.
- [ ] AC3: `get_head()` returns a valid CBOR payload whose `state_root` changes after a step that applies a rewrite.
- [ ] AC4: Calling `step` before `init` returns an error CBOR (not a panic).

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Engine instantiation, intent ingestion, tick execution, head query. All through existing wasm-bindgen exports.
**Out of Scope:** Snapshot-at (time travel), render_snapshot, query execution. Multi-warp Engine configurations.

**Test Plan:**

- **Goldens:** CBOR golden vectors for `get_head()` after genesis init (zero-tick snapshot).
- **Failures:** `step` before `init` returns structured error. Malformed intent bytes rejected gracefully (no WASM trap).
- **Edges:** `step(0)` is a no-op returning current head. `step(u32::MAX)` runs until no pending rewrites remain.
- **Fuzz/Stress:** Proptest with random intent payloads; engine must not panic.

**Blocked By:** none
**Blocking:** T-4-1-2, T-4-2-1

**Est. Hours:** 6h
**Expected Complexity:** ~250 LoC

---

## T-4-1-2: Snapshot and ViewOp drain exports

**User Story:** As a web developer, I want to drain ViewOps and request snapshots at specific ticks so that I can render simulation state in the browser.

**Requirements:**

- R1: `drain_view_ops()` must return all materialized `ViewOp` frames accumulated since the last drain, CBOR-encoded.
- R2: `snapshot_at(tick)` must replay from the provenance store and return the full `Snapshot` at the requested tick as CBOR bytes.
- R3: `render_snapshot(bytes)` must accept a Snapshot and produce ViewOps suitable for visualization.

**Acceptance Criteria:**

- [ ] AC1: After `step(1)` that fires a motion rule, `drain_view_ops()` returns at least one ViewOp frame.
- [ ] AC2: Calling `drain_view_ops()` twice in succession returns an empty array on the second call.
- [ ] AC3: `snapshot_at(0)` returns the genesis snapshot matching the initial `get_head()`.
- [ ] AC4: `snapshot_at(tick)` for a future tick returns an error payload.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** ViewOp drain, snapshot-at replay, render_snapshot projection.
**Out of Scope:** Streaming ViewOps over WebSocket. Incremental snapshot diffs.

**Test Plan:**

- **Goldens:** ViewOp CBOR output for a single-motion-rule tick golden scenario.
- **Failures:** `snapshot_at` for non-existent tick returns structured CBOR error. `render_snapshot` with garbage bytes returns error.
- **Edges:** `drain_view_ops` on fresh engine (no ticks) returns empty array. `snapshot_at(current_tick)` matches `get_head`.
- **Fuzz/Stress:** Rapid alternation of `step` and `drain_view_ops` with random budgets.

**Blocked By:** T-4-1-1
**Blocking:** T-4-2-2

**Est. Hours:** 5h
**Expected Complexity:** ~200 LoC

---

## T-4-1-3: JS/WASM memory bridge and error protocol

**User Story:** As a web developer, I want a clean TypeScript API wrapper around the raw WASM exports so that I do not deal with raw Uint8Array encoding/decoding.

**Requirements:**

- R1: Publish a `@echo/wasm-bridge` TypeScript package (or `web/` directory) that wraps each wasm-bindgen export with typed async functions.
- R2: CBOR decode all return payloads into typed TypeScript objects.
- R3: Errors from the WASM side (structured CBOR with `{ error: string, code: number }`) are surfaced as typed `EchoWasmError` exceptions.
- R4: Memory cleanup: large Uint8Array returns are freed on the WASM linear memory side after JS copies them.

**Acceptance Criteria:**

- [ ] AC1: TypeScript wrapper compiles with `tsc --strict --noEmit`.
- [ ] AC2: `bridge.init()` / `bridge.step(n)` / `bridge.getHead()` type signatures match the CBOR schema.
- [ ] AC3: An intentional error (e.g., step before init) throws `EchoWasmError` with a `.code` property.
- [ ] AC4: No WASM linear memory growth beyond O(1) per bridge call (alloc/free pairs verified via instrumentation test).

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** TypeScript wrapper, CBOR decode, error mapping, basic memory lifecycle.
**Out of Scope:** React/Svelte bindings. Worker thread offloading. Streaming APIs.

**Test Plan:**

- **Goldens:** Snapshot: bridge.getHead() output matches known JSON fixture after genesis init.
- **Failures:** Every WASM error code has a corresponding `EchoWasmError` subclass or code. Unrecognized CBOR returns a generic decode error.
- **Edges:** Empty intent bytes. Zero-length ViewOp drain. Concurrent calls (JS is single-threaded but test re-entrant safety).
- **Fuzz/Stress:** 1000 rapid step/drain cycles; monitor WASM memory (must not monotonically grow beyond blob cache).

**Blocked By:** T-4-1-1
**Blocking:** T-4-2-1, T-4-4-3

**Est. Hours:** 5h
**Expected Complexity:** ~300 LoC (TypeScript)

---
