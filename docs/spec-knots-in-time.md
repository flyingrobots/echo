# Knots In (and Over) Graphs — Time Knots for Echo

This memo captures two complementary ways to bring knot theory into Echo’s deterministic rewrite engine, and how that interacts with kinematics.

- A) Knot diagrams as first‑class graph objects inside a snapshot (Reidemeister moves as rewrite rules; invariants as folds)
- B) Time knots (braids) formed by worldlines across Chronos (and by branch/merge structure across Kairos)

It builds on TimeCube (Chronos × Kairos × Aion). See: `docs/spec-timecube.md`.

---

## A) Knot Diagrams as Typed Graphs

Represent a knot/link diagram as a typed, planar graph:

- Node types
  - `Cross`: 4‑valent vertex with an over/under bit (or a rotation system + overpass flag)
  - Optionally endpoints for tangles; closed links need none

- Edge type
  - `Arc`: oriented strand segment between crossings

- Embedding
  - Deterministic rotation system (cyclic order per vertex) to encode a planar embedding without float geometry

### Rewrites = Reidemeister Moves (DPO rules)

- R1 (twist): add/remove a kink loop (1 crossing)
- R2 (poke): add/remove a crossing pair (2 crossings)
- R3 (slide): slide a strand over another (3 crossings)

Each move is a local, typed Double‑Pushout rewrite and can be registered as an Echo rule with deterministic planning.

### Invariants as Folds (Catamorphisms)

- Crossing number, writhe: fold over crossings (with signed contribution)
- Kauffman bracket / Jones polynomial: state‑sum fold over a canonical crossing order
- Linking number: fold over components

Deterministic traversal is canonical: nodes by `NodeId`, edges per node by `EdgeId`, reachable from a chosen root. Invariants computed as folds are reproducible across peers.

---

## B) Time Knots: Braids in Chronos × Kairos

Two flavors that summarize “entanglement” deterministically:

1) **Worldline braids (Chronos)**
   - Choose a canonical 1‑D projection (e.g., x‑coordinate or lane index with a stable tiebreaker)
   - At each tick: sort entities; record adjacent swaps as Artin generators (sign from who passes “in front” under the projection)
   - Over a window of ticks: produce a braid word; closure yields a link; compute writhe/Jones/crossing count as folds

2) **Branch/merge braids (Kairos)**
   - Treat forks/merges in a branch DAG as a braid under a canonical branch ordering
   - A topological measure of “merge complexity”; can feed Aion (e.g., high complexity → high significance) without altering structure

Both are read‑only folds over commits; they do not change physics or rewrite semantics. They are deterministic analytics you can surface in the inspector or use to bias choices via Aion policies.

---

## Kinematics: Where Knots Touch Physics

We keep physics a **fold** over the graph and combine it with Chronos Timespans to obtain deterministic swept bounds.

1) Chronos: `Timespan { start: Transform, end: Transform }` per entity (n→n+1)
2) Geometry fold: local shape → world AABB at `start` and at `end`
3) Swept AABB (conservative swept volume proxy)
   - Pure translation by `d`: exact swept volume = Minkowski sum `K ⊕ segment[0,d]`; swept AABB equals hull of start/end world AABBs
   - With rotation: use conservative hull of start/end world AABBs (determinstic and fast); refine later if needed
4) Kairos::swept: build `SweptVolumeProxy { entity, tick, fat: Aabb }` and insert into broad‑phase (pairs in canonical order)

This is orthogonal to knot diagrams; the latter lives in the state graph as its own domain with its own rewrites and invariants.

---

## Determinism & Identity (No “Teleporting” States)

Echo commits are Merkle nodes (see `spec-timecube.md`). A snapshot’s hash includes:

- Ancestry (parents[])
- Canonical state root (reachable‑only graph hash; fixed sort orders)
- Plan/decision digests (candidate ordering and Aion‑biased tie‑break inputs when used)
- Applied rewrite digest (ordered)

If two peers share a commit hash, all folds (rendering, physics, knot invariants) produce identical results. There is no ambiguous arrival at a state through a different path.

---

## Roadmap (Small, Safe Steps)

1) Knot Diagram Demo (A)
   - Types: `knot::{Diagram, Cross, Arc}`
   - Rewrites: R1/R2/R3 rules (Echo DPO rules)
   - Folds: writhe/crossing count with tests (trefoil, figure‑eight)

2) Worldline Braid Metric (B1)
   - Fold a braid word from worldlines under a canonical projection per tick
   - Compute crossing count/writhe/Jones (state‑sum) as read‑only analytics
   - Inspector view: braid/entanglement overlay

3) Optional: Branch Braid Metric (B2)
   - Canonical branch ordering; braid from merges across a window; fold invariants

4) Docs
   - Link Minkowski addition primer (K ⊕ segment) in `kairos::cspace` rustdoc
   - Record invariants/algorithms as canonical folds in the code docs

---

## Notes on Minkowski Addition (Primer)

For convex sets `A, B ⊂ ℝ^n`: `A ⊕ B = { a + b | a∈A, b∈B }`.

- Collision: `A ∩ B ≠ ∅ ⇔ 0 ∈ A ⊕ (−B)` (basis for GJK/MPR)
- Translation: swept volume of `K` under translation by `d` over a timespan is `K ⊕ segment[0,d]`
- AABB of `K ⊕ segment[0,d]` equals the component‑wise hull of world AABBs at start and end

This is why our conservative swept bound is deterministic and exact for pure translation.

