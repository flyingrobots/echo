# Contributing to Echo

Welcome, chrononaut! Echo thrives when timelines collaborate. Please read this guide before diving in.

## Table of Contents
- [Project Philosophy](#project-philosophy)
- [Getting Started](#getting-started)
- [Branching & Workflow](#branching--workflow)
- [Testing Expectations](#testing-expectations)
- [Documentation & Telemetry](#documentation--telemetry)
- [Submitting Changes](#submitting-changes)
- [Code Style](#code-style)
- [Communication](#communication)

## Project Philosophy
Echo is a deterministic, renderer-agnostic engine. We prioritize:
- **Determinism**: every change must preserve reproducible simulations.
- **Documentation**: specs and decision logs live alongside code.
- **Temporal Tooling**: features support branching timelines and merges.

## Getting Started
1. Clone the repo and run `cargo check` to ensure the Rust workspace builds.
2. Read `docs/architecture-outline.md` and `docs/execution-plan.md`.
3. Register yourself in Neo4j via `AGENTS.md` instructions and note your display handle.

## Branching & Workflow
- Keep `main` pristine. Create feature branches like `echo/<feature>` or `timeline/<experiment>`.
- Before starting work, ensure `git status` is clean. If not, resolve or coordinate with the human operator.
- Log intent in Neo4j (`[Echo]` tag) at the start of each session.

## Testing Expectations
- Write tests before or alongside code changes.
- `cargo test` must pass locally before PR submission.
- Add unit/integration coverage for new logic; Lua/TypeScript tooling will regain coverage when reintroduced.

## Documentation & Telemetry
- Update relevant docs in `docs/` whenever behavior or architecture changes.
- Record major decisions or deviations in the execution plan or decision log tables.
- Use Neo4j to leave breadcrumbs for future Codex agents.

## Submitting Changes
1. Run `cargo fmt`, `cargo clippy`, and `cargo test`.
2. Commit with meaningful messages (no conventional prefixes; tell the story).
3. Push your branch and open a PR. Include:
   - Summary of changes and motivation.
   - Tests performed.
   - Any timeline or determinism considerations.
4. Request review from maintainers (see CODEOWNERS).

## Code Style
- Rust code must pass `cargo fmt` and `cargo clippy` without warnings.
- Lua scripts should remain deterministic (no uncontrolled globals, RNG via engine services).
- TypeScript tooling (when active) lives in `reference/typescript/`; follow local lint configs when reactivated.
- Avoid non-deterministic APIs (no wall-clock, no uncontrolled randomness). Use Echo’s deterministic services.

## Communication
- Major updates logged in Neo4j threads (`echo-devlog`, `echo-spec`).
- Use GitHub discussions or issues for larger design proposals.
- Respect the temporal theme—leave the codebase cleaner for the next timeline traveler.

Thanks for helping forge Echo’s spine. 🌀
