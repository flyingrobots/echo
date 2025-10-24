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
1. Clone the repo and run `pnpm install`.
2. Read `docs/echo/architecture-outline.md` and `docs/echo/execution-plan.md`.
3. Register yourself in Neo4j via `AGENTS.md` instructions and note your display handle.

## Branching & Workflow
- Keep `main` pristine. Create feature branches like `echo/<feature>` or `timeline/<experiment>`.
- Before starting work, ensure `git status` is clean. If not, resolve or coordinate with the human operator.
- Log intent in Neo4j (`[Echo]` tag) at the start of each session.

## Testing Expectations
- Write tests before or alongside code changes.
- `pnpm test` must pass locally before PR submission.
- Add Vitest coverage for new logic; integration tests will live under `apps/playground` when ready.

## Documentation & Telemetry
- Update relevant docs in `docs/echo/` whenever behavior or architecture changes.
- Record major decisions or deviations in the execution plan or decision log tables.
- Use Neo4j to leave breadcrumbs for future Codex agents.

## Submitting Changes
1. Run `pnpm lint` and `pnpm test`.
2. Commit with meaningful messages (no conventional prefixes; tell the story).
3. Push your branch and open a PR. Include:
   - Summary of changes and motivation.
   - Tests performed.
   - Any timeline or determinism considerations.
4. Request review from maintainers (see CODEOWNERS).

## Code Style
- TypeScript + ESLint + Prettier (config provided).
- Prefer explicit types when clarity improves comprehension.
- Avoid non-deterministic APIs (no `Math.random`, `Date.now`, etc.). Use Echo’s math/PRNG services.

## Communication
- Major updates logged in Neo4j threads (`echo-devlog`, `echo-spec`).
- Use GitHub discussions or issues for larger design proposals.
- Respect the temporal theme—leave the codebase cleaner for the next timeline traveler.

Thanks for helping forge Echo’s spine. 🌀
