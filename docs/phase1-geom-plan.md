# Phase 1 Geometry/Collision Plan (Chronos/Kairos/Aion)

## Scope
- Time‑aware collision pipeline with deterministic broad/narrow phases and CCD.
- Shapes: AABB, Sphere, Capsule, OBB, Convex Hull (narrow later), Static Mesh BVH.
- No dynamics/solver in this phase (contacts only); rigid body adapter later.

## Milestones
- M0 — Scaffolding
  - Crate `rmg-geom` with `types/{transform,aabb,sphere,capsule,obb}.rs`.
  - Temporal types (`TickId`, `TimeSpan`, `TemporalTransform`, `TemporalProxy`).
  - Tests: encoding/decoding, basic overlaps, determinism hashes.
- M1 — Broad Phase v1
  - Dynamic AABB Tree; deterministic updates and pair emission.
  - Benchmarks vs grid/sap (micro); property tests (pairs invariant).
- M2 — Narrow Phase v1
  - Sphere/sphere, sphere/AABB, capsule/capsule, OBB/OBB (SAT).
  - Manifold builder (2–4 pts) with stable ordering.
- M3 — GJK + EPA
  - Convex/convex intersection and penetration; warm‑start cache.
  - Robust tolerances and fallbacks; golden tests for degenerate configs.
- M4 — CCD v1
  - Conservative advancement; swept sphere/capsule; quantized `toi_s`.
  - Policy thresholds; budget accounting.
- M5 — Static Mesh BVH
  - Offline/online build; dynamic–static queries; determinism tests.
- M6 — ECS Integration
  - Components (`Transform`, `Velocity`, `Collider`, `CcdPolicy`), systems (`broad`, `narrow`, `events`).
  - Inspector packet with `ContactEvent`s and hashes.
- M7 — Hardening
  - Fuzz/property tests; perf passes; docs completion; CI gates and coverage.

## Determinism & Time Contracts
- Fixed `dt`; CCD substeps bounded and recorded.
- Stable sorts/tie‑breakers by ids; centralized tolerances in one module.
- Quantization for `toi_s` and manifold points to avoid drift.
- Derived state only; contacts/pairs are recomputed per tick.

## Risks & Mitigations
- Numerical robustness (GJK/EPA/SAT):
  - Use tolerances/quantization; fallback paths; golden tests for degenerate configs.
- Performance regressions:
  - Bench suites; fat‑AABB sizing tuned; policy‑guided CCD only for fast movers.
- Determinism under parallelism:
  - Single‑thread by default; island partition + sorted merges for parallel mode.

## Test Strategy
- Unit tests per module; property tests for overlap symmetry/transitivity.
- Determinism tests: hash(pairs,contacts) stable across runs/platforms.
- CCD tests: bullets vs thin walls; quantized `toi_s` repeatability.

## Deliverables
- `docs/spec-geom-collision.md` (this spec) + API docs in code.
- `rmg-geom` crate with 90%+ coverage gate; CI lints identical to core.
- Inspector hooks and example scene demonstrating CCD and Aion tagging.

