<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Phase 8 Runtime Schema Mapping Contract

- **Status:** Echo-local mapping contract locked for `feat/adr-0008-0009-phase-8`
- **Date:** 2026-03-22
- **Primary Plan:** [Implementation Plan: ADR-0008 and ADR-0009](adr-0008-and-0009.md)
- **Prep Inventory:** [Phase 8 Prep Inventory: ADR-0008 Runtime Schema Freeze](phase-8-schema-freeze-inventory.md)
- **Conformance Audit:** [Phase 8 Runtime Schema Conformance Audit](phase-8-runtime-schema-conformance.md)

## Purpose

This document pins the Rust-side ownership and scalar-mapping rules for the
frozen ADR-0008 runtime schema.

It answers three concrete questions before any generation plumbing exists:

1. Where will generated runtime-schema types live?
2. Which frozen schema types become shared generated Rust wrappers?
3. Which ABI fields are allowed to remain raw bytes instead of typed wrappers?

## Ownership Decision

The generated-or-generation-ready Rust home for shared ADR-0008 runtime-schema
types is:

- `crates/echo-runtime-schema`

That crate now exists as the single Echo-local owner for the subset that is
already clearly shared across runtime and adapter layers:

- opaque runtime identifiers,
- logical counters,
- structural runtime key types.

Not every frozen schema type automatically belongs there. Runtime-local
behavioral types may stay hand-written in their owning crate when generation
does not buy anything yet.

The ownership split after generation lands should be:

- `echo-runtime-schema`
  generated source of truth for shared runtime-schema types
- `warp-core`
  consumes or re-exports shared semantic types
- `echo-wasm-abi`
  stays adapter-owned for host DTOs and converts to and from the shared types
  where the ABI needs a different wire shape

## Core Rule

Semantic runtime identifiers and logical coordinates must not default to raw
`Vec<u8>` or plain integers in generated Rust.

The default generated form is:

- opaque-id newtype for identifiers,
- logical-counter newtype for counters,
- structured Rust `struct` for composite keys,
- string newtype for named aliases such as `InboxAddress` only when a real
  shared/generated consumer exists

Raw bytes remain acceptable only for fields that are semantically binary
payloads or content hashes rather than typed runtime identifiers.

## Mapping Table

| Schema type     | Generated Rust home   | Generated Rust shape                                                  | ABI DTO policy                                                         | Notes                                                                           |
| --------------- | --------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `HeadId`        | `echo-runtime-schema` | `#[repr(transparent)] struct HeadId([u8; 32]);`                       | Use typed wrapper DTOs for semantic head identifiers                   | This is an opaque runtime identifier, not a generic byte vector.                |
| `WorldlineId`   | `echo-runtime-schema` | `#[repr(transparent)] struct WorldlineId([u8; 32]);`                  | Use typed wrapper DTOs for semantic worldline identifiers              | Same rule as `HeadId`.                                                          |
| `IntentKind`    | `warp-core` (Phase 8) | hand-written opaque-id newtype                                        | Use typed wrapper DTOs where the ABI exposes intent kinds semantically | Keep runtime-owned until a real generated consumer requires a shared home.      |
| `WorldlineTick` | `echo-runtime-schema` | `#[repr(transparent)] struct WorldlineTick(u64);`                     | Use typed wrapper DTOs                                                 | Logical coordinate, not wall-clock time and not a bare `u64` in generated code. |
| `GlobalTick`    | `echo-runtime-schema` | `#[repr(transparent)] struct GlobalTick(u64);`                        | Use typed wrapper DTOs                                                 | Logical cycle stamp, not wall-clock time.                                       |
| `RunId`         | `echo-runtime-schema` | `#[repr(transparent)] struct RunId(u64);`                             | Use typed wrapper DTOs                                                 | Control-plane generation token.                                                 |
| `InboxAddress`  | `warp-core` (Phase 8) | hand-written string newtype                                           | Use typed wrapper DTOs when the field is semantically an inbox alias   | Keep runtime-owned; centralizing a plain routing alias buys nothing today.      |
| `WriterHeadKey` | `echo-runtime-schema` | `struct WriterHeadKey { worldline_id: WorldlineId, head_id: HeadId }` | Use typed wrapper DTOs                                                 | Structural runtime key; its fields should stay typed.                           |

## ABI Raw-Byte Exception Rule

The ABI may keep raw `Vec<u8>` fields only for values that are inherently
binary artifacts or open payloads.

Allowed raw-byte categories:

- content hashes such as `state_root`, `commit_id`, and `artifact_hash`
- channel identifiers and channel payload bytes
- opaque payload envelopes such as `vars_bytes`, `data`, and intent bodies
- compatibility-sensitive byte-oriented blobs that are not part of the
  runtime-schema freeze set

Disallowed raw-byte default:

- semantic runtime identifiers such as `HeadId`, `WorldlineId`, and
  `WriterHeadKey`
- logical coordinates such as `WorldlineTick`, `GlobalTick`, and `RunId`

## Immediate Consequences For Existing Code

### `warp-core`

- `warp-core` already consumes or re-exports the shared logical counters and
  core opaque ids/key types introduced in `echo-runtime-schema`.
- Its role is semantic consumer plus runtime behavior owner.
- `IntentKind` and `InboxAddress` remain intentionally hand-written here for
  Phase 8 because they do not yet justify a shared generated home.

### `echo-wasm-abi`

- `echo-wasm-abi` remains the owner of host DTO layout and CBOR envelope rules.
- It now consumes the shared logical counters from `echo-runtime-schema`.
- It should keep explicit adapter wrappers where the wire contract differs from
  the shared semantic type, such as byte-serialized `HeadId` and `WorldlineId`.
- Existing raw-byte identifier fields outside those typed wrappers are now
  technical debt to retire, not neutral defaults.

### `echo-wesley-gen`

- The current generic GraphQL mappings such as `Int -> i32` and `ID -> String`
  are not sufficient for the ADR-0008 runtime schema.
- The runtime schema depends on custom scalar mappings and typed wrapper output,
  not the generic DTO defaults used by the current IR-to-Rust path.

## Out of Scope

- wiring `cargo xtask wesley sync`
- changing current ABI v3 wire fields in this document alone
- ADR-0009 transport/conflict schema mapping

## Recommended Next Slice

With ownership and scalar-mapping rules pinned and the shared crate now
scaffolded, the next honest implementation slice is:

1. treat the shared-owner split as intentionally complete for Phase 8 unless a
   real generated consumer appears for `IntentKind`,
2. leave hashes and payload blobs as raw bytes,
3. keep Wesley/codegen plumbing deferred until the upstream schema/compiler
   contract stabilizes.
