// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tests for canonical floating-point decoding.
//!
//! Verifies that the decoder rejects non-canonical floating-point representations
//! (e.g. floats that should be integers, or f64s that fit in f32/f16).

use echo_wasm_abi::{CanonError, decode_value};

#[test]
fn test_rejects_non_canonical_floats() {
    // 1.0 encoded as f32 (0xfa3f800000)
    // Should be integer 1 (0x01)
    let one_f32 = vec![0xfa, 0x3f, 0x80, 0x00, 0x00];
    let res = decode_value(&one_f32);
    assert_eq!(
        res.unwrap_err(),
        CanonError::FloatShouldBeInt,
        "1.0 as f32 should be rejected"
    );

    // 1.5 encoded as f64 (0xfb3ff8000000000000)
    // Should be f16 (0xf93e00) or f32
    // 1.5 is 0x3fc00000 in f32, 0x3e00 in f16
    let one_point_five_f64 = vec![0xfb, 0x3f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let res = decode_value(&one_point_five_f64);
    assert_eq!(
        res.unwrap_err(),
        CanonError::NonCanonicalFloat,
        "1.5 as f64 should be rejected (fits in f16/f32)"
    );

    // 1.5 encoded as f32 (0xfa3fc00000)
    // Should be f16 (0xf93e00)
    let one_point_five_f32 = vec![0xfa, 0x3f, 0xc0, 0x00, 0x00];
    let res = decode_value(&one_point_five_f32);
    assert_eq!(
        res.unwrap_err(),
        CanonError::NonCanonicalFloat,
        "1.5 as f32 should be rejected (fits in f16)"
    );
}

#[test]
fn test_rejects_int_as_float() {
    // 42.0 encoded as f16 (0xf95140)
    // Should be integer 42 (0x18 0x2a)
    let forty_two_f16 = vec![0xf9, 0x51, 0x40];
    let res = decode_value(&forty_two_f16);
    assert_eq!(
        res.unwrap_err(),
        CanonError::FloatShouldBeInt,
        "42.0 as f16 should be rejected"
    );
}
