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
//!
//! A [`TickReceipt`] is one shell family in Echo's broader admission
//! architecture. It should not be mistaken for the universal carrier of every
//! witness-bearing publication the runtime may emit.

use blake3::Hasher;
use thiserror::Error;

use crate::admission::AdmissionOutcomeKind;
use crate::ident::{Hash, NodeKey};
use crate::tx::TxId;

/// A tick receipt: the per-candidate outcomes for a single commit attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickReceipt {
    tx: TxId,
    entries: Vec<TickReceiptEntry>,
    blocked_by: Vec<Vec<u32>>,
    digest: Hash,
}

/// Error returned when reconstructing a tick receipt from retained parts.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TickReceiptPartsError {
    /// Candidate entries and blocker lists must remain parallel.
    #[error("tick receipt entry count {entries} does not match blocker-list count {blocked_by}")]
    LengthMismatch {
        /// Number of candidate entries.
        entries: usize,
        /// Number of blocker lists.
        blocked_by: usize,
    },
    /// One blocker list is not in strictly increasing canonical order.
    #[error(
        "tick receipt entry {entry} blocker {blocker} does not strictly follow blocker {previous}"
    )]
    BlockersNotStrictlyIncreasing {
        /// Candidate entry whose blocker list is non-canonical.
        entry: usize,
        /// Blocker immediately preceding the invalid value.
        previous: u32,
        /// Duplicate or descending blocker value.
        blocker: u32,
    },
    /// A blocker does not refer to an earlier candidate entry.
    #[error("tick receipt entry {entry} blocker {blocker} is not an earlier candidate")]
    BlockerNotEarlier {
        /// Candidate entry carrying the invalid blocker.
        entry: usize,
        /// Same-position, forward, or out-of-range blocker index.
        blocker: u32,
    },
    /// A blocker refers to a candidate that was not applied.
    #[error("tick receipt entry {entry} blocker {blocker} does not name an applied candidate")]
    BlockerNotApplied {
        /// Rejected candidate carrying the invalid blocker.
        entry: usize,
        /// Earlier candidate whose disposition is not applied.
        blocker: u32,
    },
    /// Applied candidates cannot carry blocking attribution.
    #[error("applied tick receipt entry {entry} carries {blocker_count} blockers")]
    AppliedEntryHasBlockers {
        /// Applied candidate carrying blockers.
        entry: usize,
        /// Number of invalid blockers.
        blocker_count: usize,
    },
    /// Footprint-conflict rejection requires at least one applied blocker.
    #[error("rejected tick receipt entry {entry} carries no blockers")]
    RejectedEntryMissingBlockers {
        /// Rejected candidate missing its blocking attribution.
        entry: usize,
    },
    /// A non-conflict obstruction cannot attribute an applied blocker.
    #[error("obstructed tick receipt entry {entry} carries {blocker_count} blockers")]
    ObstructedEntryHasBlockers {
        /// Obstructed candidate carrying invalid blocker attribution.
        entry: usize,
        /// Number of invalid blockers.
        blocker_count: usize,
    },
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

    /// Reconstructs a receipt value from retained canonical parts.
    ///
    /// Constructing this value does not admit it as Echo history. Admission
    /// boundaries must still validate its digest against causal commitments.
    ///
    /// # Errors
    ///
    /// Returns an error when candidate entries and blocker lists are not
    /// parallel, or when blocker attribution violates canonical ordering,
    /// backward-reference, or disposition invariants.
    pub fn try_from_retained_parts(
        tx: TxId,
        entries: Vec<TickReceiptEntry>,
        blocked_by: Vec<Vec<u32>>,
    ) -> Result<Self, TickReceiptPartsError> {
        if entries.len() != blocked_by.len() {
            return Err(TickReceiptPartsError::LengthMismatch {
                entries: entries.len(),
                blocked_by: blocked_by.len(),
            });
        }
        for (entry_index, (entry, blockers)) in entries.iter().zip(&blocked_by).enumerate() {
            match entry.disposition {
                TickReceiptDisposition::Applied if !blockers.is_empty() => {
                    return Err(TickReceiptPartsError::AppliedEntryHasBlockers {
                        entry: entry_index,
                        blocker_count: blockers.len(),
                    });
                }
                TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict)
                    if blockers.is_empty() =>
                {
                    return Err(TickReceiptPartsError::RejectedEntryMissingBlockers {
                        entry: entry_index,
                    });
                }
                TickReceiptDisposition::Rejected(
                    TickReceiptRejection::ExecutableOperationObstruction,
                ) if !blockers.is_empty() => {
                    return Err(TickReceiptPartsError::ObstructedEntryHasBlockers {
                        entry: entry_index,
                        blocker_count: blockers.len(),
                    });
                }
                TickReceiptDisposition::Applied
                | TickReceiptDisposition::Rejected(
                    TickReceiptRejection::FootprintConflict
                    | TickReceiptRejection::ExecutableOperationObstruction,
                ) => {}
            }
            if let Some(pair) = blockers.windows(2).find(|pair| pair[0] >= pair[1]) {
                return Err(TickReceiptPartsError::BlockersNotStrictlyIncreasing {
                    entry: entry_index,
                    previous: pair[0],
                    blocker: pair[1],
                });
            }
            for &blocker in blockers {
                let Ok(blocker_index) = usize::try_from(blocker) else {
                    return Err(TickReceiptPartsError::BlockerNotEarlier {
                        entry: entry_index,
                        blocker,
                    });
                };
                if blocker_index >= entry_index {
                    return Err(TickReceiptPartsError::BlockerNotEarlier {
                        entry: entry_index,
                        blocker,
                    });
                }
                if entries[blocker_index].disposition != TickReceiptDisposition::Applied {
                    return Err(TickReceiptPartsError::BlockerNotApplied {
                        entry: entry_index,
                        blocker,
                    });
                }
            }
        }
        let digest = compute_tick_receipt_digest(&entries);
        Ok(Self {
            tx,
            entries,
            blocked_by,
            digest,
        })
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
    /// Instance-scoped scope node supplied when `Engine::apply` was invoked.
    ///
    /// This is a [`NodeKey`]: it contains both the warp instance identifier
    /// (`warp_id`) and the local node identifier within that instance (`local_id`).
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

impl TickReceiptDisposition {
    /// Maps the tick-local disposition into Echo's shared lawful outcome family.
    ///
    /// A rejected candidate in the current tick kernel is obstructed rather than
    /// transformed into explicit conflict residue.
    #[must_use]
    pub fn admission_outcome_kind(self) -> AdmissionOutcomeKind {
        match self {
            Self::Applied => AdmissionOutcomeKind::Derived,
            Self::Rejected(_) => AdmissionOutcomeKind::Obstruction,
        }
    }
}

/// Why a tick candidate was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickReceiptRejection {
    /// Candidate footprint conflicts with an already-accepted footprint.
    FootprintConflict,
    /// Echo's bounded executable-operation evaluator returned a typed
    /// obstruction and no candidate mutation.
    ExecutableOperationObstruction,
}

fn compute_tick_receipt_digest(entries: &[TickReceiptEntry]) -> Hash {
    if entries.is_empty() {
        return crate::constants::digest_len0_u64();
    }
    let mut hasher = Hasher::new();
    // Receipt format version tag.
    hasher.update(&2u16.to_le_bytes());
    // Entry count.
    hasher.update(&(entries.len() as u64).to_le_bytes());
    for entry in entries {
        hasher.update(&entry.rule_id);
        hasher.update(&entry.scope_hash);
        hasher.update(entry.scope.warp_id.as_bytes());
        hasher.update(entry.scope.local_id.as_bytes());
        let code = match entry.disposition {
            TickReceiptDisposition::Applied => 1u8,
            TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict) => 2u8,
            TickReceiptDisposition::Rejected(
                TickReceiptRejection::ExecutableOperationObstruction,
            ) => 3u8,
        };
        hasher.update(&[code]);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::{make_node_id, make_warp_id};

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
        assert_ne!(digest_a, crate::constants::digest_len0_u64());
    }

    #[test]
    fn receipt_digest_includes_warp_id() {
        let warp_id_a = make_warp_id("receipt-test-warp-a");
        let warp_id_b = make_warp_id("receipt-test-warp-b");
        let node = make_node_id("receipt-test-node");

        let entries_a = vec![TickReceiptEntry {
            rule_id: [1u8; 32],
            scope_hash: [2u8; 32],
            scope: NodeKey {
                warp_id: warp_id_a,
                local_id: node,
            },
            disposition: TickReceiptDisposition::Applied,
        }];

        let entries_b = vec![TickReceiptEntry {
            rule_id: [1u8; 32],
            scope_hash: [2u8; 32],
            scope: NodeKey {
                warp_id: warp_id_b,
                local_id: node,
            },
            disposition: TickReceiptDisposition::Applied,
        }];

        let digest_a = compute_tick_receipt_digest(&entries_a);
        let digest_b = compute_tick_receipt_digest(&entries_b);
        assert_ne!(digest_a, digest_b);
    }

    #[test]
    fn tick_receipt_disposition_maps_to_shared_admission_outcome_kind() {
        assert_eq!(
            TickReceiptDisposition::Applied.admission_outcome_kind(),
            AdmissionOutcomeKind::Derived
        );
        assert_eq!(
            TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict)
                .admission_outcome_kind(),
            AdmissionOutcomeKind::Obstruction
        );
    }
}
