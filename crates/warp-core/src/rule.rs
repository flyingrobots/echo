// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Rewrite rule definitions.
#![cfg_attr(not(feature = "native_rule_bootstrap"), allow(dead_code))]

use crate::execution_graph_view::ExecutionGraphView;
use crate::footprint::Footprint;
use crate::graph_view::GraphView;
use crate::ident::{Hash, NodeId, TypeId};
use crate::TickDelta;

/// Pattern metadata used by a rewrite rule to describe the input graph shape.
#[derive(Debug)]
pub struct PatternGraph {
    /// Ordered list of type identifiers that make up the pattern.
    pub nodes: Vec<TypeId>,
}

/// Function pointer used to determine whether a rule matches the provided scope.
///
/// Phase 5 signature: matchers read from an immutable [`GraphView`]
/// to inspect graph state without mutation.
///
/// Parameters:
/// - `GraphView`: Read-only view over the graph state (Copy type, 8 bytes)
/// - `&NodeId`: The candidate scope node to test
pub type MatchFn = for<'a> fn(GraphView<'a>, &NodeId) -> bool;

/// Legacy function pointer that applies a rewrite to the given scope.
///
/// This ABI cannot record graph reads and therefore always produces
/// `UnavailableLegacyExecutor` evidence. It remains for frozen compatibility
/// callbacks such as provider-v1. Native bootstrap rules use
/// [`ObservedExecuteFn`] instead. Both ABIs preserve the separation between
/// observation and mutation: executors emit mutations to a [`TickDelta`].
///
/// Parameters:
/// - `GraphView`: Read-only view over the graph state (Copy type, 8 bytes)
/// - `&NodeId`: The node ID where the rewrite is applied
/// - `&mut TickDelta`: Mutable reference to record emitted changes
pub type ExecuteFn = for<'a> fn(GraphView<'a>, &NodeId, &mut TickDelta);

/// Function pointer for an executor whose graph reads are recorded.
///
/// Unlike legacy [`ExecuteFn`], the observed callback receives an exclusive
/// borrow of [`ExecutionGraphView`]. The view records each attempted read before
/// applying the declared-footprint guard, so an unwind cannot erase the access
/// that caused it.
pub type ObservedExecuteFn =
    for<'store, 'frame> fn(&mut ExecutionGraphView<'store, 'frame>, &NodeId, &mut TickDelta);

/// Execution ABI selected for one rewrite rule.
///
/// Native rules use [`Observed`](Self::Observed). [`Legacy`](Self::Legacy)
/// remains explicit for frozen compatibility callbacks, including provider-v1;
/// it never produces an authoritative read-footprint record.
#[derive(Clone, Copy)]
pub enum RuleExecutor {
    /// Executor whose reads pass through [`ExecutionGraphView`].
    Observed(ObservedExecuteFn),
    /// Compatibility executor whose reads pass through legacy [`GraphView`].
    Legacy(ExecuteFn),
}

impl RuleExecutor {
    /// Wraps an executor that records graph reads.
    #[must_use]
    pub const fn observed(executor: ObservedExecuteFn) -> Self {
        Self::Observed(executor)
    }

    /// Wraps a frozen compatibility executor.
    #[must_use]
    pub const fn legacy(executor: ExecuteFn) -> Self {
        Self::Legacy(executor)
    }

    /// Returns `true` when the callback uses the observed executor ABI.
    #[must_use]
    pub const fn is_observed(self) -> bool {
        matches!(self, Self::Observed(_))
    }
}

impl core::fmt::Debug for RuleExecutor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Observed(_) => "RuleExecutor::Observed",
            Self::Legacy(_) => "RuleExecutor::Legacy",
        })
    }
}

/// Function pointer that computes a rewrite footprint at the provided scope.
///
/// Phase 5 signature: footprint computation reads from an immutable
/// [`GraphView`] to declare the read/write sets without mutation. The declared
/// [`Footprint`] is also the first-cut substrate from which Echo derives a
/// [`BoundedSite`](crate::admission::BoundedSite) for admission.
///
/// Parameters:
/// - `GraphView`: Read-only view over the graph state (Copy type, 8 bytes)
/// - `&NodeId`: The scope node for which to compute the footprint
pub type FootprintFn = for<'a> fn(GraphView<'a>, &NodeId) -> Footprint;

/// Conflict resolution policies for independence failures.
#[derive(Debug, Clone, Copy)]
pub enum ConflictPolicy {
    /// Abort the rewrite when a conflict is detected.
    Abort,
    /// Retry (re-match) against the latest state.
    Retry,
    /// Attempt a join using a rule-provided strategy.
    ///
    /// Requires the rule's [`RewriteRule::join_fn`] field to be `Some`; rules
    /// specifying `Join` without providing a join function will be rejected at
    /// registration time in future revisions.
    Join,
}

/// Optional join strategy used when `conflict_policy == ConflictPolicy::Join`.
///
/// The spike does not use joins yet; the signature is kept minimal until
/// pending rewrite metadata stabilises across modules.
pub type JoinFn = fn(/* left */ &NodeId, /* right */ &NodeId) -> bool;

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
    /// Observed or explicitly legacy callback that applies the rewrite.
    pub executor: RuleExecutor,
    /// Callback that computes a footprint for independence checks.
    pub compute_footprint: FootprintFn,
    /// Spatial partition bitmask used as an O(1) prefilter.
    pub factor_mask: u64,
    /// Conflict resolution policy when independence fails.
    pub conflict_policy: ConflictPolicy,
    /// Join function required when `conflict_policy == ConflictPolicy::Join`.
    ///
    /// Invariant: If `conflict_policy` is `ConflictPolicy::Join`, this field
    /// must be `Some`. Rules that violate this invariant are subject to being
    /// rejected by `Engine::register_rule` in future revisions.
    pub join_fn: Option<JoinFn>,
}

impl core::fmt::Debug for RewriteRule {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RewriteRule")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}
