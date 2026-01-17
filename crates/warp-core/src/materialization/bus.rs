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
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use super::channel::{ChannelConflict, ChannelId, ChannelPolicy, MaterializationErrorKind};
use super::emit_key::EmitKey;

/// Error returned when the same `(channel, EmitKey)` pair is emitted twice.
///
/// This is a structural invariant: even if the payloads are identical, duplicate
/// emissions indicate a bug in the rule (e.g., iterating a non-deterministic source
/// like an unordered container without proper subkey differentiation).
///
/// # Why Reject Identical Payloads?
///
/// Allowing "identical payload = OK" encourages sloppy code that emits redundantly.
/// Then someone changes a field and tests fail mysteriously. Rejecting always forces
/// rule authors to think: "Am I iterating deterministically? Do I need unique subkeys?"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateEmission {
    /// The channel that received the duplicate.
    pub channel: ChannelId,
    /// The key that was duplicated.
    pub key: EmitKey,
}

impl core::fmt::Display for DuplicateEmission {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "duplicate emission for channel {:?} key {:?}",
            self.channel, self.key
        )
    }
}

impl std::error::Error for DuplicateEmission {}

/// Internal materialization bus for collecting and finalizing channel emissions.
///
/// This is tick-scoped: emissions accumulate during a tick, then `finalize()` is
/// called post-commit to produce output. The bus is cleared after finalization.
///
/// # Thread Safety
///
/// `MaterializationBus` uses [`RefCell`] for interior mutability and is intentionally
/// **not thread-safe** (`!Sync`). This is by design: emissions occur within a single
/// tick's execution context, which is single-threaded. The rewrite engine processes
/// rules sequentially within a tick, so no synchronization is needed.
///
/// If parallel emission were ever required (e.g., concurrent rule execution), the
/// design would need to change to use `RwLock<BTreeMap<...>>` or similar.
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

/// Report from finalizing all channels.
///
/// Unlike a `Result`, this type always succeeds. Channels that finalize
/// successfully appear in `channels`; channels that fail (e.g., `StrictSingle`
/// conflicts) appear in `errors`. This design ensures:
///
/// 1. **No data loss**: A failing channel doesn't erase other channels' outputs
/// 2. **Deterministic errors**: Same emissions → same errors, always observable
/// 3. **No panics**: Callers handle errors explicitly, not via `expect()`
///
/// # Invariant
///
/// `channels` and `errors` partition the set of channels that had emissions.
/// A channel appears in exactly one of the two lists.
#[derive(Debug, Default, Clone)]
#[must_use = "materialization errors must be checked"]
pub struct FinalizeReport {
    /// Successfully finalized channels.
    pub channels: Vec<FinalizedChannel>,
    /// Channels that failed to finalize.
    pub errors: Vec<ChannelConflict>,
}

impl FinalizeReport {
    /// Returns `true` if all channels finalized successfully.
    #[inline]
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns `true` if any channel failed to finalize.
    #[inline]
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Panics if any channel failed to finalize.
    ///
    /// Use this in tests or contexts where materialization errors are fatal
    /// and should never be silently ignored.
    ///
    /// # Panics
    ///
    /// Panics with a message listing which channels had conflicts and how many
    /// errors occurred.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let report = bus.finalize();
    /// report.assert_clean(); // Panics if any errors
    /// ```
    #[inline]
    #[allow(clippy::panic)] // Intentional: this is a test/CI helper
    pub fn assert_clean(&self) -> &Self {
        if self.has_errors() {
            let details: Vec<String> = self
                .errors
                .iter()
                .map(|c| format!("channel {:?}: {:?}", c.channel, c.kind))
                .collect();
            panic!(
                "materialization errors must be checked: {} error(s) - [{}]",
                self.errors.len(),
                details.join(", ")
            );
        }
        self
    }
}

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
    ///
    /// # Size Limit
    ///
    /// Individual emitted entries larger than 4 GiB (2^32 bytes) are **not supported**.
    /// During finalization, the `Log` policy uses u32 length-prefixing, so entries
    /// exceeding `u32::MAX` bytes will have their length silently truncated. Callers
    /// must validate payload sizes or split large payloads before calling `emit()`.
    ///
    /// # Errors
    ///
    /// Returns [`DuplicateEmission`] if this `(channel, emit_key)` pair has
    /// already been emitted during this tick. This is always an error, even
    /// if the payload bytes are identical—it indicates a bug in the rule
    /// (e.g., iterating an unordered container without proper subkey differentiation).
    #[inline]
    pub fn emit(
        &self,
        channel: ChannelId,
        emit_key: EmitKey,
        data: Vec<u8>,
    ) -> Result<(), DuplicateEmission> {
        let mut pending = self.pending.borrow_mut();
        let channel_map = pending.entry(channel).or_default();

        match channel_map.entry(emit_key) {
            Entry::Vacant(e) => {
                e.insert(data);
                Ok(())
            }
            Entry::Occupied(_) => Err(DuplicateEmission {
                channel,
                key: emit_key,
            }),
        }
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
    /// - `Log`: All values concatenated in `EmitKey` order, length-prefixed
    /// - `StrictSingle`: At most one emission allowed; errors if more than one
    /// - `Reduce(op)`: Values merged via the reduce operation
    ///
    /// Clears pending emissions after finalization.
    ///
    /// # Design
    ///
    /// This method **never fails**. Instead, it returns a [`FinalizeReport`] that
    /// partitions channels into successes and errors. This ensures:
    ///
    /// - A failing channel (e.g., `StrictSingle` conflict) doesn't erase other
    ///   channels' outputs
    /// - Errors are always observable in the returned report
    /// - Callers don't need `expect()` or `unwrap()`
    #[must_use = "materialization errors must be checked"]
    pub fn finalize(&self) -> FinalizeReport {
        let mut pending = self.pending.borrow_mut();
        let mut report = FinalizeReport::default();

        // Channels iterate in deterministic order (BTreeMap).
        for (&channel, emissions) in &*pending {
            let policy = self.policies.get(&channel).copied().unwrap_or_default();
            match Self::finalize_channel(channel, emissions, policy) {
                Ok(data) => report.channels.push(FinalizedChannel { channel, data }),
                Err(conflict) => report.errors.push(conflict),
            }
        }

        pending.clear();
        report
    }

    /// Clears all pending emissions without finalizing (for abort path).
    #[inline]
    pub fn clear(&self) {
        self.pending.borrow_mut().clear();
    }

    /// Finalizes a single channel according to its policy.
    #[allow(clippy::cast_possible_truncation)]
    fn finalize_channel(
        channel: ChannelId,
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
                let n = emissions.len();
                if n > 1 {
                    return Err(ChannelConflict {
                        channel,
                        emission_count: n,
                        kind: MaterializationErrorKind::StrictSingleConflict,
                    });
                }
                // Return the single value (or empty if none)
                Ok(emissions.values().next().cloned().unwrap_or_default())
            }
            ChannelPolicy::Reduce(op) => {
                // Apply the reduce operation to values in EmitKey order
                Ok(op.apply(emissions.values().cloned()))
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
        bus.emit(ch, key(2, 1), vec![2]).expect("emit");
        bus.emit(ch, key(1, 1), vec![1]).expect("emit");
        bus.emit(ch, key(1, 2), vec![3]).expect("emit");

        let report = bus.finalize();
        assert!(report.is_ok(), "finalize should succeed");
        assert_eq!(report.channels.len(), 1);

        // Should be ordered by EmitKey: (h(1), 1), (h(1), 2), (h(2), 1)
        // Each entry is length-prefixed
        let data = &report.channels[0].data;
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
        bus1.emit(ch, key(1, 1), vec![1]).expect("emit");
        bus1.emit(ch, key(2, 1), vec![2]).expect("emit");
        bus1.emit(ch, key(1, 2), vec![3]).expect("emit");

        // Bus 2: emit in different order
        bus2.emit(ch, key(2, 1), vec![2]).expect("emit");
        bus2.emit(ch, key(1, 2), vec![3]).expect("emit");
        bus2.emit(ch, key(1, 1), vec![1]).expect("emit");

        let report1 = bus1.finalize();
        let report2 = bus2.finalize();

        assert!(report1.is_ok() && report2.is_ok());
        assert_eq!(
            report1.channels[0].data, report2.channels[0].data,
            "order should not matter"
        );
    }

    #[test]
    fn strict_single_allows_one() {
        let mut bus = MaterializationBus::new();
        let ch = make_channel_id("test:strict");
        bus.register_channel(ch, ChannelPolicy::StrictSingle);

        bus.emit(ch, key(1, 1), vec![42]).expect("emit");

        let report = bus.finalize();
        assert!(
            report.is_ok(),
            "finalize should succeed for single emission"
        );
        assert_eq!(report.channels.len(), 1);
        assert_eq!(report.channels[0].data, vec![42]);
    }

    #[test]
    fn strict_single_rejects_multiple() {
        let mut bus = MaterializationBus::new();
        let ch = make_channel_id("test:strict");
        bus.register_channel(ch, ChannelPolicy::StrictSingle);

        bus.emit(ch, key(1, 1), vec![1]).expect("emit");
        bus.emit(ch, key(2, 1), vec![2]).expect("emit");

        let report = bus.finalize();
        assert!(report.has_errors(), "finalize should have errors");
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].emission_count, 2);
        assert_eq!(
            report.errors[0].kind,
            MaterializationErrorKind::StrictSingleConflict
        );
    }

    #[test]
    fn strict_single_zero_emissions_succeeds() {
        // StrictSingle means "at most one emission", not "exactly one".
        // Zero emissions is valid and returns empty data.
        let mut bus = MaterializationBus::new();
        let ch = make_channel_id("test:strict-zero");
        bus.register_channel(ch, ChannelPolicy::StrictSingle);

        // Perform NO emit() calls for this channel

        // Create an emission on a different channel so finalize() has something to process
        // and we can verify the StrictSingle channel doesn't appear in errors
        let other_ch = make_channel_id("test:other");
        bus.emit(other_ch, key(1, 1), vec![99]).expect("emit");

        let report = bus.finalize();
        assert!(
            report.is_ok(),
            "finalize should succeed with zero emissions on StrictSingle"
        );
        // The StrictSingle channel with zero emissions won't appear in results
        // (no emissions = no entry in pending map)
        assert_eq!(report.channels.len(), 1);
        assert_eq!(report.channels[0].channel, other_ch);
    }

    #[test]
    fn clear_removes_pending() {
        let bus = MaterializationBus::new();
        let ch = make_channel_id("test:clear");

        bus.emit(ch, key(1, 1), vec![1]).expect("emit");
        assert!(!bus.is_empty());

        bus.clear();
        assert!(bus.is_empty());

        let report = bus.finalize();
        assert!(report.is_ok());
        assert!(report.channels.is_empty());
    }

    #[test]
    fn multiple_channels() {
        let bus = MaterializationBus::new();
        let ch1 = make_channel_id("channel:one");
        let ch2 = make_channel_id("channel:two");

        bus.emit(ch1, key(1, 1), vec![1]).expect("emit");
        bus.emit(ch2, key(1, 1), vec![2]).expect("emit");

        let report = bus.finalize();
        assert!(report.is_ok());
        assert_eq!(report.channels.len(), 2);

        // Channels should be in deterministic order (by ChannelId)
        let ids: Vec<_> = report.channels.iter().map(|r| r.channel).collect();
        assert!(ids[0] != ids[1], "channels should have different IDs");
    }

    #[test]
    fn default_policy_is_log() {
        let bus = MaterializationBus::new();
        let ch = make_channel_id("test:unregistered");
        assert_eq!(bus.policy(&ch), ChannelPolicy::Log);
    }
}
