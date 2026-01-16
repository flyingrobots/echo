<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP Demo Roadmap (Phase 1 Targets)

This document captures the interactive demos and performance milestones we want to hit as we implement the Rust-based WARP runtime. Each demo proves a key property of Echo’s deterministic multiverse architecture.

---

## Demo 1: Deterministic Netcode

**Goal:** Show two instances running locally in lockstep and prove graph hash equality every frame.

- Two Echo instances (no network) consume identical input streams generated from a shared seed (deterministic RNG feeding input script).
- Each frame serializes the world graph in canonical order (sorted node/edge IDs, component payload bytes) and hashes it with BLAKE3 to produce the “frame hash”.
- Inspectors display the frame hashes side-by-side and flag divergence immediately. Success = 100% equality across a 10 000-frame run.
- Determinism safeguards: freeze wall clock, mock OS timers, clamp floating-point math to deterministic fixed-point helpers, forbid nondeterministic APIs.
- Output artifact: JSON trace (`frame`, `hash`, `inputs_consumed`) plus a screenshot/video for the showcase.

## Demo 2: Scheduler Rewrite Benchmark

**Goal:** Benchmark the rewrite executor under scripted workloads.

- Criterion-based benches exercise flat, chained, branching, and timeline-flush scenarios (mirrors `docs/scheduler-benchmarks.md`).
- Success criteria: median tick time < 0.5 ms for toy workload (100 entities, 10 rules); percentile tails recorded.
- Bench harness outputs JSON summaries (mean, median, std dev) consumed by the inspector.
- Deterministic PRNG seeds recorded so benches are reproducible across CI machines.

## Demo 3: Timeline Fork/Merge Replay

**Goal:** Demonstrate branching timelines, paradox detection, and canonical merges.

- Start from a baseline snapshot, fork into three branches with scripted rewrites, deliberately introduce a conflict on one branch.
- Inspector view shows divergence tree, entropy deltas, and paradox quarantine in real time.
- Success criteria: merge replay produces the documented canonical hash, paradox branch quarantined with deterministic error log, entropy metrics trend as expected.
- Deliverable: recorded replay plus JSON report showing branch IDs, merge decisions, and resulting hashes.

## Demo 4: Rhai Live Coding Loop

**Goal:** Prove Rhai bindings support hot reload without breaking determinism.

- Script registers a system that increments a component each tick; developer edits Rhai code mid-run via CLI hot-reload.
- Engine stages rewrite intents from Rhai through the FFI; after reload, replay the prior ticks to confirm deterministic equivalence.
- Success: frame hashes before/after reload identical when replayed from the same snapshot; inspector shows live diff of system graphs.
- Includes integration test capturing reload latency budget (< 50 ms) and ensuring queued rewrites survive reload boundary.

## Demo 5: Confluence Sync Showcase

**Goal:** Synchronise two peers via rewrite transactions, demonstrating deterministic convergence.

- Peer A applies scripted rewrites while offline, then pushes transactions to Peer B via the Confluence protocol.
- Both peers compute snapshot hashes before/after sync; success when hashes converge with zero conflicts.
- Includes failure injection (duplicate transaction, out-of-order delivery) to show deterministic resolution path.
- Inspector UI plots sync throughput (transactions/sec) and latency.

## Success Criteria Summary

- **Frame Hash Integrity:** For Demo 1 and Demo 3, identical BLAKE3 hashes across peers/branches every tick. Any discrepancy fails the demo.
- **Input Stream Discipline:** Inputs recorded as timestamped events with deterministic seeds. Replay harness reuses the same log to verify determinism.
- **Floating-Point Policy:** All demos rely on fixed-point math or deterministic float wrappers; document configuration in README.
- **Performance Targets:**
  - Demo 1: tick time ≤ 2 ms on reference hardware (M2 Pro / 32 GB).
  - Demo 2: criterion bench median ≤ 0.5 ms; 99th percentile ≤ 1.0 ms.
  - Demo 5: sync 10 000 transactions in under 2 s with zero conflicts.

## Roadmap / Dependencies

| Phase | Demo Coverage | Dependencies |
| ----- | ------------- | ------------- |
| 1A    | Demo 2 harness scaffolding | Criterion setup, synthetic rewrite fixtures |
| 1B    | Demo 1 prototype (local hash) | Motion rewrite spike, snapshot hashing |
| 1C    | Demo 4 Rhai API | `warp-ffi` bindings, hot-reload CLI |
| 1D    | Demo 3 timeline tooling | Branch tree diff viewer, entropy metrics |
| 1E    | Demo 5 networking | Confluence transaction protocol, replay verification |
| 1F    | Demo dashboards | Inspector frame overlays, JSON ingestion |


**Prerequisites:** BLAKE3 hashing utilities, deterministic PRNG module, snapshot serialiser, inspector graph viewer, CI runners with wasm/criterion toolchains.


**Timeline:**
- Milestone Alpha (end 1B): Demo 1 frame-hash prototype + Demo 2 toy bench executed manually.
- Milestone Beta (end 1D): Demos 1–3 automated in CI with golden outputs.
- Milestone GA (end 1F): Full demo suite (all five) runnable via `cargo xtask demo` and published as part of release notes.
