<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo

An deterministic causal graph-rewrite simulation engine.

Echo is designed for anything high-frequency, like real-time interactive simulations, to massive-scale causal graph analysis, providing bit-perfect replayability across platforms and concurrency profiles as an inherent system property.

[![Determinism CI](https://github.com/flyingrobots/echo/actions/workflows/determinism.yml/badge.svg)](https://github.com/flyingrobots/echo/actions/workflows/determinism.yml)
[![CI](https://github.com/flyingrobots/echo/actions/workflows/ci.yml/badge.svg)](https://github.com/flyingrobots/echo/actions/workflows/ci.yml)

![ECHO](https://github.com/user-attachments/assets/bef3fab9-cfc7-4601-b246-67ef7416ae75)

Echo executes parallel graph rewrites with 0-ULP cross-platform determinism, structurally preventing concurrency issues through immutable snapshots and canonical delta merging.

Echo is designed for everything from high-frequency game logic to massive-scale causal graph analysis. It provides bit-perfect replayability across platforms and concurrency profiles as an inherent system property, not an afterthought.

## Built Different

> **You debug**: "Mutating hierarchical state machines" while struggling with concurrency issues.  
> **Echo says**: "State is immutable; it evolves canonically with optics that produce holograms"—and abolished concurrency issues by choosing better invariants.

Echo isn't just another simulation engine; it is a causal substrate. While traditional runtimes struggle to manage concurrency through locks and "hope," Echo eliminates concurrency issues by making them mathematically impossible by construction.

### WARP Graphs: The Double-Decker Architecture

At its core, Echo operates on WARP graphs (**W**orldline **A**lgebra for **R**ecursive **P**rovenance). Unlike flat data structures, a WARP graph is a recursive, "double-decker" system:

1. **The Skeleton Plane**: Provides the immutable geometric structure.
2. **The Attachment Plane**: Where values and data live.

In Echo, nodes and edges can both host attachments—and those attachments can be WARP graphs themselves. It is recursion in its purest form: **graphs all the way down**.

### Canonical Evolution via DPO Rewriting

Instead of modeling state as a hierarchy of mutable containers, WARP graphs are strictly immutable. When Echo "ticks," it does not mutate the graph; it admits a new state.

Echo uses Optics to perform lawful Double Push-Out (DPO) graph rewriting. The runtime "cuts out" a specific semantic footprint and replaces it with a new configuration, producing two inseparable artifacts:

1. **A New State**: The next canonical step in the worldline.
2. **A Provenance Payload**: A minimal, cryptographic witness of the transition.

### Holography & The Death of "Debugging"

Because every transition produces an information-complete witness, WARP graphs are inherently holographic. An Echo state contains an encoded boundary structure that—combined with its provenance—can recover any previous state in its entire causal history.

This enables Always-On Time-Travel. In Echo, "debugging" is a legacy term. Since state evolves through a deterministic algorithm that selects optics with non-overlapping focal closures, every run is bit-perfect and repeatable across varying hardware and concurrency profiles. Each tick is cryptographically verifiable, meaning every time Echo simulates a given tick, it produces the same bit-perfect state every time.

You aren't "debugging" a ghost in the machine; you are a `ReaderHead` performing forensic revelation—stepping backward or jumping across the worldline to examine the autobiography of your data.

### Observer Geometry

An observer in Echo is not a scalar; it is a structural 5-tuple $\Omega = (O, B, M, K, E)$ that defines the aperture of revelation:

- Projection ($O$): The mapping from the causal substrate to the display.
- Basis ($B$): The native coordinate system of events.
- State ($M$): The accumulated observational memory.
- Update ($K$): The transition law for integrating new observations.
- Emission ($E$): The structural description produced by the observation act.

### Aperture & Degeneracy

**Aperture**: The measure of what task-relevant distinctions survive observation. We distinguish between _Projection_, _Basis_, and _Accumulated_ apertures to govern what is visible in a raw trace versus what is revealed over time.

**Degeneracy**: The hidden multiplicity behind an observation. The debugger's job is to surface degeneracy, not collapse it. Through counterfactual inspection, Echo allows you to explore the degeneracy of the worldline—seeing not just what happened, but the "plurality" of what could have been.

### Worldlines & Suffix Transport

Worldlines are not timelines. A worldline is a causal history—a chain of patches with deterministic materialization.

Echo achieves massive parallel execution by maximizing hardware resources without locks or synchronization. The same input always produces the same output, cryptographically verifiable, whether the host is a legacy machine or future hardware. Every replay of a given frame is always bit-for-bit identical, regardless of the concurrency profile.

## TLDR; What's Special About Echo?

- **Parallelism without Synchronization**: Rewrite rules read from an immutable snapshot and write to a private delta. Deltas merge in canonical order, ensuring the same result on 1 thread or 32.
- **0-ULP Determinism**: Identical tick hashes across Linux, macOS, and Windows. Echo bans standard floats, system time, and unseeded randomness to ensure absolute inevitability.
- **WARP Substrate**: Native support for tamper-evident artifacts, always-on time-travel, and counterfactual forking.
- **Footprint Enforcement**: Rules declare their graph regions (read/write sets). The scheduler ensures independence, and the runtime poisons any deltas that violate their contracts.

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
- **[Architecture](./docs/architecture-outline.md)**: The authoritative system map and layer model.
- **[DIND](./docs/dind-harness.md)**: Determinism verification and the "Drill Sergeant" discipline.
- **[Theory](./docs/THEORY.md)**: Theoretical foundations (AION Foundations series).
- **[Continuum](./CONTINUUM.md)**: The multi-repo system model and hot-runtime role.

---

Built with terminal ambition by [FLYING•ROBOTS](https://github.com/flyingrobots)
