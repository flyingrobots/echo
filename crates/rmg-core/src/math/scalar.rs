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
//! - Subnormal flushing (to be handled by concrete float wrappers in a
//!   follow-up task).
//! - Lookup-table or polynomial-backed trig implementations (tracked separately;
//!   this trait only declares the API).
//! - Concrete backends: `F32Scalar` and `DFix64` will implement this trait in
//!   subsequent changes.
//!
//! Determinism contract:
//! - Operations must be pure and total for all valid inputs of the
//!   implementation’s domain.
//! - For floating-point backends, implementations are responsible for any
//!   canonicalization/flush semantics required by Echo’s determinism policy.
//! - Trigonometric functions interpret arguments as radians and must be
//!   consistent across platforms for identical inputs (e.g., via LUT/polynomial
//!   in later work).

use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};

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
        (self.sin(), self.cos())
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
    value: f32,
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
        Self { value: num + 0.0 }
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
        Self::new(self.value.sin())
    }

    fn cos(self) -> Self {
        Self::new(self.value.cos())
    }

    fn sin_cos(self) -> (Self, Self) {
        (Self::new(self.value.sin()), Self::new(self.value.cos()))
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
