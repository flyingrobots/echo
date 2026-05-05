<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Live-Tail Honesty Tests

Status: complete.

## Completion evidence

- Added `crates/warp-core/tests/optic_live_tail_tests.rs`.
- Added a regression proving a frontier read after checkpoint plus live-tail
  commit names `WitnessBasis::CheckpointPlusTail`.
- The live-tail witness set records the checkpoint basis, tail provenance refs,
  and non-empty tail digest instead of reusing a checkpoint-only identity.

Depends on:

- [Echo Optics reading envelope and identity](./PLATFORM_echo-optics-reading-envelope-identity.md)
- [Echo Optics witness basis and retained key](./PLATFORM_echo-optics-witness-basis-retained-key.md)

Design source:
[TASK-011](../../../design/0018-echo-optics-api-design/design.md#task-011-add-live-tail-hash-honesty-tests)

## Goal

Prevent stale checkpoint hashes from identifying live optic readings.

## Files likely touched

- `crates/warp-core/tests/optic_live_tail_tests.rs`
- `crates/warp-core/src/observation.rs`

## Acceptance criteria

- A live frontier with checkpoint plus tail cannot return checkpoint-only
  identity.
- Result either reduces live tail, names checkpoint-plus-tail witness basis, or
  obstructs.

## Non-goals

- Do not implement production compaction/wormholes.

## Test expectations

- Add tick after checkpoint; live read identity changes and names tail evidence.
