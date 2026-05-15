// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for capability grant validation obstruction facts.

use warp_core::{
    AuthorityContext, AuthorityPolicy, AuthorityPolicyEvaluation, CapabilityGrantExpiryPosture,
    CapabilityGrantIntent, CapabilityGrantIntentGate, CapabilityGrantValidationObstruction,
    CapabilityGrantValidationOutcome, CapabilityGrantValidationPosture, GraphFact,
    OpticAdmissionRequirements, OpticArtifact, OpticArtifactOperation, OpticArtifactRegistry,
    OpticCapabilityPresentation, OpticRegistrationDescriptor, PrincipalRef,
};

fn principal(id: &str) -> PrincipalRef {
    PrincipalRef { id: id.to_owned() }
}

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

fn fixture_presentation(grant_id: &str) -> OpticCapabilityPresentation {
    OpticCapabilityPresentation {
        presentation_id: "presentation:fixture".to_owned(),
        bound_grant_id: Some(grant_id.to_owned()),
    }
}

fn fixture_registered_artifact() -> Result<warp_core::RegisteredOpticArtifact, String> {
    let mut registry = OpticArtifactRegistry::new();
    let handle = registry
        .register_optic_artifact(fixture_artifact(), fixture_descriptor())
        .map_err(|err| format!("fixture descriptor should register: {err:?}"))?;
    registry
        .resolve_optic_artifact_handle(&handle)
        .cloned()
        .map_err(|err| format!("registered handle should resolve: {err:?}"))
}

fn fixture_gate_with_grant(grant: CapabilityGrantIntent) -> CapabilityGrantIntentGate {
    let mut gate = CapabilityGrantIntentGate::new();
    let _ = gate.submit_grant_intent(grant, fixture_authority_context());
    gate
}

fn obstructed_posture(
    outcome: &CapabilityGrantValidationOutcome,
) -> Result<&CapabilityGrantValidationPosture, String> {
    match outcome {
        CapabilityGrantValidationOutcome::Obstructed(posture) => Ok(posture),
        CapabilityGrantValidationOutcome::IdentityCovered(_) => {
            Err("expected grant validation obstruction".to_owned())
        }
    }
}

fn latest_validation_obstruction_fact(
    gate: &CapabilityGrantIntentGate,
) -> Result<&GraphFact, String> {
    gate.published_graph_facts()
        .last()
        .map(|published| &published.fact)
        .ok_or_else(|| "expected capability grant validation obstruction fact".to_owned())
}

#[test]
fn grant_validation_obstructs_artifact_hash_mismatch() -> Result<(), String> {
    let registered = fixture_registered_artifact()?;
    let mut grant = fixture_grant("grant:artifact-mismatch");
    grant.artifact_hash = "artifact-hash:other".to_owned();
    let mut gate = fixture_gate_with_grant(grant);

    let outcome = gate.validate_capability_presentation_for_artifact(
        &fixture_presentation("grant:artifact-mismatch"),
        &registered,
        CapabilityGrantExpiryPosture::NotEvaluated,
    );

    let posture = obstructed_posture(&outcome)?;
    assert_eq!(
        posture.obstruction,
        CapabilityGrantValidationObstruction::ArtifactHashMismatch
    );
    assert!(matches!(
        latest_validation_obstruction_fact(&gate)?,
        GraphFact::CapabilityGrantValidationObstructed {
            grant_artifact_hash,
            expected_artifact_hash,
            obstruction,
            ..
        } if grant_artifact_hash.as_deref() == Some("artifact-hash:other")
            && expected_artifact_hash == "artifact-hash:stack-witness-0001"
            && *obstruction == warp_core::CapabilityGrantValidationObstructionKind::ArtifactHashMismatch
    ));
    Ok(())
}

#[test]
fn grant_validation_obstructs_operation_id_mismatch() -> Result<(), String> {
    let registered = fixture_registered_artifact()?;
    let mut grant = fixture_grant("grant:operation-mismatch");
    grant.operation_id = "operation:replaceRange:v0".to_owned();
    let mut gate = fixture_gate_with_grant(grant);

    let outcome = gate.validate_capability_presentation_for_artifact(
        &fixture_presentation("grant:operation-mismatch"),
        &registered,
        CapabilityGrantExpiryPosture::NotEvaluated,
    );

    let posture = obstructed_posture(&outcome)?;
    assert_eq!(
        posture.obstruction,
        CapabilityGrantValidationObstruction::OperationIdMismatch
    );
    Ok(())
}

#[test]
fn grant_validation_obstructs_requirements_digest_mismatch() -> Result<(), String> {
    let registered = fixture_registered_artifact()?;
    let mut grant = fixture_grant("grant:requirements-mismatch");
    grant.requirements_digest = "requirements-digest:other".to_owned();
    let mut gate = fixture_gate_with_grant(grant);

    let outcome = gate.validate_capability_presentation_for_artifact(
        &fixture_presentation("grant:requirements-mismatch"),
        &registered,
        CapabilityGrantExpiryPosture::NotEvaluated,
    );

    let posture = obstructed_posture(&outcome)?;
    assert_eq!(
        posture.obstruction,
        CapabilityGrantValidationObstruction::RequirementsDigestMismatch
    );
    Ok(())
}

#[test]
fn grant_validation_obstructs_expired_grant_if_expiry_exists() -> Result<(), String> {
    let registered = fixture_registered_artifact()?;
    let grant = fixture_grant("grant:expired");
    let mut gate = fixture_gate_with_grant(grant);

    let outcome = gate.validate_capability_presentation_for_artifact(
        &fixture_presentation("grant:expired"),
        &registered,
        CapabilityGrantExpiryPosture::Expired,
    );

    let posture = obstructed_posture(&outcome)?;
    assert_eq!(
        posture.obstruction,
        CapabilityGrantValidationObstruction::ExpiredGrant
    );
    Ok(())
}

#[test]
fn grant_validation_obstruction_publishes_graph_fact() -> Result<(), String> {
    let registered = fixture_registered_artifact()?;
    let mut grant = fixture_grant("grant:publish-fact");
    grant.requirements_digest = "requirements-digest:other".to_owned();
    let mut gate = fixture_gate_with_grant(grant);

    let outcome = gate.validate_capability_presentation_for_artifact(
        &fixture_presentation("grant:publish-fact"),
        &registered,
        CapabilityGrantExpiryPosture::NotEvaluated,
    );

    assert_eq!(
        obstructed_posture(&outcome)?.obstruction,
        CapabilityGrantValidationObstruction::RequirementsDigestMismatch
    );
    assert!(matches!(
        latest_validation_obstruction_fact(&gate)?,
        GraphFact::CapabilityGrantValidationObstructed {
            presentation_id,
            grant_id,
            artifact_handle_id,
            expected_operation_id,
            grant_operation_id,
            expected_requirements_digest,
            grant_requirements_digest,
            obstruction,
            ..
        } if presentation_id == "presentation:fixture"
            && grant_id.as_deref() == Some("grant:publish-fact")
            && artifact_handle_id == "optic-artifact-handle:0000000000000001"
            && expected_operation_id == "operation:textWindow:v0"
            && grant_operation_id.as_deref() == Some("operation:textWindow:v0")
            && expected_requirements_digest == "requirements-digest:stack-witness-0001"
            && grant_requirements_digest.as_deref() == Some("requirements-digest:other")
            && *obstruction == warp_core::CapabilityGrantValidationObstructionKind::RequirementsDigestMismatch
    ));
    Ok(())
}

#[test]
fn grant_validation_obstruction_fact_digest_is_deterministic() {
    let first = GraphFact::CapabilityGrantValidationObstructed {
        presentation_id: "presentation:fixture".to_owned(),
        grant_id: Some("grant:fixture".to_owned()),
        artifact_handle_id: "optic-artifact-handle:0000000000000001".to_owned(),
        expected_artifact_hash: "artifact-hash:stack-witness-0001".to_owned(),
        grant_artifact_hash: Some("artifact-hash:other".to_owned()),
        expected_operation_id: "operation:textWindow:v0".to_owned(),
        grant_operation_id: Some("operation:textWindow:v0".to_owned()),
        expected_requirements_digest: "requirements-digest:stack-witness-0001".to_owned(),
        grant_requirements_digest: Some("requirements-digest:stack-witness-0001".to_owned()),
        obstruction: warp_core::CapabilityGrantValidationObstructionKind::ArtifactHashMismatch,
    };
    let repeated = first.clone();

    assert_eq!(first.digest(), repeated.digest());
}
