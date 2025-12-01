// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]

use rmg_core::math::scalar::F32Scalar;
use rmg_core::math::Scalar;

// NOTE: These tests describe the intended strict determinism policy.
// They currently fail because F32Scalar only canonicalizes -0.0.
// They are commented out until the "CanonicalF32" work (Issue #XXX) is landed.

#[test]
fn test_policy_nan_canonicalization() {
    // Construct different NaNs
    let pos_qnan = f32::from_bits(0x7fc00000);
    let neg_qnan = f32::from_bits(0xffc00000);
    let signaling_nan = f32::from_bits(0x7f800001); // Exponent all 1s, MSB mantissa 0
    let payload_nan = f32::from_bits(0x7f80dead);

    let s1 = F32Scalar::new(pos_qnan);
    let s2 = F32Scalar::new(neg_qnan);
    let s3 = F32Scalar::new(signaling_nan);
    let s4 = F32Scalar::new(payload_nan);

    // All must be bitwise identical to the canonical NaN
    let canonical_bits = 0x7fc00000; // Standard positive quiet NaN

    assert_eq!(
        s1.to_f32().to_bits(),
        canonical_bits,
        "Positive qNaN not canonicalized"
    );
    assert_eq!(
        s2.to_f32().to_bits(),
        canonical_bits,
        "Negative qNaN not canonicalized"
    );
    assert_eq!(
        s3.to_f32().to_bits(),
        canonical_bits,
        "Signaling NaN not canonicalized"
    );
    assert_eq!(
        s4.to_f32().to_bits(),
        canonical_bits,
        "Payload NaN not canonicalized"
    );
}

#[test]
fn test_policy_subnormal_flushing() {
    // Smallest positive subnormal: 1 bit set in mantissa, 0 exponent
    let small_sub = f32::from_bits(1);
    // Largest subnormal
    let large_sub = f32::from_bits(0x007fffff);

    let s1 = F32Scalar::new(small_sub);
    let s2 = F32Scalar::new(large_sub);

    // Must be flushed to positive zero
    assert_eq!(
        s1.to_f32().to_bits(),
        0,
        "Small subnormal not flushed to +0.0"
    );
    assert_eq!(
        s2.to_f32().to_bits(),
        0,
        "Large subnormal not flushed to +0.0"
    );
    assert_eq!(
        s1.to_f32().to_bits(),
        F32Scalar::ZERO.to_f32().to_bits(),
        "Small subnormal is not equal to ZERO"
    );
    assert_eq!(
        s2.to_f32().to_bits(),
        F32Scalar::ZERO.to_f32().to_bits(),
        "Large subnormal is not equal to ZERO"
    );

    // Verify negative subnormals go to +0.0 too (since -0.0 -> +0.0)
    let neg_sub = f32::from_bits(0x80000001);
    let s3 = F32Scalar::new(neg_sub);
    assert_eq!(
        s3.to_f32().to_bits(),
        0,
        "Negative subnormal not flushed to +0.0"
    );
    assert_eq!(
        s3.to_f32().to_bits(),
        F32Scalar::ZERO.to_f32().to_bits(),
        "Negative subnormal is not equal to ZERO"
    );
}

#[test]
#[cfg(feature = "serde")]
fn test_policy_serialization_guard() {
    // Manually construct JSON with -0.0
    let json = r#"-0.0"#;

    // If Deserialize is derived, this will put -0.0 into the struct, violating the invariant.
    // If implemented manually via new(), it should be +0.0.
    let s: F32Scalar = serde_json::from_str(json).expect("Failed to deserialize");

    assert_eq!(
        s.to_f32().to_bits(),
        0,
        "Deserialized -0.0 was not canonicalized!"
    );
}
