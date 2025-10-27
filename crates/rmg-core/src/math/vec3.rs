use crate::math::EPSILON;

/// Deterministic 3D vector used throughout the engine.
///
/// Invariants and conventions:
/// - Components encode world‑space meters and may represent either points or
///   directions depending on the calling context.
/// - Operations compute in `f32` to match the runtime’s float32 mode.
/// - For transforms, use [`crate::math::Mat4::transform_point`] for points
///   (homogeneous `w = 1`) and [`crate::math::Mat4::transform_direction`] for
///   directions (homogeneous `w = 0`).
///
/// # Examples
/// Constructing and normalizing:
/// ```
/// use rmg_core::math::Vec3;
/// let v = Vec3::new(3.0, 0.0, 4.0);
/// assert_eq!(v.normalize().to_array(), [0.6, 0.0, 0.8]);
/// ```
/// Basis vectors:
/// ```
/// use rmg_core::math::Vec3;
/// assert_eq!(Vec3::UNIT_X.to_array(), [1.0, 0.0, 0.0]);
/// assert_eq!(Vec3::ZERO.to_array(), [0.0, 0.0, 0.0]);
/// ```
///
/// # Precision
/// - Uses `f32` throughout; operations may accumulate rounding error.
/// - Normalization uses [`EPSILON`](crate::math::EPSILON) to avoid division by
///   very small magnitudes and returns `ZERO` for degenerate inputs.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec3 {
    data: [f32; 3],
}

impl Vec3 {
    /// Standard zero vector (0, 0, 0).
    pub const ZERO: Self = Self { data: [0.0, 0.0, 0.0] };
    /// Unit X basis vector (1, 0, 0).
    pub const UNIT_X: Self = Self { data: [1.0, 0.0, 0.0] };
    /// Unit Y basis vector (0, 1, 0).
    pub const UNIT_Y: Self = Self { data: [0.0, 1.0, 0.0] };
    /// Unit Z basis vector (0, 0, 1).
    pub const UNIT_Z: Self = Self { data: [0.0, 0.0, 1.0] };

    /// Creates a vector from components.
    ///
    /// Inputs are interpreted as meters in world coordinates; callers must
    /// supply finite values.
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

    /// Constructs the zero vector.
    pub const fn zero() -> Self {
        Self::ZERO
    }

    /// Adds two vectors component‑wise.
    pub fn add(&self, other: &Self) -> Self {
        Self::new(
            self.component(0) + other.component(0),
            self.component(1) + other.component(1),
            self.component(2) + other.component(2),
        )
    }

    /// Subtracts another vector component‑wise.
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

    /// Normalizes the vector, returning zero vector if length is ~0.
    ///
    /// Zero‑length inputs remain the zero vector so downstream callers can
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

impl Default for Vec3 {
    fn default() -> Self {
        Self::ZERO
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

impl core::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.component(0) * rhs, self.component(1) * rhs, self.component(2) * rhs)
    }
}

impl core::ops::Mul<Vec3> for f32 {
    type Output = Vec3;
    fn mul(self, rhs: Vec3) -> Self::Output {
        rhs * self
    }
}

impl core::ops::MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}
