<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0007 — Braid geometry and neighborhood publication

_Make Echo strands capable of read-only braid geometry and publish one honest
local site object for Continuum / `warp-ttd` consumption._

Legend: KERNEL

Depends on:

- [0004 — Strand contract](./0004-strand-contract.md)
- [0005 — Echo TTD witness surface](./0005-echo-ttd-witness-surface.md)
- [0006 — Echo Continuum alignment](./0006-echo-continuum-alignment.md)
- external Continuum packets:
    - `0001` through `0015`
    - `OVERVIEW`

## Why this cycle exists

The bootstrap strand contract in `0004` was intentionally narrow:

- strands are first-class speculative lanes
- source-basis provenance is explicit
- advancement happens only through ordinary ingress + `super_tick()`
- support pins exist structurally, but MUST be empty in bootstrap

That was the right first cut, but it is not enough for parity with
`git-warp`, and it is not enough for honest Continuum publication.

Echo still lacks two things:

1. **real braid geometry**
    - a strand can name another lane as read-only support at an exact
      coordinate
2. **native neighborhood publication**
    - the kernel can publish which lanes actually participate in the local
      site being observed, without making adapters reconstruct that shape from
      scattered runtime objects

This cycle therefore defines the first non-empty support-pin contract and the
first honest local-site publication boundary.

Settlement remains separate. This packet is about local plural read geometry,
not compare/import/conflict law. That follow-on is now
[0008 — Strand settlement and conflict artifacts](./0008-strand-settlement.md).

## Design decision

Echo should align with Continuum braid theory by making two publication
surfaces first-class, while keeping the authoritative runtime-control
ontology anchored in
[0009 — Strand Runtime Graph Ontology](./0009-strand-runtime-graph-ontology.md):

1. **Support pins become real, read-only braid geometry inputs on strands.**
2. **Observation publishes a first-class local site object whose participants
   come from the observed lane, its source-basis relation, and its declared support
   pins.**

The resulting publication must stay intentionally narrow:

- it need not enumerate every nearby speculative alternative in the whole
  runtime
- it does need to publish the actual participating lanes for the observed site
- it must distinguish singleton from plural sites honestly

This is the missing leg between bootstrap strands and a Continuum-aligned
`NeighborhoodCore`.

## Human users / jobs / hills

### Primary human users

- kernel contributors implementing strand and observation runtime truth
- debugger contributors making `warp-ttd` speak the same neighborhood nouns
  across Echo and `git-warp`
- advanced users exploring plural local sites without flattening them into one
  fake line

### Human jobs

1. Declare a strand that reads through support overlays without mutating those
   support lanes.
2. Observe one coordinate and know which lanes actually participate in the
   local site.
3. Publish that site to a debugger without adapter folklore.

### Human hill

A human can inspect one Echo coordinate and see whether it is a singleton or a
plural site, which lanes participate, and why those lanes are in scope.

## Agent users / jobs / hills

### Primary agent users

- agents implementing Echo host bridges and Continuum/Wesley proof slices
- agents auditing Echo for parity with `git-warp`

### Agent jobs

1. Determine whether an observed site is singleton or braided.
2. Read the participating lane set without reconstructing it from multiple
   unrelated runtime objects.
3. Map Echo publication into Continuum `NeighborhoodCore` without guessing.

### Agent hill

An agent can consume one Echo neighborhood publication object and derive the
same top-level debugger nouns that `warp-ttd` expects from any host.

## Core distinction

Echo must keep three things separate:

- **strand**
    - a speculative lane relation over a child worldline
- **braid geometry**
    - read-only local plurality declared by support pins
- **settlement**
    - compare/import/conflict law over lane history

Braid is geometry. Settlement is history law. Neither should be collapsed into
the other.

## First non-empty support-pin contract

Support pins should graduate from placeholder to first-class read geometry with
the following contract.

### Support pin semantics

A support pin is a read-only, exact-coordinate reference from one live strand
to another live strand's child worldline.

It means:

- "when reading this strand's local site, also include that support lane at
  this exact pinned coordinate"

It does **not** mean:

- copy or import support history
- share writer heads
- make the support lane runnable
- settle or merge anything
- create a new worldline

### Support-pin invariants

In addition to the existing `SupportPin` fields in `strand.rs`, the first
non-empty contract should enforce:

- **BG-1 (Live target):** the pinned `strand_id` must exist in
  `StrandRegistry`.
- **BG-2 (Worldline agreement):** `worldline_id` must equal the target
  strand's `child_worldline_id`.
- **BG-3 (Pinned coordinate exists):** `pinned_tick` must exist in the target
  worldline provenance.
- **BG-4 (Pinned state agreement):** `state_hash` must match the target
  worldline's state root at `pinned_tick`.
- **BG-5 (No self-pin):** a strand must not support-pin itself.
- **BG-6 (Quantum agreement):** the primary strand and support strand must
  share the same `tick_quantum`.
- **BG-7 (No duplicate support target):** one strand must not carry multiple
  support pins to the same target strand in the same publication slice.
- **BG-8 (Read-only support):** support pins do not authorize writes through
  the support lane.

### Support-pin mutation surface

The first useful mutation surface is explicit and small:

- `pin_support(strand_id, support_strand_id, pinned_tick)`
- `unpin_support(strand_id, support_strand_id)`
- `list_support_pins(strand_id)`

`pin_support(...)` resolves and stores the exact target worldline ID and state
hash at the pinned tick. Pinning is therefore deterministic and replayable.

### Drop posture

Bootstrap braid geometry should reject dropping a strand while it is pinned by
another live strand.

That is stricter than automatic cleanup, but it preserves a simple truth:

- a published plural site cannot silently lose one of its declared
  participants because another operation hard-deleted it out from under the
  geometry

If a looser cleanup model is desired later, it should be an explicit follow-on
design, not incidental side effect.

## Local site publication

Echo needs a first-class publication object for the local site being observed.

The exact type name is not yet fixed. In this packet it is called
`NeighborhoodSite`.

`NeighborhoodSite` is not a new worldline and not a universal braid object. It
is a bounded publication object for one observed coordinate.

### Minimum `NeighborhoodSite` contract

```text
NeighborhoodSite {
    site_id:        NeighborhoodSiteId,
    anchor:         ResolvedObservationCoordinate,
    plurality:      SitePlurality,
    participants:   Vec<SiteParticipant>,
    outcome_kind:   AdmissionOutcomeKind,   // derived from plurality in the first cut
}

SiteParticipant {
    worldline_id:   WorldlineId,
    strand_id:      Option<StrandId>,
    role:           ParticipantRole,
    tick:           WorldlineTick,
    state_hash:     Hash,
}
```

Where:

- `ParticipantRole::Primary`
    - the observed lane at the observed coordinate
- `ParticipantRole::BasisAnchor`
    - the source lane/coordinate from `ForkBasisRef` when the primary lane is a strand
- `ParticipantRole::Support`
    - a read-only support-pinned lane

And:

- `SitePlurality::Singleton`
    - only the primary lane participates
- `SitePlurality::Braided`
    - one or more additional participants are present

At the shared lawful-outcome layer, the first cut maps:

- `SitePlurality::Singleton` => `Derived`
- `SitePlurality::Braided` => `Plural`

This should remain a derived relation in the first cut. Echo does not need a
separate stored field to tell the same story twice.

### What counts as a participant

The first publication must stay narrow and explicit.

At an observed coordinate, participants are:

1. the observed primary lane
2. the basis anchor, if the primary lane is a strand
3. every declared support pin on that strand, resolved to its pinned
   coordinate

This is enough to publish real local plurality without pretending Echo already
has a perfect global neighborhood search or full nearby-alternative catalog.

### Site identity

`site_id` should be stable over:

- the primary lane identity
- the observed coordinate
- the declared participant set and their roles

The adapter may still format a transport-specific string or opaque ID, but the
site identity must come from kernel-truth inputs, not UI heuristics.

## Observation integration

`ObservationArtifact` should remain the record of:

- resolved coordinate
- frame
- projection
- payload
- deterministic artifact hash

It should **not** be overloaded to carry every neighborhood concern inline.

Instead, Echo should add a neighboring publication path:

- observe frame/projection → `ObservationArtifact`
- observe local site at coordinate → `NeighborhoodSite`

Those two objects are related, but not the same.

This keeps the layering clean:

- observation artifact = what was read
- neighborhood site = which lanes define the local site around that read

## Kernel truth vs adapter summary

### Kernel runtime truth

Echo kernel/runtime should own:

- strand -> child-worldline truth
- fork-basis truth
- authorised head truth
- support-pin declarations or cache records when enabled
- participant roles
- pinned coordinates and state hashes
- site plurality inputs
- stable site identity inputs

`NeighborhoodSite` itself is a derived publication object in the first
cut. It is computed from authoritative runtime truth; it is not the
authoritative store of that truth.

### Adapter / debugger summary

Adapters such as Echo → `warp-ttd` may still own:

- transport DTO layout
- UI ordering and labels
- debugger-local `frameIndex` mapping
- host-specific shell decoration

But adapters must stop owning the core local-site reconstruction.

If the adapter has to rediscover participants from raw `ObservationArtifact`,
`Strand`, and provenance state, Echo is still not aligned.

## Continuum mapping

This packet is the Echo-side bridge into Continuum's shared observer/debugger
nouns.

The intended mapping is:

- `NeighborhoodSite.anchor` → shared coordinate/frame truth
- `NeighborhoodSite.participants` → shared lane participation
- `NeighborhoodSite.plurality` → shared singleton-vs-plural site truth
- `NeighborhoodSite.outcome_kind` → shared lawful outcome family (`Derived` / `Plural`)
- support/basis roles → Continuum neighborhood semantics

This is the first real Echo-side grounding for a shared `NeighborhoodCore`.

It does **not** yet solve:

- reintegration detail
- settlement
- observer trace
- global nearby-alternative enumeration

Those remain separate layers.

## What this cycle does not do

- define settlement/import/conflict law
- define a full global braid graph
- define all nearby alternatives around a site
- make support lanes writable
- flatten plural sites into synthetic worldlines

## Immediate implementation consequences

1. `StrandRegistry` and strand runtime APIs must allow non-empty
   `support_pins` under the new invariants, while keeping them
   explicitly non-authoritative in the first cut.
2. Echo must track reverse pin usage well enough to reject dropping a pinned
   support strand.
3. Observation/publication code must expose a native `NeighborhoodSite`
   boundary.
4. Echo's `warp-ttd` host bridge should map `NeighborhoodSite` directly into
   shared neighborhood publication instead of reconstructing participants
   indirectly.
5. `KERNEL_strand-settlement` should build on this geometry, not redefine it.

## Open questions

1. Should support pins remain strand-to-strand only, or eventually admit
   canonical-worldline support references too?
2. Does `NeighborhoodSite` belong beside `ObservationArtifact`, or should both
   be grouped under a higher observation/publication envelope?
3. Is `BasisAnchor` always a participant in the published site, or can some
   observers request a reduced publication that omits provenance-only lanes?

## Decision

Echo should not wait for a perfect global braid model before publishing plural
local sites.

It should:

- make support pins real as read-only braid geometry inputs
- publish one native `NeighborhoodSite` object for the observed local site
- keep settlement separate
- let adapters consume that derived publication instead of inventing it

That is the smallest honest step from bootstrap strands toward parity with
`git-warp` and toward shared Continuum debugger nouns.
