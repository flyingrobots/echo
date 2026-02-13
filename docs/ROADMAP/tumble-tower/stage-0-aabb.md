<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Tumble Tower](README.md) | **Priority:** P2
>
> This feature is a skeleton. Tasks will be expanded as the GDD matures.

# Stage 0: AABB

**Issue:** #231

**User Story:** As a learner studying deterministic physics, I want the simplest possible physics simulation (axis-aligned bounding boxes with gravity and stacking) so that I can understand deterministic collision detection from first principles.

## Requirements

- R1: Implement 2D AABB (axis-aligned bounding box) representation: position, half-extents, velocity.
- R2: Implement gravity as a constant downward acceleration applied per tick.
- R3: Implement AABB-vs-AABB overlap detection (separating axis test on 2 axes).
- R4: Implement contact resolution: push overlapping boxes apart along the minimum penetration axis; zero out velocity along the contact normal.
- R5: All arithmetic uses `F32Scalar` (or `DFix64`) — no raw `f32` operations.
- R6: Per-tick `physics_fingerprint` for determinism verification.

## Acceptance Criteria

- [ ] AC1: A single box dropped from height settles on the ground plane within 60 ticks.
- [ ] AC2: A stack of 5 boxes remains stable for 200 ticks (no interpenetration, no drift).
- [ ] AC3: Two runs with identical initial conditions produce identical per-tick fingerprint sequences.
- [ ] AC4: Golden vector: specific 5-box stack scenario produces a known `physics_fingerprint` at tick 200.

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** AABB representation, gravity, overlap detection, contact resolution, fingerprinting.
**Out of Scope:** Rotation (Stage 1); friction/restitution (Stage 2); sleeping (Stage 3); networking.

## Test Plan

- **Goldens:** Golden fingerprint sequence for a 5-box drop scenario over 200 ticks.
- **Failures:** Box with zero half-extents (degenerate AABB, rejected at construction); negative gravity (boxes fly up, simulation still valid).
- **Edges:** Two boxes landing simultaneously; box exactly touching ground (zero penetration); very high stack (20 boxes).
- **Fuzz/Stress:** Property test: random initial positions and velocities for 10 boxes, verify no interpenetration after 500 ticks of settling.

**Blocked By:** none (First Light is an external blocker at the milestone level)
**Blocking:** stage-1-rotation, lockstep-harness, visualization

**Est. Hours:** 6h
**Expected Complexity:** ~450 LoC
