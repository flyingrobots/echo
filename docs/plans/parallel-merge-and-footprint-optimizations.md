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

Per-shard deltas are already partially sorted by construction: shards are
assigned by `lowbits(NodeId) & (SHARDS - 1)`, so ops within a shard share
a bit prefix. If ops are accumulated in insertion order within each shard,
the per-worker deltas are pre-sorted runs.

### Idea

Replace the current flatten-and-sort in `merge_deltas()` with a k-way merge
over the per-worker deltas using a min-heap. This is O(n log k) instead of
O(n log n), where k = worker count (bounded, typically 4–16).

### Why It Works

The canonical merge must produce ops in `(WarpOpKey, OpOrigin)` order for
deterministic hashing. A k-way merge of pre-sorted runs produces the same
canonical order — it's just cheaper when the runs are already partially
ordered, which they are by shard assignment.

### Constraints

- The determinism guarantee is non-negotiable: the merged output must be
  identical regardless of thread count or execution order.
- Dedup and conflict detection still apply during merge.

---

## 2. Shard-Aware Footprint Overlap Checks

### Current Behavior

Footprint overlap testing currently considers all rule pairs. But rules
whose footprints land in different shards are structurally disjoint by
construction — the shard assignment (`lowbits(NodeId)`) already proves
non-overlap.

### Proposed Change

Only perform footprint overlap checks for rules within the same shard.
Cross-shard pairs are guaranteed independent and skip the check entirely.

For the remaining same-shard pairs, use a bloom filter over read/write
slots: hash each footprint's slots into a small bit vector, AND two
filters — if the result is zero, the pair is guaranteed disjoint (no
false negatives possible). Only pairs that survive the bloom filter need
the real overlap check.

### Rationale

In a well-distributed workload, most rules land in different shards, making
the overlap test set much smaller than the full rule set. The bloom filter
further prunes within-shard pairs at near-zero cost. False positives just
mean conservative serialization, which is safe — never incorrect.

### Bounds

- Shard assignment must remain deterministic (it already is: BLAKE3 bits).
- Bloom filter parameters need tuning to balance false positive rate vs
  memory. A small fixed-size filter (e.g., 256 bits) per footprint is
  likely sufficient.
- This optimization is most valuable when the rule count is high and
  footprints are sparse (the common case for well-designed systems).
