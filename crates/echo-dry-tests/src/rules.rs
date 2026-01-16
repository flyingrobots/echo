// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic rule builders for tests.
//!
//! Provides pre-built rule components (matchers, executors, footprints)
//! and a builder for creating custom synthetic rules.

use crate::hashes::make_rule_id;
use warp_core::{
    ConflictPolicy, Footprint, GraphStore, Hash, NodeId, PatternGraph, RewriteRule,
};

// --- Matcher Functions ---

/// Matcher that always returns true.
pub fn always_match(_: &GraphStore, _: &NodeId) -> bool {
    true
}

/// Matcher that always returns false.
pub fn never_match(_: &GraphStore, _: &NodeId) -> bool {
    false
}

/// Matcher that returns true if the scope node exists.
pub fn scope_exists(store: &GraphStore, scope: &NodeId) -> bool {
    store.node(scope).is_some()
}

// --- Executor Functions ---

/// Executor that does nothing.
pub fn noop_exec(_: &mut GraphStore, _: &NodeId) {}

// --- Footprint Functions ---

/// Footprint that claims no reads or writes.
pub fn empty_footprint(_: &GraphStore, _: &NodeId) -> Footprint {
    Footprint::default()
}

/// Footprint that writes to the scope node.
pub fn write_scope_footprint(_: &GraphStore, scope: &NodeId) -> Footprint {
    let mut fp = Footprint::default();
    fp.n_write.insert_node(scope);
    fp.factor_mask = 1;
    fp
}

/// Footprint that reads from the scope node.
pub fn read_scope_footprint(_: &GraphStore, scope: &NodeId) -> Footprint {
    let mut fp = Footprint::default();
    fp.n_read.insert_node(scope);
    fp.factor_mask = 1;
    fp
}

/// Footprint that writes to scope and a derived "other" node.
pub fn write_scope_and_other_footprint(_: &GraphStore, scope: &NodeId) -> Footprint {
    let mut fp = Footprint::default();
    fp.n_write.insert_node(scope);
    fp.n_write.insert_node(&other_node_of(scope));
    fp.factor_mask = 1;
    fp
}

/// Derive an "other" node ID from a scope (useful for conflict tests).
pub fn other_node_of(scope: &NodeId) -> NodeId {
    NodeId(blake3::hash(&scope.0).into())
}

// --- Pre-built Rules ---

/// A no-op rule that always matches and does nothing.
///
/// Useful for testing rule registration and basic scheduling.
pub struct NoOpRule;

impl NoOpRule {
    /// Create a no-op rule with the given name.
    ///
    /// Note: The name must be a `&'static str` because `RewriteRule::name`
    /// requires a static lifetime.
    pub fn new(name: &'static str) -> RewriteRule {
        SyntheticRuleBuilder::new(name)
            .matcher(always_match)
            .executor(noop_exec)
            .footprint(empty_footprint)
            .build()
    }

    /// Create a no-op rule named "noop".
    pub fn default_rule() -> RewriteRule {
        Self::new("noop")
    }
}

/// Builder for creating synthetic rules in tests.
///
/// # Example
///
/// ```
/// use echo_dry_tests::{SyntheticRuleBuilder, rules::always_match, rules::noop_exec};
///
/// let rule = SyntheticRuleBuilder::new("test-rule")
///     .matcher(always_match)
///     .executor(noop_exec)
///     .build();
/// ```
pub struct SyntheticRuleBuilder {
    name: &'static str,
    id: Option<Hash>,
    matcher: fn(&GraphStore, &NodeId) -> bool,
    executor: fn(&mut GraphStore, &NodeId),
    footprint: fn(&GraphStore, &NodeId) -> Footprint,
    factor_mask: u64,
    conflict_policy: ConflictPolicy,
}

impl SyntheticRuleBuilder {
    /// Create a new builder with the given rule name.
    ///
    /// Note: The name must be a `&'static str` because `RewriteRule::name`
    /// requires a static lifetime.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            id: None,
            matcher: always_match,
            executor: noop_exec,
            footprint: empty_footprint,
            factor_mask: 0,
            conflict_policy: ConflictPolicy::Abort,
        }
    }

    /// Set a custom rule ID (default is derived from name).
    pub fn id(mut self, id: Hash) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the matcher function.
    pub fn matcher(mut self, f: fn(&GraphStore, &NodeId) -> bool) -> Self {
        self.matcher = f;
        self
    }

    /// Set the executor function.
    pub fn executor(mut self, f: fn(&mut GraphStore, &NodeId)) -> Self {
        self.executor = f;
        self
    }

    /// Set the footprint function.
    pub fn footprint(mut self, f: fn(&GraphStore, &NodeId) -> Footprint) -> Self {
        self.footprint = f;
        self
    }

    /// Set the factor mask.
    pub fn factor_mask(mut self, mask: u64) -> Self {
        self.factor_mask = mask;
        self
    }

    /// Set the conflict policy.
    pub fn conflict_policy(mut self, policy: ConflictPolicy) -> Self {
        self.conflict_policy = policy;
        self
    }

    /// Use the "always match" matcher.
    pub fn always_matches(self) -> Self {
        self.matcher(always_match)
    }

    /// Use the "never match" matcher.
    pub fn never_matches(self) -> Self {
        self.matcher(never_match)
    }

    /// Use the "scope exists" matcher.
    pub fn matches_if_scope_exists(self) -> Self {
        self.matcher(scope_exists)
    }

    /// Use the "write scope" footprint.
    pub fn writes_scope(self) -> Self {
        self.footprint(write_scope_footprint)
    }

    /// Use the "read scope" footprint.
    pub fn reads_scope(self) -> Self {
        self.footprint(read_scope_footprint)
    }

    /// Build the rule.
    pub fn build(self) -> RewriteRule {
        RewriteRule {
            id: self.id.unwrap_or_else(|| make_rule_id(&self.name)),
            name: self.name,
            left: PatternGraph { nodes: vec![] },
            matcher: self.matcher,
            executor: self.executor,
            compute_footprint: self.footprint,
            factor_mask: self.factor_mask,
            conflict_policy: self.conflict_policy,
            join_fn: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_rule_creation() {
        let rule = NoOpRule::new("test-noop");
        assert_eq!(rule.name, "test-noop");
    }

    #[test]
    fn synthetic_builder_defaults() {
        let rule = SyntheticRuleBuilder::new("my-rule").build();
        assert_eq!(rule.name, "my-rule");
        assert_eq!(rule.factor_mask, 0);
    }

    #[test]
    fn synthetic_builder_custom_id() {
        let custom_id: Hash = [42u8; 32];
        let rule = SyntheticRuleBuilder::new("custom")
            .id(custom_id)
            .build();
        assert_eq!(rule.id, custom_id);
    }

    #[test]
    fn synthetic_builder_fluent_api() {
        let rule = SyntheticRuleBuilder::new("fluent")
            .always_matches()
            .writes_scope()
            .factor_mask(7)
            .conflict_policy(ConflictPolicy::Abort)
            .build();

        assert_eq!(rule.name, "fluent");
        assert_eq!(rule.factor_mask, 7);
    }

    #[test]
    fn other_node_is_deterministic() {
        let scope = NodeId([1u8; 32]);
        let other1 = other_node_of(&scope);
        let other2 = other_node_of(&scope);
        assert_eq!(other1, other2);
        assert_ne!(scope, other1);
    }
}
