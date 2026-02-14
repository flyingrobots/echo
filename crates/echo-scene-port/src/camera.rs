// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Camera state types for scene rendering.

use core::f32::consts::FRAC_PI_3;

/// Camera projection type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionKind {
    /// Perspective projection (objects farther away appear smaller).
    Perspective,
    /// Orthographic projection (no perspective distortion).
    Orthographic,
}

/// Camera state for rendering.
///
/// Defines the view and projection parameters for the scene camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraState {
    /// Camera position in world space.
    pub position: [f32; 3],
    /// Look-at target in world space.
    pub target: [f32; 3],
    /// Up vector.
    pub up: [f32; 3],
    /// Projection type.
    pub projection: ProjectionKind,
    /// Field of view in radians (for perspective).
    pub fov_y_radians: f32,
    /// Orthographic scale (for orthographic).
    pub ortho_scale: f32,
    /// Near clipping plane.
    ///
    /// Values < 0.1 may cause depth buffer precision issues.
    pub near: f32,
    /// Far clipping plane.
    pub far: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            projection: ProjectionKind::Perspective,
            fov_y_radians: FRAC_PI_3, // 60 degrees
            ortho_scale: 10.0,
            near: 0.1,
            far: 10000.0,
        }
    }
}
