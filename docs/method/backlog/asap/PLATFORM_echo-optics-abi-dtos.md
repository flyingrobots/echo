<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics ABI DTOs

Status: complete. The ABI exposes the optic DTO surface needed by generated
request builders, round-trips the required observe/dispatch DTO set
deterministically, and has a generated-helper-shaped smoke crate compiling
against `echo-wasm-abi`.

Depends on:

- [Echo Optics observe model](./PLATFORM_echo-optics-observe-model.md)
- [Echo Optics dispatch intent model](./PLATFORM_echo-optics-dispatch-intent-model.md)
- [Echo Optics obstruction and admission results](./PLATFORM_echo-optics-obstruction-admission-results.md)

Design source:
[TASK-017](../../../design/0018-echo-optics-api-design/design.md#task-017-add-echo-optics-abi-dtos-required-by-generated-bindings)

## Goal

Add the minimum ABI DTOs needed for generated optic request builders to compile
against `echo-wasm-abi`.

## Files likely touched

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `crates/echo-wesley-gen/tests/generation.rs`

## Acceptance criteria

- ABI exposes `OpticId`, `OpticFocus`, `EchoCoordinate`, `OpticAperture`,
  `ObserveOpticRequest`, `DispatchOpticIntentRequest`, `OpticIntentPayload`,
  `IntentDispatchResult`, and supporting refs.
- DTOs serialize deterministically.
- Generated optic helper smoke crate compiles against the ABI.

Evidence:

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/echo-wasm-abi/src/lib.rs`
- `crates/echo-wesley-gen/tests/generation.rs`

## Non-goals

- Do not implement full runtime semantics.
- Do not add global graph APIs.

## Test expectations

- ABI encode/decode round-trips.
- Generated consumer crate compiles with generated optic helpers.
