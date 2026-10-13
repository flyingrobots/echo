<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Participant History as a Braid

- **Status:** Accepted design. **Not implemented.** The section
  [Current implementation](#current-implementation) states what the runtime
  does today.
- **Decision date:** 2026-10-13
- **Refines:** [Continuum Transport](continuum-transport.md),
  [Strands and Braids](../topics/StrandsAndBraids.md),
  [There Is No Graph](there-is-no-graph.md)
- **Related:** [Strand contract invariant](../invariants/STRAND-CONTRACT.md),
  [Sliced readings and copy-on-write strands](sliced-readings-and-copy-on-write-strands.md),
  [Kitten sessions and delegated authority](../topics/KittenSessionsAndDelegatedAuthority.md)

## Decision

History that arrives from another participant is held as a strand in a braid.
It is not merged into local history on arrival.

Where that history and local history touch the same site, the result is a
recorded conflict. A conflict must be settled before anything acts on the site
it covers. Work on every other site continues, and a braid may stay plural
elsewhere for as long as its law allows.

[Continuum Transport](continuum-transport.md) states what crosses the boundary
and how Echo classifies an import. This page states how Echo holds what
arrived, and what an unsettled conflict stops.

## Terms

- A **participant** is anything that takes an active part in witnessed
  history: a person, an agent, an application, a service, a debugger or a
  policy governor. The cross-project name is a _Kitten_. Participants are
  sovereign. One is not a copy of another, so this page does not say
  "replica".
- The **receiving authority** is the participant that judges an incoming
  history under its own law.
- A **footprint** is what one tick consulted and changed. Its **use set** is
  everything whose stability the tick depended on.
- A **site** is the bounded set of slots where two histories touch.
- A **settlement** is a recorded act of the receiving authority that decides
  the result at a site.
- Every admission ends in one of four outcomes: **Derived**, **Plural**,
  **Conflict** or **Obstruction**. Each is lawful and witnessed. Plural is a
  result, not a failure.

## The model

1. A participant sends witnessed history, never state. The unit is the causal
   suffix bundle of [Continuum Transport](continuum-transport.md).
2. The receiving authority weaves the incoming history into a braid as a
   member strand. Nothing local changes at this step.
3. The receiving authority finds the common frontier: the latest history both
   sides can justify.
4. It carries each incoming tick across its own history after that frontier,
   in its own deterministic order.
5. If the incoming tick and a local tick are independent, the incoming tick
   may move across. Independent footprints make that possible; they do not
   prove it. The governing law and the evidence must also show that the tick
   still means what its author meant on the new basis. Only then is the
   outcome Derived.
6. If they interfere, the outcome at that site is Conflict, or Plural when the
   law allows both claims to stand.
7. If the evidence or the authority needed to judge is missing, the outcome is
   Obstruction.

Two ticks **interfere** when the writes or deletes of one touch the use set of
the other. This is stricter than two writes to one slot. A write to a slot
that the other tick only read is also interference.

## The site rule

A conflict covers a site. Until that site is settled:

- the receiving authority admits no tick whose use set touches it;
- it carries no later incoming tick across it; and
- it refuses both with an obstruction that names the conflict.

The unsettled site grows by dependence. If a later incoming tick is refused
because it uses the site, its own writes join the site. A tick that would have
read those writes cannot be judged either.

Everything outside the site is unaffected. Independent incoming ticks continue
to arrive as Derived, and local ticks on other sites continue to commit.

Plural follows the same rule with a different release. A site that holds
plural claims is not a single value, so a tick that needs a single value there
is refused. A tick whose law accepts plurality may still act.

## Settlement

- Only the receiving authority settles what it receives, and under its own
  law. A sender keeps its own authority over its own history. It cannot
  decide the outcome for another participant.
- A settlement names its bases, its participants, the site and the conflict
  it answers. It is new witnessed history, and it releases the site.
- A settlement may end in a derived value, in retained plurality, or in an
  obstruction that stays on record.
- Plural needs no later resolution. Nothing queues it for collapse.
- An obstruction names the evidence or the authority that was missing, so a
  later crossing can supply it.
- Collapsing a braid records that plural claims became one, and which law
  decided it.
- Nothing is settled by arrival order, by host time, or by a hash of visible
  state.

## Laws

1. **Arrival changes nothing.** Weaving a strand into a braid does not change
   what any local reading returns.
2. **No silent winner.** Interference always leaves a recorded outcome. No
   claim is dropped or overwritten without one.
3. **Conflict is scoped.** An unsettled conflict stops work on its site and
   nowhere else.
4. **The site is closed under dependence.** Nothing that depends on an
   unsettled site is admitted as if the site were settled.
5. **Settlement is an act.** It is admitted like any other topology-changing
   operation, on explicit bases, and it leaves a receipt.
6. **Determinism.** Two receiving authorities with the same local history, the
   same law and the same incoming bundle reach the same outcomes and the same
   unsettled sites.
7. **Footprints must be trusted.** Independence is judged from footprints. A
   footprint that omits something the tick used makes the judgement unsound,
   so an incoming tick whose footprint cannot be trusted is obstructed.
8. **Carriage grants no authority.** Whatever delivers the history, and
   whoever sent it, gains no right to settle. Delivery of the bytes,
   admission of the strand and the outcome at a site are three separate
   facts, and each is reported as itself.

## Alternatives not chosen

- **Every conflict blocks the whole worldline until it is resolved.** This is
  simpler to reason about. One disagreement about one entity would then stop a
  participant completely.
- **Automatic resolution by order or time.** A last-writer-wins rule erases a
  claim without a witness, and host time is not part of history.
- **Exchange of state.** Two participants can reach the same visible state
  with different provenance and intent. State alone cannot show a conflict
  that was erased.

## Open decisions

- Which footprint is trusted for independence: the one a contract declares,
  the one execution observed, or both. Issue #716 retains observed footprints.
- Whether a slot's reads and writes are enough, or whether deletes and
  identity anchors must be recorded separately, as the use set requires.
- The default plural law. Today plurality is refused unless a policy allows
  it.
- How a tick's law states that it accepts a plural site.
- Whether a settlement may be delegated, and what evidence the delegation
  leaves.

## Sources

- James Ross, _Observer Geometry II: Distributed Observer Fields_, version
  0.9.0-rc.1, DOI `10.5281/zenodo.19873664`. It defines the witness-bearing
  update, the common frontier, the footprint and use set, the interference
  relation and suffix transport. It says "replica" where this page says
  "participant".
- James Ross, _History Over State: A Reader's Constitution for Profunctor
  Optics_ (2026). It defines the Kitten, the receiving authority's sole right
  to settle, and the four-outcome algebra.

Both are cited as the origin of the vocabulary and the rules. Neither is
evidence that Echo implements them.

## Current implementation

The runtime does not implement this design. It implements the local half.

- `SettlementService` compares, plans and settles a strand into its base
  worldline inside one runtime. A plan walks the strand's suffix in order and
  gives each entry one decision: import candidate, conflict artifact or
  plural alternative. Settlement appends those decisions to provenance and
  can return a braid shell.
- **Blocking is by suffix, not by site.** After the first entry that ends in
  conflict or plurality, every later entry of that suffix is recorded as a
  conflict, whether or not it touches the same slots. The site rule above is
  narrower.
- **An unsettled conflict stops nothing.** A recorded conflict artifact does
  not prevent later ticks on the slots it names. No index of unsettled sites
  exists, and the scheduler does not consult one.
- Overlap is judged from the `in_slots` and `out_slots` of tick patches
  against the strand's owned closed footprint (INV-S10). Deletes and identity
  anchors are not recorded separately.
- `PluralSettlementPolicy` refuses plurality by default and can allow it over
  footprint overlap. `AdmissionOutcomeKind` carries the four outcomes.
- `export_suffix` and `import_suffix` build and classify a causal suffix
  bundle as admitted, staged, plural, conflict or obstructed. They do not
  append provenance or apply patches, and the module states that it is not
  transport, remote synchronization or import execution.
- Braid events record creation, member weaving, settlement and collapse. No
  code weaves a strand that came from another participant.
- Within one tick, an executable-operation Action whose footprint conflicts
  with another's is refused with a typed footprint-conflict outcome.
- Planning a settlement applies each entry to a copy of the whole base state
  and hashes the whole state. Executing one copies the runtime for rollback.
  Both costs grow with the graph.
- Echo has no network transport. Continuum is the planned carrier.

Implementation state is owned by
[issue #798](https://github.com/flyingrobots/echo/issues/798), not by this
page.
