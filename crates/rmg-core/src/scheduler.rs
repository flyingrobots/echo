//! Deterministic rewrite scheduler and pending queue.
//!
//! Ordering invariant
//! - Rewrites for a transaction are executed in ascending lexicographic order
//!   of `(scope_hash, rule_id)`. This ordering is stable across platforms and
//!   runs and is enforced before returning the pending queue to callers.
use std::collections::{BTreeMap, HashMap};

use crate::footprint::Footprint;
use crate::ident::{CompactRuleId, Hash, NodeId};
#[cfg(feature = "telemetry")]
use crate::telemetry;
use crate::tx::TxId;

/// Ordering queue that guarantees rewrites execute deterministically.
#[derive(Debug, Default)]
pub(crate) struct DeterministicScheduler {
    pub(crate) pending: HashMap<TxId, BTreeMap<(Hash, Hash), PendingRewrite>>,
    pub(crate) active: HashMap<TxId, Vec<Footprint>>, // Reserved/Committed frontier
    #[cfg(feature = "telemetry")]
    pub(crate) counters: HashMap<TxId, (u64, u64)>, // (reserved, conflict)
}

/// Internal representation of a rewrite waiting to be applied.
#[derive(Debug)]
pub(crate) struct PendingRewrite {
    /// Identifier of the rule to execute.
    pub rule_id: Hash,
    /// Compact in-process rule handle used on hot paths.
    #[allow(dead_code)]
    pub compact_rule: CompactRuleId,
    /// Scope hash used for deterministic ordering together with `rule_id`.
    pub scope_hash: Hash,
    /// Scope node supplied when `apply` was invoked.
    pub scope: NodeId,
    /// Footprint used for independence checks and conflict resolution.
    #[allow(dead_code)]
    pub footprint: Footprint,
    /// State machine phase for the rewrite.
    #[allow(dead_code)]
    pub phase: RewritePhase,
}

/// Phase of a pending rewrite in the lock-free scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RewritePhase {
    /// Match found and footprint computed.
    Matched,
    /// Passed independence checks and reserved.
    #[allow(dead_code)]
    Reserved,
    /// Successfully applied.
    #[allow(dead_code)]
    Committed,
    /// Aborted due to conflict or validation failure.
    #[allow(dead_code)]
    Aborted,
}

impl DeterministicScheduler {
    /// Removes and returns all pending rewrites for `tx`, ordered by
    /// `(scope_hash, rule_id)` in ascending lexicographic order.
    pub(crate) fn drain_for_tx(&mut self, tx: TxId) -> Vec<PendingRewrite> {
        self.pending
            .remove(&tx)
            .map(|map| map.into_values().collect())
            .unwrap_or_default()
    }

    /// Attempts to reserve a rewrite by checking independence against the
    /// active frontier for `tx`. On success, pushes the footprint into the
    /// frontier and transitions the phase to `Reserved`.
    ///
    /// Current implementation: O(n) scan of the active frontier. For large
    /// transaction sizes, consider spatial indexing or hierarchical structures
    /// to reduce reservation cost.
    pub(crate) fn reserve(&mut self, tx: TxId, pr: &mut PendingRewrite) -> bool {
        let frontier = self.active.entry(tx).or_default();
        for fp in frontier.iter() {
            if !pr.footprint.independent(fp) {
                pr.phase = RewritePhase::Aborted;
                #[cfg(feature = "telemetry")]
                {
                    let entry = self.counters.entry(tx).or_default();
                    entry.1 += 1;
                }
                #[cfg(feature = "telemetry")]
                telemetry::conflict(tx, &pr.rule_id);
                return false;
            }
        }
        pr.phase = RewritePhase::Reserved;
        frontier.push(pr.footprint.clone());
        #[cfg(feature = "telemetry")]
        {
            let entry = self.counters.entry(tx).or_default();
            entry.0 += 1;
        }
        #[cfg(feature = "telemetry")]
        telemetry::reserved(tx, &pr.rule_id);
        true
    }

    /// Finalizes accounting for `tx`: emits a telemetry summary when enabled
    /// and clears the active frontier and counters for the transaction.
    pub(crate) fn finalize_tx(&mut self, tx: TxId) {
        #[cfg(feature = "telemetry")]
        if let Some((reserved, conflict)) = self.counters.remove(&tx) {
            telemetry::summary(tx, reserved, conflict);
        }
        self.active.remove(&tx);
    }
}
