// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure deterministic construction of the first Echo provider artifact closure.
//!
//! These values describe Edict-authored provider semantics. They do not admit
//! artifacts into Echo, install operations, grant runtime authority, schedule
//! work, or observe causal history. Every byte consumed by this module is an
//! explicit argument, and every emitted CBOR value is checked against both the
//! generated self-contained schema and, when Edict owns the format, the
//! independently admitted upstream root.

use std::fmt;
use std::sync::Arc;

use cddl_cat::cbor::validate_cbor;
use cddl_cat::context::BasicContext;
use cddl_cat::flatten::flatten_from_str;
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use wesley_core::{
    GenerationArtifactReferenceV1, GenerationContractError, GenerationContractErrorKind,
};

use crate::provider_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_v1, encode_canonical_cbor_v1,
    CanonicalValueErrorKind, CanonicalValueV1,
};
use crate::provider_contract_pack::{
    AdmittedProviderContractPackV1, ProviderContractValidationErrorKind,
};
use crate::provider_generation::ProviderGenerationInputV1;
use crate::provider_semantics::{
    ArtifactResourceDeclaration, ArtifactResourceProvision, AuthorityClass,
    AuthorityFactSourceKind, EffectKindHint, ExecutionClass, GeneratedArtifactDeclaration,
    GeneratedArtifactKind, LawpackVerifierDeclaration, OpticKind, ProviderSemanticSourceV1,
};

const SCHEMA_ROLE: &str = "schema.echo-provider-artifacts";
const GENERATED_SCHEMA_DOMAIN: &str = "echo.provider-artifacts.cddl@1";
const GENERATED_PROFILE_ROOT: &str = "generated-artifact-profile";
const GENERATED_PROFILE_DOMAIN: &str = "echo.generated-artifact-profile/v1";

const PROVIDER_SCHEMA_SUFFIX: &str = r#"

; --- Echo provider-generated declarative contracts -----------------------
; These roots validate provider semantic descriptions. They confer no Echo
; runtime authority and contain no package, component, or installation state.

generated-artifact-profile = {
  apiVersion: "echo.generated-artifact-profile/v1",
  targetProfile: tstr,
  types: { * tstr => echo-generated-type },
  operations: { * tstr => generated-operation },
}

echo-generated-type = echo-generated-string-alias / echo-generated-record
echo-generated-string-alias = {
  kind: "coreStringAlias",
  maxScalarValues: uint,
  canonical: "raw-utf8",
}
echo-generated-record = {
  kind: "record",
  fields: [* echo-generated-record-field],
}
echo-generated-record-field = { name: tstr, type: tstr }

generated-operation = {
  inputType: tstr,
  outputType: tstr,
  effect: tstr,
  operationProfile: tstr,
  opticContract: tstr,
  budget: tstr,
  invocationKind: "mutation" / "observer",
  implementation: { kind: "native" / "directAdapter", coordinate: tstr },
  obstructionMappings: { * tstr => tstr },
}

echo-provider-conformance-corpus = {
  apiVersion: "echo.edict-provider.conformance-corpus/v1",
  class: "declarative",
  operations: { * tstr => null },
  capabilities: { * tstr => null },
  semanticEffects: { * tstr => null },
  cases: [],
}

echo-provider-lawpack-compatibility = {
  apiVersion: "echo.edict-provider.lawpack-compatibility/v1",
  class: "declarative",
  acceptedCoreAbi: { * tstr => null },
  acceptedTargetProfiles: { * tstr => null },
  semanticEffects: { * tstr => null },
}

echo-provider-lawpack-target-adapter = {
  apiVersion: "echo.edict-provider.lawpack-target-adapter/v1",
  class: "declarative",
  targetProfile: tstr,
  targetIrDomain: tstr,
  effectImplementations: { * tstr => echo-effect-implementation },
}

echo-effect-implementation = echo-native-effect-implementation / echo-direct-effect-implementation
echo-native-effect-implementation = {
  kind: "native",
  capability: tstr,
  writeClass: tstr,
}
echo-direct-effect-implementation = {
  kind: "directAdapter",
  adapter: tstr,
  capability: tstr,
  writeClass: tstr,
}

echo-provider-lawpack-verifier = {
  apiVersion: "echo.edict-provider.lawpack-verifier/v1",
  class: "declarative",
  operationObstructions: { * tstr => echo-operation-obstructions },
}
echo-operation-obstructions = {
  effect: tstr,
  failureMappings: { * failure-ident => tstr },
}

echo-dpo-bundle = {
  apiVersion: "echo.dpo.bundle/v1",
  class: "declarative",
  applicationModel: tstr,
  readConsistency: tstr,
  operationProfiles: { * tstr => null },
}

echo-dpo-cost = {
  apiVersion: "echo.dpo.cost/v1",
  class: "declarative",
  capabilities: { * tstr => echo-cost-capability },
}
echo-cost-capability = {
  effect: tstr,
  costTemplate: tstr,
  semanticObligation: tstr,
}

echo-dpo-footprint = {
  apiVersion: "echo.dpo.footprint/v1",
  class: "declarative",
  capabilities: { * tstr => echo-footprint-capability },
}
echo-footprint-capability = {
  effect: tstr,
  footprintTemplate: tstr,
  semanticObligation: tstr,
  writeClass: tstr,
}

echo-span-ir = {
  apiVersion: "echo.span-ir/v1",
  class: "declarative",
  domain: tstr,
  targetProfile: tstr,
  capabilities: { * tstr => null },
}

echo-dpo-lowerer = {
  apiVersion: "echo.dpo.lowerer/v1",
  class: "declarative",
  acceptedCoreAbi: { * tstr => null },
  outputDomain: tstr,
  targetProfile: tstr,
  effectImplementations: { * tstr => echo-effect-implementation },
  opticContracts: { * tstr => tstr },
}

echo-dpo-obstructions = {
  apiVersion: "echo.dpo.obstructions/v1",
  class: "declarative",
  effectFailures: { * tstr => { authorityClass: authority-class, payloadType: tstr } },
  domainObstructions: { * tstr => { authorityClass: authority-class, payloadSchema: tstr } },
}

echo-dpo-verifier = {
  apiVersion: "echo.dpo.verifier/v1",
  class: "declarative",
  targetProfile: tstr,
  targetIrDomain: tstr,
  capabilities: { * tstr => null },
  operationProfiles: { * tstr => null },
  opticContracts: { * tstr => tstr },
}

echo-nonempty-tstr = tstr .regexp "(?s).+"

generated-artifact = {
  apiVersion: "echo.generated-artifact/v1",
  profile: resource-ref,
  operation: echo-nonempty-tstr,
  mediaType: echo-nonempty-tstr,
  bytes: bstr,
}

review-payload = {
  apiVersion: "echo.review-payload/v1",
  authoritative: false,
  subject: resource-ref,
  mediaType: echo-nonempty-tstr,
  bytes: bstr,
}

verifier-report = {
  apiVersion: "echo.verifier-report/v1",
  targetIr: resource-ref,
  outcome: "accepted" / "rejected",
  diagnosticAbi: resource-ref,
  diagnosticBytes: bstr,
}
"#;

/// Stable failure categories returned by provider artifact construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderArtifactGenerationErrorKind {
    /// Requested primary projection roles differed from the validated source.
    ProjectionClosureMismatch,
    /// A required generated or external resource could not be resolved.
    ResourceClosureMismatch,
    /// The self-contained provider CDDL could not be compiled or lacked a root.
    SchemaGenerationFailed,
    /// A provider value could not be encoded as canonical CBOR.
    CanonicalEncodingFailed,
    /// Canonical bytes did not satisfy their generated or Edict-owned root.
    OwningRootRejected,
    /// A raw or domain-framed digest could not be constructed.
    DigestConstructionFailed,
    /// A validated semantic declaration needed by projection was absent.
    SemanticProjectionMismatch,
    /// Wesley rejected an emitted exact-byte content reference.
    WesleyContractRejected,
}

impl ProviderArtifactGenerationErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::ProjectionClosureMismatch => "projection-closure-mismatch",
            Self::ResourceClosureMismatch => "resource-closure-mismatch",
            Self::SchemaGenerationFailed => "schema-generation-failed",
            Self::CanonicalEncodingFailed => "canonical-encoding-failed",
            Self::OwningRootRejected => "owning-root-rejected",
            Self::DigestConstructionFailed => "digest-construction-failed",
            Self::SemanticProjectionMismatch => "semantic-projection-mismatch",
            Self::WesleyContractRejected => "wesley-contract-rejected",
        }
    }
}

/// Structured, stable failure from provider artifact construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderArtifactGenerationError {
    kind: ProviderArtifactGenerationErrorKind,
    subject: String,
    reference: String,
    canonical_kind: Option<CanonicalValueErrorKind>,
    contract_kind: Option<ProviderContractValidationErrorKind>,
    wesley_kind: Option<GenerationContractErrorKind>,
}

impl ProviderArtifactGenerationError {
    /// Returns the stable high-level failure category.
    #[must_use]
    pub const fn kind(&self) -> ProviderArtifactGenerationErrorKind {
        self.kind
    }

    /// Returns the role, coordinate, or schema root that failed.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the expected coordinate, root, or other stable reference.
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Returns the typed canonical-value cause, when applicable.
    #[must_use]
    pub const fn canonical_value_kind(&self) -> Option<CanonicalValueErrorKind> {
        self.canonical_kind
    }

    /// Returns the typed upstream contract cause, when applicable.
    #[must_use]
    pub const fn contract_validation_kind(&self) -> Option<ProviderContractValidationErrorKind> {
        self.contract_kind
    }

    /// Returns the typed Wesley content-reference failure, when applicable.
    #[must_use]
    pub const fn wesley_contract_kind(&self) -> Option<GenerationContractErrorKind> {
        self.wesley_kind
    }

    fn new(
        kind: ProviderArtifactGenerationErrorKind,
        subject: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject: subject.into(),
            reference: reference.into(),
            canonical_kind: None,
            contract_kind: None,
            wesley_kind: None,
        }
    }

    fn canonical(subject: &str, error: CanonicalValueErrorKind) -> Self {
        let mut result = Self::new(
            ProviderArtifactGenerationErrorKind::CanonicalEncodingFailed,
            subject,
            "edict.canonical-cbor/v1",
        );
        result.canonical_kind = Some(error);
        result
    }

    fn wesley(error: GenerationContractError) -> Self {
        let kind = error.kind;
        let mut result = Self::new(
            ProviderArtifactGenerationErrorKind::WesleyContractRejected,
            error.subject,
            kind.as_str(),
        );
        result.wesley_kind = Some(kind);
        result
    }
}

impl fmt::Display for ProviderArtifactGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider artifact generation {}: {} -> {}",
            self.kind.label(),
            self.subject,
            self.reference
        )
    }
}

impl std::error::Error for ProviderArtifactGenerationError {}

/// One canonical provider output that has passed its generated owning root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaValidatedCanonicalProviderOutputV1 {
    role: String,
    coordinate: String,
    schema_contract: String,
    owning_root: String,
    digest_domain: String,
    canonical_value: CanonicalValueV1,
    canonical_bytes: Vec<u8>,
    domain_framed_digest: String,
    content_reference: GenerationArtifactReferenceV1,
}

impl SchemaValidatedCanonicalProviderOutputV1 {
    /// Returns the source-local artifact or resource role.
    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }
    /// Returns the stable artifact coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }
    /// Returns the declared ABI or schema contract.
    #[must_use]
    pub fn schema_contract(&self) -> &str {
        &self.schema_contract
    }
    /// Returns the exact CDDL root used to validate the value.
    #[must_use]
    pub fn owning_root(&self) -> &str {
        &self.owning_root
    }
    /// Returns the domain used for the Edict digest frame.
    #[must_use]
    pub fn digest_domain(&self) -> &str {
        &self.digest_domain
    }
    /// Returns the admitted canonical value.
    #[must_use]
    pub const fn canonical_value(&self) -> &CanonicalValueV1 {
        &self.canonical_value
    }
    /// Returns exact Edict canonical-CBOR bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
    /// Returns the Edict domain-framed digest.
    #[must_use]
    pub fn domain_framed_digest(&self) -> &str {
        &self.domain_framed_digest
    }
    /// Returns the Wesley exact-byte content reference.
    #[must_use]
    pub const fn content_reference(&self) -> &GenerationArtifactReferenceV1 {
        &self.content_reference
    }
}

/// Self-contained CDDL emitted alongside the provider artifact closure.
#[derive(Clone)]
pub struct GeneratedProviderSchemaV1 {
    role: String,
    coordinate: String,
    bytes: Vec<u8>,
    content_reference: GenerationArtifactReferenceV1,
    context: Arc<BasicContext>,
}

impl fmt::Debug for GeneratedProviderSchemaV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeneratedProviderSchemaV1")
            .field("role", &self.role)
            .field("coordinate", &self.coordinate)
            .field("bytes", &self.bytes)
            .field("content_reference", &self.content_reference)
            .finish_non_exhaustive()
    }
}

impl PartialEq for GeneratedProviderSchemaV1 {
    fn eq(&self, other: &Self) -> bool {
        self.role == other.role
            && self.coordinate == other.coordinate
            && self.bytes == other.bytes
            && self.content_reference == other.content_reference
    }
}

impl Eq for GeneratedProviderSchemaV1 {}

impl GeneratedProviderSchemaV1 {
    /// Returns the generated artifact role.
    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }
    /// Returns the generated schema coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }
    /// Returns exact self-contained UTF-8 CDDL bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Returns the exact-byte schema content reference.
    #[must_use]
    pub const fn content_reference(&self) -> &GenerationArtifactReferenceV1 {
        &self.content_reference
    }

    /// Revalidates one canonical provider output against its declared root.
    ///
    /// # Errors
    ///
    /// Returns a stable error when the root is absent, bytes are not exact
    /// canonical CBOR, or the decoded value fails its owning CDDL rule.
    pub fn validate_output(
        &self,
        output: &SchemaValidatedCanonicalProviderOutputV1,
    ) -> Result<CanonicalValueV1, ProviderArtifactGenerationError> {
        validate_schema_bytes(&self.context, &output.owning_root, &output.canonical_bytes)
    }

    /// Authenticates exact canonical bytes against one named provider root.
    ///
    /// This is the raw component-boundary form of [`Self::validate_output`]. It
    /// validates encoding and schema shape only; it does not install an
    /// artifact or grant Echo runtime authority.
    ///
    /// # Errors
    ///
    /// Returns a stable error when the root is absent, the bytes are not exact
    /// Edict canonical CBOR, or the decoded value fails the selected CDDL rule.
    pub fn validate_root_bytes(
        &self,
        root: &str,
        bytes: &[u8],
    ) -> Result<CanonicalValueV1, ProviderArtifactGenerationError> {
        validate_schema_bytes(&self.context, root, bytes)
    }
}

/// Complete first-slice primary provider artifact closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderPrimaryArtifactsV1 {
    generation_input_digest: String,
    projection_roles: Vec<String>,
    artifacts: Vec<SchemaValidatedCanonicalProviderOutputV1>,
    resources: Vec<SchemaValidatedCanonicalProviderOutputV1>,
    schema: GeneratedProviderSchemaV1,
}

impl ProviderPrimaryArtifactsV1 {
    /// Returns the exact Wesley generation input that produced this closure.
    #[must_use]
    pub fn generation_input_digest(&self) -> &str {
        &self.generation_input_digest
    }
    /// Returns the exact sorted primary role closure, including the schema.
    #[must_use]
    pub fn projection_roles(&self) -> &[String] {
        &self.projection_roles
    }
    /// Returns the five canonical-CBOR primary artifacts in role order.
    #[must_use]
    pub fn artifacts(&self) -> &[SchemaValidatedCanonicalProviderOutputV1] {
        &self.artifacts
    }
    /// Returns the fourteen generated canonical resources in role order.
    #[must_use]
    pub fn resources(&self) -> &[SchemaValidatedCanonicalProviderOutputV1] {
        &self.resources
    }
    /// Returns the generated self-contained CDDL artifact.
    #[must_use]
    pub const fn schema(&self) -> &GeneratedProviderSchemaV1 {
        &self.schema
    }
    /// Looks up one primary canonical artifact by role.
    #[must_use]
    pub fn artifact(&self, role: &str) -> Option<&SchemaValidatedCanonicalProviderOutputV1> {
        self.artifacts.iter().find(|artifact| artifact.role == role)
    }
    /// Looks up one generated canonical resource by role.
    #[must_use]
    pub fn resource(&self, role: &str) -> Option<&SchemaValidatedCanonicalProviderOutputV1> {
        self.resources.iter().find(|resource| resource.role == role)
    }
}

/// Generates the deterministic primary provider artifact closure from explicit input.
///
/// The function is pure: it performs no filesystem, registry, environment,
/// process, clock, or network discovery. The returned values are provider
/// semantic descriptions, not Echo runtime admissions or grants.
///
/// # Errors
///
/// Returns a structured error if the requested projection closure disagrees
/// with the validated semantic source, a required resource cannot be resolved,
/// schema construction fails, canonical encoding fails, or any emitted value
/// is rejected by its owning CDDL root.
pub fn generate_provider_primary_artifacts_v1(
    input: &ProviderGenerationInputV1,
    contract_pack: &AdmittedProviderContractPackV1,
) -> Result<ProviderPrimaryArtifactsV1, ProviderArtifactGenerationError> {
    let source = input.semantic_source().source();
    let projection_roles = expected_projection_roles(source);
    if input.wesley_input().projection_roles != projection_roles {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::ProjectionClosureMismatch,
            "projectionRoles",
            projection_roles.join(","),
        ));
    }
    let schema = build_schema(source, contract_pack)?;

    let mut resources = Vec::new();
    for declaration in source
        .artifact_resources
        .iter()
        .filter(|resource| resource.provision == ArtifactResourceProvision::Generated)
    {
        let value = build_resource_value(source, declaration)?;
        resources.push(make_output(
            declaration.role.clone(),
            declaration.coordinate.clone(),
            declaration.schema_contract.clone(),
            resource_root(&declaration.role)?,
            declaration.coordinate.clone(),
            value,
            resource_edict_contract(&declaration.role),
            &schema,
            contract_pack,
        )?);
    }
    resources.sort_by(|left, right| left.role.cmp(&right.role));
    if resources.len() != 14 {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::ResourceClosureMismatch,
            "generatedResources",
            "14",
        ));
    }

    let profile_declaration =
        artifact_by_kind(source, GeneratedArtifactKind::GeneratedArtifactProfile)?;
    let profile = make_output(
        profile_declaration.role.clone(),
        profile_declaration.coordinate.clone(),
        profile_declaration.schema_contract.clone(),
        GENERATED_PROFILE_ROOT,
        GENERATED_PROFILE_DOMAIN.to_owned(),
        build_generated_profile(source)?,
        None,
        &schema,
        contract_pack,
    )?;

    let target_declaration = artifact_by_kind(source, GeneratedArtifactKind::TargetProfile)?;
    let target = make_output(
        target_declaration.role.clone(),
        target_declaration.coordinate.clone(),
        target_declaration.schema_contract.clone(),
        "target-profile-manifest",
        "edict.target-profile/v1".to_owned(),
        build_target_profile(source, &resources, &profile, contract_pack)?,
        Some("target-profile-manifest"),
        &schema,
        contract_pack,
    )?;

    let lawpack_declaration = artifact_by_kind(source, GeneratedArtifactKind::Lawpack)?;
    let lawpack = make_output(
        lawpack_declaration.role.clone(),
        lawpack_declaration.coordinate.clone(),
        lawpack_declaration.schema_contract.clone(),
        "lawpack-manifest",
        "edict.lawpack/v1".to_owned(),
        build_lawpack(source, &resources, &target)?,
        Some("lawpack-manifest"),
        &schema,
        contract_pack,
    )?;

    let mut artifacts = vec![profile, target, lawpack];
    for declaration in source
        .generated_artifacts
        .iter()
        .filter(|artifact| artifact.kind == GeneratedArtifactKind::AuthorityFacts)
    {
        let fact_source = declaration.authority_fact_source.as_ref().ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                &declaration.role,
                "authorityFactSource",
            )
        })?;
        let source_artifact = match fact_source.kind {
            AuthorityFactSourceKind::TargetProfile => artifacts
                .iter()
                .find(|artifact| artifact.role == source.target_profile_projection.artifact_role),
            AuthorityFactSourceKind::Lawpack => artifacts
                .iter()
                .find(|artifact| artifact.role == source.lawpack_projection.artifact_role),
        }
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::ProjectionClosureMismatch,
                &declaration.role,
                &fact_source.coordinate,
            )
        })?;
        artifacts.push(make_output(
            declaration.role.clone(),
            declaration.coordinate.clone(),
            declaration.schema_contract.clone(),
            "authority-facts",
            "edict.authority-facts/v1".to_owned(),
            build_authority_facts(source, declaration, source_artifact)?,
            Some("authority-facts"),
            &schema,
            contract_pack,
        )?);
    }
    artifacts.sort_by(|left, right| left.role.cmp(&right.role));
    if artifacts.len() != 5 {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::ProjectionClosureMismatch,
            "canonicalPrimaryArtifacts",
            "5",
        ));
    }

    Ok(ProviderPrimaryArtifactsV1 {
        generation_input_digest: input.digest().to_owned(),
        projection_roles,
        artifacts,
        resources,
        schema,
    })
}

fn expected_projection_roles(source: &ProviderSemanticSourceV1) -> Vec<String> {
    source
        .generated_artifacts
        .iter()
        .filter(|artifact| {
            !matches!(
                artifact.kind,
                GeneratedArtifactKind::GenerationProvenance | GeneratedArtifactKind::ReviewArtifact
            )
        })
        .map(|artifact| artifact.role.clone())
        .collect()
}

fn artifact_by_kind(
    source: &ProviderSemanticSourceV1,
    kind: GeneratedArtifactKind,
) -> Result<&GeneratedArtifactDeclaration, ProviderArtifactGenerationError> {
    source
        .generated_artifacts
        .iter()
        .find(|artifact| artifact.kind == kind)
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::ProjectionClosureMismatch,
                "generatedArtifacts",
                artifact_kind_label(kind),
            )
        })
}

const fn artifact_kind_label(kind: GeneratedArtifactKind) -> &'static str {
    match kind {
        GeneratedArtifactKind::Lawpack => "lawpack",
        GeneratedArtifactKind::TargetProfile => "target-profile",
        GeneratedArtifactKind::AuthorityFacts => "authority-facts",
        GeneratedArtifactKind::ProviderManifest => "provider-manifest",
        GeneratedArtifactKind::ReviewArtifact => "review-artifact",
        GeneratedArtifactKind::GeneratedArtifactProfile => "generated-artifact-profile",
        GeneratedArtifactKind::GenerationProvenance => "generation-provenance",
        GeneratedArtifactKind::ArtifactSchema => "artifact-schema",
    }
}

fn build_schema(
    source: &ProviderSemanticSourceV1,
    contract_pack: &AdmittedProviderContractPackV1,
) -> Result<GeneratedProviderSchemaV1, ProviderArtifactGenerationError> {
    let declaration = artifact_by_kind(source, GeneratedArtifactKind::ArtifactSchema)?;
    if declaration.role != SCHEMA_ROLE || declaration.coordinate != GENERATED_SCHEMA_DOMAIN {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::ProjectionClosureMismatch,
            &declaration.role,
            GENERATED_SCHEMA_DOMAIN,
        ));
    }
    let mut bytes = contract_pack.schema_bytes().to_vec();
    bytes.extend_from_slice(PROVIDER_SCHEMA_SUFFIX.as_bytes());
    let admitted_schema = std::str::from_utf8(contract_pack.schema_bytes()).map_err(|_| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SchemaGenerationFailed,
            contract_pack.coordinate(),
            "utf-8",
        )
    })?;
    let admitted_rules = flatten_from_str(admitted_schema).map_err(|_| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SchemaGenerationFailed,
            contract_pack.coordinate(),
            "compiled-admitted-cddl",
        )
    })?;
    let generated_rules = flatten_from_str(PROVIDER_SCHEMA_SUFFIX).map_err(|_| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SchemaGenerationFailed,
            &declaration.coordinate,
            "compiled-generated-cddl",
        )
    })?;
    if let Some(rule) = generated_rules
        .keys()
        .find(|rule| admitted_rules.contains_key(*rule))
    {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SchemaGenerationFailed,
            rule,
            "duplicate-cddl-rule",
        ));
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SchemaGenerationFailed,
            &declaration.coordinate,
            "utf-8",
        )
    })?;
    let rules = flatten_from_str(text).map_err(|_| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SchemaGenerationFailed,
            &declaration.coordinate,
            "compiled-cddl",
        )
    })?;
    let context = Arc::new(BasicContext::new(rules));
    for root in required_generated_roots(source) {
        if !context.rules.contains_key(root) {
            return Err(ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::SchemaGenerationFailed,
                root,
                &declaration.coordinate,
            ));
        }
    }
    validate_invocation_root_fixtures(&context)?;
    let content_reference =
        GenerationArtifactReferenceV1::for_bytes(declaration.coordinate.clone(), &bytes)
            .map_err(ProviderArtifactGenerationError::wesley)?;
    Ok(GeneratedProviderSchemaV1 {
        role: declaration.role.clone(),
        coordinate: declaration.coordinate.clone(),
        bytes,
        content_reference,
        context,
    })
}

fn validate_invocation_root_fixtures(
    context: &BasicContext,
) -> Result<(), ProviderArtifactGenerationError> {
    let zero_digest = format!("sha256:{}", "00".repeat(32));
    let reference = resource_reference("echo.fixture@1", &zero_digest)?;
    for (root, value) in [
        (
            "generated-artifact",
            json!({
                "apiVersion": "echo.generated-artifact/v1",
                "profile": reference,
                "operation": "echo.fixture@1.operation",
                "mediaType": "application/cbor",
                "bytes": { "$canonicalBytes": "00" },
            }),
        ),
        (
            "review-payload",
            json!({
                "apiVersion": "echo.review-payload/v1",
                "authoritative": false,
                "subject": reference,
                "mediaType": "application/json",
                "bytes": { "$canonicalBytes": "00" },
            }),
        ),
        (
            "verifier-report",
            json!({
                "apiVersion": "echo.verifier-report/v1",
                "targetIr": reference,
                "outcome": "accepted",
                "diagnosticAbi": reference,
                "diagnosticBytes": { "$canonicalBytes": "" },
            }),
        ),
    ] {
        let canonical = canonical_from_json(value, root)?;
        let bytes = encode_canonical_cbor_v1(&canonical)
            .map_err(|error| ProviderArtifactGenerationError::canonical(root, error.kind()))?;
        validate_schema_bytes(context, root, &bytes)?;
    }
    Ok(())
}

fn required_generated_roots(source: &ProviderSemanticSourceV1) -> Vec<&str> {
    let mut roots = source
        .artifact_resources
        .iter()
        .filter(|resource| resource.provision == ArtifactResourceProvision::Generated)
        .filter_map(|resource| resource_root(&resource.role).ok())
        .collect::<Vec<_>>();
    roots.extend([
        GENERATED_PROFILE_ROOT,
        "lawpack-manifest",
        "target-profile-manifest",
        "authority-facts",
    ]);
    roots.extend(
        source
            .schema_bindings
            .iter()
            .map(|binding| binding.root_rule.as_str()),
    );
    roots.sort_unstable();
    roots.dedup();
    roots
}

fn resource_root(role: &str) -> Result<&'static str, ProviderArtifactGenerationError> {
    let root = match role {
        "resource.conformance-corpus" => "echo-provider-conformance-corpus",
        "resource.lawpack-compatibility" => "echo-provider-lawpack-compatibility",
        "resource.lawpack-exports" => "lawpack-exports",
        "resource.lawpack-target-adapter" => "echo-provider-lawpack-target-adapter",
        "resource.lawpack-verifier" => "echo-provider-lawpack-verifier",
        "resource.target-bundle-profile" => "echo-dpo-bundle",
        "resource.target-cost-algebra" => "echo-dpo-cost",
        "resource.target-footprint-algebra" => "echo-dpo-footprint",
        "resource.target-intrinsics" => "intrinsics-document",
        "resource.target-ir" => "echo-span-ir",
        "resource.target-lowerer-contract" => "echo-dpo-lowerer",
        "resource.target-obstruction-taxonomy" => "echo-dpo-obstructions",
        "resource.target-operation-profiles" => "operation-profiles-document",
        "resource.target-verifier-contract" => "echo-dpo-verifier",
        _ => {
            return Err(ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::ResourceClosureMismatch,
                role,
                "generated-resource-root",
            ));
        }
    };
    Ok(root)
}

fn resource_edict_contract(role: &str) -> Option<&'static str> {
    match role {
        "resource.lawpack-exports" => Some("lawpack-exports"),
        "resource.target-intrinsics" => Some("target-profile-intrinsics"),
        "resource.target-operation-profiles" => Some("target-profile-operation-profiles"),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn make_output(
    role: String,
    coordinate: String,
    schema_contract: String,
    owning_root: impl Into<String>,
    digest_domain: String,
    json_value: JsonValue,
    edict_contract: Option<&str>,
    schema: &GeneratedProviderSchemaV1,
    contract_pack: &AdmittedProviderContractPackV1,
) -> Result<SchemaValidatedCanonicalProviderOutputV1, ProviderArtifactGenerationError> {
    let owning_root = owning_root.into();
    let canonical_value = canonical_from_json(json_value, &role)?;
    let canonical_bytes = encode_canonical_cbor_v1(&canonical_value)
        .map_err(|error| ProviderArtifactGenerationError::canonical(&role, error.kind()))?;
    validate_schema_bytes(&schema.context, &owning_root, &canonical_bytes)?;
    if let Some(contract) = edict_contract {
        contract_pack
            .validate_contract_bytes(contract, &canonical_bytes)
            .map_err(|error| {
                let mut result = ProviderArtifactGenerationError::new(
                    ProviderArtifactGenerationErrorKind::OwningRootRejected,
                    &role,
                    contract,
                );
                result.contract_kind = Some(error.kind());
                result
            })?;
    }
    let domain_framed_digest = digest_canonical_value_v1(&digest_domain, &canonical_value)
        .map_err(|error| {
            let mut result = ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::DigestConstructionFailed,
                &role,
                &digest_domain,
            );
            result.canonical_kind = Some(error.kind());
            result
        })?;
    let content_reference =
        GenerationArtifactReferenceV1::for_bytes(coordinate.clone(), &canonical_bytes)
            .map_err(ProviderArtifactGenerationError::wesley)?;
    Ok(SchemaValidatedCanonicalProviderOutputV1 {
        role,
        coordinate,
        schema_contract,
        owning_root,
        digest_domain,
        canonical_value,
        canonical_bytes,
        domain_framed_digest,
        content_reference,
    })
}

fn validate_schema_bytes(
    context: &BasicContext,
    root: &str,
    bytes: &[u8],
) -> Result<CanonicalValueV1, ProviderArtifactGenerationError> {
    let value = decode_canonical_cbor_v1(bytes).map_err(|error| {
        let mut result = ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::CanonicalEncodingFailed,
            root,
            "edict.canonical-cbor/v1",
        );
        result.canonical_kind = Some(error.kind());
        result
    })?;
    let cbor_value: ciborium::Value = ciborium::from_reader(bytes).map_err(|_| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::CanonicalEncodingFailed,
            root,
            "cbor-value",
        )
    })?;
    let rule = context.rules.get(root).ok_or_else(|| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SchemaGenerationFailed,
            root,
            "owning-root",
        )
    })?;
    validate_cbor(rule, &cbor_value, context).map_err(|_| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::OwningRootRejected,
            root,
            "generated-provider-schema",
        )
    })?;
    Ok(value)
}

fn canonical_from_json(
    value: JsonValue,
    subject: &str,
) -> Result<CanonicalValueV1, ProviderArtifactGenerationError> {
    match value {
        JsonValue::Null => Ok(CanonicalValueV1::Null),
        JsonValue::Bool(value) => Ok(CanonicalValueV1::Bool(value)),
        JsonValue::Number(value) => value
            .as_u64()
            .map(|value| CanonicalValueV1::Integer(i128::from(value)))
            .or_else(|| {
                value
                    .as_i64()
                    .map(|value| CanonicalValueV1::Integer(i128::from(value)))
            })
            .ok_or_else(|| {
                ProviderArtifactGenerationError::new(
                    ProviderArtifactGenerationErrorKind::CanonicalEncodingFailed,
                    subject,
                    "integer-only-json",
                )
            }),
        JsonValue::String(value) => Ok(CanonicalValueV1::Text(value)),
        JsonValue::Array(values) => values
            .into_iter()
            .map(|value| canonical_from_json(value, subject))
            .collect::<Result<Vec<_>, _>>()
            .map(CanonicalValueV1::Array),
        JsonValue::Object(mut entries) => {
            if entries.len() == 1 {
                if let Some(JsonValue::String(bytes)) = entries.remove("$canonicalBytes") {
                    return hex::decode(bytes)
                        .map(CanonicalValueV1::Bytes)
                        .map_err(|_| {
                            ProviderArtifactGenerationError::new(
                                ProviderArtifactGenerationErrorKind::DigestConstructionFailed,
                                subject,
                                "lowercase-hex-bytes",
                            )
                        });
                }
            }
            entries
                .into_iter()
                .map(|(key, value)| {
                    Ok((
                        CanonicalValueV1::Text(key),
                        canonical_from_json(value, subject)?,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(CanonicalValueV1::Map)
        }
    }
}

fn build_target_profile(
    source: &ProviderSemanticSourceV1,
    resources: &[SchemaValidatedCanonicalProviderOutputV1],
    generated_profile: &SchemaValidatedCanonicalProviderOutputV1,
    contract_pack: &AdmittedProviderContractPackV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let target = &source.target_profile_projection;
    if target.generated_artifact_profile_roles != [generated_profile.role.clone()] {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::ProjectionClosureMismatch,
            "targetProfile.generatedArtifactProfiles",
            &generated_profile.role,
        ));
    }
    let mut value = JsonMap::new();
    value.insert("apiVersion".to_owned(), json!("edict.target-profile/v1"));
    value.insert("id".to_owned(), json!(target.id));
    value.insert("version".to_owned(), json!(target.version));
    value.insert(
        "acceptedCoreAbi".to_owned(),
        json!(target.accepted_core_abis),
    );
    for (field, role) in [
        ("intrinsics", target.intrinsics_resource.as_str()),
        (
            "operationProfiles",
            target.operation_profiles_resource.as_str(),
        ),
        (
            "footprintAlgebra",
            target.footprint_algebra_resource.as_str(),
        ),
        ("costAlgebra", target.cost_algebra_resource.as_str()),
        ("targetIr", target.target_ir_resource.as_str()),
        (
            "obstructionTaxonomy",
            target.obstruction_taxonomy_resource.as_str(),
        ),
        ("verifier", target.verifier_resource.as_str()),
        ("lowerer", target.lowerer_resource.as_str()),
        ("sandbox", target.sandbox_resource.as_str()),
        ("fuelModel", target.fuel_model_resource.as_str()),
        ("bundleProfile", target.bundle_profile_resource.as_str()),
        (
            "canonicalEncodingRules",
            target.canonical_encoding_rules_resource.as_str(),
        ),
        ("diagnosticAbi", target.diagnostic_abi_resource.as_str()),
        (
            "deterministicExecution",
            target.deterministic_execution_resource.as_str(),
        ),
        (
            "conformanceFixtureCorpus",
            target.conformance_fixture_corpus_resource.as_str(),
        ),
    ] {
        value.insert(
            field.to_owned(),
            resource_reference_for_role(source, resources, contract_pack, role)?,
        );
    }
    value.insert(
        "intrinsicNamespace".to_owned(),
        json!(target.intrinsic_namespace),
    );
    value.insert(
        "generatedArtifactProfiles".to_owned(),
        JsonValue::Array(vec![output_reference(generated_profile)?]),
    );
    if !target.accepted_lawpack_adapter_abis.is_empty() {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
            "targetProfile.acceptedLawpackAdapterAbi",
            "empty-v1-reservation",
        ));
    }
    value.insert(
        "applicationModel".to_owned(),
        json!(target.application_model),
    );
    value.insert("readConsistency".to_owned(), json!(target.read_consistency));
    value.insert("guardEvaluation".to_owned(), json!(target.guard_evaluation));
    value.insert(
        "obstructionRollback".to_owned(),
        json!(target.obstruction_rollback),
    );
    value.insert("multiTarget".to_owned(), json!(target.multi_target));
    value.insert(
        "postconditionSupport".to_owned(),
        json!(target.postcondition_support),
    );
    Ok(JsonValue::Object(value))
}

fn build_lawpack(
    source: &ProviderSemanticSourceV1,
    resources: &[SchemaValidatedCanonicalProviderOutputV1],
    target_profile: &SchemaValidatedCanonicalProviderOutputV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let lawpack = &source.lawpack_projection;
    if !lawpack.dependencies.is_empty() {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
            "lawpack.dependencies",
            "empty-first-closure",
        ));
    }
    let adapters = lawpack
        .target_adapters
        .iter()
        .map(|adapter| {
            if adapter.accepted_target_profile_role != target_profile.role {
                return Err(ProviderArtifactGenerationError::new(
                    ProviderArtifactGenerationErrorKind::ProjectionClosureMismatch,
                    &adapter.accepted_target_profile_role,
                    &target_profile.role,
                ));
            }
            Ok(json!({
                "acceptedTargetProfile": output_reference(target_profile)?,
                "acceptedTargetIr": generated_resource_reference(
                    source,
                    resources,
                    &adapter.accepted_target_ir_resource,
                )?,
                "adapter": generated_resource_reference(
                    source,
                    resources,
                    &adapter.adapter_resource,
                )?,
            }))
        })
        .collect::<Result<Vec<_>, ProviderArtifactGenerationError>>()?;
    let LawpackVerifierDeclaration::Declarative { ruleset_resource } = &lawpack.verifier;
    Ok(json!({
        "apiVersion": "edict.lawpack/v1",
        "id": lawpack.id,
        "version": lawpack.version,
        "acceptedCoreAbi": lawpack.accepted_core_abis,
        "dependencies": [],
        "exports": generated_resource_reference(source, resources, &lawpack.exports_resource)?,
        "targetAdapters": adapters,
        "verifier": {
            "class": "declarative",
            "ruleset": generated_resource_reference(source, resources, ruleset_resource)?,
        },
        "compatibility": generated_resource_reference(
            source,
            resources,
            &lawpack.compatibility_resource,
        )?,
        "conformanceFixtureCorpus": generated_resource_reference(
            source,
            resources,
            &lawpack.conformance_fixture_corpus_resource,
        )?,
    }))
}

fn build_authority_facts(
    source: &ProviderSemanticSourceV1,
    declaration: &GeneratedArtifactDeclaration,
    source_artifact: &SchemaValidatedCanonicalProviderOutputV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let fact_source = declaration.authority_fact_source.as_ref().ok_or_else(|| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
            &declaration.role,
            "authorityFactSource",
        )
    })?;
    if fact_source.coordinate != source_artifact.coordinate {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::ProjectionClosureMismatch,
            &fact_source.coordinate,
            &source_artifact.coordinate,
        ));
    }
    let (kind, operation_profiles, effect_write_classes, budgets) = match fact_source.kind {
        AuthorityFactSourceKind::TargetProfile => (
            "targetProfile",
            target_authority_profiles(source),
            target_effect_write_classes(source)?,
            JsonValue::Object(JsonMap::new()),
        ),
        AuthorityFactSourceKind::Lawpack => (
            "lawpack",
            JsonValue::Object(JsonMap::new()),
            JsonValue::Object(JsonMap::new()),
            lawpack_authority_budgets(source),
        ),
    };
    Ok(json!({
        "apiVersion": "edict.authority-facts/v1",
        "source": {
            "kind": kind,
            "coordinate": fact_source.coordinate,
            "digest": typed_digest(&source_artifact.domain_framed_digest)?,
        },
        "operationProfiles": operation_profiles,
        "effectWriteClasses": effect_write_classes,
        "budgets": budgets,
    }))
}

fn target_authority_profiles(source: &ProviderSemanticSourceV1) -> JsonValue {
    let mut profiles = JsonMap::new();
    for profile in &source.profiles {
        let allowed = profile
            .allowed_write_classes
            .iter()
            .map(|write_class| (write_class.clone(), JsonValue::Null))
            .collect::<JsonMap<_, _>>();
        for source_name in &profile.source_names {
            profiles.insert(
                source_name.clone(),
                json!({
                    "core": profile.identity.coordinate,
                    "allowedWriteClasses": allowed,
                }),
            );
        }
    }
    JsonValue::Object(profiles)
}

fn target_effect_write_classes(
    source: &ProviderSemanticSourceV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let mut write_classes = JsonMap::new();
    for capability in &source.capabilities {
        insert_effect_projection(
            &mut write_classes,
            &capability.effect,
            JsonValue::String(capability.write_class.clone()),
        )?;
    }
    for adapter in &source.direct_adapters {
        let capability = find_capability(source, &adapter.capability)?;
        insert_effect_projection(
            &mut write_classes,
            &adapter.consumes_effect,
            JsonValue::String(capability.write_class.clone()),
        )?;
    }
    Ok(JsonValue::Object(write_classes))
}

fn lawpack_authority_budgets(source: &ProviderSemanticSourceV1) -> JsonValue {
    JsonValue::Object(
        source
            .budgets
            .iter()
            .map(|budget| {
                (
                    budget.identity.coordinate.clone(),
                    json!({
                        "maxSteps": budget.max_steps,
                        "maxAllocatedBytes": budget.max_allocated_bytes,
                        "maxOutputBytes": budget.max_output_bytes,
                    }),
                )
            })
            .collect(),
    )
}

fn resource_reference_for_role(
    source: &ProviderSemanticSourceV1,
    resources: &[SchemaValidatedCanonicalProviderOutputV1],
    contract_pack: &AdmittedProviderContractPackV1,
    role: &str,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let declaration = source
        .artifact_resources
        .iter()
        .find(|resource| resource.role == role)
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::ResourceClosureMismatch,
                role,
                "artifact-resource-declaration",
            )
        })?;
    match declaration.provision {
        ArtifactResourceProvision::Generated => {
            generated_resource_reference(source, resources, role)
        }
        ArtifactResourceProvision::External => {
            let resource = contract_pack
                .resource(&declaration.coordinate)
                .ok_or_else(|| {
                    ProviderArtifactGenerationError::new(
                        ProviderArtifactGenerationErrorKind::ResourceClosureMismatch,
                        role,
                        &declaration.coordinate,
                    )
                })?;
            resource_reference(resource.coordinate(), resource.domain_framed_digest())
        }
    }
}

fn generated_resource_reference(
    source: &ProviderSemanticSourceV1,
    resources: &[SchemaValidatedCanonicalProviderOutputV1],
    role: &str,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let declaration = source
        .artifact_resources
        .iter()
        .find(|resource| resource.role == role)
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::ResourceClosureMismatch,
                role,
                "generated-resource-declaration",
            )
        })?;
    if declaration.provision != ArtifactResourceProvision::Generated {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::ResourceClosureMismatch,
            role,
            "generated-resource",
        ));
    }
    let output = resources
        .iter()
        .find(|resource| resource.role == role)
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::ResourceClosureMismatch,
                role,
                &declaration.coordinate,
            )
        })?;
    output_reference(output)
}

fn output_reference(
    output: &SchemaValidatedCanonicalProviderOutputV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    resource_reference(&output.coordinate, &output.domain_framed_digest)
}

fn resource_reference(
    coordinate: &str,
    digest: &str,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    Ok(json!({
        "id": coordinate,
        "digest": typed_digest(digest)?,
    }))
}

fn typed_digest(digest: &str) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let hex_digest = digest.strip_prefix("sha256:").ok_or_else(|| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::DigestConstructionFailed,
            digest,
            "sha256:<64-lowercase-hex>",
        )
    })?;
    let decoded = hex::decode(hex_digest).map_err(|_| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::DigestConstructionFailed,
            digest,
            "sha256:<64-lowercase-hex>",
        )
    })?;
    if decoded.len() != 32 || hex::encode(&decoded) != hex_digest {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::DigestConstructionFailed,
            digest,
            "sha256:<64-lowercase-hex>",
        ));
    }
    Ok(json!(["sha256", { "$canonicalBytes": hex_digest }]))
}

fn build_resource_value(
    source: &ProviderSemanticSourceV1,
    declaration: &ArtifactResourceDeclaration,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    match declaration.role.as_str() {
        "resource.conformance-corpus" => Ok(json!({
            "apiVersion": "echo.edict-provider.conformance-corpus/v1",
            "class": "declarative",
            "operations": coordinate_set(source.operations.iter().map(|item| &item.identity.coordinate)),
            "capabilities": coordinate_set(source.capabilities.iter().map(|item| &item.identity.coordinate)),
            "semanticEffects": coordinate_set(source.effects.iter().map(|item| &item.identity.coordinate)),
            "cases": [],
        })),
        "resource.lawpack-compatibility" => Ok(json!({
            "apiVersion": "echo.edict-provider.lawpack-compatibility/v1",
            "class": "declarative",
            "acceptedCoreAbi": coordinate_set(source.lawpack_projection.accepted_core_abis.iter()),
            "acceptedTargetProfiles": string_set([format!(
                "{}@{}",
                source.target_profile_projection.id,
                source.target_profile_projection.version
            )]),
            "semanticEffects": coordinate_set(source.effects.iter().map(|item| &item.identity.coordinate)),
        })),
        "resource.lawpack-exports" => build_lawpack_exports(source),
        "resource.lawpack-target-adapter" => build_target_adapter(source, &declaration.role),
        "resource.lawpack-verifier" => Ok(build_lawpack_verifier(source)),
        "resource.target-bundle-profile" => Ok(json!({
            "apiVersion": "echo.dpo.bundle/v1",
            "class": "declarative",
            "applicationModel": source.target_profile_projection.application_model,
            "readConsistency": source.target_profile_projection.read_consistency,
            "operationProfiles": coordinate_set(source.profiles.iter().map(|item| &item.identity.coordinate)),
        })),
        "resource.target-cost-algebra" => Ok(build_cost_algebra(source)),
        "resource.target-footprint-algebra" => Ok(build_footprint_algebra(source)),
        "resource.target-intrinsics" => build_intrinsics(source),
        "resource.target-ir" => Ok(json!({
            "apiVersion": "echo.span-ir/v1",
            "class": "declarative",
            "domain": declaration.coordinate,
            "targetProfile": format!(
                "{}@{}",
                source.target_profile_projection.id,
                source.target_profile_projection.version
            ),
            "capabilities": coordinate_set(source.capabilities.iter().map(|item| &item.identity.coordinate)),
        })),
        "resource.target-lowerer-contract" => build_lowerer_contract(source),
        "resource.target-obstruction-taxonomy" => Ok(build_obstruction_taxonomy(source)),
        "resource.target-operation-profiles" => build_operation_profiles(source),
        "resource.target-verifier-contract" => build_verifier_contract(source),
        _ => Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::ResourceClosureMismatch,
            &declaration.role,
            &declaration.coordinate,
        )),
    }
}

fn coordinate_set<'a>(values: impl Iterator<Item = &'a String>) -> JsonValue {
    string_set(values.cloned())
}

fn string_set(values: impl IntoIterator<Item = String>) -> JsonValue {
    JsonValue::Object(
        values
            .into_iter()
            .map(|value| (value, JsonValue::Null))
            .collect(),
    )
}

fn build_lawpack_exports(
    source: &ProviderSemanticSourceV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let types = source
        .types
        .iter()
        .filter_map(|declaration| {
            declaration.shape.core_type_coordinate().map(|definition| {
                json!({
                    "coordinate": declaration.identity.coordinate,
                    "definition": definition,
                })
            })
        })
        .collect::<Vec<_>>();
    let effects = source
        .effects
        .iter()
        .map(|effect| {
            let input_type = effect.parameter_types.first().ok_or_else(|| {
                ProviderArtifactGenerationError::new(
                    ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                    &effect.identity.coordinate,
                    "one-effect-input-type",
                )
            })?;
            Ok(json!({
                "coordinate": effect.identity.coordinate,
                "typeParameters": [],
                "inputType": input_type,
                "outputType": effect.result_type,
                "executionClass": execution_class(effect.execution_class),
                "effectKindHint": effect_kind(effect.effect_kind_hint),
                "footprintObligation": effect.footprint_obligation,
                "costObligation": effect.cost_obligation,
                "effectFailures": effect_failure_map(effect),
                "guardSupport": effect.guard_support,
            }))
        })
        .collect::<Result<Vec<_>, ProviderArtifactGenerationError>>()?;
    let obstructions = source
        .obstructions
        .iter()
        .map(|obstruction| {
            json!({
                "coordinate": obstruction.identity.coordinate,
                "authorityClass": authority_class(obstruction.authority_class),
                "payloadSchema": obstruction.payload_schema,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "types": types,
        "constants": [],
        "pureFunctions": [],
        "effects": effects,
        "obstructions": obstructions,
        "operationProfiles": {},
    }))
}

fn build_target_adapter(
    source: &ProviderSemanticSourceV1,
    resource_role: &str,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let target_ir_domain = one_target_ir_domain(source)?;
    let adapter = source
        .lawpack_projection
        .target_adapters
        .iter()
        .find(|adapter| adapter.adapter_resource == resource_role)
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                resource_role,
                "lawpack-target-adapter",
            )
        })?;
    Ok(json!({
        "apiVersion": "echo.edict-provider.lawpack-target-adapter/v1",
        "class": "declarative",
        "targetProfile": format!(
            "{}@{}",
            source.target_profile_projection.id,
            source.target_profile_projection.version
        ),
        "targetIrDomain": target_ir_domain,
        "effectImplementations": selected_effect_implementation_map(source, &adapter.effects)?,
    }))
}

fn build_lawpack_verifier(source: &ProviderSemanticSourceV1) -> JsonValue {
    let mut operations = JsonMap::new();
    for operation in &source.operations {
        let mut mappings = JsonMap::new();
        for mapping in &operation.obstruction_mappings {
            mappings.insert(
                mapping.failure.clone(),
                JsonValue::String(mapping.obstruction.clone()),
            );
        }
        operations.insert(
            operation.identity.coordinate.clone(),
            json!({
                "effect": operation.effect,
                "failureMappings": mappings,
            }),
        );
    }
    json!({
        "apiVersion": "echo.edict-provider.lawpack-verifier/v1",
        "class": "declarative",
        "operationObstructions": operations,
    })
}

fn build_cost_algebra(source: &ProviderSemanticSourceV1) -> JsonValue {
    let mut capabilities = JsonMap::new();
    for capability in &source.capabilities {
        capabilities.insert(
            capability.identity.coordinate.clone(),
            json!({
                "effect": capability.effect,
                "costTemplate": capability.cost_template,
                "semanticObligation": capability.semantic_discharge.cost_obligation,
            }),
        );
    }
    json!({
        "apiVersion": "echo.dpo.cost/v1",
        "class": "declarative",
        "capabilities": capabilities,
    })
}

fn build_footprint_algebra(source: &ProviderSemanticSourceV1) -> JsonValue {
    let mut capabilities = JsonMap::new();
    for capability in &source.capabilities {
        capabilities.insert(
            capability.identity.coordinate.clone(),
            json!({
                "effect": capability.effect,
                "footprintTemplate": capability.footprint_template,
                "semanticObligation": capability.semantic_discharge.footprint_obligation,
                "writeClass": capability.write_class,
            }),
        );
    }
    json!({
        "apiVersion": "echo.dpo.footprint/v1",
        "class": "declarative",
        "capabilities": capabilities,
    })
}

fn build_intrinsics(
    source: &ProviderSemanticSourceV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let mut intrinsics = JsonMap::new();
    for capability in &source.capabilities {
        let effect = find_effect(source, &capability.effect)?;
        intrinsics.insert(
            capability.identity.coordinate.clone(),
            json!({
                "intrinsicClass": "effect",
                "typeParameters": [],
                "argumentTypes": effect.parameter_types,
                "returnType": effect.result_type,
                "effectKind": effect_kind(capability.effect_kind),
                "effectFailures": effect_failure_map(effect),
                "guardSupport": capability.guard_support,
                "footprintTemplate": capability.footprint_template,
                "costTemplate": capability.cost_template,
                "writeClass": capability.write_class,
                "canParticipateInAtomicGuard": capability.can_participate_in_atomic_guard,
            }),
        );
    }
    Ok(json!({
        "apiVersion": "edict.target-profile.intrinsics/v1",
        "intrinsics": intrinsics,
    }))
}

fn build_lowerer_contract(
    source: &ProviderSemanticSourceV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    Ok(json!({
        "apiVersion": "echo.dpo.lowerer/v1",
        "class": "declarative",
        "acceptedCoreAbi": coordinate_set(source.target_profile_projection.accepted_core_abis.iter()),
        "outputDomain": one_target_ir_domain(source)?,
        "targetProfile": format!(
            "{}@{}",
            source.target_profile_projection.id,
            source.target_profile_projection.version
        ),
        "effectImplementations": effect_implementation_map(source)?,
        "opticContracts": profile_optic_contracts(source),
    }))
}

fn build_obstruction_taxonomy(source: &ProviderSemanticSourceV1) -> JsonValue {
    let mut effect_failures = JsonMap::new();
    for effect in &source.effects {
        for failure in &effect.failures {
            effect_failures.insert(
                format!("{}.{}", effect.identity.coordinate, failure.key),
                json!({
                    "authorityClass": authority_class(failure.authority_class),
                    "payloadType": failure.payload_type,
                }),
            );
        }
    }
    let domain_obstructions = source
        .obstructions
        .iter()
        .map(|obstruction| {
            (
                obstruction.identity.coordinate.clone(),
                json!({
                    "authorityClass": authority_class(obstruction.authority_class),
                    "payloadSchema": obstruction.payload_schema,
                }),
            )
        })
        .collect::<JsonMap<_, _>>();
    json!({
        "apiVersion": "echo.dpo.obstructions/v1",
        "class": "declarative",
        "effectFailures": effect_failures,
        "domainObstructions": domain_obstructions,
    })
}

fn build_operation_profiles(
    source: &ProviderSemanticSourceV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let mut profiles = JsonMap::new();
    for profile in &source.profiles {
        profiles.insert(
            profile.identity.coordinate.clone(),
            json!({
                "opticTemplate": json_from_serializable(
                    &profile.optic_template,
                    &profile.identity.coordinate,
                )?,
                "effectPredicate": profile.effect_predicate,
            }),
        );
    }
    Ok(json!({
        "apiVersion": "edict.target-profile.operation-profiles/v1",
        "profiles": profiles,
    }))
}

fn build_verifier_contract(
    source: &ProviderSemanticSourceV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    Ok(json!({
        "apiVersion": "echo.dpo.verifier/v1",
        "class": "declarative",
        "targetProfile": format!(
            "{}@{}",
            source.target_profile_projection.id,
            source.target_profile_projection.version
        ),
        "targetIrDomain": one_target_ir_domain(source)?,
        "capabilities": coordinate_set(source.capabilities.iter().map(|item| &item.identity.coordinate)),
        "operationProfiles": coordinate_set(source.profiles.iter().map(|item| &item.identity.coordinate)),
        "opticContracts": profile_optic_contracts(source),
    }))
}

fn build_generated_profile(
    source: &ProviderSemanticSourceV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let mut types = JsonMap::new();
    for declaration in &source.types {
        types.insert(
            declaration.identity.coordinate.clone(),
            json_from_serializable(&declaration.shape, &declaration.identity.coordinate)?,
        );
    }
    let mut operations = JsonMap::new();
    for operation in &source.operations {
        let profile = source
            .profiles
            .iter()
            .find(|profile| profile.identity.coordinate == operation.profile)
            .ok_or_else(|| {
                ProviderArtifactGenerationError::new(
                    ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                    &operation.identity.coordinate,
                    &operation.profile,
                )
            })?;
        let invocation_kind = match profile.optic_template.optic_kind {
            OpticKind::Revelation => "observer",
            OpticKind::AffectReintegration => "mutation",
        };
        let implementation = implementation_projection(operation)?;
        let obstruction_mappings = operation
            .obstruction_mappings
            .iter()
            .map(|mapping| {
                (
                    mapping.failure.clone(),
                    JsonValue::String(mapping.obstruction.clone()),
                )
            })
            .collect::<JsonMap<_, _>>();
        operations.insert(
            operation.identity.coordinate.clone(),
            json!({
                "inputType": operation.input_type,
                "outputType": operation.output_type,
                "effect": operation.effect,
                "operationProfile": operation.profile,
                "opticContract": profile.optic_contract,
                "budget": operation.budget,
                "invocationKind": invocation_kind,
                "implementation": implementation,
                "obstructionMappings": obstruction_mappings,
            }),
        );
    }
    Ok(json!({
        "apiVersion": GENERATED_PROFILE_DOMAIN,
        "targetProfile": format!(
            "{}@{}",
            source.target_profile_projection.id,
            source.target_profile_projection.version
        ),
        "types": types,
        "operations": operations,
    }))
}

fn implementation_projection(
    operation: &crate::provider_semantics::ProviderOperationDeclaration,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let serialized =
        json_from_serializable(&operation.implementation, &operation.identity.coordinate)?;
    let object = serialized.as_object().ok_or_else(|| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
            &operation.identity.coordinate,
            "implementation-object",
        )
    })?;
    let kind = object
        .get("kind")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                &operation.identity.coordinate,
                "implementation-kind",
            )
        })?;
    let coordinate_field = match kind {
        "native" => "capability",
        "directAdapter" => "adapter",
        _ => {
            return Err(ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                &operation.identity.coordinate,
                "known-implementation-kind",
            ));
        }
    };
    let coordinate = object
        .get(coordinate_field)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                &operation.identity.coordinate,
                coordinate_field,
            )
        })?;
    Ok(json!({ "kind": kind, "coordinate": coordinate }))
}

fn effect_failure_map(effect: &crate::provider_semantics::SemanticEffectDeclaration) -> JsonValue {
    JsonValue::Object(
        effect
            .failures
            .iter()
            .map(|failure| {
                (
                    failure.key.clone(),
                    json!({
                        "authorityClass": authority_class(failure.authority_class),
                        "payloadType": failure.payload_type,
                    }),
                )
            })
            .collect(),
    )
}

fn effect_implementation_map(
    source: &ProviderSemanticSourceV1,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let mut implementations = JsonMap::new();
    for capability in &source.capabilities {
        insert_effect_projection(
            &mut implementations,
            &capability.effect,
            json!({
                "kind": "native",
                "capability": capability.identity.coordinate,
                "writeClass": capability.write_class,
            }),
        )?;
    }
    for adapter in &source.direct_adapters {
        let capability = find_capability(source, &adapter.capability)?;
        insert_effect_projection(
            &mut implementations,
            &adapter.consumes_effect,
            json!({
                "kind": "directAdapter",
                "adapter": adapter.identity.coordinate,
                "capability": capability.identity.coordinate,
                "writeClass": capability.write_class,
            }),
        )?;
    }
    Ok(JsonValue::Object(implementations))
}

fn selected_effect_implementation_map(
    source: &ProviderSemanticSourceV1,
    effects: &[String],
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    let JsonValue::Object(all) = effect_implementation_map(source)? else {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
            "effectImplementations",
            "object",
        ));
    };
    let selected = effects
        .iter()
        .map(|effect| {
            all.get(effect).cloned().map_or_else(
                || {
                    Err(ProviderArtifactGenerationError::new(
                        ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                        "lawpack.targetAdapters.effects",
                        effect,
                    ))
                },
                |implementation| Ok((effect.clone(), implementation)),
            )
        })
        .collect::<Result<JsonMap<_, _>, _>>()?;
    Ok(JsonValue::Object(selected))
}

fn profile_optic_contracts(source: &ProviderSemanticSourceV1) -> JsonValue {
    JsonValue::Object(
        source
            .profiles
            .iter()
            .map(|profile| {
                (
                    profile.identity.coordinate.clone(),
                    JsonValue::String(profile.optic_contract.clone()),
                )
            })
            .collect(),
    )
}

fn insert_effect_projection(
    projections: &mut JsonMap<String, JsonValue>,
    effect: &str,
    projection: JsonValue,
) -> Result<(), ProviderArtifactGenerationError> {
    if projections.insert(effect.to_owned(), projection).is_some() {
        return Err(ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
            "effectImplementations",
            effect,
        ));
    }
    Ok(())
}

fn find_capability<'a>(
    source: &'a ProviderSemanticSourceV1,
    coordinate: &str,
) -> Result<
    &'a crate::provider_semantics::TargetCapabilityDeclaration,
    ProviderArtifactGenerationError,
> {
    source
        .capabilities
        .iter()
        .find(|capability| capability.identity.coordinate == coordinate)
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                coordinate,
                "target-capability",
            )
        })
}

fn one_target_ir_domain(
    source: &ProviderSemanticSourceV1,
) -> Result<&str, ProviderArtifactGenerationError> {
    source
        .capabilities
        .first()
        .map(|capability| capability.target_ir_domain.as_str())
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                "capabilities",
                "targetIrDomain",
            )
        })
}

fn find_effect<'a>(
    source: &'a ProviderSemanticSourceV1,
    coordinate: &str,
) -> Result<&'a crate::provider_semantics::SemanticEffectDeclaration, ProviderArtifactGenerationError>
{
    source
        .effects
        .iter()
        .find(|effect| effect.identity.coordinate == coordinate)
        .ok_or_else(|| {
            ProviderArtifactGenerationError::new(
                ProviderArtifactGenerationErrorKind::SemanticProjectionMismatch,
                coordinate,
                "semantic-effect",
            )
        })
}

fn json_from_serializable(
    value: &impl serde::Serialize,
    subject: &str,
) -> Result<JsonValue, ProviderArtifactGenerationError> {
    serde_json::to_value(value).map_err(|_| {
        ProviderArtifactGenerationError::new(
            ProviderArtifactGenerationErrorKind::CanonicalEncodingFailed,
            subject,
            "semantic-json-projection",
        )
    })
}

const fn authority_class(value: AuthorityClass) -> &'static str {
    match value {
        AuthorityClass::DomainMappable => "domainMappable",
        AuthorityClass::ParticipantOwned => "participantOwned",
        AuthorityClass::IntegrityFault => "integrityFault",
        AuthorityClass::ResourceFault => "resourceFault",
        AuthorityClass::InternalFault => "internalFault",
    }
}

const fn execution_class(value: ExecutionClass) -> &'static str {
    match value {
        ExecutionClass::ProofOnly => "proofOnly",
        ExecutionClass::Runtime => "runtime",
    }
}

const fn effect_kind(value: EffectKindHint) -> &'static str {
    match value {
        EffectKindHint::Read => "read",
        EffectKindHint::Create => "create",
        EffectKindHint::Ensure => "ensure",
        EffectKindHint::Replace => "replace",
        EffectKindHint::Delete => "delete",
        EffectKindHint::Append => "append",
        EffectKindHint::Reduce => "reduce",
        EffectKindHint::SemanticEmit => "semantic.emit",
        EffectKindHint::Custom => "custom",
    }
}
