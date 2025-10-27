use crate::math::EPSILON;

/// Deterministic 3D vector used throughout the engine.
///
/// * Components encode world-space metres and may represent either points or
///   directions depending on the calling context.
/// * Arithmetic uses `f32` so results round like the runtime's float32 mode.
/// * Use [`crate::math::Mat4::transform_point`] for points (homogeneous `w = 1`)
///   and [`crate::math::Mat4::transform_direction`] for directions (homogeneous
///   `w = 0`).
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec3 {
    data: [f32; 3],
}

impl Vec3 {
    /// Standard zero vector (0, 0, 0).
    pub const ZERO: Self = Self {
        data: [0.0, 0.0, 0.0],
    };
    /// Unit vector pointing along the positive X axis.
    pub const UNIT_X: Self = Self::new(1.0, 0.0, 0.0);

    /// Unit vector pointing along the positive Y axis.
    pub const UNIT_Y: Self = Self::new(0.0, 1.0, 0.0);

    /// Unit vector pointing along the positive Z axis.
    pub const UNIT_Z: Self = Self::new(0.0, 0.0, 1.0);

    /// Creates a vector from components.
    ///
    /// Inputs are interpreted as metres in world coordinates; callers must
    /// ensure values are finite.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { data: [x, y, z] }
    }

    /// Constructs the zero vector.
    pub const fn zero() -> Self {
        Self::ZERO
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

    /// Normalises the vector, returning the zero vector if length ≤ `EPSILON`.
    ///
    /// `EPSILON` is a degeneracy threshold (not numeric precision): vectors
    /// with length ≤ `EPSILON` are considered degenerate and normalized to
    /// zero so downstream callers can detect them deterministically.
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len <= EPSILON {
            return Self::new(0.0, 0.0, 0.0);
        }
        self.scale(1.0 / len)
    }
}

/// Converts a 3-element `[f32; 3]` array into a `Vec3` interpreted as `(x, y, z)`.
///
/// # Examples
/// ```
/// use rmg_core::math::Vec3;
/// let v = Vec3::from([1.0, 2.0, 3.0]);
/// assert_eq!(v.to_array(), [1.0, 2.0, 3.0]);
/// ```
impl From<[f32; 3]> for Vec3 {
    fn from(value: [f32; 3]) -> Self {
        Self { data: value }
    }
}

impl core::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.component(0) + rhs.component(0),
            self.component(1) + rhs.component(1),
            self.component(2) + rhs.component(2),
        )
    }
}

impl core::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(
            self.component(0) - rhs.component(0),
            self.component(1) - rhs.component(1),
            self.component(2) - rhs.component(2),
        )
    }
}

impl core::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(
            self.component(0) * rhs,
            self.component(1) * rhs,
            self.component(2) * rhs,
        )
    }
}

impl core::ops::Mul<Vec3> for f32 {
    type Output = Vec3;
    fn mul(self, rhs: Vec3) -> Self::Output {
        rhs * self
    }
}

impl core::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl core::ops::SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl core::ops::MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}
