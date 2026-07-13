// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Strict, deterministic semantic-source validation for the Echo Edict provider.
//!
//! This module is deliberately separate from the additive, tolerant Wesley IR
//! compatibility model. Provider generation must fail closed when semantic
//! declarations conflict or reference facts outside the explicit source graph.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

/// Semantic-source API accepted by [`parse_provider_semantic_source_v1`].
pub const ECHO_PROVIDER_SEMANTIC_SOURCE_API_V1: &str = "echo.edict-provider-semantics/v1";

/// One strict Echo-owned semantic source for provider generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderSemanticSourceV1 {
    /// Exact semantic-source API version.
    pub api_version: String,
    /// Stable identity of these semantic source bytes.
    pub coordinate: String,
    /// Explicit artifacts that own declarations in this source graph.
    pub authority_sources: Vec<AuthoritySourceDeclaration>,
    /// Provider-facing type catalog.
    pub types: Vec<SemanticTypeDeclaration>,
    /// Write-class catalog.
    pub write_classes: Vec<NamedSemanticFact>,
    /// Obstruction taxonomy.
    pub obstructions: Vec<ObstructionDeclaration>,
    /// Semantic effect catalog.
    pub effects: Vec<SemanticEffectDeclaration>,
    /// Operation-profile catalog.
    pub profiles: Vec<OperationProfileDeclaration>,
    /// Compile-time budget catalog.
    pub budgets: Vec<BudgetDeclaration>,
    /// Native target capability catalog.
    pub capabilities: Vec<TargetCapabilityDeclaration>,
    /// Optional one-hop direct adapter catalog.
    pub direct_adapters: Vec<DirectAdapterDeclaration>,
    /// Provider operation catalog.
    pub operations: Vec<ProviderOperationDeclaration>,
    /// Digest-locked resources referenced by generated lawpack/profile manifests.
    pub artifact_resources: Vec<ArtifactResourceDeclaration>,
    /// Complete lawpack-manifest projection for the selected semantic closure.
    pub lawpack_projection: LawpackProjectionDeclaration,
    /// Complete target-profile projection for the selected target.
    pub target_profile_projection: TargetProfileProjectionDeclaration,
    /// Metadata artifacts generated from this source.
    pub generated_artifacts: Vec<GeneratedArtifactDeclaration>,
    /// Package-root provider manifest assembled after components exist.
    pub package_manifest: PackageManifestProjectionDeclaration,
    /// Host-owned input roles whose domains require immutable schema bindings.
    pub invocation_inputs: Vec<InvocationInputDeclaration>,
    /// Runtime component outputs authorized by the provider contract.
    pub invocation_outputs: Vec<InvocationOutputDeclaration>,
    /// Immutable schema-domain bindings for component inputs and outputs.
    pub schema_bindings: Vec<ArtifactSchemaBindingDeclaration>,
}

/// One artifact that owns a family of semantic facts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthoritySourceDeclaration {
    /// Stable authority-source coordinate.
    pub coordinate: String,
    /// Kind of authored source.
    pub kind: AuthoritySourceKind,
    /// Repository-relative artifact locator, optionally with a fragment.
    pub artifact: String,
}

/// Supported authority locations for provider semantic facts.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthoritySourceKind {
    /// Echo-owned GraphQL/Wesley source.
    Graphql,
    /// Echo-owned non-GraphQL semantic declaration.
    EchoSemanticDeclaration,
    /// Echo-owned target metadata.
    TargetMetadata,
    /// Echo runtime implementation.
    RuntimeImplementation,
}

/// Identity and ownership shared by every semantic fact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticFactIdentity {
    /// Stable fact coordinate.
    pub coordinate: String,
    /// Canonical semantic domain for this fact.
    pub domain: String,
    /// Coordinate of the fact's one authoritative source artifact.
    pub authority: String,
}

/// A fact whose complete declaration is its identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamedSemanticFact {
    /// Stable identity and authority.
    pub identity: SemanticFactIdentity,
}

/// Edict authority classes for typed effect failures and obstructions.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthorityClass {
    /// Source may map the failure into a typed domain obstruction.
    DomainMappable,
    /// Participant or admission policy owns the failure.
    ParticipantOwned,
    /// Integrity validation owns the failure.
    IntegrityFault,
    /// Resource accounting owns the failure.
    ResourceFault,
    /// Compiler, host, or component implementation owns the failure.
    InternalFault,
}

/// One typed domain obstruction exported by the provider closure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObstructionDeclaration {
    /// Stable identity and authority.
    pub identity: SemanticFactIdentity,
    /// Authority class governing the obstruction.
    pub authority_class: AuthorityClass,
    /// Bounded Core type coordinate for the obstruction payload.
    pub payload_schema: String,
}

/// One provider-facing type declaration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticTypeDeclaration {
    /// Stable identity and authority.
    pub identity: SemanticFactIdentity,
    /// Bounded type shape used by the first provider fixture.
    pub shape: SemanticTypeShape,
}

/// Bounded provider-facing type shapes supported by the v1 source.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SemanticTypeShape {
    /// Echo-owned alias of one bounded Edict Core string type.
    CoreStringAlias {
        /// Maximum number of Unicode scalar values selected by the alias.
        max_scalar_values: u64,
        /// Edict Core string canonicalization selected by the alias.
        canonical: CoreStringCanonicalization,
    },
    /// Record whose fields form a name-keyed set.
    Record {
        /// Record fields.
        fields: Vec<SemanticFieldDeclaration>,
    },
}

impl SemanticTypeShape {
    /// Returns the canonical Edict Core type coordinate for an external alias.
    #[must_use]
    pub fn core_type_coordinate(&self) -> Option<String> {
        match self {
            Self::CoreStringAlias {
                max_scalar_values,
                canonical,
            } => Some(format!(
                "String<max={max_scalar_values},canonical={}>",
                canonical.as_str()
            )),
            Self::Record { .. } => None,
        }
    }

    /// Checks a raw Edict Core string value against this alias's scalar bound.
    ///
    /// Returns `None` for non-string shapes. The alpha source only admits
    /// `raw-utf8`, so no normalization step is required before counting.
    #[must_use]
    pub fn accepts_raw_string(&self, value: &str) -> Option<bool> {
        match self {
            Self::CoreStringAlias {
                max_scalar_values,
                canonical: CoreStringCanonicalization::RawUtf8,
            } => Some(
                u64::try_from(value.chars().count()).is_ok_and(|count| count <= *max_scalar_values),
            ),
            Self::Record { .. } => None,
        }
    }
}

/// Edict Core canonicalization policies admitted by the alpha source.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CoreStringCanonicalization {
    /// Preserve the exact Unicode scalar sequence without normalization.
    #[serde(rename = "raw-utf8")]
    RawUtf8,
}

impl CoreStringCanonicalization {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RawUtf8 => "raw-utf8",
        }
    }
}

/// One field in a provider-facing record type.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticFieldDeclaration {
    /// Field name.
    pub name: String,
    /// Referenced semantic type coordinate.
    #[serde(rename = "type")]
    pub type_coordinate: String,
}

/// One semantic effect and its target-independent obligations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticEffectDeclaration {
    /// Stable identity and authority.
    pub identity: SemanticFactIdentity,
    /// Ordered parameter type coordinates.
    pub parameter_types: Vec<String>,
    /// Result type coordinate.
    pub result_type: String,
    /// Whether the effect is proof-only or touches runtime execution.
    pub execution_class: ExecutionClass,
    /// Advisory effect kind exported through the lawpack ABI.
    pub effect_kind_hint: EffectKindHint,
    /// Required guard kinds, treated as a set.
    pub guard_kinds: Vec<String>,
    /// Failure-key to obstruction mappings, treated as a key set.
    pub failures: Vec<EffectFailureDeclaration>,
    /// Required footprint obligation.
    pub footprint_obligation: String,
    /// Required cost obligation.
    pub cost_obligation: String,
    /// Whether the semantic effect can participate in a target guard.
    pub guard_support: bool,
}

/// Runtime posture of a semantic effect.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionClass {
    /// The effect establishes a proof without runtime execution.
    ProofOnly,
    /// The effect requires runtime execution.
    Runtime,
}

/// Closed Edict v1 effect-kind vocabulary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EffectKindHint {
    /// Read existing target state.
    Read,
    /// Create a value that must be absent.
    Create,
    /// Ensure a value exists with the requested identity.
    Ensure,
    /// Replace a value that must be present.
    Replace,
    /// Delete a value that must be present.
    Delete,
    /// Append to target state.
    Append,
    /// Reduce target state.
    Reduce,
    /// Emit a portable semantic fact.
    #[serde(rename = "semantic.emit")]
    SemanticEmit,
    /// Target- or lawpack-defined effect semantics.
    Custom,
}

/// One failure key emitted by a semantic effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectFailureDeclaration {
    /// Effect-local stable failure key.
    pub key: String,
    /// Canonical domain for the failure fact.
    pub domain: String,
    /// Coordinate of the failure taxonomy's authoritative source.
    pub authority: String,
    /// Authority class governing source mapping.
    pub authority_class: AuthorityClass,
    /// Bounded Core type coordinate for the failure payload.
    pub payload_type: String,
}

/// One operation profile accepted by the provider target.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationProfileDeclaration {
    /// Stable identity and authority.
    pub identity: SemanticFactIdentity,
    /// Source-level profile names that resolve to this canonical profile.
    pub source_names: Vec<String>,
    /// Allowed write-class coordinates, treated as a set.
    pub allowed_write_classes: Vec<String>,
    /// Supported guard kinds, treated as a set.
    pub guard_kinds: Vec<String>,
    /// Required atomicity posture.
    pub atomicity: String,
    /// Whether the target supports postcondition evaluation.
    pub postcondition_support: bool,
    /// Edict ABI optic template supplied by this operation profile.
    pub optic_template: OpticTemplateDeclaration,
    /// Target-owned operation-mode predicate coordinate.
    pub effect_predicate: String,
    /// Supported optic contract coordinate.
    pub optic_contract: String,
}

/// Edict v1 optic kinds.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OpticKind {
    /// Read-only projection.
    Revelation,
    /// Runtime affect followed by lawful reintegration.
    AffectReintegration,
}

/// Edict v1 optic boundary kinds.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BoundaryKind {
    /// Projection-only boundary.
    Projection,
    /// Runtime-affecting boundary.
    Affect,
}

/// Typed aperture requirement supplied by an operation profile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ApertureRequirementDeclaration {
    /// Exact target footprint ceiling.
    FootprintCeiling {
        /// Footprint ceiling coordinate.
        #[serde(rename = "ref")]
        reference: String,
    },
    /// Abstract semantic footprint obligation.
    AbstractFootprintObligation {
        /// Footprint obligation coordinate.
        #[serde(rename = "ref")]
        reference: String,
    },
}

impl ApertureRequirementDeclaration {
    fn reference(&self) -> &str {
        match self {
            Self::FootprintCeiling { reference }
            | Self::AbstractFootprintObligation { reference } => reference,
        }
    }
}

/// Exact Edict operation-profile optic template.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpticTemplateDeclaration {
    /// Optic classification.
    pub optic_kind: OpticKind,
    /// Authority boundary classification.
    pub boundary_kind: BoundaryKind,
    /// Canonical support policy coordinate.
    pub support_policy: String,
    /// Canonical support-loss disposition coordinate.
    pub loss_disposition: String,
    /// Optional profile-owned basis template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basis_template: Option<String>,
    /// Optional typed aperture requirement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aperture_requirement: Option<ApertureRequirementDeclaration>,
}

/// One exact Core evaluation budget.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetDeclaration {
    /// Stable identity and authority.
    pub identity: SemanticFactIdentity,
    /// Maximum Core evaluation steps.
    pub max_steps: u64,
    /// Maximum allocated bytes.
    pub max_allocated_bytes: u64,
    /// Maximum output bytes.
    pub max_output_bytes: u64,
}

/// One target-native capability exposed to lowerability and lowering.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetCapabilityDeclaration {
    /// Stable target-intrinsic identity and authority.
    pub identity: SemanticFactIdentity,
    /// Semantic effect implemented natively by this capability.
    pub effect: String,
    /// Authoritative target effect kind.
    pub effect_kind: EffectKindHint,
    /// Authoritative target write class.
    pub write_class: String,
    /// Whether the target intrinsic supports guards.
    pub guard_support: bool,
    /// Target footprint template coordinate.
    pub footprint_template: String,
    /// Target cost template coordinate.
    pub cost_template: String,
    /// Explicit discharge of target-independent semantic obligations.
    pub semantic_discharge: SemanticEffectDischargeDeclaration,
    /// Whether the intrinsic can participate in an atomic guard.
    pub can_participate_in_atomic_guard: bool,
    /// Target profile that owns the capability.
    pub target_profile: String,
    /// Inner target IR domain produced by the capability's lowerer.
    pub target_ir_domain: String,
}

/// Explicit mapping from one target capability to its semantic obligations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticEffectDischargeDeclaration {
    /// Advisory effect-kind hint being discharged.
    pub effect_kind_hint: EffectKindHint,
    /// Abstract footprint obligation being discharged.
    pub footprint_obligation: String,
    /// Abstract cost obligation being discharged.
    pub cost_obligation: String,
}

/// One optional one-hop direct adapter declaration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectAdapterDeclaration {
    /// Stable adapter identity and authority.
    pub identity: SemanticFactIdentity,
    /// Semantic effect consumed by the adapter.
    pub consumes_effect: String,
    /// Native capability used by the adapter.
    pub capability: String,
    /// Semantic effects emitted by the adapter, empty for v1 one-hop support.
    pub emits_effects: Vec<String>,
}

/// One operation in the provider semantic closure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderOperationDeclaration {
    /// Stable operation identity and authority.
    pub identity: SemanticFactIdentity,
    /// Input type coordinate.
    pub input_type: String,
    /// Output type coordinate.
    pub output_type: String,
    /// Semantic effect coordinate.
    pub effect: String,
    /// Canonical operation-profile coordinate.
    pub profile: String,
    /// Budget coordinate.
    pub budget: String,
    /// Exhaustive source-level failure-to-obstruction mapping.
    pub obstruction_mappings: Vec<ObstructionMappingDeclaration>,
    /// Native or direct-adapter implementation selection.
    pub implementation: OperationImplementation,
}

/// One source-level effect failure to domain obstruction mapping.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObstructionMappingDeclaration {
    /// Effect-local failure key.
    pub failure: String,
    /// Domain obstruction coordinate.
    pub obstruction: String,
}

/// Implementation selected for one provider operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OperationImplementation {
    /// Direct target-native capability.
    Native {
        /// Target capability coordinate.
        capability: String,
    },
    /// Exactly one declared direct adapter.
    DirectAdapter {
        /// Direct adapter coordinate.
        adapter: String,
    },
}

/// One resource consumed or produced while generating a manifest closure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactResourceDeclaration {
    /// Source-local role used by projection fields.
    pub role: String,
    /// Stable resource coordinate before its digest is known.
    pub coordinate: String,
    /// ABI or Echo-owned schema contract for the resource bytes.
    pub schema_contract: String,
    /// Whether generation emits the resource or requires it as an explicit input.
    pub provision: ArtifactResourceProvision,
}

/// How one digest-locked manifest resource becomes available.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactResourceProvision {
    /// The Echo provider generator emits the resource.
    Generated,
    /// The caller must supply and digest-lock the resource explicitly.
    External,
}

/// Complete lawpack-manifest projection for the first provider closure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LawpackProjectionDeclaration {
    /// Authored source owning lawpack projection facts.
    pub authority: String,
    /// Generated lawpack artifact role.
    pub artifact_role: String,
    /// Lawpack identifier.
    pub id: String,
    /// Lawpack version.
    pub version: String,
    /// Accepted Edict Core ABI coordinates.
    pub accepted_core_abis: Vec<String>,
    /// Digest-locked lawpack dependency artifact roles.
    pub dependencies: Vec<String>,
    /// Resource role containing the lawpack export surface.
    pub exports_resource: String,
    /// Target adapters required by runtime semantic effects.
    pub target_adapters: Vec<LawpackTargetAdapterDeclaration>,
    /// Declarative or executable lawpack verifier contract.
    pub verifier: LawpackVerifierDeclaration,
    /// Resource role containing compatibility claims.
    pub compatibility_resource: String,
    /// Resource role containing the conformance corpus.
    pub conformance_fixture_corpus_resource: String,
}

/// One target adapter emitted into the lawpack manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LawpackTargetAdapterDeclaration {
    /// Generated target-profile artifact role accepted by the adapter.
    pub accepted_target_profile_role: String,
    /// Resource role for the accepted inner Target IR contract.
    pub accepted_target_ir_resource: String,
    /// Resource role for the adapter declaration.
    pub adapter_resource: String,
    /// Runtime semantic effects discharged by this adapter.
    pub effects: Vec<String>,
}

/// Lawpack verifier representation selected by the generator.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "class",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LawpackVerifierDeclaration {
    /// Generated declarative verifier ruleset.
    Declarative {
        /// Resource role containing the ruleset.
        ruleset_resource: String,
    },
}

/// Complete target-profile manifest projection for the first provider closure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetProfileProjectionDeclaration {
    /// Target metadata source owning profile projection facts.
    pub authority: String,
    /// Generated target-profile artifact role.
    pub artifact_role: String,
    /// Target-profile identifier.
    pub id: String,
    /// Target-profile version.
    pub version: String,
    /// Accepted Edict Core ABI coordinates.
    pub accepted_core_abis: Vec<String>,
    /// Namespace owning target intrinsics.
    pub intrinsic_namespace: String,
    /// Resource role for the intrinsic corpus.
    pub intrinsics_resource: String,
    /// Resource role for operation profiles.
    pub operation_profiles_resource: String,
    /// Resource role for footprint algebra.
    pub footprint_algebra_resource: String,
    /// Resource role for cost algebra.
    pub cost_algebra_resource: String,
    /// Resource role for the inner Target IR contract.
    pub target_ir_resource: String,
    /// Resource role for the target obstruction taxonomy.
    pub obstruction_taxonomy_resource: String,
    /// Resource role for the target verifier contract.
    pub verifier_resource: String,
    /// Resource role for the target lowerer contract.
    pub lowerer_resource: String,
    /// Explicit sandbox contract resource role.
    pub sandbox_resource: String,
    /// Explicit fuel-model resource role.
    pub fuel_model_resource: String,
    /// Resource role for contract-bundle policy.
    pub bundle_profile_resource: String,
    /// Generated-artifact-profile package roles.
    pub generated_artifact_profile_roles: Vec<String>,
    /// Resource role for canonical encoding rules.
    pub canonical_encoding_rules_resource: String,
    /// Reserved lawpack adapter ABI list; empty in Edict v1.
    pub accepted_lawpack_adapter_abis: Vec<String>,
    /// Resource role for the diagnostic ABI.
    pub diagnostic_abi_resource: String,
    /// Atomic application doctrine.
    pub application_model: String,
    /// Read-consistency doctrine.
    pub read_consistency: String,
    /// Guard-evaluation doctrine.
    pub guard_evaluation: String,
    /// Obstruction rollback doctrine.
    pub obstruction_rollback: String,
    /// Whether a single application may span targets.
    pub multi_target: bool,
    /// Whether precommit postconditions are supported.
    pub postcondition_support: bool,
    /// Resource role for deterministic execution policy.
    pub deterministic_execution_resource: String,
    /// Resource role for the target conformance corpus.
    pub conformance_fixture_corpus_resource: String,
}

/// One metadata artifact generated from the semantic source.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedArtifactDeclaration {
    /// Provider manifest role.
    pub role: String,
    /// Provider artifact kind.
    pub kind: GeneratedArtifactKind,
    /// Stable artifact coordinate before its generated digest is known.
    pub coordinate: String,
    /// Edict ABI or schema contract the generated value must satisfy.
    pub schema_contract: String,
    /// Source manifest projected by an authority-facts document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority_fact_source: Option<AuthorityFactSourceDeclaration>,
    /// Cross-repository owner of a generated document contract, when required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_owner: Option<String>,
}

/// Package-root manifest projection assembled after components exist.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageManifestProjectionDeclaration {
    /// Package-local manifest role.
    pub role: String,
    /// Stable manifest coordinate before its generated digest is known.
    pub coordinate: String,
    /// Edict provider-manifest schema contract.
    pub schema_contract: String,
    /// Exact frozen component world implemented by the provider.
    pub provider_abi: String,
    /// Digest-bound provider resource coordinate selected during assembly.
    pub provider_coordinate: String,
}

/// Source identity bound by one generated authority-facts document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorityFactSourceDeclaration {
    /// Edict authority-facts source class.
    pub kind: AuthorityFactSourceKind,
    /// Coordinate of the generated lawpack or target profile.
    pub coordinate: String,
}

/// Source classes admitted by the Edict authority-facts v1 ABI.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthorityFactSourceKind {
    /// Facts projected from one generated lawpack.
    Lawpack,
    /// Facts projected from one generated target profile.
    TargetProfile,
}

/// Generated metadata artifact kinds used by the provider manifest.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GeneratedArtifactKind {
    /// Edict lawpack.
    Lawpack,
    /// Edict target-profile manifest.
    TargetProfile,
    /// Edict authority-facts document.
    AuthorityFacts,
    /// Edict provider manifest.
    ProviderManifest,
    /// Non-authoritative deterministic generation review.
    ReviewArtifact,
    /// Profile for one provider-generated output family.
    GeneratedArtifactProfile,
    /// Deterministic provider-generation provenance document.
    GenerationProvenance,
    /// Self-contained provider artifact schema.
    ArtifactSchema,
}

/// One host-bound artifact role supplied to a provider component.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvocationInputDeclaration {
    /// Stable invocation input role.
    pub role: String,
    /// Host routing class for the input.
    pub kind: InvocationInputKind,
    /// Canonical artifact digest domain.
    pub domain: String,
    /// Provider manifest schema role validating the domain.
    pub schema_role: String,
}

/// Input classes represented by the first lowerer/verifier contract.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InvocationInputKind {
    /// Canonical Edict Core module.
    Core,
    /// Generated target-profile manifest.
    TargetProfile,
    /// Generated lawpack manifest.
    Lawpack,
    /// One source-partitioned authority-facts document.
    AuthorityFacts,
    /// Compiler-produced lowerability requirements.
    LowerabilityFacts,
    /// Target IR supplied to the verifier.
    TargetIr,
}

/// One output role that a provider component may return.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvocationOutputDeclaration {
    /// Requested provider output role.
    pub role: String,
    /// WIT output authority kind.
    pub kind: InvocationOutputKind,
    /// Canonical returned artifact domain.
    pub domain: String,
    /// Provider manifest schema role that validates this domain.
    pub schema_role: String,
}

/// Provider WIT output kinds represented by the first source closure.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InvocationOutputKind {
    /// Canonical target IR.
    TargetIr,
    /// Generated provider-owned auxiliary artifact.
    GeneratedArtifact,
    /// Non-authoritative review projection.
    ReviewPayload,
    /// Semantic verifier report.
    VerifierReport,
}

/// One provider manifest schema-domain binding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactSchemaBindingDeclaration {
    /// Canonical artifact domain.
    pub domain: String,
    /// Generated artifact role containing the schema.
    pub schema_role: String,
    /// Schema format accepted by the Edict provider host.
    pub format: ArtifactSchemaFormat,
    /// Root rule within the self-contained schema artifact.
    pub root_rule: String,
}

/// Schema formats admitted by the Echo provider source.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactSchemaFormat {
    /// Edict's self-contained CDDL v1 provider schema format.
    SelfContainedCddlV1,
}

/// Opaque proof that a semantic source is internally complete and normalized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedProviderSemanticSourceV1 {
    source: ProviderSemanticSourceV1,
}

impl ValidatedProviderSemanticSourceV1 {
    /// Returns the normalized source declaration.
    #[must_use]
    pub const fn source(&self) -> &ProviderSemanticSourceV1 {
        &self.source
    }
}

/// Stable failure categories returned by semantic-source parsing and validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderSemanticSourceErrorKind {
    /// JSON did not match the strict v1 source shape.
    MalformedDocument,
    /// The source named an unsupported API version.
    UnsupportedApiVersion,
    /// A required coordinate, domain, role, or artifact locator was empty.
    MissingIdentity,
    /// A semantic coordinate was declared more than once.
    DuplicateCoordinate,
    /// A name-keyed field, failure, role, or domain was declared more than once.
    DuplicateKey,
    /// A fact referenced an absent authority source.
    UnknownAuthority,
    /// A fact family used a domain outside its frozen Echo source contract.
    FactDomainMismatch,
    /// A fact family selected an authority source of the wrong kind.
    FactAuthorityMismatch,
    /// A type reference was absent.
    UnknownType,
    /// A record type graph was recursive and therefore not bounded.
    UnboundedTypeGraph,
    /// An Echo-owned type attempted to claim an Edict Core coordinate.
    CoreTypeAuthorityMismatch,
    /// A write-class reference was absent.
    UnknownWriteClass,
    /// A write-class coordinate was outside Edict's frozen v1 vocabulary.
    InvalidWriteClass,
    /// An obstruction reference was absent.
    UnknownObstruction,
    /// An operation obstruction mapping named an absent effect failure.
    UnknownFailure,
    /// An effect failure key was not an Edict v1 identifier.
    InvalidFailureKey,
    /// An effect reference was absent.
    UnknownEffect,
    /// An operation-profile reference was absent.
    UnknownProfile,
    /// A budget reference was absent.
    UnknownBudget,
    /// A native target capability reference was absent.
    UnknownCapability,
    /// A direct adapter reference was absent.
    UnknownAdapter,
    /// A schema role did not identify a generated artifact schema.
    UnknownSchemaRole,
    /// An invocation input or output domain had no schema binding.
    MissingSchemaBinding,
    /// An invocation value and domain binding selected different schema roles.
    SchemaRoleMismatch,
    /// A historical or relocated artifact was named as active authority.
    NonAuthoritativeSource,
    /// Capabilities under one target profile disagreed on the inner IR domain.
    TargetIrDomainMismatch,
    /// A direct adapter attempted to emit another semantic effect.
    UnsupportedAdapterChain,
    /// More than one native capability or adapter claimed one semantic effect.
    AmbiguousEffectImplementation,
    /// A runtime semantic effect had no native or direct-adapter implementation.
    MissingEffectImplementation,
    /// An operation selected an implementation for a different semantic effect.
    ImplementationEffectMismatch,
    /// An operation profile omitted its effect's write class or guard.
    ProfileEffectMismatch,
    /// A semantic effect cannot project to the frozen Edict v1 ABI shape.
    UnsupportedEffectShape,
    /// A source obstruction map was incomplete or mapped a forbidden failure.
    ObstructionMappingMismatch,
    /// The v1 source cannot translate non-empty failure or obstruction payloads.
    UnsupportedObstructionPayloadMapping,
    /// A generated artifact kind named the wrong Edict schema contract.
    ArtifactSchemaContractMismatch,
    /// A cross-repository artifact contract named the wrong owner.
    ArtifactContractOwnerMismatch,
    /// A provider manifest was included in its own package-member inventory.
    SelfReferentialManifestInventory,
    /// The package-root manifest projection selected the wrong v1 identity.
    ProviderManifestProjectionMismatch,
    /// An authority-facts artifact selected an incompatible source projection.
    AuthorityFactProjectionMismatch,
    /// A lawpack or target-profile projection was incomplete or contradictory.
    ArtifactClosureMismatch,
    /// The pending Wesley generation-provenance contract was misidentified.
    GenerationProvenanceContractMismatch,
    /// A WIT output kind named the wrong canonical artifact domain.
    OutputDomainMismatch,
    /// A provider input kind named the wrong canonical artifact domain.
    InputDomainMismatch,
    /// The first provider invocation-input family was missing or duplicated.
    InvocationInputClosureMismatch,
    /// The first provider invocation-output family was missing or duplicated.
    InvocationOutputClosureMismatch,
    /// A schema binding did not belong to any declared invocation value.
    UnexpectedSchemaBinding,
    /// A WIT output domain named the wrong schema root.
    SchemaRootMismatch,
}

/// Structured semantic-source parsing or validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderSemanticSourceError {
    kind: ProviderSemanticSourceErrorKind,
    subject: String,
    reference: String,
}

impl ProviderSemanticSourceError {
    /// Returns the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> ProviderSemanticSourceErrorKind {
        self.kind
    }

    /// Returns the declaration or field being validated.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the conflicting, unresolved, or contract-expected value.
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    fn new(
        kind: ProviderSemanticSourceErrorKind,
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

impl fmt::Display for ProviderSemanticSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider semantic source {:?}: {} -> {}",
            self.kind, self.subject, self.reference
        )
    }
}

impl std::error::Error for ProviderSemanticSourceError {}

/// Parse, normalize, and validate one v1 Echo provider semantic source.
///
/// This function performs no source discovery or I/O. Callers must supply the
/// exact source bytes whose authority they intend to use.
///
/// # Errors
///
/// Returns a stable structured error when the document is malformed,
/// unsupported, incomplete, contradictory, unbounded, or ABI-incompatible.
pub fn parse_provider_semantic_source_v1(
    json: &str,
) -> Result<ValidatedProviderSemanticSourceV1, ProviderSemanticSourceError> {
    let source = serde_json::from_str::<ProviderSemanticSourceV1>(json).map_err(|_| {
        ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::MalformedDocument,
            "document",
            ECHO_PROVIDER_SEMANTIC_SOURCE_API_V1,
        )
    })?;
    validate_provider_semantic_source_v1(source)
}

/// Normalize and validate one already-decoded v1 semantic source.
///
/// # Errors
///
/// Returns a stable structured error when the source is incomplete,
/// contradictory, or contains dangling references.
pub fn validate_provider_semantic_source_v1(
    mut source: ProviderSemanticSourceV1,
) -> Result<ValidatedProviderSemanticSourceV1, ProviderSemanticSourceError> {
    if source.api_version != ECHO_PROVIDER_SEMANTIC_SOURCE_API_V1 {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::UnsupportedApiVersion,
            "apiVersion",
            source.api_version,
        ));
    }

    normalize_source(&mut source);
    validate_identities(&source)?;
    validate_authorities(&source)?;
    validate_fact_contracts(&source)?;
    validate_type_references(&source)?;
    validate_obstruction_references(&source)?;
    validate_profile_references(&source)?;
    validate_effect_references(&source)?;
    validate_capability_references(&source)?;
    validate_adapter_references(&source)?;
    validate_effect_implementation_closure(&source)?;
    validate_operation_references(&source)?;
    validate_generated_artifact_contracts(&source)?;
    validate_artifact_closure(&source)?;
    validate_invocation_schema_bindings(&source)?;

    Ok(ValidatedProviderSemanticSourceV1 { source })
}

fn normalize_source(source: &mut ProviderSemanticSourceV1) {
    source
        .authority_sources
        .sort_by(|left, right| left.coordinate.cmp(&right.coordinate));
    source
        .types
        .sort_by(|left, right| identity(left).cmp(identity(right)));
    for declaration in &mut source.types {
        if let SemanticTypeShape::Record { fields } = &mut declaration.shape {
            fields.sort_by(|left, right| left.name.cmp(&right.name));
        }
    }
    sort_named_facts(&mut source.write_classes);
    source
        .obstructions
        .sort_by(|left, right| identity(left).cmp(identity(right)));
    source
        .effects
        .sort_by(|left, right| identity(left).cmp(identity(right)));
    for effect in &mut source.effects {
        effect.guard_kinds.sort();
        effect
            .failures
            .sort_by(|left, right| left.key.cmp(&right.key));
    }
    source
        .profiles
        .sort_by(|left, right| identity(left).cmp(identity(right)));
    for profile in &mut source.profiles {
        profile.source_names.sort();
        profile.allowed_write_classes.sort();
        profile.guard_kinds.sort();
    }
    source
        .budgets
        .sort_by(|left, right| identity(left).cmp(identity(right)));
    source
        .capabilities
        .sort_by(|left, right| identity(left).cmp(identity(right)));
    source
        .direct_adapters
        .sort_by(|left, right| identity(left).cmp(identity(right)));
    for adapter in &mut source.direct_adapters {
        adapter.emits_effects.sort();
    }
    source
        .operations
        .sort_by(|left, right| identity(left).cmp(identity(right)));
    for operation in &mut source.operations {
        operation
            .obstruction_mappings
            .sort_by(|left, right| left.failure.cmp(&right.failure));
    }
    source
        .artifact_resources
        .sort_by(|left, right| left.role.cmp(&right.role));
    source.lawpack_projection.accepted_core_abis.sort();
    source.lawpack_projection.dependencies.sort();
    for adapter in &mut source.lawpack_projection.target_adapters {
        adapter.effects.sort();
    }
    source
        .lawpack_projection
        .target_adapters
        .sort_by(|left, right| {
            left.accepted_target_profile_role
                .cmp(&right.accepted_target_profile_role)
                .then(
                    left.accepted_target_ir_resource
                        .cmp(&right.accepted_target_ir_resource),
                )
                .then(left.adapter_resource.cmp(&right.adapter_resource))
                .then(left.effects.cmp(&right.effects))
        });
    source.target_profile_projection.accepted_core_abis.sort();
    source
        .target_profile_projection
        .generated_artifact_profile_roles
        .sort();
    source
        .target_profile_projection
        .accepted_lawpack_adapter_abis
        .sort();
    source
        .generated_artifacts
        .sort_by(|left, right| left.role.cmp(&right.role));
    source
        .invocation_inputs
        .sort_by(|left, right| left.role.cmp(&right.role));
    source
        .invocation_outputs
        .sort_by(|left, right| left.role.cmp(&right.role));
    source
        .schema_bindings
        .sort_by(|left, right| left.domain.cmp(&right.domain));
}

fn sort_named_facts(facts: &mut [NamedSemanticFact]) {
    facts.sort_by(|left, right| identity(left).cmp(identity(right)));
}

trait HasIdentity {
    fn identity(&self) -> &SemanticFactIdentity;
}

macro_rules! has_identity {
    ($($type:ty),+ $(,)?) => {
        $(
            impl HasIdentity for $type {
                fn identity(&self) -> &SemanticFactIdentity {
                    &self.identity
                }
            }
        )+
    };
}

has_identity!(
    SemanticTypeDeclaration,
    NamedSemanticFact,
    ObstructionDeclaration,
    SemanticEffectDeclaration,
    OperationProfileDeclaration,
    BudgetDeclaration,
    TargetCapabilityDeclaration,
    DirectAdapterDeclaration,
    ProviderOperationDeclaration,
);

fn identity(value: &impl HasIdentity) -> &str {
    &value.identity().coordinate
}

fn validate_identities(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    require_nonempty("source.coordinate", &source.coordinate)?;

    let mut coordinates = vec![("source", source.coordinate.as_str())];
    for authority in &source.authority_sources {
        require_nonempty("authoritySources.coordinate", &authority.coordinate)?;
        require_nonempty("authoritySources.artifact", &authority.artifact)?;
        coordinates.push(("authoritySources", authority.coordinate.as_str()));
    }
    collect_fact_identities("types", &source.types, &mut coordinates)?;
    collect_fact_identities("writeClasses", &source.write_classes, &mut coordinates)?;
    collect_fact_identities("obstructions", &source.obstructions, &mut coordinates)?;
    collect_fact_identities("effects", &source.effects, &mut coordinates)?;
    collect_fact_identities("profiles", &source.profiles, &mut coordinates)?;
    collect_fact_identities("budgets", &source.budgets, &mut coordinates)?;
    collect_fact_identities("capabilities", &source.capabilities, &mut coordinates)?;
    collect_fact_identities("directAdapters", &source.direct_adapters, &mut coordinates)?;
    collect_fact_identities("operations", &source.operations, &mut coordinates)?;
    for resource in &source.artifact_resources {
        require_nonempty("artifactResources.role", &resource.role)?;
        require_nonempty("artifactResources.coordinate", &resource.coordinate)?;
        require_nonempty(
            "artifactResources.schemaContract",
            &resource.schema_contract,
        )?;
        coordinates.push(("artifactResources", resource.coordinate.as_str()));
    }
    for artifact in &source.generated_artifacts {
        validate_artifact_identity("generatedArtifacts", artifact)?;
        coordinates.push(("generatedArtifacts", artifact.coordinate.as_str()));
    }
    validate_package_manifest_identity(&source.package_manifest)?;
    coordinates.push(("packageManifest", &source.package_manifest.coordinate));
    coordinates.push((
        "packageManifest.providerCoordinate",
        &source.package_manifest.provider_coordinate,
    ));
    coordinates.sort_by(|left, right| left.1.cmp(right.1).then(left.0.cmp(right.0)));
    if let Some(window) = coordinates
        .windows(2)
        .find(|window| window[0].1 == window[1].1)
    {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::DuplicateCoordinate,
            window[1].0,
            window[1].1,
        ));
    }

    ensure_unique_keys(
        "generatedArtifacts.role",
        source.generated_artifacts.iter().map(|item| &item.role),
    )?;
    ensure_unique_keys(
        "artifactResources.role",
        source.artifact_resources.iter().map(|item| &item.role),
    )?;
    ensure_unique_keys(
        "artifactResources.coordinate",
        source
            .artifact_resources
            .iter()
            .map(|item| &item.coordinate),
    )?;
    let mut package_roles = source
        .generated_artifacts
        .iter()
        .map(|item| &item.role)
        .collect::<Vec<_>>();
    package_roles.push(&source.package_manifest.role);
    ensure_unique_keys("packageRoles", package_roles.into_iter())?;
    ensure_unique_keys(
        "invocationInputs.role",
        source.invocation_inputs.iter().map(|item| &item.role),
    )?;
    ensure_unique_keys(
        "invocationOutputs.role",
        source.invocation_outputs.iter().map(|item| &item.role),
    )?;
    ensure_unique_keys(
        "schemaBindings.domain",
        source.schema_bindings.iter().map(|item| &item.domain),
    )?;

    for obstruction in &source.obstructions {
        require_nonempty("obstructions.payloadSchema", &obstruction.payload_schema)?;
    }
    for effect in &source.effects {
        require_nonempty("effects.resultType", &effect.result_type)?;
        require_nonempty("effects.footprintObligation", &effect.footprint_obligation)?;
        require_nonempty("effects.costObligation", &effect.cost_obligation)?;
        for failure in &effect.failures {
            require_nonempty("effects.failures.key", &failure.key)?;
            require_nonempty("effects.failures.domain", &failure.domain)?;
            require_nonempty("effects.failures.authority", &failure.authority)?;
            require_nonempty("effects.failures.payloadType", &failure.payload_type)?;
        }
    }
    for profile in &source.profiles {
        require_nonempty("profiles.atomicity", &profile.atomicity)?;
        require_nonempty("profiles.effectPredicate", &profile.effect_predicate)?;
        require_nonempty("profiles.opticContract", &profile.optic_contract)?;
        require_nonempty(
            "profiles.opticTemplate.supportPolicy",
            &profile.optic_template.support_policy,
        )?;
        require_nonempty(
            "profiles.opticTemplate.lossDisposition",
            &profile.optic_template.loss_disposition,
        )?;
        if let Some(basis) = &profile.optic_template.basis_template {
            require_nonempty("profiles.opticTemplate.basisTemplate", basis)?;
        }
        if let Some(aperture) = &profile.optic_template.aperture_requirement {
            require_nonempty(
                "profiles.opticTemplate.apertureRequirement.ref",
                aperture.reference(),
            )?;
        }
    }
    for capability in &source.capabilities {
        require_nonempty("capabilities.effect", &capability.effect)?;
        require_nonempty("capabilities.writeClass", &capability.write_class)?;
        require_nonempty(
            "capabilities.footprintTemplate",
            &capability.footprint_template,
        )?;
        require_nonempty("capabilities.costTemplate", &capability.cost_template)?;
        require_nonempty(
            "capabilities.semanticDischarge.footprintObligation",
            &capability.semantic_discharge.footprint_obligation,
        )?;
        require_nonempty(
            "capabilities.semanticDischarge.costObligation",
            &capability.semantic_discharge.cost_obligation,
        )?;
        require_nonempty("capabilities.targetProfile", &capability.target_profile)?;
        require_nonempty("capabilities.targetIrDomain", &capability.target_ir_domain)?;
    }
    for operation in &source.operations {
        for mapping in &operation.obstruction_mappings {
            require_nonempty("operations.obstructionMappings.failure", &mapping.failure)?;
            require_nonempty(
                "operations.obstructionMappings.obstruction",
                &mapping.obstruction,
            )?;
        }
    }

    for input in &source.invocation_inputs {
        require_nonempty("invocationInputs.role", &input.role)?;
        require_nonempty("invocationInputs.domain", &input.domain)?;
        require_nonempty("invocationInputs.schemaRole", &input.schema_role)?;
    }
    for output in &source.invocation_outputs {
        require_nonempty("invocationOutputs.role", &output.role)?;
        require_nonempty("invocationOutputs.domain", &output.domain)?;
        require_nonempty("invocationOutputs.schemaRole", &output.schema_role)?;
    }
    for binding in &source.schema_bindings {
        require_nonempty("schemaBindings.domain", &binding.domain)?;
        require_nonempty("schemaBindings.schemaRole", &binding.schema_role)?;
        require_nonempty("schemaBindings.rootRule", &binding.root_rule)?;
    }
    validate_projection_identities(source)?;
    Ok(())
}

fn validate_artifact_identity(
    subject: &'static str,
    artifact: &GeneratedArtifactDeclaration,
) -> Result<(), ProviderSemanticSourceError> {
    require_nonempty(subject, &artifact.role)?;
    require_nonempty(subject, &artifact.coordinate)?;
    require_nonempty(subject, &artifact.schema_contract)?;
    if let Some(contract_owner) = &artifact.contract_owner {
        require_nonempty(subject, contract_owner)?;
    }
    Ok(())
}

fn validate_package_manifest_identity(
    manifest: &PackageManifestProjectionDeclaration,
) -> Result<(), ProviderSemanticSourceError> {
    require_nonempty("packageManifest.role", &manifest.role)?;
    require_nonempty("packageManifest.coordinate", &manifest.coordinate)?;
    require_nonempty("packageManifest.schemaContract", &manifest.schema_contract)?;
    require_nonempty("packageManifest.providerAbi", &manifest.provider_abi)?;
    require_nonempty(
        "packageManifest.providerCoordinate",
        &manifest.provider_coordinate,
    )
}

fn collect_fact_identities<'a, T: HasIdentity>(
    family: &'static str,
    facts: &'a [T],
    coordinates: &mut Vec<(&'static str, &'a str)>,
) -> Result<(), ProviderSemanticSourceError> {
    for fact in facts {
        let identity = fact.identity();
        require_nonempty(family, &identity.coordinate)?;
        require_nonempty(family, &identity.domain)?;
        require_nonempty(family, &identity.authority)?;
        coordinates.push((family, identity.coordinate.as_str()));
    }
    Ok(())
}

fn require_nonempty(subject: &'static str, value: &str) -> Result<(), ProviderSemanticSourceError> {
    if value.is_empty() {
        Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::MissingIdentity,
            subject,
            value,
        ))
    } else {
        Ok(())
    }
}

fn ensure_unique_keys<'a>(
    subject: &'static str,
    values: impl Iterator<Item = &'a String>,
) -> Result<(), ProviderSemanticSourceError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.as_str()) {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::DuplicateKey,
                subject,
                value,
            ));
        }
    }
    Ok(())
}

fn validate_projection_identities(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let lawpack = &source.lawpack_projection;
    for (field, value) in [
        ("lawpackProjection.authority", lawpack.authority.as_str()),
        (
            "lawpackProjection.artifactRole",
            lawpack.artifact_role.as_str(),
        ),
        ("lawpackProjection.id", lawpack.id.as_str()),
        ("lawpackProjection.version", lawpack.version.as_str()),
        (
            "lawpackProjection.exportsResource",
            lawpack.exports_resource.as_str(),
        ),
        (
            "lawpackProjection.compatibilityResource",
            lawpack.compatibility_resource.as_str(),
        ),
        (
            "lawpackProjection.conformanceFixtureCorpusResource",
            lawpack.conformance_fixture_corpus_resource.as_str(),
        ),
    ] {
        require_nonempty(field, value)?;
    }
    ensure_unique_keys(
        "lawpackProjection.acceptedCoreAbis",
        lawpack.accepted_core_abis.iter(),
    )?;
    ensure_unique_keys(
        "lawpackProjection.dependencies",
        lawpack.dependencies.iter(),
    )?;
    ensure_unique_keys(
        "lawpackProjection.targetAdapters.acceptedTargetProfileRole",
        lawpack
            .target_adapters
            .iter()
            .map(|adapter| &adapter.accepted_target_profile_role),
    )?;
    for abi in &lawpack.accepted_core_abis {
        require_nonempty("lawpackProjection.acceptedCoreAbis", abi)?;
    }
    for adapter in &lawpack.target_adapters {
        for (field, value) in [
            (
                "lawpackProjection.targetAdapters.acceptedTargetProfileRole",
                adapter.accepted_target_profile_role.as_str(),
            ),
            (
                "lawpackProjection.targetAdapters.acceptedTargetIrResource",
                adapter.accepted_target_ir_resource.as_str(),
            ),
            (
                "lawpackProjection.targetAdapters.adapterResource",
                adapter.adapter_resource.as_str(),
            ),
        ] {
            require_nonempty(field, value)?;
        }
        ensure_unique_keys(
            "lawpackProjection.targetAdapters.effects",
            adapter.effects.iter(),
        )?;
    }
    let LawpackVerifierDeclaration::Declarative { ruleset_resource } = &lawpack.verifier;
    require_nonempty(
        "lawpackProjection.verifier.rulesetResource",
        ruleset_resource,
    )?;

    let target = &source.target_profile_projection;
    for (field, value) in [
        (
            "targetProfileProjection.authority",
            target.authority.as_str(),
        ),
        (
            "targetProfileProjection.artifactRole",
            target.artifact_role.as_str(),
        ),
        ("targetProfileProjection.id", target.id.as_str()),
        ("targetProfileProjection.version", target.version.as_str()),
        (
            "targetProfileProjection.intrinsicNamespace",
            target.intrinsic_namespace.as_str(),
        ),
        (
            "targetProfileProjection.intrinsicsResource",
            target.intrinsics_resource.as_str(),
        ),
        (
            "targetProfileProjection.operationProfilesResource",
            target.operation_profiles_resource.as_str(),
        ),
        (
            "targetProfileProjection.footprintAlgebraResource",
            target.footprint_algebra_resource.as_str(),
        ),
        (
            "targetProfileProjection.costAlgebraResource",
            target.cost_algebra_resource.as_str(),
        ),
        (
            "targetProfileProjection.targetIrResource",
            target.target_ir_resource.as_str(),
        ),
        (
            "targetProfileProjection.obstructionTaxonomyResource",
            target.obstruction_taxonomy_resource.as_str(),
        ),
        (
            "targetProfileProjection.verifierResource",
            target.verifier_resource.as_str(),
        ),
        (
            "targetProfileProjection.lowererResource",
            target.lowerer_resource.as_str(),
        ),
        (
            "targetProfileProjection.sandboxResource",
            target.sandbox_resource.as_str(),
        ),
        (
            "targetProfileProjection.fuelModelResource",
            target.fuel_model_resource.as_str(),
        ),
        (
            "targetProfileProjection.bundleProfileResource",
            target.bundle_profile_resource.as_str(),
        ),
        (
            "targetProfileProjection.canonicalEncodingRulesResource",
            target.canonical_encoding_rules_resource.as_str(),
        ),
        (
            "targetProfileProjection.diagnosticAbiResource",
            target.diagnostic_abi_resource.as_str(),
        ),
        (
            "targetProfileProjection.applicationModel",
            target.application_model.as_str(),
        ),
        (
            "targetProfileProjection.readConsistency",
            target.read_consistency.as_str(),
        ),
        (
            "targetProfileProjection.guardEvaluation",
            target.guard_evaluation.as_str(),
        ),
        (
            "targetProfileProjection.obstructionRollback",
            target.obstruction_rollback.as_str(),
        ),
        (
            "targetProfileProjection.deterministicExecutionResource",
            target.deterministic_execution_resource.as_str(),
        ),
        (
            "targetProfileProjection.conformanceFixtureCorpusResource",
            target.conformance_fixture_corpus_resource.as_str(),
        ),
    ] {
        require_nonempty(field, value)?;
    }
    ensure_unique_keys(
        "targetProfileProjection.acceptedCoreAbis",
        target.accepted_core_abis.iter(),
    )?;
    ensure_unique_keys(
        "targetProfileProjection.generatedArtifactProfileRoles",
        target.generated_artifact_profile_roles.iter(),
    )?;
    ensure_unique_keys(
        "targetProfileProjection.acceptedLawpackAdapterAbis",
        target.accepted_lawpack_adapter_abis.iter(),
    )
}

fn validate_authorities(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    for authority in &source.authority_sources {
        if !is_authoritative_locator(&authority.artifact) {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::NonAuthoritativeSource,
                &authority.coordinate,
                &authority.artifact,
            ));
        }
    }
    let authorities = source
        .authority_sources
        .iter()
        .map(|authority| authority.coordinate.as_str())
        .collect::<BTreeSet<_>>();

    for identity in all_fact_identities(source) {
        require_reference(
            &authorities,
            &identity.authority,
            ProviderSemanticSourceErrorKind::UnknownAuthority,
            &identity.coordinate,
        )?;
    }
    for effect in &source.effects {
        for failure in &effect.failures {
            require_reference(
                &authorities,
                &failure.authority,
                ProviderSemanticSourceErrorKind::UnknownAuthority,
                &effect.identity.coordinate,
            )?;
        }
    }
    for (subject, authority) in [
        ("lawpackProjection", &source.lawpack_projection.authority),
        (
            "targetProfileProjection",
            &source.target_profile_projection.authority,
        ),
    ] {
        require_reference(
            &authorities,
            authority,
            ProviderSemanticSourceErrorKind::UnknownAuthority,
            subject,
        )?;
    }
    Ok(())
}

fn is_authoritative_locator(locator: &str) -> bool {
    let mut fragments = locator.split('#');
    let path = fragments.next().unwrap_or_default();
    if path.is_empty()
        || fragments.clone().count() > 1
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
    {
        return false;
    }
    let forbidden = [
        "",
        ".",
        "..",
        "build",
        "dist",
        "generated",
        "out",
        "target",
        "wesley-relocated",
    ];
    !path
        .split('/')
        .any(|component| forbidden.contains(&component))
}

fn validate_fact_contracts(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let authorities = source
        .authority_sources
        .iter()
        .map(|authority| (authority.coordinate.as_str(), authority.kind))
        .collect::<BTreeMap<_, _>>();
    validate_fact_family(
        &source.types,
        "echo.edict-provider/value/v1",
        &[
            AuthoritySourceKind::Graphql,
            AuthoritySourceKind::EchoSemanticDeclaration,
        ],
        &authorities,
    )?;
    validate_fact_family(
        &source.write_classes,
        "echo.edict-provider/write-class/v1",
        &[AuthoritySourceKind::TargetMetadata],
        &authorities,
    )?;
    validate_fact_family(
        &source.obstructions,
        "echo.edict-provider/obstruction/v1",
        &[AuthoritySourceKind::EchoSemanticDeclaration],
        &authorities,
    )?;
    validate_fact_family(
        &source.effects,
        "echo.edict-provider/semantic-effect/v1",
        &[AuthoritySourceKind::EchoSemanticDeclaration],
        &authorities,
    )?;
    validate_fact_family(
        &source.profiles,
        "echo.edict-provider/operation-profile/v1",
        &[AuthoritySourceKind::TargetMetadata],
        &authorities,
    )?;
    validate_fact_family(
        &source.budgets,
        "echo.edict-provider/core-budget/v1",
        &[AuthoritySourceKind::EchoSemanticDeclaration],
        &authorities,
    )?;
    validate_fact_family(
        &source.capabilities,
        "echo.edict-provider/target-intrinsic/v1",
        &[AuthoritySourceKind::TargetMetadata],
        &authorities,
    )?;
    validate_fact_family(
        &source.direct_adapters,
        "echo.edict-provider/direct-adapter/v1",
        &[AuthoritySourceKind::TargetMetadata],
        &authorities,
    )?;
    validate_fact_family(
        &source.operations,
        "echo.edict-provider/operation/v1",
        &[AuthoritySourceKind::EchoSemanticDeclaration],
        &authorities,
    )?;
    for effect in &source.effects {
        for failure in &effect.failures {
            if failure.domain != "echo.edict-provider/effect-failure/v1" {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::FactDomainMismatch,
                    &effect.identity.coordinate,
                    &failure.domain,
                ));
            }
            let Some(kind) = authorities.get(failure.authority.as_str()).copied() else {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::UnknownAuthority,
                    &effect.identity.coordinate,
                    &failure.authority,
                ));
            };
            if kind != AuthoritySourceKind::TargetMetadata {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::FactAuthorityMismatch,
                    &effect.identity.coordinate,
                    &failure.authority,
                ));
            }
        }
    }
    validate_projection_authority(
        "lawpackProjection",
        &source.lawpack_projection.authority,
        AuthoritySourceKind::EchoSemanticDeclaration,
        &authorities,
    )?;
    validate_projection_authority(
        "targetProfileProjection",
        &source.target_profile_projection.authority,
        AuthoritySourceKind::TargetMetadata,
        &authorities,
    )?;
    Ok(())
}

fn validate_projection_authority(
    subject: &str,
    authority: &str,
    expected: AuthoritySourceKind,
    authorities: &BTreeMap<&str, AuthoritySourceKind>,
) -> Result<(), ProviderSemanticSourceError> {
    let Some(actual) = authorities.get(authority).copied() else {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::UnknownAuthority,
            subject,
            authority,
        ));
    };
    if actual != expected {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::FactAuthorityMismatch,
            subject,
            authority,
        ));
    }
    Ok(())
}

fn validate_fact_family<T: HasIdentity>(
    facts: &[T],
    expected_domain: &str,
    allowed_authorities: &[AuthoritySourceKind],
    authorities: &BTreeMap<&str, AuthoritySourceKind>,
) -> Result<(), ProviderSemanticSourceError> {
    for fact in facts {
        let identity = fact.identity();
        if identity.domain != expected_domain {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::FactDomainMismatch,
                &identity.coordinate,
                &identity.domain,
            ));
        }
        let Some(kind) = authorities.get(identity.authority.as_str()).copied() else {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnknownAuthority,
                &identity.coordinate,
                &identity.authority,
            ));
        };
        if !allowed_authorities.contains(&kind) {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::FactAuthorityMismatch,
                &identity.coordinate,
                &identity.authority,
            ));
        }
    }
    Ok(())
}

fn all_fact_identities(source: &ProviderSemanticSourceV1) -> Vec<&SemanticFactIdentity> {
    let mut identities = Vec::new();
    append_identities(&mut identities, &source.types);
    append_identities(&mut identities, &source.write_classes);
    append_identities(&mut identities, &source.obstructions);
    append_identities(&mut identities, &source.effects);
    append_identities(&mut identities, &source.profiles);
    append_identities(&mut identities, &source.budgets);
    append_identities(&mut identities, &source.capabilities);
    append_identities(&mut identities, &source.direct_adapters);
    append_identities(&mut identities, &source.operations);
    identities
}

fn append_identities<'a, T: HasIdentity>(
    identities: &mut Vec<&'a SemanticFactIdentity>,
    facts: &'a [T],
) {
    identities.extend(facts.iter().map(HasIdentity::identity));
}

fn validate_type_references(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let types = coordinates(&source.types);
    for declaration in &source.types {
        if is_edict_core_coordinate(&declaration.identity.coordinate) {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::CoreTypeAuthorityMismatch,
                &declaration.identity.coordinate,
                "edict.core/v1",
            ));
        }
        if let SemanticTypeShape::Record { fields } = &declaration.shape {
            ensure_unique_keys("types.fields.name", fields.iter().map(|field| &field.name))?;
            for field in fields {
                require_nonempty("types.fields.name", &field.name)?;
                require_reference(
                    &types,
                    &field.type_coordinate,
                    ProviderSemanticSourceErrorKind::UnknownType,
                    &declaration.identity.coordinate,
                )?;
            }
        }
    }
    let mut bounded = source
        .types
        .iter()
        .filter_map(|declaration| {
            matches!(declaration.shape, SemanticTypeShape::CoreStringAlias { .. })
                .then_some(declaration.identity.coordinate.as_str())
        })
        .collect::<BTreeSet<_>>();
    loop {
        let before = bounded.len();
        for declaration in &source.types {
            if let SemanticTypeShape::Record { fields } = &declaration.shape {
                if fields
                    .iter()
                    .all(|field| bounded.contains(field.type_coordinate.as_str()))
                {
                    bounded.insert(&declaration.identity.coordinate);
                }
            }
        }
        if bounded.len() == before {
            break;
        }
    }
    if bounded.len() != source.types.len() {
        if let Some(declaration) = source
            .types
            .iter()
            .find(|declaration| !bounded.contains(declaration.identity.coordinate.as_str()))
        {
            let reference = match &declaration.shape {
                SemanticTypeShape::Record { fields } => fields
                    .iter()
                    .find(|field| !bounded.contains(field.type_coordinate.as_str()))
                    .map(|field| field.type_coordinate.as_str())
                    .unwrap_or_default(),
                SemanticTypeShape::CoreStringAlias { .. } => "",
            };
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnboundedTypeGraph,
                &declaration.identity.coordinate,
                reference,
            ));
        }
    }
    Ok(())
}

fn is_edict_core_coordinate(value: &str) -> bool {
    const SCALARS: [&str; 7] = ["Bool", "I32", "I64", "U32", "U64", "Digest", "Unit"];
    const REFINED_OR_COMPOUND: [&str; 6] =
        ["String", "Bytes", "Option", "List", "Map", "CapabilityRef"];

    SCALARS.contains(&value)
        || REFINED_OR_COMPOUND.iter().any(|name| {
            value
                .strip_prefix(name)
                .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with('<'))
        })
}

fn validate_obstruction_references(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let types = coordinates(&source.types);
    for obstruction in &source.obstructions {
        require_reference(
            &types,
            &obstruction.payload_schema,
            ProviderSemanticSourceErrorKind::UnknownType,
            &obstruction.identity.coordinate,
        )?;
    }
    Ok(())
}

fn validate_profile_references(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let write_classes = coordinates(&source.write_classes);
    for write_class in &source.write_classes {
        if !is_abi_write_class(&write_class.identity.coordinate) {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::InvalidWriteClass,
                "writeClasses",
                &write_class.identity.coordinate,
            ));
        }
    }
    let mut profile_by_source_name = BTreeMap::new();
    for profile in &source.profiles {
        ensure_unique_keys("profiles.sourceNames", profile.source_names.iter())?;
        ensure_unique_keys(
            "profiles.allowedWriteClasses",
            profile.allowed_write_classes.iter(),
        )?;
        ensure_unique_keys("profiles.guardKinds", profile.guard_kinds.iter())?;
        for source_name in &profile.source_names {
            if profile_by_source_name
                .insert(source_name.as_str(), profile.identity.coordinate.as_str())
                .is_some()
            {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::DuplicateKey,
                    "profiles.sourceNames",
                    source_name,
                ));
            }
        }
        for write_class in &profile.allowed_write_classes {
            require_reference(
                &write_classes,
                write_class,
                ProviderSemanticSourceErrorKind::UnknownWriteClass,
                &profile.identity.coordinate,
            )?;
        }
    }
    Ok(())
}

fn is_abi_write_class(value: &str) -> bool {
    matches!(
        value,
        "none" | "read" | "create" | "ensure" | "append" | "replace" | "delete" | "custom"
    )
}

fn validate_effect_references(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let types = coordinates(&source.types);

    for effect in &source.effects {
        if effect.parameter_types.len() != 1 {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnsupportedEffectShape,
                &effect.identity.coordinate,
                effect.parameter_types.len().to_string(),
            ));
        }
        ensure_unique_keys("effects.guardKinds", effect.guard_kinds.iter())?;
        if !effect.guard_kinds.is_empty() && !effect.guard_support {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnsupportedEffectShape,
                &effect.identity.coordinate,
                "guardSupport",
            ));
        }
        for parameter in &effect.parameter_types {
            require_reference(
                &types,
                parameter,
                ProviderSemanticSourceErrorKind::UnknownType,
                &effect.identity.coordinate,
            )?;
        }
        require_reference(
            &types,
            &effect.result_type,
            ProviderSemanticSourceErrorKind::UnknownType,
            &effect.identity.coordinate,
        )?;
        ensure_unique_keys(
            "effects.failures.key",
            effect.failures.iter().map(|failure| &failure.key),
        )?;
        for failure in &effect.failures {
            if !is_edict_failure_identifier(&failure.key) {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::InvalidFailureKey,
                    &effect.identity.coordinate,
                    &failure.key,
                ));
            }
            require_reference(
                &types,
                &failure.payload_type,
                ProviderSemanticSourceErrorKind::UnknownType,
                &effect.identity.coordinate,
            )?;
        }
    }
    Ok(())
}

fn is_edict_failure_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_')
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return false;
    }
    !matches!(
        value,
        "package"
            | "use"
            | "type"
            | "enum"
            | "variant"
            | "intent"
            | "returns"
            | "profile"
            | "implements"
            | "basis"
            | "footprint"
            | "budget"
            | "where"
            | "let"
            | "return"
            | "require"
            | "guarantee"
            | "assert"
            | "if"
            | "then"
            | "else"
            | "for"
            | "in"
            | "bounded"
            | "yield"
            | "match"
            | "as"
            | "digest"
            | "fn"
            | "const"
            | "true"
            | "false"
    )
}

fn validate_adapter_references(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let effects = coordinates(&source.effects);
    let capabilities = coordinates(&source.capabilities);
    for adapter in &source.direct_adapters {
        require_reference(
            &effects,
            &adapter.consumes_effect,
            ProviderSemanticSourceErrorKind::UnknownEffect,
            &adapter.identity.coordinate,
        )?;
        require_reference(
            &capabilities,
            &adapter.capability,
            ProviderSemanticSourceErrorKind::UnknownCapability,
            &adapter.identity.coordinate,
        )?;
        if !adapter.emits_effects.is_empty() {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnsupportedAdapterChain,
                &adapter.identity.coordinate,
                &adapter.emits_effects[0],
            ));
        }
    }
    Ok(())
}

fn validate_capability_references(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let write_classes = coordinates(&source.write_classes);
    let target_profiles = source
        .generated_artifacts
        .iter()
        .filter(|artifact| artifact.kind == GeneratedArtifactKind::TargetProfile)
        .map(|artifact| artifact.coordinate.as_str())
        .collect::<BTreeSet<_>>();
    let effects = source
        .effects
        .iter()
        .map(|effect| (effect.identity.coordinate.as_str(), effect))
        .collect::<BTreeMap<_, _>>();
    let mut target_ir_by_profile = BTreeMap::new();
    for capability in &source.capabilities {
        let Some(effect) = effects.get(capability.effect.as_str()) else {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnknownEffect,
                &capability.identity.coordinate,
                &capability.effect,
            ));
        };
        require_reference(
            &target_profiles,
            &capability.target_profile,
            ProviderSemanticSourceErrorKind::UnknownProfile,
            &capability.identity.coordinate,
        )?;
        require_reference(
            &write_classes,
            &capability.write_class,
            ProviderSemanticSourceErrorKind::UnknownWriteClass,
            &capability.identity.coordinate,
        )?;
        if capability.write_class == "none" {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ImplementationEffectMismatch,
                &capability.identity.coordinate,
                &capability.write_class,
            ));
        }
        require_nonempty("capabilities.targetIrDomain", &capability.target_ir_domain)?;
        if let Some(previous) = target_ir_by_profile.insert(
            capability.target_profile.as_str(),
            capability.target_ir_domain.as_str(),
        ) {
            if previous != capability.target_ir_domain {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::TargetIrDomainMismatch,
                    &capability.target_profile,
                    &capability.target_ir_domain,
                ));
            }
        }
        let expected_discharge = (
            effect.effect_kind_hint,
            effect.footprint_obligation.as_str(),
            effect.cost_obligation.as_str(),
        );
        let actual_discharge = (
            capability.semantic_discharge.effect_kind_hint,
            capability.semantic_discharge.footprint_obligation.as_str(),
            capability.semantic_discharge.cost_obligation.as_str(),
        );
        let missing_guard_support = effect.guard_support && !capability.guard_support;
        let atomic_guard_mismatch = effect
            .guard_kinds
            .iter()
            .any(|guard| guard == "precommit-atomic")
            && !capability.can_participate_in_atomic_guard;
        let proof_only_mutation = effect.execution_class == ExecutionClass::ProofOnly
            && is_mutating_write_class(&capability.write_class);
        if actual_discharge != expected_discharge
            || missing_guard_support
            || atomic_guard_mismatch
            || proof_only_mutation
        {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ImplementationEffectMismatch,
                &capability.identity.coordinate,
                &capability.effect,
            ));
        }
    }
    Ok(())
}

fn validate_effect_implementation_closure(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let mut implementations = BTreeMap::new();
    for (effect, implementation) in source
        .capabilities
        .iter()
        .map(|capability| (&capability.effect, &capability.identity.coordinate))
        .chain(
            source
                .direct_adapters
                .iter()
                .map(|adapter| (&adapter.consumes_effect, &adapter.identity.coordinate)),
        )
    {
        if implementations
            .insert(effect.as_str(), implementation)
            .is_some()
        {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::AmbiguousEffectImplementation,
                "effectImplementations",
                effect,
            ));
        }
    }
    if let Some(effect) = source.effects.iter().find(|effect| {
        effect.execution_class == ExecutionClass::Runtime
            && !implementations.contains_key(effect.identity.coordinate.as_str())
    }) {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::MissingEffectImplementation,
            "effectImplementations",
            &effect.identity.coordinate,
        ));
    }
    Ok(())
}

fn is_mutating_write_class(write_class: &str) -> bool {
    !matches!(write_class, "none" | "read")
}

fn validate_operation_references(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let types = coordinates(&source.types);
    let type_declarations = source
        .types
        .iter()
        .map(|declaration| (declaration.identity.coordinate.as_str(), declaration))
        .collect::<BTreeMap<_, _>>();
    let effects = source
        .effects
        .iter()
        .map(|effect| (effect.identity.coordinate.as_str(), effect))
        .collect::<BTreeMap<_, _>>();
    let profiles = source
        .profiles
        .iter()
        .map(|profile| (profile.identity.coordinate.as_str(), profile))
        .collect::<BTreeMap<_, _>>();
    let budgets = coordinates(&source.budgets);
    let capabilities = source
        .capabilities
        .iter()
        .map(|capability| (capability.identity.coordinate.as_str(), capability))
        .collect::<BTreeMap<_, _>>();
    let adapters = source
        .direct_adapters
        .iter()
        .map(|adapter| (adapter.identity.coordinate.as_str(), adapter))
        .collect::<BTreeMap<_, _>>();
    let obstructions = source
        .obstructions
        .iter()
        .map(|obstruction| (obstruction.identity.coordinate.as_str(), obstruction))
        .collect::<BTreeMap<_, _>>();

    for operation in &source.operations {
        let subject = &operation.identity.coordinate;
        require_reference(
            &types,
            &operation.input_type,
            ProviderSemanticSourceErrorKind::UnknownType,
            subject,
        )?;
        require_reference(
            &types,
            &operation.output_type,
            ProviderSemanticSourceErrorKind::UnknownType,
            subject,
        )?;
        let Some(effect) = effects.get(operation.effect.as_str()) else {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnknownEffect,
                subject,
                &operation.effect,
            ));
        };
        let Some(profile) = profiles.get(operation.profile.as_str()) else {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnknownProfile,
                subject,
                &operation.profile,
            ));
        };
        require_reference(
            &budgets,
            &operation.budget,
            ProviderSemanticSourceErrorKind::UnknownBudget,
            subject,
        )?;
        let capability = match &operation.implementation {
            OperationImplementation::Native {
                capability: capability_coordinate,
            } => {
                let Some(capability) = capabilities.get(capability_coordinate.as_str()) else {
                    return Err(ProviderSemanticSourceError::new(
                        ProviderSemanticSourceErrorKind::UnknownCapability,
                        subject,
                        capability_coordinate,
                    ));
                };
                if capability.effect != operation.effect {
                    return Err(ProviderSemanticSourceError::new(
                        ProviderSemanticSourceErrorKind::ImplementationEffectMismatch,
                        subject,
                        &capability.effect,
                    ));
                }
                *capability
            }
            OperationImplementation::DirectAdapter {
                adapter: adapter_coordinate,
            } => {
                let Some(adapter) = adapters.get(adapter_coordinate.as_str()) else {
                    return Err(ProviderSemanticSourceError::new(
                        ProviderSemanticSourceErrorKind::UnknownAdapter,
                        subject,
                        adapter_coordinate,
                    ));
                };
                if adapter.consumes_effect != operation.effect {
                    return Err(ProviderSemanticSourceError::new(
                        ProviderSemanticSourceErrorKind::ImplementationEffectMismatch,
                        subject,
                        &adapter.consumes_effect,
                    ));
                }
                let Some(capability) = capabilities.get(adapter.capability.as_str()) else {
                    return Err(ProviderSemanticSourceError::new(
                        ProviderSemanticSourceErrorKind::UnknownCapability,
                        subject,
                        &adapter.capability,
                    ));
                };
                *capability
            }
        };
        if !profile
            .allowed_write_classes
            .contains(&capability.write_class)
        {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
                subject,
                &capability.write_class,
            ));
        }
        if let Some(guard) = effect
            .guard_kinds
            .iter()
            .find(|guard| !profile.guard_kinds.contains(guard))
        {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
                subject,
                guard,
            ));
        }
        match &profile.optic_template.aperture_requirement {
            Some(ApertureRequirementDeclaration::AbstractFootprintObligation { reference })
                if reference == &effect.footprint_obligation => {}
            Some(aperture) => {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
                    subject,
                    aperture.reference(),
                ));
            }
            None => {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
                    subject,
                    &effect.footprint_obligation,
                ));
            }
        }
        if is_mutating_write_class(&capability.write_class) {
            if profile.optic_template.optic_kind != OpticKind::AffectReintegration {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
                    subject,
                    "revelation",
                ));
            }
            if profile.optic_template.boundary_kind != BoundaryKind::Affect {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
                    subject,
                    "projection",
                ));
            }
        }
        if effect
            .guard_kinds
            .iter()
            .any(|guard| guard == "precommit-atomic")
        {
            if profile.atomicity != "atomic" {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::ProfileEffectMismatch,
                    subject,
                    &profile.atomicity,
                ));
            }
            if !capability.can_participate_in_atomic_guard {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::ImplementationEffectMismatch,
                    &capability.identity.coordinate,
                    &capability.effect,
                ));
            }
        }
        validate_operation_obstruction_mappings(
            subject,
            &operation.obstruction_mappings,
            effect,
            &obstructions,
            &type_declarations,
        )?;
    }
    Ok(())
}

fn validate_operation_obstruction_mappings(
    subject: &str,
    mappings: &[ObstructionMappingDeclaration],
    effect: &SemanticEffectDeclaration,
    obstructions: &BTreeMap<&str, &ObstructionDeclaration>,
    types: &BTreeMap<&str, &SemanticTypeDeclaration>,
) -> Result<(), ProviderSemanticSourceError> {
    ensure_unique_keys(
        "operations.obstructionMappings.failure",
        mappings.iter().map(|mapping| &mapping.failure),
    )?;
    let failures = effect
        .failures
        .iter()
        .map(|failure| (failure.key.as_str(), failure))
        .collect::<BTreeMap<_, _>>();
    let expected = effect
        .failures
        .iter()
        .filter(|failure| failure.authority_class == AuthorityClass::DomainMappable)
        .map(|failure| failure.key.as_str())
        .collect::<BTreeSet<_>>();
    let actual = mappings
        .iter()
        .map(|mapping| mapping.failure.as_str())
        .collect::<BTreeSet<_>>();

    for mapping in mappings {
        let Some(failure) = failures.get(mapping.failure.as_str()) else {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnknownFailure,
                subject,
                &mapping.failure,
            ));
        };
        if failure.authority_class != AuthorityClass::DomainMappable {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ObstructionMappingMismatch,
                subject,
                &mapping.failure,
            ));
        }
        let Some(obstruction) = obstructions.get(mapping.obstruction.as_str()) else {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnknownObstruction,
                subject,
                &mapping.obstruction,
            ));
        };
        if obstruction.authority_class != AuthorityClass::DomainMappable {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ObstructionMappingMismatch,
                subject,
                &mapping.obstruction,
            ));
        }
        if !is_empty_record_type(types, &failure.payload_type)
            || !is_empty_record_type(types, &obstruction.payload_schema)
        {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnsupportedObstructionPayloadMapping,
                subject,
                &mapping.failure,
            ));
        }
    }

    if actual != expected {
        let reference = expected
            .symmetric_difference(&actual)
            .next()
            .copied()
            .unwrap_or_default();
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ObstructionMappingMismatch,
            subject,
            reference,
        ));
    }
    Ok(())
}

fn is_empty_record_type(
    types: &BTreeMap<&str, &SemanticTypeDeclaration>,
    coordinate: &str,
) -> bool {
    matches!(
        types.get(coordinate).map(|declaration| &declaration.shape),
        Some(SemanticTypeShape::Record { fields }) if fields.is_empty()
    )
}

fn validate_generated_artifact_contracts(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    for artifact in &source.generated_artifacts {
        if artifact.kind == GeneratedArtifactKind::ProviderManifest {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::SelfReferentialManifestInventory,
                &artifact.role,
                &artifact.coordinate,
            ));
        }
        let expected = match artifact.kind {
            GeneratedArtifactKind::Lawpack => "edict.lawpack/v1",
            GeneratedArtifactKind::TargetProfile => "edict.target-profile/v1",
            GeneratedArtifactKind::AuthorityFacts => "edict.authority-facts/v1",
            GeneratedArtifactKind::ProviderManifest => "edict.provider-manifest/v1",
            GeneratedArtifactKind::ReviewArtifact => "echo.edict-provider.generation-review/v1",
            GeneratedArtifactKind::GeneratedArtifactProfile => "echo.generated-artifact-profile/v1",
            GeneratedArtifactKind::GenerationProvenance => "wesley:GenerationProvenanceManifestV1",
            GeneratedArtifactKind::ArtifactSchema => "selfContainedCddlV1",
        };
        if artifact.schema_contract != expected {
            return Err(ProviderSemanticSourceError::new(
                if artifact.kind == GeneratedArtifactKind::GenerationProvenance {
                    ProviderSemanticSourceErrorKind::GenerationProvenanceContractMismatch
                } else {
                    ProviderSemanticSourceErrorKind::ArtifactSchemaContractMismatch
                },
                &artifact.role,
                &artifact.schema_contract,
            ));
        }
        let expected_contract_owner = match artifact.kind {
            GeneratedArtifactKind::AuthorityFacts => Some("flyingrobots/edict#157"),
            GeneratedArtifactKind::GenerationProvenance => Some("flyingrobots/wesley#728"),
            _ => None,
        };
        if artifact.contract_owner.as_deref() != expected_contract_owner {
            return Err(ProviderSemanticSourceError::new(
                if artifact.kind == GeneratedArtifactKind::GenerationProvenance {
                    ProviderSemanticSourceErrorKind::GenerationProvenanceContractMismatch
                } else {
                    ProviderSemanticSourceErrorKind::ArtifactContractOwnerMismatch
                },
                &artifact.role,
                artifact.contract_owner.as_deref().unwrap_or_default(),
            ));
        }
        validate_authority_fact_projection(source, artifact)?;
    }
    if source.package_manifest.schema_contract != "edict.provider-manifest/v1" {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactSchemaContractMismatch,
            &source.package_manifest.role,
            &source.package_manifest.schema_contract,
        ));
    }
    for (actual, expected) in [
        (
            source.package_manifest.provider_abi.as_str(),
            "edict:target-provider@1.0.0",
        ),
        (
            source.package_manifest.provider_coordinate.as_str(),
            "echo.edict-provider@1",
        ),
    ] {
        if actual != expected {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ProviderManifestProjectionMismatch,
                &source.package_manifest.role,
                actual,
            ));
        }
    }
    if source.package_manifest.provider_coordinate == source.package_manifest.coordinate {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ProviderManifestProjectionMismatch,
            &source.package_manifest.role,
            &source.package_manifest.provider_coordinate,
        ));
    }
    for (kind, expected_count) in [
        (GeneratedArtifactKind::Lawpack, 1),
        (GeneratedArtifactKind::TargetProfile, 1),
        (GeneratedArtifactKind::AuthorityFacts, 2),
        (GeneratedArtifactKind::ReviewArtifact, 1),
        (GeneratedArtifactKind::GeneratedArtifactProfile, 1),
        (GeneratedArtifactKind::GenerationProvenance, 1),
        (GeneratedArtifactKind::ArtifactSchema, 1),
    ] {
        let actual = source
            .generated_artifacts
            .iter()
            .filter(|artifact| artifact.kind == kind)
            .count();
        if actual != expected_count {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
                "generatedArtifacts",
                format!("{}:{actual}", generated_artifact_kind_name(kind)),
            ));
        }
    }
    for source_kind in [
        AuthorityFactSourceKind::Lawpack,
        AuthorityFactSourceKind::TargetProfile,
    ] {
        let actual = source
            .generated_artifacts
            .iter()
            .filter_map(|artifact| artifact.authority_fact_source.as_ref())
            .filter(|fact_source| fact_source.kind == source_kind)
            .count();
        if actual != 1 {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::AuthorityFactProjectionMismatch,
                "generatedArtifacts",
                authority_fact_source_kind_name(source_kind),
            ));
        }
    }
    Ok(())
}

const fn generated_artifact_kind_name(kind: GeneratedArtifactKind) -> &'static str {
    match kind {
        GeneratedArtifactKind::Lawpack => "lawpack",
        GeneratedArtifactKind::TargetProfile => "targetProfile",
        GeneratedArtifactKind::AuthorityFacts => "authorityFacts",
        GeneratedArtifactKind::ProviderManifest => "providerManifest",
        GeneratedArtifactKind::ReviewArtifact => "reviewArtifact",
        GeneratedArtifactKind::GeneratedArtifactProfile => "generatedArtifactProfile",
        GeneratedArtifactKind::GenerationProvenance => "generationProvenance",
        GeneratedArtifactKind::ArtifactSchema => "artifactSchema",
    }
}

const fn authority_fact_source_kind_name(kind: AuthorityFactSourceKind) -> &'static str {
    match kind {
        AuthorityFactSourceKind::Lawpack => "lawpack",
        AuthorityFactSourceKind::TargetProfile => "targetProfile",
    }
}

fn validate_authority_fact_projection(
    source: &ProviderSemanticSourceV1,
    artifact: &GeneratedArtifactDeclaration,
) -> Result<(), ProviderSemanticSourceError> {
    let Some(fact_source) = &artifact.authority_fact_source else {
        if artifact.kind == GeneratedArtifactKind::AuthorityFacts {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::AuthorityFactProjectionMismatch,
                &artifact.role,
                "authorityFactSource",
            ));
        }
        return Ok(());
    };
    if artifact.kind != GeneratedArtifactKind::AuthorityFacts {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::AuthorityFactProjectionMismatch,
            &artifact.role,
            &fact_source.coordinate,
        ));
    }
    let expected_kind = match fact_source.kind {
        AuthorityFactSourceKind::Lawpack => GeneratedArtifactKind::Lawpack,
        AuthorityFactSourceKind::TargetProfile => GeneratedArtifactKind::TargetProfile,
    };
    let valid = source.generated_artifacts.iter().any(|candidate| {
        candidate.kind == expected_kind && candidate.coordinate == fact_source.coordinate
    });
    if !valid {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::AuthorityFactProjectionMismatch,
            &artifact.role,
            &fact_source.coordinate,
        ));
    }
    Ok(())
}

fn validate_artifact_closure(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    let artifacts = source
        .generated_artifacts
        .iter()
        .map(|artifact| (artifact.role.as_str(), artifact))
        .collect::<BTreeMap<_, _>>();
    let resources = source
        .artifact_resources
        .iter()
        .map(|resource| (resource.role.as_str(), resource))
        .collect::<BTreeMap<_, _>>();
    let lawpack = &source.lawpack_projection;
    let lawpack_artifact = require_artifact_kind(
        &artifacts,
        &lawpack.artifact_role,
        GeneratedArtifactKind::Lawpack,
        "lawpackProjection.artifactRole",
    )?;
    if lawpack_artifact.coordinate != format!("{}@{}", lawpack.id, lawpack.version) {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            &lawpack.artifact_role,
            &lawpack_artifact.coordinate,
        ));
    }
    if lawpack.accepted_core_abis != ["edict.core/v1"]
        || !lawpack.dependencies.is_empty()
        || lawpack.target_adapters.is_empty()
    {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            &lawpack.artifact_role,
            "lawpackManifest",
        ));
    }
    let mut used_resources = BTreeSet::new();
    require_resource(
        &resources,
        &lawpack.exports_resource,
        Some("edict.lawpack/v1#lawpack-exports"),
        Some(ArtifactResourceProvision::Generated),
        &lawpack.artifact_role,
        &mut used_resources,
    )?;
    require_resource(
        &resources,
        &lawpack.compatibility_resource,
        Some("echo.edict-provider.lawpack-compatibility/v1"),
        Some(ArtifactResourceProvision::Generated),
        &lawpack.artifact_role,
        &mut used_resources,
    )?;
    require_resource(
        &resources,
        &lawpack.conformance_fixture_corpus_resource,
        Some("echo.edict-provider.conformance-corpus/v1"),
        Some(ArtifactResourceProvision::Generated),
        &lawpack.artifact_role,
        &mut used_resources,
    )?;
    let LawpackVerifierDeclaration::Declarative { ruleset_resource } = &lawpack.verifier;
    require_resource(
        &resources,
        ruleset_resource,
        Some("echo.edict-provider.lawpack-verifier/v1"),
        Some(ArtifactResourceProvision::Generated),
        &lawpack.artifact_role,
        &mut used_resources,
    )?;

    let runtime_effects = source
        .effects
        .iter()
        .filter(|effect| effect.execution_class == ExecutionClass::Runtime)
        .map(|effect| effect.identity.coordinate.as_str())
        .collect::<BTreeSet<_>>();
    let mut adapted_effects = BTreeSet::new();
    for adapter in &lawpack.target_adapters {
        require_artifact_kind(
            &artifacts,
            &adapter.accepted_target_profile_role,
            GeneratedArtifactKind::TargetProfile,
            &lawpack.artifact_role,
        )?;
        let target_ir = require_resource(
            &resources,
            &adapter.accepted_target_ir_resource,
            Some("echo.span-ir/v1"),
            Some(ArtifactResourceProvision::Generated),
            &lawpack.artifact_role,
            &mut used_resources,
        )?;
        require_resource(
            &resources,
            &adapter.adapter_resource,
            Some("echo.edict-provider.lawpack-target-adapter/v1"),
            Some(ArtifactResourceProvision::Generated),
            &lawpack.artifact_role,
            &mut used_resources,
        )?;
        if adapter.effects.is_empty() {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
                &lawpack.artifact_role,
                &adapter.adapter_resource,
            ));
        }
        for effect in &adapter.effects {
            if !runtime_effects.contains(effect.as_str())
                || !adapted_effects.insert(effect.as_str())
            {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
                    &lawpack.artifact_role,
                    effect,
                ));
            }
        }
        if !source
            .capabilities
            .iter()
            .any(|capability| capability.target_ir_domain == target_ir.coordinate)
        {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
                &lawpack.artifact_role,
                &target_ir.coordinate,
            ));
        }
    }
    if adapted_effects != runtime_effects {
        let reference = runtime_effects
            .iter()
            .find(|effect| !adapted_effects.contains(**effect))
            .copied()
            .unwrap_or_default();
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            &lawpack.artifact_role,
            reference,
        ));
    }

    let target = &source.target_profile_projection;
    let target_artifact = require_artifact_kind(
        &artifacts,
        &target.artifact_role,
        GeneratedArtifactKind::TargetProfile,
        "targetProfileProjection.artifactRole",
    )?;
    if target_artifact.coordinate != format!("{}@{}", target.id, target.version)
        || target.intrinsic_namespace != target_artifact.coordinate
        || source
            .capabilities
            .iter()
            .any(|capability| capability.target_profile != target_artifact.coordinate)
    {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            &target.artifact_role,
            &target_artifact.coordinate,
        ));
    }
    if target.accepted_core_abis != ["edict.core/v1"]
        || !target.accepted_lawpack_adapter_abis.is_empty()
        || target.application_model != "atomic"
        || target.read_consistency != "application-snapshot"
        || target.guard_evaluation != "precommit-atomic"
        || target.obstruction_rollback != "no-visible-effects"
        || target.multi_target
    {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            &target.artifact_role,
            "targetProfileManifest",
        ));
    }
    for profile in &source.profiles {
        if profile.postcondition_support != target.postcondition_support {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
                &target.artifact_role,
                &profile.identity.coordinate,
            ));
        }
    }
    let target_slots = [
        (
            target.intrinsics_resource.as_str(),
            Some("edict.target-profile.intrinsics/v1"),
            ArtifactResourceProvision::Generated,
        ),
        (
            target.operation_profiles_resource.as_str(),
            Some("edict.target-profile.operation-profiles/v1"),
            ArtifactResourceProvision::Generated,
        ),
        (
            target.footprint_algebra_resource.as_str(),
            Some("echo.dpo.footprint/v1"),
            ArtifactResourceProvision::Generated,
        ),
        (
            target.cost_algebra_resource.as_str(),
            Some("echo.dpo.cost/v1"),
            ArtifactResourceProvision::Generated,
        ),
        (
            target.target_ir_resource.as_str(),
            Some("echo.span-ir/v1"),
            ArtifactResourceProvision::Generated,
        ),
        (
            target.obstruction_taxonomy_resource.as_str(),
            Some("echo.dpo.obstructions/v1"),
            ArtifactResourceProvision::Generated,
        ),
        (
            target.verifier_resource.as_str(),
            Some("echo.dpo.verifier/v1"),
            ArtifactResourceProvision::Generated,
        ),
        (
            target.lowerer_resource.as_str(),
            Some("echo.dpo.lowerer/v1"),
            ArtifactResourceProvision::Generated,
        ),
        (
            target.sandbox_resource.as_str(),
            Some("edict.wasm-component/v1"),
            ArtifactResourceProvision::External,
        ),
        (
            target.fuel_model_resource.as_str(),
            Some("edict.fuel/v1"),
            ArtifactResourceProvision::External,
        ),
        (
            target.bundle_profile_resource.as_str(),
            Some("echo.dpo.bundle/v1"),
            ArtifactResourceProvision::Generated,
        ),
        (
            target.canonical_encoding_rules_resource.as_str(),
            Some("edict.canonical-cbor/v1"),
            ArtifactResourceProvision::External,
        ),
        (
            target.diagnostic_abi_resource.as_str(),
            Some("edict.diagnostics/v1"),
            ArtifactResourceProvision::External,
        ),
        (
            target.deterministic_execution_resource.as_str(),
            Some("edict.determinism/v1"),
            ArtifactResourceProvision::External,
        ),
        (
            target.conformance_fixture_corpus_resource.as_str(),
            Some("echo.edict-provider.conformance-corpus/v1"),
            ArtifactResourceProvision::Generated,
        ),
    ];
    let mut target_ir = None;
    for (role, contract, provision) in target_slots {
        let resource = require_resource(
            &resources,
            role,
            contract,
            Some(provision),
            &target.artifact_role,
            &mut used_resources,
        )?;
        if role == target.target_ir_resource {
            target_ir = Some(resource.coordinate.as_str());
        }
    }
    let Some(target_ir) = target_ir else {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            &target.artifact_role,
            &target.target_ir_resource,
        ));
    };
    if source
        .capabilities
        .iter()
        .any(|capability| capability.target_ir_domain != target_ir)
    {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            &target.artifact_role,
            target_ir,
        ));
    }
    for role in &target.generated_artifact_profile_roles {
        require_artifact_kind(
            &artifacts,
            role,
            GeneratedArtifactKind::GeneratedArtifactProfile,
            &target.artifact_role,
        )?;
    }
    let declared_profile_roles = source
        .generated_artifacts
        .iter()
        .filter(|artifact| artifact.kind == GeneratedArtifactKind::GeneratedArtifactProfile)
        .map(|artifact| artifact.role.as_str())
        .collect::<BTreeSet<_>>();
    let referenced_profile_roles = target
        .generated_artifact_profile_roles
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if declared_profile_roles != referenced_profile_roles {
        let reference = declared_profile_roles
            .symmetric_difference(&referenced_profile_roles)
            .next()
            .copied()
            .unwrap_or_default();
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            &target.artifact_role,
            reference,
        ));
    }
    if used_resources.len() != resources.len() {
        let unused = resources
            .keys()
            .find(|role| !used_resources.contains(**role))
            .copied()
            .unwrap_or_default();
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            "artifactResources",
            unused,
        ));
    }
    Ok(())
}

fn require_artifact_kind<'a>(
    artifacts: &'a BTreeMap<&str, &'a GeneratedArtifactDeclaration>,
    role: &str,
    expected_kind: GeneratedArtifactKind,
    subject: &str,
) -> Result<&'a GeneratedArtifactDeclaration, ProviderSemanticSourceError> {
    let Some(artifact) = artifacts.get(role).copied() else {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            subject,
            role,
        ));
    };
    if artifact.kind != expected_kind {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            subject,
            role,
        ));
    }
    Ok(artifact)
}

fn require_resource<'a>(
    resources: &'a BTreeMap<&str, &'a ArtifactResourceDeclaration>,
    role: &str,
    expected_contract: Option<&str>,
    expected_provision: Option<ArtifactResourceProvision>,
    subject: &str,
    used: &mut BTreeSet<&'a str>,
) -> Result<&'a ArtifactResourceDeclaration, ProviderSemanticSourceError> {
    let Some(resource) = resources.get(role).copied() else {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            subject,
            role,
        ));
    };
    if expected_contract.is_some_and(|contract| resource.schema_contract != contract)
        || expected_provision.is_some_and(|provision| resource.provision != provision)
    {
        return Err(ProviderSemanticSourceError::new(
            ProviderSemanticSourceErrorKind::ArtifactClosureMismatch,
            subject,
            role,
        ));
    }
    used.insert(resource.role.as_str());
    Ok(resource)
}

fn validate_invocation_schema_bindings(
    source: &ProviderSemanticSourceV1,
) -> Result<(), ProviderSemanticSourceError> {
    for (kind, expected_count) in [
        (InvocationInputKind::Core, 1),
        (InvocationInputKind::TargetProfile, 1),
        (InvocationInputKind::Lawpack, 1),
        (InvocationInputKind::AuthorityFacts, 2),
        (InvocationInputKind::LowerabilityFacts, 1),
        (InvocationInputKind::TargetIr, 1),
    ] {
        let actual = source
            .invocation_inputs
            .iter()
            .filter(|input| input.kind == kind)
            .count();
        if actual != expected_count {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::InvocationInputClosureMismatch,
                "invocationInputs",
                invocation_input_kind_name(kind),
            ));
        }
    }
    for input in &source.invocation_inputs {
        let expected_domain = expected_input_domain(input.kind);
        if input.domain != expected_domain {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::InputDomainMismatch,
                &input.role,
                &input.domain,
            ));
        }
    }
    for kind in [
        InvocationOutputKind::TargetIr,
        InvocationOutputKind::GeneratedArtifact,
        InvocationOutputKind::ReviewPayload,
        InvocationOutputKind::VerifierReport,
    ] {
        let actual = source
            .invocation_outputs
            .iter()
            .filter(|output| output.kind == kind)
            .count();
        if actual != 1 {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::InvocationOutputClosureMismatch,
                "invocationOutputs",
                invocation_output_kind_name(kind),
            ));
        }
    }
    let schema_roles = source
        .generated_artifacts
        .iter()
        .filter(|artifact| artifact.kind == GeneratedArtifactKind::ArtifactSchema)
        .map(|artifact| artifact.role.as_str())
        .collect::<BTreeSet<_>>();
    let bindings = source
        .schema_bindings
        .iter()
        .map(|binding| (binding.domain.as_str(), binding))
        .collect::<BTreeMap<_, _>>();
    let output_domains = source
        .invocation_outputs
        .iter()
        .map(|output| output.domain.as_str())
        .collect::<BTreeSet<_>>();
    let input_domains = source
        .invocation_inputs
        .iter()
        .map(|input| input.domain.as_str())
        .collect::<BTreeSet<_>>();
    for output in &source.invocation_outputs {
        let (expected_domain, _) = expected_output_contract(output.kind);
        if output.domain != expected_domain {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::OutputDomainMismatch,
                &output.role,
                &output.domain,
            ));
        }
    }

    for binding in &source.schema_bindings {
        if !output_domains.contains(binding.domain.as_str())
            && !input_domains.contains(binding.domain.as_str())
        {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::UnexpectedSchemaBinding,
                "schemaBindings",
                &binding.domain,
            ));
        }
        require_reference(
            &schema_roles,
            &binding.schema_role,
            ProviderSemanticSourceErrorKind::UnknownSchemaRole,
            &binding.domain,
        )?;
        let expected_root = expected_schema_root(&binding.domain);
        if let Some(expected_root) = expected_root {
            if binding.root_rule != expected_root {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::SchemaRootMismatch,
                    &binding.domain,
                    &binding.root_rule,
                ));
            }
        }
    }

    let artifact_roles = source
        .generated_artifacts
        .iter()
        .map(|artifact| (artifact.role.as_str(), artifact.kind))
        .collect::<BTreeMap<_, _>>();
    for input in &source.invocation_inputs {
        require_reference(
            &schema_roles,
            &input.schema_role,
            ProviderSemanticSourceErrorKind::UnknownSchemaRole,
            &input.role,
        )?;
        if let Some(expected_kind) = input_artifact_kind(input.kind) {
            if artifact_roles.get(input.role.as_str()) != Some(&expected_kind) {
                return Err(ProviderSemanticSourceError::new(
                    ProviderSemanticSourceErrorKind::InvocationInputClosureMismatch,
                    &input.role,
                    generated_artifact_kind_name(expected_kind),
                ));
            }
        }
        let Some(binding) = bindings.get(input.domain.as_str()) else {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::MissingSchemaBinding,
                &input.role,
                &input.domain,
            ));
        };
        if binding.schema_role != input.schema_role {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::SchemaRoleMismatch,
                &input.role,
                &input.schema_role,
            ));
        }
    }

    for output in &source.invocation_outputs {
        require_reference(
            &schema_roles,
            &output.schema_role,
            ProviderSemanticSourceErrorKind::UnknownSchemaRole,
            &output.role,
        )?;
        let (_, expected_root) = expected_output_contract(output.kind);
        let Some(binding) = bindings.get(output.domain.as_str()) else {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::MissingSchemaBinding,
                &output.role,
                &output.domain,
            ));
        };
        if binding.schema_role != output.schema_role {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::SchemaRoleMismatch,
                &output.role,
                &output.schema_role,
            ));
        }
        if binding.root_rule != expected_root {
            return Err(ProviderSemanticSourceError::new(
                ProviderSemanticSourceErrorKind::SchemaRootMismatch,
                &output.role,
                &binding.root_rule,
            ));
        }
    }
    Ok(())
}

fn expected_output_contract(kind: InvocationOutputKind) -> (&'static str, &'static str) {
    match kind {
        InvocationOutputKind::TargetIr => ("edict.target-ir.artifact/v1", "target-ir-artifact"),
        InvocationOutputKind::GeneratedArtifact => {
            ("echo.generated-artifact/v1", "generated-artifact")
        }
        InvocationOutputKind::ReviewPayload => ("echo.review-payload/v1", "review-payload"),
        InvocationOutputKind::VerifierReport => ("echo.verifier-report/v1", "verifier-report"),
    }
}

const fn expected_input_domain(kind: InvocationInputKind) -> &'static str {
    match kind {
        InvocationInputKind::Core => "edict.core.module/v1",
        InvocationInputKind::TargetProfile => "edict.target-profile/v1",
        InvocationInputKind::Lawpack => "edict.lawpack/v1",
        InvocationInputKind::AuthorityFacts => "edict.authority-facts/v1",
        InvocationInputKind::LowerabilityFacts => "edict.lowering-requirements/v1",
        InvocationInputKind::TargetIr => "edict.target-ir.artifact/v1",
    }
}

const fn invocation_input_kind_name(kind: InvocationInputKind) -> &'static str {
    match kind {
        InvocationInputKind::Core => "core",
        InvocationInputKind::TargetProfile => "targetProfile",
        InvocationInputKind::Lawpack => "lawpack",
        InvocationInputKind::AuthorityFacts => "authorityFacts",
        InvocationInputKind::LowerabilityFacts => "lowerabilityFacts",
        InvocationInputKind::TargetIr => "targetIr",
    }
}

const fn input_artifact_kind(kind: InvocationInputKind) -> Option<GeneratedArtifactKind> {
    match kind {
        InvocationInputKind::TargetProfile => Some(GeneratedArtifactKind::TargetProfile),
        InvocationInputKind::Lawpack => Some(GeneratedArtifactKind::Lawpack),
        InvocationInputKind::AuthorityFacts => Some(GeneratedArtifactKind::AuthorityFacts),
        InvocationInputKind::Core
        | InvocationInputKind::LowerabilityFacts
        | InvocationInputKind::TargetIr => None,
    }
}

const fn invocation_output_kind_name(kind: InvocationOutputKind) -> &'static str {
    match kind {
        InvocationOutputKind::TargetIr => "targetIr",
        InvocationOutputKind::GeneratedArtifact => "generatedArtifact",
        InvocationOutputKind::ReviewPayload => "reviewPayload",
        InvocationOutputKind::VerifierReport => "verifierReport",
    }
}

fn expected_schema_root(domain: &str) -> Option<&'static str> {
    match domain {
        "edict.authority-facts/v1" => Some("authority-facts"),
        "edict.core.module/v1" => Some("core-module"),
        "edict.lawpack/v1" => Some("lawpack-manifest"),
        "edict.lowering-requirements/v1" => Some("lowering-requirements"),
        "edict.target-profile/v1" => Some("target-profile-manifest"),
        "edict.target-ir.artifact/v1" => Some("target-ir-artifact"),
        "echo.generated-artifact/v1" => Some("generated-artifact"),
        "echo.review-payload/v1" => Some("review-payload"),
        "echo.verifier-report/v1" => Some("verifier-report"),
        _ => None,
    }
}

fn coordinates<T: HasIdentity>(facts: &[T]) -> BTreeSet<&str> {
    facts
        .iter()
        .map(|fact| fact.identity().coordinate.as_str())
        .collect()
}

fn require_reference(
    known: &BTreeSet<&str>,
    reference: &str,
    kind: ProviderSemanticSourceErrorKind,
    subject: &str,
) -> Result<(), ProviderSemanticSourceError> {
    if known.contains(reference) {
        Ok(())
    } else {
        Err(ProviderSemanticSourceError::new(kind, subject, reference))
    }
}
