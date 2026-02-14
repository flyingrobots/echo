<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# TTD Hardening & Future

> **Milestone:** [Backlog](README.md) | **Priority:** Unscheduled

Post-merge improvements for Time Travel Debugging (TTD) and the Scene Port boundary. Focuses on robustness, performance, and causal observability.

## T-10-9-1: Fuzzing the Port

**User Story:** As a maintainer, I want to fuzz the ScenePort boundary so that I can guarantee the MockAdapter (and future production adapters) never panic on malformed CBOR or invalid operation sequences.

**Requirements:**

- R1: Implement a `proptest` harness for `SceneDelta` and its constituent types.
- R2: Feed malformed/random CBOR bytes to `decode_scene_delta`.
- R3: Feed validly-decoded but semantically "garbage" (e.g. duplicate keys, out-of-order) deltas to `MockAdapter`.
- R4: Assert that the adapter always returns `ApplyError` and never panics.

**Acceptance Criteria:**

- [ ] AC1: Proptest suite integrated into `echo-scene-codec`.
- [ ] AC2: Coverage includes both decoding and application layers.
- [ ] AC3: 10,000+ iterations pass without panics.

**Est. Hours:** 6h

---

## T-10-9-2: SIMD Canonicalization

**User Story:** As a performance-conscious developer, I want `canonicalize_position` to use SIMD intrinsics so that scene graph updates remain cheap even as the number of entities grows by orders of magnitude.

**Requirements:**

- R1: Benchmark existing `canonicalize_f32` and `canonicalize_position` (f32 x 3).
- R2: Implement a SIMD version using `std::simd` or `packed_simd`.
- R3: Ensure SIMD implementation is bit-exact with the scalar version.
- R4: Add runtime feature detection for CPU-specific optimizations.

**Acceptance Criteria:**

- [ ] AC1: Benchmark report showing throughput improvement.
- [ ] AC2: Bit-exact parity maintained across 1,000,000 random vectors.

**Est. Hours:** 8h

---

## T-10-9-3: Causal Visualizer

**User Story:** As a simulation developer debugging complex forks, I want a tool that generates Graphviz DOT files from the `MockAdapter` state so that I can visually inspect the scene graph and causal provenance.

**Requirements:**

- R1: Add a `to_dot()` method to `MockAdapter` (or a helper trait).
- R2: Map nodes, edges, and labels to DOT nodes/edges with appropriate styling.
- R3: Optionally include provenance metadata (who wrote this atom and when).
- R4: Add a CLI flag to `ttd-app` or a standalone tool to emit DOT files from captured logs.

**Acceptance Criteria:**

- [ ] AC1: Able to render a readable SVG of a complex scene graph from a captured `.eintlog`.
- [ ] AC2: Visual styling distinguishes between regular nodes and anchored labels.

**Est. Hours:** 5h
