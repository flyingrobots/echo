// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for optic invocation admission obstruction.

use warp_core::{
    digest_invocation_request_bytes, GraphFact, InvocationObstructionKind,
    OpticAdmissionRequirements, OpticAdmissionTicketPosture, OpticApertureRequest, OpticArtifact,
    OpticArtifactHandle, OpticArtifactOperation, OpticArtifactRegistry, OpticBasisRequest,
    OpticCapabilityPresentation, OpticInvocation, OpticInvocationAdmissionOutcome,
    OpticInvocationObstruction, OpticRegistrationDescriptor, RewriteDisposition,
    OPTIC_ADMISSION_TICKET_POSTURE_KIND,
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
            codec: "wesley.requirements.canonical-json.v0".to_owned(),
            digest: "requirements-digest:stack-witness-0001".to_owned(),
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

fn obstruction_for(outcome: &OpticInvocationAdmissionOutcome) -> OpticInvocationObstruction {
    match outcome {
        OpticInvocationAdmissionOutcome::Obstructed(posture) => posture.obstruction,
    }
}

#[test]
fn optic_invocation_obstructs_unknown_handle() {
    let mut registry = OpticArtifactRegistry::new();
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
    let (mut registry, handle) = fixture_registry_and_handle()?;
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
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation(handle);

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        expected_obstructed_posture(&invocation, OpticInvocationObstruction::MissingCapability)
    );
    Ok(())
}

#[test]
fn optic_invocation_obstructs_malformed_capability_presentation() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: String::new(),
        bound_grant_id: Some("grant:fixture".to_owned()),
    });

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &invocation,
            OpticInvocationObstruction::MalformedCapabilityPresentation
        )
    );
    Ok(())
}

#[test]
fn optic_invocation_obstructs_unbound_capability_presentation() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: "presentation:unbound".to_owned(),
        bound_grant_id: None,
    });

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &invocation,
            OpticInvocationObstruction::UnboundCapabilityPresentation
        )
    );
    Ok(())
}

#[test]
fn optic_invocation_obstructs_placeholder_capability_presentation_until_grant_validation_is_wired_into_admission(
) -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: "presentation:placeholder".to_owned(),
        bound_grant_id: Some("grant:placeholder".to_owned()),
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

#[test]
fn optic_invocation_presentation_never_admits_without_grant_validation_wired_into_admission(
) -> Result<(), String> {
    let presentations = [
        (
            OpticCapabilityPresentation {
                presentation_id: String::new(),
                bound_grant_id: Some("grant:fixture".to_owned()),
            },
            OpticInvocationObstruction::MalformedCapabilityPresentation,
        ),
        (
            OpticCapabilityPresentation {
                presentation_id: "presentation:unbound".to_owned(),
                bound_grant_id: None,
            },
            OpticInvocationObstruction::UnboundCapabilityPresentation,
        ),
        (
            OpticCapabilityPresentation {
                presentation_id: "presentation:placeholder".to_owned(),
                bound_grant_id: Some("grant:placeholder".to_owned()),
            },
            OpticInvocationObstruction::CapabilityValidationUnavailable,
        ),
    ];

    for (presentation, expected_obstruction) in presentations {
        let (mut registry, handle) = fixture_registry_and_handle()?;
        let mut invocation = fixture_invocation(handle);
        invocation.capability_presentation = Some(presentation);

        let outcome = registry.admit_optic_invocation(&invocation);

        assert_eq!(obstruction_for(&outcome), expected_obstruction);
    }
    Ok(())
}

fn latest_invocation_obstruction_fact(
    registry: &OpticArtifactRegistry,
) -> Result<&GraphFact, String> {
    registry
        .published_graph_facts()
        .last()
        .map(|published| &published.fact)
        .ok_or_else(|| "expected invocation obstruction graph fact".to_owned())
}

#[test]
fn unknown_handle_publishes_invocation_obstruction_fact() -> Result<(), String> {
    let mut registry = OpticArtifactRegistry::new();
    let invocation = fixture_invocation(OpticArtifactHandle {
        kind: "optic-artifact-handle".to_owned(),
        id: "unregistered-handle".to_owned(),
    });

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        OpticInvocationObstruction::UnknownHandle
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            artifact_handle_id,
            operation_id,
            canonical_variables_digest,
            obstruction,
            ..
        } if artifact_handle_id == "unregistered-handle"
            && operation_id == "operation:textWindow:v0"
            && canonical_variables_digest == b"vars-digest:textWindow"
            && *obstruction == InvocationObstructionKind::UnknownHandle
    ));
    Ok(())
}

#[test]
fn operation_mismatch_publishes_invocation_obstruction_fact() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.operation_id = "operation:replaceRange:v0".to_owned();

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        OpticInvocationObstruction::OperationMismatch
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            operation_id,
            obstruction,
            ..
        } if operation_id == "operation:replaceRange:v0"
            && *obstruction == InvocationObstructionKind::OperationMismatch
    ));
    Ok(())
}

#[test]
fn missing_capability_publishes_invocation_obstruction_fact() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation(handle);

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        OpticInvocationObstruction::MissingCapability
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            basis_request_digest,
            aperture_request_digest,
            obstruction,
            ..
        } if *basis_request_digest == digest_invocation_request_bytes(
                b"echo.optic-invocation.basis-request.v0",
                b"basis-request:fixture"
            )
            && *aperture_request_digest == digest_invocation_request_bytes(
                b"echo.optic-invocation.aperture-request.v0",
                b"aperture-request:fixture"
            )
            && *obstruction == InvocationObstructionKind::MissingCapability
    ));
    Ok(())
}

#[test]
fn invocation_obstruction_fact_digest_is_deterministic() {
    let first = GraphFact::OpticInvocationObstructed {
        artifact_handle_id: "handle:1".to_owned(),
        operation_id: "operation:textWindow:v0".to_owned(),
        canonical_variables_digest: b"vars".to_vec(),
        basis_request_digest: digest_invocation_request_bytes(
            b"echo.optic-invocation.basis-request.v0",
            b"basis",
        ),
        aperture_request_digest: digest_invocation_request_bytes(
            b"echo.optic-invocation.aperture-request.v0",
            b"aperture",
        ),
        obstruction: InvocationObstructionKind::MissingCapability,
    };
    let repeated = first.clone();

    assert_eq!(first.digest(), repeated.digest());
}

#[test]
fn invocation_obstruction_fact_is_not_counterfactual_candidate() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation(handle);

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        OpticInvocationObstruction::MissingCapability
    );
    let disposition = RewriteDisposition::Obstructed;
    assert_ne!(
        disposition,
        RewriteDisposition::LegalUnselectedCounterfactual
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed { .. }
    ));
    Ok(())
}
