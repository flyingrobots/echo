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
coordinate, aperture and law it names. It is a cache. It is not the runtime.

## What each concept becomes

| Concept    | Meaning under this design                                                                |
| ---------- | ---------------------------------------------------------------------------------------- |
| Tick       | Admits a suffix. It reads the slice for its footprint and appends the patch it produced. |
| Footprint  | The aperture of a tick: the slots it may read and write.                                 |
| Slice      | The part of history that is enough to build and justify a demanded reading.              |
| Rollback   | Does not admit the suffix. No reading has to be restored, because none was overwritten.  |
| Strand     | A lane that shares history up to its fork basis. It is copy-on-write.                    |
| Checkpoint | A retained reading at a tick boundary, with its coordinate and identity.                 |
| Wormhole   | A compressed range of ticks, with the receipts and witness that the range stands for.    |

## Laws

1. **Slices, not graphs.** No runtime operation requires a complete
   materialized graph. A tick or an observation builds the reading for its
   aperture and what that reading depends on.
2. **A tick pays for what it changes.** The cost of admitting a tick does not
   grow with the amount of history or with the size of any reading that the
   tick did not demand.
3. **A fork copies nothing.** A strand shares the history before its fork
   basis, because history is append-only. A reading on the strand resolves a
   slot from the strand's own producers, and otherwise from its source lane at
   the fork basis. A strand costs in proportion to what it appends.
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
7. **A cached reading is scoped.** A retained reading is valid for the
   coordinate, aperture, law and witness basis it names. Reusing it for another
   question is a defect.
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
   through its own producers and then through its source lane at the basis.
4. **Content identity.** A retained part is referenced by the hash of its
   content. A part may be absent from memory while references to it stay
   valid, and the physical content port can hold it.

Whether the existing canonical state-root definition changes is a separate
decision. Parts 1, 2 and 3 do not change any committed hash.

## Current implementation

The runtime does not implement this design.

- Every worldline holds one complete materialized graph (`WorldlineState`,
  `GraphStore`) in memory. Ticks run against it and mutate it in place.
- A scheduler pass clones that graph for each worldline it touches so that it
  can restore it after a failure.
- `WorldlineRuntime::fork_strand` rebuilds the source graph at the fork tick by
  replay and registers the child with an independent copy.
- `ReplayCheckpoint` stores a complete materialized graph for one tick.
- Canonical state hashing and state comparison visit the graph.
- Tick patches record their input and output slots, and
  `slice_worldline_indices` computes the tick indices in a slot's dependency
  cone. Only tests call it.
- Wormhole remains a reserved term for tick-range compression.

Implementation state is owned by
[issue #782](https://github.com/flyingrobots/echo/issues/782), not by this
page.
