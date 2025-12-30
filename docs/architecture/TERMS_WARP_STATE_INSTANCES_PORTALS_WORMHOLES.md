<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Terms: WARP State, SkeletonGraph, Instances, Portals, Wormholes

Status: Canonical Terminology (Project Law)  
Date: 2025-12-30  
Scope: Echo / WARP implementation

This document exists to prevent terminology collisions across:
- the published WARP papers (AIΩN Foundations series),
- older book drafts, and
- the Echo codebase.

If a term in code/docs conflicts with this file, **this file wins**.

---

## Core objects

### WARP state (`WarpState`)

Meaning: the theoretical two-plane state object.

`WarpState` is:
- **SkeletonGraph** `G` (explicit structure)
- **Attachment plane** `A` (payloads over skeleton vertices/edges)

Informally: `U = (G, A)` and `π(U) = G`.

In code today:
- `warp_core::WarpState` is the multi-instance container.
- Each instance’s skeleton + attachments live in `warp_core::GraphStore`.

### SkeletonGraph

Meaning: the explicit structural graph that rewriting/scheduling/slicing operates on.

SkeletonGraph contains:
- nodes, edges, ports (if applicable)
- adjacency / incidence relations
- stable ids

Law: SkeletonGraph is the **hot path**. Matching/indexing must remain skeleton-only (no attachment decode, no recursive traversal).

### Attachment plane

Meaning: attachments assigned to skeleton elements, stored separately from the skeleton.

Attachment values:
- `Atom(TypeId, Bytes)` (depth-0)
- `Descend(WarpId)` (indirection to a child instance)

Law: attachments are not “arbitrary bytes with implied structure.” Structure that matters must be explicit (see “No Hidden Edges”).

---

## Depth

### Depth 0

Meaning: a `WarpState` where all attachment values are atoms.

This is a valid state of the model (not “missing features”): atoms are the base case.

### Descended attachments (“WARPs all the way down”)

Meaning: an attachment slot value is `Descend(child_warp)` which points to a child instance.

Important: descended attachments are represented via **flattened indirection**, not nested Rust graphs/structs.

---

## Instances / layers

### `WarpInstance`

Meaning: a namespace/layer for a skeleton graph + its attachments.

Instances are identified by `WarpId`.

Each instance has:
- `warp_id`
- `root_node` (entry point into its skeleton)
- `parent: Option<AttachmentKey>` (the attachment slot that descends into it; `None` for the root instance)

Law: matching/indexing is scoped to a single instance.

### `NodeKey` / `EdgeKey`

Meaning: instance-scoped identifiers.

- `NodeKey = { warp_id, local_id }`
- `EdgeKey = { warp_id, local_id }`

This prevents cross-instance collisions and makes patching/slicing precise.

---

## Portals

### Portal

Meaning: the relationship created when an attachment slot is set to `Descend(child_warp)`.

Portals are not “edges hidden in bytes.” Portals are explicit, engine-visible indirection.

### `OpenPortal` (canonical operation)

Meaning: the atomic authoring op that:
- establishes/creates the child instance, and
- sets the `Descend(child_warp)` attachment value

Law: portals must be authored atomically. No dangling portals. No orphan instances.

---

## Wormholes (reserved term)

### Wormhole

Meaning (reserved): **history/provenance payload compression** over tick ranges.

Wormholes are a history/payload artifact (worldline compression), not a state-structure feature.

Law: do not use “wormhole” to refer to instance zoom, portals, or descended attachments.

---

## Views / projections

### Instance zoom / projection (state-scale)

Meaning: tooling view that shows instances as macro-nodes and portals as macro-edges.

This is derived from explicit `Descend(WarpId)` structure (state), not tick history (payload).

Law: instance zoom is not wormholes.

---

## Merge + slicing (DAG)

### Merge commit

Meaning: a commit with multiple parents.

Law: slot conflicts must be resolved explicitly by the merge patch (no implicit “parent order wins” semantics).

### Slice

Meaning: a provenance subgraph sufficient to replay/justify demanded outputs.

Slicing may be:
- worldline slice (single-parent fast path), or
- DAG slice (multi-parent general case)

Law: if a demanded slot is in a descended instance, the slice must include the portal chain establishing reachability.

---

## Project laws (quick list)

1) No hidden edges: never smuggle structure inside atom bytes.
2) Skeleton hot path: skeleton rewrites never decode attachments.
3) Typed atoms: Atom = (TypeId, Bytes); TypeId participates in canonical identity.
4) Flattened recursion: Descend uses indirection (WarpId), not nested structs.
5) Atomic portals: create child + set Descend in one canonical op (`OpenPortal`).
6) Merge explicitness: multi-parent conflicts resolved by merge patch writes.
7) Wormhole reserved: wormhole = history/payload tick-range compression only.
