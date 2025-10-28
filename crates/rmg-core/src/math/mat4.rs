use crate::math::{Quat, Vec3};

/// Column-major 4×4 matrix matching Echo’s deterministic math layout.
///
/// * Stored in column-major order to align with GPU uploads and ECS storage.
/// * Represents affine transforms; perspective terms are preserved but helper
///   methods treat them homogeneously (`w = 1` for points).
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Mat4 {
    data: [f32; 16],
}

impl Mat4 {
    /// Returns the identity matrix.
    ///
    /// The identity is the multiplicative neutral element for matrices:
    /// `M * I = I * M = M`. Use it as a no‑op transform or as a starting
    /// point for composing transforms.
    pub const fn identity() -> Self {
        Self {
            data: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// Builds a translation matrix in meters.
    ///
    /// Constructs a 4×4 homogeneous translation matrix intended for
    /// transforming points in world space (positioning objects). When using
    /// [`Mat4::transform_point`], the translation is applied; when using
    /// [`Mat4::transform_direction`], translation is ignored (only the upper‑left
    /// 3×3 linear part is used). Matrices are column‑major and the bottom‑right
    /// element is `1.0`.
    pub const fn translation(tx: f32, ty: f32, tz: f32) -> Self {
        Self {
            data: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, tz, 1.0,
            ],
        }
    }

    /// Builds a non-uniform scale matrix.
    ///
    /// Invariants:
    /// - Determinant is `sx * sy * sz`. Any zero component produces a
    ///   degenerate (non-invertible) matrix.
    /// - A negative component reflects about the corresponding axis; an odd
    ///   number of negative components flips handedness.
    pub const fn scale(sx: f32, sy: f32, sz: f32) -> Self {
        Self {
            data: [
                sx, 0.0, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 0.0, sz, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// Builds a rotation matrix around the X axis by `angle` radians.
    ///
    /// Right‑handed convention: positive angles rotate counter‑clockwise when
    /// looking down the +X axis toward the origin. See
    /// [`Mat4::rotation_from_euler`] for the full convention.
    pub fn rotation_x(angle: f32) -> Self {
        let (s_raw, c_raw) = angle.sin_cos();
        let s = if s_raw == 0.0 { 0.0 } else { s_raw };
        let c = if c_raw == 0.0 { 0.0 } else { c_raw };
        let ns = if s == 0.0 { 0.0 } else { -s };
        Self::new([
            1.0, 0.0, 0.0, 0.0, 0.0, c, s, 0.0, 0.0, ns, c, 0.0, 0.0, 0.0, 0.0, 1.0,
        ])
    }

    /// Builds a rotation matrix around the Y axis by `angle` radians.
    ///
    /// Right‑handed convention: positive angles rotate counter‑clockwise when
    /// looking down the +Y axis toward the origin. See
    /// [`Mat4::rotation_from_euler`] for the full convention.
    pub fn rotation_y(angle: f32) -> Self {
        let (s_raw, c_raw) = angle.sin_cos();
        let s = if s_raw == 0.0 { 0.0 } else { s_raw };
        let c = if c_raw == 0.0 { 0.0 } else { c_raw };
        let ns = if s == 0.0 { 0.0 } else { -s };
        Self::new([
            c, 0.0, ns, 0.0, 0.0, 1.0, 0.0, 0.0, s, 0.0, c, 0.0, 0.0, 0.0, 0.0, 1.0,
        ])
    }

    /// Builds a rotation matrix around the Z axis by `angle` radians.
    ///
    /// Right‑handed convention: positive angles rotate counter‑clockwise when
    /// looking down the +Z axis toward the origin. See
    /// [`Mat4::rotation_from_euler`] for the full convention.
    pub fn rotation_z(angle: f32) -> Self {
        let (s_raw, c_raw) = angle.sin_cos();
        let s = if s_raw == 0.0 { 0.0 } else { s_raw };
        let c = if c_raw == 0.0 { 0.0 } else { c_raw };
        let ns = if s == 0.0 { 0.0 } else { -s };
        Self::new([
            c, s, 0.0, 0.0, ns, c, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ])
    }

    /// Builds a rotation matrix from Euler angles in radians.
    ///
    /// Convention and order:
    /// - Constructs `R = R_y(yaw) * R_x(pitch) * R_z(roll)`.
    /// - Matrix multiplication in the code is performed left-to-right in this
    ///   same order, so the rightmost rotation (`R_z`) is applied first when
    ///   transforming a vector.
    /// - Matrices are intended for column vectors with transforms of the form
    ///   `M * v` (column-major storage; no implicit transpose).
    pub fn rotation_from_euler(yaw: f32, pitch: f32, roll: f32) -> Self {
        Self::rotation_y(yaw)
            .multiply(&Self::rotation_x(pitch))
            .multiply(&Self::rotation_z(roll))
    }

    /// Constructs a rotation matrix from an axis and angle in radians.
    ///
    /// The `axis` argument does not need to be pre‑normalised; it is
    /// normalised internally. If a zero‑length axis is supplied, the identity
    /// matrix is returned (behaviour delegated to
    /// [`Quat::from_axis_angle`](crate::math::Quat::from_axis_angle)).
    pub fn rotation_axis_angle(axis: Vec3, angle: f32) -> Self {
        Self::from_quat(&Quat::from_axis_angle(axis, angle))
    }

    /// Constructs a rotation matrix from a quaternion.
    ///
    /// Expects a unit (normalised) quaternion for a pure rotation. Passing an
    /// unnormalised quaternion scales the resulting matrix. Component order is
    /// `(x, y, z, w)` to match [`Quat`]. See [`Quat`] for construction and
    /// normalisation helpers.
    pub fn from_quat(q: &Quat) -> Self {
        q.to_mat4()
    }
    /// Creates a matrix from column-major array data.
    ///
    /// Callers must supply 16 finite values already laid out column-major.
    pub const fn new(data: [f32; 16]) -> Self {
        Self { data }
    }

    /// Returns the matrix as a column-major array.
    pub const fn to_array(self) -> [f32; 16] {
        self.data
    }

    fn at(&self, row: usize, col: usize) -> f32 {
        self.data[col * 4 + row]
    }

    /// Multiplies the matrix with another matrix (`self * rhs`).
    ///
    /// Multiplication follows column-major semantics (`self` on the left,
    /// rhs on the right) to mirror GPU-style transforms.
    pub fn multiply(&self, rhs: &Self) -> Self {
        let mut out = [0.0; 16];
        for row in 0..4 {
            for col in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.at(row, k) * rhs.at(k, col);
                }
                out[col * 4 + row] = sum;
            }
        }
        Self::new(out)
    }

    /// Transforms a point (assumes `w = 1`, no perspective divide).
    ///
    /// Translation components are applied and the resulting vector is returned
    /// with `w` implicitly equal to `1`.
    pub fn transform_point(&self, point: &Vec3) -> Vec3 {
        let x = point.component(0);
        let y = point.component(1);
        let z = point.component(2);
        let w = 1.0;

        let nx = self.at(0, 0) * x + self.at(0, 1) * y + self.at(0, 2) * z + self.at(0, 3) * w;
        let ny = self.at(1, 0) * x + self.at(1, 1) * y + self.at(1, 2) * z + self.at(1, 3) * w;
        let nz = self.at(2, 0) * x + self.at(2, 1) * y + self.at(2, 2) * z + self.at(2, 3) * w;

        Vec3::new(nx, ny, nz)
    }

    /// Transforms a direction vector (ignores translation, `w = 0`).
    ///
    /// Only the rotational and scaling parts of the matrix affect the result.
    pub fn transform_direction(&self, direction: &Vec3) -> Vec3 {
        let x = direction.component(0);
        let y = direction.component(1);
        let z = direction.component(2);

        let nx = self.at(0, 0) * x + self.at(0, 1) * y + self.at(0, 2) * z;
        let ny = self.at(1, 0) * x + self.at(1, 1) * y + self.at(1, 2) * z;
        let nz = self.at(2, 0) * x + self.at(2, 1) * y + self.at(2, 2) * z;

        Vec3::new(nx, ny, nz)
    }
}

impl From<[f32; 16]> for Mat4 {
    fn from(value: [f32; 16]) -> Self {
        Self { data: value }
    }
}

impl core::ops::Mul for Mat4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        self.multiply(&rhs)
    }
}

impl core::ops::Mul<&Mat4> for &Mat4 {
    type Output = Mat4;
    fn mul(self, rhs: &Mat4) -> Self::Output {
        self.multiply(rhs)
    }
}

impl core::ops::Mul<&Mat4> for Mat4 {
    type Output = Mat4;
    fn mul(self, rhs: &Mat4) -> Self::Output {
        self.multiply(rhs)
    }
}

impl core::ops::Mul<Mat4> for &Mat4 {
    type Output = Mat4;
    fn mul(self, rhs: Mat4) -> Self::Output {
        self.multiply(&rhs)
    }
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl core::ops::MulAssign<Mat4> for Mat4 {
    fn mul_assign(&mut self, rhs: Mat4) {
        *self = self.multiply(&rhs);
    }
}

impl core::ops::MulAssign<&Mat4> for Mat4 {
    fn mul_assign(&mut self, rhs: &Mat4) {
        *self = self.multiply(rhs);
    }
}
