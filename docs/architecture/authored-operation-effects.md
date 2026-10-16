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
- **Supersedes in part:** for bridge Actions only, ADR 0023's
  prepared-evaluation outcome: that a successful evaluation exposes one
  normal Echo patch, that every evaluation yields a closed
  `PreparedEchoOperationOutcomeV1` (`Committable` or `Obstructed`), and that
  an `Obstructed` outcome never carries a patch digest. A bridge evaluation
  yields an `EffectProposal` or a typed obstruction; Echo derives the patch
  afterwards in the same Tick construction, and the receipt posture in
  [Evidence](#evidence) replaces the closed outcome. Some obstructed postures
  record the digest of a derived patch that was never committed. ADR 0023's
  atomic visibility (one parent patch or one obstruction) still governs, and
  every other Action keeps ADR 0023 unchanged. Each superseded passage is
  marked in ADR 0023.
- **Supersedes in part:** for bridge Actions only, ADR 0025's requirements
  that a successful evaluation return one complete prepared candidate and
  that an evaluation obstruction return a lawpack-qualified coordinate. A
  bridge evaluation returns an `EffectProposal`. Its program is compiled
  from Edict source and binds no lawpack resource, so its typed obstruction
  is qualified by the admitted package and installed-operation identities
  instead. ADR 0025's other requirements,
  including that neither result mutates the parent, still govern. The
  superseded clause is marked in ADR 0025.
- **Related:** [Sliced Readings and Copy-on-Write Strands](sliced-readings-and-copy-on-write-strands.md),
  [Generated Rule Authorship](../topics/GeneratedRules.md),
  [WAL](../topics/WAL.md)

## Decision

An admitted executable operation whose program is compiled Edict may produce a
bounded set of graph effects. Evaluation proposes those effects. Echo witnesses
what the evaluation actually read and what it proposes to write, validates both
against the admitted contract and the basis, and commits all of the effects in
one Tick or none of them.

Five invariants govern the design:

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
5. **A proposal identity names exactly one evaluated proposal.** Every outcome
   that carries a proposal identity binds it to the exact effects the
   evaluation produced, whether those effects committed or were rejected. A
   rejected proposal is never ambiguous on replay or audit: re-deriving it over
   the recorded basis must reproduce effects that hash to the recorded
   identity and the recorded target-set digest, and, where a patch was
   derived, the recorded write-set and patch digests. Each slot is the target
   of at most one effect, so the derived patch applies every effect the
   identity names.

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
  (`crates/warp-core/src/footprint.rs`, `Footprint::independent`). Its key
  sets are nodes, edges, attachments, and ports; it has no key for a range
  such as "all outgoing edges of one type from a node".
- `WarpTickPatchV1::new` keeps one operation per canonical sort key, the last
  one supplied, and silently drops earlier operations with the same key
  (`crates/warp-core/src/tick_patch.rs`).
- The retained provenance record `EPRV0003` writes each receipt entry as rule,
  scope hash, scope, disposition, and blockers, with no per-entry length or
  version (`crates/warp-core/src/provenance_codec.rs`).
- An executable-operation Action batch selects at most 64 candidates
  (`ACTION_BATCH_CANDIDATE_LIMIT_V1`) and accepts at most 128 patch operations
  summed over its accepted Actions (`ACTION_BATCH_OPERATION_LIMIT_V1`); a
  candidate that would exceed the operation limit is obstructed rather than
  accepted (`crates/warp-core/src/echo_operation.rs`). These are Action-batch
  composition limits, not Tick limits: `WarpTickPatchV1::new` accepts an
  operation vector of any length.
- Recovery verifies each decided Action by outcome class
  (`crates/warp-core/src/trusted_runtime_host.rs`, ADR 0025). First, for
  every class, it recomputes the Action's basis posture against the scheduler
  Tick's parent and refuses history where the recorded posture disagrees.
  Then:
    - **Committed and `RejectedFootprintConflict`:** it reconstructs the
      preparation from the recovered parent state and refuses history whose
      package, installed-operation, invocation, basis, private-evaluation,
      patch, result, footprint, or preparation identity differs from the
      recorded evidence.
    - **Obstructed `BasisChanged`:** it verifies the invocation-admission
      evidence and the recomputed basis posture. It does not re-evaluate the
      Action.
    - **Other obstructions:** it re-evaluates the Action and requires the same
      pre-decision evidence, except that a scheduler-assigned `BudgetExceeded`
      (an Action-batch limit, decided after a successful evaluation) is
      accepted as a reconstructed preparation and verified through the batch
      reconstruction below.
      Finally, for every Action Tick and every outcome class, it reconstructs the
      whole Action batch's composition over the recovered parent state and refuses
      history where any reconstructed outcome differs from the recorded one; for
      each `RejectedFootprintConflict` it also recomputes the blocking earlier
      Actions and requires them to equal the recorded blockers.
- `echo_operation_written_slots_v1` collects the slots written between an
  origin and a commit parent by unioning each retained replay patch's
  `out_slots`. `WarpTickPatchV1::new` accepts `out_slots` independently of
  its operations, and only debug builds or `footprint_enforce_release` check
  that a rewrite stays inside its declared footprint.
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
Ticks and verifies each decided Action by its outcome class: it re-derives the
evaluation for the classes whose check does so, and verifies a `BasisChanged`
decided before evaluation without re-evaluating it (see [Recovery](#recovery)).
A pending Action is evaluated after recovery when the scheduler selects it.
Re-derivation is sound because evaluation is deterministic over a fixed
basis.

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
  cost                   steps, read bytes, write bytes; allocated bytes
}
```

Every `node` is an instance-scoped `NodeKey { warp_id, local_id }` and every
`edge` an `EdgeKey`. An edge's `from`, `to`, and `expectedTo` are local node
ids in the edge's own warp, because a `GraphStore` holds one warp and an
`EdgeRecord`'s endpoints are local `NodeId`s within it. For `EnsureNode` and
`CreateNode`, `type` is the node record's type; for `EnsureEdge` it is the edge
record's type (`EdgeRecord.ty`), validated against the edge types the schema
admits, never against node types. `atom` is an
`AtomPayload { type_id, bytes }`: its `type_id` is a separate proposition from
the node type, and the
[two-plane law](../invariants/warp-two-plane-law.md) (L3) requires it in every
encoding and digest that includes the bytes. `ReplaceAtom` replaces both the
payload type and the bytes, and its `expectedAtomDigest` covers both.

Rules:

- **Effects are the only writes.** The proposal contains no prepared patch.
  Echo derives the patch from the effects.
- **Canonical order.** Echo sorts the effects the evaluator emits; the order
  in which evaluation produced them never reaches the identity. Each effect
  has a primary slot: the node record for `EnsureNode` and `CreateNode`, the
  atom for `ReplaceAtom`, and the edge for `EnsureEdge` and `ReplaceEdge`.
  Effects sort by the primary slot's class (node record, then attachment, then
  edge), then by its warp id bytes, then its local id bytes, compared
  lexicographically. A tie can occur
  only in a proposal that obstructs with `DuplicateEffectTarget`; it breaks
  by effect kind (`EnsureNode`, `CreateNode`, `ReplaceAtom`, `EnsureEdge`,
  `ReplaceEdge`), then by the effect's canonical encoding bytes. That
  encoding includes every field of the effect, expected values included: a
  kind tag, then each field in the order the effect lists it, with each
  instance-scoped key as its warp id then its local id, ids, type ids, and
  digests at fixed width, and an atom as its `type_id`, its byte length as a
  little-endian `u64`, then its bytes. Sorting never
  removes an effect, so duplicates stay visible to the one-effect-per-slot
  rule.
- **One effect per slot.** No two effects in one proposal may target the same
  slot: one node record, one node's atom, or one edge. Targets are compared
  by canonical slot identity (the node id, the node id's atom, or the edge
  id), never by the effect's kind or form, so differently expressed effects
  cannot reach one slot undetected. `EnsureNode` and `CreateNode` target the
  node record and its atom, so an `EnsureNode` plus a `ReplaceAtom` of the
  same node collide, and an `EnsureEdge` plus a `ReplaceEdge` of the same
  edge id collide. A collision is a typed `DuplicateEffectTarget` obstruction
  before any patch is derived. Without this rule `WarpTickPatchV1::new` would
  keep only the last operation per key and silently drop effects the proposal
  identity names.
- **Slots.** One canonical slot model serves the target set, the write set,
  the read set, and the footprint. A slot is a node record, an attachment, an
  edge, or an outgoing-edge range key `(source node, edge type)` (see
  [Witnessed reads](#witnessed-reads-over-a-bounded-slice)). An attachment
  slot is an `AttachmentKey`: an owner node or edge and a plane. A node's
  atom is its alpha attachment. The same slot can instead hold a portal
  (`AttachmentValue::Descend`), so a write of an atom and a read of a portal
  on one node meet in one slot and conflict. Every slot is instance-scoped:
  node records and node attachments by `NodeKey`, edges and edge attachments
  by `EdgeKey`, and a range key by its source `NodeKey`, so equal local ids
  in different warps are different slots.
- **Portal chain.** When an effect or a read names a key in a descended
  warp, the read set, the footprint, and a scoped Action's dependency set
  include every portal attachment on the validated root-to-target
  reachability chain, as
  [Generated Rule Authorship](../topics/GeneratedRules.md) requires for
  descended targets and as Echo's existing descent check already reads
  them. Each is a `ValueDigest` dependency on its `Descend` target. A
  co-scheduled write to a portal therefore conflicts, and a portal that
  changed between origin and commit parent, even if restored, obstructs with
  `BasisChanged`. Effects never write a portal. Every key an effect names
  must lie in the root warp or in a warp reached by a valid root-to-target
  chain at the basis; a key in a missing or unreachable warp is a typed
  `UnreachableWarp` obstruction, posture `Proposed`, `NotDerived`, decided
  before any patch is derived.
- **Target set.** The proposal's target set is the canonical set of slots its
  effects target: for `EnsureNode` and `CreateNode` the node record and its
  atom, for `ReplaceAtom` the atom, and for `EnsureEdge` and `ReplaceEdge` the
  edge. It is a function of the effects alone, defined for every proposal
  before validation, and its digest is part of the evidence.
- **Write set.** The proposal's write set is the target set less the slots of
  each `Ensure` whose target already exists identically in the basis (those
  are reads), plus the outgoing-edge range key of every edge the proposal
  creates or retargets: the `(from, type)` of a creating `EnsureEdge`, and the
  source and type of the basis edge a `ReplaceEdge` retargets. Range keys
  depend on the basis, so the write set is known only once validation has
  resolved each effect against it.
- **Effect preconditions.** Each effect states what must hold for it to
  apply, whether or not the program read it:
    - `CreateNode`: the node record and its atom are both absent. If either
      exists, including an atom without a node record, the effect obstructs
      with `PreconditionMismatch`, as today's create-if-absent does;
    - `EnsureNode`: the node record and its atom are both absent, or both
      present with identical node type, atom `type_id`, and atom bytes. Any
      other state, including a record without an atom or an atom without a
      record, is `ContentIdentityConflict`;
    - `EnsureEdge`: the edge is absent, or present with identical endpoints
      and type;
    - `ReplaceAtom`: the node record exists, else `NodeMissing` (as today's
      compare-and-set reports), and its atom has the expected digest, else
      `PreconditionMismatch`. An orphan atom without its record therefore
      never satisfies a `ReplaceAtom`;
    - `ReplaceEdge`: the edge exists with the expected target;
    - a creating `EnsureEdge` and every `ReplaceEdge`: each endpoint the
      edge will name (`from` and `to`, or the retargeted edge's source and
      the new `to`) exists, unless the same proposal creates it. An
      `EnsureEdge` that finds an identical edge is a read; it adds and
      retargets nothing, so its endpoints are not checked, as invariant 4
      and [Visibility and referential integrity](#visibility-and-referential-integrity)
      limit endpoint checks to edges the proposal introduces or retargets;

    Every precondition slot is a read in the Action's footprint, is checked by
    validation, and belongs to the dependency set of a dependency-scoped Action
    (see [Basis, budget, retries](#basis-budget-retries)).

- **Ensure is idempotent only on identity.** An `EnsureNode` whose node exists
  with identical type and atom adds nothing to the patch and counts as a read.
  An existing node with different content is a typed
  `ContentIdentityConflict` obstruction. The same holds for `EnsureEdge`. This
  is the idempotent create profile that Edict's I-022 requires before
  duplicate content-addressed creation can be lawful.
- **Compare-before-replace.** `ReplaceAtom` and `ReplaceEdge` carry the value
  they expect. A mismatch is a typed `PreconditionMismatch` obstruction, the
  kind today's compare-and-set uses, with posture `Proposed`, `NotDerived`.
  It is never a merge. It is distinct from `BasisChanged`, which reports a
  dependency that changed after the origin (see
  [Basis, budget, retries](#basis-budget-retries)).
- **No deletes in this profile.** Persistent structures do not need them.
  Deletion needs its own design.
- **Identity derivation is the program's.** Echo checks that each node and edge
  id matches the identity law the package declares, so ids cannot be chosen to
  collide with unrelated nodes. A node or edge id that does not match is a
  typed `IdentityLawViolation` obstruction before any patch is derived. Echo
  does not invent ids.

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

A read of the outgoing edges of one type from a node is a range, not a set of
known edge ids, so it needs its own footprint key. Today's `Footprint` has
none: if Action A observes no `canonicalHead` edge from `B` and co-scheduled
Action B creates a fresh one, their edge-id sets are disjoint and the existing
test calls them independent, although A's observed absence no longer holds.
The bridge therefore adds an outgoing-edge range key `(source node, edge
type)`. A range read claims it as a read. Every effect that creates or
retargets an edge with that source and type claims it as a write, in addition
to the edge id. Independence treats range keys like every other key: a write
against a read or write conflicts.

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

- every node type, atom `type_id`, and edge type an effect writes is admitted
  by the package's typed graph and attachment schema, which ADR 0023 requires
  the package to bind, including which atom types a node type may carry. A
  violation is a typed `SchemaViolation` obstruction, posture `Proposed`,
  `NotDerived`;

- the witnessed read set, the precondition slots, and the proposal's target
  set lie inside the law;
- counts do not exceed the declared maxima;
- once validation has resolved each effect against the basis, the complete
  write set, range keys included, and its counts lie inside the law. A range
  key is known only after resolution: a `ReplaceEdge` whose basis edge has a
  source or type outside the law fails here, before any patch exists;
- the derived patch's actual footprint equals the proposal's write set (see
  [EffectProposal](#effectproposal)). The actual footprint is computed in the
  same slot model: each node upsert writes the node record, each attachment
  set writes the node's atom, and each edge upsert writes the edge and the
  range key of its source and type.

The first check yields a typed `SchemaViolation` obstruction; the second,
third, and fourth yield a typed `FootprintLawViolation`. All four run before
any patch is derived, with patch tag `NotDerived`. The fifth can only run after
derivation; a mismatch is a typed `FootprintMismatch` obstruction with patch
tag `Derived`, recording the patch digest it compared. Neither commits a patch.
Independence between Actions in one Tick uses the witnessed footprints,
including outgoing-edge range keys, under the existing rule: write against read
or write conflicts, read against read does not.

Reading a large shared subtree costs footprint entries but never conflicts.

An `Ensure` of a node or edge that already exists identically in the basis is
a read, not a write, so Actions that share existing structure do not conflict.
Two Actions in one Tick that both create the same new content-addressed node
do write the same slot, and the existing rule makes the later one obstruct.
This design adds no merge rule for identical writes. Every Action's write
set, and so whether its `Ensure` is a read, is resolved against the Tick's
commit parent, never against the writes of Actions accepted earlier in the
same Tick. A later Action therefore cannot turn an earlier Action's new node
into a read of its own, even under dependency-scoped validity, whose value
comparison runs at the validation position. The rejected Action's
outcome is final (see Retries); a new submission evaluated later finds the
node in its basis, so its `Ensure` is a read and does not conflict.

## Validation order

Validation of a proposal runs in one fixed order, and the first failing step
decides the outcome, so a proposal that breaks several rules has exactly one
recorded obstruction:

1. the dependency-scoped history check, for a scoped Action
   (`BasisChanged`);
2. one effect per slot (`DuplicateEffectTarget`);
3. the identity law (`IdentityLawViolation`);
4. warp reachability of every effect key (`UnreachableWarp`);
5. the schema (`SchemaViolation`);
6. the footprint law over the witnessed read set, the precondition slots,
   the target set, and the counts (`FootprintLawViolation`);
7. effect preconditions, effect by effect in canonical order, and within an
   effect in the order its precondition list gives (`NodeMissing`,
   `PreconditionMismatch`, `ContentIdentityConflict`, `DanglingReference`,
   or `BudgetExceeded` when a validation read exceeds the remaining budget);
8. the footprint law over the resolved write set
   (`FootprintLawViolation`);
9. patch derivation and the actual-footprint equality
   (`FootprintMismatch`);
10. the Action-batch operation limit (`BudgetExceeded`);
11. composition (`RejectedFootprintConflict`).

Steps 2 through 8 yield `NotDerived`, and steps 9 through 11 yield `Derived`.
Within a step that examines several effects, the first failing effect in
canonical order decides; since one kind belongs to each step except step 7,
only step 7 depends on that tie-break for its kind.

## Visibility and referential integrity

Validation enforces invariant 4 before a proposal can join a Tick, through
the endpoint preconditions defined in [EffectProposal](#effectproposal):

- every creating `EnsureEdge` and every `ReplaceEdge` endpoint exists in the
  basis or is
  created by an `EnsureNode` or `CreateNode` in the same proposal;
- every edge a `ReplaceEdge` retargets exists in the basis with the expected
  target.

A missing endpoint is a typed `DanglingReference` obstruction with no patch.
Because endpoints are precondition reads, a co-scheduled Action that writes
one conflicts, and a delayed Action whose endpoint changed in the intervening
history obstructs with `BasisChanged`, even if the endpoint was restored.

Validation examines only the edges the proposal adds or retargets, and the
endpoints those edges name. An existing dangling edge elsewhere in the basis
neither obstructs an unrelated proposal nor is repaired by it. Whether Echo
should reject dangling edges from every writer is a separate decision.

Commit then makes partial visibility impossible by construction. All of an
Action's effects belong to one Tick. Observers, readings, and later Actions bind
to committed Tick coordinates, and a Tick is published only after its decided
WAL record. The order of operations inside the Tick's patch is therefore never
observable: the coordinate before the Tick shows the old head, and the
coordinate after shows the new head with every node and edge the proposal
introduced. Pre-existing structure the head now reaches is whatever the basis
held; this design validates only what the proposal adds or retargets.

## Basis, budget, retries

- **Basis.** Reads, including observed absences, are taken from the exact
  basis. At commit, every `expected…` value and every recorded dependency that
  a co-scheduled Action wrote must still hold; otherwise the Action obstructs
  as stale. Action validity across ticks keeps its existing rules, and an
  Action is never re-stamped as if it had been evaluated against a later basis.
- **Two kinds of basis change.** Today's executable programs have fixed,
  statically known reads, so Echo decides dependency-scoped validity before
  evaluation. A bridge Action's reads are known only by evaluating it, so the
  bridge splits basis change in two:
    - **Whole-frontier `BasisChanged`,** decided before evaluation: the Action
      is not eligible for dependency-scoped validity (for example because its
      program can read edges) and its evaluation basis is no longer the commit
      parent. Posture `NoProposal`, empty read set, no evaluation.
    - **Dependency-scoped,** decided after evaluation: the Action is eligible,
      so Echo evaluates it at its recorded origin basis, then checks its
      dependency set against the history between that origin and the commit
      parent. The dependency set is every witnessed read, presence and
      absence, plus every effect precondition defined in
      [EffectProposal](#effectproposal), whether or not the program read it,
      including each edge endpoint. As the Action validity across ticks
      decision requires, a dependency that changed in that history counts as
      changed even if it was later restored.
    - **How a restored value is detected.** Comparing current values cannot
      see a change from A to B and back to A. The history check therefore
      compares slots, not values: it uses the set of slots written by the
      retained replay patches from the origin to the commit parent. A
      dependency whose slot is in that set has changed, whatever its value
      now. The set is derived from each patch's replay operations, in the
      slot model of the actual footprint, never from its declared
      `out_slots`, which a release build does not check against the
      operations (see [Current implementation](#current-implementation)).
      `echo_operation_written_slots_v1` must therefore project operations, or
      verify that `out_slots` covers them exactly and treat any mismatch as
      unknown. Value comparison is used only at validation, as the next
      bullet orders it. No new versioning system is added.
    - **Unknown history is decided before evaluation.** The written set
      depends only on the origin and the commit parent, so Echo computes it
      before evaluating the Action. If any Tick in the interval has no
      retained replay patch, or its writes cannot be derived, the set is
      unknown and the Action takes the whole-frontier rule at once: since the
      origin is not the commit parent, it obstructs with whole-frontier
      `BasisChanged`, posture `NoProposal`, empty read set, no evaluation.
    - **Evaluation obstructions stand.** An evaluation that obstructs before
      producing a proposal (`BudgetExceeded`, or an authored guard) keeps
      that obstruction with posture `NoProposal`. Its later reads never ran
      and it has no effects, so its dependency set is incomplete, and it is
      never reclassified as `BasisChanged`. It commits nothing either way.
    - **Order.** For a proposal, the history check runs immediately after
      evaluation and before any validation. If a dependency changed, the
      Action obstructs with `BasisChanged`, posture `Proposed` with patch tag
      `NotDerived`, and keeps its witnessed read set and the dependency it
      names as changed. No patch is derived from a
      proposal whose dependencies changed, so a proposal that would also have
      failed validation (for example `DuplicateEffectTarget`) still has one
      encodable outcome. Only when every dependency holds does validation run.
      Echo never validates a delayed Action without its actual dependencies.
    - **Within one Tick, footprints before earlier writes.** Every check before
      composition, preconditions and expected values included, runs against the
      Tick's commit parent, never against writes accepted earlier in the same
      Tick. Composition then reserves each candidate's complete footprint
      (witnessed reads, precondition reads, and write set) in canonical order,
      and any overlap with an earlier accepted Action in which either of the two
      writes the slot is `RejectedFootprintConflict` with its blockers, as the
      [within-one-tick rule](../topics/GeneratedRules.md#action-validity-across-ticks)
      requires.
      Read against read never conflicts, so Actions that share existing
      structure both proceed. A candidate with no such overlap depends on no
      slot an earlier Action wrote, so its values at the validation position
      (the commit parent plus earlier accepted writes) equal its values at the
      commit parent. A later Action that expects a value an earlier Action
      replaced is therefore a footprint conflict, never a
      `PreconditionMismatch`.
- **Budget.** The delegated budget stays `EchoOperationBudgetV1`, with its
  three axes: steps, read bytes, and write bytes. Evaluation is metered as
  `edict_pure` already meters it. Each effect adds a fixed write cost, and an
  effect carrying an atom also charges the atom's byte length against
  `write_bytes`, so a large payload cannot fit a budget that would not cover
  its bytes. Exhausting any evaluation limit obstructs with
  `BudgetExceeded`, posture `NoProposal`, and no patch. Allocation is not a
  delegated axis: `edict_pure` already bounds it by its own
  `max_allocated_bytes`, the host ceiling intersected with the program's
  declared maximum, and exceeding that is likewise `BudgetExceeded` with
  `NoProposal`. The allocated bytes are recorded as evidence, never compared
  against the delegated budget.
- **Validation reads are charged.** A precondition check that reads a value
  the program did not read charges that value's length against
  `read_bytes` before hashing it, as today's compare-and-set charges the
  current atom: a `ReplaceAtom` or `EnsureNode` hashes the current atom, and
  an `EnsureEdge` or `ReplaceEdge` reads a fixed-size edge record. Charges
  accrue in validation order, against the budget left after evaluation. A
  check that would exceed it obstructs with `BudgetExceeded`, posture
  `Proposed`, `NotDerived`, at that step of the validation order, before the
  value is hashed. A proposal whose derived
  patch alone exceeds the Action-batch operation limit
  (`ACTION_BATCH_OPERATION_LIMIT_V1`) also obstructs with `BudgetExceeded` and
  no committed patch, rather than splitting across Ticks, but its posture is
  `Proposed` with a derived patch: the proposal exists, so its identity,
  target-set, write-set, and patch digests are recorded, not discarded. A
  proposal that fits alone but not beside the Actions already accepted into the
  batch is handled by the existing batch rule, which obstructs that candidate in
  this batch, likewise with posture `Proposed`.
- **Retries.** A decided outcome is final. An Action is identified by its
  submission (see Identity and existing contracts); once it is decided
  (committed, obstructed, or `RejectedFootprintConflict`), its outcome,
  receipt, and attribution never change, and a later success is never written
  over an earlier conflict. Echo's intake already enforces this: resubmitting
  identical content on the same head returns the existing submission as a
  `Duplicate`, with its original outcome and no new Tick. A real new attempt
  must differ, for example an invocation rebased onto the new head, or a
  causal parent citing the earlier outcome; that difference makes it a new
  submission with its own identity, attribution, and recorded evaluation
  basis. Like any Action, it is evaluated at that recorded basis, never
  re-stamped onto whatever state is current when the scheduler selects it,
  and then committed under the whole-frontier or dependency-scoped rule.
  Echo does not retry on its own; whether to submit again is the
  application's decision.
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
- **Durability** is unchanged in kind: the decided-Tick WAL record is the only
  persistence. Proposals are never retained apart from their Tick. The record
  gains a new layout version for the new evidence (see Evidence).

## Evidence

Each Action's receipt entry binds, in addition to its current fields:

- the proposal posture, a tagged value with exactly three variants:
    - `NotBridge`: the entry is not a bridge Action. Fixed-profile Actions and
      every other existing receipt entry take this variant. It carries none of
      the bridge fields below, and its existing fields are encoded and hashed
      exactly as under receipt digest version 4, after the tag. No bridge
      evidence is manufactured for it;
    - `NoProposal`: no proposal exists, because evaluation obstructed before
      producing one (for example `BudgetExceeded` or an authored guard such
      as `BasisNotCanonical`) or never ran (whole-frontier `BasisChanged`).
      No proposal identity, target-set, write-set, or patch digest exists,
      and none is manufactured;
    - `Proposed { proposal identity, target-set digest, patch }`: evaluation
      produced a proposal. The target-set digest is always defined, because
      the target set depends only on the effects. `patch` is itself tagged:
        - `Derived { write-set digest, patch digest }` when Echo derived a
          patch: the proposal committed, was rejected at composition, or was
          obstructed by `FootprintMismatch` or an Action-batch limit. The
          write set is known only once validation has resolved each effect
          against the basis, so it is recorded only here;
        - `NotDerived` when no patch was derived: validation refused the
          proposal (`DanglingReference`, `ContentIdentityConflict`,
          `DuplicateEffectTarget`, `FootprintLawViolation`,
          `IdentityLawViolation`, `SchemaViolation`, `UnreachableWarp`,
          `NodeMissing`, `PreconditionMismatch`, or a `BudgetExceeded`
          charged by a validation read), or a
          dependency-scoped `BasisChanged` was decided before validation. No
          write-set or patch digest is manufactured; an overwrite patch that
          `Ensure` forbids is never hashed;
- for `NoProposal` and `Proposed`, the witnessed read-set digest, including
  observed absences, for every outcome that evaluated, because a refusal is
  still evidence of what the program saw; it is the empty set's digest when
  evaluation never ran;
- for `NoProposal` and `Proposed`, the validity rule, a tagged value:
    - `WholeFrontier`: the Action commits only onto its exact evaluation
      basis; the recorded basis posture is its evidence;
    - `Scoped`: the evidence the
      [Action validity across ticks](../topics/GeneratedRules.md#action-validity-across-ticks)
      decision requires: the origin basis, the validation position, and one
      of three evidence forms, each recording only a comparison that actually
      ran:
        - `OriginOnly`, for an evaluation obstruction, which runs no
          dependency comparison;
        - `HistoryChanged`, for a dependency-scoped `BasisChanged`: the
          dependency-set digest over each dependency's descriptor and
          comparison mode, and the first changed dependency in canonical
          order. Validation never ran, so no values were compared;
        - `Compared`, for every Action that passed the history check,
          whatever its later outcome: the dependency-set digest and a
          compared-values digest over the dependency values the
          pre-composition checks read at the commit parent. A dependency
          those checks never reached, because validation stopped at an
          earlier step or a validation read exhausted the budget before
          hashing its value, is recorded as not compared, with no value. For
          an accepted
          Action these equal its values at the validation position (the
          commit parent plus the writes of every earlier accepted Action),
          because composition admits no overlap; a
          `RejectedFootprintConflict` is never compared at its validation
          position;

- for `NoProposal` and `Proposed`, consumed budget. When evaluation never
  ran (whole-frontier `BasisChanged`, including unknown history), it is zero
  on every axis, and recovery verifies that.

**Dependency descriptors.** A dependency descriptor is its slot, its kind,
and its comparison mode, each kind and mode a one-byte tag:

- kind: value present `0`, observed absent `1`, expected value `2`, endpoint
  existence `3`;
- comparison mode: `ValueDigest` `0`, which compares the slot's value digest
  (value present, expected value), and `Presence` `1`, which compares only
  presence (observed absent, endpoint existence). These are the two modes the
  Action validity decision supports; revision-sensitive and
  identity-sensitive dependencies are refused at admission and have no tag.

A scoped Action reads no edge ranges (an edge read keeps the whole-frontier
rule), so its dependency slots are node records, attachments (atoms and
portals), and edges. Descriptors sort by slot encoding (defined below), then
kind tag, then mode tag; each encodes as kind tag, slot encoding, mode tag;
and the dependency-set digest hashes them in that order.

**Canonical set encoding.** The read-set, target-set, write-set,
dependency-set, and compared-values digests each hash a set of slot entries,
never a collection in discovery order. A slot encodes as its class tag followed
by its identity, all at fixed width: for a node record, its `NodeKey` (warp id,
then local id); for an edge, its `EdgeKey`; for an attachment, an owner tag
(`0` node, `1` edge), the owner's key, and a plane tag (`0` alpha, `1` beta);
for a range key, the source `NodeKey` followed by the edge-type id. Entries
sort by slot class tag, then by those identity bytes compared
lexicographically. Each entry is the slot encoding followed by what that set
records for the slot:

- read set: an outcome tag (`0` present, `1` absent) and, when present, the
  32-byte value digest defined below; a range read records the observed edges
  as a list of edge id and target node id pairs, sorted by edge id and
  preceded by its count as a little-endian `u64`;
- target set and write set: the slot alone;
- dependency set: the descriptor encoding defined under
  [Dependency descriptors](#evidence) above;
- compared values: one entry per dependency, in canonical descriptor order,
  each the descriptor encoding, a presence tag (`0` present, `1` absent at
  the commit parent as the pre-composition checks read it, or `2` not
  compared), and, for `0`, the 32-byte digest of the value compared, encoded
  as for the read set.

Each digest is BLAKE3 over a domain tag naming the set and its version, the
entry count as a little-endian `u64`, and the entries in order. Live
execution and recovery therefore produce identical bytes whatever order an
implementation visits slots in.

Slot class tags are node record `0`, attachment `1`, edge `2`, and range key
`3`.
Ids, type ids, and warp ids are 32 bytes each. A present slot's value digest
is BLAKE3 over a class-specific domain tag and its canonical fields:

- node record: `echo.bridge.node-value.v1\0`, then the node type id;
- attachment: `echo.bridge.attachment-value.v1\0`, then for an atom tag `0`,
  its `type_id`, its byte length as a little-endian `u64`, and its bytes, or
  for a portal tag `1` and its target warp id. `expectedAtomDigest` is this
  digest of the expected atom;
- edge: `echo.bridge.edge-value.v1\0`, then the edge type id and the local
  `from` and `to` node ids.

The set digests use the domain tags `echo.bridge.read-set.v1\0`,
`echo.bridge.target-set.v1\0`, `echo.bridge.write-set.v1\0`,
`echo.bridge.dependency-set.v1\0`, and `echo.bridge.compared-values.v1\0`.

The submission identity is present in every variant. An identified submission
whose evaluation produced no proposal is a different evidence state from an
evaluated proposal, and the receipt keeps them distinct.

**Proposal encoding.** The proposal identity is BLAKE3 over
`echo.bridge.effect-proposal.v1\0` followed by:

1. the Action: its installed-operation identity and its canonical invocation
   digest, as ADR 0023 binds them;
2. the evaluation basis: the `EchoOperationEvaluationBasisV1` identity;
3. the read-set digest, which stands for `readSet`;
4. the effect count as a little-endian `u64`, then each effect's canonical
   encoding (see [EffectProposal](#effectproposal)) in canonical order;
5. the result: its length as a little-endian `u64`, then its bytes;
6. the cost: steps, read bytes, write bytes, and allocated bytes, each a
   little-endian `u64`.

Effect kind tags are `EnsureNode` `0`, `CreateNode` `1`, `ReplaceAtom` `2`,
`EnsureEdge` `3`, and `ReplaceEdge` `4`.

The proposal identity is `H(Action, evaluation basis, canonical
EffectProposal)` in this encoding, and the canonical `EffectProposal` contains
every effect in canonical order, so the identity binds the exact evaluated
effects (invariant 5). The target-set digest and, when a patch was derived, the
write-set and patch digests are derived from those same effects. Proposals are
not retained apart from their Tick; the binding is checked by re-deriving the
proposal, as the recovery matrix below requires for every class that carries
one.

Today every `BasisChanged` is decided before evaluation runs
(`echo_operation.rs`). For bridge Actions that remains true only of the
whole-frontier case; a dependency-scoped `BasisChanged` is decided after
evaluation and keeps its evidence (see Basis, budget, retries).

These fields change the receipt entry's encoding, which commit hash v3 binds,
and its retained layout. `EPRV0003` has no per-entry length or version, so a
legacy decoder would read the new fields as blocker data. The new evidence is
therefore written only under a new retained provenance record version,
`EPRV0004`, and a new receipt digest version, 5. The retained magic alone is
not enough, because it sits outside the receipt digest that commit hash v3
binds: receipt digest version 5 puts every entry's proposal posture and, for
bridge entries, the target-set digest, patch tag with its digests, read-set
digest, validity rule with its scoped evidence, and consumed budget into the
digest's preimage. Two validations that compared different values therefore
have different receipt identities, and a version-4 receipt identity never
changes meaning. Ticks that mix bridge and `NotBridge` entries encode in
`EPRV0004` without fabricated evidence.

**Receipt layout.** An `EPRV0004` entry is the `EPRV0003` entry fields,
unchanged, followed by a bridge section preceded by its byte length as a
little-endian `u64`. Integers are little-endian `u64` and digests and
identities are 32 bytes. The section is:

1. the posture tag: `NotBridge` `0`, `NoProposal` `1`, `Proposed` `2`. A
   `NotBridge` section ends here;
2. for `Proposed`: the proposal identity, the target-set digest, and the
   patch tag (`NotDerived` `0`, `Derived` `1`), followed for `Derived` by
   the write-set digest and the patch digest;
3. the read-set digest;
4. the validity tag (`WholeFrontier` `0`, `Scoped` `1`). For `Scoped`: the
   origin basis identity, the validation position (the Action's index in
   the Tick's canonical candidate order), and an evidence tag
   (`OriginOnly` `0`, `HistoryChanged` `1`, `Compared` `2`). For
   `HistoryChanged`: the dependency-set digest and the changed dependency's
   descriptor encoding. For `Compared`: the dependency-set digest and the
   compared-values digest;
5. the consumed budget against the delegated `EchoOperationBudgetV1` axes,
   steps, read bytes, and write bytes, followed by the evaluator's allocated
   bytes;
6. the outcome coordinate: `0` none (committed, or
   `RejectedFootprintConflict`), `1` a bridge obstruction kind followed by
   its one-byte code, or `2` an authored obstruction (such as
   `BasisNotCanonical`) followed by its 32-byte package-declared obstruction
   identity, qualified by the package and installed operation as the ADR
   0025 supersession states. Bridge kind codes are `BasisChanged` `0`,
   `BudgetExceeded` `1`, `DuplicateEffectTarget` `2`, `IdentityLawViolation`
   `3`, `UnreachableWarp` `4`, `SchemaViolation` `5`, `FootprintLawViolation`
   `6`, `NodeMissing` `7`, `PreconditionMismatch` `8`,
   `ContentIdentityConflict` `9`, `DanglingReference` `10`, and
   `FootprintMismatch` `11`.

Every tag is one byte. Receipt digest version 5 hashes, in place of each
version-4 entry, that entry's version-4 bytes followed by its bridge section
exactly as `EPRV0004` retains it, length included, under the version-5
domain. An `EPRV0004` decoder refuses a section whose length disagrees with
its contents or that carries trailing bytes.

**Retained Action outcome.** Today's `EOACT003` record writes an obstruction
as a code from `EchoOperationObstructionKindV1`, which has none of the
bridge kinds and no authored obstruction. A bridge Action's retained outcome
is therefore written as `EOACT004`: the `EOACT003` layout with its one-byte
obstruction kind replaced by the outcome coordinate above. Fixed-profile
Actions keep `EOACT003`, its decoder is unchanged, and each record decodes
by its own magic. Recovery requires the `EOACT004` coordinate to equal the
receipt's bridge-section coordinate.
The implementation adds this row to
[Submission Attribution's format table](../topics/SubmissionAttribution.md#format-binding-and-legacy)
when it lands:

| Receipt                                   | Receipt digest | Retained record | Commit hash |
| ----------------------------------------- | -------------- | --------------- | ----------- |
| Commit-bound section with bridge evidence | version 5      | `EPRV0004`      | v3          |

The `EPRV0003` decoder and receipt digest version 4 stay unchanged, retained
legacy Ticks recover through them byte for byte, a mixed history decodes each
record by its own magic, and an `EPRV0003` record never carries the new
fields.

### Recovery

The bridge keeps Echo's existing recovery verification and extends it to the
new evidence. Verification is not re-admission.

- A decided Tick is recovered from its WAL record by replaying its patch.
  Echo then checks the patch digest and the commit hash, which binds the
  receipt and its attribution section, so the recovered evidence is exactly
  the evidence that was committed. Recovered state comes from the committed
  patch, never from a re-derived proposal.
- Recovery keeps the per-class verification described in
  [Current implementation](#current-implementation) and adds the new evidence
  to whatever each class already compares, as the matrix below states. Any
  disagreement refuses the history.
- The comparison never changes a decision. The recorded outcome, receipt, and
  attribution stand as written; a re-derived proposal is checked against them
  and then discarded.
- An Action recorded as `Scoped` follows
  [Action validity across ticks](../topics/GeneratedRules.md#action-validity-across-ticks):
  for every `Proposed` outcome, recovery recomputes its complete dependency
  set (every witnessed read and every effect precondition) and the
  written-slot set from the replay operations. It then repeats what live
  execution did, in the same order: the history check, the pre-composition
  checks against the commit parent, and the batch composition, and it
  rebuilds the recorded evidence form from the comparisons those steps
  actually make. A `RejectedFootprintConflict` is verified through its
  preparation against the commit parent and its recomputed blockers, never
  by validating it at its validation position. Recovery refuses a history
  whose evidence form, dependency-set digest, compared-values digest,
  changed dependency, or result differs. For an evaluation obstruction it
  re-evaluates at the origin.
- An undecided Action stays the same pending submission and is evaluated when
  the scheduler next selects it.

Recovery matrix. "Existing check" is what recovery verifies today for that
class; "Added" is what the bridge adds. "Effects bound" means the re-derived
effects must hash to the recorded proposal identity and reproduce the
recorded target-set digest and patch tag, with the write-set and patch
digests when it is `Derived` (invariant 5).

Two checks apply to every row and are not repeated in it, because recovery
already performs them for every Action Tick: the basis posture is recomputed
against the scheduler Tick's parent, and the whole Action batch's composition
is reconstructed and every reconstructed outcome must equal the recorded one.
The bridge keeps both and adds the outgoing-edge range keys and precondition
reads to the reconstructed composition. The scoped-validity recomputation
above likewise applies to every row recorded as `Scoped`.

| Outcome                                                                                                                                                                                                                                                            | Posture                  | Existing check                                                            | Added                                                                                                                                                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------ | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Committed                                                                                                                                                                                                                                                          | `Proposed`, `Derived`    | replay patch; reconstruct preparation; compare identities                 | effects bound; read-set digest; patch digest equals the replayed patch                                                                                                                                               |
| `RejectedFootprintConflict`                                                                                                                                                                                                                                        | `Proposed`, `Derived`    | reconstruct preparation; compare identities; recompute recorded blockers  | effects bound; read-set digest; blockers include range-key conflicts                                                                                                                                                 |
| Obstructed at validation (`DanglingReference`, `ContentIdentityConflict`, `DuplicateEffectTarget`, `FootprintLawViolation`, `IdentityLawViolation`, `SchemaViolation`, `UnreachableWarp`, `NodeMissing`, `PreconditionMismatch`, validation-read `BudgetExceeded`) | `Proposed`, `NotDerived` | re-evaluate; compare pre-decision evidence                                | effects bound; read-set digest; obstruction kind; no write-set or patch digest present                                                                                                                               |
| Obstructed by `FootprintMismatch` (after derivation)                                                                                                                                                                                                               | `Proposed`, `Derived`    | re-evaluate; compare pre-decision evidence                                | effects bound; read-set digest; re-derived patch reproduces the mismatch                                                                                                                                             |
| `BudgetExceeded`, scheduler-assigned (Action-batch limit)                                                                                                                                                                                                          | `Proposed`, `Derived`    | accept reconstructed preparation; verify through the batch reconstruction | effects bound; read-set digest                                                                                                                                                                                       |
| `BudgetExceeded` during evaluation, or an authored guard                                                                                                                                                                                                           | `NoProposal`             | re-evaluate; compare pre-decision evidence                                | re-evaluation, at the origin for a `Scoped` Action, again yields no proposal and the same obstruction; read-set digest                                                                                               |
| `BasisChanged`, whole-frontier (before evaluation, including unknown history)                                                                                                                                                                                      | `NoProposal`             | admission evidence; recomputed basis posture; no re-evaluation            | none; posture must be `NoProposal` with an empty read set and zero consumed budget                                                                                                                                   |
| `BasisChanged`, dependency-scoped (after evaluation, before validation)                                                                                                                                                                                            | `Proposed`, `NotDerived` | none today: current programs decide this before evaluation                | re-evaluate at the origin; effects bound; read-set digest; recompute the written-slot set from replay operations, the complete dependency set (witnessed reads and effect preconditions), and the changed dependency |
| Any outcome of a `NotBridge` entry                                                                                                                                                                                                                                 | `NotBridge`              | unchanged                                                                 | none; no bridge field present                                                                                                                                                                                        |

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
5. **Proposal**, in canonical order: node records, then atoms, then edges.
   Within a class the order follows id bytes, which these symbolic names do
   not show.
    - `EnsureNode T2 {bytes "hello there", lineBreaks 0}`
    - `EnsureNode H2 {byteLength 11, lineCount 1, sequence 5}`
    - `ReplaceAtom B expected digest(version 7), now {version 8}`
    - `EnsureEdge H2 -root-> T2`
    - `EnsureEdge H2 -anchor-> I`
    - `ReplaceEdge B -canonicalHead-> expected H1, now H2`
6. **Validation.** The read and write sets lie inside the footprint law. Echo
   recomputes `T2` and `H2` from their atoms and children under the declared
   identity law; a mismatch would obstruct with `IdentityLawViolation`.
   Endpoint preconditions check: `T2` and `H2` are created here, and `I` and
   `B` exist at `c`. The expected `H1` and version-7 digest hold at `c`. The
   budget holds. The Action reads edges, so its validity rule is
   `WholeFrontier`.
7. **Composition.** The write set is the eight target slots `T2`, `T2.atom`,
   `H2`, `H2.atom`, `H2.root`, `H2.anchor`, `B.canonicalHead`, and `B.atom`,
   plus the range keys `(H2, root)`, `(H2, anchor)`, and `(B, canonicalHead)`
   of the edges it creates or retargets. Its reads add the precondition slots:
   endpoints `I` and `B` exist, and `B.atom` and `B.canonicalHead` hold their
   expected values. No co-scheduled Action reads or writes these.
8. **Patch**, derived by Echo: `UpsertNode T2`, `SetAttachment T2`,
   `UpsertNode H2`, `SetAttachment H2`, `UpsertEdge H2.root`,
   `UpsertEdge H2.anchor`, `UpsertEdge B.canonicalHead` (same edge id, new
   target), `SetAttachment B`. Eight operations, whose actual footprint is
   exactly the write set above: each `SetAttachment` writes an atom slot and
   each `UpsertEdge` writes its edge and range key.
9. **Decision and durability.** The receipt binds the proposal posture
   `Proposed` with the proposal identity, the target-set digest, the patch tag
   `Derived` with the write-set and patch digests, the read-set digest, the
   validity rule `WholeFrontier`, and the consumed budget.
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
   first joins the Tick; the second gets `RejectedFootprintConflict`. Its
   posture is `Proposed` with a `Derived` patch, whose write-set and patch
   digests are recorded, but no patch of it is committed.
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

| Property                      | Test                                                                                                                                                                                                                                                                                                                                                                                                          |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Atomicity                     | A failure on the last effect of a multi-node proposal leaves the parent state root and patch unchanged.                                                                                                                                                                                                                                                                                                       |
| Ensure identity               | Identical existing node: no patch entry. Different existing node: `ContentIdentityConflict`.                                                                                                                                                                                                                                                                                                                  |
| Partial occupancy             | A node record without its atom, or an atom without its record: `CreateNode` obstructs with `PreconditionMismatch` and `EnsureNode` with `ContentIdentityConflict`; neither overwrites the orphan slot.                                                                                                                                                                                                        |
| Witnessed footprint           | A program that reads a slot outside its law obstructs, even if its writes are in bounds.                                                                                                                                                                                                                                                                                                                      |
| Stale basis                   | `ReplaceAtom` with a changed expected digest, or `ReplaceEdge` with a moved `expectedTo`, obstructs with `PreconditionMismatch`, posture `Proposed`, `NotDerived`, and no patch.                                                                                                                                                                                                                              |
| Budget                        | Exhausting steps or bytes obstructs with `BudgetExceeded` and `NoProposal`; a proposal whose patch alone exceeds the Action-batch operation limit obstructs with `BudgetExceeded`, posture `Proposed` with its digests, and no committed patch.                                                                                                                                                               |
| Atom write bytes              | A proposal whose atom bytes exceed the delegated `write_bytes` obstructs with `BudgetExceeded` and `NoProposal`; a whole-frontier `BasisChanged` records zero consumed budget.                                                                                                                                                                                                                                |
| Validation read budget        | A `ReplaceAtom` of a stored atom larger than the remaining `read_bytes` obstructs with `BudgetExceeded`, posture `Proposed`, `NotDerived`, before hashing it; allocation beyond `edict_pure`'s `max_allocated_bytes` obstructs with `BudgetExceeded` and `NoProposal`.                                                                                                                                        |
| Budget-stopped evidence       | A scoped proposal whose validation read exhausts the budget records `Compared` with that dependency, and every later one, as not compared; no value past the stop is hashed.                                                                                                                                                                                                                                  |
| Determinism                   | Same package, invocation, and evaluation basis give byte-identical proposals, proposal identities, read-set and write-set digests, and derived patches, whether the Action is evaluated alone or beside other Actions. Receipts and composite patches are compared only for identical Tick contexts.                                                                                                          |
| Canonical order               | Emitting the same effects in different orders yields the same canonical `EffectProposal` bytes and proposal identity.                                                                                                                                                                                                                                                                                         |
| Encoding vectors              | Fixed byte vectors for the proposal encoding, every value digest, every set digest (including a descriptor list with both comparison modes), and an `EPRV0004` entry of each posture, validity, and evidence tag reproduce the specified bytes and receipt-v5 digests.                                                                                                                                        |
| Canonical slots               | Differently expressed effects on one slot (`EnsureEdge` and `ReplaceEdge` of one edge id; `CreateNode` and `ReplaceAtom` of one node) obstruct with `DuplicateEffectTarget`.                                                                                                                                                                                                                                  |
| Write set                     | The derived patch's actual footprint equals the write set, including the atom slots of created nodes and the range key of every created or retargeted edge, so a committed `EnsureEdge` and `ReplaceEdge` pass the footprint check.                                                                                                                                                                           |
| Identity law                  | A proposed node or edge id that does not match the package's identity law obstructs with `IdentityLawViolation`, posture `Proposed`, `NotDerived`, and recovery reproduces it.                                                                                                                                                                                                                                |
| Unreachable warp              | An effect naming a key in a missing warp, or in a warp no valid portal chain reaches, obstructs with `UnreachableWarp`, posture `Proposed`, `NotDerived`.                                                                                                                                                                                                                                                     |
| Schema                        | An effect whose node type, atom `type_id`, or edge type the package's schema does not admit obstructs with `SchemaViolation`, posture `Proposed`, `NotDerived`, and recovery reproduces it.                                                                                                                                                                                                                   |
| Validation order              | A proposal that breaks several rules (for example a duplicate target, an unadmitted type, and a missing endpoint) records the obstruction of the earliest step in the validation order, live and on recovery.                                                                                                                                                                                                 |
| Footprint stages              | A target outside the footprint law obstructs with `FootprintLawViolation` and `NotDerived`, and so does a `ReplaceEdge` whose basis edge's range key lies outside the law; a derived patch whose footprint differs from the write set obstructs with `FootprintMismatch` and `Derived`.                                                                                                                       |
| Restored value (ABA)          | A dependency written A to B and back to A between origin and commit parent is found through the written-slot set and obstructs with `BasisChanged`, including when the intervening patch under-declared its `out_slots`; with one replay patch in the interval missing, the Action obstructs before evaluation as a whole-frontier `BasisChanged`, posture `NoProposal`, with an empty read set.              |
| Scoped preconditions          | A dependency-scoped Action whose `ReplaceAtom` expected value, or one of whose edge endpoints, changed and was restored between its origin and the commit parent obstructs with `BasisChanged`, though the program never read that slot; recovery reproduces the outcome and its changed dependency.                                                                                                          |
| Scoped conflict recovery      | A scoped Action rejected because an earlier Action in its Tick replaced a value it expects recovers as `RejectedFootprintConflict`, not `PreconditionMismatch`.                                                                                                                                                                                                                                               |
| Scoped evidence forms         | An evaluation obstruction records `OriginOnly`, a dependency-scoped `BasisChanged` records `HistoryChanged` with no compared values, and every other scoped outcome records `Compared` over commit-parent values; none is compared at a validation position that includes an earlier overlapping write.                                                                                                       |
| Retry basis                   | A new submission selected after the head advanced is evaluated at its recorded basis, not at the selection-time state.                                                                                                                                                                                                                                                                                        |
| Receipt digest v5             | A version-5 receipt digest changes when any bridge field changes, including the validation position, a dependency's comparison mode, or a compared value; version-4 receipt identities are unchanged.                                                                                                                                                                                                         |
| Proposal binding              | For every `Proposed` outcome class in the recovery matrix, including rejected ones, altering one recorded effect-derived digest or the proposal identity refuses the history.                                                                                                                                                                                                                                 |
| No-proposal evidence          | An evaluation that obstructs before proposing records `NoProposal` with its read-set digest and no proposal, write-set, or patch digest.                                                                                                                                                                                                                                                                      |
| Recovery                      | Crash before and after the decided-Tick WAL record; fresh-host recovery reproduces the same state root and receipts.                                                                                                                                                                                                                                                                                          |
| Composition                   | Two Actions with independent witnessed footprints (neither writes a slot the other reads or writes) commit in one Tick. A write/write overlap obstructs the later one, and so does a write/read overlap: A reads X and writes Y, B writes X and Z.                                                                                                                                                            |
| Shared structure              | Two Actions that `Ensure` the same node already present in the basis both commit in one Tick.                                                                                                                                                                                                                                                                                                                 |
| Same-Tick identical create    | Two Actions, dependency-scoped included, that both `Ensure` the same node absent at the commit parent: the later obstructs with `RejectedFootprintConflict` instead of reading the earlier one's write.                                                                                                                                                                                                       |
| Referential integrity         | An edge to a node neither in the basis nor created by the proposal obstructs with `DanglingReference`; a pre-existing dangling edge elsewhere in the basis does not obstruct an unrelated proposal.                                                                                                                                                                                                           |
| Portal chain                  | An effect in a descended warp records every portal on its root-to-target chain as a read; a co-scheduled portal retarget conflicts with it, and a portal changed and restored between origin and commit parent obstructs a scoped Action with `BasisChanged`.                                                                                                                                                 |
| Visibility                    | No published coordinate shows a retargeted head without every node and edge the proposal introduced; pre-existing structure the head reaches is not validated.                                                                                                                                                                                                                                                |
| Witnessed reads               | The receipt's read set equals the slots the view resolved, including for a program that reads and then obstructs.                                                                                                                                                                                                                                                                                             |
| Negative reads                | An Action that observed a node, atom, or edge set absent obstructs when a co-scheduled Action creates it, including a fresh edge of the observed source and type with a new edge id when the observed set was empty (outgoing-edge range key).                                                                                                                                                                |
| Edge-read basis               | An Action that read edges commits only onto its exact evaluation basis.                                                                                                                                                                                                                                                                                                                                       |
| Final outcomes                | A rejected Action's outcome, receipt, and attribution are unchanged after a later submission of the same edit succeeds.                                                                                                                                                                                                                                                                                       |
| Duplicate resubmission        | Resubmitting a decided Action's identical content returns `Duplicate` with the original outcome and commits no Tick.                                                                                                                                                                                                                                                                                          |
| Attribution                   | Every Action in a bridge Tick appears once in the attribution section, mapped to its own receipt entry.                                                                                                                                                                                                                                                                                                       |
| One effect per slot           | Two effects on one slot (two `ReplaceAtom`, or `EnsureNode` plus `ReplaceAtom` of the same node) obstruct with `DuplicateEffectTarget`, posture `Proposed`, `NotDerived`.                                                                                                                                                                                                                                     |
| Scoped basis change           | A dependency-scoped Action evaluated at its origin obstructs with `BasisChanged`, posture `Proposed` with `NotDerived`, and keeps its proposal, read set, and changed dependency when a dependency changed, even if its proposal would also fail validation; an unchanged dependency set lets it commit.                                                                                                      |
| Scoped evaluation obstruction | A dependency-scoped Action whose evaluation exhausts its budget, or fails an authored guard, keeps that obstruction with `NoProposal` even when one of its reads changed after the origin; recovery reproduces it.                                                                                                                                                                                            |
| Receipt format                | `EPRV0003` records recover byte for byte through the unchanged decoder; new Ticks are written as `EPRV0004`; a Tick mixing a bridge Action with fixed-profile Actions encodes the latter as `NotBridge`; a history mixing both formats recovers.                                                                                                                                                              |
| Outcome record                | A bridge obstruction, including an authored one such as `BasisNotCanonical`, is retained as `EOACT004` with the same coordinate as its receipt; fixed-profile Actions still write `EOACT003`, and mixed histories recover.                                                                                                                                                                                    |
| Recovered evidence            | For each outcome class, recovery performs that class's existing verification plus the new comparisons (posture, proposal identity, read-set, write-set, and patch digests where the class reconstructs or re-evaluates); a tampered digest refuses the history, a whole-frontier `BasisChanged` is not re-evaluated, the batch composition is reconstructed for every class, and no recorded outcome changes. |
| Pending recovery              | An Action lost with its Tick before the WAL record is recovered as the same submission, not a new one.                                                                                                                                                                                                                                                                                                        |

## Open questions

- How `edict_read` exposes outgoing edges by type, and how a bounded closure
  read guided by a pure Edict selector is expressed and budgeted.
- Whether the footprint law is per-invocation or per-package with parameters.
- Which domain-separated hash Edict source can call to derive ids.
- Whether proposals larger than the Action-batch operation limit should ever
  be committable. This design says no: such a proposal is still representable
  as evidence (`BudgetExceeded`, posture `Proposed` with a `Derived` patch, as
  [Basis, budget, retries](#basis-budget-retries) states), but it never
  commits and is never split across Ticks.
