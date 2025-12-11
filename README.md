<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# ![[echo-white.svg]]
---

> [!note]
> # ⚠️ NOTICE: Echo is Becoming the JITOS Kernel
>
> Echo is now the kernel for **JITOS**—the world's first causal operating system, where history is immutable, execution is deterministic, and debugging means time-traveling to exact states, instead of hopes and prayers. 
>
> **THE REVΩLUTION WILL BE DETERMINISTIC.**  
> **THE PROOF IS MATHEMATICAL.**  
> **TIME WILL TELL.**  
>
> 🔗 [AIΩN Organization](https://github.com/your-org) | [JITOS RFCs](https://jitos.dev/rfcs) | [CΩMPUTER Paper](link)

```rust
//! ░▒▓████████▓▒░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░░▒▓██████▓▒░
//! ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░
//! ░▒▓█▓▒░     ░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░
//! ░▒▓██████▓▒░░▒▓█▓▒░      ░▒▓████████▓▒░▒▓█▓▒░░▒▓█▓▒░
//! ░▒▓█▓▒░     ░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░
//! ░▒▓█▓▒░     ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░
//! ░▒▓████████▓▒░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░░▒▓██████▓▒░
//!
//! “What we do in life ECHOES through eternity”
//! (Recursively, in the Metaverse)
```

## **tl;dr:**

> Echo is a recursive metagraph (RMG) simulation engine that treats _everything_–code, data, and time itself—as one big living graph.
> It’s built so every change can branch, merge, and replay perfectly.

<img src="https://github.com/user-attachments/assets/d31abba2-276e-4740-b370-b4a9c80b30de" height="400" align="right" />

### Say what??

**Echo is an ambitious, mind-bending, radically different computational model for game engines and other interactive simulations.** The RMG is a powerful mathematical tool that brings the full weight of textbook category theory to interactive computational experiences. 

Most game engines are object-oriented state machines. Unity, Unreal, Godot all maintain mutable object hierarchies that update every frame. Echo says: "No, everything is a graph, and the engine rewrites that graph deterministically using typed transformation rules." 

Echo is fundamentally **built different**.

RMG provides atomic, in-place edits of recursive meta-graphs with deterministic local scheduling and snapshot isolation. It’s the core of the Echo engine: runtime, assets, networking, and tools all operate on the same living graph of graphs.

Echo is a mathematically rigorous game engine that replaces traditional OOP with deterministic graph rewriting, enabling time-travel debugging, perfect replay, and Git-like branching for game states.

### Core Principles

| Principle                | Description                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Everything is a graph    | Nodes, edges, even rewrite rules are all graphs.                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Recursive                | Graphs contain subgraphs without limit.                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Typed                    | Every node and edge carries a type hash and schema metadata.                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| DPO-i Graph Rewriting    | Based on Double Pushout approach with deterministic local scheduler (DPOi = deterministic parallel-order incremental).                                                                                                                                                                                                                                                                                                                                                                                     |
| Atomic in-place edits    | Mutations apply directly to the graph with snapshot isolation.                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Confluence               | Independent rewrite sequences that overlap converge to the same canonical graph.                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Snapshots, not logs      | Snapshots are emitted from the live graph; append-only history is optional.                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Deterministic scheduling | The same set of rewrite rules applied to the same graph always yields identical results.                                                                                                                                                                                                                                                                                                                                                                                                                   |
| QCA-Ready                | Rules can express reversible and superposed transformations for future quantum/parallel semantics. Quantum? Yep. But in this context it means the rewrite engine can be extended to tag rules as **reversible**: every transformation can be walked backward without loss. Built-in, lossless rollback and causal replay, the ability to run parallel speculative branches deterministically, and, yes, quantum, meaning that a research path toward quantum / probabilistic simulation modes is possible. |

---

### What's Echo?

Echo runs on something called an **RMG (Recursive Meta-Graph)**. Think of it as a graph-based operating system. Everything in the engine (worlds, entities, physics, shaders, even the tools) lives inside that graph.

Echo doesn’t “update objects.” It _rewrites_ parts of the graph using a set of deterministic rules. That’s what “graph rewriting” means.

## JITOS Engineering Standard (Living Specs)

Echo follows the JITOS Engineering Standard: every SPEC is simultaneously documentation, implementation, interactive demo (WASM), living test, and contributor certification. See `docs/METHODOLOGY.md` for the full 5x Duty model and workflow.

### Why Echo's Cool

- **Deterministic:** same inputs = same world every time.
- **Branching:** you can fork reality, change it, and merge it back without chaos.
- **Confluent:** independent edits always end up in the same final state.
- **Snapshot-based:** you can freeze the whole graph at any moment for replay or rollback.
- **Recursive:** a node can contain its own sub-graph—systems inside systems.

### In Plain English

Echo feels like if Minecraft, Git, and a physics engine had a baby that understood time travel.
You can pause time, fork a copy of reality, try out a new idea, and merge the timelines back together, without breaking determinism.

---

## Advantages

> _"Things are only impossible until they're not." — Jean-Luc Picard_

Can your game engine do...

### Perfect Determinism 

Same input graph + same rules = same output, always. This is huge for:

- Networked multiplayer (no desync, ever)
- Replays (just store the initial state + inputs)
- Testing (reproducible bugs)
- Time travel debugging

### Branching Timelines 

> _“All we have to decide is what to do with the time that is given to us.” — Gandalf, The Lord of the Rings_

The Git metaphor is accurate. Fork reality, try something, merge back. This enables:

- Speculative execution
- "What if?" simulation
- Save/load that's mathematically guaranteed to work

### Confluence

Independent changes converge to the same result. This is operational transformation meets game engine, and it's bonkers powerful for:

- Collaborative editing
- Distributed simulation
- Conflict-free merges

### Everything-is-a-graph 

Rules are graphs. Systems are graphs. The whole runtime is a graph. This gives you:

- Introspection at every level
- Hot-reloading without special cases
- Tools that operate on the same substrate as the engine
- Zero-copy loading

---

| Principle | Vision | Implementation |
| :--- | :--- | :--- |
| **Determinism (The "Replay")** | Same input graph + same rules = same output graph. Always. This is huge for networked multiplayer (no desync), perfect replays, and reproducible bug testing. | Achieved via an $O(n)$ deterministic scheduler. Pending graph rewrites are sorted using a stable radix sort (not a comparison-based sort) based on their scope, rule ID, and nonce. Combined with a deterministic math module (Vec3, Quat, PRNG), this ensures identical inputs always produce identical execution order and final state. |
| **Branching & Confluence (The "Time Travel")** | Fork reality, try something, and merge it back like a Git branch. Independent, non-conflicting changes converge to the same canonical state, guaranteed. | The engine's Timeline Tree (modeling Chronos, Kairos, and Aion) allows for branching realities. The engine's core transaction model (begin, apply, commit) and footprint-based independence checks (MWMR) allow for safe, parallel execution and deterministic, conflict-free merges. |
| **Snapshot Isolation (The "Commit")** | Snapshots are emitted from the live graph; append-only history is optional. This enables save/load, time-travel debugging, and collaborative editing. | Each commit produces two Merkle hashes derived from 256-bit BLAKE3: <ul><li><code>state_root</code>: deterministic hash of the reachable graph state under the current root.</li><li><code>commit hash</code> (commit_id): hash of a canonical header including <code>state_root</code>, parents, and deterministic digests for plan/decisions/rewrites.</li></ul> See <code>docs/spec-merkle-commit.md</code> for the precise encoding and invariants. |
| **Everything-is-a-Graph (The "Substrate")** | Nodes, edges, systems, assets, and even rewrite rules are all graphs. Graphs can contain subgraphs recursively. | The engine operates on typed, directed graphs. All identifiers (NodeId, TypeId, EdgeId) are domain-separated BLAKE3 hashes. Rewrite Rules are defined with a matcher, executor, and compute_footprint function, allowing the engine to deterministically transform the graph. |
| **Hexagonal Architecture (The "Ports")** | A core engine that is pure logic, completely decoupled from the outside world (rendering, input, networking). | Echo uses a Ports & Adapters design. The core engine (rmg-core) knows nothing of pixels, sockets, or key presses. It exposes narrow interfaces ("Ports") that external crates ("Adapters") implement. This allows swapping renderers (e.g., WebGPU, SDL) or physics engines without changing the core simulation logic. |

---

## Architecture

> _“Roads? Where we’re going, we don’t need roads.” — Doc Brown, Back to the Future_

Echo is a Rust workspace organized into a multi-crate setup. The core engine is pure, dependency-free Rust (#![no_std] capable) with I/O isolated to adapter crates.

```bash
echo/
├── crates/
│   ├── rmg-core/        (Core engine: RMG, scheduler, transaction model, snapshotting)
│   ├── rmg-geom/        (Geometry primitives: AABB, transforms, broad-phase)
│   ├── rmg-benches/     (Criterion microbenchmarks: snapshot_hash, scheduler_drain)
│   ├── rmg-wasm/        (WebAssembly bindings for tools and web)
│   ├── rmg-ffi/         (C ABI for host integrations; Rhai is embedded directly)
│   └── rmg-cli/         (Command-line interface, demos launcher)
├── docs/                (Comprehensive specifications and diagrams)
└── scripts/             (Build automation, benchmarking)
```

### Core Architectural Layers

1. ECS (Entity-Component-System): Type-safe components with archetype-based storage
2. Scheduler: Deterministic DAG ordering via O(n) radix sort
3. Event Bus: Command buffering for deterministic event handling
4. Timeline Tree: Branching/merging with temporal mechanics (Chronos, Kairos, Aion)
5. Ports & Adapters: Renderer, Input, Physics, Networking, Audio, Persistence
6. Deterministic Math: Vec3, Mat4, Quat, PRNG with reproducible operations

### Key Technical Concepts

#### Recursive Meta Graph Core

The engine operates on typed, directed graphs:

- Nodes = typed entities with component data
- Edges = typed relationships between nodes
- Rules = deterministic transformations that match patterns and rewrite subgraphs

All identifiers are 256-bit BLAKE3 hashes with domain separation:

```rust
pub type Hash = [u8; 32];
pub struct NodeId(pub Hash);   // Entities
pub struct TypeId(pub Hash);   // Type descriptors
pub struct EdgeId(pub Hash);   // Relationships
```

#### Deterministic Rewriting

Each tick follows a transaction model:

1. begin()         → Create new transaction
2. apply(tx, rule) → Enqueue pending rewrites
3. commit(tx)      → Execute in deterministic order, emit snapshot

#### $O(n)$ Deterministic Scheduler

Rewrites are ordered using stable radix sort (not comparison-based):  

- Order: (`scope_hash`, `rule_id`, `nonce`) lexicographically
- Time: $O(n)$ with 20 passes of 16-bit radix digits

This ensures identical initial state + rules = identical execution order and final snapshot.

#### Snapshot Hashing (Merkle Commits)

Two hashes per commit:

- `state_root`: BLAKE3 of canonical graph encoding (sorted nodes/edges)
- `commit_id`: BLAKE3 of commit header (`state_root` + parent + plan + decisions + rewrites)

### Footprints & Independence (MWMR)

For parallel rewriting:

```rust
struct Footprint {
  n_read, n_write: IdSet,     // Node reads/writes
  e_read, e_write: IdSet,     // Edge reads/writes
  b_in, b_out: PortSet,       // Boundary ports
  factor_mask: u64,           // Spatial partitioning hint
}
```

Disjoint footprints = independent rewrites = safe parallel execution.

### Component Interaction

#### `rmg-core` (`crates/rmg-core/src/`)

- `engine_impl.rs`: Transaction lifecycle, rewrite application
- `scheduler.rs`: $O(n)$ radix drain, conflict detection
- `graph.rs`: BTreeMap-based node/edge storage
- `snapshot.rs`: State root and commit ID computation
- `rule.rs`: Rewrite rule definitions with pattern matching
- `footprint.rs`: Independence checks for concurrent execution
- `math/`: Deterministic Vec3, Mat4, Quat, PRNG

## Execution Flow

```c
loop {
  let tx = engine.begin();

  // Application phase
  for rule in rules_to_apply {
      engine.apply(tx, rule, &scope)?;
  }

  // Deterministic execution
  let snapshot = engine.commit(tx)?;

  // Emit to networking, tools, etc.
  publish_snapshot(snapshot);
}
```

## Design Principles

1. Determinism as Foundation: Every operation must produce identical results given identical input
2. Snapshot Isolation: State captured as immutable graph hashes (not event logs)
3. Hexagonal Architecture: Core never touches I/O directly; all flows through ports
4. Dependency Injection: Services wired at bootstrap for hot-reload support
5. Property-Based Testing: Extensive use of proptest for mathematical invariants

## Current Status

Phase 1 MVP (active development on echo/pr-12-snapshot-bench):

✅ Completed:
- Formal confluence proofs (tick-level determinism proven)
- Rust core runtime with transaction model
- 200+ property tests validating commutativity
- Benchmark infrastructure with D3 dashboard

🚧 In Progress:
- Performance optimization (subgraph matching, spatial indexing)
- Temporal mechanics integration

## Key Files to Explore

### Documentation

- `README.md` — Project vision
- `docs/architecture-outline.md` — Full system design
- `docs/spec-rmg-core.md` — RMG Core spec v2
- `docs/spec-merkle-commit.md` — Snapshot hashing spec
- `docs/spec-scheduler.md` — Deterministic scheduler design

### Core Implementation

- `crates/rmg-core/src/engine_impl.rs` — Engine core
- `crates/rmg-core/src/scheduler.rs` — O(n) scheduler
- `crates/rmg-core/src/snapshot.rs` — Merkle hashing
- `crates/rmg-core/src/demo/motion.rs` — Example rewrite rule

### Tests & Benchmarks:

- `crates/rmg-core/tests/permutation_commute_tests.rs` — Determinism proofs
- `crates/rmg-benches/benches/snapshot_hash.rs` — Hashing throughput

## Developer: Running Benchmarks

- Command (live dashboard): `make bench-report`
  - Runs `cargo bench -p rmg-benches`, starts a local server, and opens the dashboard at `http://localhost:8000/docs/benchmarks/`.
- Command (offline static file): `make bench-bake`
  - Runs benches and bakes `docs/benchmarks/report-inline.html` with results injected so it works over `file://` (no server required).
- Docs: see `crates/rmg-benches/benches/README.md` for details, tips, and report paths.

---

## Contributing

> ***WANTED:** Somebody to go back in time with me. This is not a joke.*
> *P.O. Box 91, Ocean View, WA 99393.*
> *You’ll get paid after we get back. Must bring your own weapons.*
> *I have only done this once before. **Safety not guaranteed.***

- Start each task by verifying a clean git state and branching (`echo/<feature>` recommended).
- Tests go in `packages/echo-core/test/` (fixtures in `test/fixtures/`). End-to-end scenarios will eventually live under `apps/playground`.
- Use expressive commits (`subject` / `body` / optional `trailer`). Tell future us the *why*, not just the *what*.
- Treat determinism as sacred: use Echo’s PRNG, avoid non-deterministic APIs without wrapping them.
  
### Git Hooks

Install the repo’s hooks so formatting and quick checks run before commits:

```
make hooks
```

### Development Principles

1. **Tests First** – Write failing unit/integration/branch tests before new engine work.
2. **Branch Discipline** – Feature branches target `main`; keep `main` pristine.
3. **Document Ruthlessly** – Update specs/diagrams and log decisions.
4. **Temporal Mindset** – Think *Chronos* (sequence), *Kairos* (possibility), *Aion* (significance) whenever touching runtime code.

### Roadmap Highlights

✅ **Phase 0** – Finalize specs and design.  
⏳ **Phase 1** – Ship Echo Core MVP with tests and headless harness.  
☑️ **Phase 2** – Deliver reference render/input adapters and **the playground**.  
☑️ **Phase 3+** – Physics, WebGPU, audio, inspector, and full temporal tooling.  

**Chrononauts welcome.** Strap in, branch responsibly, and leave the timeline cleaner than you found it.

---

## License

**Licensing split:**

- Code (all source/build/tooling): [Apache 2.0](./LICENSE-APACHE)
- Theory / math / docs corpus: [Apache 2.0](./LICENSE-APACHE) OR [MIND-UCAL v1.0](./LICENSE-MIND-UCAL)

If you do not wish to use MIND-UCAL, you may freely use all theory, math, and
documentation under Apache 2.0 alone. No part of this repository requires
adopting MIND-UCAL.

See [`LICENSE`](./LICENSE) for the summary and [`NOTICE`](./NOTICE) for attribution.

© 2025 James Ross   
Ω [FLYING•ROBOTS](https://github.com/flyingrobots)
