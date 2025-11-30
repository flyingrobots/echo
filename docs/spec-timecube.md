<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# TimeCube: Chronos × Kairos × Aion

Purpose
- Make the three axes of “time” first‑class so simulation, branching, and agency remain deterministic and replayable.
- Tie commit identity to ancestry (Merkle header) so there is no ambiguous arrival at a state.
- Express all subsystems (rendering, physics, serialization) as folds (catamorphisms) over the same data.

## Axes

**Chronos (Sequence)**
- Discrete ticks per branch: `Tick(u64)`.
- Fixed step interval: `Timespan { start: Transform, end: Transform }` represents `tick n → n+1`.
- Governs step order, replay, and snapshot lineage.

**Kairos (Possibility / Branch DAG)**
- Branch identifier: `BranchId(Hash)`; ancestry forms a DAG (merges allowed, no rebase).
- Possibility space at a tick: candidate rewrites, configuration‑space operations (Minkowski add/diff).
- Broad‑phase consumes conservative swept bounds for a timespan.

**Aion (Significance / Agency Field)**
- Universe identifier: `UniverseId(Hash)`; multiple universes exist without interaction by default.
- Significance: `Significance(i64)`; deterministic policy signal used for tie‑breaks and prioritization.
- Agency appears here as a pure policy function over state + logged inputs.

## Snapshot = Merkle Commit

```
struct SnapshotHeader {
  version: u16,
  universe: UniverseId,   // Aion axis
  branch:   BranchId,     // Kairos axis
  tick:     Tick,         // Chronos axis
  parents:  Vec<Hash>,    // 1 for linear, 2+ for merges
  policy:   AionPolicyId, // version pin for agency/tie‑breaks
}

struct SnapshotPayload {
  state_root:     Hash, // canonical graph hash (reachable only; stable order)
  plan_digest:    Hash, // digest of candidate set and deterministic ordering
  decision_digest:Hash, // digest of Aion scores/tie‑break inputs when used
  rewrites_digest:Hash, // digest of applied rewrites (ordered)
}

hash = BLAKE3(encode(header) || encode(payload)) // fixed endianness + lengths
```

Properties
- If two peers have the same snapshot hash, they have the same ancestry, state root, and the same deterministic choices. There is no “teleportation” into that state from a different path.
- Merges are explicit (2+ parents) with recorded decisions.

## Folds (Catamorphisms)

Principle
- Every subsystem is a fold over the same graph; traversal orders are canonical and stable.

Traversal (canonical)
- Nodes by ascending `NodeId` (BTreeMap key order).
- For each node, outgoing edges sorted by ascending `EdgeId`.
- Reachable‑only from the commit root (deterministic BFS).

Examples
- Serialization: fold → bytes; our snapshot hash is a digest of this canonical encoding.
- Rendering: fold → stable draw list (materials, instances) with a canonical order.
- Physics – Broad‑phase: fold (entities → local AABB), then combine with Chronos `Timespan` to produce swept bounds.

## Geometry & Kinematics

Types
- `Transform` (column‑major `T * R * S`), `Aabb`, `Vec3`, `Quat` are deterministic (`const` where possible). Zero is canonicalized (no `-0.0`).
- Chronos: `Timespan { start: Transform, end: Transform }`.
- Kairos::Swept: `SweptVolumeProxy { entity: u64, tick: Tick, fat: Aabb }` (current spike name: `SweepProxy`).

Swept Volume (CAD/graphics term)
- Pure translation by `d`: exact swept volume = `K ⊕ segment[0,d]` (Minkowski sum). The swept AABB equals the hull of start/end world AABBs.
- With rotation: we use a conservative bound (AABB hull of start/end) to remain deterministic and fast; narrow‑phase can refine later.

Kinematics Pipeline (per tick)
1) Chronos fold: compute `Timespan(n→n+1)` per entity from the integrator.
2) Geometry fold: local → world AABB at `start` and at `end`.
3) Swept bound: `fat = hull(AABB_start, AABB_end)`.
4) Kairos::Swept: build `SweptVolumeProxy { entity, tick, fat }` and insert into broad‑phase.
5) Broad‑phase output pairs in canonical order; narrow‑phase can test with configuration‑space tools later.

Determinism
- All inputs (transforms, shape parameters) are finite; transforms are `const` and canonicalize `-0.0`.
- Orders are explicit; AABB hull is associative/commutative; no FMA.

## Agency (Aion) without breaking determinism

Policy
- `AionPolicy::score(state, intent, candidate) -> Significance` (pure function).
- Incorporate `Significance` into deterministic ordering: e.g., `(scope_hash, family_id, -score, stable_tie_break)`.
- If a policy affects structure, include a digest of its inputs in `decision_digest`.

Use Cases
- Tie‑break conflicting rewrites consistently.
- Prioritize expensive folds (render/physics budgets) without affecting correctness.
- Log decisions so replay is identical across peers.


## Operations

 (safe moves)

- Fork branch (Kairos): split branch at commit C; new branch’s first parent is C.
- Merge branches (Kairos): new commit with parents [L, R]; MWMR + domain joins + Aion bias (deterministic), decision logged.
- Universe fork (Aion): clone Kairos repo into new `UniverseId`; no interaction thereafter unless via portal.
- Portal (Aion): explicit cross‑universe morph `F: U→U'`; landed commit includes `F` id/digest and parent in the source universe.

## Guarantees

- Snapshot identity pins ancestry and choices (Merkle); no ambiguous arrivals.
- Folds are canonical — “one true walk” — so views (render/physics/serialization) agree across peers.
- Aion biases choices deterministically; does not change the rewrite calculus.

## Migration Plan (no behavior change to start)

Step 1 — Namespacing & Docs
- Add `chronos::{Tick, Timespan}` and `kairos::swept::{SweptVolumeProxy}` re‑exports (compat with current paths).
- Document Minkowski addition and swept AABBs; link to CAD/physics references.

Step 2 — Snapshot Header Extensions
- Switch `parent: Option<Hash>` to `parents: Vec<Hash>`.
- Add `AionPolicyId`, `plan_digest`, `decision_digest`, `rewrites_digest`.

Step 3 — Fold Traits
- Introduce a simple `SnapshotAlg` and `fold_snapshot` helper with stable iteration.
- Port the serializer and physics spike through the fold (tests stay green).

Step 4 — Optional Narrow‑phase Prep
- Add `kairos::cspace` with Minkowski add/diff helpers and support functions for future GJK/CCD.

