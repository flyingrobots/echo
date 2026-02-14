// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Float canonicalization for deterministic comparison and hashing.
//!
//! These functions are used by the codec layer when computing content hashes.
//! They ensure that equivalent floating-point values produce identical hashes.

/// Canonicalize a float for deterministic comparison and hashing.
///
/// **Purpose:** Used by codec layer when computing content hashes.
/// Do NOT use this to mutate stored positions—it's a projection for comparison.
///
/// # Panics
///
/// Panics if `x` is NaN or Infinity. Scene data must be finite.
pub fn canonicalize_f32(x: f32) -> f32 {
    assert!(x.is_finite(), "NaN/Infinity not allowed in scene data");
    // Perform scaling in f64 to match JS 'number' precision during intermediate step.
    let scaled = x as f64 * 1_000_000.0;
    let truncated = (scaled as i64) as f32 / 1_000_000.0;
    if truncated == 0.0 {
        0.0
    } else {
        truncated
    }
}

/// Canonicalize a position vector for comparison/hashing.
pub fn canonicalize_position(p: [f32; 3]) -> [f32; 3] {
    [
        canonicalize_f32(p[0]),
        canonicalize_f32(p[1]),
        canonicalize_f32(p[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negative_zero() {
        assert_eq!(canonicalize_f32(-0.0), 0.0);
    }

    #[test]
    fn test_truncation() {
        assert_eq!(canonicalize_f32(1.234_567_9), 1.234567);
    }

    #[test]
    #[should_panic(expected = "NaN")]
    fn test_nan_panics() {
        canonicalize_f32(f32::NAN);
    }

    #[test]
    #[should_panic(expected = "Infinity")]
    fn test_infinity_panics() {
        canonicalize_f32(f32::INFINITY);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_float_parity_with_js() {
        use rand::Rng;
        use std::process::Command;

        let mut rng = rand::thread_rng();
        // 10,000 might be slow if we spawn node for each, let's do 1,000 or batch.
        // Actually, spawning node 1000 times is still fast enough for a unit test.
        for _ in 0..100 {
            let val: f32 = rng.gen_range(-10000.0..10000.0);
            let rust_result = canonicalize_f32(val);
            let rust_hex = hex::encode(rust_result.to_le_bytes());

            let val_str = format!("{:.10}", val);
            let output = Command::new("node")
                .arg("../../scripts/float-parity-check.js")
                .arg(&val_str)
                .output()
                .expect("failed to execute node");

            let js_hex = String::from_utf8_lossy(&output.stdout);
            if rust_hex != js_hex {
                println!(
                    "Value: {}, String sent to JS: {}, Rust Hex: {}, JS Hex: {}",
                    val, val_str, rust_hex, js_hex
                );
            }
            assert_eq!(
                rust_hex, js_hex,
                "Float parity mismatch for {}: Rust={} (hex {}), JS={} (hex {})",
                val, rust_result, rust_hex, "?", js_hex
            );
        }
    }
}
