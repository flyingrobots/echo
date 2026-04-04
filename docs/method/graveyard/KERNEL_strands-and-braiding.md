<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Strands and braiding for Echo

Echo has fork infrastructure but no strand or braiding semantics.
git-warp has a full implementation. This item tracks bringing the
concept to Echo.

## What Echo has today

- `ProvenanceStore::fork(source, fork_tick, new_id)` — creates a
  prefix-copy worldline that diverges after fork_tick
- `ProvenanceEntry::parents: Vec<ProvenanceRef>` — DAG-ready parent
  structure (multiple parents in canonical order)
- `WorldlineRegistry` — holds multiple worldlines with independent
  frontiers
- "strand" is named in `continuum-foundations.md` as a concept that
  should have a contract owner, but no contract exists

## What Echo is missing

### Strand type

git-warp's Strand is: a pinned base observation (frozen frontier +
optional Lamport ceiling) plus an overlay writer that accumulates
speculative patches. The base never moves; the overlay grows.

Echo needs an equivalent: a Strand is a fork of a worldline at a
specific tick, with its own writer head(s), that can be materialized
independently and compared against other strands or the base.

Unlike git-warp, Echo's strands would be in-memory and ephemeral
(not persisted to Git). They're speculative execution lanes for
"what if?" scenarios.

### Braid composition

git-warp's Braid lets one strand read from support strands without
merging them. The support strands are pinned by SHA; they don't
collapse together.

Echo needs an equivalent for composing multiple speculative branches.
This is the mechanism for "show me what happens if strand A and
strand B both advance, without committing either."

### Merge/convergence

Echo can fork but cannot merge. Two forked worldlines remain separate
forever. The provenance entry schema supports multiple parents, but
no merge replay logic exists.

git-warp resolves this with CRDT convergence (OR-Set + LWW). Echo's
deterministic model is different — merge needs to produce a canonical
result, not a convergent one. This is the hardest design problem.

### Strand coordinator

No way to:

- List all strands derived from a common ancestor
- Find the common ancestor of two worldlines
- Coordinate decisions across multiple strands
- Track strand lifecycle (created, active, braided, dropped)

## What git-warp does that we should study

1. **Base observation pinning** — strands capture a frozen read
   coordinate at creation time. This is the key insight: the strand
   knows exactly what state it branched from.

2. **Overlay writer** — strands have their own isolated writer chain.
   Patches written to the strand don't affect the base worldline.

3. **Materialization** — strands can be replayed independently:
   base observation + overlay patches = strand state. Receipts are
   collected during materialization.

4. **Braiding** — one strand reads from support strands without
   collapsing. Support overlays are pinned by SHA, not merged.
   Multiple readers can coexist.

5. **Comparison** — `compareStrand()` diffs a strand's materialization
   against another coordinate. This is "what changed because of
   this strand?"

6. **Intent queue** — strands have a speculative intent queue
   (queued but not committed). This enables "what if I sent these
   intents?" without mutating any state.

## Design questions for Echo

1. Should strands be in-memory only (ephemeral) or persisted to
   echo-cas (durable)?
2. How does strand merge interact with Echo's deterministic commit
   model? git-warp uses CRDT convergence; Echo needs canonical merge.
3. Should strands have their own writer heads, or share the parent
   worldline's head registry with isolation?
4. How do strands interact with the scheduler? Are strand ticks
   scheduled or manual?
5. How does the parallel executor interact with strands? Can shards
   be strand-aware?

## Relationship to warp-ttd

warp-ttd already models strands in its protocol (`LaneKind::STRAND`,
`LaneRef::parentId`). The debugger can display strand topology if
Echo exposes it. The next warp-ttd cycle (D: strand speculation) is
building strand lifecycle into the TUI.

Echo should coordinate with warp-ttd so that the Echo host adapter
can surface strand state through the existing protocol.
