<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Cached-Reading Identity Tests

Status: complete.

## Completion evidence

- Added `RetainedReadingCache` plus retain/reveal request/result models in
  `warp-core`.
- Added regression tests proving same content bytes under different coordinates
  or apertures receive distinct retained keys.
- Added reveal tests proving content-hash-only lookup misses and exact
  `ReadIdentity` is required to reveal retained payload bytes.

Depends on:

- [Echo Optics reading envelope and identity](./PLATFORM_echo-optics-reading-envelope-identity.md)
- [Echo Optics witness basis and retained key](./PLATFORM_echo-optics-witness-basis-retained-key.md)

Design source:
[TASK-010](../../../design/0018-echo-optics-api-design/design.md#task-010-add-cached-reading-identity-tests)

## Goal

Prove retained/cached readings are keyed by read identity, not just content
hash.

## Files likely touched

- `crates/warp-core/tests/optic_retention_tests.rs`
- `crates/echo-cas/src/lib.rs`
- `crates/warp-core/src/observation.rs`

## Acceptance criteria

- Same content bytes under different coordinate or aperture produce distinct
  retained keys.
- Reveal requires matching read identity.

## Non-goals

- Do not build distributed CAS.
- Do not add semantic ontology to CAS hashes.

## Test expectations

- Content-hash-only reveal returns obstruction or lookup miss.
- Matching read identity reveals payload.
