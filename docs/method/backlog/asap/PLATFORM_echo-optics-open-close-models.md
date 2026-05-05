<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Open And Close Models

Status: complete.

Depends on:

- [Echo Optics core nouns and IDs](./PLATFORM_echo-optics-core-nouns-and-ids.md)
- [Echo Optics obstruction and admission results](./PLATFORM_echo-optics-obstruction-admission-results.md)

Design source:
[TASK-006](../../../design/0018-echo-optics-api-design/design.md#task-006-define-open_optic-and-close_optic-request-models)

## Goal

Add descriptor-validation DTOs for opening and closing session-local optic
resources.

## Files likely touched

- `crates/warp-core/src/optic.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

## Acceptance criteria

- `open_optic` validates focus, coordinate, projection law, intent family, and
  capability.
- `close_optic` releases only session-local descriptor resources.
- Closing an optic does not mutate subject history or invalidate old readings.

## Non-goals

- Do not make optics file handles.
- Do not implement mutable object handles.

## Test expectations

- Opening denied capability returns typed obstruction/error.
- Closing does not change observed frontier or provenance length.
