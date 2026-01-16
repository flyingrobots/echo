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

- `Engine::ingest_intent(intent_bytes)` ingests canonical intent envelopes into `sim/inbox`:
  - `intent_id = H(intent_bytes)` is computed immediately.
  - event node IDs are content-addressed by `intent_id` (arrival order is non-semantic).
  - pending vs applied is tracked via `edge:pending` edges; ledger/event nodes are append-only.
- `Engine::ingest_inbox_event(seq, payload)` is a legacy compatibility wrapper:
  - `seq` is ignored for identity (content addressing is by `intent_id`).
  - callers should prefer `ingest_intent(intent_bytes)` for causality-first semantics.
- `sys/dispatch_inbox` drains the inbox by deleting `edge:pending` edges only (queue maintenance).
- `sys/ack_pending` consumes exactly one pending edge for an event scope (used by canonical dispatch).

## Documentation

- Core engine specs live in `docs/`:
  - `docs/spec-ecs-storage.md`, `docs/spec-scheduler.md`,
    `docs/spec-warp-core.md`, `docs/spec-mwmr-concurrency.md`, and
    related architecture documents.
- The Core booklet (`docs/book/echo/booklet-02-core.tex`) describes the
  high-level architecture, scheduler flow, ECS storage, and game loop that
  this crate implements.
