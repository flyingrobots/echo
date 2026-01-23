<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Architecture Specification (Draft)

If you’re new here, start with:

- [/guide/start-here](/guide/start-here)
- [/guide/warp-primer](/guide/warp-primer)

This document is a high-level architecture and "why" artifact. Many sections are aspirational and
will lag behind the current Rust-first implementation; prefer WARP specs for the runtime boundary.

> **Implementation Status Legend:**
>
> - ✅ **Implemented** — exists in `warp-core` today
> - ⚠️ **Partial** — some aspects exist, others planned
> - 🗺️ **Planned** — design only, not yet implemented

## Vision

- Reimagine a battle-tested ECS core into **Echo**, a renderer-agnostic spine that survives browsers, native shells, and whatever 2125 invents next.
- Empower teams to build 2D, 3D, or abstract simulations with the same spine, swapping adapters instead of rewriting gameplay.
- Combine modern ergonomics (Rust-first core, clean public surfaces, first-class docs) with ruthless performance discipline so the engine scales from hobby jams to production.
- Preserve institutional memory—document why choices exist, what legacy quirks inspired them, and how to extend or override any piece.

## Cultural Principles

- **Just Ship, But Test**: Echo inherits the original “Just do it” ethos while insisting on automated tests and benchmark gates.
- **Automate the Boring Stuff**: Workflow automation stays core—one-command setup, reproducible builds, scripted lint/format/test pipelines.
- **Stay Focused**: Every feature must trace back to recorded goals; backlog distractions instead of half-building them.
- **Have Fun**: Echo should be a playground; tooling, docs, and samples are crafted to keep the work joyful.
- **Respect the Spine**: Keep `main` stable—feature flags, review gates, and CI guardrails preserve trust.

## Guiding Principles

- **Hexagonal Domain Boundary**: The domain never touches DOM, WebGL, or timers directly; everything outside the core arrives through narrow ports.
- **Data-Oriented Internals**: Gameplay-friendly APIs sit atop archetype/struct-of-arrays storage, pooled allocators, and cache-aware iteration.
- **Predictable Loop**: Fixed time-step simulation by default with deterministic ordering; variable step, interpolation, and rollback sit behind explicit opt-ins.
- **Tooling Is Non-Negotiable**: Debug inspector, event traces, hot-reload, and profiling hooks ship alongside the engine, not as an afterthought.
- **Extensible By Design**: Every subsystem exposes extension points, configuration, and hooks for optional native/Wasm accelerators.
- **Operational Transparency**: Metrics, logging, and failure modes are documented; Echo should be debuggable at 3 AM without spelunking source.

## Domain Layers

### Core ECS 🗺️ Planned

> **Note:** The current `warp-core` implementation uses a **WARP graph model** (nodes, edges, rewrite rules), not traditional ECS archetypes. The ECS storage model below is a future design target.

- **Entities**: Numerical IDs with sparse/high-watermark managers; creation returns pooled slots to avoid GC pressure.
- **Components**: Type-safe registrations with metadata (layout, default state, pooling policy). Storage uses archetype tables or chunked struct-of-arrays chosen at registration time.
- **Storage Model**:
    - Archetype chunks sized to fit CPU cache lines (default 16 KB) with columnar component arrays.
    - Copy-on-write handles for branch persistence; mutate operations clone only touched chunks.
    - Optional fixed-point pools for deterministic math-heavy components (physics transforms, timers).
- **ID Services**: Global registries issue deterministic type IDs; component schemas embed serialization hooks and diff strategies.
- **Systems**: Pure domain functions declaring the signature of components/events they consume. Systems declare schedule phase, dependencies, and whether they run when paused.
- **Scheduler**: Builds a directed acyclic graph of systems, resolves priorities, batches compatible systems for parallel execution (future feature), and mediates fixed-step ticks.
- **Scheduler Phases**:
    1. `initialize` (one-shot setup)
    2. `pre_update` (input assimilation, Codex’s Baby pre-flush)
    3. `update` (core systems in DAG order)
    4. `post_update` (cleanup, late bindings)
    5. `render_prep` (prepare frame packets for adapters)
    6. `present` (adapter flush; optional interpolation)
    7. `timeline_flush` (persist diffs, branch bookkeeping)
- **Parallelism Hooks**: Systems may declare `parallelizable: true`; scheduler groups disjoint signature systems into jobs respecting dependencies.
- **Queries**: Precompiled views over component sets; incremental membership tracking uses bitset signatures and dirty queues instead of per-frame scans.

### World & Scene Management 🗺️ Planned

- **World**: Owns entity/component managers, system registry, event bus, and service container. Supports multiple worlds for split-screen or background sims.
- **Prefabs & Assemblers**: Declarative definitions (JSON/YAML/TS factories) converted into entity creation commands, supporting overrides and inheritance.
- **Scene Graph / State Machine**: Stack-based and hierarchical scenes with enter/exit hooks, async loading, and transition orchestration. Integrates with scheduler via scene phases.
- **Simulation Contexts**: Support for deterministic replay, remote authority, and sub-step simulations (physics, AI planning) within world boundaries.

### Time & Simulation ⚠️ Partial

- **Clock Service**: Abstracted time source with fixed-step accumulator, variable-step mode, and manual stepping for tests.
- **Pause & Slow-Mo**: Pause flag propagates to scheduler; systems opt into running while paused; time scaling applies per system when needed.
- **Deterministic Replay**: Input/event capture via Codex’s Baby, serialized frame seeds, and re-execution hooks for debugging or multiplayer rollback.
- **Job Graph Extensions**: Future-ready hooks for job scheduling or thread pools without breaking the single-threaded baseline.
- **Temporal Axes**:
    - **Chronos (Sequence)**: Monotonic tick counter; governs simulation ordering and replay.
    - **Kairos (Possibility)**: Branch identifier; indexes alternate realities at the same Chronos tick.
    - **Aion (Significance)**: Scalar weight describing narrative gravity/entropy; influences merge priority, NPC memory retention, and paradox severity.

### Temporal Sandbox (Echo Edge) 🗺️ Planned

- **Branchable Timelines**: Worlds can fork into speculative branches mid-frame; scheduler runs branches sequentially or in parallel workers, then reports diffs back to the main timeline.
- **Frame Scrubbing**: Built-in timeline buffer stores component deltas for the last N frames; editor tooling scrubs, rewinds, and reapplies changes without restarting the sim.
- **Predictive Queries**: Renderers, netcode, or AI can request projected state N frames ahead using speculative branches, enabling latency hiding and cinematic planning.
- **Collaborative Simulation**: Multiple clients can author in shared scenes by editing branches; consensus commits merge deterministic deltas back into the root world.
- **AI Co-Pilot Hooks**: Deterministic branches allow automated agents to propose tweaks, run them in sandboxes, and surface accepted diffs to designers.

## Event Bus ✅ Implemented

> **Note:** The original "Event Bus" spec has been superseded by [ADR-0003 (MaterializationBus)](/adr/ADR-0003-Materialization-Bus.md). The MaterializationBus is now implemented with:
>
> - `EmissionPort` trait (hexagonal boundary for rule emissions)
> - `ScopedEmitter` adapter (auto-fills EmitKey from execution context)
> - `ReduceOp` enum (8 built-in deterministic reduce operations)
> - `FinalizeReport` pattern (deterministic batch finalization)
> - 128 tests covering permutation invariance, reduce algebra, and engine integration
> - Cross-platform CI (macOS + Linux, weekly via `dind-cross-platform.yml`)
>
> See `docs/rfc/mat-bus-finish.md` for the completion RFC.
>
> _The content below is preserved for historical context only._

- **Command Buffers**: Events are POD structs appended to per-type ring buffers during a frame; no immediate callbacks inside hot systems.
- **Flush Phases**: Scheduler defines flush points (pre-update, post-update, custom phases). Systems subscribe to phases matching their needs.
- **Handler Contracts**: Handlers receive batched slices; they may mutate components, enqueue new events, or schedule commands. Return values are ignored for deterministic execution.
- **Immediate Channel**: Opt-in channel for rare “now” operations; instrumented with counters and frame-budget warnings.
- **Telemetry & Debugging**: Built-in tooling to inspect event queues, handler timings, dropped events, and memory usage.
- **Integration**: Bridges input devices, networking, scripting, and editor tooling without leaking adapter concerns into the domain.
- **Inter-Branch Bridge**: Temporal mail service routes events between branches; deliveries create retro branches when targeting past Chronos ticks; paradox guard evaluates conflicts before enqueue.

## Playback & Worldlines ✅ Implemented

> **Reference:** [SPEC-0004 (Worldlines, Playback, TruthBus)](spec/SPEC-0004-worldlines-playback-truthbus.md)

SPEC-0004 introduces infrastructure for deterministic materialization, cursor-based replay, and append-only provenance tracking:

- **`crates/warp-core/src/playback.rs`** — `PlaybackCursor` for timeline position, `ViewSession` for materialized viewpoints, `TruthSink` struct for consuming view updates into stable snapshots.
- **`crates/warp-core/src/worldline.rs`** — `WorldlineId` identifiers, `HashTriplet` for cryptographic tick labeling, `WorldlineTickPatchV1` for append-only tick records; supports multi-branch lineage.
- **`crates/warp-core/src/provenance_store.rs`** — `ProvenanceStore` trait (hexagonal port), `LocalProvenanceStore` implementation for recording hash signatures and output deltas per tick; enables auditing and determinism validation.
- **`crates/warp-core/src/retention.rs`** — `RetentionPolicy` enum (variants: `KeepAll`, `CheckpointEvery`, `KeepRecent`, `ArchiveToWormhole`) for garbage collection and storage budgeting; integrates with worldline compaction.
- **`crates/warp-core/src/materialization/frame_v2.rs`** — V2 packet format with cursor stamps, enabling renderers to correlate frames with logical replay positions and support frame-accurate scrubbing.

## Ports & Adapters 🗺️ Planned

### Renderer Port 🗺️ Planned

- **Responsibilities**: Receive frame data (render commands, camera states, debug overlays), manage render resources, and report capabilities.
- **Data Flow**: Domain produces a `FramePacket` containing archetype-friendly draw data (mesh refs, transforms, materials); adapter translates into API-specific calls.
- **Adapters**: Pixi 7/WebGL2 baseline, Canvas2D fallback, WebGPU (wgpu/WASM), native renderer (Skia or bgfx), experimental TUI renderer for debugging.
- **Performance Contracts**: Frame submissions are immutable; adapters can reuse GPU buffers across frames; the port discourages per-entity draw calls.

### Input Port 🗺️ Planned

- **Responsibilities**: Aggregate device state into consumable snapshots (buttons, axes, gestures) and surface device capabilities.
- **Polling Model**: Domain polls once per frame; port ensures event strata are coalesced in consistent order. Scripted or network input injects via Codex’s Baby.
- **Adapters**: Browser (keyboard, mouse, pointer, gamepad), native (SDL), synthetic (playback), test harness stubs.

### Physics Port 🗺️ Planned

- **Responsibilities**: Advance simulation, manage bodies/colliders, and synchronize results back into components.
- **Integration Strategy**: Dual writes through data bridges. ECS components represent desired state; physics port returns authoritative transforms/velocities at sync points.
- **Adapters**: Box2D (planar), Rapier (3D/2D), custom deterministic solver, or headless stub for puzzle games.
- **Advanced Features**: Continuous collision, queries (raycasts, sweeps), event hooks for contacts funneled through Codex’s Baby.

### Networking Port 🗺️ Planned

- **Mode Support**: Single-player (loopback), lockstep peer-to-peer, host-client, dedicated server.
- **Transport Abstraction**: Reliable/unreliable channels, clock sync, session management. Adapter options: WebRTC, WebSockets, native sockets.
- **Replication Strategy**: Deterministic event replication using Codex’s Baby ledger; optional state snapshots for fast-forward joins.
- **Rollback Hooks**: Scheduler exposes rewinding API; networking port coordinates branch rewinds and replays when desync detected.
- **Security Considerations**: Capability tokens, branch validation, deterministic checksum comparison to detect tampering.

### Audio, Persistence, Telemetry Ports 🗺️ Planned

- **Audio**: Command queue for spatial/ambient playback, timeline control, and crossfade scheduling.
- **Persistence**: Abstract reader/writer for save games, cloud sync, diagnostics dumps. Supports structured snapshots and delta patches.
- **Telemetry**: Export frame metrics, event traces, and custom probes to external dashboards or editor overlays.

## Cross-Cutting Concerns ⚠️ Partial

- **Bootstrap Pipeline**: Dependency injection container wires ports, services, systems, and configuration before the first tick. Supports editor-time hot reload.
- **Resource Lifecycle**: Asset handles (textures, meshes, scripts) managed through reference-counted registries and async loaders; domain requests are idempotent.
- **Serialization**: Schema-driven serialization for components and events. Allows save/load, network replication, and state diffing.
- **Deterministic Math**: Echo Math module standardizes vector/matrix/transform operations using reproducible algorithms (configurable precision: fixed-point or IEEE-compliant float32). All systems pull from deterministic PRNG services seeded per branch.
- **Branch Persistence**:
    - Persistent archetype arena with structural sharing.
    - Diff records (component type → entity → before/after) stored per node.
    - Interval index for quick Chronos/Kairos lookup.
- **Entropy & Stability**: Global entropy meter tracks paradox risk; exposed to gameplay and tooling with thresholds triggering mitigation quests or stabilizer systems.
- **Diagnostics**: Unified logging facade, structured trace events, crash-safe dumps, and opt-in assertions for development builds.
- **Security & Sandbox**: Optional restrictions for user-generated content or multiplayer host/client boundaries; capability-based access to ports.
- **Extensibility**: Plugins define new components, systems, adapters, or editor tools; registration API enforces namespace isolation and version checks.

## Legacy Excavation Log

- **Goal**: Track every legacy file, classify (keep concept, redesign, discard), note dependencies (Mootools, globals, duplicate IDs), and record learnings to inform Echo.
- **Artifacts**: `docs/meta/legacy-excavation.md` (to be populated) with columns for file, role, verdict, action items, and notes.
- **Process**: Review file → summarize intent → capture bugs/gaps → map to Echo’s modules → decide migration path or deprecation.
- **Outcome**: Comprehensive reference that prevents accidental feature loss and keeps the rewrite grounded in historical context.

## Delivery Roadmap

> **Current Status (2026-01):** Phase 0 is largely complete for `warp-core`. The Rust-first WARP graph rewriting engine is implemented with deterministic scheduling, snapshot hashing, and basic math. ECS storage and system scheduler remain future work.

- **Phase 0 – Spec Deep Dive** ⚠️ Partial: WARP core specs finalized; ECS storage spec exists but not implemented; MaterializationBus implemented (ADR-0003).
- **Phase 1 – Echo Core MVP** 🗺️ Planned: Entity/component storage, system scheduler, MaterializationBus integration (bus complete, integration pending).
- **Phase 2 – Adapter Foundations** 🗺️ Planned _(Milestone: "Double-Jump")_: Renderer adapter, input, physics stub.
- **Phase 3 – Advanced Adapters** 🗺️ Planned: Physics engines, WebGPU, audio, telemetry.
- **Phase 4 – Tooling & Polishing** 🗺️ Planned: Inspector, hot-reload, documentation site.
- **Ongoing**: Benchmark suite, community feedback loop, incremental releases.

## Open Questions

- What minimum target hardware do we optimize for (mobile, desktop, consoles)?
- How aggressive should we be with multi-threading in v1 versus keeping single-thread determinism?
- Should the renderer port define a common material language or leave it adapter-specific?
- Do we ship editor tooling (Echo Studio) in v1 or after the core stabilizes?
- How do we version and distribute optional native/Wasm modules without fragmenting users?
- What licensing model keeps Echo open yet sustainable for long-term stewardship?
- How do Chronos/Kairos/Aion weights interplay with gameplay economy (entropy, player agency)?
- Which temporal mechanics graduate into core APIs versus sample-game features?

## Appendices

- **Glossary**: Mapping of Echo terminology (World, System Graph, Codex’s Baby) to legacy prototype terminology.
- **Reference Architectures**: Snapshots from Unity DOTS, Bevy, Godot Servers, and custom ECS implementations for comparative insight.
- **Profiling Plan**: Target frame budgets, benchmark scenarios, and instrumentation strategy for unit and integration testing.
- **Compatibility Notes**: Guidance for migrating legacy prototypes, bridging Mootools utilities, and reintroducing box2d/pixi demos on modern footing.
- **Data Structure Sketches**: (pending) diagrams for archetype arena, branch tree, Codex’s Baby queues.
- **Temporal Mechanic Catalogue**: (pending) curated list of déjà vu, Mandela artifacts, paradox mitigation, multiverse puzzles.
- **Repository Layout (Draft)**:
    - `/packages/echo-core` — deterministic ECS, scheduler, Codex’s Baby, timeline tree.
    - `/packages/echo-cli` — tooling launcher (future), wraps dev server and inspector.
    - `/packages/echo-adapters` — reference adapters (Pixi/WebGPU, browser input, etc.).
    - `/apps/playground` — Vite-driven sandbox for samples and inspector.
    - `/docs` — specs, diagrams, memorials (human-facing knowledge base).
    - `/tooling` — shared build scripts, benchmarking harness (future).
