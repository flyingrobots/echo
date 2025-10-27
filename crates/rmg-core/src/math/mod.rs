//! Deterministic math helpers covering scalar utilities, linear algebra
//! primitives, quaternions, and timeline-friendly pseudo-random numbers.
//!
//! # Math Overview
//! - Scalar type: all computations use `f32` to mirror runtime float32 mode.
//! - Coordinate system: right-handed; matrices are column-major.
//! - Multiplication order: `Mat4::multiply(a, b)` computes `a * b` (left * right).
//! - Transform conventions:
//!   - Points use homogeneous `w = 1` (`Mat4::transform_point`).
//!   - Directions use homogeneous `w = 0` (`Mat4::transform_direction`).
//! - Epsilon: [`EPSILON`] guards degeneracy (e.g., zero-length vectors).
//! - Determinism: operations avoid platform RNGs and non-deterministic sources.

use std::f32::consts::TAU;

mod mat4;
mod prng;
mod quat;
mod vec3;

pub use mat4::Mat4;
pub use prng::Prng;
pub use quat::Quat;
pub use vec3::Vec3;

/// Global epsilon used by math routines when detecting degenerate values.
pub const EPSILON: f32 = 1e-6;

/// Clamps `value` to the inclusive `[min, max]` range using float32 rounding.
///
/// # Panics
/// Panics if `min > max`.
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    assert!(min <= max, "invalid clamp range: {min} > {max}");
    value.max(min).min(max)
}

/// Converts degrees to radians with float32 precision.
pub fn deg_to_rad(value: f32) -> f32 {
    value * (TAU / 360.0)
}

/// Converts radians to degrees with float32 precision.
pub fn rad_to_deg(value: f32) -> f32 {
    value * (360.0 / TAU)
}
