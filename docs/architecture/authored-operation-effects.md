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

Echo learns no application vocabulary. A proposal is nodes, edges, and atoms.

## Current implementation

- Echo main admits two executable programs: anchored single-node alpha-atom
  compare-and-set and create-if-absent. Both write bytes supplied by the
  invocation (`crates/warp-core/src/echo_operation.rs`).
- `edict_pure` and `edict_read` evaluate compiled Edict values, but no admitted
  path turns their results into writes. `edict_read` reads alpha atoms by
  explicit node key; it cannot follow edges.
- Create-if-absent obstructs when the node already exists, even with identical
  content.
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

## Basis, budget, retries

- **Basis.** Reads are taken from the exact basis. At commit, every
  `expected…` value and every read slot that a co-scheduled Action wrote must
  still hold; otherwise the Action obstructs as stale. Action validity across
  ticks keeps its existing rules.
- **Budget.** Evaluation is metered as `edict_pure` already meters it. Each
  effect adds a fixed write cost. Exhausting any limit obstructs with
  `BudgetExceeded`, no proposal, and no patch. A proposal that would exceed the
  Tick's operation cap obstructs the same way rather than splitting.
- **Retries.** An Action is identified by package, invocation, and basis.
  Re-evaluating the same Action at the same basis yields the same proposal.
  Content-addressed `Ensure` effects make a retried, already-committed
  operation a no-op where the program says so; the operation's own `Replace`
  guard decides whether a retry is a stale basis.

## Evidence

The receipt binds, in addition to its current fields:

- the proposal identity;
- the witnessed read-set digest and the write-set digest;
- the derived patch digest;
- consumed budget.

The decided-Tick WAL record is the only durability mechanism. This design adds
no second persistence path.

## What this design does not do

- It does not create an opaque patch authority. A program cannot submit a
  patch; it can only propose typed effects that Echo validates.
- It does not add application intrinsics, a rope primitive, or a
  general-purpose virtual machine.
- It does not decide the state-root definition, and it does not make
  admission cost independent of history size.

## Required tests

Each behaviour below needs a test before implementation:

| Property            | Test                                                                                                                     |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Atomicity           | A failure on the last effect of a multi-node proposal leaves the parent state root and patch unchanged.                  |
| Ensure identity     | Identical existing node: no patch entry. Different existing node: `ContentIdentityConflict`.                             |
| Witnessed footprint | A program that reads a slot outside its law obstructs, even if its writes are in bounds.                                 |
| Stale basis         | `ReplaceEdge` with a moved `expectedTo` obstructs with no patch.                                                         |
| Budget              | Exhausting steps, bytes, or the Tick operation cap obstructs with no proposal committed.                                 |
| Determinism         | Same package, invocation, and basis give byte-identical proposals and receipts.                                          |
| Recovery            | Crash before and after the decided-Tick WAL record; fresh-host recovery reproduces the same state root and receipts.     |
| Composition         | Two Actions with disjoint witnessed write sets on one basis commit in one Tick; overlapping ones obstruct the later one. |

## Open questions

- How `edict_read` exposes outgoing edges by type, and how a bounded closure
  read guided by a pure Edict selector is expressed and budgeted.
- Whether the footprint law is per-invocation or per-package with parameters.
- Which domain-separated hash Edict source can call to derive ids.
- Whether proposals larger than one Tick's operation cap should ever be
  representable. This design says no.
