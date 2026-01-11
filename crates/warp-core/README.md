<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# warp-core

Deterministic typed graph rewriting engine used by Echo.

This crate is the Rust core. See the repository root `README.md` for the full project vision and documentation index.

## What this crate does

- Implements the core deterministic engine used by Echo:
  - typed graph storage and snapshotting,
  - rule registration and application,
  - scheduler and drain logic,
  - commit hashing via BLAKE3.
- Provides the foundational APIs that `warp-ffi`, `warp-wasm`, and higher-level
  tools build on.

## Website kernel spike (WARP graphs)

The `warp-core` crate also contains a small “website kernel spike” used by the
`flyingrobots.dev` app:

- `Engine::ingest_inbox_event(seq, payload)` inserts deterministic inbox events under `sim/inbox`
  (event nodes at `sim/inbox/event:{seq:016}`).
- `sys/dispatch_inbox` drains inbox events and (for now) routes `intent:route_push` payload bytes
  directly into `sim/state/routePath` as a `state:route_path` atom.
  - A future refactor will split explicit `cmd/*` rules (e.g. `cmd/route_push`) out of the dispatch
    rule once command scheduling is in place.

## Documentation

- Core engine specs live in `docs/`:
  - `docs/spec-ecs-storage.md`, `docs/spec-scheduler.md`,
    `docs/spec-warp-core.md`, `docs/spec-mwmr-concurrency.md`, and
    related architecture documents.
- The Core booklet (`docs/book/echo/booklet-02-core.tex`) describes the
  high-level architecture, scheduler flow, ECS storage, and game loop that
  this crate implements.
