<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Parent drift and owned-footprint revalidation

Depends on:

- [KERNEL_live-holographic-strands](../asap/KERNEL_live-holographic-strands.md)
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

The runtime can now distinguish the two parent-drift classes, but settlement
still handles overlap conservatively:

- parent movement outside the owned footprint plans a target-local import
  candidate
- parent movement inside the owned footprint maps to `ParentFootprintOverlap`

Owned-footprint overlap still needs an explicit revalidation artifact that can
resolve to clean, obstructed, or conflict.

## Done looks like

- one strand/runtime packet states the revalidation law explicitly
- the runtime has one inspectable state or artifact for overlap-driven
  revalidation
- tests prove all three cases:
    - no overlap
    - overlap but still valid
    - overlap causing obstruction or conflict
- public semantics stop implying that live-following strands are just magical
  overlays with no parent-drift law

## Repo evidence

- `docs/WARP_DRIFT.md`
- `docs/design/0004-strand-contract/design.md`
- `docs/design/0008-strand-settlement/design.md`
- `docs/design/0010-live-basis-settlement-plan/design.md`
- `crates/warp-core/src/strand.rs`
