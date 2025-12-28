// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use warp_core::math::Mat4;

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
    // We target angles that should produce exact zeros in rotation matrices:
    // multiples of π/2 yield sin/cos values in { -1, 0, 1 }, which is where
    // -0.0 might accidentally appear if we don't canonicalize zeros. We also
    // include a couple of intermediate angles as a sanity check (these should
    // not introduce exact zeros but must also not yield -0.0 anywhere).
    let angles = [
        0.0,
        core::f32::consts::FRAC_PI_6,
        core::f32::consts::FRAC_PI_3,
        core::f32::consts::FRAC_PI_2,
        core::f32::consts::PI,
        3.0 * core::f32::consts::FRAC_PI_2,
        2.0 * core::f32::consts::PI,
    ];
    let neg_zero = (-0.0f32).to_bits();
    for &a in &angles {
        let axes = [
            ("X", Mat4::rotation_x(a)),
            ("Y", Mat4::rotation_y(a)),
            ("Z", Mat4::rotation_z(a)),
        ];
        for (axis, m) in axes {
            for (idx, &e) in m.to_array().iter().enumerate() {
                assert_ne!(
                    e.to_bits(),
                    neg_zero,
                    "found -0.0 in rotation_{} matrix at element [{}] for angle {}",
                    axis,
                    idx,
                    a
                );
            }
        }
    }
}

#[test]
fn mat4_mul_assign_matches_operator_randomized() {
    // Deterministic sampling to exercise a variety of transforms (local RNG to
    // avoid depending on crate internals from an external test crate).
    struct TestRng {
        state: u64,
    }
    impl TestRng {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }
        fn next_u64(&mut self) -> u64 {
            // xorshift64*
            let mut x = self.state;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.state = x;
            x.wrapping_mul(0x2545F4914F6CDD1D)
        }
        fn next_f32(&mut self) -> f32 {
            let bits = ((self.next_u64() >> 41) as u32) | 0x3f80_0000;
            f32::from_bits(bits) - 1.0 // [0,1)
        }
        fn next_int(&mut self, min: i32, max: i32) -> i32 {
            assert!(min <= max);
            let span = (max as i64 - min as i64 + 1) as u64;
            let v = if span.is_power_of_two() {
                self.next_u64() & (span - 1)
            } else {
                let bound = u64::MAX - u64::MAX % span;
                loop {
                    let c = self.next_u64();
                    if c < bound {
                        break c % span;
                    }
                }
            };
            (v as i64 + min as i64) as i32
        }
    }
    let mut rng = TestRng::new(0x00C0_FFEE);

    // Helper to pick a random basic transform
    let rand_transform = |rng: &mut TestRng| -> Mat4 {
        let choice = rng.next_int(0, 2);
        match choice {
            0 => {
                // rotation around a random axis among X/Y/Z with angle in [-pi, pi]
                let which = rng.next_int(0, 2);
                let angle = (rng.next_f32() * 2.0 - 1.0) * core::f32::consts::PI;
                match which {
                    0 => Mat4::rotation_x(angle),
                    1 => Mat4::rotation_y(angle),
                    _ => Mat4::rotation_z(angle),
                }
            }
            1 => {
                // scale in [0.5, 2.0]
                let sx = 0.5 + 1.5 * rng.next_f32();
                let sy = 0.5 + 1.5 * rng.next_f32();
                let sz = 0.5 + 1.5 * rng.next_f32();
                Mat4::scale(sx, sy, sz)
            }
            _ => {
                // translation in [-5, 5]
                let tx = (rng.next_f32() * 10.0) - 5.0;
                let ty = (rng.next_f32() * 10.0) - 5.0;
                let tz = (rng.next_f32() * 10.0) - 5.0;
                Mat4::translation(tx, ty, tz)
            }
        }
    };

    for _ in 0..64 {
        let lhs = rand_transform(&mut rng);
        let rhs = rand_transform(&mut rng);

        // Owned rhs path
        let mut a = lhs;
        let expected_owned = (lhs * rhs).to_array();
        a *= rhs;
        approx_eq16(a.to_array(), expected_owned);

        // Borrowed rhs path (new sample to avoid aliasing concerns)
        let lhs2 = rand_transform(&mut rng);
        let rhs2 = rand_transform(&mut rng);
        let mut b = lhs2;
        let expected_borrowed = (lhs2 * rhs2).to_array();
        b *= &rhs2;
        approx_eq16(b.to_array(), expected_borrowed);

        // Composite LHS path: compose two random transforms to probe deeper paths
        let lhs_c = rand_transform(&mut rng) * rand_transform(&mut rng);
        let rhs_c = rand_transform(&mut rng);
        let mut c = lhs_c;
        let expected_c = (lhs_c * rhs_c).to_array();
        c *= rhs_c;
        approx_eq16(c.to_array(), expected_c);
    }
}
