<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

//! Deterministic math hazards and mitigation strategies.
//!
//! This document outlines the specific challenges of cross-platform deterministic floating-point
//! arithmetic (IEEE 754) and the strategies used in Echo to mitigate them.

# Deterministic Math Hazards

Achieving bit-perfect determinism across disparate hardware architectures (x86_64, AArch64, WASM32)
is difficult due to loosely defined behaviors in the IEEE 754 specification. While basic arithmetic
is largely standardized, "freaky numbers" (NaN, Subnormals, Signed Zero) introduce divergence.

## Related Docs

- **Normative policy:** `docs/SPEC_DETERMINISTIC_MATH.md`
- **Validation & CI lanes:** `docs/math-validation-plan.md`
- **Math claims / theory framing:** `docs/warp-math-claims.md`

## 1. NaN Payloads
**The Hazard:** IEEE 754 standardizes that `0.0 / 0.0` produces `NaN`, but it does *not* mandate
the exact bit pattern of that `NaN`.
*   **Sign Bit:** Some FPUs produce positive NaN, others negative.
*   **Payload Bits:** The mantissa can contain arbitrary diagnostic information ("payload").
*   **Signaling vs Quiet:** Operations might quiet a signaling NaN (sNaN -> qNaN) differently.

**Impact:** If a simulation produces a NaN, the exact bits may differ between a player on Mac (ARM)
and a player on Windows (x86). Hashing this state (`blake3(mem)`) will result in a fork (desync).

**Mitigation:**
*   **Canonicalization:** All NaNs must be clamped to a single canonical bit pattern (e.g., `0x7fc00000`)
    at the boundary of the deterministic simulation (input/output) and potentially after every operation.
*   **Avoidance:** Ideally, gameplay logic should never produce NaN.

## 2. Subnormal Numbers (Denormals)
**The Hazard:** Subnormal numbers are very small numbers close to zero (e.g., `1e-40`).
*   **Hardware Diversity:** Some CPUs (or modes like DAZ/FTZ on x86) flush these to zero for performance.
    Others (WASM, modern ARM) compute them precisely.
*   **The Fork:** A calculation `1e-40 + 0.0` yields `1e-40` on Machine A and `0.0` on Machine B.
    This tiny difference butterfly-effects into a major desync.

**Mitigation:**
*   **Software Flush-to-Zero:** The `F32Scalar` wrapper should detect subnormals and force them to
    `0.0` (with proper sign canonicalization) to ensure consistent behavior regardless of CPU flags.

## 3. Signed Zero
**The Hazard:** IEEE 754 distinguishes `+0.0` and `-0.0`.
*   **Arithmetic:** `-1.0 * 0.0 = -0.0`. `(-0.0) + (-0.0) = -0.0`.
*   **Comparison:** `+0.0 == -0.0` is true.
*   **Hashing:** `hash(+0.0) != hash(-0.0)`.
*   **Impact:** If logic relies on bits (hashing) or strict ordering (`total_cmp`), `-0.0` is a distinct
    value.

**Mitigation:**
*   **Canonicalization:** `F32Scalar` converts `-0.0` to `+0.0` on construction.

## 4. Fused Multiply-Add (FMA)
**The Hazard:** `a * b + c` can be computed as two ops (round intermediate) or one FMA op (single round).
*   **Result:** The least significant bit often differs.
*   **Compiler:** Rust/LLVM might optimize `mul` + `add` into `fma` depending on target features.

**Mitigation:**
*   **Strict Ops:** Rely on `warp-core` wrappers which enforce distinct operations.
*   **Compiler Flags:** Ensure builds do not aggressively fuse ops unless explicitly safe.

## 5. Transmutation & Zerocopy
**The Hazard:** Casting raw bytes to `f32` (`zerocopy::FromBytes`) bypasses constructor logic.
*   **Attack Vector:** A network packet contains non-canonical bytes (e.g., `-0.0` or weird `NaN`).
    If interpreted directly as `F32Scalar`, the invariant is violated.

**Mitigation:**
*   **Validation:** `FromBytes` implementations must validate or canonicalize data.
*   **Opaque Types:** Prefer opaque serialization that routes through `new()`.
