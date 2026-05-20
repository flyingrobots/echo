// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for optic invocation admission obstruction.

use warp_core::{
    digest_invocation_request_bytes, AuthorityContext, AuthorityPolicy, AuthorityPolicyEvaluation,
    CapabilityGrantIntent, CapabilityGrantIntentGate, CapabilityGrantValidationObstructionKind,
    GraphFact, InvocationObstructionKind, OpticAdmissionEvidenceAuthority,
    OpticAdmissionRequirements, OpticAdmissionTicketPosture, OpticApertureRequest, OpticArtifact,
    OpticArtifactHandle, OpticArtifactOperation, OpticArtifactRegistry, OpticBasisRequest,
    OpticBudgetRequest, OpticCapabilityPresentation, OpticInvocation,
    OpticInvocationAdmissionOutcome, OpticInvocationObstruction, OpticRegistrationDescriptor,
    PrincipalRef, RewriteDisposition, OPTIC_ADMISSION_TICKET_KIND,
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

fn fixture_evidence_authority() -> OpticAdmissionEvidenceAuthority {
    OpticAdmissionEvidenceAuthority::assume_runtime_owner()
}

fn principal(id: &str) -> PrincipalRef {
    PrincipalRef { id: id.to_owned() }
}

fn fixture_grant(grant_id: &str) -> CapabilityGrantIntent {
    CapabilityGrantIntent {
        intent_id: grant_id.to_owned(),
        proposed_by: principal("principal:issuer"),
        subject: principal("principal:jedit-session"),
        artifact_hash: "artifact-hash:stack-witness-0001".to_owned(),
        operation_id: "operation:textWindow:v0".to_owned(),
        requirements_digest: "requirements-digest:stack-witness-0001".to_owned(),
        rights: vec!["optic.invoke".to_owned()],
        scope_bytes: b"scope:fixture".to_vec(),
        expiry_bytes: Some(b"expiry:fixture".to_vec()),
        delegation_basis_bytes: Some(b"delegation-basis:fixture".to_vec()),
    }
}

fn fixture_authority_context() -> AuthorityContext {
    AuthorityContext {
        issuer: Some(principal("principal:issuer")),
        policy: Some(AuthorityPolicy {
            policy_id: "authority-policy:fixture".to_owned(),
        }),
        policy_evaluation: AuthorityPolicyEvaluation::Unsupported,
    }
}

fn fixture_gate_with_grant(grant: CapabilityGrantIntent) -> CapabilityGrantIntentGate {
    let mut gate = CapabilityGrantIntentGate::new();
    let _ = gate.submit_grant_intent(grant, fixture_authority_context());
    gate
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
        budget_request: OpticBudgetRequest {
            bytes: b"budget-request:fixture".to_vec(),
        },
        capability_presentation: None,
    }
}

fn fixture_invocation_with_resolved_basis_and_presentation(
    handle: OpticArtifactHandle,
    grant_id: &str,
) -> OpticInvocation {
    let mut invocation = fixture_invocation_with_presentation(handle, grant_id);
    invocation.basis_request = OpticBasisRequest {
        bytes: b"basis-request:resolved-fixture".to_vec(),
    };
    invocation
}

fn fixture_invocation_with_resolved_basis_aperture_and_presentation(
    handle: OpticArtifactHandle,
    grant_id: &str,
) -> OpticInvocation {
    let mut invocation = fixture_invocation_with_resolved_basis_and_presentation(handle, grant_id);
    invocation.aperture_request = OpticApertureRequest {
        bytes: b"aperture-request:resolved-fixture".to_vec(),
    };
    invocation
}

fn fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
    handle: OpticArtifactHandle,
    grant_id: &str,
) -> OpticInvocation {
    let mut invocation =
        fixture_invocation_with_resolved_basis_aperture_and_presentation(handle, grant_id);
    invocation.budget_request = OpticBudgetRequest {
        bytes: b"budget-request:resolved-fixture".to_vec(),
    };
    invocation
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
        budget_request: invocation.budget_request.clone(),
        obstruction,
    })
}

fn obstruction_for(
    outcome: &OpticInvocationAdmissionOutcome,
) -> Option<OpticInvocationObstruction> {
    match outcome {
        OpticInvocationAdmissionOutcome::Obstructed(posture) => Some(posture.obstruction),
        OpticInvocationAdmissionOutcome::Admitted(_) => None,
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
fn optic_invocation_obstructs_missing_basis_request() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.basis_request = OpticBasisRequest { bytes: Vec::new() };

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        expected_obstructed_posture(&invocation, OpticInvocationObstruction::MissingBasisRequest)
    );
    Ok(())
}

#[test]
fn optic_invocation_obstructs_missing_aperture_request() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.aperture_request = OpticApertureRequest { bytes: Vec::new() };

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &invocation,
            OpticInvocationObstruction::MissingApertureRequest
        )
    );
    Ok(())
}

#[test]
fn optic_invocation_obstructs_missing_budget_request() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.budget_request = OpticBudgetRequest { bytes: Vec::new() };

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        expected_obstructed_posture(
            &invocation,
            OpticInvocationObstruction::MissingBudgetRequest
        )
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

        assert_eq!(obstruction_for(&outcome), Some(expected_obstruction));
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

fn latest_validation_obstruction_fact(
    gate: &CapabilityGrantIntentGate,
) -> Result<&GraphFact, String> {
    gate.published_graph_facts()
        .last()
        .map(|published| &published.fact)
        .ok_or_else(|| "expected capability grant validation obstruction fact".to_owned())
}

fn fixture_invocation_with_presentation(
    handle: OpticArtifactHandle,
    grant_id: &str,
) -> OpticInvocation {
    let mut invocation = fixture_invocation(handle);
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: "presentation:fixture".to_owned(),
        bound_grant_id: Some(grant_id.to_owned()),
    });
    invocation
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
        Some(OpticInvocationObstruction::UnknownHandle)
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
fn invocation_with_unknown_grant_publishes_grant_validation_obstruction_fact() -> Result<(), String>
{
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation_with_presentation(handle, "grant:unknown");
    let mut gate = CapabilityGrantIntentGate::new();

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::CapabilityValidationUnavailable)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::CapabilityValidationUnavailable
    ));
    assert!(matches!(
        latest_validation_obstruction_fact(&gate)?,
        GraphFact::CapabilityGrantValidationObstructed {
            grant_id,
            obstruction,
            ..
        } if grant_id.as_deref() == Some("grant:unknown")
            && *obstruction == CapabilityGrantValidationObstructionKind::UnknownGrant
    ));
    Ok(())
}

#[test]
fn invocation_with_artifact_hash_mismatch_publishes_grant_validation_obstruction_fact(
) -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation_with_presentation(handle, "grant:artifact-mismatch");
    let mut grant = fixture_grant("grant:artifact-mismatch");
    grant.artifact_hash = "artifact-hash:other".to_owned();
    let mut gate = fixture_gate_with_grant(grant);

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::CapabilityValidationUnavailable)
    );
    assert!(matches!(
        latest_validation_obstruction_fact(&gate)?,
        GraphFact::CapabilityGrantValidationObstructed {
            grant_artifact_hash,
            obstruction,
            ..
        } if grant_artifact_hash.as_deref() == Some("artifact-hash:other")
            && *obstruction == CapabilityGrantValidationObstructionKind::ArtifactHashMismatch
    ));
    Ok(())
}

#[test]
fn invocation_with_operation_id_mismatch_publishes_grant_validation_obstruction_fact(
) -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation_with_presentation(handle, "grant:operation-mismatch");
    let mut grant = fixture_grant("grant:operation-mismatch");
    grant.operation_id = "operation:replaceRange:v0".to_owned();
    let mut gate = fixture_gate_with_grant(grant);

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::CapabilityValidationUnavailable)
    );
    assert!(matches!(
        latest_validation_obstruction_fact(&gate)?,
        GraphFact::CapabilityGrantValidationObstructed {
            grant_operation_id,
            obstruction,
            ..
        } if grant_operation_id.as_deref() == Some("operation:replaceRange:v0")
            && *obstruction == CapabilityGrantValidationObstructionKind::OperationIdMismatch
    ));
    Ok(())
}

#[test]
fn invocation_with_requirements_digest_mismatch_publishes_grant_validation_obstruction_fact(
) -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation_with_presentation(handle, "grant:requirements-mismatch");
    let mut grant = fixture_grant("grant:requirements-mismatch");
    grant.requirements_digest = "requirements-digest:other".to_owned();
    let mut gate = fixture_gate_with_grant(grant);

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::CapabilityValidationUnavailable)
    );
    assert!(matches!(
        latest_validation_obstruction_fact(&gate)?,
        GraphFact::CapabilityGrantValidationObstructed {
            grant_requirements_digest,
            obstruction,
            ..
        } if grant_requirements_digest.as_deref() == Some("requirements-digest:other")
            && *obstruction == CapabilityGrantValidationObstructionKind::RequirementsDigestMismatch
    ));
    Ok(())
}

#[test]
fn identity_covered_grant_obstructs_unsupported_basis_resolution_after_basis_aperture_and_budget_presence(
) -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation_with_presentation(handle, "grant:covered");
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBasisResolution)
    );
    assert!(gate.published_graph_facts().is_empty());
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::UnsupportedBasisResolution
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
        Some(OpticInvocationObstruction::OperationMismatch)
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
        Some(OpticInvocationObstruction::MissingCapability)
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
fn missing_basis_request_publishes_invocation_obstruction_fact() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.basis_request = OpticBasisRequest { bytes: Vec::new() };

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::MissingBasisRequest)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            basis_request_digest,
            obstruction,
            ..
        } if *basis_request_digest == digest_invocation_request_bytes(
                b"echo.optic-invocation.basis-request.v0",
                b""
            )
            && *obstruction == InvocationObstructionKind::MissingBasisRequest
    ));
    Ok(())
}

#[test]
fn aperture_obstruction_publishes_invocation_obstruction_fact() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.aperture_request = OpticApertureRequest { bytes: Vec::new() };

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::MissingApertureRequest)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            aperture_request_digest,
            obstruction,
            ..
        } if *aperture_request_digest == digest_invocation_request_bytes(
                b"echo.optic-invocation.aperture-request.v0",
                b""
            )
            && *obstruction == InvocationObstructionKind::MissingApertureRequest
    ));
    Ok(())
}

#[test]
fn budget_obstruction_publishes_invocation_obstruction_fact() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.budget_request = OpticBudgetRequest { bytes: Vec::new() };

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::MissingBudgetRequest)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            budget_request_digest,
            obstruction,
            ..
        } if *budget_request_digest == digest_invocation_request_bytes(
                b"echo.optic-invocation.budget-request.v0",
                b""
            )
            && *obstruction == InvocationObstructionKind::MissingBudgetRequest
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
        budget_request_digest: digest_invocation_request_bytes(
            b"echo.optic-invocation.budget-request.v0",
            b"budget",
        ),
        obstruction: InvocationObstructionKind::MissingCapability,
    };
    let repeated = first.clone();

    assert_eq!(first.digest(), repeated.digest());
}

#[test]
fn basis_obstruction_fact_digest_is_deterministic() {
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
        budget_request_digest: digest_invocation_request_bytes(
            b"echo.optic-invocation.budget-request.v0",
            b"budget",
        ),
        obstruction: InvocationObstructionKind::UnsupportedBasisResolution,
    };
    let repeated = first.clone();

    assert_eq!(first.digest(), repeated.digest());
}

#[test]
fn aperture_obstruction_fact_digest_is_deterministic() {
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
        budget_request_digest: digest_invocation_request_bytes(
            b"echo.optic-invocation.budget-request.v0",
            b"budget",
        ),
        obstruction: InvocationObstructionKind::MissingApertureRequest,
    };
    let repeated = first.clone();

    assert_eq!(first.digest(), repeated.digest());
}

#[test]
fn budget_obstruction_fact_digest_is_deterministic() {
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
        budget_request_digest: digest_invocation_request_bytes(
            b"echo.optic-invocation.budget-request.v0",
            b"budget",
        ),
        obstruction: InvocationObstructionKind::MissingBudgetRequest,
    };
    let repeated = first.clone();

    assert_eq!(first.digest(), repeated.digest());
}

#[test]
fn runtime_support_unavailable_is_unreachable_when_basis_resolution_fails() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation_with_presentation(handle, "grant:covered");
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBasisResolution)
    );
    assert_ne!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );
    let future_support_fact = GraphFact::OpticInvocationObstructed {
        artifact_handle_id: "handle:future".to_owned(),
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
        budget_request_digest: digest_invocation_request_bytes(
            b"echo.optic-invocation.budget-request.v0",
            b"budget",
        ),
        obstruction: InvocationObstructionKind::RuntimeSupportUnavailable,
    };
    assert_eq!(future_support_fact.digest(), future_support_fact.digest());
    Ok(())
}

#[test]
fn unsupported_basis_still_stops_at_unsupported_basis_resolution() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation_with_presentation(handle, "grant:covered");
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBasisResolution)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::UnsupportedBasisResolution
    ));
    Ok(())
}

#[test]
fn resolved_basis_still_obstructs_before_aperture_resolution() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation =
        fixture_invocation_with_resolved_basis_and_presentation(handle, "grant:covered");
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedApertureResolution)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            basis_request_digest,
            obstruction,
            ..
        } if *basis_request_digest == digest_invocation_request_bytes(
                b"echo.optic-invocation.basis-request.v0",
                b"basis-request:resolved-fixture"
            )
            && *obstruction == InvocationObstructionKind::UnsupportedApertureResolution
    ));
    Ok(())
}

#[test]
fn unsupported_aperture_still_stops_at_unsupported_aperture_resolution() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation =
        fixture_invocation_with_resolved_basis_and_presentation(handle, "grant:covered");
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedApertureResolution)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            aperture_request_digest,
            obstruction,
            ..
        } if *aperture_request_digest == digest_invocation_request_bytes(
                b"echo.optic-invocation.aperture-request.v0",
                b"aperture-request:fixture"
            )
            && *obstruction == InvocationObstructionKind::UnsupportedApertureResolution
    ));
    Ok(())
}

#[test]
fn aperture_resolution_is_unreachable_when_basis_resolution_fails() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation_with_presentation(handle, "grant:covered");
    invocation.aperture_request = OpticApertureRequest {
        bytes: b"aperture-request:resolved-fixture".to_vec(),
    };
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBasisResolution)
    );
    assert_ne!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedApertureResolution)
    );
    Ok(())
}

#[test]
fn resolved_aperture_still_obstructs_before_budget_resolution() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation =
        fixture_invocation_with_resolved_basis_aperture_and_presentation(handle, "grant:covered");
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBudgetResolution)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            aperture_request_digest,
            obstruction,
            ..
        } if *aperture_request_digest == digest_invocation_request_bytes(
                b"echo.optic-invocation.aperture-request.v0",
                b"aperture-request:resolved-fixture"
            )
            && *obstruction == InvocationObstructionKind::UnsupportedBudgetResolution
    ));
    Ok(())
}

#[test]
fn unsupported_budget_still_stops_at_unsupported_budget_resolution() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation =
        fixture_invocation_with_resolved_basis_aperture_and_presentation(handle, "grant:covered");
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBudgetResolution)
    );
    assert_ne!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );
    Ok(())
}

#[test]
fn budget_resolution_is_unreachable_when_basis_resolution_fails() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    invocation.basis_request = OpticBasisRequest {
        bytes: b"basis-request:unsupported".to_vec(),
    };
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBasisResolution)
    );
    assert_ne!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );
    Ok(())
}

#[test]
fn budget_resolution_is_unreachable_when_aperture_resolution_fails() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    invocation.aperture_request = OpticApertureRequest {
        bytes: b"aperture-request:unsupported".to_vec(),
    };
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedApertureResolution)
    );
    assert_ne!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );
    Ok(())
}

#[test]
fn resolved_budget_still_requires_echo_owned_runtime_support() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            budget_request_digest,
            obstruction,
            ..
        } if *budget_request_digest == digest_invocation_request_bytes(
                b"echo.optic-invocation.budget-request.v0",
                b"budget-request:resolved-fixture"
            )
            && *obstruction == InvocationObstructionKind::RuntimeSupportUnavailable
    ));
    Ok(())
}

#[test]
fn runtime_support_is_checked_only_after_budget_resolution() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let mut unsupported_basis =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    unsupported_basis.basis_request = OpticBasisRequest {
        bytes: b"basis-request:unsupported".to_vec(),
    };
    let outcome =
        registry.admit_optic_invocation_with_capability_validator(&unsupported_basis, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBasisResolution)
    );

    let mut unsupported_aperture =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    unsupported_aperture.aperture_request = OpticApertureRequest {
        bytes: b"aperture-request:unsupported".to_vec(),
    };
    let outcome =
        registry.admit_optic_invocation_with_capability_validator(&unsupported_aperture, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedApertureResolution)
    );

    let mut unsupported_budget =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle,
            "grant:covered",
        );
    unsupported_budget.budget_request = OpticBudgetRequest {
        bytes: b"budget-request:unsupported".to_vec(),
    };
    let outcome =
        registry.admit_optic_invocation_with_capability_validator(&unsupported_budget, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBudgetResolution)
    );
    Ok(())
}

#[test]
fn runtime_support_resolves_only_echo_owned_fixture() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );

    registry
        .record_runtime_support_fixture_for_artifact(
            &fixture_evidence_authority(),
            &invocation.artifact_handle,
        )
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::InvocationAdmissionUnavailable)
    );
    Ok(())
}

#[test]
fn caller_cannot_supply_runtime_support_testimony() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    invocation.canonical_variables_digest = b"runtime-support:resolved-fixture".to_vec();
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: "runtime-support:resolved-fixture".to_owned(),
        bound_grant_id: Some("grant:covered".to_owned()),
    });
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );

    registry
        .record_runtime_support_fixture_for_artifact(
            &fixture_evidence_authority(),
            &invocation.artifact_handle,
        )
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::InvocationAdmissionUnavailable)
    );
    Ok(())
}

#[test]
fn resolved_runtime_support_still_does_not_admit_invocation() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert!(matches!(
        outcome,
        OpticInvocationAdmissionOutcome::Obstructed(OpticAdmissionTicketPosture {
            obstruction: OpticInvocationObstruction::InvocationAdmissionUnavailable,
            ..
        })
    ));
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::InvocationAdmissionUnavailable
    ));
    assert!(registry.published_graph_facts().iter().all(|published| {
        matches!(
            published.fact,
            GraphFact::ArtifactRegistered { .. }
                | GraphFact::RuntimeSupportRecorded { .. }
                | GraphFact::OpticInvocationObstructed { .. }
        )
    }));
    Ok(())
}

#[test]
fn identity_coverage_and_runtime_support_still_require_invocation_admission() -> Result<(), String>
{
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::InvocationAdmissionUnavailable)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::InvocationAdmissionUnavailable
    ));
    Ok(())
}

#[test]
fn caller_cannot_supply_invocation_admission_testimony() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    invocation.canonical_variables_digest = b"invocation-admission:resolved-fixture".to_vec();
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: "invocation-admission:resolved-fixture".to_owned(),
        bound_grant_id: Some("grant:covered".to_owned()),
    });
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::InvocationAdmissionUnavailable)
    );

    registry
        .record_invocation_admission_fixture_for_artifact(
            &fixture_evidence_authority(),
            &invocation.artifact_handle,
        )
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerAdmissionUnavailable)
    );
    Ok(())
}

#[test]
fn invocation_admission_uses_echo_owned_admission_fixture_only() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    invocation.basis_request.bytes = b"basis-request:resolved-fixture".to_vec();
    invocation.aperture_request.bytes = b"aperture-request:resolved-fixture".to_vec();
    invocation.budget_request.bytes = b"budget-request:resolved-fixture".to_vec();
    invocation.canonical_variables_digest = b"caller-claims:invocation-admission-resolved".to_vec();
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::InvocationAdmissionUnavailable)
    );

    registry
        .record_invocation_admission_fixture_for_artifact(
            &fixture_evidence_authority(),
            &invocation.artifact_handle,
        )
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerAdmissionUnavailable)
    );
    Ok(())
}

#[test]
fn invocation_admission_is_checked_only_after_runtime_support() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let mut unsupported_basis =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    unsupported_basis.basis_request = OpticBasisRequest {
        bytes: b"basis-request:unsupported".to_vec(),
    };
    let outcome =
        registry.admit_optic_invocation_with_capability_validator(&unsupported_basis, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBasisResolution)
    );

    let mut unsupported_aperture =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    unsupported_aperture.aperture_request = OpticApertureRequest {
        bytes: b"aperture-request:unsupported".to_vec(),
    };
    let outcome =
        registry.admit_optic_invocation_with_capability_validator(&unsupported_aperture, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedApertureResolution)
    );

    let mut unsupported_budget =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    unsupported_budget.budget_request = OpticBudgetRequest {
        bytes: b"budget-request:unsupported".to_vec(),
    };
    let outcome =
        registry.admit_optic_invocation_with_capability_validator(&unsupported_budget, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBudgetResolution)
    );

    let supported_shape_without_support_fact =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle,
            "grant:covered",
        );
    let outcome = registry.admit_optic_invocation_with_capability_validator(
        &supported_shape_without_support_fact,
        &mut gate,
    );
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );
    Ok(())
}

#[test]
fn resolved_invocation_admission_advances_to_scheduler_admission_unavailable() -> Result<(), String>
{
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert!(matches!(
        outcome,
        OpticInvocationAdmissionOutcome::Obstructed(OpticAdmissionTicketPosture {
            obstruction: OpticInvocationObstruction::SchedulerAdmissionUnavailable,
            ..
        })
    ));
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::SchedulerAdmissionUnavailable
    ));
    assert!(registry.published_graph_facts().iter().all(|published| {
        matches!(
            published.fact,
            GraphFact::ArtifactRegistered { .. }
                | GraphFact::RuntimeSupportRecorded { .. }
                | GraphFact::InvocationAdmissionRecorded { .. }
                | GraphFact::OpticInvocationObstructed { .. }
        )
    }));
    Ok(())
}

#[test]
fn resolved_invocation_admission_still_requires_scheduler_admission() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerAdmissionUnavailable)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::SchedulerAdmissionUnavailable
    ));
    Ok(())
}

#[test]
fn caller_cannot_supply_scheduler_admission_testimony() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "scheduler-admission:resolved-fixture",
    );
    invocation.canonical_variables_digest = b"scheduler-admission:resolved-fixture".to_vec();
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: "scheduler-admission:resolved-fixture".to_owned(),
        bound_grant_id: Some("scheduler-admission:resolved-fixture".to_owned()),
    });
    let mut gate = fixture_gate_with_grant(fixture_grant("scheduler-admission:resolved-fixture"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerAdmissionUnavailable)
    );

    registry
        .record_scheduler_admission_fixture_for_artifact(
            &fixture_evidence_authority(),
            &invocation.artifact_handle,
        )
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerWorkUnavailable)
    );
    Ok(())
}

#[test]
fn scheduler_admission_uses_echo_owned_fixture_only() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    invocation.canonical_variables_digest = b"caller-claims:scheduler-admission-resolved".to_vec();
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerAdmissionUnavailable)
    );

    registry
        .record_scheduler_admission_fixture_for_artifact(
            &fixture_evidence_authority(),
            &invocation.artifact_handle,
        )
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerWorkUnavailable)
    );
    Ok(())
}

#[test]
fn scheduler_admission_is_checked_only_after_invocation_admission() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let mut unsupported_basis =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    unsupported_basis.basis_request = OpticBasisRequest {
        bytes: b"basis-request:unsupported".to_vec(),
    };
    let outcome =
        registry.admit_optic_invocation_with_capability_validator(&unsupported_basis, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBasisResolution)
    );

    let mut unsupported_aperture =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    unsupported_aperture.aperture_request = OpticApertureRequest {
        bytes: b"aperture-request:unsupported".to_vec(),
    };
    let outcome =
        registry.admit_optic_invocation_with_capability_validator(&unsupported_aperture, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedApertureResolution)
    );

    let mut unsupported_budget =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    unsupported_budget.budget_request = OpticBudgetRequest {
        bytes: b"budget-request:unsupported".to_vec(),
    };
    let outcome =
        registry.admit_optic_invocation_with_capability_validator(&unsupported_budget, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::UnsupportedBudgetResolution)
    );

    let supported_shape_without_support_fact =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    let outcome = registry.admit_optic_invocation_with_capability_validator(
        &supported_shape_without_support_fact,
        &mut gate,
    );
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );

    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    let supported_shape_without_invocation_admission =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle,
            "grant:covered",
        );
    let outcome = registry.admit_optic_invocation_with_capability_validator(
        &supported_shape_without_invocation_admission,
        &mut gate,
    );
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::InvocationAdmissionUnavailable)
    );
    Ok(())
}

#[test]
fn resolved_scheduler_admission_advances_to_scheduler_work_unavailable() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert!(matches!(
        outcome,
        OpticInvocationAdmissionOutcome::Obstructed(OpticAdmissionTicketPosture {
            obstruction: OpticInvocationObstruction::SchedulerWorkUnavailable,
            ..
        })
    ));
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::SchedulerWorkUnavailable
    ));
    assert!(registry.published_graph_facts().iter().all(|published| {
        matches!(
            published.fact,
            GraphFact::ArtifactRegistered { .. }
                | GraphFact::RuntimeSupportRecorded { .. }
                | GraphFact::InvocationAdmissionRecorded { .. }
                | GraphFact::SchedulerAdmissionRecorded { .. }
                | GraphFact::OpticInvocationObstructed { .. }
        )
    }));
    Ok(())
}

#[test]
fn resolved_scheduler_admission_still_requires_scheduler_work_candidate() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerWorkUnavailable)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::SchedulerWorkUnavailable
    ));
    Ok(())
}

#[test]
fn caller_cannot_supply_scheduler_work_candidate_testimony() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "scheduler-work-candidate:resolved-fixture",
    );
    invocation.canonical_variables_digest = b"scheduler-work-candidate:resolved-fixture".to_vec();
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: "scheduler-work-candidate:resolved-fixture".to_owned(),
        bound_grant_id: Some("scheduler-work-candidate:resolved-fixture".to_owned()),
    });
    let mut gate =
        fixture_gate_with_grant(fixture_grant("scheduler-work-candidate:resolved-fixture"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerWorkUnavailable)
    );

    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &invocation.artifact_handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::LawWitnessUnavailable)
    );
    Ok(())
}

#[test]
fn scheduler_work_candidate_uses_echo_owned_fixture_only() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    invocation.canonical_variables_digest =
        b"caller-claims:scheduler-work-candidate-resolved".to_vec();
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerWorkUnavailable)
    );

    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &invocation.artifact_handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::LawWitnessUnavailable)
    );
    Ok(())
}

#[test]
fn scheduler_work_candidate_is_checked_only_after_scheduler_admission() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let supported_shape_without_support_fact =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    let outcome = registry.admit_optic_invocation_with_capability_validator(
        &supported_shape_without_support_fact,
        &mut gate,
    );
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );

    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    let supported_shape_without_invocation_admission =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    let outcome = registry.admit_optic_invocation_with_capability_validator(
        &supported_shape_without_invocation_admission,
        &mut gate,
    );
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::InvocationAdmissionUnavailable)
    );

    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    let supported_shape_without_scheduler_admission =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle,
            "grant:covered",
        );
    let outcome = registry.admit_optic_invocation_with_capability_validator(
        &supported_shape_without_scheduler_admission,
        &mut gate,
    );
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerAdmissionUnavailable)
    );
    Ok(())
}

#[test]
fn resolved_scheduler_work_candidate_advances_to_law_witness_unavailable() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert!(matches!(
        outcome,
        OpticInvocationAdmissionOutcome::Obstructed(OpticAdmissionTicketPosture {
            obstruction: OpticInvocationObstruction::LawWitnessUnavailable,
            ..
        })
    ));
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::LawWitnessUnavailable
    ));
    assert!(registry.published_graph_facts().iter().all(|published| {
        matches!(
            published.fact,
            GraphFact::ArtifactRegistered { .. }
                | GraphFact::RuntimeSupportRecorded { .. }
                | GraphFact::InvocationAdmissionRecorded { .. }
                | GraphFact::SchedulerAdmissionRecorded { .. }
                | GraphFact::SchedulerWorkCandidateRecorded { .. }
                | GraphFact::OpticInvocationObstructed { .. }
        )
    }));
    Ok(())
}

#[test]
fn scheduler_work_candidate_fixture_publishes_graph_fact_once() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;

    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &handle,
        )
        .map_err(|err| {
            format!(
                "registered handle should record scheduler work candidate idempotently: {err:?}"
            )
        })?;

    let facts = registry
        .published_graph_facts()
        .iter()
        .filter(|published| {
            matches!(
                published.fact,
                GraphFact::SchedulerWorkCandidateRecorded { .. }
            )
        })
        .count();
    assert_eq!(facts, 1);
    Ok(())
}

#[test]
fn scheduler_work_candidate_fixture_rejects_unknown_handle_without_graph_fact() {
    let mut registry = OpticArtifactRegistry::new();
    let unknown = OpticArtifactHandle {
        kind: "optic-artifact-handle".to_owned(),
        id: "unregistered-handle".to_owned(),
    };

    let result = registry.record_scheduler_work_candidate_fixture_for_artifact(
        &fixture_evidence_authority(),
        &unknown,
    );

    assert_eq!(
        result,
        Err(warp_core::OpticArtifactRegistrationError::UnknownHandle)
    );
    assert!(registry.published_graph_facts().is_empty());
}

#[test]
fn resolved_scheduler_work_candidate_still_requires_law_witness() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::LawWitnessUnavailable)
    );
    assert!(matches!(
        latest_invocation_obstruction_fact(&registry)?,
        GraphFact::OpticInvocationObstructed {
            obstruction,
            ..
        } if *obstruction == InvocationObstructionKind::LawWitnessUnavailable
    ));
    Ok(())
}

#[test]
fn caller_cannot_supply_law_witness_testimony() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "law-witness:resolved-fixture",
    );
    invocation.canonical_variables_digest = b"law-witness:resolved-fixture".to_vec();
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: "law-witness:resolved-fixture".to_owned(),
        bound_grant_id: Some("law-witness:resolved-fixture".to_owned()),
    });
    let mut gate = fixture_gate_with_grant(fixture_grant("law-witness:resolved-fixture"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::LawWitnessUnavailable)
    );

    registry
        .record_law_witness_fixture_for_artifact(
            &fixture_evidence_authority(),
            &invocation.artifact_handle,
        )
        .map_err(|err| format!("registered handle should record law witness: {err:?}"))?;
    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert!(matches!(
        outcome,
        OpticInvocationAdmissionOutcome::Admitted(_)
    ));
    Ok(())
}

#[test]
fn law_witness_uses_echo_owned_fixture_only() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    let mut invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    invocation.canonical_variables_digest = b"caller-claims:law-witness-resolved".to_vec();
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::LawWitnessUnavailable)
    );

    registry
        .record_law_witness_fixture_for_artifact(
            &fixture_evidence_authority(),
            &invocation.artifact_handle,
        )
        .map_err(|err| format!("registered handle should record law witness: {err:?}"))?;
    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);
    assert!(matches!(
        outcome,
        OpticInvocationAdmissionOutcome::Admitted(_)
    ));
    Ok(())
}

#[test]
fn law_witness_is_checked_only_after_scheduler_work_candidate() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_law_witness_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record law witness: {err:?}"))?;
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let supported_shape_without_support_fact =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle.clone(),
            "grant:covered",
        );
    let outcome = registry.admit_optic_invocation_with_capability_validator(
        &supported_shape_without_support_fact,
        &mut gate,
    );
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::RuntimeSupportUnavailable)
    );

    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    let supported_shape_without_scheduler_work =
        fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
            handle,
            "grant:covered",
        );
    let outcome = registry.admit_optic_invocation_with_capability_validator(
        &supported_shape_without_scheduler_work,
        &mut gate,
    );
    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::SchedulerWorkUnavailable)
    );
    Ok(())
}

#[test]
fn law_witness_fixture_publishes_graph_fact_once() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;

    registry
        .record_law_witness_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record law witness: {err:?}"))?;
    registry
        .record_law_witness_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| {
            format!("registered handle should record law witness idempotently: {err:?}")
        })?;

    let facts = registry
        .published_graph_facts()
        .iter()
        .filter(|published| matches!(published.fact, GraphFact::LawWitnessRecorded { .. }))
        .count();
    assert_eq!(facts, 1);
    Ok(())
}

#[test]
fn law_witness_fixture_rejects_unknown_handle_without_graph_fact() {
    let mut registry = OpticArtifactRegistry::new();
    let unknown = OpticArtifactHandle {
        kind: "optic-artifact-handle".to_owned(),
        id: "unregistered-handle".to_owned(),
    };

    let result =
        registry.record_law_witness_fixture_for_artifact(&fixture_evidence_authority(), &unknown);

    assert_eq!(
        result,
        Err(warp_core::OpticArtifactRegistrationError::UnknownHandle)
    );
    assert!(registry.published_graph_facts().is_empty());
}

#[test]
fn resolved_law_witness_issues_admission_ticket() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    registry
        .record_law_witness_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record law witness: {err:?}"))?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    let OpticInvocationAdmissionOutcome::Admitted(ticket) = outcome else {
        return Err("resolved law witness should issue an admission ticket".to_owned());
    };
    assert_eq!(ticket.kind, OPTIC_ADMISSION_TICKET_KIND);
    assert_eq!(ticket.artifact_handle, invocation.artifact_handle);
    assert_eq!(ticket.operation_id, invocation.operation_id);
    assert_eq!(
        ticket.canonical_variables_digest,
        invocation.canonical_variables_digest
    );
    assert!(registry
        .published_graph_facts()
        .iter()
        .any(|published| { matches!(published.fact, GraphFact::AdmissionTicketIssued { .. }) }));
    Ok(())
}

#[test]
fn admission_ticket_binds_law_witness() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    registry
        .record_law_witness_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record law witness: {err:?}"))?;
    let law_witness_digest = registry
        .published_graph_facts()
        .iter()
        .find_map(|published| match &published.fact {
            GraphFact::LawWitnessRecorded {
                law_witness_digest, ..
            } => Some(*law_witness_digest),
            _ => None,
        })
        .ok_or_else(|| "law witness fact should be published".to_owned())?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    let OpticInvocationAdmissionOutcome::Admitted(ticket) = outcome else {
        return Err("resolved law witness should issue an admission ticket".to_owned());
    };
    assert_eq!(ticket.law_witness_digest, law_witness_digest);
    assert!(matches!(
        registry.published_graph_facts().last().map(|published| &published.fact),
        Some(GraphFact::AdmissionTicketIssued {
            artifact_handle_id,
            artifact_hash,
            operation_id,
            requirements_digest,
            canonical_variables_digest,
            basis_request_digest,
            aperture_request_digest,
            budget_request_digest,
            law_witness_digest: published_law_witness_digest,
            ticket_digest,
            ..
        }) if artifact_handle_id == &ticket.artifact_handle.id
            && artifact_hash == &ticket.artifact_hash
            && operation_id == &ticket.operation_id
            && requirements_digest == &ticket.requirements_digest
            && canonical_variables_digest == &ticket.canonical_variables_digest
            && basis_request_digest == &ticket.basis_request_digest
            && aperture_request_digest == &ticket.aperture_request_digest
            && budget_request_digest == &ticket.budget_request_digest
            && *published_law_witness_digest == law_witness_digest
            && ticket_digest == &ticket.ticket_digest
    ));
    Ok(())
}

#[test]
fn admission_ticket_does_not_publish_obstruction_or_runtime_motion() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    registry
        .record_runtime_support_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record runtime support: {err:?}"))?;
    registry
        .record_invocation_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record invocation admission: {err:?}"))?;
    registry
        .record_scheduler_admission_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record scheduler admission: {err:?}"))?;
    registry
        .record_scheduler_work_candidate_fixture_for_artifact(
            &fixture_evidence_authority(),
            &handle,
        )
        .map_err(|err| {
            format!("registered handle should record scheduler work candidate: {err:?}")
        })?;
    registry
        .record_law_witness_fixture_for_artifact(&fixture_evidence_authority(), &handle)
        .map_err(|err| format!("registered handle should record law witness: {err:?}"))?;
    let invocation = fixture_invocation_with_resolved_basis_aperture_budget_and_presentation(
        handle,
        "grant:covered",
    );
    let mut gate = fixture_gate_with_grant(fixture_grant("grant:covered"));

    let outcome = registry.admit_optic_invocation_with_capability_validator(&invocation, &mut gate);

    assert!(matches!(
        outcome,
        OpticInvocationAdmissionOutcome::Admitted(_)
    ));
    assert!(registry.published_graph_facts().iter().all(|published| {
        !matches!(published.fact, GraphFact::OpticInvocationObstructed { .. })
    }));
    assert!(registry.published_graph_facts().iter().all(|published| {
        matches!(
            published.fact,
            GraphFact::ArtifactRegistered { .. }
                | GraphFact::RuntimeSupportRecorded { .. }
                | GraphFact::InvocationAdmissionRecorded { .. }
                | GraphFact::SchedulerAdmissionRecorded { .. }
                | GraphFact::SchedulerWorkCandidateRecorded { .. }
                | GraphFact::LawWitnessRecorded { .. }
                | GraphFact::AdmissionTicketIssued { .. }
        )
    }));
    Ok(())
}

#[test]
fn invocation_obstruction_fact_is_not_counterfactual_candidate() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let invocation = fixture_invocation(handle);

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::MissingCapability)
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

#[test]
fn basis_obstruction_is_not_counterfactual_candidate() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.basis_request = OpticBasisRequest { bytes: Vec::new() };

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::MissingBasisRequest)
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

#[test]
fn aperture_obstruction_is_not_counterfactual_candidate() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.aperture_request = OpticApertureRequest { bytes: Vec::new() };

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::MissingApertureRequest)
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

#[test]
fn budget_obstruction_is_not_counterfactual_candidate() -> Result<(), String> {
    let (mut registry, handle) = fixture_registry_and_handle()?;
    let mut invocation = fixture_invocation(handle);
    invocation.budget_request = OpticBudgetRequest { bytes: Vec::new() };

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        obstruction_for(&outcome),
        Some(OpticInvocationObstruction::MissingBudgetRequest)
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
