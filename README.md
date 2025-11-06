# Echo

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

> _Echo is a recursive metagraph (RMG) simulation engine that treats _everything_–code, data, and time itself—as one big living graph.
> It’s built so every change can branch, merge, and replay perfectly.

<img src="https://github.com/user-attachments/assets/d31abba2-276e-4740-b370-b4a9c80b30de" height="400" align="right" />

### Say what??

**Echo is an ambitious, mind-bending, radically different computational model for game engines and other interactive simulations.** The RMG is a powerful mathematical tool that brings the full weight of textbook category theory to interactive computational experiences. 

Most game engines are object-oriented state machines. Unity, Unreal, Godot all maintain mutable object hierarchies that update every frame. Echo says: "No, everything is a graph, and the engine rewrites that graph deterministically using typed transformation rules." 

Echo is fundamentally **built different**.

RMG provides atomic, in-place edits of recursive meta-graphs with deterministic local scheduling and snapshot isolation.

It’s the core of the Echo engine: runtime, assets, networking, and tools all operate on the same living graph of graphs.

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

Echo is a Rust workspace organized into a multi-crate setup. The core engine is pure, dependency-free Rust (#![no_std] capable) with I/O isolated to adapter crates.

```bash
echo/
├── crates/
│   ├── rmg-core/        (Core engine: RMG, scheduler, transaction model, snapshotting)
│   ├── rmg-geom/        (Geometry primitives: AABB, transforms, broad-phase)
│   ├── rmg-benches/     (Criterion microbenchmarks: snapshot_hash, scheduler_drain)
│   ├── rmg-wasm/        (WebAssembly bindings for tools and web)
│   ├── rmg-ffi/         (C ABI for Lua/host integration)
│   └── rmg-cli/         (Command-line interface, demos launcher)
├── docs/                (Comprehensive specifications and diagrams)
└── scripts/             (Build automation, benchmarking)
```

### Core Layers (inside `rmg-core`)

**ECS**: Archetype-based Entity-Component-System storage.  
**Scheduler**: Deterministic $O(n)$ radix-sort-based rule scheduler.  
**GraphStore**: In-memory BTreeMap-based graph storage (ensures deterministic iteration).  
**Transaction Model**: The `begin()`, `apply()`, `commit()` lifecycle for graph rewrites.  
**Snapshotting**: `state_root` and `commit_id` (Merkle commit) computation.  
**Footprints**: Independence checks for Multi-Writer-Multi-Read (MWMR) concurrency.  
**Deterministic Math**: safe Vec3, Mat4, Quat, and PRNG.  

---

## Project Status: Phase 1 MVP (Active Development)

The project is currently focused on proving the core determinism and performance of the RMG model.

✅ Formal proofs of confluence (tick-level determinism proven).  
✅ C implementation of independence checks and footprint calculus.  
✅ Rust core runtime (`rmg-core`) with the transaction model and $O(n)$ scheduler.  
✅ Comprehensive property-based test suite (200+ iterations validating commutativity).  
✅ Benchmark infrastructure (`rmg-benches`) with a D3 visualization dashboard.  
🚧 Performance optimization (subgraph matching, spatial indexing).   
🚧 Temporal mechanics (Aion integration for significance/agency).  
🚧 Radix sort micro-tuning for scheduler.  
☑️ (not yet started) Lua scripting integration (via `rmg-ffi`).  
☑️ (not yet started) Rendering backend (adapters planned).  
☑️ (not yet started) Full physics engine integration.  
☑️ (not yet started) Inspector tooling (via rmg-wasm).

## Learning the Vision

> _“Roads? Where we’re going, we don’t need roads.” — Doc Brown, Back to the Future_

- Read [docs/architecture-outline.md](./docs/architecture-outline.md) for the full spec.
- Explore [docs/diagrams.md](./docs/diagrams.md) for Mermaid visuals.
- Track active work in [docs/execution-plan.md](./docs/execution-plan.md).

---

### The Math Checks Out

The mathematical properties of RMGs offer:

- **Folds (catamorphisms)**: there is a guaranteed, one-true way to “walk” the graph.
  - That’s how rendering, physics, and serialization all stay consistent: they’re just different folds over the same data.
- **Double-Pushout (DPO) rewriting**: a safe, proven way to modify graphs.
  - Instead of ad-hoc mutation, every change is a rewrite rule with an explicit match and replacement, so the engine can reason about merges, rollbacks, and conflicts.
- **Confluence** – when two people or two threads make compatible edits, they deterministically converge to the same state.
  - That’s the key to multiplayer sync, time-travel debugging, and collaborative editing.

There's a ton of other advanced reasons why it's cool, but that's nerd stuff. Let's just say that the RMG is weird, and **extremely powerful.**

---

### Learning the Vision

> *“Roads? Where we’re going, we don’t need roads.” — Doc Brown, Back to the Future*

- Read [`docs/architecture-outline.md`](docs/architecture-outline.md) for the full spec (storage, scheduler, ports, timelines).
- Explore [`docs/diagrams.md`](docs/diagrams.md) for Mermaid visuals of system constellations and the Chronos loop.
- Honor Caverns with [`docs/memorial.md`](docs/memorial.md)—we carry the torch forward.
- Peek at [`docs/legacy-excavation.md`](docs/legacy-excavation.md) to see which ideas survived the archaeological roast.
- Track active work in [`docs/execution-plan.md`](docs/execution-plan.md); update it every session.

### Docs

- Dev server: `make docs` (opens browser; uses `PORT` env var, default 5173)
- Static build: `make docs-build`
- Single-file rollup: `make echo-total` (generates `docs/echo-total.md` from top‑level docs; commit the result if it changes)

### CI Tips

- Manual macOS run: trigger the "CI (macOS — manual)" workflow from the Actions tab to run fmt/clippy/tests on macOS on demand.
- Reproduce CI locally:
  - Format: `cargo fmt --all -- --check`
  - Clippy: `cargo clippy --all-targets -- -D warnings -D missing_docs`
  - Tests: `cargo test --workspace`
  - Rustdoc (warnings as errors): `RUSTDOCFLAGS="-D warnings" cargo doc -p rmg-core --no-deps` (repeat for other crates)
  - Security: `cargo install cargo-audit --locked && cargo audit --deny warnings`
  - Dependency policy: `cargo deny check` (requires `cargo-deny`)

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

### Running Benchmarks

Echo takes performance and determinism seriously. The `rmg-benches` crate provides detailed microbenchmarks.

**Command (live dashboard):**

```bash
make bench-report
```

- Runs `cargo bench -p rmg-benches`, starts a local server, and opens the dashboard at `http://localhost:8000/docs/benchmarks/`.

**Command (offline static file): **

```bash
make bench-bake
```

- Runs benches and bakes `docs/benchmarks/report-inline.html` with results injected so it works over `file://` (no server required).

**Docs:** 

See [`crates/rmg-benches/benches/README.md`](./crates/rmg-benches/benches/README.md) for details.
  
### Git Hooks

Install the repo’s hooks so formatting and quick checks run before commits:

```
make hooks
```

- The pre-commit hook auto-fixes formatting by default (runs `cargo fmt --all`).
- To switch to check-only mode for a commit, set `ECHO_AUTO_FMT=0`

```
ECHO_AUTO_FMT=0 git commit -m "your message"
```

You can also export `ECHO_AUTO_FMT=0` in your shell rc if you prefer check-only always.

### Development Principles

1. **Tests First** – Write failing unit/integration/branch tests before new engine work.
2. **Branch Discipline** – Feature branches target `main`; keep `main` pristine.
3. **Document Ruthlessly** – Update specs/diagrams and log decisions.
4. **Temporal Mindset** – Think *Chronos* (sequence), *Kairos* (possibility), *Aion* (significance) whenever touching runtime code.

### Roadmap Highlights

- [x] **Phase 0** – Finalize specs and design.
- [⏳] **Phase 1** – Ship Echo Core MVP with tests and headless harness.
- [ ] **Phase 2** – Deliver reference render/input adapters and **the playground**.
- [ ] **Phase 3+** – Physics, WebGPU, audio, inspector, and full temporal tooling.

**Chrononauts welcome.** Strap in, branch responsibly, and leave the timeline cleaner than you found it.

---

## License

MIT • © J. Kirby Ross • [flyingrobots](http://github.com/flyingrobots)
