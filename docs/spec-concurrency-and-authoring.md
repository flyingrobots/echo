<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Concurrency & Authoring Specification (Phase 0.75)
> **Background:** For a gentler introduction, see [WARP Primer](/guide/warp-primer).


Clarifies Echo’s deterministic concurrency model and how Rhai/Rust developers author gameplay systems at Unity-scale without sacrificing replay guarantees.

---

## Core Principles
- **Parallelism lives in the Rust core** (scheduler, ECS, branch tree).
- **Scripting remains single-threaded** (Rhai sandbox per branch/world).
- **All side effects traverse Codex’s Baby**; no direct threaded mutations from scripts.
- **Adapters may use threads internally** but must commit results deterministically at tick boundaries.

---

## Rust Core Concurrency
- Systems declare read/write signatures; scheduler groups non-overlapping systems into parallel jobs.
- Enforcement:
  - Single-writer, multi-reader per component type per tick.
  - Job graph regenerated each tick, deterministically ordered.
  - Parallel jobs reduce into deterministic results (e.g., sorted reduction, stable merges).
- Branch tree merges and diff application leverage the same job infrastructure.

---

## Rhai Execution Model
- Each branch/world owns one Rhai engine + AST set.
- Scheduler phases (`pre_update`, `update`, `post_update`) call into Rhai sequentially.
- Rhai tasks stay single-threaded; no host threads spawned from scripts.
- GC/engine budgeting runs in deterministic steps per tick.
- Rhai “async” helpers emit events; e.g., `echo::delay(seconds, callback)` enqueues an event to Codex’s Baby targeting future Chronos.
- Note: `echo::delay(...)` and `echo::emit(...)` are **Echo-provided host functions** registered into the Rhai engine; they are not built-in Rhai constructs.

### Deterministic Async Example

```rhai
fn on_start() {
    echo::delay(3.0, || {
        echo::emit("spawn_particle", #{ pos: this.pos });
    });
}
```
- `echo::delay` schedules a timed event with `chronos + seconds * tickRate`.
- Replay reproduces identical scheduling.

---

## Adapter Threads (Physics, Rendering, Networking)
- Adapters may spawn threads (e.g., physics broadphase), but results must be committed deterministically:
  - Threaded computations produce intermediate data.
  - Results sorted / canonicalized before writing to ECS.
  - Writes occur in deterministic order within the tick’s reduction phase.
- Integration tests verify identical hashes across runs.

---

## Authoring Layers

| Layer | Language | Purpose |
| ----- | -------- | ------- |
| Rhai scripts | Rhai | Gameplay logic, event handlers, component queries |
| Rust plugins | Rust (plugin system) | New systems/components, AI planners, deterministic subsystems |
| Native adapters | C (via C ABI) | Custom renderers, physics backends |

- Rhai authors interact via `EchoWorldAPI` in scripting mode.
- Rust plugin authors register systems/components with deterministic access declarations.
- C adapters communicate through FFI, respecting capability tokens.

---

## Determinism Rules Summary
- Only core scheduler launches parallel jobs; scripts remain single-threaded.
- Rhai async → scheduled events; no OS threads.
- All mutations route through Codex’s Baby and ECS APIs.
- Adapter threads must synchronize and canonicalize outputs before commit.
- Replay (`echo replay --verify`) detects divergences caused by nondeterministic plugins/adapters.

This specification ensures large-scale authoring remains deterministic while exploiting parallel hardware safely.
