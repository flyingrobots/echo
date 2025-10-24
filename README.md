# Echo Engine

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

> ECHO is a recursive metagraph simulation engine that executes and rewrites typed graphs deterministically across branching timelines and merges them through Confluence.

## Say what??

Echo is a computer program that runs a recursive metagraph (RMG), a **typed, deterministic graph-rewriting engine**.

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
| Deterministic scheduling | The same set of rewrite rules applies to the same graph always yields identical results.                                                                                                                                                                                                                                                                                                                                                                                                                   |
| QCA-Ready                | Rules can express reversible and superposed transformations for future quantum/parallel semantics. Quantum? Yep. But in this context it means the rewrite engine can be extended to tag rules as **reversible**: every transformation can be walked backward without loss. Built-in, lossless rollback and causal replay, the ability to run parallel speculative branches deterministically, and, yes, quantum, meaning that a research path toward quantum / probabilistic simulation modes is possible. |

---

## **tl;dr:**

> ECHO is a game engine that treats _everything_—code, data, and time itself—as one big living graph.
> It’s built so every change can branch, merge, and replay perfectly.

---

### The short pitch

ECHO runs on something called an **RMG (Recursive Meta-Graph)**. Think of it as a graph-based operating system. Everything in the engine (worlds, entities, physics, shaders, even the tools) lives inside that graph.

When the engine runs, it doesn’t just “update objects.” It _rewrites_ parts of the graph using a set of deterministic rules. That’s what “graph rewriting” means.

### Why this is cool

- **Deterministic:** same inputs = same world every time.
- **Branching:** you can fork reality, change it, and merge it back without chaos.
- **Confluent:** independent edits always end up in the same final state.
- **Snapshot-based:** you can freeze the whole graph at any moment for replay or rollback.
- **Recursive:** a node can contain its own sub-graph—systems inside systems.
- **QCA-ready:** the math we’re using is general enough to support reversible and parallel updates later, so the engine can do instant rollback, run speculative branches, or even experiment with quantum-style simulations down the road.

There's a ton of other advanced reasons why it's cool, but that's nerd stuff. Let's just say that the RMG is weird, and **extremely powerful.**

### In Plain English

Echo feels like if Minecraft, Git, and a physics engine had a baby that understood time travel.
You can pause time, fork a copy of reality, try out a new idea, and merge the timelines back together, without breaking determinism.

---

## Getting Started

> “All we have to decide is what to do with the time that is given us.” — _Gandalf, The Lord of the Rings_

```bash
git clone git@github.com:flyingrobots/EchoEngine.git
<coming soon>
```

---

## Learning the Vision

> *“Roads? Where we’re going, we don’t need roads.” — Doc Brown, Back to the Future*

- Read [`docs/architecture-outline.md`](docs/architecture-outline.md) for the full spec (storage, scheduler, ports, timelines).
- Explore [`docs/diagrams.md`](docs/diagrams.md) for Mermaid visuals of system constellations and the Chronos loop.
- Honor Caverns with [`docs/memorial.md`](docs/memorial.md)—we carry the torch forward.
- Peek at [`docs/legacy-excavation.md`](docs/legacy-excavation.md) to see which ideas survived the archaeological roast.
- Track active work in [`docs/execution-plan.md`](docs/execution-plan.md); update it every session.

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
