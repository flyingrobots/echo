<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Scheduler Performance (warp-core)

This document covers **performance measurement** for the implemented `warp-core` rewrite scheduler.

It is **not** about the planned Echo ECS/system scheduler (see `docs/spec-scheduler.md`).
For a doc map of “which scheduler doc should I read?”, see `docs/scheduler.md`.

---

## What We Measure

The rewrite scheduler’s performance primarily shows up in:
- **enqueue cost**: admitting pending rewrites (`reserve()` and friends)
- **drain/commit cost**: deterministically draining accepted rewrites and executing them

Because rewrite execution work depends on rules, we try to benchmark scheduler overhead with
trivial/no-op rules where possible.

---

## Current Benchmarks

### Drain throughput

Bench file:
- `crates/warp-benches/benches/scheduler_drain.rs`

What it measures:
- applies a no-op rule to many nodes and commits
- also separates “enqueue only” and “drain only” phases

Run:

```sh
cargo bench -p warp-benches
```

### Adversarial hashing (collision behavior)

Bench file:
- `crates/warp-benches/benches/scheduler_adversarial.rs`

What it measures:
- worst-case behavior for `FxHashMap` under deliberate collisions vs random keys
- relevant because `reserve()` uses hash-backed sets for conflict detection/marking

Run:

```sh
cargo bench -p warp-benches
```

---

## Recommended Next Benches (When Needed)

These are “nice to have” when tuning the scheduler or validating complexity claims:

- A dedicated `reserve()` microbench varying:
  - `k` (number of previously reserved rewrites)
  - `m` (candidate footprint size)
- A benchmark that isolates *only* `reserve()` without engine/rule overhead (if practical).
- A benchmark suite that publishes JSON artifacts and supports regression gates (CI-stable).

---

## Documentation Hygiene

When you add/modify scheduler benches:
- update this doc to link the bench file(s),
- keep `docs/scheduler-warp-core.md` aligned if semantics are affected,
- avoid “single-run timing inside unit tests” as the foundation for performance claims.
