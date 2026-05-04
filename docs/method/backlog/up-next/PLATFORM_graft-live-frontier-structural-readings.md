<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Graft Live Frontier Structural Readings

Status: planned consumer integration.

Depends on:

- [jedit text contract MVP](./PLATFORM_jedit-text-contract-mvp.md)
- external Graft structural projection work

## Why now

Once `jedit` edits through Echo-hosted contract intents, Graft should observe
the live contract frontier instead of only saved files or ad hoc editor state.

Graft must consume readings and frontiers. It must not mutate Echo state
directly.

## What it should look like

Use Echo/jedit contract readings as structural projection bases for Graft.

Initial product readings:

- dead symbols for live frontier
- symbol history for current edit group
- impacted tests for current changes
- projection safety status
- stale projection warnings

## Acceptance criteria

- Graft projection names the Echo/jedit contract frontier it observed.
- Projection output distinguishes current, stale, partial, unsafe, and
  unavailable facts.
- `jedit` displays at least one Graft structural reading over a live Echo
  frontier.
- Proposed edits flow back through `jedit` contract intents and Echo admission.
- Graft never writes directly to Echo substrate state.

## Non-goals

- Do not implement rename automation.
- Do not build structural counterfactuals yet.
- Do not bypass Echo admission for edits.
- Do not require Continuum interop.
