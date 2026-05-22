// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Contract obstruction taxonomy regression tests.

use warp_core::{
    ContractObstruction, ContractObstructionKind, ContractObstructionSubject, ObservationAt,
    ObservationError, ReadingResidualPosture, RuntimeError, SchedulerFaultId, WorldlineId,
    WorldlineTick,
};

#[test]
fn unsupported_query_maps_to_contract_query_obstruction() {
    let obstruction =
        ContractObstruction::from_observation_error(&ObservationError::UnsupportedQuery {
            query_id: 42,
        });

    assert_eq!(obstruction.kind, ContractObstructionKind::UnsupportedQuery);
    assert_eq!(
        obstruction.subject,
        ContractObstructionSubject::Query { query_id: 42 }
    );
}

#[test]
fn observation_budget_error_maps_to_budget_obstruction() {
    let obstruction =
        ContractObstruction::from_observation_error(&ObservationError::BudgetExceeded {
            max_payload_bytes: 8,
            payload_bytes: 16,
            max_witness_refs: 1,
            witness_refs: 1,
        });

    assert_eq!(obstruction.kind, ContractObstructionKind::BudgetExceeded);
    assert_eq!(obstruction.subject, ContractObstructionSubject::Unspecified);
}

#[test]
fn invalid_observation_basis_maps_to_stale_basis_obstruction() {
    let worldline_id = WorldlineId::from_bytes([7; 32]);
    let obstruction = ContractObstruction::from_observation_error(&ObservationError::InvalidTick {
        worldline_id,
        tick: WorldlineTick::from_raw(99),
    });

    assert_eq!(obstruction.kind, ContractObstructionKind::StaleBasis);
    assert_eq!(obstruction.subject, ContractObstructionSubject::Unspecified);
}

#[test]
fn unsupported_installed_mutation_maps_to_operation_obstruction() {
    let obstruction = ContractObstruction::from_runtime_error(
        &RuntimeError::UnsupportedInstalledContractMutation { op_id: 7001 },
    );

    assert_eq!(
        obstruction.kind,
        ContractObstructionKind::UnsupportedOperation
    );
    assert_eq!(
        obstruction.subject,
        ContractObstructionSubject::Operation { op_id: 7001 }
    );
}

#[test]
fn scheduler_fault_maps_to_runtime_fault_obstruction() {
    let fault_id = SchedulerFaultId::from_bytes([9; 32]);
    let obstruction = ContractObstruction::from_runtime_error(
        &RuntimeError::SchedulerRuntimeFaultActive(fault_id),
    );

    assert_eq!(obstruction.kind, ContractObstructionKind::RuntimeFault);
    assert_eq!(
        obstruction.subject,
        ContractObstructionSubject::SchedulerFault { fault_id }
    );
}

#[test]
fn residual_reading_posture_maps_to_residual_obstruction() {
    let reading_id = [11; 32];
    let obstruction = ContractObstruction::from_residual_posture(
        ReadingResidualPosture::Residual,
        Some(reading_id),
    );

    assert_eq!(
        obstruction.as_ref().map(|obstruction| obstruction.kind),
        Some(ContractObstructionKind::ResidualReading)
    );
    assert_eq!(
        obstruction.as_ref().map(|obstruction| &obstruction.subject),
        Some(&ContractObstructionSubject::Reading { reading_id })
    );
}

#[test]
fn complete_reading_posture_does_not_map_to_obstruction() {
    assert_eq!(
        ContractObstruction::from_residual_posture(ReadingResidualPosture::Complete, None),
        None
    );
}

#[test]
fn direct_missing_retention_constructor_is_typed() {
    let retention_id = [13; 32];
    let obstruction = ContractObstruction::missing_retention(retention_id);

    assert_eq!(obstruction.kind, ContractObstructionKind::MissingRetention);
    assert_eq!(
        obstruction.subject,
        ContractObstructionSubject::Retention { retention_id }
    );
}

#[test]
fn unavailable_observation_maps_to_stale_basis_obstruction() {
    let worldline_id = WorldlineId::from_bytes([17; 32]);
    let obstruction =
        ContractObstruction::from_observation_error(&ObservationError::ObservationUnavailable {
            worldline_id,
            at: ObservationAt::Frontier,
        });

    assert_eq!(obstruction.kind, ContractObstructionKind::StaleBasis);
}

#[test]
fn codec_failure_maps_to_runtime_fault_obstruction() {
    let obstruction = ContractObstruction::from_observation_error(&ObservationError::CodecFailure(
        "canonical encoding failed".to_owned(),
    ));

    assert_eq!(obstruction.kind, ContractObstructionKind::RuntimeFault);
}

#[test]
fn unrelated_runtime_tick_overflow_maps_to_runtime_fault_obstruction() {
    let obstruction = ContractObstruction::from_runtime_error(&RuntimeError::GlobalTickOverflow);

    assert_eq!(obstruction.kind, ContractObstructionKind::RuntimeFault);
}

#[test]
fn admission_related_runtime_error_maps_to_admission_obstruction() {
    let obstruction = ContractObstruction::from_runtime_error(
        &RuntimeError::TicketedIngressAlreadyStaged([19; 32]),
    );

    assert_eq!(
        obstruction.kind,
        ContractObstructionKind::AdmissionObstruction
    );
}
