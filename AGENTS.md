<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Agent Briefing

Welcome to the **Echo** project. This file captures expectations for any LLM agent (and future-human collaborator) who touches the repo.

## Core Principles

- **Honor the Vision**: Echo is a deterministic, multiverse-aware ECS. Consult `docs/architecture-outline.md` before touching runtime code.
- **Document Ruthlessly**: Every meaningful design choice should land in `docs/` (specs, diagrams, ADRs) or PR descriptions.
- **Docstrings Aren't Optional**: Public APIs across crates (`warp-core`, `warp-ffi`, `warp-wasm`, etc.) must carry rustdoc comments that explain intent, invariants, and usage. Treat missing docs as a failing test.
- **Determinism First**: Avoid introducing sources of nondeterminism without a mitigation plan.
- **Temporal Mindset**: Think in timelines—branching, merging, entropy budgets. Feature work should map to Chronos/Kairos/Aion axes where appropriate.

## Timeline Logging

- Capture milestones, blockers, and decisions in relevant specs, ADRs, or PR descriptions.
- AGENTS.md and `TASKS-DAG.md` are append-only; see `docs/append-only-invariants.md` plus `scripts/check-append-only.js` for the enforcement plan that CI will run before merges.

## Workflows & Automation

- The contributor playbook lives in `docs/workflows.md` (policy + blessed commands + automation).
- Preferred repo maintenance entrypoint is `cargo xtask …` (see `xtask/` and `.cargo/config.toml`).
- Planning DAG artifacts live in `docs/assets/dags/` and are documented in `docs/dependency-dags.md`.
- For automated DAG refresh PRs, set `DAG_REFRESH_ISSUE=<issue-number>` as a GitHub Actions variable so the bot PR body includes `Refs #…`.

## Repository Layout

- `packages/echo-core`: Runtime core (ECS, scheduler, Codex's Baby, timelines).
- `apps/playground`: Vite sandbox and inspector (future).
- `docs/`: Specs, diagrams, memorials.
- `docs/notes`: Working notes and explorations (non-authoritative).

## Working Agreement

- **Isolated Branches**: Every new task, feature, or bugfix **MUST** begin on a fresh, isolated branch based on the latest `main` (unless context explicitly dictates otherwise). Never mix unrelated objectives on the same branch.
- Keep `main` pristine. Feature work belongs on branches named `echo/<feature>` or `timeline/<experiment>`.
- Tests and benchmarks are mandatory for runtime changes once the harness exists.
- Respect determinism: preferably no random seeds without going through the Echo PRNG.
- Run `cargo clippy --all-targets -- -D missing_docs` and `cargo test` before every PR; CI will expect a zero-warning, fully documented surface.

### PRs & Issues (Linkage Policy)

- Every PR must be tied to a GitHub Issue.
  - If no suitable issue exists, open one before you open the PR.
  - Use explicit closing keywords in the PR body: include a line like `Closes #<issue-number>` so the issue auto‑closes on merge.
  - Keep PRs single‑purpose: 1 PR = 1 thing. Avoid bundling unrelated changes.
- Branch naming: prefer `echo/<short-feature-name>` or `timeline/<experiment>` and include the issue number in the PR title.
- Project hygiene: assign the PR's linked issue to the correct Milestone and Board column (Blocked/Ready/Done) as part of the PR.

### Git Hooks & Local CI

- Install repo hooks once with `make hooks` (configures `core.hooksPath`).
- Formatting: pre-commit auto-fixes with `cargo fmt` by default. Set `ECHO_AUTO_FMT=0` to run check-only instead.
- Toolchain: pre-commit verifies your active toolchain matches `rust-toolchain.toml`.
- SPDX header policy (source): every source file must start with exactly:
  - `// SPDX-License-Identifier: Apache-2.0`
  - `// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>`
  Use the repository scripts/hooks; do not add dual-license headers to code.

## Git Real

1. **NEVER** use `--force` with any git command. If you think you need it, stop and ask the human for help.
2. **NEVER** use rebase. Embrace messy distributed history; plain merges capture the truth, rebases rewrite it.
3. **NEVER** amend a commit. Make a new commit instead of erasing recorded history.

In short: no one cares about a tidy commit graph, but everyone cares if you rewrite commits on origin.

Safe travels in the multiverse.
