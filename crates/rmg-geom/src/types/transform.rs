// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

use rmg_core::math::{Mat4, Quat, Vec3};

/// Rigid transform with non-uniform scale used by broad-phase and shape placement.
///
/// Conventions:
/// - `translation` in meters (world space).
/// - `rotation` as a unit quaternion (normalized internally when converting).
/// - `scale` is non-uniform and applied before rotation/translation.
///
/// Determinism:
/// - `to_mat4` constructs `M = T * R * S` with `f32` ops; no FMA to keep
///   results stable across CPUs/targets. Negative scales are supported but may
///   flip handedness; downstream collision policies should document any
///   restrictions if introduced later.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Transform {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl Transform {
    /// Identity transform (no translation, no rotation, unit scale).
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            translation: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }

    /// Creates a transform from components.
    #[must_use]
    pub const fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Translation component.
    #[must_use]
    pub fn translation(&self) -> Vec3 {
        self.translation
    }

    /// Rotation component.
    #[must_use]
    pub fn rotation(&self) -> Quat {
        self.rotation
    }

    /// Scale component.
    #[must_use]
    pub fn scale(&self) -> Vec3 {
        self.scale
    }

    /// Returns the column-major `Mat4` corresponding to this transform.
    #[must_use]
    pub fn to_mat4(&self) -> Mat4 {
        // M = T * R * S (column-major)
        let [sx, sy, sz] = self.scale.to_array();
        let [tx, ty, tz] = self.translation.to_array();
        // Scale matrix
        let s = Mat4::new([
            sx, 0.0, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 0.0, sz, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]);
        // Rotation from quaternion (provided by rmg-core)
        let r = self.rotation.to_mat4();
        // Translation matrix (translation in last column)
        let t = Mat4::new([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, tz, 1.0,
        ]);
        t.multiply(&r).multiply(&s)
    }
}
