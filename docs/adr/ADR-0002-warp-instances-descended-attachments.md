<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# ADR-0002: WarpInstances + Descended Attachments via Flattened Indirection

Status: Accepted  
Date: 2025-12-30  
Owners: Echo / WARP core

## Context

The WARP theory defines a two-plane state object:

- **Skeleton plane**: explicit graph structure used for matching, rewriting, scheduling, slicing, determinism.
- **Attachment plane**: payloads over vertices/edges; attachments may be:
  - atomic (depth 0), or
  - recursively descended (“WARPs all the way down”).

Echo v0 implemented depth-0 attachments as **typed atoms** (Stage B0). However, depth‑0 alone is not the canonical “graphs all the way down” object: we need a first-class way to represent descended attachments without:

- nested recursive Rust heap structures,
- hidden structure embedded inside payload bytes,
- or recursive traversal/decoding in the rewrite hot path.

## Decision

We represent descended attachments as **flattened indirection** using **WarpInstances**.

1) Introduce **WarpInstances** identified by `WarpId`.
2) Instance-scope ids using 2D keys:
   - `NodeKey = { warp_id: WarpId, local_id: NodeId }`
   - `EdgeKey = { warp_id: WarpId, local_id: EdgeId }`
3) Extend attachment values:
   - `AttachmentValue = Atom(AtomPayload) | Descend(WarpId)`
4) Make attachment slots first-class and engine-visible:
   - `AttachmentKey { owner: AttachmentOwner(NodeKey|EdgeKey), plane: Alpha|Beta }`
5) Keep the rewrite/match hot path skeleton-only:
   - matching/indexing must not decode atoms or automatically traverse descended instances
   - descent is explicit (`Descend(WarpId)`), not encoded inside bytes
6) Authoring a descended instance is **atomic**:
   - creating the child instance and setting `Descend(child_warp)` must be performed as a single canonical operation (`OpenPortal`).
7) Multi-parent merges (DAG) require explicit resolution at the patch level:
   - merge commits must resolve slot conflicts (including attachment slots) deterministically.

## Laws (Non-Negotiable Invariants)

### L1 — No hidden edges

If structure participates in matching, causality/independence, slicing correctness, or replay correctness, it must be represented explicitly in the skeleton plane and/or via attachment slots.

Payload bytes must not be treated as “embedded graphs” that the engine cannot see.

### L2 — Skeleton rewrites never decode attachments

Core matching/indexing/scheduling operates over the skeleton plane. Attachment decoding happens only when a rule/view explicitly chooses to decode.

### L3 — Descended instances are reached only via attachment slots

Cross-instance connectivity in v1 is represented exclusively by `AttachmentValue::Descend(WarpId)` through an `AttachmentKey`. No arbitrary edges across instances.

### L4 — Descent-chain footprinting

Execution inside a descended instance carries a `descent_stack: Vec<AttachmentKey>` from the root instance down to the current instance.

Any match/exec within the descended instance MUST include READs of every attachment key in the descent stack.

Rationale: changing any descent pointer changes reachability/meaning and must deterministically invalidate matches and scheduling decisions.

### L5 — Slice closure includes the descent chain

If a slice demands any slot within instance `W`, the slice must include producers for the attachment chain establishing reachability (root → … → W). This is achieved by:

- treating attachment slots as slots (`SlotId::Attachment(AttachmentKey)`), and
- ensuring descended execution reads the descent-chain slots (L4), so the generic Paper III slice algorithm pulls in the portal producers.

### L6 — Portal authoring is atomic (OpenPortal)

Creating a child instance and setting `Attachment[key] = Descend(child_warp)` MUST be atomic at the patch level.

Slices/replay must not be able to observe:
- a “dangling portal” (`Descend(child_warp)` without a corresponding `WarpInstance(child_warp)`), or
- an “orphan instance” (a `WarpInstance` whose recorded `parent` slot does not point to it).

### L7 — Merge commits resolve slot conflicts explicitly

For any merge commit with multiple parents, if two or more parents write the same slot (including `SlotId::Attachment(...)`),
the merge commit MUST contain an explicit resolution write in its patch.

The merge is a first-class event, not implicit ancestry magic.

## Tooling note: instance zoom vs wormholes

Instance zoom/projection is a **state-structure** view derived from explicit `Descend(WarpId)` relationships.

Wormholes remain a **history/payload** compression mechanism over tick ranges. These are intentionally distinct.

## Consequences

Pros:
- Enables “WARPs all the way down” without recursive hot paths.
- Makes descent visible and hashable (no hidden structure).
- Preserves determinism and scales to large graphs.
- Allows multi-scale tooling by projecting instances as macro-nodes (state zoom), without conflating with wormholes (history compression).

Cons:
- Requires instance-scoped keys and explicit attachment slot identity.
- Adds small footprint overhead (bounded by descent depth).

## Alternatives Considered

1) Embed subgraphs inside payload bytes.
   - Rejected: violates “no hidden edges”; breaks slicing/causality; unsafe typing boundary.

2) Use recursive Rust data structures (`Box`, `Rc`, etc).
   - Rejected: poor fit for cycles/sharing, determinism, and patch/slice boundary artifacts.

## Follow-ups

- Spec: Descended Attachments v1 (WarpInstances + attachment slots + footprint laws).
- Tooling: state “zoom” views projecting instances and `Descend` links (explicitly not wormholes).
