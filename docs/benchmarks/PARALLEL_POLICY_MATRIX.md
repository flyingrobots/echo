<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Parallel Policy Matrix Benchmark

## Purpose

This benchmark compares shard execution topology choices, not just raw worker
count:

- dynamic shard claiming + one delta per worker
- dynamic shard claiming + one delta per shard
- static round-robin shard assignment + one delta per worker
- static round-robin shard assignment + one delta per shard
- dedicated one-worker-per-shard + one delta per shard
- adaptive shard routing, which selects one of the pooled-worker policies from workload shape

The point is to answer a narrower question than "is parallel good?":

- which shard assignment policy is cheaper,
- which delta grouping policy is cheaper, and
- whether "one worker = one shard = one delta" is ever worth the overhead.

The harness includes canonical delta merge after parallel execution, so the
study measures the full policy cost visible to the engine for these synthetic
independent workloads, not just executor-stage delta production.

## Loads

The benchmark currently runs at:

- `100`
- `1000`
- `10000`

For the fixed dynamic/static policies, it also varies concrete worker counts:

- `1`
- `4`
- `8`

The dedicated per-shard policy intentionally ignores the worker-count knob and
spawns one thread per non-empty shard.

For the adaptive selector, the report preserves both:

- the incoming worker hint used to seed the heuristic, and
- the fixed policy/worker plan the selector actually chose for that
  workload/hint pair

That keeps the baked HTML and JSON honest when multiple hints collapse to the
same concrete plan.

## Outputs

Running the dedicated bake target produces:

- raw JSON with provenance metadata:
  [parallel-policy-matrix.json](/Users/james/git/echo/docs/benchmarks/parallel-policy-matrix.json)
- unified static benchmarks page:
  [report-inline.html](/Users/james/git/echo/docs/benchmarks/report-inline.html)
  Open the `Parallel policy matrix` tab.

Criterion's original raw estimates remain under `target/criterion/parallel_policy_matrix/`.

## Commands

Run the targeted policy study and bake outputs:

```sh
make bench-policy-bake
```

If benchmark results already exist and you only want to regenerate JSON + HTML:

```sh
make bench-policy-export
```

The export payload includes:

- generated timestamp
- git SHA
- machine descriptor
- criterion source root

To inspect the registered benchmark cases without running them:

```sh
cargo bench -p warp-benches --bench parallel_baseline -- --list
```

## Notes

- The benchmark measures execution topology overhead on a synthetic independent
  workload. It is not a substitute for end-to-end engine traces.
- The page provenance records when and where the artifact was baked from the
  local Criterion tree. It does **not** claim to know the original commit that
  produced those raw Criterion estimates if you rebake from pre-existing data.
- The dedicated per-shard policy is primarily a comparison tool. It is expected
  to pay substantial thread-spawn overhead and exists to bound the extreme
  `1 worker = 1 shard = 1 delta` shape against the pooled-worker policies.
