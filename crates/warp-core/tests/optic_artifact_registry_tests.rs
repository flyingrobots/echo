// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for Echo-owned optic artifact registration.

use warp_core::{
    OpticAdmissionRequirements, OpticArtifact, OpticArtifactHandle, OpticArtifactOperation,
    OpticArtifactRegistrationError, OpticArtifactRegistry, OpticRegistrationDescriptor,
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

#[test]
fn optic_artifact_registry_registers_wesley_descriptor_and_resolves_handle() {
    let artifact = fixture_artifact();
    let descriptor = fixture_descriptor();
    let mut registry = OpticArtifactRegistry::new();

    let handle = registry
        .register_optic_artifact(artifact.clone(), descriptor)
        .expect("fixture descriptor should register");

    assert_eq!(handle.kind, "optic-artifact-handle");
    assert!(!handle.id.is_empty());

    let registered = registry
        .resolve_optic_artifact_handle(&handle)
        .expect("fresh handle should resolve");

    assert_eq!(registered.artifact_id, artifact.artifact_id);
    assert_eq!(registered.artifact_hash, artifact.artifact_hash);
    assert_eq!(registered.schema_id, artifact.schema_id);
    assert_eq!(registered.operation_id, artifact.operation.operation_id);
    assert_eq!(registered.requirements_digest, artifact.requirements_digest);
    assert_eq!(registered.requirements, artifact.requirements);
}

#[test]
fn optic_artifact_registry_rejects_tampered_artifact_hash() {
    let artifact = fixture_artifact();
    let mut descriptor = fixture_descriptor();
    descriptor.artifact_hash = "artifact-hash:tampered".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registry
        .register_optic_artifact(artifact, descriptor)
        .expect_err("tampered artifact hash should reject");

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::ArtifactHashMismatch
    ));
}

#[test]
fn optic_artifact_registry_rejects_mismatched_artifact_id() {
    let artifact = fixture_artifact();
    let mut descriptor = fixture_descriptor();
    descriptor.artifact_id = "optic-artifact:other".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registry
        .register_optic_artifact(artifact, descriptor)
        .expect_err("mismatched artifact id should reject");

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::ArtifactIdMismatch
    ));
}

#[test]
fn optic_artifact_registry_rejects_tampered_requirements_digest() {
    let artifact = fixture_artifact();
    let mut descriptor = fixture_descriptor();
    descriptor.requirements_digest = "requirements-digest:tampered".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registry
        .register_optic_artifact(artifact, descriptor)
        .expect_err("tampered requirements digest should reject");

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::RequirementsDigestMismatch
    ));
}

#[test]
fn optic_artifact_registry_rejects_mismatched_operation_id() {
    let artifact = fixture_artifact();
    let mut descriptor = fixture_descriptor();
    descriptor.operation_id = "operation:replaceRange:v0".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registry
        .register_optic_artifact(artifact, descriptor)
        .expect_err("mismatched operation id should reject");

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::OperationIdMismatch
    ));
}

#[test]
fn optic_artifact_registry_rejects_mismatched_schema_id() {
    let artifact = fixture_artifact();
    let mut descriptor = fixture_descriptor();
    descriptor.schema_id = "schema:other:v0".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registry
        .register_optic_artifact(artifact, descriptor)
        .expect_err("mismatched schema id should reject");

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::SchemaIdMismatch
    ));
}

#[test]
fn optic_artifact_registry_rejects_unknown_handle_lookup() {
    let registry = OpticArtifactRegistry::new();
    let handle = OpticArtifactHandle {
        kind: "optic-artifact-handle".to_owned(),
        id: "unregistered-handle".to_owned(),
    };

    let err = registry
        .resolve_optic_artifact_handle(&handle)
        .expect_err("unknown handle should reject");

    assert!(matches!(err, OpticArtifactRegistrationError::UnknownHandle));
}
