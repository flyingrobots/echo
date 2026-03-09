<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
<!-- (C) James Ross FLYING*ROBOTS <https://github.com/flyingrobots> -->

# ADR-0008: Worldline Runtime Model — Heads, Scheduling, and Domain Boundaries

- **Status:** Accepted
- **Date:** 2026-03-09
- **Canonical Source:** `~/git/james-website/docs/definitive-worldline-runtime-model.md`

If another document disagrees with this one on worldline/head semantics, this
document wins.

## Context

Echo's worldline and provenance primitives have matured through several phases:
parallel execution (Phases 5-6B), provenance storage with atom write tracking
(PR #298), causal cone traversal, and golden-vector digest pinning. The engine
now records _what_ was written, _by whom_, and _why_.

But the runtime model still treats worldlines as a secondary concern — time
travel lives in the debugger (`ttd-browser`), the scheduler runs a single global
step loop, `jump_to_tick` rewrites the entire engine, and writer-head advance is
stubbed in `playback.rs`. These are not bugs; they are the scaffolding of an
engine that grew bottom-up. Now the superstructure needs its blueprint.

Three forces demand a unified model:

1. **Janus** (the debugger) needs seek/fork/step — but these must be core
   runtime operations, not debugger hacks.
2. **Gameplay mechanics** — branch-and-compare puzzles, ghost actors,
   speculative execution — require first-class worldline forking at runtime.
3. **Continuum-style systems** — process-like worldline isolation for future
   multi-tenant or distributed scenarios.

The question is not _whether_ worldlines become the central runtime primitive,
but _how_ to formalize their semantics so every consumer (App, Janus, future
systems) speaks the same language.

## Decision

### 1) Worldlines are core runtime primitives

Worldline lifecycle, fork, seek, and replay are **Echo Core** features.
They are not debugger-only features and must not depend on Janus, browser UI, or
any app-specific framework.

### 2) Writer heads and reader heads

Every worldline may have multiple **playback heads**:

- **WriterHead**: Can advance the worldline frontier by admitting and applying
  intents through deterministic commit. The scheduler owns writer-head
  advancement.
- **ReaderHead**: Can seek and replay from provenance only. Never mutates the
  worldline frontier. Used by debuggers, replay actors, observers.

### 3) SuperTick contract

The scheduler executes one **SuperTick** per cycle:

1. Determine runnable writer heads (not paused, not capability-blocked).
2. Sort by canonical key (`worldline_id`, then `head_id`) for determinism.
3. For each writer head in order: admit intents per policy/budget, execute
   deterministic commit, append provenance, publish projections.
4. Reader heads are unaffected except through explicit frontier updates and
   separate replay calls.

```text
super_tick():
  runnable = writer_heads.filter(is_runnable)
  ordered = sort_canonical(runnable)
  for head in ordered:
    admitted = admit_intents(head, policy)
    if admitted.is_empty():
      continue
    receipt = commit_head_tick(head, admitted)
    provenance.append(head.worldline_id, receipt)
    projections.publish(receipt)
```

### 4) Three domain boundaries

| Domain                 | Owns                                                                           | Must Not                                                                      |
| ---------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| **Echo Core**          | Worldline registry, head lifecycle, scheduling, commit, provenance, projection | Depend on browser UI, Janus UI, or app frameworks                             |
| **App** (website/game) | Schema, intents, UI projection                                                 | Mutate state outside Echo intents; implement independent local timeline truth |
| **Janus** (debugger)   | Session graph, debugger intents, playback workflows                            | Directly mutate App graph; bypass Core timeline APIs                          |

Wesley schema ownership follows these boundaries: `core` schema (Echo-owned),
`app` schema (App-owned), `janus` schema (Janus-owned).

### 5) All mutations flow through intents

- All state mutations come from admitted intents through deterministic commit.
- Intent identity is content-addressed and deduplicated.
- No direct App or Janus mutation path may bypass intent admission.
- Janus submits only Janus/control intents unless explicitly granted additional
  capability.

### 6) Per-head operations replace global rewinds

- `seek(head_id, target_tick)`: Rebuild head-local state from provenance for
  that worldline only. Must not alter other heads or worldlines.
- `jump_to_frontier(head_id)`: Move head to current worldline frontier.
- `fork(worldline_id, fork_tick, new_worldline_id)`: Clone prefix history
  through fork tick. New worldline has independent frontier and head set.
- `set_mode(head_id, mode)`: Controls whether the scheduler may advance that
  writer head.

Global `jump_to_tick` is retained only as an explicit administrative/testing
operation, not the default playback API.

### 7) Provenance is append-only and canonical

- Provenance is the canonical source for worldline replay.
- Replay reads from provenance; it does not execute scheduler logic for reader
  heads.
- Fork creates shared historical prefix with independent future suffix.
- Receipts/patches/hashes are sufficient to verify replay integrity at every
  tick.
- `worldline_tick` is per-worldline append index. `global_tick` (if retained)
  is correlation metadata only and not used as per-worldline append key.

## Required Invariants

### Timeline and Heads

1. Every worldline has monotonically increasing `worldline_tick`.
2. A worldline may have many heads.
3. A writer head may advance only its own worldline.
4. Reader heads never mutate worldline frontier.
5. Paused heads never advance.
6. Seek/jump is head-local and never globally rewinds unrelated worldlines.

### Determinism and Scheduling

1. SuperTick order over runnable writer heads is canonical and deterministic.
2. Commit order is deterministic for equivalent input/state.
3. Equal inputs produce equal receipts and hashes.
4. Scheduler never relies on host wall-clock timing for ordering.

### Clocks

1. `worldline_tick` is per-worldline append index.
2. `global_tick` is correlation metadata; APIs must not assume equal tick counts
   across worldlines.

## Implementation Plan (Normative Order)

| Step | Change                                                       | Current State                                    |
| ---- | ------------------------------------------------------------ | ------------------------------------------------ |
| 1    | First-class `WriterHead` object + `PlaybackHeadRegistry`     | `playback.rs` writer advance is stubbed          |
| 2    | `SchedulerCoordinator` over runnable writer heads            | `warp_kernel.rs` single global step loop         |
| 3    | Per-writer-head `IntentInbox` policy                         | `dispatch_next_intent(tx)` monolithic dequeue    |
| 4    | Wire writer-head commit to provenance in production          | PR #298 laid atom write + causal cone groundwork |
| 5    | Per-head `seek`/`jump` APIs; deprecate global `jump_to_tick` | `engine_impl.rs` global rewind                   |
| 6    | Split `worldline_tick` / `global_tick` semantics             | Currently entangled in runtime + provenance APIs |
| 7    | Multi-warp replay support policy                             | `worldline.rs` cannot replay portal/instance ops |
| 8    | Wesley core schema + generated clients for new APIs          | Depends on all above                             |

## Key Files (Observed State as of 2026-03-09)

- `crates/warp-wasm/src/warp_kernel.rs` — kernel step loop, intent dispatch
- `crates/warp-core/src/engine_impl.rs` — global commit, `jump_to_tick`
- `crates/warp-core/src/playback.rs` — cursor, stubbed writer advance
- `crates/warp-core/src/provenance_store.rs` — worldline provenance, atom
  writes, causal cone walk (PR #298)
- `crates/warp-core/src/worldline.rs` — multi-warp replay limitation
- `crates/ttd-browser/src/lib.rs` — Janus/TTD browser wrapper

## Gameplay and Non-Debug Use Cases

The runtime model natively supports:

- **Replay-actor mechanics**: Recorded past behavior injected into present
  timeline.
- **Branch-and-compare puzzle solving**: Fork, diverge, compare outcomes.
- **Speculative execution branches**: Try multiple futures, collapse to one.
- **Process-style worldline isolation**: Independent timelines for Continuum
  runtime experiments.

These are runtime capabilities, not debugger hacks.

## Test Requirements

| Category      | What to verify                                                                   |
| ------------- | -------------------------------------------------------------------------------- |
| Determinism   | Same inputs + same initial state => same receipts/hashes                         |
| Isolation     | Seeking worldline A does not mutate worldline B                                  |
| Scheduling    | Paused writer heads never advance; runnable heads advance in canonical order     |
| Provenance    | Append-only invariants hold; replay at tick _t_ reproduces expected hash triplet |
| Authorization | Janus intents cannot mutate App graph directly                                   |
| Integration   | Input routing emits intents only; UI deterministic under time-control ops        |

## Consequences

- Worldlines graduate from "internal plumbing" to the central organizing
  principle of the runtime.
- Janus becomes simpler — it's just a client with debugger-focused intents,
  not a privileged engine mutator.
- Gameplay time mechanics (fork, branch, ghost) become trivially expressible
  as worldline operations, no special-casing required.
- The scheduler refactor (Steps 1-3) is the critical path — it touches the
  kernel step loop, intent dispatch, and commit pipeline simultaneously.
- Multi-warp replay (Step 7) is the known hard problem. Portal/instance ops
  may require a bounded replay engine or explicit "no-replay" slicing.
- The 8-step plan is ordered by dependency; each step is independently
  shippable and testable.

## Non-Goals

- This ADR does not prescribe UI layout or visual design.
- This ADR does not lock a specific serialization codec.
- This ADR does not require immediate removal of all legacy APIs in one
  migration.

## Document Governance

- Any change to the invariants above requires a dedicated design amendment PR.
- PRs touching worldline/head semantics must reference this ADR.
- Workarounds that violate this model require a documented exception with owner
  and expiry date.

---

_Stellae vertuntur dum via sculpitur._
