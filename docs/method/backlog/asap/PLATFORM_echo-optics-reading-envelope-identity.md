<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Reading Envelope And Identity

Status: visible task card.

Depends on:

- [Echo Optics core nouns and IDs](./PLATFORM_echo-optics-core-nouns-and-ids.md)

Design source:
[TASK-003](../../../design/0018-echo-optics-api-design/design.md#task-003-define-readingenvelope-and-readidentity-extensions)

## Goal

Extend current reading metadata with first-class read identity fields without
breaking existing observation behavior.

## Files likely touched

- `crates/warp-core/src/observation.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/warp_kernel.rs`

## Acceptance criteria

- Read identity names optic id, focus digest, coordinate, aperture digest,
  projection version, reducer version, witness basis, rights, budget, and
  residual posture.
- Existing observations can produce compatible identity for built-in plans.

## Non-goals

- Do not make CAS hash the read identity.
- Do not require full materialization to compute identity.

## Test expectations

- Same read question yields same identity.
- Coordinate, aperture, projection version, or witness basis changes identity.
