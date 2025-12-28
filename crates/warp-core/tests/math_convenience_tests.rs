// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Focused tests for math convenience constructors to boost coverage
//! and ensure expected semantics for identity/translation/scale and
//! vector basis constants.

use warp_core::math::{Mat4, Vec3};

#[test]
fn identity_multiply_is_noop() {
    // A matrix multiplied by identity should equal the original.
    let a = Mat4::from([
        1.0, 0.0, 0.0, 0.0, // col 0
        0.0, 0.0, -1.0, 0.0, // col 1
        0.0, 1.0, 0.0, 0.0, // col 2
        5.0, -3.0, 2.0, 1.0, // col 3
    ]);
    let id = Mat4::identity();
    assert_eq!(a.multiply(&id).to_array(), a.to_array());
    assert_eq!(id.multiply(&a).to_array(), a.to_array());
}

#[test]
fn translation_affects_points_but_not_directions() {
    let t = Mat4::translation(5.0, -3.0, 2.0);
    let p = Vec3::new(2.0, 4.0, -1.0);
    let d = Vec3::new(2.0, 4.0, -1.0);

    let p2 = t.transform_point(&p);
    let d2 = t.transform_direction(&d);

    assert_eq!(p2.to_array(), [7.0, 1.0, 1.0]);
    assert_eq!(d2.to_array(), d.to_array());
}

#[test]
fn scale_stretches_points_and_directions() {
    let s = Mat4::scale(2.0, 3.0, -1.0);
    let v = Vec3::new(1.0, -2.0, 0.5);
    let p = s.transform_point(&v);
    let d = s.transform_direction(&v);
    assert_eq!(p.to_array(), [2.0, -6.0, -0.5]);
    assert_eq!(d.to_array(), [2.0, -6.0, -0.5]);
}

#[test]
fn vec3_basis_and_zero() {
    assert_eq!(Vec3::ZERO.to_array(), [0.0, 0.0, 0.0]);
    assert_eq!(Vec3::UNIT_X.to_array(), [1.0, 0.0, 0.0]);
    assert_eq!(Vec3::UNIT_Y.to_array(), [0.0, 1.0, 0.0]);
    assert_eq!(Vec3::UNIT_Z.to_array(), [0.0, 0.0, 1.0]);
    assert_eq!(Vec3::zero().to_array(), [0.0, 0.0, 0.0]);
}
