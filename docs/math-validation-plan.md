# Deterministic Math Validation Plan

Goal: ensure Echo’s math module produces identical results across environments (Node, browsers, potential native wrappers) in both float32 and fixed-point modes.

---

## Test Matrix

| Mode | Environment | Notes |
| ---- | ----------- | ----- |
| float32 | Node.js (V8) | Baseline CI target |
| float32 | Chromium | Browser check via Playwright |
| float32 | WebKit | Detect discrepancies in trig functions |
| fixed32 | Node.js | Validate fixed-point operations |
| float32 | Deno / Bun (optional) | Wider coverage if adopted |

---

## Test Categories

1. **Scalar Operations**
   - Clamp, approx, conversions (deg/rad).
   - Sin/cos approximations vs reference table.

2. **Vector/Matrix Arithmetic**
   - Addition/subtraction, dot/cross, length/normalize.
   - Matrix multiplication, inversion, transformVec.

3. **Quaternion Operations**
   - Multiplication, slerp, to/from rotation matrices.

4. **Transforms**
   - Compose/decompose transform, ensure round-trip fidelity.

5. **PRNG**
   - Sequence reproducibility across environments (same seed -> same numbers).
   - Jump consistency (forked streams diverge predictably).

6. **Stack Allocation**
   - Ensure MathStack pushes/pops deterministically (guard misuse).

---

## Tooling
- Rust harness (in `rmg-core/tests/math_validation.rs`) validates scalar/vector/matrix/quaternion + PRNG behaviour against JSON fixtures.
- Provide deterministic reference values generated offline (e.g., via high-precision Python or Rust) stored in fixtures.
- Next step: mirror the fixtures in Vitest with snapshot-style comparisons for the TypeScript layer.
- For cross-environment checks, add Playwright-driven tests that run the same suite in headless Chromium/WebKit (call into math module via bundled script).
- Fixed-point tests compare against integer expectations.

---

## Tolerances
- Float32 comparisons use epsilon `1e-6`.
- Trig functions might require looser tolerance `1e-5` depending on environment (document deviations).
- Fixed-point exact equality expected (integer comparisons).

---

## Tasks
- [x] Generate reference fixtures (JSON) for scalar/vector/matrix/quaternion/PRNG cases.
- [x] Implement Rust-based validation suite (`cargo test -p rmg-core --test math_validation`).
- [ ] Mirror fixtures in Vitest to cover the TypeScript bindings (float32 mode).
- [ ] Integrate Playwright smoke tests for browser verification.
- [ ] Add CI job running math tests across environments.
- [ ] Document any environment-specific deviations in decision log.

---

## Open Questions
- Should we bundle deterministic trig lookup tables for browsers with inconsistent `Math.sin/cos`?
- How to expose failure info to designers (e.g., CLI command to run math diagnostics)?
- Do we need wasm acceleration for fixed-point operations (profile results first)?
