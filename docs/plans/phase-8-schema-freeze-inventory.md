<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Phase 8 Prep Inventory: ADR-0008 Runtime Schema Freeze

- **Status:** Freeze inventory locked for `feat/adr-0008-0009-phase-8`
- **Date:** 2026-03-22
- **Primary Plan:** [Implementation Plan: ADR-0008 and ADR-0009](adr-0008-and-0009.md)
- **Conformance Audit:** [Phase 8 Runtime Schema Conformance Audit](phase-8-runtime-schema-conformance.md)
- **Mapping Contract:** [Phase 8 Runtime Schema Mapping Contract](phase-8-runtime-schema-mapping-contract.md)

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
3. The shared Phase 8 owner crate now exists as
   `crates/echo-runtime-schema`, and it already owns the frozen logical
   counters plus the core `HeadId`/`WorldlineId`/`WriterHeadKey` types. The
   rest of the freeze set intentionally stays hand-written in `warp-core`,
   with host-facing adapter DTOs in `echo-wasm-abi`.
4. The living plan's old `SuperTickResult` shorthand should be retired.
   The actual stable scheduler result surface is:
   `SchedulerStatus`, `SchedulerState`, `WorkState`, `RunCompletion`,
   `HeadEligibility`, and `HeadDisposition`.
5. Freezing those runtime surfaces also requires a few supporting types that the
   earlier plan text underspecified: `WorldlineId`, `InboxAddress`, `SeekThen`,
   `SchedulerMode`, and `RunId`.

## Freeze Set Inventory

| Candidate         | Canonical definition today                | Mirror / adapter surface                                                                                       | Phase 8 note                                                                                                               |
| ----------------- | ----------------------------------------- | -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `HeadId`          | `crates/echo-runtime-schema/src/lib.rs`   | ABI head-key wrappers in `crates/echo-wasm-abi/src/kernel_port.rs`                                             | Opaque hash-backed id; runtime owner is now shared, ABI wrapper remains byte-oriented                                      |
| `WorldlineId`     | `crates/echo-runtime-schema/src/lib.rs`   | ABI worldline-id wrappers in `crates/echo-wasm-abi/src/kernel_port.rs`                                         | Supporting opaque id needed by `WriterHeadKey`, `IngressTarget`, and observation/control wrappers                          |
| `WriterHeadKey`   | `crates/echo-runtime-schema/src/lib.rs`   | ABI head-key wrappers in `crates/echo-wasm-abi/src/kernel_port.rs`                                             | Stable composite runtime key; runtime owner is now shared                                                                  |
| `PlaybackMode`    | `crates/warp-core/src/playback.rs`        | TTD-generated `PlaybackMode` in `crates/ttd-protocol-rs/lib.rs` is related but not the runtime source of truth | Freeze the runtime enum first; do not treat the TTD schema as authoritative for ADR-0008                                   |
| `SeekThen`        | `crates/warp-core/src/playback.rs`        | No Wesley/runtime-generated equivalent today                                                                   | Supporting playback-control enum required to express `PlaybackMode::Seek` honestly                                         |
| `WorldlineTick`   | `crates/echo-runtime-schema/src/lib.rs`   | re-exported by `warp-core` and `echo-wasm-abi`                                                                 | Shared logical-counter owner now exists; schema must preserve logical-counter semantics                                    |
| `GlobalTick`      | `crates/echo-runtime-schema/src/lib.rs`   | re-exported by `warp-core` and `echo-wasm-abi`                                                                 | Shared logical-counter owner now exists; schema/docs must keep correlation-not-time semantics explicit                     |
| `IntentKind`      | `crates/warp-core/src/head_inbox.rs`      | No Wesley/runtime-generated equivalent today                                                                   | Stable opaque hash-backed id; intentionally runtime-owned until a real generated consumer exists                           |
| `InboxAddress`    | `crates/warp-core/src/head_inbox.rs`      | ABI/control routing byte/string mirrors in `crates/echo-wasm-abi/src/kernel_port.rs`                           | Supporting routing alias type needed to freeze `IngressTarget` honestly; intentionally runtime-owned for Phase 8           |
| `InboxPolicy`     | `crates/warp-core/src/head_inbox.rs`      | No Wesley/runtime-generated equivalent today                                                                   | Good freeze candidate once variants are confirmed complete for ADR-0008                                                    |
| `IngressTarget`   | `crates/warp-core/src/head_inbox.rs`      | ABI/control-intent routing mirrors in `crates/echo-wasm-abi/src/kernel_port.rs`                                | Good freeze candidate; schema must preserve `DefaultWriter` / `InboxAddress` / `ExactHead` split                           |
| `SchedulerMode`   | `crates/echo-wasm-abi/src/kernel_port.rs` | `ControlIntentV1::Start` mapping in `crates/warp-wasm/src/warp_kernel.rs`                                      | Supporting scheduler-control type; `SchedulerStatus.active_mode` cannot be frozen honestly without it                      |
| `SchedulerStatus` | `crates/echo-wasm-abi/src/kernel_port.rs` | Engine/runtime mapping in `crates/warp-wasm/src/warp_kernel.rs`                                                | This is the real public scheduler result object; Phase 8 should freeze it explicitly instead of reviving `SuperTickResult` |
| `SchedulerState`  | `crates/echo-wasm-abi/src/kernel_port.rs` | n/a                                                                                                            | Stable scheduler lifecycle enum                                                                                            |
| `WorkState`       | `crates/echo-wasm-abi/src/kernel_port.rs` | n/a                                                                                                            | Stable scheduler boundary/work-availability enum                                                                           |
| `RunCompletion`   | `crates/echo-wasm-abi/src/kernel_port.rs` | n/a                                                                                                            | Stable bounded-run completion enum                                                                                         |
| `RunId`           | `crates/echo-runtime-schema/src/lib.rs`   | re-exported by `echo-wasm-abi`; runtime/ABI mapping in `crates/warp-wasm/src/warp_kernel.rs`                   | Supporting control-plane token needed by `SchedulerStatus`; shared owner now exists                                        |
| `HeadEligibility` | `crates/warp-core/src/head.rs`            | ABI wrapper in `crates/echo-wasm-abi/src/kernel_port.rs`                                                       | Runtime/ABI pair must stay structurally aligned                                                                            |
| `HeadDisposition` | `crates/echo-wasm-abi/src/kernel_port.rs` | runtime truth derived in `crates/warp-wasm/src/warp_kernel.rs`                                                 | ABI-facing scheduler truth surface; freeze alongside `SchedulerStatus`                                                     |

## Current Boundary Shape

### What Wesley/codegen actually covers today

- `crates/ttd-manifest/src/lib.rs` vendors TTD IR/schema artifacts.
- `crates/ttd-protocol-rs/lib.rs` is generated from the TTD schema and serves
  browser/controller protocol needs.

That is useful, but it is **not** the Phase 8 runtime freeze target.

### What Phase 8 has now seeded in-repo

- `schemas/runtime/artifact-a-identifiers.graphql`
- `schemas/runtime/artifact-b-routing-and-admission.graphql`
- `schemas/runtime/artifact-c-playback-control.graphql`
- `schemas/runtime/artifact-d-scheduler-results.graphql`

These are the first local, human-authored ADR-0008 runtime schema fragments.
They are source files, not generated output.

## Proposed Runtime Schema Artifact Set

Phase 8 should generate from a **runtime-focused** schema set, not from the TTD
browser/controller schema. The first honest artifact sketch is:

### Artifact A: Runtime identifiers and logical counters

- `HeadId`
- `WorldlineId`
- `WriterHeadKey`
- `IntentKind`
- `WorldlineTick`
- `GlobalTick`

Source file: `schemas/runtime/artifact-a-identifiers.graphql`

These are the low-level, semantically strict building blocks. They need schema
rules for opaque ids and logical counters before any larger DTOs are generated.

### Artifact B: Runtime routing and admission

- `InboxAddress`
- `InboxPolicy`
- `IngressTarget`
- `HeadEligibility`

Source file: `schemas/runtime/artifact-b-routing-and-admission.graphql`

This artifact covers deterministic ingress routing and declarative admission,
without dragging in transport/conflict surface area.

### Artifact C: Runtime playback control

- `PlaybackMode`
- `SeekThen`

Source file: `schemas/runtime/artifact-c-playback-control.graphql`

This keeps playback semantics explicit and separate from scheduler lifecycle.

### Artifact D: Runtime scheduler result surface

- `SchedulerStatus`
- `SchedulerMode`
- `SchedulerState`
- `WorkState`
- `RunCompletion`
- `RunId`
- `HeadDisposition`

Source file: `schemas/runtime/artifact-d-scheduler-results.graphql`

This replaces the stale `SuperTickResult` shorthand with the actual stable
control-plane surface exposed by ABI v3.

### Deliberately out of this schema set

- observation DTOs such as `HeadInfo`, `HeadObservation`, and snapshot response
  envelopes
- transport/conflict types from ADR-0009
- TTD/browser/controller protocol events and models

Those remain adapter- or product-level concerns until the runtime freeze set is
pinned.

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

There is not yet a **complete** ADR-0008 runtime schema artifact set in-repo.
This branch now seeds Artifacts A-D under `schemas/runtime/`, but generated IR
and generated Rust output are still missing.

### 2. Missing generation/sync entrypoint

The repo advertises `cargo xtask wesley sync`, but `xtask/src/main.rs` does not
implement it yet. Phase 8 needs a real, deterministic regeneration path before
generated artifacts can be trusted.

For now, this branch keeps the runtime freeze loop Echo-local:

- the source of truth lives under `schemas/runtime/*.graphql`,
- local validation happens via `pnpm schema:runtime:check`,
- and Wesley sync stays deliberately deferred until the upstream Echo-facing
  schema/compiler contract stops moving.

### 3. Chosen generated owner for shared runtime-schema types

Phase 8 now has a concrete ownership decision for the generated Rust side:

- shared opaque ids, logical counters, and structural runtime key types now
  live in `crates/echo-runtime-schema`,
- `warp-core` already consumes or re-exports those shared types for the subset
  landed in this slice,
- `IntentKind` and `InboxAddress` intentionally remain in `warp-core` because
  they are frozen runtime types but not yet worth centralizing,
- and `echo-wasm-abi` should remain adapter-owned for host DTOs and convert to
  and from the shared types rather than own a duplicate generated copy.

### 4. Stale plan naming around scheduler results

The inventory resolves this now: Phase 8 should freeze `SchedulerStatus`,
`SchedulerState`, `WorkState`, `RunCompletion`, `HeadEligibility`, and
`HeadDisposition` rather than back-porting a stale `SuperTickResult` name for
cosmetic consistency.

### 5. Opaque id / logical counter mapping rules

`HeadId`, `IntentKind`, `WorldlineTick`, and `GlobalTick` all have stricter
semantics than "some bytes" or "some integer." Phase 8 needs schema-side rules
for:

- opaque fixed-size hash ids,
- logical counters that are coordinates rather than time,
- and wrapper-vs-generated ownership boundaries.

This is now captured in the
[Phase 8 Runtime Schema Mapping Contract](phase-8-runtime-schema-mapping-contract.md).
The remaining Phase 8 work is closeout and explicit deferral, not more
ownership discovery.

## Recommended Phase 8 Slice Order

### Slice A: Freeze inventory and naming reconciliation

- ratify the freeze set
- replace stale `SuperTickResult` wording in the living plan with the explicit
  scheduler result surface
- record which surfaces remain adapter-only
- pin the runtime schema artifact split before adding generation

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
- the stale `SuperTickResult` naming drift is resolved explicitly,
- the first runtime schema artifact set is sketched concretely,
- and the next implementation slice can start from a concrete schema authoring
  target instead of inference from Rust code.

That bar is now met. The next honest Phase 8 slice is Echo-local schema
hardening and deferred-generation contract work, not premature Wesley
integration.
