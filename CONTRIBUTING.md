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

Echo is a deterministic WARP runtime over witnessed causal history. We
prioritize:

- **Causal integrity**: admission, transitions, frontiers, receipts, and
  witnesses remain explicit.
- **Executable evidence**: behavior and invariants are proved by focused tests,
  gates, schemas, or artifact checks.
- **Bounded interfaces**: application nouns stay in authored contracts and
  adapters; Echo core exposes generic causal and optic boundaries.

## Getting Started

1. Clone the repo and run `cargo check` to ensure the Rust workspace builds.
2. Read `docs/architecture/outline.md`.
3. Review `AGENTS.md` for architecture, Git safety, and the executable-claim loop before touching runtime code.
4. Optional: develop inside the devcontainer for toolchain parity with CI.
    - Open in VS Code → "Reopen in Container" (requires the Dev Containers extension).

- The container includes Rust 1.90.0 (via `rust-toolchain.toml`), clippy/rustfmt, Node, and gh.
- Post-create installs the pinned toolchain (no override); wasm32 target and components are added automatically.

## Branching & Workflow

- Keep `main` pristine. Create feature branches like `echo/<feature>` or `timeline/<experiment>`.
- Before starting work, ensure `git status` is clean. If not, resolve or coordinate with the human operator.
- Keep change-local design, test plans, review state, and follow-up work in the
  GitHub issue or pull request. Record only durable architectural decisions as
  ADRs in `docs/adr/`.

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
- Add unit or integration coverage that directly witnesses new logic and
  invariants.
- For WASM work, install the required target with
  `rustup target add wasm32-unknown-unknown` and use the crate-specific build
  instructions for the surface you are changing.

## Documentation & Telemetry

- Update relevant docs in `docs/` whenever behavior or architecture changes.
- Record durable architectural decisions in ADRs (`docs/adr/`). Keep
  change-local rationale in the issue or pull request.

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
- TypeScript packages live in `packages/` and `apps/`; follow local lint configs.
- Avoid non-deterministic APIs (no wall-clock, no uncontrolled randomness). Use Echo’s deterministic services.

### Git Hooks (recommended)

- Install repo hooks once: `make hooks` (configures `core.hooksPath` to `.githooks`).
- Pre-commit runs:
    - PRNG version/golden coupling checks when the PRNG changes.
    - Toolchain and lockfile-format verification.
    - Rust and Markdown formatting (`ECHO_AUTO_FMT=1` by default).
    - Targeted verification for staged Rust crates.
    - SPDX and Markdown lint checks.
- Set `ECHO_AUTO_FMT=0` for check-only formatting.

#### Partial Staging & rustfmt

- Formatters rewrite entire files, not only staged hunks. When the relevant
  working tree is clean relative to the index, the hook auto-stages formatter
  changes and continues the commit.
- If unstaged changes existed before formatting, the hook applies formatting
  but aborts rather than guessing which staged and unstaged hunks are safe to
  combine. Review `git status` and `git diff`, restage intentionally, and commit
  again.
- Tips:
    - For a partial-staged commit, format and stage the intended whole-file
      result first, or separate formatter-only changes into their own commit.
    - With `ECHO_AUTO_FMT=0`, formatting failures abort without changing files.
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
