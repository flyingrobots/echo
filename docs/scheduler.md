<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Scheduling in Echo (Doc Map)

Echo currently has **two different “scheduler” concepts** that often get conflated in docs:

1) **WARP rewrite scheduler (implemented today)** — lives in Rust `warp-core` and is responsible for:
   - deterministic draining/ordering of pending rewrites, and
   - enforcing *footprint independence* via `reserve()` (so rewrites commute and ticks are deterministic).

2) **Echo system scheduler (future)** — the ECS-style “systems + phases + dependency DAG” scheduler planned for `@echo/core`.
   - It will coordinate systems across phases and integrate timeline/Codex concepts.
   - It is currently a **spec**, not a shipped runtime.

This page is a landing map: “which scheduler doc should I read?”

---

## Quick Map

| If you’re trying to… | Read this first | Then |
| --- | --- | --- |
| Understand determinism + rewrite scheduling in Rust | `docs/spec-warp-core.md` | `docs/scheduler-warp-core.md` |
| Validate `reserve()` correctness / determinism properties | `docs/scheduler-reserve-validation.md` | `crates/warp-core/src/scheduler.rs` tests + `crates/warp-core/tests/*` |
| Benchmark rewrite scheduler throughput | `docs/scheduler-performance-warp-core.md` | `crates/warp-benches/benches/scheduler_drain.rs` |
| Understand the planned ECS/system scheduler | `docs/spec-scheduler.md` | `docs/spec-concurrency-and-authoring.md`, `docs/spec-codex-baby.md` |

---

## WARP Rewrite Scheduler (warp-core)

**Where it lives:** `crates/warp-core/src/scheduler.rs`

Key operations (high-level):
- `reserve()` enforces footprint independence (no conflicting read/write sets).
- `drain_for_tx()` deterministically drains reserved rewrites.

Related docs:
- Spec context: `docs/spec-warp-core.md`
- Canonical warp-core scheduler doc: `docs/scheduler-warp-core.md`
- Bench plan + current benches: `docs/scheduler-performance-warp-core.md`

---

## Echo System Scheduler (planned)

**Status:** design sketch (Phase 0) — not implemented as a runtime in this repo today.

**Where it’s described:** `docs/spec-scheduler.md`

This is the “systems + phases + DAG ordering” scheduler that will eventually sit above the engine
and coordinate systems (and future ECS/timeline concepts).

Related docs:
- Authoring/concurrency model: `docs/spec-concurrency-and-authoring.md`
- Codex’s Baby integration concepts: `docs/spec-codex-baby.md`

---

## Naming Guidance (to reduce confusion)

When writing docs/issues:
- Say **“warp-core rewrite scheduler”** when you mean `reserve()/drain` determinism in Rust.
- Say **“Echo system scheduler”** when you mean the ECS/system DAG across phases.
