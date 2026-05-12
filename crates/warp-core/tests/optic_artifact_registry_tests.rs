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

fn registration_err_or_panic<T>(
    result: Result<T, OpticArtifactRegistrationError>,
    context: &str,
) -> Result<OpticArtifactRegistrationError, String> {
    match result {
        Ok(_) => Err(format!("{context}: expected registration error")),
        Err(err) => Ok(err),
    }
}

#[test]
fn optic_artifact_registry_registers_wesley_descriptor_and_resolves_handle() -> Result<(), String> {
    let artifact = fixture_artifact();
    let descriptor = fixture_descriptor();
    let mut registry = OpticArtifactRegistry::new();

    let handle = registry
        .register_optic_artifact(artifact.clone(), descriptor)
        .map_err(|err| format!("fixture descriptor should register: {err:?}"))?;

    assert_eq!(handle.kind, "optic-artifact-handle");
    assert!(!handle.id.is_empty());

    let registered = registry
        .resolve_optic_artifact_handle(&handle)
        .map_err(|err| format!("fresh handle should resolve: {err:?}"))?;

    assert_eq!(registered.artifact_id, artifact.artifact_id);
    assert_eq!(registered.artifact_hash, artifact.artifact_hash);
    assert_eq!(registered.schema_id, artifact.schema_id);
    assert_eq!(registered.operation_id, artifact.operation.operation_id);
    assert_eq!(registered.requirements_digest, artifact.requirements_digest);
    assert_eq!(registered.requirements, artifact.requirements);
    Ok(())
}

#[test]
fn optic_artifact_registry_rejects_tampered_artifact_hash() -> Result<(), String> {
    let artifact = fixture_artifact();
    let mut descriptor = fixture_descriptor();
    descriptor.artifact_hash = "artifact-hash:tampered".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registration_err_or_panic(
        registry.register_optic_artifact(artifact, descriptor),
        "tampered artifact hash should reject",
    )?;

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::ArtifactHashMismatch
    ));
    Ok(())
}

#[test]
fn optic_artifact_registry_rejects_mismatched_artifact_id() -> Result<(), String> {
    let artifact = fixture_artifact();
    let mut descriptor = fixture_descriptor();
    descriptor.artifact_id = "optic-artifact:other".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registration_err_or_panic(
        registry.register_optic_artifact(artifact, descriptor),
        "mismatched artifact id should reject",
    )?;

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::ArtifactIdMismatch
    ));
    Ok(())
}

#[test]
fn optic_artifact_registry_rejects_tampered_requirements_digest() -> Result<(), String> {
    let artifact = fixture_artifact();
    let mut descriptor = fixture_descriptor();
    descriptor.requirements_digest = "requirements-digest:tampered".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registration_err_or_panic(
        registry.register_optic_artifact(artifact, descriptor),
        "tampered requirements digest should reject",
    )?;

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::RequirementsDigestMismatch
    ));
    Ok(())
}

#[test]
fn optic_artifact_registry_rejects_mismatched_operation_id() -> Result<(), String> {
    let artifact = fixture_artifact();
    let mut descriptor = fixture_descriptor();
    descriptor.operation_id = "operation:replaceRange:v0".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registration_err_or_panic(
        registry.register_optic_artifact(artifact, descriptor),
        "mismatched operation id should reject",
    )?;

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::OperationIdMismatch
    ));
    Ok(())
}

#[test]
fn optic_artifact_registry_rejects_mismatched_schema_id() -> Result<(), String> {
    let artifact = fixture_artifact();
    let mut descriptor = fixture_descriptor();
    descriptor.schema_id = "schema:other:v0".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registration_err_or_panic(
        registry.register_optic_artifact(artifact, descriptor),
        "mismatched schema id should reject",
    )?;

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::SchemaIdMismatch
    ));
    Ok(())
}

#[test]
fn optic_artifact_registry_rejects_unknown_handle_lookup() -> Result<(), String> {
    let registry = OpticArtifactRegistry::new();
    let handle = OpticArtifactHandle {
        kind: "optic-artifact-handle".to_owned(),
        id: "unregistered-handle".to_owned(),
    };

    let err = registration_err_or_panic(
        registry.resolve_optic_artifact_handle(&handle),
        "unknown handle should reject",
    )?;

    assert!(matches!(err, OpticArtifactRegistrationError::UnknownHandle));
    Ok(())
}
