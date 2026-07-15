// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Strict admission of the Edict-owned provider contract pack.
//!
//! The generator receives the CDDL and manifest bytes explicitly. This module
//! performs no filesystem, registry, environment, or network discovery. It
//! admits only the publication merged in Edict PR #162: internal manifest
//! consistency is necessary, but does not substitute for the pinned external
//! identity checked here.

use std::fmt;
use std::sync::Arc;

use cddl_cat::cbor::validate_cbor;
use cddl_cat::context::BasicContext;
use cddl_cat::flatten::flatten_from_str;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::provider_canonical::{
    decode_canonical_cbor_v1, CanonicalValueErrorKind, CanonicalValueV1,
};

/// Exact Edict contract-pack API accepted by Echo provider generation.
pub const EDICT_PROVIDER_CONTRACT_PACK_API_V1: &str = "edict.provider-contract-pack/v1";

/// Exact Edict contract-pack coordinate accepted by Echo provider generation.
pub const EDICT_PROVIDER_CONTRACT_PACK_COORDINATE_V1: &str = "edict.provider-contract-pack.cddl@1";

/// License carried by the admitted Edict publication.
pub const EDICT_PROVIDER_CONTRACT_PACK_LICENSE: &str = "Apache-2.0";

/// SHA-256 of the admitted self-contained CDDL bytes.
pub const EDICT_PROVIDER_CONTRACT_PACK_SCHEMA_SHA256: &str =
    "92697bc9a5262c68258be9ee451ee8c144aeb363b92142915b8224430b85cf74";

/// SHA-256 of the admitted Edict publication manifest bytes.
pub const EDICT_PROVIDER_CONTRACT_PACK_MANIFEST_SHA256: &str =
    "6902467149fec3e0338bb90e8cd7963ee21b8ce24f368f9b12e748343cbe0e4f";

/// Maximum manifest size parsed at the contract-pack authority boundary.
pub const EDICT_PROVIDER_CONTRACT_PACK_MANIFEST_MAX_BYTES: usize = 61_713;

const EXPECTED_CONTRACTS: [(&str, &str); 9] = [
    ("authority-facts", "authority-facts"),
    ("core-module", "core-module"),
    ("lawpack-exports", "lawpack-exports"),
    ("lawpack-manifest", "lawpack-manifest"),
    ("lowering-requirements", "lowering-requirements"),
    ("target-ir-artifact", "target-ir-artifact"),
    ("target-profile-intrinsics", "intrinsics-document"),
    ("target-profile-manifest", "target-profile-manifest"),
    (
        "target-profile-operation-profiles",
        "operation-profiles-document",
    ),
];

const EXPECTED_DOMAINS: [(&str, &str); 6] = [
    ("edict.authority-facts/v1", "authority-facts"),
    ("edict.core.module/v1", "core-module"),
    ("edict.lawpack/v1", "lawpack-manifest"),
    ("edict.lowering-requirements/v1", "lowering-requirements"),
    ("edict.target-ir.artifact/v1", "target-ir-artifact"),
    ("edict.target-profile/v1", "target-profile-manifest"),
];

const EXPECTED_RESOURCES: [ExpectedResource; 5] = [
    ExpectedResource {
        coordinate: "edict.canonical-cbor/v1",
        raw_sha256: "8306e4f08c1e4e7d29ab22bcf55c324312712aac3eeeb675857ced57c3e48bdc",
        domain_framed_digest:
            "sha256:d1ea6d3de2a9762a438cbf4fac1d5ae2f357a4b27d13e0347e94ea655bf40f9d",
        source_path: "fixtures/target-profile/contract-resources/canonical-cbor.cbor",
    },
    ExpectedResource {
        coordinate: "edict.determinism/v1",
        raw_sha256: "84073f5c1734b625e16799048e28458d43e0a10befdae56a79d906f5e37ef76a",
        domain_framed_digest:
            "sha256:af4e6c774d5ea82db30e680be2ba7abc5ba04c6aead146139de32d8c5bb4981e",
        source_path: "fixtures/target-profile/contract-resources/determinism.cbor",
    },
    ExpectedResource {
        coordinate: "edict.diagnostics/v1",
        raw_sha256: "e465a28f1170fe478db5ff65d96a1fdbfbbcb95d327e6965021c252437991e4b",
        domain_framed_digest:
            "sha256:28fd72a98223153982ca084c29dbb1b2d430623967ab3b6db9d7fee668e614b9",
        source_path: "fixtures/target-profile/contract-resources/diagnostics.cbor",
    },
    ExpectedResource {
        coordinate: "edict.fuel/v1",
        raw_sha256: "c712c2d831cc8e731bdc1dfb8ea536f4630f38f0b0e0c6448b2df57176d3d0bd",
        domain_framed_digest:
            "sha256:006c6ebc01a3c5d36d50bd390b69d47e42378be61074e3b5c96ecb9f5ee53207",
        source_path: "fixtures/target-profile/contract-resources/fuel.cbor",
    },
    ExpectedResource {
        coordinate: "edict.wasm-component/v1",
        raw_sha256: "cd09d702db1d10be825e72effe35627bec56963cb3d78601b69478c58787e34d",
        domain_framed_digest:
            "sha256:095b4dd18f1a6a7276533f758665be319c5f476e7bdc70cc56d30b6b3e9f0a80",
        source_path: "fixtures/target-profile/contract-resources/wasm-component.cbor",
    },
];

const EDICT_REPOSITORY: &str = "https://github.com/flyingrobots/edict";

#[derive(Clone, Copy)]
struct ExpectedResource {
    coordinate: &'static str,
    raw_sha256: &'static str,
    domain_framed_digest: &'static str,
    source_path: &'static str,
}

/// One admitted contract name to CDDL root-rule binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderContractBindingV1 {
    contract: String,
    root_rule: String,
}

impl ProviderContractBindingV1 {
    /// Returns the stable contract name.
    #[must_use]
    pub fn contract(&self) -> &str {
        &self.contract
    }

    /// Returns the CDDL root rule for this contract.
    #[must_use]
    pub fn root_rule(&self) -> &str {
        &self.root_rule
    }
}

/// One admitted artifact domain to CDDL root-rule binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderDomainBindingV1 {
    domain: String,
    root_rule: String,
}

impl ProviderDomainBindingV1 {
    /// Returns the canonical artifact domain.
    #[must_use]
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Returns the CDDL root rule for this artifact domain.
    #[must_use]
    pub fn root_rule(&self) -> &str {
        &self.root_rule
    }
}

/// One immutable contract resource admitted from the Edict publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderContractResourceV1 {
    coordinate: String,
    canonical_bytes: Vec<u8>,
    raw_sha256: String,
    domain_framed_digest: String,
    repository: String,
    source_path: String,
}

impl ProviderContractResourceV1 {
    /// Returns the resource coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }

    /// Returns the exact canonical resource bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the SHA-256 of the canonical resource bytes.
    #[must_use]
    pub fn raw_sha256(&self) -> &str {
        &self.raw_sha256
    }

    /// Returns the Edict domain-framed digest published for this resource.
    #[must_use]
    pub fn domain_framed_digest(&self) -> &str {
        &self.domain_framed_digest
    }

    /// Returns the upstream repository that owns this resource.
    #[must_use]
    pub fn repository(&self) -> &str {
        &self.repository
    }

    /// Returns the upstream repository-relative source path.
    #[must_use]
    pub fn source_path(&self) -> &str {
        &self.source_path
    }
}

/// Opaque proof that explicit bytes match Echo's pinned Edict publication.
#[derive(Clone)]
pub struct AdmittedProviderContractPackV1 {
    schema_bytes: Vec<u8>,
    manifest_bytes: Vec<u8>,
    contracts: Vec<ProviderContractBindingV1>,
    domains: Vec<ProviderDomainBindingV1>,
    resources: Vec<ProviderContractResourceV1>,
    schema_context: Arc<BasicContext>,
}

impl fmt::Debug for AdmittedProviderContractPackV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmittedProviderContractPackV1")
            .field("schema_bytes", &self.schema_bytes)
            .field("manifest_bytes", &self.manifest_bytes)
            .field("contracts", &self.contracts)
            .field("domains", &self.domains)
            .field("resources", &self.resources)
            .finish_non_exhaustive()
    }
}

impl PartialEq for AdmittedProviderContractPackV1 {
    fn eq(&self, other: &Self) -> bool {
        self.schema_bytes == other.schema_bytes
            && self.manifest_bytes == other.manifest_bytes
            && self.contracts == other.contracts
            && self.domains == other.domains
            && self.resources == other.resources
    }
}

impl Eq for AdmittedProviderContractPackV1 {}

impl AdmittedProviderContractPackV1 {
    /// Returns the exact admitted API version.
    #[must_use]
    pub const fn api_version(&self) -> &'static str {
        EDICT_PROVIDER_CONTRACT_PACK_API_V1
    }

    /// Returns the exact admitted pack coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &'static str {
        EDICT_PROVIDER_CONTRACT_PACK_COORDINATE_V1
    }

    /// Returns the license declared by the admitted publication.
    #[must_use]
    pub const fn license(&self) -> &'static str {
        EDICT_PROVIDER_CONTRACT_PACK_LICENSE
    }

    /// Returns the exact admitted self-contained CDDL bytes.
    #[must_use]
    pub fn schema_bytes(&self) -> &[u8] {
        &self.schema_bytes
    }

    /// Returns the SHA-256 of the admitted CDDL bytes.
    #[must_use]
    pub const fn schema_sha256(&self) -> &'static str {
        EDICT_PROVIDER_CONTRACT_PACK_SCHEMA_SHA256
    }

    /// Returns the exact admitted publication manifest bytes.
    #[must_use]
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    /// Returns the SHA-256 of the admitted publication manifest bytes.
    #[must_use]
    pub const fn manifest_sha256(&self) -> &'static str {
        EDICT_PROVIDER_CONTRACT_PACK_MANIFEST_SHA256
    }

    /// Returns all contract to root-rule bindings in publication order.
    #[must_use]
    pub fn contracts(&self) -> &[ProviderContractBindingV1] {
        &self.contracts
    }

    /// Returns all artifact-domain to root-rule bindings in publication order.
    #[must_use]
    pub fn domains(&self) -> &[ProviderDomainBindingV1] {
        &self.domains
    }

    /// Returns all immutable contract resources in publication order.
    #[must_use]
    pub fn resources(&self) -> &[ProviderContractResourceV1] {
        &self.resources
    }

    /// Returns the number of admitted named contracts.
    #[must_use]
    pub fn contract_count(&self) -> usize {
        self.contracts.len()
    }

    /// Returns the number of admitted artifact-domain bindings.
    #[must_use]
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }

    /// Returns the number of admitted immutable contract resources.
    #[must_use]
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Returns the CDDL root rule bound to an admitted artifact domain.
    #[must_use]
    pub fn root_for_domain(&self, domain: &str) -> Option<&str> {
        self.domains
            .iter()
            .find(|binding| binding.domain == domain)
            .map(ProviderDomainBindingV1::root_rule)
    }

    /// Returns an admitted immutable resource by exact coordinate.
    #[must_use]
    pub fn resource(&self, coordinate: &str) -> Option<&ProviderContractResourceV1> {
        self.resources
            .iter()
            .find(|resource| resource.coordinate == coordinate)
    }

    /// Decode canonical bytes and validate them against one named owning root.
    ///
    /// This boundary validates only the trusted, digest-pinned Edict publication.
    /// It does not admit an artifact into the Echo runtime or grant runtime
    /// authority.
    ///
    /// # Errors
    ///
    /// Returns a stable structured error when the contract is unknown, the
    /// supplied bytes are not exact Edict canonical CBOR, or the decoded value
    /// does not satisfy the named contract's admitted CDDL root.
    pub fn validate_contract_bytes(
        &self,
        contract: &str,
        bytes: &[u8],
    ) -> Result<CanonicalValueV1, ProviderContractValidationError> {
        let root = self
            .contracts
            .iter()
            .find(|binding| binding.contract == contract)
            .map(ProviderContractBindingV1::root_rule)
            .ok_or_else(|| {
                ProviderContractValidationError::new(
                    ProviderContractValidationErrorKind::UnknownContract,
                    contract,
                    None,
                )
            })?;
        let value = decode_canonical_cbor_v1(bytes).map_err(|error| {
            ProviderContractValidationError::new(
                ProviderContractValidationErrorKind::CanonicalEncodingInvalid,
                contract,
                Some(error.kind()),
            )
        })?;
        let cbor_value: ciborium::Value = ciborium::from_reader(bytes).map_err(|_| {
            ProviderContractValidationError::new(
                ProviderContractValidationErrorKind::CanonicalEncodingInvalid,
                contract,
                None,
            )
        })?;
        let rule = self.schema_context.rules.get(root).ok_or_else(|| {
            ProviderContractValidationError::new(
                ProviderContractValidationErrorKind::SchemaMismatch,
                contract,
                None,
            )
        })?;
        validate_cbor(rule, &cbor_value, self.schema_context.as_ref()).map_err(|_| {
            ProviderContractValidationError::new(
                ProviderContractValidationErrorKind::SchemaMismatch,
                contract,
                None,
            )
        })?;
        Ok(value)
    }
}

/// Stable failure categories returned by named provider-contract validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderContractValidationErrorKind {
    /// The admitted publication does not declare the requested contract name.
    UnknownContract,
    /// The supplied bytes are not exact `edict.canonical-cbor/v1` bytes.
    CanonicalEncodingInvalid,
    /// The canonical value does not satisfy the contract's owning CDDL root.
    SchemaMismatch,
}

impl ProviderContractValidationErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::UnknownContract => "unknown-contract",
            Self::CanonicalEncodingInvalid => "canonical-encoding-invalid",
            Self::SchemaMismatch => "schema-mismatch",
        }
    }
}

/// Structured failure from canonical bytes and owning-root validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderContractValidationError {
    kind: ProviderContractValidationErrorKind,
    subject: String,
    canonical_value_kind: Option<CanonicalValueErrorKind>,
}

impl ProviderContractValidationError {
    /// Returns the stable validation failure category.
    #[must_use]
    pub const fn kind(&self) -> ProviderContractValidationErrorKind {
        self.kind
    }

    /// Returns the requested logical contract name.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the typed canonical-value cause, when decoding failed.
    #[must_use]
    pub const fn canonical_value_kind(&self) -> Option<CanonicalValueErrorKind> {
        self.canonical_value_kind
    }

    fn new(
        kind: ProviderContractValidationErrorKind,
        subject: impl Into<String>,
        canonical_value_kind: Option<CanonicalValueErrorKind>,
    ) -> Self {
        Self {
            kind,
            subject: subject.into(),
            canonical_value_kind,
        }
    }
}

impl fmt::Display for ProviderContractValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider contract validation {}: {}",
            self.kind.label(),
            self.subject
        )
    }
}

impl std::error::Error for ProviderContractValidationError {}

/// Stable failure categories returned by contract-pack admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderContractPackErrorKind {
    /// Manifest input exceeded the pinned publication's byte length.
    ManifestSizeExceeded,
    /// Manifest JSON did not match the strict publication shape.
    ManifestMalformed,
    /// The manifest selected an unsupported API version.
    UnsupportedApiVersion,
    /// The manifest selected the wrong publication coordinate.
    CoordinateMismatch,
    /// The manifest did not declare the required Apache-2.0 license.
    LicenseMismatch,
    /// A schema byte string was not canonical lowercase hexadecimal.
    SchemaHexInvalid,
    /// Supplied CDDL bytes differed from the bytes embedded in the manifest.
    SchemaBytesMismatch,
    /// The CDDL digest differed from the manifest or pinned publication.
    SchemaDigestMismatch,
    /// The named contract and root-rule inventory differed from the publication.
    ContractInventoryMismatch,
    /// The artifact-domain and root-rule inventory differed from the publication.
    DomainInventoryMismatch,
    /// A resource was missing, reordered, duplicated, or substituted.
    ResourceInventoryMismatch,
    /// A resource byte string was not canonical lowercase hexadecimal.
    ResourceHexInvalid,
    /// A resource raw digest did not bind its bytes and pinned publication.
    ResourceRawDigestMismatch,
    /// A resource domain-framed digest differed from the pinned publication.
    ResourceDomainDigestMismatch,
    /// A resource provenance record differed from the pinned publication.
    ResourceProvenanceMismatch,
    /// The manifest bytes differed despite matching known semantic fields.
    ManifestDigestMismatch,
    /// The authenticated self-contained CDDL publication failed to compile.
    SchemaCompilationFailed,
    /// The compiled publication did not contain one of its declared roots.
    SchemaRootMissing,
}

impl ProviderContractPackErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::ManifestSizeExceeded => "manifest-size-exceeded",
            Self::ManifestMalformed => "manifest-malformed",
            Self::UnsupportedApiVersion => "unsupported-api-version",
            Self::CoordinateMismatch => "coordinate-mismatch",
            Self::LicenseMismatch => "license-mismatch",
            Self::SchemaHexInvalid => "schema-hex-invalid",
            Self::SchemaBytesMismatch => "schema-bytes-mismatch",
            Self::SchemaDigestMismatch => "schema-digest-mismatch",
            Self::ContractInventoryMismatch => "contract-inventory-mismatch",
            Self::DomainInventoryMismatch => "domain-inventory-mismatch",
            Self::ResourceInventoryMismatch => "resource-inventory-mismatch",
            Self::ResourceHexInvalid => "resource-hex-invalid",
            Self::ResourceRawDigestMismatch => "resource-raw-digest-mismatch",
            Self::ResourceDomainDigestMismatch => "resource-domain-digest-mismatch",
            Self::ResourceProvenanceMismatch => "resource-provenance-mismatch",
            Self::ManifestDigestMismatch => "manifest-digest-mismatch",
            Self::SchemaCompilationFailed => "schema-compilation-failed",
            Self::SchemaRootMissing => "schema-root-missing",
        }
    }
}

/// Structured contract-pack parsing or admission failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderContractPackError {
    kind: ProviderContractPackErrorKind,
    subject: String,
    reference: String,
}

impl ProviderContractPackError {
    /// Returns the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> ProviderContractPackErrorKind {
        self.kind
    }

    /// Returns the field or resource being validated.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the conflicting or expected publication value.
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    fn new(
        kind: ProviderContractPackErrorKind,
        subject: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject: subject.into(),
            reference: reference.into(),
        }
    }
}

impl fmt::Display for ProviderContractPackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider contract pack {}: {} -> {}",
            self.kind.label(),
            self.subject,
            self.reference
        )
    }
}

impl std::error::Error for ProviderContractPackError {}

/// Authenticate and admit one explicit Edict provider contract-pack publication.
///
/// This function performs no discovery or I/O. Both byte slices must be the
/// exact reviewed publication selected by Echo.
///
/// # Errors
///
/// Returns a stable structured error when the manifest is malformed, its
/// inventory differs, supplied CDDL differs, a resource is tampered, or any
/// byte or semantic identity differs from the pinned Edict publication.
pub fn admit_provider_contract_pack_v1(
    schema_bytes: &[u8],
    manifest_bytes: &[u8],
) -> Result<AdmittedProviderContractPackV1, ProviderContractPackError> {
    if manifest_bytes.len() > EDICT_PROVIDER_CONTRACT_PACK_MANIFEST_MAX_BYTES {
        return Err(ProviderContractPackError::new(
            ProviderContractPackErrorKind::ManifestSizeExceeded,
            "manifest.bytes",
            EDICT_PROVIDER_CONTRACT_PACK_MANIFEST_MAX_BYTES.to_string(),
        ));
    }

    let manifest =
        serde_json::from_slice::<ContractPackManifest>(manifest_bytes).map_err(|_| {
            ProviderContractPackError::new(
                ProviderContractPackErrorKind::ManifestMalformed,
                "manifest",
                EDICT_PROVIDER_CONTRACT_PACK_API_V1,
            )
        })?;

    require_exact(
        &manifest.api_version,
        EDICT_PROVIDER_CONTRACT_PACK_API_V1,
        ProviderContractPackErrorKind::UnsupportedApiVersion,
        "apiVersion",
    )?;
    require_exact(
        &manifest.coordinate,
        EDICT_PROVIDER_CONTRACT_PACK_COORDINATE_V1,
        ProviderContractPackErrorKind::CoordinateMismatch,
        "coordinate",
    )?;
    require_exact(
        &manifest.license,
        EDICT_PROVIDER_CONTRACT_PACK_LICENSE,
        ProviderContractPackErrorKind::LicenseMismatch,
        "license",
    )?;

    let embedded_schema = decode_lower_hex(
        &manifest.schema.bytes_hex,
        ProviderContractPackErrorKind::SchemaHexInvalid,
        "schema.bytesHex",
    )?;
    if embedded_schema != schema_bytes {
        return Err(ProviderContractPackError::new(
            ProviderContractPackErrorKind::SchemaBytesMismatch,
            "schema.bytesHex",
            EDICT_PROVIDER_CONTRACT_PACK_SCHEMA_SHA256,
        ));
    }

    let schema_digest = sha256_hex(schema_bytes);
    if manifest.schema.raw_sha256 != schema_digest
        || schema_digest != EDICT_PROVIDER_CONTRACT_PACK_SCHEMA_SHA256
    {
        return Err(ProviderContractPackError::new(
            ProviderContractPackErrorKind::SchemaDigestMismatch,
            "schema.rawSha256",
            EDICT_PROVIDER_CONTRACT_PACK_SCHEMA_SHA256,
        ));
    }

    validate_bindings(
        &manifest.contracts,
        &EXPECTED_CONTRACTS,
        ProviderContractPackErrorKind::ContractInventoryMismatch,
        "contracts",
        |binding| (&binding.contract, &binding.root_rule),
    )?;
    validate_bindings(
        &manifest.domains,
        &EXPECTED_DOMAINS,
        ProviderContractPackErrorKind::DomainInventoryMismatch,
        "domains",
        |binding| (&binding.domain, &binding.root_rule),
    )?;

    let resources = validate_resources(manifest.resources)?;

    let manifest_digest = sha256_hex(manifest_bytes);
    if manifest_digest != EDICT_PROVIDER_CONTRACT_PACK_MANIFEST_SHA256 {
        return Err(ProviderContractPackError::new(
            ProviderContractPackErrorKind::ManifestDigestMismatch,
            "manifest",
            EDICT_PROVIDER_CONTRACT_PACK_MANIFEST_SHA256,
        ));
    }

    let schema_context = compile_schema_context(schema_bytes)?;

    Ok(AdmittedProviderContractPackV1 {
        schema_bytes: schema_bytes.to_vec(),
        manifest_bytes: manifest_bytes.to_vec(),
        contracts: manifest
            .contracts
            .into_iter()
            .map(|binding| ProviderContractBindingV1 {
                contract: binding.contract,
                root_rule: binding.root_rule,
            })
            .collect(),
        domains: manifest
            .domains
            .into_iter()
            .map(|binding| ProviderDomainBindingV1 {
                domain: binding.domain,
                root_rule: binding.root_rule,
            })
            .collect(),
        resources,
        schema_context,
    })
}

fn compile_schema_context(
    schema_bytes: &[u8],
) -> Result<Arc<BasicContext>, ProviderContractPackError> {
    let schema = std::str::from_utf8(schema_bytes).map_err(|_| {
        ProviderContractPackError::new(
            ProviderContractPackErrorKind::SchemaCompilationFailed,
            EDICT_PROVIDER_CONTRACT_PACK_COORDINATE_V1,
            "utf-8 CDDL",
        )
    })?;
    let rules = flatten_from_str(schema).map_err(|_| {
        ProviderContractPackError::new(
            ProviderContractPackErrorKind::SchemaCompilationFailed,
            EDICT_PROVIDER_CONTRACT_PACK_COORDINATE_V1,
            "compiled CDDL",
        )
    })?;
    let context = Arc::new(BasicContext::new(rules));
    for (_, root) in EXPECTED_CONTRACTS.into_iter().chain(EXPECTED_DOMAINS) {
        if !context.rules.contains_key(root) {
            return Err(ProviderContractPackError::new(
                ProviderContractPackErrorKind::SchemaRootMissing,
                root,
                EDICT_PROVIDER_CONTRACT_PACK_COORDINATE_V1,
            ));
        }
    }
    Ok(context)
}

fn validate_resources(
    resources: Vec<ManifestResource>,
) -> Result<Vec<ProviderContractResourceV1>, ProviderContractPackError> {
    if resources.len() != EXPECTED_RESOURCES.len() {
        return Err(ProviderContractPackError::new(
            ProviderContractPackErrorKind::ResourceInventoryMismatch,
            "resources",
            EXPECTED_RESOURCES.len().to_string(),
        ));
    }

    resources
        .into_iter()
        .zip(EXPECTED_RESOURCES)
        .map(|(resource, expected)| validate_resource(resource, expected))
        .collect()
}

fn validate_resource(
    resource: ManifestResource,
    expected: ExpectedResource,
) -> Result<ProviderContractResourceV1, ProviderContractPackError> {
    if resource.coordinate != expected.coordinate {
        return Err(ProviderContractPackError::new(
            ProviderContractPackErrorKind::ResourceInventoryMismatch,
            resource.coordinate,
            expected.coordinate,
        ));
    }

    let canonical_bytes = decode_lower_hex(
        &resource.canonical_bytes_hex,
        ProviderContractPackErrorKind::ResourceHexInvalid,
        expected.coordinate,
    )?;
    if sha256_hex(&canonical_bytes) != resource.raw_sha256
        || resource.raw_sha256 != expected.raw_sha256
    {
        return Err(ProviderContractPackError::new(
            ProviderContractPackErrorKind::ResourceRawDigestMismatch,
            expected.coordinate,
            expected.raw_sha256,
        ));
    }
    if resource.domain_framed_digest != expected.domain_framed_digest {
        return Err(ProviderContractPackError::new(
            ProviderContractPackErrorKind::ResourceDomainDigestMismatch,
            expected.coordinate,
            expected.domain_framed_digest,
        ));
    }
    if resource.provenance.repository != EDICT_REPOSITORY
        || resource.provenance.source_path != expected.source_path
    {
        return Err(ProviderContractPackError::new(
            ProviderContractPackErrorKind::ResourceProvenanceMismatch,
            expected.coordinate,
            expected.source_path,
        ));
    }

    Ok(ProviderContractResourceV1 {
        coordinate: resource.coordinate,
        canonical_bytes,
        raw_sha256: resource.raw_sha256,
        domain_framed_digest: resource.domain_framed_digest,
        repository: resource.provenance.repository,
        source_path: resource.provenance.source_path,
    })
}

fn validate_bindings<T, F>(
    actual: &[T],
    expected: &[(&str, &str)],
    kind: ProviderContractPackErrorKind,
    subject: &'static str,
    fields: F,
) -> Result<(), ProviderContractPackError>
where
    F: Fn(&T) -> (&String, &String),
{
    let matches = actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(binding, (expected_name, expected_root))| {
                let (name, root) = fields(binding);
                name == expected_name && root == expected_root
            });
    if matches {
        Ok(())
    } else {
        Err(ProviderContractPackError::new(
            kind,
            subject,
            EDICT_PROVIDER_CONTRACT_PACK_COORDINATE_V1,
        ))
    }
}

fn require_exact(
    actual: &str,
    expected: &'static str,
    kind: ProviderContractPackErrorKind,
    subject: &'static str,
) -> Result<(), ProviderContractPackError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ProviderContractPackError::new(kind, subject, expected))
    }
}

fn decode_lower_hex(
    value: &str,
    kind: ProviderContractPackErrorKind,
    subject: impl Into<String>,
) -> Result<Vec<u8>, ProviderContractPackError> {
    if !value.len().is_multiple_of(2)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ProviderContractPackError::new(kind, subject, "lower-hex"));
    }
    hex::decode(value).map_err(|_| ProviderContractPackError::new(kind, subject, "lower-hex"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContractPackManifest {
    api_version: String,
    coordinate: String,
    license: String,
    schema: ManifestSchema,
    contracts: Vec<ManifestContractBinding>,
    domains: Vec<ManifestDomainBinding>,
    resources: Vec<ManifestResource>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestSchema {
    bytes_hex: String,
    raw_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestContractBinding {
    contract: String,
    root_rule: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestDomainBinding {
    domain: String,
    root_rule: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestResource {
    coordinate: String,
    canonical_bytes_hex: String,
    raw_sha256: String,
    domain_framed_digest: String,
    provenance: ManifestResourceProvenance,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestResourceProvenance {
    repository: String,
    source_path: String,
}
