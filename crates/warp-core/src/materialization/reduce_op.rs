// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Built-in reduce operations for channel coalescing.
//!
//! # Algebraic Categories
//!
//! **Commutative monoids** (permutation-invariant):
//! - [`Sum`](ReduceOp::Sum), [`Max`](ReduceOp::Max), [`Min`](ReduceOp::Min),
//!   [`BitOr`](ReduceOp::BitOr), [`BitAnd`](ReduceOp::BitAnd)
//! - Result is identical regardless of emission order
//!
//! **Order-dependent** (deterministic via [`EmitKey`](super::EmitKey) order):
//! - [`First`](ReduceOp::First), [`Last`](ReduceOp::Last), [`Concat`](ReduceOp::Concat)
//! - Result depends on canonical [`EmitKey`](super::EmitKey) ordering
//! - NOT commutative—do not claim they are!
//!
//! # Empty Input Behavior
//!
//! - [`Sum`](ReduceOp::Sum) returns `[0u8; 8]` (zero as u64 LE)
//! - All others return `[]` (empty vec)

/// Built-in reduce operations for channel coalescing.
///
/// These operations are used by [`ChannelPolicy::Reduce`](super::ChannelPolicy::Reduce)
/// to combine multiple emissions into a single value.
///
/// # Safety
///
/// All operations are deterministic. Commutative ops produce the same result
/// regardless of input order. Order-dependent ops rely on the canonical
/// [`EmitKey`](super::EmitKey) ordering provided by the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReduceOp {
    // ─── COMMUTATIVE MONOIDS ───────────────────────────────────────────
    /// Sum all values as little-endian u64.
    ///
    /// Values shorter than 8 bytes are zero-padded.
    /// Values longer than 8 bytes are truncated.
    ///
    /// Empty input → `[0u8; 8]` (zero).
    Sum,

    /// Take maximum value (lexicographic byte comparison).
    ///
    /// Empty input → `[]` (empty vec).
    Max,

    /// Take minimum value (lexicographic byte comparison).
    ///
    /// Empty input → `[]` (empty vec).
    Min,

    /// Bitwise OR all values.
    ///
    /// Shorter values are zero-padded on the right.
    /// Result length = maximum input length.
    ///
    /// Empty input → `[]` (empty vec).
    BitOr,

    /// Bitwise AND all values.
    ///
    /// Result length = minimum input length (intersection semantics).
    ///
    /// Empty input → `[]` (empty vec).
    BitAnd,

    // ─── ORDER-DEPENDENT (NOT COMMUTATIVE) ─────────────────────────────
    /// Take first value by [`EmitKey`](super::EmitKey) order.
    ///
    /// Empty input → `[]` (empty vec).
    ///
    /// **WARNING**: Not commutative. Depends on canonical key ordering.
    First,

    /// Take last value by [`EmitKey`](super::EmitKey) order.
    ///
    /// Empty input → `[]` (empty vec).
    ///
    /// **WARNING**: Not commutative. Depends on canonical key ordering.
    Last,

    /// Concatenate all values in [`EmitKey`](super::EmitKey) order.
    ///
    /// Empty input → `[]` (empty vec).
    ///
    /// **WARNING**: Not commutative. Order matters for result bytes.
    Concat,
}

impl ReduceOp {
    /// Returns true if this op is a commutative monoid (permutation-invariant).
    ///
    /// Commutative ops produce the same result regardless of input order.
    /// Order-dependent ops (First, Last, Concat) return false.
    #[inline]
    pub const fn is_commutative(&self) -> bool {
        matches!(
            self,
            Self::Sum | Self::Max | Self::Min | Self::BitOr | Self::BitAnd
        )
    }

    /// Apply this reduce operation to a set of values.
    ///
    /// Values are provided in [`EmitKey`](super::EmitKey) order (required for
    /// First/Last/Concat). Returns the reduced result.
    ///
    /// # Empty Input Behavior
    ///
    /// - `Sum` returns `[0u8; 8]` (zero as u64 LE)
    /// - All others return `[]` (empty vec)
    ///
    /// # Panics
    ///
    /// This method does not panic. Empty input is handled explicitly.
    #[allow(clippy::unwrap_used)] // Safe: we check for empty input before unwrapping
    pub fn apply<I>(self, values: I) -> Vec<u8>
    where
        I: IntoIterator<Item = Vec<u8>>,
    {
        let mut iter = values.into_iter().peekable();

        // Handle empty input uniformly (except Sum)
        if iter.peek().is_none() {
            return match self {
                Self::Sum => vec![0u8; 8], // Identity for addition
                _ => Vec::new(),           // "Nothing to reduce"
            };
        }

        match self {
            // ─── COMMUTATIVE MONOIDS ───────────────────────────────────
            Self::Sum => {
                let sum: u64 = iter
                    .map(|v| {
                        let mut buf = [0u8; 8];
                        let len = v.len().min(8);
                        buf[..len].copy_from_slice(&v[..len]);
                        u64::from_le_bytes(buf)
                    })
                    .fold(0u64, u64::wrapping_add);
                sum.to_le_bytes().to_vec()
            }

            Self::Max => {
                // unwrap safe: checked non-empty above
                iter.max().unwrap()
            }

            Self::Min => {
                // unwrap safe: checked non-empty above
                iter.min().unwrap()
            }

            Self::BitOr => {
                // unwrap safe: checked non-empty above
                iter.reduce(|acc, v| bitwise_or(&acc, &v)).unwrap()
            }

            Self::BitAnd => {
                // unwrap safe: checked non-empty above
                iter.reduce(|acc, v| bitwise_and(&acc, &v)).unwrap()
            }

            // ─── ORDER-DEPENDENT (EmitKey order matters) ───────────────
            Self::First => {
                // unwrap safe: checked non-empty above
                iter.next().unwrap()
            }

            Self::Last => {
                // unwrap safe: checked non-empty above
                iter.last().unwrap()
            }

            Self::Concat => iter.flatten().collect(),
        }
    }
}

/// Bitwise OR with zero-padding for shorter operand.
fn bitwise_or(a: &[u8], b: &[u8]) -> Vec<u8> {
    let len = a.len().max(b.len());
    let mut result = vec![0u8; len];
    for (i, byte) in result.iter_mut().enumerate() {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        *byte = av | bv;
    }
    result
}

/// Bitwise AND with truncation to shorter operand (intersection semantics).
fn bitwise_and(a: &[u8], b: &[u8]) -> Vec<u8> {
    let len = a.len().min(b.len());
    (0..len).map(|i| a[i] & b[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── COMMUTATIVITY CLASSIFICATION ──────────────────────────────────

    #[test]
    fn is_commutative_returns_true_for_monoids() {
        assert!(ReduceOp::Sum.is_commutative());
        assert!(ReduceOp::Max.is_commutative());
        assert!(ReduceOp::Min.is_commutative());
        assert!(ReduceOp::BitOr.is_commutative());
        assert!(ReduceOp::BitAnd.is_commutative());
    }

    #[test]
    fn is_commutative_returns_false_for_order_dependent() {
        assert!(!ReduceOp::First.is_commutative());
        assert!(!ReduceOp::Last.is_commutative());
        assert!(!ReduceOp::Concat.is_commutative());
    }

    // ─── EMPTY INPUT BEHAVIOR ──────────────────────────────────────────

    #[test]
    fn empty_input_sum_returns_zero() {
        let empty: Vec<Vec<u8>> = vec![];
        assert_eq!(ReduceOp::Sum.apply(empty), vec![0u8; 8]);
    }

    #[test]
    fn empty_input_others_return_empty() {
        let empty: Vec<Vec<u8>> = vec![];
        assert_eq!(ReduceOp::Max.apply(empty.clone()), vec![]);
        assert_eq!(ReduceOp::Min.apply(empty.clone()), vec![]);
        assert_eq!(ReduceOp::First.apply(empty.clone()), vec![]);
        assert_eq!(ReduceOp::Last.apply(empty.clone()), vec![]);
        assert_eq!(ReduceOp::BitOr.apply(empty.clone()), vec![]);
        assert_eq!(ReduceOp::BitAnd.apply(empty.clone()), vec![]);
        assert_eq!(ReduceOp::Concat.apply(empty), vec![]);
    }

    // ─── SUM ───────────────────────────────────────────────────────────

    #[test]
    fn sum_adds_u64_le_values() {
        let values = vec![
            1u64.to_le_bytes().to_vec(),
            2u64.to_le_bytes().to_vec(),
            3u64.to_le_bytes().to_vec(),
        ];
        let result = ReduceOp::Sum.apply(values);
        assert_eq!(result, 6u64.to_le_bytes().to_vec());
    }

    #[test]
    fn sum_pads_short_values() {
        // Single byte values treated as u64
        let values = vec![vec![1], vec![2], vec![3]];
        let result = ReduceOp::Sum.apply(values);
        assert_eq!(result, 6u64.to_le_bytes().to_vec());
    }

    #[test]
    fn sum_wraps_on_overflow() {
        let values = vec![u64::MAX.to_le_bytes().to_vec(), 1u64.to_le_bytes().to_vec()];
        let result = ReduceOp::Sum.apply(values);
        // u64::MAX + 1 wraps to 0
        assert_eq!(result, 0u64.to_le_bytes().to_vec());
    }

    #[test]
    fn sum_is_permutation_invariant() {
        let a = 100u64.to_le_bytes().to_vec();
        let b = 200u64.to_le_bytes().to_vec();
        let c = 300u64.to_le_bytes().to_vec();

        let r1 = ReduceOp::Sum.apply(vec![a.clone(), b.clone(), c.clone()]);
        let r2 = ReduceOp::Sum.apply(vec![c.clone(), a.clone(), b.clone()]);
        let r3 = ReduceOp::Sum.apply(vec![b, c, a]);

        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
    }

    // ─── MAX / MIN ─────────────────────────────────────────────────────

    #[test]
    fn max_returns_lexicographic_maximum() {
        let values = vec![vec![1, 2], vec![1, 3], vec![1, 1]];
        assert_eq!(ReduceOp::Max.apply(values), vec![1, 3]);
    }

    #[test]
    fn min_returns_lexicographic_minimum() {
        let values = vec![vec![1, 2], vec![1, 3], vec![1, 1]];
        assert_eq!(ReduceOp::Min.apply(values), vec![1, 1]);
    }

    #[test]
    fn max_is_permutation_invariant() {
        let a = vec![0xFF, 0x00];
        let b = vec![0x00, 0xFF];
        let c = vec![0x80, 0x80];

        let r1 = ReduceOp::Max.apply(vec![a.clone(), b.clone(), c.clone()]);
        let r2 = ReduceOp::Max.apply(vec![c, b, a]);

        assert_eq!(r1, r2);
        assert_eq!(r1, vec![0xFF, 0x00]); // 0xFF > 0x80 > 0x00 in first byte
    }

    // ─── BITOR / BITAND ────────────────────────────────────────────────

    #[test]
    fn bitor_combines_bits() {
        let values = vec![vec![0b0000_1111], vec![0b1111_0000]];
        assert_eq!(ReduceOp::BitOr.apply(values), vec![0b1111_1111]);
    }

    #[test]
    fn bitor_pads_shorter_operand() {
        let values = vec![vec![0xFF], vec![0x00, 0xFF]];
        // First value padded to [0xFF, 0x00]
        // OR with [0x00, 0xFF] = [0xFF, 0xFF]
        assert_eq!(ReduceOp::BitOr.apply(values), vec![0xFF, 0xFF]);
    }

    #[test]
    fn bitand_intersects_bits() {
        let values = vec![vec![0b1111_1111], vec![0b1010_1010]];
        assert_eq!(ReduceOp::BitAnd.apply(values), vec![0b1010_1010]);
    }

    #[test]
    fn bitand_truncates_to_shorter() {
        let values = vec![vec![0xFF, 0xFF, 0xFF], vec![0xAA, 0xBB]];
        // Result length = min(3, 2) = 2
        assert_eq!(ReduceOp::BitAnd.apply(values), vec![0xAA, 0xBB]);
    }

    // ─── ORDER-DEPENDENT ───────────────────────────────────────────────

    #[test]
    fn first_returns_first_in_order() {
        let values = vec![vec![1], vec![2], vec![3]];
        assert_eq!(ReduceOp::First.apply(values), vec![1]);
    }

    #[test]
    fn last_returns_last_in_order() {
        let values = vec![vec![1], vec![2], vec![3]];
        assert_eq!(ReduceOp::Last.apply(values), vec![3]);
    }

    #[test]
    fn concat_preserves_order() {
        let values = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
        assert_eq!(ReduceOp::Concat.apply(values), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn order_dependent_ops_are_not_permutation_invariant() {
        let a = vec![0xAA];
        let b = vec![0xBB];

        // First depends on order
        assert_ne!(
            ReduceOp::First.apply(vec![a.clone(), b.clone()]),
            ReduceOp::First.apply(vec![b.clone(), a.clone()])
        );

        // Last depends on order
        assert_ne!(
            ReduceOp::Last.apply(vec![a.clone(), b.clone()]),
            ReduceOp::Last.apply(vec![b.clone(), a.clone()])
        );

        // Concat depends on order
        assert_ne!(
            ReduceOp::Concat.apply(vec![a.clone(), b.clone()]),
            ReduceOp::Concat.apply(vec![b, a])
        );
    }
}
