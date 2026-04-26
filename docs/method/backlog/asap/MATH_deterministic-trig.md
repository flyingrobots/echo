<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** Proof Core | **Priority:** P1

# Deterministic Trig Oracle

Pin error budget and verify the deterministic trig oracle for cross-OS audit.
Issue #177 is closed; this item now tracks the remaining release-gate evidence
gap rather than the core trig implementation.

**Issues:** #177

Status: mostly implemented. `warp_core::math::trig`, the 2048-vector golden
test, deterministic-math docs, and Linux/macOS G1 workflow steps exist. The
remaining work is to align the release claim with explicit Alpine/musl evidence
or adjust the claim if broad musl CI remains the only Alpine-class coverage.

---

## T-9-3-1: Verify and integrate deterministic trig oracle into release gate

**User Story:** As a release engineer, I want a CI gate that verifies the deterministic trig oracle (sin/cos) produces identical results across macOS, Ubuntu, and Alpine so that cross-OS determinism is proven before every release.

**Requirements:**

- R1: Keep the existing 2048-vector golden suite in
  `crates/warp-core/tests/trig_golden_vectors.rs`.
- R2: Keep the pinned 0-ULP golden-vector budget and <=16 ULP libm reference
  budget documented in determinism claims.
- R3: Preserve the existing Linux/macOS G1 workflow coverage and add explicit
  Alpine/musl evidence if DET-004 continues to claim Alpine.
- R4: Verify that the oracle's golden-vector outputs are bit-identical on every
  platform named by DET-004.
- R5: Keep the LUT-backed algorithm documented in
  `docs/determinism/SPEC_DETERMINISTIC_MATH.md`.

**Acceptance Criteria:**

- [x] AC1: Golden test suite covers 2048 input values across `[-2*TAU, 2*TAU]`
      for sin and cos.
- [ ] AC2: All 2048 values produce bit-identical results across every platform
      named by DET-004.
- [ ] AC3: CI evidence explicitly covers every platform named by DET-004.
- [x] AC4: Error budget is documented as 0 ULP for golden-vector bit identity,
      with a separate <=16 ULP budget against the libm reference.
- [x] AC5: Algorithm documentation exists in
      `docs/determinism/SPEC_DETERMINISTIC_MATH.md`.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Golden test extraction, CI matrix job, cross-OS verification, documentation.
**Out of Scope:** New trig functions (atan2, tan); trig performance optimization; WASM trig verification (separate task).

**Test Plan:**

- **Goldens:** 1000-value golden vector file for sin and cos, checked into the repository.
- **Failures:** A platform producing a different bit pattern for any value (CI fails, release blocked).
- **Edges:** sin(0), sin(pi), sin(2pi), cos(0), cos(pi/2) — boundary values where naive implementations differ.
- **Fuzz/Stress:** Extended suite: 100,000 uniformly-spaced values in [0, 2pi], verified bit-identical (CI nightly).

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~300 LoC
