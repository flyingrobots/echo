<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Issues Matrix (Active Plan)

This table mirrors the current state of active issues in Project 9 with our plan-aligned milestones and relationships. Native GitHub dependencies represent "blocked by"/"blocking"; we no longer use custom text fields for these. The Project board remains the live system of record for status.

## Managing Issue Dependencies (Blocked By / Blocking)

Echo uses **native GitHub issue dependencies** to track “blocked by” relationships (not custom text fields).

Practical note: the GitHub GraphQL API exposes dependency data/events, but dependency *mutation* is easiest via the **REST API**. In practice, we use `gh api` as the most scriptable interface.

Reference: GitHub docs “REST API endpoints for issue dependencies” (see `issues/issue-dependencies` in the REST docs).

### Common `gh api` recipes

Auth note: `gh api` uses your authenticated GitHub token (via `gh auth login` or `GH_TOKEN` env var). You do not need to manually add an `Authorization:` header unless you are reproducing these requests with another client (like `curl`).

List dependencies an issue is blocked by:

```bash
gh api \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  repos/flyingrobots/echo/issues/<ISSUE_NUMBER>/dependencies/blocked_by
```

List dependencies an issue is blocking:

```bash
gh api \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  repos/flyingrobots/echo/issues/<ISSUE_NUMBER>/dependencies/blocking
```

Note: the `blocked_by` and `blocking` relationships are inverses. Adding “issue A blocked by issue B” is equivalent to adding “issue B blocking issue A”. Choose the direction that matches your workflow.

Add a “blocked by” dependency (make `<ISSUE_NUMBER>` blocked by `<BLOCKING_ISSUE_NUMBER>`):

```bash
set -euo pipefail

# Optional (only needed if you are not already authenticated via `gh auth login` or `GH_TOKEN`):
# -H "Authorization: Bearer <YOUR-TOKEN>"
BLOCKING_ISSUE_ID="$(
  gh api \
    -H "Accept: application/vnd.github+json" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    repos/flyingrobots/echo/issues/<BLOCKING_ISSUE_NUMBER> \
    --jq .id
)" || { echo "Failed to fetch blocking issue ID" >&2; exit 1; }

if [[ -z "$BLOCKING_ISSUE_ID" ]]; then
  echo "BLOCKING_ISSUE_ID is empty; verify auth and jq extraction." >&2
  exit 1
fi

gh api \
  -X POST \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  repos/flyingrobots/echo/issues/<ISSUE_NUMBER>/dependencies/blocked_by \
  -f issue_id="$BLOCKING_ISSUE_ID"
```

Remove a “blocked by” dependency:

```bash
gh api \
  -X DELETE \
  -H "Accept: application/vnd.github+json" \
  # Optional (only needed if you are not already authenticated via `gh auth login` or `GH_TOKEN`):
  # -H "Authorization: Bearer <YOUR-TOKEN>" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  repos/flyingrobots/echo/issues/<ISSUE_NUMBER>/dependencies/blocked_by/<BLOCKING_ISSUE_ID>
```

| Issue Name | Issue # | Milestone | Priority | Estimate | Blocked By | Blocking | Parent | Children | Remarks |
| --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- |
| Benchmarks & CI Regression Gates | 22 | M1 – Golden Tests | P1 | 13h+ |  | #42,#43,#44,#45,#46 |  | 42,43,44,45,46 | Umbrella for perf pipeline |
| Create benches crate | 42 | M1 – Golden Tests | P1 | 3h | #22 | #43,#44,#45,#46 | #22 |  | Criterion + scaffolding |
| Snapshot hash microbench | 43 | M1 – Golden Tests | P1 | 5h | #22,#42 |  | #22 |  | Reachable hash microbench |
| Scheduler drain microbench | 44 | M1 – Golden Tests | P1 | 5h | #22,#42 |  | #22 |  | Deterministic rule‑order/drain |
| JSON report + CI upload | 45 | M1 – Golden Tests | P2 | 3h | #22,#42 | #46 | #22 |  | Upload Criterion JSON |
| Regression thresholds gate | 46 | M1 – Golden Tests | P1 | 8h | #22,#42,#45 |  | #22 |  | Fail on P50/P95/P99 regress |
| CLI: verify/bench/inspect | 23 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  | Grouping placeholder; break down in PRs |
| Scaffold CLI subcommands | 47 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  |  |
| Implement 'verify' | 48 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  |  |
| Implement 'bench' | 49 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  |  |
| Implement 'inspect' | 50 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  |  |
| Docs/man pages | 51 | M2.2 – Playground Slice | P2 | 5h |  |  |  |  | Tie docs to CLI UX |
| README+docs (defaults & toggles) | 41 | M4 – Determinism Proof & Publish 0.1 | P2 | 3h |  |  |  |  | Docs polish before 0.1 |

Backlog issues are labeled `backlog` and kept visible in the Project; they will be prioritized into milestones as needed.
