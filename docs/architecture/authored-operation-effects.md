<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Authored Operation Effects

- **Status:** Proposed design for review. **Not accepted. Not implemented.**
  The section [Current implementation](#current-implementation) states what
  the runtime does today.
- **Proposed:** 2026-10-15, for review in
  [flyingrobots/jedit#302](https://github.com/flyingrobots/jedit/pull/302)
- **Depends on:** [Application operation semantics authored in Edict source](../topics/GeneratedRules.md#application-operation-semantics-authored-in-edict-source)
- **Refines:** [ADR 0023](../adr/0023-admitted-executable-operation-packages.md)
  and [ADR 0025](../adr/0025-scheduler-owned-executable-operation-actions.md)
  for Actions whose program is evaluated Edict rather than a fixed
  single-node profile
- **Related:** [Sliced Readings and Copy-on-Write Strands](sliced-readings-and-copy-on-write-strands.md),
  [Generated Rule Authorship](../topics/GeneratedRules.md),
  [WAL](../topics/WAL.md)

## Decision

An admitted executable operation whose program is compiled Edict may produce a
bounded set of graph effects. Evaluation proposes those effects. Echo witnesses
what the evaluation actually read and what it proposes to write, validates both
against the admitted contract and the basis, and commits all of the effects in
one Tick or none of them.

Three invariants govern the design:

1. **Evaluation is not commitment.** A successful evaluation yields a
   proposal. Nothing it computes is authoritative until the scheduler commits
   the Tick that contains it.
2. **The footprint is witnessed, not claimed.** The evaluator records every
   slot it actually reads. The proposal states every slot it writes. Echo
   checks both against the footprint law the package declares. A declaration
   bounds the footprint; it never substitutes for it.
3. **Multi-node mutation is atomic.** A proposal commits whole within one Tick
   and its decided-Tick WAL record, or it obstructs with no parent-visible
   patch. A failed operation never leaves part of its effect in history.
4. **Nothing references what is not realized.** No committed coordinate
   contains an edge whose endpoint is absent. In particular, a head that a
   proposal retargets is never observable before every node and edge it
   reaches has been validated and committed in the same Tick.

Echo learns no application vocabulary. A proposal is nodes, edges, and atoms.

## Current implementation

- Echo main admits two executable programs: anchored single-node alpha-atom
  compare-and-set and create-if-absent. Both write bytes supplied by the
  invocation (`crates/warp-core/src/echo_operation.rs`).
- `edict_pure` and `edict_read` evaluate compiled Edict values, but no admitted
  path turns their results into writes. `edict_read` reads alpha atoms by
  explicit node key; it cannot follow edges. It counts reads against a limit
  but does not record which slots it read
  (`crates/warp-core/src/edict_read/evaluate.rs`).
- Create-if-absent obstructs when the node already exists, even with identical
  content.
- `GraphStore::upsert_edge_record` does not check that an edge's endpoints
  exist (`crates/warp-core/src/graph.rs`).
- Footprint independence is a set test: any write overlapping another Action's
  read or write conflicts, including two identical writes
  (`crates/warp-core/src/footprint.rs`, `Footprint::independent`).
- A Tick holds at most 128 graph operations.
- The state root is a full walk of the reachable graph on every commit. See the
  open state-root decision in
  [Sliced Readings and Copy-on-Write Strands](sliced-readings-and-copy-on-write-strands.md#open-decisions-that-bound-these-laws).

## Lifecycle

```text
admitted package + invocation at an exact basis
-> scheduler selects the Action while constructing a Tick
-> bounded evaluation over a sliced reading of that basis
     records: every slot read, steps, allocation
     yields:  a canonical EffectProposal, or a typed obstruction
-> validation against the package's footprint law, budget, authority,
   identity law, and the basis
-> composition with other Actions under the existing independence rule
-> one decided Tick: patch, receipt, decided-Tick WAL record
```

Evaluation runs inside Tick construction, as ADR 0025 already requires for
Actions. Proposals are not persisted on their own. Recovery replays decided
Ticks. A pending Action is re-evaluated against its basis after recovery, which
is sound because evaluation is deterministic over a fixed basis.

## EffectProposal

A proposal is a canonical value with a domain-separated identity:

```text
EffectProposal {
  basis                  exact parent coordinate evaluated against
  readSet                slots read: node, alpha atom, outgoing edges by type
  effects [              in canonical order
    EnsureNode   { node, type, atom }      create, or require identical
    CreateNode   { node, type, atom }      require absent
    ReplaceAtom  { node, expectedAtomDigest, atom }
    EnsureEdge   { edge, from, to, type }  create, or require identical
    ReplaceEdge  { edge, expectedTo, to }  retarget an existing edge
  ]
  result                 compiler-declared application result bytes
  cost                   steps, allocated bytes, read bytes, write bytes
}
```

Rules:

- **Effects are the only writes.** The proposal contains no prepared patch.
  Echo derives the patch from the effects.
- **Ensure is idempotent only on identity.** An `EnsureNode` whose node exists
  with identical type and atom adds nothing to the patch and counts as a read.
  An existing node with different content is a typed
  `ContentIdentityConflict` obstruction. The same holds for `EnsureEdge`. This
  is the idempotent create profile that Edict's I-022 requires before
  duplicate content-addressed creation can be lawful.
- **Compare-before-replace.** `ReplaceAtom` and `ReplaceEdge` carry the value
  they expect. A mismatch at commit is a stale-basis obstruction, not a merge.
- **No deletes in this profile.** Persistent structures do not need them.
  Deletion needs its own design.
- **Identity derivation is the program's.** Echo checks that each node and edge
  id matches the identity law the package declares, so ids cannot be chosen to
  collide with unrelated nodes. Echo does not invent ids.

## Witnessed reads over a bounded slice

The program never declares what it read. Evaluation reaches state only through
a recording view over the basis, and the read set is exactly the set of slots
that view resolved: node records, alpha atoms, and outgoing edges by type. A
program has no other path to state, so the record is complete by construction.
This needs new instrumentation; today's `edict_read` only counts reads.

Absence is a dependency too. Every resolution is recorded with its outcome,
against the basis it was resolved at:

| Resolution                             | Recorded dependency                                                    |
| -------------------------------------- | ---------------------------------------------------------------------- |
| node or atom present                   | the slot and a digest of its value                                     |
| node or atom absent                    | the slot, observed absent                                              |
| outgoing edges of one type from a node | the source node and type, with the target set observed, empty included |

An `Ensure` that finds no existing node depends on that absence, so a
co-scheduled Action that creates the slot invalidates it like any other
changed read.

These dependency kinds follow the
[Action validity across ticks](../topics/GeneratedRules.md#action-validity-across-ticks)
decision (#837). Value and single-key absence dependencies are the kinds that
decision supports. A read of outgoing edges by type is a topology dependency,
which that decision does not yet support, so an Action that reads edges keeps
the whole-frontier rule: it commits only onto the exact basis it was evaluated
against. Widening that requires sound edge-set tracking under #837, not a
looser rule here.

The view resolves each slot on demand from the basis, as
[Sliced Readings and Copy-on-Write Strands](sliced-readings-and-copy-on-write-strands.md)
law 1 requires. The contract never depends on a complete materialized graph:
the read limit and the footprint law bound the slice, and reaching past either
obstructs. A first implementation may still sit on today's fully materialized
worldline state, but nothing in this contract may rely on that.

## Footprint law

The package declares a footprint law: the node types, edge types, and maximum
counts an Action may read and write, relative to its invocation. At validation
Echo checks:

- the witnessed read set and the proposal's write set lie inside the law;
- counts do not exceed the declared maxima;
- the derived patch's actual footprint equals the proposal's write set.

Any violation is a typed obstruction with no patch. Independence between
Actions in one Tick uses the witnessed footprints, under the existing rule:
write against read or write conflicts, read against read does not.

Reading a large shared subtree costs footprint entries but never conflicts.

An `Ensure` of a node or edge that already exists identically in the basis is
a read, not a write, so Actions that share existing structure do not conflict.
Two Actions in one Tick that both create the same new content-addressed node
do write the same slot, and the existing rule makes the later one obstruct.
This design adds no merge rule for identical writes. The rejected Action's
outcome is final (see Retries); a new submission evaluated later finds the
node in its basis, so its `Ensure` is a read and does not conflict.

## Visibility and referential integrity

Validation enforces invariant 4 before a proposal can join a Tick:

- every `EnsureEdge` and `ReplaceEdge` endpoint exists in the basis or is
  created by an `EnsureNode` or `CreateNode` in the same proposal;
- every edge a `ReplaceEdge` retargets exists in the basis with the expected
  target.

A violation is a typed `DanglingReference` obstruction with no patch.

Commit then makes partial visibility impossible by construction. All of an
Action's effects belong to one Tick. Observers, readings, and later Actions bind
to committed Tick coordinates, and a Tick is published only after its decided
WAL record. The order of operations inside the Tick's patch is therefore never
observable: the coordinate before the Tick shows the old head, and the
coordinate after shows the new head with everything it reaches.

## Basis, budget, retries

- **Basis.** Reads, including observed absences, are taken from the exact
  basis. At commit, every `expected…` value and every recorded dependency that
  a co-scheduled Action wrote must still hold; otherwise the Action obstructs
  as stale. Action validity across ticks keeps its existing rules, and an
  Action is never re-stamped as if it had been evaluated against a later basis.
- **Budget.** Evaluation is metered as `edict_pure` already meters it. Each
  effect adds a fixed write cost. Exhausting any limit obstructs with
  `BudgetExceeded`, no proposal, and no patch. A proposal that would exceed the
  Tick's operation cap obstructs the same way rather than splitting.
- **Retries.** A decided outcome is final. An Action is identified by its
  submission (see Identity and existing contracts); once it is decided
  (committed, obstructed, or `RejectedFootprintConflict`), its outcome,
  receipt, and attribution never change, and a later success is never written
  over an earlier conflict. Echo's intake already enforces this: resubmitting
  identical content on the same head returns the existing submission as a
  `Duplicate`, with its original outcome and no new Tick. A real new attempt
  must differ, for example an invocation rebased onto the new head, or a
  causal parent citing the earlier outcome; that difference makes it a new
  submission with its own identity and attribution, evaluated against whatever
  basis is current when the scheduler selects it. Echo does not retry on its
  own; whether to submit again is the application's decision.
- **Pending is not retried.** An Action that was never decided, for example
  one whose Tick was lost in a crash before its WAL record, remains the same
  submission. Recovery re-evaluates it; that is not a retry, because no
  outcome was ever recorded for it.

## Identity and existing contracts

The bridge adds evidence, not a new identity root or a new authority path. Each
step below already exists except the proposal, which is evidence inside one
decision:

| Object     | Identity                                                                                                                              | Owner                                                                                                                             | Status             |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| Submission | `submission_id = H("echo.intent-submission.v0", worldline, head, ingress_id)`; a Continuum occurrence key is hashed into `ingress_id` | [Submission Attribution](../topics/SubmissionAttribution.md), [canonical inbox sequencing](../spec/canonical-inbox-sequencing.md) | existing           |
| Action     | admitted package and projected invocation                                                                                             | ADR 0023, ADR 0025                                                                                                                | existing           |
| Proposal   | `H(Action, evaluation basis, canonical EffectProposal)`; deterministic, so re-evaluating an undecided Action reproduces it            | this design                                                                                                                       | new, evidence only |
| Decision   | committed, obstructed, or `RejectedFootprintConflict`, recorded once in the Tick's attribution section by `submission_id`             | Submission Attribution                                                                                                            | existing           |

Consequences:

- **Admission** is unchanged: the bridge runs inside scheduler-owned Tick
  construction after ordinary admission, and adds checks; it removes none.
- **Attribution** is unchanged: an Action batch already attributes each
  candidate to its own receipt entry. The proposal identity and digests below
  ride in that entry; the attribution section still names only submissions.
- **Durability** is unchanged: the decided-Tick WAL record (`EPRV0003`) is the
  only persistence. Proposals are never retained apart from their Tick.

## Evidence

Each Action's receipt entry binds, in addition to its current fields:

- the proposal identity;
- the witnessed read-set digest, including observed absences, and the
  write-set digest;
- the derived patch digest;
- consumed budget.

These fields change the receipt entry's encoding, which commit hash v3 binds.
They therefore need a new receipt entry format version, introduced under the
same rules as Submission Attribution's format table: existing formats and
retained legacy ticks stay readable and unchanged, and a Tick written with the
new format says so.

Recovery:

- A decided Tick is recovered from its WAL record by replaying its patch.
  Echo then checks the patch digest and the commit hash, which binds the
  receipt and its attribution section, so the recovered evidence is exactly
  the evidence that was committed.
- Recovery does not re-evaluate decided Actions. Their read sets and proposals
  are evidence about a past evaluation, not inputs to replay.
- An Action admitted under the scoped validity of
  [Action validity across ticks](../topics/GeneratedRules.md#action-validity-across-ticks)
  follows that decision: recovery recomputes its validation at the recorded
  position from retained history and refuses a history whose result differs.
- An undecided Action is re-evaluated as the same submission, and determinism
  makes it reproduce the same proposal.

## Execution traces

These traces follow Jim's flat-text reference `ReplaceRange` (the first
acceptance target in flyingrobots/jedit#302). The application vocabulary below
belongs to Jim's package; Echo sees only nodes, edges, and atoms.

Basis coordinate `c` (Tick `t`):

```text
Buffer B   atom {bufferKey "a.txt", version 7}
  -canonicalHead-> H1   atom {byteLength 11, lineCount 1, sequence 4}
H1 -root->   T1   atom {bytes "hello world", lineBreaks 0}
H1 -anchor-> I    atom {fileCoordinate "a.txt", hostDigest d0}
```

Invocation: `{bufferId B, basisHeadId H1, startByte 6, endByte 11,
replacement "there"}` at basis `c`.

### Committed

1. **Admission.** The invocation is admitted at `c` under the package's
   authority and delegated budget, retained before acknowledgement.
2. **Selection.** The scheduler selects the Action while constructing Tick
   `t+1` and opens a recording view over `c`.
3. **Evaluation reads**, recorded by the view: `B` atom; `B` `canonicalHead`
   edge (to `H1`); `H1` atom; `H1` `root` edge (to `T1`); `H1` `anchor` edge
   (to `I`); `T1` atom. Six slots.
4. **Pure computation.** The guard `canonicalHead == H1` holds; range and
   boundaries check; the new bytes are `"hello there"`; the program derives ids
   `T2` and `H2` under its identity law.
5. **Proposal**, in canonical order:
    - `EnsureNode T2 {bytes "hello there", lineBreaks 0}`
    - `EnsureNode H2 {byteLength 11, lineCount 1, sequence 5}`
    - `EnsureEdge H2 -root-> T2`
    - `EnsureEdge H2 -anchor-> I`
    - `ReplaceEdge B -canonicalHead-> expected H1, now H2`
    - `ReplaceAtom B expected digest(version 7), now {version 8}`
6. **Validation.** The read and write sets lie inside the footprint law. Echo
   recomputes `T2` and `H2` from their atoms and children under the declared
   identity law. Endpoints check: `T2` and `H2` are created here, and `I` and
   `B` exist at `c`. The expected `H1` and version-7 digest hold at `c`. The
   budget holds.
7. **Composition.** The write set is `{T2, H2, H2.root, H2.anchor,
B.canonicalHead, B.atom}`. No co-scheduled Action reads or writes these.
8. **Patch**, derived by Echo: `UpsertNode T2`, `SetAttachment T2`,
   `UpsertNode H2`, `SetAttachment H2`, `UpsertEdge H2.root`,
   `UpsertEdge H2.anchor`, `UpsertEdge B.canonicalHead` (same edge id, new
   target), `SetAttachment B`. Eight operations.
9. **Decision and durability.** The receipt binds the proposal identity, the
   read-set and write-set digests, the patch digest, and the consumed budget.
   The decided Tick is written to the WAL, then published.
10. **Visibility.** Coordinate `t` shows `B → H1 → T1`. Coordinate `t+1` shows
    `B → H2 → T2`, with `H2 → I`. No coordinate shows `H2` without `T2`. `H1`
    and `T1` are no longer reachable from `B`; they leave the state root, and
    whether their bytes are kept is the retention policy's decision.

A crash before the WAL record loses the Tick entirely; the Action was never
decided, so on recovery it is the same pending submission and re-evaluates
against `c` to the same proposal. A crash
after it recovers Tick `t+1` from the WAL, with the same state root and
receipt.

### Obstructed: stale basis at evaluation

Another edit has already advanced `B` to head `H3`, so at `c` the
`canonicalHead` edge points to `H3`.

1. Steps 1–2 as above.
2. Evaluation reads the `B` atom and the `canonicalHead` edge, then the guard
   `canonicalHead == H1` fails.
3. The outcome is the typed obstruction `BasisNotCanonical`. There is no
   proposal and no patch. The receipt records the obstruction and the two-slot
   read set, so the refusal is evidence of what the program saw.
4. The state root at `t+1` is unchanged for this Action. The client must issue
   a new invocation against `H3`.

### Obstructed: conflict at composition

Two Actions, both evaluated against `H1`, are selected for the same Tick. Both
propose `ReplaceEdge B -canonicalHead-> expected H1`.

1. Each evaluates and validates on its own.
2. Their write sets overlap on `B.canonicalHead`. In deterministic order the
   first joins the Tick; the second gets `RejectedFootprintConflict`, with no
   patch.
3. The second Action's `RejectedFootprintConflict` is its final outcome. If
   the application submits the edit again, that is a new submission with its
   own identity; evaluated at `t+1`, its basis shows `H2`, its guard fails, and
   it obstructs with `BasisNotCanonical`. Echo never merges the two edits and
   never rewrites the first rejection; reconciling them is the application's
   decision.

## What this design does not do

- It does not create an opaque patch authority. A program cannot submit a
  patch; it can only propose typed effects that Echo validates.
- It does not add application intrinsics, a rope primitive, or a
  general-purpose virtual machine.
- It does not decide the state-root definition, and it does not make
  admission cost independent of history size.

## Required tests

Each behaviour below needs a test before implementation:

| Property               | Test                                                                                                                       |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Atomicity              | A failure on the last effect of a multi-node proposal leaves the parent state root and patch unchanged.                    |
| Ensure identity        | Identical existing node: no patch entry. Different existing node: `ContentIdentityConflict`.                               |
| Witnessed footprint    | A program that reads a slot outside its law obstructs, even if its writes are in bounds.                                   |
| Stale basis            | `ReplaceEdge` with a moved `expectedTo` obstructs with no patch.                                                           |
| Budget                 | Exhausting steps, bytes, or the Tick operation cap obstructs with no proposal committed.                                   |
| Determinism            | Same package, invocation, and basis give byte-identical proposals and receipts.                                            |
| Recovery               | Crash before and after the decided-Tick WAL record; fresh-host recovery reproduces the same state root and receipts.       |
| Composition            | Two Actions with disjoint witnessed write sets on one basis commit in one Tick; overlapping ones obstruct the later one.   |
| Shared structure       | Two Actions that `Ensure` the same node already present in the basis both commit in one Tick.                              |
| Referential integrity  | An edge to a node neither in the basis nor created by the proposal obstructs with `DanglingReference`.                     |
| Visibility             | No published coordinate shows a retargeted head without every node and edge it reaches.                                    |
| Witnessed reads        | The receipt's read set equals the slots the view resolved, including for a program that reads and then obstructs.          |
| Negative reads         | An Action that observed a node, atom, or edge set absent obstructs when a co-scheduled Action creates it.                  |
| Edge-read basis        | An Action that read edges commits only onto its exact evaluation basis.                                                    |
| Final outcomes         | A rejected Action's outcome, receipt, and attribution are unchanged after a later submission of the same edit succeeds.    |
| Duplicate resubmission | Resubmitting a decided Action's identical content returns `Duplicate` with the original outcome and commits no Tick.       |
| Attribution            | Every Action in a bridge Tick appears once in the attribution section, mapped to its own receipt entry.                    |
| Receipt format         | Ticks written before the new entry format recover unchanged; new Ticks declare the new format.                             |
| Recovered evidence     | A recovered Tick's proposal identity, read-set, write-set, and patch digests equal those committed, without re-evaluation. |
| Pending recovery       | An Action lost with its Tick before the WAL record is recovered as the same submission, not a new one.                     |

## Open questions

- How `edict_read` exposes outgoing edges by type, and how a bounded closure
  read guided by a pure Edict selector is expressed and budgeted.
- Whether the footprint law is per-invocation or per-package with parameters.
- Which domain-separated hash Edict source can call to derive ids.
- Whether proposals larger than one Tick's operation cap should ever be
  representable. This design says no.
