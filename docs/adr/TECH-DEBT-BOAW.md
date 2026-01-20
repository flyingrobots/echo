<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# BOAW Tech Debt & Future Work

**Last Updated:** 2026-01-19
**Related:** ADR-0007-BOAW-Storage.md, PLAN-PHASE-6B-VIRTUAL-SHARDS.md

This document tracks known technical debt, optimization opportunities, and future work
related to the BOAW (Bag of Active Workers) parallel execution system.

---

## Priority Legend

| Priority | Meaning | When to Address |
| -------- | ------- | --------------- |
| **P0** | Blocking / correctness risk | Before next release |
| **P1** | High value / low effort | Next sprint |
| **P2** | Medium value / medium effort | When convenient |
| **P3** | Nice to have / exploratory | Backlog |

---

## P1: High Priority

### 1. Delete Stride Fallback (Post-Release)

**Location:** `crates/warp-core/src/boaw/exec.rs`

**Issue:** The `execute_parallel_stride()` function is kept for A/B benchmarking but adds
code complexity and maintenance burden.

**Current State:**

- Feature-gated behind `parallel-stride-fallback`
- Requires `ECHO_PARALLEL_STRIDE=1` env var
- Prints loud ASCII warning banner

**Action:** Delete after one release cycle once we're confident sharded execution
performs well in production.

**Rationale:** P1 because it's low effort, removes dead code, and was explicitly
planned for deletion.

---

### 2. Remove Deprecated `emit_view_op_delta()`

**Location:** `crates/echo-dind-tests/src/rules.rs:591`

**Issue:** The old `emit_view_op_delta()` function is marked `#[allow(dead_code)]` with
a `**DEPRECATED**` comment. It uses `delta.len()` for sequencing which is non-deterministic
under parallel execution.

**Current State:**

- No call sites (verified)
- Kept for reference only

**Action:** Delete entirely once confident no downstream code references it.

**Rationale:** P1 because dead code is confusing, and the deprecation pattern (using
`delta.len()`) could be accidentally copied.

---

## P2: Medium Priority

### 3. Cross-Warp Parallelism

**Location:** `crates/warp-core/src/engine_impl.rs:1196-1240`

**Issue:** Currently, rewrites are grouped by `warp_id` and executed sequentially across
warps. If a tick has rewrites across many warps, we lose parallelism.

**Current State:**

```rust
for (warp_id, warp_rewrites) in by_warp {
    // Serial iteration across warps
    let deltas = execute_parallel_sharded(view, &items, workers);
}
```

**Opportunity:** Create a "multi-warp GraphView" abstraction that can handle lookups
across multiple `GraphStore` instances, enabling full parallelism regardless of warp
distribution.

**Questions to Answer First:**

1. How often does a single tick have rewrites across multiple warps? (Measure in prod)
2. What's the overhead of a multi-warp view vs. per-warp iteration?
3. Can `GraphView` be generalized without breaking the borrowing invariant?

**Rationale:** P2 because per-warp parallelism likely covers 90%+ of real workloads,
and this requires significant design work.

---

### 4. Benchmark Parallel vs Serial Performance

**Location:** N/A (new work)

**Issue:** We have no production benchmarks comparing parallel execution speedup vs.
serial baseline.

**Action:**

1. Create benchmark suite with realistic workloads (10, 100, 1000 rewrites)
2. Measure wall-clock time across worker counts [1, 2, 4, 8, 16]
3. Measure memory overhead from per-worker `TickDelta` allocation
4. Profile shard distribution evenness

**Rationale:** P2 because we should validate the "free money" claim before optimizing
further.

---

### 5. State Clone Overhead in `apply_reserved_rewrites`

**Location:** `crates/warp-core/src/engine_impl.rs:993`

**Issue:** The code contains a PERF comment:

```rust
// PERF: Full state clone; consider COW or incremental tracking for large graphs.
let state_before = self.state.clone();
```

**Opportunity:** Implement copy-on-write (COW) or incremental diff tracking to avoid
full state clones on every tick.

**Rationale:** P2 because this only affects the `delta_validate` path (test/debug),
not production. However, it could slow down CI significantly on large graphs.

---

## P3: Nice to Have

### 6. Shard Rebalancing for Uneven Distributions

**Location:** `crates/warp-core/src/boaw/shard.rs`

**Issue:** Shard routing is deterministic (`LE_u64(node_id[0..8]) & 0xFF`), but some
workloads may have skewed distributions where certain shards are much heavier.

**Opportunity:** Add optional work-stealing between shards when one worker finishes
early and another is still processing a heavy shard.

**Rationale:** P3 because the current atomic shard claiming already provides decent
load balancing, and work-stealing adds complexity.

---

### 7. SIMD-Optimized Merge Sort

**Location:** `crates/warp-core/src/boaw/merge.rs`

**Issue:** `merge_deltas()` uses standard `sort_by` which is `O(n log n)`. For very
large deltas, a SIMD-optimized sort could improve performance.

**Opportunity:** Investigate `radsort` or `voracious_radix_sort` for u128 key sorting.

**Rationale:** P3 because merge is unlikely to be the bottleneck vs. execution, and
standard sort is already well-optimized.

---

### 8. Reduce `format!` Allocations in `emit_view_op_delta_scoped`

**Location:** `crates/echo-dind-tests/src/rules.rs:556-558`

**Issue:** The current implementation uses `format!` and string collection:

```rust
let scope_hex: String = scope.0[..16].iter().map(|b| format!("{:02x}", b)).collect();
let op_id = make_node_id(&format!("sim/view/op:{}", scope_hex));
```

**Opportunity:** Use a fixed-size buffer and `write!` to avoid heap allocations:

```rust
let mut buf = [0u8; 32];
hex::encode_to_slice(&scope.0[..16], &mut buf).unwrap();
let op_id = make_node_id_from_parts(b"sim/view/op:", &buf);
```

**Rationale:** P3 because this is in test rules (DIND), not production code, and the
allocation overhead is negligible compared to graph operations.

---

### 9. Document `ExecItem` vs `PendingRewrite` Mapping

**Location:** `crates/warp-core/src/engine_impl.rs:1216-1235`

**Issue:** The conversion from `PendingRewrite` to `ExecItem` involves looking up the
executor and constructing an `OpOrigin`. This logic is inline and could benefit from
clearer documentation or extraction.

**Opportunity:** Extract to a helper function with documentation explaining the mapping.

**Rationale:** P3 because the code works correctly and is only ~20 lines.

---

## Completed (Archive)

### ✅ Engine Integration (Phase 6B)

**Completed:** 2026-01-19

`apply_reserved_rewrites()` now uses `execute_parallel_sharded()` internally.
All tests pass including DIND golden hashes.

---

### ✅ Deterministic View Op IDs

**Completed:** 2026-01-19

Fixed non-determinism bug where `emit_view_op_delta()` used `delta.len()` for sequencing.
New `emit_view_op_delta_scoped()` derives IDs from intent scope (NodeId).

---

### ✅ Worker Count Configuration

**Completed:** 2026-01-19

Added `ECHO_WORKERS` env var and `EngineBuilder::workers(n)` for explicit control.
Defaults to `available_parallelism().min(NUM_SHARDS)`.

---

## Summary Statistics

| Priority | Count | Estimated Effort |
| -------- | ----- | ---------------- |
| P1 | 2 | ~2 hours |
| P2 | 3 | ~2-4 days |
| P3 | 4 | ~1-2 weeks |

**Recommendation:** Address P1 items in the next cleanup pass. P2 items should be
data-driven (benchmark first, then optimize). P3 items are exploratory and should
only be pursued if profiling reveals bottlenecks.
