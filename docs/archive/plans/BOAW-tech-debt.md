<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BOAW Roadmap: Phase 6B → Phase 9

**Created:** 2026-01-20
**Status:** AWAITING APPROVAL
**Context:** Post-Phase 6B integration — cleanup, guardrails, and planning

---

## Classification Rubric

| If it...                             | Then it's...                           |
| ------------------------------------ | -------------------------------------- |
| Unblocks a phase                     | **Roadmap** (Tiers 1-3)                |
| Reduces risk or prevents regressions | **Guardrail** (Tier 0.5)               |
| Improves performance                 | **Perf Gate** (only after measurement) |
| Is unused code                       | **Delete immediately** (Tier 0)        |

---

## Tier 0: Cleanup (Today)

_Dead code and doc drift. Do immediately after merge._

### 0.1 Delete `emit_view_op_delta()`

| Field          | Value                                         |
| -------------- | --------------------------------------------- |
| **Location**   | `crates/echo-dind-tests/src/rules.rs:600-648` |
| **Call Sites** | 0                                             |
| **Risk**       | None                                          |

**Why:** Deprecated function using non-deterministic `delta.len()` sequencing.
Replaced by `emit_view_op_delta_scoped()`. Keeping it risks copy-paste of broken pattern.

### 0.2 Delete `execute_parallel_stride()` + Feature Gate

| Field          | Value                                       |
| -------------- | ------------------------------------------- |
| **Location**   | `crates/warp-core/src/boaw/exec.rs:176-207` |
| **Call Sites** | 3 (1 conditional, 2 Phase 6A tests)         |
| **Risk**       | Low                                         |

**Why:** Phase 6A stride execution superseded by Phase 6B sharded execution.
Feature-gated behind `parallel-stride-fallback`. Adds maintenance burden.

**Steps:**

1. Delete Phase 6A equivalence tests (`boaw_parallel_exec.rs:286-365`)
2. Remove stride fallback conditional (`exec.rs:67-83`)
3. Delete `execute_parallel_stride()` function
4. Remove `parallel-stride-fallback` feature from `Cargo.toml`

### 0.3 Doc Accuracy Pass

Verify these are still accurate post-merge:

- [ ] `TECH-DEBT-BOAW.md` — mark Phase 6B items complete
- [ ] `ADR-0007-BOAW-Storage.md` — phase status markers
- [ ] `CHANGELOG.md` — PR #257 merge recorded

---

## Tier 0.5: Correctness Guardrails (This Week)

_Tests we can land now + baseline measurements. Reduces future regression risk._

### 0.5.1 Activate Passing Tests

Some `#[ignore]` tests may now pass after Phase 6B. Audit and activate:

| Test File             | Check For                                     |
| --------------------- | --------------------------------------------- |
| `boaw_determinism.rs` | Any tests that only needed parallel execution |
| `boaw_end_to_end.rs`  | Full integration tests                        |
| `boaw_footprints.rs`  | T3.1 already passes; verify others            |

### 0.5.2 WarpOpKey Invariant Test

Verify `WarpOpKey` ordering is stable and exercised:

- Canonical sort order matches spec
- No collisions under realistic workloads
- Public API (`sort_key()`) works for external verification

### 0.5.3 Initial Benchmark Baseline

**Purpose:** Prove parallelism delivers measurable wins. Capture baseline so future
phases don't accidentally regress performance.

**Scope:** Minimal, not a full optimization suite.

| Benchmark                 | What It Measures                   |
| ------------------------- | ---------------------------------- |
| `parallel_vs_serial_10`   | 10 rewrites: parallel speedup      |
| `parallel_vs_serial_100`  | 100 rewrites: parallel speedup     |
| `parallel_vs_serial_1000` | 1000 rewrites: parallel speedup    |
| `shard_distribution`      | Are rewrites spread across shards? |

**Location:** `benches/boaw_baseline.rs` (new file)

**Success Criteria:**

- Parallel ≥ serial for n ≥ 100 (no regression)
- Document baseline numbers in `docs/notes/boaw-perf-baseline.md`

---

## Tier 1: Phase 7 — Forking

_Multi-parent commits and prerequisites. ~2-3 weeks._

### Prerequisites (Enable Forking)

| Component                        | Tests Unblocked | Notes                                                                   |
| -------------------------------- | --------------- | ----------------------------------------------------------------------- |
| **OpenPortal scheduling (T7.1)** | 4               | Scheduler tracks new warps; enforces "no same-tick writes to new warps" |
| **DeltaView**                    | 6               | Overlay + base resolution during execution                              |
| ~~**FootprintGuard**~~           | 3               | ✅ Done (44aebb0, 0d0231b)                                              |
| **SnapshotBuilder wiring**       | 1               | Connect builder to test harness                                         |

### Core Forking Work

| Component                     | Description                      |
| ----------------------------- | -------------------------------- |
| Multi-parent commit structure | Commit can have 0..n parents     |
| Worldline DAG                 | Track branch/merge topology      |
| Parent addressing             | Reference parents by commit hash |

### Tests Unblocked: 14

```text
boaw_openportal_rules.rs    — 4 tests (T7.1)
boaw_cow.rs                 — 6 tests (DeltaView)
boaw_footprints.rs          — 3 tests (FootprintGuard)
boaw_determinism.rs         — 1 test (SnapshotBuilder)
```

---

## Tier 2: Phase 8 — Collapse/Merge

_Deterministic multi-parent reconciliation. ~2-3 weeks. Requires Phase 7._

### Merge Components

| Component                     | Description                                      |
| ----------------------------- | ------------------------------------------------ |
| **Typed merge registry**      | Per-type: Sensitivity, MergeBehavior, Disclosure |
| **Merge regimes**             | Commutative (CRDT), LWW, ConflictOnly            |
| **Conflict artifacts**        | Deterministic, contains only hashes (no secrets) |
| **Canonical parent ordering** | Sort by `commit_hash` for order-dependent merges |
| **Presence policies**         | delete-wins (default), add-wins, LWW             |

### Tests Unblocked: 10

```text
boaw_merge.rs — all 10 tests
├── t6_1: Commutative merge parent-order invariance
├── t6_2: Canonical ordering for order-dependent
├── t6_3: Conflict artifact determinism
├── merge_regime_crdt_like_is_preferred
├── merge_regime_lww_with_canonical_order
├── presence_policy_delete_wins
├── presence_policy_add_wins
├── conflict_artifact_is_first_class_and_deterministic
└── conflict_artifact_contains_no_secrets
```

---

## Tier 3: Phase 9 — Privacy Claims

_Ledger-safe provenance. ~2-3 weeks. Requires Phase 8._

### Privacy Components

| Component                 | Description                                             |
| ------------------------- | ------------------------------------------------------- |
| **Atom type registry**    | Sensitivity (Public/Private/ForbiddenInLedger)          |
| **Mind mode enforcement** | Reject ForbiddenInLedger atoms in ledger                |
| **ClaimRecord structure** | claim_key, scheme_id, statement_hash, commitment, proof |
| **Commitment safety**     | Pepper-based hashing (dictionary-safe)                  |
| **ZK proof merging**      | Verify during collapse; quarantine invalid              |
| **Diagnostics mode**      | Richer introspection for trusted debugging              |

### Tests Unblocked: 9

```text
boaw_privacy.rs — all 9 tests
├── t7_1: Mind mode forbids ForbiddenInLedger
├── t7_2: Invalid proofs quarantined
├── t7_3: Conflicting valid claims → artifact
├── t7_4: Commitment dictionary-safe with pepper
├── atom_type_declares_sensitivity
├── atom_type_declares_merge_behavior
├── atom_type_declares_disclosure_policy
├── claim_record_is_canonical
└── diagnostics_mode_allows_richer_introspection
```

---

## Perf Gate (Recurring)

_Run at end of each tier. Catch regressions early._

### What to Measure

| Metric                      | Baseline (Tier 0.5) | Gate Threshold             |
| --------------------------- | ------------------- | -------------------------- |
| Parallel vs serial (n=100)  | TBD                 | No regression (≥ baseline) |
| Parallel vs serial (n=1000) | TBD                 | No regression (≥ baseline) |
| Merge time (n ops)          | TBD                 | < 2x baseline              |
| Snapshot build time         | TBD                 | < 2x baseline              |

### When to Run

- [x] After Tier 0 (cleanup) — establish baseline
- [ ] After Tier 1 (Phase 7) — verify forking doesn't regress
- [ ] After Tier 2 (Phase 8) — verify merge doesn't regress
- [ ] After Tier 3 (Phase 9) — verify privacy checks don't regress

### Optimization Work (Only If Gate Fails)

These are **not scheduled**. Only pursue if perf gate shows regression:

| Item                       | Trigger                            | Status        |
| -------------------------- | ---------------------------------- | ------------- |
| ~~Cross-warp parallelism~~ | Multi-warp ticks show poor scaling | ✅ Done       |
| State clone overhead       | CI times unacceptable              | Not scheduled |
| Shard rebalancing          | Skewed distributions measured      | Not scheduled |
| SIMD merge sort            | Merge becomes bottleneck           | Not scheduled |

---

## Test Inventory Summary

| Tier                 | Tests Unblocked     | Cumulative |
| -------------------- | ------------------- | ---------- |
| Tier 0.5             | ~2-3 (audit needed) | ~2-3       |
| Tier 1 (Phase 7)     | 14                  | ~17        |
| Tier 2 (Phase 8)     | 10                  | ~27        |
| Tier 3 (Phase 9)     | 9                   | ~36        |
| Stress (run anytime) | 1                   | 37         |

**Current:** ~17 tests passing
**After Phase 9:** ~54 tests passing (all BOAW tests enabled)

---

## Execution Checklist

### ☐ Tier 0 Cleanup

- [ ] Delete `emit_view_op_delta()` from `rules.rs`
- [ ] Delete `execute_parallel_stride()` + tests + feature gate
- [ ] Verify doc accuracy (TECH-DEBT, ADR, CHANGELOG)

### Tier 0.5: Guardrails (This Week)

- [ ] Audit `#[ignore]` tests — activate any that now pass
- [ ] Add/verify WarpOpKey invariant test
- [ ] Create `benches/boaw_baseline.rs` with minimal benchmarks
- [ ] Document baseline in `docs/notes/boaw-perf-baseline.md`
- [ ] Run perf gate, record numbers

### Tier 1: Phase 7 (Next Sprint)

- [ ] Implement OpenPortal scheduling (T7.1)
- [ ] Implement DeltaView
- [x] Implement FootprintGuard (44aebb0, 0d0231b)
- [ ] Wire SnapshotBuilder to test harness
- [ ] Core forking semantics
- [ ] Activate 14 tests
- [ ] Run perf gate

### Tier 2: Phase 8 (Following Sprint)

- [ ] Typed merge registry
- [ ] Merge regimes + conflict artifacts
- [ ] Presence policies
- [ ] Activate 10 tests
- [ ] Run perf gate

### Tier 3: Phase 9 (Future)

- [ ] Atom type registry
- [ ] Mind mode + ClaimRecord
- [ ] ZK proof merging
- [ ] Activate 9 tests
- [ ] Run perf gate

---

## References

- [ADR-0007-BOAW-Storage.md](../adr/ADR-0007-BOAW-Storage.md) — Full specification
- [TECH-DEBT-BOAW.md](../adr/TECH-DEBT-BOAW.md) — Original tracking (to be updated)
- [PR #257](https://github.com/flyingrobots/echo/pull/257) — Phase 6B implementation
- Knowledge Graph: `BOAW_Phase_6B`, `Echo_BOAW_Architecture`
