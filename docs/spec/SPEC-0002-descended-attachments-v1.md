<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# SPEC-0002: Descended Attachments v1 — WarpInstances + Flattened Indirection

Status: Draft (Implemented)  
Date: 2025-12-30  
Related:
- `docs/adr/ADR-0002-warp-instances-descended-attachments.md`
- `docs/spec/SPEC-0001-attachment-plane-v0-atoms.md`
- `docs/warp-two-plane-law.md`

## Goal

Provide a first-class mechanism for descended attachments (“WARPs all the way down”) that:

- preserves rewrite hot-path performance (skeleton-only matching/indexing),
- avoids hidden edges and recursive heap traversal,
- supports correct determinism, conflict detection (footprints), and slicing.

## Non-Goals (v1)

- Fine-grained slicing inside atom bytes (attachment “fragments”).
- Automatic recursive traversal of attachments during matching.
- Arbitrary cross-instance edges (only descent via attachment slots).
- Wormholes (history compression) — explicitly out of scope.

## Definitions

### Planes

- Skeleton plane: explicit graph structure (nodes, edges, ports).
- Attachment plane: a mapping of attachment slots to values.

### Depth

- Depth 0: all attachment values are `Atom(...)`.
- Descended: some attachment values are `Descend(warp_id)` pointing to another WarpInstance.

## Data Model

### Identifiers (instance-scoped)

- `WarpId`: identifies a WARP instance (a “layer” / namespace).
- `NodeId`, `EdgeId`: local identifiers within an instance.
- `NodeKey = { warp_id: WarpId, local_id: NodeId }`
- `EdgeKey = { warp_id: WarpId, local_id: EdgeId }`

### Attachment identity

- `AttachmentPlane = Alpha | Beta`
  - Alpha = vertex/node plane (`α`)
  - Beta = edge plane (`β`)
- `AttachmentOwner = Node(NodeKey) | Edge(EdgeKey)`
- `AttachmentKey = { owner: AttachmentOwner, plane: AttachmentPlane }`

### Attachment value

`AttachmentValue`:

- `Atom(AtomPayload)` where `AtomPayload = { type_id: TypeId, bytes: Bytes }`
- `Descend(WarpId)` (flattened indirection to another instance)

### Warp instance record

`WarpInstance`:

- `warp_id: WarpId`
- `root_node: NodeId` (local id within the instance store)
- `parent: Option<AttachmentKey>`
  - `None` for the root instance
  - `Some(k)` for descended instances reached via attachment slot `k`

The `parent` field enables deterministic “include the portal chain” slicing without searching the whole attachment plane.

## Operations

### O1: OpenPortal (atomic authoring)

`OpenPortal(key: AttachmentKey, child_warp: WarpId, child_root: NodeId, init: PortalInit)`

This is the canonical atomic operation for descended attachments. It MUST:
1) ensure `WarpInstance(child_warp)` exists with:
   - `parent = Some(key)`
   - `root_node = child_root`
2) ensure the child root node exists (via `init`)
3) set `Attachment[key] = Descend(child_warp)`

Validation invariants (post-apply):
- If `Attachment[key] == Descend(child_warp)`, then `WarpInstance(child_warp)` MUST exist.
- `WarpInstance(child_warp).parent == Some(key)`
- `WarpInstance(child_warp).root_node == child_root`

PortalInit (v1):
- `Empty { root_record }` => create the child instance/root node if missing
- `None` => require that the child instance/root node already exist

ID note (recommended, not required by this spec): `child_warp` should be deterministically authorable without randomness, but MUST be recorded in the op for replay and verification.

### O2: SetAttachment (atoms and clears)

Attachment slots may also be updated directly:

- `SetAttachment(key, Some(Atom(...)))`
- `SetAttachment(key, None)` (clears)

Setting `Descend(child_warp)` via a generic SetAttachment is discouraged in v1; prefer `OpenPortal` so portal creation and instance creation cannot be separated across ticks.

## Slot Model (for patches / footprints / slicing)

To keep slicing replayable without decoding atoms, attachments are treated as first-class slots:

- `SlotId::Node(NodeKey)` — skeleton node record
- `SlotId::Edge(EdgeKey)` — skeleton edge record
- `SlotId::Attachment(AttachmentKey)` — attachment slot value (Atom or Descend)
- `SlotId::Port(PortKey)` — boundary port value (opaque key)

## Footprint Semantics (descent-chain law)

### F1: Descent stack

Execution context carries:

- `descent_stack: Vec<AttachmentKey>`

This is the chain of attachment keys from the root instance down to the current instance.

### F2: Mandatory descent-chain reads

Any match/exec within a descended instance MUST include:

- READ `SlotId::Attachment(k)` for every `k` in `descent_stack`

This is required even if the rule does not otherwise reference those attachments.

Rationale: changing a descent pointer changes reachability/meaning and must invalidate matches deterministically.

### F3: Normal reads/writes

Reads/writes inside the current instance are tracked normally:

- reading nodes/edges/ports => READ those slots
- mutating nodes/edges/ports => WRITE those slots
- setting attachments => WRITE `SlotId::Attachment(key)`

## Slicing Semantics (Worldline + DAG)

Paper III’s payload `P` is linear, while Echo history is a DAG.

This spec defines slicing with respect to a chosen **worldline path**:

- `P = (μ0, …, μn-1)` is the patch sequence along that worldline.

### S1: Slice closure includes the portal chain

If a slice demands any slot within instance `W`, the slice must include:

- producers for the demanded slots within `W`, and
- producers for the attachment chain from root → … → `W`:
  - the `OpenPortal(...)` ops (and/or `SetAttachment(...Descend...)` legacy) that establish each descent step.

Mechanism:

- descended execution reads the descent stack (F2), so the generic Paper III slice algorithm over `in_slots`/`out_slots` pulls in the chain producers.

### S2: Conservatism rule

- Over-approximate `in_slots` is acceptable (slices get larger but remain correct).
- Under-approximate `in_slots` is forbidden (slices become incorrect).

### S3: DAG slicing (merge commits)

Worldline slicing is a fast path for single-parent history. For multi-parent history (merge commits), slicing must treat the merge patch as a first-class event.

Given a target commit `C` with parents `P1..Pn` and an initial demand set `D` (unversioned slots):

1) Include in the slice any ops in `C` that write slots in `D`.
2) For each demanded slot not written in `C`, follow parent provenance:
   - if exactly one parent can produce it, follow that parent for that slot
   - if multiple parents can produce it, the merge patch MUST resolve it (M1); otherwise the slice is invalid
3) For each included op, union its read-set into `D` and repeat until closure.
4) Portal-chain closure (Stage B1):
   - if any demanded slot is within a descendant instance `W`, include the portal chain establishing reachability (root → … → W)
   - in practice this is achieved by the descent-chain footprint law (F2), which ensures portal slots are read and pulled into the slice.

## Matching / Performance Constraints

- Matching/indexing is skeleton-only and scoped to a single `warp_id` instance.
- No automatic traversal into descended instances during match.
- Attachment decoding is forbidden in the hot path.

## State Zoom (Tooling, explicitly NOT wormholes)

Tooling may provide a “state zoom” projection (Instance Graph):

- Nodes: `WarpId` (WarpInstances)
- Edges: derived from explicit `Descend(WarpId)` portals (e.g., via OpenPortal ops or scanning attachment slots)
- Optional summaries (derived/cache):
  - node/edge counts per instance
  - root ids / root hashes
  - latest tick touching the instance

This is a projection of state structure, not a history compression mechanism.

Wormholes remain a history/payload compression mechanism (tick-range / patch-range compression) and are intentionally distinct from instance zoom.

## Merge Semantics (Multi-Parent DAG)

Echo history is a commit DAG. This spec requires:

### M1: Merge commits are first-class (explicit resolution)

A merge commit with parents `P1..Pn` MUST provide an explicit merge patch that resolves all slot conflicts deterministically.

Definition: a slot conflict exists if two or more parents contain writes to the same `SlotId` (including `SlotId::Attachment(...)`) along the ancestry relevant to the merge.

Requirement:
- If a conflict exists for slot `S`, the merge patch MUST contain a final write to `S`.
- If parents’ writes are disjoint for all slots in scope, the merge patch MAY be empty.

Rationale:
- We do not allow “implicit winner-by-parent-order” semantics to leak into determinism or slicing.
- Merge is an authored event, not implicit ancestry magic.

## Acceptance Criteria

1) Changing a portal slot value (including `OpenPortal` / `Descend`) invalidates previously valid matches inside descendant instances (via descent-chain reads).
2) A slice that demands a slot inside a descendant instance includes the portal chain establishing reachability (root → … → W).
3) Matching/indexing code paths do not decode atom bytes.
4) Hash identity changes if `TypeId` changes even with identical bytes.
5) OpenPortal is atomic: replay never observes `Descend(child_warp)` without a valid `WarpInstance(child_warp)` and root node.
6) Merge commits with conflicting parent writes to the same slot are invalid unless the merge patch writes the resolved final slot value.
