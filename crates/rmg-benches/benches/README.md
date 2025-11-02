# Echo Benches (rmg-benches)

This crate hosts Criterion microbenchmarks for Echo’s Rust core (`rmg-core`).

Benchmarks are executable documentation of performance. Each bench includes
module-level docs describing what is measured, why, and how to interpret
results. This README summarizes how to run them and read the output.

## What’s Here

- `snapshot_hash.rs`
  - Builds a linear chain of `n` entities reachable from `root` and measures
    the snapshot (state_root) hash of the reachable subgraph.
  - Throughput “elements” = nodes in the reachable set (`n` entities + 1 root).
  - Sizes: `10`, `100`, `1000` to show order-of-magnitude scaling without long
    runtimes.

- `scheduler_drain.rs`
  - Registers a trivial no-op rule and applies it to `n` entity nodes within a
    transaction to focus on scheduler overhead (not executor work).
  - Throughput “elements” = rule applications (`n`). Uses `BatchSize::PerIteration`
    so engine construction is excluded from timing.

## Run

Run the full benches suite:

```
cargo bench -p rmg-benches
```

Run a single bench target (faster dev loop):

```
cargo bench -p rmg-benches --bench snapshot_hash
cargo bench -p rmg-benches --bench scheduler_drain
```

Criterion HTML reports are written under `target/criterion/<group>/report/index.html`.

## Interpreting Results

- Use the throughput value to sanity‑check the scale of work per iteration.
- The primary signal is `time/iter` across inputs (e.g., 10 vs 100 vs 1000).
- For regressions, compare runs in `target/criterion` or host an artifact in CI
  (planned for PR‑14/15) and gate on percent deltas.

## Environment Notes

- Toolchain: `stable` Rust (see `rust-toolchain.toml`).
- Dependency policy: avoid wildcards; benches use an exact patch pin for `blake3`
  with trimmed features to avoid incidental parallelism:
  `blake3 = { version = "=1.8.2", default-features = false, features = ["std"] }`.
- Repro: keep your machine under minimal background load; prefer `--quiet` and 
  close other apps.

## Flamegraphs (optional)

If you have [`inferno`](https://github.com/jonhoo/inferno) or `cargo-flamegraph`
installed, you can profile a bench locally. Example (may require sudo on Linux):

```
cargo flamegraph -p rmg-benches --bench snapshot_hash -- --sample-size 50
```

These tools are not required for CI and are optional for local analysis.
