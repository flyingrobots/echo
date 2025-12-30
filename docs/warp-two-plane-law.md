<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP Two-Plane Law (SkeletonGraph + Attachment Plane)

Echo implements WARP as a **two-plane** state object:

- **Skeleton plane**: explicit graph structure (nodes, edges, ports).
- **Attachment plane**: payloads *over* skeleton vertices/edges.

This doc exists to prevent a recurring confusion:

> “WARP” does not mean “arbitrary bytes that we *pretend* are a subgraph.”
>
> WARP means “graph structure is explicit and rewrite-visible; attachments are separate, typed, and opaque by default.”

---

## Definitions (terms we mean precisely in this repo)

### SkeletonGraph

**SkeletonGraph** is the structural graph used for:

- matching and indexing
- scheduling and independence checks (footprints)
- delta patch generation / replay (`WarpTickPatchV1`)
- canonical hashing (`state_root`)
- slicing (Paper III) over `in_slots` / `out_slots`

**In code today:** the skeleton plane is stored in `warp_core::GraphStore`:
- nodes: `GraphStore.nodes` (`NodeId -> NodeRecord`)
- edges: `GraphStore.edges_from` (`NodeId -> Vec<EdgeRecord>`)

### WarpState

Conceptually, a WARP state is:

```
U := (G, A)
```

Where:

- `G` is the SkeletonGraph (structure).
- `A` is the attachment plane (payloads attached to skeleton vertices/edges).

### Projection π(U)

`π` is the “forgetful projection” that drops attachments:

```
π : WarpState → SkeletonGraph
π(G, A) = G
```

Echo’s rewrite hot path is intentionally defined over `π(U)` (the skeleton).

---

## Attachment plane v0: depth-0 atoms only

In Paper I terms, depth-0 attachments are **atoms** `Atom(p)`.

In Echo v0, an atom is represented as:

- `AtomPayload { type_id: TypeId, bytes: Bytes }`

Where:

- `type_id` is a deterministic meaning tag (part of the boundary hash/digest).
- `bytes` are opaque to the core engine.

**In code today:**

- `NodeRecord` / `EdgeRecord` are **skeleton-plane only** (no payload fields).
- Attachments are stored separately on `GraphStore`:
  - node/α plane: `GraphStore.node_attachments: BTreeMap<NodeId, AttachmentValue>`
  - edge/β plane: `GraphStore.edge_attachments: BTreeMap<EdgeId, AttachmentValue>`
- Depth-0 payloads are `AttachmentValue::Atom(AtomPayload)`.

---

## Project laws (non-negotiable invariants)

### L1 — Skeleton rewrites never decode attachments

Core matching/indexing/scheduling must not parse or descend attachments.

Attachment decode is allowed only when a **rule/view explicitly chooses to decode**.

### L2 — No hidden edges in payload bytes

If a dependency matters for:

- matching,
- causality / independence,
- slicing correctness,
- replay correctness,

…then it must be represented as explicit skeleton structure.

Payload bytes must not “smuggle” structural dependencies the engine cannot see.

### L3 — Typed atoms participate in digests

Any canonical encoding/digest that includes payload bytes must also include the payload `type_id`.

This prevents collisions of the form:

> same bytes, different meaning → same hash (forbidden)

### L4 — Deterministic decode failure semantics (v0)

Echo’s v0 default is conservative:

- type mismatch or strict decode failure ⇒ “rule does not apply” (`NoMatch`)

Rules may choose to surface a stronger policy later (e.g., transactional abort),
but “silent partial effects” are forbidden.

---

## Descended attachments (Stage B1)

Echo supports “WARPs all the way down” by making descent **explicit and skeleton-visible**.

The shape is **flattened indirection**, not nested Rust structs:

- Attachments remain separate from the skeleton plane (typed, opaque by default).
- Descent is `AttachmentValue::Descend(WarpId)` (not encoded inside bytes).
- Each descended attachment points to a **WarpInstance** (a namespaced skeleton store),
  tracked in `warp_core::WarpState`.

Correctness law: execution inside a descended instance must READ the attachment
keys in its descent chain (so changing a portal pointer deterministically
invalidates matches and scheduling decisions).
