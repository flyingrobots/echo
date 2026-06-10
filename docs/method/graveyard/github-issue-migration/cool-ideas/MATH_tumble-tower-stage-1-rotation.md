<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #232 is still open and blocked by
> Stage 0 (#231). `F32Scalar::sin_cos` and trig golden-vector tests exist, but
> no Tumble Tower OBB, angular dynamics, contact manifold, or torque solver
> exists yet.

# Stage 1: Rotation

**Issue:** #232

**User Story:** As a learner progressing through the physics ladder, I want rotation and oriented bounding boxes so that I can see how angular dynamics and OBB contact detection work deterministically.

## Requirements

- R1: Extend body representation with rotation angle (radians, stored as `F32Scalar`) and angular velocity.
- R2: Implement OBB-vs-OBB overlap detection using separating axis theorem (4 axes for 2D).
- R3: Implement contact point computation for OBB pairs (edge-vertex and edge-edge cases).
- R4: Apply torque from off-center contact forces (moment arm x contact impulse).
- R5: All trigonometric operations use `F32Scalar::{sin, cos, sin_cos}` backed
  by the deterministic LUT trig implementation; do not call platform
  `f32::{sin, cos}` in authoritative simulation code.

## Acceptance Criteria

- [ ] AC1: A rotated box (30 degrees) dropped onto a flat surface produces a deterministic rotation sequence.
- [ ] AC2: Two rotated boxes collide and exchange angular momentum correctly.
- [ ] AC3: Golden vector: specific rotated-stack scenario matches known fingerprint.
- [ ] AC4: Cross-OS test: same scenario on macOS and Linux produces identical fingerprints (deterministic trig verification).

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Rotation, angular velocity, OBB SAT, contact points, torque.
**Out of Scope:** Friction/restitution (Stage 2); sleeping (Stage 3); continuous collision detection.

## Test Plan

- **Goldens:** Golden fingerprint for a 3-OBB collision scenario over 100 ticks.
- **Failures:** Degenerate OBB (zero-width, handled as line segment); angular velocity exceeding one full rotation per tick (clamped).
- **Edges:** Boxes at exactly 0/90/180/270 degrees (axis-aligned OBB should match AABB results); two boxes with identical rotation and position (perfect overlap).
- **Fuzz/Stress:** Property test: random rotations and positions for 8 OBBs, verify SAT returns the same result as a brute-force overlap check.

**Blocked By:** stage-0-aabb
**Blocking:** stage-2-friction

**Est. Hours:** 6h
**Expected Complexity:** ~500 LoC
