<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

<p align="center">
  <img alt="Echo" src="https://github.com/user-attachments/assets/3d147042-5139-4289-8c22-675899ce68ab" />
</p>

![Echo (6)](https://github.com/user-attachments/assets/2bb4d1cc-3f98-491c-a050-01e72bccffbc)

Echo is a **deterministic graph‑rewrite engine + tooling** for building simulations you can replay, verify, and synchronize without guessing.

Instead of treating a game/simulation as a pile of mutable objects, Echo treats **state as a typed graph**. Each “tick” proposes a set of rewrites, executes them in a deterministic order, and emits **cryptographic hashes** of the resulting state and provenance so tools/peers can validate and converge.

![Project Status](https://github.com/user-attachments/assets/eb0f1e94-71e8-45b0-95de-23030af95d12)

**Status:** active R&D. The deterministic core (`warp-core`) and a session/tooling pipeline are implemented; higher‑level “world / ECS / systems” layers and the full timeline tree mechanics are specced and landing incrementally.

![Buckle Up](https://github.com/user-attachments/assets/da9a9056-76b8-4867-ae60-d4d9212c232a)

Start here:

- WARP primer: [`docs/guide/warp-primer.md`](docs/guide/warp-primer.md)
- Project tour: [`docs/notes/project-tour-2025-12-28.md`](docs/notes/project-tour-2025-12-28.md)
- AIΩN bridge doc: [`docs/aion-papers-bridge.md`](docs/aion-papers-bridge.md)
- Architecture outline: [`docs/architecture-outline.md`](docs/architecture-outline.md)
- Commit hashing spec: [`docs/spec-merkle-commit.md`](docs/spec-merkle-commit.md)

![AIΩN Framework](https://github.com/user-attachments/assets/45ed62bb-f436-4918-bd6c-d5efd3d5e671)

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

![Why Echo_](https://github.com/user-attachments/assets/8e215c34-c2ce-46ac-af62-1a6974aa95ee)

- **Determinism first:** same inputs → same ordered rewrites → same hashes.
- **Provenance you can trust:** snapshots and commits are content‑addressed.
- **Tooling as a first‑class citizen:** graphs stream over a canonical wire protocol; consumers verify hashes and detect desync early.

If you’re building anything that benefits from “Git‑like” properties for state (replay, branching, inspection, synchronization), Echo is designed for that.

---

![What You Get](https://github.com/user-attachments/assets/00092ca4-33f2-48d9-b812-62a4935b58bf)

### Core engine + math

- `crates/warp-core` — deterministic rewrite engine spike:
  - `Engine::{begin, apply, commit, snapshot}`
  - deterministic scheduler (radix drain ordering + footprint independence checks)
  - snapshot hashing (`state_root`) + commit hashing (`commit_id`)
  - deterministic math + PRNG (`math::{Vec3, Mat4, Quat, Prng}`)
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

### Living specs (teaching slice)

- `specs/spec-000-rewrite` — Leptos + Trunk scaffold for “Spec‑000: Everything is a Rewrite”.
- `crates/echo-wasm-abi` — WASM‑friendly DTO schema for specs.
- `crates/echo-wasm-bindings` — demo kernel + rewrite history (teaching slice; not the production engine).

For a deeper “tour” oriented around invariants and entry points, see
[`docs/notes/project-tour-2025-12-28.md`](docs/notes/project-tour-2025-12-28.md).

---

![Quickstart](https://github.com/user-attachments/assets/d059c212-27ab-48ab-acbd-d4758e6a6d74)

### Requirements

- Rust toolchain pinned by `rust-toolchain.toml` (currently `1.90.0`).
- Node.js (for docs site).

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

---

## Determinism Contracts (Stuff You Must Not Break)

Echo’s determinism story depends on a small number of “hard rules”. If you change any of these, you are changing identity.

- **Rewrite execution order (core scheduler):**
  - ordering key is lexicographic ascending: (`scope_hash`, `rule_id`, `nonce`)
  - pending queue is drained deterministically (stable radix sort for large batches).
- **Graph state hashing (`state_root`):**
  - reachable‑only traversal from the root,
  - nodes hashed in ascending id order,
  - outbound edges sorted by edge id,
  - payloads are length‑prefixed (u64 LE) then raw bytes.
- **Commit hashing (`commit_id`):**
  - versioned header including parents, `state_root`, and digests for plan/decisions/rewrites.
  - empty list digests are computed as `blake3(0u64.to_le_bytes())` (length‑prefixed canonical empty).
- **Wire protocol (session + tools):**
  - canonical CBOR encoding with strict validation,
  - JS‑ABI packet checksum is `blake3(header || payload)`.

Specs live under `docs/` (start with `docs/architecture-outline.md` and `docs/spec-merkle-commit.md`).

---

![Contributions](https://github.com/user-attachments/assets/8dbd9e6f-a39c-4ba1-b072-738d722d56c0)

- Start with `CONTRIBUTING.md` and `docs/execution-plan.md`.
- Echo is docs‑driven: behavior changes should be reflected in specs and logged in `docs/decision-log.md`.
- Determinism is sacred: avoid wall‑clock time, uncontrolled randomness, and unspecified iteration order.

---

![License](https://github.com/user-attachments/assets/50a01d02-53d7-48f2-865b-4791548438c6)

Echo is dual‑licensed. See `LICENSE`, `LICENSE-APACHE`, `LICENSE-MIND-UCAL`, and `LEGAL.md` for details.
