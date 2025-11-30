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

#[test]
fn test_f32_canonicalization() {
    let z = F32Scalar::new(0.0);
    let nz = F32Scalar::new(-0.0);

    // IEEE 754 equality: -0.0 == 0.0
    assert_eq!(z, nz);

    // Internal representation MUST be identical (canonicalized)
    // This will fail until we implement canonicalization in `new` or constructors
    assert_eq!(z.to_f32().to_bits(), nz.to_f32().to_bits());
}

#[test]
fn test_f32_traits() {
    let _z = F32Scalar::new(0.0);
    let _nz = F32Scalar::new(-0.0);

    // Test Display (this will fail to compile until Display is implemented)
    // assert_eq!(format!("{}", z), "0.0");

    // Test Eq/Ord (this will fail to compile until traits are implemented)
    // assert!(z <= nz);
    // assert_eq!(z.cmp(&nz), std::cmp::Ordering::Equal);
}
