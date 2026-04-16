<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
<!-- (C) James Ross FLYING*ROBOTS <https://github.com/flyingrobots> -->

# Determinism Claims v0.1

> **Version:** 0.1 | **Date:** 2026-03-06 | **Status:** Active
>
> This document defines what Echo's determinism guarantee means, what is proven,
> how it is proven, and what is explicitly out of scope.

## Executive Summary

Echo is a deterministic simulation engine. Given the same initial state and the
same sequence of inputs, Echo produces bit-identical outputs regardless of:

- **Host OS** (Linux, macOS, Alpine/musl)
- **Thread count** (1 to 32 worker threads)
- **Input order** (ingress permutation invariance)
- **Build environment** (WASM artifact reproducibility)

This document enumerates the specific claims, the evidence backing each claim,
and the known limits of the current proof.

## Scope

### In Scope (Proven)

| Domain             | What is deterministic                                                 |
| ------------------ | --------------------------------------------------------------------- |
| State transitions  | `state_root` hash after each tick                                     |
| Parallel execution | Serial vs N-thread `TickDelta` equivalence                            |
| Float operations   | Canonical encoding, NaN handling, zero normalization                  |
| Trigonometry       | LUT-based sin/cos with 0-ULP golden vector lock                       |
| PRNG               | Seeded XorShift with golden regression vectors                        |
| Serialization      | CBOR canonical encoding (integer widths, float widths, map key order) |
| WASM builds        | Dual-build SHA-256 hash identity                                      |

### Out of Scope (Not Yet Claimed)

| Domain                                     | Why                                                    |
| ------------------------------------------ | ------------------------------------------------------ |
| Cross-language parity (Rust/JS full stack) | Wesley type pipeline not yet integrated                |
| Time Travel / rewind                       | P3 milestone (depends on Time Semantics Lock)          |
| Snapshot/restore fuzz                      | T-9-1-2 planned but not yet implemented                |
| Network transport determinism              | Session layer is non-deterministic by design           |
| GPU rendering                              | Renderer is explicitly non-deterministic (visual only) |

## Claims Register

Each claim has a unique ID, a CI gate that enforces it, and a test artifact that
proves it. See `docs/determinism/CLAIM_MAP.yaml` for the machine-readable registry.

### DET-001: Static Nondeterminism Ban

> DET_CRITICAL crate paths contain zero matches for the banned pattern set
> (HashMap, HashSet, thread_rng, SystemTime, host callback hooks, network
> surfaces, etc.).

- **Gate:** G1 / DET-001 Static Inspection
- **Evidence:** `ban-nondeterminism.sh` ripgrep scan
- **Platforms:** Ubuntu (static analysis, platform-independent)

### DET-002 / DET-003: Float Canonicalization Parity

> Rust and JS implementations produce bit-identical outputs for all float
> canonicalization and serialization in the deterministic test corpus.

- **Gate:** G1 determinism (linux) / G1 determinism (macos)
- **Evidence:** `echo-scene-port` parity tests
- **Platforms:** Ubuntu, macOS

### DET-004: Trig Oracle Golden Vectors

> The deterministic trig oracle (sin/cos) produces bit-identical f32 outputs
> for 2048 golden vector angles, verified against a checked-in binary file.

- **Gate:** G1 determinism (linux) / G1 determinism (macos)
- **Evidence:** `trig_golden_vectors` test + `testdata/trig_golden_2048.bin`
- **Platforms:** Ubuntu, macOS
- **Error budget:** 0 ULP (exact bit match against golden file); <=16 ULP vs libm reference

### DET-005: Parallel Execution Equivalence

> The parallel execution engine (1, 2, 4, 8, 16, 32 workers) produces identical
> TickDelta output as serial execution for all parallel execution test scenarios.

- **Gate:** CI `Tests` job (warp-core test suite)
- **Evidence:** `parallel_exec.rs` — 10 tests covering serial/parallel equivalence, insertion order independence, sharded partitioning
- **Platforms:** Ubuntu, macOS (via G1), Alpine/musl

### SEC-001 through SEC-005: CBOR Decoder Security

> Malformed CBOR payloads (oversized, trailing bytes, truncated, bad version,
> invalid enum tags) are rejected before allocation.

- **Gate:** G2 decoder security tests
- **Evidence:** `echo-scene-codec` test suite, `sec-claim-map.json`
- **Platforms:** Ubuntu

### REPRO-001: WASM Build Reproducibility

> Dual WASM builds of `ttd-browser` produce bit-identical artifacts.

- **Gate:** G4 build reproducibility
- **Evidence:** SHA-256 hash comparison of two independent builds
- **Platforms:** Ubuntu (WASM target)

### PRF-001: Materialization Latency Stability

> MaterializationBus hot-path benchmark latency remains within Criterion noise
> threshold across runs.

- **Gate:** G3 perf regression (criterion)
- **Evidence:** Criterion benchmark output
- **Platforms:** Ubuntu

## Determinism Architecture

### How Determinism is Achieved

1. **No platform transcendentals.** All math (sin, cos, PRNG) uses checked-in
   lookup tables or pure-Rust implementations. `scripts/check_no_raw_trig.sh`
   enforces this in CI.

2. **No nondeterministic containers.** `HashMap`/`HashSet` are banned in
   DET_CRITICAL crates. `BTreeMap`/`BTreeSet` are used instead.
   `scripts/ban-nondeterminism.sh` enforces this.

3. **Canonical serialization.** CBOR encoding uses deterministic integer widths,
   float widths, and sorted map keys. No indefinite-length encodings.

4. **Parallel execution is order-independent.** The parallel scheduler partitions
   work into non-overlapping footprints, executes in parallel, then merges
   deltas in a canonical order. The merge is associative and commutative.

5. **Domain-separated hashing.** Every hash context uses a unique domain tag
   (`STATE_ROOT_V2`, `COMMIT_HASH_V2`, `RENDER_GRAPH_V1`, etc.) to prevent
   cross-domain collisions.

### Test Infrastructure

| Layer       | Tool                          | What it proves                                    |
| ----------- | ----------------------------- | ------------------------------------------------- |
| Static      | `ban-nondeterminism.sh`       | No banned patterns in critical paths              |
| Unit        | `deterministic_sin_cos_tests` | Trig oracle accuracy + golden bits                |
| Unit        | `trig_golden_vectors`         | 2048-angle bit-exact regression lock              |
| Unit        | `prng_golden_regression`      | PRNG output stability                             |
| Integration | `parallel_exec`               | Serial = Parallel across worker counts            |
| Integration | `parallel_determinism`        | Snapshot hash invariance under permutation        |
| Integration | `materialization_determinism` | Bus output confluence                             |
| System      | DIND harness                  | End-to-end scenario replay with hash verification |
| System      | DIND torture                  | N-rerun identical hash verification               |
| Build       | G4 dual-build                 | WASM binary reproducibility                       |

### DIND (Deterministic Ironclad Nightmare Drills)

The DIND harness replays recorded intent sequences through the full engine
pipeline and verifies that state hashes match golden files at every tick.

Scenarios cover: dense rewrites, error determinism, randomized order (with
permutation invariance), convergent rules (commutative operations), and
math/physics determinism.

**Torture mode** reruns each scenario N times (default: 20, configurable up to
100+) and asserts identical hashes across all runs. The `torture-100-reruns.sh`
script provides a turnkey 100-rerun repro for audit purposes.

## Repro Procedure

To reproduce the determinism proof locally:

```bash
# 1. Run the full test suite (includes parallel execution tests)
cargo test --workspace

# 2. Run the DIND suite (golden hash verification)
node scripts/dind-run-suite.mjs --mode run

# 3. Run 100-rerun torture (takes ~2 minutes)
scripts/torture-100-reruns.sh --runs 100

# 4. Verify trig oracle golden vectors
cargo test -p warp-core --test trig_golden_vectors

# 5. Verify PRNG golden regression
cargo test -p warp-core --features golden_prng --test prng_golden_regression
```

## Limits and Caveats

1. **Float precision is not infinite.** The trig oracle has <=16 ULP error vs
   `libm` reference. This is acceptable because (a) the error is deterministic
   across platforms, and (b) the golden vector file locks the exact output.

2. **DFix64 backend is experimental.** The `det_fixed` feature flag enables a
   fixed-point scalar backend. It passes CI but is not the default path.

3. **JavaScript parity is partial.** `echo-scene-port` parity tests verify
   float canonicalization, but full Wesley-generated type parity is not yet
   tested (planned for First Light milestone).

4. **Snapshot/restore fuzz is planned.** T-9-1-2 will add random-point
   snapshot/restore verification. Currently, snapshots are tested via WSC
   roundtrip in the CLI `verify` command.

5. **The determinism guarantee applies to the simulation core only.** Rendering,
   networking, and UI are explicitly non-deterministic.

## Version History

| Version | Date       | Changes                                                                              |
| ------- | ---------- | ------------------------------------------------------------------------------------ |
| 0.1     | 2026-03-06 | Initial claims document. 10 claims (DET-001..005, SEC-001..005, REPRO-001, PRF-001). |
