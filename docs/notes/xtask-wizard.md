<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# xtask “workday wizard” — concept note

Goal: a human-friendly `cargo xtask` (or `just`/`make` alias) that walks a contributor through starting and ending a work session, with automation hooks for branches, PRs, issues, and planning.

## Core flow

### Start session
- Prompt for intent/issue: pick from open GitHub issues (via gh CLI) or free text → writes to `docs/execution-plan.md` Today’s Intent and opens a draft entry in `docs/decision-log.md`.
- Branch helper: suggest branch name (`echo/<issue>-<slug>`), create and checkout if approved.
- Env checks: toolchain match, hooks installed (`make hooks`), `cargo fmt -- --check`/`clippy` optional preflight.

### During session
- Task DAG helper: load tasks from issue body / local `tasks.yaml`; compute simple priority/topo order (dependencies, P1/P0 tags).
- Bench/test shortcuts: menu to run common commands (clippy, cargo test -p rmg-core, bench targets).
- Docs guard assist: if runtime code touched, remind to update execution-plan + decision-log; offer to append templated entries.

### End session
- Summarize changes: gather `git status`, staged/untracked hints; prompt for decision-log entry (Context/Decision/Rationale/Consequence).
- PR prep: prompt for PR title/body template (with issue closing keywords); optionally run `git commit` and `gh pr create`.
- Issue hygiene: assign milestone/board/labels via gh CLI; auto-link PR to issue.

## Nice-to-haves
- Determinism check shortcut: run twin-engine sandbox determinism A/B (radix vs legacy) and summarize.
- Planner math: simple critical path/priority scoring across tasks.yaml; suggest next task when current is blocked.
- Cache hints: detect heavy commands run recently, skip/confirm rerun.
- Telemetry: write a small JSON session record for later blog/mining (start/end time, commands run, tests status).

## Tech sketch
- Implement under `xtask` crate in workspace; expose `cargo xtask wizard`.
- Use `dialoguer`/`inquire` for prompts; `serde_yaml/json` for tasks; `gh` CLI for GitHub ops (fallback to no-op if missing).
- Config file (`.echo/xtask.toml`) for defaults (branch prefix, issue labels, PR template path).

## Open questions
- How much is automated vs. suggested (avoid surprising commits)?
- Should Docs Guard be enforced via wizard or still via hooks?
- Where to store per-session summaries (keep in git via decision-log or external log)?

## Next steps
- Prototype a minimal “start session” + “end session” flow with `gh` optional.
- Add a `tasks.yaml` example and priority/topo helper.
- Wire into make/just: `make wizard` → `cargo xtask wizard`.
