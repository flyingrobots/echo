// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shape-only witnessed suffix admission vocabulary.
//!
//! This module names the first admission shell skeleton. It deliberately carries
//! compact provenance and basis evidence only; it does not implement transport,
//! remote synchronization, or import execution.

use blake3::Hasher;
use echo_wasm_abi::{encode_cbor, kernel_port as abi};

use crate::attachment::{AttachmentOwner, AttachmentPlane};
use crate::clock::WorldlineTick;
use crate::ident::Hash;
use crate::provenance_store::ProvenanceRef;
use crate::settlement::ConflictReason;
use crate::strand::{
    BaseRef, StrandBasisReport, StrandOverlapRevalidation, StrandRevalidationState,
};
use crate::tick_patch::SlotId;
use crate::worldline::WorldlineId;
use crate::ReadingResidualPosture;

/// Compact shell for judging a witnessed suffix without transport or sync.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessedSuffixShell {
    /// Worldline carrying the proposed suffix.
    pub source_worldline_id: WorldlineId,
    /// First source tick included in the proposed suffix.
    pub source_suffix_start_tick: WorldlineTick,
    /// Last source tick included in the proposed suffix, if any.
    pub source_suffix_end_tick: Option<WorldlineTick>,
    /// Ordered source provenance coordinates covered by the shell.
    pub source_entries: Vec<ProvenanceRef>,
    /// Boundary witness used when the shell has no importable entries yet.
    pub boundary_witness: Option<ProvenanceRef>,
    /// Deterministic digest identifying the compact shell evidence.
    pub witness_digest: Hash,
    /// Optional basis-relative settlement evidence reused by the shell.
    pub basis_report: Option<StrandBasisReport>,
}

impl WitnessedSuffixShell {
    /// Converts the shell into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::WitnessedSuffixShell {
        abi::WitnessedSuffixShell {
            source_worldline_id: worldline_id_to_abi(self.source_worldline_id),
            source_suffix_start_tick: worldline_tick_to_abi(self.source_suffix_start_tick),
            source_suffix_end_tick: self.source_suffix_end_tick.map(worldline_tick_to_abi),
            source_entries: self
                .source_entries
                .iter()
                .copied()
                .map(provenance_ref_to_abi)
                .collect(),
            boundary_witness: self.boundary_witness.map(provenance_ref_to_abi),
            witness_digest: self.witness_digest.to_vec(),
            basis_report: self
                .basis_report
                .as_ref()
                .map(settlement_basis_report_to_abi),
        }
    }
}

/// Request to judge a witnessed suffix against a target basis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessedSuffixAdmissionRequest {
    /// Source suffix and compact witness material.
    pub source_suffix: WitnessedSuffixShell,
    /// Worldline receiving the proposed admission.
    pub target_worldline_id: WorldlineId,
    /// Target basis used while judging admission.
    pub target_basis: ProvenanceRef,
    /// Optional target-basis evidence for strand/parent realization cases.
    pub basis_report: Option<StrandBasisReport>,
}

impl WitnessedSuffixAdmissionRequest {
    /// Converts the request into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::WitnessedSuffixAdmissionRequest {
        abi::WitnessedSuffixAdmissionRequest {
            source_suffix: self.source_suffix.to_abi(),
            target_worldline_id: worldline_id_to_abi(self.target_worldline_id),
            target_basis: provenance_ref_to_abi(self.target_basis),
            basis_report: self
                .basis_report
                .as_ref()
                .map(settlement_basis_report_to_abi),
        }
    }
}

/// Response to one witnessed suffix admission request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessedSuffixAdmissionResponse {
    /// Deterministic digest of the source shell being judged.
    pub source_shell_digest: Hash,
    /// Resolved target basis used for the response.
    pub target_basis: ProvenanceRef,
    /// Exactly one top-level admission outcome.
    pub outcome: WitnessedSuffixAdmissionOutcome,
}

impl WitnessedSuffixAdmissionResponse {
    /// Converts the response into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::WitnessedSuffixAdmissionResponse {
        abi::WitnessedSuffixAdmissionResponse {
            source_shell_digest: self.source_shell_digest.to_vec(),
            target_basis: provenance_ref_to_abi(self.target_basis),
            outcome: witnessed_suffix_outcome_to_abi(&self.outcome),
        }
    }
}

/// Read-only local evidence source used by witnessed suffix admission evaluation.
///
/// This trait is intentionally narrow: it does not expose runtime mutation,
/// transport, peer identity, sync loops, or raw patch streams.
pub trait WitnessedSuffixAdmissionContext {
    /// Returns the locally computed digest for a source shell, when available.
    fn source_shell_digest(&self, shell: &WitnessedSuffixShell) -> Option<Hash>;

    /// Returns deterministic local obstruction evidence when shell identity is unavailable.
    fn source_shell_obstruction_digest(&self, shell: &WitnessedSuffixShell) -> Hash {
        source_shell_obstruction_digest(shell)
    }

    /// Resolves a target basis against local evidence, when available.
    fn resolve_target_basis(&self, target_basis: ProvenanceRef) -> Option<ProvenanceRef>;

    /// Reports the local admission posture for a well-formed request.
    fn local_admission_posture(
        &self,
        request: &WitnessedSuffixAdmissionRequest,
    ) -> WitnessedSuffixLocalAdmissionPosture;
}

/// Local posture reported by the read-only admission context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WitnessedSuffixLocalAdmissionPosture {
    /// Local evidence says the suffix is admissible.
    Admissible {
        /// Target-local provenance coordinates produced or expected by admission.
        admitted_refs: Vec<ProvenanceRef>,
    },
    /// Local evidence says the suffix should be retained for later judgment.
    Staged {
        /// Source or target coordinates retained while staged.
        staged_refs: Vec<ProvenanceRef>,
    },
    /// Local evidence preserves lawful plurality.
    Plural {
        /// Candidate coordinates that remain lawful plural outcomes.
        candidate_refs: Vec<ProvenanceRef>,
    },
    /// Local evidence reports deterministic adverse admission law.
    Conflict {
        /// Deterministic reason the suffix conflicts.
        reason: ConflictReason,
        /// Source coordinate implicated in the conflict.
        source_ref: ProvenanceRef,
        /// Deterministic digest of compact conflict evidence.
        evidence_digest: Hash,
        /// Optional overlap revalidation evidence when footprint overlap caused the conflict.
        overlap_revalidation: Option<StrandOverlapRevalidation>,
    },
}

/// Evaluates one witnessed suffix admission request against local evidence.
///
/// Performs deterministic local validation before returning the classified
/// posture reported by the read-only context.
#[must_use]
pub fn evaluate_witnessed_suffix_admission(
    request: &WitnessedSuffixAdmissionRequest,
    context: &impl WitnessedSuffixAdmissionContext,
) -> WitnessedSuffixAdmissionResponse {
    let source_shell_digest = context.source_shell_digest(&request.source_suffix);
    let target_basis = context.resolve_target_basis(request.target_basis);
    let response_source_shell_digest = source_shell_digest
        .unwrap_or_else(|| context.source_shell_obstruction_digest(&request.source_suffix));
    let response_target_basis = target_basis.unwrap_or(request.target_basis);
    let source_entry_obstruction_ref = source_entry_obstruction_ref(&request.source_suffix);

    if source_shell_digest != Some(request.source_suffix.witness_digest)
        || target_basis.is_none()
        || suffix_bounds_are_inconsistent(&request.source_suffix)
        || source_entry_obstruction_ref.is_some()
        || suffix_witness_material_is_missing(&request.source_suffix)
        || basis_report_is_inconsistent(request, response_target_basis)
    {
        return obstructed_response(
            request,
            response_source_shell_digest,
            response_target_basis,
            source_entry_obstruction_ref,
        );
    }

    let normalized_request = WitnessedSuffixAdmissionRequest {
        target_basis: response_target_basis,
        ..request.clone()
    };

    let outcome = match context.local_admission_posture(&normalized_request) {
        WitnessedSuffixLocalAdmissionPosture::Admissible { admitted_refs } => {
            WitnessedSuffixAdmissionOutcome::Admitted {
                target_worldline_id: request.target_worldline_id,
                admitted_refs: canonical_provenance_refs(admitted_refs),
                basis_report: response_basis_report(request),
            }
        }
        WitnessedSuffixLocalAdmissionPosture::Staged { staged_refs } => {
            WitnessedSuffixAdmissionOutcome::Staged {
                staged_refs: canonical_provenance_refs(staged_refs),
                basis_report: response_basis_report(request),
            }
        }
        WitnessedSuffixLocalAdmissionPosture::Plural { candidate_refs } => {
            WitnessedSuffixAdmissionOutcome::Plural {
                candidate_refs: canonical_provenance_refs(candidate_refs),
                residual_posture: ReadingResidualPosture::PluralityPreserved,
                basis_report: response_basis_report(request),
            }
        }
        WitnessedSuffixLocalAdmissionPosture::Conflict {
            reason,
            source_ref,
            evidence_digest,
            overlap_revalidation,
        } => WitnessedSuffixAdmissionOutcome::Conflict {
            reason,
            source_ref,
            evidence_digest,
            overlap_revalidation,
        },
    };

    WitnessedSuffixAdmissionResponse {
        source_shell_digest: response_source_shell_digest,
        target_basis: response_target_basis,
        outcome,
    }
}

/// Top-level witnessed suffix admission posture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WitnessedSuffixAdmissionOutcome {
    /// The suffix is admissible on the named target basis.
    Admitted {
        /// Target worldline receiving the admissible suffix.
        target_worldline_id: WorldlineId,
        /// Target-local provenance coordinates produced or expected by admission.
        admitted_refs: Vec<ProvenanceRef>,
        /// Basis evidence used to classify the suffix as admitted.
        basis_report: Option<StrandBasisReport>,
    },
    /// The suffix is well-formed but retained for later judgment.
    Staged {
        /// Source or target coordinates retained while staged.
        staged_refs: Vec<ProvenanceRef>,
        /// Basis evidence used to classify the suffix as staged.
        basis_report: Option<StrandBasisReport>,
    },
    /// The suffix preserves lawful plurality instead of one admitted result.
    Plural {
        /// Candidate coordinates that remain lawful plural outcomes.
        candidate_refs: Vec<ProvenanceRef>,
        /// Read-side residual posture associated with preserved plurality.
        residual_posture: ReadingResidualPosture,
        /// Basis evidence used to classify the suffix as plural.
        basis_report: Option<StrandBasisReport>,
    },
    /// The suffix conflicts with the target basis under current admission law.
    Conflict {
        /// Deterministic reason the suffix conflicts.
        reason: ConflictReason,
        /// Source coordinate implicated in the conflict.
        source_ref: ProvenanceRef,
        /// Deterministic digest of compact conflict evidence.
        evidence_digest: Hash,
        /// Optional overlap revalidation evidence when footprint overlap caused the conflict.
        overlap_revalidation: Option<StrandOverlapRevalidation>,
    },
    /// The suffix cannot currently be judged or admitted.
    Obstructed {
        /// Source coordinate implicated in the obstruction.
        source_ref: ProvenanceRef,
        /// Read-side residual posture associated with the obstruction.
        residual_posture: ReadingResidualPosture,
        /// Deterministic digest of compact obstruction evidence.
        evidence_digest: Hash,
    },
}

fn obstruction_source_ref(
    request: &WitnessedSuffixAdmissionRequest,
    source_entry_obstruction_ref: Option<ProvenanceRef>,
) -> ProvenanceRef {
    if let Some(source_ref) = source_entry_obstruction_ref {
        return source_ref;
    }

    request
        .source_suffix
        .boundary_witness
        .or_else(|| request.source_suffix.source_entries.first().copied())
        .unwrap_or(request.target_basis)
}

fn suffix_bounds_are_inconsistent(shell: &WitnessedSuffixShell) -> bool {
    shell
        .source_suffix_end_tick
        .is_some_and(|end_tick| end_tick.as_u64() < shell.source_suffix_start_tick.as_u64())
}

fn source_entry_obstruction_ref(shell: &WitnessedSuffixShell) -> Option<ProvenanceRef> {
    suffix_entry_outside_bounds(shell)
        .or_else(|| source_entry_from_foreign_worldline(shell))
        .or_else(|| non_canonical_source_entry(shell))
}

fn suffix_entry_outside_bounds(shell: &WitnessedSuffixShell) -> Option<ProvenanceRef> {
    let start_tick = shell.source_suffix_start_tick.as_u64();
    let end_tick = shell.source_suffix_end_tick.map(WorldlineTick::as_u64);

    shell.source_entries.iter().copied().find(|source_entry| {
        let entry_tick = source_entry.worldline_tick.as_u64();
        entry_tick < start_tick || end_tick.is_some_and(|end_tick| entry_tick > end_tick)
    })
}

fn source_entry_from_foreign_worldline(shell: &WitnessedSuffixShell) -> Option<ProvenanceRef> {
    shell
        .source_entries
        .iter()
        .copied()
        .find(|source_entry| source_entry.worldline_id != shell.source_worldline_id)
}

fn non_canonical_source_entry(shell: &WitnessedSuffixShell) -> Option<ProvenanceRef> {
    shell
        .source_entries
        .windows(2)
        .find_map(|pair| (pair[0] >= pair[1]).then_some(pair[1]))
}

fn suffix_witness_material_is_missing(shell: &WitnessedSuffixShell) -> bool {
    shell.source_entries.is_empty() && shell.boundary_witness.is_none()
}

fn basis_report_is_inconsistent(
    request: &WitnessedSuffixAdmissionRequest,
    target_basis: ProvenanceRef,
) -> bool {
    response_basis_report(request)
        .as_ref()
        .is_some_and(|report| report.realized_parent_ref != target_basis)
}

fn response_basis_report(request: &WitnessedSuffixAdmissionRequest) -> Option<StrandBasisReport> {
    request
        .basis_report
        .clone()
        .or_else(|| request.source_suffix.basis_report.clone())
}

fn canonical_provenance_refs(mut refs: Vec<ProvenanceRef>) -> Vec<ProvenanceRef> {
    refs.sort_unstable();
    refs
}

fn obstructed_response(
    request: &WitnessedSuffixAdmissionRequest,
    source_shell_digest: Hash,
    target_basis: ProvenanceRef,
    source_entry_obstruction_ref: Option<ProvenanceRef>,
) -> WitnessedSuffixAdmissionResponse {
    WitnessedSuffixAdmissionResponse {
        source_shell_digest,
        target_basis,
        outcome: WitnessedSuffixAdmissionOutcome::Obstructed {
            source_ref: obstruction_source_ref(request, source_entry_obstruction_ref),
            residual_posture: ReadingResidualPosture::Obstructed,
            evidence_digest: source_shell_digest,
        },
    }
}

fn source_shell_obstruction_digest(shell: &WitnessedSuffixShell) -> Hash {
    let mut shell_without_claim = shell.to_abi();
    shell_without_claim.witness_digest.clear();

    let mut hasher = Hasher::new();
    hasher.update(b"echo:witnessed-suffix-obstruction:v1\0");
    match encode_cbor(&shell_without_claim) {
        Ok(encoded_shell) => {
            hasher.update(&encoded_shell);
        }
        Err(_) => hash_source_shell_obstruction_fallback(&mut hasher, shell),
    }
    hasher.finalize().into()
}

fn hash_source_shell_obstruction_fallback(hasher: &mut Hasher, shell: &WitnessedSuffixShell) {
    hasher.update(shell.source_worldline_id.as_bytes());
    hasher.update(&shell.source_suffix_start_tick.as_u64().to_le_bytes());
    match shell.source_suffix_end_tick {
        Some(end_tick) => {
            hasher.update(&[1]);
            hasher.update(&end_tick.as_u64().to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    hasher.update(&len_to_u64(shell.source_entries.len()).to_le_bytes());
    for source_entry in &shell.source_entries {
        hash_provenance_ref(hasher, source_entry);
    }
    match shell.boundary_witness {
        Some(boundary_witness) => {
            hasher.update(&[1]);
            hash_provenance_ref(hasher, &boundary_witness);
        }
        None => {
            hasher.update(&[0]);
        }
    }
    hasher.update(&[u8::from(shell.basis_report.is_some())]);
}

fn hash_provenance_ref(hasher: &mut Hasher, reference: &ProvenanceRef) {
    hasher.update(reference.worldline_id.as_bytes());
    hasher.update(&reference.worldline_tick.as_u64().to_le_bytes());
    hasher.update(&reference.commit_hash);
}

fn witnessed_suffix_outcome_to_abi(
    outcome: &WitnessedSuffixAdmissionOutcome,
) -> abi::WitnessedSuffixAdmissionOutcome {
    match outcome {
        WitnessedSuffixAdmissionOutcome::Admitted {
            target_worldline_id,
            admitted_refs,
            basis_report,
        } => abi::WitnessedSuffixAdmissionOutcome::Admitted {
            target_worldline_id: worldline_id_to_abi(*target_worldline_id),
            admitted_refs: admitted_refs
                .iter()
                .copied()
                .map(provenance_ref_to_abi)
                .collect(),
            basis_report: basis_report.as_ref().map(settlement_basis_report_to_abi),
        },
        WitnessedSuffixAdmissionOutcome::Staged {
            staged_refs,
            basis_report,
        } => abi::WitnessedSuffixAdmissionOutcome::Staged {
            staged_refs: staged_refs
                .iter()
                .copied()
                .map(provenance_ref_to_abi)
                .collect(),
            basis_report: basis_report.as_ref().map(settlement_basis_report_to_abi),
        },
        WitnessedSuffixAdmissionOutcome::Plural {
            candidate_refs,
            residual_posture,
            basis_report,
        } => abi::WitnessedSuffixAdmissionOutcome::Plural {
            candidate_refs: candidate_refs
                .iter()
                .copied()
                .map(provenance_ref_to_abi)
                .collect(),
            residual_posture: reading_residual_posture_to_abi(*residual_posture),
            basis_report: basis_report.as_ref().map(settlement_basis_report_to_abi),
        },
        WitnessedSuffixAdmissionOutcome::Conflict {
            reason,
            source_ref,
            evidence_digest,
            overlap_revalidation,
        } => abi::WitnessedSuffixAdmissionOutcome::Conflict {
            reason: conflict_reason_to_abi(*reason),
            source_ref: provenance_ref_to_abi(*source_ref),
            evidence_digest: evidence_digest.to_vec(),
            overlap_revalidation: overlap_revalidation
                .as_ref()
                .map(overlap_revalidation_to_abi),
        },
        WitnessedSuffixAdmissionOutcome::Obstructed {
            source_ref,
            residual_posture,
            evidence_digest,
        } => abi::WitnessedSuffixAdmissionOutcome::Obstructed {
            source_ref: provenance_ref_to_abi(*source_ref),
            residual_posture: reading_residual_posture_to_abi(*residual_posture),
            evidence_digest: evidence_digest.to_vec(),
        },
    }
}

fn settlement_basis_report_to_abi(report: &StrandBasisReport) -> abi::SettlementBasisReport {
    abi::SettlementBasisReport {
        parent_anchor: base_ref_to_abi(report.parent_anchor),
        child_worldline_id: worldline_id_to_abi(report.child_worldline_id),
        source_suffix_start_tick: worldline_tick_to_abi(report.source_suffix_start_tick),
        source_suffix_end_tick: report.source_suffix_end_tick.map(worldline_tick_to_abi),
        realized_parent_ref: provenance_ref_to_abi(report.realized_parent_ref),
        owned_closed_slot_count: len_to_u64(report.owned_divergence.closed_len()),
        parent_written_slot_count: len_to_u64(report.parent_movement.write_len()),
        parent_revalidation: settlement_parent_revalidation_to_abi(&report.parent_revalidation),
    }
}

fn settlement_parent_revalidation_to_abi(
    revalidation: &StrandRevalidationState,
) -> abi::SettlementParentRevalidation {
    match revalidation {
        StrandRevalidationState::AtAnchor => abi::SettlementParentRevalidation::AtAnchor,
        StrandRevalidationState::ParentAdvancedDisjoint {
            parent_from,
            parent_to,
        } => abi::SettlementParentRevalidation::ParentAdvancedDisjoint {
            parent_from: provenance_ref_to_abi(*parent_from),
            parent_to: provenance_ref_to_abi(*parent_to),
        },
        StrandRevalidationState::RevalidationRequired {
            parent_from,
            parent_to,
            overlapping_slots,
        } => abi::SettlementParentRevalidation::RevalidationRequired {
            parent_from: provenance_ref_to_abi(*parent_from),
            parent_to: provenance_ref_to_abi(*parent_to),
            overlapping_slot_count: len_to_u64(overlapping_slots.len()),
            overlapping_slots_digest: settlement_overlap_slots_digest(overlapping_slots).to_vec(),
        },
    }
}

fn overlap_revalidation_to_abi(
    revalidation: &StrandOverlapRevalidation,
) -> abi::SettlementOverlapRevalidation {
    match revalidation {
        StrandOverlapRevalidation::Clean { overlapping_slots } => {
            abi::SettlementOverlapRevalidation::Clean {
                overlapping_slot_count: len_to_u64(overlapping_slots.len()),
                overlapping_slots_digest: settlement_overlap_slots_digest(overlapping_slots)
                    .to_vec(),
            }
        }
        StrandOverlapRevalidation::Obstructed { overlapping_slots } => {
            abi::SettlementOverlapRevalidation::Obstructed {
                overlapping_slot_count: len_to_u64(overlapping_slots.len()),
                overlapping_slots_digest: settlement_overlap_slots_digest(overlapping_slots)
                    .to_vec(),
            }
        }
        StrandOverlapRevalidation::Conflict { overlapping_slots } => {
            abi::SettlementOverlapRevalidation::Conflict {
                overlapping_slot_count: len_to_u64(overlapping_slots.len()),
                overlapping_slots_digest: settlement_overlap_slots_digest(overlapping_slots)
                    .to_vec(),
            }
        }
    }
}

fn base_ref_to_abi(base_ref: BaseRef) -> abi::BaseRef {
    abi::BaseRef {
        source_worldline_id: worldline_id_to_abi(base_ref.source_worldline_id),
        fork_tick: worldline_tick_to_abi(base_ref.fork_tick),
        commit_hash: base_ref.commit_hash.to_vec(),
        boundary_hash: base_ref.boundary_hash.to_vec(),
        provenance_ref: provenance_ref_to_abi(base_ref.provenance_ref),
    }
}

fn provenance_ref_to_abi(reference: ProvenanceRef) -> abi::ProvenanceRef {
    abi::ProvenanceRef {
        worldline_id: worldline_id_to_abi(reference.worldline_id),
        worldline_tick: worldline_tick_to_abi(reference.worldline_tick),
        commit_hash: reference.commit_hash.to_vec(),
    }
}

fn worldline_id_to_abi(worldline_id: WorldlineId) -> abi::WorldlineId {
    abi::WorldlineId::from_bytes(*worldline_id.as_bytes())
}

fn worldline_tick_to_abi(worldline_tick: WorldlineTick) -> abi::WorldlineTick {
    abi::WorldlineTick(worldline_tick.as_u64())
}

fn conflict_reason_to_abi(reason: ConflictReason) -> abi::ConflictReason {
    match reason {
        ConflictReason::ChannelPolicyConflict => abi::ConflictReason::ChannelPolicyConflict,
        ConflictReason::UnsupportedImport => abi::ConflictReason::UnsupportedImport,
        ConflictReason::BaseDivergence => abi::ConflictReason::BaseDivergence,
        ConflictReason::ParentFootprintOverlap => abi::ConflictReason::ParentFootprintOverlap,
        ConflictReason::QuantumMismatch => abi::ConflictReason::QuantumMismatch,
    }
}

fn reading_residual_posture_to_abi(posture: ReadingResidualPosture) -> abi::ReadingResidualPosture {
    match posture {
        ReadingResidualPosture::Complete => abi::ReadingResidualPosture::Complete,
        ReadingResidualPosture::Residual => abi::ReadingResidualPosture::Residual,
        ReadingResidualPosture::PluralityPreserved => {
            abi::ReadingResidualPosture::PluralityPreserved
        }
        ReadingResidualPosture::Obstructed => abi::ReadingResidualPosture::Obstructed,
    }
}

fn settlement_overlap_slots_digest(slots: &[SlotId]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(b"echo:settlement-overlap-slots:v1\0");
    hasher.update(&len_to_u64(slots.len()).to_le_bytes());
    for slot in slots {
        hash_settlement_slot(&mut hasher, slot);
    }
    hasher.finalize().into()
}

fn hash_settlement_slot(hasher: &mut Hasher, slot: &SlotId) {
    match slot {
        SlotId::Node(node) => {
            hasher.update(&[1]);
            hasher.update(node.warp_id.as_bytes());
            hasher.update(node.local_id.as_bytes());
        }
        SlotId::Edge(edge) => {
            hasher.update(&[2]);
            hasher.update(edge.warp_id.as_bytes());
            hasher.update(edge.local_id.as_bytes());
        }
        SlotId::Attachment(attachment) => {
            hasher.update(&[3]);
            match attachment.owner {
                AttachmentOwner::Node(node) => {
                    hasher.update(&[1]);
                    hasher.update(node.warp_id.as_bytes());
                    hasher.update(node.local_id.as_bytes());
                }
                AttachmentOwner::Edge(edge) => {
                    hasher.update(&[2]);
                    hasher.update(edge.warp_id.as_bytes());
                    hasher.update(edge.local_id.as_bytes());
                }
            }
            match attachment.plane {
                AttachmentPlane::Alpha => hasher.update(&[1]),
                AttachmentPlane::Beta => hasher.update(&[2]),
            };
        }
        SlotId::Port((warp_id, port_key)) => {
            hasher.update(&[4]);
            hasher.update(warp_id.as_bytes());
            hasher.update(&port_key.to_le_bytes());
        }
    }
}

fn len_to_u64(len: usize) -> u64 {
    const {
        assert!(usize::BITS <= u64::BITS);
    }
    len as u64
}
