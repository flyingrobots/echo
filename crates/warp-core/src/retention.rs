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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetentionPolicy {
    /// Keep all history indefinitely. No checkpoints created automatically.
    KeepAll,

    /// Create checkpoints every `k` ticks. Keeps all history.
    CheckpointEvery {
        /// Interval between checkpoints in ticks.
        k: u64,
    },

    /// Keep only recent history within a sliding window.
    /// Older history is pruned but checkpoints are kept for reconstruction.
    KeepRecent {
        /// Number of ticks to keep in full detail.
        window: u64,
        /// Create checkpoints every this many ticks.
        checkpoint_every: u64,
    },

    /// Archive old history to wormhole storage (seam only, not implemented).
    /// This is a placeholder for future distributed storage integration.
    ArchiveToWormhole {
        /// Archive history older than this many ticks.
        after: u64,
        /// Create checkpoints every this many ticks before archiving.
        checkpoint_every: u64,
    },
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::KeepAll
    }
}
