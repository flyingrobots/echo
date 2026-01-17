// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Emission key for order-independent channel writes.
//!
//! The [`EmitKey`] ensures that channel emissions are deterministic regardless of
//! the order in which rewrite rules execute. This is critical for confluence-safe
//! parallel rewriting.
//!
//! # Ordering
//!
//! `EmitKey` ordering is lexicographic: `(scope_hash, rule_id, subkey)`.
//!
//! - `scope_hash`: Hash of the scope node where the rewrite was applied
//! - `rule_id`: Compact rule identifier (u32, unique per registered rule)
//! - `subkey`: Optional differentiator when a single rule emits multiple items
//!
//! This matches the scheduler's canonical ordering and ensures that finalization
//! order is deterministic regardless of execution timing.
//!
//! # Subkey Usage
//!
//! For most emissions, `subkey` is 0. When a rule needs to emit multiple items
//! to the same channel (e.g., iterating over children), use a stable subkey
//! derived from the item being emitted (e.g., hash of entity ID).

use crate::ident::Hash;

/// Key identifying a specific emission within a tick.
///
/// Ordering is lexicographic: `(scope_hash, rule_id, subkey)`.
/// All fields are computable from the executor context — no scheduler internals required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmitKey {
    /// Hash of the scope node where the rewrite was applied.
    pub scope_hash: Hash,
    /// Compact rule identifier (unique per registered rule).
    pub rule_id: u32,
    /// Differentiator for multiple emissions from the same rule invocation.
    /// Default is 0; use a stable hash when emitting multiple items.
    pub subkey: u32,
}

impl EmitKey {
    /// Creates a new emit key with subkey 0 (single emission per rule invocation).
    #[inline]
    pub const fn new(scope_hash: Hash, rule_id: u32) -> Self {
        Self {
            scope_hash,
            rule_id,
            subkey: 0,
        }
    }

    /// Creates a new emit key with a specific subkey (for multi-emission rules).
    #[inline]
    pub const fn with_subkey(scope_hash: Hash, rule_id: u32, subkey: u32) -> Self {
        Self {
            scope_hash,
            rule_id,
            subkey,
        }
    }

    /// Creates a subkey from a hash (for stable ordering of emitted items).
    ///
    /// Truncates the hash to u32 for compactness. Collisions are acceptable
    /// as long as ordering is deterministic.
    #[inline]
    pub fn subkey_from_hash(h: &Hash) -> u32 {
        u32::from_le_bytes([h[0], h[1], h[2], h[3]])
    }
}

impl PartialOrd for EmitKey {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EmitKey {
    /// Lexicographic ordering: `scope_hash`, then `rule_id`, then `subkey`.
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.scope_hash
            .cmp(&other.scope_hash)
            .then_with(|| self.rule_id.cmp(&other.rule_id))
            .then_with(|| self.subkey.cmp(&other.subkey))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn h(n: u8) -> Hash {
        let mut bytes = [0u8; 32];
        bytes[31] = n;
        bytes
    }

    #[test]
    fn ordering_scope_first() {
        let k1 = EmitKey::new(h(1), 5);
        let k2 = EmitKey::new(h(2), 1);
        assert!(k1 < k2, "lower scope_hash should come first");
    }

    #[test]
    fn ordering_rule_second() {
        let k1 = EmitKey::with_subkey(h(1), 1, 99);
        let k2 = EmitKey::with_subkey(h(1), 2, 0);
        assert!(k1 < k2, "same scope, lower rule_id should come first");
    }

    #[test]
    fn ordering_subkey_third() {
        let k1 = EmitKey::with_subkey(h(1), 1, 0);
        let k2 = EmitKey::with_subkey(h(1), 1, 1);
        assert!(
            k1 < k2,
            "same scope and rule, lower subkey should come first"
        );
    }

    #[test]
    fn equality() {
        let k1 = EmitKey::with_subkey(h(1), 2, 3);
        let k2 = EmitKey::with_subkey(h(1), 2, 3);
        assert_eq!(k1, k2);
    }

    #[test]
    fn default_subkey_is_zero() {
        let k = EmitKey::new(h(1), 2);
        assert_eq!(k.subkey, 0);
    }

    #[test]
    fn subkey_from_hash_deterministic() {
        let hash = h(42);
        let s1 = EmitKey::subkey_from_hash(&hash);
        let s2 = EmitKey::subkey_from_hash(&hash);
        assert_eq!(s1, s2);
    }

    #[test]
    fn btreemap_determinism() {
        use std::collections::BTreeMap;

        // Insert in arbitrary order, should iterate in canonical order
        let mut map = BTreeMap::new();
        map.insert(EmitKey::new(h(2), 1), "b");
        map.insert(EmitKey::new(h(1), 2), "a");
        map.insert(EmitKey::with_subkey(h(1), 1, 1), "c");
        map.insert(EmitKey::with_subkey(h(1), 1, 0), "d");

        let keys: Vec<_> = map.keys().collect();
        assert!(keys[0] < keys[1]);
        assert!(keys[1] < keys[2]);
        assert!(keys[2] < keys[3]);
    }
}
