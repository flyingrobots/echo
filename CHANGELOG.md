<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Changelog

## Unreleased

## 2026-01-17 — MaterializationBus Phase 3 Complete

- Completed MaterializationBus Phase 3 implementation:
  - FinalizeReport pattern: `finalize()` never fails, returns `{channels, errors}`
  - Prevents silent data loss when one channel has StrictSingle conflict
  - 7 new SPEC Police tests for conflict preservation
- Added new modules to `warp-core/src/materialization`:
  - `emission_port.rs` — Port abstraction for emission routing
  - `reduce_op.rs` — Reduction operation definitions
  - `scoped_emitter.rs` — Scoped emission context management
- Added CI workflows:
  - `determinism.yml` — PR-gated determinism tests
  - `dind-cross-platform.yml` — Weekly cross-platform determinism proof (Linux x64/ARM64, Windows, macOS)
- Added tooling:
  - `cargo xtask dind` command with `run`, `record`, `torture`, and `converge` subcommands
- DIND mission 100% complete.

- Added `codec` module to `echo-wasm-abi`:
  - Deterministic binary codec (`Reader`/`Writer`) for length-prefixed LE scalars
  - Q32.32 fixed-point helpers (`fx_from_i64`, `fx_from_f32`, `vec3_fx_from_*`)
  - Overflow-safe conversions with saturation for out-of-range inputs
  - `Encode`/`Decode` traits for composable serialization
- Added `fixed` module to `warp-core`:
  - `Fx32` scalar type for Q32.32 fixed-point arithmetic
  - `Vec3Fx` 3D vector type with fixed-point components
  - Overflow-safe constructors with range validation
- Added WSC (Write-Streaming Columnar) snapshot format to `warp-core`:
  - Deterministic serialization of WARP graph state with zero-copy mmap deserialization
  - 8-byte aligned columnar layout for SIMD-friendly access
  - New modules: `wsc::{build, read, types, validate, view, write}`
  - Uses `bytemuck` for safe Pod/Zeroable transmutation (no `unsafe` code)
- Upgraded canonical state hash from V1 (u32 counts) to V2 (u64 counts) for future-proofing.
- Changed generated file convention from `generated/*.rs` to `*.generated.rs`.
- Updated pre-push hook to exclude `*.generated.rs` files from `missing_docs` lint.
- Added `#[repr(transparent)]` to ID newtypes (`NodeId`, `EdgeId`, `TypeId`, `WarpId`).
- Added `as_bytes()` method to `EdgeId` and `TypeId` for consistent byte access.
- Added `crates/echo-dind-harness` to the Echo workspace (moved from flyingrobots.dev).
- Added `crates/echo-dind-tests` as the stable DIND test app (no dependency on flyingrobots.dev).
- Moved DIND scenarios and generator scripts into `testdata/dind` and `scripts/`.
- Added convergence scopes in DIND manifest; `converge` now compares projected hashes.
- Documented convergence scope semantics and added a guarded `--scope` override.
- Wired determinism guard scripts and DIND PR suite into CI.
- Added spec for canonical inbox sequencing and deterministic scheduler tie-breaks.
- Added determinism guard scripts: `scripts/ban-globals.sh`, `scripts/ban-nondeterminism.sh`, and `scripts/ban-unordered-abi.sh`.
- Added `ECHO_ROADMAP.md` to capture phased plans aligned with recent ADRs.
- Removed legacy wasm encode helpers from `warp-wasm` (TS encoders are the protocol source of truth).
