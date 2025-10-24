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
- `pnpm test:determinism` – runs replay and comparers for golden datasets.
- `pnpm test:paradox` – injects artificial read/write overlaps to validate quarantine behavior.
- `pnpm test:entropy` – verifies entropy observers and metrics.
- `pnpm test:bridge` – covers temporal bridge retro/reroute.
- `pnpm bench:scheduler` – as defined earlier, ensures performance regressions are recorded.

---

## Manual Validation
- Provide scripts to run long-form simulations (50k ticks) and ensure replay matches.
- Document steps in README for reproducibility.

---

This plan ensures Echo can prove determinism, replayability, entropy stability, and merge correctness across environments.
