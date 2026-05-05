<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Proof Core | **Priority:** P1

# Determinism Torture Harness

Prove that single-threaded and multi-threaded execution produce identical results. Snapshot/restore fuzz to catch nondeterminism in state serialization.

**Issues:** #190

Status: active backlog item. T-9-1-1 is complete; T-9-1-2 remains. Existing
determinism gates cover many related cases; this item is specifically for the
remaining 1-thread vs N-thread report and snapshot/restore fuzz gate.

---

## T-9-1-1: Implement 1-thread vs N-thread determinism harness

Status: complete.

Implementation status: complete. `warp-core` now has a test-only thread-count
determinism report harness that compares a 1-worker baseline against 1, 2, 4,
and 8 worker executions for existing parallel harness scenarios. The harness
reports deterministic JSON text without adding `serde_json` to `warp-core`,
compares `state_root`, `patch_digest`, computed `commit_id`, and scalar digest
per tick, and includes an injected divergence hook proving mismatched canonical
ordering is detected.

Completion evidence:

- `crates/warp-core/tests/determinism_thread_harness.rs` builds deterministic
  per-tick comparison reports over `common::parallel_harness()`.
- The default lane covers `F32Scalar` scenarios and zero-tick behavior.
- The `det_fixed` feature lane covers the `DFix64` backend.
- The injected divergence test mutates a worker-specific patch digest and
  reports the first divergent tick and mismatched fields.

**User Story:** As a release engineer, I want an automated harness that runs the same simulation single-threaded and multi-threaded and proves they produce identical state hashes so that I can gate releases on determinism.

**Requirements:**

- R1: Harness accepts a simulation scenario (initial state + input sequence) and runs it twice: once with 1 thread, once with N threads (configurable, default 4).
- R2: Compare `state_root` and `commit_id` at every tick; report the first divergent tick if any.
- R3: Run the scheduler's parallel drain path (existing Phase 5-6B parallel execution) and verify canonical ordering is maintained.
- R4: Support both `F32Scalar` and `DFix64` scalar backends.
- R5: Output a structured report (JSON) with per-tick comparison results.

**Acceptance Criteria:**

- [x] AC1: Harness passes with 0 divergences on the existing golden test scenarios from MS-1.
- [x] AC2: Harness passes for 1, 2, 4, and 8 thread configurations.
- [x] AC3: Harness passes for both `F32Scalar` and `DFix64` backends.
- [x] AC4: Intentionally breaking scheduler ordering (test hook) causes the harness to detect divergence.

**Definition of Done:**

- [x] Code reviewed locally
- [x] Tests pass locally
- [x] Documentation updated

**Scope:** Thread-count comparison harness, structured report, both scalar backends.
**Out of Scope:** GPU determinism; WASM vs native comparison (separate concern); performance benchmarking.

**Test Plan:**

- **Goldens:** All MS-1 golden vectors must pass through the harness with zero divergences.
- **Failures:** Simulation with an intentionally nondeterministic rule (detect and report); simulation with zero ticks (no-op, report "trivially deterministic").
- **Edges:** N=1 vs N=1 (should trivially pass); N=1 vs N=256 (extreme thread count).
- **Fuzz/Stress:** Run 100 random 50-tick simulations, each with random thread counts, verify zero divergences.

**Blocked By:** none
**Blocking:** T-9-1-2

**Est. Hours:** 5h
**Expected Complexity:** ~400 LoC

---

## T-9-1-2: Implement snapshot/restore fuzz

Status: complete.

Implementation status: complete. `warp-core` now has a snapshot/restore fuzz
integration harness that builds a 500-tick deterministic worldline, snapshots
materialized state at deterministic pseudo-random coordinates, restores from
canonical WSC bytes, replays the suffix from provenance, and compares the
restored run with the uninterrupted `state_root`. The harness renders a
deterministic JSON diagnostic report by hand and includes a corruption hook
that flips one stored snapshot byte to prove restore failure or detected
divergence.

Completion evidence:

- `crates/warp-core/tests/snapshot_restore_fuzz.rs` runs 50 snapshot/restore
  iterations over a 500-tick simulation.
- Edge cases include genesis snapshots, last-tick snapshots, and same-tick
  restore/compare windows.
- Corrupting a WSC edge byte either fails validation/restore or triggers a
  hash mismatch during suffix replay.
- The fuzz invocation asserts runtime stays under 60 seconds.

**User Story:** As a release engineer, I want fuzz testing that snapshots simulation state at random ticks, restores it, and continues execution — verifying the restored run matches the original so that I can catch nondeterminism in serialization/deserialization.

**Requirements:**

- R1: Fuzz loop: run a simulation, snapshot state at a randomly chosen tick T, restore from snapshot, continue to tick T+K, compare `state_root` at T+K with the uninterrupted run.
- R2: Vary the snapshot format (canonical encoding, debug encoding if applicable) to catch format-dependent bugs.
- R3: Run at least 50 iterations per fuzz invocation with different random snapshot points.
- R4: Report any divergence with full context: snapshot tick, restore tick, comparison tick, expected vs actual hash.

**Acceptance Criteria:**

- [x] AC1: 50 iterations with random snapshot points on a 500-tick simulation produce zero divergences.
- [x] AC2: Corrupting a single byte in the snapshot (test hook) causes the restore to fail or the comparison to detect divergence.
- [x] AC3: Fuzz runs in under 60 seconds for 50 iterations of a 500-tick simulation.
- [x] AC4: Report includes snapshot tick, restore tick, and hash comparison for each iteration.

**Definition of Done:**

- [x] Code reviewed locally
- [x] Tests pass locally
- [x] Documentation updated

**Scope:** Snapshot/restore fuzz loop, divergence detection, corruption detection.
**Out of Scope:** Snapshot performance optimization; snapshot compression; distributed snapshot.

**Test Plan:**

- **Goldens:** Each fuzz iteration's final hash must match the uninterrupted run's hash at the same tick.
- **Failures:** Corrupted snapshot (detected at restore or comparison); snapshot at tick 0 (genesis snapshot, valid).
- **Edges:** Snapshot at the last tick (restore and immediately compare); snapshot and restore at the same tick (no simulation between).
- **Fuzz/Stress:** 500 iterations on a 1000-tick simulation (extended run, CI nightly).

**Blocked By:** T-9-1-1
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~350 LoC
