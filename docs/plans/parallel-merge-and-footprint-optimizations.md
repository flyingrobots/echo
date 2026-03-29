<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Parallel Merge & Footprint Scheduling Optimizations

**Status:** Ideas — not yet designed or scheduled

See also the stricter review note:
[Parallel Merge & Footprint Optimization Design Review](parallel-merge-and-footprint-design-review.md).

Current disposition after code review:

- k-way merge remains plausible, but only if merge inputs can be proven or
  enforced to be individually sorted by the canonical `(WarpOpKey, OpOrigin)`
  order
- shard-aware cross-shard footprint skipping is **not** currently proven safe
  against the default scheduler and should be treated as a hypothesis, not an
  implementation-ready optimization

Two optimization opportunities for the parallel execution pipeline, both
exploiting structure that already exists in the shard-based architecture.

---

## 1. K-Way Merge for Canonical Delta Merge

### Observation

Per-shard deltas may be partially sorted only under stricter preconditions.
Shard assignment by `lowbits(NodeId) & (SHARDS - 1)` alone does not prove
canonical run ordering, and the current code does not yet enforce individually
sorted runs. Treat "pre-sorted runs" as a hypothesis until the design review's
proof obligations are satisfied.

### Idea

Replace the current flatten-and-sort in `merge_deltas()` with a k-way merge
over the per-worker deltas using a min-heap. This is O(n log k) instead of
O(n log n), where k = worker count (bounded, typically 4–16).

### Why It Works

If the merge inputs can be proven or enforced to be pre-sorted by the canonical
ordering required by the target path, then a k-way merge would produce the same
deterministic output at lower cost. That proof does not exist yet, so this
remains a design candidate rather than an implementation-ready optimization.

### Constraints

- The determinism guarantee is non-negotiable: the merged output must be
  identical regardless of thread count or execution order.
- Dedup and conflict detection still apply during merge.

---

## 2. Shard-Aware Footprint Overlap Checks

### Current Behavior

Footprint overlap testing currently considers all rule pairs. But rules
whose footprints land in different shards may be structurally disjoint under
additional scheduler invariants. The current review note does **not** treat
cross-shard independence as proven against the default scheduler.

### Proposed Change

Only perform footprint overlap checks for rules within the same shard if a
stronger locality invariant can first prove cross-shard independence. Until
then, cross-shard skip remains a hypothesis and not a safe implementation step.

For the remaining same-shard pairs, use a bloom filter over read/write
slots: hash each footprint's slots into a small bit vector, AND two
filters — if the result is zero, the pair is guaranteed disjoint (no
false negatives possible). Only pairs that survive the bloom filter need
the real overlap check.

### Rationale

If the stronger locality invariant holds, then in a well-distributed workload
most rules would land in different shards, making the overlap test set much
smaller than the full rule set. The bloom filter would then further prune
within-shard pairs at near-zero cost. False positives would just mean
conservative serialization, which is safe — never incorrect.

### Bounds

- Shard assignment must remain deterministic (it already is: BLAKE3 bits).
- Bloom filter parameters need tuning to balance false positive rate vs
  memory. A small fixed-size filter (e.g., 256 bits) per footprint is
  likely sufficient.
- This optimization is most valuable when the rule count is high and
  footprints are sparse (the common case for well-designed systems).
