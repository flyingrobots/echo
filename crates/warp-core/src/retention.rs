// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Retention policies for worldline history.
//!
//! This module defines [`RetentionPolicy`], which controls how much history
//! a worldline keeps and whether checkpoints are created to enable fast seeking.
//!
//! Retention policies allow balancing memory usage against the ability to
//! replay or inspect historical states:
//!
//! - [`RetentionPolicy::KeepAll`] - Keep everything, suitable for debugging or
//!   short-lived worldlines.
//! - [`RetentionPolicy::CheckpointEvery`] - Keep all history but create periodic
//!   checkpoints for faster seeking.
//! - [`RetentionPolicy::KeepRecent`] - Prune old history but keep checkpoints
//!   for reconstruction when needed.
//! - [`RetentionPolicy::ArchiveToWormhole`] - Future distributed storage
//!   integration (not yet implemented).

/// Retention policy for worldline history.
///
/// Controls how much history is kept and whether checkpoints are created
/// to enable fast seeking.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RetentionPolicy {
    /// Keep all history indefinitely. No checkpoints created automatically.
    #[default]
    KeepAll,

    /// Create checkpoints every `k` ticks. Keeps all history.
    ///
    /// # Valid Range
    ///
    /// `k` must be >= 1. A value of 0 is semantically undefined (would mean
    /// "checkpoint at every fractional tick") and will be treated as 1.
    CheckpointEvery {
        /// Interval between checkpoints in ticks. Must be >= 1.
        k: u64,
    },

    /// Keep only recent history within a sliding window.
    /// Older history is pruned but checkpoints are kept for reconstruction.
    ///
    /// # Valid Ranges
    ///
    /// - `window` must be >= 1 (a window of 0 would retain no history).
    /// - `checkpoint_every` must be >= 1 (same semantics as [`CheckpointEvery::k`]).
    KeepRecent {
        /// Number of ticks to keep in full detail. Must be >= 1.
        window: u64,
        /// Create checkpoints every this many ticks. Must be >= 1.
        checkpoint_every: u64,
    },

    /// Archive old history to wormhole storage (seam only, not implemented).
    /// This is a placeholder for future distributed storage integration.
    ///
    /// Not yet implemented — code that matches on this variant should handle it
    /// explicitly (e.g., return an error or log a warning).
    #[deprecated(note = "not yet implemented — will panic at runtime")]
    ArchiveToWormhole {
        /// Archive history older than this many ticks.
        after: u64,
        /// Create checkpoints every this many ticks before archiving.
        checkpoint_every: u64,
    },
}
