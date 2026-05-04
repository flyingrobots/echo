<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# jedit Text Contract MVP

Status: planned consumer proof.

Depends on:

- [Contract artifact retention in echo-cas](./PLATFORM_contract-artifact-retention-in-echo-cas.md)
- external `jedit` hot text runtime port
- external Wesley contract authoring support

## Why now

`jedit` is the first serious consumer for Echo as a Wesley-compiled contract
host. The text-editing model must remain application-specific. Echo should host
the generated contract through generic dispatch and observation surfaces.

## What it should look like

Define a `jedit` GraphQL contract for the minimum hot editing loop.

Candidate intents:

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

Wesley compiles the contract. Echo registers and hosts it. `jedit` adapts its
existing hot text runtime port to submit generated contract intents.

## Acceptance criteria

- `jedit` can create a buffer through a contract intent.
- `jedit` can submit a replace-range intent through Echo.
- Echo emits a contract-aware receipt for the edit.
- `jedit` can observe a buffer reading through Echo.
- Save checkpoint produces a retained contract artifact.
- Echo core contains no text-specific APIs outside generated contract payloads.

## Non-goals

- Do not add text-editing types to Echo core.
- Do not add a special `jedit` ABI.
- Do not wire Graft automation in this card.
- Do not require multi-buffer collaboration.
- Do not implement strands or counterfactual refactors here.
