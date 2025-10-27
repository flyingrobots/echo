use rmg_core::math::Mat4;

#[test]
fn mat4_mul_operator_matches_method() {
    let s = Mat4::scale(2.0, 3.0, 4.0);
    let id = Mat4::identity();
    // operator
    let op = id * s;
    // method
    let meth = id.multiply(&s);
    assert_eq!(op.to_array(), meth.to_array());
}

