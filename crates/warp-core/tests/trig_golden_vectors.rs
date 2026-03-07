// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// (C) James Ross FLYING*ROBOTS <https://github.com/flyingrobots>

#![allow(
    missing_docs,
    clippy::cast_precision_loss,
    clippy::unreadable_literal,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::print_stderr
)]

//! Trig oracle golden vector test.
//!
//! Verifies that `sin_cos` produces bit-identical outputs for 2048 evenly-spaced
//! angles covering [-2*TAU, 2*TAU]. If ANY output bit changes, this test fails —
//! catching regressions in the LUT, interpolation, or quadrant logic.
//!
//! The golden vectors are checked into `testdata/trig_golden_2048.bin`.
//! To regenerate after an intentional algorithm change:
//!   cargo test -p warp-core --test trig_golden_vectors -- --ignored generate_golden_vectors

use std::f32::consts::TAU;
use std::path::PathBuf;
use warp_core::math::scalar::F32Scalar;
use warp_core::math::Scalar;

const N: usize = 2048;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("cannot find project root")
        .to_path_buf()
}

fn golden_path() -> PathBuf {
    project_root().join("testdata/trig_golden_2048.bin")
}

/// Compute the canonical angle for index i in [0, N).
fn angle_for(i: usize) -> f32 {
    // Cover [-2*TAU, 2*TAU] with N evenly-spaced samples.
    let t = i as f32 / (N - 1) as f32;
    -2.0 * TAU + t * 4.0 * TAU
}

/// Compute golden vectors: for each angle, store (sin_bits, cos_bits) as u32 LE.
fn compute_vectors() -> Vec<u8> {
    // Layout: N entries of [angle_bits:u32, sin_bits:u32, cos_bits:u32] = 12 bytes each.
    let mut buf = Vec::with_capacity(N * 12);
    for i in 0..N {
        let angle = angle_for(i);
        let scalar = F32Scalar::new(angle);
        let (s, c) = scalar.sin_cos();
        buf.extend_from_slice(&angle.to_bits().to_le_bytes());
        buf.extend_from_slice(&s.to_f32().to_bits().to_le_bytes());
        buf.extend_from_slice(&c.to_f32().to_bits().to_le_bytes());
    }
    buf
}

#[test]
fn trig_oracle_matches_golden_vectors() {
    let path = golden_path();
    let expected = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "Golden vector file not found at {}: {e}\n\
             Run: cargo test -p warp-core --test trig_golden_vectors -- --ignored generate_golden_vectors",
            path.display()
        )
    });

    let actual = compute_vectors();

    assert_eq!(
        actual.len(),
        expected.len(),
        "Golden vector size mismatch: expected {} bytes, got {}",
        expected.len(),
        actual.len()
    );

    // Find first divergence for a useful error message.
    for i in 0..N {
        let off = i * 12;
        let a_angle = u32::from_le_bytes(actual[off..off + 4].try_into().unwrap());
        let a_sin = u32::from_le_bytes(actual[off + 4..off + 8].try_into().unwrap());
        let a_cos = u32::from_le_bytes(actual[off + 8..off + 12].try_into().unwrap());

        let e_angle = u32::from_le_bytes(expected[off..off + 4].try_into().unwrap());
        let e_sin = u32::from_le_bytes(expected[off + 4..off + 8].try_into().unwrap());
        let e_cos = u32::from_le_bytes(expected[off + 8..off + 12].try_into().unwrap());

        assert_eq!(
            a_angle, e_angle,
            "angle bits mismatch at index {i}: actual=0x{a_angle:08x} expected=0x{e_angle:08x}"
        );
        assert_eq!(
            a_sin, e_sin,
            "sin bits mismatch at index {i} (angle={:.6}): actual=0x{a_sin:08x} expected=0x{e_sin:08x}",
            f32::from_bits(a_angle)
        );
        assert_eq!(
            a_cos, e_cos,
            "cos bits mismatch at index {i} (angle={:.6}): actual=0x{a_cos:08x} expected=0x{e_cos:08x}",
            f32::from_bits(a_angle)
        );
    }
}

#[test]
#[ignore = "Run manually to regenerate golden vectors after intentional algorithm changes"]
fn generate_golden_vectors() {
    let buf = compute_vectors();
    let path = golden_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &buf).unwrap();
    eprintln!(
        "Wrote {} golden vectors ({} bytes) to {}",
        N,
        buf.len(),
        path.display()
    );
}
