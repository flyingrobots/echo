<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Tumble Tower](README.md) | **Priority:** P2
>
> This feature is a skeleton. Tasks will be expanded as the GDD matures.

# Visualization

**Issue:** #237

**User Story:** As a learner, I want a 2D visualization of Tumble Tower with debug overlays showing AABB outlines, velocity vectors, contact points, and sleep state so that I can visually understand what the physics engine is doing.

## Requirements

- R1: Render bodies as filled rectangles with rotation (OBB visual) in the browser WASM target.
- R2: Debug overlay toggles:
    - AABB outlines (green wireframe around the axis-aligned bounding box).
    - Velocity vectors (arrows from body center showing linear velocity direction/magnitude).
    - Contact points (red dots at contact locations, yellow for friction direction).
    - Sleep indicator (dim/grey bodies that are sleeping).
- R3: Display tick counter, body count, sleeping count, and `physics_fingerprint` in a HUD.
- R4: Rendering reads state via inspector protocol (read-only).

## Acceptance Criteria

- [ ] AC1: Bodies render with correct rotation and position.
- [ ] AC2: Each debug overlay toggle works independently.
- [ ] AC3: HUD displays correct tick, body count, sleeping count, and fingerprint.
- [ ] AC4: Rendering at 60fps for a 20-body scene.

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** 2D body rendering, 4 debug overlays, HUD, browser WASM target.
**Out of Scope:** 3D rendering; physics stage selection UI; recording/export.

## Test Plan

- **Goldens:** Screenshot golden for a 5-body scene with all debug overlays enabled.
- **Failures:** Zero bodies (empty scene, HUD still visible); body with extremely high velocity (arrow clipped to viewport).
- **Edges:** Body rotated exactly 360 degrees (renders same as 0); all bodies sleeping (entire scene dimmed).
- **Fuzz/Stress:** 100 bodies with all overlays enabled, maintain > 30fps.

**Blocked By:** stage-0-aabb
**Blocking:** course-material

**Est. Hours:** 6h
**Expected Complexity:** ~500 LoC
