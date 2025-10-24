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

<img src="https://github.com/user-attachments/assets/d31abba2-276e-4740-b370-b4a9c80b30de" height="500" align="right" />


> _Echo is a recursive metagraph (RMG) simulation engine that executes and rewrites typed graphs deterministically across branching timelines and merges them through confluence._

### Say what??

**Echo is an ambitious, mind-bending, fundamentally different computational model for game engines and other interactive simulations.** The RMG is a powerful mathematical tool that brings the full weight of textbook category theory to interactive computational experiences. 

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

## **tl;dr:**

> ECHO is a game engine that treats _everything_—code, data, and time itself—as one big living graph.
> It’s built so every change can branch, merge, and replay perfectly.

---

### The short pitch

ECHO runs on something called an **RMG (Recursive Meta-Graph)**. Think of it as a graph-based operating system. Everything in the engine (worlds, entities, physics, shaders, even the tools) lives inside that graph.

Echo doesn’t “update objects.” It _rewrites_ parts of the graph using a set of deterministic rules. That’s what “graph rewriting” means.

### Why this is cool

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

> *"Things are only impossible until they're not."* — Jean-Luc Picard

Can your game engine do...

### Perfect Determinism 

Same input graph + same rules = same output, always. This is huge for:

- Networked multiplayer (no desync, ever)
- Replays (just store the initial state + inputs)
- Testing (reproducible bugs)
- Time travel debugging

### Branching Timelines 

> “All we have to decide is what to do with the time that is given us.” — _Gandalf, The Lord of the Rings_

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

## Current Status

Echo is in **active development**. We're currently:

- ✅ Formal proofs of confluence (tick-level determinism proven)
- ✅ [C implementation](http://github.com/meta-graph/core) of independence checks and footprint calculus
- ✅ 200-iteration property tests validating commutativity
- 🚧 Performance optimization (subgraph matching, spatial indexing)
- 🚧 Rust rewrite of core runtime
- ❌ Lua scripting integration (not started)
- ❌ Rendering backend (not started)

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

- Read [`docs/echo/architecture-outline.md`](docs/echo/architecture-outline.md) for the full spec (storage, scheduler, ports, timelines).
- Explore [`docs/echo/diagrams.md`](docs/echo/diagrams.md) for Mermaid visuals of system constellations and the Chronos loop.

---

## Contributing

> ***WANTED:** Somebody to go back in time with me. This is not a joke.*
> *P.O. Box 91, Ocean View, WA 99393.*
> *You’ll get paid after we get back. Must bring your own weapons.*
> *I have only done this once before. **Safety not guaranteed.***

- Start each task by verifying a clean git state and branching (`echo/<feature>` recommended).
- Tests go in `packages/echo-core/test/` (fixtures in `test/fixtures/`). End-to-end scenarios will eventually live under `apps/playground`.
- Use expressive commits (`subject` / `body` / optional `trailer`)—tell future us the *why*, not just the *what*.
- Treat determinism as sacred: prefer Echo’s PRNG, avoid non-deterministic APIs without wrapping them.
  
### Development Principles

1. **Tests First** – Write failing unit/integration/branch tests before new engine work.
2. **Branch Discipline** – Feature branches target `main`; keep `main` pristine.
3. **Document Ruthlessly** – Update specs/diagrams and log decisions.
4. **Temporal Mindset** – Think *Chronos* (sequence), *Kairos* (possibility), *Aion* (significance) whenever touching runtime code.

### Roadmap Highlights

- [x] **Phase 0** – Finalize specs and design.
- [ ] **Phase 1** – Ship Echo Core MVP with tests and headless harness.
- [ ] **Phase 2 “Double-Jump”** – Deliver reference render/input adapters and the playground.
- [ ] **Phase 3+** – Physics, WebGPU, audio, inspector, and full temporal tooling.

Chrononauts welcome. Strap in, branch responsibly, and leave the timeline cleaner than you found it.
