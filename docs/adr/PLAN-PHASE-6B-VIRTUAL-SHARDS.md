<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Phase 6B Handoff: Engine Integration Planning

**Status:** HANDOFF — Ready for next agent
**Date:** 2026-01-18
**Branch:** `graph-boaw`

---

## TL;DR FOR THE NEXT AGENT

You're here to **plan how to wire Phase 6B sharded execution into the Engine pipeline**.

Phase 6B primitives are DONE and TESTED. What remains is integrating them with `engine_impl.rs`.

**Read these first:**

1. `docs/adr/ADR-0007-BOAW-Storage.md` — full architecture context
2. `docs/adr/ADR-0007-PART-6-FREE-MONEY.md` — Phase 6 locked spec
3. `crates/warp-core/src/boaw/` — the implementation you're integrating

---

## WHAT WAS JUST SHIPPED (Phase 6B)

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

## WHAT YOU NEED TO PLAN

### The Problem

`engine_impl.rs::apply_reserved_rewrites()` (lines 1044-1105) currently uses **serial execution**:

```rust
let mut delta = TickDelta::new();
for rewrite in rewrites {
    let executor = self.rule_by_compact(id).executor;
    let store = self.state.store(&rewrite.scope.warp_id);
    let view = GraphView::new(store);
    (executor)(view, &rewrite.scope.local_id, &mut delta);
}
let ops = delta.finalize();
```

### The Challenge

The Phase 6B `execute_parallel_sharded()` was designed for testing with a **single GraphView**. The real Engine handles:

1. **Multiple warps**: Each rewrite can be on a different `warp_id`
2. **Executor lookup**: Need to map `compact_rule_id` → executor function
3. **Store lookup**: Need to get the right `GraphStore` for each warp

### Options to Explore

1. **Per-warp parallelism**: Group rewrites by `warp_id`, parallelize within each warp
2. **Cross-warp view**: Create a unified view abstraction that handles multi-warp lookups
3. **Executor registry**: Pass executor registry to workers instead of baking it into `ExecItem`
4. **Staged approach**: Use sharded execution for single-warp cases first

### Key Files to Study

- `engine_impl.rs:1044-1105` — Current serial execution path
- `engine_impl.rs:860-900` — How `reserve_for_receipt()` produces `Vec<PendingRewrite>`
- `scheduler.rs` — How footprints relate to parallelism
- `boaw/exec.rs` — Current `ExecItem` + `execute_parallel_sharded()`
- `graph_view.rs` — `GraphView` constraints

### Questions to Answer

1. How often does a single tick have rewrites across multiple warps?
2. Should `ExecItem` store a `warp_id` or just a unified scope?
3. Can we create a "multi-warp GraphView" that handles lookups?
4. What's the simplest correct integration vs. the optimal one?

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

## GO TIME

1. Read `ADR-0007-BOAW-Storage.md` (especially § 7 and § 8)
2. Read `ADR-0007-PART-6-FREE-MONEY.md` for phase invariants
3. Study `engine_impl.rs::apply_reserved_rewrites()`
4. Propose an integration plan
5. Get approval before implementing

CLAUDESPEED. 🫡
