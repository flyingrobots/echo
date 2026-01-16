<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Implementation Guide — Deterministic `sin/cos` for `F32Scalar` (LUT-backed)

This document is a step-by-step, code-oriented guide for implementing a deterministic `sin`, `cos`, and `sin_cos` backend for `warp_core::math::scalar::F32Scalar`.

## Status

As of **2026-01-01**, this LUT-backed backend is implemented on the `F32Scalar/sin-cos` branch:

- Implementation: `crates/warp-core/src/math/trig.rs`
- LUT data: `crates/warp-core/src/math/trig_lut.rs`
- Tests: `crates/warp-core/tests/deterministic_sin_cos_tests.rs`

It is written to match the current test scaffolding on the `F32Scalar/sin-cos` branch:

- `crates/warp-core/tests/deterministic_sin_cos_tests.rs`

The spec/policy drivers for this work live here:

- `docs/SPEC_DETERMINISTIC_MATH.md` (policy, checklist)
- `crates/warp-core/src/math/scalar.rs` (current `Scalar` trait + `F32Scalar` impl)

---

## Goal

Replace the hardware/libc-backed trig:

- `F32Scalar::sin()` **must not** delegate to `f32::sin()`
- `F32Scalar::cos()` **must not** delegate to `f32::cos()`

…with an implementation that is **bit-stable across supported platforms** (native + WASM) while keeping `F32Scalar`’s canonicalization invariants.

---

## Non-goals (for this iteration)

- Perfectly matching the platform `libm` behavior.
- Maximum-accuracy transcendental math.
- Implementing the fixed-point trig backend.
- Designing the “forever” math backend architecture.

The intent is: *ship a deterministic trig backend with a known, documented error budget*, then iterate.

---

## Determinism & API contract

Before writing code, decide and *write down* the exact contract the implementation must obey.

### Inputs

`F32Scalar`’s private `value` is constructed via `F32Scalar::new`, which already:

- canonicalizes `-0.0` → `+0.0`
- canonicalizes `NaN` → `0x7fc0_0000`
- flushes subnormals → `+0.0`

So the trig backend can assume its `self.value` is canonical **as stored**.

### Outputs (required)

For any input, `sin/cos` must return a canonical `F32Scalar`:

- never `-0.0`
- never subnormal
- if NaN, only the canonical NaN bit pattern `0x7fc0_0000`

This can be enforced by ending the computation with `F32Scalar::new(result_f32)`.

### Non-finite inputs (decide explicitly)

The tests currently assume:

- `sin(±∞)` and `cos(±∞)` return NaN (then canonicalized)
- `sin(NaN)` and `cos(NaN)` return NaN (canonical)

Keep this behavior unless/until the spec says otherwise.

Implementation rule:

```text
if !angle.is_finite() => return (NaN, NaN) (canonicalized via F32Scalar::new)
```

---

## Approach overview (recommended)

Use a **lookup table (LUT)** plus simple interpolation:

1. Deterministic range-reduction to a canonical interval (e.g., `[0, TAU)`).
2. Convert the reduced angle to a deterministic table index + fraction.
3. Lookup adjacent samples and interpolate.
4. Apply quadrant symmetries to avoid a full-table footprint (optional but recommended).
5. Wrap results with `F32Scalar::new` for canonicalization.

This keeps:

- determinism: no platform `libm`
- speed: O(1) lookup, few ops
- controllable accuracy: choose table resolution & interpolation

---

## Step-by-step implementation plan

### Step 1 — Pin the table design (N, symmetry, interpolation)

Pick **one** and document it (constants should be checked into the repo).

Recommended starting point:

- `N = 4096` samples over `[0, TAU)` (power of two for cheap masking)
- Linear interpolation between adjacent samples
- Quarter-wave symmetry to reduce table size by ~4× (optional)

Trade-offs:

- Higher `N` lowers error but increases binary size.
- Linear interpolation is easy and deterministic; cubic interpolation may improve accuracy but is more code and more ops.

### Step 2 — Decide how the LUT is stored

Store `u32` bit patterns, not `f32` literals:

- avoids any “float literal parsing” concerns
- makes it easy to diff the table and compute checksums/digests

Pattern:

```rust
const SIN_LUT_BITS: [u32; N] = [ /* ... */ ];
#[inline]
fn sin_lut(i: usize) -> f32 { f32::from_bits(SIN_LUT_BITS[i]) }
```

If you use quarter-wave symmetry, store only the first quadrant (plus endpoint):

- `NQ = N/4`
- store `NQ + 1` entries for `[0, PI/2]` so the boundary is exact and avoids off-by-one wrap issues.

### Step 3 — Add a table module (keep `scalar.rs` readable)

Create a small internal module under `warp-core`:

- Option A: `crates/warp-core/src/math/trig_lut.rs`
- Option B: `crates/warp-core/src/math/scalar_trig.rs`

Prefer a module that:

- exports a single `pub(crate) fn sin_cos_f32(angle: f32) -> (f32, f32)`
- keeps LUT + index math private

Then wire it into `F32Scalar::sin/cos/sin_cos` in:

- `crates/warp-core/src/math/scalar.rs`

### Step 4 — Range reduction (deterministic)

Goal: map any finite `angle` (radians) into a stable interval.

Simplest acceptable form:

- `r = angle.rem_euclid(TAU)`

Notes:

- Use `TAU` from `std::f32::consts::TAU` (already used in the codebase).
- Avoid calling `sin/cos` anywhere in this step.
- Keep the computation in `f32` (not `f64`) initially to avoid cross-type subtlety.

### Step 5 — Map reduced angle to table index + fraction

With `N` samples over `[0, TAU)`:

- `scale = N as f32 / TAU`
- `t = r * scale`  (expected in `[0, N)`)
- `i0 = floor(t)` as usize
- `frac = t - (i0 as f32)` in `[0, 1)`
- `i1 = (i0 + 1) & (N - 1)` if `N` is power-of-two, else modulo

Then linear interpolation:

- `v0 = lut[i0]`
- `v1 = lut[i1]`
- `v = v0 + frac * (v1 - v0)`

Important: ensure the implementation cannot produce out-of-bounds indices at `r == TAU`.

### Step 6 — Use symmetries (optional but recommended)

To reduce table size and keep interpolation stable at quadrant boundaries:

1. Map `r` into quadrant `q ∈ {0,1,2,3}` and local angle `a` in `[0, PI/2]`.
2. Compute `sin(a)` and `cos(a)` from the quarter-wave table (cos via `sin(PI/2 - a)`).
3. Apply signs/swaps based on quadrant:

```text
q=0: ( s,  c) = ( +sin(a), +cos(a) )
q=1: ( s,  c) = ( +cos(a), -sin(a) )
q=2: ( s,  c) = ( -sin(a), -cos(a) )
q=3: ( s,  c) = ( -cos(a), +sin(a) )
```

This avoids table wrap-around edge cases and makes interpolation easier to reason about.

### Step 7 — Canonicalize outputs

At the very end:

- `s = F32Scalar::new(s).to_f32()`
- `c = F32Scalar::new(c).to_f32()`

Or, when returning `F32Scalar`:

- `Self::new(s)`
- `Self::new(c)`

This guarantees:

- `-0.0` becomes `+0.0`
- subnormals flush to zero
- NaNs canonicalize

### Step 8 — Wire into the `Scalar` impl for `F32Scalar`

Update:

- `impl Scalar for F32Scalar` in `crates/warp-core/src/math/scalar.rs`

So that:

- `sin()` / `cos()` call the deterministic backend
- `sin_cos()` calls the backend once (no duplicated range reduction)

### Step 9 — Lock in tests (incrementally)

Use the existing test file:

- `crates/warp-core/tests/deterministic_sin_cos_tests.rs`

Suggested test progression:

1. Keep the special-case “golden bits” test passing (NaN/inf/subnormal handling).
2. Keep the “outputs are canonical” test passing for a sample sweep.
3. Turn on the WIP error-budget test:
   - un-ignore it
   - decide a concrete `max_ulp` and/or `max_abs` threshold
   - commit that threshold with a short rationale in the test doc comment
4. Add a compact “finite golden vector” (optional):
   - pick ~32 angles (including quadrant boundaries and midpoints)
   - assert `sin.to_bits()` and `cos.to_bits()` equal committed constants

### Step 10 — Document the policy compliance

When the backend lands, update:

- `docs/SPEC_DETERMINISTIC_MATH.md` checklist (`sin/cos` deterministic approximation)

Document:

- the chosen LUT resolution/interpolation
- the accepted error budget
- how to regenerate the LUT (if applicable)

---

## LUT generation guidance

The LUT must be deterministic and reproducible.

Two workable strategies:

### Strategy A — Commit the table as data (recommended)

1. Write a tiny generator tool (Rust `xtask` or a script under `scripts/`).
2. Use a known-stable reference implementation to generate high-precision values:
   - If using Python, pin interpreter + deps and emit u32 bits.
   - If using Rust, consider a BigFloat crate or a known “software libm” implementation.
3. Emit `u32` bit patterns into a Rust source file.
4. Commit the generated file so all builds use identical bits.

### Strategy B — Generate at build time (not recommended initially)

Generate LUT in `build.rs` and include it.

Downsides:

- build times increase
- “reproducible builds” become harder to audit

---

## Pitfalls checklist

- Off-by-one at `angle == TAU` after range reduction.
- Table wrap-around (especially if using full-wave LUT without symmetry).
- Using `f32::sin/cos` or any platform `libm` in generation or runtime by accident.
- Accidentally introducing `-0.0` at quadrant boundaries (canonicalize via `F32Scalar::new`).
- Depending on subnormal behavior in intermediate math (prefer to canonicalize at the end; if needed, consider using `F32Scalar` ops internally).

---

## “Done” criteria (for the eventual finish)

- `F32Scalar::sin/cos/sin_cos` no longer call hardware/libc trig.
- `cargo test -p warp-core --test deterministic_sin_cos_tests` passes with **no ignored tests**.
- Determinism policy docs are updated and explain the chosen approximation + error budget.
