<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

//! Math Determinism Specification & Policy.
//!
//! Defines the strict policies Echo enforces to guarantee bit-perfect determinism across all
//! supported platforms. This spec takes precedence over hardware defaults.

# Policy: Strictly Deterministic Math

All math within the simulation loop (`warp-core`) must adhere to these rules.

## 1. Floating Point (f32)

We wrap `f32` in `F32Scalar` to enforce these invariants.

| Feature | Policy | Implementation Strategy |
| :--- | :--- | :--- |
| **Signed Zero** | **Strict (+0.0)** | `new()` maps `-0.0` to `+0.0`. |
| **NaN Payloads** | **Strict (Canonical)** | All `NaN` values are mapped to `0x7fc00000` (Positive Quiet NaN). |
| **Subnormals** | **Flush-to-Zero** | Inputs with biased exponent `0` are flushed to `+0.0`. |
| **Rounding** | **Ties-to-Even** | Standard IEEE 754 default (Rust default). |
| **Transcendental** | **Software / LUT** | `sin`/`cos` must use software approximation (e.g., `fdlibm` port or LUT), never hardware instructions which vary by uLP. |

### Reflexivity Note
Implementations of `Eq` for floating-point types **must** be reflexive.
*   `NaN == NaN` must be **TRUE**.
*   Use `total_cmp` or check `is_nan()`.
*   This prevents logic errors in collections (`HashSet`, `BTreeMap`) which rely on `x == x`.

## 2. Zerocopy & Serialization

*   **No Direct Casts:** `F32Scalar` must **not** implement `zerocopy::FromBytes` blindly. Raw bytes could contain non-canonical values (`-0.0`, `sNaN`).
*   **Deserialize:** Must route through `F32Scalar::new()` or a validator that applies canonicalization.
*   **Serialize:** Safe to dump bytes *if* the value is already canonical.

## 3. Audit Findings (2025-11-30)

An audit of `warp-core` identified the following risks:

*   **Hardware Transcendentals:** `F32Scalar::sin/cos` previously delegated to `f32::sin/cos`. **Risk:** High (varies across libc/hardware implementations).
    *   *Status:* Implemented deterministic LUT-backed trig in `warp_core::math::trig` (Issue #115).
*   **Implicit Hardware Ops:** `Add`, `Sub`, `Mul`, `Div` rely on standard `f32` ops.
    *   *Risk:* Subnormal handling (DAZ/FTZ) depends on CPU flags.
    *   *Status:* `F32Scalar::new` flushes subnormals to `+0.0` at construction and after operations.
*   **NaN Propagation:** `f32` ops produce hardware-specific NaN payloads.
    *   *Status:* `F32Scalar::new` canonicalizes NaNs to `0x7fc0_0000`.

## 4. Implementation Checklist

- [x] Canonicalize `-0.0` to `+0.0` (PR #123).
- [x] Canonicalize `NaN` payloads (`F32Scalar::new`).
- [x] Flush subnormals to `+0.0` (`F32Scalar::new`).
- [x] Replace `sin`/`cos` with deterministic approximation (`warp_core::math::trig` LUT backend).
