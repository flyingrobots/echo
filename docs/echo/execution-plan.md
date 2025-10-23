# Echo Execution Plan (Living Document)

This is Codex’s working map for building Echo. Update it relentlessly—each session, checkpoint what moved, what’s blocked, and what future-Codex must know.

---

## Operating Rhythm
- **Before Starting**
  1. Ensure `git status` is clean. If not, document in Neo4j and wait for human guidance.
  2. Read the latest entries tagged `[Echo]` in Neo4j (`scripts/neo4j-msg.js messages --thread "echo-devlog"`).
  3. Update the *Today’s Intent* section below.
- **During Work**
  - Log major decisions, blockers, or epiphanies via Neo4j (`msg-send` with `[Echo]` prefix).
  - Keep this document current: mark completed tasks, add new sub-items, refine specs.
- **After Work**
  1. Summarize outcomes, next steps, and open questions in Neo4j.
  2. Update the “Next Up” queue.
  3. Push branches / PRs or leave explicit instructions for future Codex.

---

## Phase Overview

| Phase | Codename | Goal | Status | Notes |
| ----- | -------- | ---- | ------ | ----- |
| 0 | **Spec Forge** | Finalize ECS storage, scheduler, event bus, and timeline designs with diagrams + pseudo-code. | In Progress | Build data-structure diagrams; prototype membership benchmarks. |
| 1 | **Core Ignition** | Implement `@echo/core` MVP: entity manager, component archetypes, scheduler, Codex’s Baby basics, deterministic math utilities, tests. | Backlog | Requires spec sign-off. |
| 2 | **Double-Jump** | Deliver reference adapters (Pixi/WebGL renderer, browser input), seed playground app, timeline inspector scaffolding. | Backlog | Depends on Phase 1 stability. |
| 3 | **Temporal Bloom** | Advanced ports (physics, audio, network), branch merging tools, debugging overlays. | Backlog | Long-term horizon. |

---

## Today’s Intent
> Write the top priority for the current session and what “done” means.

- **Focus**: Outline scheduler benchmark prototype (goals, metrics, tooling).
- **Definition of done**: Notes on benchmark scenarios, measurement approach, and tasks for implementation.

---

## Immediate Backlog

- [x] ECS storage blueprint (archetype layout, chunk metadata, copy-on-write strategy).
- [x] Scheduler pseudo-code and DAG resolution rules.
- [x] Codex’s Baby command lifecycle with flush phases + backpressure policies.
- [x] Branch tree persistence spec (node structure, diff format, GC policy).
- [x] Deterministic math module API surface (vectors, matrices, PRNG, fixed-point toggles).

### Code Tasks (Phase 1 prep)
- [x] Install & configure Vitest (current `pnpm test` fails: `vitest` missing).
- [ ] Set up `packages/echo-core/test/` with Vitest configuration + helpers.
- [ ] Write failing tests for entity ID allocation + recycling.
- [ ] Prototype `TimelineFingerprint` hashing & equality tests.
- [ ] Scaffold deterministic PRNG wrapper with tests.
- [ ] Establish `pnpm test` pipeline in CI (incoming GitHub Actions).

### Tooling & Docs
- [ ] Build `docs/echo/data-structures.md` with Mermaid diagrams for storage + timeline tree.
- [ ] Extend `docs/echo/diagrams.md` with scheduler flow & command queue animations.
- [ ] Prepare Neo4j query cheatsheet for faster journaling.
- [ ] Design test fixture layout (`test/fixtures/…`) with sample component schemas.

---

## Decision Log (High-Level)

| Date | Decision | Context | Follow-up |
| ---- | -------- | ------- | --------- |
| 2025-10-23 | Monorepo seeded with pnpm & TypeScript skeleton | Baseline repo reset from Caverns to Echo | Implement Phase 0 specs |
| _…_ | | | |

(Keep this table updated; link to Neo4j message IDs when useful.)

---

## Next Up Queue
1. Scheduler pseudo-code benchmarks (prototype)
2. Codex’s Baby instrumentation plan
3. Deterministic math module validation tests

Populate with concrete tasks in priority order. When you start one, move it to “Today’s Intent.”

---

## Notes to Future Codex
- Use `Neo4j thread: echo-devlog` for daily runtime updates.
- Record test coverage gaps as they appear; they inform future backlog items.
- If the playground app needs new adapters, draft the API contracts here before coding.
- When finishing a milestone, snapshot the diagrams and link them in the memorial for posterity.

Remember: every entry here shrinks temporal drift between Codices. Leave breadcrumbs; keep Echo’s spine alive. 🌀
