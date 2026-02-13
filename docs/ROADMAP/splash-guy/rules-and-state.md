<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Splash Guy](README.md) | **Priority:** P2
>
> This feature is a skeleton. Tasks will be expanded as the GDD matures.

# Rules & State Model

**Issue:** #222

**User Story:** As a learner studying deterministic game design, I want a complete, simple game with deterministic rules (grid arena, water balloons, fuse timers, chain reactions) so that I can see how all state transitions are pure functions of inputs.

## Requirements

- R1: Define the game state model: grid (NxM), player positions, balloon placements (position + fuse timer), explosion masks, score.
- R2: Implement deterministic rules: player movement (4-directional, collision with walls), balloon placement (fuse countdown per tick), explosion propagation (chain reactions via adjacency), player elimination.
- R3: All state is stored as Echo graph nodes/edges using Wesley-generated types.
- R4: State transitions are pure functions of (current state, admitted inputs) with no HostTime dependency.
- R5: Compute a per-tick `state_fingerprint` (hash of the full game state) for determinism verification.

## Acceptance Criteria

- [ ] AC1: Unit test: a scripted 20-tick game produces a deterministic final state hash (golden vector).
- [ ] AC2: Unit test: two runs with identical inputs produce identical per-tick fingerprint sequences.
- [ ] AC3: Chain reaction test: 3 adjacent balloons detonate in sequence, producing expected explosion pattern.
- [ ] AC4: Player elimination test: player caught in explosion is removed from the game state.

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Game state model, deterministic rules, per-tick fingerprinting.
**Out of Scope:** Networking (lockstep-protocol); rendering (visualization); AI opponents.

## Test Plan

- **Goldens:** Golden state hash for a scripted 20-tick game with 2 players, 5 balloon placements.
- **Failures:** Invalid move (off-grid) is rejected; balloon placement on occupied cell is rejected.
- **Edges:** Two balloons detonating on the same tick; player at grid boundary; grid completely filled with explosions.
- **Fuzz/Stress:** Property test: random input sequences always produce valid game states (no panics, no NaN, all positions in-bounds).

**Blocked By:** none (First Light is an external blocker at the milestone level)
**Blocking:** lockstep-protocol, controlled-desync, visualization

**Est. Hours:** 6h
**Expected Complexity:** ~500 LoC
