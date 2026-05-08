<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract Strands And Counterfactuals

Folded from: #245

Status: planned kernel/runtime implementation.

Depends on:

- [Graft live frontier structural readings](./PLATFORM_graft-live-frontier-structural-readings.md)
- [0010 - Live-basis settlement correction plan](../../../design/0010-live-basis-settlement-plan/design.md)

## Why now

After Echo can host contract intents and readings, speculative work should use
generic strands over contract families rather than application-specific branch
APIs.

`jedit` can use this for refactor previews and agent containment. Graft can use
it for structural impact prediction. Echo should only provide the substrate:
strand basis, local divergence, revalidation, admission, conflict, obstruction,
plurality, and retained readings.

## What it should look like

Add generic contract-aware strand operations:

- create strand over contract basis
- dispatch contract intent into strand
- observe contract reading from strand
- compare strand with parent basis
- admit, preserve plurality, conflict, obstruct, or discard

## Merge and settlement semantics

The old TT1 framing asked about merging debugger-era per-source admission
records across worldlines. The current Echo substrate should not preserve that
as a special ontology. The useful requirement belongs here:

- divergent work is represented as worldline/strand/braid history
- each member has an explicit causal basis and actor/cause evidence
- settlement is an Intent/admission operation, not a direct service mutation
- independent work may admit, stage, preserve plurality, conflict, or obstruct
- conflicts are typed by contract law and witness basis, not by host-time order
- adapters such as `warp-ttd` can explain these outcomes, but Echo owns the
  substrate decision and receipt evidence

## Acceptance criteria

- A fake contract intent can be applied inside a strand without changing the
  parent frontier.
- Observing the strand returns a reading envelope that names strand basis and
  contract identity.
- Parent movement outside owned divergence revalidates cleanly.
- Parent overlap returns explicit conflict or obstruction.
- Settlement of divergent contract work produces a typed admission posture and
  receipt/witness evidence.
- `jedit` and Graft examples remain consumer-level, not Echo core APIs.

## Non-goals

- Do not implement final multi-party braid collapse.
- Do not implement semantic refactor prediction here.
- Do not add text or Graft domain types to Echo core.
- Do not require durable strand persistence in the first slice.
- Do not model historical stream/debugger frame names as Echo core nouns.
