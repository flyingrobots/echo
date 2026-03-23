<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Phase 8 Runtime Schema Conformance Audit

- **Status:** Echo-local conformance audit for `feat/adr-0008-0009-phase-8`
- **Date:** 2026-03-22
- **Primary Plan:** [Implementation Plan: ADR-0008 and ADR-0009](adr-0008-and-0009.md)
- **Prep Inventory:** [Phase 8 Prep Inventory: ADR-0008 Runtime Schema Freeze](phase-8-schema-freeze-inventory.md)
- **Mapping Contract:** [Phase 8 Runtime Schema Mapping Contract](phase-8-runtime-schema-mapping-contract.md)

## Purpose

This audit answers a narrower question than the prep inventory:

- given the frozen Artifacts A-D under `schemas/runtime/`,
- and given the current hand-written runtime and ABI types,
- where does Echo already conform,
- where is the schema using an intentional GraphQL wrapper shape,
- and where does naming or ownership drift still block honest generation?

Phase 8 should not wire Wesley until these answers are explicit.

## Status Legend

- **Aligned:** the schema meaning matches the current Rust surface directly.
- **Intentional wrapper:** the schema uses a GraphQL-specific carrier shape
  such as `*Input`, `*Kind`, or unions; this is expected and not drift.
- **Blocking drift:** current Rust naming or ownership would make generation
  dishonest or noisy unless reconciled first.
- **Adapter-owned:** the frozen type is real, but its canonical owner is the
  ABI/control-plane layer rather than `warp-core`.

## Executive Summary

1. The schema fragments are semantically sound against the current ADR-0008
   runtime model. There is no evidence that Artifacts A-D froze the wrong
   concepts.
2. The biggest ownership blocker from the earlier Phase 8 slices has now been
   resolved: shared logical counters and the core
   `HeadId`/`WorldlineId`/`WriterHeadKey` types live in
   `crates/echo-runtime-schema`.
3. The biggest blockers before generation are:
    - remaining ABI-edge raw-byte DTOs outside the newly typed
      `WorldlineId`/`HeadId`/`WriterHeadKey` path, where the frozen schema still
      wants semantic wrappers
    - the remaining freeze-set types that still live only in `warp-core`
      instead of the shared owner crate, especially `IntentKind` and
      `InboxAddress`
4. GraphQL-specific input wrappers are expected. They should be treated as
   schema transport encodings, not evidence that the core runtime surface is
   wrong.

## Artifact A: Identifiers and Logical Counters

| Schema type          | Canonical Rust owner today              | Status              | Notes                                                                                                                         |
| -------------------- | --------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `HeadId`             | `crates/echo-runtime-schema/src/lib.rs` | Aligned             | Opaque hash-backed newtype matches the scalar intent. `warp-core` now consumes the shared owner and the ABI keeps a byte DTO. |
| `WorldlineId`        | `crates/echo-runtime-schema/src/lib.rs` | Aligned             | Opaque worldline identifier matches the scalar intent and now has a shared owner.                                             |
| `IntentKind`         | `crates/warp-core/src/head_inbox.rs`    | Aligned             | Domain-separated hash-backed newtype already matches the frozen scalar semantics.                                             |
| `WorldlineTick`      | `crates/echo-runtime-schema/src/lib.rs` | Aligned             | Shared logical-counter owner now exists and both `warp-core` and `echo-wasm-abi` consume it.                                  |
| `GlobalTick`         | `crates/echo-runtime-schema/src/lib.rs` | Aligned             | Same as `WorldlineTick`: shared owner exists, semantics stay intact.                                                          |
| `RunId`              | `crates/echo-runtime-schema/src/lib.rs` | Aligned             | Shared owner exists and the ABI re-exports the same control-plane token type.                                                 |
| `WriterHeadKey`      | `crates/echo-runtime-schema/src/lib.rs` | Aligned             | Runtime and ABI now agree on both name and typed field shape; runtime owner is shared.                                        |
| `WriterHeadKeyInput` | none; schema-only wrapper               | Intentional wrapper | GraphQL needs an explicit input mirror even though the runtime only needs `WriterHeadKey`.                                    |

### Artifact A blockers

- **Remaining shared-owner gap:** `IntentKind` still lives only in `warp-core`,
  so the freeze-set owner split is not fully complete for Artifact A.

## Artifact B: Routing and Admission

| Schema type                                | Canonical Rust owner today                                                                  | Status              | Notes                                                                                                                               |
| ------------------------------------------ | ------------------------------------------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `InboxAddress`                             | `crates/warp-core/src/head_inbox.rs`                                                        | Aligned             | Runtime newtype over `String` matches the scalar alias intent.                                                                      |
| `HeadEligibility`                          | `crates/warp-core/src/head.rs` with ABI mirror in `crates/echo-wasm-abi/src/kernel_port.rs` | Aligned             | The two-state model matches exactly across runtime and ABI.                                                                         |
| `IngressTarget`                            | `crates/warp-core/src/head_inbox.rs`                                                        | Intentional wrapper | Core enum is correct. Schema unions plus `IngressTargetInput` are GraphQL carriers for the same three-way split.                    |
| `IngressTargetInput` / `IngressTargetKind` | none; schema-only wrappers                                                                  | Intentional wrapper | Required because GraphQL does not have native input unions.                                                                         |
| `InboxPolicy`                              | `crates/warp-core/src/head_inbox.rs`                                                        | Intentional wrapper | Core enum is correct. Schema unions plus `InboxPolicyInput` are a transport encoding for `AcceptAll`, `KindFilter`, and `Budgeted`. |
| `InboxPolicyInput` / `InboxPolicyKind`     | none; schema-only wrappers                                                                  | Intentional wrapper | Required by GraphQL input limitations.                                                                                              |

### Artifact B notes

- `InboxPolicy::Budgeted { max_per_tick: u32 }` maps cleanly to the schema's
  `maxPerTick: Int!`, but the positive-value rule remains semantic validation,
  not a stronger type-level guarantee.
- `IngressTarget::ExactHead { key: WriterHeadKey }` now inherits the aligned
  typed-id ABI surface from Artifact A rather than a raw-byte fallback.

## Artifact C: Playback Control

| Schema type                              | Canonical Rust owner today         | Status              | Notes                                                                                           |
| ---------------------------------------- | ---------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------- |
| `SeekThen`                               | `crates/warp-core/src/playback.rs` | Aligned             | Two-state follow-up enum matches exactly.                                                       |
| `PlaybackMode`                           | `crates/warp-core/src/playback.rs` | Intentional wrapper | Core enum is correct. Schema unions plus `PlaybackModeInput` encode the same modes for GraphQL. |
| `PlaybackModeInput` / `PlaybackModeKind` | none; schema-only wrappers         | Intentional wrapper | Required because GraphQL input unions do not exist.                                             |

### Artifact C notes

- No semantic mismatch was found here.
- The main rule for generation is to keep `PlaybackModeInput` as a carrier DTO
  rather than pretending it is the canonical runtime enum.

## Artifact D: Scheduler Results

| Schema type                                | Canonical Rust owner today                | Status              | Notes                                                                                                          |
| ------------------------------------------ | ----------------------------------------- | ------------------- | -------------------------------------------------------------------------------------------------------------- |
| `SchedulerMode`                            | `crates/echo-wasm-abi/src/kernel_port.rs` | Adapter-owned       | The ABI is the real owner today. The schema fragment matches the single `UntilIdle` mode honestly.             |
| `SchedulerModeInput` / `SchedulerModeKind` | none; schema-only wrappers                | Intentional wrapper | Required for GraphQL input encoding.                                                                           |
| `SchedulerState`                           | `crates/echo-wasm-abi/src/kernel_port.rs` | Adapter-owned       | Enum matches exactly.                                                                                          |
| `WorkState`                                | `crates/echo-wasm-abi/src/kernel_port.rs` | Adapter-owned       | Enum matches exactly.                                                                                          |
| `RunCompletion`                            | `crates/echo-wasm-abi/src/kernel_port.rs` | Adapter-owned       | Enum matches exactly.                                                                                          |
| `HeadDisposition`                          | `crates/echo-wasm-abi/src/kernel_port.rs` | Adapter-owned       | This is an ABI truth surface derived from runtime state; no core-owned counterpart is required.                |
| `SchedulerStatus`                          | `crates/echo-wasm-abi/src/kernel_port.rs` | Adapter-owned       | Struct shape matches the fragment. The schema is freezing the control-plane surface, not a `warp-core` struct. |

### Artifact D notes

- `HeadDisposition` is not missing just because `warp-core` does not own it.
  It is intentionally an ABI/control-plane truth type.
- The scheduler-result freeze set is therefore a mix of core-owned concepts
  (`HeadEligibility`) and ABI-owned DTOs (`SchedulerStatus`, `HeadDisposition`,
  `SchedulerMode`).

## Cross-Cutting Generation Blockers

### 1. Opaque ids should not dissolve into raw byte vectors at the ABI edge

`HeadId`, `WorldlineId`, and `WriterHeadKey` are frozen as typed opaque runtime
concepts. The current ABI still exposes some of them as raw `Vec<u8>` fields.

That is workable for hand-written DTOs, but it is the wrong source-of-truth
shape for generated runtime-schema types.

### 2. Shared-owner expansion is not complete yet

Phase 8 has already moved the frozen logical counters and core opaque ids/key
types into `crates/echo-runtime-schema`, and `warp-core`/`echo-wasm-abi` now
consume that shared owner where their semantics match.

The remaining question is whether other freeze-set semantic types such as
`IntentKind` and `InboxAddress` should join that shared owner before generation
plumbing lands.

### 3. GraphQL wrapper DTOs must stay wrapper DTOs

The schema's `*Input` and `*Kind` types exist because GraphQL cannot express
input unions directly. They should not be mistaken for evidence that the core
runtime enums are wrong or incomplete.

This matters for generation layout:

- core runtime generation should target the semantic enums and keys,
- adapter generation may also emit GraphQL carrier wrappers,
- but those wrappers should not leak back into `warp-core` as the new source of
  truth.

## Follow-On

The scalar and ownership rules are now pinned in the
[Phase 8 Runtime Schema Mapping Contract](phase-8-runtime-schema-mapping-contract.md).
The next honest implementation slice is the remaining shared-owner decision for
`IntentKind` and `InboxAddress`, followed by explicit deferral of Wesley
generation plumbing until the upstream contract stabilizes.
