<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
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

> 2025-12-28 — PR #141 follow-up (new CodeRabbit review @ `b563359`) (COMPLETED)

- Goal: address newly posted CodeRabbit review comments on PR #141 (including minor/nitpick) and repair any newly failing CI jobs.
- Scope: re-extract paginated PR comments; bucket by severity; implement fixes with tests + doc alignment; update burn-down index + consolidated PR comment with fix SHAs.
- Exit criteria: `cargo test` + `cargo clippy --all-targets -- -D warnings -D missing_docs` green; PR checks green; consolidated summary comment updated with fix SHAs.
- Evidence: MSRV standardization + CI guard in `0f8e95d`; workspace deps fixes in `150415b` + `2ee0a07`; audit ignore DRY in `3570069` + `e5954e4`; deny license justification in `3e5b52d`; remove `"wasm"` categories in `3ccaf47`; stale advisory ignore removed in `1bf90d3`; Makefile guard rails in `8db8ac6`; doc style tweaks in `82fce3f`.

> 2025-12-28 — PR #141 follow-up (new CodeRabbit comments after `c8111ec`) (COMPLETED)

- Goal: address newly posted CodeRabbit review comments on PR #141 (including minor/nitpick) and ship a clean follow-up push.
- Scope: re-extract paginated PR comments; bucket by severity; implement fixes with tests + doc alignment; update burn-down index + consolidated PR comment with fix SHAs.
- Exit criteria: `cargo test` + `cargo clippy --all-targets -- -D warnings -D missing_docs` green; PR checks green; consolidated summary comment updated with new SHAs.
- Evidence: task-list/CI hardening in `602ba1e`, SPDX policy alignment in `042ec2b`, follow-up nits in `5086881`, docs fixes in `6ee8811` + `a55e1e0`, deny justification in `17687f2`.

> 2025-12-28 — PR #141 follow-up (new CodeRabbit round: Leptos bump + Rewrite semantics) (COMPLETED)

- Goal: address newly posted PR #141 review comments (Leptos 0.8.15 bump + fix `Rewrite` semantics around `old_value`) and ship a clean follow-up push.
- Scope: re-extract review comments with pagination; implement fixes with tests + doc alignment; re-check CI and repair any failing jobs; post one consolidated PR summary comment with fix SHAs.
- Exit criteria: `cargo test` + `cargo clippy --all-targets -- -D warnings -D missing_docs` green; PR checks green; summary comment updated with new fix SHAs.
- Evidence: `Rewrite` semantics fix in `1f36f77`, Leptos bump in `1a0c870`, and the refreshed burn-down index in `docs/notes/pr-141-comment-burn-down.md`.

> 2025-12-28 — PR #141 follow-up (new review comments + CI fixes) (COMPLETED)

- Goal: resolve newly posted PR review comments on #141, fix failing CI jobs, and ship a clean follow-up push.
- Scope: re-extract review comments with pagination; bucket by severity; implement fixes with tests + docs; inspect the latest GitHub Actions run and repair failing jobs/workflows if needed; post one consolidated PR summary comment with fix SHAs.
- Exit criteria: PR checks green; new review comments addressed; `cargo test` + `cargo clippy --all-targets -- -D missing_docs` green; follow-up summary comment posted.
- Evidence: follow-up fixes landed in `46bc079` (see `docs/notes/pr-141-comment-burn-down.md`).

> 2025-12-28 — PR #141 review comment burn-down (COMPLETED)

- Goal: extract, bucket, and resolve every PR comment on #141 with tests, fixes, and doc alignment.
- Scope: use `gh` + API to enumerate review + issue comments; verify stale vs actionable; implement fixes with minimal deterministic surface changes; update `docs/decision-log.md` and any impacted specs.
- Exit criteria: `cargo test` + `cargo clippy --all-targets -- -D missing_docs` green; PR thread includes fix SHAs; branch is pushable. (See `docs/notes/pr-141-comment-burn-down.md` @ `933239a`, PR comment: <https://github.com/flyingrobots/echo/pull/141#issuecomment-3694739980>)

> 2025-12-13 — WS gateway disconnect hygiene + Spec-000 WASM gating (COMPLETED)

- Goal: keep `cargo build`/`cargo test` green for the host target while still supporting `trunk serve` (wasm32) builds.
- Scope: gate `spec-000-rewrite` WASM entry points correctly; ensure `echo-session-ws-gateway` closes WS + stops ping task when upstream UDS disconnects.
- Status: completed; Spec-000 entrypoint is wasm32-gated and the WS gateway now closes + cancels ping on upstream disconnect. (PR #141: commits `2fec335`, `970a4b5`)

> 2025-12-11 — WebSocket gateway for session hub (COMPLETED)

- Goal: allow browser clients to connect to the Unix-socket session bus via a secure WS bridge.
- Scope: new `echo-session-ws-gateway` crate with WS→UDS forwarding, frame guards, origin allowlist, optional TLS.
- Status: completed; gateway parses JS-ABI frame lengths, enforces 8 MiB cap, and proxies binary frames over WS. (PR #141: commit `785c14e`; hardening in `89c2bb1`)

> 2025-12-11 — Scripting pivot to Rhai (COMPLETED)

- Goal: cement Rhai as the scripting layer across design/docs, update scripting backlog items, and log the pivot.
- Scope: execution plan, decision log, scripting/spec docs, FFI descriptions.
- Status: completed; scripting plans now target Rhai with deterministic sandboxing, prior scripting references removed. (commit `30b3b82`)

> 2025-12-11 — RMG authority enforcement (IMPLEMENTED; PENDING MERGE)

- Goal: Reject non-owner publishes on RMG channels and surface explicit errors to clients.
- Scope: `echo-session-service` (producer lock + error frames), `echo-session-client` (map error frames to notifications), protocol tasks checklist.
- Status: implemented on branch `echo/rmg-view-protocol-spec` (commit `237460e`); not yet merged to `main`.

> 2025-12-10 — CI cargo-deny index failures (COMPLETED)

- Goal: stop noisy `warning[index-failure]: unable to check for yanked crates` in GitHub Actions by ensuring `cargo-deny` has a warm crates.io index.
- Scope: `.github/workflows/ci.yml` deny job (prime cargo index before running `cargo deny`).
- Status: completed; deny job now runs `cargo fetch --locked` before `cargo deny`.

> 2025-12-10 — CI cargo-audit unmaintained warnings (COMPLETED)

- Goal: keep `cargo audit --deny warnings` green despite unavoidable unmaintained transitive `paste` (via wgpu) and legacy `serde_cbor` advisory.
- Scope: `.github/workflows/security-audit.yml` and `.github/workflows/ci.yml` (add `--ignore RUSTSEC-2024-0436` and `--ignore RUSTSEC-2021-0127`).
- Status: completed; audit steps now ignore these advisories explicitly until upstreams replace them.

> 2025-12-10 — RMG View Protocol tasks (IN PROGRESS)

- Goal: land the RMG View Protocol/EIP checklist and execute slices toward multi-viewer sharing demo.
- Scope: tracked in `docs/tasks.md` with stepwise commits as items complete.
- Status: checklist drafted.

> 2025-12-10 — CBOR migration + viewer input gating (COMPLETED)

- Goal: swap serde_cbor for maintained ciborium, harden canonical encoding/decoding, and keep viewer input/render stacks consistent.
- Scope: `crates/echo-session-proto` (ciborium + serde_value bridge, canonical encoder/decoder), `crates/echo-graph` (ciborium canonical bytes + non_exhaustive enums), `crates/rmg-viewer` (egui patch alignment, input/app events/session_logic gating, hash mismatch desync), dependency lockfile.
- Status: completed; wire encoding now uses ciborium with checked integer handling and canonical ordering, graph hashing returns Result, viewer controls are gated to View screen with safer event handling and consistent egui versions.

> 2025-12-10 — Session client framing & non-blocking polling (COMPLETED)

- Goal: make session client polling non-blocking, bounded, and checksum-aligned.
- Scope: `crates/echo-session-client/src/lib.rs` (buffered try_read polling, MAX_PAYLOAD guard, checksum-respecting frame sizing, notification drain, tests).
- Status: completed; poll_message is now non-blocking, enforces an 8 MiB cap with checked arithmetic, preserves buffered partials, and poll_notifications drains buffered notifications only.

> 2025-12-10 — Viewer timing & viewport safety (COMPLETED)

- Goal: stabilize per-frame timing and prevent viewport unwrap panics.
- Scope: `crates/rmg-viewer/src/app_frame.rs` (dt reuse, angular velocity with dt, safe viewport access, single aspect computation, window lifetime).
- Status: completed; dt is captured once per frame, spins/decay use that dt, viewport access is guarded, and helper signatures no longer require 'static windows.

> 2025-12-10 — Config + docs alignment (COMPLETED)

- Goal: keep docs aligned with code and maintained deps.
- Scope: `crates/echo-config-fs/README.md` (ConfigStore naming, doc path), `crates/echo-session-proto/src/lib.rs` (explicit reexports, AckStatus casing), `docs/book/echo/sections/06-editor-constellation.tex` + TikZ legend/label tweaks.
- Status: completed; README references correct traits/paths, proto surface is explicit with serde renames, figure labeled/cross-referenced with anchored legend.

> 2025-12-06 — Tool crate docs + crate map (COMPLETED)

- Goal: tighten docs around the tool hexagon pattern and make crate-level READMEs point at the Echo booklets as the canonical source of truth.
- Scope: `docs/book/echo/sections/09-tool-hex-pattern.tex` (crate map), READMEs and `Cargo.toml` `readme` fields for `echo-app-core`, `echo-config-fs`, `echo-session-proto`, `echo-session-service`, `echo-session-client`, and `rmg-viewer`.
- Status: completed; Tools booklet now includes a crate map, and each tool-related crate README has a “What this crate does” + “Documentation” section pointing back to the relevant booklets/ADR/ARCH specs.

> 2025-12-06 — JS-ABI + RMG streaming docs alignment (COMPLETED)

- Goal: Align Echo’s book-level docs with the JS-ABI v1.0 deterministic encoding + framing decisions (ADR-0013 / ARCH-0013) and the new RMG streaming stack.
- Scope: `docs/book/echo/sections/{13-networking-wire-protocol,14-rmg-stream-consumers,07-session-service,08-rmg-viewer-spec}.tex` (cross-links, diagrams, tables).
- Status: completed; Core booklet now documents JS-ABI framing + generic RMG consumer contract (with role summary), and Tools booklet’s Session Service + RMG Viewer sections cross-reference that contract instead of re-specifying it.

> 2025-12-04 — Sync roadmap with session streaming progress (COMPLETED)

- Goal: capture the new canonical `echo-graph` crate + gapless RMG streaming path, and queue remaining engine/viewer wiring tasks.
- Scope: update `crates/rmg-viewer/ROADMAP.md`, note outstanding engine emitter + client extraction; log decisions.
- Status: completed.

> 2025-12-03 — Recover rmg-viewer ROADMAP after VSCode crash

- Goal: confirm whether roadmap edits were lost and restore the latest saved state.
- Scope: `crates/rmg-viewer/ROADMAP.md` sanity check vs git.
- Status: completed; file matches last commit (no recovery needed).

> 2025-12-03 — Persist rmg-viewer camera + HUD settings between runs (COMPLETED)

- Goal: write config load/save so camera + HUD toggles survive restarts.
- Scope: `crates/rmg-viewer/src/main.rs`, add serde/directories deps; update roadmap/docs.
- Status: completed; config saved to OS config dir `rmg-viewer.json`, loads on startup, saves on close.

> 2025-12-03 — Extract core app services and refactor viewer (COMPLETED)

- Goal: stop config/toast creep in rmg-viewer; introduce shared core + fs adapter; make viewer consume injected prefs.
- Scope: new crates `echo-app-core` (ConfigService/ToastService/ViewerPrefs) and `echo-config-fs`; rewire `rmg-viewer` to use them and drop serde/directories.
- Status: completed; prefs load/save via ConfigService+FsConfigStore; viewer owns only rendering + HUD state; toast rendering pending.

> 2025-12-04 — Session proto/service/client skeleton (COMPLETED)

- Goal: set up the distributed session slice with shared wire types and stub endpoints.
- Scope: new crates `echo-session-proto` (messages), `echo-session-service` (stub hub), `echo-session-client` (stub API); roadmap/docs updates.
- Status: completed; schema covers Hello/RegisterRmg/RmgDiff+Snapshot/Command+Ack/Notification; transport and viewer binding are next.

> 2025-12-01 — LaTeX skeleton + booklets + onboarding/glossary (COMPLETED)

- Goal: scaffold reusable LaTeX parts (master + per-shelf booklets), wire logos, and seed onboarding + glossary content for Orientation.
- Scope: `docs/book/echo` (preamble, title/legal pages, parts/booklets, Makefile) and new Orientation chapters.
- Status: completed; master + booklets build, onboarding/glossary live.

> 2025-12-01 — Set canonical package manager to pnpm in `package.json`

- Goal: declare pnpm as the repo’s package manager via the `packageManager` field.
- Scope: `package.json` only.
- Status: completed; set to `pnpm@10.23.0` to match local toolchain.

> 2025-12-01 — Fix cargo panic warning in bench profile (COMPLETED)

- Goal: Silence the `warning: panic setting is ignored for bench profile` message during `cargo test`.
- Scope: `Cargo.toml`.
- Changes: Removed `panic = "abort"` from `[profile.bench]`.
- Status: Completed; warning no longer appears.

> 2025-11-30 – Handle subnormal f32 values in F32Scalar

- Goal: Canonicalize subnormal f32 values to zero.
- Scope: subnormals, F32Scalars.
- Plan: Make 'em zero.

> 2025-12-01 — Fix “How Echo Works” LaTeX build (non-interactive PDF)

- Goal: unblock `docs/guides/how-do-echo-work` PDF generation without interactive TeX prompts.
- Scope: tidy TikZ arrows/ampersands, add Rust listing language, harden LaTeX Makefile to fail fast.
- Plan: clean artifacts, adjust TeX sources, re-run `make` until `main.pdf` builds cleanly.

> 2025-12-01 — Book accuracy + visuals refresh

- Goal: align the “How Echo Works” guide with the current code (scheduler kinds, sandbox, math invariants) and add clearer visuals/tables.
- Scope: scan `rmg-core` for scheduler, sandbox, and math implementations; update prose, tables, and TikZ diagrams; remove layout warnings.
- Status: completed; PDF now builds cleanly with updated figures and code snippets.

> 2025-12-01 — License appendix + SPDX CI

- Goal: add a LaTeX license appendix and wire CI to enforce SPDX headers.
- Scope: new `legal-appendix.tex` included in the guide; GitHub Action `spdx-header-check.yml` runs `scripts/check_spdx.sh --check --all`.
- Status: added appendix and workflow.

> 2025-11-30 — PR #121 feedback (perf/scheduler)

- Goal: triage and address CodeRabbit review feedback on scheduler radix drain/footprint changes; ensure determinism and docs guard stay green.
- Scope: `crates/rmg-core/src/scheduler.rs`, related engine wiring, and any doc/bench fallout; keep PendingTx private and fail-fast drain semantics intact.
- Plan: classify feedback (P0–P3), implement required fixes on `perf/scheduler`, update Decision Log + docs guard, run `cargo clippy --all-targets` and relevant tests.
- Added: pluggable scheduler kind (Radix default, Legacy BTreeMap option) via `SchedulerKind`; legacy path kept for side-by-side comparisons.
- Risks: regress deterministic ordering or footprint conflict semantics; ensure histogram O(n) performance and radix counts remain u32 without overflow.

> 2025-12-01 — Sandbox harness for deterministic A/B tests

- Goal: enable spawning isolated Echo instances (Engine + GraphStore) from configs to compare schedulers and determinism.
- Scope: `rmg-core::sandbox` with `EchoConfig`, `build_engine`, `run_pair_determinism`; public `SchedulerKind` (Radix/Legacy).
- Behavior: seed + rules provided as factories per instance; synchronous per-step determinism check helper; threaded runs left to callers.

> 2025-11-06 — Unblock commit: rmg-core scheduler Clippy fixes (follow-up)

- Goal: make pre-commit Clippy pass without `--no-verify`, preserving determinism.
- Scope: `crates/rmg-core/src/scheduler.rs` only; no API surface changes intended.
- Changes:
  - Doc lint: add backticks in `scheduler.rs` docs for `b_in`/`b_out` and `GenSet(s)`.
  - Reserve refactor: split `DeterministicScheduler::reserve` into `has_conflict`, `mark_all`, `on_conflict`, `on_reserved` (fix `too_many_lines`).
  - Tests hygiene: move inner `pack_port` helper above statements (`items_after_statements`), remove `println!`, avoid `unwrap()`/`panic!`, use captured format args.
  - Numeric idioms: replace boolean→int and lossless casts with `u64::from(...)` / `u32::from(...)`.
  - Benches: drop unused imports in `reserve_scaling.rs` to avoid workspace clippy failures when checking all targets.
- Expected behavior: identical drain order and semantics; minor memory increase for counts on 64‑bit.
- Next: run full workspace Clippy + tests, then commit.
  - CI follow-up: add `PortSet::iter()` (additive API) to satisfy scheduler iteration on GH runners.
> 2025-11-30 – F32Scalar canonicalization and trait implementations (COMPLETED)

- Goal: Ensure bit-level deterministic handling of zero for `F32Scalar` and implement necessary traits for comprehensive numerical behavior.
- Scope: `crates/rmg-core/src/math/scalar.rs` and `crates/rmg-core/tests/math_scalar_tests.rs`.
- Changes:
    - `F32Scalar` canonicalizes `-0.0` to `+0.0` on construction.
    - `F32Scalar` canonicalizes all NaNs to `0x7fc00000` on construction (new).
    - `value` field made private.
    - `PartialEq` implemented via `Ord` (total_cmp) to ensure `NaN == NaN` (reflexivity).
    - `Eq`, `PartialOrd`, `Ord`, `Display` traits implemented.
- Added: Tests for zero canonicalization, trait behavior, and NaN reflexivity.
- Risks: Introducing unexpected performance overhead or subtly breaking existing math operations; mitigated by unit tests and focused changes.

> 2025-11-29 – Finish off `F32Scalar` implementation

- Added `rmg-core::math::scalar::F32Scalar` type.

> 2025-11-03 — Issue #115: Scalar trait scaffold

- Added `rmg-core::math::scalar::Scalar` trait declaring deterministic scalar operations.
- Arithmetic is required via operator supertraits: `Add/Sub/Mul/Div/Neg` with `Output = Self` for ergonomic `+ - * / -` use in generics.
- Explicit APIs included: `zero`, `one`, `sin`, `cos`, `sin_cos` (default), `from_f32`, `to_f32`.
- No implementations yet (F32Scalar/DFix64 follow); no canonicalization or LUTs in this change.
- Exported via `rmg-core::math::Scalar` for consumers.

> 2025-11-02 — PR-12: benches updates (CI docs guard)

- Dependency policy: pin `blake3` in `rmg-benches` to exact patch `=1.8.2` with
  `default-features = false, features = ["std"]` (no rayon; deterministic, lean).
- snapshot_hash bench: precompute `link` type id once; fix edge labels to `e-i-(i+1)`.
- scheduler_drain bench: builder returns `Vec<NodeId>` to avoid re-hashing labels; bench loop uses the precomputed ids.

> 2025-11-02 — PR-12: benches polish (constants + docs)

- snapshot_hash: extract all magic strings to constants; clearer edge ids using `<from>-to-<to>` labels; use `iter_batched` to avoid redundant inputs; explicit throughput semantics.
- scheduler_drain: DRY rule name/id prefix constants; use `debug_assert!` inside hot path; black_box the post-commit snapshot; added module docs and clarified BatchSize rationale.
- blake3 policy: keep exact patch `=1.8.2` and disable default features to avoid
  rayon/parallel hashing in benches.

> 2025-11-02 — PR-12: benches README

- Added `crates/rmg-benches/benches/README.md` documenting how to run and interpret
  benchmarks, report locations, and optional flamegraph usage.
- Linked it from the main `README.md`.

> 2025-11-02 — PR-12: benches polish and rollup refresh

- Pin `blake3` in benches to `=1.8.2` and disable defaults to satisfy cargo-deny
  wildcard bans while keeping benches single-threaded.
- snapshot_hash bench: precompute `link` type id and fix edge labels to `e-i-(i+1)`.
- scheduler_drain bench: return `Vec<NodeId>` from builder and avoid re-hashing node ids in the apply loop.

> 2025-11-02 — Benches DX: offline report + server fix

- Fix `Makefile` `bench-report` recipe to keep the background HTTP server alive using `nohup`; add `bench-status` and `bench-stop` helpers.
- Add offline path: `scripts/bench_bake.py` injects Criterion results into `docs/benchmarks/index.html` to produce `docs/benchmarks/report-inline.html` that works over `file://`.
- Update dashboard to prefer inline data when present (skips fetch). Update READMEs with `make bench-bake` instructions.
  - Improve `bench-report`: add `BENCH_PORT` var, kill stale server, wait-for-ready loop with curl before opening the browser; update `bench-serve/bench-open/bench-status` to honor `BENCH_PORT`.

> 2025-11-02 — PR-12: Sync with main + benches metadata

- Target: `echo/pr-12-snapshot-bench` (PR #113).
- Merged `origin/main` into the branch (merge commit, no rebase) to clear GitHub conflict status.
- Resolved `crates/rmg-benches/Cargo.toml` conflict by keeping:
  - `license = "Apache-2.0"` and `blake3 = { version = "=1.8.2", default-features = false, features = ["std"] }` in dev-dependencies.
  - Version-pinned path dep: `rmg-core = { version = "0.1.0", path = "../rmg-core" }`.
  - Bench entries: `motion_throughput`, `snapshot_hash`, `scheduler_drain`.
- Benches code present/updated: `crates/rmg-benches/benches/snapshot_hash.rs`, `crates/rmg-benches/benches/scheduler_drain.rs`.
- Scope: benches + metadata only; no runtime changes. Hooks (fmt, clippy, tests, rustdoc) were green locally before push.

> 2025-11-02 — PR-11 hotfix-deterministic-rollup-check

- Switch to `echo/hotfix-deterministic-rollup-check`, fetch and merge `origin/main` (merge commit; no rebase).
- Fix CI cargo-deny failures:
  - Add `license = "Apache-2.0"` to `crates/rmg-benches/Cargo.toml`.
  - Ensure no wildcard dependency remains in benches (use workspace path dep for `rmg-core`).
- Modernize `deny.toml` (remove deprecated `copyleft` and `unlicensed` keys per cargo-deny PR #611); enforcement still via explicit allowlist.

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

> 2025-12-01 — Docs rollup retired

- Cleaned SPDX checker skip list now that the rollup no longer exists.

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

> 2025-10-30 — PR-06: Motion negative tests (opened)

- Added tests in `rmg-core` covering NaN/Infinity propagation and invalid payload size returning `NoMatch`. Tests-only; documents expected behavior; no runtime changes.

> 2025-10-30 — PR-09: BLAKE3 header tests (tests-only)

- Added unit tests under `rmg-core` (in `snapshot.rs`) that:
  - Build canonical commit header bytes and assert `compute_commit_hash` equals `blake3(header)`.
  - Spot-check LE encoding (version u16 = 1, parents length as u64 LE).
- Assert that reversing parent order changes the hash. No runtime changes.

> 2025-10-30 — PR-10: README (macOS manual + local CI tips)

- Added a short CI Tips section to README covering how to trigger the manual macOS workflow and reproduce CI locally (fmt, clippy, tests, rustdoc, audit, deny).

> 2025-11-01 — PR-10 scope hygiene

- Removed commit‑header tests from `crates/rmg-core/src/snapshot.rs` on this branch to keep PR‑10 strictly docs/CI/tooling. Those tests live in PR‑09 (`echo/pr-09-blake3-header-tests`). No runtime changes here.


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
- [ ] Expose C ABI for host integrations and embed Rhai for scripting.
- [ ] Integrate Rhai runtime with deterministic sandboxing and host modules.
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
| 2025-10-25 | Language direction pivot: Echo core to Rust | TypeScript validated specs; long-term determinism enforced via Rust + C ABI + Rhai scripting | Update Phase 1 backlog: scaffold Rust workspace, port ECS/diff engine, FFI bindings |
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
> 2025-11-02 — Hotfix: deterministic rollup check (CI)

- Made CI rollup check robust against legacy non-deterministic headers by normalizing out lines starting with `Generated:` before comparing. Current generator emits a stable header, but this guards older branches and avoids false negatives.

> 2025-11-02 — Hotfix follow-up: tighter normalization + annotation

> 2025-11-02 — PR-11: benches crate skeleton (M1)

- Add `crates/rmg-benches` with Criterion harness and a minimal motion-throughput benchmark that exercises public `rmg-core` APIs.
- Scope: benches-only; no runtime changes. Document local run (`cargo bench -p rmg-benches`).
