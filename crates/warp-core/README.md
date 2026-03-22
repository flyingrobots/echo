<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
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
- Provides the foundational APIs that `warp-wasm` and higher-level tools build
  on.

## Website kernel spike (WARP graphs)

The `warp-core` crate also contains a small “website kernel spike” used by the
`flyingrobots.dev` app:

- `WorldlineRuntime::ingest(IngressEnvelope)` is the live ingress surface:
    - envelopes resolve deterministically to a writer head by `DefaultWriter`,
      `InboxAddress`, or `ExactHead`,
    - per-head inboxes dedupe by content-addressed `ingress_id`,
    - committed duplicates are tracked per resolved writer head.
- `SchedulerCoordinator::super_tick(...)` is the internal logical cycle:
    - runnable writer heads advance in canonical `(worldline_id, head_id)` order,
    - commits run against the shared `WorldlineState` frontier for that worldline,
    - empty inboxes do not advance frontier ticks,
    - it is runtime internals, not a public WASM control export.
- `ObservationService::observe(...)` is the canonical read path:
    - every read names an explicit worldline, coordinate, frame, and projection,
    - commit-boundary reads and recorded-truth reads share one deterministic
      artifact model,
    - observation is read-only and does not mutate runtime, provenance, inboxes,
      or compatibility mirrors.
- `WorldlineTick` and `GlobalTick` are explicit logical coordinates:
    - `WorldlineTick` is the monotone per-worldline append-order coordinate used
      to identify committed positions within one worldline,
    - `GlobalTick` is scheduler-cycle correlation metadata used to relate work
      across worldlines without implying wall-clock time or append order,
    - neither carries wall-clock semantics.
- Runtime control is intent-shaped:
    - domain writes and privileged scheduler/control requests both enter through
      canonical EINT intents,
    - scheduler lifecycle requests route through control intents and do not
      directly invoke `SchedulerCoordinator::super_tick(...)`.
- The runtime/kernel production path no longer uses `sim/inbox`,
  `edge:pending`, or `Engine::dispatch_next_intent(...)`.
- Phase 6 / ABI v3 removed legacy public read adapters, removed public
  `step(...)`, and exposed `observe(...)` plus read-only scheduler metadata as
  the canonical WASM boundary.
- `Engine::ingest_intent(intent_bytes)` and `Engine::ingest_inbox_event(seq, payload)`
  remain legacy compatibility helpers for isolated tests and older spike call sites.

## Documentation

- Core engine specs live in `docs/`:
    - `docs/spec-ecs-storage.md`, `docs/spec-scheduler.md`,
      `docs/spec-warp-core.md`, `docs/spec-mwmr-concurrency.md`, and
      related architecture documents.
- The Core booklet (`docs/book/echo/booklet-02-core.tex`) describes the
  high-level architecture, scheduler flow, ECS storage, and game loop that
  this crate implements.
