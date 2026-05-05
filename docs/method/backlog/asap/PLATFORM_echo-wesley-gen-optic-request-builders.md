<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Wesley Gen Optic Request Builders

Status: complete.

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

## Completion evidence

- `echo-wesley-gen` now emits query `*_observe_optic_request` and
  `*_observe_optic_request_raw_vars` helpers alongside existing
  `*_observation_request` compatibility helpers.
- Mutation ops now emit `*_dispatch_optic_intent_request` and
  `*_dispatch_optic_intent_request_raw_vars` helpers that require explicit
  `base_coordinate`, focus, intent family, cause, capability, and admission law.
- The generated EINT pack helpers remain available and are used as the inner
  payload for optic dispatch requests.
- Setter-like mutation names are rewritten for optic helpers as proposal
  builders, for example `propose_set_theme_dispatch_optic_intent_request`.
- Std and no-std generated smoke crates compile with the new helper surface.
