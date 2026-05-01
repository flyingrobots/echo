// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

use echo_wasm_abi::kernel_port as abi;

use crate::{
    ConflictReason, ProvenanceRef, ReadingResidualPosture, WitnessedSuffixAdmissionOutcome,
    WitnessedSuffixAdmissionRequest, WitnessedSuffixAdmissionResponse, WitnessedSuffixShell,
    WorldlineId, WorldlineTick,
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
