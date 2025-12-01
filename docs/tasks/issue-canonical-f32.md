<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Title: feat(rmg-core): Implement strict determinism for F32Scalar (NaNs, Subnormals)

## Summary
Upgrade `F32Scalar` to enforce strict bit-level determinism across all platforms by handling "freaky numbers" (NaN payloads and subnormals) in software. Currently, `F32Scalar` only canonicalizes `-0.0`.

## Problem
IEEE 754 floating point behavior varies across architectures (x86, ARM, WASM):
1.  **NaN Payloads:** `0.0/0.0` produces different bit patterns on different CPUs.
2.  **Subnormals:** Some environments flush subnormals to zero (FTZ/DAZ), others do not.
3.  **Serialization:** Raw deserialization can bypass invariants if not carefully guarded (fixed in `scalar.rs`, but needs verifying).

This divergence breaks the determinism guarantee required for Echo's simulation loop.

## Requirements (Strict Policy)
Modify `F32Scalar::new(f32)` to apply the following transformations:

1.  **NaN Canonicalization:** If `input.is_nan()`, replace it with a single canonical quiet NaN value (e.g., `0x7fc00000`).
2.  **Subnormal Flushing:** If `input` is subnormal (exponent is 0 but mantissa is non-zero), replace it with `+0.0` (preserving sign canonicalization).
3.  **Signed Zero:** Continue to map `-0.0` to `+0.0`.

## Test Plan
Enable the commented-out tests in `crates/rmg-core/tests/determinism_policy_tests.rs`:
*   `test_policy_nan_canonicalization`: Verify positive/negative/signaling/payload NaNs all map to the canonical bits.
*   `test_policy_subnormal_flushing`: Verify small/large/negative subnormals map to `+0.0`.
*   `test_policy_serialization_guard`: Verify deserializing `-0.0` results in `+0.0`.

## Definition of Done
*   `F32Scalar::new` implements the full sanitization logic.
*   All tests in `determinism_policy_tests.rs` are uncommented and passing.
*   Benchmarks confirm acceptable overhead.
