//! Deterministic rewrite scheduler and pending queue.
use std::collections::{BTreeMap, HashMap};

use crate::ident::{Hash, NodeId};
use crate::tx::TxId;

/// Ordering queue that guarantees rewrites execute deterministically.
#[derive(Debug, Default)]
pub struct DeterministicScheduler {
    pub(crate) pending: HashMap<TxId, BTreeMap<(Hash, Hash), PendingRewrite>>,
}

/// Internal representation of a rewrite waiting to be applied.
#[derive(Debug)]
pub struct PendingRewrite {
    /// Transaction identifier that enqueued the rewrite.
    pub tx: TxId,
    /// Identifier of the rule to execute.
    pub rule_id: Hash,
    /// Scope node supplied when `apply` was invoked.
    pub scope: NodeId,
}

impl DeterministicScheduler {
    pub(crate) fn drain_for_tx(&mut self, tx: TxId) -> Vec<PendingRewrite> {
        self.pending
            .remove(&tx)
            .map(|map| map.into_values().collect())
            .unwrap_or_default()
    }
}
