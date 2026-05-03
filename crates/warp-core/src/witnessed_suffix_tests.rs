// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

use echo_wasm_abi::kernel_port as abi;

use crate::{
    evaluate_witnessed_suffix_admission, make_node_id, make_strand_id, BaseRef, ConflictReason,
    Hash, NodeKey, ParentMovementFootprint, ProvenanceRef, ReadingResidualPosture, SlotId,
    StrandBasisReport, StrandDivergenceFootprint, StrandOverlapRevalidation,
    StrandRevalidationState, WarpId, WitnessedSuffixAdmissionContext,
    WitnessedSuffixAdmissionOutcome, WitnessedSuffixAdmissionRequest,
    WitnessedSuffixAdmissionResponse, WitnessedSuffixLocalAdmissionPosture,
    WitnessedSuffixLocalAdmissionPostureError, WitnessedSuffixShell, WorldlineId, WorldlineTick,
};

fn worldline(seed: u8) -> WorldlineId {
    WorldlineId::from_bytes([seed; 32])
}

fn tick(value: u64) -> WorldlineTick {
    WorldlineTick::from_raw(value)
}

fn provenance_ref(seed: u8, worldline_tick: u64) -> ProvenanceRef {
    ProvenanceRef {
        worldline_id: worldline(seed),
        worldline_tick: tick(worldline_tick),
        commit_hash: [seed.wrapping_add(1); 32],
    }
}

fn node_slot(label: &str) -> SlotId {
    SlotId::Node(NodeKey {
        warp_id: WarpId([91; 32]),
        local_id: make_node_id(label),
    })
}

fn basis_report(realized_parent_ref: ProvenanceRef) -> StrandBasisReport {
    let parent_anchor = provenance_ref(20, 1);

    StrandBasisReport {
        strand_id: make_strand_id("witnessed-suffix-test"),
        parent_anchor: BaseRef {
            source_worldline_id: parent_anchor.worldline_id,
            fork_tick: parent_anchor.worldline_tick,
            commit_hash: parent_anchor.commit_hash,
            boundary_hash: [21; 32],
            provenance_ref: parent_anchor,
        },
        child_worldline_id: worldline(22),
        source_suffix_start_tick: tick(2),
        source_suffix_end_tick: Some(tick(4)),
        realized_parent_ref,
        owned_divergence: StrandDivergenceFootprint::default(),
        parent_movement: ParentMovementFootprint::default(),
        parent_revalidation: StrandRevalidationState::AtAnchor,
    }
}

fn shell_with_entries(entries: Vec<ProvenanceRef>) -> WitnessedSuffixShell {
    WitnessedSuffixShell {
        source_worldline_id: worldline(3),
        source_suffix_start_tick: tick(2),
        source_suffix_end_tick: Some(tick(4)),
        source_entries: entries,
        boundary_witness: Some(provenance_ref(5, 1)),
        witness_digest: [6; 32],
        basis_report: None,
    }
}

fn request() -> WitnessedSuffixAdmissionRequest {
    WitnessedSuffixAdmissionRequest {
        source_suffix: shell_with_entries(vec![provenance_ref(3, 3)]),
        target_worldline_id: worldline(11),
        target_basis: provenance_ref(12, 9),
        basis_report: None,
    }
}

fn response(outcome: WitnessedSuffixAdmissionOutcome) -> WitnessedSuffixAdmissionResponse {
    WitnessedSuffixAdmissionResponse {
        source_shell_digest: [6; 32],
        target_basis: provenance_ref(12, 9),
        outcome,
    }
}

struct FakeAdmissionContext {
    expected_shell_digest: Option<Hash>,
    resolved_target_basis: Option<ProvenanceRef>,
    posture: WitnessedSuffixLocalAdmissionPosture,
}

impl WitnessedSuffixAdmissionContext for FakeAdmissionContext {
    fn source_shell_digest(&self, _shell: &WitnessedSuffixShell) -> Option<Hash> {
        self.expected_shell_digest
    }

    fn resolve_target_basis(&self, _target_basis: ProvenanceRef) -> Option<ProvenanceRef> {
        self.resolved_target_basis
    }

    fn local_admission_posture(
        &self,
        _request: &WitnessedSuffixAdmissionRequest,
    ) -> WitnessedSuffixLocalAdmissionPosture {
        self.posture.clone()
    }
}

struct TargetBasisEchoAdmissionContext {
    resolved_target_basis: ProvenanceRef,
}

impl WitnessedSuffixAdmissionContext for TargetBasisEchoAdmissionContext {
    fn source_shell_digest(&self, _shell: &WitnessedSuffixShell) -> Option<Hash> {
        Some([6; 32])
    }

    fn resolve_target_basis(&self, _target_basis: ProvenanceRef) -> Option<ProvenanceRef> {
        Some(self.resolved_target_basis)
    }

    fn local_admission_posture(
        &self,
        request: &WitnessedSuffixAdmissionRequest,
    ) -> WitnessedSuffixLocalAdmissionPosture {
        // This trait method cannot return Result, so this fixture uses the raw
        // shape only where the resolved basis is already deterministic.
        WitnessedSuffixLocalAdmissionPosture::Admissible {
            admitted_refs: vec![request.target_basis],
        }
    }
}

fn clean_context(posture: WitnessedSuffixLocalAdmissionPosture) -> FakeAdmissionContext {
    FakeAdmissionContext {
        expected_shell_digest: Some([6; 32]),
        resolved_target_basis: Some(provenance_ref(12, 9)),
        posture,
    }
}

fn admissible_posture(
    refs: Vec<ProvenanceRef>,
) -> Result<WitnessedSuffixLocalAdmissionPosture, WitnessedSuffixLocalAdmissionPostureError> {
    WitnessedSuffixLocalAdmissionPosture::admissible(refs)
}

fn staged_posture(
    refs: Vec<ProvenanceRef>,
) -> Result<WitnessedSuffixLocalAdmissionPosture, WitnessedSuffixLocalAdmissionPostureError> {
    WitnessedSuffixLocalAdmissionPosture::staged(refs)
}

fn plural_posture(
    refs: Vec<ProvenanceRef>,
) -> Result<WitnessedSuffixLocalAdmissionPosture, WitnessedSuffixLocalAdmissionPostureError> {
    WitnessedSuffixLocalAdmissionPosture::plural(refs)
}

fn conflict_posture(
    reason: ConflictReason,
    source_ref: ProvenanceRef,
    evidence_digest: Hash,
    overlap_revalidation: Option<StrandOverlapRevalidation>,
) -> WitnessedSuffixLocalAdmissionPosture {
    WitnessedSuffixLocalAdmissionPosture::conflict(
        reason,
        source_ref,
        evidence_digest,
        overlap_revalidation,
    )
}

#[test]
fn witnessed_suffix_core_request_converts_to_abi_shape() {
    let request = request();
    let converted: abi::WitnessedSuffixAdmissionRequest = request.to_abi();

    assert_eq!(
        converted.source_suffix.source_worldline_id,
        abi::WorldlineId::from_bytes([3; 32])
    );
    assert_eq!(
        converted.source_suffix.source_entries,
        vec![abi::ProvenanceRef {
            worldline_id: abi::WorldlineId::from_bytes([3; 32]),
            worldline_tick: abi::WorldlineTick(3),
            commit_hash: vec![4; 32],
        }]
    );
    assert_eq!(
        converted.target_basis,
        abi::ProvenanceRef {
            worldline_id: abi::WorldlineId::from_bytes([12; 32]),
            worldline_tick: abi::WorldlineTick(9),
            commit_hash: vec![13; 32],
        }
    );
}

#[test]
fn witnessed_suffix_core_response_converts_admitted_outcome_to_abi() {
    let response = response(WitnessedSuffixAdmissionOutcome::Admitted {
        target_worldline_id: worldline(11),
        admitted_refs: vec![provenance_ref(30, 10)],
        basis_report: None,
    });

    let converted: abi::WitnessedSuffixAdmissionResponse = response.to_abi();

    assert!(matches!(
        converted.outcome,
        abi::WitnessedSuffixAdmissionOutcome::Admitted { .. }
    ));
}

#[test]
fn witnessed_suffix_core_response_converts_staged_outcome_to_abi() {
    let response = response(WitnessedSuffixAdmissionOutcome::Staged {
        staged_refs: vec![provenance_ref(32, 11)],
        basis_report: None,
    });

    let converted: abi::WitnessedSuffixAdmissionResponse = response.to_abi();

    assert!(matches!(
        converted.outcome,
        abi::WitnessedSuffixAdmissionOutcome::Staged { .. }
    ));
}

#[test]
fn witnessed_suffix_core_response_converts_plural_outcome_to_abi() {
    let response = response(WitnessedSuffixAdmissionOutcome::Plural {
        candidate_refs: vec![provenance_ref(33, 12), provenance_ref(34, 13)],
        residual_posture: ReadingResidualPosture::PluralityPreserved,
        basis_report: None,
    });

    let converted: abi::WitnessedSuffixAdmissionResponse = response.to_abi();

    assert!(matches!(
        converted.outcome,
        abi::WitnessedSuffixAdmissionOutcome::Plural {
            residual_posture: abi::ReadingResidualPosture::PluralityPreserved,
            ..
        }
    ));
}

#[test]
fn witnessed_suffix_core_response_converts_conflict_outcome_to_abi() {
    let response = response(WitnessedSuffixAdmissionOutcome::Conflict {
        reason: ConflictReason::ParentFootprintOverlap,
        source_ref: provenance_ref(35, 14),
        evidence_digest: [36; 32],
        overlap_revalidation: None,
    });

    let converted: abi::WitnessedSuffixAdmissionResponse = response.to_abi();

    assert!(matches!(
        converted.outcome,
        abi::WitnessedSuffixAdmissionOutcome::Conflict {
            reason: abi::ConflictReason::ParentFootprintOverlap,
            ..
        }
    ));
}

#[test]
fn witnessed_suffix_core_response_converts_obstructed_outcome_to_abi() {
    let response = response(WitnessedSuffixAdmissionOutcome::Obstructed {
        source_ref: provenance_ref(37, 15),
        residual_posture: ReadingResidualPosture::Obstructed,
        evidence_digest: [38; 32],
    });

    let converted: abi::WitnessedSuffixAdmissionResponse = response.to_abi();

    assert!(matches!(
        converted.outcome,
        abi::WitnessedSuffixAdmissionOutcome::Obstructed {
            residual_posture: abi::ReadingResidualPosture::Obstructed,
            ..
        }
    ));
}

#[test]
fn witnessed_suffix_local_posture_admissible_constructor_canonicalizes_refs() {
    assert_eq!(
        WitnessedSuffixLocalAdmissionPosture::admissible(vec![
            provenance_ref(30, 12),
            provenance_ref(30, 10),
        ]),
        Ok(WitnessedSuffixLocalAdmissionPosture::Admissible {
            admitted_refs: vec![provenance_ref(30, 10), provenance_ref(30, 12)],
        })
    );
}

#[test]
fn witnessed_suffix_local_posture_staged_constructor_canonicalizes_refs() {
    assert_eq!(
        WitnessedSuffixLocalAdmissionPosture::staged(vec![
            provenance_ref(32, 12),
            provenance_ref(32, 11),
        ]),
        Ok(WitnessedSuffixLocalAdmissionPosture::Staged {
            staged_refs: vec![provenance_ref(32, 11), provenance_ref(32, 12)],
        })
    );
}

#[test]
fn witnessed_suffix_local_posture_plural_constructor_canonicalizes_refs() {
    assert_eq!(
        WitnessedSuffixLocalAdmissionPosture::plural(vec![
            provenance_ref(34, 13),
            provenance_ref(33, 12),
        ]),
        Ok(WitnessedSuffixLocalAdmissionPosture::Plural {
            candidate_refs: vec![provenance_ref(33, 12), provenance_ref(34, 13)],
        })
    );
}

#[test]
fn witnessed_suffix_local_posture_constructors_reject_duplicate_refs() {
    let duplicate_ref = provenance_ref(30, 10);

    for duplicate_result in [
        WitnessedSuffixLocalAdmissionPosture::admissible(vec![duplicate_ref, duplicate_ref]),
        WitnessedSuffixLocalAdmissionPosture::staged(vec![duplicate_ref, duplicate_ref]),
        WitnessedSuffixLocalAdmissionPosture::plural(vec![duplicate_ref, duplicate_ref]),
    ] {
        assert_eq!(
            duplicate_result,
            Err(
                WitnessedSuffixLocalAdmissionPostureError::DuplicateProvenanceRef {
                    provenance_ref: duplicate_ref,
                }
            )
        );
    }
}

#[test]
fn witnessed_suffix_local_posture_constructors_reject_duplicates_after_sorting() {
    let duplicate_ref = provenance_ref(30, 10);
    let intervening_ref = provenance_ref(31, 10);

    assert_eq!(
        WitnessedSuffixLocalAdmissionPosture::admissible(vec![
            duplicate_ref,
            intervening_ref,
            duplicate_ref,
        ]),
        Err(
            WitnessedSuffixLocalAdmissionPostureError::DuplicateProvenanceRef {
                provenance_ref: duplicate_ref,
            },
        )
    );
}

#[test]
fn witnessed_suffix_local_posture_conflict_constructor_names_all_evidence() {
    let overlap_revalidation = StrandOverlapRevalidation::Conflict {
        overlapping_slots: vec![node_slot("constructor-overlap-a")],
    };

    let posture = WitnessedSuffixLocalAdmissionPosture::conflict(
        ConflictReason::ParentFootprintOverlap,
        provenance_ref(35, 14),
        [36; 32],
        Some(overlap_revalidation.clone()),
    );

    assert_eq!(
        posture,
        WitnessedSuffixLocalAdmissionPosture::Conflict {
            reason: ConflictReason::ParentFootprintOverlap,
            source_ref: provenance_ref(35, 14),
            evidence_digest: [36; 32],
            overlap_revalidation: Some(overlap_revalidation),
        }
    );
}

#[test]
fn witnessed_suffix_evaluator_admits_clean_suffix(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let request = request();
    let context = clean_context(admissible_posture(vec![provenance_ref(30, 10)])?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert_eq!(response.source_shell_digest, [6; 32]);
    assert_eq!(response.target_basis, provenance_ref(12, 9));
    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Admitted {
            target_worldline_id,
            admitted_refs,
            ..
        } if target_worldline_id == worldline(11)
            && admitted_refs == vec![provenance_ref(30, 10)]
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_stages_boundary_only_suffix(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let request = WitnessedSuffixAdmissionRequest {
        source_suffix: shell_with_entries(Vec::new()),
        target_worldline_id: worldline(11),
        target_basis: provenance_ref(12, 9),
        basis_report: None,
    };
    let context = clean_context(staged_posture(vec![provenance_ref(5, 1)])?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Staged { staged_refs, .. }
            if staged_refs == vec![provenance_ref(5, 1)]
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_obstructs_empty_suffix_without_boundary_witness(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let mut request = WitnessedSuffixAdmissionRequest {
        source_suffix: shell_with_entries(Vec::new()),
        target_worldline_id: worldline(11),
        target_basis: provenance_ref(12, 9),
        basis_report: None,
    };
    request.source_suffix.boundary_witness = None;
    let context = clean_context(staged_posture(Vec::new())?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        }
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_allows_equal_start_and_end_ticks(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let mut request = request();
    request.source_suffix.source_suffix_end_tick =
        Some(request.source_suffix.source_suffix_start_tick);
    request.source_suffix.source_entries = vec![provenance_ref(3, 2)];
    let context = clean_context(admissible_posture(vec![provenance_ref(30, 10)])?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Admitted {
            target_worldline_id,
            admitted_refs,
            ..
        } if target_worldline_id == worldline(11)
            && admitted_refs == vec![provenance_ref(30, 10)]
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_stages_when_target_basis_is_boundary_witness(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let boundary_witness = provenance_ref(5, 1);
    let request = WitnessedSuffixAdmissionRequest {
        source_suffix: shell_with_entries(Vec::new()),
        target_worldline_id: worldline(11),
        target_basis: boundary_witness,
        basis_report: None,
    };
    let context = FakeAdmissionContext {
        expected_shell_digest: Some([6; 32]),
        resolved_target_basis: Some(boundary_witness),
        posture: staged_posture(vec![boundary_witness])?,
    };

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert_eq!(response.target_basis, boundary_witness);
    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Staged { staged_refs, .. }
            if staged_refs == vec![boundary_witness]
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_preserves_plural_outcome(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let request = request();
    let candidates = vec![provenance_ref(33, 12), provenance_ref(34, 13)];
    let context = clean_context(plural_posture(candidates.clone())?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Plural {
            candidate_refs,
            residual_posture: ReadingResidualPosture::PluralityPreserved,
            ..
        } if candidate_refs == candidates
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_conflicts_with_adverse_admission_law() {
    let request = request();
    let context = clean_context(conflict_posture(
        ConflictReason::ParentFootprintOverlap,
        provenance_ref(35, 14),
        [36; 32],
        None,
    ));

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Conflict {
            reason: ConflictReason::ParentFootprintOverlap,
            source_ref,
            evidence_digest,
            overlap_revalidation: None,
        } if source_ref == provenance_ref(35, 14) && evidence_digest == [36; 32]
    ));
}

#[test]
fn witnessed_suffix_evaluator_preserves_conflict_overlap_revalidation() {
    let request = request();
    let overlap_revalidation = StrandOverlapRevalidation::Conflict {
        overlapping_slots: vec![node_slot("overlap-a"), node_slot("overlap-b")],
    };
    let context = clean_context(conflict_posture(
        ConflictReason::ParentFootprintOverlap,
        provenance_ref(35, 14),
        [36; 32],
        Some(overlap_revalidation.clone()),
    ));

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        &response.outcome,
        WitnessedSuffixAdmissionOutcome::Conflict {
            reason: ConflictReason::ParentFootprintOverlap,
            overlap_revalidation: Some(actual),
            ..
        } if actual == &overlap_revalidation
    ));

    let converted = response.to_abi();

    assert!(matches!(
        converted.outcome,
        abi::WitnessedSuffixAdmissionOutcome::Conflict {
            overlap_revalidation: Some(abi::SettlementOverlapRevalidation::Conflict {
                overlapping_slot_count: 2,
                ..
            }),
            ..
        }
    ));
}

#[test]
fn witnessed_suffix_evaluator_classifies_against_resolved_target_basis() {
    let request = request();
    let resolved_basis = provenance_ref(44, 20);
    let context = TargetBasisEchoAdmissionContext {
        resolved_target_basis: resolved_basis,
    };

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert_eq!(response.target_basis, resolved_basis);
    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Admitted { admitted_refs, .. }
            if admitted_refs == vec![resolved_basis]
    ));
}

#[test]
fn witnessed_suffix_evaluator_preserves_matching_request_basis_report(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let mut request = request();
    let report = basis_report(provenance_ref(12, 9));
    request.basis_report = Some(report.clone());
    request.source_suffix.basis_report = Some(basis_report(provenance_ref(12, 9)));
    let context = clean_context(admissible_posture(vec![provenance_ref(30, 10)])?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Admitted {
            basis_report: Some(actual),
            ..
        } if actual == report
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_falls_back_to_matching_source_basis_report(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let mut request = request();
    let report = basis_report(provenance_ref(12, 9));
    request.source_suffix.basis_report = Some(report.clone());
    let context = clean_context(staged_posture(vec![provenance_ref(5, 1)])?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Staged {
            basis_report: Some(actual),
            ..
        } if actual == report
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_obstructs_stale_basis_report(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let mut request = request();
    request.basis_report = Some(basis_report(provenance_ref(99, 99)));
    let context = clean_context(admissible_posture(vec![provenance_ref(30, 10)])?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        }
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_normalizes_raw_admitted_refs() {
    let request = request();
    let context = clean_context(WitnessedSuffixLocalAdmissionPosture::Admissible {
        admitted_refs: vec![provenance_ref(30, 12), provenance_ref(30, 10)],
    });

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Admitted { admitted_refs, .. }
            if admitted_refs == vec![provenance_ref(30, 10), provenance_ref(30, 12)]
    ));
}

#[test]
fn witnessed_suffix_evaluator_normalizes_raw_staged_refs() {
    let request = request();
    let context = clean_context(WitnessedSuffixLocalAdmissionPosture::Staged {
        staged_refs: vec![provenance_ref(32, 12), provenance_ref(32, 11)],
    });

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Staged { staged_refs, .. }
            if staged_refs == vec![provenance_ref(32, 11), provenance_ref(32, 12)]
    ));
}

#[test]
fn witnessed_suffix_evaluator_normalizes_raw_plural_candidate_refs() {
    let request = request();
    let context = clean_context(WitnessedSuffixLocalAdmissionPosture::Plural {
        candidate_refs: vec![provenance_ref(34, 13), provenance_ref(33, 12)],
    });

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Plural { candidate_refs, .. }
            if candidate_refs == vec![provenance_ref(33, 12), provenance_ref(34, 13)]
    ));
}

#[test]
fn witnessed_suffix_evaluator_obstructs_mismatched_digest(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let request = request();
    let context = FakeAdmissionContext {
        expected_shell_digest: Some([99; 32]),
        resolved_target_basis: Some(provenance_ref(12, 9)),
        posture: admissible_posture(vec![provenance_ref(30, 10)])?,
    };

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        }
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_obstructs_missing_local_source_digest_without_reusing_request_digest(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let request = request();
    let context = FakeAdmissionContext {
        expected_shell_digest: None,
        resolved_target_basis: Some(provenance_ref(12, 9)),
        posture: admissible_posture(vec![provenance_ref(30, 10)])?,
    };

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert_ne!(
        response.source_shell_digest,
        request.source_suffix.witness_digest
    );
    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            evidence_digest,
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        } if evidence_digest == response.source_shell_digest
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_obstructs_unknown_target_basis(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let request = request();
    let context = FakeAdmissionContext {
        expected_shell_digest: Some([6; 32]),
        resolved_target_basis: None,
        posture: admissible_posture(vec![provenance_ref(30, 10)])?,
    };

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        }
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_obstructs_inconsistent_bounds(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let mut request = request();
    request.source_suffix.source_suffix_start_tick = tick(5);
    request.source_suffix.source_suffix_end_tick = Some(tick(4));
    let context = clean_context(admissible_posture(vec![provenance_ref(30, 10)])?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        }
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_obstructs_source_entry_outside_suffix_bounds(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    for outside_tick in [1, 5] {
        let mut request = request();
        let offending_ref = provenance_ref(3, outside_tick);
        request.source_suffix.source_entries = vec![offending_ref];
        let context = clean_context(admissible_posture(vec![provenance_ref(30, 10)])?);

        let response = evaluate_witnessed_suffix_admission(&request, &context);

        assert!(matches!(
            response.outcome,
            WitnessedSuffixAdmissionOutcome::Obstructed {
                source_ref,
                residual_posture: ReadingResidualPosture::Obstructed,
                ..
            } if source_ref == offending_ref
        ));
    }
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_obstructs_source_entry_from_foreign_worldline(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let mut request = request();
    let offending_ref = provenance_ref(4, 3);
    request.source_suffix.source_entries = vec![offending_ref];
    let context = clean_context(admissible_posture(vec![provenance_ref(30, 10)])?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            source_ref,
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        } if source_ref == offending_ref
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_obstructs_out_of_order_source_entries(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let mut request = request();
    let offending_ref = provenance_ref(3, 3);
    request.source_suffix.source_entries = vec![provenance_ref(3, 4), offending_ref];
    let context = clean_context(admissible_posture(vec![provenance_ref(30, 10)])?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            source_ref,
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        } if source_ref == offending_ref
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_obstructs_duplicate_source_entries(
) -> Result<(), WitnessedSuffixLocalAdmissionPostureError> {
    let mut request = request();
    let duplicate_ref = provenance_ref(3, 3);
    request.source_suffix.source_entries = vec![duplicate_ref, duplicate_ref];
    let context = clean_context(admissible_posture(vec![provenance_ref(30, 10)])?);

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            source_ref,
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        } if source_ref == duplicate_ref
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_evaluator_has_no_transport_or_sync_surface() {
    let source = include_str!("witnessed_suffix.rs");

    assert!(!source.contains("transport_endpoint"));
    assert!(!source.contains("network_sync_api"));
    assert!(!source.contains("peer_identity"));
    assert!(!source.contains("sync_daemon"));
    assert!(!source.contains("raw_patch_stream"));
}

#[test]
fn witnessed_suffix_evaluator_has_no_status_or_execution_surface() {
    let source = include_str!("witnessed_suffix.rs");
    let forbidden = [
        "status: String",
        "status: &str",
        "string_status",
        "execute_import",
        "import_executor",
        "append_local_commit",
        "append_recorded_event",
        "append_provenance",
        "apply_to_worldline_state",
        "worldline_state_mut",
        "WorldlineRuntime",
        "tokio::runtime",
        "async_runtime",
        "TcpListener",
        "UdpSocket",
        "socket_listener",
    ];

    for term in forbidden {
        assert!(
            !source.contains(term),
            "forbidden evaluator surface: {term}"
        );
    }
}
