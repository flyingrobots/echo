<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Stale-Basis Obstruction Tests

Status: visible task card.

Depends on:

- [Echo Optics dispatch intent model](./PLATFORM_echo-optics-dispatch-intent-model.md)

Design source:
[TASK-009](../../../design/0018-echo-optics-api-design/design.md#task-009-add-stale-basis-obstruction-tests)

## Goal

Prove stale base coordinate does not silently mutate current frontier.

## Files likely touched

- `crates/warp-core/tests/optic_dispatch_tests.rs`
- `crates/warp-core/src/optic.rs`

## Acceptance criteria

- Dispatch against stale base returns Obstructed, Staged, Plural, Conflict, or
  explicitly law-admitted result.
- The default path must not mutate latest frontier silently.

## Non-goals

- Do not implement rebase workflow.
- Do not hide host-time ordering.

## Test expectations

- Provenance length and current head remain unchanged for obstructed stale-base
  dispatch.
