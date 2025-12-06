<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `rmg-geom`

Geometry primitives for Echo: AABBs, transforms, temporal proxies, and
broad-phase scaffolding.

## What this crate does

- Provides foundational geometry types used by Echo’s engine and tools:
  - `Aabb` (axis-aligned bounding boxes),
  - `Transform` (position/rotation/scale),
  - temporal helpers such as `Timespan` and sweep proxies for motion.
- Serves as the basis for broad-phase collision and spatial indexing that can
  be shared between the engine (`rmg-core`) and visual tools (e.g.,
  future collision inspectors).

## Documentation

- Geometry and temporal background is documented in:
  - `docs/spec-deterministic-math.md`,
  - `docs/DETERMINISTIC_MATH.md`,
  - and related math/geometry notes in `docs/`.
- The Math booklet (`docs/book/echo/booklet-03-math.tex`) provides the
  conceptual backdrop for these types; future sections may call out `rmg-geom`
  explicitly as the implementation reference.

