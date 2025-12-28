<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Language & Responsibility Map (Phase 1)

Echo’s runtime stack is intentionally stratified. Rust owns the deterministic graph engine; Rhai sits on top for gameplay scripting; TypeScript powers the tooling layer via WebAssembly bindings. This document captures what lives where as we enter Phase 1 (Core Ignition).

---

## Rust (warp-core, ffi, wasm, cli)

### Responsibilities
- WARP engine: GraphStore, PatternGraph, RewriteRule, DeterministicScheduler, commit/Snapshot APIs.
- ECS foundations: Worlds, Systems, Components expressed as rewrite rules.
- Timeline & Branch tree: rewrite transactions, snapshot hashing, concurrency guard rails.
- Math/PRNG: deterministic float32 / fixed32 modules shared with gameplay.
- Netcode: lockstep / rollback / authority modes using rewrite transactions.
- Asset pipeline: import/export graphs, payload storage, zero-copy access.
- Confluence: distributed synchronization of rewrite transactions.
- Rhai engine hosting: embed Rhai with deterministic module set; expose WARP bindings.
- CLI tools: `warp` command for apply/snapshot/diff/verify.

### Key Crates
- `warp-core` – core engine
- `warp-ffi` – C ABI for host/native consumers; Rhai binds directly in-process
- `warp-wasm` – WASM build for tooling/editor
- `warp-cli` – CLI utilities

---

## Rhai (gameplay authoring layer)

### Responsibilities
- Gameplay systems & components (e.g., AI state machines, quests, input handling).
- Component registration, entity creation/destruction via exposed APIs.
- Scripting for deterministic “async” (scheduled events through Codex’s Baby).
- Editor lenses and inspector overlays written in Rhai for rapid iteration.

### Constraints
- Single-threaded per branch; no OS threads.
- Engine budgeted deterministically per tick.
- Mutations occur through rewrite intents (`warp.apply(...)`), not raw memory access.

### Bindings
- `warp` Rhai module providing:
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
- Uses `warp-wasm` to call into WARP engine from the browser.
- IPC/WebSocket for live inspector feeds (`InspectorEnvelope`).
- Works with JSONL logs for offline analysis.
- All mutations go through bindings; tooling never mutates state outside WARP APIs.

### Tech
- Frontend frameworks: React/Svelte/Vanilla as needed.
- WebGPU/WebGL for graph visualization.
- TypeScript ensures type safety for tooling code.

---

## Summary
- Rust: core deterministic runtime + binding layers.
- Rhai: gameplay logic, editor lenses, deterministic script-level behavior.
- TypeScript: visualization and tooling on top of WASM/IPC.

This division keeps determinism and performance anchored in Rust while giving designers and tooling engineers approachable layers tailored for their workflows.
