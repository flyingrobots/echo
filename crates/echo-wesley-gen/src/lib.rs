// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Adapter from Wesley-compiled runtime optic artifacts into Echo runtime types.
//!
//! Wesley owns compiled artifact truth: artifact hashes, schema ids, operation
//! ids, requirements digests, and registration descriptors. `warp-core` owns
//! runtime registration and opaque handles. This crate is the dependency seam
//! that may see both sides.
//!
//! The adapter imports Wesley-owned canonical admission requirement bytes,
//! codec ids, and digests into `warp-core` without reserializing Wesley
//! requirement structs. Enforcement, grant validation, admission tickets,
//! witnesses, and execution are intentionally out of scope for this adapter.

/// Strict Echo-owned semantic source for generated Edict provider artifacts.
pub mod provider_semantics;

/// Explicit admission boundary for the Edict-owned provider contract pack.
pub mod provider_contract_pack;

/// Edict canonical values, CBOR encoding, and domain-framed artifact digests.
pub mod provider_canonical;

/// Canonical Wesley input assembly for Echo provider artifact generation.
pub mod provider_generation;

/// Deterministic, schema-validated primary Edict provider artifacts.
pub mod provider_artifacts;

/// Exact-material Wesley provenance for provider artifact generation.
pub mod provider_provenance;

/// Deterministic non-authoritative Wesley review of provider generation.
pub mod provider_review;

/// Exact generator source identity and checked provider artifact corpus.
pub mod provider_corpus;

/// Deterministic digest-locked Echo Edict provider package.
pub mod provider_package;

/// Imported Wesley runtime optic artifact ready for Echo registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedRuntimeOpticArtifact {
    /// Echo runtime artifact shape.
    pub artifact: warp_core::OpticArtifact,
    /// Wesley registration descriptor mirrored into Echo's registration shape.
    pub descriptor: warp_core::OpticRegistrationDescriptor,
}

/// Imports a Wesley-compiled runtime optic artifact into Echo runtime structs.
///
/// This does not register the artifact and does not issue authority. Echo still
/// verifies the descriptor through [`warp_core::OpticArtifactRegistry`] and
/// returns the opaque runtime-local handle only after registration succeeds.
pub fn import_runtime_optic_artifact(
    artifact: &wesley_core::OperationArtifact,
) -> anyhow::Result<ImportedRuntimeOpticArtifact> {
    if artifact.requirements_digest != artifact.requirements_artifact.digest {
        anyhow::bail!(
            "Wesley artifact requirements digest does not match requirements artifact digest"
        );
    }

    Ok(ImportedRuntimeOpticArtifact {
        artifact: warp_core::OpticArtifact {
            artifact_id: artifact.artifact_id.clone(),
            artifact_hash: artifact.artifact_hash.clone(),
            schema_id: artifact.schema_id.clone(),
            requirements_digest: artifact.requirements_artifact.digest.clone(),
            operation: warp_core::OpticArtifactOperation {
                operation_id: artifact.operation.operation_id.clone(),
            },
            requirements: warp_core::OpticAdmissionRequirements {
                codec: artifact.requirements_artifact.codec.clone(),
                digest: artifact.requirements_artifact.digest.clone(),
                bytes: artifact.requirements_artifact.bytes.clone(),
            },
        },
        descriptor: import_registration_descriptor(&artifact.registration),
    })
}

/// Imports a Wesley registration descriptor into Echo's registration shape.
pub fn import_registration_descriptor(
    descriptor: &wesley_core::OperationRegistrationDescriptor,
) -> warp_core::OpticRegistrationDescriptor {
    warp_core::OpticRegistrationDescriptor {
        artifact_id: descriptor.artifact_id.clone(),
        artifact_hash: descriptor.artifact_hash.clone(),
        schema_id: descriptor.schema_id.clone(),
        operation_id: descriptor.operation_id.clone(),
        requirements_digest: descriptor.requirements_digest.clone(),
    }
}
