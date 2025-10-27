//! Deterministic rewrite scheduler and pending queue.
//!
//! Ordering invariant
//! - Rewrites for a transaction are executed in ascending lexicographic order
//!   of `(scope_hash, rule_id)`. This ordering is stable across platforms and
//!   runs and is enforced before returning the pending queue to callers.
use std::collections::{BTreeMap, HashMap};

use crate::ident::{Hash, NodeId};
use crate::tx::TxId;

/// Ordering queue that guarantees rewrites execute deterministically.
#[derive(Debug, Default)]
pub(crate) struct DeterministicScheduler {
    pub(crate) pending: HashMap<TxId, BTreeMap<(Hash, Hash), PendingRewrite>>,
}

/// Internal representation of a rewrite waiting to be applied.
#[derive(Debug)]
pub(crate) struct PendingRewrite {
    /// Identifier of the rule to execute.
    pub rule_id: Hash,
    /// Scope hash used for deterministic ordering together with `rule_id`.
    pub scope_hash: Hash,
    /// Scope node supplied when `apply` was invoked.
    pub scope: NodeId,
}

impl DeterministicScheduler {
    /// Removes and returns all pending rewrites for `tx`, ordered by
    /// `(scope_hash, rule_id)` in ascending lexicographic order.
    pub(crate) fn drain_for_tx(&mut self, tx: TxId) -> Vec<PendingRewrite> {
        let mut items: Vec<PendingRewrite> = self
            .pending
            .remove(&tx)
            .map(|map| map.into_values().collect())
            .unwrap_or_default();
        items.sort_by(|a, b| {
            a.scope_hash
                .cmp(&b.scope_hash)
                .then(a.rule_id.cmp(&b.rule_id))
        });
        items
    }
}
