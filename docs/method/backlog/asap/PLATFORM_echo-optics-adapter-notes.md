<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Adapter Notes

Status: complete.

## Completion evidence

- Added [Echo Optics Adapter Notes](../../../architecture/echo-optics-adapter-notes.md).
- The note keeps GraphQL as an adapter/authoring illustration, rejects global
  state adapters and host-bag abstractions, and names `jedit` only as an
  ergonomic example consumer.

Depends on:

- [Echo Optics example implementation](./PLATFORM_echo-optics-example-implementation.md)
- [Echo Wesley Gen optic request builders](./PLATFORM_echo-wesley-gen-optic-request-builders.md)

Design source:
[TASK-014](../../../design/0018-echo-optics-api-design/design.md#task-014-add-adapter-notes-for-future-consumers)

## Goal

Document how editors, debuggers, inspectors, replay tools, import/export flows,
retained reading caches, and GraphQL adapters should sit above the core Optics
API.

## Files likely touched

- `docs/architecture/echo-optics-adapter-notes.md`
- `docs/design/0018-echo-optics-api-design/design.md`

## Acceptance criteria

- Notes clearly say GraphQL is an adapter illustration, not the runtime
  substrate.
- Notes reject global state adapters and host-bag abstractions.
- Notes show `jedit` only as an ergonomic example consumer.

## Non-goals

- Do not design product-specific APIs.
- Do not add a sync daemon or git-warp dependency.

## Test expectations

- Docs checks pass.
- Links from design packet and backlog card resolve in docs build.
