<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Tumble Tower](README.md) | **Priority:** P2
>
> This feature is a skeleton. Tasks will be expanded as the GDD matures.

# Desync Breakers

**Issue:** #236

**User Story:** As a learner, I want to intentionally break physics determinism in specific ways so that I understand why deterministic math and canonical ordering matter for physics simulations.

## Requirements

- R1: Create 3 physics desync scenarios, each as a toggleable flag:
    - Scenario A: Use `f32::sin`/`f32::cos` instead of the deterministic trig backend (cross-OS divergence in rotation).
    - Scenario B: Resolve contacts in HashMap iteration order instead of canonical order (nondeterministic resolution).
    - Scenario C: Use `f64` for intermediate impulse calculations and truncate to `f32` (precision-dependent results).
- R2: Each scenario has a before/after explanation.
- R3: The lockstep harness (lockstep-harness) detects each desync.

## Acceptance Criteria

- [ ] AC1: Scenario A: two peers on different platforms diverge within 50 ticks.
- [ ] AC2: Scenario B: two peers with different HashMap seeds diverge within 20 ticks.
- [ ] AC3: Scenario C: two peers diverge when intermediate precision differs.
- [ ] AC4: Each scenario has a doc comment explaining the physics determinism lesson.

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** 3 physics desync scenarios with detection and documentation.
**Out of Scope:** Desync recovery; scenarios involving sleeping bodies specifically.

## Test Plan

- **Goldens:** Golden "first divergent tick" for each scenario given fixed initial conditions.
- **Failures:** Scenario flag not set — no desync.
- **Edges:** Scenario A with axis-aligned rotation (sin/cos might not diverge; ensure the test uses non-trivial angles).
- **Fuzz/Stress:** n/a

**Blocked By:** stage-3-sleeping, lockstep-harness
**Blocking:** course-material

**Est. Hours:** 5h
**Expected Complexity:** ~350 LoC
