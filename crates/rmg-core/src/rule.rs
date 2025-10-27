//! Rewrite rule definitions.
use crate::graph::GraphStore;
use crate::ident::{Hash, NodeId, TypeId};

/// Pattern metadata used by a rewrite rule to describe the input graph shape.
#[derive(Debug)]
pub struct PatternGraph {
    /// Ordered list of type identifiers that make up the pattern.
    pub nodes: Vec<TypeId>,
}

/// Function pointer used to determine whether a rule matches the provided scope.
pub type MatchFn = fn(&GraphStore, &NodeId) -> bool;

/// Function pointer that applies a rewrite to the given scope.
pub type ExecuteFn = fn(&mut GraphStore, &NodeId);

/// Descriptor for a rewrite rule registered with the engine.
///
/// Each rule owns:
/// * a deterministic identifier (`id`)
/// * a human-readable name
/// * a left pattern (currently unused by the spike)
/// * callbacks for matching and execution
pub struct RewriteRule {
    /// Deterministic identifier for the rewrite rule.
    pub id: Hash,
    /// Human-readable name for logs and debugging.
    pub name: &'static str,
    /// Pattern used to describe the left-hand side of the rule.
    pub left: PatternGraph,
    /// Callback used to determine if the rule matches the provided scope.
    pub matcher: MatchFn,
    /// Callback that applies the rewrite to the provided scope.
    pub executor: ExecuteFn,
}

