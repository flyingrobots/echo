<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Scheduler Specification (Phase 0)
> **Background:** For a gentler introduction, see [Scheduler Hub](/scheduler).


This document defines the **planned** ECS-style system scheduler (systems + phases + a dependency DAG) for Echo core.

**Status (2026-01-02): spec only.** The implemented scheduler in this repo today is the `warp-core` rewrite scheduler (`reserve()` / deterministic drain).

Start here for the doc map:
- `docs/scheduler.md`

---

## Goals
- Deterministic ordering of systems based on declared dependencies and phases.
- Support pause-aware execution, unpauseable systems, and optional parallel batches.
- Integrate branch management (Chronos/Kairos/Aion) in a predictable, replayable manner.
- Provide instrumentation for profiling and timeline inspection without perturbing determinism.

---

## Concepts & Data Model

### Scheduler Phases
1. `initialize` – one-time setup for newly added systems.
2. `pre_update` – assimilate input, flush Codex’s Baby pre-queues, prepare branch jobs.
3. `update` – core simulation systems.
4. `post_update` – cleanup, late binding, physics sync.
5. `render_prep` – assemble render frames & diagnostics payloads.
6. `present` – hand-off to renderer ports / adapters.
7. `timeline_flush` – persist diff metadata, entropy metrics, branch bookkeeping.

Systems declare which phases they participate in (default `update`).

### System Descriptor
```ts
interface SystemDescriptor {
  readonly id: number;
  readonly name: string;
  readonly phases: readonly SchedulerPhase[];
  readonly before?: readonly number[];   // system IDs this system must run before
  readonly after?: readonly number[];    // system IDs this system must run after
  readonly unpauseable?: boolean;
  readonly parallelizable?: boolean;
  readonly priority?: number;            // tie-breaker within DAG (higher runs earlier)
  readonly signature?: ComponentSignature; // optional query signature hint
  readonly handler: SystemHandler;       // function invoked by scheduler
}
```

### Graph Structures
- `phaseBuckets: Map<SchedulerPhase, PhaseGraph>`
- `PhaseGraph` holds:
  - `nodes: Map<SystemId, GraphNode>`
  - `edges: Map<SystemId, Set<SystemId>>` (outgoing edges)
  - `inDegree: Map<SystemId, number>`
  - `topologyCache: SystemId[]` (recomputed on dirty flag)
- Dirty flag triggers re-toposort when systems added/updated.

### Branch Context
- Each tick uses `TimelineFingerprint` (Chronos/Kairos/Aion).
- Scheduler stores current branch context so systems know which branch they operate in.
- Speculative branches have their own scheduler instances or share graph with context-specific runtime queues (implementation detail TBD).

---

## Registration Workflow
Pseudo-code:
```ts
function registerSystem(descriptor: SystemDescriptor): void {
  for (const phase of descriptor.phases ?? ["update"]) {
    const graph = phaseBuckets.getOrCreate(phase);
    if (graph.nodes.has(descriptor.id)) throw duplicate;
    graph.nodes.set(descriptor.id, {
      descriptor,
      status: "pending",
      // additional runtime metadata (profiling counters, lastDuration, etc.)
    });
    // Establish edges
    for (const afterId of descriptor.after ?? []) {
      graph.edges.get(afterId)?.add(descriptor.id) ?? graph.edges.set(afterId, new Set([descriptor.id]));
      graph.inDegree.set(descriptor.id, (graph.inDegree.get(descriptor.id) ?? 0) + 1);
    }
    for (const beforeId of descriptor.before ?? []) {
      graph.edges.get(descriptor.id)?.add(beforeId) ?? graph.edges.set(descriptor.id, new Set([beforeId]));
      graph.inDegree.set(beforeId, (graph.inDegree.get(beforeId) ?? 0) + 1);
    }
    graph.dirty = true;
  }
}
```
- `priority` influences topological ordering by adjusting insertion order (e.g., using min-heap keyed by `(topologyLevel, -priority)`).
- Validate acyclic graph: after inserting edges, run cycle detection; if cycle detected, throw descriptive error listing cycle path.

---

## Tick Execution Flow

```ts
function runTick(context: TickContext) {
  const phases = [PRE_UPDATE, UPDATE, POST_UPDATE, RENDER_PREP, PRESENT, TIMELINE_FLUSH];
  if (isFirstTick) runPhase(INITIALIZE, context);
  for (const phase of phases) {
    runPhase(phase, context);
  }
}

function runPhase(phase: SchedulerPhase, context: TickContext) {
  const graph = phaseBuckets.get(phase);
  if (!graph) return;
  if (graph.dirty) recomputeTopology(graph);

  const batchPlan = phase === UPDATE ? planParallelBatches(graph, context) : sequentialPlan(graph);
  for (const batch of batchPlan) {
    executeBatch(batch, context);
  }
}
```

### Topology Computation
```ts
function recomputeTopology(graph: PhaseGraph) {
  const queue = PriorityQueue<SystemId>({ // compare by priority and descriptor id for determinism
    compare(a, b) {
      const pa = nodes.get(a)!.descriptor.priority ?? 0;
      const pb = nodes.get(b)!.descriptor.priority ?? 0;
      return pb - pa || a - b;
    }
  });
  const inDegree = clone(graph.inDegree);
  for (const [id] of graph.nodes) {
    if ((inDegree.get(id) ?? 0) === 0) queue.push(id);
  }
  const ordered: SystemId[] = [];
  while (!queue.isEmpty()) {
    const id = queue.pop();
    ordered.push(id);
    for (const neighbor of graph.edges.get(id) ?? []) {
      const deg = (inDegree.get(neighbor) ?? 0) - 1;
      inDegree.set(neighbor, deg);
      if (deg === 0) queue.push(neighbor);
    }
  }
  if (ordered.length !== graph.nodes.size) throw cycleError();
  graph.topologyCache = ordered;
  graph.dirty = false;
}
```

### Parallel Batch Planning
- Only for phases that allow parallel execution (initially `update`).
- Approach:
  1. Walk `topologyCache` in order.
  2. Maintain `readySet` of systems whose dependencies have been scheduled but not yet executed.
  3. For each system:
     - If `descriptor.parallelizable` and not `unpauseable`, try to place into current batch.
     - Ensure no resource conflicts (e.g., two systems writing to same exclusive resource). For initial version, require manual declarations of exclusive tags or rely on heuristics (e.g., overlapping component signatures) to avoid collisions.
  4. If system cannot be parallelized, flush current batch, execute sequentially, then resume batching.
- Implementation may begin sequential (no parallelism) and introduce batches after profiling.

### Pause Handling
- `isPaused` flag passed into `runTick`.
- Systems marked `unpauseable` execute even when paused.
- Others are skipped when paused, except phases `render_prep` and `present` which may still run minimal tasks (e.g., debug overlay).

### Timeline & Codex Integration
- `pre_update` phase flushes Codex’s Baby input queues and registers branch jobs.
- After each `executeBatch`, record profiling data (duration, branch ID) for inspector.
- `timeline_flush` phase writes diff metadata to branch tree and updates entropy.

---

## executeBatch
```ts
function executeBatch(batch: Batch, context: TickContext) {
  if (batch.parallel) {
    // future extension: run via worker pool / job scheduler
    for (const systemId of batch.systemIds) {
      runSystem(systemId, context); // sequential fallback for now
    }
  } else {
    for (const systemId of batch.systemIds) {
      runSystem(systemId, context);
    }
  }
}

function runSystem(systemId: number, context: TickContext) {
  const node = nodes.get(systemId)!;
  const start = now();
  node.descriptor.handler(context); // handler receives TickContext + DI container
  const end = now();
  node.lastDuration = end - start;
  // update profiling / instrumentation structures
}
```
- `handler` signature will later include typed accessors (queries, command writers, diagnostics).
- `now()` uses deterministic-safe clock (monotonic per tick) to avoid cross-platform drift (profiling only).

---

## Error Handling & Diagnostics
- Registration: validation errors include system name, conflicting dependencies, cycle path.
- Runtime: exceptions bubble up to scheduler; engine should capture, log, and halt tick deterministically.
- Provide hooks to attach debug callbacks (e.g., before/after system runs).
- Timeline inspector can query `graph.topologyCache`, `node.lastDuration`, and `batchPlan`.

---

## Determinism Considerations
- Topology queue uses deterministic priority comparison (priority desc, system ID asc).
- Batching respects original order when non-parallel; ensures consistent results across runs.
- `context` includes deterministic delta time; no direct wall-clock usage allowed inside systems.
- `runSystem` should guard against asynchronous operations (throw if handler returns Promise).

---

## Benchmark Scenarios (Future)

These benchmarks apply to the **planned Echo ECS/system scheduler**, not the implemented `warp-core` rewrite scheduler.

For the current `warp-core` rewrite scheduler benchmarks, see:
- `docs/scheduler-performance-warp-core.md`

Objective: validate scheduler behavior and complexity under realistic dependency graphs *before* implementation and during future tuning.

### Scenarios

1. **Flat Update Loop**
   - 10, 50, 100 systems in the `update` phase with no dependencies.
   - Measure cost per system invocation and scheduler overhead.

2. **Dependency Chain**
   - Linear chain of 100 systems (`A -> B -> C -> ...`).
   - Validate topological ordering and detect any O(n^2) behavior.

3. **Branching Graph**
   - DAG with 10 layers, each 10 systems wide; edges from each layer to next.
   - Pin deterministic tie-breaking for same-level priority.

4. **Parallelizable Mix**
   - Systems tagged `parallelizable` with no conflicts; simulate runtime by running sequentially but tracking the planned batch schedule.
   - Later extend to actual parallel execution.

5. **Pause Semantics**
   - Mix of pauseable/unpauseable systems. Toggle pause flag mid-run.
   - Validate that skipped systems remain skipped deterministically (and that required phases still run).

6. **Branch Context Switching**
   - Simulate multiple branches (Kairos IDs) within benchmarks to capture timeline flush behavior and branch-local queues.

### Metrics
- Average and max time per phase (`pre_update`, `update`, `post_update`, `render_prep`, `timeline_flush`).
- Overhead vs pure system execution (scheduler time / total time).
- Number of batches formed (parallel planning), and batch size distribution.
- Cycle detection latency (time to detect graph updates).
- Entropy/timeline flush cost (simulate diff persistence stub).

### Tooling
- Use Criterion for statistical benchmarking (or a JS benchmark harness if implemented in TS first).
- Output results as JSON for inspector consumption.
- Reuse the deterministic math PRNG for synthetic workload generation (avoid `Math.random()` / wall clocks).

### Tasks
- [ ] Implement system-scheduler benchmark harness once the system scheduler exists.
- [ ] Implement mock system descriptors for each scenario.
- [ ] Integrate with timeline fingerprint to simulate branches.
- [ ] Record baseline numbers and link them from `docs/scheduler.md`.
- [ ] Automate periodic runs (future CI step) once benchmarks stabilize.

## Open Questions
- How to model resource conflicts for parallel execution (manual tags vs automatic detection).
- Whether phase-specific priorities should be allowed (e.g., `render_prep` custom ordering).
- Strategy for cross-branch scheduling: separate scheduler per branch vs shared graph with branch-specific execution queues.
- Should initialization phase run lazily when systems added mid-game, or strictly at startup?

Document updates feed into implementation tasks (GitHub Issues).
