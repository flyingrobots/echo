<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# KERNEL

Core simulation engine.

## What it covers

WARP graph rewrites, deterministic scheduling, tick patches, parallel
execution, canonical commit, worldline runtime model, provenance.

## What success looks like

Every mutation is deterministic, reproducible, and witnessed by tests.
The scheduler drains rewrites in canonical order. Parallel execution
produces identical results to serial execution.

## How you know

- `cargo test` in `warp-core` passes.
- Golden vectors lock deterministic output across platforms.
- DIND (Deterministic Ironclad Nightmare Drills) find no divergence.
- Benchmark regressions are caught by perf baselines.
