<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARP Primer (Echo)

This is a newcomer-friendly explanation of what “WARP” means in the Echo codebase.
You do not need to know the papers or any category theory to follow this.

If you only remember one thing:

> Echo treats **structure** (the graph) and **data** (attachments) as separate planes.
> Rewriting, scheduling, hashing, and slicing happen over the **structure plane** unless a rule explicitly opts into decoding data.

---

## 1. The basic idea: state is a graph, changes are rewrites

Echo’s core state is a directed graph:

- **Nodes** represent “things”.
- **Edges** represent relationships.

A “tick” (one step of time) runs a set of deterministic rewrite rules:

- Each rule can match a local pattern around a chosen **scope** node.
- If it matches, it emits edits to the graph.
- Multiple candidate rewrites may be proposed; the engine deterministically chooses a conflict-free subset to apply.

This gives you:

- deterministic simulation
- replayable history
- auditability (“why did this happen?”)
- slicing (“what minimal history explains this output?”)

---

## 2. Two planes: SkeletonGraph + Attachment Plane

WARP state in Echo is “two-plane”:

```text
WarpState := (SkeletonGraph, Attachments)
π(WarpState) = SkeletonGraph
```

### 2.1 Skeleton plane (the hot path)

The **SkeletonGraph** is the explicit structure:

- node identities and node records
- edge identities and edge records
- adjacency relationships

This is the hot path:

- matching/indexing
- scheduling (conflict detection)
- canonical hashing (`state_root`)
- tick patch generation and replay
- slicing

In code, the skeleton plane for a single instance is stored in `warp_core::GraphStore`.

### 2.2 Attachment plane (typed atoms by default)

Attachments are payloads “over” skeleton elements (nodes/edges).

Echo represents a depth-0 attachment as a **typed atom**:

- `AtomPayload { type_id: TypeId, bytes: Bytes }`

Key property:

- The `type_id` is part of the deterministic boundary.
  - same bytes with a different meaning is not allowed to collide.

Attachment bytes are opaque to the engine unless a rule explicitly decodes them.

### 2.3 The “no hidden edges” law

Attachments must not be used to smuggle graph structure that the engine cannot see.

If something matters for:

- match applicability
- scheduling conflicts / causality
- slicing correctness
- replay correctness

…then it must be represented as explicit skeleton structure (or explicit attachment slot identity),
not buried inside payload bytes.

This is how Echo avoids building a system that “looks like WARP” but produces incorrect slices.

Canonical statement: `docs/warp-two-plane-law.md`.

Enforcement in `warp-core` is by construction:

- The only engine-recognized “structure inside data” mechanism is `AttachmentValue::Descend(WarpId)` (explicit portals).
  - `AtomPayload.bytes` are treated as opaque data; the engine never interprets them as skeleton structure.
- Matching/indexing/scheduling operate on `GraphStore` skeleton structure.
  - Attachments are only read/decoded if a rule explicitly calls attachment APIs.
- Typed decode failure is deterministic at the rule boundary.
  - For example, invalid motion payload bytes result in `ApplyResult::NoMatch` (rule does not apply):
    `crates/warp-core/tests/engine_motion_negative_tests.rs`.
- Attachment identity includes `type_id` at the deterministic boundary:
  `crates/warp-core/tests/atom_payload_digest_tests.rs`.

---

## 3. “Graphs all the way down” without recursive Rust structs (Stage B1)

If you literally store “a graph inside a node payload”, you get two problems:

- matching/scheduling can’t see structure inside bytes (violates “no hidden edges”)
- recursive traversal/decoding in the hot loop destroys performance and determinism ergonomics

Echo’s solution is **flattened indirection**:

- Attachments are still atoms by default.
- An attachment may also be `Descend(child_warp_id)`.

That `Descend(...)` does not contain a graph.
It points to another graph **instance** that lives alongside the current one.

Terminology note: “portals/instances” are *state recursion*. “Wormholes” are a different concept
(tick-range compression in the history/provenance plane). See
`docs/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md`.

### 3.1 WarpInstances (namespaces / layers)

Stage B1 introduces multiple graph instances:

- Each instance has a `WarpId`.
- Each instance has its own `GraphStore`.

This gives you “recursive state” without recursive storage:

- The overall state is `warp_core::WarpState` (many instances).
- Each instance is a normal `GraphStore` (fast skeleton operations).

### 3.2 Instance-scoped identity: `NodeId` vs `NodeKey`

Inside one instance, nodes are identified by `NodeId`.
Across instances, you must use `NodeKey`:

- `NodeKey { warp_id, local_id: NodeId }`

Same idea for edges (`EdgeId` vs `EdgeKey`).

This prevents accidental collisions where two instances both have a “node A”.

---

## 4. Portals and descent chains (why recursion stays slice-safe)

A **portal** is simply an attachment slot whose value is `Descend(child_warp_id)`.

But to make replay and slicing correct, portals have strict invariants:

- No **dangling portal**: you may not observe `Descend(child)` without the corresponding child instance existing.
- No **orphan instance**: a child instance that declares a `parent` portal must be realized by that portal slot pointing back to it.

### 4.1 Atomic portal authoring (`OpenPortal`)

To enforce those invariants at the boundary artifact level, portal authoring is atomic:

- `WarpOp::OpenPortal { key, child_warp, child_root, init }`

This single op:

1) ensures the child instance exists and is consistent
2) ensures the child root node exists (create or validate)
3) sets the portal slot to `Descend(child_warp)`

This prevents history where “the portal was set” but “the child universe never existed” (or vice versa).

### 4.2 Descent-chain footprinting

The subtle correctness law is:

- Any rewrite executed *inside* a descended instance must record READs of the portal chain that makes that instance reachable.

Echo enforces this in `Engine::apply_in_warp(tx, warp_id, rule, scope, descent_stack)`:

- callers pass `descent_stack: &[AttachmentKey]` (root → … → current)
- the engine injects those keys into the rewrite footprint as attachment reads

Why:

- changing any portal pointer changes reachability / meaning
- therefore it must invalidate matches deterministically

Downstream, those reads become `SlotId::Attachment(...)` entries in the tick patch `in_slots`,
which makes slicing automatically include the portal chain.

---

## 5. What happens in a tick (runtime pipeline)

At a high level, `warp-core` runs:

1) **Begin** a transaction
2) **Apply** rules to enqueue candidate rewrites
3) **Commit**:
   - deterministically schedule a conflict-free subset
   - execute them
   - emit deterministic boundary artifacts

Conflict detection is based on **Footprints** (read/write sets over nodes/edges/attachments/ports).

---

## 6. The boundary artifacts (history is first-class)

Each committed tick can emit:

- **Snapshot**
  - commits to `state_root` (canonical hash of reachable state)
  - commits to `patch_digest` (the replayable delta boundary)
- **TickReceipt**
  - records which candidates were applied vs rejected (and why)
  - can include within-tick blocking causality
- **WarpTickPatchV1**
  - the replayable delta (“these canonical ops happened”)
  - conservative `in_slots` / `out_slots` for slicing

The key design stance is:

> The patch is the boundary artifact for “what happened”.
> Receipts and planner digests are deterministic diagnostics, not consensus reality.

---

## 7. Slicing (why the portal-chain law matters)

Slicing answers:

> “Given a final slot/value, what subset of tick patches must I keep to replay just the dependency cone for it?”

If every patch has:

- `out_slots` = what it produced
- `in_slots` = what it depended on

…then slicing is just a backwards walk:

- find the last tick that produced the target slot
- include that tick
- enqueue its `in_slots`
- repeat

Stage B1 makes this work for descended instances because:

- work inside a child instance reads the portal chain slots
- therefore the slice includes the portal-opening tick(s)

---

## 8. Where to go next (Start here path)

Recommended reading order:

1) `docs/guide/warp-primer.md` — you are here (what WARP means in Echo).
2) `docs/spec-warp-core.md` — `warp-core` crate tour and API map.
3) `docs/warp-two-plane-law.md` — the hard laws (structure vs data, no hidden edges).
4) `docs/spec-merkle-commit.md` — state hashing + commit header semantics.
5) `docs/spec-warp-tick-patch.md` — tick patch boundary artifact (delta ops, hashing).
6) `docs/spec/SPEC-0002-descended-attachments-v1.md` — WarpInstances, portals, merge/DAG slicing semantics.
   (Terminology note: wormholes are *history compression*, not state descent; see the terms doc above.)
