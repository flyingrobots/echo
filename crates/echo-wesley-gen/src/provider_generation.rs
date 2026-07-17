// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure assembly of the canonical Wesley input for Echo provider generation.
//!
//! The first checked Echo provider closure deliberately has no GraphQL Shape
//! facts. Its operation is an Echo semantic declaration, not a synthetic
//! GraphQL root field. This module therefore supplies Wesley's exact empty L1
//! Shape and operation catalog and binds the Echo source, Edict CDDL, Edict
//! manifest, and settings bytes without discovering any of them.

use std::fmt;

use serde::Deserialize;
use wesley_core::{
    compute_generation_artifact_digest_v1, ExtensionGenerationInputV1, GenerationArtifactContentV1,
    GenerationArtifactReferenceV1, GenerationContractError, WesleyIR,
    WESLEY_EXTENSION_GENERATOR_ABI_VERSION,
};

use crate::provider_contract_pack::AdmittedProviderContractPackV1;
use crate::provider_semantics::{
    parse_provider_semantic_source_v1, AuthoritySourceKind, GeneratedArtifactKind,
    ProviderSemanticSourceError, ProviderSemanticSourceErrorKind,
    ValidatedProviderSemanticSourceV1,
};

/// Exact API accepted for Echo provider generator settings.
pub const ECHO_PROVIDER_GENERATION_SETTINGS_API_V1: &str =
    "echo.edict-provider-generation-settings/v1";

/// Source coordinate used for the exact Edict contract-pack manifest bytes.
pub const EDICT_PROVIDER_CONTRACT_PACK_MANIFEST_COORDINATE_V1: &str =
    "edict.provider-contract-pack.manifest@1";

const NO_SHAPE_SOURCE: &str = "none";
const EMPTY_WESLEY_SHAPE_VERSION: &str = "1.0.0";
const CANONICAL_ARTIFACT_ENCODING: &str = "edict.canonical-cbor/v1";
const MAX_SEMANTIC_SOURCE_BYTES: usize = 1_048_576;
const MAX_SETTINGS_BYTES: usize = 16_384;

/// Canonical generation input plus exact materials needed for provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderGenerationInputV1 {
    semantic_source: ValidatedProviderSemanticSourceV1,
    settings_bytes: Vec<u8>,
    wesley_input: ExtensionGenerationInputV1,
    canonical_bytes: Vec<u8>,
    digest: String,
    source_artifacts: Vec<GenerationArtifactContentV1>,
}

impl ProviderGenerationInputV1 {
    /// Returns the normalized, validated Echo semantic source.
    #[must_use]
    pub const fn semantic_source(&self) -> &ValidatedProviderSemanticSourceV1 {
        &self.semantic_source
    }

    /// Returns the exact versioned settings bytes bound by the input digest.
    #[must_use]
    pub fn settings_bytes(&self) -> &[u8] {
        &self.settings_bytes
    }

    /// Returns the normalized Wesley extension-generation input.
    #[must_use]
    pub const fn wesley_input(&self) -> &ExtensionGenerationInputV1 {
        &self.wesley_input
    }

    /// Returns canonical Wesley generation-input bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the domain-separated Wesley generation-input digest.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Returns exact source materials used for later provenance verification.
    #[must_use]
    pub fn source_artifacts(&self) -> &[GenerationArtifactContentV1] {
        &self.source_artifacts
    }
}

/// Stable failure categories returned while assembling generation input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderGenerationErrorKind {
    /// Semantic source or settings input exceeded its pre-parse size bound.
    InputSizeExceeded,
    /// Exact semantic-source bytes were not valid UTF-8 or failed validation.
    SemanticSourceInvalid,
    /// Generator settings did not match the strict versioned JSON shape.
    SettingsMalformed,
    /// Generator settings selected an unsupported API version.
    UnsupportedSettingsApiVersion,
    /// Generator settings selected a different frozen input contract.
    SettingsContractMismatch,
    /// A GraphQL authority source had no explicit Shape source bytes.
    GraphqlSourceMissing,
    /// Wesley rejected owner declarations or canonical generation input.
    WesleyContractRejected,
}

impl ProviderGenerationErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::InputSizeExceeded => "input-size-exceeded",
            Self::SemanticSourceInvalid => "semantic-source-invalid",
            Self::SettingsMalformed => "settings-malformed",
            Self::UnsupportedSettingsApiVersion => "unsupported-settings-api-version",
            Self::SettingsContractMismatch => "settings-contract-mismatch",
            Self::GraphqlSourceMissing => "graphql-source-missing",
            Self::WesleyContractRejected => "wesley-contract-rejected",
        }
    }
}

/// Structured provider-generation input failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderGenerationError {
    kind: ProviderGenerationErrorKind,
    semantic_source_kind: Option<ProviderSemanticSourceErrorKind>,
    subject: String,
    reference: String,
}

impl ProviderGenerationError {
    /// Returns the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> ProviderGenerationErrorKind {
        self.kind
    }

    /// Returns the underlying semantic-source failure category, when present.
    #[must_use]
    pub const fn semantic_source_kind(&self) -> Option<ProviderSemanticSourceErrorKind> {
        self.semantic_source_kind
    }

    /// Returns the input field or coordinate that failed.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the stable conflicting or expected reference.
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    fn new(
        kind: ProviderGenerationErrorKind,
        subject: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            semantic_source_kind: None,
            subject: subject.into(),
            reference: reference.into(),
        }
    }

    fn semantic_source(error: ProviderSemanticSourceError) -> Self {
        Self {
            kind: ProviderGenerationErrorKind::SemanticSourceInvalid,
            semantic_source_kind: Some(error.kind()),
            subject: error.subject().to_owned(),
            reference: error.reference().to_owned(),
        }
    }

    fn wesley(error: GenerationContractError) -> Self {
        Self::new(
            ProviderGenerationErrorKind::WesleyContractRejected,
            error.subject,
            error.kind.as_str(),
        )
    }
}

impl fmt::Display for ProviderGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider generation {}: {} -> {}",
            self.kind.label(),
            self.subject,
            self.reference
        )
    }
}

impl std::error::Error for ProviderGenerationError {}

/// Builds the canonical Wesley input for one explicit Echo provider invocation.
///
/// The exact semantic-source, contract-pack, and settings bytes are caller
/// inputs. This function performs no filesystem, registry, environment, clock,
/// process, or network access.
///
/// # Errors
///
/// Returns a stable structured error when source or settings bytes are invalid,
/// select a different frozen contract, or cannot form a canonical Wesley input.
pub fn build_provider_generation_input_v1(
    semantic_source_bytes: &[u8],
    contract_pack: &AdmittedProviderContractPackV1,
    settings_bytes: &[u8],
) -> Result<ProviderGenerationInputV1, ProviderGenerationError> {
    preflight_size(
        semantic_source_bytes,
        MAX_SEMANTIC_SOURCE_BYTES,
        "semanticSource.bytes",
    )?;
    preflight_size(settings_bytes, MAX_SETTINGS_BYTES, "settings.bytes")?;

    let semantic_source_text = std::str::from_utf8(semantic_source_bytes).map_err(|_| {
        ProviderGenerationError::new(
            ProviderGenerationErrorKind::SemanticSourceInvalid,
            "semanticSource",
            "utf-8",
        )
    })?;
    let semantic_source = parse_provider_semantic_source_v1(semantic_source_text)
        .map_err(ProviderGenerationError::semantic_source)?;
    validate_settings(settings_bytes, contract_pack)?;
    if let Some(authority) = semantic_source
        .source()
        .authority_sources
        .iter()
        .find(|authority| authority.kind == AuthoritySourceKind::Graphql)
    {
        return Err(ProviderGenerationError::new(
            ProviderGenerationErrorKind::GraphqlSourceMissing,
            &authority.coordinate,
            &authority.artifact,
        ));
    }

    let shape_ir = WesleyIR {
        version: EMPTY_WESLEY_SHAPE_VERSION.to_owned(),
        metadata: None,
        types: Vec::new(),
    };
    let operations = Vec::new();

    let source_artifacts = vec![
        GenerationArtifactContentV1::new(
            semantic_source.source().coordinate.clone(),
            semantic_source_bytes.to_vec(),
        ),
        GenerationArtifactContentV1::new(
            contract_pack.coordinate(),
            contract_pack.schema_bytes().to_vec(),
        ),
        GenerationArtifactContentV1::new(
            EDICT_PROVIDER_CONTRACT_PACK_MANIFEST_COORDINATE_V1,
            contract_pack.manifest_bytes().to_vec(),
        ),
    ];
    let owner_declarations = source_artifacts
        .iter()
        .map(GenerationArtifactContentV1::reference)
        .collect::<Vec<GenerationArtifactReferenceV1>>();
    let projection_roles = semantic_source
        .source()
        .generated_artifacts
        .iter()
        .filter(|artifact| {
            !matches!(
                artifact.kind,
                GeneratedArtifactKind::GenerationProvenance | GeneratedArtifactKind::ReviewArtifact
            )
        })
        .map(|artifact| artifact.role.clone())
        .collect();

    let wesley_input = ExtensionGenerationInputV1::new(
        shape_ir,
        operations,
        None,
        owner_declarations,
        compute_generation_artifact_digest_v1(settings_bytes),
        projection_roles,
    )
    .map_err(ProviderGenerationError::wesley)?;
    let canonical_bytes = wesley_input
        .canonical_bytes()
        .map_err(ProviderGenerationError::wesley)?;
    let digest = wesley_input
        .digest()
        .map_err(ProviderGenerationError::wesley)?;
    Ok(ProviderGenerationInputV1 {
        semantic_source,
        settings_bytes: settings_bytes.to_vec(),
        wesley_input,
        canonical_bytes,
        digest,
        source_artifacts,
    })
}

fn preflight_size(
    bytes: &[u8],
    maximum: usize,
    subject: &'static str,
) -> Result<(), ProviderGenerationError> {
    if bytes.len() <= maximum {
        Ok(())
    } else {
        Err(ProviderGenerationError::new(
            ProviderGenerationErrorKind::InputSizeExceeded,
            subject,
            maximum.to_string(),
        ))
    }
}

fn validate_settings(
    settings_bytes: &[u8],
    contract_pack: &AdmittedProviderContractPackV1,
) -> Result<(), ProviderGenerationError> {
    let settings =
        serde_json::from_slice::<ProviderGenerationSettings>(settings_bytes).map_err(|_| {
            ProviderGenerationError::new(
                ProviderGenerationErrorKind::SettingsMalformed,
                "settings",
                ECHO_PROVIDER_GENERATION_SETTINGS_API_V1,
            )
        })?;
    if settings.api_version != ECHO_PROVIDER_GENERATION_SETTINGS_API_V1 {
        return Err(ProviderGenerationError::new(
            ProviderGenerationErrorKind::UnsupportedSettingsApiVersion,
            "settings.apiVersion",
            ECHO_PROVIDER_GENERATION_SETTINGS_API_V1,
        ));
    }
    for (subject, actual, expected) in [
        (
            "settings.shapeSource",
            settings.shape_source.as_str(),
            NO_SHAPE_SOURCE,
        ),
        (
            "settings.canonicalArtifactEncoding",
            settings.canonical_artifact_encoding.as_str(),
            CANONICAL_ARTIFACT_ENCODING,
        ),
        (
            "settings.contractPack",
            settings.contract_pack.as_str(),
            contract_pack.coordinate(),
        ),
        (
            "settings.generatorAbi",
            settings.generator_abi.as_str(),
            WESLEY_EXTENSION_GENERATOR_ABI_VERSION,
        ),
    ] {
        if actual != expected {
            return Err(ProviderGenerationError::new(
                ProviderGenerationErrorKind::SettingsContractMismatch,
                subject,
                expected,
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProviderGenerationSettings {
    api_version: String,
    shape_source: String,
    canonical_artifact_encoding: String,
    contract_pack: String,
    generator_abi: String,
}
