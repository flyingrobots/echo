// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure construction and comparison of the checked Edict provider artifact corpus.
//!
//! The generator identity is an explicit, versioned frame of exact source and
//! dependency-lock bytes. The artifact corpus is assembled only from immutable,
//! already validated generation values. This module performs no filesystem,
//! environment, registry, process, clock, or network discovery.

use crate::provider_artifacts::ProviderPrimaryArtifactsV1;
use crate::provider_generation::ProviderGenerationInputV1;
use crate::provider_provenance::{
    ProviderGenerationProvenanceV1, ProviderGeneratorMaterialV1, ProviderProvenanceError,
    ProviderProvenanceErrorKind,
};
use crate::provider_review::{
    generate_provider_generation_review_v1, ProviderGenerationReviewV1, ProviderReviewError,
    ProviderReviewErrorKind,
};
use std::collections::BTreeMap;
use std::fmt;
use wesley_core::GenerationContractErrorKind;

const SOURCE_BUNDLE_PREFIX: &[u8] = b"echo.provider-artifact-generator.source-bundle/v1\0";
const GENERATOR_COORDINATE: &str = "echo-wesley-gen.provider-artifact-generator@1";
const GENERATED_CORPUS_PREFIX: &str = "schemas/edict-provider/generated/v1";
const EXPECTED_CORPUS_FILE_COUNT: usize = 22;
const MAX_SOURCE_FILE_COUNT: usize = 64;
const MAX_SOURCE_PATH_BYTES: usize = 512;
const MAX_SOURCE_FILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_SOURCE_BUNDLE_BYTES: usize = 32 * 1024 * 1024;
const MAX_CORPUS_PATH_BYTES: usize = 512;

/// Stable failure categories for generator-source and corpus construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderArtifactCorpusErrorKind {
    /// No generator source file was supplied.
    GeneratorSourceMissing,
    /// A generator source path is unsafe or non-canonical.
    GeneratorSourcePathInvalid,
    /// A generator source file has no exact bytes.
    GeneratorSourceEmpty,
    /// A generator source input exceeds a fixed v1 bound.
    GeneratorSourceTooLarge,
    /// The same generator source path was supplied more than once.
    GeneratorSourceDuplicate,
    /// The exact source frame could not form a Wesley generator identity.
    GeneratorMaterialInvalid,
    /// Provenance does not verify against the supplied exact materials.
    ProvenanceInvalid,
    /// The deterministic Wesley review could not be re-derived.
    ReviewInvalid,
    /// The supplied review differs from the review re-derived from provenance.
    ReviewMismatch,
    /// A corpus-relative path is unsafe or non-canonical.
    CorpusPathInvalid,
    /// The same corpus-relative path was supplied more than once.
    CorpusPathDuplicate,
    /// The rendered corpus does not contain the exact v1 file count.
    CorpusInventoryInvalid,
}

impl ProviderArtifactCorpusErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::GeneratorSourceMissing => "generator-source-missing",
            Self::GeneratorSourcePathInvalid => "generator-source-path-invalid",
            Self::GeneratorSourceEmpty => "generator-source-empty",
            Self::GeneratorSourceTooLarge => "generator-source-too-large",
            Self::GeneratorSourceDuplicate => "generator-source-duplicate",
            Self::GeneratorMaterialInvalid => "generator-material-invalid",
            Self::ProvenanceInvalid => "provenance-invalid",
            Self::ReviewInvalid => "review-invalid",
            Self::ReviewMismatch => "review-mismatch",
            Self::CorpusPathInvalid => "corpus-path-invalid",
            Self::CorpusPathDuplicate => "corpus-path-duplicate",
            Self::CorpusInventoryInvalid => "corpus-inventory-invalid",
        }
    }
}

/// Stable structured generator-source or corpus construction failure.
#[derive(Debug, PartialEq, Eq)]
pub struct ProviderArtifactCorpusError {
    kind: ProviderArtifactCorpusErrorKind,
    subject: String,
    reference: String,
    provenance_kind: Option<ProviderProvenanceErrorKind>,
    review_kind: Option<ProviderReviewErrorKind>,
    wesley_contract_kind: Option<GenerationContractErrorKind>,
}

impl ProviderArtifactCorpusError {
    /// Returns the stable top-level failure category.
    #[must_use]
    pub const fn kind(&self) -> ProviderArtifactCorpusErrorKind {
        self.kind
    }

    /// Returns the stable subject that failed validation.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the stable offending reference or field value.
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Returns the typed provenance failure when provenance verification failed.
    #[must_use]
    pub const fn provenance_kind(&self) -> Option<ProviderProvenanceErrorKind> {
        self.provenance_kind
    }

    /// Returns the typed review failure when review derivation failed.
    #[must_use]
    pub const fn review_kind(&self) -> Option<ProviderReviewErrorKind> {
        self.review_kind
    }

    /// Returns the nested typed Wesley failure when one caused this error.
    #[must_use]
    pub const fn wesley_contract_kind(&self) -> Option<GenerationContractErrorKind> {
        self.wesley_contract_kind
    }

    fn new(
        kind: ProviderArtifactCorpusErrorKind,
        subject: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject: subject.into(),
            reference: reference.into(),
            provenance_kind: None,
            review_kind: None,
            wesley_contract_kind: None,
        }
    }

    fn generator_material(error: ProviderProvenanceError) -> Self {
        Self {
            kind: ProviderArtifactCorpusErrorKind::GeneratorMaterialInvalid,
            subject: error.subject().to_owned(),
            reference: error.reference().to_owned(),
            provenance_kind: Some(error.kind()),
            review_kind: None,
            wesley_contract_kind: error.wesley_contract_kind(),
        }
    }

    fn provenance(error: ProviderProvenanceError) -> Self {
        Self {
            kind: ProviderArtifactCorpusErrorKind::ProvenanceInvalid,
            subject: error.subject().to_owned(),
            reference: error.reference().to_owned(),
            provenance_kind: Some(error.kind()),
            review_kind: None,
            wesley_contract_kind: error.wesley_contract_kind(),
        }
    }

    fn review(error: ProviderReviewError) -> Self {
        Self {
            kind: ProviderArtifactCorpusErrorKind::ReviewInvalid,
            subject: error.subject().to_owned(),
            reference: error.reference().to_owned(),
            provenance_kind: None,
            review_kind: Some(error.kind()),
            wesley_contract_kind: error.wesley_contract_kind(),
        }
    }
}

impl fmt::Display for ProviderArtifactCorpusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {} ({})",
            self.kind.label(),
            self.subject,
            self.reference
        )
    }
}

impl std::error::Error for ProviderArtifactCorpusError {}

/// One exact source file included in provider generator identity v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderGeneratorSourceFileV1 {
    path: String,
    bytes: Vec<u8>,
}

impl ProviderGeneratorSourceFileV1 {
    /// Constructs one exact source file after validating its repo-relative path.
    ///
    /// # Errors
    ///
    /// Returns a stable failure for unsafe paths, generated-output paths, empty
    /// content, or content beyond the fixed v1 source-file bound.
    pub fn new(path: impl Into<String>, bytes: &[u8]) -> Result<Self, ProviderArtifactCorpusError> {
        let path = path.into();
        validate_source_path(&path)?;
        if bytes.is_empty() {
            return Err(ProviderArtifactCorpusError::new(
                ProviderArtifactCorpusErrorKind::GeneratorSourceEmpty,
                &path,
                "0",
            ));
        }
        if bytes.len() > MAX_SOURCE_FILE_BYTES {
            return Err(ProviderArtifactCorpusError::new(
                ProviderArtifactCorpusErrorKind::GeneratorSourceTooLarge,
                &path,
                bytes.len().to_string(),
            ));
        }
        Ok(Self {
            path,
            bytes: bytes.to_vec(),
        })
    }

    /// Returns the canonical repo-relative POSIX path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the exact, unnormalized file bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Canonical v1 frame of the exact provider generator source closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderGeneratorSourceBundleV1 {
    source_files: Vec<ProviderGeneratorSourceFileV1>,
    canonical_bytes: Vec<u8>,
}

impl ProviderGeneratorSourceBundleV1 {
    /// Returns source files sorted by raw UTF-8 path bytes.
    #[must_use]
    pub fn source_files(&self) -> &[ProviderGeneratorSourceFileV1] {
        &self.source_files
    }

    /// Returns the exact canonical source-bundle frame.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Constructs Wesley generator material from the exact source-bundle frame.
    ///
    /// # Errors
    ///
    /// Returns a stable failure if Wesley rejects the fixed coordinate, crate
    /// version, or exact source-bundle bytes.
    pub fn generator_material(
        &self,
    ) -> Result<ProviderGeneratorMaterialV1, ProviderArtifactCorpusError> {
        ProviderGeneratorMaterialV1::new(
            GENERATOR_COORDINATE,
            env!("CARGO_PKG_VERSION"),
            &self.canonical_bytes,
        )
        .map_err(ProviderArtifactCorpusError::generator_material)
    }
}

/// Builds a canonical source-bundle frame from explicit exact file bytes.
///
/// Source declaration order is set-like: paths are sorted by raw UTF-8 bytes.
/// File paths and contents remain exact and are never discovered or normalized.
///
/// # Errors
///
/// Returns a stable failure for an empty or oversized collection, duplicate
/// paths, or a source frame beyond the fixed v1 bound.
pub fn build_provider_generator_source_bundle_v1(
    mut source_files: Vec<ProviderGeneratorSourceFileV1>,
) -> Result<ProviderGeneratorSourceBundleV1, ProviderArtifactCorpusError> {
    if source_files.is_empty() {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::GeneratorSourceMissing,
            GENERATOR_COORDINATE,
            "source-files",
        ));
    }
    if source_files.len() > MAX_SOURCE_FILE_COUNT {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::GeneratorSourceTooLarge,
            GENERATOR_COORDINATE,
            source_files.len().to_string(),
        ));
    }

    source_files.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    for pair in source_files.windows(2) {
        if pair[0].path == pair[1].path {
            return Err(ProviderArtifactCorpusError::new(
                ProviderArtifactCorpusErrorKind::GeneratorSourceDuplicate,
                &pair[0].path,
                "source-files",
            ));
        }
    }

    let mut canonical_bytes = Vec::new();
    extend_source_bundle(&mut canonical_bytes, SOURCE_BUNDLE_PREFIX, "bundle-prefix")?;
    append_u64(
        &mut canonical_bytes,
        source_files.len(),
        "source-file-count",
    )?;
    for source in &source_files {
        append_u64(&mut canonical_bytes, source.path.len(), &source.path)?;
        extend_source_bundle(&mut canonical_bytes, source.path.as_bytes(), &source.path)?;
        append_u64(&mut canonical_bytes, source.bytes.len(), &source.path)?;
        extend_source_bundle(&mut canonical_bytes, &source.bytes, &source.path)?;
    }

    Ok(ProviderGeneratorSourceBundleV1 {
        source_files,
        canonical_bytes,
    })
}

/// Builds the exact, compile-time-enumerated provider generator source closure.
///
/// Authored semantic input, settings, admitted contract bytes, and every
/// generated output are deliberately absent. Wesley binds those materials in
/// the generation input, provenance sources, and emitted-output closure.
///
/// # Errors
///
/// Returns a stable failure if a pinned source entry violates the v1 framing
/// contract. No filesystem or environment discovery is performed.
pub fn checked_provider_generator_source_bundle_v1(
) -> Result<ProviderGeneratorSourceBundleV1, ProviderArtifactCorpusError> {
    let sources: [(&str, &[u8]); 20] = [
        (
            "Cargo.lock",
            include_bytes!("../assets/v1/repository/Cargo.lock.source"),
        ),
        (
            "Cargo.toml",
            include_bytes!("../assets/v1/repository/Cargo.toml.source"),
        ),
        (
            "crates/echo-edict-canonical/Cargo.toml",
            include_bytes!("../assets/v1/repository/crates/echo-edict-canonical/Cargo.toml.source"),
        ),
        (
            "crates/echo-edict-canonical/src/lib.rs",
            include_bytes!("../assets/v1/repository/crates/echo-edict-canonical/src/lib.rs.source"),
        ),
        (
            "crates/echo-registry-api/Cargo.toml",
            include_bytes!("../assets/v1/repository/crates/echo-registry-api/Cargo.toml.source"),
        ),
        (
            "crates/echo-registry-api/src/lib.rs",
            include_bytes!("../assets/v1/repository/crates/echo-registry-api/src/lib.rs.source"),
        ),
        (
            "crates/echo-registry-api/src/provider.rs",
            include_bytes!(
                "../assets/v1/repository/crates/echo-registry-api/src/provider.rs.source"
            ),
        ),
        (
            "crates/echo-wesley-gen/Cargo.toml",
            include_bytes!("../assets/v1/repository/crates/echo-wesley-gen/Cargo.toml.source"),
        ),
        (
            "crates/echo-wesley-gen/src/bin/echo-edict-provider-artifacts.rs",
            include_bytes!("bin/echo-edict-provider-artifacts.rs"),
        ),
        (
            "crates/echo-wesley-gen/src/lib.rs",
            include_bytes!("lib.rs"),
        ),
        (
            "crates/echo-wesley-gen/src/provider_artifacts.rs",
            include_bytes!("provider_artifacts.rs"),
        ),
        (
            "crates/echo-wesley-gen/src/provider_canonical.rs",
            include_bytes!("provider_canonical.rs"),
        ),
        (
            "crates/echo-wesley-gen/src/provider_contract_pack.rs",
            include_bytes!("provider_contract_pack.rs"),
        ),
        (
            "crates/echo-wesley-gen/src/provider_corpus.rs",
            include_bytes!("provider_corpus.rs"),
        ),
        (
            "crates/echo-wesley-gen/src/provider_corpus_fs.rs",
            include_bytes!("provider_corpus_fs.rs"),
        ),
        (
            "crates/echo-wesley-gen/src/provider_generation.rs",
            include_bytes!("provider_generation.rs"),
        ),
        (
            "crates/echo-wesley-gen/src/provider_provenance.rs",
            include_bytes!("provider_provenance.rs"),
        ),
        (
            "crates/echo-wesley-gen/src/provider_review.rs",
            include_bytes!("provider_review.rs"),
        ),
        (
            "crates/echo-wesley-gen/src/provider_semantics.rs",
            include_bytes!("provider_semantics.rs"),
        ),
        (
            "rust-toolchain.toml",
            include_bytes!("../assets/v1/repository/rust-toolchain.toml.source"),
        ),
    ];
    let mut files = Vec::with_capacity(sources.len());
    for (path, bytes) in sources {
        files.push(ProviderGeneratorSourceFileV1::new(path, bytes)?);
    }
    build_provider_generator_source_bundle_v1(files)
}

/// One exact file in the checked provider artifact corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderArtifactCorpusFileV1 {
    relative_path: String,
    bytes: Vec<u8>,
}

impl ProviderArtifactCorpusFileV1 {
    /// Constructs an exact corpus file with a safe relative POSIX path.
    ///
    /// # Errors
    ///
    /// Returns a stable failure when the path is absolute, traversing, empty,
    /// contains control characters or backslashes, has a Windows drive prefix,
    /// or exceeds the fixed v1 path bound.
    pub fn new(
        relative_path: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, ProviderArtifactCorpusError> {
        let relative_path = relative_path.into();
        validate_corpus_path(&relative_path)?;
        Ok(Self {
            relative_path,
            bytes: bytes.to_vec(),
        })
    }

    /// Returns the slash-normalized path relative to the corpus root.
    #[must_use]
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    /// Returns the exact file bytes without re-encoding or normalization.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Complete deterministic v1 provider artifact corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderArtifactCorpusV1 {
    files: Vec<ProviderArtifactCorpusFileV1>,
}

impl ProviderArtifactCorpusV1 {
    /// Returns all 22 exact files in lexicographic path order.
    #[must_use]
    pub fn files(&self) -> &[ProviderArtifactCorpusFileV1] {
        &self.files
    }

    /// Returns one exact corpus file by relative path.
    #[must_use]
    pub fn file(&self, relative_path: &str) -> Option<&ProviderArtifactCorpusFileV1> {
        self.files
            .iter()
            .find(|file| file.relative_path == relative_path)
    }
}

/// Loads the current checked 22-file provider corpus introduced by #652.
///
/// This is an explicit source boundary for later package assembly. It performs
/// no filesystem, registry, environment, process, clock, or network discovery.
///
/// # Errors
///
/// Returns a structured error if a checked logical path no longer satisfies the
/// bounded v1 corpus grammar.
pub fn checked_provider_artifact_corpus_v1(
) -> Result<ProviderArtifactCorpusV1, ProviderArtifactCorpusError> {
    let checked: [(&str, &[u8]); EXPECTED_CORPUS_FILE_COUNT] = [
        (
            "evidence/provenance.provider-generation.json",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/evidence/provenance.provider-generation.json"
            ),
        ),
        (
            "evidence/review.provider-generation.json",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/evidence/review.provider-generation.json"
            ),
        ),
        (
            "primary/authority-facts.echo-dpo.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/primary/authority-facts.echo-dpo.cbor"
            ),
        ),
        (
            "primary/authority-facts.echo-lawpack.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/primary/authority-facts.echo-lawpack.cbor"
            ),
        ),
        (
            "primary/generated-artifact-profile.echo-dpo-registration.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/primary/generated-artifact-profile.echo-dpo-registration.cbor"
            ),
        ),
        (
            "primary/lawpack.echo-dpo.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/primary/lawpack.echo-dpo.cbor"
            ),
        ),
        (
            "primary/schema.echo-provider-artifacts.cddl",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/primary/schema.echo-provider-artifacts.cddl"
            ),
        ),
        (
            "primary/target-profile.echo-dpo.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/primary/target-profile.echo-dpo.cbor"
            ),
        ),
        (
            "resources/resource.conformance-corpus.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.conformance-corpus.cbor"
            ),
        ),
        (
            "resources/resource.lawpack-compatibility.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.lawpack-compatibility.cbor"
            ),
        ),
        (
            "resources/resource.lawpack-exports.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.lawpack-exports.cbor"
            ),
        ),
        (
            "resources/resource.lawpack-target-adapter.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.lawpack-target-adapter.cbor"
            ),
        ),
        (
            "resources/resource.lawpack-verifier.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.lawpack-verifier.cbor"
            ),
        ),
        (
            "resources/resource.target-bundle-profile.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.target-bundle-profile.cbor"
            ),
        ),
        (
            "resources/resource.target-cost-algebra.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.target-cost-algebra.cbor"
            ),
        ),
        (
            "resources/resource.target-footprint-algebra.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.target-footprint-algebra.cbor"
            ),
        ),
        (
            "resources/resource.target-intrinsics.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.target-intrinsics.cbor"
            ),
        ),
        (
            "resources/resource.target-ir.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.target-ir.cbor"
            ),
        ),
        (
            "resources/resource.target-lowerer-contract.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.target-lowerer-contract.cbor"
            ),
        ),
        (
            "resources/resource.target-obstruction-taxonomy.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.target-obstruction-taxonomy.cbor"
            ),
        ),
        (
            "resources/resource.target-operation-profiles.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.target-operation-profiles.cbor"
            ),
        ),
        (
            "resources/resource.target-verifier-contract.cbor",
            include_bytes!(
                "../assets/v1/edict-provider/package/v1/generated/resources/resource.target-verifier-contract.cbor"
            ),
        ),
    ];
    let files = checked
        .into_iter()
        .map(|(path, bytes)| ProviderArtifactCorpusFileV1::new(path, bytes))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ProviderArtifactCorpusV1 { files })
}

/// Stable kinds of checked-corpus drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProviderArtifactCorpusDriftKind {
    /// An expected file is absent.
    Missing,
    /// An expected path contains different exact bytes.
    Changed,
    /// An actual path is not part of the exact v1 corpus.
    Unexpected,
}

impl ProviderArtifactCorpusDriftKind {
    /// Returns the stable diagnostic spelling for this drift kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Changed => "changed",
            Self::Unexpected => "unexpected",
        }
    }
}

/// One stable exact-byte corpus drift diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderArtifactCorpusDriftV1 {
    kind: ProviderArtifactCorpusDriftKind,
    relative_path: String,
}

impl ProviderArtifactCorpusDriftV1 {
    /// Returns whether this file is missing, changed, or unexpected.
    #[must_use]
    pub const fn kind(&self) -> ProviderArtifactCorpusDriftKind {
        self.kind
    }

    /// Returns the stable corpus-relative path.
    #[must_use]
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }
}

/// Renders the exact 22-file provider corpus from verified generation values.
///
/// Before exposing bytes, this re-verifies provenance against the supplied
/// input, primary outputs, and generator material, then re-derives and compares
/// the non-authoritative review. CBOR, CDDL, and JSON bytes are copied exactly.
///
/// # Errors
///
/// Returns a stable structured failure for incoherent generation materials,
/// review mismatch, unsafe role-derived paths, duplicates, or wrong inventory.
pub fn render_provider_artifact_corpus_v1(
    input: &ProviderGenerationInputV1,
    primary: &ProviderPrimaryArtifactsV1,
    generator: &ProviderGeneratorMaterialV1,
    provenance: &ProviderGenerationProvenanceV1,
    review: &ProviderGenerationReviewV1,
) -> Result<ProviderArtifactCorpusV1, ProviderArtifactCorpusError> {
    provenance
        .verify_exact_materials(input, primary, generator)
        .map_err(ProviderArtifactCorpusError::provenance)?;
    let expected_review = generate_provider_generation_review_v1(input, provenance)
        .map_err(ProviderArtifactCorpusError::review)?;
    if &expected_review != review {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::ReviewMismatch,
            review.coordinate(),
            &review.content_reference().digest,
        ));
    }

    let mut files = Vec::with_capacity(EXPECTED_CORPUS_FILE_COUNT);
    for artifact in primary.artifacts() {
        validate_filename_role(artifact.role())?;
        files.push(ProviderArtifactCorpusFileV1::new(
            format!("primary/{}.cbor", artifact.role()),
            artifact.canonical_bytes(),
        )?);
    }
    validate_filename_role(primary.schema().role())?;
    files.push(ProviderArtifactCorpusFileV1::new(
        format!("primary/{}.cddl", primary.schema().role()),
        primary.schema().bytes(),
    )?);
    for resource in primary.resources() {
        validate_filename_role(resource.role())?;
        files.push(ProviderArtifactCorpusFileV1::new(
            format!("resources/{}.cbor", resource.role()),
            resource.canonical_bytes(),
        )?);
    }
    validate_filename_role(provenance.role())?;
    files.push(ProviderArtifactCorpusFileV1::new(
        format!("evidence/{}.json", provenance.role()),
        provenance.canonical_bytes(),
    )?);
    validate_filename_role(review.role())?;
    files.push(ProviderArtifactCorpusFileV1::new(
        format!("evidence/{}.json", review.role()),
        review.canonical_bytes(),
    )?);

    files.sort_by(|left, right| {
        left.relative_path
            .as_bytes()
            .cmp(right.relative_path.as_bytes())
    });
    for pair in files.windows(2) {
        if pair[0].relative_path == pair[1].relative_path {
            return Err(ProviderArtifactCorpusError::new(
                ProviderArtifactCorpusErrorKind::CorpusPathDuplicate,
                &pair[0].relative_path,
                "rendered-corpus",
            ));
        }
    }
    if files.len() != EXPECTED_CORPUS_FILE_COUNT {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::CorpusInventoryInvalid,
            "provider-artifact-corpus/v1",
            files.len().to_string(),
        ));
    }
    Ok(ProviderArtifactCorpusV1 { files })
}

/// Compares an actual file inventory with the exact rendered corpus.
///
/// The returned drift is sorted first by `missing`, `changed`, `unexpected`,
/// and then lexicographically by relative path. This function is pure and does
/// not read, write, create, normalize, or delete filesystem entries.
///
/// # Errors
///
/// Returns a stable failure if the actual inventory repeats a relative path.
pub fn diff_provider_artifact_corpus_v1(
    expected: &ProviderArtifactCorpusV1,
    actual: &[ProviderArtifactCorpusFileV1],
) -> Result<Vec<ProviderArtifactCorpusDriftV1>, ProviderArtifactCorpusError> {
    diff_exact_corpus_files_v1(&expected.files, actual)
}

/// Compares two exact file inventories without imposing a corpus-specific size.
///
/// The returned drift is sorted first by `missing`, `changed`, `unexpected`,
/// and then lexicographically by relative path. This function is pure and does
/// not read, write, create, normalize, or delete filesystem entries.
///
/// # Errors
///
/// Returns a stable failure if either inventory repeats a relative path.
pub fn diff_exact_corpus_files_v1(
    expected: &[ProviderArtifactCorpusFileV1],
    actual: &[ProviderArtifactCorpusFileV1],
) -> Result<Vec<ProviderArtifactCorpusDriftV1>, ProviderArtifactCorpusError> {
    let expected_by_path = corpus_inventory(expected, "expected-corpus")?;
    let actual_by_path = corpus_inventory(actual, "actual-corpus")?;
    let mut drift = Vec::new();

    for (path, expected_bytes) in &expected_by_path {
        match actual_by_path.get(path) {
            None => drift.push(ProviderArtifactCorpusDriftV1 {
                kind: ProviderArtifactCorpusDriftKind::Missing,
                relative_path: (*path).to_owned(),
            }),
            Some(actual_bytes) if *actual_bytes != *expected_bytes => {
                drift.push(ProviderArtifactCorpusDriftV1 {
                    kind: ProviderArtifactCorpusDriftKind::Changed,
                    relative_path: (*path).to_owned(),
                });
            }
            Some(_) => {}
        }
    }
    for path in actual_by_path.keys() {
        if !expected_by_path.contains_key(path) {
            drift.push(ProviderArtifactCorpusDriftV1 {
                kind: ProviderArtifactCorpusDriftKind::Unexpected,
                relative_path: (*path).to_owned(),
            });
        }
    }
    drift.sort_by(|left, right| {
        left.kind.cmp(&right.kind).then_with(|| {
            left.relative_path
                .as_bytes()
                .cmp(right.relative_path.as_bytes())
        })
    });
    Ok(drift)
}

fn validate_source_path(path: &str) -> Result<(), ProviderArtifactCorpusError> {
    if path.len() > MAX_SOURCE_PATH_BYTES {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::GeneratorSourceTooLarge,
            path,
            path.len().to_string(),
        ));
    }
    let valid_characters = path
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'));
    if !valid_characters || !has_safe_relative_segments(path) {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::GeneratorSourcePathInvalid,
            path,
            "relative-posix-source-path",
        ));
    }
    if path == GENERATED_CORPUS_PREFIX
        || path
            .strip_prefix(GENERATED_CORPUS_PREFIX)
            .is_some_and(|suffix| suffix.starts_with('/'))
    {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::GeneratorSourcePathInvalid,
            path,
            "generated-output-self-reference",
        ));
    }
    Ok(())
}

pub(crate) fn validate_corpus_path(path: &str) -> Result<(), ProviderArtifactCorpusError> {
    if path.len() > MAX_CORPUS_PATH_BYTES {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::CorpusPathInvalid,
            path,
            path.len().to_string(),
        ));
    }
    let windows_drive_prefix = path.as_bytes().get(1) == Some(&b':')
        && path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic);
    if path.chars().any(char::is_control)
        || path.contains('\\')
        || windows_drive_prefix
        || !has_safe_relative_segments(path)
    {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::CorpusPathInvalid,
            path,
            "relative-posix-path",
        ));
    }
    Ok(())
}

fn has_safe_relative_segments(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.ends_with('/')
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn validate_filename_role(role: &str) -> Result<(), ProviderArtifactCorpusError> {
    if role.is_empty()
        || !role.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
    {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::CorpusPathInvalid,
            role,
            "artifact-role-filename",
        ));
    }
    Ok(())
}

fn append_u64(
    output: &mut Vec<u8>,
    value: usize,
    subject: &str,
) -> Result<(), ProviderArtifactCorpusError> {
    let value = u64::try_from(value).map_err(|_| {
        ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::GeneratorSourceTooLarge,
            subject,
            "u64-length",
        )
    })?;
    extend_source_bundle(output, &value.to_be_bytes(), subject)
}

fn extend_source_bundle(
    output: &mut Vec<u8>,
    bytes: &[u8],
    subject: &str,
) -> Result<(), ProviderArtifactCorpusError> {
    let length = output.len().checked_add(bytes.len()).ok_or_else(|| {
        ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::GeneratorSourceTooLarge,
            subject,
            "source-bundle-length-overflow",
        )
    })?;
    if length > MAX_SOURCE_BUNDLE_BYTES {
        return Err(ProviderArtifactCorpusError::new(
            ProviderArtifactCorpusErrorKind::GeneratorSourceTooLarge,
            subject,
            length.to_string(),
        ));
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn corpus_inventory<'a>(
    files: &'a [ProviderArtifactCorpusFileV1],
    reference: &str,
) -> Result<BTreeMap<&'a str, &'a [u8]>, ProviderArtifactCorpusError> {
    let mut inventory = BTreeMap::new();
    for file in files {
        if inventory
            .insert(file.relative_path(), file.bytes())
            .is_some()
        {
            return Err(ProviderArtifactCorpusError::new(
                ProviderArtifactCorpusErrorKind::CorpusPathDuplicate,
                file.relative_path(),
                reference,
            ));
        }
    }
    Ok(inventory)
}
