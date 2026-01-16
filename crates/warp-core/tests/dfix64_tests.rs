// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
#![cfg(feature = "det_fixed")]

use warp_core::math::scalar::DFix64;
use warp_core::math::Scalar;

#[test]
fn dfix64_constants_and_raw_encoding() {
    assert_eq!(DFix64::ZERO.raw(), 0);
    assert_eq!(DFix64::ONE.raw(), 1_i64 << 32);
}

#[test]
fn dfix64_from_f32_exact_values() {
    assert_eq!(DFix64::from_f32(0.0).raw(), 0);
    assert_eq!(DFix64::from_f32(-0.0).raw(), 0);

    assert_eq!(DFix64::from_f32(1.0).raw(), 1_i64 << 32);
    assert_eq!(DFix64::from_f32(-1.0).raw(), -(1_i64 << 32));

    assert_eq!(DFix64::from_f32(0.5).raw(), 1_i64 << 31);
    assert_eq!(DFix64::from_f32(1.5).raw(), (1_i64 << 32) + (1_i64 << 31));
}

#[test]
fn dfix64_to_f32_roundtrips_basic_values() {
    let values = [0.0, -0.0, 1.0, -1.0, 0.5, 1.5];
    for v in values {
        let fx = DFix64::from_f32(v);
        assert_eq!(fx.to_f32(), v);
    }
}

#[test]
fn dfix64_infinite_inputs_saturate() {
    let pos_inf = f32::from_bits(0x7f80_0000);
    let neg_inf = f32::from_bits(0xff80_0000);
    assert_eq!(DFix64::from_f32(pos_inf).raw(), i64::MAX);
    assert_eq!(DFix64::from_f32(neg_inf).raw(), i64::MIN);
}

#[test]
fn dfix64_nan_inputs_become_zero() {
    let nan = f32::from_bits(0x7fc0_0000);
    assert_eq!(DFix64::from_f32(nan).raw(), 0);
}

#[test]
fn dfix64_basic_arithmetic_is_reasonable() {
    let a = DFix64::from_f32(1.5);
    let b = DFix64::from_f32(2.0);
    assert_eq!((a + b).to_f32(), 3.5);
    assert_eq!((b - a).to_f32(), 0.5);
    assert_eq!((a * b).to_f32(), 3.0);
    assert_eq!((b / a).to_f32(), (2.0_f32 / 1.5_f32));
}

#[test]
fn dfix64_sin_cos_at_zero_is_exact() {
    let (s, c) = DFix64::from_f32(0.0).sin_cos();
    assert_eq!(s.raw(), 0);
    assert_eq!(c.raw(), 1_i64 << 32);
}
