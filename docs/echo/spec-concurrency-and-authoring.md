# Concurrency & Authoring Specification (Phase 0.75)

Clarifies Echo’s deterministic concurrency model and how Lua/Rust developers author gameplay systems at Unity-scale without sacrificing replay guarantees.

---

## Core Principles
- **Parallelism lives in the Rust core** (scheduler, ECS, branch tree).
- **Scripting remains single-threaded** (Lua sandbox per branch/world).
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

## Lua Execution Model
- Each branch/world owns one Lua VM.
- Scheduler phases (`pre_update`, `update`, `post_update`) call into Lua sequentially.
- Coroutine usage allowed intra-VM but cannot mutate world state across threads; resumed within the tick.
- GC runs in stepped deterministic mode with fixed budget per tick.
- Lua “async” tasks emit events; e.g., `echo.delay(seconds, fn)` enqueues an event to Codex’s Baby targeting future Chronos.

### Deterministic Async Example
```lua
function on_start()
  echo.delay(3.0, function()
    echo.emit("spawn_particle", {pos = self.pos})
  end)
end
```
- `echo.delay` schedules a timed event with `chronos + seconds * tickRate`.
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
| Lua scripts | Lua 5.4 | Gameplay logic, event handlers, component queries |
| Rust plugins | Rust (plugin system) | New systems/components, AI planners, deterministic subsystems |
| Native adapters | C (via C ABI) | Custom renderers, physics backends |

- Lua authors interact via `EchoWorldAPI` in scripting mode.
- Rust plugin authors register systems/components with deterministic access declarations.
- C adapters communicate through FFI, respecting capability tokens.

---

## Determinism Rules Summary
- Only core scheduler launches parallel jobs; scripts remain single-threaded.
- Lua async → scheduled events; no OS threads.
- All mutations route through Codex’s Baby and ECS APIs.
- Adapter threads must synchronize and canonicalize outputs before commit.
- Replay (`echo replay --verify`) detects divergences caused by nondeterministic plugins/adapters.

This specification ensures large-scale authoring remains deterministic while exploiting parallel hardware safely.
