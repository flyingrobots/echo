#![allow(missing_docs)]
use rmg_core::math::Mat4;

fn approx_eq16(a: [f32; 16], b: [f32; 16]) {
    for i in 0..16 {
        assert!((a[i] - b[i]).abs() < 1e-6, "index {i}: {a:?} vs {b:?}");
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
}
