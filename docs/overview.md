<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo: The Causal Operating System

Echo is a high-performance, deterministic graph-rewriting engine designed for the next century of simulation, gaming, and distributed systems. It is not merely a game engine; it is a **Causal Kernel** that treats Time and Determinism as first-class primitives rather than post-hoc features.

## What is Echo?

Echo (part of the **Continuum** project) reimagines the relationship between state, logic, and time. By utilizing a custom **WARP graph substrate**, Echo bypasses the performance limitations of traditional version control (like Git) and the non-deterministic drift of standard simulation loops.

### The Core Mandate: Inevitability

Echo does not strive for "correctness"—it asserts **Inevitability**. Through **DIND** (Deterministic Ironclad Nightmare Drills), the engine ensures that a given set of inputs will produce the exact same state hash across any platform, any architecture, and any timeframe.

---

## How It Works

### 1. The Intent ABI (SLAPS)

All mutations enter the system as **Intents**. There is no "global state" to poke; there are only admitted intents processed through a deterministic pipeline.

- **Domain Intents:** Gameplay or simulation logic.
- **Control Intents:** Privileged operations like starting the scheduler or admitting a new worldline fork.

### 2. The WARP Substrate

Echo uses a directed graph of Nodes and Edges where logic is attached via **Atoms**. Unlike ECS models that rely on linear arrays, Echo’s graph allows for complex, multi-layered relationships that remain bit-exact during serialization.

### 3. Worldlines & Honest Clocks

Echo decouples "Logical Position" from "Real-World Time."

- **WorldlineTick:** The unique append-index of a specific history.
- **GlobalTick:** A correlation stamp representing a specific scheduler cycle (SuperTick).
- **Temporal Sovereignty:** Every worldline can be forked, scrubbed, or paused independently without affecting the global state.

### 4. Hexagonal Port Architecture

Echo is renderer-agnostic. The core logic lives in a pure Rust/Wasm "Spine," communicating with the outside world (Three.js, physics solvers, input devices) through bit-exact **Ports**.

---

## Market Analysis & Positioning

### Target Domains

- **Simulation & Digital Twins:** High-fidelity systems requiring 100% replay integrity for audit and analysis.
- **Competitive Gaming:** Native support for lockstep networking and rollback without desync risk.
- **AI Training & Orchestration:** Using worldline forking to run millions of speculative "what-if" scenarios in parallel.
- **Causal Computing:** Systems where the provenance of data (who changed what and why) is as important as the data itself.

### The Competition: Nearest Neighbors

| Product                 | Paradigm          | Determinism                       | Time Travel              | Scalability               |
| :---------------------- | :---------------- | :-------------------------------- | :----------------------- | :------------------------ |
| **Bevy / Unity DOTS**   | ECS               | Optimistic (Requires manual care) | Complex (Snapshot-based) | High (CPU Cache)          |
| **Git**                 | Content-Addressed | Absolute                          | Historical Only          | Low (Slow with many refs) |
| **Actor Models (Akka)** | Message Passing   | Low (Order dependent)             | None                     | Very High                 |
| **Echo**                | **Causal Graph**  | **Absolute (Enforced)**           | **Native (Worldlines)**  | **High (WARP Substrate)** |

---

## Feature Matrix

| Feature         | Echo (ABI v3+)                                                       | Traditional Engines                                        |
| :-------------- | :------------------------------------------------------------------- | :--------------------------------------------------------- |
| **Determinism** | **Hard-Locked.** Fixed-point math and sorted-key CBOR are mandatory. | **Soft.** Float-drift and unseeded randoms are common.     |
| **Clocks**      | **Typed Coordinates.** (Worldline vs Global)                         | **Verbs.** (dt / Step)                                     |
| **Replay**      | **Provenance-Native.** Replay is the primary read-path.              | **Post-hoc.** Requires custom serialization/event logging. |
| **Forking**     | **Native.** Zero-copy branching of the entire simulation.            | **Expensive.** Requires full state cloning.                |
| **Safety**      | **Panic-Free & Zero-Global.** No `unsafe` in core logic.             | **Variable.** Often relies on global pointers/singletons.  |
| **Boundaries**  | **Hexagonal Ports.** bit-exact WASM FFI.                             | **Tight Coupling.** Logic tied to a specific renderer.     |

---

## Strategic Roadmap

1. **Phase 6 (Current):** ABI v3 transition. Establishing "Honest Clocks" and runtime-owned scheduling.
2. **Phase 7:** **Full-State Replay.** Perfect reconstruction of graph state including portal and instance operations.
3. **Phase 8:** **Wesley Schema Freeze.** Stabilizing the cross-language bridge for production use.
4. **Phase 9-11:** **Cross-Worldline Transport.** Handling causal interference and deterministic conflict resolution between timelines.

---

## Why Echo?

Traditional software is built on "Now." Echo is built on **"Always."**

By treating the simulation as a record of causal events rather than a series of frames, Echo provides the stability required for the next generation of complex, autonomous, and distributed systems. It is the engine for builders who refuse to accept desync, drift, or "it works on my machine" as valid engineering outcomes.
