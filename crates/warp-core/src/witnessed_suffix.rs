// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shape-only witnessed suffix admission vocabulary.
//!
//! This module names the first admission shell skeleton. It deliberately carries
//! compact provenance and basis evidence only; export/import here means suffix
//! shell construction and admission classification, not transport, remote
//! synchronization, or import execution.

use blake3::Hasher;
use echo_wasm_abi::{encode_cbor, kernel_port as abi};
use thiserror::Error;

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

/// Request to export a witnessed causal suffix rooted at a known source frontier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportSuffixRequest {
    /// Source worldline carrying the suffix.
    pub source_worldline_id: WorldlineId,
    /// Known source basis before the suffix begins.
    pub base_frontier: ProvenanceRef,
    /// Optional requested source frontier to export through. If omitted, the
    /// export context may expose the current known suffix frontier.
    pub target_frontier: Option<ProvenanceRef>,
    /// Optional basis-relative settlement evidence reused by the exported shell.
    pub basis_report: Option<StrandBasisReport>,
}

impl ExportSuffixRequest {
    /// Converts the export request into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::ExportSuffixRequest {
        abi::ExportSuffixRequest {
            source_worldline_id: worldline_id_to_abi(self.source_worldline_id),
            base_frontier: provenance_ref_to_abi(self.base_frontier),
            target_frontier: self.target_frontier.map(provenance_ref_to_abi),
            basis_report: self
                .basis_report
                .as_ref()
                .map(settlement_basis_report_to_abi),
        }
    }
}

/// Witnessed suffix bundle exchanged across a hot/cold runtime boundary.
///
/// The bundle is a compact causal shell. It is not a materialized state
/// snapshot, not a raw patch stream, and not a transport endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalSuffixBundle {
    /// Known source basis before the suffix begins.
    pub base_frontier: ProvenanceRef,
    /// Source frontier reached by this exported suffix shell.
    pub target_frontier: ProvenanceRef,
    /// Compact source suffix and its witness digest.
    pub source_suffix: WitnessedSuffixShell,
    /// Deterministic digest of the bundle identity used for retained shell
    /// equivalence and loop-prevention surfaces.
    pub bundle_digest: Hash,
}

impl CausalSuffixBundle {
    /// Builds a bundle and derives canonical source-shell and bundle digests.
    #[must_use]
    pub fn new(
        base_frontier: ProvenanceRef,
        target_frontier: ProvenanceRef,
        mut source_suffix: WitnessedSuffixShell,
    ) -> Self {
        source_suffix.witness_digest = derive_witnessed_suffix_shell_digest(&source_suffix);
        let bundle_digest =
            derive_causal_suffix_bundle_digest(base_frontier, target_frontier, &source_suffix);
        Self {
            base_frontier,
            target_frontier,
            source_suffix,
            bundle_digest,
        }
    }

    /// Returns the deterministic digest used to compare retained shell results.
    #[must_use]
    pub const fn shell_equivalence_digest(&self) -> Hash {
        self.bundle_digest
    }

    /// Converts the bundle into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::CausalSuffixBundle {
        abi::CausalSuffixBundle {
            base_frontier: provenance_ref_to_abi(self.base_frontier),
            target_frontier: provenance_ref_to_abi(self.target_frontier),
            source_suffix: self.source_suffix.to_abi(),
            bundle_digest: self.bundle_digest.to_vec(),
        }
    }
}

/// Obstruction returned when Echo cannot produce a witnessed suffix bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportSuffixObstruction {
    /// Source coordinate implicated in the obstruction.
    pub source_ref: ProvenanceRef,
    /// Read-side residual posture associated with the obstruction.
    pub residual_posture: ReadingResidualPosture,
    /// Deterministic digest of compact obstruction evidence.
    pub evidence_digest: Hash,
}

impl ExportSuffixObstruction {
    /// Converts the obstruction into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::ExportSuffixObstruction {
        abi::ExportSuffixObstruction {
            source_ref: provenance_ref_to_abi(self.source_ref),
            residual_posture: reading_residual_posture_to_abi(self.residual_posture),
            evidence_digest: self.evidence_digest.to_vec(),
        }
    }
}

/// Request to import one witnessed causal suffix bundle by classifying it
/// against a target basis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportSuffixRequest {
    /// Source bundle being judged.
    pub bundle: CausalSuffixBundle,
    /// Worldline receiving the proposed admission.
    pub target_worldline_id: WorldlineId,
    /// Target basis used while judging admission.
    pub target_basis: ProvenanceRef,
    /// Optional target-basis evidence for strand/parent realization cases.
    pub basis_report: Option<StrandBasisReport>,
}

impl ImportSuffixRequest {
    /// Converts the import request into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::ImportSuffixRequest {
        abi::ImportSuffixRequest {
            bundle: self.bundle.to_abi(),
            target_worldline_id: worldline_id_to_abi(self.target_worldline_id),
            target_basis: provenance_ref_to_abi(self.target_basis),
            basis_report: self
                .basis_report
                .as_ref()
                .map(settlement_basis_report_to_abi),
        }
    }
}

/// Result of importing one witnessed causal suffix bundle into local admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportSuffixResult {
    /// Bundle identity retained for shell-equivalence and loop-prevention checks.
    pub bundle_digest: Hash,
    /// Admission classifier response for the bundle's source suffix.
    pub admission: WitnessedSuffixAdmissionResponse,
}

impl ImportSuffixResult {
    /// Returns the deterministic digest used to compare retained shell results.
    #[must_use]
    pub const fn retained_shell_equivalence_digest(&self) -> Hash {
        self.bundle_digest
    }

    /// Converts the import result into its ABI DTO.
    #[must_use]
    pub fn to_abi(&self) -> abi::ImportSuffixResult {
        abi::ImportSuffixResult {
            bundle_digest: self.bundle_digest.to_vec(),
            admission: self.admission.to_abi(),
        }
    }
}

/// Read-only export evidence source used by witnessed suffix bundle construction.
///
/// This trait is intentionally narrow. It supplies suffix coordinates and
/// boundary witness material, but exposes no runtime mutation, network
/// transport, patch stream, or sync loop.
pub trait WitnessedSuffixExportContext {
    /// Returns the source provenance coordinates covered by the requested suffix.
    fn source_entries(&self, request: &ExportSuffixRequest) -> Option<Vec<ProvenanceRef>>;

    /// Returns a boundary witness when the suffix has no importable entries yet.
    fn boundary_witness(&self, request: &ExportSuffixRequest) -> Option<ProvenanceRef>;
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

/// Exports a witnessed causal suffix bundle from local read-only evidence.
///
/// This constructs a causal shell and witness digest. It does not execute
/// transport and does not mutate any receiving worldline.
pub fn export_suffix(
    request: &ExportSuffixRequest,
    context: &impl WitnessedSuffixExportContext,
) -> Result<CausalSuffixBundle, ExportSuffixObstruction> {
    if request.base_frontier.worldline_id != request.source_worldline_id {
        return Err(export_obstruction(request));
    }
    if let Some(target_frontier) = request.target_frontier {
        if target_frontier.worldline_id != request.source_worldline_id
            || target_frontier.worldline_tick.as_u64()
                < request.base_frontier.worldline_tick.as_u64()
        {
            return Err(export_obstruction(request));
        }
    }

    let Some(entries) = context.source_entries(request) else {
        return Err(export_obstruction(request));
    };
    let Ok(entries) = canonical_unique_provenance_refs(entries) else {
        return Err(export_obstruction(request));
    };
    let boundary_witness = context.boundary_witness(request);

    if entries.is_empty() && boundary_witness.is_none() {
        return Err(export_obstruction(request));
    }

    for source_entry in &entries {
        if source_entry.worldline_id != request.source_worldline_id
            || source_entry.worldline_tick.as_u64() <= request.base_frontier.worldline_tick.as_u64()
            || request.target_frontier.is_some_and(|target_frontier| {
                source_entry.worldline_tick.as_u64() > target_frontier.worldline_tick.as_u64()
            })
        {
            return Err(export_obstruction(request));
        }
    }

    let derived_target_frontier = match (entries.last().copied(), request.target_frontier) {
        (Some(last_entry), Some(target_frontier)) if last_entry == target_frontier => {
            target_frontier
        }
        (None, Some(target_frontier)) if target_frontier == request.base_frontier => {
            target_frontier
        }
        (Some(_) | None, Some(_)) => return Err(export_obstruction(request)),
        (Some(last_entry), None) => last_entry,
        (None, None) => request.base_frontier,
    };

    let source_suffix_start_tick = entries
        .first()
        .map_or(request.base_frontier.worldline_tick, |entry| {
            entry.worldline_tick
        });
    let source_suffix_end_tick = entries.last().map(|entry| entry.worldline_tick);
    let source_suffix = WitnessedSuffixShell {
        source_worldline_id: request.source_worldline_id,
        source_suffix_start_tick,
        source_suffix_end_tick,
        source_entries: entries,
        boundary_witness,
        witness_digest: [0; 32],
        basis_report: request.basis_report.clone(),
    };

    Ok(CausalSuffixBundle::new(
        request.base_frontier,
        derived_target_frontier,
        source_suffix,
    ))
}

/// Imports one witnessed causal suffix bundle by classifying it against the
/// local target basis.
///
/// The returned result is an admission shell result. It does not append
/// provenance or apply patches directly.
#[must_use]
pub fn import_suffix(
    request: &ImportSuffixRequest,
    context: &impl WitnessedSuffixAdmissionContext,
) -> ImportSuffixResult {
    let admission_request = WitnessedSuffixAdmissionRequest {
        source_suffix: request.bundle.source_suffix.clone(),
        target_worldline_id: request.target_worldline_id,
        target_basis: request.target_basis,
        basis_report: request.basis_report.clone(),
    };
    let admission = evaluate_witnessed_suffix_admission(&admission_request, context);

    ImportSuffixResult {
        bundle_digest: request.bundle.bundle_digest,
        admission,
    }
}

/// Error returned when constructing a canonical local admission posture fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum WitnessedSuffixLocalAdmissionPostureError {
    /// A provenance coordinate appeared more than once in a posture vector.
    #[error("duplicate witnessed suffix local admission provenance ref: {provenance_ref:?}")]
    DuplicateProvenanceRef {
        /// Duplicate provenance coordinate.
        provenance_ref: ProvenanceRef,
    },
}

/// Local posture reported by the read-only admission context.
///
/// Prefer [`Self::admissible`], [`Self::staged`], or [`Self::plural`] for
/// ordinary construction so ref vectors are sorted canonically and duplicate
/// provenance refs are rejected before reaching an ABI-visible response. Direct
/// enum construction remains available for raw-shape tests and defensive
/// evaluator inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WitnessedSuffixLocalAdmissionPosture {
    /// Local evidence says the suffix is admissible.
    Admissible {
        /// Target-local provenance coordinates produced or expected by admission.
        ///
        /// Use [`WitnessedSuffixLocalAdmissionPosture::admissible`] for normal
        /// construction.
        admitted_refs: Vec<ProvenanceRef>,
    },
    /// Local evidence says the suffix should be retained for later judgment.
    Staged {
        /// Source or target coordinates retained while staged.
        ///
        /// Use [`WitnessedSuffixLocalAdmissionPosture::staged`] for normal
        /// construction.
        staged_refs: Vec<ProvenanceRef>,
    },
    /// Local evidence preserves lawful plurality.
    Plural {
        /// Candidate coordinates that remain lawful plural outcomes.
        ///
        /// Use [`WitnessedSuffixLocalAdmissionPosture::plural`] for normal
        /// construction.
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

impl WitnessedSuffixLocalAdmissionPosture {
    /// Builds an admissible posture with canonical admitted refs.
    pub fn admissible(
        admitted_refs: Vec<ProvenanceRef>,
    ) -> Result<Self, WitnessedSuffixLocalAdmissionPostureError> {
        Ok(Self::Admissible {
            admitted_refs: canonical_unique_provenance_refs(admitted_refs)?,
        })
    }

    /// Builds a staged posture with canonical staged refs.
    pub fn staged(
        staged_refs: Vec<ProvenanceRef>,
    ) -> Result<Self, WitnessedSuffixLocalAdmissionPostureError> {
        Ok(Self::Staged {
            staged_refs: canonical_unique_provenance_refs(staged_refs)?,
        })
    }

    /// Builds a plural posture with canonical candidate refs.
    pub fn plural(
        candidate_refs: Vec<ProvenanceRef>,
    ) -> Result<Self, WitnessedSuffixLocalAdmissionPostureError> {
        Ok(Self::Plural {
            candidate_refs: canonical_unique_provenance_refs(candidate_refs)?,
        })
    }

    /// Builds a conflict posture from named conflict evidence.
    #[must_use]
    pub fn conflict(
        reason: ConflictReason,
        source_ref: ProvenanceRef,
        evidence_digest: Hash,
        overlap_revalidation: Option<StrandOverlapRevalidation>,
    ) -> Self {
        Self::Conflict {
            reason,
            source_ref,
            evidence_digest,
            overlap_revalidation,
        }
    }
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

fn canonical_unique_provenance_refs(
    mut refs: Vec<ProvenanceRef>,
) -> Result<Vec<ProvenanceRef>, WitnessedSuffixLocalAdmissionPostureError> {
    refs.sort_unstable();

    for window in refs.windows(2) {
        if window[0] == window[1] {
            return Err(
                WitnessedSuffixLocalAdmissionPostureError::DuplicateProvenanceRef {
                    provenance_ref: window[0],
                },
            );
        }
    }

    Ok(refs)
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

/// Derives the canonical digest for a witnessed suffix shell.
///
/// The shell's caller-supplied `witness_digest` field is ignored while deriving
/// identity so export does not trust a prefilled claim.
#[must_use]
pub fn derive_witnessed_suffix_shell_digest(shell: &WitnessedSuffixShell) -> Hash {
    let mut shell_without_claim = shell.to_abi();
    shell_without_claim.witness_digest.clear();

    let mut hasher = Hasher::new();
    hasher.update(b"echo:witnessed-suffix-shell:v1\0");
    match encode_cbor(&shell_without_claim) {
        Ok(encoded_shell) => {
            hasher.update(&encoded_shell);
        }
        Err(_) => hash_source_shell_obstruction_fallback(&mut hasher, shell),
    }
    hasher.finalize().into()
}

fn derive_causal_suffix_bundle_digest(
    base_frontier: ProvenanceRef,
    target_frontier: ProvenanceRef,
    source_suffix: &WitnessedSuffixShell,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(b"echo:causal-suffix-bundle:v1\0");
    hash_provenance_ref(&mut hasher, &base_frontier);
    hash_provenance_ref(&mut hasher, &target_frontier);
    hasher.update(&source_suffix.witness_digest);
    hasher.finalize().into()
}

fn export_obstruction(request: &ExportSuffixRequest) -> ExportSuffixObstruction {
    ExportSuffixObstruction {
        source_ref: request.base_frontier,
        residual_posture: ReadingResidualPosture::Obstructed,
        evidence_digest: export_suffix_obstruction_digest(request),
    }
}

fn export_suffix_obstruction_digest(request: &ExportSuffixRequest) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(b"echo:export-suffix-obstruction:v1\0");
    hasher.update(request.source_worldline_id.as_bytes());
    hash_provenance_ref(&mut hasher, &request.base_frontier);
    match request.target_frontier {
        Some(target_frontier) => {
            hasher.update(&[1]);
            hash_provenance_ref(&mut hasher, &target_frontier);
        }
        None => {
            hasher.update(&[0]);
        }
    }
    hasher.update(&[u8::from(request.basis_report.is_some())]);
    hasher.finalize().into()
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
