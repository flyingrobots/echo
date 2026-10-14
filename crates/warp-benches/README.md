<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
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

## Action-path benchmarks live in `warp-core`

Two benchmarks drive a trusted host with real compare-and-set Actions:
`action_footprint_overlap` and `wal_record_cost`. They are bench targets of
`warp-core`, not of this crate, because they need dependencies this crate does
not have, and adding them would change `Cargo.lock`, which the provider
package binds. Run them with:

```sh
cargo bench -p warp-core --features native_rule_bootstrap,trusted_runtime \
  --bench action_footprint_overlap
cargo bench -p warp-core --features native_rule_bootstrap,trusted_runtime \
  --bench wal_record_cost
```

Each prints a table. The header comment of each file lists its settings.

## Documentation

- See `docs/benchmarks/` and the repository `README.md` for instructions on
  running and interpreting the benchmarks and for details on the current
  benchmark suite.
