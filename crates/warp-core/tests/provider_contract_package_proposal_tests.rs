// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Non-authoritative provider package proposal witnesses.

use echo_registry_api::{
    OpKind, ProviderBundleIdentityV1, ProviderDigestIdentityV1, ProviderFootprintIdentityV1,
    ProviderOperationV1, ProviderRegistryV1, ProviderSchemaIdentityV1, ProviderSemanticIdentityV1,
    ProviderValueContractV1, CONTRACT_HOST_HELPER_API_VERSION, ECHO_CONTRACT_ABI_VERSION,
    LITTLE_ENDIAN_CODEC_V1_ID, RESERVED_CONTROL_OPERATION_ID, SEMANTIC_OPERATION_ID_LAW_V1,
};
use warp_core::{
    propose_provider_contract_package_v1, ContractPackageIdentity, Footprint,
    GeneratedProviderMutationDispatchV1, GraphView, NodeId, ProviderMutationHooksV1,
    ProviderMutationHostV1, ProviderMutationImplementationIdentityV1,
    ProviderPackageProposalErrorKind, TickDelta,
};

use std::sync::atomic::{AtomicUsize, Ordering};

const OPERATION_ID: u32 = 3_389_142_194;
const RAW_SCHEMA_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const DIGEST: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const RULE_NAME: &str = concat!(
    "cmd/contract/",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "/3389142194/a.b@1.t"
);

const TARGET_IR: ProviderDigestIdentityV1<'static> = ProviderDigestIdentityV1 {
    coordinate: "echo.span-ir/v1",
    digest_domain: "edict.target-ir.artifact/v1",
    digest: DIGEST,
};
const TARGET_PROFILE: ProviderDigestIdentityV1<'static> = ProviderDigestIdentityV1 {
    coordinate: "echo.dpo@1",
    digest_domain: "edict.target-profile/v1",
    digest: DIGEST,
};
const GENERATED_PROFILE: ProviderDigestIdentityV1<'static> = ProviderDigestIdentityV1 {
    coordinate: "echo.dpo.registration/v1",
    digest_domain: "echo.generated-artifact-profile/v1",
    digest: DIGEST,
};
const OPERATION_PROFILES: ProviderDigestIdentityV1<'static> = ProviderDigestIdentityV1 {
    coordinate: "echo.dpo.operation-profiles/v1",
    digest_domain: "echo.dpo.operation-profiles/v1",
    digest: DIGEST,
};
const TARGET_BUNDLE_PROFILE: ProviderDigestIdentityV1<'static> = ProviderDigestIdentityV1 {
    coordinate: "echo.dpo.bundle/v1",
    digest_domain: "echo.dpo.bundle/v1",
    digest: DIGEST,
};
const PROVIDER_SCHEMA: ProviderSchemaIdentityV1<'static> = ProviderSchemaIdentityV1 {
    coordinate: "echo.provider-artifacts.cddl@1",
    raw_sha256_hex: RAW_SCHEMA_SHA256,
};
const BUNDLE: ProviderBundleIdentityV1<'static> = ProviderBundleIdentityV1 {
    semantic_digest_domain: "edict.bundle.semantic/v1",
    semantic_digest: DIGEST,
    release_digest_domain: "edict.bundle.release/v1",
    release_digest: DIGEST,
};
const OPERATION: ProviderOperationV1<'static> = ProviderOperationV1 {
    coordinate: "a.b@1.t",
    semantic_domain: "echo.edict-provider/operation/v1",
    kind: OpKind::Mutation,
    operation_id_law: SEMANTIC_OPERATION_ID_LAW_V1,
    operation_id: OPERATION_ID,
    input: ProviderValueContractV1 {
        schema_coordinate: "a.b@1.Input",
        schema_domain: "echo.edict-provider/value/v1",
        codec_id: LITTLE_ENDIAN_CODEC_V1_ID,
    },
    output: ProviderValueContractV1 {
        schema_coordinate: "a.b@1.Output",
        schema_domain: "echo.edict-provider/value/v1",
        codec_id: LITTLE_ENDIAN_CODEC_V1_ID,
    },
    target_failure_schema: "target.replace.rejected",
    obstruction: ProviderSemanticIdentityV1 {
        coordinate: "domain.WriteRejected",
        semantic_domain: "echo.edict-provider/obstruction/v1",
    },
    obstruction_payload_schema: "domain.WriteRejected.Payload",
    target_ir: TARGET_IR,
    target_profile: TARGET_PROFILE,
    generated_artifact_profile: GENERATED_PROFILE,
    operation_profile: ProviderSemanticIdentityV1 {
        coordinate: "continuum.profile.write/v1",
        semantic_domain: "echo.edict-provider/operation-profile/v1",
    },
    operation_profiles: OPERATION_PROFILES,
    footprint: ProviderFootprintIdentityV1 {
        obligation: "target.replace.footprint",
        algebra_coordinate: "echo.dpo.footprint/v1",
        algebra_digest_domain: "echo.dpo.footprint/v1",
        algebra_digest: DIGEST,
    },
};
const OPERATIONS: [ProviderOperationV1<'static>; 1] = [OPERATION];
const REGISTRY: ProviderRegistryV1<'static> = ProviderRegistryV1 {
    echo_contract_abi_version: ECHO_CONTRACT_ABI_VERSION,
    helper_api_version: CONTRACT_HOST_HELPER_API_VERSION,
    provider_schema: PROVIDER_SCHEMA,
    target_bundle_profile: TARGET_BUNDLE_PROFILE,
    bundle: BUNDLE,
    operations: &OPERATIONS,
};
const IMPLEMENTATION_IDENTITY: ProviderMutationImplementationIdentityV1<'static> =
    ProviderMutationImplementationIdentityV1 {
        echo_contract_abi_version: ECHO_CONTRACT_ABI_VERSION,
        helper_api_version: CONTRACT_HOST_HELPER_API_VERSION,
        provider_schema: PROVIDER_SCHEMA,
        target_bundle_profile: TARGET_BUNDLE_PROFILE,
        bundle: BUNDLE,
        operation: OPERATION,
    };
const OCCURRENCE: ContractPackageIdentity<'static> = ContractPackageIdentity {
    package_name: "a.b",
    package_version: "1.0.0",
    artifact_hash_hex: RAW_SCHEMA_SHA256,
};

static MATCHER_CALLS: AtomicUsize = AtomicUsize::new(0);
static EXECUTOR_CALLS: AtomicUsize = AtomicUsize::new(0);
static FOOTPRINT_CALLS: AtomicUsize = AtomicUsize::new(0);

fn matcher(_view: GraphView<'_>, _scope: &NodeId) -> bool {
    false
}

fn execute(_view: GraphView<'_>, _scope: &NodeId, _delta: &mut TickDelta) {}

fn footprint(_view: GraphView<'_>, _scope: &NodeId) -> Footprint {
    Footprint::default()
}

fn counting_matcher(_view: GraphView<'_>, _scope: &NodeId) -> bool {
    MATCHER_CALLS.fetch_add(1, Ordering::SeqCst);
    false
}

fn counting_execute(_view: GraphView<'_>, _scope: &NodeId, _delta: &mut TickDelta) {
    EXECUTOR_CALLS.fetch_add(1, Ordering::SeqCst);
}

fn counting_footprint(_view: GraphView<'_>, _scope: &NodeId) -> Footprint {
    FOOTPRINT_CALLS.fetch_add(1, Ordering::SeqCst);
    Footprint::default()
}

struct TestHost;

impl ProviderMutationHostV1 for TestHost {
    fn execute(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
        execute(view, scope, delta);
    }

    fn effect_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
        footprint(view, scope)
    }
}

struct CountingHost;

impl ProviderMutationHostV1 for CountingHost {
    fn execute(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
        counting_execute(view, scope, delta);
    }

    fn effect_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
        counting_footprint(view, scope)
    }
}

fn proposal_with<'a>(
    occurrence: ContractPackageIdentity<'a>,
    registry: ProviderRegistryV1<'a>,
    dispatch: GeneratedProviderMutationDispatchV1,
    identity: &ProviderMutationImplementationIdentityV1<'a>,
) -> Result<warp_core::ProviderContractPackageProposalV1<'a>, warp_core::ProviderPackageProposalError>
{
    propose_provider_contract_package_v1(
        occurrence,
        registry,
        dispatch,
        ProviderMutationHooksV1::for_host::<TestHost>(*identity),
    )
}

fn assert_identity_mismatch(
    identity: &ProviderMutationImplementationIdentityV1<'static>,
    expected: ProviderPackageProposalErrorKind,
) {
    let error = proposal_with(
        OCCURRENCE,
        REGISTRY,
        GeneratedProviderMutationDispatchV1::new(OPERATION_ID, RULE_NAME, matcher),
        identity,
    )
    .err()
    .map(|error| error.kind());
    assert_eq!(error, Some(expected));
}

#[test]
fn provider_proposal_binds_exactly_one_generated_mutation(
) -> Result<(), warp_core::ProviderPackageProposalError> {
    let proposal = propose_provider_contract_package_v1(
        OCCURRENCE,
        REGISTRY,
        GeneratedProviderMutationDispatchV1::new(OPERATION_ID, RULE_NAME, matcher),
        ProviderMutationHooksV1::for_host::<TestHost>(IMPLEMENTATION_IDENTITY),
    )?;

    assert_eq!(proposal.occurrence(), &OCCURRENCE);
    assert_eq!(proposal.registry(), &REGISTRY);
    assert_eq!(
        proposal.mutation_operation_ids().collect::<Vec<_>>(),
        vec![OPERATION_ID]
    );
    Ok(())
}

#[test]
fn provider_proposal_refuses_every_mismatched_implementation_claim() {
    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.echo_contract_abi_version += 1;
    assert_identity_mismatch(&identity, ProviderPackageProposalErrorKind::EchoAbiMismatch);

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.helper_api_version += 1;
    assert_identity_mismatch(
        &identity,
        ProviderPackageProposalErrorKind::HelperApiMismatch,
    );

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.provider_schema.raw_sha256_hex = "wrong";
    assert_identity_mismatch(&identity, ProviderPackageProposalErrorKind::SchemaMismatch);

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.target_bundle_profile.digest = "sha256:wrong";
    assert_identity_mismatch(
        &identity,
        ProviderPackageProposalErrorKind::TargetBundleProfileMismatch,
    );

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.bundle.semantic_digest = "sha256:wrong";
    assert_identity_mismatch(&identity, ProviderPackageProposalErrorKind::BundleMismatch);

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.operation.coordinate = "a.b@1.other";
    assert_identity_mismatch(
        &identity,
        ProviderPackageProposalErrorKind::OperationMismatch,
    );

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.operation.input.codec_id = "wrong-codec/v1";
    assert_identity_mismatch(&identity, ProviderPackageProposalErrorKind::CodecMismatch);

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.operation.output.schema_coordinate = "a.b@1.OtherOutput";
    assert_identity_mismatch(&identity, ProviderPackageProposalErrorKind::SchemaMismatch);

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.operation.target_ir.digest = "sha256:wrong";
    assert_identity_mismatch(
        &identity,
        ProviderPackageProposalErrorKind::TargetIrMismatch,
    );

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.operation.target_profile.digest = "sha256:wrong";
    assert_identity_mismatch(
        &identity,
        ProviderPackageProposalErrorKind::TargetProfileMismatch,
    );

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.operation.generated_artifact_profile.digest = "sha256:wrong";
    assert_identity_mismatch(
        &identity,
        ProviderPackageProposalErrorKind::GeneratedArtifactProfileMismatch,
    );

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.operation.obstruction.coordinate = "domain.Other";
    assert_identity_mismatch(
        &identity,
        ProviderPackageProposalErrorKind::ObstructionMismatch,
    );

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.operation.operation_profiles.digest = "sha256:wrong";
    assert_identity_mismatch(
        &identity,
        ProviderPackageProposalErrorKind::OperationProfileMismatch,
    );

    let mut identity = IMPLEMENTATION_IDENTITY;
    identity.operation.footprint.obligation = "target.other.footprint";
    assert_identity_mismatch(
        &identity,
        ProviderPackageProposalErrorKind::FootprintMismatch,
    );
}

#[test]
fn provider_proposal_refuses_invalid_occurrence_registry_and_dispatch() {
    for (occurrence, expected) in [
        (
            ContractPackageIdentity {
                package_name: "",
                ..OCCURRENCE
            },
            ProviderPackageProposalErrorKind::EmptyPackageName,
        ),
        (
            ContractPackageIdentity {
                package_version: "",
                ..OCCURRENCE
            },
            ProviderPackageProposalErrorKind::EmptyPackageVersion,
        ),
        (
            ContractPackageIdentity {
                artifact_hash_hex: "ABC",
                ..OCCURRENCE
            },
            ProviderPackageProposalErrorKind::MalformedPackageArtifactHash,
        ),
    ] {
        let kind = proposal_with(
            occurrence,
            REGISTRY,
            GeneratedProviderMutationDispatchV1::new(OPERATION_ID, RULE_NAME, matcher),
            &IMPLEMENTATION_IDENTITY,
        )
        .err()
        .map(|error| error.kind());
        assert_eq!(kind, Some(expected));
    }

    let empty_registry = ProviderRegistryV1 {
        operations: &[],
        ..REGISTRY
    };
    let kind = proposal_with(
        OCCURRENCE,
        empty_registry,
        GeneratedProviderMutationDispatchV1::new(OPERATION_ID, RULE_NAME, matcher),
        &IMPLEMENTATION_IDENTITY,
    )
    .err()
    .map(|error| error.kind());
    assert_eq!(
        kind,
        Some(ProviderPackageProposalErrorKind::UnsupportedOperationCount)
    );

    let duplicate_registry = ProviderRegistryV1 {
        operations: &[OPERATION, OPERATION],
        ..REGISTRY
    };
    let kind = proposal_with(
        OCCURRENCE,
        duplicate_registry,
        GeneratedProviderMutationDispatchV1::new(OPERATION_ID, RULE_NAME, matcher),
        &IMPLEMENTATION_IDENTITY,
    )
    .err()
    .map(|error| error.kind());
    assert_eq!(
        kind,
        Some(ProviderPackageProposalErrorKind::UnsupportedOperationCount)
    );

    let query = ProviderOperationV1 {
        kind: OpKind::Query,
        ..OPERATION
    };
    let query_registry = ProviderRegistryV1 {
        operations: &[query],
        ..REGISTRY
    };
    let kind = proposal_with(
        OCCURRENCE,
        query_registry,
        GeneratedProviderMutationDispatchV1::new(OPERATION_ID, RULE_NAME, matcher),
        &IMPLEMENTATION_IDENTITY,
    )
    .err()
    .map(|error| error.kind());
    assert_eq!(
        kind,
        Some(ProviderPackageProposalErrorKind::UnsupportedOperationKind)
    );

    let reserved = ProviderOperationV1 {
        operation_id: RESERVED_CONTROL_OPERATION_ID,
        ..OPERATION
    };
    let reserved_registry = ProviderRegistryV1 {
        operations: &[reserved],
        ..REGISTRY
    };
    let kind = proposal_with(
        OCCURRENCE,
        reserved_registry,
        GeneratedProviderMutationDispatchV1::new(RESERVED_CONTROL_OPERATION_ID, RULE_NAME, matcher),
        &ProviderMutationImplementationIdentityV1 {
            operation: reserved,
            ..IMPLEMENTATION_IDENTITY
        },
    )
    .err()
    .map(|error| error.kind());
    assert_eq!(
        kind,
        Some(ProviderPackageProposalErrorKind::ReservedOperationId)
    );

    for (dispatch, expected) in [
        (
            GeneratedProviderMutationDispatchV1::new(OPERATION_ID + 1, RULE_NAME, matcher),
            ProviderPackageProposalErrorKind::OperationMismatch,
        ),
        (
            GeneratedProviderMutationDispatchV1::new(OPERATION_ID, "wrong-rule", matcher),
            ProviderPackageProposalErrorKind::RuleIdentityMismatch,
        ),
    ] {
        let kind = proposal_with(OCCURRENCE, REGISTRY, dispatch, &IMPLEMENTATION_IDENTITY)
            .err()
            .map(|error| error.kind());
        assert_eq!(kind, Some(expected));
    }
}

#[test]
fn provider_proposal_construction_invokes_no_runtime_callback(
) -> Result<(), warp_core::ProviderPackageProposalError> {
    MATCHER_CALLS.store(0, Ordering::SeqCst);
    EXECUTOR_CALLS.store(0, Ordering::SeqCst);
    FOOTPRINT_CALLS.store(0, Ordering::SeqCst);

    let _proposal = propose_provider_contract_package_v1(
        OCCURRENCE,
        REGISTRY,
        GeneratedProviderMutationDispatchV1::new(OPERATION_ID, RULE_NAME, counting_matcher),
        ProviderMutationHooksV1::for_host::<CountingHost>(IMPLEMENTATION_IDENTITY),
    )?;

    assert_eq!(MATCHER_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(EXECUTOR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(FOOTPRINT_CALLS.load(Ordering::SeqCst), 0);
    Ok(())
}
