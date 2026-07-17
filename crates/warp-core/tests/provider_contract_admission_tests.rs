#![allow(clippy::expect_used)]
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Trusted-host admission contract for an exact Edict provider proposal.

use std::sync::atomic::{AtomicUsize, Ordering};

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../echo-edict-provider-lowerer/tests/fixtures/generated_echo_dpo.rs"]
mod checked_generated_helper;

use checked_generated_helper::echo_dpo as generated;
use echo_registry_api::OpKind;
use warp_core::{
    make_node_id, make_type_id, propose_provider_contract_package_v1, ContractPackageIdentity,
    EngineBuilder, Footprint, GeneratedProviderMutationDispatchV1, GraphStore, GraphView, NodeId,
    NodeRecord, ProviderContractAdmissionErrorKind, ProviderContractAdmissionPolicyV1,
    ProviderContractPackageProposalV1, ProviderMutationHooksV1, ProviderMutationHostV1,
    SchedulerKind, TickDelta, TrustedRuntimeHost, WorldlineRuntime,
};

const SEMANTIC_DIGEST: &str =
    "sha256:d3b9170373dc30369b1c7d3435f8c3d2183de063dc9e3b18d4b1f41eeac334c9";
const RELEASE_DIGEST: &str =
    "sha256:c39449495281b51f978468d08c21e93bcfa423176063b41675da61e4674b0066";
const UNAPPROVED_RELEASE_DIGEST: &str =
    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const PACKAGE_ARTIFACT_SHA256: &str =
    "fc00dd73a26bbd6699668ddb3ba1e3db9beb9bd7655e270824019d1e6e33f712";
const UNAPPROVED_PACKAGE_ARTIFACT_SHA256: &str =
    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

static MATCHER_CALLS: AtomicUsize = AtomicUsize::new(0);
static EXECUTOR_CALLS: AtomicUsize = AtomicUsize::new(0);
static FOOTPRINT_CALLS: AtomicUsize = AtomicUsize::new(0);

fn bundle_pin(
    release_digest: &'static str,
) -> generated::ExpectedContractBundleIdentityV1<'static> {
    generated::ExpectedContractBundleIdentityV1 {
        semantic_digest_domain: generated::SEMANTIC_BUNDLE_DIGEST_DOMAIN,
        semantic_digest: SEMANTIC_DIGEST,
        release_digest_domain: generated::RELEASE_BUNDLE_DIGEST_DOMAIN,
        release_digest,
    }
}

fn bundle_identity(release_digest: &'static str) -> generated::ContractBundleIdentityV1<'static> {
    generated::ContractBundleIdentityV1 {
        semantic_digest_domain: generated::SEMANTIC_BUNDLE_DIGEST_DOMAIN,
        semantic_digest: SEMANTIC_DIGEST,
        release_digest_domain: generated::RELEASE_BUNDLE_DIGEST_DOMAIN,
        release_digest,
        operation_coordinate: generated::OPERATION_COORDINATE,
        operation_domain: generated::OPERATION_DOMAIN,
        operation_id_law: generated::OPERATION_ID_LAW,
        operation_id: generated::OPERATION_ID,
        value_codec: generated::VALUE_CODEC_ID,
        target_ir_coordinate: generated::TARGET_IR_COORDINATE,
        target_ir_digest_domain: generated::TARGET_IR_DIGEST_DOMAIN,
        target_ir_digest: generated::TARGET_IR_DIGEST,
        target_profile_coordinate: generated::TARGET_PROFILE_COORDINATE,
        target_profile_digest_domain: generated::TARGET_PROFILE_DIGEST_DOMAIN,
        target_profile_digest: generated::TARGET_PROFILE_DIGEST,
        target_bundle_profile_coordinate: generated::TARGET_BUNDLE_PROFILE_COORDINATE,
        target_bundle_profile_digest_domain: generated::TARGET_BUNDLE_PROFILE_DIGEST_DOMAIN,
        target_bundle_profile_digest: generated::TARGET_BUNDLE_PROFILE_DIGEST,
        echo_contract_abi_version: generated::ECHO_CONTRACT_ABI_VERSION,
        helper_api_version: generated::CONTRACT_HOST_HELPER_API_VERSION,
        provider_schema_coordinate: generated::PROVIDER_SCHEMA_COORDINATE,
        provider_schema_sha256_hex: generated::PROVIDER_SCHEMA_SHA256_HEX,
        input_schema: generated::INPUT_SCHEMA,
        output_schema: generated::OUTPUT_SCHEMA,
        type_schema_domain: generated::TYPE_SCHEMA_DOMAIN,
        obstruction_coordinate: generated::OBSTRUCTION_COORDINATE,
        obstruction_domain: generated::OBSTRUCTION_DOMAIN,
        effect_failure_schema: generated::EFFECT_FAILURE_SCHEMA,
        obstruction_payload_schema: generated::OBSTRUCTION_PAYLOAD_SCHEMA,
        generated_artifact_profile: generated::GENERATED_ARTIFACT_PROFILE,
        generated_artifact_profile_digest_domain:
            generated::GENERATED_ARTIFACT_PROFILE_DIGEST_DOMAIN,
        generated_artifact_profile_digest: generated::GENERATED_ARTIFACT_PROFILE_DIGEST,
        operation_profile: generated::OPERATION_PROFILE,
        operation_profile_domain: generated::OPERATION_PROFILE_DOMAIN,
        operation_profiles_coordinate: generated::OPERATION_PROFILES_COORDINATE,
        operation_profiles_digest_domain: generated::OPERATION_PROFILES_DIGEST_DOMAIN,
        operation_profiles_digest: generated::OPERATION_PROFILES_DIGEST,
        footprint_obligation: generated::FOOTPRINT_OBLIGATION,
        footprint_algebra: generated::FOOTPRINT_ALGEBRA,
        footprint_algebra_digest_domain: generated::FOOTPRINT_ALGEBRA_DIGEST_DOMAIN,
        footprint_algebra_digest: generated::FOOTPRINT_ALGEBRA_DIGEST,
    }
}

fn descriptor(release_digest: &'static str) -> generated::RegistrationDescriptorV1<'static> {
    generated::bind_contract_bundle(bundle_pin(release_digest), &bundle_identity(release_digest))
        .expect("self-consistent generated claims bind before Echo admission")
}

const fn occurrence(artifact_hash_hex: &'static str) -> ContractPackageIdentity<'static> {
    ContractPackageIdentity {
        package_name: "echo.edict-provider",
        package_version: "1.0.0",
        artifact_hash_hex,
    }
}

fn counting_matcher(_view: GraphView<'_>, _scope: &NodeId) -> bool {
    MATCHER_CALLS.fetch_add(1, Ordering::SeqCst);
    true
}

struct CountingHost;

impl ProviderMutationHostV1 for CountingHost {
    fn execute(_view: GraphView<'_>, _scope: &NodeId, _delta: &mut TickDelta) {
        EXECUTOR_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    fn effect_footprint(_view: GraphView<'_>, _scope: &NodeId) -> Footprint {
        FOOTPRINT_CALLS.fetch_add(1, Ordering::SeqCst);
        Footprint::default()
    }
}

fn proposal(
    release_digest: &'static str,
    artifact_hash_hex: &'static str,
) -> ProviderContractPackageProposalV1<'static> {
    let descriptor = descriptor(release_digest);
    let rule_name = Box::leak(
        format!(
            "cmd/contract/{}/{}/{}",
            generated::PROVIDER_SCHEMA_SHA256_HEX,
            generated::OPERATION_ID,
            generated::OPERATION_COORDINATE
        )
        .into_boxed_str(),
    );
    propose_provider_contract_package_v1(
        occurrence(artifact_hash_hex),
        descriptor.provider_registry(),
        GeneratedProviderMutationDispatchV1::new(
            generated::OPERATION_ID,
            rule_name,
            counting_matcher,
        ),
        ProviderMutationHooksV1::for_host::<CountingHost>(
            descriptor.mutation_implementation_identity(),
        ),
    )
    .expect("self-consistent generated and host claims produce a proposal")
}

fn admission_policy() -> ProviderContractAdmissionPolicyV1<'static> {
    ProviderContractAdmissionPolicyV1 {
        expected_occurrence: occurrence(PACKAGE_ARTIFACT_SHA256),
        expected_registry: descriptor(RELEASE_DIGEST).provider_registry(),
    }
}

fn empty_host() -> TrustedRuntimeHost {
    let mut store = GraphStore::default();
    let root = make_node_id("root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );
    let engine = EngineBuilder::new(store, root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();
    TrustedRuntimeHost::new(WorldlineRuntime::new(), engine)
        .expect("trusted host initializes for proposal admission")
}

fn assert_no_install_or_callback(host: &TrustedRuntimeHost) {
    assert!(host
        .engine()
        .installed_contract_mutation_package_id(generated::OPERATION_ID)
        .is_none());
    assert_eq!(MATCHER_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(EXECUTOR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(FOOTPRINT_CALLS.load(Ordering::SeqCst), 0);
}

fn assert_exact_proposal_refused_by_policy(
    policy: ProviderContractAdmissionPolicyV1<'static>,
    expected_kind: ProviderContractAdmissionErrorKind,
    expected_subject: &str,
    expected_reference: Option<&str>,
) {
    MATCHER_CALLS.store(0, Ordering::SeqCst);
    EXECUTOR_CALLS.store(0, Ordering::SeqCst);
    FOOTPRINT_CALLS.store(0, Ordering::SeqCst);

    let host = empty_host();
    let error = host
        .admit_provider_contract_package_v1(
            &policy,
            proposal(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256),
        )
        .expect_err("an exact proposal outside the independently pinned policy must fail closed");
    assert_eq!(error.kind(), expected_kind);
    assert_eq!(error.subject(), expected_subject);
    assert_eq!(error.reference(), expected_reference);
    assert_no_install_or_callback(&host);
}

#[test]
fn exact_policy_equality_retains_stable_catch_all_refusals() {
    assert_ne!(
        ProviderContractAdmissionErrorKind::PackageOccurrenceMismatch,
        ProviderContractAdmissionErrorKind::RegistryMismatch
    );
}

#[test]
fn every_current_policy_field_family_has_a_stable_typed_refusal() {
    let mut cases = Vec::new();

    let mut policy = admission_policy();
    policy.expected_occurrence.package_name = "unapproved.provider";
    cases.push((
        policy,
        ProviderContractAdmissionErrorKind::PackageNameMismatch,
        "provider.package.name",
        Some("echo.edict-provider"),
    ));

    let mut policy = admission_policy();
    policy.expected_occurrence.package_version = "9.9.9";
    cases.push((
        policy,
        ProviderContractAdmissionErrorKind::PackageVersionMismatch,
        "provider.package.version",
        Some("1.0.0"),
    ));

    let mut policy = admission_policy();
    policy.expected_occurrence.artifact_hash_hex = UNAPPROVED_PACKAGE_ARTIFACT_SHA256;
    cases.push((
        policy,
        ProviderContractAdmissionErrorKind::PackageArtifactHashMismatch,
        "provider.package.artifact-hash",
        Some(PACKAGE_ARTIFACT_SHA256),
    ));

    let mut policy = admission_policy();
    policy.expected_registry.echo_contract_abi_version += 1;
    cases.push((
        policy,
        ProviderContractAdmissionErrorKind::EchoAbiMismatch,
        "provider.registry.echo-contract-abi-version",
        Some("1"),
    ));

    let mut policy = admission_policy();
    policy.expected_registry.helper_api_version += 1;
    cases.push((
        policy,
        ProviderContractAdmissionErrorKind::HelperApiMismatch,
        "provider.registry.helper-api-version",
        Some("1"),
    ));

    let mut policy = admission_policy();
    policy.expected_registry.provider_schema.coordinate = "unapproved.provider-schema@1";
    cases.push((
        policy,
        ProviderContractAdmissionErrorKind::ProviderSchemaMismatch,
        "provider.registry.schema.coordinate",
        Some(generated::PROVIDER_SCHEMA_COORDINATE),
    ));

    let mut policy = admission_policy();
    policy.expected_registry.target_bundle_profile.coordinate =
        "unapproved.target-bundle-profile@1";
    cases.push((
        policy,
        ProviderContractAdmissionErrorKind::TargetBundleProfileMismatch,
        "provider.registry.target-bundle-profile.coordinate",
        Some(generated::TARGET_BUNDLE_PROFILE_COORDINATE),
    ));

    let mut policy = admission_policy();
    policy.expected_registry.bundle.semantic_digest =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    cases.push((
        policy,
        ProviderContractAdmissionErrorKind::SemanticBundleMismatch,
        "provider.registry.bundle.semantic",
        Some(SEMANTIC_DIGEST),
    ));

    let mut policy = admission_policy();
    policy.expected_registry.bundle.release_digest = UNAPPROVED_RELEASE_DIGEST;
    cases.push((
        policy,
        ProviderContractAdmissionErrorKind::ReleaseBundleMismatch,
        "provider.registry.bundle.release",
        Some(RELEASE_DIGEST),
    ));

    let mut policy = admission_policy();
    let mut operation = policy.expected_registry.operations[0];
    operation.target_ir.digest =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    policy.expected_registry.operations = Box::leak(Box::new([operation]));
    cases.push((
        policy,
        ProviderContractAdmissionErrorKind::OperationSetMismatch,
        "provider.registry.operations",
        Some("a.b@1.t#3389142194"),
    ));

    for (policy, kind, subject, reference) in cases {
        assert_exact_proposal_refused_by_policy(policy, kind, subject, reference);
    }
}

#[test]
fn trusted_host_admits_only_the_exact_checked_provider_proposal_without_installing_it() {
    MATCHER_CALLS.store(0, Ordering::SeqCst);
    EXECUTOR_CALLS.store(0, Ordering::SeqCst);
    FOOTPRINT_CALLS.store(0, Ordering::SeqCst);

    let host = empty_host();
    let policy = admission_policy();
    let admitted = host
        .admit_provider_contract_package_v1(
            &policy,
            proposal(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256),
        )
        .expect("the independently pinned exact provider proposal is admitted");

    assert_eq!(admitted.occurrence(), &policy.expected_occurrence);
    assert_eq!(admitted.registry(), &policy.expected_registry);
    assert_eq!(
        admitted.registry().provider_schema.coordinate,
        generated::PROVIDER_SCHEMA_COORDINATE
    );
    assert_eq!(
        admitted.registry().provider_schema.raw_sha256_hex,
        generated::PROVIDER_SCHEMA_SHA256_HEX
    );
    assert_eq!(
        admitted.registry().target_bundle_profile,
        policy.expected_registry.target_bundle_profile
    );
    assert_eq!(admitted.registry().bundle.semantic_digest, SEMANTIC_DIGEST);
    assert_eq!(admitted.registry().bundle.release_digest, RELEASE_DIGEST);
    assert_eq!(admitted.registry().operations.len(), 1);
    assert_eq!(admitted.registry().operations[0].coordinate, "a.b@1.t");
    assert_eq!(admitted.registry().operations[0].kind, OpKind::Mutation);
    assert_eq!(
        admitted.registry().operations[0].operation_id,
        3_389_142_194
    );
    assert_eq!(
        admitted.mutation_operation_ids().collect::<Vec<_>>(),
        vec![generated::OPERATION_ID]
    );
    assert_no_install_or_callback(&host);

    let release_error = host
        .admit_provider_contract_package_v1(
            &policy,
            proposal(UNAPPROVED_RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256),
        )
        .expect_err("an independently self-consistent but unapproved release must fail closed");
    assert_eq!(
        release_error.kind(),
        ProviderContractAdmissionErrorKind::ReleaseBundleMismatch
    );
    assert_eq!(release_error.subject(), "provider.registry.bundle.release");
    assert_eq!(release_error.reference(), Some(UNAPPROVED_RELEASE_DIGEST));
    assert_no_install_or_callback(&host);

    let artifact_error = host
        .admit_provider_contract_package_v1(
            &policy,
            proposal(RELEASE_DIGEST, UNAPPROVED_PACKAGE_ARTIFACT_SHA256),
        )
        .expect_err("an unapproved package artifact occurrence must fail closed");
    assert_eq!(
        artifact_error.kind(),
        ProviderContractAdmissionErrorKind::PackageArtifactHashMismatch
    );
    assert_eq!(artifact_error.subject(), "provider.package.artifact-hash");
    assert_eq!(
        artifact_error.reference(),
        Some(UNAPPROVED_PACKAGE_ARTIFACT_SHA256)
    );
    assert_no_install_or_callback(&host);
}
