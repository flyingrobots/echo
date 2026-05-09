<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# jedit Text Contract Hosting MVP

Status: planned Echo host integration proof.

Depends on:

- [Contract artifact retention in echo-cas](./PLATFORM_contract-artifact-retention-in-echo-cas.md)
- external `jedit` Text File Optic contract surface
- external `jedit` hot text runtime port
- external Wesley contract authoring support

## Why now

`jedit` is the first serious consumer for Echo as a Wesley-compiled contract
host. The text-editing model, GraphQL contract, editor runtime, buffer law,
edit-group law, and UI behavior all remain application-specific and belong in
the external `jedit` repo.

Echo should only prove that an externally authored and Wesley-compiled `jedit`
contract can use generic dispatch, observation, registry, receipt, reading, and
retention surfaces.

## What it should look like

Do not define or implement the `jedit` GraphQL contract in Echo. The external
`jedit` repo owns that contract and adapts its hot text runtime to generated
contract helpers.

This Echo card proves host compatibility for externally owned operation
families such as:

- create buffer
- replace range
- open edit group
- include tick in edit group
- close edit group
- save checkpoint

Candidate observations:

- buffer reading
- dirty delta
- edit group history
- checkpoint status

Wesley compiles the external contract. `jedit` adapts its existing hot text
runtime port to generated app-level code that validates payloads, packs EINT op
ids and vars, calls Echo's existing `dispatch_intent(...)`, reads registry
metadata for handshake, and decodes observations. Echo stays the generic host.

Echo tests may use generated `jedit` Wesley output as a fixture. That fixture is
consumer evidence, not Echo-owned source authority: it should be refreshed from
the external `jedit` contract generation path and used only to prove generic
host behavior.

## Acceptance criteria

- Echo can install or accept registry metadata for an externally generated
  `jedit` contract artifact.
- Echo integration tests can exercise generated `jedit` Wesley fixture output
  without requiring Echo to author text-editing SDL.
- Echo accepts generated `jedit` EINT bytes through the existing WASM intent
  ingress without adding text-specific code paths.
- Echo emits contract-aware receipts and readings whose identity includes the
  external contract and operation/query basis.
- Echo can return a typed obstruction for stale basis, unsupported query,
  missing witness, or unavailable retained artifact.
- `jedit` can create a buffer, submit a replace-range intent, and observe a
  buffer reading through Echo-owned generic host surfaces.
- Save checkpoint produces a retained contract artifact through generic
  retention rules.
- Echo core contains no text-specific APIs outside generated contract payloads
  and test fixtures.

## Non-goals

- Do not author or edit the `jedit` GraphQL contract in Echo.
- Do not implement `jedit`'s hot text runtime in Echo.
- Do not treat generated `jedit` fixture output as Echo-owned source truth.
- Do not add text-editing types to Echo core.
- Do not add a special `jedit` ABI.
- Do not move `jedit` payload validation into Echo core unless the registry
  boundary decision explicitly requires Echo-side validation.
- Do not wire Graft automation in this card.
- Do not require multi-buffer collaboration.
- Do not implement strands or counterfactual refactors here.
