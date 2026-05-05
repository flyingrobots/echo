<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Obstruction And Admission Results

Status: complete.

Depends on:

- [Echo Optics core nouns and IDs](./PLATFORM_echo-optics-core-nouns-and-ids.md)

Design source:
[TASK-005](../../../design/0018-echo-optics-api-design/design.md#task-005-define-obstruction-and-admission-result-families)

## Goal

Add typed `OpticObstruction` and `IntentDispatchResult` enums.

## Files likely touched

- `crates/warp-core/src/optic.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/warp_kernel.rs`

## Acceptance criteria

- Outcomes include Admitted, Staged, Plural, Conflict, and Obstructed.
- Stale basis, missing witness, budget exceeded, capability denied, and
  attachment descent required are distinct obstruction kinds.

## Non-goals

- Do not model outcomes as `Ok/Err`, bool, or string status.
- Do not introduce latest-writer-wins fallback.

## Test expectations

- ABI serialization preserves outcome variants.
- Exhaustive matching tests fail if variants collapse.
