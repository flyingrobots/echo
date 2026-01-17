// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deterministic fixed-point helpers (Q32.32).

/// Q32.32 fixed-point scalar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fx32(i64);

impl Fx32 {
    /// Construct from an integer value (n << 32).
    #[must_use]
    pub fn from_i64(n: i64) -> Self {
        Self(n << 32)
    }

    /// Construct directly from raw Q32.32 bits.
    #[must_use]
    pub fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    /// Return the raw Q32.32 representation.
    #[must_use]
    pub fn raw(self) -> i64 {
        self.0
    }
}

/// 3D vector in Q32.32 fixed-point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vec3Fx {
    pub x: Fx32,
    pub y: Fx32,
    pub z: Fx32,
}

impl Vec3Fx {
    /// Construct from integer components (each converted to Q32.32).
    #[must_use]
    pub fn new_i64(x: i64, y: i64, z: i64) -> Self {
        Self {
            x: Fx32::from_i64(x),
            y: Fx32::from_i64(y),
            z: Fx32::from_i64(z),
        }
    }

    /// Construct from raw Q32.32 components.
    #[must_use]
    pub fn from_raw(raw: [i64; 3]) -> Self {
        Self {
            x: Fx32::from_raw(raw[0]),
            y: Fx32::from_raw(raw[1]),
            z: Fx32::from_raw(raw[2]),
        }
    }

    /// Return raw Q32.32 components.
    #[must_use]
    pub fn to_raw(self) -> [i64; 3] {
        [self.x.raw(), self.y.raw(), self.z.raw()]
    }
}
