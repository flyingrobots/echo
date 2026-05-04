<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Observe Model

Status: visible task card.

Depends on:

- [Echo Optics open and close models](./PLATFORM_echo-optics-open-close-models.md)
- [Echo Optics reading envelope and identity](./PLATFORM_echo-optics-reading-envelope-identity.md)
- [Echo Optics witness basis and retained key](./PLATFORM_echo-optics-witness-basis-retained-key.md)

Design source:
[TASK-007](../../../design/0018-echo-optics-api-design/design.md#task-007-define-observe_optic-model-with-bounds-and-aperture)

## Goal

Add the bounded read request/result model and adapt one existing
`ObservationService` path through it.

## Files likely touched

- `crates/warp-core/src/optic.rs`
- `crates/warp-core/src/observation.rs`
- `crates/warp-wasm/src/warp_kernel.rs`

## Acceptance criteria

- Observe request includes optic id, focus, coordinate, aperture, projection
  version, reducer version, and capability ref.
- Result returns reading or obstruction.
- No hidden full materialization fallback exists.

## Non-goals

- Do not replace all `ObservationService` internals in this slice.
- Do not add global graph query API.

## Test expectations

- Bounded head/snapshot optic returns read identity.
- Oversized aperture returns budget obstruction.
