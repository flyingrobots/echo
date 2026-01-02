<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Scheduler `reserve()` Time Complexity Analysis

## Current Implementation (GenSet-based)

### Code Structure (scheduler.rs)

```
reserve(tx, pending_rewrite):
  Phase 1: Conflict Detection
    for node in n_write:           // |n_write| iterations
      if nodes_written.contains() OR nodes_read.contains():  // O(1) each
        return false

    for node in n_read:            // |n_read| iterations
      if nodes_written.contains(): // O(1)
        return false

    for edge in e_write:           // |e_write| iterations
      if edges_written.contains() OR edges_read.contains():  // O(1) each
        return false

    for edge in e_read:            // |e_read| iterations
      if edges_written.contains(): // O(1)
        return false

    for port in b_in:              // |b_in| iterations
      if ports.contains():         // O(1)
        return false

    for port in b_out:             // |b_out| iterations
      if ports.contains():         // O(1)
        return false

  Phase 2: Marking
    for node in n_write: mark()    // |n_write| × O(1)
    for node in n_read: mark()     // |n_read| × O(1)
    for edge in e_write: mark()    // |e_write| × O(1)
    for edge in e_read: mark()     // |e_read| × O(1)
    for port in b_in: mark()       // |b_in| × O(1)
    for port in b_out: mark()      // |b_out| × O(1)
```

### Complexity Breakdown

**Phase 1 (worst case - no early exit):**
- Node write checks: |n_write| × 2 hash lookups = |n_write| × O(1)
- Node read checks: |n_read| × 1 hash lookup = |n_read| × O(1)
- Edge write checks: |e_write| × 2 hash lookups = |e_write| × O(1)
- Edge read checks: |e_read| × 1 hash lookup = |e_read| × O(1)
- Port in checks: |b_in| × 1 hash lookup = |b_in| × O(1)
- Port out checks: |b_out| × 1 hash lookup = |b_out| × O(1)

**Total Phase 1:** O(|n_write| + |n_read| + |e_write| + |e_read| + |b_in| + |b_out|)

**Phase 2 (only if Phase 1 succeeds):**
- Same as Phase 1 but marking instead of checking: O(m)

**Total:** O(m) where **m = |n_write| + |n_read| + |e_write| + |e_read| + |b_in| + |b_out|**

### Important Notes

1. **Hash Table Complexity / Assumptions:**
   - GenSet uses `FxHashMap` which is O(1) average case.
   - Worst case with pathological hash collisions: O(log n) or O(n).
   - Assumes no adversarial inputs targeting collisions; production should evaluate collision-resistant hashers (aHash/SipHash) and/or adversarial benchmarks before release.

2. **Early Exit Optimization:**
   - Phase 1 returns immediately on first conflict
   - Best case (early conflict): O(1)
   - Worst case (no conflict or late conflict): O(m)

3. **Counting the Loops:** 12 total (6 conflict checks, 6 marks), each over disjoint footprint subsets.
4. **Follow-up:** Add adversarial-collision benchmarks and evaluate collision-resistant hashers before claiming worst-case O(1) in production.

## Previous Implementation (`Vec<Footprint>`-based)

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

| Metric | GenSet (New) | `Vec<Footprint>` (Old) |
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
