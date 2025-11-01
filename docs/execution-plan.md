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

> 2025-10-30 — PR-01: Golden motion fixtures (tests-only)

- Add JSON golden fixtures and a minimal harness for the motion rule under `crates/rmg-core/tests/`.
- Scope: tests-only; no runtime changes.
- Links: PR-01 and tracking issue are associated for visibility.

> 2025-10-30 — Templates + Project board (PR: templates)

- Added GitHub templates (Bug, Feature, Task), PR template, and RFC discussion template.
- Configured Echo Project (Projects v2) Status options to include Blocked/Ready/Done.
- YAML lint nits fixed (no trailing blank lines; quoted placeholders).

> 2025-10-30 — Templates PR cleanup (scope hygiene)

- Cleaned branch `echo/pr-templates-and-project` to keep "one thing" policy: restored unrelated files to match `origin/main` so this PR only contains templates and the minimal Docs Guard notes.
- Verified YAML lint feedback: removed trailing blank lines and quoted the `#22` placeholder in Task template.
- Updated `docs/execution-plan.md` and `docs/decision-log.md` to satisfy Docs Guard for non-doc file changes.

> 2025-10-30 — Deterministic math spec (MD022)

- On branch `echo/docs-math-harness-notes`, fixed Markdown lint MD022 by inserting a blank line after subheadings (e.g., `### Mat3 / Mat4`, `### Quat`, `### Vec2 / Vec3 / Vec4`). No content changes.

> 2025-10-30 — Bug template triage fields

- Enhanced `.github/ISSUE_TEMPLATE/bug.yml` with optional fields for `Stack Trace / Error Logs` and `Version / Commit` to improve first‑pass triage quality.

> 2025-10-30 — Bug template wording consistency

- Standardized description capitalization in bug template to imperative form ("Provide …") for consistency with existing fields.

> 2025-10-30 — PR-03: proptest seed pinning (tests-only)

- Added `proptest` as a dev‑dependency in `rmg-core` and a single example test `proptest_seed_pinning.rs` that pins a deterministic RNG seed and validates the motion rule under generated inputs. This demonstrates how to reproduce failures via a fixed seed across CI and local runs (no runtime changes).

> 2025-10-30 — PR-04: CI matrix (glibc + musl; macOS manual)

- CI: Added a musl job (`Tests (musl)`) that installs `musl-tools`, adds target `x86_64-unknown-linux-musl`, and runs `cargo test -p rmg-core --target x86_64-unknown-linux-musl`.
- CI: Added a separate macOS workflow (`CI (macOS — manual)`) triggered via `workflow_dispatch` to run fmt/clippy/tests on `macos-latest` when needed, avoiding default macOS runner costs.

> 2025-10-30 — PR-05: docs rollup (echo-total.md)

- Added `scripts/gen-echo-total.sh` to generate `docs/echo-total.md` by concatenating top‑level docs in a stable order (priority: docs-index, architecture outline, execution plan, decision log; then others alphabetically). The rollup carries file banners and a generated timestamp.

> 2025-10-30 — PR-05 review fixes

- CI: In `ci.yml`, documented why the MUSL job tests only `rmg-core` (wasm/FFI intentional exclusions).
- Script portability: replaced echo with `printf` (and a plain `echo '---'`) to emit real newlines in `scripts/gen-echo-total.sh`; removed non-portable `\n` echo usage.
- Synced with `origin/main` via merge (no rebase/force).

> 2025-10-30 — PR-06: Motion negative tests (opened)

- Added tests in `rmg-core` covering NaN/Infinity propagation and invalid payload size returning `NoMatch`. Tests-only; documents expected behavior; no runtime changes.

> 2025-10-30 — PR-07: echo-total rollup check (CI)

- Added workflow `.github/workflows/echo-total-check.yml` that regenerates `docs/echo-total.md` and fails the PR if the file differs, prompting authors to update the rollup. Keeps the single-file doc in sync.

> 2025-10-30 — PR-08: Makefile target + README note (docs tooling)

- Added `make echo-total` target to run the rollup generator. README now documents `docs` commands and the rollup target.

> 2025-10-30 — PR-09: BLAKE3 header tests (tests-only)

- Added unit tests under `rmg-core` (in `snapshot.rs`) that:
  - Build canonical commit header bytes and assert `compute_commit_hash` equals `blake3(header)`.
  - Spot-check LE encoding (version u16 = 1, parents length as u64 LE).
  - Assert that reversing parent order changes the hash. No runtime changes.


> 2025-10-29 — Geom fat AABB midpoint sampling (merge-train)

- Update `rmg-geom::temporal::Timespan::fat_aabb` to union AABBs at start, mid (t=0.5), and end to conservatively bound rotations about off‑centre pivots.
- Add test `fat_aabb_covers_mid_rotation_with_offset` to verify the fat box encloses the mid‑pose AABB.

> 2025-10-29 — Pre-commit format policy

- Change auto-format behavior: when `cargo fmt` would modify files, the hook now applies formatting then aborts the commit with guidance to review and restage. This preserves partial-staging semantics and avoids accidentally staging unrelated hunks.

> 2025-10-29 — CI/security hardening

- CI now includes `cargo audit` and `cargo-deny` jobs to catch vulnerable/deprecated dependencies early.
- Rustdoc warnings gate covers rmg-core, rmg-geom, rmg-ffi, and rmg-wasm.
- Devcontainer runs `make hooks` post-create to install repo hooks by default.
- Note: switched audit action to `rustsec/audit-check@v1` (previous attempt to pin a non-existent tag failed).
- Added `deny.toml` with an explicit permissive-license allowlist (Apache-2.0, MIT, BSD-2/3, CC0-1.0, MIT-0, Unlicense, Unicode-3.0, BSL-1.0, Apache-2.0 WITH LLVM-exception) to align cargo-deny with our dependency set.
 - Audit job runs `cargo audit` on Rust 1.75.0 (explicit `RUSTUP_TOOLCHAIN=1.75.0`) to satisfy tool MSRV; workspace MSRV remains 1.71.1.

> 2025-10-29 — Snapshot commit spec

- Added `docs/spec-merkle-commit.md` defining `state_root` vs `commit_id` encoding and invariants.
- Linked the spec from `crates/rmg-core/src/snapshot.rs` and README.

> 2025-10-28 — PR #13 (math polish) opened

- Focus: canonicalize -0.0 in Mat4 trig constructors and add MulAssign ergonomics.
- Outcome: Opened PR echo/core-math-canonical-zero with tests; gather feedback before merge.

> 2025-10-29 — Hooks formatting gate (PR #12)

- Pre-commit: add rustfmt check for staged Rust files (`cargo fmt --all -- --check`).
- Keep PRNG coupling guard, but avoid early exit so formatting still runs when PRNG file isn't staged.
- .editorconfig: unify whitespace rules (LF, trailing newline, 2-space for JS/TS, 4-space for Rust).

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

> 2025-10-30 — rmg-core determinism tests and API hardening

- **Focus**: Address PR feedback for the split-core-math-engine branch. Add tests for snapshot reachability, tx lifecycle, scheduler drain order, and duplicate rule registration. Harden API docs and FFI (TxId repr, const ctors).
- **Definition of done**: `cargo test -p rmg-core` passes; clippy clean for rmg-core with strict gates; no workspace pushes yet (hold for more feedback).

> 2025-10-30 — CI toolchain policy: use stable everywhere

- **Focus**: Simplify CI by standardizing on `@stable` toolchain (fmt, clippy, tests, audit). Remove MSRV job; developers default to stable via `rust-toolchain.toml`.
- **Definition of done**: CI workflows updated; Security Audit uses latest cargo-audit on stable; docs updated.

> 2025-10-30 — Minor rustdoc/lint cleanups (rmg-core)

- **Focus**: Address clippy::doc_markdown warning by clarifying Snapshot docs (`state_root` backticks).
- **Definition of done**: Lints pass under pedantic; no behavior changes.

> 2025-10-30 — Spec + lint hygiene (core)

- **Focus**: Remove duplicate clippy allow in `crates/rmg-core/src/lib.rs`; clarify `docs/spec-merkle-commit.md` (edge_count may be 0; explicit empty digests; genesis parents).
- **Definition of done**: Docs updated; clippy clean.

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
