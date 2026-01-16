// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deterministic helpers for the Q32.32 fixed-point encoding used by Echo.
//!
//! These helpers are shared by:
//! - `DFix64` (feature-gated fixed-point scalar backend), and
//! - payload encodings that use fixed-point as a stable, cross-language wire format.
//!
//! The representation is an `i64` storing an integer scaled by `2^32`:
//! `real_value = raw / 2^32`.

/// Number of fractional bits in the Q32.32 fixed-point encoding.
pub(crate) const FRAC_BITS: u32 = 32;

/// The raw integer value corresponding to `1.0` in Q32.32.
#[cfg(feature = "det_fixed")]
pub(crate) const ONE_RAW: i64 = 1_i64 << FRAC_BITS;

fn round_shift_right_u64(value: u64, shift: u32) -> u64 {
    if shift == 0 {
        return value;
    }
    if shift >= 64 {
        return 0;
    }

    let q = value >> shift;
    let mask = (1_u64 << shift) - 1;
    let r = value & mask;
    let half = 1_u64 << (shift - 1);

    if r > half {
        q + 1
    } else if r < half {
        q
    } else if (q & 1) == 1 {
        q + 1
    } else {
        q
    }
}

fn round_shift_right_u128(value: u128, shift: u32) -> u128 {
    if shift == 0 {
        return value;
    }
    if shift >= 128 {
        return 0;
    }

    let q = value >> shift;
    let mask = (1_u128 << shift) - 1;
    let r = value & mask;
    let half = 1_u128 << (shift - 1);

    if r > half {
        q + 1
    } else if r < half {
        q
    } else if (q & 1) == 1 {
        q + 1
    } else {
        q
    }
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

/// Deterministically converts an `f32` to a Q32.32 raw `i64`.
///
/// Semantics:
/// - `NaN` maps to `0` (fixed-point has no NaN representation).
/// - `+∞`/`-∞` saturate to `i64::MAX`/`i64::MIN`.
/// - Values are rounded to nearest with ties-to-even at the Q32.32 boundary.
pub fn from_f32(value: f32) -> i64 {
    if value.is_nan() {
        return 0;
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            i64::MAX
        } else {
            i64::MIN
        };
    }

    let bits = value.to_bits();
    let sign = (bits >> 31) != 0;
    // Masking yields a value in 0..=255.
    #[allow(clippy::cast_possible_truncation)]
    let exp_u8 = ((bits >> 23) & 0xff) as u8;
    let exp = i32::from(exp_u8);
    let mant = bits & 0x7fffff;

    if exp == 0 && mant == 0 {
        return 0;
    }

    let mantissa: u64 = if exp == 0 {
        // subnormal: exponent is fixed at -126, no implicit 1.
        u64::from(mant)
    } else {
        // normal: implicit leading 1.
        u64::from((1_u32 << 23) | mant)
    };

    // value = mantissa * 2^(exp - 127 - 23)
    // scaled = value * 2^FRAC_BITS = mantissa * 2^(exp - 127 + (FRAC_BITS - 23))
    // For subnormals exp is treated as 1 - 127 = -126.
    let unbiased = if exp == 0 { -126 } else { exp - 127 };
    #[allow(clippy::cast_possible_wrap)]
    let frac_i32 = FRAC_BITS as i32;
    let shift = unbiased + (frac_i32 - 23);

    // Produce the signed fixed-point raw value, saturating if needed.
    let abs_raw: i128 = if shift >= 0 {
        // `shift` is non-negative in this branch; unsigned_abs preserves the value.
        let shift_u = shift.unsigned_abs();
        // mantissa is ~24 bits; shifting beyond 103 would exceed i128's range.
        if shift_u > 103 {
            i128::MAX
        } else {
            i128::from(mantissa) << shift_u
        }
    } else {
        // Safe: shift is negative; `unsigned_abs` handles the i32::MIN case.
        let rshift = shift.unsigned_abs();
        let rounded = round_shift_right_u64(mantissa, rshift);
        i128::from(rounded)
    };

    let signed_raw = if sign { -abs_raw } else { abs_raw };
    saturate_i128_to_i64(signed_raw)
}

/// Deterministically converts a Q32.32 raw `i64` to an `f32`.
///
/// Rounds to nearest with ties-to-even at the `f32` boundary.
pub fn to_f32(raw: i64) -> f32 {
    if raw == 0 {
        return 0.0;
    }

    let sign = raw.is_negative();
    let abs: u64 = raw.unsigned_abs();
    if abs == 0 {
        // Canonicalize -0.0 to +0.0.
        return 0.0;
    }

    // raw is an integer scaled by 2^32.
    // If raw's highest set bit is at position k, then:
    // abs ∈ [2^k, 2^(k+1)) and value = abs * 2^-32 has exponent (k - 32).
    let k = 63_u32.saturating_sub(abs.leading_zeros());
    #[allow(clippy::cast_possible_wrap)]
    let frac_i32 = FRAC_BITS as i32;
    #[allow(clippy::cast_possible_wrap)]
    let mut exp = (k as i32) - frac_i32;

    // Build a 24-bit significand (including the implicit leading 1) with
    // ties-to-even rounding, then drop the implicit bit into the mantissa field.
    let mut sig: u128 = if k > 23 {
        let rshift = k - 23;
        round_shift_right_u128(u128::from(abs), rshift)
    } else {
        let lshift = 23 - k;
        u128::from(abs) << lshift
    };

    // Handle rounding overflow (e.g., 1.111.. rounds up to 10.000..).
    if sig >= (1_u128 << 24) {
        sig >>= 1;
        exp = exp.saturating_add(1);
    }

    #[allow(clippy::cast_sign_loss)]
    let exp_field = (exp + 127) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let mantissa = (sig & ((1_u128 << 23) - 1)) as u32;
    let bits = (u32::from(sign) << 31) | (exp_field << 23) | mantissa;
    f32::from_bits(bits)
}
