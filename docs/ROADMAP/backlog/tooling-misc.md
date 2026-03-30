<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Tooling & Misc

> **Milestone:** [Backlog](../../ROADMAP.md) | **Priority:** Unscheduled

Housekeeping tasks: documentation, logging, naming consistency, and debugger UX design.

**Issues:** #79, #207, #239

## T-10-8-1: Docs / Logging Improvements (#79)

**User Story:** As a contributor, I want improved documentation and structured logging so that onboarding is faster and runtime behavior is observable.

**Requirements:**

- R1: Audit existing doc comments for completeness — add missing module-level docs
- R2: Standardize log levels across crates (`trace` for hot path, `debug` for internals, `info` for lifecycle, `warn`/`error` for problems)
- R3: Add structured fields to log events (e.g., `tick=`, `entity_id=`, `component=`)
- R4: Document the logging configuration in the contributor guide

**Acceptance Criteria:**

- [ ] AC1: Every public module has a `//!` doc comment
- [ ] AC2: Log events use consistent levels per the standard
- [ ] AC3: At least 10 key log sites use structured fields
- [ ] AC4: Contributor guide includes a "Logging" section

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Documentation, logging standardization, contributor guide update.
**Out of Scope:** Log aggregation infrastructure, metrics, tracing spans.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~200 LoC (doc comments + logging changes)

---

## T-10-8-2: Naming Consistency Audit (#207)

**User Story:** As a user, I want consistent naming across Echo, WARP, Wesley, and Engram so that there is no confusion about product names in code, docs, and CLI output.

**Requirements:**

- R1: Run a noisy-line test (grep for all naming variants) across all repos
- R2: Catalog every instance of inconsistent naming (old names, abbreviations, typos)
- R3: Produce a migration plan with specific find-and-replace operations
- R4: Apply fixes to the echo repo (other repos tracked separately)

**Acceptance Criteria:**

- [ ] AC1: Audit report listing all inconsistencies is produced
- [ ] AC2: All inconsistencies in the echo repo are fixed
- [ ] AC3: CI includes a grep-based lint to catch future naming regressions
- [ ] AC4: Migration plan for other repos is filed as issues in those repos

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Echo repo naming fixes. Audit report for other repos.
**Out of Scope:** Fixing naming in non-echo repos (tracked separately).

**Test Plan:**

- **Goldens:** n/a
- **Failures:** CI lint fails on intentional introduction of old name
- **Edges:** Names in string literals, names in comments, names in URLs
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 3h
**Expected Complexity:** ~50 LoC (lint script + renames)

---

## T-10-8-3: Reliving Debugger UX Design (#239)

**User Story:** As a simulation developer, I want a UX design for the reliving debugger so that the Constraint Lens and Provenance Heatmap features are well-specified before implementation begins.

**Requirements:**

- R1: Design the Constraint Lens view (visualize which constraints are active per-entity per-tick)
- R2: Design the Provenance Heatmap view (color-code state by how recently/frequently it was written)
- R3: Specify the data model backing each view (what queries are needed)
- R4: Produce wireframes or mockups (low-fidelity is fine)
- R5: Identify which runtime hooks/APIs are needed to feed data into the views

**Acceptance Criteria:**

- [ ] AC1: UX design document exists at `docs/designs/RELIVING-DEBUGGER-UX.md`
- [ ] AC2: Both Constraint Lens and Provenance Heatmap are specified
- [ ] AC3: Data model and required runtime APIs are listed
- [ ] AC4: At least two wireframes (one per view)

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** UX design, wireframes, data model specification.
**Out of Scope:** Implementation, frontend framework choice, performance optimization.

**Test Plan:**

- **Goldens:** n/a (design document)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~300 lines (markdown + diagrams)

---

## T-10-8-4: Local Rustdoc Warning Gate

**User Story:** As a contributor, I want the Rustdoc warnings gate available locally so that private intra-doc link failures and other doc regressions are caught before CI.

**Requirements:**

- R1: Add a single local entry point for the current Rustdoc gate commands on the critical crates
- R2: Ensure the command runs with `RUSTDOCFLAGS="-D warnings"` so it matches the CI rustdoc gate
- R3: Document when contributors should run it, how it differs from plain `cargo doc`, and which broader compile/doc gates remain separate (`RUSTFLAGS="-Dwarnings"`, `cargo clippy --all-targets -- -D missing_docs`, `cargo test`)
- R4: Keep the crate list aligned with the CI rustdoc gate

**Acceptance Criteria:**

- [ ] AC1: One documented command runs the Rustdoc gate locally
- [ ] AC2: The command fails on intentional intra-doc link / warning regressions
- [ ] AC3: Contributor-facing docs mention the gate and its purpose
- [ ] AC4: The local crate list matches the CI rustdoc job

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Local tooling, contributor docs, and parity with the CI Rustdoc warnings gate only.
**Out of Scope:** Changing which crates the CI rustdoc job covers, or replacing the repo's separate compile/clippy/test gates.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Intentionally introduce a rustdoc warning and verify the local gate fails
- **Edges:** Private intra-doc links, crate not present, contributors confusing this gate with the separate `RUSTFLAGS` / clippy / test checks
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 2h
**Expected Complexity:** ~40 LoC (script/xtask + docs)

---

## T-10-8-5: Deterministic Test Engine Helper

**User Story:** As a test author, I want one shared deterministic engine-builder helper so that golden/property tests do not silently inherit ambient worker-count entropy.

**Requirements:**

- R1: Introduce a shared helper for single-worker deterministic test engines
- R2: Migrate the remaining determinism-sensitive tests that still hand-roll `.workers(1)`
- R3: Document when tests should use the helper versus explicit multi-worker coverage
- R4: Keep the helper narrow enough that test intent stays obvious

**Completed already:**

- Determinism property tests and golden-vector harnesses are pinned to single-worker builders.

**Acceptance Criteria:**

- [ ] AC1: Determinism-sensitive tests use a shared helper instead of repeated `.workers(1)` chains
- [ ] AC2: Multi-worker invariance tests still opt into explicit worker counts directly
- [ ] AC3: A short contributor note explains which path to use
- [x] AC4: No golden/property harness depends on host default worker count

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Test helper extraction plus migration of the remaining determinism-sensitive harnesses.
**Out of Scope:** Changing production engine defaults.

**Test Plan:**

- **Goldens:** Run the DIND (Deterministic Ironclad Nightmare Drills) golden hash-chain harness plus the existing golden vector suite unchanged
- **Failures:** Helper misuse should be caught by determinism/property tests
- **Edges:** Tests that intentionally vary worker count remain explicit
- **Fuzz/Stress:** Existing property tests; determinism-sensitive helper changes must include DIND coverage so canonical outputs cannot drift silently

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 3h
**Expected Complexity:** ~80 LoC (helper + test migrations)

---

## T-10-8-6: Current-Head PR Review / Merge Summary Tool

**User Story:** As a reviewer, I want a lightweight current-head PR summary
so that unresolved threads, failing checks, historical noise, and
merge-readiness state are visible before push/merge decisions.

**Requirements:**

- R1: Add a `cargo xtask ...` command that summarizes unresolved review-thread
  counts for a PR using paginated GitHub API queries
- R2: Include failing/pending check names, the current head SHA, and the
  current approval / merge-readiness state
- R3: Distinguish live unresolved threads on the current head from historical
  comment noise and, when possible, show deltas since the last pushed SHA
- R4: Make the output fast to scan in terminal use
- R5: Keep the tool read-only; it should not mutate PR state

**Acceptance Criteria:**

- [ ] AC1: One command prints exact unresolved thread counts, key checks, head
      SHA, and approval / merge-readiness state for the current PR head
- [ ] AC2: Output distinguishes pending vs failing vs passing checks
- [ ] AC3: Output can separate current actionable review state from historical review chatter
- [ ] AC4: The summary is useful before merge or review-follow-up pushes
- [ ] AC5: Tool works with the existing `gh`-based workflow

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** CLI/script support for current-head review-state summarization and
merge-readiness visibility.
**Out of Scope:** Auto-replying to review comments, auto-merging.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Simulate missing `gh` auth / bad PR number handling
- **Edges:** PR with zero threads, PR with more than 100 review threads, PR with only pending checks, mixed push+PR runs, stale historical comment noise with zero unresolved threads
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~180 LoC (script + docs)

---

## T-10-8-7: CI Trigger Rationalization

**User Story:** As a contributor, I want less duplicated CI noise so that I can interpret check state quickly without sifting through redundant push/pull_request runs.

**Requirements:**

- R1: Audit which jobs truly need both `push` and `pull_request` triggers
- R2: Preserve required branch-protection coverage while reducing redundant executions
- R3: Document the final trigger policy so future workflows follow the same pattern
- R4: Verify that status checks remain stable from GitHub’s perspective after the cleanup

**Acceptance Criteria:**

- [ ] AC1: Duplicated jobs are reduced where they do not add signal
- [ ] AC2: Required checks still appear reliably on PRs
- [ ] AC3: Workflow docs explain the trigger policy
- [ ] AC4: Contributors can tell which run is authoritative for merge readiness

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Workflow trigger cleanup and documentation.
**Out of Scope:** Rewriting the CI matrix logic or changing branch-protection policy itself.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Verify required checks still report on PRs
- **Edges:** Branch pushes without PRs, PR updates, workflow-dispatch/manual flows
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~60 LoC (workflow edits + docs)

---

## T-10-8-8: Background Cargo Lock Isolation

**User Story:** As a contributor, I want background Cargo activity isolated from manual verification so that ad hoc review fixes and hook-driven checks do not waste time waiting on unrelated workspace builds.

**Requirements:**

- R1: Audit long-lived background Cargo producers in the desktop app / local tooling flow
- R2: Route background workspace checks to an isolated `CARGO_TARGET_DIR` or equivalent non-conflicting build path
- R3: Surface lock contention clearly when it still occurs so contributors can tell queue time from actual compile time
- R4: Document the isolation policy so future background tooling does not reuse the default repo target directory by accident

**Acceptance Criteria:**

- [ ] AC1: Background Cargo activity no longer steals the default build lock from manual repo verification by default
- [ ] AC2: Contributors can distinguish lock-wait time from active compile/test time in the local workflow
- [ ] AC3: The isolation approach is documented for future tool authors
- [ ] AC4: Existing local/CI behavior remains functionally unchanged aside from reduced contention

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Local developer workflow, desktop app background checks, target-dir isolation, and lock-visibility improvements.
**Out of Scope:** Rewriting Cargo itself, changing CI target-dir strategy, or removing useful background checks entirely.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Verify the warning path when contention still happens
- **Edges:** Background check starts before manual verification, background check starts during manual verification, isolated target dir missing
- **Fuzz/Stress:** Repeated back-to-back manual verification with background checks enabled

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~100 LoC (tooling config + docs)

---

## T-10-8-9: Small-Commit Pre-Commit Latency Reduction

**User Story:** As a contributor, I want tiny review-fix commits to complete quickly so that one-line test/doc/tooling follow-ups do not trigger disproportionately expensive staged verification.

**Requirements:**

- R1: Audit staged pre-commit lanes to identify avoidable heavy work for tiny review-fix commits
- R2: Preserve truthfulness: any narrowing must still cover the changed surface honestly
- R3: Separate lock-wait / queue time from active lane runtime in local timing output so regressions are obvious
- R4: Document the expected fast path for small doc/test/tooling-only follow-up commits

**Acceptance Criteria:**

- [ ] AC1: Tiny staged follow-up commits have a measurably faster default path
- [ ] AC2: Timing output separates queue/lock delay from active runtime
- [ ] AC3: Narrowed staged verification still matches the changed surface truthfully
- [ ] AC4: Contributor docs explain the intended fast path and when a broader manual gate is still required

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Pre-commit / staged local verification latency, timing visibility, and contributor workflow docs.
**Out of Scope:** Weakening merge-time CI gates or skipping validation for semantic code changes.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Intentionally introduce a staged regression outside the narrowed surface and verify the broader path still catches it when appropriate
- **Edges:** Single-file test fix, docs-only change, tooling-only change, mixed code+docs staged commit
- **Fuzz/Stress:** Repeated tiny staged commits with warm cache

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~120 LoC (scripts + docs + timing assertions)

---

## T-10-8-10: Feature-Gate Contract Verification

**User Story:** As a contributor, I want explicit feature-contract checks for
no-std / alloc-only crates so that feature-gating regressions are caught before
PR review or CI.

**Requirements:**

- R1: Identify crates whose manifests promise meaningful `--no-default-features`
  or alloc-only support
- R2: Add a `cargo xtask ...` local and CI-visible verification path that
  exercises those feature contracts directly
- R3: Keep the lane scoped so it stays fast enough to run during review-fix
  loops
- R4: Document which crates are covered and what the lane is proving

**Acceptance Criteria:**

- [ ] AC1: At least `echo-runtime-schema` and `echo-wasm-abi` have an explicit
      `cargo xtask ...` `--no-default-features` verification path
- [ ] AC2: A deliberate `std` leak in a gated crate fails the lane
- [ ] AC3: Contributor docs explain when to run the lane and what it covers
- [ ] AC4: The covered crate list is easy to keep aligned with manifest truth

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Feature-gate verification for crates that claim no-std or alloc-only
support.
**Out of Scope:** Broad workspace-wide no-std support or changing crate feature
semantics.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Intentionally introduce a `std` dependency in a gated path and
  verify the lane fails
- **Edges:** `default-features = false`, alloc-only mode, transitive feature
  forwarding
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 2h
**Expected Complexity:** ~60 LoC (lane wiring + docs)

---

## T-10-8-11: PR Review Thread Reply / Resolution Helper

**User Story:** As a reviewer, I want a safe helper for replying to and
resolving PR review threads so that GitHub thread state does not lag behind the
branch state after review-fix pushes.

**Requirements:**

- R1: Enumerate unresolved review threads for a PR with pagination
- R2: Support drafting or posting explicit replies for selected review
  comments/threads via a `cargo xtask ...` entrypoint
- R3: Support resolving selected or all unresolved threads after a verified
  push via the same `cargo xtask ...` entrypoint
- R4: Keep the helper explicit and human-driven; it must not auto-generate
  reply text or auto-resolve based on heuristics alone
- R5: Show enough context (path, author, URL) for a reviewer to confirm the
  action before mutating GitHub state
- R6: Reconcile current-head code state against GitHub thread state after reply
  / resolve actions so outdated-but-unresolved threads are easy to distinguish
  from genuinely still-open review debt

**Acceptance Criteria:**

- [ ] AC1: One `cargo xtask ...` command can list unresolved review threads
      with exact counts
- [ ] AC2: One command can post or stage a reply for chosen review comment ids
      after human-authored input
- [ ] AC3: One command can resolve chosen thread ids after human confirmation
- [ ] AC4: The helper works with the existing `gh`-based workflow
- [ ] AC5: Contributor docs explain when to use it and when to reply manually
- [ ] AC6: After replies / resolutions, the helper can recount unresolved
      threads and highlight outdated-vs-current review state explicitly

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Local tooling for review-thread listing, explicit replies, and
explicit resolution.
**Out of Scope:** Auto-reply generation, auto-merging, or policy decisions
about which comments deserve direct replies.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Bad PR number, missing `gh` auth, invalid thread id
- **Edges:** More than 100 review threads, mixed resolved/unresolved state,
  outdated but unresolved threads, replying to outdated inline comments
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~120 LoC (script + docs)

---

## T-10-8-12: Shell Script Style / Format Lane

**User Story:** As a maintainer, I want a dedicated shell-style lane for
maintained hook scripts so that shell regressions are caught before PR review or
merge.

**Requirements:**

- R1: Cover maintained shell scripts under `.githooks/`, `scripts/`, and
  `tests/hooks/` with a consistent format/style policy
- R2: Use standard shell tooling (`shfmt`, `shellcheck`, or an equivalent
  documented combination)
- R3: Keep the lane fast enough for local review-fix loops and visible in CI
- R4: Document which scripts are covered and how contributors run the lane

**Acceptance Criteria:**

- [ ] AC1: One local command checks formatting/style for maintained shell
      scripts
- [ ] AC2: A representative shell-style regression fails the lane
- [ ] AC3: The lane is wired into local verification and visible in CI
- [ ] AC4: Contributor docs explain the shell-tooling entrypoint

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Formatting/style verification for maintained repository shell
scripts.
**Out of Scope:** Rewriting shell tooling in another language or enforcing style
rules on archived scripts.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Deliberately misformat a maintained hook script or add a
  shellcheck-detectable issue
- **Edges:** macOS `/bin/bash` vs Linux `/bin/bash`, sourced helper scripts,
  executable and non-executable shell files
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 3h
**Expected Complexity:** ~100 LoC (lane wiring + docs)

---

## T-10-8-13: Review-Fix Fast Path for Staged Verification

**User Story:** As a contributor, I want small review-fix commits to verify
quickly so that post-review iteration does not spend minutes rerunning unrelated
lanes.

**Requirements:**

- R1: Measure the current staged pre-commit path and identify the slowest review
  loop bottlenecks
- R2: Add a safe fast path for small review-fix batches without weakening the
  full push gate
- R3: Keep the fast path explicit and easy to reason about from changed-file
  scope
- R4: Document when contributors should trust the fast path versus the full
  local gate

**Acceptance Criteria:**

- [ ] AC1: Small docs/tooling review-fix commits avoid obviously unrelated
      heavyweight lanes during staged verification
- [ ] AC2: Full push-time verification remains unchanged in coverage
- [ ] AC3: Timing evidence shows a meaningful staged-latency reduction on a
      representative review-fix batch
- [ ] AC4: Contributor docs explain the fast path and its limits

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Staged pre-commit verification latency reduction for small review-fix
batches.
**Out of Scope:** Weakening branch-protection gates or skipping required push
checks.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Ensure a deliberately touched covered file still trips its
  required lane
- **Edges:** docs-only review fix, shell-script-only fix, single-crate Rust fix,
  mixed-code-and-docs patch
- **Fuzz/Stress:** Compare before/after timing on representative review-fix
  commits

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~180 LoC (lane logic + timing + docs)

---

## T-10-8-14: Pre-PR Preflight Gate

**User Story:** As a contributor, I want one high-signal preflight command
before opening a PR so that obvious CI/review churn is caught locally first.

**Requirements:**

- R1: Add a `cargo xtask ...` preflight entrypoint for pre-PR use
- R2: Cover the highest-signal local lanes for review churn: formatting, docs
  lint, runtime schema validation, feature-contract checks, maintained shell
  checks, and the relevant targeted Rust validation
- R3: Print a concise summary of failed sub-checks so authors know what to fix
  before opening a PR
- R4: Support a fast changed-scope mode and an explicit full mode without
  weakening existing push-time verification
- R5: Document when contributors should run the preflight and what it is
  intentionally not proving

**Acceptance Criteria:**

- [ ] AC1: One `cargo xtask ...` command runs the pre-PR preflight locally
- [ ] AC2: A representative docs/tooling drift issue is caught before PR open
- [ ] AC3: A representative feature-contract or schema-contract issue is caught
      before PR open
- [ ] AC4: Output is brief enough to use as part of the normal author loop

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Pre-PR local verification workflow design and implementation.
**Out of Scope:** Replacing the full push gate or auto-opening pull requests.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Introduce one docs lint failure, one schema validation failure,
  and one `--no-default-features` leak and verify the preflight reports each
- **Edges:** docs-only PR, tooling-only PR, small Rust-only PR, mixed-code PR
- **Fuzz/Stress:** Compare changed-scope vs full-mode runtime on representative
  branches

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~180 LoC (xtask + lane wiring + docs)

---

## T-10-8-15: Self-Review Command

**User Story:** As an author, I want a harsh local self-review against the
merge target before opening a PR so that contract drift, missing negative tests,
and stale docs are found before Rabbit or humans spend cycles on them.

**Requirements:**

- R1: Add a `cargo xtask ...` self-review entrypoint that diffs against the
  merge target ref (default `origin/main`)
- R2: Emit findings with severity, file/line evidence, issue type, and a
  suggested mitigation prompt
- R3: Bias findings toward the classes that repeatedly caused late PR churn in
  Echo: contract drift, portability/tooling assumptions, missing negative
  tests, stale docs/spec text, and ownership ambiguity
- R4: Keep the command read-only; it must not mutate code or GitHub state
- R5: Document how authors should use the output before PR creation

**Acceptance Criteria:**

- [ ] AC1: One `cargo xtask ...` command prints a self-review findings report
      against `origin/main`
- [ ] AC2: The report includes file references and mitigation prompts, not just
      generic prose
- [ ] AC3: The command handles zero-finding branches cleanly
- [ ] AC4: Contributor docs position the self-review as a standard pre-PR step

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Local author-side self-review tooling and workflow guidance.
**Out of Scope:** Auto-fixing findings or posting review comments to GitHub.

**Test Plan:**

- **Goldens:** Snapshot a representative findings report format
- **Failures:** Deliberately introduce stale-doc and portability issues and
  verify they are surfaced with evidence
- **Edges:** Empty diff, docs-only diff, very large diff, more than one crate
  touched
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~200 LoC (xtask + report formatting + docs)

---

## T-10-8-16: Pre-PR Checklist and Boundary-Change Policy

**User Story:** As an author or reviewer, I want a written pre-PR checklist for
boundary and tooling work so that the repo has a shared definition of “ready to
open a PR.”

**Requirements:**

- R1: Document when a branch should be split into smaller PRs instead of
  carrying mixed code/tooling/docs churn
- R2: Document mandatory negative-test expectations for new boundary types,
  parsers, adapters, and schema surfaces
- R3: Document the lockstep rule for code plus spec/schema/docs updates when a
  frozen surface changes
- R4: Document that post-merge backlog grooming should not be added to a green
  merge-candidate branch unless another CI/review cycle is intended
- R5: Link the checklist to the preflight and self-review entrypoints once they
  exist

**Acceptance Criteria:**

- [ ] AC1: A contributor-facing checklist exists in repo docs
- [ ] AC2: The checklist is specific enough to apply to ABI/schema/runtime
      boundary changes
- [ ] AC3: The checklist references the local preflight and self-review
      commands
- [ ] AC4: Review instructions and contributor docs do not contradict the
      checklist

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Contributor process documentation for pre-PR discipline.
**Out of Scope:** Automating every checklist item or replacing reviewer
judgment.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** n/a
- **Edges:** ABI/schema branch, docs-only branch, tooling-only branch
- **Fuzz/Stress:** n/a

**Blocked By:** T-10-8-14, T-10-8-15
**Blocking:** none

**Est. Hours:** 2h
**Expected Complexity:** ~60 LoC (docs + links)

---

## T-10-8-17: Docs Validation Beyond Markdown

**User Story:** As a contributor, I want docs validation to cover the real docs
surface, not just Markdown, so that broken static-HTML links and other live-doc
regressions are caught before PR review.

**Requirements:**

- R1: Expand docs validation so it covers `docs/public/*.html` and any other
  live non-Markdown docs entrypoints
- R2: Add static-HTML link and asset checks for repo-local routes and
  references
- R3: Keep the lane scoped enough that docs-only changes remain fast to verify
- R4: Document exactly which doc surfaces are covered and which are still
  intentionally excluded

**Acceptance Criteria:**

- [ ] AC1: A broken local route or asset reference in `docs/public/*.html`
      fails the docs validation lane
- [ ] AC2: Docs validation is no longer effectively Markdown-only
- [ ] AC3: Contributors can run one documented command to check the covered docs
      surfaces locally
- [ ] AC4: The collision-tour style regression class is caught before review

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Validation for live docs surfaces, including Markdown plus static
HTML entrypoints and their local links/assets.
**Out of Scope:** External-link availability checks or full website end-to-end
crawling.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Intentionally break a local static-HTML route and a local asset
  link and verify the lane fails
- **Edges:** `file://`-style static docs, generated HTML, root-relative vs
  relative links
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~140 LoC (validation wiring + docs + tests)

---

## T-10-8-18: Implementation-Backed Docs Claims Policy

**User Story:** As a maintainer, I want contributor guidance and lightweight
checks around strong claims like `bit-exact`, `canonical`, and `deterministic`
so that docs do not overstate what the code actually guarantees.

**Requirements:**

- R1: Define a short docs-claims checklist for implementation-backed guarantee
  language
- R2: Identify especially sensitive claim words and the evidence expected for
  each
- R3: Add a lightweight lint, review checklist, or equivalent guard for the
  most failure-prone phrases
- R4: Document where stronger claims belong (specs, claim registers, crate
  docs) versus where contributor docs should stay conservative

**Acceptance Criteria:**

- [ ] AC1: A contributor-facing checklist exists for strong guarantee wording
- [ ] AC2: The repo has at least one lightweight guard or review rubric for
      claim words like `bit-exact`, `canonical`, and `deterministic`
- [ ] AC3: A representative overclaim is caught before PR review
- [ ] AC4: Docs and spec surfaces describe the evidence expectation clearly

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Docs wording discipline, lightweight guardrails, and contributor
guidance.
**Out of Scope:** Proving every guarantee in the repo or replacing reviewer
judgment with a perfect linter.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Introduce a representative wording overclaim and verify the new
  checklist / guard catches it
- **Edges:** Claim appears in roadmap docs, architecture docs, crate docs, or
  generated reference pages
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 3h
**Expected Complexity:** ~80 LoC (docs + guard/checklist)

---

## T-10-8-19: Remove Committed Generated DAG Artifacts

**User Story:** As a maintainer, I want generated DAG outputs out of the main
docs tree so that the repo keeps source-of-truth inputs, not churn-heavy baked
artifacts.

**Requirements:**

- R1: Identify which DAG outputs are generated and should no longer live as
  committed source files
- R2: Keep only the canonical DAG inputs and generation entrypoints in the repo
- R3: Move generated DAG viewing/sharing to on-demand generation, CI artifacts,
  or another explicit publication path
- R4: Update docs and validation so they no longer assume committed generated
  DAG outputs are the truth

**Acceptance Criteria:**

- [ ] AC1: Generated DAG artifacts are removed from the committed live docs
      surface
- [ ] AC2: Contributors still have one documented way to generate or inspect
      the DAG outputs when needed
- [ ] AC3: CI or release workflow still has a clear path for sharing generated
      DAG views when useful
- [ ] AC4: Docs validation no longer depends on stale committed DAG outputs

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Generated dependency/task DAG artifacts and their publication path.
**Out of Scope:** Removing the underlying DAG sources or DAG generation logic
entirely.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Remove a generated artifact and verify the documented
  generation/view path still works
- **Edges:** Offline local viewing, CI artifact upload, docs links that used to
  target committed outputs
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~120 LoC (docs + workflow/tooling cleanup)
