<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Phase 6B: Engine Integration — COMPLETE

**Status:** ✅ COMPLETE
**Date:** 2026-01-19
**Branch:** `graph-boaw`
**Commit:** `feat(boaw): parallel execution with deterministic merge ordering`

---

## SUMMARY

Phase 6B is **COMPLETE**. The sharded parallel execution primitives have been integrated into
`engine_impl.rs::apply_reserved_rewrites()`. All success criteria have been met.

### What Was Delivered

1. **Engine integration**: `apply_reserved_rewrites()` now uses `execute_parallel_sharded()`
2. **Per-warp parallelism**: Rewrites grouped by `warp_id`, parallelized within each warp
3. **Configurable workers**: `ECHO_WORKERS` env var or `EngineBuilder::workers(n)`
4. **Determinism fix**: `emit_view_op_delta_scoped()` derives IDs from intent scope, not `delta.len()`
5. **All tests pass**: Including DIND golden hashes regenerated with parallel execution

### Success Criteria — All Met ✅

| Criterion | Status |
| --------- | ------ |
| `apply_reserved_rewrites()` uses `execute_parallel_sharded()` | ✅ |
| All existing tests pass (including DIND golden hashes) | ✅ |
| Worker count defaults to `available_parallelism()` | ✅ |
| Serial fallback for edge cases | ✅ (`ECHO_WORKERS=1`) |
| No new `unsafe` code | ✅ |

---

## REFERENCE DOCS

1. `docs/adr/ADR-0007-BOAW-Storage.md` — full architecture context
2. `docs/adr/ADR-0007-PART-6-FREE-MONEY.md` — Phase 6 locked spec
3. `docs/adr/TECH-DEBT-BOAW.md` — future work and optimization opportunities
4. `crates/warp-core/src/boaw/` — the parallel execution implementation

---

## WHAT WAS SHIPPED (Phase 6B Primitives)

### New Files

- `boaw/shard.rs` — `shard_of()` + `partition_into_shards()` + `NUM_SHARDS = 256`

### Modified Files

- `boaw/exec.rs` — Added `execute_parallel_sharded()`, renamed stride to `execute_parallel_stride()`
- `boaw/mod.rs` — Exports for shard module + new functions
- `lib.rs` — Public exports: `shard_of`, `NUM_SHARDS`, `execute_parallel_sharded`
- `Cargo.toml` — Added `parallel-stride-fallback` feature
- `constants.rs` — Documented `NUM_SHARDS` as protocol constant
- `tests/boaw_parallel_exec.rs` — 5 new Phase 6B tests (12 total)
- `ADR-0007-BOAW-Storage.md` — Added § 7.1 frozen shard routing spec
- `CHANGELOG.md` — Phase 6B section
- `README.md` — Updated parallel execution description

### Key Design Decisions

1. **Shard routing is frozen:**

   ```text
   shard = LE_u64(node_id.as_bytes()[0..8]) & (NUM_SHARDS - 1)
   ```

   - NUM_SHARDS = 256 (protocol constant, cannot change)
   - First 8 bytes of NodeId's 32-byte hash, little-endian
   - 5 hardcoded test vectors prevent regression

2. **Sharded execution uses atomic shard claiming:**
   - Workers race to claim shards via `AtomicUsize::fetch_add`
   - Items in same shard processed together (cache locality)
   - Workers capped at `min(workers, NUM_SHARDS)`

3. **Stride fallback is feature-gated:**
   - Requires `parallel-stride-fallback` feature + `ECHO_PARALLEL_STRIDE=1`
   - Prints loud ASCII warning banner
   - Keep for one release, then delete

4. **Merge is unchanged:**
   - `merge_deltas()` still sorts by `(WarpOpKey, OpOrigin)`
   - Determinism enforced at merge, not execution

---

## IMPLEMENTATION DETAILS (Commit 2)

### Approach Chosen: Per-Warp Parallelism

We chose **Option 1: Per-warp parallelism** from the original options. Rewrites are grouped by
`warp_id`, and `execute_parallel_sharded()` runs within each warp's scope.

```rust
// engine_impl.rs::apply_reserved_rewrites() - simplified
let by_warp: BTreeMap<WarpId, Vec<_>> = group_by_warp(rewrites);

for (warp_id, warp_rewrites) in by_warp {
    let store = self.state.store(&warp_id);
    let view = GraphView::new(store);

    let items: Vec<ExecItem> = warp_rewrites.into_iter()
        .map(|(rw, exec)| ExecItem { exec, scope: rw.scope.local_id, origin: ... })
        .collect();

    let deltas = execute_parallel_sharded(view, &items, workers);
    all_deltas.extend(deltas);
}

let ops = merge_deltas(all_deltas)?;  // Canonical merge
```

### Why Per-Warp?

1. **Preserves `GraphView` invariant**: `GraphView` borrows a single `GraphStore` immutably
2. **Simple and correct**: No new abstractions needed
3. **Still gets parallelism**: Most ticks operate on a single warp anyway
4. **Cross-warp optimization deferred**: See TECH-DEBT-BOAW.md for future work

### Key Bug Fixed: Non-Deterministic View Op IDs

The DIND tests were producing different hashes under parallel execution because
`emit_view_op_delta()` used `delta.len()` to sequence view operations:

```rust
// BEFORE (non-deterministic under parallel)
let op_ix = delta.len();  // Worker-local! Varies by shard claim order
let op_id = make_node_id(&format!("sim/view/op:{:016}", op_ix));
```

**Fix**: New `emit_view_op_delta_scoped()` derives the op ID from the intent's scope (NodeId),
which is content-addressed and deterministic:

```rust
// AFTER (deterministic)
let scope_hex: String = scope.0[..16].iter().map(|b| format!("{:02x}", b)).collect();
let op_id = make_node_id(&format!("sim/view/op:{}", scope_hex));
```

### Files Changed in Commit 2

| File | Changes |
| ---- | ------- |
| `engine_impl.rs` | +231 lines: worker infrastructure, per-warp parallel execution |
| `rules.rs` | +102 lines: `emit_view_op_delta_scoped()`, warp-scoped footprints |
| `tick_patch.rs` | +47 lines: `WarpOpKey` warp-distinction test |
| `*.hashes.json` | Regenerated golden files |

---

## TEST COMMANDS

```bash
# Run Phase 6B tests
cargo test -p warp-core --features "delta_validate,parallel-stride-fallback" --test boaw_parallel_exec

# Run all warp-core tests
cargo test -p warp-core --features "delta_validate,parallel-stride-fallback"

# Run DIND determinism suite
cargo test -p echo-dind-harness
```

---

## SUCCESS CRITERIA FOR ENGINE INTEGRATION

1. `apply_reserved_rewrites()` uses `execute_parallel_sharded()` internally
2. All existing tests pass (including DIND golden hashes)
3. Worker count defaults to `available_parallelism()`
4. Serial fallback for edge cases (if needed)
5. No new `unsafe` code

---

## COMPLETION NOTES

Phase 6B engine integration is **DONE**. For future optimization opportunities, see:

- `docs/adr/TECH-DEBT-BOAW.md` — prioritized tech debt and future work

### Next Steps (Optional)

1. **Benchmark** parallel vs serial performance on real workloads
2. **Consider** cross-warp parallelism if profiling shows warp iteration as a bottleneck
3. **Delete** stride fallback after one release cycle

CLAUDESPEED. 🫡
