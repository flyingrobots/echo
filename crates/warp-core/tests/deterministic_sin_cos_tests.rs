// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]

use std::f32::consts::TAU;

use warp_core::math::scalar::F32Scalar;
use warp_core::math::Scalar;

const ZERO: u32 = 0x0000_0000;
const ONE: u32 = 0x3f80_0000;
const CANON_NAN: u32 = 0x7fc0_0000;

/// Temporary helper for tests while the deterministic trig backend is in flux.
///
/// Once `F32Scalar::{sin,cos,sin_cos}` are backed by a deterministic LUT or
/// polynomial, these tests should validate both determinism and error budgets.
fn deterministic_sin_cos_f32(angle: f32) -> (f32, f32) {
    let scalar = F32Scalar::new(angle);
    let (s, c) = scalar.sin_cos();
    (s.to_f32(), c.to_f32())
}

fn oracle_sin_cos_f64(angle: f32) -> (f64, f64) {
    let angle64 = angle as f64;
    (libm::sin(angle64), libm::cos(angle64))
}

fn ulp_diff(a: f32, b: f32) -> u32 {
    if a.is_nan() || b.is_nan() {
        return u32::MAX;
    }

    let a_bits = a.to_bits();
    let b_bits = b.to_bits();

    // "Ordered float" mapping so `abs_diff` matches ULP distance:
    // - negative floats map to the lower half of the integer range (in order)
    // - positive floats map to the upper half (in order)
    fn ordered(bits: u32) -> u32 {
        if bits & 0x8000_0000 != 0 {
            !bits
        } else {
            bits | 0x8000_0000
        }
    }

    ordered(a_bits).abs_diff(ordered(b_bits))
}

fn assert_canonical_f32(value: f32) {
    let bits = value.to_bits();

    // Canonicalize -0.0 => +0.0.
    assert_ne!(bits, 0x8000_0000, "value must never be -0.0");

    // Determinism policy: flush subnormals to +0.0.
    assert!(!value.is_subnormal(), "value must never be subnormal");

    // Determinism policy: canonicalize NaNs to a single bit-pattern.
    if value.is_nan() {
        assert_eq!(bits, 0x7fc0_0000, "NaN must be canonical");
    }
}

#[test]
fn test_trig_special_cases_golden_bits() {
    // (angle_bits, expected_sin_bits, expected_cos_bits)
    let vectors: &[(u32, u32, u32)] = &[
        // 0 and -0 are canonicalized to +0; sin(0)=0, cos(0)=1.
        (0x0000_0000, ZERO, ONE),
        (0x8000_0000, ZERO, ONE),
        // Subnormals are flushed to +0 at construction time.
        (0x0000_0001, ZERO, ONE),
        (0x8000_0001, ZERO, ONE),
        (0x007f_ffff, ZERO, ONE),
        (0x807f_ffff, ZERO, ONE),
        // Infinities yield NaN for sin/cos (then canonicalized).
        (0x7f80_0000, CANON_NAN, CANON_NAN),
        (0xff80_0000, CANON_NAN, CANON_NAN),
        // NaNs are canonicalized before and after trig.
        (0x7fc0_0000, CANON_NAN, CANON_NAN),
        (0xffc0_0000, CANON_NAN, CANON_NAN),
        (0x7f80_dead, CANON_NAN, CANON_NAN),
    ];

    for (angle_bits, expected_sin_bits, expected_cos_bits) in vectors {
        let angle = f32::from_bits(*angle_bits);
        let (s, c) = deterministic_sin_cos_f32(angle);

        assert_eq!(
            s.to_bits(),
            *expected_sin_bits,
            "sin bits mismatch for angle_bits={:#010x}",
            angle_bits
        );
        assert_eq!(
            c.to_bits(),
            *expected_cos_bits,
            "cos bits mismatch for angle_bits={:#010x}",
            angle_bits
        );
    }
}

#[test]
fn test_trig_outputs_are_canonical_over_sample_range() {
    let step = TAU / 1024.0;
    for i in 0..=2048_u32 {
        let angle = -TAU + (i as f32) * step;
        let (s, c) = deterministic_sin_cos_f32(angle);

        assert_canonical_f32(s);
        assert_canonical_f32(c);

        if angle.is_finite() {
            assert!(!s.is_nan(), "sin must be finite for finite angle={angle}");
            assert!(!c.is_nan(), "cos must be finite for finite angle={angle}");
        }
    }
}

#[test]
fn test_trig_known_angle_golden_bits() {
    // These angles are chosen to be exactly representable `f32` constants so
    // that this test is stable across platforms and toolchains.
    //
    // (angle_bits, expected_sin_bits, expected_cos_bits)
    let vectors: &[(u32, u32, u32)] = &[
        // pi/8
        (0x3ec9_0fdb, 0x3ec3_ef15, 0x3f6c_835e),
        // pi/4
        (0x3f49_0fdb, 0x3f35_04f3, 0x3f35_04f3),
        // pi/2
        (0x3fc9_0fdb, 0x3f80_0000, 0x0000_0000),
        // pi
        (0x4049_0fdb, 0x0000_0000, 0xbf80_0000),
        // 3pi/2
        (0x4096_cbe4, 0xbf80_0000, 0x0000_0000),
        // 2pi
        (0x40c9_0fdb, 0x0000_0000, 0x3f80_0000),
        // -pi/8
        (0xbec9_0fdb, 0xbec3_ef15, 0x3f6c_835e),
    ];

    for (angle_bits, expected_sin_bits, expected_cos_bits) in vectors {
        let angle = f32::from_bits(*angle_bits);
        let (s, c) = deterministic_sin_cos_f32(angle);

        assert_eq!(
            s.to_bits(),
            *expected_sin_bits,
            "sin bits mismatch for angle_bits={:#010x}",
            angle_bits
        );
        assert_eq!(
            c.to_bits(),
            *expected_cos_bits,
            "cos bits mismatch for angle_bits={:#010x}",
            angle_bits
        );
    }
}

#[test]
fn test_sin_cos_error_budget_pinned_against_deterministic_oracle() {
    // Deterministic oracle:
    // - Uses the pure-Rust `libm` crate so the reference does not depend on the
    //   host platform's libc/libm implementation.
    // - Compares our deterministic float32 output against the libm reference in
    //   two ways:
    //   - ULP budget: measured in f32 space vs the f32-rounded reference.
    //   - Absolute error budget: measured in f64 space vs the f64 reference.

    // NOTE: These thresholds are pinned to the current LUT+interpolation
    // backend in `warp_core::math::trig` and should only be loosened with an
    // explicit decision-log entry.
    //
    // ULP metrics across a zero crossing are not especially meaningful, so we
    // only apply the ULP budget when the f32-rounded reference magnitude is
    // reasonably away from zero.
    // Only apply ULP budgeting when the reference is "large enough" that ULPs
    // are a stable, meaningful metric. Near zero, ULP distance tends to
    // over-penalize small sign/magnitude differences that are better measured
    // with an absolute-error bound.
    const MIN_ULP_MAG: f32 = 0.25;

    // TODO(#177): Tighten these once we have an explicit error budget decision
    // and a longer audit run (e.g. denser sampling or a wider domain).
    const MAX_ULP_BUDGET: u32 = 16;
    const MAX_ABS_BUDGET: f64 = 5.0e-7;

    let mut max_ulp: u32 = 0;
    let mut max_abs: f64 = 0.0;
    let mut worst_angle_ulp: f32 = 0.0;
    let mut worst_angle_abs: f32 = 0.0;
    let mut worst_s: f32 = 0.0;
    let mut worst_c: f32 = 0.0;
    let mut worst_s_ref32: f32 = 0.0;
    let mut worst_c_ref32: f32 = 0.0;
    let mut worst_s_ref64: f64 = 0.0;
    let mut worst_c_ref64: f64 = 0.0;
    let mut sign_mismatch_count: u32 = 0;
    let mut sign_mismatch_worst_angle: f32 = 0.0;

    let step = TAU / 4096.0;
    for i in 0..=16_384_u32 {
        let angle = -2.0 * TAU + (i as f32) * step;
        let (s, c) = deterministic_sin_cos_f32(angle);

        let (s_ref64, c_ref64) = oracle_sin_cos_f64(angle);
        let s_ref32 = F32Scalar::new(s_ref64 as f32).to_f32();
        let c_ref32 = F32Scalar::new(c_ref64 as f32).to_f32();

        // ULP budget only when the reference is sufficiently away from zero.
        // When near zero, use the absolute budget instead.
        if s_ref32.abs() >= MIN_ULP_MAG {
            if s.is_sign_negative() != s_ref32.is_sign_negative() {
                sign_mismatch_count = sign_mismatch_count.saturating_add(1);
                sign_mismatch_worst_angle = angle;
            }

            let sin_ulp = ulp_diff(s, s_ref32);
            if sin_ulp > max_ulp {
                max_ulp = sin_ulp;
                worst_angle_ulp = angle;
                worst_s = s;
                worst_s_ref32 = s_ref32;
                worst_s_ref64 = s_ref64;
                // Keep cos context for debugging, even though sin drove max_ulp.
                worst_c = c;
                worst_c_ref32 = c_ref32;
                worst_c_ref64 = c_ref64;
            }
        }
        if c_ref32.abs() >= MIN_ULP_MAG {
            if c.is_sign_negative() != c_ref32.is_sign_negative() {
                sign_mismatch_count = sign_mismatch_count.saturating_add(1);
                sign_mismatch_worst_angle = angle;
            }

            let cos_ulp = ulp_diff(c, c_ref32);
            if cos_ulp > max_ulp {
                max_ulp = cos_ulp;
                worst_angle_ulp = angle;
                worst_c = c;
                worst_c_ref32 = c_ref32;
                worst_c_ref64 = c_ref64;
                // Keep sin context for debugging, even though cos drove max_ulp.
                worst_s = s;
                worst_s_ref32 = s_ref32;
                worst_s_ref64 = s_ref64;
            }
        }

        let sin_abs = ((s as f64) - s_ref64).abs();
        let cos_abs = ((c as f64) - c_ref64).abs();
        let abs = sin_abs.max(cos_abs);
        if abs > max_abs {
            max_abs = abs;
            worst_angle_abs = angle;
        }
    }

    if std::env::var("ECHO_TRIG_AUDIT_PRINT").is_ok() {
        eprintln!(
            "trig error audit (oracle=libm): max_ulp={max_ulp} (angle={worst_angle_ulp}) max_abs={max_abs:e} (angle={worst_angle_abs})"
        );
        eprintln!(
            "worst_ulp details: s={worst_s} (0x{sb:08x}) c={worst_c} (0x{cb:08x}) s_ref32={worst_s_ref32} (0x{srb:08x}) c_ref32={worst_c_ref32} (0x{crb:08x}) s_ref64={worst_s_ref64:e} c_ref64={worst_c_ref64:e}",
            sb = worst_s.to_bits(),
            cb = worst_c.to_bits(),
            srb = worst_s_ref32.to_bits(),
            crb = worst_c_ref32.to_bits(),
        );
        eprintln!(
            "sign mismatch count (|ref| >= {MIN_ULP_MAG}): {sign_mismatch_count} (last angle={sign_mismatch_worst_angle})"
        );
    }

    assert_eq!(
        sign_mismatch_count, 0,
        "trig sign mismatch beyond near-zero tolerance: count={sign_mismatch_count} (example angle={sign_mismatch_worst_angle})"
    );
    assert!(
        max_ulp <= MAX_ULP_BUDGET,
        "trig ULP budget exceeded: max_ulp={max_ulp} budget={MAX_ULP_BUDGET} worst_angle={worst_angle_ulp}"
    );
    assert!(
        max_abs <= MAX_ABS_BUDGET,
        "trig abs-error budget exceeded: max_abs={max_abs:e} budget={MAX_ABS_BUDGET:e} worst_angle={worst_angle_abs}"
    );
}
