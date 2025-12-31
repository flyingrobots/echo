// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use core::f32::consts::FRAC_PI_2;
use warp_core::math::{Mat4, Vec3};

fn approx_eq3(a: [f32; 3], b: [f32; 3]) {
    const ABS_TOL: f32 = 1e-7;
    const REL_TOL: f32 = 1e-6;
    for i in 0..3 {
        let ai = a[i];
        let bi = b[i];
        let diff = (ai - bi).abs();
        let scale = ai.abs().max(bi.abs());
        let tol = ABS_TOL.max(REL_TOL * scale);
        assert!(
            diff <= tol,
            "index {i}: {a:?} vs {b:?}, diff={diff}, tol={tol} (scale={scale})"
        );
    }
}

#[test]
fn rot_z_maps_x_to_y() {
    let y = Mat4::rotation_z(FRAC_PI_2).transform_direction(&Vec3::UNIT_X);
    approx_eq3(y.to_array(), [0.0, 1.0, 0.0]);
}

#[test]
fn rot_y_maps_z_to_x() {
    let x = Mat4::rotation_y(FRAC_PI_2).transform_direction(&Vec3::UNIT_Z);
    approx_eq3(x.to_array(), [1.0, 0.0, 0.0]);
}

#[test]
fn rot_x_maps_y_to_z() {
    let z = Mat4::rotation_x(FRAC_PI_2).transform_direction(&Vec3::UNIT_Y);
    approx_eq3(z.to_array(), [0.0, 0.0, 1.0]);
}

#[test]
fn axis_angle_matches_axis_specific_rotation() {
    // Y-rotation via axis-angle should match rotation_y.
    let aa = Mat4::rotation_axis_angle(Vec3::UNIT_Y, FRAC_PI_2);
    let ry = Mat4::rotation_y(FRAC_PI_2);
    let v = Vec3::UNIT_Z;
    approx_eq3(
        aa.transform_direction(&v).to_array(),
        ry.transform_direction(&v).to_array(),
    );
}

#[test]
fn euler_matches_axis_specific_rotations() {
    // Yaw only
    let e = Mat4::rotation_from_euler(FRAC_PI_2, 0.0, 0.0);
    let y = Mat4::rotation_y(FRAC_PI_2);
    approx_eq3(
        e.transform_direction(&Vec3::UNIT_Z).to_array(),
        y.transform_direction(&Vec3::UNIT_Z).to_array(),
    );

    // Pitch only
    let e = Mat4::rotation_from_euler(0.0, FRAC_PI_2, 0.0);
    let x = Mat4::rotation_x(FRAC_PI_2);
    approx_eq3(
        e.transform_direction(&Vec3::UNIT_Y).to_array(),
        x.transform_direction(&Vec3::UNIT_Y).to_array(),
    );

    // Roll only
    let e = Mat4::rotation_from_euler(0.0, 0.0, FRAC_PI_2);
    let z = Mat4::rotation_z(FRAC_PI_2);
    approx_eq3(
        e.transform_direction(&Vec3::UNIT_X).to_array(),
        z.transform_direction(&Vec3::UNIT_X).to_array(),
    );
}
