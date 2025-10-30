# Time‑Aware Geometry, Collision, and CCD (Deterministic v1)

> Chronos (sequence) drives simulation, Kairos (possibility) enables branching, Aion (significance) informs precision and logging. This spec defines Echo’s deterministic, time‑aware collision pipeline and APIs.

This module is a graph‑first design: every artifact (proxies, pairs, contacts,
events, policies) is a typed node/edge within the world’s Recursive Meta Graph
and created via deterministic rewrite rules.

## Goals
- Deterministic across platforms and runs; branchable and mergeable.
- Minimal, composable primitives: geometry types, broad/narrow phases, CCD, and contact events.
- ECS‑friendly API; one concept per file; tests separate from code.
- Document tolerances and quantization to eliminate drift.

## Terminology
- Chronos: fixed timestep driving the engine (`dt`, `TickId`).
- Kairos: branching timelines/speculative substeps.
- Aion: event significance; which effects persist across branches and are surfaced to tools.

## Temporal Types
- `TickId(u64)`: discrete tick index.
- `TimeSpan { tick: TickId, dt: f32 }`: tick duration.
- `TemporalTransform { start: Transform, end: Transform }`: pose over a tick.
- `TemporalProxy { aabb_fat: Aabb, id: ProxyId, layers: u64 }`: broad‑phase proxy fattened to cover motion over `[start,end]`.
- `ContactId(Hash)`: stable hash of (ProxyId A, ProxyId B, feature ids, branch id).
- `Toi { s: f32, normal: Vec3, point: Vec3 }`: time‑of‑impact in `[0,1]` with contact info.

## Determinism Invariants
1. Fixed `dt` in core; substeps only through CCD with capped, recorded iterations.
2. Stable sorts everywhere (proxies, pairs, features, events); ties break by ids.
3. Tolerances centralized and quantized: overlap epsilon, TOI epsilon, manifold reduction.
4. Contacts, pairs, and manifolds are derived state each tick; not authoritative.
5. Identical inputs + rules ⇒ identical `Contact` and `ContactEvent` sequences and hashes.

## Geometry Types
- `Transform` (position `Vec3`, rotation `Quat`, scale `Vec3`).
- Primitive volumes: `Aabb`, `Sphere`, `Capsule`, `Obb`, `Hull` (convex).
- Static mesh: triangle lists + immutable BVH (for environment colliders).

## Broad Phase (Chronos‑aware)
- Dynamic AABB Tree (default)
  - Update proxies with fat AABBs based on velocity and angular bounds: `pad = v_max*dt + rot_margin` (quantized).
  - Deterministic updates: sorted insert/remove by `ProxyId`.
  - Outputs deterministic `PotentialPair { a, b }[]` sorted by (min_id, max_id).
- Sweep and Prune (optional)
  - Stable sort by endpoints, then by (id, axis). Scan for overlaps on `[start,end]`.
- Spatial Hash Grid (optional)
  - Canonical neighbor iteration order; fixed cell hashing.
- Static BVH for meshes
  - Prebuilt at load; deterministic traversal order and culling stats.

## Narrow Phase (Precise)
- Fast paths: sphere/sphere, sphere/AABB, capsule/capsule, OBB/OBB (SAT).
- Convex–convex: GJK for intersection/closest, EPA for penetration depth/direction.
- Manifold builder: clip and reduce to 2–4 points; stable point ordering by feature ids; quantized outputs.

## CCD
- Strategy: Conservative Advancement (CA) for general convex shapes; swept tests for spheres/capsules.
- Output: `Toi { s, normal, point }` with `s` quantized to bins in `[0,1]`.
- Policy (deterministic): CCD when any holds: `|v|*dt > size_thresh`, `ang*dt > angle_thresh`, or material requires CCD.

## Events (Temporal Bridge)
- `ContactEvent` kinds: `Begin { toi_s }`, `Persist`, `End`.
- Emission order per tick: sort by `(toi_s, ContactId)`; include Aion tags (materials/layers).
- Inspector packets include hashes of pair order and contact manifolds for divergence checks.

## ECS Integration (Phase 1 scope)
- Components
  - `Transform`, `Velocity` (for CCD), `Collider { shape, layer_mask, material, aabb_cache }`.
  - Internal staging: `PotentialPairs`, `Contacts`.
  - Policy: `CcdPolicy { thresholds, quant_bins }`.
- Systems
  - `broad_phase_system`: builds/updates proxies → emits deterministic `PotentialPairs`.
  - `narrow_phase_system`: consumes pairs → writes `Contacts` with manifolds/TOI.
  - `events_system`: compares last vs current contacts → emits `ContactEvent`s with Aion tagging.

## Public Traits (sketch)
```rust
pub trait BroadPhase {
    fn update(&mut self, tick: TickId, proxies: &[TemporalProxy]);
    fn find_pairs(&self, out: &mut Vec<PotentialPair>);
}

pub trait NarrowPhase {
    fn collide(&mut self, a: &Collider, ta: &TemporalTransform,
                        b: &Collider, tb: &TemporalTransform,
                        policy: &CcdPolicy) -> ContactOutcome;
}
```

## Instrumentation
- Per tick: proxies, pairs, CCD count, average `toi_s`, substeps, temporal budget usage.
- Hashes: `hash_pairs`, `hash_contacts` for quick determinism checks.

## Open Questions
- Exact quantization bins for `toi_s` (power‑of‑two vs decimal) and impact on merging.
- Obb/Obb preference: SAT vs GJK (choose based on shape type or heuristics?).
- Mesh collider scope in v1 (static only vs limited dynamic with compound shapes).
- Parallelization strategy and island sorting without violating determinism.
-

## Graph Mapping (Everything is a Graph)

Typed nodes (with indicative fields):
- `Tick` { id: TickId, dt: f32 }
- `Transform` { entity: NodeId, pos: Vec3, rot: Quat, scale: Vec3 }
- `Velocity` { entity: NodeId, lin: Vec3, ang: Vec3 }
- `Collider` { entity: NodeId, shape: ShapeRef, layer_mask: u64, material: Hash }
- `TemporalProxy` { id: TemporalProxyId, entity: NodeId, tick: TickId, aabb_fat: Aabb }
- `PotentialPair` { id: PairId, a: NodeId, b: NodeId, tick: TickId }
- `Contact` { id: ContactId, pair: PairId, tick: TickId, manifold: Manifold }
- `Toi` { pair: PairId, tick: TickId, s: Quantized<f32>, normal: Vec3, point: Vec3 }
- `ContactEvent` { kind: Begin|Persist|End, tick: TickId, pair: PairId, toi_s: Quantized<f32> }
- `CcdPolicy` { thresholds, quant_bins }
- `Material`, `Layer` (Aion tagging sources)

Typed edges (examples):
- `has_component(entity → Transform|Velocity|Collider)`
- `produced_in(x → Tick)` for all temporal artifacts
- `has_proxy(entity → TemporalProxy)`
- `pair_of(PotentialPair → a,b)`
- `contact_of(Contact → PotentialPair)`
- `event_of(ContactEvent → Contact)`
- `policy_for(CcdPolicy → Layer|Material)`

Deterministic ID recipes:
- `PairId = H(min(entityA,entityB) || max(entityA,entityB) || branch_id)`
- `ContactId = H(PairId || feature_ids || branch_id)`
- `TemporalProxyId = H(entity_id || tick_id || branch_id)`

## Rewrite Rules (DPOi sketches)

BuildTemporalProxy (pre_update):
- LHS: `Collider(e)`, `Transform(e)`, optional `Velocity(e)`, `Tick(n)`; no `TemporalProxy(e,n)`
- K: `Collider(e)`, `Transform(e)`, `Tick(n)`
- RHS: add `TemporalProxy(e,n)` with fat AABB; `has_proxy(e→proxy)`, `produced_in(proxy→Tick n)`

BroadPhasePairing (update):
- LHS: `TemporalProxy(a,n)`, `TemporalProxy(b,n)` with overlapping fat AABBs; no `PotentialPair(a,b,n)`
- K: both proxies
- RHS: add `PotentialPair(a,b,n)`; `pair_of(pair→a,b)`, `produced_in(pair→Tick n)`

NarrowPhaseDiscrete (update):
- LHS: `PotentialPair(a,b,n)`; discrete test says overlap at end pose; no `Contact(pair,n)`
- K: pair
- RHS: add `Contact(pair,n)` with `Manifold`; `contact_of(contact→pair)`, `produced_in(contact→Tick n)`

NarrowPhaseCCD (update):
- LHS: `PotentialPair(a,b,n)`; CCD policy says “run CA/swept”
- K: pair
- RHS: if intersect at `toi_s<1`: add/update `Toi(pair,n)`, `Contact(pair,n)`; else ensure `Contact` absent

ContactEvents (post_update):
- LHS: `Contact(pair,n−1)` and `Contact(pair,n)` (or absence)
- K: pair
- RHS: add `ContactEvent(kind, pair, tick=n, toi_s)`; link to `Contact` and `Tick`

GC Ephemeral (timeline_flush):
- LHS: `TemporalProxy|PotentialPair|Toi|Contact` older than retention and unreferenced
- RHS: delete node (deterministic order by id)

## Scheduler Phase Mapping
- `pre_update`: BuildTemporalProxy
- `update`: BroadPhasePairing → NarrowPhaseDiscrete/CCD (sorted scopes)
- `post_update`: ContactEvents (sort by `(toi_s, ContactId)`)
- `timeline_flush`: GC Ephemeral; persist Aion‑worthy events/metrics

## Diagrams (SVG)

- BuildTemporalProxy: docs/assets/collision/dpo_build_temporal_proxy.svg
- BroadPhasePairing: docs/assets/collision/dpo_broad_phase_pairing.svg
- NarrowPhaseDiscrete: docs/assets/collision/dpo_narrow_phase_discrete.svg
- NarrowPhaseCCD: docs/assets/collision/dpo_narrow_phase_ccd.svg
- ContactEvents: docs/assets/collision/dpo_contact_events.svg
- GC Ephemeral: docs/assets/collision/dpo_gc_ephemeral.svg
- Phase Mapping: docs/assets/collision/scheduler_phase_mapping.svg

These SVGs carry semantic classes (node, edge, interfaceK, added, removed, scope) and optional animation hooks (pulse-add/pulse-remove). You can style/animate them via CSS using docs/assets/collision/diagrams.css.
- See the visual tour for step‑by‑step DPO rules and world views: [`collision-dpo-tour.html`](./collision-dpo-tour.html).
