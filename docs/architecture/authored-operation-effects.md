<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Authored Operation Effects

- **Status:** Accepted design, effective when
  [flyingrobots/echo#889](https://github.com/flyingrobots/echo/pull/889) merges
  to `main`. Before that merge it is a proposal and governs nothing.
  **Not implemented.** Acceptance establishes the contract; it does not claim
  the bridge exists. The section
  [Current implementation](#current-implementation) states what the runtime
  does today.
- **Proposed:** 2026-10-15 in
  [flyingrobots/echo#889](https://github.com/flyingrobots/echo/pull/889);
  application context in
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
4. **A proposal introduces no dangling reference.** Every edge a proposal
   adds or retargets has endpoints that exist in the basis or are created in
   the same proposal. In particular, a head that a proposal retargets is never
   observable before every node and edge the proposal makes it reach has been
   validated and committed in the same Tick. This invariant covers only the
   edges a proposal introduces or retargets. It does not claim that the basis
   is free of dangling edges: other writers do not check endpoints today (see
   [Current implementation](#current-implementation)), and this design does not
   validate the rest of the graph.

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
- An executable-operation Action batch selects at most 64 candidates
  (`ACTION_BATCH_CANDIDATE_LIMIT_V1`) and accepts at most 128 patch operations
  summed over its accepted Actions (`ACTION_BATCH_OPERATION_LIMIT_V1`); a
  candidate that would exceed the operation limit is obstructed rather than
  accepted (`crates/warp-core/src/echo_operation.rs`). These are Action-batch
  composition limits, not Tick limits: `WarpTickPatchV1::new` accepts an
  operation vector of any length.
- Recovery re-derives each committed and footprint-conflicted Action's
  preparation from the recovered parent state and refuses history whose
  package, invocation, basis, private-evaluation, patch, result, footprint, or
  preparation identity differs from the recorded outcome; obstructed Actions
  are re-evaluated and their pre-decision evidence compared
  (`crates/warp-core/src/trusted_runtime_host.rs`, ADR 0025).
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
Ticks and re-derives each decided Action's evaluation to verify the recorded
evidence (see [Recovery](#recovery)). A pending Action is re-evaluated after
recovery when the scheduler selects it. Both are sound because evaluation is
deterministic over a fixed basis.

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

Validation examines only the edges the proposal adds or retargets, and the
endpoints those edges name. An existing dangling edge elsewhere in the basis
neither obstructs an unrelated proposal nor is repaired by it. Whether Echo
should reject dangling edges from every writer is a separate decision.

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
  effect adds a fixed write cost. Exhausting any evaluation limit obstructs
  with `BudgetExceeded`, no proposal, and no patch. A proposal whose derived
  patch alone exceeds the Action-batch operation limit
  (`ACTION_BATCH_OPERATION_LIMIT_V1`) obstructs the same way rather than
  splitting across Ticks. A proposal that fits alone but not beside the
  Actions already accepted into the batch is handled by the existing batch
  rule, which obstructs that candidate in this batch.
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
  submission. It is evaluated again when the scheduler selects it; that is not
  a retry, because no outcome was ever recorded for it.
- **Verification is not re-admission.** Recovery re-evaluates decided Actions
  only to verify their recorded evidence (see [Recovery](#recovery)). It never
  re-decides them, never stamps a new outcome, and never applies a re-derived
  proposal.

## Identity and existing contracts

The bridge adds evidence, not a new identity root or a new authority path. Each
step below already exists except the proposal, which is evidence inside one
decision:

| Object     | Identity                                                                                                                                                                                      | Owner                                                                                                                             | Status             |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| Submission | `submission_id = H("echo.intent-submission.v0", worldline, head, ingress_id)`; a Continuum occurrence key is hashed into `ingress_id`                                                         | [Submission Attribution](../topics/SubmissionAttribution.md), [canonical inbox sequencing](../spec/canonical-inbox-sequencing.md) | existing           |
| Action     | admitted package and projected invocation                                                                                                                                                     | ADR 0023, ADR 0025                                                                                                                | existing           |
| Proposal   | `H(Action, evaluation basis, canonical EffectProposal)` when evaluation produced a proposal; otherwise none (see Evidence); deterministic, so re-evaluation over the same basis reproduces it | this design                                                                                                                       | new, evidence only |
| Decision   | committed, obstructed, or `RejectedFootprintConflict`, recorded once in the Tick's attribution section by `submission_id`                                                                     | Submission Attribution                                                                                                            | existing           |

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

- the proposal posture, a tagged value with exactly two variants:
    - `NoProposal`: evaluation obstructed before it produced a proposal, for
      example `BudgetExceeded` or an authored guard such as
      `BasisNotCanonical`. No proposal identity, write-set digest, or patch
      digest exists, and none is manufactured;
    - `Proposed { proposal identity, write-set digest, derived patch digest }`:
      evaluation produced a proposal, whether it then committed, obstructed at
      validation, or was rejected at composition;
- the witnessed read-set digest, including observed absences, in both
  variants, because a refusal is still evidence of what the program saw;
- consumed budget.

The submission identity is present in both variants. An identified submission
whose evaluation produced no proposal is a different evidence state from an
evaluated proposal, and the receipt keeps them distinct.

These fields change the receipt entry's encoding, which commit hash v3 binds.
They therefore need a new receipt entry format version, introduced under the
same rules as Submission Attribution's format table: existing formats and
retained legacy ticks stay readable and unchanged, and a Tick written with the
new format says so.

### Recovery

The bridge keeps Echo's existing recovery verification and extends it to the
new evidence. Verification is not re-admission.

- A decided Tick is recovered from its WAL record by replaying its patch.
  Echo then checks the patch digest and the commit hash, which binds the
  receipt and its attribution section, so the recovered evidence is exactly
  the evidence that was committed. Recovered state comes from the committed
  patch, never from a re-derived proposal.
- As recovery already does for committed and footprint-conflicted Actions
  (`trusted_runtime_host.rs`, ADR 0025), it re-derives each decided Action's
  evaluation over the recovered parent state and compares the result with the
  recorded receipt entry: the proposal posture and, where present, the
  proposal identity, the read-set and write-set digests, and the derived patch
  digest, in addition to the identities compared today. An obstructed Action's
  pre-decision evidence is compared the same way. Any disagreement refuses the
  history.
- The comparison never changes a decision. The recorded outcome, receipt, and
  attribution stand as written; a re-derived proposal is checked against them
  and then discarded.
- An Action admitted under the scoped validity of
  [Action validity across ticks](../topics/GeneratedRules.md#action-validity-across-ticks)
  follows that decision: recovery recomputes its validation at the recorded
  position from retained history and refuses a history whose result differs.
- An undecided Action stays the same pending submission and is evaluated when
  the scheduler next selects it.

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
9. **Decision and durability.** The receipt binds the proposal posture
   `Proposed` with the proposal identity, the write-set and patch digests, the
   read-set digest, and the consumed budget.
   The decided Tick is written to the WAL, then published.
10. **Visibility.** Coordinate `t` shows `B → H1 → T1`. Coordinate `t+1` shows
    `B → H2 → T2`, with `H2 → I`. No coordinate shows `H2` without `T2`. `H1`
    and `T1` are no longer reachable from `B`; they leave the state root, and
    whether their bytes are kept is the retention policy's decision.

A crash before the WAL record loses the Tick entirely; the Action was never
decided, so after recovery it is the same pending submission, evaluated again
when the scheduler selects it. Over the unchanged basis `c` it reproduces the
same proposal. A crash after the WAL record recovers Tick `t+1` by replaying
its patch, re-derives the Action's evaluation over `c` to verify the recorded
proposal posture and digests, and yields the same state root and receipt.

### Obstructed: stale basis at evaluation

After coordinate `c`, another edit commits at Tick `u > t` and moves `B` to
head `H3`. Coordinate `c` still shows `H1`; the later coordinate `c'` (Tick
`u`) shows `B -canonicalHead-> H3`. A client that is still displaying `H1`
submits the same invocation, `basisHeadId H1`, and it is admitted at `c'`.

1. **Admission** at `c'`, as in the committed trace.
2. **Selection.** The scheduler selects the Action while constructing Tick
   `u+1` and opens a recording view over `c'`.
3. **Evaluation** reads the `B` atom and the `canonicalHead` edge (to `H3`),
   then the authored guard `canonicalHead == basisHeadId` fails.
4. The outcome is the typed obstruction `BasisNotCanonical` with proposal
   posture `NoProposal`. There is no proposal identity and no patch. The
   receipt records the obstruction and the two-slot read set, so the refusal
   is evidence of what the program saw.
5. The state root at `u+1` is unchanged by this Action. The client must issue
   a new invocation naming `H3`.

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
- It does not implement or imply Continuum transport, comparable-basis
  transport, replica import, or distributed settlement. It defines generic
  authored effects inside one Echo's existing admission, scheduling, WAL, and
  recovery architecture. That Echo can commit an Edict-authored proposal says
  nothing about whether a remote history can be imported; those are separate
  contracts with their own reviews.

## Required tests

Each behaviour below needs a test before implementation:

| Property               | Test                                                                                                                                                                                                                                                                                                 |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Atomicity              | A failure on the last effect of a multi-node proposal leaves the parent state root and patch unchanged.                                                                                                                                                                                              |
| Ensure identity        | Identical existing node: no patch entry. Different existing node: `ContentIdentityConflict`.                                                                                                                                                                                                         |
| Witnessed footprint    | A program that reads a slot outside its law obstructs, even if its writes are in bounds.                                                                                                                                                                                                             |
| Stale basis            | `ReplaceEdge` with a moved `expectedTo` obstructs with no patch.                                                                                                                                                                                                                                     |
| Budget                 | Exhausting steps or bytes obstructs with `NoProposal`; a proposal whose patch alone exceeds the Action-batch operation limit obstructs with no patch.                                                                                                                                                |
| Determinism            | Same package, invocation, and evaluation basis give byte-identical proposals, proposal identities, read-set and write-set digests, and derived patches, whether the Action is evaluated alone or beside other Actions. Receipts and composite patches are compared only for identical Tick contexts. |
| No-proposal evidence   | An evaluation that obstructs before proposing records `NoProposal` with its read-set digest and no proposal, write-set, or patch digest.                                                                                                                                                             |
| Recovery               | Crash before and after the decided-Tick WAL record; fresh-host recovery reproduces the same state root and receipts.                                                                                                                                                                                 |
| Composition            | Two Actions with independent witnessed footprints (neither writes a slot the other reads or writes) commit in one Tick. A write/write overlap obstructs the later one, and so does a write/read overlap: A reads X and writes Y, B writes X and Z.                                                   |
| Shared structure       | Two Actions that `Ensure` the same node already present in the basis both commit in one Tick.                                                                                                                                                                                                        |
| Referential integrity  | An edge to a node neither in the basis nor created by the proposal obstructs with `DanglingReference`; a pre-existing dangling edge elsewhere in the basis does not obstruct an unrelated proposal.                                                                                                  |
| Visibility             | No published coordinate shows a retargeted head without every node and edge it reaches.                                                                                                                                                                                                              |
| Witnessed reads        | The receipt's read set equals the slots the view resolved, including for a program that reads and then obstructs.                                                                                                                                                                                    |
| Negative reads         | An Action that observed a node, atom, or edge set absent obstructs when a co-scheduled Action creates it.                                                                                                                                                                                            |
| Edge-read basis        | An Action that read edges commits only onto its exact evaluation basis.                                                                                                                                                                                                                              |
| Final outcomes         | A rejected Action's outcome, receipt, and attribution are unchanged after a later submission of the same edit succeeds.                                                                                                                                                                              |
| Duplicate resubmission | Resubmitting a decided Action's identical content returns `Duplicate` with the original outcome and commits no Tick.                                                                                                                                                                                 |
| Attribution            | Every Action in a bridge Tick appears once in the attribution section, mapped to its own receipt entry.                                                                                                                                                                                              |
| Receipt format         | Ticks written before the new entry format recover unchanged; new Ticks declare the new format.                                                                                                                                                                                                       |
| Recovered evidence     | Recovery replays the committed patch, re-derives each decided Action's evaluation, and accepts the history only when the proposal posture, proposal identity, read-set, write-set, and patch digests equal those recorded; a tampered digest refuses the history, and no recorded outcome changes.   |
| Pending recovery       | An Action lost with its Tick before the WAL record is recovered as the same submission, not a new one.                                                                                                                                                                                               |

## Open questions

- How `edict_read` exposes outgoing edges by type, and how a bounded closure
  read guided by a pure Edict selector is expressed and budgeted.
- Whether the footprint law is per-invocation or per-package with parameters.
- Which domain-separated hash Edict source can call to derive ids.
- Whether proposals larger than the Action-batch operation limit should ever
  be representable. This design says no.
