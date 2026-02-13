<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Lock the Hashes](README.md) | **Priority:** P0

# Benchmarks Pipeline Cleanup

Issue: #22

All child issues (#42-#46) are closed. The umbrella issue #22 ("Benchmarks & CI Regression Gates") needs a final audit to verify all children shipped correctly, confirm the `crates/warp-benches` crate is integrated into CI, and then close the umbrella.

## T-1-2-1: Audit and close benchmarks umbrella issue #22

**User Story:** As a project maintainer, I want the benchmarks umbrella issue closed with a verified audit trail so that the M1 milestone can be marked complete.

**Requirements:**

- [x] R1: Verify each child issue (#42, #43, #44, #45, #46) is closed and its PR merged.
- [x] R2: Verify `crates/warp-benches` contains working benchmarks.
- [x] R3: Verify CI workflow runs benchmarks.
- [x] R4: Verify JSON artifact upload and regression gates are operational.
- [x] R5: Add a closing comment on #22 summarizing the audit findings.
- [x] R6: Close #22.

**Acceptance Criteria:**

- [x] AC1: All five child issues (#42-#46) confirmed closed with merged PRs.
- [x] AC2: `cargo bench --package warp-benches` compiles and runs without error.
- [x] AC3: CI configuration includes benchmark compilation gate.
- [x] AC4: Issue #22 is closed with an [audit summary comment](https://github.com/flyingrobots/echo/issues/22#issuecomment-3894974740).

**Definition of Done:**

- [x] Code reviewed and merged (PR [#265](https://github.com/flyingrobots/echo/pull/265), merged 2026-02-13T05:45:06Z)
- [ ] Milestone documentation finalized (PR [#266](https://github.com/flyingrobots/echo/pull/266), pending)
- [x] Tests pass (CI green: [Workflow Run](https://github.com/flyingrobots/echo/actions/runs/13284974740))
- [x] Documentation updated (CHANGELOG.md, README.md)
- [x] Audit summary comment on Issue [#22](https://github.com/flyingrobots/echo/issues/22#issuecomment-3894974740) verified (AC4 / R5)

**Scope:** Audit of existing merged work. Closing comment on #22. Minor CI fixes if benchmarks fail to compile on current `main`.
**Out of Scope:** New benchmark development. Performance optimization. Issue #41 (README+docs, milestone M4).

**Test Plan:**

- **Goldens:** N/A (audit task).
- **Failures:** If any child PR is missing or benchmarks fail to compile, file a follow-up issue before closing #22.
- **Edges:** Verify benchmarks compile on both macOS and Linux CI runners.
- **Fuzz/Stress:** N/A.

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 2h
**Expected Complexity:** ~0 LoC (audit and issue management only; minor CI fix if needed ~20 LoC)
