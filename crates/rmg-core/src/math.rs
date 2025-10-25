//! Deterministic math helpers covering scalar utilities, linear algebra
//! primitives, quaternions, and a timeline-friendly PRNG.
//!
//! The API intentionally rounds everything to `f32` to mirror the engine's
//! float32 mode and keep behaviour identical across environments.

use std::f32::consts::TAU;

const EPSILON: f32 = 1e-6;

/// Clamps `value` to the inclusive `[min, max]` range using float32 rounding.
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    debug_assert!(min <= max, "invalid clamp range: {min} > {max}");
    value.max(min).min(max)
}

/// Converts degrees to radians with float32 precision.
pub fn deg_to_rad(value: f32) -> f32 {
    value * (TAU / 360.0)
}

/// Converts radians to degrees with float32 precision.
pub fn rad_to_deg(value: f32) -> f32 {
    value * (360.0 / TAU)
}

/// 3D vector with deterministic float32 operations.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec3 {
    data: [f32; 3],
}

impl Vec3 {
    /// Creates a vector from components.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { data: [x, y, z] }
    }

    /// Returns the components as an array.
    pub fn to_array(self) -> [f32; 3] {
        self.data
    }

    fn component(&self, idx: usize) -> f32 {
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

    /// Normalises the vector, returning zero vector if length is ~0.
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len.abs() <= EPSILON {
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

/// Column-major 4x4 matrix.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Mat4 {
    data: [f32; 16],
}

impl Mat4 {
    /// Creates a matrix from column-major array data.
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

    /// Multiplies the matrix with another matrix (self * rhs).
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

    /// Transforms a point (assumes w = 1).
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
}

impl From<[f32; 16]> for Mat4 {
    fn from(value: [f32; 16]) -> Self {
        Self { data: value }
    }
}

/// Quaternion stored as (x, y, z, w).
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Quat {
    data: [f32; 4],
}

impl Quat {
    /// Creates a quaternion from components.
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

    /// Constructs a quaternion from a rotation axis (assumed non-zero) and angle in radians.
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let norm_axis = axis.normalize();
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

    /// Multiplies two quaternions (self * other).
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
        if len.abs() <= EPSILON {
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

    /// Converts the quaternion to a rotation matrix (column-major 4x4).
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

/// Counter-based PRNG derived from xoroshiro128**.
#[derive(Debug, Clone, Copy)]
pub struct Prng {
    state: [u64; 2],
}

impl Prng {
    /// Constructs a PRNG from two 64-bit seeds.
    pub fn from_seed(seed0: u64, seed1: u64) -> Self {
        let mut state = [seed0, seed1];
        if state[0] == 0 && state[1] == 0 {
            state[0] = 0x9e3779b97f4a7c15;
        }
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let s0 = self.state[0];
        let mut s1 = self.state[1];
        let result = s0.wrapping_add(s1);

        s1 ^= s0;
        self.state[0] = s0.rotate_left(55) ^ s1 ^ (s1 << 14);
        self.state[1] = s1.rotate_left(36);

        result
    }

    /// Returns the next float in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        let raw = self.next_u64();
        let bits = ((raw >> 40) as u32) | 0x3f80_0000;
        f32::from_bits(bits) - 1.0
    }

    /// Returns the next integer in the inclusive range `[min, max]`.
    pub fn next_int(&mut self, min: i32, max: i32) -> i32 {
        assert!(min <= max, "invalid range: {min}..={max}");
        let span = (max - min + 1) as u32;
        let value = (self.next_u64() % span as u64) as u32;
        min + value as i32
    }
}
