<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Benches (warp-benches)

This crate hosts Criterion microbenchmarks for Echo’s Rust core (`warp-core`).

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

- `parallel_baseline.rs`
    - Compares serial execution, the current shard-parallel baseline, the Phase 6B
      work-queue pipeline, worker-count scaling, and the shard-policy matrix.
    - The policy matrix compares:
        - dynamic shard claiming + per-worker deltas
        - dynamic shard claiming + per-shard deltas
        - static round-robin shard assignment + per-worker deltas
        - static round-robin shard assignment + per-shard deltas
        - dedicated one-worker-per-shard + one-delta-per-shard
    - Each case includes canonical delta merge after parallel execution, so the
      study reflects full policy cost for the synthetic independent workload.
    - The policy matrix runs across loads `100`, `1000`, and `10000`, with worker
      counts `1`, `4`, and `8` where the policy uses a worker pool.
    - Throughput “elements” = executed items in the synthetic independent workload.

## Run

Run the full benches suite:

```sh
cargo bench -p warp-benches
```

Run a single bench target (faster dev loop):

```sh
cargo bench -p warp-benches --bench snapshot_hash
cargo bench -p warp-benches --bench scheduler_drain
cargo bench -p warp-benches --bench parallel_baseline
```

Criterion HTML reports are written under `target/criterion/<group>/report/index.html`.

### Charts & Reports

- Live server + dashboard: `make bench-report` opens `http://localhost:8000/docs/benchmarks/`.
- Offline static report (no server): `make bench-bake` writes `docs/benchmarks/report-inline.html` with results, policy payload, and provenance injected.
    - Open the file directly (Finder or `open docs/benchmarks/report-inline.html`).
- The same static page also hosts the parallel shard-policy study.
    - Run `make bench-policy-bake`, then open the `Parallel policy matrix` tab.
    - `make bench-policy-export` rebakes from the existing local Criterion tree without rerunning benches.

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

```sh
cargo flamegraph -p warp-benches --bench snapshot_hash -- --sample-size 50
```

These tools are not required for CI and are optional for local analysis.
