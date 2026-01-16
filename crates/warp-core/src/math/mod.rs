// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Deterministic math helpers covering scalar utilities, linear algebra
//! primitives, quaternions, and timeline-friendly pseudo-random numbers.
//!
//! All operations round to `f32` to mirror the runtime’s float32 mode.

use std::f32::consts::TAU;

/// Deterministic Q32.32 conversion helpers used by fixed-point lanes and payload codecs.
pub mod fixed_q32_32;
mod mat4;
mod prng;
mod quat;
pub mod scalar;
mod trig;
mod trig_lut;
mod vec3;

#[doc(inline)]
pub use mat4::Mat4;
#[doc(inline)]
pub use prng::Prng;
#[doc(inline)]
pub use quat::Quat;
#[doc(inline)]
pub use scalar::Scalar;
#[doc(inline)]
pub use vec3::Vec3;

/// Degeneracy threshold used by math routines to detect near-zero magnitudes.
///
/// This is not a generic numeric-precision epsilon; it is used to classify
/// vectors/quaternions with magnitude ≤ `EPSILON` as degenerate so that
/// operations like normalization can return stable, deterministic sentinels
/// (e.g., the zero vector or identity quaternion).
pub const EPSILON: f32 = 1e-6;

/// Clamps `value` to the inclusive `[min, max]` range using float32 rounding.
///
/// # Panics
/// Panics if `min > max`.
///
/// # NaN handling
/// If `value`, `min`, or `max` is `NaN`, the result is `NaN`. Callers must
/// ensure inputs are finite if deterministic behavior is required.
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    assert!(min <= max, "invalid clamp range: {min} > {max}");
    value.clamp(min, max)
}

/// Converts degrees to radians with float32 precision.
pub fn deg_to_rad(value: f32) -> f32 {
    value * (TAU / 360.0)
}

/// Converts radians to degrees with float32 precision.
pub fn rad_to_deg(value: f32) -> f32 {
    value * (360.0 / TAU)
}
