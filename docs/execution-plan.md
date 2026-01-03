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

> 2026-01-03 — PR #213: merge `origin/main` + resolve review feedback (IN PROGRESS)

- Goal: land PR #213 cleanly by merging the latest `origin/main`, resolving conflicts deterministically, and addressing all review comments.
- Scope:
  - Merge `origin/main` into the PR branch (no rebase) and resolve conflicts.
  - Pull PR comments/review threads and implement requested fixes (docs + tooling as needed).
  - Verify `pnpm docs:build` is green before pushing.
- Exit criteria: conflicts resolved, review feedback addressed, and the updated branch pushed to the PR remote.
> 2026-01-02 — Dependency DAG sketches (issues + milestones) (IN PROGRESS)

- Goal: produce a durable “do X before Y” visual map across a subset of open GitHub Issues + Milestones so we can sequence work intentionally (especially around TT0/TT1/TT2/TT3 and S1 dependencies).
- Scope:
  - Add confidence-styled dependency graphs (DOT sources + rendered SVGs) under `docs/assets/dags/`.
  - Add a small explainer doc (`docs/dependency-dags.md`) that defines edge direction + confidence styling and links the rendered artifacts.
  - Add a repo generator (`scripts/generate-dependency-dags.js`) plus a config file (`docs/assets/dags/deps-config.json`) so the diagrams can be regenerated and extended deterministically.
  - Expose the generator via `cargo xtask` for a consistent repo tooling entrypoint.
  - Add a scheduled GitHub Action that refreshes the DAGs (PR only if outputs change).
  - Add `docs/workflows.md` and link it from README + AGENTS so contributors can discover the official entrypoints (`make`, `cargo xtask`, CI automation).
- Keep the diagrams explicitly “planning sketches” (not a replacement for GitHub Project state or native dependency edges).
- Exit criteria: both DAGs render locally via Graphviz (`dot -Tsvg …`) and the doc index links to `docs/dependency-dags.md`.

> 2026-01-03 — PR #178: TT0 TimeStreams + wormholes spec lock (IN PROGRESS)

- Goal: merge `origin/main` into `echo/time-streams-wormholes-166`, resolve review feedback, and push updates to unblock CodeRabbit re-review.
- Scope:
  - Merge `origin/main` into the branch (no rebase).
  - Address all actionable CodeRabbit review items (correctness + doc lint).
  - Ensure all “explicitly deferred” sections are linked to tracking issues.
- Exit criteria: actionable review list is empty and the branch pushes cleanly.

> 2026-01-02 — Docs audit: purge/merge/splurge pass (IN PROGRESS)

- Goal: audit Echo docs for staleness and overlap, then decide which docs should be purged, merged, or expanded (starting with `docs/math-validation-plan.md`).
- Scope:
  - Refresh `docs/math-validation-plan.md` to match the current deterministic math implementation and CI coverage.
  - Produce a short audit memo listing candidate docs to purge/merge/splurge with rationale.
  - Fix a concrete dead-link cluster by making the collision tour build-visible (`docs/public/collision-dpo-tour.html`) and adding a `docs/spec-geom-collision.md` stub.
  - Keep changes single-purpose: documentation only (no runtime changes).
- Exit criteria: audit memo committed + updated math validation plan; PR opened (tracked under issue #208).

> 2026-01-02 — Issue #214: strict Origin allowlist semantics (IN PROGRESS)

- Goal: keep `echo-session-ws-gateway`’s `--allow-origin` behavior strict and make that policy obvious to operators and contributors.
- Scope:
  - Document “strict allowlist” behavior in `crates/echo-session-ws-gateway/README.md` (missing `Origin` is rejected when `--allow-origin` is configured).
  - Add a unit test in `crates/echo-session-ws-gateway/src/main.rs` that locks the behavior (missing `Origin` rejected only when allowlist is present).
- Exit criteria: issue #214 is closed by a small PR; `cargo test -p echo-session-ws-gateway` is green.
- Tracking: GitHub issue #214.

> 2026-01-02 — Issue #215: CI Playwright dashboard smoke job (IN PROGRESS)

- Goal: add a GitHub Actions job that runs the Playwright Session Dashboard smoke test on PRs/pushes and publishes artifacts so embedded-tooling regressions can’t silently land.
- Scope:
  - Extend `.github/workflows/ci.yml` with a Playwright job that:
    - installs Node + pnpm (pinned to `package.json`),
    - installs Chromium via `playwright install`,
    - runs `pnpm exec playwright test e2e/session-dashboard.spec.ts`,
    - uploads `playwright-report/` and `test-results/` as artifacts (even on failure).
  - Keep `ECHO_CAPTURE_DASHBOARD_SCREENSHOT` disabled in CI (CI should not mutate tracked files); rely on Playwright attachments/artifacts for visual inspection.
- Exit criteria: PR is open + green (new CI job passes); artifacts are present on failure; `ci.yml` change is documented in `docs/decision-log.md`.
- Tracking: GitHub issue #215.

> 2026-01-02 — PR triage pipeline: start with PR #179 (IN PROGRESS)

- Goal: sync to PR #179 and validate the tooling for pulling PR review comments and extracting actionable issues (including human reviewer notes), so we can systematically close review feedback across the open PR queue and merge cleanly once approved.
- Scope:
  - Checkout PR #179’s branch locally.
  - Identify where the tool pulls PR comments from (GitHub API / `gh` CLI / local refs) and what comment types it includes (issue comments, review comments, review summaries).
  - Ensure the report is attributable (comment author is included) so non-CodeRabbit actionables are not lost.
  - Ensure “✅ Addressed in commit …” ack markers cannot be spoofed by templated bot text (require a human-authored ack with a real PR commit SHA).
  - Run the tool against at least one PR to confirm output format and any required auth/config.
- Exit criteria: documented “how to run” steps for the tool; confidence that we can repeatably extract issues from PR comments for subsequent PRs.

> 2026-01-02 — Issue #177: deterministic trig audit oracle + pinned error budgets (IN PROGRESS)

- Goal: un-ignore the trig “error budget” test by replacing its platform-libm reference with a deterministic oracle, then pin explicit accuracy thresholds so CI can catch regressions in the LUT-backed trig backend.
- Scope:
  - Use a pure-Rust oracle (`libm`) so the reference is not host libc/libm dependent.
  - Measure both:
    - absolute error vs the f64 oracle (robust near zero),
    - ULP distance vs the f32-rounded oracle (applied only when |ref| ≥ 0.25 so ULPs remain meaningful).
  - Remove the repo-root scratchpad `freaky_numbers.rs` if it is not used by any crate/tests.
- Exit criteria: `cargo test -p warp-core --test deterministic_sin_cos_tests` is green with the audit test enabled by default; budgets are documented in `docs/decision-log.md`.
- Tracking: GitHub issue #177.

> 2026-01-01 — Issue #180: Paper VI notes + capability matrix (IN PROGRESS)

- Goal: turn “Pulse” time/determinism/tooling insights into durable artifacts (Paper VI notes + a crisp ownership matrix for Echo).
- Scope:
  - Add `docs/capability-ownership-matrix.md` (template + first pass).
  - Extend Paper VI notes in `aion-paper-06` (HostTime/HistoryTime, decision records, multi-clock streams, replay integrity hooks).
- Exit criteria: matrix + notes are concrete enough to guide near-term implementation choices and future tool UX.
- Tracking: GitHub issue #180.

> 2026-01-01 — PR hygiene: standardize CodeRabbitAI review triage (COMPLETED)

- Goal: make CodeRabbitAI review loops cheap and unambiguous by codifying how we extract actionable comments from the current PR head diff.
- Scope:
  - Add mandatory procedures under `docs/procedures/` for PR submission and review comment extraction.
  - Add a helper script `.github/scripts/extract-actionable-comments.sh` to automate review comment bucketing and produce a Markdown report.
- Exit criteria: a contributor can run one command and get a clean actionables list without re-reading the entire PR history.

> 2026-01-01 — T2 (#168): embedded session dashboard baseline (COMPLETED)

- Goal: keep the “run a binary, open a page” dashboard workflow stable while standardizing styling and keeping docs screenshots honest.
- Scope:
  - Serve a static dashboard from `echo-session-ws-gateway` (`/dashboard`) plus `/api/metrics`.
  - Vendor Open Props CSS into the gateway and serve it under `/vendor/*.css` for offline use.
  - Add Playwright smoke tests that exercise the dashboard and optionally regenerate the screenshot used in `docs/guide/wvp-demo.md`.
- Exit criteria: `cargo clippy -p echo-session-ws-gateway --all-targets -- -D warnings` green; `pnpm exec playwright test` green; updated screenshot checked in.
- Evidence:
  - PR #176 (session dashboard + Playwright smoke + screenshot regen)
  - Dashboard: `crates/echo-session-ws-gateway/assets/dashboard.html`
  - Routes: `crates/echo-session-ws-gateway/src/main.rs`
  - e2e: `e2e/session-dashboard.spec.ts`
  - Docs screenshot: `docs/assets/wvp/session-dashboard.png`

> 2026-01-01 — T2 (#168): make dashboard smoke tests self-contained (COMPLETED)

- Goal: ensure the Playwright “Session Dashboard” smoke test can build and run all required binaries from a clean checkout.
- Scope:
  - Add a tiny `echo-session-client` example (`publish_pulse`) used by the e2e test to generate deterministic, gapless snapshot+diff traffic.
- Exit criteria: `pnpm exec playwright test` no longer depends on local stashes / untracked artifacts.
- Evidence:
  - `pnpm exec playwright test e2e/session-dashboard.spec.ts` (green)

> 2026-01-01 — Issue #169: harden WVP demo with loopback tests (COMPLETED)

- Goal: prevent WVP demo regressions by pinning protocol invariants (snapshot-first, gapless epochs, authority enforcement) in automated tests.
- Scope:
  - Add a `UnixStream::pair()` loopback test for `echo-session-service` that exercises handshake, subscribe, and publish error cases without binding a real UDS path.
  - Add `warp-viewer` unit tests for publish gating and publish-state transitions (snapshot-first, epoch monotonicity, pending ops clearing).
- Exit criteria: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green; tests document the demo invariants.
- Tracking: GitHub issue #169.
- Evidence:
  - PR #175 (loopback tests + publish behavior pinned; follow-up hardening for defensive test checks)

> 2026-01-01 — PR #167: deterministic math follow-ups + merge `main` (COMPLETED)

- Goal: address all CodeRabbit review comments on PR #167 with minimal churn, keep the PR tightly scoped to deterministic math + warp-core motion payload work, and restore mergeability by merging `main` and resolving docs guard conflicts.
- Scope:
  - Resolve merge conflicts from `origin/main` in `docs/decision-log.md` and `docs/execution-plan.md` while preserving both the WVP hardening timeline and the deterministic math timeline.
  - Keep deterministic trig guardrails stable (`scripts/check_no_raw_trig.sh`) so raw platform trig calls cannot sneak back into runtime math code.
- Exit criteria: `cargo test -p warp-core` and `cargo test -p warp-core --features det_fixed` are green; `cargo clippy -p warp-core --all-targets -- -D warnings -D missing_docs` is green; PR is mergeable and CI stays green.
- Evidence: merged to `main` as PR #167 (merge commit `54d7626`; closes #165).

> 2026-01-01 — Motion payload v2 (Q32.32) + `Scalar` port (COMPLETED)

- Goal: move the motion demo payload to a deterministic Q32.32 fixed-point encoding (v2) while preserving decode compatibility for the legacy v0 `f32` payload; port the motion executor to use the `Scalar` abstraction and upgrade v0 payloads to v2 on write.
- Evidence: `cargo test -p warp-core` green; `cargo test -p warp-core --features det_fixed` green; `cargo clippy -p warp-core --all-targets -- -D warnings -D missing_docs` green; `cargo clippy -p warp-core --all-targets --features det_fixed -- -D warnings -D missing_docs` green.

> 2026-01-01 — Deterministic fixed-point lane (`DFix64`) + CI coverage (COMPLETED)

- Goal: land a deterministic fixed-point scalar backend (`DFix64`, Q32.32) behind a `det_fixed` feature flag, add a dedicated test suite, and extend CI with explicit `--features det_fixed` lanes (including MUSL) so we continuously exercise cross-platform behavior.
- Evidence: commit `57d2ec3` plus the above motion work continues to validate the det_fixed lane in CI.

> 2026-01-01 — Implement deterministic `F32Scalar` trig (COMPLETED)

- Goal: replace `F32Scalar::{sin,cos,sin_cos}`’s platform transcendentals with a deterministic LUT-backed backend, check in the LUT, and promote the existing trig test scaffold into a cross-platform golden-vector suite.
- Evidence: `cargo test -p warp-core` green; `cargo test -p warp-core --test deterministic_sin_cos_tests` green (error-budget audit test remains `#[ignore]`); `cargo clippy -p warp-core --all-targets -- -D warnings -D missing_docs` green.

> 2025-12-30 — Branch maintenance: resurrect `F32Scalar/sin-cos` (COMPLETED)

- Goal: merge `main` into the legacy deterministic trig test branch, resolve the `rmg-core`→`warp-core` rename conflict, and leave the WIP test compiling (ignored by default).
- Evidence: merge commit `6cfa64d` (“Merge branch 'main' into F32Scalar/sin-cos”); `cargo test -p warp-core --test deterministic_sin_cos_tests` passes (ignored test remains opt-in).

> 2025-12-30 — Issue #163: WVP demo path (IN PROGRESS)

- Goal: complete the WARP View Protocol demo path (publisher + subscriber) by adding outbound publish support to `echo-session-client` and wiring publish/subscribe toggles + a dirty publish loop in `warp-viewer`.
- Scope:
  - `echo-session-client`: bidirectional tool connection (receive + publish `warp_stream`).
  - `warp-viewer`: publish/subscribe toggles, deterministic local mutation to generate diffs, gapless epoch publish, surface hub errors as notifications/toasts.
  - Review follow-ups (PR #164): resolve CodeRabbit/Codex actionables (rustdoc coverage, reconnect snapshot reset, and no-silent-encode-failure logging).
  - Docs: update `docs/tasks.md` and add a short “two viewers + hub” demo walkthrough.
- Exit criteria: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green; demo is reproducible locally; PR opened.
- Tracking: GitHub issue #163.

> 2025-12-30 — PR #162: Address CodeRabbit doc nits (COMPLETED)

- Goal: close out CodeRabbit review comments on the THEORY doc with minimal churn.
- Scope:
  - Tighten small prose redundancies in `docs/THEORY.md` (wordiness/repetition).
  - Double-check markdownlint-sensitive formatting (no trailing whitespace; headings surrounded by blank lines).
- Exit criteria: CodeRabbit review comments are resolved; hooks remain green.
- Evidence:
  - Commit: `22ba855` (THEORY prose tightening + session logging)

> 2025-12-30 — THEORY.md paper alignment callouts (COMPLETED)

- Goal: annotate the local THEORY paraphrase so readers can see where Echo intentionally diverges from the published papers.
- Scope:
  - Add `[!note]` callouts explaining “Echo does this differently … because …” where implementation differs.
  - Commit `docs/THEORY.md` + the `assets/echo-white-radial.svg` brand asset; remove the local-only `assets/readme/` image scratchpad.
- Exit criteria: THEORY doc is self-aware (differences + rationale); files pass hooks (SPDX guard, fmt/clippy/tests).
- Evidence:
  - Commit: `c029c82` (`docs/THEORY.md` + `assets/echo-white-radial.svg`)

> 2025-12-30 — Post-merge housekeeping: sync roadmap/checklists (COMPLETED)

- Goal: keep the living roadmap accurate now that WARP Stage B1 landed and merged.
- Scope:
  - Mark completed items in the “Immediate Backlog” that are now delivered in `main`.
  - Remove/replace references to non-existent paths (e.g. `packages/…`) with current Rust workspace paths.
  - Update `docs/docs-index.md` “Getting Started” so it points at the current WARP entry path (`docs/guide/warp-primer.md`).
- Exit criteria: roadmap reflects current repo structure; no “(IN PROGRESS)” entries for merged work.
- Evidence:
  - Commit: `a0e908a` (roadmap housekeeping)

> 2025-12-30 — PR #159: Address CodeRabbit actionables (COMPLETED)

- Goal: close the remaining review items on PR #159 (docs + demo hygiene).
- Scope:
  - Add Evidence blocks (commit SHAs) for completed 2025-12-30 entries below.
  - Clarify “no hidden edges” enforcement references in `docs/guide/warp-primer.md`.
  - Clarify Paper I notation context (`U`, `π(U)`) in `docs/spec-warp-core.md`.
  - Minor demo hygiene: document init/update behavior in the port demo executor.
- Exit criteria: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green; CodeRabbit re-review clean.
- Evidence:
  - Merge: `f2d6a68` (PR #159 merged to `main`)

> 2025-12-30 — Add WARP primer + wire into “Start here” (COMPLETED)

- Goal: make WARP approachable to newcomers and pin canonical “start here” docs order.
- Scope:
  - Add `docs/guide/warp-primer.md`.
  - Link it as step 1 from README + `docs/spec-warp-core.md`.
  - Keep formatting markdownlint-friendly (esp. MD022 heading spacing).
- Exit criteria: docs read cleanly; CodeRabbit markdown nits avoided.
- Evidence:
  - Commit: `1b40f66` (docs primer + links)
  - Docs: `docs/guide/warp-primer.md`, `docs/spec-warp-core.md`
  - README: `README.md` (Start here list)

> 2025-12-30 — PR #159: Address CodeRabbit majors (warp-core tour examples) (COMPLETED)

- Goal: close remaining “Major” review gaps by making Stage B1 behavior concrete and easy to adopt.
- Scope:
  - Expand `docs/spec-warp-core.md` with a worked Stage B1 example showing:
    - portal authoring (`OpenPortal`) + `Descend(WarpId)` semantics,
    - `Engine::apply_in_warp` usage with a `descent_stack`,
    - how descent-chain reads become `Footprint.a_read` and therefore `in_slots`,
    - Paper III worldline slicing that includes the portal chain.
  - Add a minimal `Engine` constructor for initializing from an existing `WarpState` (needed to make examples concrete without exposing internals).
  - Close remaining review gaps on portal invariants by enforcing and testing:
    - no dangling `Descend(WarpId)` portals without a corresponding `WarpInstance`,
    - no orphan instances (`WarpInstance.parent` must be realized by an attachment slot).
- Exit criteria: `cargo fmt --all`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green.
- Evidence:
  - Commits: `875690c`, `7a02123`, `846723d`, `27e490a`
  - Docs: `docs/spec-warp-core.md` (Stage B1 quickstart + worked examples)
  - Implementation: `crates/warp-core/src/engine_impl.rs`, `crates/warp-core/src/tick_patch.rs`
  - Tests: portal invariant tests in `crates/warp-core/src/tick_patch.rs`

> 2025-12-30 — Touch-ups (receipts/docs + wasm/ffi ergonomics + graph delete perf) (COMPLETED)

- Goal: address follow-up review nits and a concrete graph-store performance trap without changing deterministic semantics.
- Scope:
  - Clarify `TickReceiptEntry.scope` semantics (it is a `NodeKey`, not a `NodeId`) and keep receipt digest encoding stable.
  - Make the WASM boundary less opaque: replace `ok()??` with explicit matches; log spawn failures behind `console-panic` and return a clear sentinel (`Uint8Array` length 0).
  - Add `Engine::insert_node_with_attachment` for atomic bootstrapping of demo nodes (avoids partial init if an invariant is violated).
  - Eliminate `delete_node_cascade`’s `O(total_edges)` inbound scan by maintaining a reverse inbound index (`edges_to`) plus `EdgeId -> to` index.
  - Clarify `docs/spec-warp-tick-patch.md` that the section 1.2 op list is semantic and does not imply encoding tag order.
- Exit criteria: `cargo fmt --all`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green.
- Evidence:
  - Commits: `8282a29`, `ef89bc7`, `a070374`, `31e0836`, `889932d`
  - Implementation: `crates/warp-core/src/receipt.rs`, `crates/warp-wasm/src/lib.rs`, `crates/warp-ffi/src/lib.rs`, `crates/warp-core/src/graph.rs`
  - Docs: `docs/spec-warp-tick-patch.md`

> 2025-12-30 — Touch-ups: encapsulation + ergonomics + micro-perf (COMPLETED)

- Goal: address review touch-ups to keep public APIs stable, avoid internal field coupling, and remove sharp edges in docs/tests/benchmarks.
- Scope:
  - Clarify attachment-plane terminology in `docs/warp-two-plane-law.md`.
  - Improve worldline slice work-queue allocation (`Vec::with_capacity`) for large patch sets.
  - Add small encapsulation helpers (`GraphStore::has_edge`, `WarpId::as_bytes`, `NodeId::as_bytes`) and update callers to avoid `.0` tuple-field indexing.
  - Tighten `Engine` accessors: distinguish “unknown warp store” vs “missing node/attachment” via `Result<Option<...>, EngineError>`.
  - Make root-store mutation methods (`insert_node`, `set_node_attachment`) return `Result` and surface invariant violation (`UnknownWarp`) rather than silently dropping writes.
  - Improve diagnostics in motion benchmarks’ panic paths.
- Exit criteria: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green; docs guard updated.
- Evidence:
  - Commits: `31e0836`, `889932d`
  - Implementation: `crates/warp-core/src/engine_impl.rs`, `crates/warp-core/src/graph.rs`, `crates/warp-core/src/tick_patch.rs`, `crates/warp-core/src/ident.rs`
  - Benches: `crates/warp-benches/benches/motion_throughput.rs`

> 2025-12-30 — Stage B1.1: Atomic portals + merge/DAG slicing semantics (COMPLETED)

- Goal: make descended attachments “slice-safe” by introducing an atomic portal authoring op (`OpenPortal`), then lock down merge semantics and terminology to prevent long-term drift.
- Scope:
  - Add `WarpOp::OpenPortal { key, child_warp, child_root, init }` as the canonical portal authoring operation.
  - Update patch replay validation to forbid dangling portals / orphan instances.
  - Update `diff_state` to emit `OpenPortal` for new descended instances, preventing portal/instance creation from being separable across ticks.
  - Document merge semantics (explicit conflict resolution) and DAG slicing algorithm.
  - Add a terminology law doc to pin “instance zoom vs wormholes”.
- Exit criteria: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green; docs guard updated.
- Evidence:
  - Implementation:
    - `crates/warp-core/src/tick_patch.rs` (`WarpOp::OpenPortal`, replay validation, diff_state portal canonicalization)
  - Docs:
    - `docs/adr/ADR-0002-warp-instances-descended-attachments.md` (atomic portals + merge law)
    - `docs/spec/SPEC-0002-descended-attachments-v1.md` (OpenPortal + merge/DAG slicing + zoom tooling note)
    - `docs/spec-warp-tick-patch.md` (OpenPortal op encoding)
    - `docs/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md` (terminology law)

> 2025-12-30 — Stage B1: WarpInstances + descended attachments (COMPLETED)

- Goal: implement “WARPs all the way down” without recursive traversal in the rewrite hot path by modeling descent as flattened indirection (WarpInstances).
- Scope:
  - Introduce WarpInstances:
    - `WarpId`
    - `WarpInstance { warp_id, root_node, parent: Option<AttachmentKey> }`
  - Make ids instance-scoped:
    - `NodeKey { warp_id, local_id }`
    - `EdgeKey { warp_id, local_id }`
  - Upgrade attachments from depth-0 atoms to `AttachmentValue = Atom(AtomPayload) | Descend(WarpId)` and make attachment slots first-class:
    - `AttachmentKey { owner: NodeKey|EdgeKey, plane: Alpha|Beta }`
    - `SlotId::Attachment(AttachmentKey)` (tick patches + slicing)
  - Enforce the Paper I/II “no hidden edges” and descent-chain correctness laws:
    - Matching/indexing stays skeleton-only within an instance.
    - Any match/exec within a descended instance must READ every `AttachmentKey` in the descent stack (so changing a descent pointer deterministically invalidates the match).
  - Slicing integration: a demanded value in instance `W` must pull in the attachment chain (root → W) producers via `SetAttachment(...Descend...)` ops, with no decoding of atoms.
- Exit criteria: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green; new ADR + SPEC for Stage B1; decision log updated.
- Evidence:
  - Implementation:
    - `crates/warp-core/src/warp_state.rs` (WarpState/WarpInstance)
    - `crates/warp-core/src/attachment.rs` (AttachmentKey/Value incl. `Descend`)
    - `crates/warp-core/src/snapshot.rs` (state_root reachability across instances via `Descend`)
    - `crates/warp-core/src/engine_impl.rs` (apply_in_warp + descent_chain reads)
    - `crates/warp-core/src/footprint.rs` + `crates/warp-core/src/scheduler.rs` (attachment conflicts)
    - `crates/warp-core/src/tick_patch.rs` (SlotId::Attachment, instance ops, patch_digest v2)
  - Docs:
    - `docs/adr/ADR-0002-warp-instances-descended-attachments.md`
    - `docs/spec/SPEC-0002-descended-attachments-v1.md`
    - `docs/spec-warp-tick-patch.md` (updated to v2 encoding)
    - `docs/spec-merkle-commit.md` (updated state_root encoding)
    - `docs/warp-two-plane-law.md` (updated to reflect B1 reality)
  - Tests:
    - `crates/warp-core/src/tick_patch.rs` (portal-chain slice test)
    - `crates/warp-core/src/scheduler.rs` (descent-chain conflict test)

> 2025-12-29 — WARP two-plane semantics: typed atom attachments (COMPLETED)

- Goal: align Echo’s `warp-core` implementation with Paper I/II “two-plane” semantics without slowing the rewrite hot path.
- Scope:
  - Treat `GraphStore` as the **SkeletonGraph** (π(U)): the structure used for matching, rewriting, scheduling, slicing, and hashing.
  - Model attachment-plane payloads as **typed atoms** (depth-0): `AtomPayload { type_id: TypeId, bytes: Bytes }`.
  - Update snapshot hashing + tick patch canonical encoding so payload `type_id` participates in digests (no “same bytes, different meaning” collisions).
  - Introduce a minimal codec boundary (`Codec<T>` + registry concept) for typed decode/encode at rule/view boundaries; core matching/indexing remains skeleton-only unless a rule explicitly decodes.
  - Document the project laws (“no hidden edges in payload bytes”, “skeleton rewrites never decode attachments”) and record the decision in ADR + SPEC form.
- Exit criteria: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green; docs guard updated (`docs/decision-log.md` + new ADR/SPEC + law doc).
- Evidence:
  - Implementation: `crates/warp-core/src/attachment.rs`, `crates/warp-core/src/record.rs`, `crates/warp-core/src/snapshot.rs`, `crates/warp-core/src/tick_patch.rs`.
  - Docs: `docs/warp-two-plane-law.md`, `docs/adr/ADR-0001-warp-two-plane-skeleton-and-attachments.md`, `docs/spec/SPEC-0001-attachment-plane-v0-atoms.md`.
  - Tests: new digest/type-id assertions in `crates/warp-core/tests/atom_payload_digest_tests.rs`; workspace tests + clippy rerun green.

> 2025-12-29 — Follow-up: tick patch hygiene (COMPLETED)

- Goal: clean up `tick_patch` sharp edges so the patch boundary stays deterministic, well-documented, and resistant to misuse.
- Scope:
  - `diff_store`: avoid double map lookups and expand rustdoc (intent, invariants, semantics, edge cases, perf).
  - `TickPatchError`: switch to `thiserror` derive (remove boilerplate).
  - `encode_ops`: document that digest tag bytes are distinct from replay sort ordering.
  - `WarpTickPatchV1::new`: dedupe duplicate ops by sort key to avoid replay errors.
  - `Hash` naming: alias `crate::ident::Hash` to `ContentHash` to avoid confusion with `derive(Hash)`.
- Exit criteria: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green.
- Evidence: Issue `#156` / PR `#157` (commits `793322f`, `615b9e9`, `5e1e502`) — `crates/warp-core/src/tick_patch.rs` implements the changes; tests/clippy rerun green.

> 2025-12-29 — Follow-up: `EdgeRecord` equality (COMPLETED)

- Goal: remove the ad-hoc `edge_record_eq` helper so `EdgeRecord` equality is defined by the type, not by duplicated helper logic in `tick_patch`.
- Scope: derive `PartialEq` + `Eq` on `EdgeRecord`; replace `edge_record_eq(a, b)` call sites with idiomatic `a == b`; delete the helper from `tick_patch.rs`.
- Exit criteria: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green.
- Evidence: `EdgeRecord` derives `PartialEq, Eq` and `tick_patch` uses `==` directly; helper removed; tests/clippy rerun green.

> 2025-12-29 — Follow-ups: policy_id plumbing + edge replay index (COMPLETED)

- Goal: eliminate “TODO-vibes” follow-ups by making policy id handling explicit/configurable and by removing the O(total_edges) edge scan from tick patch replay.
- Scope: thread `policy_id` through `Engine` as an explicit engine parameter (defaulting to `POLICY_ID_NO_POLICY_V0`); add a `GraphStore` reverse index (`EdgeId -> from`) to support O(bucket) edge migration/removal during patch replay; keep commit hash semantics unchanged (still commits to `state_root` + `patch_digest` + policy id).
- Exit criteria: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` green; no remaining local-only scope hash clones; no edge replay full-scan.
- Evidence: `Engine::{with_policy_id, with_scheduler_and_policy_id}` added; tick patch replay now uses `GraphStore` reverse index; strict clippy/tests gates rerun green.

> 2025-12-29 — Tick receipts: blocking causality (COMPLETED)

- Goal: finish the Paper II tick receipt slice by recording *blocking causality* for rejected candidates (a poset edge list) in `warp-core`.
- Scope: extend `TickReceipt` to expose the applied candidates that blocked a `Rejected(FootprintConflict)` entry; keep `decision_digest` stable (digest commits only to accept/reject outcomes, not blocker metadata).
- Exit criteria: new tests cover multi-blocker cases; `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` are green; bridge docs updated to match implementation; decision log entry recorded.
- Evidence: implemented `TickReceipt::blocked_by` and blocker attribution in `Engine::commit_with_receipt`; added multi-blocker tests; updated `docs/aion-papers-bridge.md` and `docs/spec-merkle-commit.md`; validated via `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs`.

> 2025-12-29 — Delta tick patches + commit hash v2 (COMPLETED)

- Goal: implement Paper III-aligned **delta tick patches** (`WarpTickPatchV1`) in `warp-core` and switch commit hashing to **v2** so `commit_id` commits only to the replayable delta (`patch_digest`).
- Scope: define canonical patch ops + slot sets (`in_slots`/`out_slots` as *unversioned* slots); compute `patch_digest` from canonical patch encoding; update `Snapshot` and commit hashing so v2 commits to `(parents, state_root, patch_digest, policy_id, version)` and treats planner/scheduler digests + receipts as diagnostics only.
- Exit criteria: new spec doc for `WarpTickPatchV1` + commit hash v2; tests cover deterministic patch generation + patch replay; `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` are green; decision log entry recorded.
- Evidence: added `docs/spec-warp-tick-patch.md`; upgraded `docs/spec-merkle-commit.md` to v2; implemented `crates/warp-core/src/tick_patch.rs` + wired patch generation into `Engine::commit_with_receipt`; added patch replay test (`Engine` unit test) + updated receipt tests; validated via `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs`.

> 2025-12-28 — Promote AIΩN bridge doc + add tick receipts (COMPLETED)

- Goal: promote the AIΩN Foundations ↔ Echo bridge from a dated note into a canonical doc, then implement Paper II “tick receipts” in `warp-core`.
- Scope: move the bridge into `docs/` (keep a stub for historical links); add `TickReceipt` + `Engine::commit_with_receipt`; commit receipt outcomes via `decision_digest`; update `docs/spec-merkle-commit.md` to define the receipt digest encoding.
- Exit criteria: `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` are green; the bridge is indexed in `docs/docs-index.md`; the decision log records rationale.
- Evidence: added `docs/aion-papers-bridge.md` and indexed it; implemented `crates/warp-core/src/receipt.rs` + `Engine::commit_with_receipt`; updated `docs/spec-merkle-commit.md`; validated via `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs`.

> 2025-12-28 — WARP rename + AIΩN docs sanity pass (COMPLETED)

- Goal: confirm the WARP-first terminology sweep and AIΩN linkage are consistent and build-clean after the rename.
- Scope: rerun `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs`; verify README links and ensure the only remaining `rmg_*` mentions are explicit compatibility/historical references.
- Exit criteria: tests + clippy green; no stray “RMG / recursive metagraph” naming remains in code/docs beyond the bridge note and wire-compat aliases.
- Evidence: workspace tests + clippy rerun green; wire decoder keeps a short transition window for `subscribe_rmg`/`rmg_stream` and `rmg_id` aliases (documented in `docs/spec-warp-view-protocol.md`).

> 2025-12-28 — Repo survey + briefing capture (COMPLETED)

- Goal: refresh a full-stack mental model of Echo as it exists *today* (Rust workspace + session tooling) and capture a durable briefing for future work.
- Scope: read architecture + determinism specs; map crate boundaries; trace the rewrite/commit hashing pipeline and the session wire/protocol pipeline; review AIΩN/WARP paper sources and map them onto the current repo; note any doc/code drift.
- Exit criteria: publish a concise repo map + invariants note in `docs/notes/` and repair any obvious spec drift discovered during the survey.
- Evidence: added `docs/notes/project-tour-2025-12-28.md` and `docs/notes/aion-papers-bridge.md`; corrected the canonical empty digest semantics in `docs/spec-merkle-commit.md` to match `warp-core`; refreshed `README.md` to link the AIΩN Framework repo + Foundations series.

> 2025-12-28 — RMG → WARP rename sweep (COMPLETED)

- Goal: eliminate the legacy “RMG / recursive metagraph” naming drift by renaming the workspace to WARP-first terminology.
- Scope: rename crates (`rmg-*` → `warp-*`), update Rust identifiers (`Rmg*` → `Warp*`), align the session proto/service/client/viewer, and sweep docs/specs/book text to match.
- Exit criteria: `cargo test --workspace` passes; the only remaining `rmg_*` strings are explicit compatibility aliases and historical bridge-note references.
- Evidence: workspace builds and tests pass under WARP naming; the wire decoder accepts legacy `subscribe_rmg` / `rmg_stream` and `rmg_id` as transition aliases; docs/specs/book now describe WARP streams and WarpIds.

> 2025-12-28 — PR #141 follow-up (new CodeRabbit nits @ `4469b9e`) (COMPLETED)

- Goal: address the two newly posted CodeRabbit nitpicks on PR #141 (latest review on commit `4469b9e`).
- Scope: bucket new actionable threads; implement fixes (incl. rust-version guard + workspace package metadata inheritance) with tests where applicable; update burn-down index + decision log; reply on each thread with fix SHAs; land PR.
- Exit criteria: pre-push hooks green; `gh pr checks 141` green; explicit PR comment posted mapping issue IDs → fixing SHAs; PR merged.
- Evidence: rust-version guard now supports `rust-version.workspace = true` + avoids `sed` (`7e84b16`); Spec-000 rewrite inherits shared package metadata from `[workspace.package]` (`e4e5c19`).

> 2025-12-28 — PR #141 follow-up (new CodeRabbit review @ `639235b`) (COMPLETED)

- Goal: address newly posted CodeRabbit review comments on PR #141 (latest review on commit `639235b`), including any high-priority blockers.
- Scope: re-extract paginated PR comments; bucket new actionable threads; implement fixes with tests + doc alignment; update burn-down index + consolidated PR comment with fix SHAs.
- Exit criteria: `cargo test --workspace` + `cargo clippy --all-targets -- -D warnings -D missing_docs` green; PR checks green; explicit PR comment posted mapping issue IDs → fixing SHAs.
- Evidence: restore `"wasm"` categories in `84e63d3`; Spec-000 docs fixes in `922553f`; workspace dep cleanup in `dfa938a`; CI + rust-version guard hardening in `56a37f8`.

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

> 2025-12-11 — WARP authority enforcement (IMPLEMENTED; PENDING MERGE)

- Goal: Reject non-owner publishes on WARP channels and surface explicit errors to clients.
- Scope: `echo-session-service` (producer lock + error frames), `echo-session-client` (map error frames to notifications), protocol tasks checklist.
- Status: implemented on branch `echo/warp-view-protocol-spec` (commit `237460e`); not yet merged to `main`.

> 2025-12-10 — CI cargo-deny index failures (COMPLETED)

- Goal: stop noisy `warning[index-failure]: unable to check for yanked crates` in GitHub Actions by ensuring `cargo-deny` has a warm crates.io index.
- Scope: `.github/workflows/ci.yml` deny job (prime cargo index before running `cargo deny`).
- Status: completed; deny job now runs `cargo fetch --locked` before `cargo deny`.

> 2025-12-10 — CI cargo-audit unmaintained warnings (COMPLETED)

- Goal: keep `cargo audit --deny warnings` green despite unavoidable unmaintained transitive `paste` (via wgpu) and legacy `serde_cbor` advisory.
- Scope: `.github/workflows/security-audit.yml` and `.github/workflows/ci.yml` (add `--ignore RUSTSEC-2024-0436` and `--ignore RUSTSEC-2021-0127`).
- Status: completed; audit steps now ignore these advisories explicitly until upstreams replace them.

> 2025-12-10 — WARP View Protocol tasks (IN PROGRESS)

- Goal: land the WARP View Protocol/EIP checklist and execute slices toward multi-viewer sharing demo.
- Scope: tracked in `docs/tasks.md` with stepwise commits as items complete.
- Status: checklist drafted.

> 2025-12-10 — CBOR migration + viewer input gating (COMPLETED)

- Goal: swap serde_cbor for maintained ciborium, harden canonical encoding/decoding, and keep viewer input/render stacks consistent.
- Scope: `crates/echo-session-proto` (ciborium + serde_value bridge, canonical encoder/decoder), `crates/echo-graph` (ciborium canonical bytes + non_exhaustive enums), `crates/warp-viewer` (egui patch alignment, input/app events/session_logic gating, hash mismatch desync), dependency lockfile.
- Status: completed; wire encoding now uses ciborium with checked integer handling and canonical ordering, graph hashing returns Result, viewer controls are gated to View screen with safer event handling and consistent egui versions.

> 2025-12-10 — Session client framing & non-blocking polling (COMPLETED)

- Goal: make session client polling non-blocking, bounded, and checksum-aligned.
- Scope: `crates/echo-session-client/src/lib.rs` (buffered try_read polling, MAX_PAYLOAD guard, checksum-respecting frame sizing, notification drain, tests).
- Status: completed; poll_message is now non-blocking, enforces an 8 MiB cap with checked arithmetic, preserves buffered partials, and poll_notifications drains buffered notifications only.

> 2025-12-10 — Viewer timing & viewport safety (COMPLETED)

- Goal: stabilize per-frame timing and prevent viewport unwrap panics.
- Scope: `crates/warp-viewer/src/app_frame.rs` (dt reuse, angular velocity with dt, safe viewport access, single aspect computation, window lifetime).
- Status: completed; dt is captured once per frame, spins/decay use that dt, viewport access is guarded, and helper signatures no longer require 'static windows.

> 2025-12-10 — Config + docs alignment (COMPLETED)

- Goal: keep docs aligned with code and maintained deps.
- Scope: `crates/echo-config-fs/README.md` (ConfigStore naming, doc path), `crates/echo-session-proto/src/lib.rs` (explicit reexports, AckStatus casing), `docs/book/echo/sections/06-editor-constellation.tex` + TikZ legend/label tweaks.
- Status: completed; README references correct traits/paths, proto surface is explicit with serde renames, figure labeled/cross-referenced with anchored legend.

> 2025-12-06 — Tool crate docs + crate map (COMPLETED)

- Goal: tighten docs around the tool hexagon pattern and make crate-level READMEs point at the Echo booklets as the canonical source of truth.
- Scope: `docs/book/echo/sections/09-tool-hex-pattern.tex` (crate map), READMEs and `Cargo.toml` `readme` fields for `echo-app-core`, `echo-config-fs`, `echo-session-proto`, `echo-session-service`, `echo-session-client`, and `warp-viewer`.
- Status: completed; Tools booklet now includes a crate map, and each tool-related crate README has a “What this crate does” + “Documentation” section pointing back to the relevant booklets/ADR/ARCH specs.

> 2025-12-06 — JS-ABI + WARP streaming docs alignment (COMPLETED)

- Goal: Align Echo’s book-level docs with the JS-ABI v1.0 deterministic encoding + framing decisions (ADR-0013 / ARCH-0013) and the new WARP streaming stack.
- Scope: `docs/book/echo/sections/{13-networking-wire-protocol,14-warp-stream-consumers,07-session-service,08-warp-viewer-spec}.tex` (cross-links, diagrams, tables).
- Status: completed; Core booklet now documents JS-ABI framing + generic WARP consumer contract (with role summary), and Tools booklet’s Session Service + WARP Viewer sections cross-reference that contract instead of re-specifying it.

> 2025-12-04 — Sync roadmap with session streaming progress (COMPLETED)

- Goal: capture the new canonical `echo-graph` crate + gapless WARP streaming path, and queue remaining engine/viewer wiring tasks.
- Scope: update `crates/warp-viewer/ROADMAP.md`, note outstanding engine emitter + client extraction; log decisions.
- Status: completed.

> 2025-12-03 — Recover warp-viewer ROADMAP after VSCode crash

- Goal: confirm whether roadmap edits were lost and restore the latest saved state.
- Scope: `crates/warp-viewer/ROADMAP.md` sanity check vs git.
- Status: completed; file matches last commit (no recovery needed).

> 2025-12-03 — Persist warp-viewer camera + HUD settings between runs (COMPLETED)

- Goal: write config load/save so camera + HUD toggles survive restarts.
- Scope: `crates/warp-viewer/src/main.rs`, add serde/directories deps; update roadmap/docs.
- Status: completed; config saved to OS config dir `warp-viewer.json`, loads on startup, saves on close.

> 2025-12-03 — Extract core app services and refactor viewer (COMPLETED)

- Goal: stop config/toast creep in warp-viewer; introduce shared core + fs adapter; make viewer consume injected prefs.
- Scope: new crates `echo-app-core` (ConfigService/ToastService/ViewerPrefs) and `echo-config-fs`; rewire `warp-viewer` to use them and drop serde/directories.
- Status: completed; prefs load/save via ConfigService+FsConfigStore; viewer owns only rendering + HUD state; toast rendering pending.

> 2025-12-04 — Session proto/service/client skeleton (COMPLETED)

- Goal: set up the distributed session slice with shared wire types and stub endpoints.
- Scope: new crates `echo-session-proto` (messages), `echo-session-service` (stub hub), `echo-session-client` (stub API); roadmap/docs updates.
- Status: completed; schema covers Handshake/SubscribeWarp/WarpStream (snapshot/diff)/Notification; transport and viewer binding are next.

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
- Scope: scan `warp-core` for scheduler, sandbox, and math implementations; update prose, tables, and TikZ diagrams; remove layout warnings.
- Status: completed; PDF now builds cleanly with updated figures and code snippets.

> 2025-12-01 — License appendix + SPDX CI

- Goal: add a LaTeX license appendix and wire CI to enforce SPDX headers.
- Scope: new `legal-appendix.tex` included in the guide; GitHub Action `spdx-header-check.yml` runs `scripts/check_spdx.sh --check --all`.
- Status: added appendix and workflow.

> 2025-11-30 — PR #121 feedback (perf/scheduler)

- Goal: triage and address CodeRabbit review feedback on scheduler radix drain/footprint changes; ensure determinism and docs guard stay green.
- Scope: `crates/warp-core/src/scheduler.rs`, related engine wiring, and any doc/bench fallout; keep PendingTx private and fail-fast drain semantics intact.
- Plan: classify feedback (P0–P3), implement required fixes on `perf/scheduler`, update Decision Log + docs guard, run `cargo clippy --all-targets` and relevant tests.
- Added: pluggable scheduler kind (Radix default, Legacy BTreeMap option) via `SchedulerKind`; legacy path kept for side-by-side comparisons.
- Risks: regress deterministic ordering or footprint conflict semantics; ensure histogram O(n) performance and radix counts remain u32 without overflow.

> 2025-12-01 — Sandbox harness for deterministic A/B tests

- Goal: enable spawning isolated Echo instances (Engine + GraphStore) from configs to compare schedulers and determinism.
- Scope: `warp-core::sandbox` with `EchoConfig`, `build_engine`, `run_pair_determinism`; public `SchedulerKind` (Radix/Legacy).
- Behavior: seed + rules provided as factories per instance; synchronous per-step determinism check helper; threaded runs left to callers.

> 2025-11-06 — Unblock commit: warp-core scheduler Clippy fixes (follow-up)

- Goal: make pre-commit Clippy pass without `--no-verify`, preserving determinism.
- Scope: `crates/warp-core/src/scheduler.rs` only; no API surface changes intended.
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
- Scope: `crates/warp-core/src/math/scalar.rs` and `crates/warp-core/tests/math_scalar_tests.rs`.
- Changes:
    - `F32Scalar` canonicalizes `-0.0` to `+0.0` on construction.
    - `F32Scalar` canonicalizes all NaNs to `0x7fc00000` on construction (new).
    - `value` field made private.
    - `PartialEq` implemented via `Ord` (total_cmp) to ensure `NaN == NaN` (reflexivity).
    - `Eq`, `PartialOrd`, `Ord`, `Display` traits implemented.
- Added: Tests for zero canonicalization, trait behavior, and NaN reflexivity.
- Risks: Introducing unexpected performance overhead or subtly breaking existing math operations; mitigated by unit tests and focused changes.

> 2025-11-29 – Finish off `F32Scalar` implementation

- Added `warp-core::math::scalar::F32Scalar` type.

> 2025-11-03 — Issue #115: Scalar trait scaffold

- Added `warp-core::math::scalar::Scalar` trait declaring deterministic scalar operations.
- Arithmetic is required via operator supertraits: `Add/Sub/Mul/Div/Neg` with `Output = Self` for ergonomic `+ - * / -` use in generics.
- Explicit APIs included: `zero`, `one`, `sin`, `cos`, `sin_cos` (default), `from_f32`, `to_f32`.
- No implementations yet (F32Scalar/DFix64 follow); no canonicalization or LUTs in this change.
- Exported via `warp-core::math::Scalar` for consumers.

> 2025-11-02 — PR-12: benches updates (CI docs guard)

- Dependency policy: pin `blake3` in `warp-benches` to exact patch `=1.8.2` with
  `default-features = false, features = ["std"]` (no rayon; deterministic, lean).
- snapshot_hash bench: precompute `link` type id once; fix edge labels to `e-i-(i+1)`.
- scheduler_drain bench: builder returns `Vec<NodeId>` to avoid re-hashing labels; bench loop uses the precomputed ids.

> 2025-11-02 — PR-12: benches polish (constants + docs)

- snapshot_hash: extract all magic strings to constants; clearer edge ids using `&lt;from&gt;-to-&lt;to&gt;` labels; use `iter_batched` to avoid redundant inputs; explicit throughput semantics.
- scheduler_drain: DRY rule name/id prefix constants; use `debug_assert!` inside hot path; black_box the post-commit snapshot; added module docs and clarified BatchSize rationale.
- blake3 policy: keep exact patch `=1.8.2` and disable default features to avoid
  rayon/parallel hashing in benches.

> 2025-11-02 — PR-12: benches README

- Added `crates/warp-benches/benches/README.md` documenting how to run and interpret
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
- Resolved `crates/warp-benches/Cargo.toml` conflict by keeping:
  - `license = "Apache-2.0"` and `blake3 = { version = "=1.8.2", default-features = false, features = ["std"] }` in dev-dependencies.
  - Version-pinned path dep: `warp-core = { version = "0.1.0", path = "../warp-core" }`.
  - Bench entries: `motion_throughput`, `snapshot_hash`, `scheduler_drain`.
- Benches code present/updated: `crates/warp-benches/benches/snapshot_hash.rs`, `crates/warp-benches/benches/scheduler_drain.rs`.
- Scope: benches + metadata only; no runtime changes. Hooks (fmt, clippy, tests, rustdoc) were green locally before push.

> 2025-11-02 — PR-11 hotfix-deterministic-rollup-check

- Switch to `echo/hotfix-deterministic-rollup-check`, fetch and merge `origin/main` (merge commit; no rebase).
- Fix CI cargo-deny failures:
  - Add `license = "Apache-2.0"` to `crates/warp-benches/Cargo.toml`.
  - Ensure no wildcard dependency remains in benches (use workspace path dep for `warp-core`).
- Modernize `deny.toml` (remove deprecated `copyleft` and `unlicensed` keys per cargo-deny PR #611); enforcement still via explicit allowlist.

> 2025-10-30 — PR-01: Golden motion fixtures (tests-only)

- Add JSON golden fixtures and a minimal harness for the motion rule under `crates/warp-core/tests/`.
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

- Added `proptest` as a dev‑dependency in `warp-core` and a single example test `proptest_seed_pinning.rs` that pins a deterministic RNG seed and validates the motion rule under generated inputs. This demonstrates how to reproduce failures via a fixed seed across CI and local runs (no runtime changes).

> 2025-10-30 — PR-04: CI matrix (glibc + musl; macOS manual)

- CI: Added a musl job (`Tests (musl)`) that installs `musl-tools`, adds target `x86_64-unknown-linux-musl`, and runs `cargo test -p warp-core --target x86_64-unknown-linux-musl`.
- CI: Added a separate macOS workflow (`CI (macOS — manual)`) triggered via `workflow_dispatch` to run fmt/clippy/tests on `macos-latest` when needed, avoiding default macOS runner costs.

> 2025-10-30 — PR-06: Motion negative tests (opened)

- Added tests in `warp-core` covering NaN/Infinity propagation and invalid payload size returning `NoMatch`. Tests-only; documents expected behavior; no runtime changes.

> 2025-10-30 — PR-09: BLAKE3 header tests (tests-only)

- Added unit tests under `warp-core` (in `snapshot.rs`) that:
  - Build canonical commit header bytes and assert `compute_commit_hash` equals `blake3(header)`.
  - Spot-check LE encoding (version u16 = 1, parents length as u64 LE).
- Assert that reversing parent order changes the hash. No runtime changes.

> 2025-10-30 — PR-10: README (macOS manual + local CI tips)

- Added a short CI Tips section to README covering how to trigger the manual macOS workflow and reproduce CI locally (fmt, clippy, tests, rustdoc, audit, deny).

> 2025-11-01 — PR-10 scope hygiene

- Removed commit‑header tests from `crates/warp-core/src/snapshot.rs` on this branch to keep PR‑10 strictly docs/CI/tooling. Those tests live in PR‑09 (`echo/pr-09-blake3-header-tests`). No runtime changes here.


> 2025-10-29 — Geom fat AABB midpoint sampling (merge-train)

- Update `warp-geom::temporal::Timespan::fat_aabb` to union AABBs at start, mid (t=0.5), and end to conservatively bound rotations about off‑centre pivots.
- Add test `fat_aabb_covers_mid_rotation_with_offset` to verify the fat box encloses the mid‑pose AABB.

> 2025-10-29 — Pre-commit format policy

- Change auto-format behavior: when `cargo fmt` would modify files, the hook now applies formatting then aborts the commit with guidance to review and restage. This preserves partial-staging semantics and avoids accidentally staging unrelated hunks.

> 2025-10-29 — CI/security hardening

- CI now includes `cargo audit` and `cargo-deny` jobs to catch vulnerable/deprecated dependencies early.
- Rustdoc warnings gate covers warp-core, warp-geom, warp-ffi, and warp-wasm.
- Devcontainer runs `make hooks` post-create to install repo hooks by default.
- Note: switched audit action to `rustsec/audit-check@v1` (previous attempt to pin a non-existent tag failed).
- Added `deny.toml` with an explicit permissive-license allowlist (Apache-2.0, MIT, BSD-2/3, CC0-1.0, MIT-0, Unlicense, Unicode-3.0, BSL-1.0, Apache-2.0 WITH LLVM-exception) to align cargo-deny with our dependency set.
 - Audit job runs `cargo audit` on Rust 1.75.0 (explicit `RUSTUP_TOOLCHAIN=1.75.0`) to satisfy tool MSRV; workspace MSRV remains 1.71.1.

> 2025-10-29 — Snapshot commit spec

- Added `docs/spec-merkle-commit.md` defining `state_root` vs `commit_id` encoding and invariants.
- Linked the spec from `crates/warp-core/src/snapshot.rs` and README.

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

> 2025-10-29 — warp-core snapshot header + tx/rules hardening (PR #9 base)

- Adopt Snapshot v1 header shape in `warp-core` with `parents: Vec<Hash>`, and canonical digests:
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

> 2025-10-28 — PR #8 (warp-geom foundation) updates

- Focus: compile + clippy pass for the new geometry crate baseline.
- Changes in this branch:
  - warp-geom crate foundations: `types::{Aabb, Transform}`, `temporal::{Tick, Timespan, SweepProxy}`.
  - Removed premature `pub mod broad` (broad-phase lands in a separate PR) to fix E0583.
  - Transform::to_mat4 now builds `T*R*S` using `Mat4::new` and `Quat::to_mat4` (no dependency on warp-core helpers).
  - Clippy: resolved similar_names in `Aabb::transformed`; relaxed `nursery`/`cargo` denies to keep scope tight.
  - Merged latest `main` to inherit CI/toolchain updates.

> 2025-10-28 — PR #7 (warp-core engine spike)

- Landed on main; see Decision Log for summary of changes and CI outcomes.

> 2025-10-30 — warp-core determinism tests and API hardening

- **Focus**: Address PR feedback for the split-core-math-engine branch. Add tests for snapshot reachability, tx lifecycle, scheduler drain order, and duplicate rule registration. Harden API docs and FFI (TxId repr, const ctors).
- **Definition of done**: `cargo test -p warp-core` passes; clippy clean for warp-core with strict gates; no workspace pushes yet (hold for more feedback).

> 2025-10-30 — CI toolchain policy: use stable everywhere

- **Focus**: Simplify CI by standardizing on `@stable` toolchain (fmt, clippy, tests, audit). Remove MSRV job; developers default to stable via `rust-toolchain.toml`.
- **Definition of done**: CI workflows updated; Security Audit uses latest cargo-audit on stable; docs updated.

> 2025-10-30 — Minor rustdoc/lint cleanups (warp-core)

- **Focus**: Address clippy::doc_markdown warning by clarifying Snapshot docs (`state_root` backticks).
- **Definition of done**: Lints pass under pedantic; no behavior changes.

> 2025-10-30 — Spec + lint hygiene (core)

- **Focus**: Remove duplicate clippy allow in `crates/warp-core/src/lib.rs`; clarify `docs/spec-merkle-commit.md` (edge_count may be 0; explicit empty digests; genesis parents).
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
- [x] Scaffold Rust workspace (`crates/warp-core`, `crates/warp-ffi`, `crates/warp-wasm`, `crates/warp-cli`).
- [ ] Port ECS archetype storage + branch diff engine to Rust.
- [x] Implement deterministic PRNG + math module in Rust.
- [x] Expose C ABI for host integrations (`warp-ffi`).
- [ ] Embed Rhai for scripting (deterministic sandbox + host modules).
- [ ] Integrate Rhai runtime with deterministic sandboxing and host modules.
- [ ] Adapt TypeScript CLI/inspector to Rust backend (WASM/FFI).
- [ ] Archive TypeScript prototype under `/reference/` as spec baseline.
- [x] Add Rust CI jobs (cargo test, replay verification).
- [ ] Integrate roaring bitmaps into ECS dirty tracking.
- [ ] Implement chunk epoch counters on mutation.
- [ ] Add deterministic hashing module (canonical encode + BLAKE3).
- [ ] Build DirtyChunkIndex pipeline from ECS to branch tree.

### Code Tasks (Phase 1 prep)
- [x] Install & configure Vitest.
- [x] Install & configure Vitest.
- [x] Set up `crates/warp-core/tests/common/` helpers & fixtures layout.
- [ ] Write failing tests for entity ID allocation + recycling.
- [ ] Prototype `TimelineFingerprint` hashing & equality tests.
- [x] Scaffold deterministic PRNG wrapper with tests.
- [x] Establish `cargo test` pipeline in CI (incoming GitHub Actions).
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
| 2025-10-23 | Monorepo seeded with pnpm & TypeScript skeleton | Baseline repo reset from legacy prototypes to Echo | Implement Phase 0 specs |
| 2025-10-24 | Branch tree spec v0.1: roaring bitmaps, chunk epochs, content-addressed IDs | Feedback loop to handle deterministic merges | Implement roaring bitmap integration |
| 2025-10-25 | Language direction pivot: Echo core to Rust | TypeScript validated specs; long-term determinism enforced via Rust + C ABI + Rhai scripting | Update Phase 1 backlog: scaffold Rust workspace, port ECS/diff engine, FFI bindings |
| 2025-10-25 | Math validation fixtures & Rust test harness | Established deterministic scalar/vector/matrix/quaternion/PRNG coverage in warp-core | Extend coverage to browser environments and fixed-point mode |
| 2025-10-26 | Adopt WARP + Confluence as core architecture | WARP v2 (typed DPOi engine) + Confluence replication baseline | Scaffold warp-core/ffi/wasm/cli crates; implement rewrite executor spike; integrate Rust CI; migrate TS prototype to `/reference` |
| 2025-12-28 | Mechanical rename: RMG → WARP | Align the repo’s terminology and public surface to the AIΩN Foundations Series naming | Keep decode aliases during transition; update bridge/mapping docs as divergences land |

(Keep this table updated; include file references or commit hashes when useful.)

---

## Next Up Queue

1. WARP View Protocol demo path (`docs/tasks.md`)
2. ECS storage implementation plan
3. Branch tree BlockStore abstraction design
4. Temporal Bridge implementation plan
5. Confluence + serialization protocol review

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

- Add `crates/warp-benches` with Criterion harness and a minimal motion-throughput benchmark that exercises public `warp-core` APIs.
- Scope: benches-only; no runtime changes. Document local run (`cargo bench -p warp-benches`).
