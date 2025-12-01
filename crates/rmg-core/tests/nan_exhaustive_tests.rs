// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Comprehensive tests for NaN canonicalization and determinism.
//!
//! This test suite validates that `F32Scalar` correctly implements the strict
//! determinism policy by:
//! 1. Canonicalizing all NaN bit patterns (signaling, quiet, payload) to a
//!    single standard Positive Quiet NaN (`0x7fc00000`).
//! 2. Preserving Infinities (distinct from NaNs).
//! 3. Ensuring reflexivity (`x == x`) holds for the canonicalized NaNs.

use rmg_core::math::scalar::F32Scalar;
use rmg_core::math::Scalar;

/// Verifies that various classes of NaN values are correctly canonicalized.
///
/// This includes:
/// - Specific edge cases (smallest/largest mantissas).
/// - Signaling vs Quiet NaNs.
/// - Positive vs Negative NaNs.
/// - NaNs with arbitrary payloads.
///
/// It also performs a sweep of low and high mantissa bits to catch potential
/// off-by-one errors in bitmask logic.
#[test]
fn test_comprehensive_nan_coverage() {
    // IEEE 754 float32:
    // Sign: 1 bit (31)
    // Exponent: 8 bits (23-30) -> All 1s for NaN/Inf (0xFF)
    // Mantissa: 23 bits (0-22) -> Non-zero for NaN (Zero for Inf)

    let exponent_mask = 0x7F800000;
    let mantissa_mask = 0x007FFFFF;
    let sign_mask = 0x80000000u32;

    // 1. Verify specific edge case NaNs
    let patterns = vec![
        0x7F800001, // Smallest mantissa, positive
        0x7FFFFFFF, // Largest mantissa, positive
        0xFF800001, // Smallest mantissa, negative
        0xFFFFFFFF, // Largest mantissa, negative
        0x7FC00000, // Canonical qNaN positive
        0xFFC00000, // Canonical qNaN negative
        0x7FA00000, // Some payload
        0x7F80DEAD, // Dead beef payload
    ];

    for bits in patterns {
        let f = f32::from_bits(bits);
        // Pre-condition: verify our assumption that these ARE NaNs according to Rust
        assert!(f.is_nan(), "Rust did not identify {:#x} as NaN", bits);

        let scalar = F32Scalar::new(f);
        let out_bits = scalar.to_f32().to_bits();

        assert_eq!(
            out_bits, 0x7fc00000,
            "Input NaN {:#x} was not canonicalized to 0x7fc00000, got {:#x}",
            bits, out_bits
        );

        // Explicitly test reflexivity for the canonicalized NaN
        assert_eq!(
            scalar, scalar,
            "Reflexivity failed for canonicalized NaN from input {:#x}",
            bits
        );
        assert_eq!(
            scalar.cmp(&scalar),
            std::cmp::Ordering::Equal,
            "Ordering reflexivity failed for canonicalized NaN from input {:#x}",
            bits
        );
    }

    // 2. Fuzz / Sweep a range of mantissas
    // We can't check all 2^23 * 2 NaNs, but we can check a lot.
    // Let's check the first 1000 and last 1000 mantissas for both signs.

    let signs = [0u32, sign_mask];

    for sign in signs {
        // Mantissa cannot be 0 (that's Infinity)
        // Loop 1..1000
        for m in 1..1000 {
            let bits = sign | exponent_mask | m;
            let f = f32::from_bits(bits);
            let s = F32Scalar::new(f);
            assert_eq!(
                s.to_f32().to_bits(),
                0x7fc00000,
                "Failed low mantissa {:#x}",
                bits
            );
        }
        // Loop max-1000..max
        for m in (mantissa_mask - 1000)..=mantissa_mask {
            let bits = sign | exponent_mask | m;
            let f = f32::from_bits(bits);
            let s = F32Scalar::new(f);
            assert_eq!(
                s.to_f32().to_bits(),
                0x7fc00000,
                "Failed high mantissa {:#x}",
                bits
            );
        }
    }
}

/// Verifies that Infinity values are preserved and NOT canonicalized.
///
/// The determinism policy requires finite numbers and infinities to be preserved
/// (modulo -0.0 normalization), while only NaNs are collapsed.
#[test]
fn test_infinity_preservation() {
    // Ensure we didn't accidentally canonicalize Infinity
    let pos_inf = f32::from_bits(0x7F800000);
    let neg_inf = f32::from_bits(0xFF800000);

    assert!(!pos_inf.is_nan());
    assert!(!neg_inf.is_nan());

    let s_pos = F32Scalar::new(pos_inf);
    let s_neg = F32Scalar::new(neg_inf);

    assert_eq!(s_pos.to_f32().to_bits(), 0x7F800000);
    assert_eq!(s_neg.to_f32().to_bits(), 0xFF800000);
}
