<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Testing & Replay Plan (Phase 0.5)

Defines how Echo proves determinism end-to-end: automated tests, replay tooling, and golden datasets.

---

## Replay CLI Contract

`echo replay --from <nodeId> --until <nodeId> --verify`

- Loads block manifest spanning `from` → `until`.
- Replays diffs using canonical decoding, enforcing PRNG spans and capability rules.
- Verification: recompute `worldHash` at each node and compare with recorded hash; mismatches flagged.
- Outputs `VerificationReport` with pass/fail, mismatch details, and entropy trail.

```ts
interface VerificationReport {
  readonly from: NodeId;
  readonly until: NodeId;
  readonly success: boolean;
  readonly mismatches?: readonly Mismatch[];
  readonly stats: {
    replayedDiffs: number;
    elapsedMs: number;
    entropyTrail: number[];
  };
}
```

---

## Golden Hash Dataset

- Maintained under `tests/golden/` with recorded blocks for canonical scenarios (each engine subsystem).
- CI job replays golden datasets across Node, Chromium, WebKit; asserts identical hashes.
- Golden scenarios include: idle world, branching + merge, paradox quarantine, entropy surges.

---

## Differential Merge Checker

- For any branch merge, store both diff chains and run a comparer ensuring three-way merge produced expected result.
- Tool `echo diff-compare --base <node> --a <node> --b <node>` outputs conflict list and merged hash; used in tests.

---

## Entropy Regression Tests

- Simulate deterministic sequences (forks, merges, paradoxes) and assert entropy meter matches expected values.
- Tests fail if entropy formula or weights change without updating test expectations.

---

## Automation Plan

Once implemented, the automated test suite will include:

- PLANNED: `cargo test --package warp-core --features determinism` – runs replay and comparers for golden datasets.
- PLANNED: `cargo test --package warp-core --test paradox` – injects artificial read/write overlaps to validate quarantine behavior.
- PLANNED: `cargo test --package warp-core --test entropy` – verifies entropy observers and metrics.
- PLANNED: `cargo test --package warp-core --test bridge` – covers temporal bridge retro/reroute.
- TODO: Add Criterion-based scheduler benches to CI once implemented (Phase 1 task).

### BOAW Compliance Tests (Implemented)

The BOAW (Base-Overlay-Apply-Write) test harness is now implemented per ADR-0007:

- `cargo test --package warp-core --test boaw_determinism` – 8 determinism tests with real engine hashes
- `EngineHarness` trait provides a real harness that wraps `warp-core::Engine`
- `BoawSnapshot` captures state for determinism verification
- `boaw/touch` test rule exercises the core rewrite pipeline

**Phase 3 Progress:**

- `TickDelta` module now available for collecting ops during execution
- Validation infrastructure ready with `assert_delta_matches_diff()` helper (gated by `delta_validate` feature)

## Phase 4: SnapshotAccumulator Validation

Under the `delta_validate` feature, Phase 4 adds a second validation layer:

1. **Delta-to-diff validation** (Phase 3): `delta.finalize()` ops must match `diff_state()` output *exactly* (full `WarpOp` equality, including payloads — not just `sort_key()`)
2. **Accumulator validation** (Phase 4): `SnapshotAccumulator` built from `base + ops` must produce the same `state_root` as legacy computation

Run with: `cargo test -p warp-core --features delta_validate`

## Phase 5: Read-Only Execution (Complete)

Phase 5 completes the BOAW execution model transition:

1. **Read-only execution**: Executors receive `GraphView` (read-only) instead of `&mut GraphStore`
2. **Op emission only**: No GraphStore mutations during execution — rules emit ops to `TickDelta`
3. **Post-execution state update**: State updated after execution via `apply_to_state()`
4. **Signature change**: `ExecuteFn` now takes `(&GraphView, &mut TickDelta, &NodeId)` instead of `(&mut GraphStore, &NodeId)`

This milestone enables:

- True parallel execution (thread-local deltas, no shared mutable state)
- Removal of `state_before = self.state.clone()` overhead
- Removal of `diff_state()` post-hoc diffing
- Foundation for structural sharing and immutable snapshots

---

## Manual Validation

- Provide scripts to run long-form simulations (50k ticks) and ensure replay matches.
- Document steps in README for reproducibility.

---

This plan ensures Echo can prove determinism, replayability, entropy stability, and merge correctness across environments.
