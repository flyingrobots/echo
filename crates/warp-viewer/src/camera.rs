// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Camera math and controls.

use glam::{Mat4, Quat, Vec2, Vec3};
use std::f32::consts::PI;

pub const MAX_PITCH: f32 = PI * 0.5 - 0.01;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub pos: Vec3,
    pub orientation: Quat,
    pub pitch: f32,
    pub fov_y: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Vec3::new(0.0, 0.0, 300.0),
            orientation: Quat::IDENTITY,
            pitch: 0.0,
            fov_y: 60f32.to_radians(),
        }
    }
}

impl Camera {
    fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let forward = self.orientation * -Vec3::Z;
        let right = self.orientation * Vec3::X;
        let up = self.orientation * Vec3::Y;
        (forward, right, up)
    }

    pub fn view_proj(&self, aspect: f32, radius: f32) -> Mat4 {
        let (f, _, u) = self.basis();
        let view = Mat4::look_to_rh(self.pos, f, u);
        let proj = Mat4::perspective_rh(
            self.fov_y,
            aspect.max(0.1),
            0.1,
            (radius + 5000.0).max(10.0),
        );
        proj * view
    }

    pub fn pick_ray(&self, ndc: Vec2, aspect: f32) -> Vec3 {
        let (f, r, u) = self.basis();
        let t = (self.fov_y * 0.5).tan();
        (f + r * (ndc.x * t * aspect) + u * (ndc.y * t)).normalize()
    }

    pub fn zoom_fov(&mut self, scale: f32) {
        self.fov_y = (self.fov_y * scale).clamp(15f32.to_radians(), 120f32.to_radians());
    }

    pub fn move_relative(&mut self, delta: Vec3) {
        let (f, r, u) = self.basis();
        self.pos += f * delta.z + r * delta.x + u * delta.y;
    }

    pub fn rotate_by_mouse(&mut self, delta: Vec2, invert_x: bool, invert_y: bool) {
        let sensitivity = 0.0025;
        let yaw_delta = delta.x * sensitivity * if invert_x { -1.0 } else { 1.0 };
        let pitch_delta = (-delta.y) * sensitivity * if invert_y { -1.0 } else { 1.0 };

        let yaw_q = Quat::from_axis_angle(Vec3::Y, yaw_delta);
        self.orientation = yaw_q * self.orientation;

        let new_pitch = (self.pitch + pitch_delta).clamp(-MAX_PITCH, MAX_PITCH);
        let applied = new_pitch - self.pitch;
        if applied.abs() > 1e-6 {
            let right = self.orientation * Vec3::X;
            let pitch_q = Quat::from_axis_angle(right, applied);
            self.orientation = pitch_q * self.orientation;
            self.pitch = new_pitch;
        }
        self.orientation = self.orientation.normalize();
    }
}
