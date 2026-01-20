// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic rule builders for tests.
//!
//! Provides pre-built rule components (matchers, executors, footprints)
//! and a builder for creating custom synthetic rules.

use crate::hashes::make_rule_id;
#[cfg(test)]
use warp_core::GraphStore;
use warp_core::{
    ConflictPolicy, Footprint, GraphView, Hash, NodeId, PatternGraph, RewriteRule, TickDelta,
};

/// Type alias for join functions matching warp-core's `JoinFn`.
pub type JoinFn = fn(&NodeId, &NodeId) -> bool;

// --- Matcher Functions ---

/// Matcher that always returns true.
pub fn always_match(_: GraphView<'_>, _: &NodeId) -> bool {
    true
}

/// Matcher that always returns false.
pub fn never_match(_: GraphView<'_>, _: &NodeId) -> bool {
    false
}

/// Matcher that returns true if the scope node exists.
pub fn scope_exists(view: GraphView<'_>, scope: &NodeId) -> bool {
    view.node(scope).is_some()
}

// --- Executor Functions ---

/// Executor that does nothing.
pub fn noop_exec(_: GraphView<'_>, _: &NodeId, _: &mut TickDelta) {}

// --- Footprint Functions ---

/// Footprint that claims no reads or writes.
pub fn empty_footprint(_: GraphView<'_>, _: &NodeId) -> Footprint {
    Footprint::default()
}

/// Footprint that writes to the scope node.
pub fn write_scope_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut fp = Footprint::default();
    fp.n_write.insert_with_warp(warp_id, *scope);
    fp.factor_mask = 1;
    fp
}

/// Footprint that reads from the scope node.
pub fn read_scope_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut fp = Footprint::default();
    fp.n_read.insert_with_warp(warp_id, *scope);
    fp.factor_mask = 1;
    fp
}

/// Footprint that writes to scope and a derived "other" node.
pub fn write_scope_and_other_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut fp = Footprint::default();
    fp.n_write.insert_with_warp(warp_id, *scope);
    fp.n_write.insert_with_warp(warp_id, other_node_of(scope));
    fp.factor_mask = 1;
    fp
}

/// Derive an "other" node ID from a scope (useful for conflict tests).
///
/// Uses domain-separated hashing (prefixed with `b"other-node:"`) for
/// consistency with other hash generation functions in this crate.
pub fn other_node_of(scope: &NodeId) -> NodeId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"other-node:");
    hasher.update(&scope.0);
    NodeId(hasher.finalize().into())
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
    pub fn create(name: &'static str) -> RewriteRule {
        SyntheticRuleBuilder::new(name)
            .matcher(always_match)
            .executor(noop_exec)
            .footprint(empty_footprint)
            .build()
    }

    /// Create a no-op rule named "noop".
    pub fn default_rule() -> RewriteRule {
        Self::create("noop")
    }
}

/// Type alias for Phase 5 BOAW matcher functions.
pub type MatcherFn = for<'a> fn(GraphView<'a>, &NodeId) -> bool;

/// Type alias for Phase 5 BOAW executor functions.
pub type ExecutorFn = for<'a> fn(GraphView<'a>, &NodeId, &mut TickDelta);

/// Type alias for Phase 5 BOAW footprint functions.
pub type FootprintFn = for<'a> fn(GraphView<'a>, &NodeId) -> Footprint;

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
    matcher: MatcherFn,
    executor: ExecutorFn,
    footprint: FootprintFn,
    factor_mask: u64,
    conflict_policy: ConflictPolicy,
    join_fn: Option<JoinFn>,
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
            join_fn: None,
        }
    }

    /// Set a custom rule ID (default is derived from name).
    pub fn id(mut self, id: Hash) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the matcher function.
    pub fn matcher(mut self, f: MatcherFn) -> Self {
        self.matcher = f;
        self
    }

    /// Set the executor function.
    pub fn executor(mut self, f: ExecutorFn) -> Self {
        self.executor = f;
        self
    }

    /// Set the footprint function.
    pub fn footprint(mut self, f: FootprintFn) -> Self {
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

    /// Set the join function (required when `conflict_policy` is `ConflictPolicy::Join`).
    pub fn join_fn(mut self, f: JoinFn) -> Self {
        self.join_fn = Some(f);
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
            id: self.id.unwrap_or_else(|| make_rule_id(self.name)),
            name: self.name,
            left: PatternGraph { nodes: vec![] },
            matcher: self.matcher,
            executor: self.executor,
            compute_footprint: self.footprint,
            factor_mask: self.factor_mask,
            conflict_policy: self.conflict_policy,
            join_fn: self.join_fn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_rule_creation() {
        let rule = NoOpRule::create("test-noop");
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
        let rule = SyntheticRuleBuilder::new("custom").id(custom_id).build();
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

    // --- Behavioral Tests ---

    /// Helper to check if a NodeSet contains a specific node for the default (root) warp.
    fn node_set_contains(set: &warp_core::NodeSet, node: &NodeId) -> bool {
        // For tests with GraphStore::default(), the warp_id is make_warp_id("root")
        let root_warp = warp_core::make_warp_id("root");
        set.iter()
            .any(|key| key.warp_id == root_warp && key.local_id == *node)
    }

    #[test]
    fn matcher_scope_exists_returns_true_when_node_present() {
        use warp_core::NodeRecord;

        let mut store = GraphStore::default();
        let scope = other_node_of(&NodeId([0xAAu8; 32]));
        let ty = warp_core::make_type_id("test-type");

        // Node not yet present: matcher should return false
        let view = GraphView::new(&store);
        assert!(!scope_exists(view, &scope));

        // Insert node into store
        store.insert_node(scope, NodeRecord { ty });

        // Node now present: matcher should return true
        let view = GraphView::new(&store);
        assert!(scope_exists(view, &scope));
    }

    #[test]
    fn matcher_always_and_never_match_behavior() {
        let store = GraphStore::default();
        let view = GraphView::new(&store);
        let scope = NodeId([0xBBu8; 32]);

        assert!(always_match(view, &scope));
        assert!(!never_match(view, &scope));
    }

    #[test]
    fn footprint_write_scope_produces_expected_footprint() {
        let store = GraphStore::default();
        let view = GraphView::new(&store);
        let scope = NodeId([0xCCu8; 32]);

        let fp = write_scope_footprint(view, &scope);

        assert!(node_set_contains(&fp.n_write, &scope));
        assert!(!node_set_contains(&fp.n_read, &scope));
        assert_eq!(fp.factor_mask, 1);
    }

    #[test]
    fn footprint_read_scope_produces_expected_footprint() {
        let store = GraphStore::default();
        let view = GraphView::new(&store);
        let scope = NodeId([0xDDu8; 32]);

        let fp = read_scope_footprint(view, &scope);

        assert!(node_set_contains(&fp.n_read, &scope));
        assert!(!node_set_contains(&fp.n_write, &scope));
        assert_eq!(fp.factor_mask, 1);
    }

    #[test]
    fn footprint_write_scope_and_other_includes_both_nodes() {
        let store = GraphStore::default();
        let view = GraphView::new(&store);
        let scope = NodeId([0xEEu8; 32]);
        let other = other_node_of(&scope);

        let fp = write_scope_and_other_footprint(view, &scope);

        assert!(node_set_contains(&fp.n_write, &scope));
        assert!(node_set_contains(&fp.n_write, &other));
        assert_eq!(fp.factor_mask, 1);
    }

    #[test]
    fn footprint_empty_produces_default_footprint() {
        let store = GraphStore::default();
        let view = GraphView::new(&store);
        let scope = NodeId([0xFFu8; 32]);

        let fp = empty_footprint(view, &scope);

        assert!(!node_set_contains(&fp.n_read, &scope));
        assert!(!node_set_contains(&fp.n_write, &scope));
        assert_eq!(fp.factor_mask, 0);
    }

    #[test]
    fn builder_conflict_policy_propagates_abort() {
        let rule = SyntheticRuleBuilder::new("abort-rule")
            .conflict_policy(ConflictPolicy::Abort)
            .build();

        assert!(matches!(rule.conflict_policy, ConflictPolicy::Abort));
    }

    #[test]
    fn builder_conflict_policy_propagates_retry() {
        let rule = SyntheticRuleBuilder::new("retry-rule")
            .conflict_policy(ConflictPolicy::Retry)
            .build();

        assert!(matches!(rule.conflict_policy, ConflictPolicy::Retry));
    }

    #[test]
    fn builder_conflict_policy_propagates_join() {
        fn dummy_join(_left: &NodeId, _right: &NodeId) -> bool {
            true
        }

        let rule = SyntheticRuleBuilder::new("join-rule")
            .conflict_policy(ConflictPolicy::Join)
            .join_fn(dummy_join)
            .build();

        assert!(matches!(rule.conflict_policy, ConflictPolicy::Join));
        assert!(rule.join_fn.is_some());
    }

    #[test]
    fn builder_join_fn_propagates_to_rule() {
        fn my_join(left: &NodeId, right: &NodeId) -> bool {
            left.0[0] < right.0[0]
        }

        let rule = SyntheticRuleBuilder::new("join-fn-rule")
            .join_fn(my_join)
            .build();

        let join = rule.join_fn.expect("join_fn should be Some");
        let left = NodeId([0x10u8; 32]);
        let right = NodeId([0x20u8; 32]);
        assert!(join(&left, &right));
        assert!(!join(&right, &left));
    }

    #[test]
    fn other_node_of_uses_domain_separation() {
        // Verify that other_node_of produces a different result than a naive
        // blake3::hash call (demonstrating domain separation is in effect).
        let scope = NodeId([0x42u8; 32]);
        let other = other_node_of(&scope);

        // Naive hash without domain prefix
        let naive_hash: [u8; 32] = blake3::hash(&scope.0).into();

        // Domain-separated hash should differ from naive hash
        assert_ne!(other.0, naive_hash);
    }
}
