use crate::math::EPSILON;

/// Deterministic 3D vector used throughout the engine.
///
/// * Components encode world-space metres and may represent either points or
///   directions depending on the calling context.
/// * Arithmetic clamps to `f32` so results match the runtime’s float32 mode.
/// * Use [`crate::math::Mat4::transform_point`] for points (homogeneous `w = 1`)
///   and [`crate::math::Mat4::transform_direction`] for directions (homogeneous
///   `w = 0`).
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec3 {
    data: [f32; 3],
}

impl Vec3 {
    /// Creates a vector from components.
    ///
    /// Inputs are interpreted as metres in world coordinates; callers must
    /// ensure values are finite.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { data: [x, y, z] }
    }

    /// Returns the components as an array.
    pub fn to_array(self) -> [f32; 3] {
        self.data
    }

    pub(crate) fn component(&self, idx: usize) -> f32 {
        self.data[idx]
    }

    /// Adds two vectors.
    pub fn add(&self, other: &Self) -> Self {
        Self::new(
            self.component(0) + other.component(0),
            self.component(1) + other.component(1),
            self.component(2) + other.component(2),
        )
    }

    /// Subtracts another vector.
    pub fn sub(&self, other: &Self) -> Self {
        Self::new(
            self.component(0) - other.component(0),
            self.component(1) - other.component(1),
            self.component(2) - other.component(2),
        )
    }

    /// Scales the vector by a scalar.
    pub fn scale(&self, scalar: f32) -> Self {
        Self::new(
            self.component(0) * scalar,
            self.component(1) * scalar,
            self.component(2) * scalar,
        )
    }

    /// Dot product with another vector.
    pub fn dot(&self, other: &Self) -> f32 {
        self.component(0) * other.component(0)
            + self.component(1) * other.component(1)
            + self.component(2) * other.component(2)
    }

    /// Cross product with another vector.
    pub fn cross(&self, other: &Self) -> Self {
        let ax = self.component(0);
        let ay = self.component(1);
        let az = self.component(2);
        let bx = other.component(0);
        let by = other.component(1);
        let bz = other.component(2);
        Self::new(ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx)
    }

    /// Vector length (magnitude).
    pub fn length(&self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Squared magnitude of the vector.
    pub fn length_squared(&self) -> f32 {
        self.dot(self)
    }

    /// Normalises the vector, returning zero vector if length is ~0.
    ///
    /// Zero-length inputs remain the zero vector so downstream callers can
    /// detect degenerate directions deterministically.
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len <= EPSILON {
            return Self::new(0.0, 0.0, 0.0);
        }
        self.scale(1.0 / len)
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(value: [f32; 3]) -> Self {
        Self { data: value }
    }
}
