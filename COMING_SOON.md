<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo & Wesley: The Causal Application Guide

Welcome to the future of causal development. This document explains how **Echo** (the substrate) and **Wesley** (the law-giver) work together to create deterministic, time-travelable applications.

---

## 1. The Core Philosophy: "Law vs. Physics"

Building an application on Echo is different from traditional state-management. We split the universe into two layers:

1. **The Law (Wesley)**: Defines _what_ exists and _what_ is allowed to happen. It is expressed in GraphQL SDL with WARP directives.
2. **The Physics (Echo)**: The high-performance graph substrate that executes the laws, enforces constraints, and records the history of every atom.

---

## 2. Wesley: The Schema Compiler

Wesley is not a runtime; it is a **Law Compiler**. When you build an application, you start by writing a schema.

### Defining the Ontology

In a `.graphql` file, you define:

- **Types**: The "Atoms" of your graph (e.g., `User`, `Position`, `InventoryItem`).
- **Channels**: The event buses where data is emitted (e.g., `PhysicsUpdates`, `ChatMessages`).
- **Policies**: How data on those channels is handled (`StrictSingle`, `Reduce:Sum`, or `Log`).

### Defining Operations (The Intent ABI)

Instead of arbitrary functions, you define **Operations (Ops)**. An Op is a declaration of intent to change the graph.

```graphql
type Mutation {
    movePlayer(id: ID!, delta: Vec3!): MoveResult @warp(opId: 101)
}
```

Wesley compiles this into an **Intermediate Representation (IR)**. Echo's code generator (`echo-ttd-gen`) then consumes this IR to produce:

- Type-safe Rust structs.
- Enforcement tables (Footprints) that declare exactly which nodes an Op is allowed to read or write.

---

## 3. Echo: The Causal Substrate

Echo takes the artifacts from Wesley and provides the execution environment.

### Graph Rewrites

Every change in Echo is a **Graph Rewrite**. When an application triggers an Op (like `movePlayer`):

1. **Intent**: An `EINT` (Echo Intent) frame is created.
2. **Scheduling**: The Echo Scheduler looks at the Op's **Footprint**. If two Ops touch different parts of the graph, they can run in parallel.
3. **Execution**: The rewrite rule is applied. This is a pure function: `(PriorState, OpArgs) -> (NewState, Emissions)`.
4. **Commit**: The new state is hashed (BLAKE3) and committed to the **Provenance Store**.

### Determinism Guards

Echo enforces "Ironclad Determinism":

- **Floating Point**: All math uses `DFix64` (fixed-point) to ensure bit-exact results across Intel, ARM, and WASM.
- **No Side Effects**: Rewrite rules cannot call `Date.now()` or `Math.random()`. All entropy must be passed in as a seeded "Paradox" value.

---

## 4. The Time-Travel Debugger (TTD)

The TTD is not just a UI; it is a fundamental property of the **Provenance Store**.

### Worldlines & Forks

Because every tick is a content-addressed snapshot, Echo supports **Causal Branching**:

- **Playback**: You can seek a "Cursor" to any tick in the past.
- **Forking**: You can create a new `WorldlineId` starting from a past tick. You can then apply different intents to see a "What If" scenario.
- **Replay**: The TTD can re-play an entire session and verify that the `state_root` hashes match the "Golden" run.

### The Receipt System

Every execution produces a **TTDR Receipt**. This is a cryptographically signed proof that:
_"At Tick X, Op Y was applied to State Z, resulting in State A and Emissions B."_

---

## 5. How to Build an "Echo App"

### Step 1: The Wesley Sync

Write your schema and run `cargo xtask wesley sync`. This vendors the types and manifests into your project.

### Step 2: Implement Rewrite Rules

In Rust, you implement the logic for your Ops. Echo provides a `GraphView` that enforces your footprint at runtime.

```rust
fn handle_move_player(view: &mut GuardedView, args: MoveArgs) -> StepResult {
    let mut pos = view.get_component::<Position>(args.id)?;
    pos.x += args.delta.x;
    view.set_component(args.id, pos)?;
    Ok(Advanced)
}
```

### Step 3: Define the Scene Port

Use `echo-scene-port` to map your graph state to visual objects. This produces a `SceneDelta`—a language-agnostic list of "Add Node", "Move Edge", or "Set Label" commands.

### Step 4: The Frontend

Wire the WASM `TtdEngine` into your React/Three.js app. The engine handles the worldlines; your UI just renders the current "Truth Frames" arriving on the subscribed channels.

---

## 6. Coming Soon: The "Drill Sergeant" Workflow

We are moving toward a workflow where **Determinism isn't Optional**.

- **DIND (Deterministic Ironclad Nightmare Drills)**: Your app will be subjected to randomized operation orders to ensure it always converges to the same state.
- **Fuzzing the Law**: Wesley will generate "hostile" inputs to try and crash your rewrite rules.

_Echo is more than an engine; it is a guarantee that causality is absolute._
