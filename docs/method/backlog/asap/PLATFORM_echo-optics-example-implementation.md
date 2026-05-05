<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Example Implementation

Status: complete.

Depends on:

- [Echo Optics observe model](./PLATFORM_echo-optics-observe-model.md)
- [Echo Optics dispatch intent model](./PLATFORM_echo-optics-dispatch-intent-model.md)
- [Echo Optics stale-basis obstruction tests](./PLATFORM_echo-optics-stale-basis-obstruction-tests.md)
- [Echo Optics ABI DTOs](./PLATFORM_echo-optics-abi-dtos.md)

Design source:
[TASK-013](../../../design/0018-echo-optics-api-design/design.md#task-013-add-narrow-fakeexample-optic-implementation)

## Goal

Implement one simple optic path to validate ergonomics without broad runtime
abstraction.

## Files likely touched

- `crates/warp-core/src/optic.rs`
- `crates/warp-core/tests/optic_example_tests.rs`
- `crates/warp-wasm/src/warp_kernel.rs`

## Acceptance criteria

- Example optic can read a worldline head or QueryBytes-style payload.
- Example optic can dispatch one EINT intent with explicit base coordinate.
- It uses typed read/admission outcomes.

## Non-goals

- Do not implement a universal optic engine.
- Do not use jedit as the concrete runtime dependency.

## Test expectations

- Read, dispatch, stale-basis, and obstruction tests pass on the example.

## Completion evidence

- Added `WorldlineHeadOptic` as a narrow generic request-builder example in
  `crates/warp-core/src/optic.rs`.
- Added `KernelPort::observe_optic` and the `warp-wasm` `observe_optic` export
  so the example read path crosses the same ABI boundary as dispatch.
- Added `crates/warp-core/tests/optic_example_tests.rs` for bounded head reads,
  QueryBytes-style typed obstruction, EINT proposal construction, and
  stale-basis obstruction.
- Added an engine-backed `warp-wasm` test proving the example reads and stages
  an EINT proposal through `WarpKernel`.
