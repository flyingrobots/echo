// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Boundary port for materialization subscriptions.
//!
//! The [`MaterializationPort`] is the external-facing API for consuming
//! materialized channel data. It provides:
//!
//! - **Subscriptions**: Subscribe to specific channels by ID
//! - **Replay(1)**: Late joiners receive the last finalized value per channel
//! - **Batched drain**: Collect pending frames for transport
//!
//! # Usage
//!
//! ```rust
//! use warp_core::materialization::{make_channel_id, FinalizedChannel, MaterializationPort};
//!
//! let mut port = MaterializationPort::new();
//! let position_channel = make_channel_id("demo:position");
//!
//! // Subscribe to a channel (returns cached value if available)
//! let cached = port.subscribe(position_channel);
//! assert!(cached.is_none());
//!
//! // After engine commit, port receives finalized data
//! port.receive_finalized(vec![FinalizedChannel {
//!     channel: position_channel,
//!     data: vec![1, 2, 3],
//! }]);
//!
//! // Drain pending frames for transport
//! let frames = port.drain();
//! assert_eq!(frames.len(), 1);
//! ```

use std::collections::{BTreeMap, BTreeSet};

use super::bus::FinalizedChannel;
use super::channel::ChannelId;
use super::frame::MaterializationFrame;

/// Boundary port for external materialization consumers.
///
/// Manages subscriptions, replay cache, and pending frame queue.
#[derive(Debug, Default)]
pub struct MaterializationPort {
    /// Set of subscribed channel IDs.
    subscriptions: BTreeSet<ChannelId>,

    /// Replay(1) cache: last finalized data per channel.
    /// Updated for all channels, not just subscribed ones.
    replay_cache: BTreeMap<ChannelId, Vec<u8>>,

    /// Pending frames to be drained by the transport layer.
    /// Only includes frames for subscribed channels.
    pending_frames: Vec<MaterializationFrame>,
}

impl MaterializationPort {
    /// Creates a new empty port with no subscriptions.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribes to a channel.
    ///
    /// Returns the cached value if available (replay(1) semantics).
    /// Future emissions to this channel will be queued for drain.
    #[inline]
    pub fn subscribe(&mut self, channel: ChannelId) -> Option<Vec<u8>> {
        self.subscriptions.insert(channel);
        self.replay_cache.get(&channel).cloned()
    }

    /// Unsubscribes from a channel.
    ///
    /// Future emissions to this channel will not be queued.
    /// The replay cache is NOT cleared (other subscribers may need it).
    #[inline]
    pub fn unsubscribe(&mut self, channel: ChannelId) {
        self.subscriptions.remove(&channel);
    }

    /// Returns true if subscribed to the given channel.
    #[inline]
    pub fn is_subscribed(&self, channel: &ChannelId) -> bool {
        self.subscriptions.contains(channel)
    }

    /// Returns the number of active subscriptions.
    #[inline]
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Receives finalized channel data from the bus.
    ///
    /// Updates the replay cache for all channels, and queues frames
    /// for subscribed channels.
    pub fn receive_finalized(&mut self, finalized: Vec<FinalizedChannel>) {
        for fc in finalized {
            // Always update replay cache
            self.replay_cache.insert(fc.channel, fc.data.clone());

            // Only queue frame if subscribed
            if self.subscriptions.contains(&fc.channel) {
                self.pending_frames
                    .push(MaterializationFrame::new(fc.channel, fc.data));
            }
        }
    }

    /// Returns true if there are pending frames to drain.
    #[inline]
    pub fn has_pending(&self) -> bool {
        !self.pending_frames.is_empty()
    }

    /// Returns the number of pending frames.
    #[inline]
    pub fn pending_count(&self) -> usize {
        self.pending_frames.len()
    }

    /// Drains all pending frames, returning them for transport.
    ///
    /// The pending queue is cleared after this call.
    #[inline]
    pub fn drain(&mut self) -> Vec<MaterializationFrame> {
        std::mem::take(&mut self.pending_frames)
    }

    /// Drains pending frames as encoded bytes (concatenated frames).
    pub fn drain_encoded(&mut self) -> Vec<u8> {
        let frames = self.drain();
        super::frame::encode_frames(&frames)
    }

    /// Clears all state: subscriptions, cache, and pending frames.
    pub fn clear(&mut self) {
        self.subscriptions.clear();
        self.replay_cache.clear();
        self.pending_frames.clear();
    }

    /// Gets the cached value for a channel without subscribing.
    #[inline]
    pub fn peek_cache(&self, channel: &ChannelId) -> Option<&Vec<u8>> {
        self.replay_cache.get(channel)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::materialization::channel::make_channel_id;

    fn finalized(label: &str, data: Vec<u8>) -> FinalizedChannel {
        FinalizedChannel {
            channel: make_channel_id(label),
            data,
        }
    }

    #[test]
    fn subscribe_returns_none_initially() {
        let mut port = MaterializationPort::new();
        let ch = make_channel_id("test:channel");
        assert!(port.subscribe(ch).is_none());
    }

    #[test]
    fn subscribe_returns_cached_value() {
        let mut port = MaterializationPort::new();
        let ch = make_channel_id("test:channel");

        // Receive some data first (simulating prior tick)
        port.receive_finalized(vec![FinalizedChannel {
            channel: ch,
            data: vec![1, 2, 3],
        }]);

        // Now subscribe - should get cached value
        let cached = port.subscribe(ch);
        assert_eq!(cached, Some(vec![1, 2, 3]));
    }

    #[test]
    fn receive_queues_for_subscribed() {
        let mut port = MaterializationPort::new();
        let ch1 = make_channel_id("channel:one");
        // ch2 not subscribed, so we don't need to track it
        let _ch2 = make_channel_id("channel:two");

        // Subscribe to ch1 only
        port.subscribe(ch1);

        // Receive data for both channels
        port.receive_finalized(vec![
            finalized("channel:one", vec![1]),
            finalized("channel:two", vec![2]),
        ]);

        // Only ch1 should be in pending
        assert_eq!(port.pending_count(), 1);

        let frames = port.drain();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].channel, ch1);
        assert_eq!(frames[0].data, vec![1]);
    }

    #[test]
    fn receive_updates_cache_for_all() {
        let mut port = MaterializationPort::new();
        let ch1 = make_channel_id("channel:one");
        let ch2 = make_channel_id("channel:two");

        // Don't subscribe to anything
        port.receive_finalized(vec![
            finalized("channel:one", vec![1]),
            finalized("channel:two", vec![2]),
        ]);

        // Cache should have both
        assert_eq!(port.peek_cache(&ch1), Some(&vec![1]));
        assert_eq!(port.peek_cache(&ch2), Some(&vec![2]));

        // But pending should be empty
        assert!(!port.has_pending());
    }

    #[test]
    fn unsubscribe_stops_queueing() {
        let mut port = MaterializationPort::new();
        let ch = make_channel_id("test:channel");

        port.subscribe(ch);
        port.receive_finalized(vec![finalized("test:channel", vec![1])]);
        assert_eq!(port.pending_count(), 1);
        port.drain(); // clear

        port.unsubscribe(ch);
        port.receive_finalized(vec![finalized("test:channel", vec![2])]);
        assert_eq!(port.pending_count(), 0);
    }

    #[test]
    fn drain_clears_pending() {
        let mut port = MaterializationPort::new();
        let ch = make_channel_id("test:channel");

        port.subscribe(ch);
        port.receive_finalized(vec![finalized("test:channel", vec![1])]);

        assert!(port.has_pending());
        let _ = port.drain();
        assert!(!port.has_pending());
    }

    #[test]
    fn drain_encoded_produces_valid_frames() {
        let mut port = MaterializationPort::new();
        let ch = make_channel_id("test:channel");

        port.subscribe(ch);
        port.receive_finalized(vec![finalized("test:channel", vec![1, 2, 3])]);

        let encoded = port.drain_encoded();

        // Should be decodable
        let frames = crate::materialization::frame::decode_frames(&encoded)
            .expect("decode drain_encoded output");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, vec![1, 2, 3]);
    }

    #[test]
    fn clear_removes_everything() {
        let mut port = MaterializationPort::new();
        let ch = make_channel_id("test:channel");

        port.subscribe(ch);
        port.receive_finalized(vec![finalized("test:channel", vec![1])]);

        assert!(port.is_subscribed(&ch));
        assert!(port.has_pending());
        assert!(port.peek_cache(&ch).is_some());

        port.clear();

        assert!(!port.is_subscribed(&ch));
        assert!(!port.has_pending());
        assert!(port.peek_cache(&ch).is_none());
    }
}
