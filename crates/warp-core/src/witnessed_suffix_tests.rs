// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

use echo_wasm_abi::kernel_port as abi;

use crate::{
    evaluate_witnessed_suffix_admission, ConflictReason, Hash, ProvenanceRef,
    ReadingResidualPosture, WitnessedSuffixAdmissionContext, WitnessedSuffixAdmissionOutcome,
    WitnessedSuffixAdmissionRequest, WitnessedSuffixAdmissionResponse,
    WitnessedSuffixLocalAdmissionPosture, WitnessedSuffixShell, WorldlineId, WorldlineTick,
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
        source_suffix: shell_with_entries(vec![provenance_ref(4, 3)]),
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

fn clean_context(posture: WitnessedSuffixLocalAdmissionPosture) -> FakeAdmissionContext {
    FakeAdmissionContext {
        expected_shell_digest: Some([6; 32]),
        resolved_target_basis: Some(provenance_ref(12, 9)),
        posture,
    }
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
            worldline_id: abi::WorldlineId::from_bytes([4; 32]),
            worldline_tick: abi::WorldlineTick(3),
            commit_hash: vec![5; 32],
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
fn witnessed_suffix_evaluator_admits_clean_suffix() {
    let request = request();
    let context = clean_context(WitnessedSuffixLocalAdmissionPosture::Admissible {
        admitted_refs: vec![provenance_ref(30, 10)],
    });

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
}

#[test]
fn witnessed_suffix_evaluator_stages_boundary_only_suffix() {
    let request = WitnessedSuffixAdmissionRequest {
        source_suffix: shell_with_entries(Vec::new()),
        target_worldline_id: worldline(11),
        target_basis: provenance_ref(12, 9),
        basis_report: None,
    };
    let context = clean_context(WitnessedSuffixLocalAdmissionPosture::Staged {
        staged_refs: vec![provenance_ref(5, 1)],
    });

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Staged { staged_refs, .. }
            if staged_refs == vec![provenance_ref(5, 1)]
    ));
}

#[test]
fn witnessed_suffix_evaluator_obstructs_empty_suffix_without_boundary_witness() {
    let mut request = WitnessedSuffixAdmissionRequest {
        source_suffix: shell_with_entries(Vec::new()),
        target_worldline_id: worldline(11),
        target_basis: provenance_ref(12, 9),
        basis_report: None,
    };
    request.source_suffix.boundary_witness = None;
    let context = clean_context(WitnessedSuffixLocalAdmissionPosture::Staged {
        staged_refs: Vec::new(),
    });

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        }
    ));
}

#[test]
fn witnessed_suffix_evaluator_allows_equal_start_and_end_ticks() {
    let mut request = request();
    request.source_suffix.source_suffix_end_tick =
        Some(request.source_suffix.source_suffix_start_tick);
    let context = clean_context(WitnessedSuffixLocalAdmissionPosture::Admissible {
        admitted_refs: vec![provenance_ref(30, 10)],
    });

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
}

#[test]
fn witnessed_suffix_evaluator_stages_when_target_basis_is_boundary_witness() {
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
        posture: WitnessedSuffixLocalAdmissionPosture::Staged {
            staged_refs: vec![boundary_witness],
        },
    };

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert_eq!(response.target_basis, boundary_witness);
    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Staged { staged_refs, .. }
            if staged_refs == vec![boundary_witness]
    ));
}

#[test]
fn witnessed_suffix_evaluator_preserves_plural_outcome() {
    let request = request();
    let candidates = vec![provenance_ref(33, 12), provenance_ref(34, 13)];
    let context = clean_context(WitnessedSuffixLocalAdmissionPosture::Plural {
        candidate_refs: candidates.clone(),
    });

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Plural {
            candidate_refs,
            residual_posture: ReadingResidualPosture::PluralityPreserved,
            ..
        } if candidate_refs == candidates
    ));
}

#[test]
fn witnessed_suffix_evaluator_conflicts_with_adverse_admission_law() {
    let request = request();
    let context = clean_context(WitnessedSuffixLocalAdmissionPosture::Conflict {
        reason: ConflictReason::ParentFootprintOverlap,
        source_ref: provenance_ref(35, 14),
        evidence_digest: [36; 32],
    });

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
fn witnessed_suffix_evaluator_obstructs_mismatched_digest() {
    let request = request();
    let context = FakeAdmissionContext {
        expected_shell_digest: Some([99; 32]),
        resolved_target_basis: Some(provenance_ref(12, 9)),
        posture: WitnessedSuffixLocalAdmissionPosture::Admissible {
            admitted_refs: vec![provenance_ref(30, 10)],
        },
    };

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        }
    ));
}

#[test]
fn witnessed_suffix_evaluator_obstructs_unknown_target_basis() {
    let request = request();
    let context = FakeAdmissionContext {
        expected_shell_digest: Some([6; 32]),
        resolved_target_basis: None,
        posture: WitnessedSuffixLocalAdmissionPosture::Admissible {
            admitted_refs: vec![provenance_ref(30, 10)],
        },
    };

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        }
    ));
}

#[test]
fn witnessed_suffix_evaluator_obstructs_inconsistent_bounds() {
    let mut request = request();
    request.source_suffix.source_suffix_start_tick = tick(5);
    request.source_suffix.source_suffix_end_tick = Some(tick(4));
    let context = clean_context(WitnessedSuffixLocalAdmissionPosture::Admissible {
        admitted_refs: vec![provenance_ref(30, 10)],
    });

    let response = evaluate_witnessed_suffix_admission(&request, &context);

    assert!(matches!(
        response.outcome,
        WitnessedSuffixAdmissionOutcome::Obstructed {
            residual_posture: ReadingResidualPosture::Obstructed,
            ..
        }
    ));
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
