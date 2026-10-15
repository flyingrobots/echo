// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded application-facing projection of retained causal receipt history.

use thiserror::Error;

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
use crate::{
    contract_inverse::recover_contract_inverse_derivation, ProvenanceService, ProvenanceStore,
    ReceiptCorrelationRecord, WorldlineRuntime,
};
use crate::{
    CausalTickReceiptRef, ContractInverseDerivation, ContractInverseHistoryObstruction, Hash,
    InstalledInvocationEvidence, WorldlineId, WorldlineTick,
};

/// Maximum number of causal parents one history entry may cite.
///
/// Admission does not yet bound an ingress's causal parents, so the query
/// does: each cited parent costs a lookup and a provenance check, and this
/// keeps one page's work and size proportional to its page size. An entry
/// over the bound is an obstruction, never a truncated parent list.
pub const MAX_CAUSAL_RECEIPT_HISTORY_PARENTS: usize = 64;

/// Maximum number of retained receipt entries returned by one history page.
pub const MAX_CAUSAL_RECEIPT_HISTORY_PAGE_SIZE: usize = 256;

/// Maximum number of ticks one history page inspects, whether or not they
/// yield entries. A page that reaches it returns a continuation cursor, so a
/// long run of ticks with no submissions costs several pages, never one
/// unbounded page.
pub const MAX_CAUSAL_RECEIPT_HISTORY_INSPECTED_TICKS: u64 = 1024;

/// Maximum number of attribution records or legacy correlations one history
/// page inspects. With the page size and the per-entry parent bound, it bounds
/// a page's work: each inspected record costs a fixed number of index
/// lookups and at most [`MAX_CAUSAL_RECEIPT_HISTORY_PARENTS`] parent checks.
pub const MAX_CAUSAL_RECEIPT_HISTORY_INSPECTED_RECORDS: usize = 1024;

/// Canonical traversal direction for causal receipt history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CausalReceiptHistoryDirection {
    /// Traverse from the earliest admitted receipt toward the selected basis.
    OldestFirst,
    /// Traverse from the selected basis toward the earliest admitted receipt.
    NewestFirst,
}

/// Opaque continuation position for one basis-pinned history traversal.
///
/// Cursors are issued by Echo and bind the worldline, basis and direction,
/// plus the next tick to inspect and, within it, the last submission already
/// returned. A cursor can therefore resume inside a tick, or at a tick a
/// budget-limited page did not reach. It cannot be reused for a different
/// history view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalReceiptHistoryCursor {
    worldline_id: WorldlineId,
    basis_frontier_tick: WorldlineTick,
    direction: CausalReceiptHistoryDirection,
    next_tick: WorldlineTick,
    after_submission: Option<Hash>,
}

/// Whether the ticks one page inspected could all be checked for
/// completeness (#873).
///
/// The claim covers only [`CausalReceiptHistoryPage::inspected`], under the
/// page's pinned basis. It says nothing about ticks other pages inspect, so a
/// traversal is complete only if every one of its pages is `Established`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CausalReceiptHistoryCompleteness {
    /// Every inspected tick committed an attribution section, and every
    /// submission those ticks decided was returned or lies ahead of the
    /// page's cursor.
    Established,
    /// At least one inspected tick is a legacy tick, which commits no
    /// attribution section. Its retained correlations are returned, but Echo
    /// cannot prove none is missing.
    UnestablishedForLegacyTicks {
        /// The first legacy tick the page inspected, in traversal order.
        first_legacy_tick: WorldlineTick,
    },
}

/// The ticks one page inspected, in traversal order, inclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalReceiptHistoryTickRange {
    /// First inspected tick (provenance index).
    pub first: WorldlineTick,
    /// Last inspected tick (provenance index).
    pub last: WorldlineTick,
}

/// Request for one bounded page of retained causal receipt history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalReceiptHistoryRequest {
    /// Worldline whose admitted receipt history should be projected.
    pub worldline_id: WorldlineId,
    /// Explicit frontier basis that bounds the projection.
    pub basis_frontier_tick: WorldlineTick,
    /// Canonical traversal direction.
    pub direction: CausalReceiptHistoryDirection,
    /// Maximum number of entries requested for this page.
    pub page_size: usize,
    /// Echo-issued continuation cursor from the preceding page, if any.
    pub cursor: Option<CausalReceiptHistoryCursor>,
}

/// One application-facing entry derived from retained Echo evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalReceiptHistoryEntry {
    /// Exact causal coordinate of the admitted transition.
    pub receipt_ref: CausalTickReceiptRef,
    /// Witnessed submission decided by the transition.
    pub submission_id: Hash,
    /// Installed-contract evidence attached to the transition, if any.
    pub contract: Option<InstalledInvocationEvidence>,
    /// Canonical retained receipt references cited by the admitted intent.
    pub causal_parent_receipts: Vec<CausalTickReceiptRef>,
    /// Typed inverse derivation when this transition is a contract inverse.
    pub inverse_derivation: Option<ContractInverseDerivation>,
}

/// One bounded page of application-facing causal receipt history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalReceiptHistoryPage {
    /// Entries in the requested canonical traversal order.
    pub entries: Vec<CausalReceiptHistoryEntry>,
    /// Continuation cursor when the traversal has not reached its end. A page
    /// may hold fewer entries than requested, or none, and still continue,
    /// when it spent its inspection budget.
    pub next_cursor: Option<CausalReceiptHistoryCursor>,
    /// The ticks this page inspected, or `None` when it inspected none.
    pub inspected: Option<CausalReceiptHistoryTickRange>,
    /// Whether every inspected tick could be checked for completeness. The
    /// claim covers `inspected` only.
    pub completeness: CausalReceiptHistoryCompleteness,
}

/// Typed obstruction while projecting retained causal receipt history.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum CausalReceiptHistoryObstruction {
    /// The requested page size was zero or exceeded Echo's fixed upper bound.
    #[error(
        "causal receipt history page size must be within 1..={max_page_size}, got {requested_page_size}"
    )]
    InvalidPageSize {
        /// Rejected caller-supplied page size.
        requested_page_size: usize,
        /// Maximum page size accepted by this runtime.
        max_page_size: usize,
    },
    /// The requested worldline is not registered in the recovered runtime.
    #[error("causal receipt history worldline is unavailable: {worldline_id:?}")]
    WorldlineUnavailable {
        /// Missing worldline.
        worldline_id: WorldlineId,
    },
    /// The requested basis lies beyond the recovered worldline frontier.
    #[error(
        "causal receipt history basis {basis_frontier_tick} exceeds frontier {current_frontier_tick} for {worldline_id:?}"
    )]
    BasisBeyondFrontier {
        /// Requested worldline.
        worldline_id: WorldlineId,
        /// Requested basis frontier.
        basis_frontier_tick: WorldlineTick,
        /// Current recovered frontier.
        current_frontier_tick: WorldlineTick,
    },
    /// Provenance does not retain the transition immediately preceding a nonzero basis.
    #[error(
        "causal receipt history basis evidence is unavailable at frontier {basis_frontier_tick} for {worldline_id:?}"
    )]
    BasisEvidenceUnavailable {
        /// Requested worldline.
        worldline_id: WorldlineId,
        /// Requested basis frontier.
        basis_frontier_tick: WorldlineTick,
    },
    /// The continuation cursor was minted for a different request scope.
    #[error(
        "causal receipt history cursor does not match the requested worldline, basis, or direction"
    )]
    CursorScopeMismatch,
    /// The continuation cursor names a tick outside the basis, or a submission
    /// that tick does not account for.
    #[error(
        "causal receipt history cursor position is unavailable: tick {worldline_tick}, after {after_submission:?}"
    )]
    CursorPositionUnavailable {
        /// Tick the cursor names.
        worldline_tick: WorldlineTick,
        /// Submission the cursor resumes after, if any.
        after_submission: Option<Hash>,
    },
    /// A receipt correlation does not agree with its exact receipt coordinate.
    #[error("causal receipt history correlation is inconsistent: {receipt_ref:?}")]
    ReceiptEvidenceInconsistent {
        /// Exact receipt whose retained fields disagree.
        receipt_ref: Box<CausalTickReceiptRef>,
    },
    /// More than one retained correlation claims the same exact receipt
    /// coordinate, so one of them is shadowed.
    #[error("causal receipt history has duplicate receipt evidence")]
    DuplicateReceiptEvidence,
    /// A receipt names a transition that retained history does not hold, or
    /// holds with a different commit or receipt content, so the history before
    /// the basis is not fully available.
    #[error("causal receipt history evidence is not retained for {receipt_ref:?}")]
    RetentionGap {
        /// First receipt whose transition is not retained as named.
        receipt_ref: Box<CausalTickReceiptRef>,
    },
    /// Retained history holds no transition for a tick before the basis.
    #[error("causal receipt history tick {worldline_tick} is not retained")]
    TickUnavailable {
        /// Provenance index of the missing tick.
        worldline_tick: WorldlineTick,
    },
    /// A receipt correlation is indexed for a tick whose committed
    /// attribution does not name its submission.
    #[error("causal receipt history correlation is not attributed by its tick: {receipt_ref:?}")]
    UnattributedCorrelation {
        /// Receipt of the unexplained correlation.
        receipt_ref: Box<CausalTickReceiptRef>,
    },
    /// A tick's committed attribution names a ticketed submission that has no
    /// receipt correlation: retained correlation material was lost.
    #[error(
        "causal receipt history submission {submission_id:?} decided at tick {worldline_tick} has no receipt correlation"
    )]
    AttributedSubmissionUncorrelated {
        /// Provenance index of the deciding tick.
        worldline_tick: WorldlineTick,
        /// Submission the tick attributes.
        submission_id: Hash,
    },
    /// A tick's committed attribution names a submission that was never
    /// ticketed, so it has no receipt identity to report.
    #[error(
        "causal receipt history submission {submission_id:?} decided at tick {worldline_tick} has no receipt identity"
    )]
    SubmissionWithoutReceiptIdentity {
        /// Provenance index of the deciding tick.
        worldline_tick: WorldlineTick,
        /// Submission the tick attributes.
        submission_id: Hash,
    },
    /// A history entry cites more causal parents than one entry may return.
    #[error(
        "causal receipt history entry {receipt_ref:?} cites {parent_count} causal parents, over the bound {max}"
    )]
    CausalParentsExceedBound {
        /// Receipt whose causal parents exceed the bound.
        receipt_ref: Box<CausalTickReceiptRef>,
        /// Number of causal parents the entry cites.
        parent_count: usize,
        /// [`MAX_CAUSAL_RECEIPT_HISTORY_PARENTS`].
        max: usize,
    },
    /// A retained receipt does not have its witnessed submission record and envelope.
    #[error(
        "causal receipt history submission is unavailable for {receipt_ref:?}: {submission_id:?}"
    )]
    SubmissionUnavailable {
        /// Exact receipt whose submission material is missing.
        receipt_ref: Box<CausalTickReceiptRef>,
        /// Missing witnessed submission.
        submission_id: Hash,
    },
    /// Witnessed submission material disagrees with the receipt correlation.
    #[error("causal receipt history submission is inconsistent for {receipt_ref:?}")]
    SubmissionEvidenceInconsistent {
        /// Exact receipt whose witnessed submission material disagrees.
        receipt_ref: Box<CausalTickReceiptRef>,
    },
    /// A retained causal-parent reference cannot be resolved to receipt evidence.
    #[error(
        "causal receipt history parent {parent_receipt_ref:?} is unavailable for {receipt_ref:?}"
    )]
    CausalParentUnavailable {
        /// Exact child receipt.
        receipt_ref: Box<CausalTickReceiptRef>,
        /// Missing parent receipt.
        parent_receipt_ref: Box<CausalTickReceiptRef>,
    },
    /// Typed contract-inverse history could not be recovered.
    #[error("causal receipt history inverse derivation is obstructed: {0}")]
    ContractInverse(#[from] ContractInverseHistoryObstruction),
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
pub(crate) fn causal_receipt_history(
    runtime: &WorldlineRuntime,
    provenance: &ProvenanceService,
    request: CausalReceiptHistoryRequest,
) -> Result<CausalReceiptHistoryPage, CausalReceiptHistoryObstruction> {
    if request.page_size == 0 || request.page_size > MAX_CAUSAL_RECEIPT_HISTORY_PAGE_SIZE {
        return Err(CausalReceiptHistoryObstruction::InvalidPageSize {
            requested_page_size: request.page_size,
            max_page_size: MAX_CAUSAL_RECEIPT_HISTORY_PAGE_SIZE,
        });
    }

    let frontier = runtime.worldlines().get(&request.worldline_id).ok_or(
        CausalReceiptHistoryObstruction::WorldlineUnavailable {
            worldline_id: request.worldline_id,
        },
    )?;
    let current_frontier_tick = frontier.frontier_tick();
    if request.basis_frontier_tick > current_frontier_tick {
        return Err(CausalReceiptHistoryObstruction::BasisBeyondFrontier {
            worldline_id: request.worldline_id,
            basis_frontier_tick: request.basis_frontier_tick,
            current_frontier_tick,
        });
    }
    if request.basis_frontier_tick != WorldlineTick::ZERO {
        let evidence_tick = request.basis_frontier_tick.checked_sub(1).ok_or(
            CausalReceiptHistoryObstruction::BasisEvidenceUnavailable {
                worldline_id: request.worldline_id,
                basis_frontier_tick: request.basis_frontier_tick,
            },
        )?;
        provenance
            .commit_summary(request.worldline_id, evidence_tick)
            .map_err(
                |_| CausalReceiptHistoryObstruction::BasisEvidenceUnavailable {
                    worldline_id: request.worldline_id,
                    basis_frontier_tick: request.basis_frontier_tick,
                },
            )?;
    }

    let tick_count = request.basis_frontier_tick.as_u64();
    let newest_first = request.direction == CausalReceiptHistoryDirection::NewestFirst;
    let mut position = match &request.cursor {
        Some(cursor) => {
            if cursor.worldline_id != request.worldline_id
                || cursor.basis_frontier_tick != request.basis_frontier_tick
                || cursor.direction != request.direction
            {
                return Err(CausalReceiptHistoryObstruction::CursorScopeMismatch);
            }
            if cursor.next_tick.as_u64() >= tick_count {
                return Err(CausalReceiptHistoryObstruction::CursorPositionUnavailable {
                    worldline_tick: cursor.next_tick,
                    after_submission: cursor.after_submission,
                });
            }
            Some((cursor.next_tick.as_u64(), cursor.after_submission))
        }
        None if tick_count == 0 => None,
        None if newest_first => Some((tick_count - 1, None)),
        None => Some((0, None)),
    };

    // Both writers of the correlation indexes refuse a receipt that is already
    // indexed, so one correlation per receipt is a write-time invariant. This
    // O(1) count comparison is a backstop for corrupt state only.
    if runtime.receipt_correlation_index_has_shadowed_records() {
        return Err(CausalReceiptHistoryObstruction::DuplicateReceiptEvidence);
    }

    // Walk ticks, not the correlation index: a tick's committed attribution is
    // the authority for which submissions it decided, and the index is a
    // cache checked against it (#873).
    let mut walk = PageWalk {
        runtime,
        provenance,
        request: &request,
        entries: Vec::with_capacity(request.page_size),
        records_inspected: 0,
    };
    let mut completeness = CausalReceiptHistoryCompleteness::Established;
    let mut inspected: Option<CausalReceiptHistoryTickRange> = None;
    let mut ticks_inspected = 0_u64;
    let mut next_cursor = None;
    let cursor_at = |tick: u64, after_submission: Option<Hash>| CausalReceiptHistoryCursor {
        worldline_id: request.worldline_id,
        basis_frontier_tick: request.basis_frontier_tick,
        direction: request.direction,
        next_tick: WorldlineTick::from_raw(tick),
        after_submission,
    };
    while let Some((tick, after)) = position {
        if walk.page_full() || ticks_inspected == MAX_CAUSAL_RECEIPT_HISTORY_INSPECTED_TICKS {
            next_cursor = Some(cursor_at(tick, after));
            break;
        }
        ticks_inspected += 1;
        let worldline_tick = WorldlineTick::from_raw(tick);
        inspected = Some(CausalReceiptHistoryTickRange {
            first: inspected.map_or(worldline_tick, |range| range.first),
            last: worldline_tick,
        });
        match walk.visit_tick(tick, after)? {
            TickVisit::Finished { attributed } => {
                if !attributed && completeness == CausalReceiptHistoryCompleteness::Established {
                    completeness = CausalReceiptHistoryCompleteness::UnestablishedForLegacyTicks {
                        first_legacy_tick: worldline_tick,
                    };
                }
            }
            TickVisit::Stopped { attributed, last } => {
                if !attributed && completeness == CausalReceiptHistoryCompleteness::Established {
                    completeness = CausalReceiptHistoryCompleteness::UnestablishedForLegacyTicks {
                        first_legacy_tick: worldline_tick,
                    };
                }
                next_cursor = Some(cursor_at(tick, last));
                break;
            }
        }
        position = if newest_first {
            tick.checked_sub(1).map(|previous| (previous, None))
        } else {
            Some(tick + 1)
                .filter(|next| *next < tick_count)
                .map(|next| (next, None))
        };
    }

    Ok(CausalReceiptHistoryPage {
        entries: walk.entries,
        next_cursor,
        inspected,
        completeness,
    })
}

/// How far one page got through one tick.
#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
enum TickVisit {
    /// Every submission of the tick after the starting point was inspected.
    Finished { attributed: bool },
    /// The page filled or spent its record budget; `last` is the last
    /// submission inspected in this tick, where the next page resumes.
    Stopped {
        attributed: bool,
        last: Option<Hash>,
    },
}

/// One page's traversal state.
#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
struct PageWalk<'a> {
    runtime: &'a WorldlineRuntime,
    provenance: &'a ProvenanceService,
    request: &'a CausalReceiptHistoryRequest,
    entries: Vec<CausalReceiptHistoryEntry>,
    records_inspected: usize,
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
impl<'a> PageWalk<'a> {
    fn page_full(&self) -> bool {
        self.entries.len() == self.request.page_size
    }

    fn budget_spent(&self) -> bool {
        self.page_full() || self.records_inspected == MAX_CAUSAL_RECEIPT_HISTORY_INSPECTED_RECORDS
    }

    /// Inspects tick `tick` from just after `after`, lazily.
    ///
    /// An attributed tick is walked in lockstep with the index entries for
    /// its commit, both in submission order, so each step consumes one
    /// attribution record and the index entries for that submission. Every
    /// index entry is therefore checked against the attribution, and a page
    /// pays only for the records it reaches. A legacy tick walks its index
    /// entries.
    fn visit_tick(
        &mut self,
        tick: u64,
        after: Option<Hash>,
    ) -> Result<TickVisit, CausalReceiptHistoryObstruction> {
        let runtime: &'a WorldlineRuntime = self.runtime;
        let provenance: &'a ProvenanceService = self.provenance;
        let worldline_id = self.request.worldline_id;
        let worldline_tick = WorldlineTick::from_raw(tick);
        let unavailable = || CausalReceiptHistoryObstruction::TickUnavailable { worldline_tick };
        let summary = provenance
            .commit_summary(worldline_id, worldline_tick)
            .map_err(|_| unavailable())?;
        let receipt = provenance
            .retained_tick_receipt(worldline_id, worldline_tick)
            .map_err(|_| unavailable())?;
        let tick_after = WorldlineTick::from_raw(tick + 1);
        let commit = CausalTickReceiptRef {
            worldline_id,
            worldline_tick_after: tick_after,
            commit_global_tick: summary.commit_global_tick,
            commit_hash: summary.commit.commit_hash,
            submission_id: [0; 32],
            ticket_digest: [0; 32],
            receipt_content_digest: [0; 32],
        };
        // Every key indexed for this tick must name this tick's commit. Keys
        // sort by global tick and commit after the tick, so checking both ends
        // checks every key between them.
        if let Some(ends) = runtime.receipt_ref_ends_at_tick(worldline_id, tick_after) {
            let foreign = |end: &CausalTickReceiptRef| {
                end.commit_global_tick != commit.commit_global_tick
                    || end.commit_hash != commit.commit_hash
            };
            let (first, last) = ends;
            let foreign_end = if foreign(&first) {
                Some(first)
            } else if foreign(&last) {
                Some(last)
            } else {
                None
            };
            if let Some(end) = foreign_end {
                return Err(
                    CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent {
                        receipt_ref: Box::new(end),
                    },
                );
            }
        }
        let newest_first = self.request.direction == CausalReceiptHistoryDirection::NewestFirst;
        let Some(attribution) = receipt.and_then(crate::TickReceipt::attribution) else {
            return self.visit_legacy_tick(&commit, after, newest_first);
        };

        let start = match after {
            None => None,
            Some(after) => Some(
                attribution
                    .binary_search_by_key(&after, |record| record.submission_id)
                    .map_err(
                        |_| CausalReceiptHistoryObstruction::CursorPositionUnavailable {
                            worldline_tick,
                            after_submission: Some(after),
                        },
                    )?,
            ),
        };
        // The submission the cursor resumes after was returned on an earlier
        // page; its evidence is checked again so a gap opened since then is
        // not stepped over.
        if let Some(index) = start {
            let record = &attribution[index];
            let mut its_entries =
                runtime.receipt_correlations_of_submission(&commit, record.submission_id);
            let indexed = its_entries.next();
            if its_entries.next().is_some() {
                return Err(Self::inconsistent(&commit, record.submission_id));
            }
            self.records_inspected += 1;
            self.checked_correlation(worldline_tick, tick_after, record.submission_id, indexed)?;
        }
        let records: Box<dyn Iterator<Item = &crate::TickSubmissionAttribution>> =
            match (start, newest_first) {
                (None, false) => Box::new(attribution.iter()),
                (None, true) => Box::new(attribution.iter().rev()),
                (Some(index), false) => Box::new(attribution[index + 1..].iter()),
                (Some(index), true) => Box::new(attribution[..index].iter().rev()),
            };
        let mut indexed = runtime
            .receipt_correlations_in_commit(&commit, after, newest_first)
            .peekable();
        let precedes = |key: &Hash, submission: &Hash| {
            if newest_first {
                key > submission
            } else {
                key < submission
            }
        };
        let mut last = after;
        for record in records {
            if self.budget_spent() {
                return Ok(TickVisit::Stopped {
                    attributed: true,
                    last,
                });
            }
            self.records_inspected += 1;
            // Index entries ordered before this submission name no attributed
            // submission.
            if let Some((key, _)) = indexed.peek() {
                if precedes(&key.submission_id, &record.submission_id) {
                    return Err(CausalReceiptHistoryObstruction::UnattributedCorrelation {
                        receipt_ref: Box::new(*key),
                    });
                }
            }
            let own = indexed.next_if(|(key, _)| key.submission_id == record.submission_id);
            if indexed
                .peek()
                .is_some_and(|(key, _)| key.submission_id == record.submission_id)
            {
                return Err(Self::inconsistent(&commit, record.submission_id));
            }
            let correlation =
                self.checked_correlation(worldline_tick, tick_after, record.submission_id, own)?;
            self.entries
                .push(history_entry(runtime, provenance, correlation)?);
            last = Some(record.submission_id);
        }
        // Whatever the index still holds for this commit is unattributed.
        if let Some((key, _)) = indexed.next() {
            return Err(CausalReceiptHistoryObstruction::UnattributedCorrelation {
                receipt_ref: Box::new(key),
            });
        }
        Ok(TickVisit::Finished { attributed: true })
    }

    /// Walks a legacy tick's index entries, which are all Echo has for it.
    fn visit_legacy_tick(
        &mut self,
        commit: &CausalTickReceiptRef,
        after: Option<Hash>,
        newest_first: bool,
    ) -> Result<TickVisit, CausalReceiptHistoryObstruction> {
        let runtime: &'a WorldlineRuntime = self.runtime;
        let provenance: &'a ProvenanceService = self.provenance;
        let worldline_tick = WorldlineTick::from_raw(commit.worldline_tick_after.as_u64() - 1);
        if let Some(after) = after {
            // The resumed-after submission was returned on an earlier page; its
            // exact receipt is checked again so a continuation point lost or
            // damaged since then is not stepped over.
            let Some(correlation) = runtime.receipt_correlation_for_submission(&after) else {
                return Err(CausalReceiptHistoryObstruction::CursorPositionUnavailable {
                    worldline_tick,
                    after_submission: Some(after),
                });
            };
            let receipt_ref = correlation.causal_receipt_ref;
            // The correlation must be the cursor's own submission, on this
            // tick's commit, not another receipt a damaged index resolves to.
            if receipt_ref.submission_id != after
                || receipt_ref.worldline_id != commit.worldline_id
                || receipt_ref.worldline_tick_after != commit.worldline_tick_after
                || receipt_ref.commit_global_tick != commit.commit_global_tick
                || receipt_ref.commit_hash != commit.commit_hash
            {
                return Err(CausalReceiptHistoryObstruction::CursorPositionUnavailable {
                    worldline_tick,
                    after_submission: Some(after),
                });
            }
            if !indexed_exactly(runtime, &receipt_ref) {
                return Err(
                    CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent {
                        receipt_ref: Box::new(receipt_ref),
                    },
                );
            }
            self.records_inspected += 1;
            verify_retained_transition(provenance, &receipt_ref)?;
        }
        let mut last = after;
        for (key, correlation) in
            runtime.receipt_correlations_in_commit(commit, after, newest_first)
        {
            if self.budget_spent() {
                return Ok(TickVisit::Stopped {
                    attributed: false,
                    last,
                });
            }
            self.records_inspected += 1;
            let correlation = correlation
                .filter(|correlation| correlation.causal_receipt_ref == key)
                .ok_or_else(
                    || CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent {
                        receipt_ref: Box::new(key),
                    },
                )?;
            verify_retained_transition(provenance, &key)?;
            self.entries
                .push(history_entry(runtime, provenance, correlation)?);
            last = Some(key.submission_id);
        }
        Ok(TickVisit::Finished { attributed: false })
    }

    /// The correlation for an attributed submission, checked against the index
    /// entry found for it, its tick and retained provenance.
    fn checked_correlation(
        &self,
        worldline_tick: WorldlineTick,
        tick_after: WorldlineTick,
        submission_id: Hash,
        indexed: Option<(CausalTickReceiptRef, Option<&ReceiptCorrelationRecord>)>,
    ) -> Result<&'a ReceiptCorrelationRecord, CausalReceiptHistoryObstruction> {
        let runtime: &'a WorldlineRuntime = self.runtime;
        let Some(correlation) = runtime.receipt_correlation_for_submission(&submission_id) else {
            if let Some((key, _)) = indexed {
                return Err(
                    CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent {
                        receipt_ref: Box::new(key),
                    },
                );
            }
            return Err(if runtime.submission_was_ticketed(&submission_id) {
                CausalReceiptHistoryObstruction::AttributedSubmissionUncorrelated {
                    worldline_tick,
                    submission_id,
                }
            } else {
                CausalReceiptHistoryObstruction::SubmissionWithoutReceiptIdentity {
                    worldline_tick,
                    submission_id,
                }
            });
        };
        let receipt_ref = correlation.causal_receipt_ref;
        let indexed_matches = indexed.is_some_and(|(key, record)| {
            key == receipt_ref
                && record.is_some_and(|record| record.causal_receipt_ref == receipt_ref)
        });
        if !indexed_matches
            || receipt_ref.worldline_id != self.request.worldline_id
            || receipt_ref.worldline_tick_after != tick_after
            || receipt_ref.submission_id != submission_id
        {
            return Err(
                CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent {
                    receipt_ref: Box::new(receipt_ref),
                },
            );
        }
        verify_retained_transition(self.provenance, &receipt_ref)?;
        Ok(correlation)
    }

    fn inconsistent(
        commit: &CausalTickReceiptRef,
        submission_id: Hash,
    ) -> CausalReceiptHistoryObstruction {
        CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent {
            receipt_ref: Box::new(CausalTickReceiptRef {
                submission_id,
                ..*commit
            }),
        }
    }
}

/// Whether the receipt index resolves `receipt_ref` to the correlation for
/// that exact receipt, not to a different one through a stale entry.
#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn indexed_exactly(runtime: &WorldlineRuntime, receipt_ref: &CausalTickReceiptRef) -> bool {
    runtime
        .receipt_correlation_for_receipt_ref(receipt_ref)
        .is_some_and(|correlation| correlation.causal_receipt_ref == *receipt_ref)
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
/// Checks that retained history holds the transition `receipt_ref` names,
/// with the same commit and the same receipt content: a receipt is never
/// presented for a tick that history lost or recorded differently.
fn verify_retained_transition(
    provenance: &ProvenanceService,
    receipt_ref: &CausalTickReceiptRef,
) -> Result<(), CausalReceiptHistoryObstruction> {
    let gap = || CausalReceiptHistoryObstruction::RetentionGap {
        receipt_ref: Box::new(*receipt_ref),
    };
    let tick = receipt_ref
        .worldline_tick_after
        .checked_sub(1)
        .ok_or_else(gap)?;
    let summary = provenance
        .commit_summary(receipt_ref.worldline_id, tick)
        .map_err(|_| gap())?;
    if summary.commit.commit_hash != receipt_ref.commit_hash
        || summary.commit_global_tick != receipt_ref.commit_global_tick
        || summary.tick_receipt_digest != Some(receipt_ref.receipt_content_digest)
    {
        return Err(gap());
    }
    Ok(())
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn history_entry(
    runtime: &WorldlineRuntime,
    provenance: &ProvenanceService,
    correlation: &ReceiptCorrelationRecord,
) -> Result<CausalReceiptHistoryEntry, CausalReceiptHistoryObstruction> {
    let receipt_ref = correlation.causal_receipt_ref;
    let coordinate_matches = receipt_ref.submission_id == correlation.submission_id
        && receipt_ref.ticket_digest == correlation.ticket_digest
        && receipt_ref.worldline_tick_after == correlation.worldline_tick_after
        && receipt_ref.commit_global_tick == correlation.commit_global_tick
        && receipt_ref.commit_hash == correlation.commit_hash
        && correlation.head_key.worldline_id == receipt_ref.worldline_id;
    let receipt_content_matches =
        receipt_ref.receipt_content_digest == correlation.tick_receipt_digest;
    if !coordinate_matches || !receipt_content_matches {
        return Err(
            CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent {
                receipt_ref: Box::new(receipt_ref),
            },
        );
    }

    let submission = runtime
        .witnessed_submission(&correlation.submission_id)
        .ok_or_else(|| CausalReceiptHistoryObstruction::SubmissionUnavailable {
            receipt_ref: Box::new(receipt_ref),
            submission_id: correlation.submission_id,
        })?;
    let envelope = runtime
        .witnessed_submission_envelope(&correlation.submission_id)
        .ok_or_else(|| CausalReceiptHistoryObstruction::SubmissionUnavailable {
            receipt_ref: Box::new(receipt_ref),
            submission_id: correlation.submission_id,
        })?;
    // The submission must be the one that produced this receipt, on this
    // receipt's worldline: same identity, same writer head, and an envelope
    // targeting the receipt's worldline.
    if submission.submission_id != correlation.submission_id
        || submission.ingress_id != correlation.ingress_id
        || envelope.ingress_id() != correlation.ingress_id
        || submission.head_key != correlation.head_key
        || envelope.target().worldline_id() != receipt_ref.worldline_id
    {
        return Err(
            CausalReceiptHistoryObstruction::SubmissionEvidenceInconsistent {
                receipt_ref: Box::new(receipt_ref),
            },
        );
    }

    let parent_count = correlation
        .causal_parent_receipts
        .len()
        .max(envelope.causal_parents().len());
    if parent_count > MAX_CAUSAL_RECEIPT_HISTORY_PARENTS {
        return Err(CausalReceiptHistoryObstruction::CausalParentsExceedBound {
            receipt_ref: Box::new(receipt_ref),
            parent_count,
            max: MAX_CAUSAL_RECEIPT_HISTORY_PARENTS,
        });
    }
    let mut causal_parent_receipts = correlation.causal_parent_receipts.clone();
    causal_parent_receipts.sort_unstable();
    causal_parent_receipts.dedup();
    let mut envelope_parent_receipts = envelope
        .causal_parents()
        .iter()
        .copied()
        .map(crate::IngressCausalParent::receipt_ref)
        .collect::<Vec<_>>();
    envelope_parent_receipts.sort_unstable();
    envelope_parent_receipts.dedup();
    if envelope_parent_receipts != causal_parent_receipts {
        return Err(
            CausalReceiptHistoryObstruction::SubmissionEvidenceInconsistent {
                receipt_ref: Box::new(receipt_ref),
            },
        );
    }
    // A parent counts as available only when retained history still holds
    // its transition; the correlation index alone is not causal authority.
    for parent_receipt_ref in &causal_parent_receipts {
        if !indexed_exactly(runtime, parent_receipt_ref)
            || verify_retained_transition(provenance, parent_receipt_ref).is_err()
        {
            return Err(CausalReceiptHistoryObstruction::CausalParentUnavailable {
                receipt_ref: Box::new(receipt_ref),
                parent_receipt_ref: Box::new(*parent_receipt_ref),
            });
        }
    }

    Ok(CausalReceiptHistoryEntry {
        receipt_ref,
        submission_id: correlation.submission_id,
        contract: correlation.contract.clone(),
        causal_parent_receipts,
        inverse_derivation: recover_contract_inverse_derivation(runtime, &receipt_ref)?,
    })
}

#[cfg(all(test, feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::clock::GlobalTick;
    use crate::{
        make_head_id, HashTriplet, ProvenanceEntry, TickReceipt, TickSubmissionAttribution,
        TickSubmissionDecision, TxId, WorldlineState, WorldlineTickHeaderV1, WorldlineTickPatchV1,
        WriterHeadKey,
    };

    fn worldline() -> WorldlineId {
        WorldlineId::from_bytes([7; 32])
    }

    fn head_key() -> WriterHeadKey {
        WriterHeadKey {
            worldline_id: worldline(),
            head_id: make_head_id("history-test"),
        }
    }

    fn runtime() -> WorldlineRuntime {
        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(worldline(), WorldlineState::empty())
            .expect("worldline registers");
        runtime
    }

    fn submission(seed: u8) -> Hash {
        [seed; 32]
    }

    /// One retained tick of the history fixture.
    struct FixtureTick {
        global_tick: GlobalTick,
        commit_hash: Hash,
        receipt_digest: Hash,
    }

    /// A runtime and provenance whose worldline retains one tick per element
    /// of `ticks`. `Some(seeds)` commits an attribution section naming those
    /// submissions (as no-match decisions, so the receipt needs no entries);
    /// `None` commits a legacy receipt with no section.
    fn history_fixture(
        ticks: &[Option<Vec<u8>>],
    ) -> (WorldlineRuntime, ProvenanceService, Vec<FixtureTick>) {
        let state = WorldlineState::empty();
        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(worldline(), &state)
            .expect("worldline registers");
        let mut retained = Vec::new();
        let mut parent = None;
        for (index, members) in ticks.iter().enumerate() {
            let tick = index as u64;
            let mut receipt = TickReceipt::new(TxId::from_raw(tick + 1), Vec::new(), Vec::new());
            if let Some(seeds) = members {
                let mut records = seeds
                    .iter()
                    .map(|seed| TickSubmissionAttribution {
                        submission_id: submission(*seed),
                        decision: TickSubmissionDecision::NoMatchingRule,
                    })
                    .collect::<Vec<_>>();
                records.sort_by_key(|record| record.submission_id);
                receipt = receipt
                    .with_attribution(records)
                    .expect("fixture attribution");
            }
            let global_tick = GlobalTick::from_raw(tick + 1);
            let commit_hash = *blake3::hash(&tick.to_le_bytes()).as_bytes();
            let patch = WorldlineTickPatchV1 {
                header: WorldlineTickHeaderV1 {
                    commit_global_tick: global_tick,
                    policy_id: 0,
                    rule_pack_id: [0; 32],
                    plan_digest: [0; 32],
                    decision_digest: receipt.digest(),
                    rewrites_digest: [0; 32],
                },
                warp_id: state.root().warp_id,
                ops: Vec::new(),
                in_slots: Vec::new(),
                out_slots: Vec::new(),
                patch_digest: [1; 32],
            };
            let entry = ProvenanceEntry::local_commit(
                worldline(),
                WorldlineTick::from_raw(tick),
                global_tick,
                head_key(),
                parent.into_iter().collect(),
                HashTriplet {
                    state_root: [1; 32],
                    patch_digest: [1; 32],
                    commit_hash,
                },
                patch,
                Vec::new(),
                Vec::new(),
            )
            .with_tick_receipt(receipt.clone());
            provenance
                .append_local_commit(entry)
                .expect("fixture tick appends");
            parent = Some(crate::ProvenanceRef {
                worldline_id: worldline(),
                worldline_tick: WorldlineTick::from_raw(tick),
                commit_hash,
            });
            retained.push(FixtureTick {
                global_tick,
                commit_hash,
                receipt_digest: receipt.digest(),
            });
        }
        let mut runtime = runtime();
        runtime.advance_frontier_for_test(worldline(), ticks.len() as u64);
        (runtime, provenance, retained)
    }

    /// Plants the correlation that names submission `seed` at fixture tick
    /// `tick`, consistent with what provenance retains for that tick.
    fn correlate(runtime: &mut WorldlineRuntime, retained: &[FixtureTick], tick: usize, seed: u8) {
        let fixture = &retained[tick];
        let receipt_ref = CausalTickReceiptRef {
            worldline_id: worldline(),
            worldline_tick_after: WorldlineTick::from_raw(tick as u64 + 1),
            commit_global_tick: fixture.global_tick,
            commit_hash: fixture.commit_hash,
            submission_id: submission(seed),
            ticket_digest: [seed; 32],
            receipt_content_digest: fixture.receipt_digest,
        };
        runtime.plant_receipt_correlation_for_test(record(receipt_ref, seed), true, true);
    }

    fn record(receipt_ref: CausalTickReceiptRef, ticketed_ingress: u8) -> ReceiptCorrelationRecord {
        ReceiptCorrelationRecord {
            ticketed_ingress_id: [ticketed_ingress; 32],
            submission_id: receipt_ref.submission_id,
            ticket_digest: receipt_ref.ticket_digest,
            causal_receipt_ref: receipt_ref,
            ingress_id: [ticketed_ingress; 32],
            head_key: head_key(),
            contract: None,
            commit_global_tick: receipt_ref.commit_global_tick,
            worldline_tick_after: receipt_ref.worldline_tick_after,
            tick_receipt_digest: receipt_ref.receipt_content_digest,
            commit_hash: receipt_ref.commit_hash,
            causal_parent_receipts: Vec::new(),
        }
    }

    fn request(
        basis: u64,
        cursor: Option<CausalReceiptHistoryCursor>,
    ) -> CausalReceiptHistoryRequest {
        CausalReceiptHistoryRequest {
            worldline_id: worldline(),
            basis_frontier_tick: WorldlineTick::from_raw(basis),
            direction: CausalReceiptHistoryDirection::OldestFirst,
            page_size: 8,
            cursor,
        }
    }

    fn cursor_at(basis: u64, tick: u64, after: Option<u8>) -> CausalReceiptHistoryCursor {
        CausalReceiptHistoryCursor {
            worldline_id: worldline(),
            basis_frontier_tick: WorldlineTick::from_raw(basis),
            direction: CausalReceiptHistoryDirection::OldestFirst,
            next_tick: WorldlineTick::from_raw(tick),
            after_submission: after.map(submission),
        }
    }

    #[test]
    fn a_fully_correlated_attributed_tick_passes_the_completeness_checks() {
        let (mut runtime, provenance, retained) = history_fixture(&[Some(vec![1, 2])]);
        correlate(&mut runtime, &retained, 0, 1);
        correlate(&mut runtime, &retained, 0, 2);
        // The planted correlations have no witnessed submission, so the first
        // obstruction comes from building the entry, after every completeness
        // and retention check has passed.
        assert!(matches!(
            causal_receipt_history(&runtime, &provenance, request(1, None)),
            Err(CausalReceiptHistoryObstruction::SubmissionUnavailable { .. })
        ));
    }

    #[test]
    fn a_dropped_correlation_of_a_ticketed_submission_is_detected() {
        let (mut runtime, provenance, retained) = history_fixture(&[Some(vec![1, 2])]);
        correlate(&mut runtime, &retained, 0, 1);
        runtime.mark_submission_ticketed_for_test(submission(2));
        // Submission 1 sorts first and fails on its missing submission record
        // only after submission 2's gap would be reached, so ask for the gap
        // directly by resuming after submission 1.
        assert_eq!(
            causal_receipt_history(
                &runtime,
                &provenance,
                request(1, Some(cursor_at(1, 0, Some(1))))
            ),
            Err(
                CausalReceiptHistoryObstruction::AttributedSubmissionUncorrelated {
                    worldline_tick: WorldlineTick::ZERO,
                    submission_id: submission(2),
                }
            )
        );
    }

    #[test]
    fn a_whole_tick_of_lost_correlations_is_detected() {
        // No correlation of tick 1 survives, so an index scan would never see
        // the tick; the attribution still names its submission.
        let (mut runtime, provenance, retained) =
            history_fixture(&[Some(Vec::new()), Some(vec![3])]);
        let _ = retained;
        runtime.mark_submission_ticketed_for_test(submission(3));
        assert_eq!(
            causal_receipt_history(&runtime, &provenance, request(2, None)),
            Err(
                CausalReceiptHistoryObstruction::AttributedSubmissionUncorrelated {
                    worldline_tick: WorldlineTick::from_raw(1),
                    submission_id: submission(3),
                }
            )
        );
    }

    #[test]
    fn an_attributed_submission_that_was_never_ticketed_has_no_receipt_identity() {
        let (runtime, provenance, _) = history_fixture(&[Some(vec![4])]);
        assert_eq!(
            causal_receipt_history(&runtime, &provenance, request(1, None)),
            Err(
                CausalReceiptHistoryObstruction::SubmissionWithoutReceiptIdentity {
                    worldline_tick: WorldlineTick::ZERO,
                    submission_id: submission(4),
                }
            )
        );
    }

    #[test]
    fn a_correlation_its_tick_does_not_attribute_is_refused() {
        let (mut runtime, provenance, retained) = history_fixture(&[Some(Vec::new())]);
        correlate(&mut runtime, &retained, 0, 5);
        assert!(matches!(
            causal_receipt_history(&runtime, &provenance, request(1, None)),
            Err(CausalReceiptHistoryObstruction::UnattributedCorrelation { .. })
        ));
    }

    #[test]
    fn legacy_ticks_report_that_completeness_is_not_established() {
        let (runtime, provenance, _) = history_fixture(&[None, Some(Vec::new())]);
        let page = causal_receipt_history(&runtime, &provenance, request(2, None)).unwrap();
        assert!(page.entries.is_empty());
        assert_eq!(page.next_cursor, None);
        assert_eq!(
            page.completeness,
            CausalReceiptHistoryCompleteness::UnestablishedForLegacyTicks {
                first_legacy_tick: WorldlineTick::ZERO,
            }
        );
        // The claim is scoped to the inspected ticks.
        assert_eq!(
            page.inspected,
            Some(CausalReceiptHistoryTickRange {
                first: WorldlineTick::ZERO,
                last: WorldlineTick::from_raw(1),
            })
        );

        let (runtime, provenance, _) = history_fixture(&[Some(Vec::new())]);
        assert_eq!(
            causal_receipt_history(&runtime, &provenance, request(1, None))
                .unwrap()
                .completeness,
            CausalReceiptHistoryCompleteness::Established
        );
    }

    #[test]
    fn an_indexed_legacy_receipt_without_its_correlation_is_refused() {
        let (mut runtime, provenance, retained) = history_fixture(&[None]);
        let fixture = &retained[0];
        let receipt_ref = CausalTickReceiptRef {
            worldline_id: worldline(),
            worldline_tick_after: WorldlineTick::from_raw(1),
            commit_global_tick: fixture.global_tick,
            commit_hash: fixture.commit_hash,
            submission_id: submission(6),
            ticket_digest: [6; 32],
            receipt_content_digest: fixture.receipt_digest,
        };
        runtime.plant_receipt_correlation_for_test(record(receipt_ref, 6), true, false);
        // Keep the index sizes equal so the shadow backstop does not fire.
        runtime.plant_receipt_correlation_for_test(
            record(
                CausalTickReceiptRef {
                    worldline_tick_after: WorldlineTick::from_raw(9),
                    ..receipt_ref
                },
                7,
            ),
            false,
            true,
        );
        assert!(matches!(
            causal_receipt_history(&runtime, &provenance, request(1, None)),
            Err(CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent { .. })
        ));
    }

    #[test]
    fn a_long_run_of_empty_ticks_spends_the_tick_budget_and_continues() {
        let ticks = MAX_CAUSAL_RECEIPT_HISTORY_INSPECTED_TICKS + 2;
        let (runtime, provenance, _) =
            history_fixture(&vec![Some(Vec::new()); usize::try_from(ticks).unwrap()]);
        let first = causal_receipt_history(&runtime, &provenance, request(ticks, None)).unwrap();
        assert!(
            first.entries.is_empty(),
            "a page may return nothing and continue"
        );
        let cursor = first.next_cursor.expect("the budget stops the page early");
        assert_eq!(
            cursor.next_tick.as_u64(),
            MAX_CAUSAL_RECEIPT_HISTORY_INSPECTED_TICKS
        );
        let second =
            causal_receipt_history(&runtime, &provenance, request(ticks, Some(cursor))).unwrap();
        assert!(second.entries.is_empty());
        assert_eq!(second.next_cursor, None, "each page makes forward progress");
    }

    #[test]
    fn a_cursor_outside_its_view_is_refused() {
        let (runtime, provenance, _) = history_fixture(&[Some(vec![1])]);
        // Another basis.
        assert_eq!(
            causal_receipt_history(
                &runtime,
                &provenance,
                request(1, Some(cursor_at(0, 0, None)))
            ),
            Err(CausalReceiptHistoryObstruction::CursorScopeMismatch)
        );
        // A tick at or beyond the basis.
        assert_eq!(
            causal_receipt_history(
                &runtime,
                &provenance,
                request(1, Some(cursor_at(1, 1, None)))
            ),
            Err(CausalReceiptHistoryObstruction::CursorPositionUnavailable {
                worldline_tick: WorldlineTick::from_raw(1),
                after_submission: None,
            })
        );
        // A submission the tick does not account for.
        assert_eq!(
            causal_receipt_history(
                &runtime,
                &provenance,
                request(1, Some(cursor_at(1, 0, Some(9))))
            ),
            Err(CausalReceiptHistoryObstruction::CursorPositionUnavailable {
                worldline_tick: WorldlineTick::ZERO,
                after_submission: Some(submission(9)),
            })
        );
    }

    #[test]
    fn a_stale_index_entry_reusing_an_attributed_submission_is_refused() {
        let (mut runtime, provenance, retained) = history_fixture(&[Some(vec![1])]);
        correlate(&mut runtime, &retained, 0, 1);
        // A second index key for submission 1, with another ticket, still
        // resolves to submission 1's proper correlation.
        let fixture = &retained[0];
        let stale = CausalTickReceiptRef {
            worldline_id: worldline(),
            worldline_tick_after: WorldlineTick::from_raw(1),
            commit_global_tick: fixture.global_tick,
            commit_hash: fixture.commit_hash,
            submission_id: submission(1),
            ticket_digest: [9; 32],
            receipt_content_digest: fixture.receipt_digest,
        };
        runtime.plant_receipt_correlation_for_test(record(stale, 1), true, false);
        assert!(matches!(
            causal_receipt_history(&runtime, &provenance, request(1, None)),
            Err(CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent { .. })
        ));
    }

    #[test]
    fn a_cursor_resuming_after_a_lost_correlation_is_refused() {
        let (mut runtime, provenance, retained) = history_fixture(&[Some(vec![1, 2])]);
        // Submission 1 was returned on an earlier page; its correlation has
        // since been lost from the cache.
        runtime.mark_submission_ticketed_for_test(submission(1));
        correlate(&mut runtime, &retained, 0, 2);
        assert_eq!(
            causal_receipt_history(
                &runtime,
                &provenance,
                request(1, Some(cursor_at(1, 0, Some(1))))
            ),
            Err(
                CausalReceiptHistoryObstruction::AttributedSubmissionUncorrelated {
                    worldline_tick: WorldlineTick::ZERO,
                    submission_id: submission(1),
                }
            )
        );
    }

    #[test]
    fn a_legacy_cursor_whose_index_entry_was_lost_is_refused() {
        let (mut runtime, provenance, retained) = history_fixture(&[None]);
        let fixture = &retained[0];
        let resumed = CausalTickReceiptRef {
            worldline_id: worldline(),
            worldline_tick_after: WorldlineTick::from_raw(1),
            commit_global_tick: fixture.global_tick,
            commit_hash: fixture.commit_hash,
            submission_id: submission(1),
            ticket_digest: [1; 32],
            receipt_content_digest: fixture.receipt_digest,
        };
        // The resumed-after submission keeps its correlation record, but its
        // receipt-reference index entry is gone.
        runtime.plant_receipt_correlation_for_test(record(resumed, 1), false, true);
        // Keep the index sizes equal so the shadow backstop does not fire.
        runtime.plant_receipt_correlation_for_test(
            record(
                CausalTickReceiptRef {
                    worldline_tick_after: WorldlineTick::from_raw(9),
                    ..resumed
                },
                9,
            ),
            true,
            false,
        );
        assert!(matches!(
            causal_receipt_history(
                &runtime,
                &provenance,
                request(1, Some(cursor_at(1, 0, Some(1))))
            ),
            Err(CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent { .. })
        ));
    }

    #[test]
    fn a_legacy_cursor_resolving_to_another_submission_is_refused() {
        let (mut runtime, provenance, retained) = history_fixture(&[None]);
        // Submission 2 is retained and indexed; submission 1's index entry
        // now points at submission 2's correlation.
        correlate(&mut runtime, &retained, 0, 2);
        runtime.misdirect_submission_index_for_test(submission(1), [2; 32]);
        assert!(matches!(
            causal_receipt_history(
                &runtime,
                &provenance,
                request(1, Some(cursor_at(1, 0, Some(1))))
            ),
            Err(CausalReceiptHistoryObstruction::CursorPositionUnavailable { .. })
        ));
    }

    /// One retained tick that applied submission 1 through a receipt entry
    /// and recorded submission 2 as no-match, both correlated.
    fn mixed_decision_tick() -> (WorldlineRuntime, ProvenanceService) {
        let state = WorldlineState::empty();
        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(worldline(), &state)
            .expect("worldline registers");
        let mut records = vec![
            TickSubmissionAttribution {
                submission_id: submission(1),
                decision: TickSubmissionDecision::ReceiptEntries(vec![0]),
            },
            TickSubmissionAttribution {
                submission_id: submission(2),
                decision: TickSubmissionDecision::NoMatchingRule,
            },
        ];
        records.sort_by_key(|record| record.submission_id);
        let receipt = TickReceipt::new(
            TxId::from_raw(1),
            vec![crate::TickReceiptEntry {
                rule_id: [5; 32],
                scope_hash: [6; 32],
                scope: crate::NodeKey {
                    warp_id: state.root().warp_id,
                    local_id: crate::NodeId([1; 32]),
                },
                disposition: crate::TickReceiptDisposition::Applied,
            }],
            vec![Vec::new()],
        )
        .with_attribution(records)
        .expect("fixture attribution");
        let global_tick = GlobalTick::from_raw(1);
        let commit_hash = [8; 32];
        let entry = ProvenanceEntry::local_commit(
            worldline(),
            WorldlineTick::ZERO,
            global_tick,
            head_key(),
            Vec::new(),
            HashTriplet {
                state_root: [1; 32],
                patch_digest: [1; 32],
                commit_hash,
            },
            WorldlineTickPatchV1 {
                header: WorldlineTickHeaderV1 {
                    commit_global_tick: global_tick,
                    policy_id: 0,
                    rule_pack_id: [0; 32],
                    plan_digest: [0; 32],
                    decision_digest: receipt.digest(),
                    rewrites_digest: [0; 32],
                },
                warp_id: state.root().warp_id,
                ops: Vec::new(),
                in_slots: Vec::new(),
                out_slots: Vec::new(),
                patch_digest: [1; 32],
            },
            Vec::new(),
            Vec::new(),
        )
        .with_tick_receipt(receipt);
        provenance
            .append_local_commit(entry)
            .expect("fixture tick appends");
        let mut runtime = runtime();
        runtime.advance_frontier_for_test(worldline(), 1);
        for seed in [1, 2] {
            runtime.mark_submission_ticketed_for_test(submission(seed));
        }
        (runtime, provenance)
    }

    #[test]
    fn losing_either_correlation_of_a_mixed_decision_tick_is_detected() {
        for (kept, lost) in [(1, 2), (2, 1)] {
            let (mut runtime, provenance) = mixed_decision_tick();
            let fixture = FixtureTick {
                global_tick: GlobalTick::from_raw(1),
                commit_hash: [8; 32],
                receipt_digest: provenance
                    .retained_tick_receipt(worldline(), WorldlineTick::ZERO)
                    .unwrap()
                    .unwrap()
                    .digest(),
            };
            correlate(&mut runtime, std::slice::from_ref(&fixture), 0, kept);
            // Resume after the kept submission when it sorts first, so its
            // missing submission record does not end the page before the
            // lost one is reached.
            let cursor = (kept < lost).then(|| cursor_at(1, 0, Some(kept)));
            assert_eq!(
                causal_receipt_history(&runtime, &provenance, request(1, cursor)),
                Err(
                    CausalReceiptHistoryObstruction::AttributedSubmissionUncorrelated {
                        worldline_tick: WorldlineTick::ZERO,
                        submission_id: submission(lost),
                    }
                ),
                "losing submission {lost}'s correlation must be detected"
            );
        }
    }

    #[test]
    fn two_correlations_claiming_one_receipt_are_refused() {
        let (mut runtime, provenance, retained) = history_fixture(&[Some(vec![1])]);
        correlate(&mut runtime, &retained, 0, 1);
        let fixture = &retained[0];
        let shared = CausalTickReceiptRef {
            worldline_id: worldline(),
            worldline_tick_after: WorldlineTick::from_raw(1),
            commit_global_tick: fixture.global_tick,
            commit_hash: fixture.commit_hash,
            submission_id: submission(1),
            ticket_digest: [1; 32],
            receipt_content_digest: fixture.receipt_digest,
        };
        // A second correlation for the same receipt shadows the first.
        runtime.plant_receipt_correlation_for_test(record(shared, 2), false, true);
        assert_eq!(
            causal_receipt_history(&runtime, &provenance, request(1, None)),
            Err(CausalReceiptHistoryObstruction::DuplicateReceiptEvidence)
        );
    }

    /// A provenance service holding one committed tick of `worldline()`, and
    /// the receipt reference that names it exactly.
    fn provenance_with_one_tick(retain_receipt: bool) -> (ProvenanceService, CausalTickReceiptRef) {
        let state = WorldlineState::empty();
        let mut provenance = ProvenanceService::new();
        provenance
            .register_worldline(worldline(), &state)
            .expect("worldline registers");
        let receipt = TickReceipt::new(TxId::from_raw(1), Vec::new(), Vec::new());
        let patch = WorldlineTickPatchV1 {
            header: WorldlineTickHeaderV1 {
                commit_global_tick: GlobalTick::from_raw(1),
                policy_id: 0,
                rule_pack_id: [0; 32],
                plan_digest: [0; 32],
                decision_digest: receipt.digest(),
                rewrites_digest: [0; 32],
            },
            warp_id: state.root().warp_id,
            ops: Vec::new(),
            in_slots: Vec::new(),
            out_slots: Vec::new(),
            patch_digest: [1; 32],
        };
        let expected = HashTriplet {
            state_root: [1; 32],
            patch_digest: [1; 32],
            commit_hash: [3; 32],
        };
        let mut entry = ProvenanceEntry::local_commit(
            worldline(),
            WorldlineTick::ZERO,
            GlobalTick::from_raw(1),
            WriterHeadKey {
                worldline_id: worldline(),
                head_id: make_head_id("history-test"),
            },
            Vec::new(),
            expected,
            patch,
            Vec::new(),
            Vec::new(),
        );
        if retain_receipt {
            entry = entry.with_tick_receipt(receipt.clone());
        }
        provenance
            .append_local_commit(entry)
            .expect("committed tick appends");
        let mut receipt_ref = CausalTickReceiptRef {
            worldline_id: worldline(),
            worldline_tick_after: WorldlineTick::ZERO,
            commit_global_tick: GlobalTick::ZERO,
            commit_hash: [1; 32],
            submission_id: [1; 32],
            ticket_digest: [1; 32],
            receipt_content_digest: [1; 32],
        };
        receipt_ref.worldline_tick_after = WorldlineTick::from_raw(1);
        receipt_ref.commit_global_tick = GlobalTick::from_raw(1);
        receipt_ref.commit_hash = [3; 32];
        receipt_ref.receipt_content_digest = receipt.digest();
        (provenance, receipt_ref)
    }

    #[test]
    fn a_receipt_matching_its_retained_transition_is_accepted() {
        let (provenance, receipt_ref) = provenance_with_one_tick(true);
        assert_eq!(
            verify_retained_transition(&provenance, &receipt_ref),
            Ok(())
        );
    }

    #[test]
    fn a_receipt_whose_content_differs_from_the_retained_receipt_is_a_retention_gap() {
        let (provenance, mut receipt_ref) = provenance_with_one_tick(true);
        receipt_ref.receipt_content_digest = [9; 32];
        assert!(matches!(
            verify_retained_transition(&provenance, &receipt_ref),
            Err(CausalReceiptHistoryObstruction::RetentionGap { .. })
        ));
    }

    #[test]
    fn a_transition_retained_without_its_receipt_is_a_retention_gap() {
        let (provenance, receipt_ref) = provenance_with_one_tick(false);
        assert!(matches!(
            verify_retained_transition(&provenance, &receipt_ref),
            Err(CausalReceiptHistoryObstruction::RetentionGap { .. })
        ));
    }

    #[test]
    fn a_receipt_naming_a_different_commit_is_a_retention_gap() {
        let (provenance, mut receipt_ref) = provenance_with_one_tick(true);
        receipt_ref.commit_hash = [4; 32];
        assert!(matches!(
            verify_retained_transition(&provenance, &receipt_ref),
            Err(CausalReceiptHistoryObstruction::RetentionGap { .. })
        ));
    }
}
