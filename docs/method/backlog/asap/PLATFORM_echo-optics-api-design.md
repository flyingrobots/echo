<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics API Design

Status: active sequencing card.

Design packet:
[0018 - Echo Optics API Design](../../../design/0018-echo-optics-api-design/design.md)

Source request:
[request.md](../../../design/0018-echo-optics-api-design/request.md)

## Why now

Echo already has observation artifacts, reading envelopes, witness-bearing
suffix admission, strands, and retention doctrine. The missing step is the
small public API noun that binds those facts into a bounded, causal,
capability-scoped read/propose surface.

The optic API must be generic enough for editors, debuggers, inspectors, replay
tools, import/export flows, and retained reading caches without becoming a
global graph API or file-handle API.

## Sequence

1. Add doctrine/design packet for Echo Optics.
2. Define core optic nouns and IDs.
3. Define ReadingEnvelope and ReadIdentity.
4. Define WitnessBasis and retained reading key.
5. Define optic obstruction/admission result families.
6. Define `open_optic` / `close_optic` request models.
7. Define `observe_optic` model with bounds and aperture.
8. Define `dispatch_optic_intent` model with explicit base coordinate.
9. Add stale-basis obstruction tests.
10. Add cached-reading identity tests.
11. Add live-tail hash honesty tests.
12. Add attachment boundary/descent placeholder model.
13. Add narrow fake/example optic implementation for one simple contract.
14. Add adapter notes for future editor/debugger/replay consumers.
15. Add Echo-owned Wesley optic binding spec.
16. Extend `echo-wesley-gen` with optic request builders.
17. Add Echo Optics ABI DTOs required by generated bindings.

## Acceptance criteria

- Design packet uses the requested section structure.
- API surface is small, typed, bounded, causal, and capability-scoped.
- Direct mutation, setters, global graph/state APIs, hidden materialization
  fallback, and broad host-bag abstractions are rejected.
- Backlog tasks each name goal, files likely touched, acceptance criteria,
  non-goals, and test expectations.
- The design explains how Wesley-compiled output becomes typed optic bindings
  while preserving explicit Echo intent dispatch.

## Non-goals

- Do not implement runtime code in this sequencing card.
- Do not design jedit as the primary task.
- Do not make GraphQL the core runtime API.
