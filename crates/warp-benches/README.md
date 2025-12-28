<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `warp-benches`

Criterion-based microbenchmarks for Echo’s engine.

## What this crate does

- Hosts performance microbenchmarks for `warp-core`, including:
  - snapshot hashing throughput,
  - scheduler drain throughput under different graph shapes,
  - adversarial scheduler scenarios.
- Produces HTML reports (via Criterion) to help evaluate engine changes and
  detect regressions over time.

## Documentation

- See `docs/benchmarks/` and the repository `README.md` for instructions on
  running and interpreting the benchmarks and for details on the current
  benchmark suite.
