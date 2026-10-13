<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Copy-on-Write Worldline State

- **Status:** Accepted design. **Not implemented.** The section
  [Current implementation](#current-implementation) states what the runtime
  does today.
- **Decision date:** 2026-10-12
- **Refines:** [Strands and Braids](../topics/StrandsAndBraids.md),
  [Terms: WARP State, SkeletonGraph, Instances, Portals, Wormholes](TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md)
- **Related:** [There Is No Graph](there-is-no-graph.md),
  [Echo × Keep physical content boundary](echo-keep-physical-content-boundary.md),
  [Fixed timestep invariant](../invariants/FIXED-TIMESTEP.md),
  [Strand contract invariant](../invariants/STRAND-CONTRACT.md)

## Decision

Materialized worldline state is a persistent structure. Two versions of a
state share every part that did not change between them.

A state version is named by its root. A root is a small value. Holding a root
keeps that whole version readable.

Each part of the structure is identified by the hash of its content. Equal
content is one part, wherever it appears. Parts can therefore be held by
content-addressed storage.

Materialization is driven by demand. To read or change some entities at a
basis, Echo needs the slice for them: the patches in their dependency cone,
with the portal chain that makes them reachable. Echo does not need the rest of
the state. A fully materialized state is a cache. It is never a prerequisite.

Echo does not assume that a state fits in memory. The parts held in memory are
a bounded working set. Echo loads a part when something demands it and may
release a part that content storage retains. No operation may require the whole
state in memory, and no operation may require two copies of it.

This page does not change what witnessed causal history is. History stays the
territory. A materialized state stays a reading of that history. Sharing is a
fact about how readings are represented.

## What each concept becomes

| Concept    | Meaning under this design                                                                 |
| ---------- | ----------------------------------------------------------------------------------------- |
| Tick       | Produces a new root that shares all unchanged parts with the previous root.               |
| Rollback   | Keeps the previous root. Nothing is restored, because nothing was overwritten.            |
| Strand     | Starts as another reference to the root at its fork basis. Copy-on-write from there.      |
| Checkpoint | A retained root at a tick boundary.                                                       |
| Slice      | The dependency cone that is enough to materialize the demanded entities at a basis.       |
| Wormhole   | A compressed range of ticks between two roots, with its retained receipts and witnesses.  |

## Laws

1. **A fork copies no state.** A strand costs storage and time in proportion to
   what it changes after the fork, not to the size of the state it forked from.
2. **A tick pays for what it changes.** The cost of committing a tick does not
   grow with the size of the unchanged state or with the length of history.
3. **Rollback does not copy.** An abandoned tick or an undurable commit is
   discarded by keeping the previous root.
4. **Materialize the cone, not the graph.** A tick or an observation
   materializes the entities in its footprint and what they depend on. Work
   that is proportional to the whole state is an optional cache fill, not a
   step that correctness requires.
5. **Memory holds a working set.** Correctness never depends on holding a
   whole state in memory. Hashing, comparison, checkpointing, forking and
   recovery work on the parts they touch. An operation that must visit every
   part streams them and does not hold them all.
6. **Sharing is not identity.** Two strands that share parts are still distinct
   causal lanes with distinct ancestry, authority and evidence. Sharing grants
   no authority and proves no relationship.
7. **Representation is deterministic.** The structure, its iteration order and
   its part identities depend only on content. They do not depend on memory
   addresses, insertion order or the path by which a state was reached.
8. **A wormhole keeps its obligations.** Compressing a tick range may drop or
   move the patches inside it. It may not drop the receipts and witness that
   the range stands for. A read that needs evidence that is not retained fails
   closed.
9. **History is not rewritten.** Rollback, stepping to an earlier tick and
   forking select roots. They never alter admitted history.

## Design decomposition

The design has four separable parts. Each can land without the next.

1. **Structural sharing in memory.** The graph store keeps its present
   interface and stores its maps as shared trees. Copying a state becomes
   copying a root.
2. **Content identity.** Each tree part carries the hash of its content. A
   state root can then be updated along the changed path only, and a part has
   an identity that the physical content port can hold.
3. **Emitted deltas.** A tick records the changes it makes as it makes them.
   No step compares one whole state with another.
4. **Sliced materialization.** A demanded entity is resolved from the parts
   under the current root, then from content storage, then by replaying its
   slice from the nearest retained root. Parts that nothing demands are never
   built. A reference to a part is its content identity, so a part may be
   absent from memory while the state that refers to it stays valid.

Whether the existing canonical state-root definition changes is a separate
decision. Parts 1, 3 and 4 do not change any committed hash.

## Current implementation

The runtime does not implement this design.

- `GraphStore` holds ordinary ordered maps and is mutated in place. Cloning a
  state copies every node, edge and attachment.
- A scheduler pass clones the graph state of each worldline it touches so that
  it can restore it after a failure.
- `WorldlineRuntime::fork_strand` rebuilds the source state at the fork tick by
  replay and registers the child with an independent copy of that state.
- `ReplayCheckpoint` stores a complete materialized state for one tick.
- Wormhole remains a reserved term for tick-range compression. No wormhole
  between state roots exists.
- Tick patches record their input and output slots, and
  `slice_worldline_indices` computes the tick indices in a slot's dependency
  cone. Only tests call it. The runtime keeps a complete materialized state for
  every worldline and runs each tick against that state.
- Every worldline's whole state is held in memory, and a scheduler pass holds
  more than one copy of it. Canonical state hashing and state comparison visit
  every reachable element.

Implementation state is owned by GitHub, not by this page.
