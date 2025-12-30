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

## Slicing Semantics (worldline-scoped)

Paper III’s payload `P` is linear, while Echo history is a DAG.

This spec defines slicing with respect to a chosen **worldline path**:

- `P = (μ0, …, μn-1)` is the patch sequence along that worldline.

### S1: Slice closure includes the portal chain

If a slice demands any slot within instance `W`, the slice must include:

- producers for the demanded slots within `W`, and
- producers for the attachment chain from root → … → `W`:
  - the `SetAttachment(...Descend...)` ops that establish each descent step.

Mechanism:

- descended execution reads the descent stack (F2), so the generic Paper III slice algorithm over `in_slots`/`out_slots` pulls in the chain producers.

### S2: Conservatism rule

- Over-approximate `in_slots` is acceptable (slices get larger but remain correct).
- Under-approximate `in_slots` is forbidden (slices become incorrect).

## Matching / Performance Constraints

- Matching/indexing is skeleton-only and scoped to a single `warp_id` instance.
- No automatic traversal into descended instances during match.
- Attachment decoding is forbidden in the hot path.

## State Zoom (Tooling, explicitly NOT wormholes)

Tooling may provide a “state zoom” projection:

- treat each `WarpInstance` as a macro-node
- treat each `Descend` link as a macro-edge
- summarize instance content (counts, hashes) without expanding

This is a projection of state structure, not a history compression mechanism.

## Acceptance Criteria

1) Changing a `SetAttachment(...Descend...)` invalidates previously valid matches inside descendant instances (via descent-chain reads).
2) A slice that demands a slot inside a descendant instance includes the portal chain establishing reachability (root → … → W).
3) Matching/indexing code paths do not decode atom bytes.
4) Hash identity changes if `TypeId` changes even with identical bytes.
