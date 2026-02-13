<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Tumble Tower](README.md) | **Priority:** P2
>
> This feature is a skeleton. Tasks will be expanded as the GDD matures.

# Course Material

**Issue:** #238

**User Story:** As a learner following the Echo tutorial path, I want structured course modules that walk me through the 4 physics stages so that I understand deterministic physics from AABB basics to sleeping/stability.

## Requirements

- R1: Write 5 course modules:
    - Module 1: "Boxes fall down" — AABB stacking, gravity, contact resolution.
    - Module 2: "Boxes spin" — rotation, OBB contacts, torque.
    - Module 3: "Boxes bounce and stick" — friction, restitution, material properties.
    - Module 4: "Boxes go to sleep" — sleeping, islands, performance.
    - Module 5: "Break the physics" — desync scenarios and lessons learned.
- R2: Each module includes runnable code snippets and a checkpoint assertion (expected fingerprint at a specific tick).
- R3: Modules are published as VitePress pages under `docs/guide/`.
- R4: Cross-reference the Splash Guy networking modules for learners doing both tracks.

## Acceptance Criteria

- [ ] AC1: All 5 modules are written and render correctly in VitePress.
- [ ] AC2: Code snippets compile and run against the current codebase.
- [ ] AC3: Each module has at least one checkpoint assertion with expected fingerprint.
- [ ] AC4: A reviewer can follow Module 1 from scratch and reach the checkpoint.

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** 5 course modules with code snippets, checkpoints, and cross-references.
**Out of Scope:** Video content; interactive playground beyond First Light; translations.

## Test Plan

- **Goldens:** n/a (docs)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** desync-breakers, visualization
**Blocking:** none

**Est. Hours:** 6h
**Expected Complexity:** ~1000 LoC (markdown + code snippets)
