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
    attribution: Option<Vec<TickSubmissionAttribution>>,
    digest: Hash,
}

/// The decision a tick committed for one accepted submission (#873).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickSubmissionAttribution {
    /// The accepted submission the tick decided.
    pub submission_id: Hash,
    /// How the tick decided it.
    pub decision: TickSubmissionDecision,
}

/// How a tick decided one accepted submission.
///
/// The decision refers to the receipt entries that carry it rather than
/// copying their dispositions. A submission that no rule matched has its own
/// variant, so an empty entry list never stands for "no match".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickSubmissionDecision {
    /// The receipt entries, by index into [`TickReceipt::entries`], that carry
    /// this submission's outcome. Never empty; strictly increasing.
    ReceiptEntries(Vec<u32>),
    /// The tick committed the submission, but no rule matched it, so no
    /// receipt entry exists for it.
    NoMatchingRule,
}

/// Error returned when an attribution section does not fit its receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TickAttributionError {
    /// Attribution records must be in strictly increasing submission order,
    /// which also makes each submission appear once.
    #[error("tick attribution record {index} does not strictly follow the previous submission")]
    SubmissionsNotStrictlyIncreasing {
        /// Position of the out-of-order or repeated record.
        index: usize,
    },
    /// An entry-backed decision must name at least one receipt entry.
    #[error("tick attribution record {index} names no receipt entry")]
    EmptyReceiptAssociation {
        /// Position of the record.
        index: usize,
    },
    /// Receipt entry indices must be strictly increasing within a record.
    #[error("tick attribution record {index} entry {entry} does not strictly follow {previous}")]
    EntriesNotStrictlyIncreasing {
        /// Position of the record.
        index: usize,
        /// Entry index immediately preceding the invalid value.
        previous: u32,
        /// Repeated or descending entry index.
        entry: u32,
    },
    /// A record names a receipt entry the receipt does not have.
    #[error("tick attribution record {index} names entry {entry}, beyond the receipt's entries")]
    EntryOutOfRange {
        /// Position of the record.
        index: usize,
        /// Entry index outside the receipt.
        entry: u32,
    },
    /// Two submissions claim the same receipt entry.
    #[error("tick receipt entry {entry} is attributed to more than one submission")]
    EntryAttributedTwice {
        /// The doubly-claimed entry index.
        entry: u32,
    },
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
        let digest = compute_tick_receipt_digest(&entries, None);
        Self {
            tx,
            entries,
            blocked_by,
            attribution: None,
            digest,
        }
    }

    /// Adds the attribution section, which names every accepted submission
    /// the tick decided and the decision it committed for each (#873).
    ///
    /// A receipt with an attribution section uses receipt format version 3;
    /// one without keeps version 2, so its digest is unchanged. An empty
    /// section is valid and different from no section: it records that the
    /// tick decided no submission.
    ///
    /// # Errors
    ///
    /// Returns an error when records are not in strictly increasing
    /// submission order, when an entry-backed decision names no entry, names
    /// entries out of order or out of range, or when two submissions claim one
    /// entry.
    pub fn with_attribution(
        mut self,
        attribution: Vec<TickSubmissionAttribution>,
    ) -> Result<Self, TickAttributionError> {
        validate_attribution(self.entries.len(), &attribution)?;
        self.digest = compute_tick_receipt_digest(&self.entries, Some(&attribution));
        self.attribution = Some(attribution);
        Ok(self)
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
        let digest = compute_tick_receipt_digest(&entries, None);
        Ok(Self {
            tx,
            entries,
            blocked_by,
            attribution: None,
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

    /// Returns the attribution section, or `None` for a receipt written
    /// before attribution existed (receipt format version 2).
    #[must_use]
    pub fn attribution(&self) -> Option<&[TickSubmissionAttribution]> {
        self.attribution.as_deref()
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
    /// - the number of entries,
    /// - the ordered per-entry content, and
    /// - from format version 3, the attribution section.
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

fn validate_attribution(
    entry_count: usize,
    attribution: &[TickSubmissionAttribution],
) -> Result<(), TickAttributionError> {
    let mut owned = vec![false; entry_count];
    for (index, record) in attribution.iter().enumerate() {
        if index > 0 && attribution[index - 1].submission_id >= record.submission_id {
            return Err(TickAttributionError::SubmissionsNotStrictlyIncreasing { index });
        }
        let TickSubmissionDecision::ReceiptEntries(entries) = &record.decision else {
            continue;
        };
        if entries.is_empty() {
            return Err(TickAttributionError::EmptyReceiptAssociation { index });
        }
        for (position, &entry) in entries.iter().enumerate() {
            if position > 0 && entries[position - 1] >= entry {
                return Err(TickAttributionError::EntriesNotStrictlyIncreasing {
                    index,
                    previous: entries[position - 1],
                    entry,
                });
            }
            let slot = usize::try_from(entry)
                .ok()
                .and_then(|entry| owned.get_mut(entry))
                .ok_or(TickAttributionError::EntryOutOfRange { index, entry })?;
            if *slot {
                return Err(TickAttributionError::EntryAttributedTwice { entry });
            }
            *slot = true;
        }
    }
    Ok(())
}

fn compute_tick_receipt_digest(
    entries: &[TickReceiptEntry],
    attribution: Option<&[TickSubmissionAttribution]>,
) -> Hash {
    if entries.is_empty() && attribution.is_none() {
        return crate::constants::digest_len0_u64();
    }
    let mut hasher = Hasher::new();
    // Receipt format version tag: 3 when the attribution section is present.
    let version: u16 = if attribution.is_some() { 3 } else { 2 };
    hasher.update(&version.to_le_bytes());
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
    if let Some(attribution) = attribution {
        hasher.update(&(attribution.len() as u64).to_le_bytes());
        for record in attribution {
            hasher.update(&record.submission_id);
            match &record.decision {
                TickSubmissionDecision::ReceiptEntries(entries) => {
                    hasher.update(&[1u8]);
                    hasher.update(&(entries.len() as u64).to_le_bytes());
                    for entry in entries {
                        hasher.update(&entry.to_le_bytes());
                    }
                }
                TickSubmissionDecision::NoMatchingRule => {
                    hasher.update(&[2u8]);
                }
            }
        }
    }
    hasher.finalize().into()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
        let digest_a = compute_tick_receipt_digest(&entries, None);
        let digest_b = compute_tick_receipt_digest(&entries, None);
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

        let digest_a = compute_tick_receipt_digest(&entries_a, None);
        let digest_b = compute_tick_receipt_digest(&entries_b, None);
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

    fn sample_entries() -> Vec<TickReceiptEntry> {
        let warp_id = make_warp_id("receipt-attribution-warp");
        vec![
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
                    TickReceiptRejection::ExecutableOperationObstruction,
                ),
            },
        ]
    }

    fn sample_receipt() -> TickReceipt {
        TickReceipt::new(
            TxId::from_raw(1),
            sample_entries(),
            vec![Vec::new(), Vec::new()],
        )
    }

    fn entries_decision(entries: &[u32]) -> TickSubmissionDecision {
        TickSubmissionDecision::ReceiptEntries(entries.to_vec())
    }

    fn record(submission: u8, decision: TickSubmissionDecision) -> TickSubmissionAttribution {
        TickSubmissionAttribution {
            submission_id: [submission; 32],
            decision,
        }
    }

    /// The version 2 layout, written out independently of the implementation,
    /// so a change to legacy receipt digests fails here.
    fn frozen_v2_digest(entries: &[TickReceiptEntry]) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(&2u16.to_le_bytes());
        hasher.update(&(entries.len() as u64).to_le_bytes());
        for entry in entries {
            hasher.update(&entry.rule_id);
            hasher.update(&entry.scope_hash);
            hasher.update(entry.scope.warp_id.as_bytes());
            hasher.update(entry.scope.local_id.as_bytes());
            hasher.update(&[match entry.disposition {
                TickReceiptDisposition::Applied => 1u8,
                TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict) => 2,
                TickReceiptDisposition::Rejected(
                    TickReceiptRejection::ExecutableOperationObstruction,
                ) => 3,
            }]);
        }
        hasher.finalize().into()
    }

    #[test]
    fn a_receipt_without_attribution_keeps_its_version_2_digest() {
        let receipt = sample_receipt();
        assert_eq!(receipt.attribution(), None);
        assert_eq!(receipt.digest(), frozen_v2_digest(&sample_entries()));
        let empty = TickReceipt::new(TxId::from_raw(1), Vec::new(), Vec::new());
        assert_eq!(empty.digest(), crate::constants::digest_len0_u64());
    }

    #[test]
    fn attribution_is_covered_by_the_digest() {
        let legacy = sample_receipt();
        let attributed = sample_receipt()
            .with_attribution(vec![record(1, entries_decision(&[0, 1]))])
            .unwrap();
        assert_ne!(attributed.digest(), legacy.digest());
        let other_submission = sample_receipt()
            .with_attribution(vec![record(2, entries_decision(&[0, 1]))])
            .unwrap();
        assert_ne!(attributed.digest(), other_submission.digest());
        let split = sample_receipt()
            .with_attribution(vec![
                record(1, entries_decision(&[0])),
                record(2, entries_decision(&[1])),
            ])
            .unwrap();
        assert_ne!(attributed.digest(), split.digest());
        assert_eq!(attributed.attribution().map(<[_]>::len), Some(1));
    }

    #[test]
    fn an_empty_attribution_section_differs_from_none() {
        let none = TickReceipt::new(TxId::from_raw(1), Vec::new(), Vec::new());
        let empty = TickReceipt::new(TxId::from_raw(1), Vec::new(), Vec::new())
            .with_attribution(Vec::new())
            .unwrap();
        assert_eq!(empty.attribution(), Some(&[][..]));
        assert_ne!(empty.digest(), none.digest());
    }

    #[test]
    fn no_matching_rule_is_its_own_decision() {
        let no_match = TickReceipt::new(TxId::from_raw(1), Vec::new(), Vec::new())
            .with_attribution(vec![record(1, TickSubmissionDecision::NoMatchingRule)])
            .unwrap();
        let empty = TickReceipt::new(TxId::from_raw(1), Vec::new(), Vec::new())
            .with_attribution(Vec::new())
            .unwrap();
        assert_ne!(no_match.digest(), empty.digest());
        // An empty entry list is never a stand-in for "no match".
        assert_eq!(
            TickReceipt::new(TxId::from_raw(1), Vec::new(), Vec::new())
                .with_attribution(vec![record(1, entries_decision(&[]))]),
            Err(TickAttributionError::EmptyReceiptAssociation { index: 0 })
        );
    }

    #[test]
    fn malformed_attribution_is_refused() {
        let attribute = |records| sample_receipt().with_attribution(records);
        assert_eq!(
            attribute(vec![
                record(2, entries_decision(&[0])),
                record(1, entries_decision(&[1])),
            ]),
            Err(TickAttributionError::SubmissionsNotStrictlyIncreasing { index: 1 })
        );
        assert_eq!(
            attribute(vec![
                record(1, entries_decision(&[0])),
                record(1, entries_decision(&[1])),
            ]),
            Err(TickAttributionError::SubmissionsNotStrictlyIncreasing { index: 1 })
        );
        assert_eq!(
            attribute(vec![record(1, entries_decision(&[1, 0]))]),
            Err(TickAttributionError::EntriesNotStrictlyIncreasing {
                index: 0,
                previous: 1,
                entry: 0
            })
        );
        assert_eq!(
            attribute(vec![record(1, entries_decision(&[2]))]),
            Err(TickAttributionError::EntryOutOfRange { index: 0, entry: 2 })
        );
        assert_eq!(
            attribute(vec![
                record(1, entries_decision(&[0])),
                record(2, entries_decision(&[0])),
            ]),
            Err(TickAttributionError::EntryAttributedTwice { entry: 0 })
        );
    }
}
