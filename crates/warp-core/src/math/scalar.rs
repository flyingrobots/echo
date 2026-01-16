// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Deterministic scalar arithmetic abstraction for Echo math.
//!
//! This trait provides a minimal, platform-stable surface for numeric code in
//! Echo to depend on without committing to a single concrete representation.
//! Implementations must uphold determinism across supported platforms and are
//! expected to encapsulate representation-specific policies (e.g., float32
//! canonicalization or fixed-point scaling).
//!
//! Scope (Issue #115):
//! - Core arithmetic: add, sub, mul, div, neg.
//! - Core transcendentals: sin, cos (angles in radians).
//!
//! Out of scope for this commit:
//! - Scalar backend selection plumbing across the whole engine (feature gates
//!   exist, but wiring generic engine code to switch lanes is follow-up work).
//! - More advanced deterministic transcendental backends (e.g., higher-order
//!   interpolation or polynomial approximations) beyond the initial LUT-backed
//!   implementation.
//!
//! Determinism contract:
//! - Operations must be pure and total for all valid inputs of the
//!   implementation’s domain.
//! - For floating-point backends, implementations are responsible for any
//!   canonicalization/flush semantics required by Echo’s determinism policy.
//! - Trigonometric functions interpret arguments as radians and must be
//!   consistent across platforms for identical inputs (e.g., via LUT/polynomial
//!   in later work).
//!
//! Implementation note:
//! - `F32Scalar::{sin,cos,sin_cos}` are implemented using a deterministic
//!   LUT-backed approximation in `warp_core::math::trig`.

use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};

use crate::math::trig;

#[cfg(feature = "det_fixed")]
use crate::math::fixed_q32_32;

/// Deterministic scalar arithmetic and basic transcendentals.
///
/// This trait abstracts the numeric core used by Echo so that engine code can
/// be written generically and later bound to either a deterministic float32
/// wrapper (`F32Scalar`) or a fixed-point implementation (`DFix64`). Arithmetic
/// operators are required via the standard operator traits for ergonomic use of
/// `+`, `-`, `*`, `/`, and unary `-` in generic code.
pub trait Scalar:
    Copy
    + core::fmt::Debug
    + PartialEq
    + Send
    + Sync
    + 'static
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    /// Returns the additive identity (zero).
    fn zero() -> Self;

    /// Returns the multiplicative identity (one).
    fn one() -> Self;

    /// Returns the sine of `self` (radians) under deterministic semantics.
    fn sin(self) -> Self;

    /// Returns the cosine of `self` (radians) under deterministic semantics.
    fn cos(self) -> Self;

    /// Returns both sine and cosine of `self` (radians).
    ///
    /// Default implementation computes `sin` and `cos` separately; concrete
    /// implementations may override for efficiency or shared range reduction.
    fn sin_cos(self) -> (Self, Self) {
        (Self::sin(self), Self::cos(self))
    }

    /// Converts from `f32` into this scalar type.
    ///
    /// This is intended for boundary crossings (e.g., deserializing payloads)
    /// and test scaffolding. Implementations must apply any necessary
    /// canonicalization required by Echo’s determinism policy.
    fn from_f32(value: f32) -> Self;

    /// Converts this scalar value to `f32` for interop and diagnostics.
    ///
    /// Implementations should define rounding policy precisely (e.g., ties to
    /// even) and ensure platform-stable results.
    fn to_f32(self) -> f32;
}

/// Deterministic f32 value
#[derive(Debug, Copy, Clone)]
pub struct F32Scalar {
    /// The wrapped f32 value
    ///
    /// # Invariant
    /// This field is private to enforce canonicalization via `new()`.
    /// It must NEVER contain `-0.0`, non-canonical NaNs, or subnormals.
    value: f32,
}

#[cfg(feature = "serde")]
impl serde::Serialize for F32Scalar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.value.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for F32Scalar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f32::deserialize(deserializer)?;
        Ok(Self::new(v))
    }
}

impl F32Scalar {
    /// Nil value
    pub const ZERO: Self = Self::new(0.0);

    /// Identity value
    pub const ONE: Self = Self::new(1.0);

    /// Constructs a `F32Scalar` with the specified value `num`.
    ///
    /// Canonicalizes `-0.0` to `+0.0` to ensure deterministic zero handling.
    pub const fn new(num: f32) -> Self {
        if num.is_nan() {
            // Canonical NaN: 0x7fc00000 (Positive Quiet NaN)
            Self {
                value: f32::from_bits(0x7fc0_0000),
            }
        } else if num.is_subnormal() {
            Self {
                value: f32::from_bits(0),
            }
        } else {
            // Canonicalize -0.0 to +0.0
            Self { value: num + 0.0 }
        }
    }
}

impl PartialEq for F32Scalar {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for F32Scalar {}

impl PartialOrd for F32Scalar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for F32Scalar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.total_cmp(&other.value)
    }
}

impl fmt::Display for F32Scalar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Scalar for F32Scalar {
    fn zero() -> Self {
        Self::ZERO
    }

    fn one() -> Self {
        Self::ONE
    }

    fn sin(self) -> Self {
        let (s, _) = trig::sin_cos_f32(self.value);
        Self::new(s)
    }

    fn cos(self) -> Self {
        let (_, c) = trig::sin_cos_f32(self.value);
        Self::new(c)
    }

    fn sin_cos(self) -> (Self, Self) {
        let (s, c) = trig::sin_cos_f32(self.value);
        (Self::new(s), Self::new(c))
    }

    fn from_f32(value: f32) -> Self {
        Self::new(value)
    }

    fn to_f32(self) -> f32 {
        self.value
    }
}

impl Add for F32Scalar {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.value + rhs.value)
    }
}

impl Sub for F32Scalar {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.value - rhs.value)
    }
}

impl Mul for F32Scalar {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::new(self.value * rhs.value)
    }
}

impl Div for F32Scalar {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Self::new(self.value / rhs.value)
    }
}

impl Neg for F32Scalar {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.value)
    }
}

/// Deterministic fixed-point scalar with Q32.32 encoding stored in an `i64`.
///
/// The underlying integer stores the value scaled by `2^32`:
///
/// ```text
/// real_value = raw / 2^32
/// ```
///
/// # Determinism contract
///
/// - All arithmetic is performed in integer space with saturating overflow.
/// - Multiplication/division use round-to-nearest, ties-to-even semantics.
/// - `from_f32` is deterministic and does not rely on platform transcendentals.
#[cfg(feature = "det_fixed")]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct DFix64 {
    raw: i64,
}

#[cfg(feature = "det_fixed")]
impl DFix64 {
    const FRAC_BITS: u32 = fixed_q32_32::FRAC_BITS;
    const ONE_RAW: i64 = fixed_q32_32::ONE_RAW;

    /// The fixed-point zero value.
    pub const ZERO: Self = Self { raw: 0 };

    /// The fixed-point one value.
    pub const ONE: Self = Self { raw: Self::ONE_RAW };

    /// Constructs a fixed-point value from a raw Q32.32 integer.
    ///
    /// This is an exact conversion (no scaling or rounding). `raw` is interpreted as
    /// `real_value = raw / 2^32`.
    #[must_use]
    pub const fn from_raw(raw: i64) -> Self {
        Self { raw }
    }

    /// Returns the underlying Q32.32 raw storage value.
    pub const fn raw(self) -> i64 {
        self.raw
    }

    fn saturate_i128_to_i64(value: i128) -> i64 {
        i64::try_from(value).unwrap_or_else(|_| {
            if value.is_negative() {
                i64::MIN
            } else {
                i64::MAX
            }
        })
    }

    fn saturating_add_raw(a: i64, b: i64) -> i64 {
        Self::saturate_i128_to_i64(i128::from(a) + i128::from(b))
    }

    fn saturating_sub_raw(a: i64, b: i64) -> i64 {
        Self::saturate_i128_to_i64(i128::from(a) - i128::from(b))
    }

    fn saturating_neg_raw(a: i64) -> i64 {
        if a == i64::MIN {
            i64::MAX
        } else {
            -a
        }
    }

    fn mul_raw(a: i64, b: i64) -> i64 {
        let prod = i128::from(a) * i128::from(b);
        let abs: u128 = prod.unsigned_abs();
        let q = abs >> Self::FRAC_BITS;
        let r = abs & ((1_u128 << Self::FRAC_BITS) - 1);
        let half = 1_u128 << (Self::FRAC_BITS - 1);

        let mut rounded = q;
        if r > half || (r == half && (q & 1) == 1) {
            rounded = rounded.saturating_add(1);
        }

        let rounded_i128 = i128::try_from(rounded).map_or(i128::MAX, |v| v);
        let signed = if prod.is_negative() {
            -rounded_i128
        } else {
            rounded_i128
        };

        Self::saturate_i128_to_i64(signed)
    }

    fn div_raw(a: i64, b: i64) -> i64 {
        if b == 0 {
            if a == 0 {
                // Determinism policy: 0/0 → 0 (not NaN) to preserve integer semantics.
                return 0;
            }
            return if a.is_negative() { i64::MIN } else { i64::MAX };
        }

        let num = i128::from(a) << Self::FRAC_BITS;
        let den = i128::from(b);

        let abs_num: u128 = num.unsigned_abs();
        let abs_den: u128 = den.unsigned_abs();

        let q = abs_num / abs_den;
        let r = abs_num % abs_den;

        let mut rounded = q;
        let twice_r = r.saturating_mul(2);
        if twice_r > abs_den || (twice_r == abs_den && (q & 1) == 1) {
            rounded = rounded.saturating_add(1);
        }

        let rounded_i128 = i128::try_from(rounded).map_or(i128::MAX, |v| v);
        let signed = if (a < 0) ^ (b < 0) {
            -rounded_i128
        } else {
            rounded_i128
        };

        Self::saturate_i128_to_i64(signed)
    }
}

#[cfg(feature = "det_fixed")]
impl Scalar for DFix64 {
    fn zero() -> Self {
        Self::ZERO
    }

    fn one() -> Self {
        Self::ONE
    }

    fn sin(self) -> Self {
        let (s, _) = crate::math::trig::sin_cos_f32(self.to_f32());
        Self::from_f32(s)
    }

    fn cos(self) -> Self {
        let (_, c) = crate::math::trig::sin_cos_f32(self.to_f32());
        Self::from_f32(c)
    }

    fn sin_cos(self) -> (Self, Self) {
        let (s, c) = crate::math::trig::sin_cos_f32(self.to_f32());
        (Self::from_f32(s), Self::from_f32(c))
    }

    fn from_f32(value: f32) -> Self {
        Self::from_raw(fixed_q32_32::from_f32(value))
    }

    fn to_f32(self) -> f32 {
        fixed_q32_32::to_f32(self.raw)
    }
}

#[cfg(feature = "det_fixed")]
impl Add for DFix64 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::from_raw(Self::saturating_add_raw(self.raw, rhs.raw))
    }
}

#[cfg(feature = "det_fixed")]
impl Sub for DFix64 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::from_raw(Self::saturating_sub_raw(self.raw, rhs.raw))
    }
}

#[cfg(feature = "det_fixed")]
impl Mul for DFix64 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self::from_raw(Self::mul_raw(self.raw, rhs.raw))
    }
}

#[cfg(feature = "det_fixed")]
impl Div for DFix64 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self::from_raw(Self::div_raw(self.raw, rhs.raw))
    }
}

#[cfg(feature = "det_fixed")]
impl Neg for DFix64 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::from_raw(Self::saturating_neg_raw(self.raw))
    }
}
