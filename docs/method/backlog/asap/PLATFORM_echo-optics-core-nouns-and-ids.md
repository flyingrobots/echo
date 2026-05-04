<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Core Nouns And IDs

Status: visible task card.

Depends on:

- [Echo Optics API Design](./PLATFORM_echo-optics-api-design.md)

Design source:
[TASK-002](../../../design/0018-echo-optics-api-design/design.md#task-002-define-core-optic-nouns-and-ids)

## Goal

Add initial Rust DTOs for `EchoOptic`, `OpticId`, `OpticFocus`,
`OpticAperture`, `EchoCoordinate`, `ProjectionVersion`, and `ReducerVersion`.

## Files likely touched

- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/lib.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

## Acceptance criteria

- DTOs are deterministic, serializable where ABI-facing, and domain-separated
  where hashed.
- Focus covers worldline, strand, braid, retained reading, and attachment
  boundary without exposing a global graph handle.

## Non-goals

- Do not add a universal optic engine.
- Do not add jedit/editor/file types.

## Test expectations

- Unit tests cover stable ID hashing and focus/coordinate encoding.
- ABI round-trip tests cover public DTOs.
