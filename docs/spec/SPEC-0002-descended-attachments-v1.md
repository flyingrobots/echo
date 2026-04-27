<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0002 - Descended Attachments v1

_Define how an attachment slot can open into another WARP instance without creating hidden edges or recursive hot-path traversal._

Legend: KERNEL

Depends on:

- [SPEC-0001 - Attachment Plane v0 Typed Atoms](SPEC-0001-attachment-plane-v0-atoms.md)
- [WARP Tick Patch](warp-tick-patch.md)
- [Merkle Commit](merkle-commit.md)

## Why this packet exists

Echo needs nested state without losing deterministic replay or turning payload inspection into implicit graph traversal. Descended attachments solve that by making descent an explicit attachment-slot value and by recording the portal chain as witnessed causality.

## Human users / jobs / hills

Human users need nested state that can be inspected, replayed, and explained.

The hill: when a user asks why a value inside a descendant exists, the slice pulls in the portal chain that made the descendant reachable.

## Agent users / jobs / hills

Agent users need local reasoning. A rule running inside a descendant should not scan the whole runtime to discover how it got there.

The hill: the agent can read `WarpInstance.parent` and the descent stack to find the attachment slots that define reachability.

## Decision 1: Descent is flattened indirection

A descended attachment is `AttachmentValue::Descend(WarpId)`. The child instance is recorded separately as `WarpInstance { warp_id, root_node, parent }`.

## Decision 2: OpenPortal is the atomic authoring operation

Portal creation must not be split across independent edits. The replayable op is `OpenPortal { key, child_warp, child_root, init }`. Applying it establishes the child instance and writes the descent value into the attachment slot.

## Decision 3: Descended execution reads the descent chain

Any match or execution inside a descended instance must include read footprints for every attachment key in the descent stack. A descendant's meaning depends on the portal chain that makes it reachable, so portal changes must conflict with or invalidate descendant work.

## Decision 4: Matching stays instance-local

The scheduler and matcher operate within a named WARP instance unless a rule explicitly chooses another instance. There is no automatic recursive descent during matching.

## Decision 5: Slices close over portal producers

A slice that demands a slot inside a descendant must include the patches that established the portal chain. Portal slots appear in `in_slots`, so normal backward slice closure pulls in their producers.

## Consequences

Nested state is first-class and replayable. Descent does not weaken no-hidden-edges: the portal is an explicit attachment slot, and the child is a named instance with a recorded root.

## Open work

Finer-grained slicing within atom bytes, cross-instance edges that bypass portals, and history-compression behavior remain out of scope.
