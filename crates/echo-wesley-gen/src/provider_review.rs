// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deterministic non-authoritative review of verified provider generation.
//!
//! This module derives Wesley's review projection from an already-verified
//! provider provenance wrapper. Review bytes are convenient human/tooling
//! evidence, never semantic authority or Echo runtime admission.

use std::fmt;

use wesley_core::{
    GenerationArtifactReferenceV1, GenerationContractError, GenerationContractErrorKind,
    GenerationReviewV1,
};

use crate::provider_generation::ProviderGenerationInputV1;
use crate::provider_provenance::ProviderGenerationProvenanceV1;
use crate::provider_semantics::GeneratedArtifactKind;

/// Stable failure categories returned while deriving provider review evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderReviewErrorKind {
    /// The semantic source did not declare its required review evidence.
    EvidenceDeclarationMissing,
    /// Wesley rejected the input, provenance, review, or content reference.
    WesleyContractRejected,
}

impl ProviderReviewErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::EvidenceDeclarationMissing => "evidence-declaration-missing",
            Self::WesleyContractRejected => "wesley-contract-rejected",
        }
    }
}

/// Structured, stable failure from provider review construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderReviewError {
    kind: ProviderReviewErrorKind,
    subject: String,
    reference: String,
    wesley_kind: Option<GenerationContractErrorKind>,
}

impl ProviderReviewError {
    /// Returns the stable high-level failure category.
    #[must_use]
    pub const fn kind(&self) -> ProviderReviewErrorKind {
        self.kind
    }

    /// Returns the coordinate or contract field that failed.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the expected identity or stable upstream error code.
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
        kind: ProviderReviewErrorKind,
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
            ProviderReviewErrorKind::WesleyContractRejected,
            error.subject,
            kind.as_str(),
        );
        result.wesley_kind = Some(kind);
        result
    }
}

impl fmt::Display for ProviderReviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider generation review {}: {} -> {}",
            self.kind.label(),
            self.subject,
            self.reference
        )
    }
}

impl std::error::Error for ProviderReviewError {}

/// Canonical non-authoritative review derived from verified provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderGenerationReviewV1 {
    role: String,
    coordinate: String,
    schema_contract: String,
    review: GenerationReviewV1,
    canonical_bytes: Vec<u8>,
    content_reference: GenerationArtifactReferenceV1,
}

impl ProviderGenerationReviewV1 {
    /// Returns the source-declared review role.
    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Returns the source-declared review coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }

    /// Returns the Wesley-owned review schema contract.
    #[must_use]
    pub fn schema_contract(&self) -> &str {
        &self.schema_contract
    }

    /// Returns the admitted Wesley review projection.
    #[must_use]
    pub const fn review(&self) -> &GenerationReviewV1 {
        &self.review
    }

    /// Returns exact canonical Wesley review JSON bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the exact-byte reference for the canonical review JSON.
    #[must_use]
    pub const fn content_reference(&self) -> &GenerationArtifactReferenceV1 {
        &self.content_reference
    }
}

/// Derives canonical non-authoritative review JSON from verified provenance.
///
/// The function is pure and performs no filesystem, process, environment,
/// registry, clock, network, package installation, or Echo runtime admission.
///
/// # Errors
///
/// Returns a structured failure when review evidence is undeclared or Wesley
/// rejects the input/provenance relationship, review, or content reference.
pub fn generate_provider_generation_review_v1(
    input: &ProviderGenerationInputV1,
    provenance: &ProviderGenerationProvenanceV1,
) -> Result<ProviderGenerationReviewV1, ProviderReviewError> {
    let declaration = input
        .semantic_source()
        .source()
        .generated_artifacts
        .iter()
        .find(|artifact| artifact.kind == GeneratedArtifactKind::ReviewArtifact)
        .ok_or_else(|| {
            ProviderReviewError::new(
                ProviderReviewErrorKind::EvidenceDeclarationMissing,
                "generationReview",
                "generatedArtifacts",
            )
        })?;
    let review = GenerationReviewV1::from_manifest(input.wesley_input(), provenance.manifest())
        .map_err(ProviderReviewError::wesley)?;
    let canonical_bytes = review
        .canonical_bytes()
        .map_err(ProviderReviewError::wesley)?;
    let content_reference =
        GenerationArtifactReferenceV1::for_bytes(&declaration.coordinate, &canonical_bytes)
            .map_err(ProviderReviewError::wesley)?;

    Ok(ProviderGenerationReviewV1 {
        role: declaration.role.clone(),
        coordinate: declaration.coordinate.clone(),
        schema_contract: declaration.schema_contract.clone(),
        review,
        canonical_bytes,
        content_reference,
    })
}
