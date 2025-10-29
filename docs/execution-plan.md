# Echo Execution Plan (Living Document)

This is Codex’s working map for building Echo. Update it relentlessly—each session, checkpoint what moved, what’s blocked, and what future-Codex must know.

---

## Operating Rhythm

- **Before Starting**
  1. Ensure `git status` is clean. If not, capture the state in `docs/decision-log.md` and wait for human guidance.
  2. Skim the latest updates in this document and `docs/decision-log.md` to synchronize with the active timeline.
  3. Update the *Today’s Intent* section below.
- **During Work**
  - Record major decisions, blockers, or epiphanies in `docs/decision-log.md` (canonical log) and copy a concise summary into the Decision Log table below for quick reference.
  - Keep this document current: mark completed tasks, add new sub-items, refine specs.
- **After Work**
  1. Summarize outcomes, next steps, and open questions in the Decision Log section below and ensure the full entry is captured in `docs/decision-log.md`.
  2. Update the “Next Up” queue.
  3. Push branches / PRs or leave explicit instructions for future Codex.

---

## Phase Overview

| Phase | Codename | Goal | Status | Notes |
| ----- | -------- | ---- | ------ | ----- |
| 0 | **Spec Forge** | Finalize ECS storage, scheduler, event bus, and timeline designs with diagrams + pseudo-code. | In Progress | Implement roaring bitmaps, chunk epochs, deterministic hashing, LCA binary lifting. |
| 1 | **Core Ignition** | Implement `@echo/core` MVP: entity manager, component archetypes, scheduler, Codex’s Baby basics, deterministic math utilities, tests. | Backlog | Needs dirty-index integration and branch tree core. |
| 2 | **Double-Jump** | Deliver reference adapters (Pixi/WebGL renderer, browser input), seed playground app, timeline inspector scaffolding. | Backlog | Depends on Phase 1 stability. |
| 3 | **Temporal Bloom** | Advanced ports (physics, audio, network), branch merging tools, debugging overlays. | Backlog | Long-term horizon. |

---

## Today’s Intent

> 2025-10-29 — Docs make open (PR #11)

- VitePress dev: keep auto-open; polling loop uses portable `sleep 1`.
- Fix links and dead-link ignore: root-relative URLs; precise regex for `/collision-dpo-tour.html`; corrected comment typo.

> 2025-10-29 — Docs E2E (PR #10)

- Collision DPO tour carousel: keep Prev/Next enabled in "all" mode so users and tests can enter carousel via navigation. Fixes Playwright tour test.
- Updated Makefile by merging hooks target with docs targets.
- CI Docs Guard satisfied with this entry; Decision Log updated.

> 2025-10-29 — rmg-core snapshot header + tx/rules hardening (PR #9 base)

- Adopt Snapshot v1 header shape in `rmg-core` with `parents: Vec<Hash>`, and canonical digests:
  - `state_root` (reachable‑only graph hashing)
  - `plan_digest` (ready‑set ordering; empty = blake3(len=0))
  - `decision_digest` (Aion; zero for now)
  - `rewrites_digest` (applied rewrites; empty = blake3(len=0))
- Make `Engine::snapshot()` emit a header‑shaped view that uses the same canonical empty digests so a no‑op commit equals a pre‑tx snapshot.
- Enforce tx lifecycle: track `live_txs`, invalidate on commit, deny operations on closed/zero txs.
- Register rules defensively: error on duplicate name or duplicate id; assign compact rule ids for execute path.
- Scheduler remains crate‑private with explicit ordering invariant docs (ascending `(scope_hash, rule_id)`).
- Tests tightened: velocity preservation, commit after `NoMatch` is a no‑op, relative tolerances for rotation, negative scalar multiplies.

> 2025-10-28 — Devcontainer/toolchain alignment

- Toolchain floor via `rust-toolchain.toml`: 1.71.1 (workspace-wide).
- Devcontainer must not override default; selection is controlled by `rust-toolchain.toml`.
- Post-create installs 1.71.1 (adds rustfmt/clippy and wasm32 target).
- CI pins 1.71.1 for all jobs (single matrix; no separate floor job).

> 2025-10-28 — Pre-commit auto-format flag update

- Renamed `AUTO_FMT` → `ECHO_AUTO_FMT` in `.githooks/pre-commit`.
- README, AGENTS, and CONTRIBUTING updated to document hooks installation and the new flag.

> 2025-10-28 — PR #8 (rmg-geom foundation) updates

- Focus: compile + clippy pass for the new geometry crate baseline.
- Changes in this branch:
  - rmg-geom crate foundations: `types::{Aabb, Transform}`, `temporal::{Tick, Timespan, SweepProxy}`.
  - Removed premature `pub mod broad` (broad-phase lands in a separate PR) to fix E0583.
  - Transform::to_mat4 now builds `T*R*S` using `Mat4::new` and `Quat::to_mat4` (no dependency on rmg-core helpers).
  - Clippy: resolved similar_names in `Aabb::transformed`; relaxed `nursery`/`cargo` denies to keep scope tight.
  - Merged latest `main` to inherit CI/toolchain updates.

> 2025-10-28 — PR #7 (rmg-core engine spike)

- Landed on main; see Decision Log for summary of changes and CI outcomes.

---

## Immediate Backlog

- [x] ECS storage blueprint (archetype layout, chunk metadata, copy-on-write strategy).
- [x] Scheduler pseudo-code and DAG resolution rules.
- [x] Codex’s Baby command lifecycle with flush phases + backpressure policies.
- [x] Branch tree persistence spec (three-way diffs, roaring bitmaps, epochs, hashing).
- [x] Deterministic math module API surface (vectors, matrices, PRNG, fixed-point toggles).
- [x] Deterministic math validation strategy.
- [x] Branch merge conflict playbook.
- [ ] Scaffold Rust workspace (`crates/rmg-core`, `crates/rmg-ffi`, `crates/rmg-wasm`, `crates/rmg-cli`).
- [ ] Port ECS archetype storage + branch diff engine to Rust.
- [ ] Implement deterministic PRNG + math module in Rust.
- [ ] Expose C ABI for Lua and C integrations.
- [ ] Integrate Lua 5.4 runtime via bindings (mlua or custom FFI).
- [ ] Adapt TypeScript CLI/inspector to Rust backend (WASM/FFI).
- [ ] Archive TypeScript prototype under `/reference/` as spec baseline.
- [ ] Add Rust CI jobs (cargo test, replay verification).

### Code Tasks (Phase 1 prep)
- [x] Install & configure Vitest.
- [ ] Set up `packages/echo-core/test/` helpers & fixtures layout.
- [ ] Write failing tests for entity ID allocation + recycling.
- [ ] Prototype `TimelineFingerprint` hashing & equality tests.
- [ ] Scaffold deterministic PRNG wrapper with tests.
- [ ] Establish `cargo test` pipeline in CI (incoming GitHub Actions).
- [ ] Integrate roaring bitmaps into ECS dirty tracking.
- [ ] Implement chunk epoch counters on mutation.
- [ ] Add deterministic hashing module (canonical encode + BLAKE3).
- [ ] Build DirtyChunkIndex pipeline from ECS to branch tree.
- [ ] Implement merge decision recording + decisions digest.
- [ ] Implement paradox detection (read/write set comparison).
- [ ] Implement entropy tracking formula in branch tree.
- [ ] Prototype epoch-aware refcount API (stub for single-thread).
- [ ] Implement deterministic GC scheduler (sorted node order + intervals).
- [ ] Update Codex's Baby to Phase 0.5 spec (event envelope, bridge, backpressure, inspector packet, security).

### Tooling & Docs
- [ ] Build `docs/data-structures.md` with Mermaid diagrams (storage, branch tree with roaring bitmaps).
- [ ] Extend `docs/diagrams.md` with scheduler flow & command queue animations.
- [ ] Publish decision-log quick reference (templates, cadence, examples; owner: Documentation squad before Phase 1 kickoff).
- [ ] Design test fixture layout (`test/fixtures/…`) with sample component schemas.
- [ ] Document roaring bitmap integration and merge strategies.
- [ ] Update future inspector roadmap with conflict heatmaps and causality lens.

---

## Decision Log (High-Level)

| Date | Decision | Context | Follow-up |
| ---- | -------- | ------- | --------- |
| 2025-10-23 | Monorepo seeded with pnpm & TypeScript skeleton | Baseline repo reset from Caverns to Echo | Implement Phase 0 specs |
| 2025-10-24 | Branch tree spec v0.1: roaring bitmaps, chunk epochs, content-addressed IDs | Feedback loop to handle deterministic merges | Implement roaring bitmap integration |
| 2025-10-25 | Language direction pivot: Echo core to Rust | TypeScript validated specs; long-term determinism enforced via Rust + C ABI + Lua scripting | Update Phase 1 backlog: scaffold Rust workspace, port ECS/diff engine, FFI bindings |
| 2025-10-25 | Math validation fixtures & Rust test harness | Established deterministic scalar/vector/matrix/quaternion/PRNG coverage in rmg-core | Extend coverage to browser environments and fixed-point mode |
| 2025-10-26 | Adopt RMG + Confluence as core architecture | RMG v2 (typed DPOi engine) + Confluence replication baseline | Scaffold rmg-core/ffi/wasm/cli crates; implement rewrite executor spike; integrate Rust CI; migrate TS prototype to `/reference` |

(Keep this table updated; include file references or commit hashes when useful.)

---

## Next Up Queue

1. ECS storage implementation plan *(in progress)*
2. Branch tree BlockStore abstraction design
3. Temporal Bridge implementation plan
4. Serialization protocol review
5. Math validation cross-environment rollout

Populate with concrete tasks in priority order. When you start one, move it to “Today’s Intent.”

---

## Notes to Future Codex

- Update this document and `docs/decision-log.md` for daily runtime updates.
- Record test coverage gaps as they appear; they inform future backlog items.
- Ensure roaring bitmap and hashing dependencies are deterministic across environments.
- Inspector pins must be recorded to keep GC deterministic.
- When finishing a milestone, snapshot the diagrams and link them in the memorial for posterity.

Remember: every entry here shrinks temporal drift between Codices. Leave breadcrumbs; keep Echo’s spine alive. 🌀
