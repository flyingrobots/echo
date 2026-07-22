#![allow(clippy::expect_used, clippy::panic)]
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Trusted-host admission contract for an exact Edict provider proposal.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, MutexGuard,
    },
};

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../echo-edict-provider-lowerer/tests/fixtures/generated_echo_dpo.rs"]
mod checked_generated_helper;

use checked_generated_helper::echo_dpo as generated;
use echo_registry_api::{
    ContractArtifactVerificationPolicy, ObjectDef, OpDef, OpKind, ProviderOperationV1,
    RegistryInfo, RegistryProvider,
};
use warp_core::{
    causal_wal::{WalRecordKind, WalRuntimeStateDeltaRecord},
    make_head_id, make_intent_kind, make_node_id, make_type_id,
    propose_provider_contract_package_v1, ConflictPolicy, ContractInverseAdmissionRequest,
    ContractMutationHandler, ContractPackageIdentity, EngineBuilder, Footprint,
    GeneratedProviderMutationDispatchV1, GraphStore, GraphView, InboxPolicy, IngressEnvelope,
    IngressTarget, InstalledContractPackage, InstalledContractPackageError,
    InstalledInvocationEvidence, IntentOutcome, NodeId, NodeRecord, PatternGraph, PlaybackMode,
    ProviderContractAdmissionErrorKind, ProviderContractAdmissionPolicyV1,
    ProviderContractInstallationErrorKind, ProviderContractPackageProposalV1,
    ProviderMutationHooksV1, ProviderMutationHostV1, ProviderMutationMatchFnV1,
    ProviderPackageReferenceV1, RewriteRule, RuntimeError, SchedulerKind, TickDelta,
    TrustedRuntimeHost, TrustedRuntimeWalConfig, WorldlineId, WorldlineRuntime, WorldlineState,
    WriterHead, WriterHeadKey,
};

const SEMANTIC_DIGEST: &str =
    "sha256:d3b9170373dc30369b1c7d3435f8c3d2183de063dc9e3b18d4b1f41eeac334c9";
const RELEASE_DIGEST: &str =
    "sha256:c39449495281b51f978468d08c21e93bcfa423176063b41675da61e4674b0066";
const UNAPPROVED_RELEASE_DIGEST: &str =
    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const PACKAGE_ARTIFACT_SHA256: &str =
    "ee870c75ec08c8818b3f80ab6562ae62a5cf741cd709edcee0085d951c5d5a7b";
const UNAPPROVED_PACKAGE_ARTIFACT_SHA256: &str =
    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const PROVIDER_PACKAGE_COORDINATE: &str = "echo.edict-provider@1";
const LEGACY_SCHEMA_SHA256_HEX: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const LEGACY_RULE_NAME: &str = "cmd/contract/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/3389142194/legacyCollision";

static MATCHER_CALLS: AtomicUsize = AtomicUsize::new(0);
static EXECUTOR_CALLS: AtomicUsize = AtomicUsize::new(0);
static FOOTPRINT_CALLS: AtomicUsize = AtomicUsize::new(0);
static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);
static CALLBACK_TEST_LOCK: Mutex<()> = Mutex::new(());

static LEGACY_OPS: &[OpDef] = &[OpDef {
    kind: OpKind::Mutation,
    name: "legacyCollision",
    op_id: generated::OPERATION_ID,
    args: &[],
    result_ty: "LegacyResult",
    directives_json: "{}",
    footprint_certificate: None,
}];

struct LegacyCollisionRegistry;

impl RegistryProvider for LegacyCollisionRegistry {
    fn info(&self) -> RegistryInfo {
        RegistryInfo {
            echo_abi_version: 1,
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: LEGACY_SCHEMA_SHA256_HEX,
            wesley_generator_version: "echo-wesley-gen/0.1.0",
            helper_api_version: 1,
        }
    }

    fn op_by_id(&self, op_id: u32) -> Option<&'static OpDef> {
        LEGACY_OPS.iter().find(|operation| operation.op_id == op_id)
    }

    fn all_ops(&self) -> &'static [OpDef] {
        LEGACY_OPS
    }

    fn all_enums(&self) -> &'static [echo_registry_api::EnumDef] {
        &[]
    }

    fn all_objects(&self) -> &'static [ObjectDef] {
        &[]
    }
}

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

fn refusing_matcher(_view: GraphView<'_>, _scope: &NodeId) -> bool {
    MATCHER_CALLS.fetch_add(1, Ordering::SeqCst);
    false
}

fn counting_executor(_view: GraphView<'_>, _scope: &NodeId, _delta: &mut TickDelta) {
    EXECUTOR_CALLS.fetch_add(1, Ordering::SeqCst);
}

fn counting_footprint(_view: GraphView<'_>, _scope: &NodeId) -> Footprint {
    FOOTPRINT_CALLS.fetch_add(1, Ordering::SeqCst);
    Footprint::default()
}

struct CountingHost;

impl ProviderMutationHostV1 for CountingHost {
    fn execute(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
        counting_executor(view, scope, delta);
    }

    fn effect_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
        counting_footprint(view, scope)
    }
}

fn legacy_collision_package() -> InstalledContractPackage<'static> {
    static REGISTRY: LegacyCollisionRegistry = LegacyCollisionRegistry;
    InstalledContractPackage {
        identity: ContractPackageIdentity {
            package_name: "legacy-provider-collision",
            package_version: "1.0.0",
            artifact_hash_hex: LEGACY_SCHEMA_SHA256_HEX,
        },
        registry: &REGISTRY,
        verification_policy: ContractArtifactVerificationPolicy {
            echo_abi_version: 1,
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: LEGACY_SCHEMA_SHA256_HEX,
            wesley_generator_version: "echo-wesley-gen/0.1.0",
            helper_api_version: 1,
            footprint_certificates: &[],
            require_mutation_footprint_certificates: false,
        },
        mutation_handlers: vec![ContractMutationHandler {
            op_id: generated::OPERATION_ID,
            rule: RewriteRule {
                id: make_type_id(LEGACY_RULE_NAME).0,
                name: LEGACY_RULE_NAME,
                left: PatternGraph { nodes: vec![] },
                matcher: counting_matcher,
                executor: counting_executor,
                compute_footprint: counting_footprint,
                factor_mask: 0,
                conflict_policy: ConflictPolicy::Abort,
                join_fn: None,
            },
        }],
        inverse_handlers: vec![],
        query_observers: vec![],
    }
}

fn package_reference() -> ProviderPackageReferenceV1 {
    ProviderPackageReferenceV1::new(
        PROVIDER_PACKAGE_COORDINATE,
        format!("sha256:{PACKAGE_ARTIFACT_SHA256}"),
    )
}

fn reset_callback_counts() {
    MATCHER_CALLS.store(0, Ordering::SeqCst);
    EXECUTOR_CALLS.store(0, Ordering::SeqCst);
    FOOTPRINT_CALLS.store(0, Ordering::SeqCst);
}

fn callback_test_guard() -> MutexGuard<'static, ()> {
    CALLBACK_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn assert_no_callback() {
    assert_eq!(MATCHER_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(EXECUTOR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(FOOTPRINT_CALLS.load(Ordering::SeqCst), 0);
}

fn temp_provider_wal_dir(label: &str) -> PathBuf {
    let root = PathBuf::from("target").join("warp-core-test-tmp");
    std::fs::create_dir_all(&root).expect("provider WAL fixture root is created");
    for _ in 0..1024 {
        let unique = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = root.join(format!("echo-provider-native-{label}-{unique}"));
        match std::fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!(
                "failed to create provider WAL fixture {}: {error}",
                path.display()
            ),
        }
    }
    panic!("exhausted deterministic provider WAL fixture attempts for {label}");
}

fn encoded_field_offset(bytes: &[u8], field: &[u8]) -> usize {
    let mut matches = bytes
        .windows(field.len())
        .enumerate()
        .filter_map(|(offset, candidate)| (candidate == field).then_some(offset));
    let offset = matches.next().expect("encoded field should be present");
    assert!(
        matches.next().is_none(),
        "encoded field should occur exactly once"
    );
    offset
}

fn remove_encoded_string(bytes: &mut Vec<u8>, value: &str) {
    let offset = encoded_field_offset(bytes, value.as_bytes());
    let length_offset = offset
        .checked_sub(8)
        .expect("retained strings have an eight-byte length prefix");
    bytes[length_offset..offset].copy_from_slice(&0_u64.to_le_bytes());
    bytes.drain(offset..offset + value.len());
}

fn proposal(
    release_digest: &'static str,
    artifact_hash_hex: &'static str,
) -> ProviderContractPackageProposalV1<'static> {
    proposal_with_matcher(release_digest, artifact_hash_hex, counting_matcher)
}

fn proposal_with_matcher(
    release_digest: &'static str,
    artifact_hash_hex: &'static str,
    matcher: ProviderMutationMatchFnV1,
) -> ProviderContractPackageProposalV1<'static> {
    let descriptor = descriptor(release_digest);
    let rule_name = provider_rule_name();
    propose_provider_contract_package_v1(
        occurrence(artifact_hash_hex),
        descriptor.provider_registry(),
        GeneratedProviderMutationDispatchV1::new(generated::OPERATION_ID, rule_name, matcher),
        ProviderMutationHooksV1::for_host::<CountingHost>(
            descriptor.mutation_implementation_identity(),
        ),
    )
    .expect("self-consistent generated and host claims produce a proposal")
}

fn provider_rule_name() -> &'static str {
    Box::leak(
        format!(
            "cmd/contract/{}/{}/{}",
            generated::PROVIDER_SCHEMA_SHA256_HEX,
            generated::OPERATION_ID,
            generated::OPERATION_COORDINATE
        )
        .into_boxed_str(),
    )
}

fn admission_policy() -> ProviderContractAdmissionPolicyV1<'static> {
    ProviderContractAdmissionPolicyV1 {
        expected_occurrence: occurrence(PACKAGE_ARTIFACT_SHA256),
        expected_registry: descriptor(RELEASE_DIGEST).provider_registry(),
    }
}

fn proposal_and_policy_for_operation(
    operation: &ProviderOperationV1<'static>,
) -> (
    ProviderContractAdmissionPolicyV1<'static>,
    ProviderContractPackageProposalV1<'static>,
) {
    let operation = *operation;
    let operations: &'static [ProviderOperationV1<'static>] = Box::leak(Box::new([operation]));
    let mut registry = descriptor(RELEASE_DIGEST).provider_registry();
    registry.operations = operations;

    let mut implementation_identity = descriptor(RELEASE_DIGEST).mutation_implementation_identity();
    implementation_identity.operation = operation;
    let rule_name = Box::leak(
        format!(
            "cmd/contract/{}/{}/{}",
            registry.provider_schema.raw_sha256_hex, operation.operation_id, operation.coordinate
        )
        .into_boxed_str(),
    );
    let proposal = propose_provider_contract_package_v1(
        occurrence(PACKAGE_ARTIFACT_SHA256),
        registry,
        GeneratedProviderMutationDispatchV1::new(
            operation.operation_id,
            rule_name,
            counting_matcher,
        ),
        ProviderMutationHooksV1::for_host::<CountingHost>(implementation_identity),
    )
    .expect("a self-consistent malformed evidence fixture reaches Echo admission");
    let policy = ProviderContractAdmissionPolicyV1 {
        expected_occurrence: occurrence(PACKAGE_ARTIFACT_SHA256),
        expected_registry: registry,
    };
    (policy, proposal)
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

fn provider_runtime_host() -> (TrustedRuntimeHost, WorldlineId) {
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

    let mut runtime = WorldlineRuntime::new();
    let worldline_id = WorldlineId::from_bytes([1; 32]);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .expect("provider runtime worldline registers");
    runtime
        .register_writer_head(WriterHead::with_routing(
            WriterHeadKey {
                worldline_id,
                head_id: make_head_id("default"),
            },
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("provider runtime writer head registers");

    (
        TrustedRuntimeHost::new(runtime, engine).expect("provider runtime host initializes"),
        worldline_id,
    )
}

fn host_with_provider_rule_name_reserved() -> TrustedRuntimeHost {
    let mut store = GraphStore::default();
    let root = make_node_id("root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );
    let mut engine = EngineBuilder::new(store, root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();
    let rule_name = provider_rule_name();
    engine
        .register_rule(RewriteRule {
            id: make_type_id(rule_name).0,
            name: rule_name,
            left: PatternGraph { nodes: vec![] },
            matcher: counting_matcher,
            executor: counting_executor,
            compute_footprint: counting_footprint,
            factor_mask: 0,
            conflict_policy: ConflictPolicy::Abort,
            join_fn: None,
        })
        .expect("the provider rule identity is reserved before package installation");
    TrustedRuntimeHost::new(WorldlineRuntime::new(), engine)
        .expect("trusted host initializes with the reserved provider rule")
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
    let _callback_guard = callback_test_guard();
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
    let _callback_guard = callback_test_guard();
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

#[test]
fn legacy_first_provider_second_refuses_without_partial_provider_installation() {
    let _callback_guard = callback_test_guard();
    reset_callback_counts();
    let mut host = empty_host();
    host.register_contract_package(legacy_collision_package())
        .expect("the legacy collision fixture installs before the provider package");
    assert!(host
        .engine()
        .installed_contract_mutation_evidence(generated::OPERATION_ID)
        .is_some());

    let admitted = host
        .admit_provider_contract_package_v1(
            &admission_policy(),
            proposal(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256),
        )
        .expect("the exact provider proposal remains admissible independently of installation");
    let package_reference = package_reference();
    let error = host
        .install_admitted_provider_contract_package_v1_trusted(package_reference.clone(), admitted)
        .expect_err("a provider operation cannot capture a legacy-owned operation id");

    assert_eq!(
        error.kind(),
        ProviderContractInstallationErrorKind::LegacyOperationConflict
    );
    assert_eq!(error.subject(), "legacy.contract.operation.id");
    assert_eq!(
        error.reference(),
        Some(generated::OPERATION_ID.to_string().as_str())
    );
    assert!(host
        .engine()
        .installed_provider_contract_mutation_package_id(generated::OPERATION_ID)
        .is_none());
    assert!(host
        .engine()
        .installed_provider_contract_package_by_reference(&package_reference)
        .is_none());
    assert!(host
        .engine()
        .installed_contract_mutation_evidence(generated::OPERATION_ID)
        .is_some());
    assert_no_callback();
}

#[test]
fn provider_first_legacy_second_refuses_without_partial_legacy_installation() {
    let _callback_guard = callback_test_guard();
    reset_callback_counts();
    let mut host = empty_host();
    let admitted = host
        .admit_provider_contract_package_v1(
            &admission_policy(),
            proposal(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256),
        )
        .expect("the exact provider proposal is independently admitted");
    let package_reference = package_reference();
    let installed = host
        .install_admitted_provider_contract_package_v1_trusted(package_reference.clone(), admitted)
        .expect("the provider package installs before the legacy collision fixture");

    let error = host
        .register_contract_package(legacy_collision_package())
        .expect_err("a legacy package cannot capture a provider-owned operation id");
    assert!(matches!(
        error,
        InstalledContractPackageError::ProviderOperationConflict { op_id }
            if op_id == generated::OPERATION_ID
    ));
    assert!(host
        .engine()
        .installed_contract_mutation_package_id(generated::OPERATION_ID)
        .is_none());
    assert!(host
        .engine()
        .installed_contract_mutation_evidence(generated::OPERATION_ID)
        .is_none());
    assert_eq!(
        host.engine()
            .installed_provider_contract_mutation_package_id(generated::OPERATION_ID),
        Some(installed.package_id())
    );
    assert_eq!(
        host.engine()
            .installed_provider_contract_package_by_reference(&package_reference),
        Some(&installed)
    );
    assert_no_callback();
}

#[test]
fn trusted_installation_lower_boundary_rejects_unproven_package_root_disagreement() {
    let _callback_guard = callback_test_guard();
    let cases = [
        (
            "sha256:ABC",
            ProviderContractInstallationErrorKind::MalformedPackageReferenceDigest,
            "provider.package.reference.digest",
            "sha256:ABC",
        ),
        (
            "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            ProviderContractInstallationErrorKind::PackageArtifactDigestMismatch,
            "provider.package.occurrence.artifact-hash",
            PACKAGE_ARTIFACT_SHA256,
        ),
    ];

    for (digest, expected_kind, expected_subject, expected_reference) in cases {
        reset_callback_counts();
        let mut host = empty_host();
        let admitted = host
            .admit_provider_contract_package_v1(
                &admission_policy(),
                proposal(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256),
            )
            .expect("the exact proposal is admitted independently of the root claim");
        let reference = ProviderPackageReferenceV1::new("echo.edict-provider@1", digest);
        let error = host
            .install_admitted_provider_contract_package_v1_trusted(reference.clone(), admitted)
            .expect_err("the trusted lower boundary must reject package-root disagreement");

        assert_eq!(error.kind(), expected_kind);
        assert_eq!(error.subject(), expected_subject);
        assert_eq!(error.reference(), Some(expected_reference));
        assert!(host
            .engine()
            .installed_provider_contract_package_by_reference(&reference)
            .is_none());
        assert!(host
            .engine()
            .installed_provider_contract_mutation_package_id(generated::OPERATION_ID)
            .is_none());
        assert_no_callback();
    }
}

#[test]
fn installation_rejects_provider_evidence_that_recovery_would_reject() {
    let _callback_guard = callback_test_guard();
    let valid_operation = descriptor(RELEASE_DIGEST).provider_registry().operations[0];
    let cases = [
        (
            "empty operation coordinate",
            {
                let mut operation = valid_operation;
                operation.coordinate = "";
                operation
            },
            ProviderContractInstallationErrorKind::EmptyOperationCoordinate,
            "provider.operation.coordinate",
        ),
        (
            "empty Target IR coordinate",
            {
                let mut operation = valid_operation;
                operation.target_ir.coordinate = "";
                operation
            },
            ProviderContractInstallationErrorKind::EmptyTargetIrCoordinate,
            "provider.operation.target-ir.coordinate",
        ),
        (
            "empty Target IR digest domain",
            {
                let mut operation = valid_operation;
                operation.target_ir.digest_domain = "";
                operation
            },
            ProviderContractInstallationErrorKind::EmptyTargetIrDigestDomain,
            "provider.operation.target-ir.digest-domain",
        ),
        (
            "empty Target IR digest",
            {
                let mut operation = valid_operation;
                operation.target_ir.digest = "";
                operation
            },
            ProviderContractInstallationErrorKind::MalformedTargetIrDigest,
            "provider.operation.target-ir.digest",
        ),
        (
            "Target IR digest without an algorithm prefix",
            {
                let mut operation = valid_operation;
                operation.target_ir.digest =
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
                operation
            },
            ProviderContractInstallationErrorKind::MalformedTargetIrDigest,
            "provider.operation.target-ir.digest",
        ),
        (
            "uppercase Target IR digest hexadecimal",
            {
                let mut operation = valid_operation;
                operation.target_ir.digest =
                    "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
                operation
            },
            ProviderContractInstallationErrorKind::MalformedTargetIrDigest,
            "provider.operation.target-ir.digest",
        ),
        (
            "63-character Target IR SHA-256 payload",
            {
                let mut operation = valid_operation;
                operation.target_ir.digest =
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
                operation
            },
            ProviderContractInstallationErrorKind::MalformedTargetIrDigest,
            "provider.operation.target-ir.digest",
        ),
        (
            "65-character Target IR SHA-256 payload",
            {
                let mut operation = valid_operation;
                operation.target_ir.digest =
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
                operation
            },
            ProviderContractInstallationErrorKind::MalformedTargetIrDigest,
            "provider.operation.target-ir.digest",
        ),
        (
            "non-hexadecimal Target IR SHA-256 payload",
            {
                let mut operation = valid_operation;
                operation.target_ir.digest =
                    "sha256:gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg";
                operation
            },
            ProviderContractInstallationErrorKind::MalformedTargetIrDigest,
            "provider.operation.target-ir.digest",
        ),
    ];

    for (case, operation, expected_kind, expected_subject) in cases {
        reset_callback_counts();
        let (policy, proposal) = proposal_and_policy_for_operation(&operation);
        let mut host = empty_host();
        let admitted = host
            .admit_provider_contract_package_v1(&policy, proposal)
            .expect("exact policy equality admits the malformed structural fixture");
        let reference = package_reference();
        let error = host
            .install_admitted_provider_contract_package_v1_trusted(reference.clone(), admitted)
            .expect_err(case);

        assert_eq!(error.kind(), expected_kind, "{case}");
        assert_eq!(error.subject(), expected_subject, "{case}");
        let expected_reference = (expected_kind
            == ProviderContractInstallationErrorKind::MalformedTargetIrDigest)
            .then_some(operation.target_ir.digest);
        assert_eq!(error.reference(), expected_reference, "{case}");

        assert!(host
            .engine()
            .installed_provider_contract_package_by_reference(&reference)
            .is_none());
        assert!(host
            .engine()
            .installed_provider_contract_mutation_package_id(operation.operation_id)
            .is_none());
        assert!(host
            .engine()
            .installed_provider_contract_mutation_evidence_v1(operation.operation_id)
            .is_none());
        assert_no_callback();
    }
}

#[test]
fn provider_rule_conflict_refuses_before_any_provider_index_changes() {
    let _callback_guard = callback_test_guard();
    reset_callback_counts();
    let mut host = host_with_provider_rule_name_reserved();
    let admitted = host
        .admit_provider_contract_package_v1(
            &admission_policy(),
            proposal(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256),
        )
        .expect("the exact proposal remains independently admissible");
    let reference = package_reference();
    let error = host
        .install_admitted_provider_contract_package_v1_trusted(reference.clone(), admitted)
        .expect_err("a provider package cannot capture an existing scheduler rule name");

    assert_eq!(
        error.kind(),
        ProviderContractInstallationErrorKind::DuplicateRuleName
    );
    assert_eq!(error.subject(), "provider.mutation.rule.name");
    assert_eq!(error.reference(), Some(provider_rule_name()));
    assert!(host
        .engine()
        .installed_provider_contract_package_by_reference(&reference)
        .is_none());
    assert!(host
        .engine()
        .installed_provider_contract_mutation_package_id(generated::OPERATION_ID)
        .is_none());
    assert!(host
        .engine()
        .installed_contract_mutation_evidence(generated::OPERATION_ID)
        .is_none());
    assert_no_callback();
}

#[test]
fn provider_matcher_refusal_cannot_be_reported_as_provider_execution() {
    let _callback_guard = callback_test_guard();
    reset_callback_counts();
    let (mut host, worldline_id) = provider_runtime_host();
    let admitted = host
        .admit_provider_contract_package_v1(
            &admission_policy(),
            proposal_with_matcher(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256, refusing_matcher),
        )
        .expect("the exact provider proposal is admitted");
    host.install_admitted_provider_contract_package_v1_trusted(package_reference(), admitted)
        .expect("the exact provider package installs");

    let submission = host
        .app()
        .submit_intent(IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("echo.intent/eint-v1"),
            echo_wasm_abi::pack_intent_v1(generated::OPERATION_ID, &[])
                .expect("provider EINT intent packs"),
        ))
        .expect("provider EINT intent is witnessed");
    host.admit_provider_contract_submission_v1(submission.submission_id)
        .expect("installed provider submission stages");
    host.run_until_idle(4)
        .expect("shared scheduler records the refusing matcher posture");

    assert!(MATCHER_CALLS.load(Ordering::SeqCst) > 0);
    assert_eq!(EXECUTOR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(FOOTPRINT_CALLS.load(Ordering::SeqCst), 0);
    assert!(matches!(
        host.app().observe_intent_outcome(&submission.submission_id),
        IntentOutcome::Obstructed {
            obstruction,
            ..
        } if obstruction.kind == warp_core::ContractObstructionKind::RuntimeFault
            && obstruction.subject
                == warp_core::ContractObstructionSubject::Operation {
                    op_id: generated::OPERATION_ID,
                }
    ));
}

#[test]
fn provider_native_invocation_retains_provider_evidence_through_receipt() {
    let _callback_guard = callback_test_guard();
    reset_callback_counts();
    let wal_root = temp_provider_wal_dir("invocation-recovery");
    let (mut host, worldline_id) = provider_runtime_host();
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("provider invocation WAL initializes");
    let policy = admission_policy();
    let admitted = host
        .admit_provider_contract_package_v1(
            &policy,
            proposal(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256),
        )
        .expect("the exact provider proposal is admitted");
    let installed = host
        .install_admitted_provider_contract_package_v1_trusted(package_reference(), admitted)
        .expect("the admitted provider package installs");

    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("echo.intent/eint-v1"),
            echo_wasm_abi::pack_intent_v1(generated::OPERATION_ID, &[])
                .expect("provider EINT intent packs"),
        ))
        .expect("provider EINT intent is witnessed")
    };

    assert!(matches!(
        host.admit_installed_contract_submission(submission.submission_id),
        Err(RuntimeError::UnsupportedInstalledContractMutation { op_id })
            if op_id == generated::OPERATION_ID
    ));
    assert_eq!(host.runtime().ticketed_runtime_ingress_count(), 0);
    assert_no_callback();

    let staged = host
        .admit_provider_contract_submission_v1(submission.submission_id)
        .expect("Echo admits installed provider work without caller ticket authority");
    let staged_record = match staged {
        warp_core::TicketedRuntimeIngressDisposition::Staged { record, .. }
        | warp_core::TicketedRuntimeIngressDisposition::Duplicate { record } => record,
    };
    let evidence = staged_record
        .contract
        .expect("provider ingress retains installed-operation evidence");
    let InstalledInvocationEvidence::ProviderV1(provider) = &evidence else {
        panic!("provider ingress must not fabricate legacy contract evidence");
    };
    assert_eq!(provider.package_id(), installed.package_id());
    assert_eq!(provider.package_reference(), installed.package_reference());
    assert_eq!(provider.operation_id(), generated::OPERATION_ID);
    assert_eq!(provider.operation_coordinate(), "a.b@1.t");
    assert_eq!(
        provider.target_ir(),
        installed.registry().operations()[0].target_ir()
    );
    assert_eq!(provider.rule_id(), installed.mutation_rule().rule_id());
    assert!(host
        .engine()
        .installed_contract_mutation_evidence(generated::OPERATION_ID)
        .is_none());
    assert_no_callback();

    host.run_until_idle(4)
        .expect("the provider mutation executes only through the scheduler");
    assert!(MATCHER_CALLS.load(Ordering::SeqCst) > 0);
    assert!(EXECUTOR_CALLS.load(Ordering::SeqCst) > 0);
    assert!(FOOTPRINT_CALLS.load(Ordering::SeqCst) > 0);

    let outcome = {
        let app = host.app();
        app.observe_intent_outcome(&submission.submission_id)
    };
    let IntentOutcome::Applied { receipt, .. } = outcome else {
        panic!("provider mutation should produce one applied Echo receipt");
    };
    assert_eq!(receipt.contract.as_ref(), Some(&evidence));
    assert_eq!(receipt.rule_id, *provider.rule_id());
    assert!(
        receipt.retained_evidence.is_empty(),
        "provider evidence must not fabricate a legacy retained coordinate"
    );

    let runtime_wal = host
        .runtime_wal()
        .expect("provider invocation WAL remains configured");
    let recovery = runtime_wal
        .recover_read_only()
        .expect("provider invocation evidence recovers from retained WAL bytes");
    let recovered_correlation = recovery
        .receipt_correlations
        .iter()
        .find(|correlation| correlation.submission_id == submission.submission_id)
        .expect("provider receipt correlation recovers by submission identity");
    assert_eq!(recovered_correlation.contract.as_ref(), Some(&evidence));
    assert_eq!(recovery.provenance_entries.len(), 1);

    let state_delta_bytes = runtime_wal
        .frames()
        .into_iter()
        .find(|frame| frame.header.record_kind == WalRecordKind::RuntimeStateDeltaRecorded)
        .expect("provider scheduler tick retains one state-delta frame")
        .payload
        .canonical_bytes;
    let contract_tag_offset = b"ERSD0001".len() + 32;
    assert_eq!(state_delta_bytes[contract_tag_offset], 2);
    let decoded = WalRuntimeStateDeltaRecord::from_payload_bytes(&state_delta_bytes)
        .expect("provider state-delta bytes decode");
    assert_eq!(decoded.contract(), Some(&evidence));

    let mut empty_package_coordinate = state_delta_bytes.clone();
    remove_encoded_string(
        &mut empty_package_coordinate,
        provider.package_reference().coordinate(),
    );
    assert_eq!(
        WalRuntimeStateDeltaRecord::from_payload_bytes(&empty_package_coordinate),
        Err(warp_core::RetainedProvenanceError::Inconsistent(
            "provider package reference coordinate"
        ))
    );

    let mut malformed_package_digest = state_delta_bytes.clone();
    let package_digest_offset = encoded_field_offset(
        &malformed_package_digest,
        provider.package_reference().digest().as_bytes(),
    );
    malformed_package_digest[package_digest_offset + "sha256:".len()] = b'G';
    assert_eq!(
        WalRuntimeStateDeltaRecord::from_payload_bytes(&malformed_package_digest),
        Err(warp_core::RetainedProvenanceError::Inconsistent(
            "provider package reference digest"
        ))
    );

    let operation_id_bytes = provider.operation_id().to_le_bytes();
    let operation_coordinate_offset = encoded_field_offset(
        &state_delta_bytes,
        provider.operation_coordinate().as_bytes(),
    );
    let operation_id_offset = operation_coordinate_offset
        .checked_sub(8 + operation_id_bytes.len())
        .expect("provider operation id precedes its length-framed coordinate");
    assert_eq!(
        &state_delta_bytes[operation_id_offset..operation_id_offset + operation_id_bytes.len()],
        operation_id_bytes.as_slice()
    );
    let mut reserved_operation_id = state_delta_bytes.clone();
    reserved_operation_id[operation_id_offset..operation_id_offset + operation_id_bytes.len()]
        .copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        WalRuntimeStateDeltaRecord::from_payload_bytes(&reserved_operation_id),
        Err(warp_core::RetainedProvenanceError::Inconsistent(
            "provider operation id"
        ))
    );

    for (value, inconsistent_field) in [
        (
            provider.operation_coordinate(),
            "provider operation coordinate",
        ),
        (
            provider.target_ir().coordinate(),
            "provider Target IR coordinate",
        ),
        (
            provider.target_ir().digest_domain(),
            "provider Target IR digest domain",
        ),
    ] {
        let mut empty_field = state_delta_bytes.clone();
        remove_encoded_string(&mut empty_field, value);
        assert_eq!(
            WalRuntimeStateDeltaRecord::from_payload_bytes(&empty_field),
            Err(warp_core::RetainedProvenanceError::Inconsistent(
                inconsistent_field
            ))
        );
    }

    let mut malformed_target_ir_digest = state_delta_bytes.clone();
    let target_ir_digest_offset = encoded_field_offset(
        &malformed_target_ir_digest,
        provider.target_ir().digest().as_bytes(),
    );
    malformed_target_ir_digest[target_ir_digest_offset + "sha256:".len()] = b'G';
    assert_eq!(
        WalRuntimeStateDeltaRecord::from_payload_bytes(&malformed_target_ir_digest),
        Err(warp_core::RetainedProvenanceError::Inconsistent(
            "provider Target IR digest"
        ))
    );

    let mut invalid_utf8 = state_delta_bytes.clone();
    invalid_utf8[operation_coordinate_offset] = u8::MAX;
    assert_eq!(
        WalRuntimeStateDeltaRecord::from_payload_bytes(&invalid_utf8),
        Err(warp_core::RetainedProvenanceError::InvalidUtf8)
    );

    for end in 0..state_delta_bytes.len() {
        assert!(
            WalRuntimeStateDeltaRecord::from_payload_bytes(&state_delta_bytes[..end]).is_err(),
            "provider state-delta truncation at byte {end} must fail closed"
        );
    }

    let mut unknown_tag = state_delta_bytes.clone();
    unknown_tag[contract_tag_offset] = u8::MAX;
    assert_eq!(
        WalRuntimeStateDeltaRecord::from_payload_bytes(&unknown_tag),
        Err(warp_core::RetainedProvenanceError::UnknownTag {
            family: "optional contract evidence",
            tag: u8::MAX,
        })
    );

    let mut changed_package_id = state_delta_bytes;
    changed_package_id[contract_tag_offset + 1] ^= 0x80;
    let changed = WalRuntimeStateDeltaRecord::from_payload_bytes(&changed_package_id)
        .expect("well-formed changed provider evidence still decodes as distinct evidence");
    assert_ne!(changed.contract(), decoded.contract());
    assert_ne!(
        changed.digest().expect("changed state delta hashes"),
        decoded.digest().expect("original state delta hashes")
    );

    let target_receipt_ref = receipt.causal_receipt_ref;
    let current_frontier_tick = host
        .runtime()
        .worldlines()
        .get(&worldline_id)
        .expect("provider worldline remains registered")
        .frontier_tick();
    let inverse_error = host
        .app()
        .submit_contract_inverse_with_runtime_wal_ack(ContractInverseAdmissionRequest {
            target_receipt_ref,
            current_target: IngressTarget::DefaultWriter { worldline_id },
            expected_current_frontier_tick: current_frontier_tick,
            policy_bytes: b"provider-inverse-not-admitted".to_vec(),
        })
        .expect_err("provider mutation has no admitted inverse law in this slice");
    assert!(matches!(
        inverse_error,
        warp_core::TrustedRuntimeHostError::ContractInverse(
            warp_core::ContractInverseObstruction::ProviderTargetUnsupported {
                target_receipt_ref: refused,
            }
        ) if *refused == target_receipt_ref
    ));

    let expected_outcome = IntentOutcome::Applied {
        submission_id: submission.submission_id,
        receipt,
    };
    let expected_package_id = installed.package_id();
    drop(host);

    reset_callback_counts();
    let (mut reconstructed, reconstructed_worldline_id) = provider_runtime_host();
    assert_eq!(reconstructed_worldline_id, worldline_id);
    reconstructed
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("fresh trusted host activates the provider invocation WAL");
    let reconstructed_admitted = reconstructed
        .admit_provider_contract_package_v1(
            &admission_policy(),
            proposal(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256),
        )
        .expect("fresh host independently admits the exact provider proposal");
    let reconstructed_installed = reconstructed
        .install_admitted_provider_contract_package_v1_trusted(
            package_reference(),
            reconstructed_admitted,
        )
        .expect("fresh host reinstalls the exact provider package as host configuration");
    assert_eq!(reconstructed_installed.package_id(), expected_package_id);
    assert_no_callback();
    assert_eq!(
        reconstructed
            .app()
            .observe_intent_outcome(&submission.submission_id),
        expected_outcome
    );
    assert!(matches!(
        reconstructed
            .admit_provider_contract_submission_v1(submission.submission_id)
            .expect("recovered provider invocation remains idempotent"),
        warp_core::TicketedRuntimeIngressDisposition::Duplicate { .. }
    ));
    assert_eq!(
        reconstructed
            .run_until_idle(1)
            .expect("recovered provider invocation has no duplicate work")
            .committed_steps,
        0
    );
    assert_no_callback();
    drop(reconstructed);
    std::fs::remove_dir_all(&wal_root).expect("provider WAL fixture cleanup succeeds");
}

#[test]
fn provider_native_admission_refuses_unknown_and_malformed_eint_without_staging() {
    let _callback_guard = callback_test_guard();
    reset_callback_counts();
    let (mut host, worldline_id) = provider_runtime_host();
    let admitted = host
        .admit_provider_contract_package_v1(
            &admission_policy(),
            proposal(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256),
        )
        .expect("the exact provider proposal is admitted");
    host.install_admitted_provider_contract_package_v1_trusted(package_reference(), admitted)
        .expect("the exact provider package installs");

    let unknown_op_id = generated::OPERATION_ID.wrapping_add(1);
    let unknown = host
        .app()
        .submit_intent(IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("echo.intent/eint-v1"),
            echo_wasm_abi::pack_intent_v1(unknown_op_id, &[])
                .expect("unknown provider EINT still has canonical framing"),
        ))
        .expect("unknown provider intent is witnessed before admission");
    assert!(matches!(
        host.admit_provider_contract_submission_v1(unknown.submission_id),
        Err(RuntimeError::UnsupportedInstalledProviderContractMutation {
            op_id,
        }) if op_id == unknown_op_id
    ));

    let wrong_kind = make_intent_kind("echo.intent/not-eint-v1");
    let relabeled = host
        .app()
        .submit_intent(IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            wrong_kind,
            echo_wasm_abi::pack_intent_v1(generated::OPERATION_ID, &[])
                .expect("relabeled provider payload remains canonical EINT bytes"),
        ))
        .expect("relabeled provider intent is witnessed before admission");
    assert!(matches!(
        host.admit_provider_contract_submission_v1(relabeled.submission_id),
        Err(RuntimeError::InstalledContractIntentKindMismatch {
            expected,
            actual,
        }) if expected == make_intent_kind("echo.intent/eint-v1") && actual == wrong_kind
    ));

    let malformed = host
        .app()
        .submit_intent(IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("echo.intent/eint-v1"),
            b"not-canonical-eint".to_vec(),
        ))
        .expect("malformed provider intent is witnessed before admission");
    assert!(matches!(
        host.admit_provider_contract_submission_v1(malformed.submission_id),
        Err(RuntimeError::MalformedInstalledContractIntent)
    ));

    assert_eq!(host.runtime().ticketed_runtime_ingress_count(), 0);
    assert_no_callback();
}
