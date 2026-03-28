<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Parallel Merge & Footprint Optimization Design Review

- **Status:** Review complete; no implementation approved yet
- **Date:** 2026-03-28
- **Idea Note:** [Parallel Merge & Footprint Scheduling Optimizations](parallel-merge-and-footprint-optimizations.md)

## Purpose

The earlier optimization note records two attractive ideas:

1. replace parallel delta flatten-and-sort with a k-way merge, and
2. skip footprint checks for cross-shard rewrites.

This document answers a narrower question:

- what is actually true in Echo today,
- what would have to be proven before either optimization is safe,
- which idea is still worth investigating, and
- which idea should be treated as suspect until stronger evidence exists.

## Executive Summary

1. The **k-way merge** idea is still plausible, but the current note overstates
   why it works. The current executor returns **per-worker unsorted deltas**,
   not per-shard canonical runs, so the required sorted-run invariant is not
   yet established.
2. The **shard-aware footprint skip** idea is much weaker against the current
   implementation. The default scheduler is already the `GenSet`-based
   `RadixScheduler`, whose reservation path scales like `O(m)` in candidate
   footprint size rather than `O(k×m)` in the number of previously admitted
   rewrites.
3. The cross-shard independence claim is **not currently proven**. Shard routing
   is by the scope node's `NodeId`, while footprint conflicts are checked over
   warp-scoped nodes, edges, attachments, and ports. Those are not the same
   keyspace, and current runtime enforcement does not prove they always align.
4. Recommendation:
    - keep investigating k-way merge, but only behind an explicit sorted-run
      proof obligation and benchmark plan
    - do **not** implement cross-shard footprint skipping until a stronger
      locality invariant is proven

## Current Code Reality

### Parallel delta merge

Today the merge path is:

1. execute work in parallel
2. collect one `TickDelta` per worker
3. flatten all worker deltas
4. sort globally by `(WarpOpKey, OpOrigin)`

Relevant code:

- `execute_parallel_sharded()` returns one `TickDelta` per worker in
  `crates/warp-core/src/parallel/exec.rs`
- `merge_deltas()` in `crates/warp-core/src/parallel/merge.rs` flattens all
  worker outputs and sorts the combined vector
- `TickDelta::into_parts_unsorted()` in `crates/warp-core/src/tick_delta.rs`
  explicitly exposes unsorted emission order

So the current implementation does **not** already materialize the
"per-shard pre-sorted run" structure that the idea note assumes.

### Scheduler complexity

The default scheduler is the `RadixScheduler`, not the legacy
`Vec<Footprint>` frontier scan.

- `RadixScheduler::reserve()` uses generation-stamped sets (`GenSet`) for
  membership checks in `crates/warp-core/src/scheduler.rs`
- `docs/scheduler-warp-core.md` already documents the default path as `O(m)`,
  where `m` is the candidate footprint size
- the legacy pairwise frontier scan still exists as `LegacyScheduler`, but it
  is not the default hot path

This matters because the shard-aware footprint idea primarily helps pairwise
all-frontier overlap checks. That is no longer the main scheduler algorithm.

### Footprint keys vs shard keys

Shard routing and footprint conflict detection are based on different data:

- shard routing uses the scoped node's `NodeId` low bits in
  `crates/warp-core/src/parallel/shard.rs`
- footprint conflicts are checked over warp-scoped nodes, edges,
  attachments, and ports in `crates/warp-core/src/footprint.rs` and
  `crates/warp-core/src/scheduler.rs`

Current runtime enforcement proves footprints are **warp-local**, not
**shard-local**:

- `FootprintGuard::new()` in `crates/warp-core/src/footprint_guard.rs`
  asserts against cross-warp entries
- it does not assert that every touched slot maps to the same shard as
  `scope(r)`

That distinction is exactly why the cross-shard skip needs a proof instead of a
performance intuition.

## 1. K-Way Merge Assessment

### What would have to be true

For a k-way merge to be a correct replacement for the current global sort, we
need a family of runs `R1..Rk` such that:

- each `Ri` is already sorted by the exact canonical order
  `(WarpOpKey, OpOrigin)`, and
- the current merged output is
  `sort(flatten(R1..Rk))`

Under those conditions, a standard heap-based merge is correct:

```text
kway_merge(R1..Rk) == sort(flatten(R1..Rk))
```

This is the real proof obligation. The current note skips straight to the
conclusion without establishing that the merge inputs satisfy the premise.

### What is true today

What we actually have today is weaker:

- the executor returns one `TickDelta` per worker, not per shard
- workers may process many shards
- `TickDelta` collects operations in emission order, not canonical order
- canonical ordering is imposed later by `merge_deltas()`

So "shard assignment exists" does **not** imply "merge inputs are sorted runs."

### What about the 1-core / 1-worker case?

That case is important because it exposes the missing invariant cleanly.

If `k = 1`:

- a k-way merge only helps if the single input run is already sorted by the
  canonical key
- otherwise we still need to sort, and the optimization collapses back into a
  normal sort path

So the "what if 1 shard because 1 CPU core" question has a direct answer:

- if the single worker delta is unsorted, the k-way merge idea provides no
  algorithmic win
- if the single worker delta is already canonically sorted, then the merge is
  effectively a linear pass, but that is a stronger invariant than the current
  implementation documents

### Recommendation

The k-way merge idea remains worth investigating, but only in this order:

1. decide whether Echo should produce **per-shard canonical runs** or
   **per-worker canonical runs**
2. prove or enforce that each run is already sorted by `(WarpOpKey, OpOrigin)`
3. benchmark:
    - current flatten-and-sort
    - sort-each-run-plus-merge
    - true pre-sorted k-way merge
4. only keep the optimization if the canonical-output equality is explicit and
   the benchmark win survives review

## 2. Shard-Aware Footprint Skip Assessment

### The claimed invariant

The idea note assumes:

```text
shard(r1) != shard(r2) => footprint(r1) and footprint(r2) are disjoint
```

That is the key claim. Without it, skipping cross-shard overlap checks is not
conservative, and the optimization is unsound.

### Why the claim is not currently established

The current code only guarantees:

- the rewrite has a scoped node
- shard routing is a deterministic function of that scope node
- footprint slots are warp-scoped
- footprint guards reject cross-warp entries

It does **not** currently prove:

- every node touched by the rewrite hashes to the same shard as the scope node
- every edge touched by the rewrite hashes to that same shard
- every attachment touched by the rewrite belongs to resources in that same shard
- every boundary port touched by the rewrite belongs to that same shard

That is enough to reject the note's current "structurally disjoint by
construction" wording.

### Could there still be overlapping footprints?

Yes. Unless we prove a stronger locality invariant, the answer is plainly yes.

The dangerous pattern is:

```text
rewrite r has scope node A
shard(scope(A)) = s1
rewrite body touches some other slot X
shard(slot(X)) = s2
```

If `X` can differ from the scope shard, then two rewrites can land in different
scope shards and still overlap through some footprinted slot.

That possibility is enough to block the optimization until the invariant is
settled.

### Does the "1 / shard_count" math still matter?

Only for a pairwise overlap algorithm.

If we were still using the old pairwise frontier scan, then under a uniform
distribution:

- probability of same-shard pair: `1 / S`
- expected candidate pairs surviving the shard gate:
  `C(N, 2) / S`

That math is fine as a performance estimate for the **legacy** pairwise model.

But the current default scheduler is already `O(m)` with `GenSet`s, not
`O(k×m)` over frontier size, so there is no honest shard-count crossover to
claim against today's default path without fresh benchmarks.

### What about the bloom-filter idea?

The bloom-style same-shard prefilter is the least controversial part.

If two footprints share a real slot and the filter is built from those slots,
then they must share at least one set bit. Therefore:

- `filter_a & filter_b == 0` implies no shared slot represented in the filter
- false positives are possible
- false negatives are not acceptable

So the same-shard prefilter is conceptually fine as a conservative
implementation detail. The problem is the earlier step: the current note has
not earned the right to skip **cross-shard** checks yet.

## Proof Obligations

### K-Way Merge Benchmarks

Before implementation, prove or enforce:

1. every merge input run is sorted by `(WarpOpKey, OpOrigin)`
2. the k-way merge produces byte-for-byte identical canonical output to the
   current flatten-and-sort path
3. dedupe and conflict detection semantics are unchanged

Acceptable proof styles:

- a direct design proof over sorted runs
- property tests comparing `kway_merge` against `sort(flatten(...))`
- deterministic regression tests across worker counts and shard layouts

### Cross-shard footprint skip

Before implementation, prove:

```text
for every rewrite r and every footprinted slot x in r,
shard(x) == shard(scope(r))
```

Then derive:

```text
shard(r1) != shard(r2) => independent(r1, r2)
```

Until that implication is proved, the optimization should be treated as unsafe.

Acceptable proof styles:

- a written invariant tied to the rewrite API and enforced by runtime guards
- property tests that generate rewrites and verify footprint slots stay on the
  scope shard
- bounded model checking if a sufficiently small executable model exists

Formal-methods note:

- a tool like Kani or a separate executable model could help for bounded cases
- but the first useful step is still to write down the invariant precisely
- without that, "use a formal tool" just formalizes an ambiguous claim

## Benchmark Plan

### K-Way Merge Kill Criteria

Benchmark only after the sorted-run invariant is explicit.

Compare:

1. current flatten-and-sort
2. sort-each-run then k-way merge
3. true pre-sorted k-way merge

Measure:

- total merge wall time
- allocation count / bytes
- sensitivity to worker count
- sensitivity to skewed shard distributions

### Shard-aware footprint skipping

Do not benchmark first. Prove the invariant first.

If the invariant is ever proven, then benchmark against the current
`RadixScheduler`, not against the legacy pairwise scheduler alone.

## Kill Criteria

### K-way merge

Reject the optimization if any of these are true:

- the merge inputs cannot be made individually canonical without an equivalent
  sorting cost
- the implementation complicates determinism reasoning materially
- benchmarks do not show a real win on representative shard distributions

### Shard-aware footprint skip

Reject the optimization if any of these are true:

- a rewrite can touch any slot outside the scope shard
- runtime enforcement cannot cheaply verify the required locality invariant
- the benchmark only beats the legacy scheduler but not the current default
  `GenSet` scheduler

## Final Recommendation

Treat the two ideas differently.

- **K-way merge:** keep alive as a plausible optimization candidate, but convert
  it into a real design with explicit sorted-run obligations.
- **Shard-aware footprint skip:** downgrade from "optimization candidate" to
  "hypothesis requiring a proof." Until the stronger shard-locality invariant is
  stated and enforced, it should not move toward implementation.
