// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Deterministic `sin`/`cos` backend for float32.
//!
//! This module provides a bit-stable approximation for `sin`/`cos` intended for
//! use inside the simulation loop. It intentionally does **not** call platform
//! transcendentals (`f32::{sin,cos}`), which can vary across hardware/libm.
//!
//! Strategy:
//! - range-reduce to `[0, TAU)` using `rem_euclid`
//! - map into a quarter-wave and use a checked-in lookup table (LUT)
//! - linearly interpolate between adjacent samples
//! - apply quadrant symmetries to reconstruct full-wave `sin` and `cos`

use core::f32;

use super::trig_lut::{sin_qtr_sample, SIN_QTR_SEGMENTS_F32};

/// Deterministic `sin` and `cos` for `f32` radians.
///
/// - For non-finite inputs (NaN/±∞), returns `(NaN, NaN)` (caller canonicalizes).
/// - For finite inputs, returns finite `f32` values in `[-1, 1]`.
pub(crate) fn sin_cos_f32(angle: f32) -> (f32, f32) {
    if !angle.is_finite() {
        return (f32::NAN, f32::NAN);
    }

    let r = angle.rem_euclid(f32::consts::TAU);

    // Range-split into quadrants using comparisons to avoid the subtle
    // rounding hazard where `r / (PI/2)` can round up to 4.0 at the top edge.
    let (quadrant, a) = if r < f32::consts::FRAC_PI_2 {
        (0_u8, r)
    } else if r < f32::consts::PI {
        (1_u8, r - f32::consts::FRAC_PI_2)
    } else if r < (3.0 * f32::consts::FRAC_PI_2) {
        (2_u8, r - f32::consts::PI)
    } else {
        (3_u8, r - (3.0 * f32::consts::FRAC_PI_2))
    };

    let s = sin_qtr_interp(a);
    let c = sin_qtr_interp(f32::consts::FRAC_PI_2 - a);

    match quadrant {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        // 3
        _ => (-c, s),
    }
}

#[inline]
fn sin_qtr_interp(angle_qtr: f32) -> f32 {
    // `angle_qtr` should always be within [0, PI/2] here, but keep behavior
    // defined even if upstream range reduction changes.
    if !(0.0..=f32::consts::FRAC_PI_2).contains(&angle_qtr) {
        return f32::NAN;
    }

    let t = angle_qtr * SIN_QTR_SEGMENTS_F32 / f32::consts::FRAC_PI_2;

    if t >= SIN_QTR_SEGMENTS_F32 {
        // Inclusive endpoint (PI/2) maps to exactly 1.0.
        return 1.0;
    }

    // Safe: 0 <= t < SIN_QTR_SEGMENTS, so i0 in 0..SIN_QTR_SEGMENTS.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let i0 = t as usize;
    let frac = t.fract();
    let y0 = sin_qtr_sample(i0);
    let y1 = sin_qtr_sample(i0 + 1);
    y0 + frac * (y1 - y0)
}
