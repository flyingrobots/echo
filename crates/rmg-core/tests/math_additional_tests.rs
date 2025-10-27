#![allow(missing_docs)]
use rmg_core::math::{self, Mat4, Quat, Vec3};

fn approx_eq(a: f32, b: f32) {
    let diff = (a - b).abs();
    assert!(diff <= 1e-6, "expected {b}, got {a} (diff {diff})");
}

#[allow(dead_code)]
fn approx_eq3(a: [f32; 3], b: [f32; 3]) {
    for i in 0..3 {
        approx_eq(a[i], b[i]);
    }
}

#[test]
fn vec3_normalize_degenerate_returns_zero() {
    let v = Vec3::new(1e-12, -1e-12, 0.0);
    let n = v.normalize();
    assert_eq!(n.to_array(), [0.0, 0.0, 0.0]);
}

#[test]
fn quat_identity_properties() {
    let id = Quat::identity();
    // identity * identity == identity
    let composed = id.multiply(&id);
    assert_eq!(composed.to_array(), id.to_array());
    // to_mat4(identity) == Mat4::identity()
    let m = id.to_mat4();
    assert_eq!(m.to_array(), Mat4::identity().to_array());
}

#[test]
fn deg_rad_roundtrip_basic_angles() {
    for deg in [0.0f32, 45.0, 90.0, 180.0, -90.0] {
        let rad = math::deg_to_rad(deg);
        let back = math::rad_to_deg(rad);
        approx_eq(back, deg);
    }
}
