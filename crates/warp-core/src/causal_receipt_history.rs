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
/// Cursors are issued by Echo and bind the worldline, basis, direction, and
/// exact last receipt. A cursor cannot be reused for a different history view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalReceiptHistoryCursor {
    worldline_id: WorldlineId,
    basis_frontier_tick: WorldlineTick,
    direction: CausalReceiptHistoryDirection,
    last_receipt_ref: CausalTickReceiptRef,
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
    /// Continuation cursor when more entries remain at the same basis.
    pub next_cursor: Option<CausalReceiptHistoryCursor>,
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
    /// The continuation cursor names receipt evidence unavailable at the selected basis.
    #[error("causal receipt history cursor receipt is unavailable: {cursor_receipt_ref:?}")]
    CursorReceiptUnavailable {
        /// Exact cursor receipt that could not be resumed.
        cursor_receipt_ref: Box<CausalTickReceiptRef>,
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
            .entry(request.worldline_id, evidence_tick)
            .map_err(
                |_| CausalReceiptHistoryObstruction::BasisEvidenceUnavailable {
                    worldline_id: request.worldline_id,
                    basis_frontier_tick: request.basis_frontier_tick,
                },
            )?;
    }

    if let Some(cursor) = &request.cursor {
        if cursor.worldline_id != request.worldline_id
            || cursor.basis_frontier_tick != request.basis_frontier_tick
            || cursor.direction != request.direction
        {
            return Err(CausalReceiptHistoryObstruction::CursorScopeMismatch);
        }
        // The cursor's receipt must still be retained on this worldline at or
        // before the basis; otherwise the continuation point is gone.
        if cursor.last_receipt_ref.worldline_id != request.worldline_id
            || cursor.last_receipt_ref.worldline_tick_after > request.basis_frontier_tick
            || !indexed_exactly(runtime, &cursor.last_receipt_ref)
        {
            return Err(CausalReceiptHistoryObstruction::CursorReceiptUnavailable {
                cursor_receipt_ref: Box::new(cursor.last_receipt_ref),
            });
        }
        // The scan starts after the cursor and never examines it, so its
        // transition is checked here: a lost continuation point is a gap
        // between pages.
        verify_retained_transition(provenance, &cursor.last_receipt_ref)?;
    }
    // Both writers of the correlation indexes refuse a receipt that is already
    // indexed, so one correlation per receipt is a write-time invariant and a
    // shadowed correlation cannot arise from the runtime's own operations.
    // This O(1) count comparison is a backstop for corrupt state only: a
    // shadowed record paired with an unrelated dangling entry cancels out.
    // (A dangling entry on this page is reported by the scan below, as
    // ReceiptEvidenceInconsistent.)
    if runtime.receipt_correlation_index_has_shadowed_records() {
        return Err(CausalReceiptHistoryObstruction::DuplicateReceiptEvidence);
    }

    // A range scan from the cursor: one page costs O(log n + page size).
    let window = runtime.receipt_correlations_through_basis(
        request.worldline_id,
        request.basis_frontier_tick,
        request
            .cursor
            .as_ref()
            .map(|cursor| cursor.last_receipt_ref),
        request.direction == CausalReceiptHistoryDirection::NewestFirst,
    );
    let mut entries = Vec::with_capacity(request.page_size);
    let mut has_more = false;
    for (receipt_ref, correlation) in window {
        if entries.len() == request.page_size {
            has_more = true;
            break;
        }
        // The index entry must resolve to the correlation for this exact
        // receipt; a stale entry resolving to another one is inconsistent.
        let correlation = correlation
            .filter(|correlation| correlation.causal_receipt_ref == receipt_ref)
            .ok_or_else(
                || CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent {
                    receipt_ref: Box::new(receipt_ref),
                },
            )?;
        verify_retained_transition(provenance, &receipt_ref)?;
        entries.push(history_entry(runtime, provenance, correlation)?);
    }
    let next_cursor =
        entries
            .last()
            .filter(|_| has_more)
            .map(|last_entry| CausalReceiptHistoryCursor {
                worldline_id: request.worldline_id,
                basis_frontier_tick: request.basis_frontier_tick,
                direction: request.direction,
                last_receipt_ref: last_entry.receipt_ref,
            });

    Ok(CausalReceiptHistoryPage {
        entries,
        next_cursor,
    })
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
    let entry = provenance
        .entry(receipt_ref.worldline_id, tick)
        .map_err(|_| gap())?;
    if entry.expected.commit_hash != receipt_ref.commit_hash
        || entry.commit_global_tick != receipt_ref.commit_global_tick
        || entry.tick_receipt.as_ref().map(crate::TickReceipt::digest)
            != Some(receipt_ref.receipt_content_digest)
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
        make_head_id, HashTriplet, ProvenanceEntry, TickReceipt, TxId, WorldlineState,
        WorldlineTickHeaderV1, WorldlineTickPatchV1, WriterHeadKey,
    };

    fn worldline() -> WorldlineId {
        WorldlineId::from_bytes([7; 32])
    }

    fn runtime() -> WorldlineRuntime {
        let mut runtime = WorldlineRuntime::new();
        runtime
            .register_worldline(worldline(), WorldlineState::empty())
            .expect("worldline registers");
        runtime
    }

    fn receipt_ref(seed: u8) -> CausalTickReceiptRef {
        CausalTickReceiptRef {
            worldline_id: worldline(),
            worldline_tick_after: WorldlineTick::ZERO,
            commit_global_tick: GlobalTick::ZERO,
            commit_hash: [seed; 32],
            submission_id: [seed; 32],
            ticket_digest: [seed; 32],
            receipt_content_digest: [seed; 32],
        }
    }

    fn record(receipt_ref: CausalTickReceiptRef, ticketed_ingress: u8) -> ReceiptCorrelationRecord {
        ReceiptCorrelationRecord {
            ticketed_ingress_id: [ticketed_ingress; 32],
            submission_id: receipt_ref.submission_id,
            ticket_digest: receipt_ref.ticket_digest,
            causal_receipt_ref: receipt_ref,
            ingress_id: [ticketed_ingress; 32],
            head_key: WriterHeadKey {
                worldline_id: worldline(),
                head_id: make_head_id("history-test"),
            },
            contract: None,
            commit_global_tick: receipt_ref.commit_global_tick,
            worldline_tick_after: receipt_ref.worldline_tick_after,
            tick_receipt_digest: receipt_ref.receipt_content_digest,
            commit_hash: receipt_ref.commit_hash,
            causal_parent_receipts: Vec::new(),
        }
    }

    fn request(cursor: Option<CausalReceiptHistoryCursor>) -> CausalReceiptHistoryRequest {
        CausalReceiptHistoryRequest {
            worldline_id: worldline(),
            basis_frontier_tick: WorldlineTick::ZERO,
            direction: CausalReceiptHistoryDirection::OldestFirst,
            page_size: 8,
            cursor,
        }
    }

    fn cursor(last_receipt_ref: CausalTickReceiptRef) -> CausalReceiptHistoryCursor {
        CausalReceiptHistoryCursor {
            worldline_id: worldline(),
            basis_frontier_tick: WorldlineTick::ZERO,
            direction: CausalReceiptHistoryDirection::OldestFirst,
            last_receipt_ref,
        }
    }

    #[test]
    fn a_cursor_for_a_receipt_no_longer_retained_is_refused() {
        let result = causal_receipt_history(
            &runtime(),
            &ProvenanceService::new(),
            request(Some(cursor(receipt_ref(1)))),
        );
        assert!(matches!(
            result,
            Err(CausalReceiptHistoryObstruction::CursorReceiptUnavailable { .. })
        ));
    }

    #[test]
    fn a_cursor_whose_receipt_is_on_another_worldline_is_refused() {
        let mut foreign = receipt_ref(1);
        foreign.worldline_id = WorldlineId::from_bytes([9; 32]);
        let result = causal_receipt_history(
            &runtime(),
            &ProvenanceService::new(),
            request(Some(cursor(foreign))),
        );
        assert!(matches!(
            result,
            Err(CausalReceiptHistoryObstruction::CursorReceiptUnavailable { .. })
        ));
    }

    #[test]
    fn two_correlations_claiming_one_receipt_are_refused() {
        let mut runtime = runtime();
        let shared = receipt_ref(1);
        runtime.plant_receipt_correlation_for_test(record(shared, 1), true, true);
        // A second correlation for the same receipt shadows the first.
        runtime.plant_receipt_correlation_for_test(record(shared, 2), false, true);
        let result = causal_receipt_history(&runtime, &ProvenanceService::new(), request(None));
        assert!(matches!(
            result,
            Err(CausalReceiptHistoryObstruction::DuplicateReceiptEvidence)
        ));
    }

    #[test]
    fn an_indexed_receipt_without_its_correlation_is_refused() {
        let mut runtime = runtime();
        // An index entry whose correlation record is gone: a dangling entry,
        // not a duplicate.
        runtime.plant_receipt_correlation_for_test(record(receipt_ref(1), 1), true, false);
        let result = causal_receipt_history(&runtime, &ProvenanceService::new(), request(None));
        assert!(matches!(
            result,
            Err(CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent { .. })
        ));
    }

    #[test]
    fn a_receipt_whose_transition_history_does_not_hold_is_a_retention_gap() {
        let mut runtime = runtime();
        runtime.plant_receipt_correlation_for_test(record(receipt_ref(1), 1), true, true);
        // Provenance holds no transition for this receipt, so the page must not
        // silently omit or present it.
        let result = causal_receipt_history(&runtime, &ProvenanceService::new(), request(None));
        assert!(matches!(
            result,
            Err(CausalReceiptHistoryObstruction::RetentionGap { .. })
        ));
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
        let mut receipt_ref = receipt_ref(1);
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

    #[test]
    fn a_cursor_whose_transition_history_lost_is_refused() {
        let mut runtime = runtime();
        let lost = receipt_ref(1);
        // The cursor's correlation is still indexed, but provenance holds no
        // transition for it, so continuing after it would skip a gap.
        runtime.plant_receipt_correlation_for_test(record(lost, 1), true, true);
        let result = causal_receipt_history(
            &runtime,
            &ProvenanceService::new(),
            request(Some(cursor(lost))),
        );
        assert!(matches!(
            result,
            Err(CausalReceiptHistoryObstruction::RetentionGap { .. })
        ));
    }

    #[test]
    fn a_stale_index_entry_resolving_to_another_receipt_is_refused() {
        let mut runtime = runtime();
        let mut elsewhere = receipt_ref(2);
        elsewhere.worldline_id = WorldlineId::from_bytes([9; 32]);
        runtime.plant_receipt_correlation_for_test(record(elsewhere, 2), true, true);
        // A stale entry for receipt 1 points at the other worldline's record.
        let mut stale = record(receipt_ref(1), 2);
        stale.ticketed_ingress_id = [2; 32];
        runtime.plant_receipt_correlation_for_test(stale, true, false);
        let result = causal_receipt_history(&runtime, &ProvenanceService::new(), request(None));
        assert!(matches!(
            result,
            Err(CausalReceiptHistoryObstruction::ReceiptEvidenceInconsistent { .. })
        ));
    }

    #[test]
    fn a_cursor_whose_index_entry_resolves_to_another_receipt_is_refused() {
        let mut runtime = runtime();
        runtime.plant_receipt_correlation_for_test(record(receipt_ref(2), 2), true, true);
        let mut stale = record(receipt_ref(1), 2);
        stale.ticketed_ingress_id = [2; 32];
        runtime.plant_receipt_correlation_for_test(stale, true, false);
        let result = causal_receipt_history(
            &runtime,
            &ProvenanceService::new(),
            request(Some(cursor(receipt_ref(1)))),
        );
        assert!(matches!(
            result,
            Err(CausalReceiptHistoryObstruction::CursorReceiptUnavailable { .. })
        ));
    }
}
