<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Scheduler Performance (warp-core)

This document covers **performance measurement** for the implemented `warp-core` rewrite scheduler.

It is not a standalone scheduler spec. For scheduler semantics, see
[`../spec/scheduler-warp-core.md`](../spec/scheduler-warp-core.md).

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

### Footprint overlap between Actions

Bench:

- `crates/warp-core/benches/action_footprint_overlap.rs`

Submits a frame of compare-and-set Actions in which a chosen share target a
cell another Action of the frame also targets, and reports frame time with
how many Actions commit, are rejected for a footprint conflict, or are
obstructed. It runs with
`cargo bench -p warp-core --features native_rule_bootstrap,trusted_runtime --bench action_footprint_overlap`
and prints a table; see the benchmark guide for why it lives in `warp-core`.

## Documentation Hygiene

When you add/modify scheduler benches:

- update this doc to link the bench file(s),
- keep `docs/spec/scheduler-warp-core.md` aligned if semantics are affected,
- avoid “single-run timing inside unit tests” as the foundation for performance claims.
