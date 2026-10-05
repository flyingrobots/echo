// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Hand-derived raw Q32.32 vectors for Bunny's canonical Echo boundary.

use warp_math::fixed_q32_32::{from_f32, to_f32, FixedQ32_32, FloatConversionError};

const ONE: i64 = 4_294_967_296;
const HALF: i64 = 2_147_483_648;

// Product raw = lhs * rhs / 2^32. These straddle each half-unit boundary.
const MULTIPLICATION: &[(i64, i64, i64)] = &[
    // Exact scaled product exceeds MAX by 8_365_928 / 2^32 but rounds into range.
    (199_032_858_228_936, 199_032_871_303_925, i64::MAX),
    (1, HALF - 1, 0),
    (1, HALF + 1, 1),
    (1, HALF, 0),
    (3, HALF, 2),
    (-1, HALF - 1, 0),
    (-1, HALF + 1, -1),
    (-1, HALF, 0),
    (-3, HALF, -2),
    (3, -HALF, -2),
    (-3, -HALF, 2),
    (i64::MIN, ONE, i64::MIN),
    (i64::MAX, ONE, i64::MAX),
];

// Quotient raw = lhs * 2^32 / rhs, rounded to the nearest even integer.
const DIVISION: &[(i64, i64, i64)] = &[
    (1, 2 * ONE + 1, 0),
    (1, 2 * ONE - 1, 1),
    (1, 2 * ONE, 0),
    (3, 2 * ONE, 2),
    (-1, 2 * ONE + 1, 0),
    (-1, 2 * ONE - 1, -1),
    (-1, 2 * ONE, 0),
    (-3, 2 * ONE, -2),
    (3, -2 * ONE, -2),
    (-3, -2 * ONE, 2),
    (i64::MIN, ONE, i64::MIN),
    (i64::MAX, ONE, i64::MAX),
];

#[test]
fn canonical_checked_raw_arithmetic() {
    for raw in [i64::MIN, -ONE, -1, 0, 1, ONE, i64::MAX] {
        assert_eq!(FixedQ32_32::from_raw(raw).raw(), raw);
    }
    for &(lhs, rhs, expected) in MULTIPLICATION {
        assert_eq!(
            FixedQ32_32::from_raw(lhs)
                .checked_mul(FixedQ32_32::from_raw(rhs))
                .map(FixedQ32_32::raw),
            Some(expected),
            "multiply {lhs}, {rhs}"
        );
    }
    for &(lhs, rhs, expected) in DIVISION {
        assert_eq!(
            FixedQ32_32::from_raw(lhs)
                .checked_div(FixedQ32_32::from_raw(rhs))
                .map(FixedQ32_32::raw),
            Some(expected),
            "divide {lhs}, {rhs}"
        );
    }
    let max = FixedQ32_32::from_raw(i64::MAX);
    let min = FixedQ32_32::from_raw(i64::MIN);
    let quantum = FixedQ32_32::from_raw(1);
    assert_eq!(max.checked_add(quantum), None);
    assert_eq!(min.checked_sub(quantum), None);
    assert_eq!(min.checked_neg(), None);
    assert_eq!(max.checked_mul(FixedQ32_32::from_raw(2 * ONE)), None);
    assert_eq!(min.checked_div(FixedQ32_32::from_raw(-ONE)), None);
    for raw in [-ONE, 0, ONE] {
        assert_eq!(
            FixedQ32_32::from_raw(raw).checked_div(FixedQ32_32::ZERO),
            None
        );
    }
    assert_eq!(
        max.checked_add(FixedQ32_32::from_raw(-1))
            .map(FixedQ32_32::raw),
        Some(i64::MAX - 1)
    );
    assert_eq!(
        min.checked_sub(FixedQ32_32::from_raw(-1))
            .map(FixedQ32_32::raw),
        Some(i64::MIN + 1)
    );
}

#[test]
fn canonical_float_ingress_and_legacy_saturation_have_explicit_policies() {
    for (bits, raw) in [
        (0x0000_0000, 0),
        (0x8000_0000, 0),
        (0x0000_0001, 0),
        (0x8000_0001, 0),
        (0x2f00_0000, 0),
        (0xaf00_0000, 0), // +/- 0.5 raw: even zero
        (0x2f40_0000, 1),
        (0xaf40_0000, -1), // +/- 0.75 raw
        (0x2fc0_0000, 2),
        (0xafc0_0000, -2), // +/- 1.5 raw: even two
        (0x3f80_0000, ONE),
        (0xbf80_0000, -ONE),
        (0xcf00_0000, i64::MIN), // -2^31 is representable
    ] {
        let value = f32::from_bits(bits);
        assert_eq!(from_f32(value), raw);
        assert_eq!(
            FixedQ32_32::try_from_f32(value).map(FixedQ32_32::raw),
            Ok(raw)
        );
    }
    for bits in [0x7fc0_0000, 0xffc0_0000, 0x7f80_0000, 0xff80_0000] {
        assert_eq!(
            FixedQ32_32::try_from_f32(f32::from_bits(bits)),
            Err(FloatConversionError::NonFinite)
        );
    }
    for bits in [0x4f00_0000, 0x7f7f_ffff, 0xff7f_ffff] {
        assert_eq!(
            FixedQ32_32::try_from_f32(f32::from_bits(bits)),
            Err(FloatConversionError::OutOfRange)
        );
    }
    assert_eq!(from_f32(f32::from_bits(0x7fc0_0000)), 0);
    assert_eq!(from_f32(f32::from_bits(0x7f80_0000)), i64::MAX);
    assert_eq!(from_f32(f32::from_bits(0xff80_0000)), i64::MIN);
    assert_eq!(from_f32(f32::from_bits(0x4f00_0000)), i64::MAX);
}

#[test]
fn raw_to_float_uses_even_ieee_significands() {
    // An f32 ULP at +1 is 512 raw units. Halfway 256 selects even 1.0;
    // halfway 768 selects the even second successor, not the odd first.
    for (raw, bits) in [
        (0, 0x0000_0000),
        (1, 0x2f80_0000),
        (-1, 0xaf80_0000),
        (ONE + 256, 0x3f80_0000),
        (ONE + 768, 0x3f80_0002),
        (-ONE - 256, 0xbf80_0000),
        (-ONE - 768, 0xbf80_0002),
        (i64::MAX, 0x4f00_0000),
        (i64::MIN, 0xcf00_0000),
    ] {
        assert_eq!(to_f32(raw).to_bits(), bits);
    }
}

#[test]
fn old_wasm_ingress_truncation_remains_distinct() {
    for (bits, canonical, legacy) in [(0x2fc0_0000, 2, 1), (0xafc0_0000, -2, -1)] {
        let value = f32::from_bits(bits);
        assert_eq!(from_f32(value), canonical);
        assert_eq!(echo_wasm_abi::codec::fx_from_f32(value), legacy);
    }
}

#[cfg(feature = "det_fixed")]
#[test]
fn dfix64_preserves_saturating_raw_results() {
    use warp_math::scalar::DFix64;
    for &(lhs, rhs, expected) in MULTIPLICATION {
        assert_eq!(
            (DFix64::from_raw(lhs) * DFix64::from_raw(rhs)).raw(),
            expected
        );
    }
    for &(lhs, rhs, expected) in DIVISION {
        assert_eq!(
            (DFix64::from_raw(lhs) / DFix64::from_raw(rhs)).raw(),
            expected
        );
    }
    let max = DFix64::from_raw(i64::MAX);
    let min = DFix64::from_raw(i64::MIN);
    let quantum = DFix64::from_raw(1);
    assert_eq!((max + quantum).raw(), i64::MAX);
    assert_eq!((min - quantum).raw(), i64::MIN);
    assert_eq!((-min).raw(), i64::MAX);
    assert_eq!((max * DFix64::from_raw(2 * ONE)).raw(), i64::MAX);
    assert_eq!((min / DFix64::from_raw(-ONE)).raw(), i64::MAX);
    for (raw, expected) in [(-ONE, i64::MIN), (0, 0), (ONE, i64::MAX)] {
        assert_eq!((DFix64::from_raw(raw) / DFix64::ZERO).raw(), expected);
    }
}
