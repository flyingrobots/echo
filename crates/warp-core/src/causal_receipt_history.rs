// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded application-facing projection of retained causal receipt history.

use thiserror::Error;

use crate::{
    contract_inverse::recover_contract_inverse_derivation, CausalTickReceiptRef,
    ContractEvidenceIdentity, ContractInverseDerivation, ContractInverseHistoryObstruction, Hash,
    ProvenanceService, ProvenanceStore, ReceiptCorrelationRecord, WorldlineId, WorldlineRuntime,
    WorldlineTick,
};

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
    pub contract: Option<ContractEvidenceIdentity>,
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
    /// More than one retained correlation claims the same exact receipt coordinate.
    #[error("causal receipt history has duplicate evidence: {receipt_ref:?}")]
    DuplicateReceiptEvidence {
        /// Exact receipt claimed by multiple correlations.
        receipt_ref: Box<CausalTickReceiptRef>,
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
    }

    let mut correlations = runtime
        .receipt_correlations()
        .filter(|correlation| {
            correlation.causal_receipt_ref.worldline_id == request.worldline_id
                && correlation.causal_receipt_ref.worldline_tick_after
                    <= request.basis_frontier_tick
        })
        .collect::<Vec<_>>();
    correlations.sort_unstable_by_key(|correlation| correlation.causal_receipt_ref);
    if let Some(duplicate) = correlations.windows(2).find_map(|pair| {
        (pair[0].causal_receipt_ref == pair[1].causal_receipt_ref)
            .then_some(pair[0].causal_receipt_ref)
    }) {
        return Err(CausalReceiptHistoryObstruction::DuplicateReceiptEvidence {
            receipt_ref: Box::new(duplicate),
        });
    }
    if request.direction == CausalReceiptHistoryDirection::NewestFirst {
        correlations.reverse();
    }

    let start = if let Some(cursor) = &request.cursor {
        correlations
            .iter()
            .position(|correlation| correlation.causal_receipt_ref == cursor.last_receipt_ref)
            .map(|index| index + 1)
            .ok_or_else(
                || CausalReceiptHistoryObstruction::CursorReceiptUnavailable {
                    cursor_receipt_ref: Box::new(cursor.last_receipt_ref),
                },
            )?
    } else {
        0
    };
    let available = &correlations[start..];
    let has_more = available.len() > request.page_size;
    let selected = available.iter().take(request.page_size);
    let mut entries = Vec::with_capacity(request.page_size.min(available.len()));
    for correlation in selected {
        entries.push(history_entry(runtime, correlation)?);
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

fn history_entry(
    runtime: &WorldlineRuntime,
    correlation: &ReceiptCorrelationRecord,
) -> Result<CausalReceiptHistoryEntry, CausalReceiptHistoryObstruction> {
    let receipt_ref = correlation.causal_receipt_ref;
    let coordinate_matches = receipt_ref.submission_id == correlation.submission_id
        && receipt_ref.ticket_digest == correlation.ticket_digest
        && receipt_ref.worldline_tick_after == correlation.worldline_tick_after
        && receipt_ref.commit_global_tick == correlation.commit_global_tick
        && receipt_ref.commit_hash == correlation.commit_hash;
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
    if submission.submission_id != correlation.submission_id
        || submission.ingress_id != correlation.ingress_id
        || envelope.ingress_id() != correlation.ingress_id
    {
        return Err(
            CausalReceiptHistoryObstruction::SubmissionEvidenceInconsistent {
                receipt_ref: Box::new(receipt_ref),
            },
        );
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
    for parent_receipt_ref in &causal_parent_receipts {
        if runtime
            .receipt_correlation_for_receipt_ref(parent_receipt_ref)
            .is_none()
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
