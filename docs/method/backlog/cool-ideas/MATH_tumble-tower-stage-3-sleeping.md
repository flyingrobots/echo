<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #234 is still open and blocked by
> Stage 2 (#233). `docs/guide/tumble-tower.md` defines sleeping/islands as
> Stage 3, but no Tumble Tower solver, sleep-state model, island builder, or
> performance benchmark exists yet.

# Stage 3: Sleeping

**Issue:** #234

**User Story:** As a learner completing the physics ladder, I want sleeping bodies and stable stacks so that I can see how physics engines optimize for steady-state configurations without breaking determinism.

## Requirements

- R1: Implement sleep detection: a body is a sleep candidate when its linear and angular velocity magnitudes are below a threshold for N consecutive ticks.
- R2: Sleeping bodies skip integration and collision response (but remain in the broad phase for wake-up detection).
- R3: Wake-up: a sleeping body is awakened when a non-sleeping body contacts it or when an external force is applied.
- R4: Island detection: groups of mutually-contacting bodies sleep/wake as a unit.
- R5: Sleep state is included in `physics_fingerprint` (sleeping vs awake is deterministic).

## Acceptance Criteria

- [ ] AC1: A stack of 10 boxes settles and all bodies enter sleep within 500 ticks.
- [ ] AC2: Dropping a new box on a sleeping stack wakes the contacted bodies (and their island).
- [ ] AC3: Sleep reduces per-tick computation: a 100-body sleeping scene processes in < 50% of the time of the same scene with sleep disabled.
- [ ] AC4: Golden vector: 10-box stack scenario with sleep produces identical fingerprints to the same scenario without sleep (sleep does not change physics outcomes, only skip computation).

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Sleep detection, wake-up, island grouping, performance optimization.
**Out of Scope:** Continuous collision detection for fast-moving bodies; adaptive timestep; GPU acceleration.

## Test Plan

- **Goldens:** Golden fingerprint for a 10-box stack with sleep enabled over 500 ticks (must match sleep-disabled golden).
- **Failures:** Sleep threshold of 0 (nothing ever sleeps, valid degenerate case); wake-up with no contact (external force API).
- **Edges:** Single body alone (island of 1); all bodies sleeping and no new inputs (simulation is effectively idle); body oscillating exactly at the sleep threshold.
- **Fuzz/Stress:** Benchmark: 500 bodies, measure tick time with and without sleep enabled over 1000 ticks.

**Blocked By:** stage-2-friction; operationally blocked until a runnable physics
solver exists to put bodies to sleep.
**Blocking:** desync-breakers

**Est. Hours:** 6h
**Expected Complexity:** ~450 LoC
