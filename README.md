<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo

An industrial-grade graph-rewrite simulation engine. Echo runs parallel graph rewrites with 0-ULP cross-platform determinism, structurally preventing concurrency issues through immutable snapshots and canonical delta merging.

Echo is designed for the systems engineer who demands geometric lawfulness in simulation. It scales from high-frequency game logic to massive-scale causal graph analysis, providing perfect replayability as an inherent system property.

[![Determinism CI](https://github.com/flyingrobots/echo/actions/workflows/determinism.yml/badge.svg)](https://github.com/flyingrobots/echo/actions/workflows/determinism.yml)
[![CI](https://github.com/flyingrobots/echo/actions/workflows/ci.yml/badge.svg)](https://github.com/flyingrobots/echo/actions/workflows/ci.yml)

![ECHO](https://github.com/user-attachments/assets/bef3fab9-cfc7-4601-b246-67ef7416ae75)

## Why Echo?

Unlike traditional engines that rely on mutexes, locks, or best-effort eventual consistency, Echo structurally prevents non-determinism at the bedrock.

- **Parallelism without Synchronization**: Every rule reads from an immutable snapshot and writes to a private delta. Deltas merge in canonical order, ensuring the same result on 1 thread or 32.
- **0-ULP Determinism**: Identical hashes across Linux, macOS, and Windows. Echo bans standard floats, system time, and unseeded randomness to ensure absolute inevitability.
- **WARP Graph Substrate**: A worldline algebra for recursive provenance. Every tick is a cryptographic commit in a hash chain, providing native time-travel debugging and counterfactual forking.
- **Footprint Enforcement**: Rules declare their graph regions (read/write sets). The scheduler ensures independence, and runtime guards poison deltas that violate their contracts.

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
