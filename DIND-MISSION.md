# RUSTAGEDDON TRIALS: DIND (Deterministic Ironclad Nightmare Drills)

This mission implements a rigorous determinism verification suite for the Continuum engine.

## Doctrine
We do not "hope" for determinism. We assert inevitability.
1. Same inputs ⇒ same outputs (byte-for-byte).
2. Same inputs ⇒ same intermediate states.
3. Same inputs ⇒ same errors.
4. Across runs, threads, and platforms.

## Phase 1: The Heartbeat (Canonical State Hash)
- [ ] **1. Implement `canonical_state_hash` in `warp-core`:**
    - [ ] Create a `CanonicalHash` trait or method on `GraphStore`.
    - [ ] Must traverse nodes/edges in sorted order (by ID).
    - [ ] Must serialize attachments deterministically (already done via "Mr Clean", but double check iteration order).
    - [ ] Use BLAKE3.
    - [ ] Expose this via `EchoKernel::state_hash()`.

## Phase 2: The Harness (DIND Runner)
- [ ] **2. Create `crates/echo-dind-harness`:**
    - [ ] CLI tool to run scenarios.
    - [ ] Input: `.eintlog` (Sequence of `pack_intent_v1` bytes).
    - [ ] Output: `hashes.json` (Array of state hashes after each op).
    - [ ] Logic: Init kernel -> Apply Op -> Hash -> Repeat -> Assert match.

## Phase 3: The Drills (Scenarios & Stress)
- [ ] **3. Create DIND Scenarios (`vendor/echo/testdata/dind/`):**
    - [ ] `000_smoke_transcript.eintlog`: 50 ops, basic state changes.
    - [ ] `010_graph_rewrite_dense.eintlog`: Saturation test (1k steps).
    - [ ] `020_conflict_policy.eintlog`: Abort vs Retry stability.
- [ ] **4. Add Regression Test:**
    - [ ] Add a standard Rust test that runs the harness against committed `hashes.json` for the smoke scenario.

## Phase 4: Policy Enforcement
- [ ] **5. Ban Nondeterminism:**
    - [ ] Verify `std::collections::HashMap` is not iterated in hash-sensitive paths (or use `BTreeMap` / sorted iterators).
    - [ ] Add CI grep-check for `std::time`, `rand::thread_rng`.

## Execution Order
1. Implement `canonical_state_hash` (The Prerequisite).
2. Create the Harness + Smoke Scenario (The MVP).
3. Lock it in with a regression test.
4. Expand scenarios.
