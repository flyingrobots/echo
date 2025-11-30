// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use rmg_core::math::scalar::F32Scalar;
use rmg_core::math::Scalar;

#[test]
fn test_f32_basics() {
    // constants
    let zero = F32Scalar::ZERO;
    let one = F32Scalar::ONE;
    assert_eq!(zero.value, 0.0);
    assert_eq!(one.value, 1.0);

    // basic math
    let a = F32Scalar::new(5.0);
    let b = F32Scalar::new(2.0);
    assert_eq!((a + b).value, 7.0);
    assert_eq!((a - b).value, 3.0);

    assert_eq!((a * b).value, 10.0);
    assert_eq!((a / b).value, 2.5);

    let angle = F32Scalar::new(std::f32::consts::PI);
    assert_eq!(angle.sin().value, angle.value.sin());
    assert_eq!(angle.cos().value, angle.value.cos());
}
