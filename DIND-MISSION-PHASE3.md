<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# RUSTAGEDDON TRIALS: DIND Phase 3+

We are moving from "determinism exists" to "determinism is inevitable".

## Phase 3: Torture Mode
- [ ] **1. Update `echo-dind-harness` CLI:**
    - [ ] Add `Torture { scenario: PathBuf, runs: u32, threads: Option<String> }` subcommand.
    - [ ] Implement repeated in-process execution loop.
    - [ ] Compare full hash chain across runs.
    - [ ] Report first divergence (run index, step index, expected vs actual).

## Phase 4: The Drills (Real Scenarios)
- [ ] **2. Scenario 010: Dense Rewrite Saturation**
    - [ ] Create `scripts/gen_dense_rewrite.mjs`.
    - [ ] Generate 1k-5k ops (node/edge churn).
    - [ ] Record golden hashes.
- [ ] **3. Scenario 030: Error Determinism**
    - [ ] Create `scripts/gen_error_determinism.mjs`.
    - [ ] Generate invalid ops (bad payloads, invalid IDs).
    - [ ] Assert state hash stability (no partial commits).

## Phase 5: Randomized Construction
- [ ] **4. Randomized Order Drill**
    - [ ] Create `scripts/gen_randomized_order.mjs`.
    - [ ] Generate equivalent graph states via permuted op orders.
    - [ ] Verify final state hashes match.

## CI & Policy
- [ ] **5. CI Integration**
    - [ ] Add `make dind` or `cargo xtask dind` to run suite.
    - [ ] Add grep-checks for `SystemTime`, `Instant`, `rand`, `HashMap`.
