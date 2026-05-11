<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# RE-028 — Merkle-Tree Memoization in Snapshot Accumulator

Legend: [RE — Runtime Engine]

## Idea

The `SnapshotAccumulator` currently re-calculates the BLAKE3 hash for the entire graph hierarchy on every tick. In systems with very deep WARP nesting (graphs all the way down), this results in redundant hashing of unchanged sub-graphs.

Implement a memoization strategy: cache the hash of each `WarpInstance` keyed by its own internal tick/hash triplet. If a sub-graph was not modified during the current tick (determined by its footprint), reuse the cached hash instead of descending.

## Why

1. **Performance**: Significantly reduces hashing overhead for complex simulations.
2. **Scalability**: Enables massive-scale causal graphs with thousands of nested instances.
3. **Efficiency**: Moves the cost of state integrity from O(TotalNodes) to O(ModifiedNodes).

## Effort

Medium — requires adding a hash-cache to the accumulator and integrating it with the instance-dirty tracking.
