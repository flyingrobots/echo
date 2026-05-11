<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# jedit Contract Proof Fixture

Status: planned consumer proof fixture.

Depends on:

- [Contract retention and streaming seams](./PLATFORM_contract-retention-and-streaming-seams.md)
- [Generic contract braid substrate](./KERNEL_generic-contract-braid-substrate.md)
- [Contract inverse admission hook](./KERNEL_contract-inverse-admission-hook.md)
- external `jedit` contract/runtime work

## Why now

The generic substrate needs a serious consumer-shaped proof. `jedit` supplies
the text contract fixture, but Echo must not import its nouns into core.

## What it should look like

Create an application-owned GraphQL fixture that models:

- buffer worldlines;
- rope-like heads and leaves;
- blob-backed fragments;
- text windows;
- ticks and receipts;
- checkpoints;
- braids and braid members;
- inverse policies and obstructions.

The exact directive names may change. The fixture should remain an application
contract compiled by Wesley and hosted by Echo.

## Acceptance criteria

- `createBufferWorldline`, `replaceRangeAsTick`, `createBraid`,
  `appendBraidEdit`, `unapplyTick`, and `unapplyTickSequence` all enter through
  `dispatch_intent`.
- `textWindow`, `braidProjection`, and `tickReceipt` enter through
  QueryView/Query observations.
- Large-file fixture returns only the requested aperture.
- Typing `hello` then unapplying the third tick yields `helo` by appending an
  inverse tick.
- Sequential braid edits produce ordered members and projection digests.
- Missing inverse fragment returns typed obstruction.
- Echo core contains no jedit, rope, buffer, or editor APIs outside generated
  fixture payloads.

## Non-goals

- Do not build the jedit product UI.
- Do not make the fixture a privileged Echo ontology.
- Do not implement Graft automation in Echo core.
