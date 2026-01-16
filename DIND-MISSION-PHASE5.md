<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# RUSTAGEDDON TRIALS: DIND Phase 5 (The Shuffle)

This phase tests robustness against insertion order and HashMap iteration leaks.

## Doctrine
- **Invariant A (Self-Consistency):** A specific shuffled transcript must be deterministic across runs/platforms.
- **Invariant B (Convergence):** Different shuffles of *commutative* operations must yield the same final state hash.

## Prerequisite: ID Stability
- Current Status: IDs are hashes of string labels (e.g., `make_node_id("label")`).
- This means IDs *are* stable/explicit provided the labels are deterministic.
- If we shuffle `InsertNode("A")` and `InsertNode("B")`, the resulting IDs are `hash("node:A")` and `hash("node:B")` regardless of order.
- **Verdict:** We are ready for Invariant B (Convergence).

## Tasks
- [ ] **1. Randomized Generator (`scripts/bootstrap_randomized_order.mjs`):**
    - [ ] Input: `--seed`, `--out`.
    - [ ] Use seeded Xorshift32 (already implemented in dense rewrite script, extract/reuse?).
    - [ ] Pattern:
        - Create N nodes with deterministic labels (`node_0`..`node_N`).
        - Shuffle creation order.
        - Create M edges connecting random pairs (deterministic pairs based on seed, but shuffled insertion).
        - Set K attachments (shuffled).
        - **Critical:** Ensure no duplicate edges/attachments that would trigger overwrite behavior unless intended.
- [ ] **2. Generate Scenarios (`050_randomized_order_small`):**
    - [ ] Generate 10 seeds (0001..0010).
    - [ ] Record goldens for all.
- [ ] **3. Harness Update (`echo-dind-harness`):**
    - [ ] Add `Converge { scenarios: Vec<PathBuf> }` command.
    - [ ] Runs all inputs, asserts final state hashes are identical.
- [ ] **4. CI Integration:**
    - [ ] Run seeds 1-3 in PR check.
    - [ ] Run `converge` on 1-3.
