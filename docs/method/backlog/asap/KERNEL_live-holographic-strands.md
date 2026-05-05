<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Live holographic strands

Status: complete. The first live-basis strand slice is implemented across
strand reports, settlement planning, observation/read artifacts, and the
normative strand invariant.

## Completion evidence

- `Strand::live_basis_report(...)` reports parent basis movement, owned
  divergence footprint, and revalidation posture.
- Settlement planning imports disjoint parent movement, revalidates clean
  overlap, and records `ParentFootprintOverlap` conflict residue when parent
  movement changes owned state.
- Observation artifacts expose `StrandAtAnchor`,
  `StrandParentAdvancedDisjoint`, and `StrandRevalidationRequired` basis
  postures with ABI evidence.
- `docs/invariants/STRAND-CONTRACT.md` now defines strands by live-basis
  semantics rather than treating prefix-copy child worldlines as the ontology.

Depends on:

- [0004 — Strand contract](../../../design/0004-strand-contract/design.md)
- [0008 — Strand settlement](../../../design/0008-strand-settlement/design.md)
- [0010 — Live-basis settlement correction plan](../../../design/0010-live-basis-settlement-plan/design.md)

## Why now

Echo's current strand cut is still a bootstrap strand:

- fork a child worldline at one exact tick
- pin that copied prefix forever
- tick the child manually
- hard-delete the whole speculative lane on drop

That got honest substrate nouns into the repo, but it is no longer
the right theory target. Paper VII now says a strand is a real
speculative lane whose realised state is resolved against inherited
parent history at the chosen basis. Bounded reads should materialize
the backward causal cone required by the local divergence and optic
footprint, not a fully copied child world.

Echo needs to stop hardening the bootstrap cut into ontology.

## What it should look like

- A strand is rooted at a parent worldline plus an anchor coordinate,
  but it follows the parent live for untouched regions.
- Local divergence owns only the closed optic footprint required for
  lawful speculative change.
- Materialization is holographic:
    - resolve inherited parent history at a chosen basis
    - overlay only the strand-owned divergence
    - slice only the backward causal cone needed for the read
- Parent changes outside the owned footprint flow through.
- Parent changes overlapping the owned footprint force revalidation,
  explicit conflict, or obstruction. No fake cleanliness.
- `support_pins` remain a comparison/braid aid, not an excuse to
  collapse plurality early.
- The implementation may still use child-worldline machinery
  internally, but the public/runtime semantics must be
  live-following strand semantics, not frozen-fork semantics.
- Dropping a strand should drop the live handle and caches. It should
  not require the theory to pretend the speculative lane was never
  real.

## Current implementation slice

The first slice is now deliberately smaller than the full target:

- `Strand::live_basis_report(...)` reports parent basis movement, child owned
  footprint, and revalidation state.
- settlement compare/plan carries the basis report internally.
- disjoint parent movement is detected separately from owned-footprint overlap.
- disjoint parent movement now plans a target-local import candidate instead of
  a conflict artifact.
- owned-footprint overlap now runs explicit settlement revalidation:
    - replay already satisfied on the parent basis imports as `Clean`
    - replay failure is `Obstructed`
    - replay that would mutate overlapped parent state is `Conflict` residue
      under `ParentFootprintOverlap`

The runtime settlement path has the first concrete overlap revalidation law,
and observer/read artifacts consume the same posture instead of inventing a
separate reading law. The full decision record and runway live in
[0010 — Live-basis settlement correction plan](../../../design/0010-live-basis-settlement-plan/design.md).

## Done looks like

- `docs/invariants/STRAND-CONTRACT.md` no longer defines a strand as
  merely a prefix-copy child worldline with hard-delete semantics.
- `warp_core::strand` distinguishes:
    - parent anchor
    - owned local divergence
    - revalidation state
    - realised basis
- one bounded materialization path proves:
    - untouched parent regions follow live truth
    - owned regions stay local
    - overlap with parent change yields explicit revalidation or
      conflict
- one comparison path proves braid/settlement reads are basis-relative
  presentations over plural lane claims, not fake merge previews.

## Repo evidence

- `crates/warp-core/src/strand.rs`
- `docs/invariants/STRAND-CONTRACT.md`
- `docs/design/0004-strand-contract/design.md`
- `docs/design/0008-strand-settlement/design.md`
- `docs/design/0010-live-basis-settlement-plan/design.md`

## Non-goals

- Do not design final multi-party braid collapse in this item.
- Do not require durable strand persistence in the first slice.
- Do not throw away child-worldline machinery if it remains useful as
  a realization detail.
