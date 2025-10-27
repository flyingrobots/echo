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
    pub const fn identity() -> Self {
        Self {
            data: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// Builds a translation matrix in meters.
    pub const fn translation(tx: f32, ty: f32, tz: f32) -> Self {
        Self {
            data: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                tx,  ty,  tz,  1.0,
            ],
        }
    }

    /// Builds a non-uniform scale matrix.
    pub const fn scale(sx: f32, sy: f32, sz: f32) -> Self {
        Self {
            data: [
                sx,  0.0, 0.0, 0.0,
                0.0, sy,  0.0, 0.0,
                0.0, 0.0, sz,  0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// Builds a rotation matrix around the X axis by `angle` radians.
    pub fn rotation_x(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self::new([
            1.0, 0.0, 0.0, 0.0,
            0.0, c,   s,   0.0,
            0.0, -s,  c,   0.0,
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    /// Builds a rotation matrix around the Y axis by `angle` radians.
    pub fn rotation_y(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self::new([
            c,   0.0, -s,  0.0,
            0.0, 1.0, 0.0, 0.0,
            s,   0.0, c,   0.0,
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    /// Builds a rotation matrix around the Z axis by `angle` radians.
    pub fn rotation_z(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self::new([
            c,   s,   0.0, 0.0,
            -s,  c,   0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    /// Builds a rotation matrix from Euler angles in radians.
    pub fn rotation_from_euler(yaw: f32, pitch: f32, roll: f32) -> Self {
        Self::rotation_y(yaw)
            .multiply(&Self::rotation_x(pitch))
            .multiply(&Self::rotation_z(roll))
    }

    /// Constructs a rotation matrix from an axis and angle in radians.
    pub fn rotation_axis_angle(axis: Vec3, angle: f32) -> Self {
        Self::from_quat(&Quat::from_axis_angle(axis, angle))
    }

    /// Constructs a rotation matrix from a quaternion.
    pub fn from_quat(q: &Quat) -> Self { q.to_mat4() }
    /// Creates a matrix from column-major array data.
    ///
    /// Callers must supply 16 finite values already laid out column-major.
    pub const fn new(data: [f32; 16]) -> Self {
        Self { data }
    }

    /// Returns the matrix as a column-major array.
    pub fn to_array(self) -> [f32; 16] {
        self.data
    }

    fn at(&self, row: usize, col: usize) -> f32 {
        self.data[col * 4 + row]
    }

    /// Multiplies the matrix with another matrix (`self * rhs`).
    ///
    /// Multiplication follows column-major semantics (`self` on the left,
    /// [`rhs`] on the right) to mirror GPU-style transforms.
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
    fn mul(self, rhs: Self) -> Self::Output { self.multiply(&rhs) }
}
