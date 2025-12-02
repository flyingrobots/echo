// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]

use core::f32;

use rmg_core::math::scalar::F32Scalar;
use rmg_core::math::Scalar;

fn ulp_diff(a: f32, b: f32) -> u32 {
    let ai = a.to_bits() ^ ((((a.to_bits() as i32) >> 31) as u32) >> 1);
    let bi = b.to_bits() ^ ((((b.to_bits() as i32) >> 31) as u32) >> 1);
    return ai.abs_diff(bi);
}

#[test]
fn test_sin_cos_error() {
    let mut max_ulp: f32 = 0;

    let setp = f32::consts::TAU / 4096;

    let mut angle: f32 = -2 * f32::consts::TAU;

    while angle <= 2.0 * f32::consts::TAU {
        let (s, c) = deterministic_sin_cos_f32(angle);
    }
}
