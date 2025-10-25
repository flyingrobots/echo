# Language & Responsibility Map (Phase 1)

Echo’s runtime stack is intentionally stratified. Rust owns the deterministic graph engine; Lua sits on top for gameplay scripting; TypeScript powers the tooling layer via WebAssembly bindings. This document captures what lives where as we enter Phase 1 (Core Ignition).

---

## Rust (rmg-core, ffi, wasm, cli)

### Responsibilities
- RMG engine: GraphStore, PatternGraph, RewriteRule, DeterministicScheduler, commit/Snapshot APIs.
- ECS foundations: Worlds, Systems, Components expressed as rewrite rules.
- Timeline & Branch tree: rewrite transactions, snapshot hashing, concurrency guard rails.
- Math/PRNG: deterministic float32 / fixed32 modules shared with gameplay.
- Netcode: lockstep / rollback / authority modes using rewrite transactions.
- Asset pipeline: import/export graphs, payload storage, zero-copy access.
- Confluence: distributed synchronization of rewrite transactions.
- Lua VM hosting: embed Lua 5.4, expose RMG bindings via FFI.
- CLI tools: `rmg` command for apply/snapshot/diff/verify.

### Key Crates
- `rmg-core` – core engine
- `rmg-ffi` – C ABI for Lua and other native consumers
- `rmg-wasm` – WASM build for tooling/editor
- `rmg-cli` – CLI utilities

---

## Lua (gameplay authoring layer)

### Responsibilities
- Gameplay systems & components (e.g., AI state machines, quests, input handling).
- Component registration, entity creation/destruction via exposed APIs.
- Scripting for deterministic “async” (scheduled events through Codex’s Baby).
- Editor lenses and inspector overlays written in Lua for rapid iteration.

### Constraints
- Single-threaded per branch; no OS threads.
- GC runs in deterministic stepped mode, bounded per tick.
- Mutations occur through rewrite intents (`rmg.apply(...)`), not raw memory access.

### Bindings
- `rmg` Lua module providing:
  - `apply(rule_name, scope, params)`
  - `delay(seconds, fn)` (schedules replay-safe events)
  - Query helpers (read components, iterate entities)
  - Capability-guarded operations (world:rewrite, asset:import, etc.)

---

## TypeScript / Web Tooling

### Responsibilities
- Echo Studio (graph IDE) – visualizes world graph, rewrites, branch tree.
- Inspector dashboards – display Codex, entropy, paradox frames.
- Replay/rollback visualizers, network debugging tools.
- Plugin builders and determinism test harness UI.

### Integration
- Uses `rmg-wasm` to call into RMG engine from the browser.
- IPC/WebSocket for live inspector feeds (`InspectorEnvelope`).
- Works with JSONL logs for offline analysis.
- All mutations go through bindings; tooling never mutates state outside RMG APIs.

### Tech
- Frontend frameworks: React/Svelte/Vanilla as needed.
- WebGPU/WebGL for graph visualization.
- TypeScript ensures type safety for tooling code.

---

## Summary
- Rust: core deterministic runtime + binding layers.
- Lua: gameplay logic, editor lenses, deterministic script-level behavior.
- TypeScript: visualization and tooling on top of WASM/IPC.

This division keeps determinism and performance anchored in Rust while giving designers and tooling engineers approachable layers tailored for their workflows.
