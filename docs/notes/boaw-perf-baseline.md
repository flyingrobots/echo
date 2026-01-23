<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BOAW Performance Baseline

**Date:** 2026-01-20
**Phase:** 6B (Sharded Parallel Execution)
**Benchmark:** `cargo +nightly bench --package warp-benches --bench boaw_baseline`

---

## Environment

| Component | Value                                                                                            |
| --------- | ------------------------------------------------------------------------------------------------ |
| **CPU**   | Apple M1 Pro (arm64)                                                                             |
| **Rust**  | nightly (required for `#[bench]`-less Criterion; run `rustc +nightly --version` for exact build) |
| **OS**    | macOS 24.3.0 (Darwin)                                                                            |
| **Cores** | 10 (8 performance + 2 efficiency)                                                                |

---

## Baseline Numbers

### Serial vs Parallel (4 workers)

| Workload   | Serial     | Parallel (4w) | Ratio       |
| ---------- | ---------- | ------------- | ----------- |
| 10 items   | 1,187 ns   | 65,433 ns     | 55x slower  |
| 100 items  | 10,241 ns  | 75,158 ns     | 7.3x slower |
| 1000 items | 100,734 ns | 133,849 ns    | 1.3x slower |

### Worker Scaling (100 items)

| Workers | Time (ns) | vs Serial    |
| ------- | --------- | ------------ |
| Serial  | 10,241    | 1.0x         |
| 1       | 35,805    | 3.5x slower  |
| 2       | 49,668    | 4.8x slower  |
| 4       | 74,803    | 7.3x slower  |
| 8       | 126,711   | 12.4x slower |
| 16      | 235,094   | 23x slower   |

### Large Workload Scaling (1000 items)

| Workers | Time (ns) | vs Serial   |
| ------- | --------- | ----------- |
| Serial  | 100,734   | 1.0x        |
| 4       | 133,849   | 1.3x slower |
| 8       | 184,301   | 1.8x slower |
| 16      | 296,992   | 2.9x slower |

> **Note:** The measurements above are point estimates from Criterion. Full
> CI/variance data (including confidence intervals and outlier classification)
> is available in the raw Criterion output directory (`target/criterion/`).

---

## Interpretation

### Why Serial Wins

The benchmark uses a **trivial executor** (`touch_executor`) that performs a single
`SetAttachment` operation. This takes ~100ns per item. Thread spawn overhead dominates:

- `std::thread::scope()` setup: ~30,000-60,000 ns
- Per-worker thread spawn: ~5,000-10,000 ns each
- Synchronization overhead: ~5,000 ns

For a 10-item workload (1,187 ns serial), the parallel version spends 98% of its time
on thread management.

### When Parallel Will Help

Parallelism wins when:

1. **Executor cost >> thread overhead**: Real rules with graph traversals, complex
   pattern matching, or attachment serialization will benefit more
2. **Large workloads**: At 1000+ items, we're approaching break-even even with trivial
   executors
3. **Per-warp parallelism**: The engine groups rewrites by warp, so cross-warp work
   stays serial while intra-warp work can parallelize

### Baseline Purpose

This baseline captures the **overhead floor** of the parallel execution system. Future
phases should not regress beyond these numbers. If parallel execution becomes slower
than these baselines, investigate:

- Thread pool overhead increases
- Lock contention in merge
- Shard distribution imbalance

---

## Perf Gate Thresholds

Use these thresholds for CI perf gates:

| Metric                  | Baseline   | Gate (fail if slower than) |
| ----------------------- | ---------- | -------------------------- |
| serial_1000             | 100,734 ns | 200,000 ns (2x)            |
| parallel_1000_workers_4 | 133,849 ns | 270,000 ns (2x)            |
| worker_scaling_100_w4   | 74,803 ns  | 150,000 ns (2x)            |

---

## Re-running Benchmarks

```sh
# Requires nightly Rust for Criterion benchmarks
cargo +nightly bench --package warp-benches --bench boaw_baseline
```

To compare against baseline, use Criterion's built-in comparison. Run the
benchmark twice (it stores history in `target/criterion/`) and Criterion will
report regressions/improvements automatically. For machine-readable output,
use `--message-format=json` or inspect the JSON files in `target/criterion/`.
