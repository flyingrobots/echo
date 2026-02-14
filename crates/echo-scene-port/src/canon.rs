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
    // Explicitly reject values too large for precision-scaled truncation.
    // i64::MAX is ~9.22e18, so x * 1e6 must be < ~9.22e18.
    // We cap at 1e12 to be safe and maintain high precision.
    assert!(
        x.abs() < 1_000_000_000_000.0,
        "Scene coordinate magnitude exceeds 1e12 limit"
    );
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
    #[should_panic(expected = "magnitude")]
    fn test_canonicalize_large_values() {
        // large value that would overflow i64 when scaled by 1,000,000
        let val1 = 1e15_f32;
        canonicalize_f32(val1);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_float_parity_with_js() {
        use rand::Rng;

        use std::process::Command;

        let mut rng = rand::thread_rng();

        const NUM_VECTORS: usize = 1000;

        let mut inputs = Vec::with_capacity(NUM_VECTORS);

        let mut rust_hexes = Vec::with_capacity(NUM_VECTORS);

        for _ in 0..NUM_VECTORS {
            let val: f32 = rng.gen_range(-10000.0..10000.0);

            let rust_result = canonicalize_f32(val);

            inputs.push(val);

            rust_hexes.push(hex::encode(rust_result.to_le_bytes()));
        }

        let input_json = serde_json::to_string(&inputs).expect("failed to serialize inputs");

        let output = Command::new("node")
            .arg("../../scripts/float-parity-check.js")
            .arg(&input_json)
            .output()
            .expect("failed to execute node");

        if !output.status.success() {
            panic!(
                "Node process failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let js_hexes: Vec<String> =
            serde_json::from_slice(&output.stdout).expect("failed to parse JS output");

        assert_eq!(
            rust_hexes.len(),
            js_hexes.len(),
            "JS did not return expected number of results"
        );

        for i in 0..NUM_VECTORS {
            assert_eq!(
                rust_hexes[i],
                js_hexes[i].trim(),
                "Float parity mismatch for input[{}]: {} (Rust hex: {}, JS hex: {})",
                i,
                inputs[i],
                rust_hexes[i],
                js_hexes[i]
            );
        }
    }
}
