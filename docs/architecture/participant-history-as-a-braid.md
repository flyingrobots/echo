<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Participant History as a Braid

- **Status:** Accepted design. **Not implemented**, and not yet precise enough
  to implement without the decisions listed under
  [Open decisions](#open-decisions). The section
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

Where that history and local history interfere, the receiving authority
records an outcome at the site they share: Conflict, or Plural when its law
lets both claims stand. Until a conflict is settled, nothing that depends on
it is admitted. Work that does not depend on it continues.

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
- A **footprint** is what one tick consulted and changed: its reads, writes,
  deletes and the identities it needs to stay fixed. Its **use set** is its
  reads, its writes and those identities: everything whose stability the tick
  depended on. A write target is in the use set.
- A **site** is the bounded set of slots where two histories interfere.
- A **barrier** is the admission rule that an unsettled site imposes.
- A **settlement** is a recorded act of the receiving authority that answers
  a conflict.
- Every judgement ends in one of four outcomes: **Derived**, **Plural**,
  **Conflict** or **Obstruction**. Each is lawful and witnessed. Plural is a
  result, not a failure.
- **Staged** is not a fifth outcome. It is the posture of history that is
  held and not yet judged. It comes before the four outcomes and is reported
  as itself, never as Obstruction.

## Three separate steps

Holding, judging and applying are different acts. An implementation may run
them together. Their meanings stay distinct.

1. **Hold.** The receiving authority weaves the incoming history into a braid
   as a member strand. Braid membership changes. No reading of application
   state changes, and nothing becomes inadmissible. History that is held and
   not yet judged is Staged. Held history and its evidence are retained for
   as long as a claim or a conflict depends on them. They do not end with the
   session that received them.
2. **Judge.** The receiving authority carries each incoming tick across its
   own history and records an outcome. Judging changes no reading of
   application state. It can raise a barrier.
3. **Apply.** A Derived result becomes part of the receiving worldline only
   through a witnessed commit on an explicit basis. Readings of application
   state change at this step and at no other.

## Judging

1. A participant sends witnessed history, never state. The unit is the causal
   suffix bundle of [Continuum Transport](continuum-transport.md).
2. The receiving authority finds the common frontier: the set of latest
   events that both sides can justify. It is a set, because two histories can
   share several points that are not ordered against each other. If the
   receiver cannot justify any common basis it trusts, the outcome is
   Obstruction.
3. It carries each incoming tick across its own history after that frontier,
   in its own deterministic order.
4. If the incoming tick interferes with no local tick it is carried across,
   it may move across. Independent footprints make that possible; they do not
   prove it. The governing law and the evidence must also show that the tick
   still means what its author meant on the new basis. Only then is the
   outcome Derived.
5. If it interferes with any of them, the outcome at that site is Conflict, or
   Plural when the law lets both claims stand. A tick is never both Derived
   and in conflict.
6. If the evidence or the authority needed to judge is missing, the outcome is
   Obstruction. It names what was missing.

Two ticks **interfere** when the writes or deletes of one meet the use set of
the other, in either direction. Two writes to one slot interfere, because a
write target is in the use set. A write to a slot that the other tick only
read also interferes.

A footprint must cover what the tick depended on, including what it found
absent. A tick that acted because a query matched nothing depends on that
absence. So does one that depended on the bounds of a range or on membership
of a collection.

## The barrier

An unsettled conflict covers a site and raises a barrier there. The barrier
exists from the moment the conflict is judged. Holding a tick raises none;
judging it as Conflict raises one at once.

**What the barrier refuses.** A tick is not admitted if its use set or its
deletes meet the site. The refusal is an obstruction that names the conflict.

**Settlement is not refused.** The act that answers the conflict has to
address the site. It passes the barrier by a narrow path of its own: it names
the conflict, its basis and its authority, and it is checked against changes
made since it was planned.

**What happens to later incoming ticks.** An incoming tick that depends on a
blocked incoming tick is held and not judged. It stays Staged. Its dependence must be shown by
validated causal evidence in the held strand, not inferred from a footprint
that a sender merely claimed.

**What a held tick does not block.** A held tick has produced no writes. Its
intended writes raise no barrier against local work or against unrelated
incoming work. They are potential outputs of blocked work, and the receiving
authority may report them as such. When the conflict is settled, each held
tick is judged then, against the history of that moment. If it interferes
with something admitted in the meantime, that is a new outcome at a new site.

This matters for safety. If intended writes raised barriers, a participant
could send one tick that reads a disputed slot and claims to write everywhere,
and so stop the receiver completely.

**What stays admissible.** Work whose validated dependencies do not meet an
unsettled site stays admissible. This is a conditional promise. A dependency
that is truly global can lawfully block global work. The receiving authority
must then be able to show why the blockage is that wide.

Plural raises a different barrier. A site that holds plural claims is not a
single value. A tick that needs a single value there is refused. A tick whose
law accepts that plurality, and whose effects are safe under each claim, may
act.

## Settlement

- Only the receiving authority settles what it receives, and under its own
  law. A sender keeps its own authority over its own history. It cannot
  decide the outcome for another participant.
- A settlement names its bases, its participants, the site and the conflict
  it answers. It is new witnessed history. It changes neither participant's
  past: each author's claim stays on record as that author made it.
- A settlement is planned against a basis. If that basis has moved when the
  settlement is committed, the commit is refused with an obstruction that
  records the stale basis as its reason. A new plan is a new settlement and
  needs its own authorization. An authorized settlement is never altered to
  fit a basis it was not planned on.
- Dependent work never resumes on its old readings. It is judged again.
- Plural needs no later resolution. Nothing queues it for collapse.
- Collapsing a braid records that plural claims became one, and which law
  decided it.
- Nothing is settled by arrival order, by host time, or by a hash of visible
  state.

What a settlement does to the barrier depends on its result:

| Result               | Admission afterwards                                                                  |
| -------------------- | ------------------------------------------------------------------------------------- |
| Derived value        | The barrier is lifted. Dependent work is judged again, against the new basis.         |
| Retained plurality   | Only work whose law accepts that plurality is admitted.                               |
| Conflict             | The barrier stays, on the site the new conflict names. A later settlement answers it. |
| Retained obstruction | The barrier stays. The obstruction names the conditions under which it can lift.      |

## Laws

1. **Holding changes no application reading.** Weaving a strand into a braid
   changes braid membership and nothing else.
2. **No silent winner.** Interference always leaves a recorded outcome. No
   claim is dropped or overwritten without one.
3. **Every admitted effect has a justified basis.** That basis includes the
   absence of unsettled dependencies, or their explicit acceptance under a
   law that allows it.
4. **A barrier is scoped by validated dependence.** It refuses work that
   depends on its site, and it must be able to say why.
5. **Only admitted history enlarges a barrier.** A submission that was
   refused or is held enlarges none.
6. **Settlement is an act.** It is admitted on explicit bases through its own
   path, and it leaves a receipt.
7. **Determinism.** Two receiving authorities reach the same outcomes and the
   same barriers when all of these are equal: their local history, including
   braid members and unsettled outcomes; the incoming bundle; the law, by
   exact identity and version; the authority and delegation evidence they
   hold; and every budget or limit that can change a classification.
8. **Footprints must be trusted.** A footprint that omits something the tick
   used makes the judgement unsound, so an incoming tick whose footprint
   cannot be trusted is obstructed. Authentic causal history does not prove
   a footprint complete. Completeness is checked separately.
9. **Carriage grants no authority.** Whatever delivers the history, and
   whoever sent it, gains no right to settle. Delivery of the bytes,
   admission of the strand and the outcome at a site are three separate
   facts, and each is reported as itself.

## A worked trace

Participant A holds local history. Participant B sends a strand of three
ticks, made on a basis both share.

| Step | Event                                           | Result at A                                                            |
| ---- | ----------------------------------------------- | ---------------------------------------------------------------------- |
| 1    | B's strand arrives                              | Held. Braid membership changes. No application reading changes.        |
| 2    | B1 writes slot X. A already wrote X locally     | Conflict at site {X}. A barrier is raised on X.                        |
| 3    | B2 writes slot Y. Nothing at A touches Y        | Derived. B2 is applied by a witnessed commit.                          |
| 4    | B3 read X as B1 left it, and writes slot Z      | Held. Its dependence on B1 is in the strand's own evidence.            |
| 5    | A's next local tick writes Z                    | Admitted. B3's intended write to Z raises no barrier.                  |
| 6    | A's next local tick reads X                     | Refused with an obstruction that names the conflict.                   |
| 7    | A settles the conflict to a derived value of X  | The barrier on X is lifted. The settlement is new history.             |
| 8    | B3 is judged now                                | It meets A's write to Z from step 5. A new outcome is recorded at {Z}. |
| 9    | The refused tick from step 6 is submitted again | Judged against the new basis, and admitted if nothing else stops it.   |

## Alternatives not chosen

- **Every conflict blocks the whole worldline until it is resolved.** This is
  simpler to reason about. One disagreement about one entity would then stop a
  participant completely.
- **Intended writes of held ticks raise barriers.** This looks like the safe
  reading of dependence. It lets a sender widen a receiver's barriers at will.
- **Automatic resolution by order or time.** A last-writer-wins rule erases a
  claim without a witness, and host time is not part of history.
- **Exchange of state.** Two participants can reach the same visible state
  with different provenance and intent. State alone cannot show a conflict
  that was erased.

## Open decisions

These must be settled before the design can be implemented.

- **Footprint trust as a proof obligation.** A contract's declared bounds,
  the accesses that execution observed, and the receiver's own verification
  do different jobs. The rule that combines them is not written. Issue #716
  retains observed footprints.
- **What a footprint must record.** Whether slot reads and writes are enough,
  or whether deletes, identity anchors, absence, ranges and collection
  membership need their own entries.
- **A set-valued basis.** The common frontier can be several unordered
  events. The transport artifact must be able to name all of them.
- **Overlapping conflicts.** Whether the sites of two conflicts combine or
  keep separate identities, and whether one can be settled without the other.
- **Partial settlement.** Whether part of a site can be settled when the law
  proves that part independent.
- **A durable carrier for held history.** A strand is scoped to a session
  today. Held history needs a retained carrier that outlives the session, or
  the strand lifecycle must be refined.
- **Outcomes for part of a bundle.** One bundle can hold ticks that end
  Derived, in Conflict and Staged. The boundary needs an artifact that
  reports an outcome for each.
- **Retries with new evidence.** An exact repeat of a bundle is the same
  question again: it is recognized by the bundle's identity, it weaves no
  second member, and it returns the retained outcome.
  [Continuum Transport](continuum-transport.md) already requires that. Open
  is how newly supplied evidence is linked to the obstruction it answers.
- **Accepting plurality.** How a tick's law states that it accepts a plural
  site, and how its effects are shown safe under each claim. An effect that
  leaves Echo is not made safe by plurality alone.
- **The default plural law.** Today plurality is refused unless a policy
  allows it.
- **Delegation.** Whether a settlement may be delegated, and what evidence
  the delegation leaves.

## Sources

- James Ross, _Observer Geometry II: Distributed Observer Fields_, version
  0.9.0-rc.1, DOI `10.5281/zenodo.19873664`. It defines the witness-bearing
  update, the common frontier as an antichain, the footprint and use set, the
  interference relation and suffix transport. It says "replica" where this
  page says "participant".
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
- **Blocking is by suffix, not by dependence.** After the first entry that
  ends in conflict or plurality, every later entry of that suffix is recorded
  as a conflict, whether or not it depends on the first.
- **An unsettled conflict raises no barrier.** A recorded conflict artifact
  does not prevent later ticks on the slots it names. No index of unsettled
  sites exists, and the scheduler does not consult settlement results.
- **Holding, judging and applying are not separate.** Planning judges, and
  settling applies and records in one call.
- No act answers a recorded conflict later. A conflict artifact stays as
  residue.
- Overlap is judged from the `in_slots` and `out_slots` of tick patches
  against the strand's owned closed footprint (INV-S10). Deletes, identity
  anchors, absence and ranges are not recorded separately.
- `PluralSettlementPolicy` refuses plurality by default and can allow it over
  footprint overlap. `AdmissionOutcomeKind` carries the four outcomes.
- `export_suffix` and `import_suffix` build and classify a causal suffix
  bundle as admitted, staged, plural, conflict or obstructed. They do not
  append provenance or apply patches, and the module states that it is not
  transport, remote synchronization or import execution.
- A `CausalSuffixBundle` names its base frontier and its target frontier as
  one provenance coordinate each. It cannot carry a common frontier of
  several unordered events.
- An admission response carries one outcome for the whole bundle. It cannot
  report different outcomes for different ticks.
- A strand is scoped to its session (INV-S3), and dropping it removes its
  worldline and provenance (INV-S11). Nothing retains a received strand
  beyond the session.
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
