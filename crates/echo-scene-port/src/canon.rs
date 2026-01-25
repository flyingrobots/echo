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
    // Use integer cast for truncation (no_std compatible).
    // Cast to i64 to handle the full f32 range when scaled.
    let scaled = x * 1_000_000.0;
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
}
