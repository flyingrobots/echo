# Echo Agent Briefing

Welcome to the **Echo** project. This file captures expectations for any LLM agent (and future-human collaborator) who touches the repo.

## Core Principles
- **Honor the Vision**: Echo is a deterministic, multiverse-aware ECS. Consult `docs/architecture-outline.md` before touching runtime code.
- **Document Ruthlessly**: Every meaningful design choice should land in `docs/` (specs, diagrams, memorials) or other durable repo artifacts (e.g. `docs/decision-log.md`).
- **Docstrings Aren't Optional**: Public APIs across crates (`rmg-core`, `rmg-ffi`, `rmg-wasm`, etc.) must carry rustdoc comments that explain intent, invariants, and usage. Treat missing docs as a failing test.
- **Determinism First**: Avoid introducing sources of nondeterminism without a mitigation plan.
- **Temporal Mindset**: Think in timelines—branching, merging, entropy budgets. Feature work should map to Chronos/Kairos/Aion axes where appropriate.

## Timeline Logging
- Start each session by updating *Today’s Intent* in `docs/execution-plan.md`.
- Capture milestones, blockers, and decisions directly in this repo (e.g. `docs/decision-log.md`, relevant specs, or PR descriptions).
- When wrapping up, record outcomes and next steps in the Decision Log and ensure any impacted docs stay in sync.

## Repository Layout
- `packages/echo-core`: Runtime core (ECS, scheduler, Codex’s Baby, timelines).
- `apps/playground`: Vite sandbox and inspector (future).
- `docs/`: Specs, diagrams, memorials.
- `docs/legacy`: Preserved artifacts from the Caverns era.

## Working Agreement
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
- Docs Guard: when a PR touches non‑doc code, update `docs/execution-plan.md` and `docs/decision-log.md` in the same PR.
- Project hygiene: assign the PR’s linked issue to the correct Milestone and Board column (Blocked/Ready/Done) as part of the PR.

### Git Hooks & Local CI
- Install repo hooks once with `make hooks` (configures `core.hooksPath`).
- Formatting: pre-commit auto-fixes with `cargo fmt` by default. Set `ECHO_AUTO_FMT=0` to run check-only instead.
- Toolchain: pre-commit verifies your active toolchain matches `rust-toolchain.toml`.
- Docs Guard: when core API files change, the hook requires updating `docs/execution-plan.md` and `docs/decision-log.md` (mirrors the CI check).

## Git Real
1. **NEVER** use `--force` with any git command. If you think you need it, stop and ask the human for help.
2. **NEVER** use rebase. Embrace messy distributed history; plain merges capture the truth, rebases rewrite it.
3. **NEVER** amend a commit. Make a new commit instead of erasing recorded history.

In short: no one cares about a tidy commit graph, but everyone cares if you rewrite commits on origin.

## Contact Threads
- Docs `decision-log.md`: Chronological design decisions.
- Docs `execution-plan.md`: Working map of tasks, intent, and progress.

Safe travels in the multiverse. Logged timelines are happy timelines. 🌀
