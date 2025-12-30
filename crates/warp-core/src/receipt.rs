// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tick receipts (Paper II): accepted vs rejected rewrites.
//!
//! In the AIΩN Foundations terminology, a *tick* is an atomic commit attempt that
//! considers a deterministically ordered candidate set of rewrites and applies a
//! scheduler-admissible, conflict-free subset.
//!
//! Echo’s `warp-core` already exposes `plan_digest` (candidate set + ordering)
//! and `rewrites_digest` (the applied subset). A `TickReceipt` fills the gap:
//! it records, in canonical order, whether each candidate was accepted or
//! rejected and (when rejected) why.
//!
//! Today (engine spike), the only rejection reason is footprint conflict with
//! previously accepted rewrites in the same tick.

use blake3::Hasher;

use crate::ident::{Hash, NodeKey};
use crate::tx::TxId;

/// A tick receipt: the per-candidate outcomes for a single commit attempt.
#[derive(Debug, Clone)]
pub struct TickReceipt {
    tx: TxId,
    entries: Vec<TickReceiptEntry>,
    blocked_by: Vec<Vec<u32>>,
    digest: Hash,
}

impl TickReceipt {
    pub(crate) fn new(tx: TxId, entries: Vec<TickReceiptEntry>, blocked_by: Vec<Vec<u32>>) -> Self {
        assert_eq!(
            entries.len(),
            blocked_by.len(),
            "blocked_by must be parallel to entries"
        );
        let digest = compute_tick_receipt_digest(&entries);
        Self {
            tx,
            entries,
            blocked_by,
            digest,
        }
    }

    /// Transaction identifier associated with the tick receipt.
    #[must_use]
    pub fn tx(&self) -> TxId {
        self.tx
    }

    /// Returns the entries in canonical plan order.
    #[must_use]
    pub fn entries(&self) -> &[TickReceiptEntry] {
        &self.entries
    }

    /// Returns the indices of the candidates that blocked entry `idx`.
    ///
    /// This is the (currently minimal) *blocking causality poset* witness described
    /// by Paper II: when a candidate is rejected due to a footprint conflict, the
    /// receipt records which already-applied candidates blocked it.
    ///
    /// Semantics and invariants:
    /// - Returned indices are indices into [`TickReceipt::entries`].
    /// - The list is sorted in ascending order and contains no duplicates.
    /// - Every returned index is strictly less than `idx`.
    /// - For an [`TickReceiptDisposition::Applied`] entry, the list is empty.
    /// - For a [`TickReceiptDisposition::Rejected`] entry, the list is expected to
    ///   be non-empty for `Rejected(FootprintConflict)`.
    ///
    /// Note: these blocker indices are *not* included in [`TickReceipt::digest`]
    /// today; the digest commits only to accepted vs rejected outcomes.
    ///
    /// # Panics
    /// Panics if `idx` is out of bounds for [`TickReceipt::entries`].
    #[must_use]
    pub fn blocked_by(&self, idx: usize) -> &[u32] {
        assert!(
            idx < self.blocked_by.len(),
            "blocked_by index {idx} out of bounds for {} entries",
            self.blocked_by.len()
        );
        &self.blocked_by[idx]
    }

    /// Canonical digest of the tick receipt entries.
    ///
    /// This digest is stable across architectures and depends only on:
    /// - the receipt format version,
    /// - the number of entries, and
    /// - the ordered per-entry content.
    ///
    /// It intentionally does **not** include blocking attribution metadata
    /// (see [`TickReceipt::blocked_by`]) so that commit hashes remain stable
    /// across improvements to rejection explanations.
    ///
    /// It intentionally does **not** include `tx` so that receipts can be
    /// compared across runs that use different transaction numbering.
    #[must_use]
    pub fn digest(&self) -> Hash {
        self.digest
    }
}

/// One candidate rewrite and its tick outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickReceiptEntry {
    /// Canonical rule family id.
    pub rule_id: Hash,
    /// Scope hash used in the scheduler’s sort key.
    pub scope_hash: Hash,
    /// Scope node supplied when `Engine::apply` was invoked.
    pub scope: NodeKey,
    /// Outcome of the candidate rewrite in this tick.
    pub disposition: TickReceiptDisposition,
}

/// Outcome of a tick candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickReceiptDisposition {
    /// Candidate rewrite was accepted and applied.
    Applied,
    /// Candidate rewrite was rejected.
    Rejected(TickReceiptRejection),
}

/// Why a tick candidate was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickReceiptRejection {
    /// Candidate footprint conflicts with an already-accepted footprint.
    FootprintConflict,
}

fn compute_tick_receipt_digest(entries: &[TickReceiptEntry]) -> Hash {
    if entries.is_empty() {
        return *crate::constants::DIGEST_LEN0_U64;
    }
    let mut hasher = Hasher::new();
    // Receipt format version tag.
    hasher.update(&2u16.to_le_bytes());
    // Entry count.
    hasher.update(&(entries.len() as u64).to_le_bytes());
    for entry in entries {
        hasher.update(&entry.rule_id);
        hasher.update(&entry.scope_hash);
        hasher.update(&(entry.scope.warp_id).0);
        hasher.update(&(entry.scope.local_id).0);
        let code = match entry.disposition {
            TickReceiptDisposition::Applied => 1u8,
            TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict) => 2u8,
        };
        hasher.update(&[code]);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::make_node_id;

    #[test]
    fn receipt_digest_is_stable_for_same_entries() {
        let warp_id = crate::ident::make_warp_id("receipt-test-warp");
        let entries = vec![
            TickReceiptEntry {
                rule_id: [1u8; 32],
                scope_hash: [2u8; 32],
                scope: NodeKey {
                    warp_id,
                    local_id: make_node_id("a"),
                },
                disposition: TickReceiptDisposition::Applied,
            },
            TickReceiptEntry {
                rule_id: [3u8; 32],
                scope_hash: [4u8; 32],
                scope: NodeKey {
                    warp_id,
                    local_id: make_node_id("b"),
                },
                disposition: TickReceiptDisposition::Rejected(
                    TickReceiptRejection::FootprintConflict,
                ),
            },
        ];
        let digest_a = compute_tick_receipt_digest(&entries);
        let digest_b = compute_tick_receipt_digest(&entries);
        assert_eq!(digest_a, digest_b);
        assert_ne!(digest_a, *crate::constants::DIGEST_LEN0_U64);
    }
}
