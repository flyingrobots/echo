// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The fixtures' pseudo-random generator.
//!
//! `rand`'s `StdRng` and `SmallRng` carry no reproducibility guarantee across
//! versions, and `warp-core` takes no `rand` dependency (it would change the
//! `Cargo.lock` the provider package binds). Fixtures use SplitMix64, defined
//! here, and every mapping built on it is pinned with its own vectors.
//!
//! Changing an output sequence or a mapping is a generator-version change.

/// Version of the generator and every mapping in this file.
pub const GENERATOR_VERSION: u32 = 1;

/// Name recorded in manifests.
pub const ALGORITHM: &str = "splitmix64";

/// SplitMix64 (Steele, Lea and Flood, 2014), as in Vigna's reference code.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator whose first output is derived from `seed`.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform integer in `[0, bound)`, or `None` when `bound` is zero.
    ///
    /// Mapping: draw outputs until one is at least
    /// `(2^64 - bound) mod bound`, then return it modulo `bound`. The
    /// rejected low zone makes the result exactly uniform. Each draw consumes
    /// one or more outputs; the count depends only on the outputs.
    pub fn below(&mut self, bound: u64) -> Option<u64> {
        if bound == 0 {
            return None;
        }
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let draw = self.next_u64();
            if draw >= threshold {
                return Some(draw % bound);
            }
        }
    }

    /// `len` bytes: successive outputs in little-endian order, the last one
    /// truncated.
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let word = self.next_u64().to_le_bytes();
            let take = (len - out.len()).min(word.len());
            out.extend_from_slice(&word[..take]);
        }
        out
    }
}
