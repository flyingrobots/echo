// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Internal materialization bus for tick-scoped channel emissions.
//!
//! The [`MaterializationBus`] collects emissions from rewrite rules during a tick.
//! Emissions are stored in an order-independent manner (keyed by [`EmitKey`]),
//! then finalized post-commit according to each channel's policy.
//!
//! # Order Independence
//!
//! The bus uses `BTreeMap<ChannelId, BTreeMap<EmitKey, Vec<u8>>>` internally.
//! This ensures that:
//! 1. Insertion order doesn't affect the final result
//! 2. Finalization iterates in deterministic (canonical) order
//! 3. Confluence is preserved regardless of rewrite execution order
//!
//! # Usage
//!
//! Rules emit via `bus.emit(channel, emit_key, data)`. After commit, the engine
//! calls `bus.finalize()` to resolve each channel according to its policy and
//! produce the final output.

use std::cell::RefCell;
use std::collections::BTreeMap;

use super::channel::{ChannelConflict, ChannelId, ChannelPolicy};
use super::emit_key::EmitKey;

/// Internal materialization bus for collecting and finalizing channel emissions.
///
/// This is tick-scoped: emissions accumulate during a tick, then `finalize()` is
/// called post-commit to produce output. The bus is cleared after finalization.
#[derive(Debug, Default)]
pub struct MaterializationBus {
    /// Pending emissions: `channel -> (emit_key -> data)`.
    /// Uses `BTreeMap` for deterministic iteration order.
    pending: RefCell<BTreeMap<ChannelId, BTreeMap<EmitKey, Vec<u8>>>>,

    /// Channel policies (looked up during finalization).
    policies: BTreeMap<ChannelId, ChannelPolicy>,
}

/// Result of finalizing a single channel.
#[derive(Debug, Clone)]
pub struct FinalizedChannel {
    /// The channel that was finalized.
    pub channel: ChannelId,
    /// The finalized data (format depends on policy).
    pub data: Vec<u8>,
}

/// Result of finalizing all channels.
pub type FinalizeResult = Result<Vec<FinalizedChannel>, ChannelConflict>;

impl MaterializationBus {
    /// Creates a new empty bus.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a channel with a specific policy.
    ///
    /// If not registered, channels default to [`ChannelPolicy::Log`].
    #[inline]
    pub fn register_channel(&mut self, channel: ChannelId, policy: ChannelPolicy) {
        self.policies.insert(channel, policy);
    }

    /// Emits data to a channel with the given emit key.
    ///
    /// Multiple emissions to the same channel are collected and resolved
    /// during finalization according to the channel's policy.
    #[inline]
    pub fn emit(&self, channel: ChannelId, emit_key: EmitKey, data: Vec<u8>) {
        self.pending
            .borrow_mut()
            .entry(channel)
            .or_default()
            .insert(emit_key, data);
    }

    /// Returns the policy for a channel (defaults to Log if not registered).
    #[inline]
    pub fn policy(&self, channel: &ChannelId) -> ChannelPolicy {
        self.policies.get(channel).copied().unwrap_or_default()
    }

    /// Returns true if there are no pending emissions.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pending.borrow().is_empty()
    }

    /// Finalizes all pending emissions according to channel policies.
    ///
    /// For each channel:
    /// - `Log`: All values concatenated in `EmitKey` order
    /// - `StrictSingle`: Error if >1 emission, otherwise single value
    /// - `Reduce`: Values merged via join function (not yet implemented)
    ///
    /// Clears pending emissions after finalization.
    ///
    /// # Errors
    ///
    /// Returns [`ChannelConflict`] if a `StrictSingle` channel has multiple emissions.
    pub fn finalize(&self) -> FinalizeResult {
        let mut pending = self.pending.borrow_mut();
        let mut results = Vec::with_capacity(pending.len());

        for (channel, emissions) in &*pending {
            let policy = self.policies.get(channel).copied().unwrap_or_default();
            let data = Self::finalize_channel(channel, emissions, policy)?;
            results.push(FinalizedChannel {
                channel: *channel,
                data,
            });
        }

        pending.clear();
        Ok(results)
    }

    /// Clears all pending emissions without finalizing (for abort path).
    #[inline]
    pub fn clear(&self) {
        self.pending.borrow_mut().clear();
    }

    /// Finalizes a single channel according to its policy.
    #[allow(clippy::cast_possible_truncation)]
    fn finalize_channel(
        channel: &ChannelId,
        emissions: &BTreeMap<EmitKey, Vec<u8>>,
        policy: ChannelPolicy,
    ) -> Result<Vec<u8>, ChannelConflict> {
        match policy {
            ChannelPolicy::Log => {
                // Concatenate all values in EmitKey order.
                // Format: length-prefixed entries for parseability.
                let mut result = Vec::new();
                for data in emissions.values() {
                    // Prefix each entry with its length as u32 LE
                    // Truncation is acceptable: entries >4GB are not supported
                    result.extend_from_slice(&(data.len() as u32).to_le_bytes());
                    result.extend_from_slice(data);
                }
                Ok(result)
            }
            ChannelPolicy::StrictSingle => {
                if emissions.len() > 1 {
                    return Err(ChannelConflict {
                        channel: *channel,
                        emission_count: emissions.len(),
                    });
                }
                // Return the single value (or empty if none)
                Ok(emissions.values().next().cloned().unwrap_or_default())
            }
            ChannelPolicy::Reduce { join_fn_id: _ } => {
                // TODO: Implement reduce via join function registry
                // For now, fall back to Log behavior
                let mut result = Vec::new();
                for data in emissions.values() {
                    result.extend_from_slice(&(data.len() as u32).to_le_bytes());
                    result.extend_from_slice(data);
                }
                Ok(result)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::ident::Hash;
    use crate::materialization::channel::make_channel_id;

    fn h(n: u8) -> Hash {
        let mut bytes = [0u8; 32];
        bytes[31] = n;
        bytes
    }

    fn key(scope: u8, rule: u32) -> EmitKey {
        EmitKey::new(h(scope), rule)
    }

    #[test]
    fn emit_and_finalize_log() {
        let bus = MaterializationBus::new();
        let ch = make_channel_id("test:log");

        // Emit in arbitrary order
        bus.emit(ch, key(2, 1), vec![2]);
        bus.emit(ch, key(1, 1), vec![1]);
        bus.emit(ch, key(1, 2), vec![3]);

        let result = bus.finalize().expect("finalize should succeed");
        assert_eq!(result.len(), 1);

        // Should be ordered by EmitKey: (h(1), 1), (h(1), 2), (h(2), 1)
        // Each entry is length-prefixed
        let data = &result[0].data;
        assert_eq!(data[0..4], 1u32.to_le_bytes()); // len = 1
        assert_eq!(data[4], 1); // value
        assert_eq!(data[5..9], 1u32.to_le_bytes()); // len = 1
        assert_eq!(data[9], 3); // value
        assert_eq!(data[10..14], 1u32.to_le_bytes()); // len = 1
        assert_eq!(data[14], 2); // value
    }

    #[test]
    fn emit_order_independence() {
        // Two buses with same emissions in different order should produce same result
        let bus1 = MaterializationBus::new();
        let bus2 = MaterializationBus::new();
        let ch = make_channel_id("test:order");

        // Bus 1: emit in one order
        bus1.emit(ch, key(1, 1), vec![1]);
        bus1.emit(ch, key(2, 1), vec![2]);
        bus1.emit(ch, key(1, 2), vec![3]);

        // Bus 2: emit in different order
        bus2.emit(ch, key(2, 1), vec![2]);
        bus2.emit(ch, key(1, 2), vec![3]);
        bus2.emit(ch, key(1, 1), vec![1]);

        let result1 = bus1.finalize().expect("finalize bus1");
        let result2 = bus2.finalize().expect("finalize bus2");

        assert_eq!(result1[0].data, result2[0].data, "order should not matter");
    }

    #[test]
    fn strict_single_allows_one() {
        let mut bus = MaterializationBus::new();
        let ch = make_channel_id("test:strict");
        bus.register_channel(ch, ChannelPolicy::StrictSingle);

        bus.emit(ch, key(1, 1), vec![42]);

        let result = bus
            .finalize()
            .expect("finalize should succeed for single emission");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].data, vec![42]);
    }

    #[test]
    fn strict_single_rejects_multiple() {
        let mut bus = MaterializationBus::new();
        let ch = make_channel_id("test:strict");
        bus.register_channel(ch, ChannelPolicy::StrictSingle);

        bus.emit(ch, key(1, 1), vec![1]);
        bus.emit(ch, key(2, 1), vec![2]);

        let result = bus.finalize();
        let err = result.expect_err("finalize should fail with multiple emissions");
        assert_eq!(err.emission_count, 2);
    }

    #[test]
    fn clear_removes_pending() {
        let bus = MaterializationBus::new();
        let ch = make_channel_id("test:clear");

        bus.emit(ch, key(1, 1), vec![1]);
        assert!(!bus.is_empty());

        bus.clear();
        assert!(bus.is_empty());

        let result = bus.finalize().expect("finalize after clear");
        assert!(result.is_empty());
    }

    #[test]
    fn multiple_channels() {
        let bus = MaterializationBus::new();
        let ch1 = make_channel_id("channel:one");
        let ch2 = make_channel_id("channel:two");

        bus.emit(ch1, key(1, 1), vec![1]);
        bus.emit(ch2, key(1, 1), vec![2]);

        let result = bus.finalize().expect("finalize multi-channel");
        assert_eq!(result.len(), 2);

        // Channels should be in deterministic order (by ChannelId)
        let ids: Vec<_> = result.iter().map(|r| r.channel).collect();
        assert!(ids[0] != ids[1], "channels should have different IDs");
    }

    #[test]
    fn default_policy_is_log() {
        let bus = MaterializationBus::new();
        let ch = make_channel_id("test:unregistered");
        assert_eq!(bus.policy(&ch), ChannelPolicy::Log);
    }
}
