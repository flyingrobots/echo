// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for optic invocation admission obstruction.

use warp_core::{
    OpticAdmissionRequirements, OpticAdmissionTicketPosture, OpticApertureRequest, OpticArtifact,
    OpticArtifactHandle, OpticArtifactOperation, OpticArtifactRegistry, OpticBasisRequest,
    OpticCapabilityPresentation, OpticInvocation, OpticInvocationAdmissionOutcome,
    OpticInvocationObstruction, OpticRegistrationDescriptor, OPTIC_ADMISSION_TICKET_POSTURE_KIND,
};

fn fixture_artifact() -> OpticArtifact {
    OpticArtifact {
        artifact_id: "optic-artifact:stack-witness-0001".to_owned(),
        artifact_hash: "artifact-hash:stack-witness-0001".to_owned(),
        schema_id: "schema:jedit-text-buffer-optic:v0".to_owned(),
        requirements_digest: "requirements-digest:stack-witness-0001".to_owned(),
        operation: OpticArtifactOperation {
            operation_id: "operation:textWindow:v0".to_owned(),
        },
        requirements: OpticAdmissionRequirements {
            bytes: b"fixture admission requirements".to_vec(),
        },
    }
}

fn fixture_descriptor() -> OpticRegistrationDescriptor {
    OpticRegistrationDescriptor {
        artifact_id: "optic-artifact:stack-witness-0001".to_owned(),
        artifact_hash: "artifact-hash:stack-witness-0001".to_owned(),
        schema_id: "schema:jedit-text-buffer-optic:v0".to_owned(),
        operation_id: "operation:textWindow:v0".to_owned(),
        requirements_digest: "requirements-digest:stack-witness-0001".to_owned(),
    }
}

fn fixture_registry_and_handle() -> Result<(OpticArtifactRegistry, OpticArtifactHandle), String> {
    let mut registry = OpticArtifactRegistry::new();
    let handle = registry
        .register_optic_artifact(fixture_artifact(), fixture_descriptor())
        .map_err(|err| format!("fixture descriptor should register: {err:?}"))?;
    Ok((registry, handle))
}

fn fixture_invocation(handle: OpticArtifactHandle) -> OpticInvocation {
    OpticInvocation {
        artifact_handle: handle,
        operation_id: "operation:textWindow:v0".to_owned(),
        canonical_variables_digest: b"vars-digest:textWindow".to_vec(),
        basis_request: OpticBasisRequest {
            bytes: b"basis-request:fixture".to_vec(),
        },
        aperture_request: OpticApertureRequest {
            bytes: b"aperture-request:fixture".to_vec(),
        },
        capability_presentation: None,
    }
}

fn expected_obstructed_posture(
    invocation: &OpticInvocation,
    obstruction: OpticInvocationObstruction,
) -> OpticInvocationAdmissionOutcome {
    OpticInvocationAdmissionOutcome::Obstructed(OpticAdmissionTicketPosture {
        kind: OPTIC_ADMISSION_TICKET_POSTURE_KIND.to_owned(),
        artifact_handle: invocation.artifact_handle.clone(),
        operation_id: invocation.operation_id.clone(),
        canonical_variables_digest: invocation.canonical_variables_digest.clone(),
        basis_request: invocation.basis_request.clone(),
        aperture_request: invocation.aperture_request.clone(),
        obstruction,
    })
}

#[test]
fn optic_invocation_obstructs_unknown_handle() {
    let registry = OpticArtifactRegistry::new();
    let invocation = fixture_invocation(OpticArtifactHandle {
        kind: "optic-artifact-handle".to_owned(),
        id: "unregistered-handle".to_owned(),
    });

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        expected_obstructed_posture(&invocation, OpticInvocationObstruction::UnknownHandle)
    );
}

#[test]
fn optic_invocation_obstructs_operation_mismatch() -> Result<(), String> {
    let (registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.operation_id = "operation:replaceRange:v0".to_owned();

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        expected_obstructed_posture(&invocation, OpticInvocationObstruction::OperationMismatch)
    );
    Ok(())
}

#[test]
fn optic_invocation_obstructs_missing_capability_for_registered_handle() -> Result<(), String> {
    let (registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation(handle);

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        expected_obstructed_posture(&invocation, OpticInvocationObstruction::MissingCapability)
    );
    Ok(())
}

#[test]
fn optic_invocation_obstructs_placeholder_capability_presentation_until_grant_validation_exists(
) -> Result<(), String> {
    let (registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: "presentation:placeholder".to_owned(),
    });

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &invocation,
            OpticInvocationObstruction::CapabilityValidationUnavailable
        )
    );
    Ok(())
}
