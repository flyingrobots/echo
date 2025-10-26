use crate::math::{EPSILON, Mat4, Vec3};

/// Quaternion stored as `(x, y, z, w)` with deterministic float32 rounding.
///
/// * All angles are expressed in radians.
/// * Normalisation clamps to `f32` to match runtime behaviour.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Quat {
    data: [f32; 4],
}

impl Quat {
    /// Creates a quaternion from components.
    ///
    /// Callers should provide finite components; use
    /// [`Quat::from_axis_angle`] for axis/angle construction.
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { data: [x, y, z, w] }
    }

    /// Returns the quaternion as an array.
    pub fn to_array(self) -> [f32; 4] {
        self.data
    }

    fn component(&self, idx: usize) -> f32 {
        self.data[idx]
    }

    /// Constructs a quaternion from a rotation axis and angle in radians.
    ///
    /// Returns the identity quaternion when the axis has zero length to avoid
    /// undefined orientations and preserve deterministic behaviour.
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let len_sq = axis.length_squared();
        if len_sq <= EPSILON * EPSILON {
            return Self::identity();
        }
        let len = len_sq.sqrt();
        let norm_axis = axis.scale(1.0 / len);
        let half = angle * 0.5;
        let (sin_half, cos_half) = half.sin_cos();
        let scaled = norm_axis.scale(sin_half);
        Self::new(
            scaled.component(0),
            scaled.component(1),
            scaled.component(2),
            cos_half,
        )
    }

    /// Multiplies two quaternions (`self * other`).
    pub fn multiply(&self, other: &Self) -> Self {
        let ax = self.component(0);
        let ay = self.component(1);
        let az = self.component(2);
        let aw = self.component(3);

        let bx = other.component(0);
        let by = other.component(1);
        let bz = other.component(2);
        let bw = other.component(3);

        Self::new(
            aw * bx + ax * bw + ay * bz - az * by,
            aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw,
            aw * bw - ax * bx - ay * by - az * bz,
        )
    }

    /// Normalises the quaternion; returns identity when norm is ~0.
    pub fn normalize(&self) -> Self {
        let len = (self.component(0) * self.component(0)
            + self.component(1) * self.component(1)
            + self.component(2) * self.component(2)
            + self.component(3) * self.component(3))
        .sqrt();
        if len <= EPSILON {
            return Self::identity();
        }
        let inv = 1.0 / len;
        Self::new(
            self.component(0) * inv,
            self.component(1) * inv,
            self.component(2) * inv,
            self.component(3) * inv,
        )
    }

    /// Returns the identity quaternion.
    pub const fn identity() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    /// Converts the quaternion to a rotation matrix (column-major 4×4).
    pub fn to_mat4(&self) -> Mat4 {
        let q = self.normalize();
        let x = q.component(0);
        let y = q.component(1);
        let z = q.component(2);
        let w = q.component(3);

        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let xy = x * y;
        let xz = x * z;
        let yz = y * z;
        let wx = w * x;
        let wy = w * y;
        let wz = w * z;

        Mat4::new([
            1.0 - 2.0 * (yy + zz),
            2.0 * (xy + wz),
            2.0 * (xz - wy),
            0.0,
            2.0 * (xy - wz),
            1.0 - 2.0 * (xx + zz),
            2.0 * (yz + wx),
            0.0,
            2.0 * (xz + wy),
            2.0 * (yz - wx),
            1.0 - 2.0 * (xx + yy),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        ])
    }
}

impl From<[f32; 4]> for Quat {
    fn from(value: [f32; 4]) -> Self {
        Self { data: value }
    }
}
