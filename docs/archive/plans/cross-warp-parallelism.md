<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Cross-Warp Parallelism

**Created:** 2026-01-20
**Status:** IMPLEMENTED
**Context:** Performance optimization — parallelize execution across warps

---

## Problem Statement

In `engine_impl.rs:1220`, warps are processed serially:

```rust
for (warp_id, warp_rewrites) in by_warp {
    let view = GraphView::new(store);  // borrows per-warp store
    let deltas = execute_parallel_sharded(view, &items, workers);
    all_deltas.extend(deltas);
}
```

While `execute_parallel_sharded()` parallelizes _within_ each warp, multi-warp ticks
still execute warp-by-warp. With N warps and S shards each, latency is O(N) rather
than O(1) when parallelism is available.

---

## Recommended Approach

**Global work queue of `(warp_id, shard_id)` units** — flat parallelism, no nesting.

1. **Partition rewrites by warp** — group by `WarpId`
2. **Within each warp, partition into shards** — reuse existing `shard_of()` (256 shards)
3. **Build work units** — `WorkUnit { warp_id, shard_id, items: &[ExecItem] }`
4. **Spawn fixed worker pool** — `available_parallelism()` threads, spawned once
5. **Atomic work claiming** — workers claim next unit via `AtomicUsize` index
6. **Execute with warp-local view** — each unit resolves its warp's `GraphView`

**Pros:** Scalable, clean, deterministic (canonical merge order), no API churn.
**Cons:** Slightly more wiring than per-warp threading, but avoids nested spawns.

---

## Constraints (Non-Negotiable)

1. **No nested threading** — `execute_work_queue()` is the _only_ spawn site. Units
   call serial execution internally, never `execute_parallel_sharded()`.

2. **No long-lived borrows across warps** — worker loop must: resolve `GraphView`,
   execute unit, drop view, move on. No caching `&GraphStore` across iterations.

3. **Keep `ExecItem` unchanged** — `WorkUnit` carries `warp_id + Vec<ExecItem>`.
   Do not widen `ExecItem`'s API surface.

---

## Implementation Steps

| Step | Description                                          | Files          |
| ---- | ---------------------------------------------------- | -------------- |
| 1    | Add `WorkUnit { warp_id, shard_id, items }` struct   | exec.rs        |
| 2    | Add `build_work_units()` — partition by warp + shard | exec.rs        |
| 3    | Add `execute_work_queue()` — atomic claim loop       | exec.rs        |
| 4    | Replace serial for-loop with `execute_work_queue()`  | engine_impl.rs |
| 5    | Add `#[cfg(feature = "cross-warp-parallel")]` gate   | Cargo.toml     |

---

## Files Modified

| File                                  | Change                                      |
| ------------------------------------- | ------------------------------------------- |
| `crates/warp-core/src/boaw/exec.rs`   | WorkUnit struct, build_work_units, executor |
| `crates/warp-core/src/engine_impl.rs` | Replace serial loop with work queue call    |
| `crates/warp-core/Cargo.toml`         | Feature gate (optional)                     |

---

## Success Criteria

- [x] Multi-warp tick executes all warp-shards concurrently
- [x] Fixed worker pool (no nested spawning)
- [x] Determinism preserved (canonical unit ordering + merge)
- [x] No regression on single-warp benchmarks

---

## Minimal Success Test

Integration test proving correctness:

- **Setup:** 2 warps × many shards (e.g., 100 items per warp)
- **Worker counts:** `{1, 2, 8, 32}` — all must produce identical results
- **Assertion:** Same `commit_hash` per warp (or engine receipt hash) across all runs

If this passes, the design is correct.
