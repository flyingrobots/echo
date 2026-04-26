<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Splash Guy | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #224 is still live and blocks the
> Splash Guy course track (#226). `docs/guide/splash-guy.md` and the course
> shell define the lesson; no controlled-desync harness exists yet.

# Controlled Desync

**Issue:** #224

**User Story:** As a learner, I want to intentionally break determinism in specific, documented ways so that I understand what goes wrong and how to detect it.

## Requirements

- R1: Create 3 desync scenarios, each as a toggleable flag:
    - Scenario A: Use `rand()` instead of seeded PRNG for balloon fuse timer
      (nondeterministic initial state).
    - Scenario B: Deliberately bypass `DFix64`/`F32Scalar` by using raw `f64`
      for explosion radius (cross-platform divergence).
    - Scenario C: Process inputs in arrival order instead of canonical order
      (network ordering nondeterminism).
- R2: Each scenario has a before/after explanation documenting what went wrong and why.
- R3: The two-peer harness detects the desync via fingerprint mismatch and reports which tick diverged.
- R4: Each scenario is runnable via a CLI flag or feature toggle.

## Acceptance Criteria

- [ ] AC1: Scenario A: two peers diverge within 10 ticks when `rand()` is used.
- [ ] AC2: Scenario B: two peers on different platforms (or with different compiler settings) produce different explosion results.
- [ ] AC3: Scenario C: two peers with different network arrival order produce different game states.
- [ ] AC4: Each scenario has a doc comment explaining the lesson learned.

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** 3 desync scenarios with detection and documentation.
**Out of Scope:** Desync recovery/repair; scenarios involving more than 2 peers.

## Test Plan

- **Goldens:** Golden "first divergent tick" for each scenario given fixed seeds/inputs.
- **Failures:** Scenario flag not set — no desync (confirms the flag is necessary).
- **Edges:** Desync on tick 0 (Scenario A with immediate divergence); desync on the last tick only.
- **Fuzz/Stress:** n/a (scenarios are deterministic given the flag).

**Blocked By:** lockstep-protocol
**Blocking:** course-material

**Est. Hours:** 5h
**Expected Complexity:** ~350 LoC
