<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# PLATFORM

Tooling and infrastructure.

## What it covers

WASM compilation, `cargo xtask` CLI, CI pipelines, benchmarking
infrastructure, content-addressed storage (echo-cas), Wesley schema
integration, developer experience.

## What success looks like

A developer can clone the repo, run `cargo xtask`, and have a working
development loop. CI catches regressions before merge. Benchmarks are
automated and honest. WASM builds are reproducible.

## How you know

- `cargo xtask` subcommands cover common workflows.
- GitHub Actions CI passes on every PR.
- Perf baselines are auto-generated and reviewed.
- WASM build reproduces the same binary hash.
