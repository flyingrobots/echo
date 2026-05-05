<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Attachment Boundary Model

Status: complete.

## Completion evidence

- Added `crates/warp-core/tests/optic_attachment_tests.rs`.
- Attachment-boundary reads with `BoundaryOnly` now return typed
  `AttachmentDescentRequired` posture instead of falling through to generic
  unsupported projection.
- Explicit attachment descent requires positive attachment budget and otherwise
  returns typed `AttachmentDescentDenied` until a capability checker and
  projection law are installed.

Depends on:

- [Echo Optics observe model](./PLATFORM_echo-optics-observe-model.md)

Design source:
[TASK-012](../../../design/0018-echo-optics-api-design/design.md#task-012-add-attachment-boundarydescent-placeholder-model)

## Goal

Make attachments explicit aperture boundaries in optic reads.

## Files likely touched

- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/attachment.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`

## Acceptance criteria

- Default readings expose attachment refs or obstruction posture.
- Recursive descent requires explicit aperture, capability, budget, and law.

## Non-goals

- Do not recursively materialize attachments by default.
- Do not implement nested WARP runtime.

## Test expectations

- Read with no descent returns attachment boundary posture.
- Read with unauthorized descent returns typed obstruction.
