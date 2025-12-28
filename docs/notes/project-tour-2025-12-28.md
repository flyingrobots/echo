<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Project Tour (2025-12-28)

This note is a fast “become dangerous” map of the repository as it exists today.
It’s written for future-Codex and humans who want to orient quickly without
re-reading every spec end-to-end.

## TL;DR

Echo is a deterministic simulation engine built around **typed graph rewriting**.
The core invariant is: **same inputs → same ordered rewrites → same snapshot hashes**.

Today’s repo is a Rust workspace that already contains:

- a deterministic rewrite engine spike (`warp-core`) with snapshot hashing,
- a deterministic wire protocol + session hub + viewer toolchain for streaming graphs,
- a “living spec” scaffold (Spec-000) and a demo WASM kernel API (teaching slice).

## Mental Model: “Git, but for Reality”

The stable story that matches both docs and code:

- The *state* of the world is a graph (nodes + edges + payloads).
- A *change* is a rewrite (rule applied at a scope).
- A *frame / tick* is a transaction:
  - `begin()` → collect candidate rewrites
  - `apply(...)` → match + enqueue rewrites
  - `commit()` → deterministically order + execute an independent subset → emit a snapshot hash
- Snapshots can be streamed to tools as full snapshots + gapless diffs (epoch-to-epoch).
- Hashes are the checksum of truth: if peers disagree, you detect desync early.

## What’s Implemented vs Aspirational

Implemented (today):

- `warp-core` rewrite engine spike:
  - deterministic pending queue and deterministic drain ordering,
  - footprint-based independence checks,
  - reachable-only graph hashing (`state_root`) and commit header hashing (`commit_id`),
  - deterministic math primitives + PRNG.
- Session/tooling pipeline:
  - deterministic JS-ABI v1.0 framing + canonical CBOR encoding (`echo-session-proto`),
  - Unix socket hub (`echo-session-service`),
  - tool client + port abstraction (`echo-session-client`),
  - WGPU viewer that reconstructs and validates streamed graphs (`warp-viewer`).
- Living spec scaffolding:
  - Spec-000 Leptos/Trunk shell (`specs/spec-000-rewrite`),
  - DTO schema (`echo-wasm-abi`) + demo kernel (`echo-wasm-bindings`).

Aspirational / partially specified (not fully implemented yet):

- Full DPO/DPOi typed rewriting (beyond the spike rules).
- True MWMR parallel commit, optimized bitmaps, and high-performance store layouts.
- Branch trees (Chronos/Kairos/Aion) as first-class runtime structures.
- A system scheduler (phases + dependencies) layered above the rewrite substrate.

## Crate Map (How the Pieces Fit)

### Core engine + math

- `crates/warp-core`
  - Engine transaction model: `Engine::begin`, `Engine::apply`, `Engine::commit`, `Engine::snapshot`
  - Deterministic scheduler: radix drain ordering + footprint independence checks
  - Snapshot hashing: `state_root` and `commit_id`
  - Deterministic math: `math::{Vec3, Mat4, Quat, Prng}`
- `crates/warp-geom`
  - Geometry primitives (AABB, transforms, temporal helpers).

### Tooling ports

- `crates/echo-app-core`
  - “tool hexagon” ports/services: config, toasts, redraw port, etc.
- `crates/echo-config-fs`
  - Filesystem config adapter for tool prefs (implements the `ConfigStore` port).

### Session and streaming graph

- `crates/echo-graph`
  - Canonical renderable graph (`RenderGraph`) + diff ops (`WarpOp`)
  - Canonical hashing via deterministic CBOR bytes (node/edge sorting before encoding)
- `crates/echo-session-proto`
  - Wire types (`Message`, `OpEnvelope`, notifications, WARP stream payload)
  - Deterministic CBOR canonicalization + JS-ABI v1.0 framing + BLAKE3 checksum
- `crates/echo-session-service`
  - Hub process: handshake, monotonic `ts`, subscriptions, gapless diff enforcement, fan-out
- `crates/echo-session-client`
  - Client helpers + `tool::SessionPort` abstraction for UIs
- `crates/echo-session-ws-gateway`
  - WebSocket ↔ Unix-socket bridge for browser-based consumers.

### Tools / adapters

- `crates/warp-viewer`
  - Native viewer: subscribes to an WARP stream, applies snapshots/diffs, verifies hashes, renders.
- `crates/warp-ffi`
  - Thin C ABI surface over `warp-core` (currently focused on the motion demo rule).
- `crates/warp-wasm`
  - wasm-bindgen bindings for `warp-core` (tooling/web environments).
- `crates/warp-cli`
  - Placeholder CLI home.
- `crates/warp-benches`
  - Criterion microbenchmarks (scheduler drain, snapshot hash, etc.).

### Living specs (teaching slice)

- `crates/echo-wasm-abi`
  - WASM-friendly DTO schema for Spec-000 and future living specs.
- `crates/echo-wasm-bindings`
  - Demo kernel + rewrite history (teaching slice; not the production engine).
- `specs/spec-000-rewrite`
  - Leptos/Trunk scaffold; currently not yet wired to the demo kernel bindings.

## Core Determinism Invariants (Code-Backed)

### Rewrite ordering (warp-core scheduler)

- Deterministic sort key:
  - (`scope_hash`, `rule_id`, `nonce`) in ascending lexicographic order.
- Implementation detail:
  - stable LSD radix sort (16-bit digits; 20 passes) for `O(n)` drain,
  - tiny batches use a comparison sort fast-path.
- Pending queue semantics:
  - last-wins de-dupe on (`scope_hash`, `compact_rule_id`) within a tx queue.

### Independence (MWMR groundwork)

- Each pending rewrite computes a `Footprint`:
  - node read/write sets, edge read/write sets, boundary port sets, plus a coarse `factor_mask`.
- Independence fails if any of the following intersect:
  - writes vs prior reads/writes, on nodes and edges
  - any overlap on boundary ports
  - `factor_mask` overlap (used as a coarse “might-touch” prefilter)

### Snapshot hashing (warp-core)

- `state_root` is BLAKE3 over a canonical byte stream of the reachable subgraph:
  - reachability: deterministic BFS from root following outbound edges
  - node order: ascending `NodeId` (32-byte lexicographic)
  - edge order: per source node, edges sorted by `EdgeId`, include only edges to reachable nodes
  - payloads: `u64` little-endian length prefix + raw bytes

### Commit hashing (warp-core)

- `commit_id` is BLAKE3 over a commit header:
  - header version `u16 = 1`
  - parent commit hashes (length-prefixed)
  - `state_root` + plan/decision/rewrites digests + policy id
- Empty digests for *length-prefixed list digests* use `blake3(0u64.to_le_bytes())`.

### Wire protocol (echo-session-proto)

- JS-ABI v1.0 packet:
  - `MAGIC(4) || VERSION(2) || FLAGS(2) || LENGTH(4) || PAYLOAD || CHECKSUM(32)`
  - checksum = blake3(header||payload)
- PAYLOAD is canonical CBOR:
  - definite lengths only, no tags, minimal integer widths
  - floats encoded at the smallest width that round-trips
  - forbid “int as float” encodings
  - map keys sorted by their CBOR byte encoding; duplicates rejected

## “Follow the Code” Entry Points

- Engine core:
  - `crates/warp-core/src/engine_impl.rs` (begin/apply/commit)
  - `crates/warp-core/src/scheduler.rs` (deterministic ordering + independence)
  - `crates/warp-core/src/snapshot.rs` (state_root + commit_id hashing)
- Wire protocol:
  - `crates/echo-session-proto/src/wire.rs` (packet framing + encode/decode)
  - `crates/echo-session-proto/src/canonical.rs` (canonical CBOR)
- Hub + viewer:
  - `crates/echo-session-service/src/main.rs` (hub state machine + enforcement)
  - `crates/warp-viewer/src/session_logic.rs` (apply frames + hash checks)

## Commands (Common Workflows)

- Core validation: `cargo test --workspace`
- Docs gate: `cargo clippy --all-targets -- -D warnings -D missing_docs`
- Docs site: `make docs` (VitePress)
- Benches: `make bench-report`
- Spec-000 (WASM): `make spec-000-dev`

## Known “Docs vs Code” Drift to Watch

- Some older specs are TypeScript-first and describe the planned system scheduler;
  today’s implemented deterministic scheduler is the rewrite scheduler in `warp-core`.
- `docs/spec-merkle-commit.md` historically claimed empty list digests used `blake3(b"")`;
  the engine uses `blake3(0u64.to_le_bytes())` for length-prefixed list digests.
  Keep this consistent, since it affects hash identity.

