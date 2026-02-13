<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Tumble Tower](README.md) | **Priority:** P2
>
> This feature is a skeleton. Tasks will be expanded as the GDD matures.

# Stage 2: Friction

**Issue:** #233

**User Story:** As a learner progressing through the physics ladder, I want friction and restitution (bounce) so that I can see how material properties affect deterministic physics.

## Requirements

- R1: Add per-body material properties: static friction coefficient, dynamic friction coefficient, restitution (coefficient of restitution).
- R2: Implement Coulomb friction model at contact points: tangential impulse clamped by `mu * normal_impulse`.
- R3: Implement restitution: relative velocity along contact normal scaled by coefficient of restitution to compute bounce impulse.
- R4: Combined material properties for a contact pair: use geometric mean for friction, max for restitution (configurable).
- R5: All fixed-point / scalar arithmetic remains deterministic across platforms.

## Acceptance Criteria

- [ ] AC1: A box dropped on a high-friction surface stops sliding within 20 ticks.
- [ ] AC2: A box dropped on a zero-friction surface slides indefinitely (velocity does not decay).
- [ ] AC3: A bouncy box (restitution = 0.9) bounces at least 5 times before settling.
- [ ] AC4: Golden vector: specific friction + restitution scenario matches known fingerprint at tick 300.

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Friction model, restitution, material properties, combined material computation.
**Out of Scope:** Sleeping (Stage 3); anisotropic friction; deformable bodies.

## Test Plan

- **Goldens:** Golden fingerprint for a 3-body scenario with mixed materials over 300 ticks.
- **Failures:** Negative friction coefficient (clamped to 0); restitution > 1.0 (energy gain, allowed but warned).
- **Edges:** Zero restitution (perfectly inelastic collision); friction coefficient of exactly 0 vs exactly 1; two bodies with identical material properties.
- **Fuzz/Stress:** Property test: random material properties for 10 bodies, verify energy is non-increasing (within floating-point tolerance) per tick when restitution <= 1.0.

**Blocked By:** stage-1-rotation
**Blocking:** stage-3-sleeping

**Est. Hours:** 5h
**Expected Complexity:** ~400 LoC
