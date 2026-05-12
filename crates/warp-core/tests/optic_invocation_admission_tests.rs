// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for optic invocation admission obstruction.

use warp_core::{
    OpticAdmissionRequirements, OpticApertureRequest, OpticArtifact, OpticArtifactHandle,
    OpticArtifactOperation, OpticArtifactRegistry, OpticBasisRequest, OpticCapabilityPresentation,
    OpticInvocation, OpticInvocationAdmissionOutcome, OpticInvocationObstruction,
    OpticRegistrationDescriptor,
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

fn fixture_registry_and_handle() -> (OpticArtifactRegistry, OpticArtifactHandle) {
    let mut registry = OpticArtifactRegistry::new();
    let handle = registry
        .register_optic_artifact(fixture_artifact(), fixture_descriptor())
        .expect("fixture descriptor should register");
    (registry, handle)
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
        OpticInvocationAdmissionOutcome::Obstructed(OpticInvocationObstruction::UnknownHandle)
    );
}

#[test]
fn optic_invocation_obstructs_operation_mismatch() {
    let (registry, handle) = fixture_registry_and_handle();
    let mut invocation = fixture_invocation(handle);
    invocation.operation_id = "operation:replaceRange:v0".to_owned();

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        OpticInvocationAdmissionOutcome::Obstructed(OpticInvocationObstruction::OperationMismatch)
    );
}

#[test]
fn optic_invocation_obstructs_missing_capability_for_registered_handle() {
    let (registry, handle) = fixture_registry_and_handle();
    let invocation = fixture_invocation(handle);

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        OpticInvocationAdmissionOutcome::Obstructed(OpticInvocationObstruction::MissingCapability)
    );
}

#[test]
fn optic_invocation_obstructs_placeholder_capability_presentation_until_grant_validation_exists() {
    let (registry, handle) = fixture_registry_and_handle();
    let mut invocation = fixture_invocation(handle);
    invocation.capability_presentation = Some(OpticCapabilityPresentation {
        presentation_id: "presentation:placeholder".to_owned(),
    });

    let outcome = registry.admit_optic_invocation(&invocation);

    assert_eq!(
        outcome,
        OpticInvocationAdmissionOutcome::Obstructed(
            OpticInvocationObstruction::CapabilityValidationUnavailable
        )
    );
}
