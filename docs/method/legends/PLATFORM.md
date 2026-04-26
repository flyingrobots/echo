<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# PLATFORM — Tooling and Infrastructure

_Legend for everything that surrounds the kernel: build tooling,
developer experience, CI, WASM, storage, and schema integration._

## Goal

Make the development loop fast, the CI honest, and the deployment
pipeline reproducible. A developer should be able to clone, build,
test, and ship without tribal knowledge.

This legend covers work like:

- `cargo xtask` CLI and METHOD subcommands
- WASM compilation and browser runtime
- CI pipelines and pre-commit hooks
- benchmarking infrastructure and perf baselines
- content-addressed storage (echo-cas)
- Wesley schema integration and codegen
- developer onboarding and local setup

## Human users

- James, maintaining the build and release pipeline
- future contributors who need to clone and build on day one
- CI systems running automated gates

## Agent users

- agents running xtask commands to assess repo state
- agents generating or validating benchmark artifacts
- agents scaffolding new crates or test fixtures

## Human hill

A human can clone the repo, run `cargo xtask` and `cargo test`, and
have a working development loop within minutes — no undocumented
setup steps, no missing tools, no tribal knowledge.

## Agent hill

An agent can run `cargo xtask method status` and `cargo test` to
programmatically determine the project state, what's passing, what's
failing, and what to work on next.

## Core invariants

- WASM builds are reproducible (same source → same binary hash).
- CI catches regressions before merge.
- Perf baselines are auto-generated and reviewed.
- No secrets in the repo; Vault for secrets management.
- Pre-commit hooks enforce lint and format with zero warnings.

## Current cycle and backlog

- active cycle:
    - `0002-xtask-method-status` (in design)
- live backlog:
    - `asap/PLATFORM_benchmarks-cleanup.md`
    - `asap/PLATFORM_cli-bench.md`
    - `asap/PLATFORM_cli-inspect.md`
    - `asap/PLATFORM_cli-scaffold.md`
    - `asap/PLATFORM_cli-verify.md`
    - `asap/PLATFORM_xtask-method-close.md`
    - `asap/PLATFORM_xtask-method-drift.md`
    - `asap/PLATFORM_xtask-method-pull.md`
