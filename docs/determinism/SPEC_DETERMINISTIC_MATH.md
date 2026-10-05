<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Policy: Strictly Deterministic Math

This document is **normative**: if it conflicts with other docs, this wins.
For background hazards and motivation, see
[DETERMINISTIC_MATH.md](DETERMINISTIC_MATH.md).

All math within deterministic runtime paths must adhere to these rules.

## Docs Map

> **You are here:** normative policy (this document wins on conflicts).
>
> | Doc                                            | Role                                               |
> | ---------------------------------------------- | -------------------------------------------------- |
> | [DETERMINISTIC_MATH.md](DETERMINISTIC_MATH.md) | Hazard catalog (IEEE 754 pitfalls and mitigations) |

## 1. Floating Point (f32)

We wrap `f32` in `F32Scalar` to enforce these invariants.

| Feature            | Policy                 | Implementation Strategy                                                                                                  |
| :----------------- | :--------------------- | :----------------------------------------------------------------------------------------------------------------------- |
| **Signed Zero**    | **Strict (+0.0)**      | `new()` maps `-0.0` to `+0.0`.                                                                                           |
| **NaN Payloads**   | **Strict (Canonical)** | All `NaN` values are mapped to `0x7fc00000` (Positive Quiet NaN).                                                        |
| **Subnormals**     | **Flush-to-Zero**      | Inputs with biased exponent `0` are flushed to `+0.0`.                                                                   |
| **Rounding**       | **Ties-to-Even**       | Standard IEEE 754 default (Rust default).                                                                                |
| **Transcendental** | **Software / LUT**     | `sin`/`cos` must use software approximation (e.g., `fdlibm` port or LUT), never hardware instructions which vary by uLP. |

### Reflexivity Note

Implementations of `Eq` for floating-point types **must** be reflexive.

- `NaN == NaN` must be **TRUE**.
- Use `total_cmp` or check `is_nan()`.
- This prevents logic errors in collections (`HashSet`, `BTreeMap`) which rely on `x == x`.

## 2. Zerocopy & Serialization

- **No Direct Casts:** `F32Scalar` must **not** implement `zerocopy::FromBytes` blindly. Raw bytes could contain non-canonical values (`-0.0`, `sNaN`).
- **Deserialize:** Must route through `F32Scalar::new()` or a validator that applies canonicalization.
- **Serialize:** Safe to dump bytes _if_ the value is already canonical.

## 3. Canonical Signed Q32.32 (Bunny)

Echo and Edict pin `bunny-num` exactly to **0.6.0** as the normative fixed-point
arithmetic foundation. Bunny's [Numeric Constitution](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/docs/NUMERIC_CONSTITUTION.md)
defines the arithmetic algorithms and policy choices. Echo exposes the same
`FixedQ32_32` type at `warp_math::fixed_q32_32::FixedQ32_32` and through
`warp_core::math::fixed_q32_32`.

- Representation is a signed two's-complement `i64`, with value `raw / 2^32`.
  `from_raw` and `raw` preserve bits. Equality and ordering compare raw values.
- Canonical numerical callers use `checked_add`, `checked_sub`, `checked_neg`,
  `checked_mul`, and `checked_div`. Overflow and division by zero return `None`;
  they must not be converted into successful canonical results.
- Multiplication and division use wide integer intermediates and round to nearest
  with ties-to-even before checking representability. Negative ties use the same
  even-result rule. U64 integer operations are not Q32.32 operations.
- Validated float ingress uses `try_from_f32`, rejecting non-finite and
  out-of-range values. Float egress is lossy and rounds to nearest ties-to-even.
  Canonical arithmetic does not pass through floats.
- Existing `DFix64` operators delegate to Bunny's saturating compatibility
  operators: add/sub/neg/mul/div clamp on overflow; nonzero divided by zero
  saturates by numerator sign, and `0 / 0` remains zero.
- Existing `fixed_q32_32::from_f32` and motion-payload conversion delegate to
  Bunny's saturating conversion: NaN becomes zero and infinities or finite
  out-of-range values clamp. They are not validating ingress APIs.
- Motion v2 preserves six little-endian signed Q32.32 raw values (48 bytes),
  existing payload TypeIds, and legacy v0 decoding. Debug text is not wire data.

The legacy `echo_wasm_abi::codec::fx_from_f32` and its vector helper retain
truncation toward zero. An input of 1.5 raw units gives 1 there and 2 under Bunny;
the negative input gives -1 and -2 respectively. These APIs are compatibility
boundaries, not alternative definitions of canonical math.

This foundation does not add fixed-point source syntax, compiler lowering, or a
new executable Edict operation profile. `DFix64` trigonometry still converts
through Echo's deterministic f32 LUT. Float-mode linear algebra and geometry
extraction are separate boundaries; their migration is not implied here.

## 4. Local Validation (CI parity)

Echo’s deterministic-math CI lanes are intentionally “boring”: they run the same commands you
should run locally before proposing changes to scalar backends or transcendentals.

### Default lane (`det_float`)

The default `warp-math` build uses the float32-backed lane (`F32Scalar`) and the deterministic
trig backend (`warp_math::trig`). `warp-core` re-exports the math surface for
compatibility, but deterministic math validation lives in `warp-math`.

- `cargo test -p warp-math`
- `cargo clippy -p warp-math --lib -- -D warnings -D missing_docs`

### Fixed-point lane (`det_fixed`)

Bunny's checked Q32.32 type and motion conversions are available by default.
`DFix64` remains feature-gated as a compatibility scalar adapter. Hand-derived
raw/IEEE vectors cover rounding and refusal separately from saturation; literal
motion bytes witness the runtime consumer boundary.

- `cargo test -p warp-math --features det_fixed`
- `cargo test -p warp-core --test bunny_motion_compatibility`
- `cargo clippy -p warp-math --all-targets --features det_fixed -- -D warnings -D missing_docs`

### MUSL (Linux portability lane)

CI also runs `warp-core` under MUSL to catch portability and toolchain drift.

- Install: `sudo apt-get update && sudo apt-get install -y musl-tools`
- Test (float lane): `cargo test -p warp-core --target x86_64-unknown-linux-musl`
- Test (fixed lane): `cargo test -p warp-core --features det_fixed --target x86_64-unknown-linux-musl`
