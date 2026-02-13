<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Splash Guy](README.md) | **Priority:** P2
>
> This feature is a skeleton. Tasks will be expanded as the GDD matures.

# Course Material

**Issue:** #226

**User Story:** As a learner following the Echo tutorial path, I want structured course modules that walk me through building Splash Guy step by step so that I understand deterministic networking from first principles.

## Requirements

- R1: Write 4 course modules:
    - Module 1: "State is a pure function" — introduce the game state model and deterministic rules.
    - Module 2: "Lockstep means trust" — walk through the input protocol and fingerprint exchange.
    - Module 3: "Break it to understand it" — guide through the 3 desync scenarios.
    - Module 4: "See what you built" — explain the rendering path and debug overlay.
- R2: Each module includes runnable code snippets that correspond to the actual implementation.
- R3: Each module ends with a "checkpoint" — a verifiable assertion the learner can confirm (e.g., "your fingerprint at tick 20 should be 0xABCD...").
- R4: Modules are published as VitePress pages under `docs/guide/`.

## Acceptance Criteria

- [ ] AC1: All 4 modules are written and render correctly in VitePress.
- [ ] AC2: Code snippets in each module compile and run against the current codebase.
- [ ] AC3: Each module has at least one checkpoint assertion.
- [ ] AC4: A reviewer who is not the author can follow Module 1 from scratch and reach the checkpoint.

## Definition of Done

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** 4 course modules with code snippets and checkpoints.
**Out of Scope:** Video content; interactive playground (beyond what First Light provides); translations.

## Test Plan

- **Goldens:** n/a (docs)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** controlled-desync, visualization
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~800 LoC (markdown + code snippets)
