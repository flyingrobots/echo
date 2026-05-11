<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# AUDIT: READY-TO-SHIP ASSESSMENT (2026-04-11)

## 1. QUALITY & MAINTAINABILITY ASSESSMENT (EXHAUSTIVE)

1.1. **Technical Debt Score (1-10):** 2 - **Justification:** 1. **Complex Footprint Overlap Logic**: The spatial conflict detection in the scheduler is highly optimized but has high cognitive complexity. 2. **Manual Wesley Sync**: The generated code path still requires manual oversight rather than being a "set-and-forget" build step. 3. **Dual Float Strategy**: Maintaining both IEEE and Fixed-point paths creates a large testing surface for determinism.

1.2. **Readability & Consistency:** - **Issue 1:** `warp-core` uses Lamport ticks but refers to them as "Ticks" while the renderer refers to them as "Frames." - **Mitigation Prompt 1:** `Standardize on 'Tick' for all causal coordinates and 'Frame' only for the final visual projection in the ScenePort.` - **Issue 2:** The "Mr. Clean" (panic-free) discipline is enforced via lints but not always explicitly documented in function signatures. - **Mitigation Prompt 2:** `Add '# Errors' sections to all public CasService/WarpCore methods detailing the stable error variants instead of relying on Result<T, anyhow::Error>.` - **Issue 3:** The `MaterializationBus` reduction logic is spread across 8 built-in variants with minimal domain-level explanation. - **Mitigation Prompt 3:** `Create 'docs/spec/materialization-reduction.md' explaining the deterministic algebra behind Sum, Min, Max, and Consensus reduce operations.`

1.3. **Code Quality Violation:** - **Violation 1: SRP (`SnapshotAccumulator`)**: It handles both state hashing and the Merkle-tree diff generation. - **Violation 2: SoC (`warp-cli`)**: The CLI handles both local testing and remote TTD host bridging. - **Violation 3: SRP (`ProvenanceStore`)**: The local implementation handles both file I/O and hash-locked indexing.

## 2. PRODUCTION READINESS & RISK ASSESSMENT (EXHAUSTIVE)

2.1. **Top 3 Immediate Ship-Stopping Risks (The "Hard No"):** - **Risk 1: Floating Point Poisoning (High)**: If an external library uses standard `f32` in a core path, it will break determinism across platforms. - **Mitigation Prompt 7:** `Add a lint to scripts/ban-nondeterminism.sh that detects any direct usage of std::f32/f64 in non-WVP crates.` - **Risk 2: Merkle Hash Collision (Medium)**: BLAKE3 is strong, but the way sub-graphs are keyed in the snapshot accumulator needs a formal collision-avoidance audit. - **Mitigation Prompt 8:** `Conduct a 'Hash Collision Audit' of the Merkle Commit implementation, ensuring that type IDs and node IDs are salted before hashing to prevent cross-graph collisions.` - **Risk 3: OOM during Merkle Finalization (Low)**: Giant graphs could cause memory exhaustion during the recursive hash calculation. - **Mitigation Prompt 9:** `Implement a 'Streaming Snapshot Accumulator' that yields hashes incrementally rather than building the entire Merkle tree in memory.`

2.2. **Security Posture:** - **Vulnerability 1: Side-Channel Data Leak**: The replay history contains every rejected counterfactual. If those patches contain PII, the history is a target. - **Mitigation Prompt 10:** `Implement 'Privacy Redaction' in the ProvenanceStore, allowing sensitive properties to be zeroed out after a worldline is sealed.` - **Vulnerability 2: Host Fingerprinting**: Deterministic math LUTs might theoretically leak CPU architecture details if not strictly bounds-checked. - **Mitigation Prompt 11:** `Audit all math LUT implementations for constant-time access and ensure no branching occurs based on input values.`

2.3. **Operational Gaps:** - **Gap 1: Profiling Integration**: No built-in way to export Flamegraph data for rule-level bottleneck detection. - **Gap 2: Remote TTD Stability**: The bridge between Echo WASM and `warp-ttd` is still under active churn. - **Gap 3: CI Throughput**: DIND tests are slow and may block the release pipeline as seed counts increase.

## 3. FINAL RECOMMENDATIONS & NEXT STEP

3.1. **Final Ship Recommendation:** **YES, BUT...** (Harden the floating-point ban and audit Merkle collision safety).

3.2. **Prioritized Action Plan:** - **Action 1 (High Urgency):** Enforce the `det_fixed` build profile by default. - **Action 2 (Medium Urgency):** Standardize on Wesley-generated contracts for TTD. - **Action 3 (Low Urgency):** Implement Merkle-tree memoization for deep graphs.
