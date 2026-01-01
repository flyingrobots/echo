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
    let mut angle: f32 = -TAU;

    while angle <= TAU {
        let (s, c) = deterministic_sin_cos_f32(angle);

        assert_canonical_f32(s);
        assert_canonical_f32(c);

        if angle.is_finite() {
            assert!(!s.is_nan(), "sin must be finite for finite angle={angle}");
            assert!(!c.is_nan(), "cos must be finite for finite angle={angle}");
        }

        angle += step;
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
// TODO(#177): Replace libm-derived reference with a deterministic oracle and pin an explicit budget.
#[ignore = "Reference uses platform libm (see #177); keep ignored unless auditing error budgets"]
fn test_sin_cos_error_budget_wip() {
    // NOTE: This test intentionally measures error against a high-precision-ish
    // reference, but does not yet pin an explicit budget. Once the deterministic
    // backend is implemented, add concrete acceptance thresholds and a compact
    // "golden vector" suite for cross-platform CI.

    let mut max_ulp: u32 = 0;
    let mut max_abs: f32 = 0.0;
    let mut worst_angle: f32 = 0.0;

    let step = TAU / 4096.0;
    let mut angle: f32 = -2.0 * TAU;

    while angle <= 2.0 * TAU {
        let (s, c) = deterministic_sin_cos_f32(angle);

        // Reference: f64 trig, then cast down to float32. This is a measurement
        // baseline only; it is not currently a strict determinism oracle.
        let angle64 = angle as f64;
        let s_ref = (angle64.sin() as f32) + 0.0;
        let c_ref = (angle64.cos() as f32) + 0.0;

        let sin_ulp = ulp_diff(s, s_ref);
        let cos_ulp = ulp_diff(c, c_ref);

        let ulp = sin_ulp.max(cos_ulp);
        if ulp > max_ulp {
            max_ulp = ulp;
            worst_angle = angle;
        }

        max_abs = max_abs.max((s - s_ref).abs());
        max_abs = max_abs.max((c - c_ref).abs());

        angle += step;
    }

    eprintln!("wip trig error: max_ulp={max_ulp} max_abs={max_abs} at angle={worst_angle}");
}
