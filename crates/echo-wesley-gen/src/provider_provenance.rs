// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Exact-material Wesley provenance for deterministic provider generation.
//!
//! Provenance binds the exact authored sources, versioned settings, explicit
//! generator component bytes, and six non-derived primary outputs. The
//! generated resources remain transitively bound through those primary bytes;
//! provenance and review are excluded because either would be self-referential.
//! This module performs no filesystem, process, environment, registry, clock,
//! or network discovery.

use std::fmt;

use wesley_core::{
    compute_generation_artifact_digest_v1, GenerationArtifactContentV1,
    GenerationArtifactReferenceV1, GenerationContractError, GenerationContractErrorKind,
    GenerationProvenanceManifestV1, GenerationProvenanceVerificationV1, GeneratorIdentityV1,
};

use crate::provider_artifacts::ProviderPrimaryArtifactsV1;
use crate::provider_generation::ProviderGenerationInputV1;
use crate::provider_semantics::GeneratedArtifactKind;

/// Stable failure categories returned while constructing provider provenance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderProvenanceErrorKind {
    /// The semantic source did not declare its required provenance evidence.
    EvidenceDeclarationMissing,
    /// Primary input identity or roles did not match the Wesley closure.
    PrimaryOutputClosureMismatch,
    /// Exact settings bytes did not reproduce the Wesley settings digest.
    SettingsMaterialMismatch,
    /// Supplied generator coordinate or version differed from the manifest.
    GeneratorIdentityMismatch,
    /// Generator coordinate collided with a declared provider coordinate.
    GeneratorCoordinateConflict,
    /// Wesley rejected an identity, reference, manifest, or exact material.
    WesleyContractRejected,
}

impl ProviderProvenanceErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::EvidenceDeclarationMissing => "evidence-declaration-missing",
            Self::PrimaryOutputClosureMismatch => "primary-output-closure-mismatch",
            Self::SettingsMaterialMismatch => "settings-material-mismatch",
            Self::GeneratorIdentityMismatch => "generator-identity-mismatch",
            Self::GeneratorCoordinateConflict => "generator-coordinate-conflict",
            Self::WesleyContractRejected => "wesley-contract-rejected",
        }
    }
}

/// Structured, stable failure from provider provenance construction or verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderProvenanceError {
    kind: ProviderProvenanceErrorKind,
    subject: String,
    reference: String,
    wesley_kind: Option<GenerationContractErrorKind>,
}

impl ProviderProvenanceError {
    /// Returns the stable high-level failure category.
    #[must_use]
    pub const fn kind(&self) -> ProviderProvenanceErrorKind {
        self.kind
    }

    /// Returns the coordinate or contract field that failed.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the expected identity, digest, conflicting role, or upstream code.
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Returns the typed Wesley contract cause, when applicable.
    #[must_use]
    pub const fn wesley_contract_kind(&self) -> Option<GenerationContractErrorKind> {
        self.wesley_kind
    }

    fn new(
        kind: ProviderProvenanceErrorKind,
        subject: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject: subject.into(),
            reference: reference.into(),
            wesley_kind: None,
        }
    }

    fn wesley(error: GenerationContractError) -> Self {
        let kind = error.kind;
        let mut result = Self::new(
            ProviderProvenanceErrorKind::WesleyContractRejected,
            error.subject,
            kind.as_str(),
        );
        result.wesley_kind = Some(kind);
        result
    }
}

impl fmt::Display for ProviderProvenanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider provenance {}: {} -> {}",
            self.kind.label(),
            self.subject,
            self.reference
        )
    }
}

impl std::error::Error for ProviderProvenanceError {}

/// Explicit generator identity and exact bytes used for provenance verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderGeneratorMaterialV1 {
    identity: GeneratorIdentityV1,
    bytes: Vec<u8>,
}

impl ProviderGeneratorMaterialV1 {
    /// Constructs generator material from explicit component bytes.
    ///
    /// # Errors
    ///
    /// Returns a typed Wesley contract error when the coordinate or version is
    /// invalid. No executable, path, environment, or process discovery occurs.
    pub fn new(
        coordinate: impl Into<String>,
        version: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, ProviderProvenanceError> {
        let identity = GeneratorIdentityV1::for_bytes(coordinate, version, bytes)
            .map_err(ProviderProvenanceError::wesley)?;
        Ok(Self {
            identity,
            bytes: bytes.to_vec(),
        })
    }

    /// Returns the identity computed from the exact supplied bytes.
    #[must_use]
    pub const fn identity(&self) -> &GeneratorIdentityV1 {
        &self.identity
    }

    /// Returns the exact generator component bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Canonical, immediately verified provider-generation provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderGenerationProvenanceV1 {
    role: String,
    coordinate: String,
    schema_contract: String,
    manifest: GenerationProvenanceManifestV1,
    canonical_bytes: Vec<u8>,
    content_reference: GenerationArtifactReferenceV1,
    verification: GenerationProvenanceVerificationV1,
}

impl ProviderGenerationProvenanceV1 {
    /// Returns the source-declared provenance role.
    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Returns the source-declared provenance coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }

    /// Returns the Wesley-owned provenance schema contract.
    #[must_use]
    pub fn schema_contract(&self) -> &str {
        &self.schema_contract
    }

    /// Returns the admitted Wesley provenance manifest.
    #[must_use]
    pub const fn manifest(&self) -> &GenerationProvenanceManifestV1 {
        &self.manifest
    }

    /// Returns exact canonical Wesley provenance JSON bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the exact-byte reference for the canonical provenance JSON.
    #[must_use]
    pub const fn content_reference(&self) -> &GenerationArtifactReferenceV1 {
        &self.content_reference
    }

    /// Returns the receipt from immediate exact-material verification.
    #[must_use]
    pub const fn verification(&self) -> &GenerationProvenanceVerificationV1 {
        &self.verification
    }

    /// Re-verifies this manifest against explicit generation materials.
    ///
    /// # Errors
    ///
    /// Returns a stable structured failure when settings, projection roles,
    /// generator identity, or any exact source or output byte differs.
    pub fn verify_exact_materials(
        &self,
        input: &ProviderGenerationInputV1,
        primary: &ProviderPrimaryArtifactsV1,
        generator: &ProviderGeneratorMaterialV1,
    ) -> Result<GenerationProvenanceVerificationV1, ProviderProvenanceError> {
        validate_input_materials(input, primary)?;
        if generator.identity.coordinate != self.manifest.generator.coordinate {
            return Err(ProviderProvenanceError::new(
                ProviderProvenanceErrorKind::GeneratorIdentityMismatch,
                "generator.coordinate",
                &self.manifest.generator.coordinate,
            ));
        }
        if generator.identity.version != self.manifest.generator.version {
            return Err(ProviderProvenanceError::new(
                ProviderProvenanceErrorKind::GeneratorIdentityMismatch,
                "generator.version",
                &self.manifest.generator.version,
            ));
        }
        let emitted = emitted_materials(primary);
        self.manifest
            .verify(
                input.wesley_input(),
                generator.bytes(),
                input.source_artifacts(),
                &emitted,
            )
            .map_err(ProviderProvenanceError::wesley)
    }
}

/// Constructs canonical provenance and immediately verifies every exact material.
///
/// The manifest names exactly six non-derived primary outputs: the five
/// schema-validated canonical artifacts and the raw self-contained CDDL. The
/// function is pure and performs no hidden discovery or runtime admission.
///
/// # Errors
///
/// Returns a structured failure when the semantic evidence declaration is
/// missing, the primary closure or settings material disagrees with the Wesley
/// input, the generator coordinate aliases the provider closure, or Wesley
/// rejects any identity, reference, manifest, or exact byte.
pub fn generate_provider_generation_provenance_v1(
    input: &ProviderGenerationInputV1,
    primary: &ProviderPrimaryArtifactsV1,
    generator: &ProviderGeneratorMaterialV1,
) -> Result<ProviderGenerationProvenanceV1, ProviderProvenanceError> {
    validate_input_materials(input, primary)?;
    validate_generator_coordinate(input, generator)?;
    let declaration = input
        .semantic_source()
        .source()
        .generated_artifacts
        .iter()
        .find(|artifact| artifact.kind == GeneratedArtifactKind::GenerationProvenance)
        .ok_or_else(|| {
            ProviderProvenanceError::new(
                ProviderProvenanceErrorKind::EvidenceDeclarationMissing,
                "generationProvenance",
                "generatedArtifacts",
            )
        })?;
    let emitted = emitted_materials(primary);
    let manifest = GenerationProvenanceManifestV1::new(
        input.wesley_input(),
        generator.identity().clone(),
        emitted
            .iter()
            .map(GenerationArtifactContentV1::reference)
            .collect(),
    )
    .map_err(ProviderProvenanceError::wesley)?;
    let verification = manifest
        .verify(
            input.wesley_input(),
            generator.bytes(),
            input.source_artifacts(),
            &emitted,
        )
        .map_err(ProviderProvenanceError::wesley)?;
    let canonical_bytes = manifest
        .canonical_bytes()
        .map_err(ProviderProvenanceError::wesley)?;
    let content_reference =
        GenerationArtifactReferenceV1::for_bytes(&declaration.coordinate, &canonical_bytes)
            .map_err(ProviderProvenanceError::wesley)?;

    Ok(ProviderGenerationProvenanceV1 {
        role: declaration.role.clone(),
        coordinate: declaration.coordinate.clone(),
        schema_contract: declaration.schema_contract.clone(),
        manifest,
        canonical_bytes,
        content_reference,
        verification,
    })
}

fn validate_generator_coordinate(
    input: &ProviderGenerationInputV1,
    generator: &ProviderGeneratorMaterialV1,
) -> Result<(), ProviderProvenanceError> {
    let source = input.semantic_source().source();
    let coordinate = &generator.identity().coordinate;
    let mut conflicting_roles = source
        .generated_artifacts
        .iter()
        .filter(|artifact| artifact.coordinate == *coordinate)
        .map(|artifact| artifact.role.as_str())
        .chain(
            source
                .artifact_resources
                .iter()
                .filter(|resource| resource.coordinate == *coordinate)
                .map(|resource| resource.role.as_str()),
        )
        .collect::<Vec<_>>();
    if source.package_manifest.coordinate == *coordinate {
        conflicting_roles.push(&source.package_manifest.role);
    }
    if source.package_manifest.provider_coordinate == *coordinate {
        conflicting_roles.push("packageManifest.providerCoordinate");
    }
    conflicting_roles.sort_unstable();
    if let Some(role) = conflicting_roles.first() {
        return Err(ProviderProvenanceError::new(
            ProviderProvenanceErrorKind::GeneratorCoordinateConflict,
            "generator.coordinate",
            *role,
        ));
    }
    Ok(())
}

fn validate_input_materials(
    input: &ProviderGenerationInputV1,
    primary: &ProviderPrimaryArtifactsV1,
) -> Result<(), ProviderProvenanceError> {
    if primary.generation_input_digest() != input.digest() {
        return Err(ProviderProvenanceError::new(
            ProviderProvenanceErrorKind::PrimaryOutputClosureMismatch,
            "generationInputDigest",
            input.digest(),
        ));
    }
    if primary.projection_roles() != input.wesley_input().projection_roles {
        return Err(ProviderProvenanceError::new(
            ProviderProvenanceErrorKind::PrimaryOutputClosureMismatch,
            "projectionRoles",
            input.wesley_input().projection_roles.join(","),
        ));
    }
    let settings_digest = compute_generation_artifact_digest_v1(input.settings_bytes());
    if settings_digest != input.wesley_input().settings_digest {
        return Err(ProviderProvenanceError::new(
            ProviderProvenanceErrorKind::SettingsMaterialMismatch,
            "settingsDigest",
            settings_digest,
        ));
    }
    Ok(())
}

fn emitted_materials(primary: &ProviderPrimaryArtifactsV1) -> Vec<GenerationArtifactContentV1> {
    primary
        .artifacts()
        .iter()
        .map(|artifact| {
            GenerationArtifactContentV1::new(
                artifact.coordinate(),
                artifact.canonical_bytes().to_vec(),
            )
        })
        .chain(std::iter::once(GenerationArtifactContentV1::new(
            primary.schema().coordinate(),
            primary.schema().bytes().to_vec(),
        )))
        .collect()
}
