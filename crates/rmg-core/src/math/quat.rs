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
    /// Components are interpreted as `(x, y, z, w)` with `w` the scalar part.
    /// In debug builds this asserts that all components are finite; in release
    /// builds construction is unchecked. Prefer [`Quat::from_axis_angle`] for
    /// axis/angle construction when possible.
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        debug_assert!(x.is_finite() && y.is_finite() && z.is_finite() && w.is_finite());
        Self { data: [x, y, z, w] }
    }

    /// Returns the quaternion as an array `[x, y, z, w]`.
    pub fn to_array(self) -> [f32; 4] {
        self.data
    }

    fn component(&self, idx: usize) -> f32 {
        self.data[idx]
    }

    /// Constructs a quaternion from a rotation axis and angle in radians.
    ///
    /// Returns the identity quaternion when the axis length is ≤ `EPSILON` to avoid
    /// undefined orientations and preserve deterministic behaviour. No small-angle approximation is applied.
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

    /// Hamilton product of two quaternions (`self * other`).
    ///
    /// Operand order matters: the result composes the rotation represented by
    /// `other` followed by the rotation represented by `self`. Quaternion
    /// multiplication is non‑commutative, so reversing the order yields a
    /// different orientation in general.
    ///
    /// Component layout is `(x, y, z, w)` with `w` as the scalar part. Inputs
    /// need not be normalized; however, when both operands are unit
    /// quaternions, the result represents the composed rotation and remains a
    /// unit quaternion up to floating‑point error (consider re‑normalizing over
    /// long chains).
    ///
    /// # Examples
    /// ```
    /// use core::f32::consts::FRAC_PI_2;
    /// use rmg_core::math::{Quat, Vec3};
    /// // Compose: 90° pitch around X, then 90° yaw around Y
    /// let pitch = Quat::from_axis_angle(Vec3::from([1.0, 0.0, 0.0]), FRAC_PI_2);
    /// let yaw = Quat::from_axis_angle(Vec3::from([0.0, 1.0, 0.0]), FRAC_PI_2);
    /// let composed = yaw.multiply(&pitch); // pitch first, then yaw
    /// // Reversing order gives different result
    /// let reversed = pitch.multiply(&yaw);
    /// assert_ne!(composed.to_array(), reversed.to_array());
    /// ```
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

    /// Returns a unit quaternion (magnitude 1) pointing in the same direction.
    ///
    /// Quaternion operations can accumulate floating-point error; normalise
    /// periodically to maintain unit length for accurate rotations. If the
    /// magnitude is ≤ `EPSILON`, returns the identity quaternion to avoid
    /// division by near‑zero (a degenerate quaternion cannot represent a rotation).
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
    ///
    /// Represents no rotation (the multiplicative identity for quaternion
    /// multiplication).
    pub const fn identity() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    /// Converts the quaternion to a 4×4 rotation matrix in column‑major order.
    ///
    /// The quaternion is normalised before conversion to ensure a valid
    /// rotation matrix. The resulting matrix is a homogeneous transform with
    /// the rotation in the upper‑left 3×3 block and `[0, 0, 0, 1]` in the last
    /// row and column. Use this to integrate quaternion rotations into
    /// matrix‑based pipelines and composition with translations/scales.
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

/// Converts a 4‑element `[f32; 4]` array `(x, y, z, w)` into a `Quat`.
/// The components are taken verbatim; callers typically pass unit quaternions
/// for rotations, but normalization is not enforced by this conversion.
impl From<[f32; 4]> for Quat {
    fn from(value: [f32; 4]) -> Self {
        Self { data: value }
    }
}
