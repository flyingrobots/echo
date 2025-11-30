<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Determinism Invariants

Echo guarantees the following invariants. Any violation aborts the tick deterministically and emits an error node for replay analysis.

1. **World Equivalence:** Identical diff sequences and merge decisions yield identical world hash.
2. **Merge Determinism:** Given the same base snapshot, diffs, and merge strategies, the resulting snapshot and diff hashes are identical.
3. **Temporal Stability:** GC, compression, and inspector activity do not alter logical state.
4. **Schema Consistency:** Component layout hashes must match before merges; mismatches block the merge.
5. **Causal Integrity:** Writes cannot modify values they transitively read earlier in Chronos; paradoxes are detected and isolated.
6. **Entropy Reproducibility:** Branch entropy is a deterministic function of recorded events.
7. **Replay Integrity:** Replaying from node A to B produces identical world hash, event order, and PRNG draw counts.

These invariants guide both implementation and test suites.
