// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Scoped adapter that auto-fills [`EmitKey`] from execution context.
//!
//! Created by the engine for each rule invocation. Captures the scope hash
//! and rule ID, preventing rules from forging keys.

use crate::ident::Hash;

use super::bus::{DuplicateEmission, MaterializationBus};
use super::channel::ChannelId;
use super::emission_port::EmissionPort;
use super::emit_key::EmitKey;

/// Scoped adapter that auto-fills [`EmitKey`] from execution context.
///
/// Created by the engine for each rule invocation. Captures the scope hash
/// and rule ID, preventing rules from forging keys.
///
/// # Usage
///
/// ```ignore
/// // In Engine::execute_rule() or similar
/// let emitter = ScopedEmitter::new(&self.bus, scope_node.hash(), rule.id());
/// rule.execute(context, &emitter)?;
/// ```
///
/// # Key Construction
///
/// The [`EmitKey`] is derived from:
/// - `scope_hash`: Content hash of the scope node (deterministic per scope)
/// - `rule_id`: Compact rule ID from the `RuleRegistry`
/// - `subkey`: 0 for `emit()`, caller-provided for `emit_with_subkey()`
///
/// This ensures that the same rule execution always produces the same [`EmitKey`],
/// regardless of scheduling order—critical for confluence.
pub struct ScopedEmitter<'a> {
    bus: &'a MaterializationBus,
    scope_hash: Hash,
    rule_id: u32,
}

impl<'a> ScopedEmitter<'a> {
    /// Create a new scoped emitter for a rule execution.
    ///
    /// # Arguments
    ///
    /// * `bus` - The materialization bus to emit to
    /// * `scope_hash` - Content hash of the scope node
    /// * `rule_id` - Compact rule ID from the `RuleRegistry`
    #[inline]
    pub fn new(bus: &'a MaterializationBus, scope_hash: Hash, rule_id: u32) -> Self {
        Self {
            bus,
            scope_hash,
            rule_id,
        }
    }

    /// Returns the scope hash this emitter is bound to.
    #[inline]
    pub fn scope_hash(&self) -> &Hash {
        &self.scope_hash
    }

    /// Returns the rule ID this emitter is bound to.
    #[inline]
    pub fn rule_id(&self) -> u32 {
        self.rule_id
    }
}

impl EmissionPort for ScopedEmitter<'_> {
    #[inline]
    fn emit(&self, channel: ChannelId, data: Vec<u8>) -> Result<(), DuplicateEmission> {
        let key = EmitKey::new(self.scope_hash, self.rule_id);
        self.bus.emit(channel, key, data)
    }

    #[inline]
    fn emit_with_subkey(
        &self,
        channel: ChannelId,
        subkey: u32,
        data: Vec<u8>,
    ) -> Result<(), DuplicateEmission> {
        let key = EmitKey::with_subkey(self.scope_hash, self.rule_id, subkey);
        self.bus.emit(channel, key, data)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::materialization::channel::make_channel_id;

    fn test_hash(tag: u8) -> Hash {
        let mut h = [0u8; 32];
        h[31] = tag;
        h
    }

    #[test]
    fn scoped_emitter_constructs_correct_key() {
        let bus = MaterializationBus::new();
        let ch = make_channel_id("test:scoped");

        let emitter = ScopedEmitter::new(&bus, test_hash(42), 7);
        emitter.emit(ch, vec![0xDE, 0xAD]).expect("emit");

        let report = bus.finalize();
        assert!(report.is_ok());
        assert_eq!(report.channels.len(), 1);
        assert_eq!(report.channels[0].channel, ch);
    }

    #[test]
    fn scoped_emitter_subkey_differentiates() {
        let bus = MaterializationBus::new();
        let ch = make_channel_id("test:subkey");

        let emitter = ScopedEmitter::new(&bus, test_hash(1), 1);

        // Same scope+rule, different subkeys should all succeed
        emitter.emit_with_subkey(ch, 0, vec![0]).expect("subkey 0");
        emitter.emit_with_subkey(ch, 1, vec![1]).expect("subkey 1");
        emitter.emit_with_subkey(ch, 2, vec![2]).expect("subkey 2");

        let report = bus.finalize();
        assert!(report.is_ok());
        assert_eq!(report.channels.len(), 1);

        // Count entries in log format
        let data = &report.channels[0].data;
        let mut count = 0;
        let mut offset = 0;
        while offset < data.len() {
            let len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4 + len;
            count += 1;
        }
        assert_eq!(count, 3, "all 3 subkey emissions preserved");
    }

    #[test]
    fn scoped_emitter_rejects_duplicate() {
        let bus = MaterializationBus::new();
        let ch = make_channel_id("test:dup");

        let emitter = ScopedEmitter::new(&bus, test_hash(1), 1);

        emitter.emit(ch, vec![1]).expect("first");
        let err = emitter.emit(ch, vec![2]).expect_err("duplicate");

        assert_eq!(err.channel, ch);
    }

    #[test]
    fn different_scopes_same_rule_are_independent() {
        let bus = MaterializationBus::new();
        let ch = make_channel_id("test:scope-ind");

        let emitter_a = ScopedEmitter::new(&bus, test_hash(1), 42);
        let emitter_b = ScopedEmitter::new(&bus, test_hash(2), 42);

        // Same rule_id but different scope_hash → different keys
        emitter_a.emit(ch, vec![0xAA]).expect("scope A");
        emitter_b.emit(ch, vec![0xBB]).expect("scope B");

        let report = bus.finalize();
        assert!(report.is_ok());
        assert_eq!(report.channels.len(), 1);
    }

    #[test]
    fn accessors_return_bound_values() {
        let bus = MaterializationBus::new();
        let hash = test_hash(99);
        let rule_id = 123;

        let emitter = ScopedEmitter::new(&bus, hash, rule_id);

        assert_eq!(emitter.scope_hash(), &hash);
        assert_eq!(emitter.rule_id(), rule_id);
    }
}
