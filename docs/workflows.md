<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Workflows (Contributor Playbook)

This doc is the “official workflow index” for Echo: how we work, what invariants we enforce, and the blessed entrypoints/scripts.

> Echo is docs-driven and determinism-driven. When in doubt, prefer writing down the invariant and then encoding it as a test or guard.

---

## Session Workflow

- Start a work session by updating *Today’s Intent* in `docs/execution-plan.md`.
- During work, record decisions and blockers in `docs/decision-log.md` (canonical), and keep `docs/execution-plan.md` in sync.
- Before opening a PR, confirm the docs guard requirements below.

---

## Branch + PR Workflow

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

Docs guard (CI enforces this):
- If a PR touches **non-doc** files, it must also update:
  - `docs/execution-plan.md`
  - `docs/decision-log.md`

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

If you add new issues/milestones that should appear in the graph, update `docs/assets/dags/deps-config.json` (and consider annotating edges with confidence).
