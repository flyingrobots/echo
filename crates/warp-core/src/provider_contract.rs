// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Provider-generic, non-installing contract package proposals.
//!
//! A proposal retains exact generated identities and one explicit host
//! implementation binding. It grants no admission, installation, scheduling,
//! execution, durability, receipt, or observation authority.

use core::fmt;

use echo_registry_api::{
    is_reserved_operation_id, OpKind, ProviderBundleIdentityV1, ProviderDigestIdentityV1,
    ProviderOperationV1, ProviderRegistryV1, ProviderSchemaIdentityV1,
};

use crate::contract_host::runtime_ingress_eint_read_footprint;
use crate::contract_registry::{ContractMutationHandler, ContractPackageIdentity};
use crate::footprint::Footprint;
use crate::graph_view::GraphView;
use crate::ident::{make_type_id, NodeId};
use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};
use crate::TickDelta;

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
