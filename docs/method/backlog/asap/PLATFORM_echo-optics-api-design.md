<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics API Design

Status: design/spec packet complete; executable work split into visible cards.

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

## Completed on this branch

- Doctrine/design packet for Echo Optics.
- Echo-owned Wesley optic binding companion spec.

## Visible execution cards

Executable follow-through is tracked by separate backlog cards so dependencies
are visible to METHOD and to agents reading the filesystem.

1. [Core optic nouns and IDs](./PLATFORM_echo-optics-core-nouns-and-ids.md)
2. [Reading envelope and identity](./PLATFORM_echo-optics-reading-envelope-identity.md)
3. [Witness basis and retained reading key](./PLATFORM_echo-optics-witness-basis-retained-key.md)
4. [Obstruction and admission results](./PLATFORM_echo-optics-obstruction-admission-results.md)
5. [Open and close models](./PLATFORM_echo-optics-open-close-models.md)
6. [Observe model](./PLATFORM_echo-optics-observe-model.md)
7. [Dispatch intent model](./PLATFORM_echo-optics-dispatch-intent-model.md)
8. [Stale-basis obstruction tests](./PLATFORM_echo-optics-stale-basis-obstruction-tests.md)
9. [Cached-reading identity tests](./PLATFORM_echo-optics-cached-reading-identity-tests.md)
10. [Live-tail honesty tests](./PLATFORM_echo-optics-live-tail-honesty-tests.md)
11. [Attachment boundary model](./PLATFORM_echo-optics-attachment-boundary-model.md)
12. [Echo Optics ABI DTOs](./PLATFORM_echo-optics-abi-dtos.md)
13. [Example optic implementation](./PLATFORM_echo-optics-example-implementation.md)
14. [Wesley optic request builders](./PLATFORM_echo-wesley-gen-optic-request-builders.md)
15. [Adapter notes](./PLATFORM_echo-optics-adapter-notes.md)

## Dependency rule

Do not add new executable work only as a numbered sequence inside this card or
inside the design packet. Add a visible backlog card with a `Depends on:`
section instead.

## Acceptance criteria

- Design packet uses the requested section structure.
- API surface is small, typed, bounded, causal, and capability-scoped.
- Direct mutation, setters, global graph/state APIs, hidden materialization
  fallback, and broad host-bag abstractions are rejected.
- Backlog tasks each name goal, files likely touched, acceptance criteria,
  non-goals, and test expectations.
- The design explains how Wesley-compiled output becomes typed optic bindings
  while preserving explicit Echo intent dispatch.
- Executable follow-through exists as visible METHOD cards with dependency
  links rather than hidden subtasks.

## Non-goals

- Do not implement runtime code in this sequencing card.
- Do not design jedit as the primary task.
- Do not make GraphQL the core runtime API.
