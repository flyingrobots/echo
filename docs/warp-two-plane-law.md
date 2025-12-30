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

**In code today:** `warp_core::GraphStore` is the SkeletonGraph.

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

- `NodeRecord.payload: Option<AtomPayload>`
- `EdgeRecord.payload: Option<AtomPayload>`

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

## Future: descended attachments (Stage B1)

Echo will support “WARPs all the way down” by making descent **explicit and skeleton-visible**.

The planned shape is **flattened indirection**, not nested Rust structs:

- attachments remain separate from the skeleton plane,
- descent happens through explicit references / skeleton links,
- matching/slicing can “see” the reference edges (no hidden structure).

This keeps the hot path fast while enabling recursive WARP structure for tools,
provenance, and multi-scale views.

