<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

<p align="center">
  <img alt="ECHO" src="https://github.com/user-attachments/assets/bef3fab9-cfc7-4601-b246-67ef7416ae75" />
</p>

---

Echo is a **deterministic graph‑rewrite engine + tooling** for building simulations you can replay, verify, and synchronize without guessing.

Instead of treating a game/simulation as a pile of mutable objects, Echo treats **state as a typed graph**. Each “tick” proposes a set of rewrites, executes them in a deterministic order, and emits **cryptographic hashes** of the resulting state and provenance so tools/peers can validate and converge.

## Project Status

**Status (2026-01):** Active R&D. The deterministic core (`warp-core`) and session/tooling pipeline are implemented. Higher-level layers (ECS storage, system scheduler, timeline tree) are specced but not yet built. See [`docs/architecture-outline.md`](docs/architecture-outline.md) for per-section implementation status.

[I post weekly updates in Echo's GitHub Discussions](https://github.com/flyingrobots/echo/discussions/255)

## Buckle Up

Start here:

- Start Here: [`docs/guide/start-here.md`](docs/guide/start-here.md)
- Non-programmer on-ramp: [`docs/guide/eli5.md`](docs/guide/eli5.md)
- WARP primer: [`docs/guide/warp-primer.md`](docs/guide/warp-primer.md)
- Docs map: [`docs/meta/docs-index.md`](docs/meta/docs-index.md)
- AIΩN bridge doc: [`docs/aion-papers-bridge.md`](docs/aion-papers-bridge.md)
- Architecture outline: [`docs/architecture-outline.md`](docs/architecture-outline.md)
- Commit hashing spec: [`docs/spec-merkle-commit.md`](docs/spec-merkle-commit.md)

## AIΩN Framework

Echo is part of the **AIΩN Framework**:

- AIΩN repo: <https://github.com/flyingrobots/aion>

Research lineage (AIΩN Foundations series):

- Paper I — *WARP Graphs: A Worldline Algebra for Recursive Provenance* ([doi:10.5281/zenodo.17908005](https://doi.org/10.5281/zenodo.17908005))
- Paper II — *WARP Graphs: Canonical State Evolution and Deterministic Worldlines* ([doi:10.5281/zenodo.17934512](https://doi.org/10.5281/zenodo.17934512))
- Paper III — *WARP Graphs: Computational Holography & Provenance Payloads* ([doi:10.5281/zenodo.17963669](https://doi.org/10.5281/zenodo.17963669))
- Paper IV — *WARP Graphs: Rulial Distance & Observer Geometry* ([doi:10.5281/zenodo.18038297](https://doi.org/10.5281/zenodo.18038297))
- Paper V — *WARP Graphs: Ethics of Deterministic Replay & Provenance Sovereignty* (not yet published)
- Paper VI — *The AIΩN Computer: Architecture & Operating System* (not yet published)

---

## Why?

- **Determinism first:** same inputs → same ordered rewrites → same hashes.
- **Provenance you can trust:** snapshots and commits are content‑addressed.
- **Tooling as a first‑class citizen:** graphs stream over a canonical wire protocol; consumers verify hashes and detect desync early.

If you’re building anything that benefits from “Git‑like” properties for state (replay, branching, inspection, synchronization), Echo is designed for that.

---

## What It Does Right Now

### Core engine + math

- `crates/warp-core` — deterministic rewrite engine spike:
  - `Engine::{begin, apply, commit, snapshot}`
  - deterministic scheduler (radix drain ordering + footprint independence checks)
  - snapshot hashing (`state_root`) + commit hashing (`commit_id`)
  - deterministic math + PRNG (`math::{Vec3, Mat4, Quat, Prng}`)
  - WSC (Write-Streaming Columnar) snapshot format (`wsc::*`) for zero-copy mmap access
  - materialization bus (`MaterializationBus`) for order-independent channel emissions
- `crates/warp-geom` — geometry primitives shared by engine/tools.

### Session + streaming pipeline

- `crates/echo-graph` — canonical renderable graph (`RenderGraph`) + diff ops (`WarpOp`) + deterministic graph hashing.
- `crates/echo-session-proto` — deterministic JS‑ABI v1.0 framing + canonical CBOR + wire schema.
- `crates/echo-session-service` — headless Unix‑socket hub:
  - handshake + monotonic `ts`
  - subscriptions per `WarpId`
  - gapless diff enforcement (snapshot resets; diffs must be consecutive epochs)
- `crates/echo-session-client` — client helpers + tool port abstraction (`tool::SessionPort`).
- `crates/echo-session-ws-gateway` — WebSocket ↔ Unix‑socket bridge for browser‑based tools.

### Tools + adapters

- `crates/warp-viewer` — native WGPU viewer:
  - subscribes to a WARP stream,
  - applies snapshots/diffs,
  - verifies `state_hash` per frame (declares desync on mismatch).
- `crates/echo-app-core` / `crates/echo-config-fs` — “tool hexagon” ports + filesystem config adapter.
- `crates/warp-ffi` / `crates/warp-wasm` — bindings around `warp-core`.
- `crates/warp-benches` — Criterion microbenchmarks (scheduler drain, snapshot hash, etc.).
- `crates/echo-dind-harness` — determinism drill runner (DIND suite; cross‑platform hash verification).
- `crates/echo-dind-tests` — stable test app used by the DIND harness.

### Living specs (teaching slice)

- `specs/spec-000-rewrite` — Leptos + Trunk scaffold for “Spec‑000: Everything is a Rewrite”.
- `crates/echo-wasm-abi` — WASM‑friendly DTO schema for specs.
- `crates/echo-wasm-bindings` — demo kernel + rewrite history (teaching slice; not the production engine).

For a deeper tour, see [`docs/meta/docs-index.md`](docs/meta/docs-index.md).

---

## Quickstart

### Requirements

- Rust toolchain pinned by `rust-toolchain.toml` (currently `1.90.0`).
- Node.js (for docs site). The docs toolchain uses `vitepress@1.6.4`; supported Node versions are pinned via `package.json` (currently `>=18 <25`). For best results, use an LTS (Node 18/20/22).

### Common commands

Install repo hooks:

```bash
make hooks
```

Run the workspace tests:

```bash
cargo test --workspace
```

Run clippy with the repo’s docs gate:

```bash
cargo clippy --all-targets -- -D warnings -D missing_docs
```

Run the docs site (VitePress):

```bash
make docs
```

Directly (useful when debugging):

```bash
pnpm install
pnpm docs:dev
```

CI-style build (includes link checking):

```bash
pnpm docs:build
```

Run the session hub:

```bash
cargo run -p echo-session-service
```

Run the viewer:

```bash
cargo run -p warp-viewer
```

Run Spec‑000 (WASM dev server; requires `trunk` installed):

```bash
make spec-000-dev
```

Run DIND (cross-platform determinism verification):

```bash
cargo xtask dind run
```

---

## Contributions

- Start with `CONTRIBUTING.md`.
- Echo is docs-driven: behavior changes should be reflected in specs and ADRs.
- Determinism is sacred: avoid wall‑clock time, uncontrolled randomness, and unspecified iteration order.

### Determinism guard scripts

Echo enforces determinism guardrails via scripts in `scripts/`:

- `scripts/ban-globals.sh`
- `scripts/ban-nondeterminism.sh`
- `scripts/ban-unordered-abi.sh`

## Workflows

Echo has a few “official workflows” (policy + blessed scripts/entrypoints), documented here:

- [`docs/workflows.md`](docs/workflows.md) — contributor playbook (PR policy, docs guard, `cargo xtask`, scheduled automations)
- [`docs/dependency-dags.md`](docs/dependency-dags.md) — issue + milestone dependency DAGs (DOT/SVG) and how to regenerate them

---

## License

Echo is dual‑licensed. See `LICENSE`, `LICENSE-APACHE`, `LICENSE-MIND-UCAL`, and `LEGAL.md` for details.
