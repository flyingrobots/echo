/// Stateful `xoroshiro128+` pseudo-random number generator for deterministic timelines.
///
/// * Not cryptographically secure; use only for gameplay/state simulation.
/// * Seeding controls reproducibility within a single process/run and matching
///   seeds yield identical sequences across supported platforms.
#[derive(Debug, Clone)]
pub struct Prng {
    state: [u64; 2],
}

impl Prng {
    /// Constructs a PRNG from two 64-bit seeds.
    ///
    /// Identical seeds produce identical sequences; the generator remains
    /// deterministic as long as each process consumes random numbers in the
    /// same order.
    ///
    /// If both `seed0` and `seed1` are zero, the implementation replaces them
    /// with a fixed non-zero constant so the internal state is never all-zero
    /// (avoids the xoroshiro128+ sink).
    pub fn from_seed(seed0: u64, seed1: u64) -> Self {
        let mut state = [seed0, seed1];
        if state[0] == 0 && state[1] == 0 {
            state[0] = 0x9e37_79b9_7f4a_7c15;
        }
        Self { state }
    }

    /// Constructs a PRNG from a single 64-bit seed via SplitMix64 expansion.
    pub fn from_seed_u64(seed: u64) -> Self {
        fn splitmix64(state: &mut u64) -> u64 {
            *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = *state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        let mut sm_state = seed;
        let mut state = [splitmix64(&mut sm_state), splitmix64(&mut sm_state)];
        if state[0] == 0 && state[1] == 0 {
            state[0] = 0x9e37_79b9_7f4a_7c15;
        }
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let s0 = self.state[0];
        let mut s1 = self.state[1];
        let result = s0.wrapping_add(s1);

        s1 ^= s0;
        self.state[0] = s0.rotate_left(55) ^ s1 ^ (s1 << 14);
        self.state[1] = s1.rotate_left(36);

        result
    }

    /// Returns the next float in `[0, 1)`.
    ///
    /// Uses the high 23 bits of the xoroshiro128+ state to fill the mantissa,
    /// ensuring uniform float32 sampling without relying on platform RNGs.
    pub fn next_f32(&mut self) -> f32 {
        let raw = self.next_u64();
        let bits = ((raw >> 41) as u32) | 0x3f80_0000;
        f32::from_bits(bits) - 1.0
    }

    /// Returns the next integer in the inclusive range `[min, max]`.
    ///
    /// # Panics
    /// Panics if `min > max`.
    ///
    /// Uses rejection sampling with a power-of-two fast path to avoid modulo
    /// bias, and supports the full `i32` span.
    pub fn next_int(&mut self, min: i32, max: i32) -> i32 {
        assert!(min <= max, "invalid range: {min}..={max}");
        let span = (i64::from(max) - i64::from(min)) as u64 + 1;
        if span == 1 {
            return min;
        }

        let value = if span.is_power_of_two() {
            self.next_u64() & (span - 1)
        } else {
            let bound = u64::MAX - u64::MAX % span;
            loop {
                let candidate = self.next_u64();
                if candidate < bound {
                    break candidate % span;
                }
            }
        };

        let offset = value as i64 + i64::from(min);
        offset as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_int_returns_single_value_for_equal_bounds() {
        let mut prng = Prng::from_seed(42, 99);
        assert_eq!(prng.next_int(7, 7), 7);
    }

    #[test]
    fn next_int_deterministic_across_calls() {
        let mut a = Prng::from_seed(123, 456);
        let mut b = Prng::from_seed(123, 456);
        for _ in 0..100 {
            assert_eq!(a.next_int(-10, 10), b.next_int(-10, 10));
        }
    }

    #[test]
    fn next_int_respects_bounds() {
        let mut prng = Prng::from_seed(42, 99);
        for _ in 0..1_000 {
            let v = prng.next_int(-10, 10);
            assert!((-10..=10).contains(&v));
        }
        for _ in 0..1_000 {
            let v = prng.next_int(i32::MIN, i32::MAX);
            // All i32 are valid; this simply exercises the path.
            let _ = v;
        }
    }

    #[cfg(feature = "golden_prng")]
    #[test]
    fn next_int_golden_regression() {
        let mut prng = Prng::from_seed(0xDEAD_BEEF, 0xFACE_FEED);
        let values: Vec<i32> = (0..3).map(|_| prng.next_int(i32::MIN, i32::MAX)).collect();
        assert_eq!(values, vec![1_501_347_292, 1_946_982_111, -117_316_573]);
    }
}
