// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared viewer preferences used by Echo tools (camera + HUD flags).

use serde::{Deserialize, Serialize};

/// Saved preferences for a viewer surface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ViewerPrefs {
    /// Camera pose and projection.
    pub camera: CameraPrefs,
    /// HUD toggles.
    pub hud: HudPrefs,
}

/// Camera position and projection parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraPrefs {
    /// World-space camera position.
    pub pos: [f32; 3],
    /// Quaternion orientation (x, y, z, w).
    pub orientation: [f32; 4],
    /// Pitch (radians).
    pub pitch: f32,
    /// Vertical field of view (radians).
    pub fov_y: f32,
}

impl Default for CameraPrefs {
    fn default() -> Self {
        Self {
            pos: [0.0, 0.0, 520.0],
            orientation: [0.0, 0.0, 0.0, 1.0],
            pitch: 0.0,
            fov_y: 60f32.to_radians(),
        }
    }
}

/// HUD debug and overlay toggles.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HudPrefs {
    /// Show bounding sphere debug overlay.
    pub debug_show_sphere: bool,
    /// Show arc-drag debug vector.
    pub debug_show_arc: bool,
    /// Invert camera X input.
    pub debug_invert_cam_x: bool,
    /// Invert camera Y input.
    pub debug_invert_cam_y: bool,
    /// Display watermark overlay.
    pub show_watermark: bool,
    /// Present in vsync mode.
    pub vsync: bool,
}
