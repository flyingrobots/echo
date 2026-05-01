<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `warp-geom`

Geometry primitives for Echo: AABBs, transforms, temporal proxies, and
broad-phase scaffolding.

## What this crate does

- Provides foundational geometry types used by Echo’s engine and tools:
    - `Aabb` (axis-aligned bounding boxes),
    - `Transform` (position/rotation/scale),
    - temporal helpers such as `Timespan` and sweep proxies for motion.
- Serves as the basis for broad-phase collision and spatial indexing that can
  be shared between the engine (`warp-core`) and visual tools (e.g.,
  future collision inspectors).

## Documentation

- Geometry and temporal background is documented in:
    - `docs/determinism/DETERMINISTIC_MATH.md`,
    - `docs/determinism/SPEC_DETERMINISTIC_MATH.md`,
    - `docs/invariants/FIXED-TIMESTEP.md`,
    - and related determinism evidence in `docs/determinism/`.
