<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Sliced Readings and Copy-on-Write Strands

- **Status:** Accepted design. **Not implemented.** The section
  [Current implementation](#current-implementation) states what the runtime
  does today.
- **Decision date:** 2026-10-12
- **Refines:** [There Is No Graph](there-is-no-graph.md),
  [Strands and Braids](../topics/StrandsAndBraids.md),
  [Terms: WARP State, SkeletonGraph, Instances, Portals, Wormholes](TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md)
- **Related:** [Echo × Keep physical content boundary](echo-keep-physical-content-boundary.md),
  [Fixed timestep invariant](../invariants/FIXED-TIMESTEP.md),
  [Strand contract invariant](../invariants/STRAND-CONTRACT.md)

## Decision

Echo keeps witnessed causal history. It does not keep a graph.

[There Is No Graph](there-is-no-graph.md) already states that a materialized
graph is an optional reading. This page states what follows for the runtime's
own machinery: the runtime may not depend on a complete materialized graph
either.

A tick, an observation, a fork and a recovery each need a reading. A reading
names a causal coordinate and an aperture. Echo builds the slice that the
aperture demands and nothing else.

The value of a slot at a coordinate is the value that its latest producing
tick at or before that coordinate wrote. If no tick produced it, the value is
the one in the lane's initial boundary. Echo resolves a slot through an index
of its producers. It does not replay ticks that the slot does not depend on.

A materialized reading may be retained and reused. It stays valid only for the
question it answers, as [There Is No Graph](there-is-no-graph.md) scopes a
graph-shaped reading. It is a cache. It is not the runtime.

This page does not change the
[strand contract](../invariants/STRAND-CONTRACT.md). Where the two meet, the
strand contract governs.

## What each concept becomes

| Concept    | Meaning under this design                                                                |
| ---------- | ---------------------------------------------------------------------------------------- |
| Tick       | Admits a suffix. It reads the slice for its footprint and appends the patch it produced. |
| Footprint  | The aperture of a tick: the slots it may read and write.                                 |
| Slice      | The part of history that is enough to build and justify a demanded reading.              |
| Rollback   | Does not admit the suffix. No reading has to be restored, because none was overwritten.  |
| Strand     | A lane that shares history with its source lane and owns what it appends. Copy-on-write. |
| Checkpoint | A retained reading at a tick boundary, with its coordinate and identity.                 |
| Wormhole   | A compressed range of ticks, with the receipts and witness that the range stands for.    |

## Laws

1. **Slices, not graphs.** No runtime operation requires a complete
   materialized graph. A tick or an observation builds the reading for its
   aperture and what that reading depends on.
2. **A tick pays for what it changes.** The cost of admitting a tick does not
   grow with the amount of history or with the size of any reading that the
   tick did not demand.
3. **A fork copies nothing.** A strand shares history with its source lane,
   because history is append-only. A reading on the strand resolves a slot
   from the strand's own producers. A slot that the strand has not produced is
   read through the source lane under the basis posture that strand invariant
   INV-S10 requires: the parent remains at the anchor, the parent advanced
   outside the strand-owned footprint, or the parent advanced inside it and
   revalidation is required. The reading states its posture. A strand is never
   treated as a frozen copy of its anchor. A strand costs in proportion to what
   it appends.
4. **Rollback does not copy.** An abandoned tick or an undurable commit is not
   admitted. Echo does not copy a reading in advance so that it can put it
   back.
5. **Memory holds a working set.** Echo does not assume that the readings over
   a lane fit in memory, and it never requires two copies of one. It loads a
   part when something demands it and may release a part that content storage
   retains. An operation works on slices wherever a slice is enough. An
   operation that must visit every part streams them.
6. **Missing support is a posture, not a fallback.** When a demanded part is
   not retained, the result is an explicit obstruction or a rehydration
   requirement. Echo does not substitute another materialization silently.
7. **A cached reading is scoped.** A retained reading is valid for the causal
   coordinate, the optic or observer law, the aperture, the witness basis, the
   rights posture, the budget posture, the projection and reducer versions, and
   the residual, plurality or obstruction posture it names. Its identity
   includes all of them. Reusing it for a question that differs in any of them
   is a defect, and reuse never bypasses an obstruction.
8. **Sharing is not identity.** Two strands that share history or retained
   parts are still distinct causal lanes with distinct ancestry, authority and
   evidence. Sharing grants no authority and proves no relationship.
9. **Representation is deterministic.** The identity of a retained part and
   the order in which parts are visited depend only on content. They do not
   depend on memory addresses, insertion order or the path by which a reading
   was built.
10. **A wormhole keeps its obligations.** Compressing a tick range may drop or
    move the patches inside it. It may not drop the receipts and witness that
    the range stands for.
11. **History is not rewritten.** Rollback, reading at an earlier tick and
    forking select coordinates. They never alter admitted history.

These laws constrain internal machinery. They do not add a public state
object. Callers still propose intents against an explicit basis or observe
through a bounded optic.

## Design decomposition

The design has four parts. Sliced readings are the foundation.

1. **Sliced readings.** Each lane keeps an index from a slot to the ticks that
   produced it. A demanded slot is resolved through that index. A tick is
   prepared from the slots in its footprint.
2. **Emitted deltas.** A tick records the changes it makes as it makes them.
   Admitting a tick appends its patch and updates the index for the slots it
   wrote. No step compares one whole reading with another.
3. **Copy-on-write strands.** Forking records the basis. The strand resolves
   through its own producers and then through its source lane, with the basis
   posture of INV-S10.
4. **Content identity.** A retained part is referenced by the hash of its
   content. A part may be absent from memory while references to it stay
   valid, and the physical content port can hold it.

Parts 1, 2 and 3 do not change any committed hash.

## Open decisions that bound these laws

Two existing mechanisms prevent laws 1 and 2 from holding in full. Until each
is decided, no change may claim the law it blocks.

- **State root.** Every admitted tick commits a state root, and the current
  definition hashes a canonical stream of the whole reachable state. While
  that definition stands, admitting a tick costs in proportion to the
  reachable state, so law 2 is not met. Meeting it needs a decision that this
  page does not make: an incremental commitment that provably yields the same
  roots, or a migration of the root definition.
- **Footprint discovery for native rules.** A native `RewriteRule` computes
  its match and its footprint from a view over a complete instance, so its
  footprint is not known before the instance is materialized. Law 1 therefore
  applies first to Echo-interpreted operations, whose footprint is fixed by
  the admitted invocation. The native rule path stays outside these laws until
  it has a bounded discovery aperture or a footprint contract that does not
  depend on state.

## Current implementation

The runtime does not implement this design.

- Every worldline holds one complete materialized graph (`WorldlineState`,
  `GraphStore`) in memory. Ticks run against it and mutate it in place.
- A scheduler pass clones that graph for each worldline it touches so that it
  can restore it after a failure.
- `WorldlineRuntime::fork_strand` rebuilds the source graph at the fork tick by
  replay and registers the child with an independent copy.
- `ReplayCheckpoint` stores a complete materialized graph for one tick.
- Canonical state hashing and state comparison visit the graph. Each admitted
  tick computes a state root over the whole reachable state.
- A native rule's matcher and footprint function read a view over a complete
  instance.
- Tick patches record their input and output slots, and
  `slice_worldline_indices` computes the tick indices in a slot's dependency
  cone. Only tests call it.
- Wormhole remains a reserved term for tick-range compression.

Implementation state is owned by
[issue #782](https://github.com/flyingrobots/echo/issues/782), not by this
page.
