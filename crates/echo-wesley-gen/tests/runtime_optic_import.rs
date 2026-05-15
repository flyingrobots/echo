// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! Proof that `echo-wesley-gen` adapts real Wesley runtime optic artifacts into Echo.

use echo_wesley_gen::import_runtime_optic_artifact;
use warp_core::{
    OpticArtifactRegistrationError, OpticArtifactRegistry, OPTIC_ARTIFACT_HANDLE_KIND,
};
use wesley_core::compile_runtime_optic;

const WORKSPACE_SCHEMA: &str = r"
directive @wes_law(id: String!) on FIELD
directive @wes_footprint(
  reads: [String!]!
  writes: [String!]!
  forbids: [String!]
) on FIELD

schema {
  query: Query
  mutation: Mutation
}

type Query {
  basis(ref: ID!): WorkspaceBasis
}

type Mutation {
  renameSymbol(input: RenameSymbolInput!): RenameSymbolResult!
}

input RenameSymbolInput {
  basisRef: ID!
  path: String!
  symbol: String!
  nextName: String!
}

type RenameSymbolResult {
  receipt: RewriteReceipt!
  changedFiles: [ChangedFile!]!
}

type RewriteReceipt {
  basisRef: ID!
  resultRef: ID!
  operationId: String!
  witnessDigest: String!
}

type ChangedFile {
  path: String!
  beforeDigest: String!
  afterDigest: String!
}

type WorkspaceBasis {
  ref: ID!
  digest: String!
}
";

const RENAME_SYMBOL_OPERATION: &str = r#"
mutation RenameSymbol($input: RenameSymbolInput!) {
  renameSymbol(input: $input)
    @wes_law(id: "bounded.rewrite.v1")
    @wes_footprint(
      reads: ["workspace.files", "symbol.index"]
      writes: ["workspace.files"]
      forbids: ["secrets", "git.refs"]
    ) {
    receipt {
      basisRef
      resultRef
      operationId
      witnessDigest
    }
  }
}
"#;

fn compile_fixture_artifact() -> wesley_core::OpticArtifact {
    compile_runtime_optic(
        WORKSPACE_SCHEMA,
        RENAME_SYMBOL_OPERATION,
        Some("RenameSymbol"),
    )
    .expect("fixture runtime optic should compile")
}

#[test]
fn imports_real_wesley_runtime_optic_artifact() {
    let wesley_artifact = compile_fixture_artifact();
    let imported =
        import_runtime_optic_artifact(&wesley_artifact).expect("artifact import should succeed");
    let repeated =
        import_runtime_optic_artifact(&wesley_artifact).expect("artifact import should repeat");

    assert_eq!(imported, repeated);
    assert_eq!(imported.artifact.artifact_id, wesley_artifact.artifact_id);
    assert_eq!(
        imported.artifact.artifact_hash,
        wesley_artifact.artifact_hash
    );
    assert_eq!(imported.artifact.schema_id, wesley_artifact.schema_id);
    assert_eq!(
        imported.artifact.requirements_digest,
        wesley_artifact.requirements_digest
    );
    assert_eq!(
        imported.artifact.operation.operation_id,
        wesley_artifact.operation.operation_id
    );
    assert_eq!(
        imported.descriptor.artifact_id,
        wesley_artifact.registration.artifact_id
    );
    assert_eq!(
        imported.descriptor.artifact_hash,
        wesley_artifact.registration.artifact_hash
    );
    assert_eq!(
        imported.descriptor.schema_id,
        wesley_artifact.registration.schema_id
    );
    assert_eq!(
        imported.descriptor.operation_id,
        wesley_artifact.registration.operation_id
    );
    assert_eq!(
        imported.descriptor.requirements_digest,
        wesley_artifact.registration.requirements_digest
    );
    assert!(
        !imported.artifact.requirements.bytes.is_empty(),
        "Wesley requirements must be imported into Echo registry bytes"
    );
}

#[test]
fn registers_imported_wesley_artifact_and_returns_opaque_handle() {
    let imported = import_runtime_optic_artifact(&compile_fixture_artifact())
        .expect("artifact import should succeed");
    let mut registry = OpticArtifactRegistry::new();

    let handle = registry
        .register_optic_artifact(imported.artifact.clone(), imported.descriptor.clone())
        .expect("imported Wesley artifact should register");
    let registered = registry
        .resolve_optic_artifact_handle(&handle)
        .expect("registered handle should resolve internally");

    assert_eq!(handle.kind, OPTIC_ARTIFACT_HANDLE_KIND);
    assert!(!handle.id.is_empty());
    assert_eq!(registered.handle, handle);
    assert_eq!(registered.artifact_hash, imported.artifact.artifact_hash);
    assert_eq!(
        registered.requirements_digest,
        imported.artifact.requirements_digest
    );
    assert_eq!(
        registered.operation_id,
        imported.artifact.operation.operation_id
    );
    assert_eq!(registered.requirements, imported.artifact.requirements);
}

#[test]
fn rejects_tampered_artifact_hash() {
    let imported = import_runtime_optic_artifact(&compile_fixture_artifact())
        .expect("artifact import should succeed");
    let mut registry = OpticArtifactRegistry::new();
    let mut descriptor = imported.descriptor;
    descriptor.artifact_hash = "tampered-artifact-hash".to_string();

    assert!(matches!(
        registry.register_optic_artifact(imported.artifact, descriptor),
        Err(OpticArtifactRegistrationError::ArtifactHashMismatch)
    ));
}

#[test]
fn rejects_tampered_schema_id() {
    let imported = import_runtime_optic_artifact(&compile_fixture_artifact())
        .expect("artifact import should succeed");
    let mut registry = OpticArtifactRegistry::new();
    let mut descriptor = imported.descriptor;
    descriptor.schema_id = "tampered-schema-id".to_string();

    assert!(matches!(
        registry.register_optic_artifact(imported.artifact, descriptor),
        Err(OpticArtifactRegistrationError::SchemaIdMismatch)
    ));
}

#[test]
fn rejects_tampered_operation_id() {
    let imported = import_runtime_optic_artifact(&compile_fixture_artifact())
        .expect("artifact import should succeed");
    let mut registry = OpticArtifactRegistry::new();
    let mut descriptor = imported.descriptor;
    descriptor.operation_id = "tampered-operation-id".to_string();

    assert!(matches!(
        registry.register_optic_artifact(imported.artifact, descriptor),
        Err(OpticArtifactRegistrationError::OperationIdMismatch)
    ));
}

#[test]
fn rejects_tampered_requirements_digest() {
    let imported = import_runtime_optic_artifact(&compile_fixture_artifact())
        .expect("artifact import should succeed");
    let mut registry = OpticArtifactRegistry::new();
    let mut descriptor = imported.descriptor;
    descriptor.requirements_digest = "tampered-requirements-digest".to_string();

    assert!(matches!(
        registry.register_optic_artifact(imported.artifact, descriptor),
        Err(OpticArtifactRegistrationError::RequirementsDigestMismatch)
    ));
}

#[test]
fn warp_core_does_not_depend_on_wesley_core() {
    let warp_core_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root ancestor missing")
        .join("crates/warp-core/Cargo.toml");
    let manifest =
        std::fs::read_to_string(warp_core_manifest).expect("warp-core manifest should be readable");

    assert!(
        !manifest.contains("wesley-core"),
        "warp-core must not depend on wesley-core; the dependency seam belongs in echo-wesley-gen"
    );
}
