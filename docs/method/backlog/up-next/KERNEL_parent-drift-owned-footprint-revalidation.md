<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Parent drift and owned-footprint revalidation

Depends on:

- [0010 — Live-basis settlement correction plan](../../../design/0010-live-basis-settlement-plan/design.md)

## Why now

The strand correction path now says the right high-level thing:

- a strand follows inherited parent history for untouched regions
- the strand owns only its actual local divergence footprint
- overlap between parent change and owned local regions should not be silently
  smoothed over

What is still too implicit is the exact law for parent drift crossing the owned
footprint boundary.

If Echo leaves this fuzzy, the runtime can accidentally drift back toward one of
two bad outcomes:

- fake cleanliness, where parent movement is treated as harmless when it is not
- fake isolation, where the strand behaves like a frozen fork again

## What it should look like

- the runtime can distinguish parent movement outside the owned footprint from
  parent movement inside it
- parent movement outside the owned footprint flows through normally
- parent movement inside the owned footprint forces explicit revalidation
- revalidation can resolve to:
    - still valid
    - obstructed
    - explicit conflict
- the revalidation state is inspectable and not just an internal retry loop

## Current implementation consequence

The runtime can now distinguish the two parent-drift classes, and settlement
has the first explicit overlap revalidation law:

- parent movement outside the owned footprint plans a target-local import
  candidate
- parent movement inside the owned footprint is checked against the current
  target frontier
- overlapped replay that is already satisfied imports as `Clean`
- overlapped replay that fails to apply is `Obstructed`
- overlapped replay that would mutate target state remains
  `ParentFootprintOverlap` residue with `Conflict` revalidation metadata

Owned-footprint overlap still needs the same posture threaded into observer and
bounded-read artifacts. The current tests cover no-overlap, disjoint parent
advance, clean overlap, and conflicting overlap; an obstruction-specific fixture
should be added when a natural patch-level obstruction case is available.

## Done looks like

- one strand/runtime packet states the revalidation law explicitly
- the runtime has one inspectable state or artifact for overlap-driven
  revalidation
- tests prove the settlement cases:
    - no overlap
    - overlap but still valid
    - overlap causing conflict
- observer/read artifacts consume the same revalidation law
- public semantics stop implying that live-following strands are just magical
  overlays with no parent-drift law

## Repo evidence

- `docs/architecture/WARP_DRIFT.md`
- `docs/design/0004-strand-contract/design.md`
- `docs/design/0008-strand-settlement/design.md`
- `docs/design/0010-live-basis-settlement-plan/design.md`
- `crates/warp-core/src/strand.rs`
