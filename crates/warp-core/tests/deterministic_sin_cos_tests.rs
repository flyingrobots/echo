// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]

use std::f32;

use warp_core::math::scalar::F32Scalar;
use warp_core::math::Scalar;

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
    let a_bits = a.to_bits();
    let b_bits = b.to_bits();

    // "Ordered float" mapping (flip sign bit ordering) so abs_diff matches ULP distance.
    let ai = a_bits ^ ((((a_bits as i32) >> 31) as u32) >> 1);
    let bi = b_bits ^ ((((b_bits as i32) >> 31) as u32) >> 1);
    ai.abs_diff(bi)
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
    const ZERO: u32 = 0x0000_0000;
    const ONE: u32 = 0x3f80_0000;
    const CANON_NAN: u32 = 0x7fc0_0000;

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
    let step = f32::consts::TAU / 1024.0;
    let mut angle: f32 = -f32::consts::TAU;

    while angle <= f32::consts::TAU {
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
#[ignore = "WIP: deterministic trig backend (LUT/polynomial) not implemented yet"]
fn test_sin_cos_error_budget_wip() {
    // NOTE: This test intentionally measures error against a high-precision-ish
    // reference, but does not yet pin an explicit budget. Once the deterministic
    // backend is implemented, add concrete acceptance thresholds and a compact
    // "golden vector" suite for cross-platform CI.

    let mut max_ulp: u32 = 0;
    let mut max_abs: f32 = 0.0;
    let mut worst_angle: f32 = 0.0;

    let step = f32::consts::TAU / 4096.0;
    let mut angle: f32 = -2.0 * f32::consts::TAU;

    while angle <= 2.0 * f32::consts::TAU {
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
