<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Workflows (Contributor Playbook)

This doc is the “official workflow index” for Echo: how we work, what invariants we enforce, and the blessed entrypoints/scripts.

> Echo is docs-driven and determinism-driven. When in doubt, prefer writing down the invariant and then encoding it as a test or guard.

---

## Session Workflow

- Record architectural decisions in ADRs (`docs/adr/`) or PR descriptions.
- Before opening a PR, run the validation workflow below.

### Agent Context System (AI Agents)

AI agents use a **2-tier context system** for seamless handoffs. See
[`docs/archive/AGENTS.md`](./archive/AGENTS.md) for full details.

| Tier      | Store                               | Purpose                                    |
| --------- | ----------------------------------- | ------------------------------------------ |
| Immediate | Redis stream (`echo:agent:handoff`) | Current task state, branch, blockers       |
| Deep      | Knowledge graph                     | Architecture decisions, patterns, entities |

**Quick reference:**

- **Session start**: `XRANGE echo:agent:handoff - + COUNT 5` + `search_nodes("<feature>")`
- **During work**: Update Redis after significant actions
- **Session end**: Always write a handoff entry with `branch`, `status`, `next_steps`

---

## Branch + PR Workflow

- **Isolated Branches**: Every new task, feature, or bugfix **MUST** begin on a fresh, isolated branch based on the latest `main` (unless context explicitly dictates otherwise). Never mix unrelated objectives on the same branch.
- Keep `main` pristine; do work on a branch (prefer `echo/<feature>` or `timeline/<experiment>`).
- Do not rebase; do not force push; do not amend commits.
- Every PR must be tied to a GitHub Issue. If no issue exists, open one before opening the PR.
- Include an explicit closing keyword in the PR body (example): `Closes #123`.

---

## Validation Workflow

Common checks:

```sh
cargo fmt --all
cargo test --workspace
cargo clippy --all-targets -- -D warnings -D missing_docs
```

Validation commands:

---

## Tooling Entry Points

Echo uses a few “blessed” command entry points.

### `make`

- `make hooks` installs repo git hooks.
- `make docs` runs the VitePress docs site.
- `make dags` regenerates dependency DAGs from cached snapshots.
- `make dags-fetch` regenerates dependency DAGs from GitHub data (requires `gh` auth + network).

### `cargo xtask`

The repo also exposes maintenance commands via `cargo xtask …`:

- `cargo xtask dags` regenerates dependency DAG DOT/SVG assets.
- `cargo xtask dags --fetch` fetches snapshots via `gh` before generating.
- `cargo xtask dags --snapshot-label none` omits snapshot labels (best for CI automation).
- `cargo xtask dags --snapshot-label rolling` emits a stable `rolling` label.
- `cargo xtask dags --snapshot-label 2026-01-02` pins a date label (useful for comparisons).
- `cargo xtask pr-status` summarizes the current PR head, exact unresolved review-thread count, grouped check state, and a concise current-blockers section.
- `cargo xtask pr-status 306` targets an explicit PR number instead of the current branch PR.
- `cargo xtask pr-snapshot` records a durable local PR review snapshot under `artifacts/pr-review/`.
- `cargo xtask pr-snapshot 308` targets an explicit PR number instead of the current branch PR.
- `cargo xtask pr-threads list` lists unresolved review threads for the current PR with thread ids, comment ids, path, author, URL, and a short preview.
- `cargo xtask pr-threads list 306` targets an explicit PR number instead of the current branch PR.
- `cargo xtask pr-threads reply 123456789 --body-file /tmp/reply.md` posts a human-authored reply to a review comment id on the current branch PR.
- `cargo xtask pr-threads reply 123456789 --selector 306 --body-file /tmp/reply.md` targets an explicit PR when the review comment belongs to another repo/PR context.
- `cargo xtask pr-threads resolve --all --selector 306 --yes` resolves all unresolved review threads for a PR after you have verified the fix batch.
- `cargo xtask pr-threads resolve --yes THREAD_ID_A THREAD_ID_B` resolves explicit GitHub review thread ids when you already know the targets.
- `cargo xtask pr-preflight` runs the default changed-scope pre-PR gate against `origin/main`.
- `cargo xtask pr-preflight --full` runs the broader explicit full pre-PR gate.
- `cargo xtask dind` runs the DIND (Deterministic Ironclad Nightmare Drills) harness locally.

### Pre-PR Preflight

Before opening a PR, run:

```sh
cargo xtask pr-preflight
```

What it proves:

- the changed surface passes the normal local `verify-local` PR gate
- changed Markdown docs pass `markdownlint`, and docs branches also get dead-ref checking
- runtime schema changes get explicit `pnpm schema:runtime:check`
- feature-sensitive crates get explicit `--no-default-features` checks
- maintained shell scripts get a syntax pass via `bash -n`

What it intentionally does **not** prove:

- full CI parity for every matrix lane
- review-thread state or merge readiness on GitHub
- human/self-review quality

Use `cargo xtask pr-preflight --full` when you want the broader local proof before a high-risk or cross-cutting PR. `make pr-preflight ARGS='--full'` remains available as a thin alias.

### PR Flight Recorder

When a review cycle starts getting noisy, capture the current state before and after a fix batch:

```sh
cargo xtask pr-snapshot
```

The recorder writes timestamped JSON + Markdown under `artifacts/pr-review/pr-<number>/`
and refreshes `latest.json` / `latest.md` for the selected PR. When a prior local snapshot
exists, the recorder also writes timestamped and `latest.delta.*` semantic delta artifacts so
you can answer "what changed since the last sortie?" without diffing raw JSON by hand.
Those artifacts are local-only and gitignored on purpose. They exist to make review-state
drift legible: head SHA, grouped checks, unresolved threads, review decision, merge state,
current blockers, and the transition between successive snapshots.

Use `make pr-snapshot ARGS='308'` if you prefer a Make alias or need to target an
explicit PR number.

---

## Dependency DAG Workflow

Artifacts:

- DOT + SVG output lives under `docs/assets/dags/`.
- The hand-maintained edge list lives in `docs/assets/dags/deps-config.json`.
- The generator is `scripts/generate-dependency-dags.js` (wrapped by `cargo xtask dags`).
- The explainer doc is `docs/dependency-dags.md`.

Automation:

- GitHub Action `Refresh Dependency DAGs` runs on a schedule and opens a PR only if outputs changed:
    - workflow file: `.github/workflows/refresh-dependency-dags.yml`
    - uses `--snapshot-label none` to avoid “date churn” diffs

Issue linkage for automation PRs:

- If you enforce strict PR↔Issue linkage, create a single tracking issue (example: “Automate dependency DAG refresh”) and set a repository Actions variable:
    - `DAG_REFRESH_ISSUE=<issue-number>`
- The workflow will include `Refs #<issue-number>` in the PR body when `DAG_REFRESH_ISSUE` is set.

If you add new issues/milestones that should appear in the graph, update `docs/assets/dags/deps-config.json` (and consider annotating edges with confidence).
