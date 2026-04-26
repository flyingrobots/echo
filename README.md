<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

<p align="center">
  <img alt="ECHO" src="https://github.com/user-attachments/assets/bef3fab9-cfc7-4601-b246-67ef7416ae75" />
</p>

<p align="center">
  <strong>A deterministic causal graph-rewrite simulation engine</strong>
</p>

<p align="center">
  <a href="docs/guide/start-here.md">Get Started</a> •
  <a href="docs/architecture/outline.md">Architecture</a> •
  <a href="CONTINUUM.md">Continuum</a> •
  <a href="docs/index.md">Docs</a> •
  <a href="https://github.com/flyingrobots/aion">AIΩN Framework</a>
</p>

<p align="center">
    <a href="https://github.com/flyingrobots/echo/actions/workflows/determinism.yml" ><img src="https://github.com/flyingrobots/echo/actions/workflows/determinism.yml/badge.svg" alt="Determinism CI" /></a>
    <a href="https://github.com/flyingrobots/echo/actions/workflows/ci.yml" ><img src="https://github.com/flyingrobots/echo/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
    <img src="https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-blue" alt="Platforms" />
</p>

Echo executes parallel graph rewrites with 0-ULP cross-platform determinism, structurally abolishing concurrency issues through immutable snapshots and canonical delta merging. Designed for everything from high-frequency interactive simulations to massive-scale causal graph analysis, Echo provides bit-perfect replayability across platforms and concurrency profiles as an inherent system property — not an afterthought.

---

## The Problem With Your Runtime

You model state as a hierarchy of mutable containers. You manage concurrency through locks, mutexes, and prayer. You "debug" by scattering print statements across a non-deterministic execution that will never reproduce the same way twice. When something goes wrong, you squint at logs and guess.

Echo doesn't fix concurrency bugs. It makes them architecturally impossible.

State is immutable. It evolves canonically through optics that produce holograms. There is nothing to lock, nothing to race, and nothing to guess about — because every transition is a witnessed, cryptographically verifiable admission of new truth.

---

## WARP Graphs

At its core, Echo operates on WARP graphs (**W**orldline **A**lgebra for **R**ecursive **P**rovenance).

A WARP graph is not a flat data structure. It is a recursive double-decker system:

- **The Skeleton Plane** provides the immutable geometric structure — the shape of the graph.
- **The Attachment Plane** is where values and data live.

Nodes and edges can both host attachments, and those attachments can themselves be WARP graphs. Graphs all the way down.

Structure and data are governed by different laws. The skeleton is the causal geometry; the attachments are the payload that rides it. Separating them means rewrites can reason about shape independently of content, and content can be projected independently of shape.

---

## How State Evolves

WARP graphs are strictly immutable. When Echo ticks, it does not mutate the graph. It admits a new state.

Echo uses **optics** to perform lawful Double Push-Out (DPO) graph rewriting. An optic declares a semantic footprint — a bounded region of the graph it intends to read and write. The runtime cuts out that footprint and replaces it with a new configuration, producing two inseparable artifacts:

1. **A new state**: the next canonical step in the worldline.
2. **A provenance payload**: a minimal, cryptographic witness of the transition — what changed, why it was lawful, and what it replaced.

The witness is not optional metadata. It is a structural consequence of the rewrite. Every tick is self-documenting by construction.

---

## Determinism by Construction

Echo achieves parallelism without synchronization. Rewrite rules read from an immutable snapshot and write to a private delta. Deltas merge in canonical order. The result is identical whether the host runs 1 thread or 32.

This is not "mostly deterministic." It is 0-ULP deterministic:

- Standard floats are banned. Echo uses fixed-point or otherwise platform-invariant arithmetic.
- System time is banned. Simulation time is a causal property of the worldline, not a wall-clock reading.
- Unseeded randomness is banned. If a tick uses randomness, the seed is part of the state.

The same tick hash on Linux, macOS, and Windows. The same tick hash today, next year, and on hardware that doesn't exist yet. This is what **absolute inevitability** means: given the same input, the output is not just likely identical — it is mathematically guaranteed.

**Footprint enforcement** is the mechanism that makes this survive parallelism. Optics declare their graph regions. The scheduler proves independence. Any delta that violates its declared contract is poisoned — not patched, not retried, but structurally rejected. The system does not tolerate lying about what you touch.

---

## Holography and the Death of Debugging

Because every transition produces an information-complete witness, WARP graphs are inherently **holographic**. An Echo state contains encoded boundary structure that — combined with its provenance chain — can recover any previous state in its entire causal history.

This gives you always-on time-travel. Not as a dev tool bolted on after the fact, but as a structural property of the substrate. Every state knows its own autobiography.

"Debugging" is a legacy term. In Echo, you are a **ReaderHead** performing forensic revelation — stepping backward, jumping across the worldline, inspecting the causal ancestry of any value. You don't hunt for bugs in a non-deterministic ghost. You read history.

---

## Observer Geometry

An observer in Echo is not a scalar. It is a structural 5-tuple that defines the **aperture of revelation**:

| Component | Name       | Role                                                       |
| --------- | ---------- | ---------------------------------------------------------- |
| **O**     | Projection | The mapping from the causal substrate to what is displayed |
| **B**     | Basis      | The native coordinate system of events                     |
| **M**     | State      | Accumulated observational memory                           |
| **K**     | Update     | The transition law for integrating new observations        |
| **E**     | Emission   | The structural description produced by the observation act |

Two concepts matter here:

**Aperture** is the measure of what task-relevant distinctions survive observation. Not everything in the worldline is visible to every observer. Aperture governs what a raw trace reveals versus what accumulates over time, split across projection aperture, basis aperture, and accumulated aperture.

**Degeneracy** is the hidden multiplicity behind an observation. Two worldline states can look identical under one projection while being structurally different underneath. The job of forensic inspection is to surface degeneracy — not collapse it. Through counterfactual forking, Echo lets you explore not just what happened, but the plurality of what could have been.

This is where the deeper architecture shows through. Observation is not a passive read. It is a structured act with its own geometry, and different observers can hold lawfully different views of the same causal history.

---

## The Broader Architecture

Echo is the engine layer of a larger stack called **WARP** — a recursive, witnessed admission architecture that governs the transition from private speculation to shared causal reality.

Above the engine, the WARP stack handles:

- **Braids**: when multiple strands of causal history meet at a frontier and must be judged — joined, preserved as plurality, surfaced as conflict, or declared obstructed.
- **Commitment, Folding, and Revelation**: three distinct operations. What becomes true. How admitted history is lawfully compressed. What a bounded observer can actually see.
- **Reliance**: trust over proof-bearing artifacts as a first-class admission domain — certificates are issued, activated, superseded, suspended, or revoked through the same witnessed kernel.

Echo provides the deterministic, holographic substrate. The upper stack provides the governance. Together, they form a system where collaboration itself becomes a witnessed, rights-bearing admission problem rather than a soft social assumption layered on top of software.

---

## In Short

| Property                                | How                                                                     |
| --------------------------------------- | ----------------------------------------------------------------------- |
| **Parallelism without synchronization** | Immutable snapshots, private deltas, canonical merge order              |
| **0-ULP cross-platform determinism**    | No floats, no system time, no unseeded randomness                       |
| **Always-on time-travel**               | Holographic provenance witnesses on every transition                    |
| **Footprint enforcement**               | Optics declare regions; the runtime poisons liars                       |
| **Observer geometry**                   | Structural 5-tuple aperture, not scalar subscription                    |
| **WARP substrate**                      | Tamper-evident, recursively provenance-bearing, graphs all the way down |

## Quick Start

### 1. Repository Setup

Install the guardrails and verify the environment.

```bash
make hooks
cargo check
```

### 2. Verify Determinism

Run the cross-platform DIND (Determinism-in-Determinism) harness.

```bash
cargo xtask dind run
```

### 3. Build Documentation

Generate the high-fidelity docs site.

```bash
make docs
```

## Stack

| Component           | Role                                                               |
| :------------------ | :----------------------------------------------------------------- |
| **`warp-core`**     | The rewrite engine, deterministic math, and transaction kernel.    |
| **`echo-app-core`** | Application lifecycle, system orchestration, and effect pipelines. |
| **`ttd-browser`**   | Browser-hostable TTD/runtime bridge surfaces over Echo WASM.       |
| **`echo-dind-*`**   | Cross-platform test harness for hash convergence verification.     |

## Documentation

- **[Guide](./docs/guide/start-here.md)**: Orientation, the fast path, and core concepts.
- **[Architecture](./docs/architecture/outline.md)**: Draft architecture map and layer model.
- **[DIND](./docs/determinism/dind-harness.md)**: Determinism verification and the "Drill Sergeant" discipline.
- **[Theory](./docs/theory/THEORY.md)**: Theoretical foundations (AION Foundations series).
- **[Continuum](./CONTINUUM.md)**: The multi-repo system model and hot-runtime role.

---

<p align="center">
  <sub>Built by <a href="https://github.com/flyingrobots">FLYING•ROBOTS</a></sub>
</p>
