<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Scheduler Benchmark Plan (Phase 0)

This document is intentionally split into two benchmark tracks, because Echo has two “scheduler” concepts:

1) **WARP rewrite scheduler (implemented today, Rust `warp-core`)** — we already have Criterion benches.
2) **Echo system scheduler (planned, `@echo/core`)** — scenarios remain useful, but are future work.

For the canonical doc map, see `docs/scheduler.md`.

---

## Track A — WARP Rewrite Scheduler (warp-core, implemented)

### Current benches

- Drain throughput: `crates/warp-benches/benches/scheduler_drain.rs`
  - Measures apply/enqueue/drain costs with a no-op rule to isolate scheduler overhead.
- Adversarial hashing notes: `crates/warp-benches/benches/scheduler_adversarial.rs`
  - Benchmarks hash-table collision behavior relevant to `reserve()`’s GenSet approach.

### Suggested additions (when needed)
- A dedicated `reserve()` microbench (vary `k` reserved rewrites and `m` footprint size).
- A baseline comparison bench (historical Vec<Footprint> vs current GenSet) if we ever resurrect the old implementation behind a feature flag for measurement.

---

## Track B — Echo System Scheduler (future, planned)

Objective: validate the **system scheduler spec** under realistic workloads before implementation.
The scenarios below are still useful, but they apply to `docs/spec-scheduler.md` (systems + phases + DAG), not `warp-core`.

---

### Scenarios

1. **Flat Update Loop**
   - 10, 50, 100 systems in the `update` phase with no dependencies.
   - Measure cost per system invocation and scheduler overhead.

2. **Dependency Chain**
   - Linear chain of 100 systems (`A -> B -> C ...`).
   - Validate topological ordering and detect any O(n^2) behavior.

3. **Branching Graph**
   - DAG with 10 layers, each 10 systems wide; edges from each layer to next.
   - Tests to ensure priority ties stay deterministic.

4. **Parallelizable Mix**
   - Systems tagged `parallelizable` with no conflicts; simulate runtime by running sequentially but tracking batch plan.
   - Later extend to actual parallel execution.

5. **Pause Semantics**
   - Mix of pauseable/unpauseable systems. Toggle pause flag mid-run.

6. **Branch Context Switching**
   - Simulate multiple branches (Kairos IDs) within benchmarks to capture timeline flush behavior.

---

### Metrics
- Average and max time per phase (pre, update, post, render_prep, timeline_flush).
- Overhead vs pure system execution (scheduler time / total time).
- Number of batches formed (parallel planning).
- Cycle detection latency (time to detect graph updates).
- Entropy/timeline flush cost (simulate diff persistence stub).

---

## Tooling
*Future-facing. The `warp-core` benches already use Criterion; this section refers to the system scheduler track.*
- Use Criterion for statistical benchmarking (or a JS benchmark harness if implemented in TS first).
- Output results as JSON for inspector consumption.
- Reuse deterministic math PRNG for synthetic workload generation.

---


## Tasks
- [ ] Implement system-scheduler benchmark harness once the system scheduler exists.
- [ ] Implement mock system descriptors for each scenario.
- [ ] Integrate with timeline fingerprint to simulate branches.
- [ ] Record baseline numbers in docs and add to decision log.
- [ ] Automate nightly run (future CI step).
