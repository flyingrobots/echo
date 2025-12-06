<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# rmg-core

Deterministic typed graph rewriting engine used by Echo.

This crate is the Rust core. See the repository root `README.md` for the full project vision and documentation index.

## What this crate does

- Implements the core deterministic engine used by Echo:
  - typed graph storage and snapshotting,
  - rule registration and application,
  - scheduler and drain logic,
  - commit hashing via BLAKE3.
- Provides the foundational APIs that `rmg-ffi`, `rmg-wasm`, and higher-level
  tools build on.

## Documentation

- Core engine specs live in `docs/`:
  - `docs/spec-ecs-storage.md`, `docs/spec-scheduler.md`,
    `docs/spec-rmg-core.md`, `docs/spec-mwmr-concurrency.md`, and
    related architecture documents.
- The Core booklet (`docs/book/echo/booklet-02-core.tex`) describes the
  high-level architecture, scheduler flow, ECS storage, and game loop that
  this crate implements.
