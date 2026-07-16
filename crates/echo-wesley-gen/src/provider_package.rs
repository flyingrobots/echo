// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure assembly and admission of the digest-locked Echo Edict provider package.
//!
//! The Edict provider manifest is a typed JSON interchange envelope, not an
//! Edict canonical-CBOR artifact. Echo therefore owns its exact deterministic
//! JSON rendering and a separate, non-self-referential package identity. The
//! package identity binds the manifest semantics plus the raw exact-byte
//! identity of every non-manifest member. It does not install a provider or
//! grant Echo runtime authority.

use std::fmt;

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_v1, encode_canonical_cbor_v1, CanonicalValueV1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wesley_core::{
    GenerationArtifactReferenceV1, GenerationContractError, GenerationContractErrorKind,
    GenerationProvenanceManifestV1, GenerationReviewV1,
};

use crate::provider_artifacts::{
    GeneratedProviderSchemaV1, ProviderPrimaryArtifactsV1, SchemaValidatedCanonicalProviderOutputV1,
};
use crate::provider_corpus::{render_provider_artifact_corpus_v1, validate_corpus_path};
use crate::provider_generation::ProviderGenerationInputV1;
use crate::provider_provenance::{ProviderGenerationProvenanceV1, ProviderGeneratorMaterialV1};
use crate::provider_review::ProviderGenerationReviewV1;
use crate::provider_semantics::{
    ArtifactSchemaFormat, GeneratedArtifactDeclaration, GeneratedArtifactKind,
    ProviderComponentDeclaration, ProviderComponentKind,
};

/// Domain framing the Echo-owned package-closure identity.
pub const ECHO_PROVIDER_PACKAGE_CLOSURE_DOMAIN_V1: &str = "echo.edict-provider-package-closure/v1";
/// Exact deterministic JSON rendering contract for the Edict manifest.
pub const ECHO_PROVIDER_MANIFEST_ENCODING_V1: &str = "echo.provider-manifest.pretty-json/v1";
/// Logical package path of the derived provider manifest.
pub const ECHO_PROVIDER_MANIFEST_PATH_V1: &str = "provider-manifest.echo.json";

const PROVIDER_MANIFEST_API_V1: &str = "edict.provider-manifest/v1";
const PROVIDER_ABI_V1: &str = "edict:target-provider@1.0.0";
const PROVIDER_COORDINATE_V1: &str = "echo.edict-provider@1";
const PROVIDER_MANIFEST_ROLE_V1: &str = "provider-manifest.echo";
const PROVIDER_MANIFEST_COORDINATE_V1: &str = "echo.edict-provider-manifest@1";
const SEMANTIC_SOURCE_COORDINATE_V1: &str = "echo.semantic-schema@1";
const GENERATOR_COORDINATE_V1: &str = "echo-wesley-gen.provider-artifact-generator@1";
const EXPECTED_ARTIFACT_COUNT: usize = 10;
const EXPECTED_SCHEMA_BINDING_COUNT: usize = 9;
const EXPECTED_MEMBER_COUNT: usize = 24;
const EXPECTED_FILE_COUNT: usize = 25;
const MAX_COMPONENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_PACKAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_EVIDENCE_BYTES: usize = 4 * 1024 * 1024;
const MAX_DIAGNOSTIC_BYTES: usize = 256;

const EXPECTED_PACKAGE_PATHS: [&str; EXPECTED_FILE_COUNT] = [
    "components/lowerer.echo-dpo.component.wasm",
    "components/verifier.echo-dpo.component.wasm",
    "generated/evidence/provenance.provider-generation.json",
    "generated/evidence/review.provider-generation.json",
    "generated/primary/authority-facts.echo-dpo.cbor",
    "generated/primary/authority-facts.echo-lawpack.cbor",
    "generated/primary/generated-artifact-profile.echo-dpo-registration.cbor",
    "generated/primary/lawpack.echo-dpo.cbor",
    "generated/primary/schema.echo-provider-artifacts.cddl",
    "generated/primary/target-profile.echo-dpo.cbor",
    "generated/resources/resource.conformance-corpus.cbor",
    "generated/resources/resource.lawpack-compatibility.cbor",
    "generated/resources/resource.lawpack-exports.cbor",
    "generated/resources/resource.lawpack-target-adapter.cbor",
    "generated/resources/resource.lawpack-verifier.cbor",
    "generated/resources/resource.target-bundle-profile.cbor",
    "generated/resources/resource.target-cost-algebra.cbor",
    "generated/resources/resource.target-footprint-algebra.cbor",
    "generated/resources/resource.target-intrinsics.cbor",
    "generated/resources/resource.target-ir.cbor",
    "generated/resources/resource.target-lowerer-contract.cbor",
    "generated/resources/resource.target-obstruction-taxonomy.cbor",
    "generated/resources/resource.target-operation-profiles.cbor",
    "generated/resources/resource.target-verifier-contract.cbor",
    ECHO_PROVIDER_MANIFEST_PATH_V1,
];

/// Stable failures returned by provider-package assembly and admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderPackageErrorKind {
    /// Explicit component material was empty, oversized, or named an invalid role.
    ComponentInvalid,
    /// A source-declared component had no exact supplied bytes.
    ComponentMissing,
    /// A component role was supplied more than once.
    ComponentDuplicate,
    /// Supplied component material did not belong to the source-owned closure.
    ComponentUnexpected,
    /// The verified generated corpus could not be reproduced from exact inputs.
    GeneratedCorpusInvalid,
    /// A generated artifact did not match its source declaration.
    ArtifactMismatch,
    /// A required generated artifact was absent.
    ArtifactMissing,
    /// A package path was unsafe or outside the fixed v1 inventory.
    PackagePathInvalid,
    /// The complete explicit package exceeded the fixed v1 byte bound.
    PackageSizeExceeded,
    /// A required package member was absent.
    PackageMemberMissing,
    /// A package member path was repeated.
    PackageMemberDuplicate,
    /// A package contained a member outside the exact v1 closure.
    PackageMemberUnexpected,
    /// Supplied package files were not in strict path order.
    PackageMemberOutOfOrder,
    /// The manifest was not strict JSON matching the Echo v1 mirror.
    ManifestMalformed,
    /// The manifest bytes were an alternate rendering of the typed value.
    ManifestNoncanonical,
    /// The manifest selected a wrong API, ABI, role, coordinate, or closure.
    ManifestContractMismatch,
    /// A routed artifact's bytes did not reproduce its manifest digest.
    ArtifactDigestMismatch,
    /// A routed artifact used an incompatible provenance envelope.
    ArtifactSourceMismatch,
    /// Packaged Wesley provenance disagreed with generated routes or primary bytes.
    GenerationProvenanceMismatch,
    /// Packaged Wesley review disagreed with the canonical provenance projection.
    GenerationReviewMismatch,
    /// The manifest schema-domain closure differed from the exact source contract.
    SchemaBindingMismatch,
    /// A digest review string was not strict lowercase SHA-256.
    DigestInvalid,
    /// The canonical package-closure value could not be encoded or hashed.
    PackageIdentityFailed,
    /// The manifest provider identity, computed closure, and caller pin disagreed.
    ProviderIdentityMismatch,
    /// Deterministic manifest serialization failed.
    ManifestSerializationFailed,
}

impl ProviderPackageErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::ComponentInvalid => "component-invalid",
            Self::ComponentMissing => "component-missing",
            Self::ComponentDuplicate => "component-duplicate",
            Self::ComponentUnexpected => "component-unexpected",
            Self::GeneratedCorpusInvalid => "generated-corpus-invalid",
            Self::ArtifactMismatch => "artifact-mismatch",
            Self::ArtifactMissing => "artifact-missing",
            Self::PackagePathInvalid => "package-path-invalid",
            Self::PackageSizeExceeded => "package-size-exceeded",
            Self::PackageMemberMissing => "package-member-missing",
            Self::PackageMemberDuplicate => "package-member-duplicate",
            Self::PackageMemberUnexpected => "package-member-unexpected",
            Self::PackageMemberOutOfOrder => "package-member-out-of-order",
            Self::ManifestMalformed => "manifest-malformed",
            Self::ManifestNoncanonical => "manifest-noncanonical",
            Self::ManifestContractMismatch => "manifest-contract-mismatch",
            Self::ArtifactDigestMismatch => "artifact-digest-mismatch",
            Self::ArtifactSourceMismatch => "artifact-source-mismatch",
            Self::GenerationProvenanceMismatch => "generation-provenance-mismatch",
            Self::GenerationReviewMismatch => "generation-review-mismatch",
            Self::SchemaBindingMismatch => "schema-binding-mismatch",
            Self::DigestInvalid => "digest-invalid",
            Self::PackageIdentityFailed => "package-identity-failed",
            Self::ProviderIdentityMismatch => "provider-identity-mismatch",
            Self::ManifestSerializationFailed => "manifest-serialization-failed",
        }
    }
}

/// Structured provider-package failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderPackageError {
    kind: ProviderPackageErrorKind,
    subject: String,
    reference: String,
    wesley_contract_kind: Option<GenerationContractErrorKind>,
}

impl ProviderPackageError {
    /// Returns the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> ProviderPackageErrorKind {
        self.kind
    }

    /// Returns the member, role, or contract being validated.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the conflicting or rejected identity.
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Returns the typed Wesley cause for provenance or review rejection.
    #[must_use]
    pub const fn wesley_contract_kind(&self) -> Option<GenerationContractErrorKind> {
        self.wesley_contract_kind
    }

    fn new(
        kind: ProviderPackageErrorKind,
        subject: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject: bounded_diagnostic(subject.into()),
            reference: bounded_diagnostic(reference.into()),
            wesley_contract_kind: None,
        }
    }

    fn wesley(mut self, error: &GenerationContractError) -> Self {
        self.wesley_contract_kind = Some(error.kind);
        self
    }
}

impl fmt::Display for ProviderPackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider package {}: {} -> {}",
            self.kind.label(),
            self.subject,
            self.reference
        )
    }
}

impl std::error::Error for ProviderPackageError {}

/// Required digest-locked resource reference in the generated Edict manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderManifestResourceRefV1 {
    /// Stable owner-defined resource coordinate.
    pub coordinate: String,
    /// Strict lowercase SHA-256 review rendering.
    pub digest: String,
}

/// Artifact kinds mirrored from Edict's `edict.provider-manifest/v1` envelope.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderManifestArtifactKindV1 {
    /// Canonical Edict lawpack manifest.
    Lawpack,
    /// Canonical target-profile manifest.
    TargetProfile,
    /// Canonical authority-facts document.
    AuthorityFacts,
    /// Provider manifest artifact; excluded from this manifest's own inventory.
    ProviderManifest,
    /// Non-authoritative generation review JSON.
    ReviewArtifact,
    /// Echo generated-artifact profile.
    GeneratedArtifactProfile,
    /// Wesley generation-provenance JSON.
    GenerationProvenance,
    /// Self-contained generated CDDL schema.
    ArtifactSchema,
    /// Target-provider lowerer component.
    Lowerer,
    /// Target-provider verifier component.
    Verifier,
}

impl ProviderManifestArtifactKindV1 {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Lawpack => "lawpack",
            Self::TargetProfile => "targetProfile",
            Self::AuthorityFacts => "authorityFacts",
            Self::ProviderManifest => "providerManifest",
            Self::ReviewArtifact => "reviewArtifact",
            Self::GeneratedArtifactProfile => "generatedArtifactProfile",
            Self::GenerationProvenance => "generationProvenance",
            Self::ArtifactSchema => "artifactSchema",
            Self::Lowerer => "lowerer",
            Self::Verifier => "verifier",
        }
    }
}

/// Provenance envelope for one manifest artifact route.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ProviderManifestArtifactSourceV1 {
    /// Generated metadata bound to exact authored source and generator identities.
    Generated {
        /// Raw exact-byte semantic-source reference.
        #[serde(rename = "semanticSource")]
        semantic_source: ProviderManifestResourceRefV1,
        /// Raw exact-byte generator source-bundle reference.
        generator: ProviderManifestResourceRefV1,
    },
    /// Executable component bound to its own exact raw bytes.
    Component {
        /// Resource-identical component reference.
        component: ProviderManifestResourceRefV1,
    },
}

/// One deterministic artifact route in the Edict provider manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderManifestArtifactV1 {
    /// Unique package-local role.
    pub role: String,
    /// Generic Edict artifact class.
    pub artifact_kind: ProviderManifestArtifactKindV1,
    /// Digest-locked semantic or exact-byte artifact identity.
    pub resource: ProviderManifestResourceRefV1,
    /// Generated or component provenance route.
    pub source: ProviderManifestArtifactSourceV1,
}

/// Schema formats accepted by the v1 Edict host.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderManifestSchemaFormatV1 {
    /// Self-contained generated CDDL.
    SelfContainedCddlV1,
}

impl ProviderManifestSchemaFormatV1 {
    const fn as_str(self) -> &'static str {
        match self {
            Self::SelfContainedCddlV1 => "selfContainedCddlV1",
        }
    }
}

/// One immutable domain-to-schema binding in the Edict provider manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderManifestSchemaBindingV1 {
    /// Canonical artifact domain.
    pub domain: String,
    /// Manifest role containing the self-contained schema.
    pub schema_role: String,
    /// Frozen schema format.
    pub format: ProviderManifestSchemaFormatV1,
    /// Exact CDDL root rule.
    pub root_rule: String,
}

/// Deterministically serialized mirror of Edict's provider-manifest v1 value.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderManifestV1 {
    /// Exact Edict manifest API version.
    pub api_version: String,
    /// Exact frozen target-provider component ABI.
    pub provider_abi: String,
    /// Echo-owned non-self-referential provider package identity.
    pub provider: ProviderManifestResourceRefV1,
    /// Ten exact generated/component artifact routes.
    pub artifacts: Vec<ProviderManifestArtifactV1>,
    /// Nine exact immutable schema-domain bindings.
    pub schema_bindings: Vec<ProviderManifestSchemaBindingV1>,
}

/// Explicit exact bytes for one source-declared provider component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderPackageComponentMaterialV1 {
    role: String,
    bytes: Vec<u8>,
}

impl ProviderPackageComponentMaterialV1 {
    /// Constructs bounded component material without path or registry discovery.
    ///
    /// # Errors
    ///
    /// Returns a stable error for an empty/oversized byte string or empty role.
    pub fn new(role: impl Into<String>, bytes: &[u8]) -> Result<Self, ProviderPackageError> {
        let role = role.into();
        if role.is_empty() || bytes.is_empty() || bytes.len() > MAX_COMPONENT_BYTES {
            return Err(ProviderPackageError::new(
                ProviderPackageErrorKind::ComponentInvalid,
                role,
                bytes.len().to_string(),
            ));
        }
        Ok(Self {
            role,
            bytes: bytes.to_vec(),
        })
    }

    /// Returns the source-declared manifest role.
    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Returns the exact component bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// One exact logical file in a provider package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderPackageFileV1 {
    relative_path: String,
    bytes: Vec<u8>,
}

impl ProviderPackageFileV1 {
    /// Constructs a package file with a safe relative POSIX path.
    ///
    /// # Errors
    ///
    /// Returns a stable error for paths outside the bounded corpus grammar.
    pub fn new(
        relative_path: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, ProviderPackageError> {
        let relative_path = relative_path.into();
        validate_corpus_path(&relative_path).map_err(|error| {
            ProviderPackageError::new(
                ProviderPackageErrorKind::PackagePathInvalid,
                relative_path.clone(),
                error.reference(),
            )
        })?;
        Ok(Self {
            relative_path,
            bytes: bytes.to_vec(),
        })
    }

    /// Returns the slash-normalized logical package path.
    #[must_use]
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    /// Returns the exact file bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// One non-manifest physical member bound into the provider closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderPackageMemberV1 {
    file: ProviderPackageFileV1,
    raw_digest: String,
}

impl ProviderPackageMemberV1 {
    /// Returns the member's logical package path.
    #[must_use]
    pub fn relative_path(&self) -> &str {
        self.file.relative_path()
    }

    /// Returns the member's exact bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.file.bytes()
    }

    /// Returns raw SHA-256 of the exact package occurrence.
    #[must_use]
    pub fn raw_digest(&self) -> &str {
        &self.raw_digest
    }

    fn new(file: ProviderPackageFileV1) -> Self {
        let raw_digest = raw_sha256(file.bytes());
        Self { file, raw_digest }
    }
}

/// Complete deterministic package materials assembled from exact inputs.
///
/// This value proves package construction only. It is not Edict schema or
/// component compatibility proof and is not Echo runtime authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderPackageV1 {
    manifest: ProviderManifestV1,
    manifest_bytes: Vec<u8>,
    manifest_content_reference: GenerationArtifactReferenceV1,
    closure_bytes: Vec<u8>,
    members: Vec<ProviderPackageMemberV1>,
    files: Vec<ProviderPackageFileV1>,
}

impl ProviderPackageV1 {
    /// Returns the typed provider manifest.
    #[must_use]
    pub const fn manifest(&self) -> &ProviderManifestV1 {
        &self.manifest
    }

    /// Returns exact deterministic pretty-JSON manifest bytes.
    #[must_use]
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    /// Returns the externally pinnable package-closure identity.
    #[must_use]
    pub const fn provider_reference(&self) -> &ProviderManifestResourceRefV1 {
        &self.manifest.provider
    }

    /// Returns raw exact-byte identity of the derived manifest occurrence.
    #[must_use]
    pub const fn manifest_content_reference(&self) -> &GenerationArtifactReferenceV1 {
        &self.manifest_content_reference
    }

    /// Returns the exact canonical-CBOR package-closure preimage value bytes.
    #[must_use]
    pub fn closure_bytes(&self) -> &[u8] {
        &self.closure_bytes
    }

    /// Returns all 24 non-manifest members in exact path order.
    #[must_use]
    pub fn members(&self) -> &[ProviderPackageMemberV1] {
        &self.members
    }

    /// Returns all 25 package files in exact path order.
    #[must_use]
    pub fn files(&self) -> &[ProviderPackageFileV1] {
        &self.files
    }
}

/// Opaque proof that explicit package bytes match their caller-pinned digest root.
///
/// This proof authenticates inventory, manifest routing, packaged Wesley
/// evidence, and exact bytes. Edict schema construction and component preflight
/// remain separate required crossings before guest execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigestAdmittedProviderPackageV1 {
    manifest: ProviderManifestV1,
    provider_reference: ProviderManifestResourceRefV1,
    files: Vec<ProviderPackageFileV1>,
}

impl DigestAdmittedProviderPackageV1 {
    /// Returns the exact manifest admitted by the package boundary.
    #[must_use]
    pub const fn manifest(&self) -> &ProviderManifestV1 {
        &self.manifest
    }

    /// Returns the caller-pinned, recomputed provider package identity.
    #[must_use]
    pub const fn provider_reference(&self) -> &ProviderManifestResourceRefV1 {
        &self.provider_reference
    }

    /// Returns the admitted exact file closure retained in memory.
    #[must_use]
    pub fn files(&self) -> &[ProviderPackageFileV1] {
        &self.files
    }
}

/// Assembles the deterministic package from verified generated values and exact components.
///
/// This function is pure. It performs no filesystem, registry, environment,
/// process, clock, network, component execution, or Echo runtime admission.
///
/// # Errors
///
/// Returns a structured error when provenance, review, generated artifacts,
/// source-owned component declarations, member inventory, serialization, or
/// package identity disagree.
pub fn assemble_provider_package_v1(
    input: &ProviderGenerationInputV1,
    primary: &ProviderPrimaryArtifactsV1,
    generator: &ProviderGeneratorMaterialV1,
    provenance: &ProviderGenerationProvenanceV1,
    review: &ProviderGenerationReviewV1,
    mut components: Vec<ProviderPackageComponentMaterialV1>,
) -> Result<ProviderPackageV1, ProviderPackageError> {
    let corpus = render_provider_artifact_corpus_v1(input, primary, generator, provenance, review)
        .map_err(|error| {
            ProviderPackageError::new(
                ProviderPackageErrorKind::GeneratedCorpusInvalid,
                error.subject(),
                error.reference(),
            )
        })?;
    let source = input.semantic_source().source();
    let package_declaration = &source.package_manifest;
    if package_declaration.role != PROVIDER_MANIFEST_ROLE_V1
        || package_declaration.coordinate != PROVIDER_MANIFEST_COORDINATE_V1
        || package_declaration.schema_contract != PROVIDER_MANIFEST_API_V1
        || package_declaration.provider_abi != PROVIDER_ABI_V1
        || package_declaration.provider_coordinate != PROVIDER_COORDINATE_V1
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ManifestContractMismatch,
            &package_declaration.role,
            &package_declaration.coordinate,
        ));
    }

    let semantic_source = input
        .source_artifacts()
        .iter()
        .find(|artifact| artifact.coordinate == source.coordinate)
        .ok_or_else(|| {
            ProviderPackageError::new(
                ProviderPackageErrorKind::ArtifactMissing,
                "semanticSource",
                &source.coordinate,
            )
        })?
        .reference();
    let generated_source = ProviderManifestArtifactSourceV1::Generated {
        semantic_source: manifest_ref(&semantic_source.coordinate, &semantic_source.digest),
        generator: manifest_ref(
            &generator.identity().coordinate,
            &generator.identity().digest,
        ),
    };

    let mut artifacts = Vec::with_capacity(EXPECTED_ARTIFACT_COUNT);
    for declaration in &source.generated_artifacts {
        artifacts.push(generated_manifest_artifact(
            declaration,
            primary,
            provenance,
            review,
            &generated_source,
        )?);
    }

    components.sort_by(|left, right| left.role.as_bytes().cmp(right.role.as_bytes()));
    if let Some(pair) = components
        .windows(2)
        .find(|pair| pair[0].role == pair[1].role)
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ComponentDuplicate,
            &pair[0].role,
            &pair[1].role,
        ));
    }
    let mut component_members = Vec::with_capacity(package_declaration.components.len());
    for declaration in &package_declaration.components {
        let material = components
            .iter()
            .find(|material| material.role == declaration.role)
            .ok_or_else(|| {
                ProviderPackageError::new(
                    ProviderPackageErrorKind::ComponentMissing,
                    &declaration.role,
                    &declaration.coordinate,
                )
            })?;
        let resource = manifest_ref(&declaration.coordinate, &raw_sha256(&material.bytes));
        artifacts.push(component_manifest_artifact(declaration, resource.clone()));
        component_members.push(ProviderPackageMemberV1::new(ProviderPackageFileV1::new(
            format!("components/{}.component.wasm", declaration.role),
            &material.bytes,
        )?));
    }
    if let Some(material) = components.iter().find(|material| {
        !package_declaration
            .components
            .iter()
            .any(|declaration| declaration.role == material.role)
    }) {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ComponentUnexpected,
            &material.role,
            "packageManifest.components",
        ));
    }
    artifacts.sort_by(|left, right| left.role.as_bytes().cmp(right.role.as_bytes()));

    let schema_bindings = source
        .schema_bindings
        .iter()
        .map(|binding| ProviderManifestSchemaBindingV1 {
            domain: binding.domain.clone(),
            schema_role: binding.schema_role.clone(),
            format: match binding.format {
                ArtifactSchemaFormat::SelfContainedCddlV1 => {
                    ProviderManifestSchemaFormatV1::SelfContainedCddlV1
                }
            },
            root_rule: binding.root_rule.clone(),
        })
        .collect::<Vec<_>>();

    let mut members = corpus
        .files()
        .iter()
        .map(|file| {
            ProviderPackageFileV1::new(format!("generated/{}", file.relative_path()), file.bytes())
                .map(ProviderPackageMemberV1::new)
        })
        .collect::<Result<Vec<_>, _>>()?;
    members.extend(component_members);
    members.sort_by(|left, right| {
        left.relative_path()
            .as_bytes()
            .cmp(right.relative_path().as_bytes())
    });
    validate_member_count_and_order(&members)?;

    let provider_digest = package_closure_digest(&artifacts, &schema_bindings, &members)?;
    let manifest = ProviderManifestV1 {
        api_version: PROVIDER_MANIFEST_API_V1.to_owned(),
        provider_abi: PROVIDER_ABI_V1.to_owned(),
        provider: manifest_ref(PROVIDER_COORDINATE_V1, &provider_digest),
        artifacts,
        schema_bindings,
    };
    let manifest_bytes = render_manifest(&manifest)?;
    let manifest_content_reference =
        GenerationArtifactReferenceV1::for_bytes(PROVIDER_MANIFEST_COORDINATE_V1, &manifest_bytes)
            .map_err(|_error| {
                ProviderPackageError::new(
                    ProviderPackageErrorKind::ManifestSerializationFailed,
                    PROVIDER_MANIFEST_COORDINATE_V1,
                    "wesley-content-reference",
                )
            })?;
    let closure_value =
        package_closure_value(&manifest.artifacts, &manifest.schema_bindings, &members)?;
    let closure_bytes = encode_canonical_cbor_v1(&closure_value).map_err(|_error| {
        ProviderPackageError::new(
            ProviderPackageErrorKind::PackageIdentityFailed,
            ECHO_PROVIDER_PACKAGE_CLOSURE_DOMAIN_V1,
            "canonical-cbor",
        )
    })?;
    let mut files = members
        .iter()
        .map(|member| member.file.clone())
        .collect::<Vec<_>>();
    files.push(ProviderPackageFileV1::new(
        ECHO_PROVIDER_MANIFEST_PATH_V1,
        &manifest_bytes,
    )?);
    files.sort_by(|left, right| {
        left.relative_path
            .as_bytes()
            .cmp(right.relative_path.as_bytes())
    });
    validate_file_inventory(&files)?;
    validate_total_package_size(&files)?;

    Ok(ProviderPackageV1 {
        manifest,
        manifest_bytes,
        manifest_content_reference,
        closure_bytes,
        members,
        files,
    })
}

/// Digest-admits an explicit in-memory package against a caller-supplied identity.
///
/// The boundary requires the exact v1 file inventory and rendering, reproduces
/// every routed artifact identity, binds every non-manifest member into the
/// package root, and returns no partial package on failure. It performs no I/O,
/// does not replace Edict schema/component preflight, and grants no Echo runtime
/// authority.
///
/// # Errors
///
/// Returns a structured error for malformed or alternate manifest bytes,
/// missing/duplicate/unexpected/out-of-order files, wrong artifact bytes or
/// provenance, schema-closure disagreement, or provider-root disagreement.
pub fn admit_provider_package_v1(
    files: Vec<ProviderPackageFileV1>,
    expected_provider: &ProviderManifestResourceRefV1,
) -> Result<DigestAdmittedProviderPackageV1, ProviderPackageError> {
    validate_file_inventory(&files)?;
    validate_total_package_size(&files)?;
    let manifest_file = file_by_path(&files, ECHO_PROVIDER_MANIFEST_PATH_V1)?;
    validate_member_size(manifest_file, MAX_MANIFEST_BYTES)?;
    let manifest: ProviderManifestV1 =
        serde_json::from_slice(manifest_file.bytes()).map_err(|_error| {
            ProviderPackageError::new(
                ProviderPackageErrorKind::ManifestMalformed,
                ECHO_PROVIDER_MANIFEST_PATH_V1,
                ECHO_PROVIDER_MANIFEST_ENCODING_V1,
            )
        })?;
    if render_manifest(&manifest)? != manifest_file.bytes() {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ManifestNoncanonical,
            ECHO_PROVIDER_MANIFEST_PATH_V1,
            raw_sha256(manifest_file.bytes()),
        ));
    }
    validate_expected_provider(&manifest.provider, expected_provider)?;
    validate_manifest_contract(&manifest, &files)?;

    let members = files
        .iter()
        .filter(|file| file.relative_path != ECHO_PROVIDER_MANIFEST_PATH_V1)
        .cloned()
        .map(ProviderPackageMemberV1::new)
        .collect::<Vec<_>>();
    validate_member_count_and_order(&members)?;
    let computed = manifest_ref(
        PROVIDER_COORDINATE_V1,
        &package_closure_digest(&manifest.artifacts, &manifest.schema_bindings, &members)?,
    );
    if manifest.provider != computed || expected_provider != &computed {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ProviderIdentityMismatch,
            PROVIDER_COORDINATE_V1,
            computed.digest,
        ));
    }

    Ok(DigestAdmittedProviderPackageV1 {
        manifest,
        provider_reference: computed,
        files,
    })
}

fn generated_manifest_artifact(
    declaration: &GeneratedArtifactDeclaration,
    primary: &ProviderPrimaryArtifactsV1,
    provenance: &ProviderGenerationProvenanceV1,
    review: &ProviderGenerationReviewV1,
    source: &ProviderManifestArtifactSourceV1,
) -> Result<ProviderManifestArtifactV1, ProviderPackageError> {
    let (artifact_kind, resource) = match declaration.kind {
        GeneratedArtifactKind::Lawpack => (
            ProviderManifestArtifactKindV1::Lawpack,
            canonical_artifact_ref(declaration, primary)?,
        ),
        GeneratedArtifactKind::TargetProfile => (
            ProviderManifestArtifactKindV1::TargetProfile,
            canonical_artifact_ref(declaration, primary)?,
        ),
        GeneratedArtifactKind::AuthorityFacts => (
            ProviderManifestArtifactKindV1::AuthorityFacts,
            canonical_artifact_ref(declaration, primary)?,
        ),
        GeneratedArtifactKind::ProviderManifest => {
            return Err(ProviderPackageError::new(
                ProviderPackageErrorKind::ManifestContractMismatch,
                &declaration.role,
                &declaration.coordinate,
            ));
        }
        GeneratedArtifactKind::ReviewArtifact => {
            require_declared_json(
                declaration,
                review.role(),
                review.coordinate(),
                review.schema_contract(),
            )?;
            (
                ProviderManifestArtifactKindV1::ReviewArtifact,
                manifest_ref(review.coordinate(), &review.content_reference().digest),
            )
        }
        GeneratedArtifactKind::GeneratedArtifactProfile => (
            ProviderManifestArtifactKindV1::GeneratedArtifactProfile,
            canonical_artifact_ref(declaration, primary)?,
        ),
        GeneratedArtifactKind::GenerationProvenance => {
            require_declared_json(
                declaration,
                provenance.role(),
                provenance.coordinate(),
                provenance.schema_contract(),
            )?;
            (
                ProviderManifestArtifactKindV1::GenerationProvenance,
                manifest_ref(
                    provenance.coordinate(),
                    &provenance.content_reference().digest,
                ),
            )
        }
        GeneratedArtifactKind::ArtifactSchema => {
            require_declared_schema(declaration, primary.schema())?;
            (
                ProviderManifestArtifactKindV1::ArtifactSchema,
                manifest_ref(
                    primary.schema().coordinate(),
                    &primary.schema().content_reference().digest,
                ),
            )
        }
    };
    Ok(ProviderManifestArtifactV1 {
        role: declaration.role.clone(),
        artifact_kind,
        resource,
        source: source.clone(),
    })
}

fn canonical_artifact_ref(
    declaration: &GeneratedArtifactDeclaration,
    primary: &ProviderPrimaryArtifactsV1,
) -> Result<ProviderManifestResourceRefV1, ProviderPackageError> {
    let artifact = primary.artifact(&declaration.role).ok_or_else(|| {
        ProviderPackageError::new(
            ProviderPackageErrorKind::ArtifactMissing,
            &declaration.role,
            &declaration.coordinate,
        )
    })?;
    require_declared_canonical(declaration, artifact)?;
    Ok(manifest_ref(
        artifact.coordinate(),
        artifact.domain_framed_digest(),
    ))
}

fn require_declared_canonical(
    declaration: &GeneratedArtifactDeclaration,
    artifact: &SchemaValidatedCanonicalProviderOutputV1,
) -> Result<(), ProviderPackageError> {
    if declaration.role != artifact.role()
        || declaration.coordinate != artifact.coordinate()
        || declaration.schema_contract != artifact.schema_contract()
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ArtifactMismatch,
            &declaration.role,
            artifact.coordinate(),
        ));
    }
    Ok(())
}

fn require_declared_schema(
    declaration: &GeneratedArtifactDeclaration,
    schema: &GeneratedProviderSchemaV1,
) -> Result<(), ProviderPackageError> {
    if declaration.role != schema.role() || declaration.coordinate != schema.coordinate() {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ArtifactMismatch,
            &declaration.role,
            schema.coordinate(),
        ));
    }
    Ok(())
}

fn require_declared_json(
    declaration: &GeneratedArtifactDeclaration,
    role: &str,
    coordinate: &str,
    schema_contract: &str,
) -> Result<(), ProviderPackageError> {
    if declaration.role != role
        || declaration.coordinate != coordinate
        || declaration.schema_contract != schema_contract
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ArtifactMismatch,
            &declaration.role,
            coordinate,
        ));
    }
    Ok(())
}

fn component_manifest_artifact(
    declaration: &ProviderComponentDeclaration,
    resource: ProviderManifestResourceRefV1,
) -> ProviderManifestArtifactV1 {
    ProviderManifestArtifactV1 {
        role: declaration.role.clone(),
        artifact_kind: match declaration.kind {
            ProviderComponentKind::Lowerer => ProviderManifestArtifactKindV1::Lowerer,
            ProviderComponentKind::Verifier => ProviderManifestArtifactKindV1::Verifier,
        },
        resource: resource.clone(),
        source: ProviderManifestArtifactSourceV1::Component {
            component: resource,
        },
    }
}

fn validate_manifest_contract(
    manifest: &ProviderManifestV1,
    files: &[ProviderPackageFileV1],
) -> Result<(), ProviderPackageError> {
    if manifest.api_version != PROVIDER_MANIFEST_API_V1
        || manifest.provider_abi != PROVIDER_ABI_V1
        || manifest.provider.coordinate != PROVIDER_COORDINATE_V1
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ManifestContractMismatch,
            &manifest.api_version,
            &manifest.provider_abi,
        ));
    }
    strict_digest_bytes(&manifest.provider.digest)?;
    let specs = artifact_specs();
    if manifest.artifacts.len() != specs.len() {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ManifestContractMismatch,
            "artifacts",
            manifest.artifacts.len().to_string(),
        ));
    }
    for (artifact, spec) in manifest.artifacts.iter().zip(specs) {
        if artifact.role != spec.role
            || artifact.artifact_kind != spec.artifact_kind
            || artifact.resource.coordinate != spec.coordinate
        {
            return Err(ProviderPackageError::new(
                ProviderPackageErrorKind::ManifestContractMismatch,
                &artifact.role,
                &artifact.resource.coordinate,
            ));
        }
        strict_digest_bytes(&artifact.resource.digest)?;
        validate_artifact_source(artifact, spec.component)?;
        let file = file_by_path(files, spec.path)?;
        if spec.component && (file.bytes().is_empty() || file.bytes().len() > MAX_COMPONENT_BYTES) {
            return Err(ProviderPackageError::new(
                ProviderPackageErrorKind::ComponentInvalid,
                spec.role,
                file.bytes().len().to_string(),
            ));
        }
        let actual_digest = match spec.digest_domain {
            Some(domain) => {
                let value = decode_canonical_cbor_v1(file.bytes()).map_err(|_error| {
                    ProviderPackageError::new(
                        ProviderPackageErrorKind::ArtifactDigestMismatch,
                        spec.role,
                        "canonical-cbor",
                    )
                })?;
                digest_canonical_value_v1(domain, &value).map_err(|_error| {
                    ProviderPackageError::new(
                        ProviderPackageErrorKind::ArtifactDigestMismatch,
                        spec.role,
                        domain,
                    )
                })?
            }
            None => raw_sha256(file.bytes()),
        };
        if artifact.resource.digest != actual_digest {
            return Err(ProviderPackageError::new(
                ProviderPackageErrorKind::ArtifactDigestMismatch,
                spec.role,
                actual_digest,
            ));
        }
    }
    validate_generated_source_closure(&manifest.artifacts)?;
    validate_generation_evidence(manifest, files)?;
    validate_schema_bindings(&manifest.schema_bindings)
}

fn validate_expected_provider(
    manifest_provider: &ProviderManifestResourceRefV1,
    expected_provider: &ProviderManifestResourceRefV1,
) -> Result<(), ProviderPackageError> {
    strict_digest_bytes(&manifest_provider.digest)?;
    strict_digest_bytes(&expected_provider.digest)?;
    if expected_provider.coordinate != PROVIDER_COORDINATE_V1
        || manifest_provider != expected_provider
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ProviderIdentityMismatch,
            &expected_provider.coordinate,
            &expected_provider.digest,
        ));
    }
    Ok(())
}

fn validate_generation_evidence(
    manifest: &ProviderManifestV1,
    files: &[ProviderPackageFileV1],
) -> Result<(), ProviderPackageError> {
    let provenance_file = file_by_path(
        files,
        "generated/evidence/provenance.provider-generation.json",
    )?;
    validate_member_size(provenance_file, MAX_EVIDENCE_BYTES)?;
    let provenance: GenerationProvenanceManifestV1 =
        serde_json::from_slice(provenance_file.bytes()).map_err(|_error| {
            ProviderPackageError::new(
                ProviderPackageErrorKind::GenerationProvenanceMismatch,
                "provenance.provider-generation",
                "wesley-generation-provenance-json/v1",
            )
        })?;
    let canonical_provenance = provenance.canonical_bytes().map_err(|error| {
        ProviderPackageError::new(
            ProviderPackageErrorKind::GenerationProvenanceMismatch,
            "provenance.provider-generation",
            "wesley-generation-provenance-contract/v1",
        )
        .wesley(&error)
    })?;
    if canonical_provenance != provenance_file.bytes() {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::GenerationProvenanceMismatch,
            "provenance.provider-generation",
            "noncanonical-json",
        ));
    }

    let (manifest_source, manifest_generator) = manifest
        .artifacts
        .iter()
        .find_map(|artifact| match &artifact.source {
            ProviderManifestArtifactSourceV1::Generated {
                semantic_source,
                generator,
            } => Some((semantic_source, generator)),
            ProviderManifestArtifactSourceV1::Component { .. } => None,
        })
        .ok_or_else(|| {
            ProviderPackageError::new(
                ProviderPackageErrorKind::GenerationProvenanceMismatch,
                "artifacts",
                "generated-source-missing",
            )
        })?;
    if provenance.generator.coordinate != manifest_generator.coordinate
        || provenance.generator.digest != manifest_generator.digest
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::GenerationProvenanceMismatch,
            "generator",
            &provenance.generator.digest,
        ));
    }
    let provenance_source = provenance
        .source_artifacts
        .iter()
        .find(|reference| reference.coordinate == SEMANTIC_SOURCE_COORDINATE_V1)
        .ok_or_else(|| {
            ProviderPackageError::new(
                ProviderPackageErrorKind::GenerationProvenanceMismatch,
                "sourceArtifacts",
                SEMANTIC_SOURCE_COORDINATE_V1,
            )
        })?;
    if provenance_source.coordinate != manifest_source.coordinate
        || provenance_source.digest != manifest_source.digest
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::GenerationProvenanceMismatch,
            SEMANTIC_SOURCE_COORDINATE_V1,
            &provenance_source.digest,
        ));
    }
    let expected_emitted = expected_emitted_references(files)?;
    if provenance.emitted_artifacts != expected_emitted {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::GenerationProvenanceMismatch,
            "emittedArtifacts",
            provenance.emitted_artifacts.len().to_string(),
        ));
    }

    let review_file = file_by_path(files, "generated/evidence/review.provider-generation.json")?;
    validate_member_size(review_file, MAX_EVIDENCE_BYTES)?;
    let review: GenerationReviewV1 =
        serde_json::from_slice(review_file.bytes()).map_err(|_error| {
            ProviderPackageError::new(
                ProviderPackageErrorKind::GenerationReviewMismatch,
                "review.provider-generation",
                "wesley-generation-review-json/v1",
            )
        })?;
    let canonical_review = review.canonical_bytes().map_err(|error| {
        ProviderPackageError::new(
            ProviderPackageErrorKind::GenerationReviewMismatch,
            "review.provider-generation",
            "wesley-generation-review-contract/v1",
        )
        .wesley(&error)
    })?;
    if canonical_review != review_file.bytes()
        || review.authoritative()
        || review.generation_input_digest != provenance.generation_input_digest
        || review.generator != provenance.generator
        || review.source_artifacts != provenance.source_artifacts
        || review.emitted_artifacts != provenance.emitted_artifacts
        || review.projection_roles != expected_projection_roles()
        || review.provenance_manifest_digest
            != provenance.digest().map_err(|error| {
                ProviderPackageError::new(
                    ProviderPackageErrorKind::GenerationProvenanceMismatch,
                    "provenance.provider-generation",
                    "wesley-provenance-digest/v1",
                )
                .wesley(&error)
            })?
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::GenerationReviewMismatch,
            "review.provider-generation",
            "provenance-projection",
        ));
    }
    Ok(())
}

fn expected_emitted_references(
    files: &[ProviderPackageFileV1],
) -> Result<Vec<GenerationArtifactReferenceV1>, ProviderPackageError> {
    let mut references = artifact_specs()
        .into_iter()
        .filter(|spec| {
            !spec.component
                && !matches!(
                    spec.artifact_kind,
                    ProviderManifestArtifactKindV1::GenerationProvenance
                        | ProviderManifestArtifactKindV1::ReviewArtifact
                )
        })
        .map(|spec| {
            let file = file_by_path(files, spec.path)?;
            Ok(GenerationArtifactReferenceV1 {
                coordinate: spec.coordinate.to_owned(),
                digest: raw_sha256(file.bytes()),
            })
        })
        .collect::<Result<Vec<_>, ProviderPackageError>>()?;
    references.sort_by(|left, right| left.coordinate.cmp(&right.coordinate));
    Ok(references)
}

fn expected_projection_roles() -> Vec<String> {
    [
        "authority-facts.echo-dpo",
        "authority-facts.echo-lawpack",
        "generated-artifact-profile.echo-dpo-registration",
        "lawpack.echo-dpo",
        "schema.echo-provider-artifacts",
        "target-profile.echo-dpo",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn validate_generated_source_closure(
    artifacts: &[ProviderManifestArtifactV1],
) -> Result<(), ProviderPackageError> {
    let expected = artifacts
        .iter()
        .find_map(|artifact| match &artifact.source {
            ProviderManifestArtifactSourceV1::Generated {
                semantic_source,
                generator,
            } => Some((semantic_source, generator)),
            ProviderManifestArtifactSourceV1::Component { .. } => None,
        });
    let Some((expected_source, expected_generator)) = expected else {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ArtifactSourceMismatch,
            "artifacts",
            "generated-source-missing",
        ));
    };
    for artifact in artifacts {
        if let ProviderManifestArtifactSourceV1::Generated {
            semantic_source,
            generator,
        } = &artifact.source
        {
            if semantic_source != expected_source || generator != expected_generator {
                return Err(ProviderPackageError::new(
                    ProviderPackageErrorKind::ArtifactSourceMismatch,
                    &artifact.role,
                    &semantic_source.digest,
                ));
            }
        }
    }
    Ok(())
}

fn validate_artifact_source(
    artifact: &ProviderManifestArtifactV1,
    component: bool,
) -> Result<(), ProviderPackageError> {
    match (&artifact.source, component) {
        (ProviderManifestArtifactSourceV1::Component { component }, true)
            if component == &artifact.resource =>
        {
            Ok(())
        }
        (
            ProviderManifestArtifactSourceV1::Generated {
                semantic_source,
                generator,
            },
            false,
        ) if semantic_source.coordinate == SEMANTIC_SOURCE_COORDINATE_V1
            && generator.coordinate == GENERATOR_COORDINATE_V1 =>
        {
            strict_digest_bytes(&semantic_source.digest)?;
            strict_digest_bytes(&generator.digest)?;
            Ok(())
        }
        _ => Err(ProviderPackageError::new(
            ProviderPackageErrorKind::ArtifactSourceMismatch,
            &artifact.role,
            artifact.resource.coordinate.clone(),
        )),
    }
}

fn validate_schema_bindings(
    bindings: &[ProviderManifestSchemaBindingV1],
) -> Result<(), ProviderPackageError> {
    let expected = schema_binding_specs();
    if bindings.len() != expected.len() {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::SchemaBindingMismatch,
            "schemaBindings",
            bindings.len().to_string(),
        ));
    }
    for (binding, (domain, root_rule)) in bindings.iter().zip(expected) {
        if binding.domain != domain
            || binding.schema_role != "schema.echo-provider-artifacts"
            || binding.format != ProviderManifestSchemaFormatV1::SelfContainedCddlV1
            || binding.root_rule != root_rule
        {
            return Err(ProviderPackageError::new(
                ProviderPackageErrorKind::SchemaBindingMismatch,
                &binding.domain,
                &binding.root_rule,
            ));
        }
    }
    Ok(())
}

fn package_closure_digest(
    artifacts: &[ProviderManifestArtifactV1],
    schema_bindings: &[ProviderManifestSchemaBindingV1],
    members: &[ProviderPackageMemberV1],
) -> Result<String, ProviderPackageError> {
    let value = package_closure_value(artifacts, schema_bindings, members)?;
    digest_canonical_value_v1(ECHO_PROVIDER_PACKAGE_CLOSURE_DOMAIN_V1, &value).map_err(|_error| {
        ProviderPackageError::new(
            ProviderPackageErrorKind::PackageIdentityFailed,
            ECHO_PROVIDER_PACKAGE_CLOSURE_DOMAIN_V1,
            "canonical-cbor",
        )
    })
}

fn package_closure_value(
    artifacts: &[ProviderManifestArtifactV1],
    schema_bindings: &[ProviderManifestSchemaBindingV1],
    members: &[ProviderPackageMemberV1],
) -> Result<CanonicalValueV1, ProviderPackageError> {
    let artifact_values = artifacts
        .iter()
        .map(|artifact| {
            Ok(CanonicalValueV1::Array(vec![
                text(&artifact.role),
                text(artifact.artifact_kind.as_str()),
                text(&artifact.resource.coordinate),
                CanonicalValueV1::Bytes(strict_digest_bytes(&artifact.resource.digest)?),
                source_value(&artifact.source)?,
            ]))
        })
        .collect::<Result<Vec<_>, ProviderPackageError>>()?;
    let binding_values = schema_bindings
        .iter()
        .map(|binding| {
            CanonicalValueV1::Array(vec![
                text(&binding.domain),
                text(&binding.schema_role),
                text(binding.format.as_str()),
                text(&binding.root_rule),
            ])
        })
        .collect();
    let member_values = members
        .iter()
        .map(|member| {
            Ok(CanonicalValueV1::Array(vec![
                text(member.relative_path()),
                CanonicalValueV1::Bytes(strict_digest_bytes(&member.raw_digest)?),
            ]))
        })
        .collect::<Result<Vec<_>, ProviderPackageError>>()?;
    Ok(CanonicalValueV1::Array(vec![
        text(PROVIDER_MANIFEST_ROLE_V1),
        text(PROVIDER_MANIFEST_COORDINATE_V1),
        text(ECHO_PROVIDER_MANIFEST_ENCODING_V1),
        text(PROVIDER_MANIFEST_API_V1),
        text(PROVIDER_ABI_V1),
        text(PROVIDER_COORDINATE_V1),
        CanonicalValueV1::Array(artifact_values),
        CanonicalValueV1::Array(binding_values),
        CanonicalValueV1::Array(member_values),
    ]))
}

fn source_value(
    source: &ProviderManifestArtifactSourceV1,
) -> Result<CanonicalValueV1, ProviderPackageError> {
    match source {
        ProviderManifestArtifactSourceV1::Generated {
            semantic_source,
            generator,
        } => Ok(CanonicalValueV1::Array(vec![
            text("generated"),
            text(&semantic_source.coordinate),
            CanonicalValueV1::Bytes(strict_digest_bytes(&semantic_source.digest)?),
            text(&generator.coordinate),
            CanonicalValueV1::Bytes(strict_digest_bytes(&generator.digest)?),
        ])),
        ProviderManifestArtifactSourceV1::Component { component } => {
            Ok(CanonicalValueV1::Array(vec![
                text("component"),
                text(&component.coordinate),
                CanonicalValueV1::Bytes(strict_digest_bytes(&component.digest)?),
            ]))
        }
    }
}

fn validate_member_count_and_order(
    members: &[ProviderPackageMemberV1],
) -> Result<(), ProviderPackageError> {
    if members.len() != EXPECTED_MEMBER_COUNT {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::PackageMemberMissing,
            "package-members",
            members.len().to_string(),
        ));
    }
    for pair in members.windows(2) {
        match pair[0]
            .relative_path()
            .as_bytes()
            .cmp(pair[1].relative_path().as_bytes())
        {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => {
                return Err(ProviderPackageError::new(
                    ProviderPackageErrorKind::PackageMemberDuplicate,
                    pair[0].relative_path(),
                    pair[1].relative_path(),
                ));
            }
            std::cmp::Ordering::Greater => {
                return Err(ProviderPackageError::new(
                    ProviderPackageErrorKind::PackageMemberOutOfOrder,
                    pair[0].relative_path(),
                    pair[1].relative_path(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_file_inventory(files: &[ProviderPackageFileV1]) -> Result<(), ProviderPackageError> {
    for pair in files.windows(2) {
        match pair[0]
            .relative_path
            .as_bytes()
            .cmp(pair[1].relative_path.as_bytes())
        {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => {
                return Err(ProviderPackageError::new(
                    ProviderPackageErrorKind::PackageMemberDuplicate,
                    &pair[0].relative_path,
                    &pair[1].relative_path,
                ));
            }
            std::cmp::Ordering::Greater => {
                return Err(ProviderPackageError::new(
                    ProviderPackageErrorKind::PackageMemberOutOfOrder,
                    &pair[0].relative_path,
                    &pair[1].relative_path,
                ));
            }
        }
    }
    if let Some(file) = files
        .iter()
        .find(|file| !EXPECTED_PACKAGE_PATHS.contains(&file.relative_path.as_str()))
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::PackageMemberUnexpected,
            &file.relative_path,
            "provider-package/v1",
        ));
    }
    if let Some(path) = EXPECTED_PACKAGE_PATHS
        .iter()
        .find(|path| !files.iter().any(|file| file.relative_path == **path))
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::PackageMemberMissing,
            *path,
            "provider-package/v1",
        ));
    }
    if files.len() != EXPECTED_FILE_COUNT {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::PackageMemberDuplicate,
            "provider-package/v1",
            files.len().to_string(),
        ));
    }
    Ok(())
}

fn validate_total_package_size(
    files: &[ProviderPackageFileV1],
) -> Result<(), ProviderPackageError> {
    let total_bytes = files.iter().try_fold(0_usize, |total, file| {
        total.checked_add(file.bytes.len()).ok_or_else(|| {
            ProviderPackageError::new(
                ProviderPackageErrorKind::PackageSizeExceeded,
                "provider-package/v1",
                "byte-length-overflow",
            )
        })
    })?;
    if total_bytes > MAX_PACKAGE_BYTES {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::PackageSizeExceeded,
            "provider-package/v1",
            total_bytes.to_string(),
        ));
    }
    Ok(())
}

fn validate_member_size(
    file: &ProviderPackageFileV1,
    maximum: usize,
) -> Result<(), ProviderPackageError> {
    if file.bytes().len() > maximum {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::PackageSizeExceeded,
            file.relative_path(),
            file.bytes().len().to_string(),
        ));
    }
    Ok(())
}

fn file_by_path<'a>(
    files: &'a [ProviderPackageFileV1],
    path: &str,
) -> Result<&'a ProviderPackageFileV1, ProviderPackageError> {
    files
        .iter()
        .find(|file| file.relative_path == path)
        .ok_or_else(|| {
            ProviderPackageError::new(
                ProviderPackageErrorKind::PackageMemberMissing,
                path,
                "provider-package/v1",
            )
        })
}

fn render_manifest(manifest: &ProviderManifestV1) -> Result<Vec<u8>, ProviderPackageError> {
    let mut bytes = serde_json::to_vec_pretty(manifest).map_err(|_error| {
        ProviderPackageError::new(
            ProviderPackageErrorKind::ManifestSerializationFailed,
            PROVIDER_MANIFEST_COORDINATE_V1,
            ECHO_PROVIDER_MANIFEST_ENCODING_V1,
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn manifest_ref(coordinate: &str, digest: &str) -> ProviderManifestResourceRefV1 {
    ProviderManifestResourceRefV1 {
        coordinate: coordinate.to_owned(),
        digest: digest.to_owned(),
    }
}

fn raw_sha256(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

fn strict_digest_bytes(digest: &str) -> Result<Vec<u8>, ProviderPackageError> {
    let Some(hex_digest) = digest.strip_prefix("sha256:") else {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::DigestInvalid,
            "sha256",
            digest,
        ));
    };
    if hex_digest.len() != 64
        || !hex_digest
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(ProviderPackageError::new(
            ProviderPackageErrorKind::DigestInvalid,
            "sha256",
            digest,
        ));
    }
    hex::decode(hex_digest).map_err(|_error| {
        ProviderPackageError::new(ProviderPackageErrorKind::DigestInvalid, "sha256", digest)
    })
}

fn text(value: &str) -> CanonicalValueV1 {
    CanonicalValueV1::Text(value.to_owned())
}

fn bounded_diagnostic(mut value: String) -> String {
    if value.len() <= MAX_DIAGNOSTIC_BYTES {
        return value;
    }
    let mut end = MAX_DIAGNOSTIC_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value
}

#[derive(Clone, Copy)]
struct ArtifactSpec {
    role: &'static str,
    artifact_kind: ProviderManifestArtifactKindV1,
    coordinate: &'static str,
    path: &'static str,
    digest_domain: Option<&'static str>,
    component: bool,
}

fn artifact_specs() -> [ArtifactSpec; EXPECTED_ARTIFACT_COUNT] {
    [
        ArtifactSpec {
            role: "authority-facts.echo-dpo",
            artifact_kind: ProviderManifestArtifactKindV1::AuthorityFacts,
            coordinate: "echo.dpo-authority-facts@1",
            path: "generated/primary/authority-facts.echo-dpo.cbor",
            digest_domain: Some("edict.authority-facts/v1"),
            component: false,
        },
        ArtifactSpec {
            role: "authority-facts.echo-lawpack",
            artifact_kind: ProviderManifestArtifactKindV1::AuthorityFacts,
            coordinate: "echo.dpo-lawpack-authority-facts@1",
            path: "generated/primary/authority-facts.echo-lawpack.cbor",
            digest_domain: Some("edict.authority-facts/v1"),
            component: false,
        },
        ArtifactSpec {
            role: "generated-artifact-profile.echo-dpo-registration",
            artifact_kind: ProviderManifestArtifactKindV1::GeneratedArtifactProfile,
            coordinate: "echo.dpo.registration/v1",
            path: "generated/primary/generated-artifact-profile.echo-dpo-registration.cbor",
            digest_domain: Some("echo.generated-artifact-profile/v1"),
            component: false,
        },
        ArtifactSpec {
            role: "lawpack.echo-dpo",
            artifact_kind: ProviderManifestArtifactKindV1::Lawpack,
            coordinate: "echo.dpo-lawpack@1",
            path: "generated/primary/lawpack.echo-dpo.cbor",
            digest_domain: Some("edict.lawpack/v1"),
            component: false,
        },
        ArtifactSpec {
            role: "lowerer.echo-dpo",
            artifact_kind: ProviderManifestArtifactKindV1::Lowerer,
            coordinate: "echo.dpo.lowerer/component@1",
            path: "components/lowerer.echo-dpo.component.wasm",
            digest_domain: None,
            component: true,
        },
        ArtifactSpec {
            role: "provenance.provider-generation",
            artifact_kind: ProviderManifestArtifactKindV1::GenerationProvenance,
            coordinate: "echo.edict-provider-generation-provenance@1",
            path: "generated/evidence/provenance.provider-generation.json",
            digest_domain: None,
            component: false,
        },
        ArtifactSpec {
            role: "review.provider-generation",
            artifact_kind: ProviderManifestArtifactKindV1::ReviewArtifact,
            coordinate: "echo.edict-provider-generation-review@1",
            path: "generated/evidence/review.provider-generation.json",
            digest_domain: None,
            component: false,
        },
        ArtifactSpec {
            role: "schema.echo-provider-artifacts",
            artifact_kind: ProviderManifestArtifactKindV1::ArtifactSchema,
            coordinate: "echo.provider-artifacts.cddl@1",
            path: "generated/primary/schema.echo-provider-artifacts.cddl",
            digest_domain: None,
            component: false,
        },
        ArtifactSpec {
            role: "target-profile.echo-dpo",
            artifact_kind: ProviderManifestArtifactKindV1::TargetProfile,
            coordinate: "echo.dpo@1",
            path: "generated/primary/target-profile.echo-dpo.cbor",
            digest_domain: Some("edict.target-profile/v1"),
            component: false,
        },
        ArtifactSpec {
            role: "verifier.echo-dpo",
            artifact_kind: ProviderManifestArtifactKindV1::Verifier,
            coordinate: "echo.dpo.verifier/component@1",
            path: "components/verifier.echo-dpo.component.wasm",
            digest_domain: None,
            component: true,
        },
    ]
}

fn schema_binding_specs() -> [(&'static str, &'static str); EXPECTED_SCHEMA_BINDING_COUNT] {
    [
        ("echo.generated-artifact/v1", "generated-artifact"),
        ("echo.review-payload/v1", "review-payload"),
        ("echo.verifier-report/v1", "verifier-report"),
        ("edict.authority-facts/v1", "authority-facts"),
        ("edict.core.module/v1", "core-module"),
        ("edict.lawpack/v1", "lawpack-manifest"),
        ("edict.lowering-requirements/v1", "lowering-requirements"),
        ("edict.target-ir.artifact/v1", "target-ir-artifact"),
        ("edict.target-profile/v1", "target-profile-manifest"),
    ]
}
