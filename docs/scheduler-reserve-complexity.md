<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Scheduler `reserve()` Time Complexity Analysis

This document has been **merged** into the canonical warp-core scheduler doc:

- `docs/scheduler-warp-core.md`

It remains as a stable link target for older references.

The full analysis now lives in `docs/scheduler-warp-core.md`.
4. **Follow-up:** Add adversarial-collision benchmarks and evaluate collision-resistant hashers before claiming worst-case O(1) in production.

## Previous Implementation (Vec<Footprint>-based)

### Code Structure
```
reserve(tx, pending_rewrite):
  for prev_footprint in reserved_footprints:  // k iterations
    if !footprint.independent(prev_footprint):
      return false
  reserved_footprints.push(footprint.clone())
```

### Footprint::independent() Complexity (footprint.rs:114-138)

```
independent(a, b):
  if (a.factor_mask & b.factor_mask) == 0:  // O(1) - fast path
    return true

  if ports_intersect(a, b):                 // O(min(|a.ports|, |b.ports|))
    return false

  if edges_intersect(a, b):                 // O(min(|a.e_*|, |b.e_*|))
    return false

  if nodes_intersect(a, b):                 // O(min(|a.n_*|, |b.n_*|))
    return false
```

**Set intersection uses dual-iterator on sorted BTrees:**
- Complexity: O(min(|A|, |B|)) per intersection
- 4 intersection checks per `independent()` call

### Total Complexity

**Best case (factor_mask disjoint):** O(k)

**Worst case (overlapping masks, no intersections):**
- k iterations × 4 intersection checks × O(m) per check
- **O(k × m)** where m is average footprint size

## Comparison

| Metric | GenSet (New) | Vec<Footprint> (Old) |
|--------|--------------|----------------------|
| **Best Case** | O(1) (early conflict) | O(k) (factor_mask filter) |
| **Avg Case** | O(m) | O(k × m) |
| **Worst Case** | O(m) | O(k × m) |
| **Loops** | 12 for-loops | 1 for + 4 intersections |

## Typical Values

Based on the motion demo and realistic workloads:

- **k (reserved rewrites):** 10-1000 per transaction
- **m (footprint size):** 5-50 resources per rewrite
  - n_write: 1-10 nodes
  - n_read: 1-20 nodes
  - e_write: 0-5 edges
  - e_read: 0-10 edges
  - b_in/b_out: 0-5 ports each

### Example: k=100, m=20

**Old approach:**
- 100 iterations × 4 intersections × ~10 comparisons = **~4,000 operations**

**New approach:**
- 20 hash lookups (checking) + 20 hash inserts (marking) = **~40 operations**

**Theoretical speedup: ~100x**

But actual speedup depends on:
- Cache effects (hash table vs sorted BTree)
- Early exit frequency
- Hash collision rate

## Actual Performance: Needs Benchmarking!

The claim of "10-100x faster" is **extrapolated from complexity analysis**, not measured.

**TODO:** Write benchmarks to validate this claim empirically.
