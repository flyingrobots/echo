// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Hash ID generation helpers for tests.
//!
//! These functions centralize the common pattern of creating deterministic
//! hash-based IDs for rules, intents, and other test artifacts.

use warp_core::Hash;

/// Generate a rule ID from a name.
///
/// This uses the same pattern as production code: `blake3("rule:" + name)`.
///
/// # Example
///
/// ```
/// use echo_dry_tests::make_rule_id;
///
/// let id = make_rule_id("my-rule");
/// assert_eq!(id.len(), 32);
/// ```
pub fn make_rule_id(name: &str) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:");
    hasher.update(name.as_bytes());
    hasher.finalize().into()
}

/// Generate an intent ID from raw bytes.
///
/// # Example
///
/// ```
/// use echo_dry_tests::make_intent_id;
///
/// let id = make_intent_id(b"my-intent");
/// assert_eq!(id.len(), 32);
/// ```
pub fn make_intent_id(bytes: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"intent:");
    hasher.update(bytes);
    hasher.finalize().into()
}

/// Generate a generic test hash from a label.
///
/// Useful for creating deterministic hashes in tests without coupling
/// to specific domain semantics.
pub fn make_test_hash(label: &str) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"test:");
    hasher.update(label.as_bytes());
    hasher.finalize().into()
}

/// Generate a hash from a numeric seed (useful for loops).
pub fn make_hash_from_seed(seed: u64) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"seed:");
    hasher.update(&seed.to_le_bytes());
    hasher.finalize().into()
}

/// Compute a plan digest from receipt entries (matching warp-core semantics).
///
/// This is useful for verifying tick receipt digests in tests.
pub fn compute_plan_digest(entries: &[(Hash, Hash)]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(entries.len() as u64).to_le_bytes());
    for (scope_hash, rule_id) in entries {
        hasher.update(scope_hash);
        hasher.update(rule_id);
    }
    hasher.finalize().into()
}

/// Pre-defined test rule IDs for common scenarios.
pub mod presets {
    use super::*;

    /// Rule ID for "rule-a" (useful in multi-rule tests).
    pub fn rule_a() -> Hash {
        make_rule_id("rule-a")
    }

    /// Rule ID for "rule-b" (useful in multi-rule tests).
    pub fn rule_b() -> Hash {
        make_rule_id("rule-b")
    }

    /// Rule ID for "rule-c" (useful in multi-rule tests).
    pub fn rule_c() -> Hash {
        make_rule_id("rule-c")
    }

    /// Rule ID for motion rule.
    pub fn motion_rule() -> Hash {
        make_rule_id(crate::demo_rules::MOTION_RULE_NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_id_is_deterministic() {
        let id1 = make_rule_id("test");
        let id2 = make_rule_id("test");
        assert_eq!(id1, id2);
    }

    #[test]
    fn different_names_produce_different_ids() {
        let id1 = make_rule_id("rule-a");
        let id2 = make_rule_id("rule-b");
        assert_ne!(id1, id2);
    }

    #[test]
    fn intent_id_is_deterministic() {
        let id1 = make_intent_id(b"test");
        let id2 = make_intent_id(b"test");
        assert_eq!(id1, id2);
    }

    #[test]
    fn presets_are_stable() {
        // Just verify they don't panic and return 32-byte hashes
        assert_eq!(presets::rule_a().len(), 32);
        assert_eq!(presets::rule_b().len(), 32);
        assert_eq!(presets::rule_c().len(), 32);
    }
}
