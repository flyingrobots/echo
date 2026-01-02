<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Procedure: PR Submission + CodeRabbitAI Review Loop

This document defines the required end-to-end submission workflow for this repo.

It is deliberately operational: follow it step-by-step and avoid “interpretation drift”.

---

## Rules (Non‑Negotiable)

1. No direct-to-`main` commits.
2. No admin bypass merges to skip required reviews.
3. CI green is required but not sufficient — review approval is a separate gate.
4. Iterate in small commits to reduce review ambiguity.

---

## Workflow (Branch → PR → Review → Fix → Merge)

### Step 0 — Start on a branch

Prefer a clear prefix:

- `docs/...` for docs-only changes
- `feat/...` for features
- `fix/...` for bug fixes
- `chore/...` for tooling/maintenance

```bash
git checkout -b <branch-name>
```

---

### Step 1 — Push and open a PR

```bash
git push -u origin <branch-name>
gh pr create --base main --head <branch-name>
```

---

### Step 2 — Wait for CI and CodeRabbitAI

Watch checks:

```bash
gh pr checks <PR_NUMBER> --watch
```

Then wait for CodeRabbitAI to comment. Do not merge “because CI is green”.

---

### Step 3 — Extract actionable review feedback (required)

Use:

- `docs/procedures/EXTRACT-PR-COMMENTS.md`

The outcome of this step should be a bucketed list of actionable items (P0/P1/P2/P3).

---

### Step 4 — Fix issues in batches (commit + push)

Work one bucket at a time:

- P0: correctness / determinism / security
- P1: major design/API drift
- P2: minor issues / maintainability
- P3: nits

For each batch:

```bash
git commit -m "fix: <description>"
git push
```

When replying in threads, prefer:

> ✅ Addressed in commit `abc1234`

This reduces stale-comment confusion in later rounds.

---

### Step 5 — Repeat until approved

Repeat Steps 2–4 until:

- CI checks are green, and
- CodeRabbitAI is satisfied (approved or no unresolved actionables), and
- any required human reviewer has approved.

---

### Step 6 — Merge only when approved

If branch protection requires it, enable auto-merge:

```bash
gh pr merge <PR_NUMBER> --auto --merge
```

---

## If CodeRabbitAI approved but GitHub stays blocked

Sometimes the “changes requested” status lingers even after an approving review.

Post this comment on the PR:

@coderabbitai Please review the latest commit and clear the "changes requested" status since you have already approved the changes.
