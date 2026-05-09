<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contributing to Echo

Welcome, chrononaut! Echo thrives when timelines collaborate. Please read this guide before diving in.

## Table of Contents

- [Contributing to Echo](#contributing-to-echo)
    - [Table of Contents](#table-of-contents)
    - [Project Philosophy](#project-philosophy)
    - [Getting Started](#getting-started)
    - [Branching \& Workflow](#branching--workflow)
    - [Testing Expectations](#testing-expectations)
    - [Documentation \& Telemetry](#documentation--telemetry)
    - [Submitting Changes](#submitting-changes)
    - [Code Style](#code-style)
        - [Git Hooks (recommended)](#git-hooks-recommended)
            - [Partial Staging \& rustfmt](#partial-staging--rustfmt)
    - [Communication](#communication)

## Project Philosophy

Echo is a deterministic, renderer-agnostic engine. We prioritize:

- **Determinism**: every change must preserve reproducible simulations.
- **Documentation**: specs live alongside code.
- **Temporal Tooling**: features support branching timelines and merges.

## Getting Started

1. Clone the repo and run `cargo check` to ensure the Rust workspace builds.
2. Read `docs/architecture/outline.md`.
3. Review `docs/AGENTS.md` for collaboration norms before touching runtime code.
4. Optional: develop inside the devcontainer for toolchain parity with CI.
    - Open in VS Code → "Reopen in Container" (requires the Dev Containers extension).

- The container includes Rust 1.90.0 (via `rust-toolchain.toml`), clippy/rustfmt, Node, and gh.
- Post-create installs the pinned toolchain (no override); wasm32 target and components are added automatically.

## Branching & Workflow

- Keep `main` pristine. Create feature branches like `echo/<feature>` or `timeline/<experiment>`.
- Before starting work, ensure `git status` is clean. If not, resolve or coordinate with the human operator.
- PR review loops are procedural: follow `docs/procedures/PR-SUBMISSION-REVIEW-LOOP.md` and use `docs/procedures/EXTRACT-PR-COMMENTS.md` to extract actionable CodeRabbitAI feedback per round.

## Testing Expectations

- Write tests before or alongside code changes.
- Use narrow local slices while iterating. For example:
    - `cargo xtask test-slice strand`
    - `cargo xtask test-slice settlement`
    - `cargo xtask test-slice observation`
    - `cargo xtask test-slice neighborhood`
    - `cargo xtask test-slice warp-core-smoke`
- Avoid broad filtered commands such as `cargo test -p warp-core settlement`
  during normal development. Cargo still compiles and launches every
  `warp-core` integration-test binary before applying that runtime filter.
- The broader local gate must pass before PR submission.
- Add unit/integration coverage for new logic; Rhai/TypeScript tooling will regain coverage when reintroduced.
- For WASM work, install the required target with
  `rustup target add wasm32-unknown-unknown` and use the crate-specific build
  instructions for the surface you are changing.

## Documentation & Telemetry

- Update relevant docs in `docs/` whenever behavior or architecture changes.
- Record major architectural decisions in ADRs (`docs/adr/`) or PR descriptions.

## Submitting Changes

1. Run `cargo fmt`, focused `cargo xtask test-slice …` checks for the changed surface, and the appropriate broader local gate (`cargo xtask pr-preflight` before PRs).
2. Commit with conventional commit messages: `type(scope): summary` (e.g., `fix(warp-core): prevent NaN propagation`).
3. Push your branch and open a PR. Include:
    - Summary of changes and motivation.
    - Tests performed.
    - Any timeline or determinism considerations.
4. Request review from maintainers (see CODEOWNERS).

## Code Style

- Rust code must pass `cargo fmt` and `cargo clippy` without warnings.
- Rhai scripts should remain deterministic (no uncontrolled globals, RNG via engine services).
- TypeScript packages live in `packages/` and `apps/`; follow local lint configs.
- Avoid non-deterministic APIs (no wall-clock, no uncontrolled randomness). Use Echo’s deterministic services.

### Git Hooks (recommended)

- Install repo hooks once: `make hooks` (configures `core.hooksPath` to `.githooks`).
- Pre-commit runs:
    - cargo fmt (auto-fix by default; set `ECHO_AUTO_FMT=0` for check-only)
    - Toolchain pin verification (matches `rust-toolchain.toml`)
- To auto-fix formatting on commit: `ECHO_AUTO_FMT=1 git commit -m "message"`

#### Partial Staging & rustfmt

- rustfmt formats entire files, not only staged hunks. To preserve index integrity, our pre-commit hook now aborts the commit if running `cargo fmt` would change any files. It first checks with `cargo fmt --check`, and if changes are needed it applies them and exits with a helpful message.
- Workflow when this happens:
    1. Review formatting changes: `git status` and `git diff`.
    2. Restage intentionally formatted files (e.g., `git add -A` or `git add -p`).
    3. Commit again.
- Tips:
    - If you need to keep a partial-staged commit, do two commits: first commit the formatter-only changes, then commit your code changes.
    - You can switch to check-only with `ECHO_AUTO_FMT=0` (commit will still fail on formatting issues, but nothing is auto-applied).
- Do not bypass hooks. The repo runs fmt, clippy, tests, and rustdoc on the pinned toolchain before push.
- Toolchain: pinned to Rust 1.90.0. Ensure your local override matches:
    - rustup toolchain install 1.90.0
    - rustup override set 1.90.0

### Shared Workspace Settings

- The repo tracks a minimal [.vscode/settings.json](.vscode/settings.json) for project-safe tooling settings only.
- Keep personal editor preferences such as theme, font family, and UI layout in your user-level VS Code settings, not the tracked workspace file.
- The tracked Rust Analyzer target dir uses the repo-local ignored `target-ra/` path to avoid fighting the default Cargo build directory during background checks.

## Communication

- Rely on GitHub discussions or issues for longer-form proposals.
- Respect the temporal theme—leave the codebase cleaner for the next timeline traveler.

Thanks for helping forge Echo’s spine. 🌀
