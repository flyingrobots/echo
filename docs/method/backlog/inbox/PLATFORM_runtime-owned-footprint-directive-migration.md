<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Runtime-Owned Footprint Directive Migration

Status: inbox. Echo should decide when to pull the trigger.

## Why now

Runtime optic artifacts currently use `@wes_footprint` as the authored
admission-facing footprint directive. The v0 behavior is useful: Wesley accepts
the directive on selected root fields, compiles it into admission requirements,
and rejects nested or scoped placements.

The spelling is still an ownership smell. Echo or another runtime interprets
the footprint as admission policy. Wesley validates and compiles the declared
shape, but Wesley should not imply it owns runtime admission semantics.

## Hill

Echo decides whether and when to migrate from `@wes_footprint` to a
runtime-owned spelling such as `@echo_footprint`, while preserving a deliberate
compatibility path for existing fixtures and consumers.

## Done looks like

- a migration plan states whether `@wes_footprint` remains as a legacy alias,
  warning, or removed spelling
- docs explain that Wesley compiles footprint declarations and Echo interprets
  them as admission requirements
- runtime optic tests cover the chosen compatibility behavior
- `requirements_digest` behavior is unchanged for semantically equivalent
  footprint declarations during any alias period
- nested, fragment, inline-fragment, and operation-level footprints remain
  rejected until scoped footprints are deliberately designed

## Non-goals

- Do not break current Echo fixtures just to rename the directive.
- Do not move footprint semantics into generic Wesley.
- Do not implement scoped footprints in this card.
