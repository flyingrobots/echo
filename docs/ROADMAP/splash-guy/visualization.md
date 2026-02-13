<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Splash Guy](README.md) | **Priority:** P2
>
> This feature is a skeleton. Tasks will be expanded as the GDD matures.

# Visualization

**Issue:** #225

**User Story:** As a learner, I want a simple 2D rendering of the Splash Guy grid so that I can visually follow the game state during demos and debugging.

## Requirements

- R1: Render the grid as a 2D tile map (cell colors for empty, wall, player, balloon, explosion).
- R2: Display player identifiers and balloon fuse countdown numbers.
- R3: Render in the WASM browser target (using the website demo canvas from First Light).
- R4: Support a "debug overlay" toggle showing per-tick fingerprint and tick number.
- R5: Rendering reads state via the inspector protocol (read-only, no mutation).

## Acceptance Criteria

- [ ] AC1: Grid renders correctly for a 10x10 arena with 2 players and 3 balloons.
- [ ] AC2: Explosion animation shows chain reaction propagation across ticks.
- [ ] AC3: Debug overlay displays current tick and state fingerprint.
- [ ] AC4: Rendering does not affect game state (verified by fingerprint comparison with and without rendering).

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** 2D grid rendering, debug overlay, browser WASM target.
**Out of Scope:** Sound effects; particle effects; mobile-specific rendering; native desktop rendering.

## Test Plan

- **Goldens:** Screenshot golden for a known game state (2 players, 1 active explosion, debug overlay on).
- **Failures:** Render with zero players (empty grid, no crash); render with grid larger than viewport (scrolling or clipping).
- **Edges:** All cells occupied (maximum visual density); single-cell grid.
- **Fuzz/Stress:** Maintain 60fps rendering for a 20x20 grid with 10 simultaneous explosions.

**Blocked By:** rules-and-state
**Blocking:** course-material

**Est. Hours:** 6h
**Expected Complexity:** ~400 LoC
