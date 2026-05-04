<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Wesley Gen Optic Request Builders

Status: visible task card.

Depends on:

- [Echo Optics ABI DTOs](./PLATFORM_echo-optics-abi-dtos.md)
- [Echo Optics example implementation](./PLATFORM_echo-optics-example-implementation.md)
- [Echo-owned Wesley optic binding spec](../../../design/0018-echo-optics-api-design/wesley-compiled-optic-bindings.md)

Design source:
[TASK-016](../../../design/0018-echo-optics-api-design/design.md#task-016-extend-echo-wesley-gen-with-optic-request-builders)

## Goal

Generate typed `*_observe_optic_request` and
`*_dispatch_optic_intent_request` helpers alongside existing compatibility
helpers.

## Files likely touched

- `crates/echo-wesley-gen/src/main.rs`
- `crates/echo-wesley-gen/src/ir.rs`
- `crates/echo-wesley-gen/tests/generation.rs`
- `crates/echo-wesley-gen/tests/fixtures/toy-counter/echo-ir-v1.json`

## Acceptance criteria

- Query ops emit typed optic observation request builders.
- Mutation ops emit typed optic dispatch request builders.
- Mutation builders require explicit base coordinate by default.
- Existing EINT and `ObservationRequest` helpers remain available.
- Generated names do not collide with user contract types.

## Non-goals

- Do not remove existing helper surface in this slice.
- Do not add jedit-specific codegen.

## Test expectations

- Generated std smoke crate compiles.
- Generated no-std smoke crate compiles where request builders are no-std-safe.
- Tests assert no generated method uses `set_*` naming.
