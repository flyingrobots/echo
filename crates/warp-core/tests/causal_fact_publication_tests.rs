// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for graph fact publication from optic artifact registration.

use warp_core::{
    ArtifactRegistrationObstructionKind, GraphFact, OpticAdmissionRequirements, OpticArtifact,
    OpticArtifactOperation, OpticArtifactRegistrationError, OpticArtifactRegistry,
    OpticRegistrationDescriptor, ARTIFACT_REGISTRATION_RECEIPT_KIND,
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
fn artifact_registration_publishes_graph_fact_and_receipt() -> Result<(), String> {
    let mut registry = OpticArtifactRegistry::new();
    let handle = registry
        .register_optic_artifact(fixture_artifact(), fixture_descriptor())
        .map_err(|err| format!("fixture descriptor should register: {err:?}"))?;

    assert_eq!(registry.published_graph_facts().len(), 1);
    assert_eq!(registry.artifact_registration_receipts().len(), 1);

    let published = &registry.published_graph_facts()[0];
    let receipt = &registry.artifact_registration_receipts()[0];

    assert_eq!(receipt.kind, ARTIFACT_REGISTRATION_RECEIPT_KIND);
    assert_eq!(receipt.handle_id, handle.id);
    assert_eq!(receipt.fact_digest, published.digest);
    assert_eq!(published.digest, published.fact.digest());

    assert!(matches!(
        &published.fact,
        GraphFact::ArtifactRegistered {
            handle_id,
            artifact_hash,
            schema_id,
            operation_id,
            requirements_digest,
        } if handle_id == &receipt.handle_id
            && artifact_hash == "artifact-hash:stack-witness-0001"
            && schema_id == "schema:jedit-text-buffer-optic:v0"
            && operation_id == "operation:textWindow:v0"
            && requirements_digest == "requirements-digest:stack-witness-0001"
    ));
    Ok(())
}

#[test]
fn artifact_registration_obstruction_publishes_graph_fact_without_receipt() -> Result<(), String> {
    let mut descriptor = fixture_descriptor();
    descriptor.operation_id = "operation:tampered:v0".to_owned();
    let mut registry = OpticArtifactRegistry::new();

    let err = registration_err_or_panic(
        registry.register_optic_artifact(fixture_artifact(), descriptor),
        "tampered operation id should reject",
    )?;

    assert!(matches!(
        err,
        OpticArtifactRegistrationError::OperationIdMismatch
    ));
    assert_eq!(registry.len(), 0);
    assert_eq!(registry.published_graph_facts().len(), 1);
    assert!(registry.artifact_registration_receipts().is_empty());

    let published = &registry.published_graph_facts()[0];
    assert_eq!(published.digest, published.fact.digest());
    assert!(matches!(
        &published.fact,
        GraphFact::ArtifactRegistrationObstructed {
            artifact_hash,
            obstruction,
        } if artifact_hash.as_deref() == Some("artifact-hash:stack-witness-0001")
            && *obstruction == ArtifactRegistrationObstructionKind::OperationIdMismatch
    ));
    Ok(())
}

#[test]
fn graph_fact_digest_is_deterministic_and_kind_separated() {
    let registered = GraphFact::ArtifactRegistered {
        handle_id: "handle-1".to_owned(),
        artifact_hash: "same-artifact".to_owned(),
        schema_id: "schema".to_owned(),
        operation_id: "operation".to_owned(),
        requirements_digest: "requirements".to_owned(),
    };
    let repeated = registered.clone();
    let obstructed = GraphFact::ArtifactRegistrationObstructed {
        artifact_hash: Some("same-artifact".to_owned()),
        obstruction: ArtifactRegistrationObstructionKind::ArtifactHashMismatch,
    };

    assert_eq!(registered.digest(), repeated.digest());
    assert_ne!(registered.digest(), obstructed.digest());
}

#[test]
fn graph_fact_digest_distinguishes_absent_and_empty_optional_fields() {
    let absent = GraphFact::ArtifactRegistrationObstructed {
        artifact_hash: None,
        obstruction: ArtifactRegistrationObstructionKind::ArtifactHashMismatch,
    };
    let empty = GraphFact::ArtifactRegistrationObstructed {
        artifact_hash: Some(String::new()),
        obstruction: ArtifactRegistrationObstructionKind::ArtifactHashMismatch,
    };

    assert_ne!(absent.digest(), empty.digest());
}
