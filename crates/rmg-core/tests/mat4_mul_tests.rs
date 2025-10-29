#![allow(missing_docs)]
use rmg_core::math::Mat4;

const EPS: f32 = 1e-6;

fn approx_eq16(a: [f32; 16], b: [f32; 16]) {
    for i in 0..16 {
        assert!((a[i] - b[i]).abs() <= EPS, "index {i}: {a:?} vs {b:?}");
    }
}

#[test]
fn mat4_mul_operator_matches_method() {
    let s = Mat4::scale(2.0, 3.0, 4.0);
    let id = Mat4::identity();
    // operator
    let op = id * s;
    // method
    let meth = id.multiply(&s);
    approx_eq16(op.to_array(), meth.to_array());
    // also verify the opposite order
    let op2 = s * id;
    let meth2 = s.multiply(&id);
    approx_eq16(op2.to_array(), meth2.to_array());
}

#[test]
fn mat4_mul_assign_variants_work() {
    use core::f32::consts::{FRAC_PI_3, FRAC_PI_4};
    // Owned rhs: non-trivial left-hand (rotation) and right-hand (scale)
    let lhs_rot_x = Mat4::rotation_x(FRAC_PI_4);
    let rhs_scale = Mat4::scale(2.0, 3.0, 4.0);
    let expected_owned = (lhs_rot_x * rhs_scale).to_array();
    let lhs_before = lhs_rot_x.to_array();
    let mut a = lhs_rot_x;
    a *= rhs_scale;
    // In-place result matches operator path and differs from original lhs
    approx_eq16(a.to_array(), expected_owned);
    assert_ne!(a.to_array(), lhs_before);

    // Borrowed rhs: non-trivial left-hand (rotation) and right-hand (translation)
    let lhs_rot_y = Mat4::rotation_y(FRAC_PI_3);
    let rhs_trans = Mat4::translation(1.0, 2.0, 3.0);
    let expected_borrowed = (lhs_rot_y * rhs_trans).to_array();
    let lhs_b_before = lhs_rot_y.to_array();
    let mut b = lhs_rot_y;
    b *= &rhs_trans;
    approx_eq16(b.to_array(), expected_borrowed);
    assert_ne!(b.to_array(), lhs_b_before);
}

#[test]
fn rotations_do_not_produce_negative_zero() {
    let angles = [
        0.0,
        core::f32::consts::FRAC_PI_2,
        core::f32::consts::PI,
        3.0 * core::f32::consts::FRAC_PI_2,
        2.0 * core::f32::consts::PI,
    ];
    let neg_zero = (-0.0f32).to_bits();
    for &a in &angles {
        for m in [
            Mat4::rotation_x(a),
            Mat4::rotation_y(a),
            Mat4::rotation_z(a),
        ] {
            for &e in m.to_array().iter() {
                assert_ne!(
                    e.to_bits(),
                    neg_zero,
                    "found -0.0 in rotation matrix for angle {a}"
                );
            }
        }
    }
}
