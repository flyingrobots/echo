// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Adapter from Wesley-compiled runtime optic artifacts into Echo runtime types.
//!
//! Wesley owns compiled artifact truth: artifact hashes, schema ids, operation
//! ids, requirements digests, and registration descriptors. `warp-core` owns
//! runtime registration and opaque handles. This crate is the dependency seam
//! that may see both sides.
//!
//! The v0 adapter stores Wesley admission requirements as deterministic
//! `serde_json` bytes in `warp-core`. Those bytes are registry payload only;
//! enforcement, grant validation, admission tickets, witnesses, and execution
//! are intentionally out of scope for this adapter.

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
    artifact: &wesley_core::OpticArtifact,
) -> anyhow::Result<ImportedRuntimeOpticArtifact> {
    let requirements_bytes = serde_json::to_vec(&artifact.requirements)?;

    Ok(ImportedRuntimeOpticArtifact {
        artifact: warp_core::OpticArtifact {
            artifact_id: artifact.artifact_id.clone(),
            artifact_hash: artifact.artifact_hash.clone(),
            schema_id: artifact.schema_id.clone(),
            requirements_digest: artifact.requirements_digest.clone(),
            operation: warp_core::OpticArtifactOperation {
                operation_id: artifact.operation.operation_id.clone(),
            },
            requirements: warp_core::OpticAdmissionRequirements {
                bytes: requirements_bytes,
            },
        },
        descriptor: import_registration_descriptor(&artifact.registration),
    })
}

/// Imports a Wesley registration descriptor into Echo's registration shape.
pub fn import_registration_descriptor(
    descriptor: &wesley_core::OpticRegistrationDescriptor,
) -> warp_core::OpticRegistrationDescriptor {
    warp_core::OpticRegistrationDescriptor {
        artifact_id: descriptor.artifact_id.clone(),
        artifact_hash: descriptor.artifact_hash.clone(),
        schema_id: descriptor.schema_id.clone(),
        operation_id: descriptor.operation_id.clone(),
        requirements_digest: descriptor.requirements_digest.clone(),
    }
}
