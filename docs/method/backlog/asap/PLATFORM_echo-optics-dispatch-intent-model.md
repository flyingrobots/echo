<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Dispatch Intent Model

Status: complete. Echo now has typed core/ABI optic dispatch request models,
an EINT v1 optic payload, an admission-law id, and a KernelPort route that
validates the optic proposal and carries EINT v1 through the existing
`dispatch_intent` path as a typed staged admission posture.

Depends on:

- [Echo Optics open and close models](./PLATFORM_echo-optics-open-close-models.md)
- [Echo Optics obstruction and admission results](./PLATFORM_echo-optics-obstruction-admission-results.md)
- [Witnessed suffix admission shells](./PLATFORM_witnessed-suffix-admission-shells.md)

Design source:
[TASK-008](../../../design/0018-echo-optics-api-design/design.md#task-008-define-dispatch_optic_intent-with-explicit-base-coordinate)

## Goal

Add the write-side proposal DTO and route one existing EINT path through the
optic dispatch model.

## Files likely touched

- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/head_inbox.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

## Acceptance criteria

- Request names optic id, base coordinate, intent family, focus, actor/cause,
  capability, admission law, and payload.
- Current EINT v1 payloads can be carried.
- Dispatch outcome is typed.

Evidence:

- `crates/warp-core/src/optic.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/lib.rs`
- `crates/echo-wasm-abi/src/lib.rs`

## Non-goals

- Do not add setters.
- Do not create a second intent envelope without a failing RED.

## Test expectations

- Missing base coordinate is impossible or rejected.
- Accepted intent names resulting tick/receipt/admission posture.
