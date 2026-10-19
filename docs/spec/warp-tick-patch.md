<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARP Tick Patch

_Define the replayable per-tick delta that carries witnessed state change across worldlines, slices, and audits._

Patch encoding version: 2

Depends on:

- [warp-core](warp-core.md)
- [Merkle Commit](merkle-commit.md)
- [SPEC-0002 - Descended Attachments v1](SPEC-0002-descended-attachments-v1.md)

## Purpose

Echo needs a retained artifact that says what changed without re-running the rules that produced it. The tick patch is that artifact: a canonical delta over carrier state, plus conservative slot I/O for slicing.

Replay must be independent of the original scheduler run. Given a prior state
and a patch, a verifier can apply the operations, recompute the state root, and
compare the expected hash.

The patch is also the stable unit for slicing and exchange. A consumer can
trace a target slot backward through `in_slots` and `out_slots` without
decoding opaque payload bytes.

## Decision 1: A tick patch is a delta, not a recipe

Replay applies ops; it does not invoke matchers, executors, or scheduler decisions. Receipts explain why settlement happened. Patches prescribe what to replay.

## Decision 2: Slots are unversioned in the patch

Patch slots name locations: `Node(NodeKey)`, `Edge(EdgeKey)`, `Attachment(AttachmentKey)`, `Port(PortKey)`, and `OutgoingEdges(OutgoingEdgeRange)`. Version identity is interpreted from patch position in the worldline.

## Decision 3: Ops are explicit carrier edits

Patch v2 supports `OpenPortal`, instance upsert/delete, node upsert/delete, edge upsert/delete, and `SetAttachment`. Attachment writes are explicit ops.

## Decision 4: Canonical ordering is part of the format

Slot tag order is node, edge, attachment, port, outgoing-edge range. Op replay order is open portals, upsert instances, delete instances, delete edges, delete nodes, upsert nodes, upsert edges, set attachments.

## Decision 5: `patch_digest` commits to the patch core

`patch_digest` is BLAKE3 over patch version, policy id, rule pack id, commit status, sorted `in_slots`, sorted `out_slots`, and sorted ops.

## Slicing contract

For a target slot, find the latest producer patch, add that patch, enqueue its `in_slots`, and repeat until the producer frontier closes or reaches the boundary state. Over-approximating `in_slots` is acceptable; under-approximating them is not.

## Outgoing-edge range slots

An outgoing-edge range key contains the source `NodeKey` and edge `TypeId`.
Slot tag 5 is followed by the 32-byte source WARP, 32-byte source node and
32-byte edge type, in that order, both in the patch digest stream and retained
provenance slot encoding. Existing point-slot bytes and digest inputs are
unchanged. A reader without range-tag support refuses the new tag rather than
interpreting it as a point slot.

A declared range read contributes an input slot. A declared range write
contributes both input and output slots, so tracing a range's next producer
also traces the prior aggregate. The engine's blocker reconstruction compares
these same range claims when explaining a scheduler rejection. A rejected
candidate contributes no range outputs.

The host slot reader returns matching full edge records sorted by edge ID.
An existing source with no matching edges is `Some([])`; an absent source is
`None`. A producer that depends on source presence must also record the source
node slot; the range key does not replace that point dependency. A range is an aggregate, so the point index's latest absolute write
cannot reconstruct it. Historical range reading currently clones the initial
state and replays the selected prefix. This includes relabeling, moving and
removing edges, and source deletion. Invalid replay history returns
`InvalidRangeHistory`. The borrowed initial state is never mutated.

That host reading has full-prefix replay and state-copy cost. It is not the
bounded, metered read aperture of an interpreted Edict operation. Native Tick
and retained-codec witnesses establish declared claim retention and replay;
they do not establish the authored operation effect bridge or automatically
record range reads and writes in existing graph accessors.
