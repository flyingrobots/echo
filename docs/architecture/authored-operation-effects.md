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
      (an Action-batch limit, decided after a successful evaluation) is accepted
      as a reconstructed preparation and verified through the batch
      reconstruction below. Finally, for every Action Tick and every outcome
      class, it reconstructs the whole Action batch's composition over the
      recovered parent state and refuses history where any reconstructed outcome
      differs from the recorded one; for each `RejectedFootprintConflict` it
      also recomputes the blocking earlier Actions and requires them to equal
      the recorded blockers.
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
- **Patch constructor.** The derived patch is built by
  `WarpTickPatchV1::new`, as the fixed-profile path builds its patch, with
  these arguments:
    - `policy_id`: the scheduler's policy id for the Tick;
    - `rule_pack_id`: the installed-operation identity;
    - `commit_status`: `Committed`;
    - `in_slots`: every node, edge, and attachment slot in the read set, the
      precondition and resolution slots, and the portal chains. A range key
      has no `SlotId` form, so it is not listed here; the write-set and
      read-set digests cover it;
    - `out_slots`: the write set's node, edge, and attachment slots;
    - `ops`, in canonical effect order: `UpsertNode` then `SetAttachment` for
      a creating `EnsureNode` or `CreateNode`, and nothing for an identical
      `EnsureNode`; `SetAttachment` for `ReplaceAtom`; `UpsertEdge` for a
      creating `EnsureEdge`, and nothing for an identical one; and
      `UpsertEdge` with the same id and the new target for `ReplaceEdge`.

    The constructor sorts and deduplicates slots and operations itself. The
    patch digest is `WarpTickPatchV1::digest` of that value, which hashes every
    argument above.

- **Canonical order.** Echo sorts the effects the evaluator emits; the order in
  which evaluation produced them never reaches the identity. Each effect has a
  primary slot: the node record for `EnsureNode` and `CreateNode`, the atom for
  `ReplaceAtom`, and the edge for `EnsureEdge` and `ReplaceEdge`. Effects sort
  by the primary slot's class (node record, then attachment, then edge), then by
  its warp id bytes, then its local id bytes, compared lexicographically. A tie
  can occur only in a proposal that obstructs with `DuplicateEffectTarget`; it
  breaks by effect kind (`EnsureNode`, `CreateNode`, `ReplaceAtom`,
  `EnsureEdge`, `ReplaceEdge`), then by the effect's `EffectRow` bytes with its
  `atom` range zeroed, then by its atom bytes (see [Encoding and
  memory](#encoding-and-memory)); every semantic field of the effect, expected
  values included, is in those bytes. Blob ranges are assigned only after
  sorting, so emission order never reaches them. Sorting never removes an
  effect, so duplicates stay visible to the one-effect-per-slot rule.
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
  warp, the footprint and a scoped Action's dependency set (kind `5`)
  include every portal attachment on the validated root-to-target
  reachability chain. Portals an effect's chain crosses are not part of the
  read set, which holds exactly what the program's view resolved, as
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

### Identity laws

A package names its identity law by number, and Echo recomputes ids under it;
a digest alone could not be executed. One law is defined:

- **`1`, `ContentAddressV1`.** It applies by node type: the package's
  schema marks each node type `content` or `mutable`, and a record's type,
  which every record carries, decides, so no creation history is needed. An
  edge follows its source node's type.
    - A **content** node is created only by an `EnsureNode` whose id is its
      content address, together with every outgoing edge it will ever have,
      each by an `EnsureEdge` in the same proposal. It is immutable
      afterwards: a `ReplaceAtom` of it, a `ReplaceEdge` of an edge from it,
      a creating `EnsureEdge` from it in a later proposal, or a `CreateNode`
      of a content type is `IdentityLawViolation`.
    - A **mutable** node, such as an application's anchor (the buffer `B`
      in the trace), has a program-chosen id. It is created only by a
      `CreateNode`, which requires the id to be absent, and it may be
      replaced. An `EnsureNode` of a mutable type is `IdentityLawViolation`.
    - A content node's id is BLAKE3 over `echo.bridge.node-id.v1\0`, the
      warp id, the node type id, the attachment value digest of its atom (see
      [Evidence](#evidence)), the number of its outgoing edges as a
      little-endian `u64`, and, for each edge sorted by edge type id and then
      `to` local id, the edge type id and the `to` local id. Those are exactly
      the edges its creating proposal's `EnsureEdge` effects give it.
    - An edge from a content node has the content id BLAKE3 over
      `echo.bridge.edge-id.v1\0`, the warp id, the `from` local id, the edge
      type id, and the `to` local id. It is immutable, like its source.
    - An edge from a mutable node has the slot id BLAKE3 over
      `echo.bridge.slot-edge-id.v1\0`, the warp id, the `from` local id, and
      the edge type id: one edge per source and type, whose id stays the same
      when a `ReplaceEdge` retargets it. A creating `EnsureEdge` of it must
      carry that id. Because a non-bridge writer can store an edge under any
      id, step 2 reads the source's whole outgoing bucket, charged per edge
      like a range read, for every `EnsureEdge` and `ReplaceEdge` from a
      mutable source. Any other edge of the same type from that source is an
      alias, and the effect obstructs with `IdentityLawViolation`.

    An `EnsureNode` whose content-typed target already exists adopts it only
    if the existing node's record, atom, and complete outgoing edge set equal
    what the proposal gives it. Echo then recomputes the id from those, so a
    node written under some other id is never adopted; a mismatch is
    `IdentityLawViolation`.

    Echo recomputes every id a proposal creates, and the id of every edge a
    `ReplaceEdge` or an identical `EnsureEdge` resolves from the basis (its
    slot or content id from the resolved record's source, type, and, for a
    content edge, target), and refuses a mismatch with `IdentityLawViolation`.
    A basis edge written by a non-bridge writer under some other id is
    therefore never adopted. `ReplaceAtom` names an existing mutable node,
    whose id is program-chosen.
    All ids are 32 bytes. A content node's id therefore commits to its content
    and its children, as Jim's rope requires, and no later effect can make it
    stale. A program computes the same BLAKE3 inputs; Edict must expose them
    before an effect package can be produced.

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

A range read is checked and metered before it materializes anything. A
range read of an edge type outside the footprint law's `range_edge_types`
obstructs evaluation at once (`FootprintLawViolation`, posture `NoProposal`),
before it reads anything, so a package whose law admits no range read can
never produce a proposal that depends on one, and its static scoped
eligibility holds. For an admitted type, the view takes the length of the
source node's whole outgoing bucket, without copying an edge. `GraphStore`
keeps one bucket per source, not per type, so finding the matching edges
inspects every edge in it; the view therefore charges one step and 64
`read_bytes` for every edge in the bucket, matching or not, and compares the
bucket length, not the match count, with `max_range_edges`. If the length
exceeds `max_range_edges`, or the charge exceeds the remaining budget,
evaluation obstructs with `BudgetExceeded`, posture `NoProposal`, before any
edge is inspected. Otherwise the view writes the matching edges'
`EdgePairRow`s directly into the arena. Consumed cost and the read set
therefore depend only on the basis, and a high-degree node cannot force work
or allocation the budget did not cover.

These dependency kinds follow the
[Action validity across ticks](../topics/GeneratedRules.md#action-validity-across-ticks)
decision (#837). Value and single-key absence dependencies are the kinds that
decision
supports. A read of outgoing edges by type is a topology dependency, which that
decision does not yet support. Eligibility is decided statically at admission,
never by the branch an evaluation takes. An Action keeps the whole-frontier
rule, whatever this invocation does, and commits only onto the exact basis it
was evaluated against, when its package's footprint law admits any of:

- an outgoing-edge range read (a non-empty `range_edge_types`);
- an edge write (a non-empty `write_edge_types`). Creating or retargeting an
  edge from a mutable source requires that no other edge of its type leave
  that source, and that depends on the source's whole bucket;
- a write of a content-addressed node type. Adopting an existing content node
  requires its whole outgoing edge set to match.

Each of these is a topology dependency: a write from outside the bridge (the
unrestricted `UpsertEdge` path) can add an edge to a bucket without changing any
single slot a scoped Action recorded. Every other bridge Action is eligible for
dependency-scoped validity, and its writes are node records and atoms of mutable
types. Widening that requires sound edge-set tracking under #837, not a looser
rule here.

The view resolves each slot on demand from the basis, as
[Sliced Readings and Copy-on-Write Strands](sliced-readings-and-copy-on-write-strands.md)
law 1 requires. The contract never depends on a complete materialized graph:
the read limit and the footprint law bound the slice. A range read beyond
`max_range_edges` or the budget obstructs evaluation itself (`BudgetExceeded`,
`NoProposal`), since it would materialize data; any other read outside the
footprint law is found at validation step 7 (`FootprintLawViolation`,
`Proposed`, `NotDerived`).

A read that may find nothing names the type it expects: a node read names a node
type, and an atom read names an atom `type_id`. The view records that type with
an absent outcome (in the `ReadRow`'s `subject` field), so the footprint law's
`read_node_types` or `atom_types` decides whether the probe is permitted, live
and on recovery alike. A first implementation may still sit on today's fully
materialized worldline state, but nothing in this contract may rely on that.

## Footprint law

### Set encoding

Every set-valued field in a footprint law or a schema is a CBOR array of
distinct elements in strictly ascending order. An id is a 32-byte byte string,
ordered bytewise. A pair is a two-element array ordered by its first element,
then its second. Admission refuses a law or schema with an unsorted or
duplicated set, rather than normalizing it. Map keys are text strings, ordered
as `edict.canonical-cbor/v1` orders them. The complete shapes, in CDDL, are:

```text
footprint-law = {
  "read_node_types": [* id], "write_node_types": [* id],
  "atom_types": [* id], "range_edge_types": [* id],
  "write_edge_types": [* id], "portal_owner_types": [* id],
  "max_reads": uint, "max_writes": uint, "max_range_edges": uint,
}
graph-schema = {
  "node_types": { * id => node-type },
  "edge_types": { * id => [* endpoint-pair] },
}
node-type = { "addressing": 0 / 1, "atom_types": [* id] }  ; 0 content, 1 mutable
endpoint-pair = [source: id, target: id]
id = bstr .size 32
```

No other key may appear, and every key shown is required, so each law and
schema has exactly one encoding and one identity.

The package declares a footprint law, fixed per package with no invocation
parameters. It is an `edict.canonical-cbor/v1` map, in the exact shape that [Set
encoding](#set-encoding) gives, with these entries: `read_node_types` and
`write_node_types` (sets of node type ids), `atom_types` (the atom `type_id`s it
may read or write), `range_edge_types` (edge types it may read as outgoing
ranges; empty makes it eligible for scoped validity), `write_edge_types`, the
unsigned counts `max_reads`, `max_writes`, and `max_range_edges` (per range
read), and `portal_owner_types` (node types whose alpha portals an effect or
read in a descended warp may cross; empty confines the package to its root
warp). Each portal on a validated chain must be owned by a node of one of these
types, or validation obstructs with `FootprintLawViolation`. The bridge does not
traverse edge-owned (beta) portals: an edge has no node type for this law to
admit, so a chain crossing one is `FootprintLawViolation` at step 1, before any
owner record is read. The counts are over distinct slots, after deduplication:
`max_reads` bounds the witnessed read set, where one range read is one slot (its
range key) and its edges are bounded by `max_range_edges`; `max_writes` bounds
the resolved write set after Ensure resolution, range keys included, so an
identical Ensure adds nothing. Validation reads are bounded by the budget, not
by `max_reads`. The law's canonical bytes are part of the program's identity. At
validation Echo checks:

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

The schema check yields a typed `SchemaViolation` obstruction and the law
checks a typed `FootprintLawViolation`; both run before any patch is derived,
with patch tag `NotDerived`, at the steps [Validation order](#validation-order)
gives them, after the facts an effect does not carry have been resolved. The
patch equality can only run after derivation; a mismatch is a typed
`FootprintMismatch` obstruction with patch tag `Derived`, recording the patch
digest it compared. Neither commits a patch.
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

1. warp reachability, at the evaluation basis, of every effect key and every
   key in the witnessed read set that lies in a descended warp: root first,
   effect keys in canonical effect order and then read keys in slot order,
   each chain crossed once. Each portal crossed costs one step
   and 64 `read_bytes`, charged before it is read; a crossing the remaining
   budget cannot cover is `BudgetExceeded`. A portal owned by a node whose
   type is not in the footprint law's `portal_owner_types` is
   `FootprintLawViolation`; reading the owner's node record to check that
   costs one more step and 64 `read_bytes`. A key in a missing or unreachable
   warp is `UnreachableWarp`, decided before any record is looked up. A warp
   unreachable at a scoped Action's origin obstructs even if a later Tick
   made it reachable. Each portal attachment and each portal owner's node
   record are dependencies (kind `5`), so step 3 catches a change to either,
   including an owner whose type changed;
2. resolution at the evaluation basis (for a scoped Action, its origin): in
   canonical effect order, read every record validation needs. That is the
   target of each `EnsureNode` and `CreateNode`, both its node record and its
   alpha atom slot, and the target edge record of each `EnsureEdge`, present or
   absent, which decides whether the effect writes or only confirms; for an
   `EnsureNode` whose content-typed target exists, that node's whole outgoing
   bucket, charged per edge like a range read, which the identity law needs;
   each `ReplaceAtom`'s node record; each `ReplaceEdge`'s edge record; the
   source node record of every `EnsureEdge` and `ReplaceEdge`, whose type
   selects the edge-id rule, and, when that source is mutable, its whole
   outgoing bucket for the alias check; and the target node record of a creating
   `EnsureEdge` or a `ReplaceEdge`, each unless the proposal creates that node.
   Charges follow
   [Validation reads](#basis-budget-retries). Failures are `NodeMissing` for
   a `ReplaceAtom`'s missing node, `PreconditionMismatch` for a missing edge,
   or `BudgetExceeded`; whether a present target matches is decided at
   step 8, and a missing endpoint is reported as `DanglingReference` there.
   Every record read here is a dependency (kind `4`): `ValueDigest` when
   present and `Absence` when absent, so step 3 catches a target or endpoint
   that an intervening Tick created, changed, or removed. After this step the
   complete write set is known;
3. the dependency-scoped history check, for a scoped Action
   (`BasisChanged`);
4. one effect per slot (`DuplicateEffectTarget`);
5. the schema (`SchemaViolation`), before the identity law, which reads the
   schema's node-type addressing. Every node type, edge type, and atom
   `type_id` an effect carries or step 2 resolved must be admitted; a node
   type must admit its atom type (for `EnsureNode`, `CreateNode`, and a
   `ReplaceAtom`'s resolved owner); a `ReplaceEdge`'s preserved edge type
   must be admitted; and for every creating `EnsureEdge` and every
   `ReplaceEdge`, the schema must admit the edge between its source and
   target node types, taken from the resolved records or from the
   proposal's own node effects. An endpoint step 2 found absent has no type
   and is skipped here;
6. the identity law (`IdentityLawViolation`). An edge whose source step 2
   found absent, and the proposal does not create, has no source type to
   select its id rule, so this step skips it; it is `DanglingReference` at
   step 8, as an absent endpoint is at step 5;
7. the footprint law, now that every slot is known (`FootprintLawViolation`).
   The witnessed read set must lie inside the law's read permissions, by the
   type each read records. The target set and the resolved write set, with
   its range keys, must lie inside its write permissions. The counts must not
   exceed its maxima. Precondition and resolution slots (targets, endpoints,
   sources, and buckets that step 2 read) are implied by an effect, not read
   by the program. Each is authorized by the write permission of the effect
   that implies it, never by `read_node_types`, so an absent endpoint needs
   no expected type: it passes here and is `DanglingReference` at step 8;
8. the remaining effect preconditions, effect by effect in canonical order,
   and within an effect in the order its precondition list gives
   (`PreconditionMismatch`, `ContentIdentityConflict`, `DanglingReference`,
   or `BudgetExceeded` when a validation read exceeds the remaining budget);
9. patch derivation and the actual-footprint equality
   (`FootprintMismatch`);
10. composition (`RejectedFootprintConflict`);
11. the Action-batch operation limit (`BudgetExceeded`), tested only for a
    candidate with no blockers, as today's scheduler does.

Steps 1 through 8 yield `NotDerived`, and steps 9 through 11 yield
`Derived`. Within a step that examines several effects, the first failing
effect in canonical order decides. Steps 1, 2, and 8 can yield more than one
kind; within step 1 a crossing is charged, then its owner checked, then the
next portal read.

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
  basis: the materialized state at that coordinate, which replaying the
  worldline's patches determines exactly. That state includes records no
  longer reachable from a root, and a keyed read resolves them. This is
  observable bridge semantics, not a storage detail: a record leaves the
  state only through a patch that deletes it, and no host may drop one
  outside a Tick. Retention governs only the history before a coordinate.
  The state root does not commit to unreachable records, so a keyed read of
  one is bound by the coordinate and replay. Any future reclamation of
  unreachable records must either preserve what a keyed read observes, for
  example by deleting them in a Tick's patch, or ship as a new version of
  these semantics; it can never silently change what an Action reads. At commit,
  every `expected…` value and every recorded dependency that a co-scheduled
  Action wrote must still hold; an overlap with an earlier accepted Action in
  the same Tick is `RejectedFootprintConflict` (see **Within one Tick** below).
  Action validity across ticks keeps its existing rules, and an Action is never
  re-stamped as if it had been evaluated against a later basis.
- **Two kinds of basis change.** Today's executable programs have fixed,
  statically known reads, so Echo decides dependency-scoped validity before
  evaluation. A bridge Action's reads are known only by evaluating it, so the
  bridge splits basis change in two:
    - **Whole-frontier `BasisChanged`,** decided before evaluation: the Action
      is not eligible for dependency-scoped validity (its footprint law admits
      a range read, an edge write, or a content-node write; see
      [Witnessed reads](#witnessed-reads-over-a-bounded-slice)) and its evaluation basis is
      no longer the commit parent. Posture `NoProposal`, empty read set, no
      evaluation.
    - **Dependency-scoped,** decided after evaluation: the Action is eligible,
      so Echo evaluates it at its recorded origin basis, then checks its
      dependency set against the history between that origin and the commit
      parent. The dependency set is every witnessed read, presence and
      absence, plus every effect precondition defined in
      [EffectProposal](#effectproposal), whether or not the program read it,
      including each edge endpoint. Consistent with the Action validity across
      ticks decision, a dependency that changed in that history counts as
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
      unknown. Value comparison runs only at validation, against the commit
      parent, as the **Order** bullet below places it. No new versioning system
      is added.
    - **Unknown history is decided before evaluation.** The written set
      depends only on the origin and the commit parent, so Echo computes it
      before evaluating the Action. If any Tick in the interval has no
      retained replay patch, or its writes cannot be derived, the set is
      unknown and the Action takes the whole-frontier rule at once: since the
      origin is not the commit parent, it obstructs with whole-frontier
      `BasisChanged`, posture `NoProposal`, validity `WholeFrontier`, empty
      read set, zero consumed budget, no evaluation.
    - **Evaluation obstructions stand.** An evaluation that obstructs before
      producing a proposal (`BudgetExceeded`, or an authored guard) keeps
      that obstruction with posture `NoProposal`. Its later reads never ran
      and it has no effects, so its dependency set is incomplete, and it is
      never reclassified as `BasisChanged`. It commits nothing either way.
    - **Order.** For a proposal, the history check runs immediately after
      evaluation and the reachability and origin resolution of steps 1 and 2 of
      the [Validation order](#validation-order), and before any other
      validation. If a dependency changed, the Action obstructs with
      `BasisChanged`, posture `Proposed` with patch tag `NotDerived`, and keeps
      its witnessed read set and the dependency it names as changed. No patch is
      derived from a proposal whose dependencies changed, so a proposal that
      would also have failed validation (for example `DuplicateEffectTarget`)
      still has one encodable outcome. Only when every dependency holds does
      validation run. Echo never validates a delayed Action without its actual
      dependencies.
    - **Within one Tick, footprints before earlier writes.** Every check before
      composition, preconditions and expected values included, runs against the
      Tick's commit parent, never against writes accepted earlier in the same
      Tick. The exceptions are resolution and warp reachability, which run
      at the evaluation basis (see [Validation order](#validation-order)). Composition then reserves each candidate's complete footprint
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
- **Budget.** The delegated budget stays `EchoOperationBudgetV1`, with its three
  axes: steps, read bytes, and write bytes. Evaluation is metered as
  `edict_pure` already meters it. Each effect is charged when emitted, whatever
  validation later resolves: one step, plus `write_bytes` of 64 and the atom's
  byte length for `EnsureNode`, `CreateNode`, and `ReplaceAtom` (the charge
  today's compare-and-set makes for its replacement), or 128 for `EnsureEdge`
  and `ReplaceEdge`. A large payload therefore cannot fit a budget that would
  not cover its bytes. Exhausting any evaluation limit obstructs with
  `BudgetExceeded`, posture `NoProposal`, and no patch. Allocation is not a
  delegated axis: `edict_pure` already bounds it by its own
  `max_allocated_bytes`, the host ceiling intersected with the program's
  declared maximum, and exceeding that is likewise `BudgetExceeded` with
  `NoProposal`. The allocated bytes are recorded as evidence, never compared
  against the delegated budget.
- **Validation reads are charged.** Every slot validation reads is charged
  once per Action, before it is read, at the first step that reads it, and
  never again: a node record or an atom's presence costs one step and 64
  `read_bytes`, an edge record one step and 128 `read_bytes`, and hashing an
  atom's value adds its byte length. A slot the program's view already read
  is not charged again, since its value is known. So step 1 charges portals
  and their owners' records, for effect and read keys alike, step 2 charges
  each resolved record, including every Ensure and Create target with its
  alpha slot, every edge source, and the outgoing bucket of an existing
  content node, per edge, and step 8 charges only the atoms it hashes,
  for `ReplaceAtom`'s expected digest and an existing `EnsureNode`'s
  identical-content check. Charges come from the
  budget left after evaluation. A read the remaining budget cannot cover
  obstructs with `BudgetExceeded`, posture `Proposed`, `NotDerived`, at that
  step. The receipt's consumed budget is evaluation and validation together;
  `allocated_bytes` is evaluation alone.
- **Batch limit.** A proposal whose derived
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
- **Package version.** `ExecutableOperationPackageV1` embeds
  `EchoOperationSemanticClosureV1`, which requires `lawpack_coordinate` and
  `lawpack_identity`, and `EchoOperationProgramV1`, which has only the two
  fixed profiles. A source-compiled bridge package therefore uses
  `ExecutableOperationPackageV2`. Fixed-profile programs and invocations are
  not re-encoded: they keep their V1 maps, schema strings, and identities
  byte for byte. V2 adds exactly this:
    - **Package.** The V1 package map with `schema` set to
      `echo.operation-package/v2`, `semantic_closure` set to a
      `closure-v2`, and `program` set to an effect program. Every other key,
      and its value shape, is as in V1.
    - **Semantic closure.** The V1 closure map, with `lawpack_coordinate` and
      `lawpack_identity` both omitted. A V2 package carries only an effect
      program, which requires their absence, so no sentinel lawpack is
      fabricated. A fixed-profile program stays in a V1 package with its
      lawpack, as ADR 0023 requires today. The closure has no identity of its
      own; its bytes are part of the package's.
    - **Effect program.** A new map, `program-v2-effect` below. Like a V1
      program, it carries `interpreter_profile_identity` and
      `intrinsic_profile_identity`, and admission requires each to equal the
      package's value of the same name. Its other entries are:
        - `compiled_program`, the compiled bytes;
        - `target_ir_identity`, which must equal the closure's;
        - `identity_law`, an unsigned integer naming an
          [identity law](#identity-laws); unknown values are refused;
        - `graph_schema`, the canonical bytes of a `graph-schema`;
        - `footprint_law`, the canonical bytes of a `footprint-law`;
        - `max_input_bytes`;
        - `output_type`, the compiler-declared output type coordinate of the
          program's application result, which `result_identity` hashes.

        The graph schema is the executable graph contract, a different
        artifact from the application's own schema declaration. The closure's
        `application_schema_identity` keeps naming that declaration, with its
        codecs and migration posture, as ADR 0023 requires. The graph schema
        is bound by the program identity, whose bytes contain it, so no
        equality between the two is required. The package's inherited
        `footprint_contract_identity` must equal BLAKE3 over
        `echo:operation-footprint-law:v2\0` and the `footprint_law` bytes, so
        the attested footprint contract is exactly the law validation
        executes. The `graph_schema` is a map that Echo executes, not only a
        digest. `node_types` maps each
        admitted node type to its addressing, which the
        [identity law](#identity-laws) reads, and its atom types. `edge_types`
        maps each admitted edge type to its (source node type, target node
        type) pairs. Validation step 5 reads these relations directly. No
        artifact is attested under one identity and executed under another.

    - **Effect invocation.** A new map, `invocation-v2-effect` below. It
      carries the V1 invocation's common entries other than the anchored
      node and replacement (`authority_grant_identity`, `delegated_budget`,
      `evaluation_basis`, `operation_coordinate`, `package_id`), plus the
      required `application_input_bytes`, the program's input in
      `edict.canonical-cbor/v1`. Admission checks only what it can check
      without a schema language: that the bytes are canonical and no longer
      than the program's `max_input_bytes`. The input's shape is the compiled
      program's to check, as its first act; a malformed input obstructs
      evaluation with the authored `InputRejected`, posture `NoProposal`. An
      effect program admits only effect invocations, and a fixed profile only
      its own V1 invocations, so no invocation crosses between them.

    ```text
    closure-v2 = {
      "application_schema_coordinate": tstr, "application_schema_identity": id,
      "canonical_meaning_identity": id, "core_identity": id,
      "edict_source_identity": id, "target_ir_identity": id,
    }
    program-v2-effect = {
      "schema": "echo.operation-program/v2", "kind": "edict-effect-program/v1",
      "interpreter_profile_identity": id, "intrinsic_profile_identity": id,
      "compiled_program": bstr, "target_ir_identity": id,
      "identity_law": uint, "graph_schema": bstr, "footprint_law": bstr,
      "max_input_bytes": uint, "output_type": tstr,
    }
    invocation-v2-effect = {
      "schema": "echo.operation-invocation/v2",
      "authority_grant_identity": id, "delegated_budget": budget-v1,
      "evaluation_basis": evaluation-basis-v1, "operation_coordinate": tstr,
      "package_id": id, "application_input_bytes": bstr,
    }
    ; budget-v1 and evaluation-basis-v1 are the V1 values, unchanged.
    ```

    No other key may appear in any of these maps. A V2 package, effect program,
    and effect invocation hash their identities under
    `echo:operation-package:v2\0`, `echo:operation-program:v2\0`, and
    `echo:operation-invocation:v2\0`. Their invocation-bytes digest, package
    admission, invocation admission, and installation identities use
    `echo:operation-invocation-bytes:v2\0`,
    `echo:operation-package-admission:v2\0`,
    `echo:operation-invocation-admission:v2\0`, and
    `echo:operation-installation:v2\0`. Every other domain is unchanged. So V2
    identities never coincide with V1 identities, and V1 encoding, validation,
    and identities are unchanged.

## Evidence

Each Action's receipt entry binds, in addition to its current fields:

- the proposal posture, a tagged value with exactly three variants:
    - `NotBridge`: the entry is not a bridge Action. Fixed-profile Actions and
      every other existing receipt entry take this variant. Its existing
      fields are encoded and hashed exactly as under receipt digest version
      4, and its row in the bridge table is all zero (posture `0`). No bridge
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
        - `OriginOnly`, for every outcome decided before the history check: an
          evaluation obstruction, or a failure of reachability or resolution
          (steps 1 and 2 of the [Validation order](#validation-order)). No
          dependency comparison ran;
        - `HistoryChanged`, for a dependency-scoped `BasisChanged`: the
          dependency-set digest over each dependency's descriptor and
          comparison mode, and the first changed dependency in canonical
          order. Validation never ran, so no values were compared;
        - `Compared`, for every Action that passed the history check,
          whatever its later outcome: the dependency-set digest and a
          compared-values digest. The pre-composition checks compare, at the
          commit parent, the value of every effect-precondition and
          resolved-record dependency. Witnessed reads are verified by the
          history check's written-slot test, not by value, and are recorded
          as not compared. A precondition or resolved-record dependency
          those checks never reached, because validation stopped at an
          earlier step or a validation read exhausted the budget before
          hashing its value, is recorded as not compared, with no value. For
          an accepted
          Action these equal its values at the validation position (the
          commit parent plus the writes of every earlier accepted Action),
          because composition admits no overlap; a
          `RejectedFootprintConflict` is never compared at its validation
          position;

- for `NoProposal` and `Proposed`, consumed budget: evaluation and validation
  charges together, with `allocated_bytes` from evaluation alone. When
  evaluation never ran (whole-frontier `BasisChanged`, including unknown
  history), it is zero on every axis, and recovery verifies that.

**Dependency kinds and modes.** A dependency is its slot, its kind, and its
comparison mode:

- kind: value present `0`, observed absent `1`, expected value `2`, endpoint
  existence `3`, resolved record `4` (a record step 2 resolved, including an
  Ensure or Create target and an endpoint), and portal `5` (a portal
  attachment on an effect's chain, or its owner's node record);
- comparison mode: `ValueDigest` `0`, a payload-value dependency that
  compares the slot's value digest, and `Absence` `1`, a single-key absence
  dependency. These are exactly the two kinds the Action validity decision
  admits. Value present, expected value, endpoint existence, and portal use
  `ValueDigest`: an endpoint's dependency is on its node record's value, so it
  holds only while the node exists with that value. Observed absent uses
  `Absence`. A resolved record uses `ValueDigest` when step 2 found it and
  `Absence` when it did not. Positive presence alone is not a mode.
  Revision-sensitive and identity-sensitive dependencies are refused at
  admission and have no tag.

A scoped Action reads no edge ranges (an edge read keeps the whole-frontier
rule), so its dependency slots are node records, attachments (atoms and
portals), and edges.

**Value digests.** A present slot's value digest is BLAKE3 over a
class-specific domain tag and its canonical fields:

- node record: `echo.bridge.node-value.v1\0`, then the node type id;
- attachment: `echo.bridge.attachment-value.v1\0`, then for an atom tag `0`,
  its `type_id`, its byte length as a little-endian `u64`, and its bytes, or
  for a portal tag `1` and its target warp id. `expectedAtomDigest` is this
  digest of the expected atom;
- edge: `echo.bridge.edge-value.v1\0`, then the edge type id and the local
  `from` and `to` node ids.

The read-set, target-set, write-set, dependency-set, and compared-values
digests, and the byte images they hash, are defined in
[Encoding and memory](#encoding-and-memory).

The submission identity is present in every variant. An identified submission
whose evaluation produced no proposal is a different evidence state from an
evaluated proposal, and the receipt keeps them distinct.

**Proposal identity.** The proposal identity is BLAKE3 over
`echo.bridge.effect-proposal.v1\0` followed by the proposal's canonical
image, defined in [Encoding and memory](#encoding-and-memory).

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

**Receipt and outcome records.** The `EPRV0004` bridge table, the receipt
digest version 5 preimage, and the `EOACT004` retained Action outcome are
defined in [Encoding and memory](#encoding-and-memory). Fixed-profile
Actions keep `EOACT003`, its decoder is unchanged, and each record decodes by
its own magic. Recovery requires the `EOACT004` outcome coordinate to equal
the receipt's bridge-row coordinate.
The implementation adds this row to
[Submission Attribution's format table](../topics/SubmissionAttribution.md#format-binding-and-legacy)
when it lands:

| Receipt                                   | Receipt digest | Retained record | Commit hash |
| ----------------------------------------- | -------------- | --------------- | ----------- |
| Commit-bound section with bridge evidence | version 5      | `EPRV0004`      | v3          |

Only a Tick that contains a bridge Action uses receipt digest version 5 and
`EPRV0004`; a Tick of fixed-profile Actions alone keeps version 4 and
`EPRV0003`, byte for byte.

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
  for a `Proposed` outcome recorded as `OriginOnly`, recovery replays only
  the pre-history failure (reachability or resolution) and requires the same
  obstruction, without reconstructing a dependency set that live execution
  never built. For one recorded as `HistoryChanged` or `Compared`, it
  recomputes the complete dependency set (every witnessed read and every
  effect precondition) and the
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
| Committed                                                                                                                                                                                                                                                          | `Proposed`, `Derived`    | replay patch; reconstruct preparation; compare identities                 | effects bound; read-set digest; the row's `patch_digest` equals this Action's re-derived candidate patch; the Tick's replay patch equals the batch composition of every accepted candidate                           |
| `RejectedFootprintConflict`                                                                                                                                                                                                                                        | `Proposed`, `Derived`    | reconstruct preparation; compare identities; recompute recorded blockers  | effects bound; read-set digest; blockers include range-key conflicts                                                                                                                                                 |
| Obstructed at validation (`DanglingReference`, `ContentIdentityConflict`, `DuplicateEffectTarget`, `FootprintLawViolation`, `IdentityLawViolation`, `SchemaViolation`, `UnreachableWarp`, `NodeMissing`, `PreconditionMismatch`, validation-read `BudgetExceeded`) | `Proposed`, `NotDerived` | re-evaluate; compare pre-decision evidence                                | effects bound; read-set digest; obstruction kind; no write-set or patch digest present                                                                                                                               |
| Obstructed by `FootprintMismatch` (after derivation)                                                                                                                                                                                                               | `Proposed`, `Derived`    | re-evaluate; compare pre-decision evidence                                | effects bound; read-set digest; re-derived patch reproduces the mismatch                                                                                                                                             |
| `BudgetExceeded`, scheduler-assigned (Action-batch limit)                                                                                                                                                                                                          | `Proposed`, `Derived`    | accept reconstructed preparation; verify through the batch reconstruction | effects bound; read-set digest                                                                                                                                                                                       |
| `BudgetExceeded` during evaluation, or an authored guard                                                                                                                                                                                                           | `NoProposal`             | re-evaluate; compare pre-decision evidence                                | re-evaluation, at the origin for a `Scoped` Action, again yields no proposal and the same obstruction; read-set digest                                                                                               |
| `BasisChanged`, whole-frontier (before evaluation, including unknown history)                                                                                                                                                                                      | `NoProposal`             | admission evidence; recomputed basis posture; no re-evaluation            | none; posture must be `NoProposal` with an empty read set and zero consumed budget                                                                                                                                   |
| `BasisChanged`, dependency-scoped (after evaluation, before validation)                                                                                                                                                                                            | `Proposed`, `NotDerived` | none today: current programs decide this before evaluation                | re-evaluate at the origin; effects bound; read-set digest; recompute the written-slot set from replay operations, the complete dependency set (witnessed reads and effect preconditions), and the changed dependency |
| Any outcome of a `NotBridge` entry                                                                                                                                                                                                                                 | `NotBridge`              | unchanged                                                                 | none; its bridge row is all zero                                                                                                                                                                                     |

## Encoding and memory

Every bridge record is a zero-copy image in the style of Echo's WSC snapshot
format (`crates/warp-core/src/wsc/`). It is read in place through a view that
a validator has checked, and it is never hydrated into owned structures. The
rules:

- **Rows.** Every fixed-size row is `#[repr(C)]` and `bytemuck` `Pod` and
  `Zeroable`, with explicit padding fields. Integers are little-endian, ids
  and digests are 32 bytes, and every row size is a multiple of 8 bytes.
- **Images.** An image is an 8-byte magic, a fixed header, and sections of
  rows, each section starting 8-byte aligned. Variable-length bytes live in
  a trailing blob section and are addressed by a 16-byte `Range`
  (`start_le`, `len_le`), WSC's range row.
- **One canonical image per value.** Rows appear in the sort order given
  below. Padding, and every field a row's tags make unused, are zero. Blob
  bytes are packed in row order, each run starting 8-byte aligned, with zero
  bytes between runs and none after the last. Two images of one value are
  therefore byte-identical, and every digest below is BLAKE3 over a domain
  tag and exact image bytes.
- **Validation before trust.** A reader checks the magic, the sizes,
  alignment, tags, zero padding and zero unused fields, sort order, range
  bounds, and blob packing before using a view. Any failure refuses the
  history. Validation runs in place and allocates nothing.

**Rows.**

| Row              | Bytes | Fields, in order                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ---------------- | ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SlotRow`        | 104   | `class` u8 (node record `0`, attachment `1`, edge `2`, range key `3`); `owner` u8 (attachment: node `0`, edge `1`); `plane` u8 (attachment: alpha `0`, beta `1`); 5 padding bytes; `warp_id`; `local_id` (the node, edge, attachment owner, or range source); `edge_type` (range key only)                                                                                                                                                                                   |
| `ReadRow`        | 160   | `slot` (`SlotRow`); `outcome` u8 (present `0`, absent `1`, range `2`); 7 padding bytes; `subject`, 32 bytes: the value digest when present, when absent, the expected node type id for a node-record slot or the expected atom `type_id` for an attachment slot; zero for a range; `pairs` (`Range` into the pair section; range only)                                                                                                                                       |
| `EdgePairRow`    | 64    | `edge_local_id`; `target_local_id`                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `EffectRow`      | 280   | `kind` u8 (`EnsureNode` `0`, `CreateNode` `1`, `ReplaceAtom` `2`, `EnsureEdge` `3`, `ReplaceEdge` `4`); 7 padding bytes; `key_warp_id`; `key_local_id`; `type_id` (node type, or `EnsureEdge`'s edge type); `atom_type_id`; `atom` (`Range` into the blob); `expected_digest` (`ReplaceAtom`); `from_local_id` (`EnsureEdge`); `to_local_id` (`EnsureEdge`, and `ReplaceEdge`'s new target); `expected_to_local_id` (`ReplaceEdge`)                                          |
| `DependencyRow`  | 152   | `slot` (`SlotRow`); `kind` u8; `mode` u8; 6 padding bytes, which together with `slot` form the 112-byte descriptor; `compared` u8 (present `0`, absent `1`, not compared `2`); 7 padding bytes; `compared_digest` (present only)                                                                                                                                                                                                                                             |
| `OutcomeRow`     | 40    | `outcome` u8 (none `0`, bridge kind `1`, authored `2`); `code` u8 (the bridge kind code); 6 padding bytes; `authored_id` (authored only; see below)                                                                                                                                                                                                                                                                                                                          |
| `BridgeEntryRow` | 496   | `posture`, `patch`, `validity`, and `evidence` tags, u8 each; `outcome` and `code` as in `OutcomeRow`; 2 padding bytes; `proposal_id`; `target_set`; `write_set`; `patch_digest`; `read_set`; `origin_basis`; `dependency_set`; `compared_values`; `changed_dependency` (a 112-byte descriptor); `authored_id`; `validation_position` u64; `steps`, `read_bytes`, `write_bytes`, and `allocated_bytes`, u64 each; `result_identity`; `result` (`Range` into the result blob) |

Tags not defined above use the values given in [Evidence](#evidence):
posture `NotBridge` `0`, `NoProposal` `1`, `Proposed` `2`; patch `NotDerived`
`0`, `Derived` `1`; validity `WholeFrontier` `0`, `Scoped` `1`; evidence
`OriginOnly` `0`, `HistoryChanged` `1`, `Compared` `2`. Bridge kind codes are
`BasisChanged` `0`, `BudgetExceeded` `1`, `DuplicateEffectTarget` `2`,
`IdentityLawViolation` `3`, `UnreachableWarp` `4`, `SchemaViolation` `5`,
`FootprintLawViolation` `6`, `NodeMissing` `7`, `PreconditionMismatch` `8`,
`ContentIdentityConflict` `9`, `DanglingReference` `10`, and
`FootprintMismatch` `11`. A `NotBridge` row is all zero after its posture.

`validation_position` is the Action's zero-based index among the Tick's
selected candidates in canonical order, counting every candidate whatever its
outcome; it is `0` when the validity tag is `WholeFrontier`. Each tag sets
exactly these `BridgeEntryRow` fields; every other field is zero:

| Tag value                 | Fields it sets                                                                |
| ------------------------- | ----------------------------------------------------------------------------- |
| posture `NotBridge`       | none                                                                          |
| posture `NoProposal`      | `read_set`, the budget fields, `validity`, `outcome`, `code` or `authored_id` |
| posture `Proposed`        | as `NoProposal`, plus `proposal_id`, `target_set`, `patch`                    |
| patch `Derived`           | `write_set`, `patch_digest`                                                   |
| validity `Scoped`         | `origin_basis`, `validation_position`, `evidence`                             |
| evidence `HistoryChanged` | `dependency_set`, `changed_dependency`                                        |
| evidence `Compared`       | `dependency_set`, `compared_values`                                           |
| outcome committed         | `result_identity`, `result`                                                   |

`authored_id` identifies an authored obstruction. The program reports one as
a code, a UTF-8 string of at most 1,024 bytes such as `InputRejected` or
`BasisNotCanonical`, and detail bytes in `edict.canonical-cbor/v1`, bounded
by the program's `max_allocated_bytes`. `authored_id` is BLAKE3 over
`echo.bridge.authored-obstruction.v1\0`, the program identity, the code's
length as a little-endian `u64` and its bytes, and the detail's length and
bytes. Recovery re-evaluates the Action at its origin and requires the same
identity.

`result_identity` is BLAKE3 over `echo.bridge.result.v1\0`, the program
identity, the length of the program's `output_type` as a
little-endian `u64` and its UTF-8 bytes, and the length of the result bytes as
a little-endian `u64` and the bytes: the propositions ADR 0025 binds for an
applied Action. An obstruction or conflict carries no result, and both fields
are zero. Recovery re-evaluates the Action and requires its result bytes to
equal the retained ones before it publishes the outcome.

A `Range` addresses its section in that section's units, from the section's
start: the blob in bytes, and the `EdgePairRow` section in rows.

**Sort orders.** `SlotRow`s sort by `class`, `warp_id`, `local_id`,
`edge_type`, `owner`, then `plane`, so a set's rows are in slot order.
`ReadRow`s and `DependencyRow`s sort by their slot, and `DependencyRow`s with
one slot by `kind`, then `mode`. A range's `EdgePairRow`s are contiguous,
sorted by `edge_local_id`, and the ranges follow `ReadRow` order. `EffectRow`s
follow the canonical effect order in [EffectProposal](#effectproposal).

**Images and digests.**

- **Proposal** (magic `EBPRP001`): after the magic, a 192-byte header holding
  the installed-operation identity, the canonical invocation digest, the
  `EchoOperationEvaluationBasisV1` identity, the read-set digest, the cost
  (`steps`, `read_bytes`, `write_bytes`, `allocated_bytes`), the result as a
  `Range` into the blob, the effect count, and the blob length; then the
  `EffectRow`s; then the blob, holding the atoms in effect order and then the
  result. The proposal identity is BLAKE3 over
  `echo.bridge.effect-proposal.v1\0` and the image.
- **Sets.** The read-set digest is BLAKE3 over `echo.bridge.read-set.v1\0`,
  the `ReadRow` count as a little-endian `u64`, the `ReadRow`s, the
  `EdgePairRow` count, and the `EdgePairRow`s. The target-set and write-set
  digests hash their domain tags (`echo.bridge.target-set.v1\0`,
  `echo.bridge.write-set.v1\0`), the count, and the `SlotRow`s. The
  dependency-set digest hashes `echo.bridge.dependency-set.v1\0`, the count,
  and each `DependencyRow`'s 112-byte descriptor. The compared-values digest
  hashes `echo.bridge.compared-values.v1\0`, the count, and the whole
  `DependencyRow`s.
- **`EPRV0004`**: the eight ASCII bytes `EPRV0004` as the record magic, then
  the `EPRV0003` payload that follows that record's own magic, byte for byte,
  then zero padding to an 8-byte boundary, then a bridge table of one
  `BridgeEntryRow` per receipt entry, in entry order, then the result blob:
  its length as a little-endian `u64`, then the blob. The blob follows the
  image rule above: each committed bridge Action's exact result bytes, in
  entry order, each run starting 8-byte aligned from the blob's start, with
  zero bytes between runs and none after the last. The length counts the
  padding between runs. The record ends with the blob. Each row's `result`
  range addresses the blob, which carries the
  result bytes ADR 0025 requires in the Receipt. Only the magic differs from
  `EPRV0003` before the table, so the unchanged `EPRV0003` decoder never sees
  an `EPRV0004` record. The table is indexable in place.
- **Receipt digest version 5**: the version-4 preimage with its leading
  little-endian `u16` version tag set to `5`, in the same position as
  versions 2 through 4, followed by the bridge table and result blob bytes
  exactly as `EPRV0004` retains them.
- **`EOACT004`**: a bridge Action's retained outcome. It is a sequential
  record like `EOACT003`, not one of the zero-copy images above, because it
  is read once at recovery:
    - the eight ASCII bytes `EOACT004`, the submission id and ingress id (32
      bytes each), and a one-byte variant tag (committed `1`, obstructed `2`,
      conflict `3`);
    - the 40-byte `OutcomeRow` and the Action's 496-byte `BridgeEntryRow`;
    - the 152-byte decision coordinate, exactly as `EOACT003` retains it
      (`retain_action_decision_coordinate_v1`): writer-head worldline id and
      head id, `worldline_tick_after` and `commit_global_tick` as
      little-endian `u64`s, commit id, Tick receipt digest, and member index
      as a little-endian `u64`. Every variant carries it, because a bridge
      Action is decided by a sealed Tick whether it commits, obstructs, or
      conflicts;
    - for a committed Action, its exact result bytes, as a little-endian
      `u64` length and the bytes;
    - for a conflict, the blocker count as a little-endian `u64` and each
      blocker's candidate index as a little-endian `u32`, as `EOACT003` does.

    A bridge Action has no `EchoOperationReceiptV1` and no prepared candidate:
    its evidence is the `BridgeEntryRow`, which the receipt digest already
    binds. Committed and conflict variants carry the all-zero `OutcomeRow`.
    Recovery requires the `OutcomeRow` and `BridgeEntryRow` to equal the
    receipt's byte for byte, and the decision coordinate to name the sealed
    Tick that holds that receipt entry at that member index. Fixed-profile
    Actions keep `EOACT003`.

**Memory.** The bridge allocates nothing on its hot path:

- **One reused arena per scheduler.** The scheduler owns a bridge arena that
  is reset, never freed, at each Tick boundary, as it already reuses its
  radix `scratch` buffer. Every working structure is a typed region in it:
  the recording view's `ReadRow`s and `EdgePairRow`s, the emitted
  `EffectRow`s and blob, the `DependencyRow`s and `SlotRow`s, the proposal
  images, and the bridge table.
- **Built in final form.** Records are written in place as their final
  images. Hashing, the WAL append, and receipt construction read those bytes
  directly; nothing is copied into intermediate structures.
- **Capacity fixed before evaluation.** Each region's size is a pure function of
  the selected candidates' declared footprint-law maxima, delegated budgets, and
  the program's `max_allocated_bytes`, which bounds the result bytes the blob
  must also hold, sized per candidate from its own declared maxima, never from
  the 128-operation batch limit, so a candidate whose patch exceeds that limit
  is still derived in place and then rejected with `BudgetExceeded`. The
  64-candidate limit bounds how many such candidates a Tick selects. The arena
  grows, if at all, after selection and before evaluation. Evaluation,
  validation, derivation, and composition never allocate.
- **No host-dependent decisions.** Arena capacity never changes which
  Actions a Tick contains or how they are decided. If the host cannot grow
  the arena, the Tick fails as a host fault and nothing is decided, as on a
  crash before the WAL record, rather than obstructing an Action.
- **Recovery reads views.** Recovery validates retained images in place and
  reads them through views; it rebuilds proposals in the same arena to
  compare them, and allocates nothing per Action.

`allocated_bytes` remains `edict_pure`'s metered evaluator allocation, which
is evidence about the program, not about host memory.

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
6. **Validation**, in [Validation order](#validation-order):
    1. every key is in the root warp, so no portal is crossed;
    2. resolution reads and charges the node records and alpha slots of the
       `EnsureNode T2` and `EnsureNode H2` targets and the edge records of
       the `EnsureEdge H2.root` and `H2.anchor` targets (all absent, so all
       four create); `B`'s node record, for `ReplaceAtom B` and as the
       source of `ReplaceEdge`, charged once; the `B.canonicalHead` edge
       record; and the endpoint `I`'s node record. `H2`, the source of both
       Ensures, is created here and is not read;
    3. the Action is whole-frontier, so no history check runs;
    4. no two effects share a slot;
    5. the schema admits every type: `T2` and `H2` are content types, `B` is
       mutable, and each edge's source and target types are admitted;
    6. Echo recomputes `T2` and `H2` as content addresses over their atoms
       and the edges this proposal gives them, recomputes the content ids of
       `H2.root` and `H2.anchor`, and recomputes the slot id of the resolved
       `B.canonicalHead` from its source and type, which the retarget keeps;
    7. the read set, precondition slots, and write set lie inside the
       footprint law;
    8. the expected `H1` and version-7 digest hold at `c`, and every endpoint
       exists or is created here.

    Its package's footprint law admits range reads, so its validity rule is
    `WholeFrontier` whatever this evaluation read.

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
    and `T1` are no longer reachable from `B`, so they leave the state root,
    but they stay records of the state until a patch deletes them (see
    [Basis](#basis-budget-retries)).

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

| Property                        | Test                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Atomicity                       | A failure on the last effect of a multi-node proposal leaves the parent state root and patch unchanged.                                                                                                                                                                                                                                                                                          |
| Ensure identity                 | Identical existing node: no patch entry. Different existing node: `ContentIdentityConflict`.                                                                                                                                                                                                                                                                                                     |
| Partial occupancy               | A node record without its atom, or an atom without its record: `CreateNode` obstructs with `PreconditionMismatch` and `EnsureNode` with `ContentIdentityConflict`; neither overwrites the orphan slot.                                                                                                                                                                                           |
| Witnessed footprint             | A program that reads a slot outside its law obstructs, even if its writes are in bounds.                                                                                                                                                                                                                                                                                                         |
| Stale basis                     | A `ReplaceAtom` or `ReplaceEdge` whose expected value already disagrees with the value at its evaluation basis obstructs with `PreconditionMismatch`, posture `Proposed`, `NotDerived`, and no patch; when the value instead changed after the origin, a whole-frontier Action obstructs with `BasisChanged` before evaluation and a scoped one at the history check.                            |
| Budget                          | Exhausting steps or bytes obstructs with `BudgetExceeded` and `NoProposal`; a proposal whose patch alone exceeds the Action-batch operation limit obstructs with `BudgetExceeded`, posture `Proposed` with its digests, and no committed patch.                                                                                                                                                  |
| Effect charges                  | Each effect is charged one step and its fixed `write_bytes` (64 plus atom bytes for node and atom effects, 128 for edge effects) at emission; a budget one byte short of a proposal's total obstructs with `BudgetExceeded` and `NoProposal`.                                                                                                                                                    |
| Atom write bytes                | A proposal whose atom bytes exceed the delegated `write_bytes` obstructs with `BudgetExceeded` and `NoProposal`; a whole-frontier `BasisChanged` records zero consumed budget.                                                                                                                                                                                                                   |
| Validation read budget          | A `ReplaceAtom` of a stored atom larger than the remaining `read_bytes` obstructs with `BudgetExceeded`, posture `Proposed`, `NotDerived`, before hashing it; allocation beyond `edict_pure`'s `max_allocated_bytes` obstructs with `BudgetExceeded` and `NoProposal`.                                                                                                                           |
| Budget-stopped evidence         | A scoped proposal whose validation read exhausts the budget records `Compared` with that dependency, and every later one, as not compared; no value past the stop is hashed.                                                                                                                                                                                                                     |
| Determinism                     | Same package, invocation, and evaluation basis give byte-identical proposals, proposal identities, read-set and write-set digests, and derived patches, whether the Action is evaluated alone or beside other Actions. Receipts and composite patches are compared only for identical Tick contexts.                                                                                             |
| Canonical order                 | Emitting the same effects in different orders yields the same canonical `EffectProposal` bytes and proposal identity.                                                                                                                                                                                                                                                                            |
| Blob-independent order          | Two same-kind duplicate effects on one slot with different atoms produce the same canonical image and proposal identity whichever order they were emitted in.                                                                                                                                                                                                                                    |
| Encoding vectors                | Fixed byte vectors for every row, the proposal image, every set digest, a bridge table with each posture, validity, evidence, and outcome tag, and `EOACT004` reproduce the specified bytes, sizes, and receipt-v5 digests.                                                                                                                                                                      |
| In-place validation             | Each image validates and reads through a view without heap allocation; an image with nonzero padding, a nonzero unused field, an unsorted section, an out-of-bounds range, or loosely packed blob bytes is refused.                                                                                                                                                                              |
| No hot-path allocation          | With an allocation-counting allocator, a warmed-up Tick of bridge Actions performs zero heap allocations during evaluation, validation, derivation, and composition, and recovery of that Tick allocates nothing per Action.                                                                                                                                                                     |
| Arena independence              | The same Ticks decided on hosts with different initial arena capacities produce byte-identical receipts, WAL records, and state roots.                                                                                                                                                                                                                                                           |
| Range-read entries              | An empty observed range encodes as a range entry with count zero, never as an absent outcome, and a nonempty range's entry matches the specified layout byte for byte.                                                                                                                                                                                                                           |
| Canonical slots                 | Differently expressed effects on one slot (`EnsureEdge` and `ReplaceEdge` of one edge id; `CreateNode` and `ReplaceAtom` of one node) obstruct with `DuplicateEffectTarget`.                                                                                                                                                                                                                     |
| Write set                       | The derived patch's actual footprint equals the write set, including the atom slots of created nodes and the range key of every created or retargeted edge, so a committed `EnsureEdge` and `ReplaceEdge` pass the footprint check.                                                                                                                                                              |
| Identity law                    | A proposed node or edge id that does not match the package's identity law obstructs with `IdentityLawViolation`, posture `Proposed`, `NotDerived`, and recovery reproduces it.                                                                                                                                                                                                                   |
| Node resolution                 | A `ReplaceAtom` of a node absent at the evaluation basis obstructs at step 2 with `NodeMissing`, posture `Proposed`, `NotDerived`.                                                                                                                                                                                                                                                               |
| Content address                 | Under `ContentAddressV1`, a node id equal to the specified BLAKE3 over its type, atom, and the edges its creating proposal's `EnsureEdge` effects give it, and an edge id over its endpoints and type, pass; any other id obstructs with `IdentityLawViolation`; fixed vectors pin both derivations.                                                                                             |
| Addressing by type              | A `ReplaceAtom` of a node whose type the schema marks content-addressed, or an `EnsureNode` of a mutable type, obstructs with `IdentityLawViolation`; a `ReplaceAtom` of a mutable-type node commits.                                                                                                                                                                                            |
| Edge addressing                 | An edge from a mutable node keeps its slot id when a `ReplaceEdge` retargets it; a creating `EnsureEdge` from an existing content node, a `CreateNode` of a content type, and a `ReplaceEdge` of an edge from a content node each obstruct with `IdentityLawViolation`.                                                                                                                          |
| Unreachable warp                | An effect naming a key in a missing warp, or in a warp no valid portal chain reaches, obstructs with `UnreachableWarp`, posture `Proposed`, `NotDerived`.                                                                                                                                                                                                                                        |
| Origin reachability             | A scoped Action naming a warp unreachable at its origin obstructs with `UnreachableWarp` even when an intervening Tick created the portal that reaches it.                                                                                                                                                                                                                                       |
| Portal permission and cost      | At validation step 1, an effect in a descended warp whose chain crosses a portal owned by a type outside `portal_owner_types` obstructs with `FootprintLawViolation`, and a chain whose crossings exceed the remaining budget obstructs with `BudgetExceeded`.                                                                                                                                   |
| Schema                          | An effect whose node type, atom `type_id`, or edge type the package's schema does not admit obstructs with `SchemaViolation`, posture `Proposed`, `NotDerived`, and recovery reproduces it.                                                                                                                                                                                                      |
| Resolved schema                 | A `ReplaceEdge` whose basis edge type the schema does not admit, or a `ReplaceAtom` whose owner node type does not admit the new atom type, obstructs with `SchemaViolation` after its record is read and charged.                                                                                                                                                                               |
| Endpoint types                  | A creating `EnsureEdge` or a `ReplaceEdge` whose source or target node type the schema does not admit for that edge type obstructs with `SchemaViolation`, after its endpoint records are read and charged.                                                                                                                                                                                      |
| Validation order                | A proposal that breaks several rules (for example a duplicate target, an unadmitted type, and a missing endpoint) records the obstruction of the earliest step in the validation order, live and on recovery.                                                                                                                                                                                    |
| Conflict before batch limit     | A candidate that both overlaps an earlier accepted Action and would exceed the batch operation limit records `RejectedFootprintConflict`, as today's scheduler does.                                                                                                                                                                                                                             |
| Footprint stages                | A target outside the footprint law obstructs with `FootprintLawViolation` and `NotDerived`, and so does a `ReplaceEdge` whose basis edge's range key lies outside the law; a derived patch whose footprint differs from the write set obstructs with `FootprintMismatch` and `Derived`.                                                                                                          |
| Resolved footprint              | A `ReplaceEdge` whose resolved source node lies outside the footprint law obstructs with `FootprintLawViolation`.                                                                                                                                                                                                                                                                                |
| Restored value (ABA)            | A dependency written A to B and back to A between origin and commit parent is found through the written-slot set and obstructs with `BasisChanged`, including when the intervening patch under-declared its `out_slots`; with one replay patch in the interval missing, the Action obstructs before evaluation as a whole-frontier `BasisChanged`, posture `NoProposal`, with an empty read set. |
| Scoped preconditions            | A dependency-scoped Action whose `ReplaceAtom` expected value, or one of whose edge endpoints, changed and was restored between its origin and the commit parent obstructs with `BasisChanged`, though the program never read that slot; recovery reproduces the outcome and its changed dependency.                                                                                             |
| Origin resolution               | A scoped `ReplaceEdge` whose program never read the edge, and whose source node is deleted and recreated between origin and commit parent while the edge is unchanged, obstructs with `BasisChanged`, because resolution at the origin put the source in its dependency set.                                                                                                                     |
| Scoped conflict recovery        | A scoped Action rejected because an earlier Action in its Tick replaced a value it expects recovers as `RejectedFootprintConflict`, not `PreconditionMismatch`.                                                                                                                                                                                                                                  |
| Scoped evidence forms           | An evaluation obstruction records `OriginOnly`, a dependency-scoped `BasisChanged` records `HistoryChanged` with no compared values, and every other scoped outcome records `Compared` over commit-parent values; none is compared at a validation position that includes an earlier overlapping write.                                                                                          |
| OriginOnly recovery             | A scoped proposal that fails reachability or resolution is recovered by replaying only that failure, with no dependency set rebuilt.                                                                                                                                                                                                                                                             |
| Resolution failure evidence     | A scoped `ReplaceEdge` whose edge is missing at its origin records `OriginOnly` with `PreconditionMismatch`; recovery reproduces it without running the history check.                                                                                                                                                                                                                           |
| Retry basis                     | A new submission selected after the head advanced is evaluated at its recorded basis, not at the selection-time state.                                                                                                                                                                                                                                                                           |
| Receipt digest v5               | A version-5 receipt digest begins with the little-endian `u16` `5` and changes when any bridge field changes, including the validation position, a dependency's comparison mode, or a compared value; version-4 receipt identities are unchanged.                                                                                                                                                |
| Proposal binding                | For every `Proposed` outcome class in the recovery matrix, including rejected ones, altering one recorded effect-derived digest or the proposal identity refuses the history.                                                                                                                                                                                                                    |
| No-proposal evidence            | An evaluation that obstructs before proposing records `NoProposal` with its read-set digest and no proposal, write-set, or patch digest.                                                                                                                                                                                                                                                         |
| Recovery                        | Crash before and after the decided-Tick WAL record; fresh-host recovery reproduces the same state root and receipts.                                                                                                                                                                                                                                                                             |
| Composition                     | Two Actions with independent witnessed footprints (neither writes a slot the other reads or writes) commit in one Tick. A write/write overlap obstructs the later one, and so does a write/read overlap: A reads X and writes Y, B writes X and Z.                                                                                                                                               |
| Shared structure                | Two Actions that `Ensure` the same node already present in the basis both commit in one Tick.                                                                                                                                                                                                                                                                                                    |
| Same-Tick identical create      | Two Actions, dependency-scoped included, that both `Ensure` the same node absent at the commit parent: the later obstructs with `RejectedFootprintConflict` instead of reading the earlier one's write.                                                                                                                                                                                          |
| Referential integrity           | An edge to a node neither in the basis nor created by the proposal obstructs with `DanglingReference`; a pre-existing dangling edge elsewhere in the basis does not obstruct an unrelated proposal.                                                                                                                                                                                              |
| Portal chain                    | An effect in a descended warp records every portal on its root-to-target chain as a read; a co-scheduled portal retarget conflicts with it, and a portal changed and restored between origin and commit parent obstructs a scoped Action with `BasisChanged`.                                                                                                                                    |
| Visibility                      | No published coordinate shows a retargeted head without every node and edge the proposal introduced; pre-existing structure the head reaches is not validated.                                                                                                                                                                                                                                   |
| Witnessed reads                 | The receipt's read set equals the slots the view resolved, including for a program that reads and then obstructs.                                                                                                                                                                                                                                                                                |
| Negative reads                  | An Action that observed a node, atom, or edge set absent obstructs when a co-scheduled Action creates it, including a fresh edge of the observed source and type with a new edge id when the observed set was empty (outgoing-edge range key).                                                                                                                                                   |
| Range metering                  | A range read over a node with more edges than the law's maximum, or than the remaining budget covers, obstructs with `BudgetExceeded` and `NoProposal` before copying any edge; an admissible range charges one step and 64 read bytes per edge.                                                                                                                                                 |
| Absent probes                   | A read that finds nothing records the node type it expected; a probe for a type outside `read_node_types` obstructs at step 7 with `FootprintLawViolation`, and recovery reproduces the decision from the read set alone.                                                                                                                                                                        |
| Edge-read basis                 | An Action whose package's footprint law admits a range read commits only onto its exact evaluation basis, including when this invocation reads no edge.                                                                                                                                                                                                                                          |
| Final outcomes                  | A rejected Action's outcome, receipt, and attribution are unchanged after a later submission of the same edit succeeds.                                                                                                                                                                                                                                                                          |
| Duplicate resubmission          | Resubmitting a decided Action's identical content returns `Duplicate` with the original outcome and commits no Tick.                                                                                                                                                                                                                                                                             |
| Attribution                     | Every Action in a bridge Tick appears once in the attribution section, mapped to its own receipt entry.                                                                                                                                                                                                                                                                                          |
| One effect per slot             | Two effects on one slot (two `ReplaceAtom`, or `EnsureNode` plus `ReplaceAtom` of the same node) obstruct with `DuplicateEffectTarget`, posture `Proposed`, `NotDerived`.                                                                                                                                                                                                                        |
| Scoped basis change             | A dependency-scoped Action evaluated at its origin obstructs with `BasisChanged`, posture `Proposed` with `NotDerived`, and keeps its proposal, read set, and changed dependency when a dependency changed, even if its proposal would also fail validation; an unchanged dependency set lets it commit.                                                                                         |
| Scoped evaluation obstruction   | A dependency-scoped Action whose evaluation exhausts its budget, or fails an authored guard, keeps that obstruction with `NoProposal` even when one of its reads changed after the origin; recovery reproduces it.                                                                                                                                                                               |
| Receipt format                  | `EPRV0003` records recover byte for byte through the unchanged decoder; a Tick containing a bridge Action is written as `EPRV0004`, with its fixed-profile Actions as all-zero `NotBridge` rows; a Tick of fixed-profile Actions alone stays `EPRV0003` and receipt digest version 4; a history mixing both formats recovers.                                                                    |
| Package version                 | A source-compiled package encodes as `ExecutableOperationPackageV2` with a `closure-v2` (no lawpack keys) and a `program-v2-effect`, and its identities use the `v2` domains; a fixed-profile package keeps its V1 bytes and identities exactly.                                                                                                                                                 |
| Lawpack by program              | A V2 package whose closure carries `lawpack_coordinate` or `lawpack_identity`, or whose program is a fixed profile, is refused at admission.                                                                                                                                                                                                                                                     |
| Invocation version              | An effect invocation (`invocation-v2-effect`) is admitted only for an effect-program package, and a V1 invocation only for its own fixed profile.                                                                                                                                                                                                                                                |
| Input bounds                    | An effect invocation whose `application_input_bytes` is not canonical CBOR, or is longer than `max_input_bytes`, is refused at admission; a well-formed one of the wrong shape obstructs evaluation with `InputRejected`.                                                                                                                                                                        |
| Outcome record                  | A bridge obstruction, including an authored one such as `BasisNotCanonical`, is retained as `EOACT004` with the same coordinate as its receipt; fixed-profile Actions still write `EOACT003`, and mixed histories recover.                                                                                                                                                                       |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------                                                                                                                                                                       |
| Pending recovery                | An Action lost with its Tick before the WAL record is recovered as the same submission, not a new one.                                                                                                                                                                                                                                                                                           |
| Ensure resolution               | An `EnsureNode` whose identical target already exists is a read, not a write, and the footprint law checks the write set without it; its target read is charged at step 2.                                                                                                                                                                                                                       |
| Absent endpoint history         | A scoped Action whose edge endpoint was absent at its origin and created by an intervening Tick obstructs with `BasisChanged`, through the endpoint's `Absence` dependency.                                                                                                                                                                                                                      |
| Bridge result                   | A committed bridge Action's exact result bytes are retained in `EPRV0004` and `EOACT004` and bound by `result_identity`; recovery refuses re-evaluated bytes that differ.                                                                                                                                                                                                                        |
| Absent probe row                | An absent `ReadRow` carries its expected node type in `subject`, and the image validator accepts it.                                                                                                                                                                                                                                                                                             |
| Set encoding                    | A footprint law or schema with an unsorted or duplicated set is refused at admission; two packages declaring the same sets have identical bytes.                                                                                                                                                                                                                                                 |
| Portal owner dependency         | A delayed scoped Action whose portal owner's node type changed in an intervening Tick obstructs with `BasisChanged`; the owner read is charged at step 1.                                                                                                                                                                                                                                        |
| Existing edge identity          | A `ReplaceEdge` of a basis edge whose id is not its slot id under `ContentAddressV1` obstructs with `IdentityLawViolation`.                                                                                                                                                                                                                                                                      |
| Unreachable records             | A keyed read of a record that a prior Tick made unreachable resolves identically live and after recovery, because the record stays until a patch deletes it.                                                                                                                                                                                                                                     |
| Alpha slot dependency           | A scoped `EnsureNode` whose target's alpha slot changed in an intervening Tick, with its node record unchanged, obstructs with `BasisChanged`.                                                                                                                                                                                                                                                   |
| Absent atom probe               | An absent atom read records its expected atom `type_id`; a probe for a type outside `atom_types` obstructs at step 7 with `FootprintLawViolation`.                                                                                                                                                                                                                                               |
| Content-node adoption           | An `EnsureNode` of an existing content node is adopted only when that node's record, atom, and outgoing edges recompute to its id; a node stored under another id obstructs with `IdentityLawViolation`.                                                                                                                                                                                         |
| Schema shape                    | A footprint law or schema with an unknown, missing, or reordered key, or a node-type entry of any other shape, is refused at admission.                                                                                                                                                                                                                                                          |
| Descended read reachability     | A scoped Action that only reads a descended warp records that warp's portals and owners as dependencies, enforces `portal_owner_types`, and obstructs with `BasisChanged` when an intervening Tick changes them.                                                                                                                                                                                 |
| Undeclared range read           | A range read of an edge type outside `range_edge_types` obstructs evaluation with `FootprintLawViolation`, posture `NoProposal`, before reading anything.                                                                                                                                                                                                                                        |
| Range inspection metering       | A range read over a bucket of many edges of other types is charged for every edge in the bucket and obstructs when the bucket length exceeds `max_range_edges`, even when no edge matches.                                                                                                                                                                                                       |
| Existing edge source            | An identical `EnsureEdge` whose source exists reads and charges the source record, and the edge id is recomputed under the rule its type selects.                                                                                                                                                                                                                                                |
| V2 domains                      | V2 package, program, invocation, and installed-operation identities use the `v2` schema strings and domains, and differ from V1 identities over the same content.                                                                                                                                                                                                                                |
| Topology eligibility            | An Action whose package's law admits an edge write or a content-node write commits only onto its exact evaluation basis, including when this invocation writes only atoms.                                                                                                                                                                                                                       |
| Mutable-source alias            | A creating `EnsureEdge` or a `ReplaceEdge` from a mutable source that already has another edge of the same type, under any id, obstructs with `IdentityLawViolation`.                                                                                                                                                                                                                            |
| Beta portal                     | An effect or read reached only through an edge-owned portal obstructs at step 1 with `FootprintLawViolation`.                                                                                                                                                                                                                                                                                    |
| Program map                     | An effect program map whose `schema` or `kind` differs from `program-v2-effect`, that has any other key, or whose profile identities differ from the package's, is refused at admission.                                                                                                                                                                                                         |
| Dangling source                 | A creating `EnsureEdge` whose source exists neither at the basis nor in the proposal skips the identity law and obstructs at step 8 with `DanglingReference`.                                                                                                                                                                                                                                    |
| Graph schema binding            | A V2 package whose closure names the application's own schema declaration is admitted with a different `graph_schema`; changing `graph_schema` changes the program identity.                                                                                                                                                                                                                     |
| Output type                     | A committed result's `result_identity` hashes the program's `output_type`; a program map without `output_type` is refused at admission.                                                                                                                                                                                                                                                          |
| Footprint contract              | A V2 package whose `footprint_contract_identity` is not the `v2` digest of its `footprint_law` bytes is refused at admission.                                                                                                                                                                                                                                                                    |
| Authored obstruction            | An authored obstruction's `authored_id` follows the specified recipe from its code and canonical detail, and recovery re-derives the same identity.                                                                                                                                                                                                                                              |
| Candidate and composite patches | In a Tick with two accepted bridge Actions, each row's `patch_digest` is its own candidate patch, and the replay patch is their batch composition; recovery verifies both.                                                                                                                                                                                                                       |
| Bridge decision coordinate      | Every `EOACT004` variant carries the 152-byte decision coordinate, and recovery refuses one that does not name the sealed Tick and member index of its receipt entry.                                                                                                                                                                                                                            |
| Footprint maxima                | `max_reads` and `max_writes` count distinct slots after deduplication and Ensure resolution; an identical `EnsureNode` adds no write, and one range read counts as one read.                                                                                                                                                                                                                     |

## Open questions

- How `edict_read` exposes outgoing edges by type to Edict source, and how a
  bounded closure read guided by a pure Edict selector is expressed and
  budgeted. The metering of a single range read is defined above.
- How Edict source exposes the `ContentAddressV1` inputs; the law itself
  is defined above.
- Whether proposals larger than the Action-batch operation limit should ever
  be committable. This design says no: such a proposal is still representable
  as evidence (`BudgetExceeded`, posture `Proposed` with a `Derived` patch, as
  [Basis, budget, retries](#basis-budget-retries) states), but it never
  commits and is never split across Ticks.
