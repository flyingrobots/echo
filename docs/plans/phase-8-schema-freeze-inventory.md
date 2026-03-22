<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Phase 8 Prep Inventory: ADR-0008 Runtime Schema Freeze

- **Status:** Prep inventory for `feat/adr-0008-0009-phase-8`
- **Date:** 2026-03-22
- **Primary Plan:** [Implementation Plan: ADR-0008 and ADR-0009](adr-0008-and-0009.md)

## Purpose

Phase 8 is not "generate whatever exists today." It is the point where Echo
pins the stable ADR-0008 runtime surface and only then teaches Wesley/codegen
to reproduce it.

This inventory records:

- the runtime types Phase 8 should freeze,
- where their canonical definitions live today,
- where host/runtime mirrors still exist,
- and the gaps that must be closed before generated types can replace
  hand-written copies honestly.

## Executive Findings

1. The current Wesley-generated crates are **TTD-specific**, not ADR-0008
   runtime-schema crates.
    - `crates/ttd-manifest/src/lib.rs`
    - `crates/ttd-protocol-rs/lib.rs`
2. The repo documentation says `cargo xtask wesley sync` manages those vendored
   artifacts, but `xtask/src/main.rs` does **not** implement a `wesley`
   subcommand yet.
3. The stable ADR-0008 runtime surface still lives in hand-written Rust types in
   `warp-core`, with host-facing adapter DTOs in `echo-wasm-abi`.
4. The living plan's Phase 8 freeze list contains one stale name:
   `SuperTickResult` does not exist as a stable public type in the current
   implementation. The control-plane surface now exposes
   `SchedulerStatus` plus `RunCompletion`.

## Freeze Set Inventory

| Candidate         | Canonical definition today           | Mirror / adapter surface                                                                                       | Phase 8 note                                                                                                                                                      |
| ----------------- | ------------------------------------ | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `HeadId`          | `crates/warp-core/src/head.rs`       | ABI head-key wrappers in `crates/echo-wasm-abi/src/kernel_port.rs`                                             | Opaque hash-backed id; schema must preserve byte-level opacity, not invent string semantics                                                                       |
| `WriterHeadKey`   | `crates/warp-core/src/head.rs`       | ABI head-key wrappers in `crates/echo-wasm-abi/src/kernel_port.rs`                                             | Stable composite runtime key; good freeze candidate                                                                                                               |
| `PlaybackMode`    | `crates/warp-core/src/playback.rs`   | TTD-generated `PlaybackMode` in `crates/ttd-protocol-rs/lib.rs` is related but not the runtime source of truth | Freeze the runtime enum first; do not treat the TTD schema as authoritative for ADR-0008                                                                          |
| `WorldlineTick`   | `crates/warp-core/src/clock.rs`      | ABI wrapper in `crates/echo-wasm-abi/src/kernel_port.rs`                                                       | Stable newtype candidate; schema must preserve logical-counter semantics                                                                                          |
| `GlobalTick`      | `crates/warp-core/src/clock.rs`      | ABI wrapper in `crates/echo-wasm-abi/src/kernel_port.rs`                                                       | Stable newtype candidate; schema/docs must keep correlation-not-time semantics explicit                                                                           |
| `IntentKind`      | `crates/warp-core/src/head_inbox.rs` | No Wesley/runtime-generated equivalent today                                                                   | Stable opaque hash-backed id; schema must not collapse it to an arbitrary string label                                                                            |
| `InboxPolicy`     | `crates/warp-core/src/head_inbox.rs` | No Wesley/runtime-generated equivalent today                                                                   | Good freeze candidate once variants are confirmed complete for ADR-0008                                                                                           |
| `IngressTarget`   | `crates/warp-core/src/head_inbox.rs` | ABI/control-intent routing mirrors in `crates/echo-wasm-abi/src/kernel_port.rs`                                | Good freeze candidate; schema must preserve `DefaultWriter` / `InboxAddress` / `ExactHead` split                                                                  |
| `SuperTickResult` | n/a in current code                  | n/a                                                                                                            | Living-plan name is stale; Phase 8 must decide whether to freeze `SchedulerStatus`, `RunCompletion`, or another explicitly named scheduler result surface instead |

## Current Boundary Shape

### What Wesley/codegen actually covers today

- `crates/ttd-manifest/src/lib.rs` vendors TTD IR/schema artifacts.
- `crates/ttd-protocol-rs/lib.rs` is generated from the TTD schema and serves
  browser/controller protocol needs.

That is useful, but it is **not** the Phase 8 runtime freeze target.

### What remains hand-written today

- Core runtime types in:
    - `crates/warp-core/src/head.rs`
    - `crates/warp-core/src/head_inbox.rs`
    - `crates/warp-core/src/playback.rs`
    - `crates/warp-core/src/clock.rs`
- Host-facing DTO/adapters in:
    - `crates/echo-wasm-abi/src/kernel_port.rs`

The `echo-wasm-abi` types should be treated as **adapter DTOs**, not proof that
the underlying ADR-0008 runtime schema is already frozen.

## Phase 8 Gaps To Close

### 1. Missing runtime Wesley schema

There is no dedicated ADR-0008 runtime schema artifact in-repo today. Phase 8
needs an explicit source of truth for the runtime freeze set before generated
Rust can replace hand-written definitions.

### 2. Missing generation/sync entrypoint

The repo advertises `cargo xtask wesley sync`, but `xtask/src/main.rs` does not
implement it yet. Phase 8 needs a real, deterministic regeneration path before
generated artifacts can be trusted.

### 3. Stale plan naming around scheduler results

`SuperTickResult` survived in the plan, but the implemented ABI v3 control plane
is centered on `SchedulerStatus`, `SchedulerState`, `WorkState`, and
`RunCompletion`. Phase 8 needs to freeze the **actual** stable scheduler result
surface rather than back-porting a stale name for cosmetic consistency.

### 4. Opaque id / logical counter mapping rules are not yet written down as schema rules

`HeadId`, `IntentKind`, `WorldlineTick`, and `GlobalTick` all have stricter
semantics than "some bytes" or "some integer." Phase 8 needs schema-side rules
for:

- opaque fixed-size hash ids,
- logical counters that are coordinates rather than time,
- and wrapper-vs-generated ownership boundaries.

## Recommended Phase 8 Slice Order

### Slice A: Freeze inventory and naming reconciliation

- ratify the freeze set
- replace stale `SuperTickResult` wording in the living plan with the actual
  stable scheduler result surface
- record which surfaces remain adapter-only

### Slice B: Runtime schema source of truth

- add the ADR-0008 runtime schema artifact(s) for the freeze set only
- keep ADR-0009 transport/conflict types out of scope

### Slice C: Deterministic generation plumbing

- implement `cargo xtask wesley sync` for the runtime schema path
- ensure generated output is deterministic and reviewable

### Slice D: Hand-written type reduction

- replace hand-written mirrors with generated types or thin wrappers where needed
- keep `echo-wasm-abi` as an adapter layer when ABI semantics genuinely differ

## Out of Scope For This Phase

- ADR-0009 transport/conflict schema freeze
- TTD browser/controller protocol redesign
- new ABI v4 work unrelated to the runtime freeze set
- footprint/conflict-policy work from Phase 9

## Exit Signal For The Inventory Slice

This prep slice is complete when:

- the freeze candidates and their owners are written down,
- the stale `SuperTickResult` naming drift is called out explicitly,
- and the next implementation slice can start from a concrete schema authoring
  target instead of inference from Rust code.
