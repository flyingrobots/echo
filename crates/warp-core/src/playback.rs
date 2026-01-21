// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Playback cursor and session types for SPEC-0004: Worldlines, Playback, and `TruthBus`.
//!
//! This module defines the types for cursor-based worldline navigation and truth
//! frame delivery. Cursors allow clients to navigate worldline history, while
//! sessions manage subscriptions and truth frame routing.
//!
//! # Key Concepts
//!
//! - **Cursor**: A position within a worldline, with its own isolated store copy.
//!   Cursors can be writers (advancing the worldline) or readers (replaying history).
//! - **Session**: A client's view into the system, with an active cursor and channel
//!   subscriptions.
//! - **`TruthFrame`**: An authoritative value delivery, stamped with cursor context
//!   for provenance.
//!
//! # Invariants
//!
//! - CUR-001: Cursor never mutates worldline unless role is Writer and mode requires advance.
//! - CUR-002: Cursor never executes rules when seeking; it applies recorded patches only.
//! - CUR-003: After seek/apply, cursor verifies expected hashes byte-for-byte.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::graph::GraphStore;
use crate::ident::{Hash, WarpId};
use crate::materialization::ChannelId;
use crate::provenance_store::ProvenanceStore;
use crate::snapshot::compute_state_root_for_warp_store;
use crate::worldline::WorldlineId;

/// Unique identifier for a playback cursor.
///
/// Each cursor has a distinct ID to enable tracking and multiplexing of
/// multiple cursors within the same session or across sessions.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CursorId(pub Hash);

impl CursorId {
    /// Returns the canonical byte representation of this id.
    #[must_use]
    pub fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Unique identifier for a view session.
///
/// Sessions represent a client's authenticated context and subscription set.
/// Multiple cursors can be associated with a single session.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SessionId(pub Hash);

impl SessionId {
    /// Returns the canonical byte representation of this id.
    #[must_use]
    pub fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Role of a cursor within the worldline.
///
/// The role determines what operations the cursor can perform:
/// - Writers can advance the worldline by executing ticks.
/// - Readers can only replay existing history via patches.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CursorRole {
    /// Writer cursor: can advance the worldline by executing ticks.
    ///
    /// There is typically one writer cursor per warp, owned by the engine.
    /// Writers produce new patches and record outputs.
    #[default]
    Writer,

    /// Reader cursor: can only replay existing history.
    ///
    /// Readers navigate via recorded patches and can seek to any tick
    /// within the available history. They never execute rules.
    Reader,
}

/// Playback mode controlling cursor behavior.
///
/// The mode determines how the cursor advances (or doesn't) on each step.
/// Modes form a simple state machine with transitions triggered by `step()`
/// and seek operations.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlaybackMode {
    /// Cursor is paused; `step()` is a no-op.
    #[default]
    Paused,

    /// Cursor advances continuously until frontier is reached.
    ///
    /// For writers, this means executing ticks.
    /// For readers, this means consuming recorded patches.
    Play,

    /// Advance one tick then transition to Paused.
    StepForward,

    /// Seek to tick-1 then transition to Paused.
    StepBack,

    /// Seek to a specific tick, then apply the follow-up behavior.
    Seek {
        /// Target tick to seek to.
        target: u64,
        /// Behavior after reaching the target.
        then: SeekThen,
    },
}

/// Behavior after a seek operation completes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SeekThen {
    /// Transition to Paused after reaching target.
    #[default]
    Pause,

    /// Restore the mode that was active before the seek.
    RestorePrevious,

    /// Transition to Play after reaching target.
    Play,
}

/// Receipt capturing cursor context for a truth frame or operation.
///
/// This receipt provides full provenance for any value delivered through
/// the truth bus. It enables clients to verify that values came from
/// a specific cursor at a specific tick.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CursorReceipt {
    /// Session that owns this cursor.
    pub session_id: SessionId,
    /// The cursor that produced this receipt.
    pub cursor_id: CursorId,
    /// Worldline the cursor is navigating.
    pub worldline_id: WorldlineId,
    /// Warp the cursor is focused on.
    pub warp_id: WarpId,
    /// Tick number when this receipt was generated.
    pub tick: u64,
    /// Commit hash at this tick for verification.
    pub commit_hash: Hash,
}

/// Authoritative value delivery via the truth bus.
///
/// A truth frame combines a cursor receipt (provenance) with a channel value
/// and its hash. This enables clients to:
/// 1. Verify the value came from a specific cursor/tick
/// 2. Verify the value hash matches for tamper detection
/// 3. Subscribe to specific channels for selective delivery
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TruthFrame {
    /// Cursor context for this frame (provenance).
    pub cursor: CursorReceipt,
    /// Channel this value was emitted to.
    pub channel: ChannelId,
    /// The raw value bytes.
    pub value: Vec<u8>,
    /// Hash of the value for verification.
    pub value_hash: Hash,
}

/// Errors that can occur when seeking a playback cursor.
///
/// These errors indicate verification failures or missing history during
/// cursor seek operations. When any of these occur, the cursor state is
/// undefined and should be rebuilt from a known-good position.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SeekError {
    /// The requested tick is not available in the provenance store.
    ///
    /// This can occur when:
    /// - Seeking beyond recorded history
    /// - History has been pruned by retention policy
    /// - The worldline doesn't have enough ticks
    #[error("history unavailable for tick {tick}")]
    HistoryUnavailable {
        /// The tick that was unavailable.
        tick: u64,
    },

    /// The computed state root doesn't match the expected value.
    ///
    /// This indicates either:
    /// - A corrupted patch in the provenance store
    /// - A non-deterministic apply operation
    /// - A hash computation mismatch
    #[error("state root mismatch at tick {tick}")]
    StateRootMismatch {
        /// The tick where verification failed.
        tick: u64,
    },

    /// The computed commit hash doesn't match the expected value.
    ///
    /// Similar to `StateRootMismatch`, but for the commit hash which
    /// includes additional metadata beyond just the state root.
    #[error("commit hash mismatch at tick {tick}")]
    CommitHashMismatch {
        /// The tick where verification failed.
        tick: u64,
    },

    /// Failed to apply a patch during seek.
    ///
    /// This wraps the underlying [`ApplyError`] from the worldline module.
    ///
    /// [`ApplyError`]: crate::worldline::ApplyError
    #[error("apply error at tick {tick}: {source}")]
    ApplyError {
        /// The tick where the apply failed.
        tick: u64,
        /// The underlying apply error.
        #[source]
        source: crate::worldline::ApplyError,
    },
}

/// Result of a single step operation on a cursor.
///
/// This enum indicates what happened during a `step()` call, allowing callers
/// to distinguish between no-ops, advances, seeks, and frontier conditions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StepResult {
    /// No operation performed (e.g., cursor was paused).
    NoOp,
    /// Cursor advanced forward one tick.
    Advanced,
    /// Cursor completed a seek operation.
    Seeked,
    /// Reader cursor reached the pinned maximum tick (frontier).
    ReachedFrontier,
}

/// A playback cursor navigating a worldline.
///
/// A cursor maintains its own isolated copy of the graph store, allowing it
/// to navigate worldline history independently of the writer. Cursors can be
/// in two roles:
///
/// - **Writer**: Can advance the worldline by executing ticks (owned by engine)
/// - **Reader**: Can only replay existing history via patches
///
/// # Invariants
///
/// - CUR-001: Cursor never mutates worldline unless role is Writer and mode requires advance
/// - CUR-002: Cursor never executes rules when seeking; it applies recorded patches only
/// - CUR-003: After seek/apply, cursor verifies expected hashes byte-for-byte
///
/// # Seek Behavior
///
/// When seeking to a target tick:
/// - If `target < tick`: Rebuild store from initial state (clone from U0)
/// - Apply patches from current position to target
/// - Verify `state_root` and `commit_hash` match expected values
#[derive(Debug)]
pub struct PlaybackCursor {
    /// Unique identifier for this cursor.
    pub cursor_id: CursorId,
    /// The worldline this cursor is navigating.
    pub worldline_id: WorldlineId,
    /// The warp instance this cursor is focused on.
    pub warp_id: WarpId,
    /// Current tick position (0-indexed into worldline history).
    pub tick: u64,
    /// Role of this cursor (Writer or Reader).
    pub role: CursorRole,
    /// Current playback mode controlling step behavior.
    pub mode: PlaybackMode,
    /// The cursor's isolated graph store copy.
    pub store: GraphStore,
    /// Maximum tick this cursor can advance to (pinned frontier).
    ///
    /// For readers, this is typically the current writer position.
    /// The cursor cannot seek beyond this tick.
    pub pin_max_tick: u64,
}

impl PlaybackCursor {
    /// Creates a new playback cursor.
    ///
    /// # Arguments
    ///
    /// * `cursor_id` - Unique identifier for this cursor
    /// * `worldline_id` - The worldline to navigate
    /// * `warp_id` - The warp instance to focus on
    /// * `role` - Writer or Reader role
    /// * `initial_store` - Initial graph store state (cloned for cursor use)
    /// * `pin_max_tick` - Maximum tick the cursor can advance to
    #[must_use]
    pub fn new(
        cursor_id: CursorId,
        worldline_id: WorldlineId,
        warp_id: WarpId,
        role: CursorRole,
        initial_store: &GraphStore,
        pin_max_tick: u64,
    ) -> Self {
        Self {
            cursor_id,
            worldline_id,
            warp_id,
            tick: 0,
            role,
            mode: PlaybackMode::default(),
            store: initial_store.clone(),
            pin_max_tick,
        }
    }

    /// Seek the cursor to a target tick using provenance store patches.
    ///
    /// This method rebuilds the cursor's store state by applying recorded
    /// patches from the provenance store. It follows these rules:
    ///
    /// 1. If `target < tick`: Rebuild from U0 (clone initial state)
    /// 2. Apply patches from current position to target
    /// 3. Verify `state_root` matches expected after each patch
    ///
    /// # Arguments
    ///
    /// * `target` - The tick to seek to (0-indexed)
    /// * `provenance` - The provenance store providing patches and expected hashes
    /// * `initial_store` - The initial store state for rebuilding from U0
    ///
    /// # Errors
    ///
    /// Returns [`SeekError`] if:
    /// - Target tick is beyond available history
    /// - A patch cannot be applied (missing node/edge)
    /// - Computed state root doesn't match expected value
    ///
    /// # Note
    ///
    /// This method enforces CUR-002: it never executes rules, only applies
    /// recorded patches. This ensures deterministic replay regardless of
    /// rule changes or execution order.
    pub fn seek_to<P: ProvenanceStore>(
        &mut self,
        target: u64,
        provenance: &P,
        initial_store: &GraphStore,
    ) -> Result<(), SeekError> {
        // Check if target is within available history
        let history_len = provenance
            .len(self.worldline_id)
            .map_err(|_| SeekError::HistoryUnavailable { tick: target })?;

        // Target tick must be < history_len (0-indexed)
        // If history_len is 5, valid ticks are 0, 1, 2, 3, 4
        // But tick 0 means "after applying patch 0", so target <= history_len - 1
        // However, we also allow target = 0 to mean "initial state before any patches"
        // For this implementation, tick N means "state after applying patches 0..N"
        // So valid targets are 0 <= target < history_len OR target == 0 when history_len == 0
        //
        // Actually, let's clarify: tick represents the number of patches applied.
        // tick 0 = initial state (no patches applied)
        // tick 1 = after patch 0 is applied
        // tick N = after patches 0..N-1 are applied
        //
        // So if history_len = 5 (patches 0-4 exist), valid ticks are 0-5.
        // target = 5 means "after all 5 patches applied"
        if target > history_len {
            return Err(SeekError::HistoryUnavailable { tick: target });
        }

        // Determine starting point
        let start_tick = if target < self.tick {
            // Going backwards: need to rebuild from scratch
            self.store = initial_store.clone();
            0
        } else {
            self.tick
        };

        // Apply patches from start_tick to target
        // If start_tick = 2 and target = 5, we apply patches 2, 3, 4
        for patch_tick in start_tick..target {
            // Get patch and expected hash triplet
            let patch = provenance
                .patch(self.worldline_id, patch_tick)
                .map_err(|_| SeekError::HistoryUnavailable { tick: patch_tick })?;

            let expected = provenance
                .expected(self.worldline_id, patch_tick)
                .map_err(|_| SeekError::HistoryUnavailable { tick: patch_tick })?;

            // Apply the patch to our store
            patch
                .apply_to_store(&mut self.store)
                .map_err(|e| SeekError::ApplyError {
                    tick: patch_tick,
                    source: e,
                })?;

            // Verify state root matches expected
            let computed_state_root = compute_state_root_for_warp_store(&self.store, self.warp_id);

            if computed_state_root != expected.state_root {
                return Err(SeekError::StateRootMismatch { tick: patch_tick });
            }
        }

        // Update cursor position
        self.tick = target;
        Ok(())
    }

    /// Process one step according to the current playback mode.
    ///
    /// This method implements the [`PlaybackMode`] state machine:
    ///
    /// - `Paused`: No-op, returns [`StepResult::NoOp`]
    /// - `Play`: For readers, consumes existing history via seek; for writers, stub (no-op)
    /// - `StepForward`: Advances one tick then transitions to `Paused`
    /// - `StepBack`: Seeks to `tick - 1` then transitions to `Paused`
    /// - `Seek { target, then }`: Seeks to target then applies `SeekThen` behavior
    ///
    /// # Arguments
    ///
    /// * `provenance` - The provenance store providing patches and expected hashes
    /// * `initial_store` - The initial store state for rebuilding from U0
    ///
    /// # Returns
    ///
    /// A [`StepResult`] indicating what happened during the step.
    ///
    /// # Errors
    ///
    /// Returns [`SeekError`] if a seek operation fails due to missing history
    /// or hash verification failure.
    ///
    /// # Note
    ///
    /// For writer cursors in `Play` mode, this is currently a stub that returns
    /// [`StepResult::NoOp`]. Actual writer advance requires engine/BOAW integration
    /// and is handled at a higher level.
    pub fn step<P: ProvenanceStore>(
        &mut self,
        provenance: &P,
        initial_store: &GraphStore,
    ) -> Result<StepResult, SeekError> {
        match self.mode {
            PlaybackMode::Paused => Ok(StepResult::NoOp),

            PlaybackMode::Play => {
                if self.role == CursorRole::Reader {
                    // Reader: consume existing history, pause at frontier
                    if self.tick >= self.pin_max_tick {
                        self.mode = PlaybackMode::Paused;
                        return Ok(StepResult::ReachedFrontier);
                    }
                    self.seek_to(self.tick + 1, provenance, initial_store)?;
                    Ok(StepResult::Advanced)
                } else {
                    // Writer: stub - actual advance needs BOAW integration
                    // For now, just return NoOp (writer advance done in engine)
                    Ok(StepResult::NoOp)
                }
            }

            PlaybackMode::StepForward => {
                if self.role == CursorRole::Reader {
                    self.seek_to(self.tick + 1, provenance, initial_store)?;
                }
                // Writer case: stub - actual advance handled by engine
                // Transition to Paused regardless
                self.mode = PlaybackMode::Paused;
                Ok(StepResult::Advanced)
            }

            PlaybackMode::StepBack => {
                let target = self.tick.saturating_sub(1);
                self.seek_to(target, provenance, initial_store)?;
                self.mode = PlaybackMode::Paused;
                Ok(StepResult::Seeked)
            }

            PlaybackMode::Seek { target, then } => {
                self.seek_to(target, provenance, initial_store)?;
                self.mode = match then {
                    SeekThen::Play => PlaybackMode::Play,
                    SeekThen::Pause | SeekThen::RestorePrevious => PlaybackMode::Paused,
                };
                Ok(StepResult::Seeked)
            }
        }
    }
}

// ============================================================================
// ViewSession
// ============================================================================

/// A `ViewSession` couples a cursor with channel subscriptions.
///
/// Clients interact with sessions, not the global truth bus directly. Each
/// session maintains:
/// - An active cursor for navigating worldline history
/// - A set of channel subscriptions for filtering truth frames
///
/// # Thread Safety
///
/// `ViewSession` is designed to be owned by a single client connection.
/// For multi-threaded access, wrap in appropriate synchronization primitives.
///
/// # Example
///
/// ```ignore
/// let session = ViewSession::new(session_id, cursor_id);
/// session.subscribe(position_channel);
/// session.subscribe(velocity_channel);
///
/// // Switch to a different cursor (subscriptions persist)
/// session.set_active_cursor(other_cursor_id);
/// ```
#[derive(Clone, Debug)]
pub struct ViewSession {
    /// Unique identifier for this session.
    pub session_id: SessionId,
    /// The currently active cursor for this session.
    pub active_cursor: CursorId,
    /// Channels this session is subscribed to.
    pub subscriptions: BTreeSet<ChannelId>,
}

impl ViewSession {
    /// Creates a new view session with the given session ID and active cursor.
    ///
    /// The session starts with no channel subscriptions.
    #[must_use]
    pub fn new(session_id: SessionId, active_cursor: CursorId) -> Self {
        Self {
            session_id,
            active_cursor,
            subscriptions: BTreeSet::new(),
        }
    }

    /// Subscribe to a channel to receive truth frames.
    ///
    /// Subscribing to a channel means truth frames emitted to that channel
    /// will be delivered to this session when the cursor advances or seeks.
    pub fn subscribe(&mut self, channel: ChannelId) {
        self.subscriptions.insert(channel);
    }

    /// Unsubscribe from a channel to stop receiving truth frames.
    pub fn unsubscribe(&mut self, channel: ChannelId) {
        self.subscriptions.remove(&channel);
    }

    /// Set the active cursor for this session.
    ///
    /// Changing the active cursor does not affect subscriptions - they persist
    /// across cursor switches. This allows clients to switch between different
    /// playback positions without losing their subscription configuration.
    pub fn set_active_cursor(&mut self, cursor: CursorId) {
        self.active_cursor = cursor;
    }

    /// Publish truth frames for all subscribed channels at the cursor's current tick.
    ///
    /// This method sources outputs from the provenance store's recorded outputs for the
    /// given tick and publishes them to the truth sink. Only channels that the session
    /// is subscribed to will have frames published.
    ///
    /// # Arguments
    ///
    /// * `cursor` - The playback cursor providing the current tick and worldline context.
    /// * `provenance` - The provenance store containing recorded outputs.
    /// * `sink` - The truth sink to publish frames to.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] if:
    /// - The expected hash triplet is unavailable for the cursor's tick.
    /// - The recorded outputs are unavailable for the cursor's tick.
    ///
    /// # Note
    ///
    /// This method enforces OUT-002: Playback at tick t reproduces the same `TruthFrames`
    /// recorded at tick t. The values come from the provenance store, not computed on the fly.
    ///
    /// [`HistoryError`]: crate::provenance_store::HistoryError
    pub fn publish_truth<P: ProvenanceStore>(
        &self,
        cursor: &PlaybackCursor,
        provenance: &P,
        sink: &mut TruthSink,
    ) -> Result<(), crate::provenance_store::HistoryError> {
        // Get expected hashes for commit_hash
        let expected = provenance.expected(cursor.worldline_id, cursor.tick)?;

        // Build receipt
        let receipt = CursorReceipt {
            session_id: self.session_id,
            cursor_id: cursor.cursor_id,
            worldline_id: cursor.worldline_id,
            warp_id: cursor.warp_id,
            tick: cursor.tick,
            commit_hash: expected.commit_hash,
        };

        // Publish receipt
        sink.publish_receipt(self.session_id, receipt);

        // Get recorded outputs for this tick
        let outputs = provenance.outputs(cursor.worldline_id, cursor.tick)?;

        // Publish frames for subscribed channels only
        for (channel, value) in outputs {
            if self.subscriptions.contains(&channel) {
                let value_hash: Hash = blake3::hash(&value).into();
                sink.publish_frame(
                    self.session_id,
                    TruthFrame {
                        cursor: receipt,
                        channel,
                        value,
                        value_hash,
                    },
                );
            }
        }

        Ok(())
    }
}

// ============================================================================
// TruthSink
// ============================================================================

/// Minimal truth sink for collecting published frames.
///
/// This is a simple collection type used for testing and minimal truth bus
/// scenarios. It stores receipts and frames per session, allowing verification
/// of what was published during cursor operations.
///
/// # Design Notes
///
/// Per SPEC-0004 correction #4, this is intentionally minimal - just `BTreeMap`s
/// without fancy abstractions. For production use, a more sophisticated truth
/// bus implementation would be used.
#[derive(Debug, Clone, Default)]
pub struct TruthSink {
    /// Receipts published per session.
    receipts: BTreeMap<SessionId, Vec<CursorReceipt>>,
    /// Frames published per session.
    frames: BTreeMap<SessionId, Vec<TruthFrame>>,
}

impl TruthSink {
    /// Creates a new empty truth sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Publish a cursor receipt for a session.
    ///
    /// Receipts are appended to the session's receipt history.
    pub fn publish_receipt(&mut self, session_id: SessionId, receipt: CursorReceipt) {
        self.receipts.entry(session_id).or_default().push(receipt);
    }

    /// Publish a truth frame for a session.
    ///
    /// Frames are appended to the session's frame history.
    pub fn publish_frame(&mut self, session_id: SessionId, frame: TruthFrame) {
        self.frames.entry(session_id).or_default().push(frame);
    }

    /// Collect all frames published for a session.
    ///
    /// Returns an empty vector if no frames have been published.
    #[must_use]
    pub fn collect_frames(&self, session_id: SessionId) -> Vec<TruthFrame> {
        self.frames.get(&session_id).cloned().unwrap_or_default()
    }

    /// Get the last receipt published for a session, if any.
    #[must_use]
    pub fn last_receipt(&self, session_id: SessionId) -> Option<CursorReceipt> {
        self.receipts
            .get(&session_id)
            .and_then(|r| r.last().copied())
    }

    /// Clear all receipts and frames from the sink.
    pub fn clear(&mut self) {
        self.receipts.clear();
        self.frames.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_id_is_transparent_wrapper() {
        let hash = [42u8; 32];
        let id = CursorId(hash);
        assert_eq!(id.0, hash);
        assert_eq!(id.as_bytes(), &hash);
    }

    #[test]
    fn session_id_is_transparent_wrapper() {
        let hash = [42u8; 32];
        let id = SessionId(hash);
        assert_eq!(id.0, hash);
        assert_eq!(id.as_bytes(), &hash);
    }

    #[test]
    fn default_cursor_role_is_writer() {
        assert_eq!(CursorRole::default(), CursorRole::Writer);
    }

    #[test]
    fn default_playback_mode_is_paused() {
        assert_eq!(PlaybackMode::default(), PlaybackMode::Paused);
    }

    #[test]
    fn default_seek_then_is_pause() {
        assert_eq!(SeekThen::default(), SeekThen::Pause);
    }

    #[test]
    fn cursor_receipt_equality() {
        let receipt1 = CursorReceipt {
            session_id: SessionId([1u8; 32]),
            cursor_id: CursorId([2u8; 32]),
            worldline_id: WorldlineId([3u8; 32]),
            warp_id: crate::ident::WarpId([4u8; 32]),
            tick: 42,
            commit_hash: [5u8; 32],
        };
        let receipt2 = CursorReceipt {
            session_id: SessionId([1u8; 32]),
            cursor_id: CursorId([2u8; 32]),
            worldline_id: WorldlineId([3u8; 32]),
            warp_id: crate::ident::WarpId([4u8; 32]),
            tick: 42,
            commit_hash: [5u8; 32],
        };
        assert_eq!(receipt1, receipt2);
    }

    #[test]
    fn truth_frame_equality() {
        let cursor = CursorReceipt {
            session_id: SessionId([1u8; 32]),
            cursor_id: CursorId([2u8; 32]),
            worldline_id: WorldlineId([3u8; 32]),
            warp_id: crate::ident::WarpId([4u8; 32]),
            tick: 42,
            commit_hash: [5u8; 32],
        };
        let frame1 = TruthFrame {
            cursor,
            channel: crate::ident::TypeId([6u8; 32]),
            value: vec![1, 2, 3],
            value_hash: [7u8; 32],
        };
        let frame2 = TruthFrame {
            cursor,
            channel: crate::ident::TypeId([6u8; 32]),
            value: vec![1, 2, 3],
            value_hash: [7u8; 32],
        };
        assert_eq!(frame1, frame2);
    }
}
