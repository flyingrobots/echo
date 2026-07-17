// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Provider-generic contract package proposal and runtime installation boundaries.
//!
//! A proposal retains exact generated identities and one explicit host
//! implementation binding. It grants no admission, installation, scheduling,
//! execution, durability, receipt, or observation authority.
//! Installation retains a distinct Engine-owned provider record and indexes its
//! mutation rule without admitting runtime intents or invoking host callbacks.

use core::fmt;

use echo_registry_api::{
    is_reserved_operation_id, OpKind, ProviderBundleIdentityV1, ProviderDigestIdentityV1,
    ProviderFootprintIdentityV1, ProviderOperationV1, ProviderRegistryV1, ProviderSchemaIdentityV1,
    ProviderSemanticIdentityV1, ProviderValueContractV1,
};

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
use blake3::Hasher;

use crate::contract_host::runtime_ingress_eint_read_footprint;
use crate::contract_registry::{ContractMutationHandler, ContractPackageIdentity};
use crate::footprint::Footprint;
use crate::graph_view::GraphView;
use crate::ident::{make_type_id, NodeId};
use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};
use crate::TickDelta;

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
const INSTALLED_PROVIDER_CONTRACT_PACKAGE_ID_DOMAIN: &[u8] =
    b"echo:installed-provider-contract-package-id:v1\0";

/// Generated matcher callback for one proposed provider mutation.
pub type ProviderMutationMatchFnV1 = for<'view> fn(GraphView<'view>, &NodeId) -> bool;

/// Explicit host executor callback for one proposed provider mutation.
pub type ProviderMutationExecuteFnV1 = for<'view> fn(GraphView<'view>, &NodeId, &mut TickDelta);

/// Explicit host footprint callback for one proposed provider mutation.
pub type ProviderMutationFootprintFnV1 = for<'view> fn(GraphView<'view>, &NodeId) -> Footprint;

/// Host-owned semantic implementation for one generated provider mutation.
///
/// Echo wraps the effect footprint returned here with the mandatory runtime
/// ingress EINT read used by generated matchers. Implementations therefore
/// describe only their semantic graph effects and cannot omit that Echo-owned
/// read from a proposed rule.
pub trait ProviderMutationHostV1 {
    /// Execute the semantic mutation during a scheduler-owned tick.
    fn execute(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta);

    /// Compute the semantic effect footprint beyond Echo's ingress EINT read.
    fn effect_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint;
}

fn execute_provider_host<H: ProviderMutationHostV1>(
    view: GraphView<'_>,
    scope: &NodeId,
    delta: &mut TickDelta,
) {
    H::execute(view, scope, delta);
}

fn compute_provider_host_footprint<H: ProviderMutationHostV1>(
    view: GraphView<'_>,
    scope: &NodeId,
) -> Footprint {
    let mut footprint = runtime_ingress_eint_read_footprint(view, scope);
    footprint.union_assign(&H::effect_footprint(view, scope));
    footprint.factor_mask = u64::MAX;
    footprint
}

/// Exact identities claimed by one host mutation implementation.
///
/// Equality against the generated registry detects accidental cross-binding.
/// It does not prove that arbitrary Rust callbacks semantically implement the
/// claims; that stronger proposition belongs to admitted implementation and
/// conformance evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderMutationImplementationIdentityV1<'a> {
    /// Echo contract ABI version the implementation expects.
    pub echo_contract_abi_version: u32,
    /// Contract-host helper API version the implementation expects.
    pub helper_api_version: u32,
    /// Exact provider schema identity the implementation expects.
    pub provider_schema: ProviderSchemaIdentityV1<'a>,
    /// Exact target-bundle-profile identity the implementation expects.
    pub target_bundle_profile: ProviderDigestIdentityV1<'a>,
    /// Exact semantic and release bundle identities the implementation expects.
    pub bundle: ProviderBundleIdentityV1<'a>,
    /// Exact operation and target claims the implementation expects.
    pub operation: ProviderOperationV1<'a>,
}

/// One explicit host implementation and exact identity binding.
pub struct ProviderMutationHooksV1<'a> {
    identity: ProviderMutationImplementationIdentityV1<'a>,
    executor: ProviderMutationExecuteFnV1,
    compute_footprint: ProviderMutationFootprintFnV1,
}

impl<'a> ProviderMutationHooksV1<'a> {
    /// Bind an independently claimed implementation identity to one host type.
    ///
    /// The stored footprint callback always unions the host's semantic effect
    /// claim with Echo's mandatory runtime-ingress EINT read.
    #[must_use]
    pub const fn for_host<H: ProviderMutationHostV1>(
        identity: ProviderMutationImplementationIdentityV1<'a>,
    ) -> Self {
        Self {
            identity,
            executor: execute_provider_host::<H>,
            compute_footprint: compute_provider_host_footprint::<H>,
        }
    }

    /// Return the implementation identity asserted by the host.
    #[must_use]
    pub const fn identity(&self) -> &ProviderMutationImplementationIdentityV1<'a> {
        &self.identity
    }
}

/// Generated scheduler dispatch metadata for one mutation.
///
/// This is code-generator infrastructure. The proposal constructor validates
/// its operation and deterministic rule name before retaining its matcher.
#[doc(hidden)]
pub struct GeneratedProviderMutationDispatchV1 {
    operation_id: u32,
    rule_name: &'static str,
    matcher: ProviderMutationMatchFnV1,
}

impl GeneratedProviderMutationDispatchV1 {
    /// Construct generated dispatch metadata for proposal preflight.
    #[must_use]
    pub const fn new(
        operation_id: u32,
        rule_name: &'static str,
        matcher: ProviderMutationMatchFnV1,
    ) -> Self {
        Self {
            operation_id,
            rule_name,
            matcher,
        }
    }
}

/// Stable reason a provider package proposal failed before installation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderPackageProposalErrorKind {
    /// The host supplied no runtime package name.
    EmptyPackageName,
    /// The host supplied no runtime package version.
    EmptyPackageVersion,
    /// The host package artifact hash was not lowercase raw SHA-256.
    MalformedPackageArtifactHash,
    /// The registry does not describe exactly one supported operation.
    UnsupportedOperationCount,
    /// The registry operation is not a mutation supported by this constructor.
    UnsupportedOperationKind,
    /// The registry selected an Echo-reserved operation identifier.
    ReservedOperationId,
    /// Generated dispatch or host implementation operation claims differ.
    OperationMismatch,
    /// Echo contract ABI claims differ.
    EchoAbiMismatch,
    /// Contract-host helper API claims differ.
    HelperApiMismatch,
    /// Provider schema claims differ.
    SchemaMismatch,
    /// Semantic or release bundle claims differ.
    BundleMismatch,
    /// Target-bundle-profile claims differ.
    TargetBundleProfileMismatch,
    /// Target IR claims differ.
    TargetIrMismatch,
    /// Target-profile claims differ.
    TargetProfileMismatch,
    /// Generated-artifact-profile claims differ.
    GeneratedArtifactProfileMismatch,
    /// Input or output codec claims differ.
    CodecMismatch,
    /// Failure or typed-obstruction claims differ.
    ObstructionMismatch,
    /// Semantic operation-profile claims differ.
    OperationProfileMismatch,
    /// Footprint obligation or algebra claims differ.
    FootprintMismatch,
    /// Generated dispatch names a non-canonical rule identity.
    RuleIdentityMismatch,
}

/// Structured provider package proposal failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderPackageProposalError {
    kind: ProviderPackageProposalErrorKind,
    subject: String,
    reference: Option<String>,
}

impl ProviderPackageProposalError {
    fn new(
        kind: ProviderPackageProposalErrorKind,
        subject: impl Into<String>,
        reference: Option<String>,
    ) -> Self {
        Self {
            kind,
            subject: subject.into(),
            reference,
        }
    }

    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> ProviderPackageProposalErrorKind {
        self.kind
    }

    /// Return the stable subject that failed preflight.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Return the optional conflicting value or claim name.
    #[must_use]
    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }
}

impl fmt::Display for ProviderPackageProposalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider package proposal failed for {}",
            self.subject
        )?;
        if let Some(reference) = &self.reference {
            write!(formatter, " ({reference})")?;
        }
        Ok(())
    }
}

impl std::error::Error for ProviderPackageProposalError {}

/// Independently pinned Echo policy for admitting one provider proposal.
///
/// The policy is a runtime-owner claim. It does not derive expectations from
/// the proposal it evaluates and grants no installation or execution authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderContractAdmissionPolicyV1<'a> {
    /// Exact host-approved package occurrence claim.
    pub expected_occurrence: ContractPackageIdentity<'a>,
    /// Exact host-approved provider registry proposition.
    pub expected_registry: ProviderRegistryV1<'a>,
}

/// Stable reason an Echo provider-proposal admission failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderContractAdmissionErrorKind {
    /// The runtime package name differed from policy.
    PackageNameMismatch,
    /// The runtime package version differed from policy.
    PackageVersionMismatch,
    /// The exact package artifact hash differed from policy.
    PackageArtifactHashMismatch,
    /// The package occurrence differed in a field without a narrower classifier.
    PackageOccurrenceMismatch,
    /// The Echo contract ABI version differed from policy.
    EchoAbiMismatch,
    /// The contract-host helper API version differed from policy.
    HelperApiMismatch,
    /// The provider schema coordinate or exact raw digest differed from policy.
    ProviderSchemaMismatch,
    /// The target-bundle-profile identity differed from policy.
    TargetBundleProfileMismatch,
    /// The semantic bundle identity differed from policy.
    SemanticBundleMismatch,
    /// The release bundle identity differed from policy.
    ReleaseBundleMismatch,
    /// The provider operation set differed from policy.
    OperationSetMismatch,
    /// The provider registry differed in a field without a narrower classifier.
    RegistryMismatch,
}

/// Structured failure from Echo-owned provider-proposal admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderContractAdmissionError {
    kind: ProviderContractAdmissionErrorKind,
    subject: String,
    reference: Option<String>,
}

impl ProviderContractAdmissionError {
    #[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
    fn new(
        kind: ProviderContractAdmissionErrorKind,
        subject: &'static str,
        reference: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject: subject.to_owned(),
            reference: Some(reference.into()),
        }
    }

    #[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
    fn without_reference(kind: ProviderContractAdmissionErrorKind, subject: &'static str) -> Self {
        Self {
            kind,
            subject: subject.to_owned(),
            reference: None,
        }
    }

    /// Return the stable typed failure reason.
    #[must_use]
    pub const fn kind(&self) -> ProviderContractAdmissionErrorKind {
        self.kind
    }

    /// Return the stable proposition subject that failed admission.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Return the rejected value when one is available.
    #[must_use]
    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }
}

impl fmt::Display for ProviderContractAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider contract admission failed for {}",
            self.subject
        )?;
        if let Some(reference) = &self.reference {
            write!(formatter, " ({reference})")?;
        }
        Ok(())
    }
}

impl std::error::Error for ProviderContractAdmissionError {}

/// Opaque provider package proposal with no runtime installation authority.
pub struct ProviderContractPackageProposalV1<'a> {
    occurrence: ContractPackageIdentity<'a>,
    registry: ProviderRegistryV1<'a>,
    mutation_handler: ContractMutationHandler,
}

impl<'a> ProviderContractPackageProposalV1<'a> {
    /// Return the host-owned package occurrence claim.
    #[must_use]
    pub const fn occurrence(&self) -> &ContractPackageIdentity<'a> {
        &self.occurrence
    }

    /// Return the exact provider registry retained by the proposal.
    #[must_use]
    pub const fn registry(&self) -> &ProviderRegistryV1<'a> {
        &self.registry
    }

    /// Iterate the mutation operation identifiers retained by the proposal.
    pub fn mutation_operation_ids(&self) -> impl ExactSizeIterator<Item = u32> + '_ {
        core::iter::once(self.mutation_handler.op_id)
    }
}

/// Opaque provider package admitted by an Echo runtime owner.
///
/// This token retains the exact proposal material for a later trusted
/// installation crossing. It does not install handlers, mutate a registry,
/// schedule work, execute callbacks, or grant application authority.
pub struct AdmittedProviderContractPackageV1<'a> {
    proposal: ProviderContractPackageProposalV1<'a>,
}

impl<'a> AdmittedProviderContractPackageV1<'a> {
    /// Return the exact host-admitted package occurrence claim.
    #[must_use]
    pub const fn occurrence(&self) -> &ContractPackageIdentity<'a> {
        self.proposal.occurrence()
    }

    /// Return the exact host-admitted provider registry proposition.
    #[must_use]
    pub const fn registry(&self) -> &ProviderRegistryV1<'a> {
        self.proposal.registry()
    }

    /// Iterate the mutation operation identifiers retained for later installation.
    pub fn mutation_operation_ids(&self) -> impl ExactSizeIterator<Item = u32> + '_ {
        self.proposal.mutation_operation_ids()
    }
}

impl fmt::Debug for AdmittedProviderContractPackageV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmittedProviderContractPackageV1")
            .field("occurrence", self.occurrence())
            .field("registry", self.registry())
            .finish_non_exhaustive()
    }
}

/// Exact provider package root presented to the trusted installation boundary.
///
/// Constructing this value does not authenticate package bytes. The caller of
/// the trusted host boundary remains responsible for supplying independently
/// corroborated evidence for the coordinate and digest it names.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProviderPackageReferenceV1 {
    coordinate: String,
    digest: String,
}

impl ProviderPackageReferenceV1 {
    /// Construct an explicit provider package-root claim.
    #[must_use]
    pub fn new(coordinate: impl Into<String>, digest: impl Into<String>) -> Self {
        Self {
            coordinate: coordinate.into(),
            digest: digest.into(),
        }
    }

    /// Return the provider-owned package coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }

    /// Return the exact encoded package digest, including its algorithm prefix.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }
}

/// Deterministic identity for one installed provider-native contract package.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InstalledProviderContractPackageIdV1(crate::Hash);

impl InstalledProviderContractPackageIdV1 {
    /// Reconstruct an installed-provider package id from its canonical bytes.
    #[must_use]
    pub const fn from_bytes(bytes: crate::Hash) -> Self {
        Self(bytes)
    }

    /// Return the canonical byte representation.
    #[must_use]
    pub const fn as_bytes(&self) -> &crate::Hash {
        &self.0
    }
}

/// Owned occurrence metadata retained for an installed provider package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderContractPackageOccurrenceV1 {
    package_name: String,
    package_version: String,
    artifact_hash_hex: String,
}

impl InstalledProviderContractPackageOccurrenceV1 {
    #[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
    fn from_borrowed(value: ContractPackageIdentity<'_>) -> Self {
        Self {
            package_name: value.package_name.to_owned(),
            package_version: value.package_version.to_owned(),
            artifact_hash_hex: value.artifact_hash_hex.to_owned(),
        }
    }

    /// Return the host-owned package name.
    #[must_use]
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// Return the host-owned package version.
    #[must_use]
    pub fn package_version(&self) -> &str {
        &self.package_version
    }

    /// Return the exact lowercase raw SHA-256 package artifact hash.
    #[must_use]
    pub fn artifact_hash_hex(&self) -> &str {
        &self.artifact_hash_hex
    }
}

impl PartialEq<ContractPackageIdentity<'_>> for InstalledProviderContractPackageOccurrenceV1 {
    fn eq(&self, other: &ContractPackageIdentity<'_>) -> bool {
        self.package_name == other.package_name
            && self.package_version == other.package_version
            && self.artifact_hash_hex == other.artifact_hash_hex
    }
}

impl PartialEq<InstalledProviderContractPackageOccurrenceV1> for ContractPackageIdentity<'_> {
    fn eq(&self, other: &InstalledProviderContractPackageOccurrenceV1) -> bool {
        other == self
    }
}

/// Owned identity of one digest-framed provider artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderDigestIdentityV1 {
    coordinate: String,
    digest_domain: String,
    digest: String,
}

impl InstalledProviderDigestIdentityV1 {
    fn from_borrowed(value: ProviderDigestIdentityV1<'_>) -> Self {
        Self {
            coordinate: value.coordinate.to_owned(),
            digest_domain: value.digest_domain.to_owned(),
            digest: value.digest.to_owned(),
        }
    }

    pub(crate) fn from_owned_parts(
        coordinate: String,
        digest_domain: String,
        digest: String,
    ) -> Self {
        Self {
            coordinate,
            digest_domain,
            digest,
        }
    }

    /// Return the provider artifact coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }

    /// Return the proposition domain used to frame the digest.
    #[must_use]
    pub fn digest_domain(&self) -> &str {
        &self.digest_domain
    }

    /// Return the exact encoded digest value.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }
}

/// Owned identity of the provider schema retained at installation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderSchemaIdentityV1 {
    coordinate: String,
    raw_sha256_hex: String,
}

impl InstalledProviderSchemaIdentityV1 {
    fn from_borrowed(value: ProviderSchemaIdentityV1<'_>) -> Self {
        Self {
            coordinate: value.coordinate.to_owned(),
            raw_sha256_hex: value.raw_sha256_hex.to_owned(),
        }
    }

    /// Return the provider schema coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }

    /// Return the exact raw provider schema SHA-256 rendering.
    #[must_use]
    pub fn raw_sha256_hex(&self) -> &str {
        &self.raw_sha256_hex
    }
}

/// Owned coordinate and semantic domain of one provider-owned meaning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderSemanticIdentityV1 {
    coordinate: String,
    semantic_domain: String,
}

impl InstalledProviderSemanticIdentityV1 {
    fn from_borrowed(value: ProviderSemanticIdentityV1<'_>) -> Self {
        Self {
            coordinate: value.coordinate.to_owned(),
            semantic_domain: value.semantic_domain.to_owned(),
        }
    }

    /// Return the provider-owned semantic coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }

    /// Return the domain that defines the coordinate's meaning.
    #[must_use]
    pub fn semantic_domain(&self) -> &str {
        &self.semantic_domain
    }
}

/// Owned schema and codec claims for one operation value boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderValueContractV1 {
    schema_coordinate: String,
    schema_domain: String,
    codec_id: String,
}

impl InstalledProviderValueContractV1 {
    fn from_borrowed(value: ProviderValueContractV1<'_>) -> Self {
        Self {
            schema_coordinate: value.schema_coordinate.to_owned(),
            schema_domain: value.schema_domain.to_owned(),
            codec_id: value.codec_id.to_owned(),
        }
    }

    /// Return the semantic schema coordinate.
    #[must_use]
    pub fn schema_coordinate(&self) -> &str {
        &self.schema_coordinate
    }

    /// Return the semantic domain that defines the value schema.
    #[must_use]
    pub fn schema_domain(&self) -> &str {
        &self.schema_domain
    }

    /// Return the exact codec identifier selected for the value bytes.
    #[must_use]
    pub fn codec_id(&self) -> &str {
        &self.codec_id
    }
}

/// Owned semantic-layer and release-layer provider bundle identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderBundleIdentityV1 {
    semantic_digest_domain: String,
    semantic_digest: String,
    release_digest_domain: String,
    release_digest: String,
}

impl InstalledProviderBundleIdentityV1 {
    fn from_borrowed(value: ProviderBundleIdentityV1<'_>) -> Self {
        Self {
            semantic_digest_domain: value.semantic_digest_domain.to_owned(),
            semantic_digest: value.semantic_digest.to_owned(),
            release_digest_domain: value.release_digest_domain.to_owned(),
            release_digest: value.release_digest.to_owned(),
        }
    }

    /// Return the proposition domain framing the semantic bundle digest.
    #[must_use]
    pub fn semantic_digest_domain(&self) -> &str {
        &self.semantic_digest_domain
    }

    /// Return the exact semantic bundle digest.
    #[must_use]
    pub fn semantic_digest(&self) -> &str {
        &self.semantic_digest
    }

    /// Return the proposition domain framing the release bundle digest.
    #[must_use]
    pub fn release_digest_domain(&self) -> &str {
        &self.release_digest_domain
    }

    /// Return the exact release bundle digest.
    #[must_use]
    pub fn release_digest(&self) -> &str {
        &self.release_digest
    }
}

/// Owned abstract footprint obligation and owning algebra identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderFootprintIdentityV1 {
    obligation: String,
    algebra_coordinate: String,
    algebra_digest_domain: String,
    algebra_digest: String,
}

impl InstalledProviderFootprintIdentityV1 {
    fn from_borrowed(value: ProviderFootprintIdentityV1<'_>) -> Self {
        Self {
            obligation: value.obligation.to_owned(),
            algebra_coordinate: value.algebra_coordinate.to_owned(),
            algebra_digest_domain: value.algebra_digest_domain.to_owned(),
            algebra_digest: value.algebra_digest.to_owned(),
        }
    }

    /// Return the abstract footprint obligation authored for the operation.
    #[must_use]
    pub fn obligation(&self) -> &str {
        &self.obligation
    }

    /// Return the coordinate of the footprint algebra artifact.
    #[must_use]
    pub fn algebra_coordinate(&self) -> &str {
        &self.algebra_coordinate
    }

    /// Return the proposition domain framing the footprint algebra digest.
    #[must_use]
    pub fn algebra_digest_domain(&self) -> &str {
        &self.algebra_digest_domain
    }

    /// Return the exact footprint algebra digest.
    #[must_use]
    pub fn algebra_digest(&self) -> &str {
        &self.algebra_digest
    }
}

/// Complete owned semantic, generated, and target claims for one provider operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderOperationV1 {
    coordinate: String,
    semantic_domain: String,
    kind: OpKind,
    operation_id_law: String,
    operation_id: u32,
    input: InstalledProviderValueContractV1,
    output: InstalledProviderValueContractV1,
    target_failure_schema: String,
    obstruction: InstalledProviderSemanticIdentityV1,
    obstruction_payload_schema: String,
    target_ir: InstalledProviderDigestIdentityV1,
    target_profile: InstalledProviderDigestIdentityV1,
    generated_artifact_profile: InstalledProviderDigestIdentityV1,
    operation_profile: InstalledProviderSemanticIdentityV1,
    operation_profiles: InstalledProviderDigestIdentityV1,
    footprint: InstalledProviderFootprintIdentityV1,
}

impl InstalledProviderOperationV1 {
    fn from_borrowed(value: &ProviderOperationV1<'_>) -> Self {
        Self {
            coordinate: value.coordinate.to_owned(),
            semantic_domain: value.semantic_domain.to_owned(),
            kind: value.kind,
            operation_id_law: value.operation_id_law.to_owned(),
            operation_id: value.operation_id,
            input: InstalledProviderValueContractV1::from_borrowed(value.input),
            output: InstalledProviderValueContractV1::from_borrowed(value.output),
            target_failure_schema: value.target_failure_schema.to_owned(),
            obstruction: InstalledProviderSemanticIdentityV1::from_borrowed(value.obstruction),
            obstruction_payload_schema: value.obstruction_payload_schema.to_owned(),
            target_ir: InstalledProviderDigestIdentityV1::from_borrowed(value.target_ir),
            target_profile: InstalledProviderDigestIdentityV1::from_borrowed(value.target_profile),
            generated_artifact_profile: InstalledProviderDigestIdentityV1::from_borrowed(
                value.generated_artifact_profile,
            ),
            operation_profile: InstalledProviderSemanticIdentityV1::from_borrowed(
                value.operation_profile,
            ),
            operation_profiles: InstalledProviderDigestIdentityV1::from_borrowed(
                value.operation_profiles,
            ),
            footprint: InstalledProviderFootprintIdentityV1::from_borrowed(value.footprint),
        }
    }

    /// Return the semantic operation coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }

    /// Return the domain that defines the operation coordinate.
    #[must_use]
    pub fn semantic_domain(&self) -> &str {
        &self.semantic_domain
    }

    /// Return whether the operation mutates state or performs a bounded read.
    #[must_use]
    pub const fn kind(&self) -> OpKind {
        self.kind
    }

    /// Return the law used to derive the persisted operation identifier.
    #[must_use]
    pub fn operation_id_law(&self) -> &str {
        &self.operation_id_law
    }

    /// Return the exact persisted operation identifier.
    #[must_use]
    pub const fn operation_id(&self) -> u32 {
        self.operation_id
    }

    /// Return the operation input schema and codec claims.
    #[must_use]
    pub const fn input(&self) -> &InstalledProviderValueContractV1 {
        &self.input
    }

    /// Return the operation output schema and codec claims.
    #[must_use]
    pub const fn output(&self) -> &InstalledProviderValueContractV1 {
        &self.output
    }

    /// Return the target-owned failure schema.
    #[must_use]
    pub fn target_failure_schema(&self) -> &str {
        &self.target_failure_schema
    }

    /// Return the provider-owned typed obstruction identity.
    #[must_use]
    pub const fn obstruction(&self) -> &InstalledProviderSemanticIdentityV1 {
        &self.obstruction
    }

    /// Return the provider-owned obstruction payload schema.
    #[must_use]
    pub fn obstruction_payload_schema(&self) -> &str {
        &self.obstruction_payload_schema
    }

    /// Return the exact Target IR artifact identity.
    #[must_use]
    pub const fn target_ir(&self) -> &InstalledProviderDigestIdentityV1 {
        &self.target_ir
    }

    /// Return the exact target-profile artifact identity.
    #[must_use]
    pub const fn target_profile(&self) -> &InstalledProviderDigestIdentityV1 {
        &self.target_profile
    }

    /// Return the exact generated-artifact-profile identity.
    #[must_use]
    pub const fn generated_artifact_profile(&self) -> &InstalledProviderDigestIdentityV1 {
        &self.generated_artifact_profile
    }

    /// Return the semantic operation profile identity.
    #[must_use]
    pub const fn operation_profile(&self) -> &InstalledProviderSemanticIdentityV1 {
        &self.operation_profile
    }

    /// Return the exact operation-profiles document identity.
    #[must_use]
    pub const fn operation_profiles(&self) -> &InstalledProviderDigestIdentityV1 {
        &self.operation_profiles
    }

    /// Return the exact footprint obligation and algebra claims.
    #[must_use]
    pub const fn footprint(&self) -> &InstalledProviderFootprintIdentityV1 {
        &self.footprint
    }
}

/// Complete owned provider registry proposition retained by Echo.
///
/// This type mirrors [`ProviderRegistryV1`] without fabricating legacy Wesley,
/// GraphQL, installation, or executor metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderRegistryV1 {
    echo_contract_abi_version: u32,
    helper_api_version: u32,
    provider_schema: InstalledProviderSchemaIdentityV1,
    target_bundle_profile: InstalledProviderDigestIdentityV1,
    bundle: InstalledProviderBundleIdentityV1,
    operations: Vec<InstalledProviderOperationV1>,
}

impl InstalledProviderRegistryV1 {
    fn from_borrowed(value: &ProviderRegistryV1<'_>) -> Self {
        Self {
            echo_contract_abi_version: value.echo_contract_abi_version,
            helper_api_version: value.helper_api_version,
            provider_schema: InstalledProviderSchemaIdentityV1::from_borrowed(
                value.provider_schema,
            ),
            target_bundle_profile: InstalledProviderDigestIdentityV1::from_borrowed(
                value.target_bundle_profile,
            ),
            bundle: InstalledProviderBundleIdentityV1::from_borrowed(value.bundle),
            operations: value
                .operations
                .iter()
                .map(InstalledProviderOperationV1::from_borrowed)
                .collect(),
        }
    }

    /// Return the Echo contract ABI version targeted by generated helpers.
    #[must_use]
    pub const fn echo_contract_abi_version(&self) -> u32 {
        self.echo_contract_abi_version
    }

    /// Return the contract-host helper API version targeted by generated helpers.
    #[must_use]
    pub const fn helper_api_version(&self) -> u32 {
        self.helper_api_version
    }

    /// Return the exact raw provider schema identity.
    #[must_use]
    pub const fn provider_schema(&self) -> &InstalledProviderSchemaIdentityV1 {
        &self.provider_schema
    }

    /// Return the exact target-bundle-profile artifact identity.
    #[must_use]
    pub const fn target_bundle_profile(&self) -> &InstalledProviderDigestIdentityV1 {
        &self.target_bundle_profile
    }

    /// Return the semantic-layer and release-layer bundle identities.
    #[must_use]
    pub const fn bundle(&self) -> &InstalledProviderBundleIdentityV1 {
        &self.bundle
    }

    /// Return every provider operation retained at installation.
    #[must_use]
    pub fn operations(&self) -> &[InstalledProviderOperationV1] {
        &self.operations
    }

    /// Look up one uniquely identified provider operation.
    #[must_use]
    pub fn operation_by_id(&self, operation_id: u32) -> Option<&InstalledProviderOperationV1> {
        let mut found = None;
        for operation in &self.operations {
            if operation.operation_id != operation_id {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = Some(operation);
        }
        found
    }
}

impl PartialEq<ProviderRegistryV1<'_>> for InstalledProviderRegistryV1 {
    fn eq(&self, other: &ProviderRegistryV1<'_>) -> bool {
        self == &Self::from_borrowed(other)
    }
}

impl PartialEq<InstalledProviderRegistryV1> for ProviderRegistryV1<'_> {
    fn eq(&self, other: &InstalledProviderRegistryV1) -> bool {
        other == self
    }
}

/// Stable scheduler rule identity retained for one provider mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderMutationRuleIdentityV1 {
    operation_id: u32,
    rule_id: crate::Hash,
    rule_name: String,
}

impl InstalledProviderMutationRuleIdentityV1 {
    #[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
    fn from_handler(handler: &ContractMutationHandler) -> Self {
        Self {
            operation_id: handler.op_id,
            rule_id: handler.rule.id,
            rule_name: handler.rule.name.to_owned(),
        }
    }

    /// Return the mutation operation identifier handled by the rule.
    #[must_use]
    pub const fn operation_id(&self) -> u32 {
        self.operation_id
    }

    /// Return the deterministic scheduler rule identifier.
    #[must_use]
    pub const fn rule_id(&self) -> &crate::Hash {
        &self.rule_id
    }

    /// Return the deterministic scheduler rule name.
    #[must_use]
    pub fn rule_name(&self) -> &str {
        &self.rule_name
    }
}

/// Opaque installed provider-native contract package record.
///
/// The record owns the exact occurrence, caller-supplied package reference,
/// complete provider registry proposition, and scheduler rule identity. It
/// carries no legacy Wesley or GraphQL metadata and grants no authority beyond
/// the installation state maintained by its owning [`crate::Engine`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledProviderContractPackageRecordV1 {
    package_id: InstalledProviderContractPackageIdV1,
    occurrence: InstalledProviderContractPackageOccurrenceV1,
    package_reference: ProviderPackageReferenceV1,
    registry: InstalledProviderRegistryV1,
    mutation_rule: InstalledProviderMutationRuleIdentityV1,
    mutation_operation_ids: Vec<u32>,
}

impl InstalledProviderContractPackageRecordV1 {
    /// Return the deterministic installed provider package identifier.
    #[must_use]
    pub const fn package_id(&self) -> InstalledProviderContractPackageIdV1 {
        self.package_id
    }

    /// Return the exact host-admitted package occurrence.
    #[must_use]
    pub const fn occurrence(&self) -> &InstalledProviderContractPackageOccurrenceV1 {
        &self.occurrence
    }

    /// Return the exact provider package-root reference retained at installation.
    #[must_use]
    pub const fn package_reference(&self) -> &ProviderPackageReferenceV1 {
        &self.package_reference
    }

    /// Return the complete owned provider registry proposition.
    #[must_use]
    pub const fn registry(&self) -> &InstalledProviderRegistryV1 {
        &self.registry
    }

    /// Return the retained provider mutation scheduler-rule identity.
    #[must_use]
    pub const fn mutation_rule(&self) -> &InstalledProviderMutationRuleIdentityV1 {
        &self.mutation_rule
    }

    /// Iterate the mutation operation identifiers installed by this package.
    pub fn mutation_operation_ids(&self) -> impl ExactSizeIterator<Item = u32> + '_ {
        self.mutation_operation_ids.iter().copied()
    }

    #[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
    pub(crate) fn mutation_evidence_v1(
        &self,
        operation_id: u32,
    ) -> Option<ProviderContractEvidenceIdentityV1> {
        if self.mutation_rule.operation_id() != operation_id {
            return None;
        }
        let operation = self.registry.operations().iter().find(|operation| {
            operation.operation_id() == operation_id && operation.kind() == OpKind::Mutation
        })?;
        Some(ProviderContractEvidenceIdentityV1 {
            package_id: self.package_id,
            package_reference: self.package_reference.clone(),
            operation_id,
            operation_coordinate: operation.coordinate().to_owned(),
            target_ir: operation.target_ir().clone(),
            mutation_rule_id: *self.mutation_rule.rule_id(),
        })
    }
}

/// Provider-native installed-operation evidence attached to invocation receipts.
///
/// This value binds the installed provider package, exact package root,
/// semantic operation, Target IR, and scheduler rule used by Echo. It is
/// evidence metadata only and grants no admission, execution, or observation
/// authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderContractEvidenceIdentityV1 {
    package_id: InstalledProviderContractPackageIdV1,
    package_reference: ProviderPackageReferenceV1,
    operation_id: u32,
    operation_coordinate: String,
    target_ir: InstalledProviderDigestIdentityV1,
    mutation_rule_id: crate::Hash,
}

impl ProviderContractEvidenceIdentityV1 {
    pub(crate) fn try_from_retained_parts(
        package_id: InstalledProviderContractPackageIdV1,
        package_reference: ProviderPackageReferenceV1,
        operation_id: u32,
        operation_coordinate: String,
        target_ir: InstalledProviderDigestIdentityV1,
        mutation_rule_id: crate::Hash,
    ) -> Result<Self, &'static str> {
        if package_reference.coordinate().is_empty() {
            return Err("provider package reference coordinate");
        }
        if strict_prefixed_sha256(package_reference.digest()).is_none() {
            return Err("provider package reference digest");
        }
        if is_reserved_operation_id(operation_id) {
            return Err("provider operation id");
        }
        if operation_coordinate.is_empty() {
            return Err("provider operation coordinate");
        }
        if target_ir.coordinate().is_empty() {
            return Err("provider Target IR coordinate");
        }
        if target_ir.digest_domain().is_empty() {
            return Err("provider Target IR digest domain");
        }
        if strict_prefixed_sha256(target_ir.digest()).is_none() {
            return Err("provider Target IR digest");
        }
        Ok(Self {
            package_id,
            package_reference,
            operation_id,
            operation_coordinate,
            target_ir,
            mutation_rule_id,
        })
    }

    /// Return the deterministic installed provider package id.
    #[must_use]
    pub const fn package_id(&self) -> InstalledProviderContractPackageIdV1 {
        self.package_id
    }

    /// Return the exact provider package coordinate and digest.
    #[must_use]
    pub const fn package_reference(&self) -> &ProviderPackageReferenceV1 {
        &self.package_reference
    }

    /// Return the invoked provider operation id.
    #[must_use]
    pub const fn operation_id(&self) -> u32 {
        self.operation_id
    }

    /// Return the semantic provider operation coordinate.
    #[must_use]
    pub fn operation_coordinate(&self) -> &str {
        &self.operation_coordinate
    }

    /// Return the exact Target IR artifact identity bound at installation.
    #[must_use]
    pub const fn target_ir(&self) -> &InstalledProviderDigestIdentityV1 {
        &self.target_ir
    }

    /// Return the scheduler rule id bound to the installed mutation.
    #[must_use]
    pub const fn rule_id(&self) -> &crate::Hash {
        &self.mutation_rule_id
    }
}

/// Stable reason a provider-native contract package installation failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderContractInstallationErrorKind {
    /// The supplied provider package reference has an empty coordinate.
    EmptyPackageReferenceCoordinate,
    /// The supplied provider package reference is not strict lowercase SHA-256.
    MalformedPackageReferenceDigest,
    /// The referenced package root differs from the admitted occurrence hash.
    PackageArtifactDigestMismatch,
    /// The admitted proposal does not contain exactly one supported operation.
    UnsupportedOperationCount,
    /// The admitted provider operation is not a mutation.
    UnsupportedOperationKind,
    /// The retained handler names a different operation identifier.
    MutationHandlerOperationMismatch,
    /// The retained scheduler rule identity differs from the provider proposition.
    MutationRuleIdentityMismatch,
    /// The deterministic provider package id is already installed.
    DuplicatePackageId,
    /// The exact provider package-root coordinate and digest are already installed.
    DuplicatePackageReference,
    /// The provider operation identifier is already owned by a provider package.
    DuplicateProviderOperationId,
    /// The provider operation identifier conflicts with a legacy installed contract.
    LegacyOperationConflict,
    /// The provider scheduler rule name conflicts with an installed rule.
    DuplicateRuleName,
    /// The provider scheduler rule id conflicts with an installed rule.
    DuplicateRuleId,
    /// The retained provider mutation rule violates an engine registration invariant.
    InvalidMutationRule,
    /// Engine state changed unexpectedly after complete installation preflight.
    InternalRegistrationFailure,
}

/// Structured provider-native contract package installation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderContractInstallationError {
    kind: ProviderContractInstallationErrorKind,
    subject: String,
    reference: Option<String>,
}

impl ProviderContractInstallationError {
    #[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
    pub(crate) fn new(
        kind: ProviderContractInstallationErrorKind,
        subject: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject: subject.into(),
            reference: Some(reference.into()),
        }
    }

    #[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
    pub(crate) fn without_reference(
        kind: ProviderContractInstallationErrorKind,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            subject: subject.into(),
            reference: None,
        }
    }

    /// Return the stable typed failure kind.
    #[must_use]
    pub const fn kind(&self) -> ProviderContractInstallationErrorKind {
        self.kind
    }

    /// Return the stable installation proposition that failed.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Return the offending identity, when one is available.
    #[must_use]
    pub fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }
}

impl fmt::Display for ProviderContractInstallationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider contract installation failed for {}",
            self.subject
        )?;
        if let Some(reference) = &self.reference {
            write!(formatter, " ({reference})")?;
        }
        Ok(())
    }
}

impl std::error::Error for ProviderContractInstallationError {}

/// Internal seal for Echo-selected provider-package installation owners.
///
/// This remains crate-visible so the trusted runtime host can implement the
/// public bridge trait without allowing downstream crates to manufacture an
/// installer that reports success without mutating Echo's owned registry.
pub(crate) trait SealedProviderContractPackageInstallerV1 {}

/// Echo-selected runtime-owner port for provider-native package installation.
///
/// The normal caller is
/// `echo_wesley_gen::provider_package::install_digest_corroborated_provider_contract_package_v1`,
/// which consumes exact package corroboration before invoking this port. The
/// lower port itself does not inspect or authenticate package bytes.
#[doc(hidden)]
#[allow(private_bounds)]
pub trait ProviderContractPackageInstallerV1: SealedProviderContractPackageInstallerV1 {
    /// Install one host-admitted provider package under a caller-supplied package root.
    ///
    /// # Errors
    ///
    /// Returns a structured installation failure when the package-root claim
    /// is malformed or disagrees with admission, or when the package conflicts
    /// with existing Echo-owned registry state.
    fn install_admitted_provider_contract_package_v1_trusted(
        &mut self,
        package_reference: ProviderPackageReferenceV1,
        admitted: AdmittedProviderContractPackageV1<'_>,
    ) -> Result<InstalledProviderContractPackageRecordV1, ProviderContractInstallationError>;
}

/// Provider-native installation material validated before Engine mutation.
#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
pub(crate) struct PreparedInstalledProviderContractPackageV1 {
    pub(crate) record: InstalledProviderContractPackageRecordV1,
    pub(crate) mutation_handler: ContractMutationHandler,
}

/// Prepare one admitted provider proposal for atomic Engine installation.
///
/// This pure crossing validates the explicit package-root claim, owns the full
/// admitted provider proposition, and derives a deterministic installed id. It
/// does not authenticate bytes, invoke provider callbacks, or mutate Engine
/// state.
#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
pub(crate) fn prepare_installed_provider_contract_package_v1(
    package_reference: ProviderPackageReferenceV1,
    admitted: AdmittedProviderContractPackageV1<'_>,
) -> Result<PreparedInstalledProviderContractPackageV1, ProviderContractInstallationError> {
    if package_reference.coordinate.is_empty() {
        return Err(ProviderContractInstallationError::without_reference(
            ProviderContractInstallationErrorKind::EmptyPackageReferenceCoordinate,
            "provider.package.reference.coordinate",
        ));
    }
    let Some(reference_raw_sha256) = strict_prefixed_sha256(&package_reference.digest) else {
        return Err(ProviderContractInstallationError::new(
            ProviderContractInstallationErrorKind::MalformedPackageReferenceDigest,
            "provider.package.reference.digest",
            package_reference.digest,
        ));
    };

    let ProviderContractPackageProposalV1 {
        occurrence,
        registry,
        mutation_handler,
    } = admitted.proposal;
    if reference_raw_sha256 != occurrence.artifact_hash_hex {
        return Err(ProviderContractInstallationError::new(
            ProviderContractInstallationErrorKind::PackageArtifactDigestMismatch,
            "provider.package.occurrence.artifact-hash",
            occurrence.artifact_hash_hex,
        ));
    }
    if registry.operations.len() != 1 {
        return Err(ProviderContractInstallationError::new(
            ProviderContractInstallationErrorKind::UnsupportedOperationCount,
            "provider.registry.operations",
            registry.operations.len().to_string(),
        ));
    }
    let operation = registry.operations[0];
    if operation.kind != OpKind::Mutation {
        return Err(ProviderContractInstallationError::new(
            ProviderContractInstallationErrorKind::UnsupportedOperationKind,
            operation.coordinate,
            provider_op_kind_label(operation.kind),
        ));
    }
    if mutation_handler.op_id != operation.operation_id {
        return Err(ProviderContractInstallationError::new(
            ProviderContractInstallationErrorKind::MutationHandlerOperationMismatch,
            operation.coordinate,
            mutation_handler.op_id.to_string(),
        ));
    }
    let expected_rule_name = format!(
        "cmd/contract/{}/{}/{}",
        registry.provider_schema.raw_sha256_hex, operation.operation_id, operation.coordinate
    );
    if mutation_handler.rule.name != expected_rule_name {
        return Err(ProviderContractInstallationError::new(
            ProviderContractInstallationErrorKind::MutationRuleIdentityMismatch,
            "provider.mutation.rule.name",
            mutation_handler.rule.name,
        ));
    }
    if mutation_handler.rule.id != make_type_id(&expected_rule_name).0 {
        return Err(ProviderContractInstallationError::without_reference(
            ProviderContractInstallationErrorKind::MutationRuleIdentityMismatch,
            "provider.mutation.rule.id",
        ));
    }

    let occurrence = InstalledProviderContractPackageOccurrenceV1::from_borrowed(occurrence);
    let registry = InstalledProviderRegistryV1::from_borrowed(&registry);
    let mutation_rule = InstalledProviderMutationRuleIdentityV1::from_handler(&mutation_handler);
    let package_id = installed_provider_contract_package_id_v1(
        &package_reference,
        &occurrence,
        &registry,
        &mutation_rule,
    );
    let mutation_operation_ids = vec![operation.operation_id];
    let record = InstalledProviderContractPackageRecordV1 {
        package_id,
        occurrence,
        package_reference,
        registry,
        mutation_rule,
        mutation_operation_ids,
    };

    Ok(PreparedInstalledProviderContractPackageV1 {
        record,
        mutation_handler,
    })
}

fn strict_prefixed_sha256(value: &str) -> Option<&str> {
    let raw = value.strip_prefix("sha256:")?;
    (is_raw_sha256(raw)).then_some(raw)
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
const fn provider_op_kind_label(kind: OpKind) -> &'static str {
    match kind {
        OpKind::Mutation => "mutation",
        OpKind::Query => "query",
    }
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn installed_provider_contract_package_id_v1(
    package_reference: &ProviderPackageReferenceV1,
    occurrence: &InstalledProviderContractPackageOccurrenceV1,
    registry: &InstalledProviderRegistryV1,
    mutation_rule: &InstalledProviderMutationRuleIdentityV1,
) -> InstalledProviderContractPackageIdV1 {
    let mut hasher = Hasher::new();
    hasher.update(INSTALLED_PROVIDER_CONTRACT_PACKAGE_ID_DOMAIN);
    hash_text(&mut hasher, package_reference.coordinate());
    hash_text(&mut hasher, package_reference.digest());
    hash_text(&mut hasher, occurrence.package_name());
    hash_text(&mut hasher, occurrence.package_version());
    hash_text(&mut hasher, occurrence.artifact_hash_hex());
    hash_u32(&mut hasher, registry.echo_contract_abi_version());
    hash_u32(&mut hasher, registry.helper_api_version());
    hash_provider_schema(&mut hasher, registry.provider_schema());
    hash_provider_digest(&mut hasher, registry.target_bundle_profile());
    hash_provider_bundle(&mut hasher, registry.bundle());
    hash_u64(&mut hasher, registry.operations().len() as u64);
    for operation in registry.operations() {
        hash_provider_operation(&mut hasher, operation);
    }
    hash_u32(&mut hasher, mutation_rule.operation_id());
    hash_bytes(&mut hasher, mutation_rule.rule_id());
    hash_text(&mut hasher, mutation_rule.rule_name());
    InstalledProviderContractPackageIdV1::from_bytes(hasher.finalize().into())
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_provider_operation(hasher: &mut Hasher, value: &InstalledProviderOperationV1) {
    hash_text(hasher, value.coordinate());
    hash_text(hasher, value.semantic_domain());
    hash_text(hasher, provider_op_kind_label(value.kind()));
    hash_text(hasher, value.operation_id_law());
    hash_u32(hasher, value.operation_id());
    hash_provider_value_contract(hasher, value.input());
    hash_provider_value_contract(hasher, value.output());
    hash_text(hasher, value.target_failure_schema());
    hash_provider_semantic(hasher, value.obstruction());
    hash_text(hasher, value.obstruction_payload_schema());
    hash_provider_digest(hasher, value.target_ir());
    hash_provider_digest(hasher, value.target_profile());
    hash_provider_digest(hasher, value.generated_artifact_profile());
    hash_provider_semantic(hasher, value.operation_profile());
    hash_provider_digest(hasher, value.operation_profiles());
    hash_provider_footprint(hasher, value.footprint());
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_provider_digest(hasher: &mut Hasher, value: &InstalledProviderDigestIdentityV1) {
    hash_text(hasher, value.coordinate());
    hash_text(hasher, value.digest_domain());
    hash_text(hasher, value.digest());
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_provider_schema(hasher: &mut Hasher, value: &InstalledProviderSchemaIdentityV1) {
    hash_text(hasher, value.coordinate());
    hash_text(hasher, value.raw_sha256_hex());
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_provider_semantic(hasher: &mut Hasher, value: &InstalledProviderSemanticIdentityV1) {
    hash_text(hasher, value.coordinate());
    hash_text(hasher, value.semantic_domain());
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_provider_value_contract(hasher: &mut Hasher, value: &InstalledProviderValueContractV1) {
    hash_text(hasher, value.schema_coordinate());
    hash_text(hasher, value.schema_domain());
    hash_text(hasher, value.codec_id());
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_provider_bundle(hasher: &mut Hasher, value: &InstalledProviderBundleIdentityV1) {
    hash_text(hasher, value.semantic_digest_domain());
    hash_text(hasher, value.semantic_digest());
    hash_text(hasher, value.release_digest_domain());
    hash_text(hasher, value.release_digest());
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_provider_footprint(hasher: &mut Hasher, value: &InstalledProviderFootprintIdentityV1) {
    hash_text(hasher, value.obligation());
    hash_text(hasher, value.algebra_coordinate());
    hash_text(hasher, value.algebra_digest_domain());
    hash_text(hasher, value.algebra_digest());
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_text(hasher: &mut Hasher, value: &str) {
    hash_bytes(hasher, value.as_bytes());
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_u32(hasher: &mut Hasher, value: u32) {
    hash_bytes(hasher, &value.to_le_bytes());
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_u64(hasher: &mut Hasher, value: u64) {
    hash_bytes(hasher, &value.to_le_bytes());
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn hash_bytes(hasher: &mut Hasher, value: &[u8]) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value);
}

#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
fn provider_operation_set_reference(operations: &[ProviderOperationV1<'_>]) -> String {
    if operations.is_empty() {
        return "<empty>".to_owned();
    }
    operations
        .iter()
        .map(|operation| format!("{}#{}", operation.coordinate, operation.operation_id))
        .collect::<Vec<_>>()
        .join(",")
}

/// Admit one exact provider proposal under independently supplied Echo policy.
///
/// This pure validator compares the complete occurrence and provider-registry
/// proposition before retaining the opaque proposal. It performs no registry,
/// scheduler, filesystem, environment, process, clock, randomness, or network
/// operation.
#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
pub(crate) fn admit_provider_contract_package_v1<'a>(
    policy: &ProviderContractAdmissionPolicyV1<'_>,
    proposal: ProviderContractPackageProposalV1<'a>,
) -> Result<AdmittedProviderContractPackageV1<'a>, ProviderContractAdmissionError> {
    let actual_occurrence = proposal.occurrence();
    let expected_occurrence = &policy.expected_occurrence;
    if actual_occurrence.package_name != expected_occurrence.package_name {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::PackageNameMismatch,
            "provider.package.name",
            actual_occurrence.package_name,
        ));
    }
    if actual_occurrence.package_version != expected_occurrence.package_version {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::PackageVersionMismatch,
            "provider.package.version",
            actual_occurrence.package_version,
        ));
    }
    if actual_occurrence.artifact_hash_hex != expected_occurrence.artifact_hash_hex {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::PackageArtifactHashMismatch,
            "provider.package.artifact-hash",
            actual_occurrence.artifact_hash_hex,
        ));
    }
    // Whole-value equality is the authoritative gate. The classifiers above
    // preserve useful current diagnostics; this fallback makes future fields
    // fail closed until they receive an intentional stable classification.
    if actual_occurrence != expected_occurrence {
        return Err(ProviderContractAdmissionError::without_reference(
            ProviderContractAdmissionErrorKind::PackageOccurrenceMismatch,
            "provider.package.occurrence",
        ));
    }

    let actual = proposal.registry();
    let expected = &policy.expected_registry;
    if actual.echo_contract_abi_version != expected.echo_contract_abi_version {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::EchoAbiMismatch,
            "provider.registry.echo-contract-abi-version",
            actual.echo_contract_abi_version.to_string(),
        ));
    }
    if actual.helper_api_version != expected.helper_api_version {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::HelperApiMismatch,
            "provider.registry.helper-api-version",
            actual.helper_api_version.to_string(),
        ));
    }
    if actual.provider_schema.coordinate != expected.provider_schema.coordinate {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::ProviderSchemaMismatch,
            "provider.registry.schema.coordinate",
            actual.provider_schema.coordinate,
        ));
    }
    if actual.provider_schema.raw_sha256_hex != expected.provider_schema.raw_sha256_hex {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::ProviderSchemaMismatch,
            "provider.registry.schema.raw-sha256",
            actual.provider_schema.raw_sha256_hex,
        ));
    }
    if actual.target_bundle_profile.coordinate != expected.target_bundle_profile.coordinate {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::TargetBundleProfileMismatch,
            "provider.registry.target-bundle-profile.coordinate",
            actual.target_bundle_profile.coordinate,
        ));
    }
    if actual.target_bundle_profile.digest_domain != expected.target_bundle_profile.digest_domain {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::TargetBundleProfileMismatch,
            "provider.registry.target-bundle-profile.digest-domain",
            actual.target_bundle_profile.digest_domain,
        ));
    }
    if actual.target_bundle_profile.digest != expected.target_bundle_profile.digest {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::TargetBundleProfileMismatch,
            "provider.registry.target-bundle-profile.digest",
            actual.target_bundle_profile.digest,
        ));
    }
    if actual.bundle.semantic_digest_domain != expected.bundle.semantic_digest_domain {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::SemanticBundleMismatch,
            "provider.registry.bundle.semantic",
            actual.bundle.semantic_digest_domain,
        ));
    }
    if actual.bundle.semantic_digest != expected.bundle.semantic_digest {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::SemanticBundleMismatch,
            "provider.registry.bundle.semantic",
            actual.bundle.semantic_digest,
        ));
    }
    if actual.bundle.release_digest_domain != expected.bundle.release_digest_domain {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::ReleaseBundleMismatch,
            "provider.registry.bundle.release",
            actual.bundle.release_digest_domain,
        ));
    }
    if actual.bundle.release_digest != expected.bundle.release_digest {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::ReleaseBundleMismatch,
            "provider.registry.bundle.release",
            actual.bundle.release_digest,
        ));
    }
    if actual.operations != expected.operations {
        return Err(ProviderContractAdmissionError::new(
            ProviderContractAdmissionErrorKind::OperationSetMismatch,
            "provider.registry.operations",
            provider_operation_set_reference(actual.operations),
        ));
    }
    // As above, admit only the complete registry proposition, including fields
    // added after this classifier list was written.
    if actual != expected {
        return Err(ProviderContractAdmissionError::without_reference(
            ProviderContractAdmissionErrorKind::RegistryMismatch,
            "provider.registry",
        ));
    }

    Ok(AdmittedProviderContractPackageV1 { proposal })
}

/// Construct one pure provider mutation package proposal.
///
/// The function compares every generated registry claim with the independently
/// supplied host implementation identity before materializing a private,
/// conservative scheduler rule. It performs no registration or runtime I/O.
///
/// # Errors
///
/// Returns [`ProviderPackageProposalError`] when occurrence metadata is
/// malformed, the registry is not exactly one mutation, any identity differs,
/// or generated dispatch metadata is inconsistent.
pub fn propose_provider_contract_package_v1<'a>(
    occurrence: ContractPackageIdentity<'a>,
    registry: ProviderRegistryV1<'a>,
    dispatch: GeneratedProviderMutationDispatchV1,
    hooks: ProviderMutationHooksV1<'a>,
) -> Result<ProviderContractPackageProposalV1<'a>, ProviderPackageProposalError> {
    validate_occurrence(occurrence)?;
    if registry.operations.len() != 1 {
        return Err(ProviderPackageProposalError::new(
            ProviderPackageProposalErrorKind::UnsupportedOperationCount,
            "registry.operations",
            Some(registry.operations.len().to_string()),
        ));
    }

    let operation = &registry.operations[0];
    if operation.kind != OpKind::Mutation {
        return Err(ProviderPackageProposalError::new(
            ProviderPackageProposalErrorKind::UnsupportedOperationKind,
            operation.coordinate,
            Some("query".to_owned()),
        ));
    }
    if is_reserved_operation_id(operation.operation_id) {
        return Err(ProviderPackageProposalError::new(
            ProviderPackageProposalErrorKind::ReservedOperationId,
            operation.coordinate,
            Some(operation.operation_id.to_string()),
        ));
    }
    if dispatch.operation_id != operation.operation_id {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::OperationMismatch,
            operation,
            "dispatch.operationId",
        ));
    }

    validate_implementation_identity(&registry, operation, &hooks.identity)?;
    let expected_rule_name = format!(
        "cmd/contract/{}/{}/{}",
        registry.provider_schema.raw_sha256_hex, operation.operation_id, operation.coordinate
    );
    if dispatch.rule_name != expected_rule_name {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::RuleIdentityMismatch,
            operation,
            dispatch.rule_name,
        ));
    }

    let mutation_handler = materialize_mutation_handler(
        operation.operation_id,
        dispatch,
        hooks.executor,
        hooks.compute_footprint,
    );

    Ok(ProviderContractPackageProposalV1 {
        occurrence,
        registry,
        mutation_handler,
    })
}

fn materialize_mutation_handler(
    operation_id: u32,
    dispatch: GeneratedProviderMutationDispatchV1,
    executor: ProviderMutationExecuteFnV1,
    compute_footprint: ProviderMutationFootprintFnV1,
) -> ContractMutationHandler {
    ContractMutationHandler {
        op_id: operation_id,
        rule: RewriteRule {
            id: make_type_id(dispatch.rule_name).0,
            name: dispatch.rule_name,
            left: PatternGraph { nodes: Vec::new() },
            matcher: dispatch.matcher,
            executor,
            compute_footprint,
            factor_mask: u64::MAX,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        },
    }
}

fn validate_occurrence(
    occurrence: ContractPackageIdentity<'_>,
) -> Result<(), ProviderPackageProposalError> {
    if occurrence.package_name.is_empty() {
        return Err(ProviderPackageProposalError::new(
            ProviderPackageProposalErrorKind::EmptyPackageName,
            "package.name",
            None,
        ));
    }
    if occurrence.package_version.is_empty() {
        return Err(ProviderPackageProposalError::new(
            ProviderPackageProposalErrorKind::EmptyPackageVersion,
            "package.version",
            None,
        ));
    }
    if !is_raw_sha256(occurrence.artifact_hash_hex) {
        return Err(ProviderPackageProposalError::new(
            ProviderPackageProposalErrorKind::MalformedPackageArtifactHash,
            "package.artifactHash",
            Some(occurrence.artifact_hash_hex.to_owned()),
        ));
    }
    Ok(())
}

fn validate_implementation_identity(
    registry: &ProviderRegistryV1<'_>,
    operation: &ProviderOperationV1<'_>,
    identity: &ProviderMutationImplementationIdentityV1<'_>,
) -> Result<(), ProviderPackageProposalError> {
    if identity.echo_contract_abi_version != registry.echo_contract_abi_version {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::EchoAbiMismatch,
            operation,
            "echoContractAbiVersion",
        ));
    }
    if identity.helper_api_version != registry.helper_api_version {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::HelperApiMismatch,
            operation,
            "helperApiVersion",
        ));
    }
    if identity.provider_schema != registry.provider_schema {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::SchemaMismatch,
            operation,
            "providerSchema",
        ));
    }
    if identity.target_bundle_profile != registry.target_bundle_profile {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::TargetBundleProfileMismatch,
            operation,
            "targetBundleProfile",
        ));
    }
    if identity.bundle != registry.bundle {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::BundleMismatch,
            operation,
            "bundle",
        ));
    }
    compare_operation_identity(operation, &identity.operation)
}

fn compare_operation_identity(
    expected: &ProviderOperationV1<'_>,
    actual: &ProviderOperationV1<'_>,
) -> Result<(), ProviderPackageProposalError> {
    if expected.coordinate != actual.coordinate
        || expected.semantic_domain != actual.semantic_domain
        || expected.kind != actual.kind
        || expected.operation_id_law != actual.operation_id_law
        || expected.operation_id != actual.operation_id
    {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::OperationMismatch,
            expected,
            "operation",
        ));
    }
    if expected.input.codec_id != actual.input.codec_id
        || expected.output.codec_id != actual.output.codec_id
    {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::CodecMismatch,
            expected,
            "valueCodec",
        ));
    }
    if expected.input.schema_coordinate != actual.input.schema_coordinate
        || expected.input.schema_domain != actual.input.schema_domain
        || expected.output.schema_coordinate != actual.output.schema_coordinate
        || expected.output.schema_domain != actual.output.schema_domain
    {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::SchemaMismatch,
            expected,
            "operationSchema",
        ));
    }
    if expected.target_ir != actual.target_ir {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::TargetIrMismatch,
            expected,
            "targetIr",
        ));
    }
    if expected.target_profile != actual.target_profile {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::TargetProfileMismatch,
            expected,
            "targetProfile",
        ));
    }
    if expected.generated_artifact_profile != actual.generated_artifact_profile {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::GeneratedArtifactProfileMismatch,
            expected,
            "generatedArtifactProfile",
        ));
    }
    if expected.target_failure_schema != actual.target_failure_schema
        || expected.obstruction != actual.obstruction
        || expected.obstruction_payload_schema != actual.obstruction_payload_schema
    {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::ObstructionMismatch,
            expected,
            "obstruction",
        ));
    }
    if expected.operation_profile != actual.operation_profile
        || expected.operation_profiles != actual.operation_profiles
    {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::OperationProfileMismatch,
            expected,
            "operationProfile",
        ));
    }
    if expected.footprint != actual.footprint {
        return Err(mismatch(
            ProviderPackageProposalErrorKind::FootprintMismatch,
            expected,
            "footprint",
        ));
    }
    Ok(())
}

fn mismatch(
    kind: ProviderPackageProposalErrorKind,
    operation: &ProviderOperationV1<'_>,
    reference: &str,
) -> ProviderPackageProposalError {
    ProviderPackageProposalError::new(kind, operation.coordinate, Some(reference.to_owned()))
}

fn is_raw_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher(_view: GraphView<'_>, _scope: &NodeId) -> bool {
        false
    }

    fn executor(_view: GraphView<'_>, _scope: &NodeId, _delta: &mut TickDelta) {}

    fn footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
        let mut footprint = Footprint::default();
        footprint.n_write.insert_with_warp(view.warp_id(), *scope);
        footprint
    }

    struct TestHost;

    impl ProviderMutationHostV1 for TestHost {
        fn execute(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
            executor(view, scope, delta);
        }

        fn effect_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
            footprint(view, scope)
        }
    }

    #[test]
    fn provider_proposal_materializes_a_conservative_private_rule() {
        let handler = materialize_mutation_handler(
            7,
            GeneratedProviderMutationDispatchV1::new(7, "cmd/provider/test", matcher),
            execute_provider_host::<TestHost>,
            compute_provider_host_footprint::<TestHost>,
        );

        assert_eq!(handler.op_id, 7);
        assert_eq!(handler.rule.id, make_type_id("cmd/provider/test").0);
        assert_eq!(handler.rule.name, "cmd/provider/test");
        assert!(handler.rule.left.nodes.is_empty());
        assert!(std::ptr::fn_addr_eq(
            handler.rule.matcher,
            matcher as ProviderMutationMatchFnV1
        ));
        assert!(std::ptr::fn_addr_eq(
            handler.rule.executor,
            execute_provider_host::<TestHost> as ProviderMutationExecuteFnV1
        ));
        assert!(std::ptr::fn_addr_eq(
            handler.rule.compute_footprint,
            compute_provider_host_footprint::<TestHost> as ProviderMutationFootprintFnV1
        ));
        let warp_id = crate::ident::WarpId([1; 32]);
        let scope = NodeId([2; 32]);
        let mut store = crate::graph::GraphStore::new(warp_id);
        store.insert_node(
            scope,
            crate::record::NodeRecord {
                ty: make_type_id("provider-test-ingress"),
            },
        );
        let declared = (handler.rule.compute_footprint)(GraphView::new(&store), &scope);
        assert_eq!(declared.n_read.len(), 1);
        assert_eq!(declared.a_read.len(), 1);
        assert_eq!(declared.n_write.len(), 1);
        assert_eq!(declared.factor_mask, u64::MAX);
        assert_eq!(handler.rule.factor_mask, u64::MAX);
        assert!(matches!(
            handler.rule.conflict_policy,
            ConflictPolicy::Abort
        ));
        assert!(handler.rule.join_fn.is_none());
    }
}
