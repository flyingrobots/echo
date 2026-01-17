// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Channel identifiers and finalization policies.
//!
//! Channels are the unit of subscription and emission in the materialization bus.
//! Each channel has an identifier ([`ChannelId`]) and a finalization policy
//! ([`ChannelPolicy`]) that determines how multiple emissions are resolved.

use crate::ident::TypeId;

use super::reduce_op::ReduceOp;

/// Unique identifier for a materialization channel.
///
/// This is a 256-bit hash derived from a domain-separated label.
/// Use [`make_channel_id`] to create channel identifiers.
pub type ChannelId = TypeId;

/// Creates a channel identifier from a label string.
///
/// The label is domain-separated to avoid collisions with other hash usages.
///
/// # Examples
///
/// ```
/// use warp_core::materialization::make_channel_id;
///
/// let position_channel = make_channel_id("entity:position");
/// let velocity_channel = make_channel_id("entity:velocity");
///
/// assert_ne!(position_channel, velocity_channel);
/// ```
#[inline]
pub fn make_channel_id(label: &str) -> ChannelId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"channel:");
    hasher.update(label.as_bytes());
    TypeId(hasher.finalize().into())
}

/// Policy for resolving multiple emissions to the same channel within a tick.
///
/// The materialization bus collects emissions keyed by [`EmitKey`](super::EmitKey).
/// At finalization (post-commit), each channel resolves its emissions according
/// to its policy.
///
/// # Confluence Safety
///
/// All policies preserve confluence: the semantic result is independent of
/// rewrite execution order. Silent "winner picks" (e.g., max-key-wins) are
/// **banned** because they violate confluence by discarding values.
///
/// If you want single-value semantics, use [`StrictSingle`](Self::StrictSingle)
/// to catch violations, footprints (`b_out`) to enforce single-writer at
/// scheduling level, or [`Reduce`](Self::Reduce) with an explicit merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelPolicy {
    /// All emissions preserved in `EmitKey` order (default, confluence-safe).
    ///
    /// The finalized output is a sequence of all emissions, ordered by their
    /// canonical keys. Use this for event streams, logs, traces, and any
    /// multi-writer channel.
    #[default]
    Log,

    /// Error if more than one emission (catches bugs).
    ///
    /// Raises a [`ChannelConflict`] error if multiple rules emit to this
    /// channel in the same tick. Use this to enforce single-writer semantics
    /// and catch violations early.
    StrictSingle,

    /// Merge multiple emissions via a built-in reduce operation.
    ///
    /// The reduce operation is applied to values in `EmitKey` order. For truly
    /// order-independent results, use a commutative op like [`ReduceOp::Sum`].
    ///
    /// Use this for semantic coalescing where you need a single output value
    /// but multiple writers are expected.
    ///
    /// # Example
    ///
    /// ```
    /// use warp_core::materialization::{ChannelPolicy, ReduceOp};
    ///
    /// // Sum all emissions as u64
    /// let sum_policy = ChannelPolicy::Reduce(ReduceOp::Sum);
    ///
    /// // Take first emission by EmitKey order
    /// let first_policy = ChannelPolicy::Reduce(ReduceOp::First);
    /// ```
    Reduce(ReduceOp),
}

/// Classification of materialization errors.
///
/// This enum identifies the semantic reason for a finalization failure.
/// Having a typed "why" makes errors actionable and future-proofs for new
/// error categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationErrorKind {
    /// A [`ChannelPolicy::StrictSingle`] channel received multiple emissions.
    StrictSingleConflict,
    // Future candidates:
    // ReduceDecodeError,        // if reduce expects specific encoding
    // ReduceInvariantViolation, // if reduce op has preconditions
}

impl core::fmt::Display for MaterializationErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::StrictSingleConflict => write!(f, "strict single conflict"),
        }
    }
}

/// Error raised when a channel fails to finalize.
///
/// This is a structured error that identifies both the channel and the reason
/// for failure. The error is deterministic: same emissions → same error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelConflict {
    /// The channel that failed to finalize.
    pub channel: ChannelId,
    /// Number of distinct emissions (relevant for `StrictSingleConflict`).
    pub emission_count: usize,
    /// Classification of the error.
    pub kind: MaterializationErrorKind,
}

impl core::fmt::Display for ChannelConflict {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "channel {:?} failed: {} ({} emissions)",
            self.channel, self.kind, self.emission_count
        )
    }
}

impl std::error::Error for ChannelConflict {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_id_deterministic() {
        let id1 = make_channel_id("test:channel");
        let id2 = make_channel_id("test:channel");
        assert_eq!(id1, id2, "same label should produce same id");
    }

    #[test]
    fn channel_id_unique() {
        let id1 = make_channel_id("channel:a");
        let id2 = make_channel_id("channel:b");
        assert_ne!(id1, id2, "different labels should produce different ids");
    }

    #[test]
    fn channel_id_domain_separation() {
        // Even if the bytes are similar, domain prefix should differentiate
        let channel_id = make_channel_id("foo");
        let type_id = crate::ident::make_type_id("foo");
        assert_ne!(
            channel_id.0, type_id.0,
            "channel id and type id should differ due to domain separation"
        );
    }

    #[test]
    fn default_policy_is_log() {
        assert_eq!(ChannelPolicy::default(), ChannelPolicy::Log);
    }
}
