<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Witness Basis And Retained Key

Status: complete.

Depends on:

- [Echo Optics reading envelope and identity](./PLATFORM_echo-optics-reading-envelope-identity.md)

Design source:
[TASK-004](../../../design/0018-echo-optics-api-design/design.md#task-004-define-witnessbasis-and-retained-reading-key)

## Goal

Represent commit, checkpoint-plus-tail, witness-set, and missing-basis postures
for honest retained readings.

## Files likely touched

- `crates/warp-core/src/observation.rs`
- `crates/echo-cas/src/lib.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

## Acceptance criteria

- Retained reading key includes content hash and semantic read identity.
- Checkpoint-plus-tail identity cannot collapse to checkpoint hash alone.

## Non-goals

- Do not build storage GC policy.
- Do not implement proof systems.

## Test expectations

- Retained reading lookup by content hash alone fails.
- Checkpoint-plus-tail and checkpoint-only identities differ.
