<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Plan: Incremental TTD Integration

This plan outlines the surgical decomposition of the `ttd-spec` branch into seven (7) sequential Pull Requests. Each step is gated by rigorous testing requirements to address criticisms regarding WASM FFI safety, determinism integrity, and data privacy.

---

## PR 1: Protocol & Core Hardening (The Foundation) ✅ [DONE]

**Goal:** Establish the wire format and core state types with domain-separated hashing and header integrity.

### Changes (PR 1)

- Implement `EINT v2` and `TTDR v2` wire codecs in `crates/echo-session-proto`.
- Introduce `HashTriplet` and `WorldlineId` types in `crates/warp-core`.
- Update `TruthSink` with `clear_session(SessionId)` and isolated buffers.

### Extra Tests (PR 1)

- **Header Integrity Drill:** Verified BLAKE3 commitment failure on truncated/swapped headers.
- **Domain Separation Check:** Verified distinct hashes for `EINT` vs. `TTDR` frames.
- **Decoder Fuzzer:** `Proptest` integrated to feed randomized bytes to decoders.
- **Cascading Cleanup Test:** Verified memory safety in `clear_session`.

---

## PR 2: Deterministic Scene Data (The Data Model) ✅ [DONE]

**Goal:** Define the visual data model (`ScenePort`) and its stable serialization.

### Changes (PR 2)

- Implement `crates/echo-scene-port` (traits and types).
- Implement `crates/echo-scene-codec` (minicbor-based serialization).
- Implement `MockAdapter` for headless state tracking.

### Extra Tests (PR 2)

- **Float Parity Proof:** Bit-exact parity verified against Node.js implementation with 10,000 randomized vectors.
- **Atomic Scene Stress:** Multi-threaded concurrency test for `apply_scene_delta`.
- **Truncated CBOR Drill:** Structured error handling for incomplete `SceneDelta` payloads.

---

## PR 3: Robust Code Generation & Manifests ✅ [DONE]

**Goal:** Establish build-time tooling to generate types from Wesley schemas with strict validation.

### Changes (PR 3)

- Enhanced `echo-wesley-gen` with `--no-std` and `--minicbor` support.
- Hardened `echo-registry-api` for WASM guest usage.
- Mapped GraphQL `ID` to `[u8; 32]` for zero-allocation kernel handling.

### Extra Tests (PR 3)

- **Minicbor Artifact Test:** Verified generated `Encode`/`Decode` traits with explicit field indexing.
- **no_std Integration Test:** Verified that generated artifacts compile in pure `#![no_std]` environments.

---

## PR 4: Safe WASM FFI & Privacy ✅ [DONE]

**Goal:** Implement the WASM engine with structured errors and data redaction.

### Changes (PR 4)

- Implemented `PrivacyMask` for field-level redaction (Public, Pseudonymized, Private).
- Added opaque `SessionToken` to prevent raw pointer leakage across JS/WASM.
- Upgraded workspace to `thiserror` v2.0 for `no_std` error derives.

### Extra Tests (PR 4)

- **Redaction Verification:** Verified that `Pseudonymized` values are hashed/truncated.
- **Lifecycle Test:** Verified session opening/closing and token validation.

---

## PR 5: Frontend Design System & Persistence ✅ [DONE]

**Goal:** Establish the UI foundation with a documented design system.

### Changes (PR 5)

- Restored `apps/ttd-app` and supporting packages from `ttd-spec` branch.
- Established `pnpm-workspace.yaml` for multi-package frontend management.
- Hardened TypeScript protocol bridge (fixed circular dependencies and missing imports).

### Extra Tests (PR 5)

- **Production Build:** Verified successful `pnpm build` with Vite/React/TypeScript.

---

## PR 6: Real-World UI Binding ✅ [DONE]

**Goal:** Remove all mocks and wire the real WASM engine to the UI.

### Changes (PR 6)

- Restored `ttd-browser` WASM engine and integrated it into `useTtdEngine` hook.
- Re-exported `compute_emissions_digest` in `warp-core` for browser consumption.
- Unified TypeScript interfaces with `wasm-bindgen` snake_case API.

### Extra Tests (PR 6)

- **WASM Integration:** Verified `ttd-app` build includes valid `.wasm` assets and initializes correctly.

---

## PR 7: Final Documentation & CI Policy ⚠️ [IN PROGRESS]

**Goal:** Lock in the new standards.

### Changes (PR 7)

- Finalize `docs/architecture-outline.md` with TTD/Scene Port status.
- Update `DIND-MISSION.md` with completion status.
- Finalize `AGENTS.md` with "Drill Sergeant" instructions.
- Restore strict `markdownlint` and `SPDX` header checks.

### Extra Tests (PR 7)

- **CI Policy Guard:** Verify that `pre-push` hooks correctly enforce formatting and header rules.
