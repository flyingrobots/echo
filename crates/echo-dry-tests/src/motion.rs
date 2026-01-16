// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Motion payload encoding helpers for tests.

use warp_core::{encode_motion_atom_payload, AtomPayload};

/// Default position used in motion tests: [1.0, 2.0, 3.0]
pub const DEFAULT_MOTION_POSITION: [f32; 3] = [1.0, 2.0, 3.0];

/// Default velocity used in motion tests: [0.5, -1.0, 0.25]
pub const DEFAULT_MOTION_VELOCITY: [f32; 3] = [0.5, -1.0, 0.25];

/// Builder for creating motion payloads in tests.
///
/// # Example
///
/// ```
/// use echo_dry_tests::MotionPayloadBuilder;
///
/// let payload = MotionPayloadBuilder::new()
///     .position([10.0, 20.0, 30.0])
///     .velocity([1.0, 0.0, 0.0])
///     .build();
/// ```
pub struct MotionPayloadBuilder {
    position: [f32; 3],
    velocity: [f32; 3],
}

impl Default for MotionPayloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MotionPayloadBuilder {
    /// Create a new builder with default position and velocity.
    pub fn new() -> Self {
        Self {
            position: DEFAULT_MOTION_POSITION,
            velocity: DEFAULT_MOTION_VELOCITY,
        }
    }

    /// Create a builder with zero position and velocity.
    pub fn zero() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
        }
    }

    /// Set the position.
    pub fn position(mut self, pos: [f32; 3]) -> Self {
        self.position = pos;
        self
    }

    /// Set the velocity.
    pub fn velocity(mut self, vel: [f32; 3]) -> Self {
        self.velocity = vel;
        self
    }

    /// Set position X component.
    pub fn px(mut self, x: f32) -> Self {
        self.position[0] = x;
        self
    }

    /// Set position Y component.
    pub fn py(mut self, y: f32) -> Self {
        self.position[1] = y;
        self
    }

    /// Set position Z component.
    pub fn pz(mut self, z: f32) -> Self {
        self.position[2] = z;
        self
    }

    /// Set velocity X component.
    pub fn vx(mut self, x: f32) -> Self {
        self.velocity[0] = x;
        self
    }

    /// Set velocity Y component.
    pub fn vy(mut self, y: f32) -> Self {
        self.velocity[1] = y;
        self
    }

    /// Set velocity Z component.
    pub fn vz(mut self, z: f32) -> Self {
        self.velocity[2] = z;
        self
    }

    /// Build the atom payload.
    pub fn build(self) -> AtomPayload {
        encode_motion_atom_payload(self.position, self.velocity)
    }

    /// Get the position.
    pub fn get_position(&self) -> [f32; 3] {
        self.position
    }

    /// Get the velocity.
    pub fn get_velocity(&self) -> [f32; 3] {
        self.velocity
    }
}

/// Create a motion payload with default position and velocity.
pub fn default_motion_payload() -> AtomPayload {
    MotionPayloadBuilder::new().build()
}

/// Create a motion payload with zero position and velocity.
pub fn zero_motion_payload() -> AtomPayload {
    MotionPayloadBuilder::zero().build()
}

/// Create a motion payload with the given position and zero velocity.
pub fn stationary_at(position: [f32; 3]) -> AtomPayload {
    MotionPayloadBuilder::zero().position(position).build()
}

/// Create a motion payload with zero position and the given velocity.
pub fn moving_from_origin(velocity: [f32; 3]) -> AtomPayload {
    MotionPayloadBuilder::zero().velocity(velocity).build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp_core::decode_motion_atom_payload;

    #[test]
    fn builder_with_defaults() {
        let payload = MotionPayloadBuilder::new().build();
        let (pos, vel) = decode_motion_atom_payload(&payload).expect("decode");
        assert_eq!(pos, DEFAULT_MOTION_POSITION);
        assert_eq!(vel, DEFAULT_MOTION_VELOCITY);
    }

    #[test]
    fn builder_zero() {
        let payload = MotionPayloadBuilder::zero().build();
        let (pos, vel) = decode_motion_atom_payload(&payload).expect("decode");
        assert_eq!(pos, [0.0, 0.0, 0.0]);
        assert_eq!(vel, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn builder_fluent_api() {
        let payload = MotionPayloadBuilder::new()
            .position([10.0, 20.0, 30.0])
            .velocity([1.0, 2.0, 3.0])
            .build();
        let (pos, vel) = decode_motion_atom_payload(&payload).expect("decode");
        assert_eq!(pos, [10.0, 20.0, 30.0]);
        assert_eq!(vel, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn builder_individual_components() {
        let payload = MotionPayloadBuilder::zero()
            .px(1.0)
            .py(2.0)
            .pz(3.0)
            .vx(4.0)
            .vy(5.0)
            .vz(6.0)
            .build();
        let (pos, vel) = decode_motion_atom_payload(&payload).expect("decode");
        assert_eq!(pos, [1.0, 2.0, 3.0]);
        assert_eq!(vel, [4.0, 5.0, 6.0]);
    }

    #[test]
    fn stationary_at_helper() {
        let payload = stationary_at([5.0, 10.0, 15.0]);
        let (pos, vel) = decode_motion_atom_payload(&payload).expect("decode");
        assert_eq!(pos, [5.0, 10.0, 15.0]);
        assert_eq!(vel, [0.0, 0.0, 0.0]);
    }
}
