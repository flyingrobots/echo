//! Deterministic math helpers covering scalar utilities, linear algebra
//! primitives, quaternions, and timeline-friendly pseudo-random numbers.
//!
//! All operations round to `f32` to mirror the runtime’s float32 mode.

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
