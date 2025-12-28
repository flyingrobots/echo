// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use warp_core::math::Vec3;

#[test]
fn add_sub_mul_ops_work() {
    let a = Vec3::new(1.0, -2.0, 0.5);
    let b = Vec3::new(-3.0, 4.0, 1.5);
    assert_eq!((a + b).to_array(), [-2.0, 2.0, 2.0]);
    assert_eq!((a - b).to_array(), [4.0, -6.0, -1.0]);
    assert_eq!((a * 2.0).to_array(), [2.0, -4.0, 1.0]);
    assert_eq!((2.0 * a).to_array(), [2.0, -4.0, 1.0]);
    // Negative scalar multiply (both orders)
    assert_eq!((a * -2.0).to_array(), [-2.0, 4.0, -1.0]);
    assert_eq!((-2.0 * a).to_array(), [-2.0, 4.0, -1.0]);
}

#[test]
fn add_assign_sub_assign_mul_assign_work() {
    let mut v = Vec3::new(1.0, 2.0, 3.0);
    v += Vec3::new(-1.0, 1.0, 0.0);
    assert_eq!(v.to_array(), [0.0, 3.0, 3.0]);
    v -= Vec3::new(0.0, 1.0, 1.0);
    assert_eq!(v.to_array(), [0.0, 2.0, 2.0]);
    v *= 0.5;
    assert_eq!(v.to_array(), [0.0, 1.0, 1.0]);
}
