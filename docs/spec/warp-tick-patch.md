<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARP Tick Patch

_Define the replayable per-tick delta that carries witnessed state change across worldlines, slices, and audits._

Legend: KERNEL

Patch encoding version: 2

Depends on:

- [warp-core](warp-core.md)
- [Merkle Commit](merkle-commit.md)
- [SPEC-0002 - Descended Attachments v1](SPEC-0002-descended-attachments-v1.md)

## Why this packet exists

Echo needs a retained artifact that says what changed without re-running the rules that produced it. The tick patch is that artifact: a canonical delta over carrier state, plus conservative slot I/O for slicing.

## Human users / jobs / hills

Human users need replay to be independent of the original scheduler run.

The hill: given a prior state and a patch, a verifier can apply the ops, recompute the state root, and compare the expected hash.

## Agent users / jobs / hills

Agent users need a stable unit for slicing and exchange.

The hill: an agent can trace a target slot backward through `in_slots` and `out_slots` without decoding opaque payload bytes.

## Decision 1: A tick patch is a delta, not a recipe

Replay applies ops; it does not invoke matchers, executors, or scheduler decisions. Receipts explain why settlement happened. Patches prescribe what to replay.

## Decision 2: Slots are unversioned in the patch

Patch slots name locations: `Node(NodeKey)`, `Edge(EdgeKey)`, `Attachment(AttachmentKey)`, and `Port(PortKey)`. Version identity is interpreted from patch position in the worldline.

## Decision 3: Ops are explicit carrier edits

Patch v2 supports `OpenPortal`, instance upsert/delete, node upsert/delete, edge upsert/delete, and `SetAttachment`. Attachment writes are explicit ops.

## Decision 4: Canonical ordering is part of the format

Slot tag order is node, edge, attachment, port. Op replay order is open portals, upsert instances, delete instances, delete edges, delete nodes, upsert nodes, upsert edges, set attachments.

## Decision 5: `patch_digest` commits to the patch core

`patch_digest` is BLAKE3 over patch version, policy id, rule pack id, commit status, sorted `in_slots`, sorted `out_slots`, and sorted ops.

## Slicing contract

For a target slot, find the latest producer patch, add that patch, enqueue its `in_slots`, and repeat until the producer frontier closes or reaches the boundary state. Over-approximating `in_slots` is acceptable; under-approximating them is not.
