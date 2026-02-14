<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# RUSTAGEDDON TRIALS: DIND (Deterministic Ironclad Nightmare Drills)

This mission implements a rigorous determinism verification suite for the Continuum engine.

## Doctrine

We do not "hope" for determinism. We assert inevitability.

1. Same inputs ⇒ same outputs (byte-for-byte).
2. Same inputs ⇒ same intermediate states.
3. Same inputs ⇒ same errors.
4. Across runs, threads, and platforms.

## Phase 1: The Heartbeat (Canonical State Hash)

- [x] **1. Implement `canonical_state_hash` in `warp-core`:**
    - [x] Create a `CanonicalHash` trait or method on `GraphStore`.
    - [x] Must traverse nodes/edges in sorted order (by ID).
    - [x] Must serialize attachments deterministically (already done via "Mr Clean", but double check iteration order).
    - [x] Use BLAKE3.
    - [x] Expose this via `EchoKernel::state_hash()`.

## Phase 2: The Harness (DIND Runner)

- [x] **2. Create `crates/echo-dind-harness`:**
    - [x] CLI tool to run scenarios.
    - [x] Input: `.eintlog` (Sequence of `pack_intent_v1` bytes).
    - [x] Output: `hashes.json` (Array of state hashes after each op).
    - [x] Logic: Init kernel -> Apply Op -> Hash -> Repeat -> Assert match.

## Phase 3: The Drills (Scenarios & Stress)

- [x] **3. Create DIND Scenarios (`vendor/echo/testdata/dind/`):**
    - [x] `000_smoke_transcript.eintlog`: 50 ops, basic state changes.
    - [x] `010_graph_rewrite_dense.eintlog`: Saturation test (1k steps).
    - [x] `020_conflict_policy.eintlog`: Abort vs Retry stability.
- [x] **4. Add Regression Test:**
    - [x] Add a standard Rust test that runs the harness against committed `hashes.json` for the smoke scenario.

## Phase 4: Policy Enforcement

- [x] **5. Ban Nondeterminism:**
    - [x] Verify `std::collections::HashMap` is not iterated in hash-sensitive paths (or use `BTreeMap` / sorted iterators).
    - [x] Add CI grep-check for `std::time`, `rand::thread_rng`.

## Execution Order

1. Implement `canonical_state_hash` (The Prerequisite).
2. Create the Harness + Smoke Scenario (The MVP).
3. Lock it in with a regression test.
4. Expand scenarios.
