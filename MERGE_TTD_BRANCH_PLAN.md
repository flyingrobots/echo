<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Plan: Incremental TTD Integration

This plan outlines the surgical decomposition of the `ttd-spec` branch into seven (7) sequential Pull Requests. Each step is gated by rigorous testing requirements to address criticisms regarding WASM FFI safety, determinism integrity, and data privacy.

---

## PR 1: Protocol & Core Hardening (The Foundation)

**Goal:** Establish the wire format and core state types with domain-separated hashing and header integrity.

### Changes (PR 1)

- Implement `EINT v2` and `TTDR v2` wire codecs in `crates/echo-session-proto`.
- Introduce `HashTriplet` and `WorldlineId` types in `crates/warp-core`.
- Update `TruthSink` with `clear_session(SessionId)` and isolated buffers.

### Extra Tests (PR 1)

- **Header Integrity Drill:** A test that attempts to truncate or swap headers between different `TTDR` versions and asserts that the BLAKE3 commitment fails.
- **Domain Separation Check:** Verify that hashing the same content as an `EINT` vs. a `TTDR` frame yields different hashes (prevents structural collision).
- **Decoder Fuzzer:** Use `cargo-fuzz` or a property-based test (Proptest) to feed 1,000,000 randomized bytes to `decode_ttdr_v2` ensuring no panics.
- **Cascading Cleanup Test:** Verify that `clear_session` not only removes the map entry but also drops all held `TruthFrame` vectors to prevent memory leaks.

---

## PR 2: Deterministic Scene Data (The Data Model)

**Goal:** Define the visual data model (`ScenePort`) and its stable serialization.

### Changes (PR 2)

- Implement `crates/echo-scene-port` (traits and types).
- Implement `crates/echo-scene-codec` (minicbor-based serialization).
- Implement `MockAdapter` for headless state tracking.

### Extra Tests (PR 2)

- **Float Parity Proof:** A test suite comparing `canonicalize_f32` against a reference JavaScript implementation (via a Node bridge or simulated logic) with 10,000 randomized floating-point vectors.
- **Atomic Scene Stress:** A multi-threaded test that calls `apply_scene_delta` concurrently with the same `(cursor_id, epoch)` to verify that state mutations are atomic and no silent data loss occurs.
- **Truncated CBOR Drill:** Verify that the decoder returns a structured error (not a panic) when fed incomplete `SceneDelta` payloads.

---

## PR 3: Robust Code Generation & Manifests

**Goal:** Establish build-time tooling to generate types from Wesley schemas with strict validation.

### Changes (PR 3)

- Implement `crates/echo-ttd-gen` (The syn/quote generator).
- Update `xtask` with `wesley sync` and `wesley check` commands.
- Vendor `ttd-manifest` JSON files.

### Extra Tests (PR 3)

- **Malicious IR Fixture:** Add a test fixture `malicious.json` containing illegal Rust identifiers (e.g., `"channels": [{"name": "enum; drop table users;"}]`) and verify the generator escapes or rejects them.
- **Cardinality Bounds:** A test that generates an IR with 10,001 channels and verifies the generator aborts with a "Cardinality Limit Exceeded" error.
- **Manifest Cross-Validator:** Implement a check in `xtask` that fails if a `TypeId` referenced in `contracts.json` is missing from the master `schema.json`.

---

## PR 4: Safe WASM FFI & Privacy

**Goal:** Implement the WASM engine with structured errors and data redaction.

### Changes (PR 4)

- Implement `crates/ttd-browser` bindings.
- Implement structured `TtdError` enum (replaces `JsError` strings).
- Implement `parse_policy` with proper whitespace normalization.

### Extra Tests (PR 4)

- **Redaction Verification:** A test asserting that `TruthFrame` projections for "Guest" sessions do not contain raw `AtomWrite` values for sensitive channels.
- **Policy Parser Matrix:** Test `parse_policy` with inputs like `"Reduce : Sum "`, `"reduce:sum"`, and `"Log"` to ensure all resolve deterministically or fail loudly.
- **Structured Error Round-trip:** Verify that a Rust `TtdError::SessionNotFound` is deserializable in JS as a structured object with a numeric code, not just a string.

---

## PR 5: Frontend Design System & Persistence

**Goal:** Establish the UI foundation with a documented design system.

### Changes (PR 5)

- Implement `apps/ttd-app` base and `index.css`.
- Document CSS variables in `docs/design-tokens.md`.
- Add `persist` middleware to `ttdStore` (Zustand).

### Extra Tests (PR 5)

- **A11y Audit:** Add a Playwright/Axe test to verify that the dark-mode palette (`--bg-primary` vs `--text-secondary`) meets WCAG AA contrast ratios.
- **Persistence Stress:** A test that simulates a page refresh and verifies the `ttdStore` restores the active Worldline and Cursor position from `localStorage`.

---

## PR 6: Real-World UI Binding

**Goal:** Remove all mocks and wire the real WASM engine to the UI.

### Changes (PR 6)

- Replace `useTtdEngine` mock with actual `ttd-browser` WASM import.
- Bind `Timeline` and `WorldlineTree` to live engine data.

### Extra Tests (PR 6)

- **Loopback Integration:** A Playwright test that performs a "Fork" in the UI and asserts the WASM engine's provenance store contains the new worldline hash.
- **Marker Data-Binding:** Verify that "Violation" markers on the timeline correspond to actual entries in the WASM `get_compliance()` report.

---

## PR 7: Final Documentation & CI Policy

**Goal:** Lock in the new standards.

### Changes (PR 7)

- Finalize `AGENTS.md` with "Drill Sergeant" instructions.
- Restore strict `markdownlint` and `SPDX` header checks.

### Extra Tests (PR 7)

- **CI Policy Guard:** A test that attempts to commit a file with a relaxed lint config and verifies the `pre-push` hook rejects it.
