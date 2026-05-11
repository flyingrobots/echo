<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Guide — Echo

This is the developer-level operator guide for Echo. Use it for orientation, the productive-fast path, and to understand how the simulation engine orchestrates the causal graph.

For deep-track doctrine, theoretical foundations (AION Foundations), and internal spec details, use [ADVANCED_GUIDE.md](./ADVANCED_GUIDE.md).

## Choose Your Lane

### 1. Build a Causal Simulation

Integrate deterministic graph rewriting into your application or game.

- **Read**: [Start Here](./docs/guide/start-here.md)
- **Host**: [Architecture](./ARCHITECTURE.md) (Engine pipeline)

### 2. Verify Determinism (DIND)

Use the "Drill Sergeant" discipline to prove cross-platform convergence.

- **Read**: [DIND Harness](./docs/dind-harness.md)
- **Run**: `cargo xtask dind run`

### 3. Time Travel Debugging

Explore the worldline algebra through the interactive debugger.

- **WASM**: [ttd-browser](./crates/ttd-browser)
- **Host**: [echo-ttd](./crates/echo-ttd)

### 4. Continuous Integration

Understand the guardrails that prevent non-determinism from entering main.

- **Check**: [`det-policy.yaml`](./det-policy.yaml)
- **Scripts**: `scripts/ban-nondeterminism.sh`

## Big Picture: System Orchestration

Echo is a tiered engine. You choose your depth based on the task:

1. **Ingress Surfaces (Surfaces)**: The CLI, WASM guest, and App Core are thin interfaces that communicate with the engine. They ensure that transitions are always structured.
2. **warp-core (The Engine)**: The primary domain kernel. It orchestrates parallel rule execution, private deltas, and canonical merge. It ensures that concurrency is structurally prevented.
3. **WARP (Memory)**: The Structural Worldline Memory that tracks the evolution of your simulation state through hash-locked ticks.

## Orientation Checklist

- [ ] **I am setting up the repo**: Run `make hooks` and `cargo check`.
- [ ] **I am writing a new rule**: Declare your `Footprint` and test against `delta_validate`.
- [ ] **I am debugging a desync**: Run `cargo xtask dind run --seed <N>` to reproduce.
- [ ] **I am contributing to Echo**: Read `METHOD.md` and `docs/BEARING.md`.

## Rule of Thumb

If you need a comprehensive spec, use the [docs/index.md](./docs/index.md) map.

If you need to know "what's true right now," use [docs/BEARING.md](./docs/BEARING.md).

If you are just starting, use the [README.md](./README.md) and the orientation tracks above.

---

**The goal is inevitability. Every state transition is a provable consequence of its causal history.**
