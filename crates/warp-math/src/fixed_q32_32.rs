// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bunny's canonical signed Q32.32 foundation and Echo compatibility conversions.
//!
//! The exact `bunny-num` version in the workspace owns arithmetic and conversion
//! algorithms. Raw values are signed `i64` integers scaled by `2^32`. New
//! canonical callers use [`FixedQ32_32`]'s checked arithmetic and validated float
//! ingress; overflow and division by zero must not become successful results.
//!
//! The value-returning conversion helpers below retain Echo's existing
//! saturating policy for `DFix64` and motion payloads. They are not validating
//! ingress. The older `echo-wasm-abi` codec conversion separately keeps its
//! documented truncation policy.

#[doc(inline)]
pub use bunny_num::{FixedQ32_32, FloatConversionError};

/// The raw integer value corresponding to `1.0` in Q32.32.
#[cfg(feature = "det_fixed")]
pub(crate) const ONE_RAW: i64 = FixedQ32_32::ONE.raw();

/// Deterministically converts an `f32` to a Q32.32 raw `i64` using Bunny.
///
/// This compatibility conversion rounds to nearest with ties-to-even. `NaN`
/// maps to zero; infinities and finite out-of-range values saturate to the
/// nearest raw bound. Use [`FixedQ32_32::try_from_f32`] when ingress must reject
/// non-finite or out-of-range input instead.
pub fn from_f32(value: f32) -> i64 {
    bunny_num::fixed_q32_32::from_f32(value)
}

/// Deterministically converts a Q32.32 raw `i64` to an `f32` using Bunny.
///
/// Rounds to nearest with ties-to-even at the lossy `f32` boundary.
pub fn to_f32(raw: i64) -> f32 {
    bunny_num::fixed_q32_32::to_f32(raw)
}
