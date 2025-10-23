# Scheduler Benchmark Plan (Phase 0)

Objective: validate the scheduler design under realistic workloads before full implementation. These notes outline benchmark scenarios, metrics, and tooling.

---

## Scenarios

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

## Metrics
- Average and max time per phase (pre, update, post, render_prep, timeline_flush).
- Overhead vs pure system execution (scheduler time / total time).
- Number of batches formed (parallel planning).
- Cycle detection latency (time to detect graph updates).
- Entropy/timeline flush cost (simulate Diff persistence stub).


---

## Tooling
- Use Vitest benchmarks (`@vitest/benchmark`) or simple `performance.now()` wrappers.
- Provide CLI entry point in `packages/echo-core/scripts/bench-scheduler.ts`.
- Output results as JSON for inspector consumption.
- Reuse deterministic math PRNG for synthetic workload generation.

---

## Tasks
- [ ] Scaffold benchmark harness (TS script + pnpm script `bench:scheduler`).
- [ ] Implement mock system descriptors for each scenario.
- [ ] Integrate with timeline fingerprint to simulate branches.
- [ ] Record baseline numbers in docs and add to decision log.
- [ ] Automate nightly run (future CI step).

