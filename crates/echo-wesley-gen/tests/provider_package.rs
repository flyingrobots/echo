// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Deterministic, digest-locked Echo Edict provider package.

use std::sync::atomic::{AtomicUsize, Ordering};

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../echo-edict-provider-lowerer/tests/fixtures/generated_echo_dpo.rs"]
mod checked_generated_helper;

use checked_generated_helper::echo_dpo as generated;
use echo_wesley_gen::provider_artifacts::{
    generate_provider_primary_artifacts_v1, ProviderPrimaryArtifactsV1,
};
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, AdmittedProviderContractPackV1,
};
use echo_wesley_gen::provider_corpus::{
    checked_provider_artifact_corpus_v1, checked_provider_generator_source_bundle_v1,
};
use echo_wesley_gen::provider_generation::{
    build_provider_generation_input_v1, ProviderGenerationInputV1,
};
use echo_wesley_gen::provider_package::{
    admit_provider_package_v1, assemble_provider_package_v1,
    corroborate_admitted_provider_contract_package_v1,
    install_digest_corroborated_provider_contract_package_v1, ProviderManifestArtifactKindV1,
    ProviderManifestArtifactSourceV1, ProviderManifestResourceRefV1,
    ProviderPackageComponentMaterialV1, ProviderPackageCorroborationErrorKind,
    ProviderPackageErrorKind, ProviderPackageFileV1, ProviderPackageV1,
    ECHO_PROVIDER_MANIFEST_PATH_V1,
};
use echo_wesley_gen::provider_provenance::{
    generate_provider_generation_provenance_v1, ProviderGenerationProvenanceV1,
    ProviderGeneratorMaterialV1,
};
use echo_wesley_gen::provider_review::{
    generate_provider_generation_review_v1, ProviderGenerationReviewV1,
};
use warp_core::{
    make_node_id, make_type_id, ContractPackageIdentity, EngineBuilder, Footprint, GraphStore,
    GraphView, NodeId, NodeRecord, ProviderContractAdmissionPolicyV1,
    ProviderContractInstallationErrorKind, ProviderContractPackageProposalV1,
    ProviderMutationHooksV1, ProviderMutationHostV1, ProviderPackageReferenceV1, SchedulerKind,
    TickDelta, TrustedRuntimeHost, WorldlineRuntime,
};
use wesley_core::{compute_generation_artifact_digest_v1, GenerationContractErrorKind};

const SOURCE: &[u8] = include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] = include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json");
const CONTRACT_CDDL: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const CONTRACT_MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
const LOWERER: &[u8] = include_bytes!(
    "../assets/v1/edict-provider/package/v1/components/lowerer.echo-dpo.component.wasm"
);
const VERIFIER: &[u8] = include_bytes!(
    "../assets/v1/edict-provider/package/v1/components/verifier.echo-dpo.component.wasm"
);
const SEMANTIC_DIGEST: &str =
    "sha256:d3b9170373dc30369b1c7d3435f8c3d2183de063dc9e3b18d4b1f41eeac334c9";
const RELEASE_DIGEST: &str =
    "sha256:c39449495281b51f978468d08c21e93bcfa423176063b41675da61e4674b0066";
const PACKAGE_ARTIFACT_SHA256: &str =
    "ee870c75ec08c8818b3f80ab6562ae62a5cf741cd709edcee0085d951c5d5a7b";
const OTHER_PACKAGE_ARTIFACT_SHA256: &str =
    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

static EXECUTOR_CALLS: AtomicUsize = AtomicUsize::new(0);
static FOOTPRINT_CALLS: AtomicUsize = AtomicUsize::new(0);

fn bundle_pin() -> generated::ExpectedContractBundleIdentityV1<'static> {
    generated::ExpectedContractBundleIdentityV1 {
        semantic_digest_domain: generated::SEMANTIC_BUNDLE_DIGEST_DOMAIN,
        semantic_digest: SEMANTIC_DIGEST,
        release_digest_domain: generated::RELEASE_BUNDLE_DIGEST_DOMAIN,
        release_digest: RELEASE_DIGEST,
    }
}

fn bundle_identity() -> generated::ContractBundleIdentityV1<'static> {
    generated::ContractBundleIdentityV1 {
        semantic_digest_domain: generated::SEMANTIC_BUNDLE_DIGEST_DOMAIN,
        semantic_digest: SEMANTIC_DIGEST,
        release_digest_domain: generated::RELEASE_BUNDLE_DIGEST_DOMAIN,
        release_digest: RELEASE_DIGEST,
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

fn descriptor() -> generated::RegistrationDescriptorV1<'static> {
    generated::bind_contract_bundle(bundle_pin(), &bundle_identity())
        .expect("checked generated claims bind before Echo admission")
}

const fn occurrence(artifact_hash_hex: &'static str) -> ContractPackageIdentity<'static> {
    ContractPackageIdentity {
        package_name: "echo.edict-provider",
        package_version: "1.0.0",
        artifact_hash_hex,
    }
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

fn proposal(artifact_hash_hex: &'static str) -> ProviderContractPackageProposalV1<'static> {
    proposal_for_occurrence(occurrence(artifact_hash_hex))
}

fn proposal_for_occurrence(
    occurrence: ContractPackageIdentity<'static>,
) -> ProviderContractPackageProposalV1<'static> {
    let descriptor = Box::leak(Box::new(descriptor()));
    descriptor
        .propose_contract_package(
            occurrence,
            ProviderMutationHooksV1::for_host::<CountingHost>(
                descriptor.mutation_implementation_identity(),
            ),
        )
        .expect("checked generated and host claims produce a proposal")
}

fn corroborated_package(
    host: &TrustedRuntimeHost,
    occurrence: ContractPackageIdentity<'static>,
) -> echo_wesley_gen::provider_package::DigestCorroboratedProviderContractPackageV1<'static> {
    let package = assemble(false);
    let package_proof =
        admit_provider_package_v1(package.files().to_vec(), package.provider_reference())
            .expect("the checked 25-file package is independently digest-admitted");
    let policy = ProviderContractAdmissionPolicyV1 {
        expected_occurrence: occurrence,
        expected_registry: descriptor().provider_registry(),
    };
    let admitted = host
        .admit_provider_contract_package_v1(&policy, proposal_for_occurrence(occurrence))
        .expect("the matching proposal is independently admitted by Echo policy");
    corroborate_admitted_provider_contract_package_v1(package_proof, admitted)
        .expect("the exact package proof and admitted occurrence corroborate")
}

fn admission_policy(artifact_hash_hex: &'static str) -> ProviderContractAdmissionPolicyV1<'static> {
    ProviderContractAdmissionPolicyV1 {
        expected_occurrence: occurrence(artifact_hash_hex),
        expected_registry: descriptor().provider_registry(),
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
        .expect("trusted host initializes for package corroboration")
}

fn assert_no_install_or_callback(host: &TrustedRuntimeHost) {
    assert!(host
        .engine()
        .installed_contract_mutation_package_id(generated::OPERATION_ID)
        .is_none());
    assert_eq!(EXECUTOR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(FOOTPRINT_CALLS.load(Ordering::SeqCst), 0);
}

fn admitted_pack() -> AdmittedProviderContractPackV1 {
    admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked Edict provider contract pack is admitted")
}

fn generate() -> (
    ProviderGenerationInputV1,
    ProviderPrimaryArtifactsV1,
    ProviderGeneratorMaterialV1,
    ProviderGenerationProvenanceV1,
    ProviderGenerationReviewV1,
) {
    let pack = admitted_pack();
    let input = build_provider_generation_input_v1(SOURCE, &pack, SETTINGS)
        .expect("checked provider generation input builds");
    let primary = generate_provider_primary_artifacts_v1(&input, &pack)
        .expect("checked primary provider artifacts generate");
    let generator = checked_provider_generator_source_bundle_v1()
        .expect("checked generator source bundle builds")
        .generator_material()
        .expect("checked generator material builds");
    let provenance = generate_provider_generation_provenance_v1(&input, &primary, &generator)
        .expect("checked provider provenance generates");
    let review = generate_provider_generation_review_v1(&input, &provenance)
        .expect("checked provider review generates");
    (input, primary, generator, provenance, review)
}

fn components(reverse: bool) -> Vec<ProviderPackageComponentMaterialV1> {
    let mut components = vec![
        ProviderPackageComponentMaterialV1::new("lowerer.echo-dpo", LOWERER)
            .expect("lowerer material is bounded"),
        ProviderPackageComponentMaterialV1::new("verifier.echo-dpo", VERIFIER)
            .expect("verifier material is bounded"),
    ];
    if reverse {
        components.reverse();
    }
    components
}

fn assemble(reverse_components: bool) -> ProviderPackageV1 {
    let (input, primary, generator, provenance, review) = generate();
    assemble_provider_package_v1(
        &input,
        &primary,
        &generator,
        &provenance,
        &review,
        components(reverse_components),
    )
    .expect("checked provider package assembles")
}

fn replace_manifest(
    package: &ProviderPackageV1,
    mutate: impl FnOnce(&mut echo_wesley_gen::provider_package::ProviderManifestV1),
) -> Vec<ProviderPackageFileV1> {
    let mut manifest = package.manifest().clone();
    mutate(&mut manifest);
    let mut bytes = serde_json::to_vec_pretty(&manifest).expect("mutated manifest serializes");
    bytes.push(b'\n');
    let mut files = package.files().to_vec();
    let index = files
        .iter()
        .position(|file| file.relative_path() == ECHO_PROVIDER_MANIFEST_PATH_V1)
        .expect("manifest file exists");
    files[index] = ProviderPackageFileV1::new(ECHO_PROVIDER_MANIFEST_PATH_V1, &bytes)
        .expect("manifest path is valid");
    files
}

#[test]
fn checked_materials_assemble_one_digest_locked_provider_package() {
    let package = assemble(false);
    let reordered = assemble(true);

    assert_eq!(package, reordered);
    assert_eq!(package.members().len(), 24);
    assert_eq!(package.files().len(), 25);
    assert_eq!(package.manifest().artifacts.len(), 10);
    assert_eq!(package.manifest().schema_bindings.len(), 24);
    assert_eq!(
        package
            .manifest()
            .artifacts
            .iter()
            .map(|artifact| artifact.role.as_str())
            .collect::<Vec<_>>(),
        vec![
            "authority-facts.echo-dpo",
            "authority-facts.echo-lawpack",
            "generated-artifact-profile.echo-dpo-registration",
            "lawpack.echo-dpo",
            "lowerer.echo-dpo",
            "provenance.provider-generation",
            "review.provider-generation",
            "schema.echo-provider-artifacts",
            "target-profile.echo-dpo",
            "verifier.echo-dpo",
        ]
    );
    assert!(
        package
            .manifest()
            .artifacts
            .iter()
            .all(|artifact| artifact.artifact_kind
                != ProviderManifestArtifactKindV1::ProviderManifest)
    );
    assert!(package
        .members()
        .iter()
        .all(|member| member.relative_path() != ECHO_PROVIDER_MANIFEST_PATH_V1));
    assert_eq!(
        package
            .files()
            .iter()
            .filter(|file| file.relative_path() == ECHO_PROVIDER_MANIFEST_PATH_V1)
            .count(),
        1
    );
    assert!(package.manifest_bytes().ends_with(b"\n"));
    assert_eq!(package.provider_reference(), &package.manifest().provider);
    assert_ne!(
        package.provider_reference().digest,
        package.manifest_content_reference().digest
    );
    assert!(package
        .files()
        .windows(2)
        .all(|pair| pair[0].relative_path().as_bytes() < pair[1].relative_path().as_bytes()));

    let admitted =
        admit_provider_package_v1(package.files().to_vec(), package.provider_reference())
            .expect("exact package admits against its external identity");
    assert_eq!(admitted.provider_reference(), package.provider_reference());
    assert_eq!(admitted.manifest(), package.manifest());
}

#[test]
fn digest_admitted_package_corroborates_only_the_exact_host_admitted_occurrence() {
    EXECUTOR_CALLS.store(0, Ordering::SeqCst);
    FOOTPRINT_CALLS.store(0, Ordering::SeqCst);

    let package = assemble(false);
    let exact_proof =
        admit_provider_package_v1(package.files().to_vec(), package.provider_reference())
            .expect("the checked 25-file package is independently digest-admitted");
    let host = empty_host();
    let exact_policy = admission_policy(PACKAGE_ARTIFACT_SHA256);
    let admitted = host
        .admit_provider_contract_package_v1(&exact_policy, proposal(PACKAGE_ARTIFACT_SHA256))
        .expect("the matching proposal is independently admitted by Echo policy");
    let corroborated = corroborate_admitted_provider_contract_package_v1(exact_proof, admitted)
        .expect("the exact package proof and admitted occurrence corroborate");

    assert_eq!(
        corroborated.provider_reference().coordinate,
        "echo.edict-provider@1"
    );
    assert_eq!(
        corroborated.provider_reference().digest,
        format!("sha256:{PACKAGE_ARTIFACT_SHA256}")
    );
    assert_eq!(corroborated.occurrence(), &exact_policy.expected_occurrence);
    assert_eq!(corroborated.registry(), &exact_policy.expected_registry);
    assert_eq!(
        corroborated.mutation_operation_ids().collect::<Vec<_>>(),
        vec![generated::OPERATION_ID]
    );
    assert_no_install_or_callback(&host);

    let mismatch_proof =
        admit_provider_package_v1(package.files().to_vec(), package.provider_reference())
            .expect("the same checked package can be independently admitted again");
    let other_policy = admission_policy(OTHER_PACKAGE_ARTIFACT_SHA256);
    let other_admitted = host
        .admit_provider_contract_package_v1(&other_policy, proposal(OTHER_PACKAGE_ARTIFACT_SHA256))
        .expect("the internally consistent alternate occurrence claim admits under its own policy");
    let error = corroborate_admitted_provider_contract_package_v1(mismatch_proof, other_admitted)
        .expect_err("a proposal naming different package bytes must not corroborate");
    assert_eq!(
        error.kind(),
        ProviderPackageCorroborationErrorKind::PackageArtifactDigestMismatch
    );
    assert_eq!(error.subject(), "provider.package.artifact-hash");
    assert_eq!(error.reference(), Some(OTHER_PACKAGE_ARTIFACT_SHA256));
    assert_no_install_or_callback(&host);
}

#[test]
fn digest_corroborated_package_installs_as_provider_native_evidence() {
    EXECUTOR_CALLS.store(0, Ordering::SeqCst);
    FOOTPRINT_CALLS.store(0, Ordering::SeqCst);

    let package = assemble(false);
    let package_proof =
        admit_provider_package_v1(package.files().to_vec(), package.provider_reference())
            .expect("the checked 25-file package is independently digest-admitted");
    let mut host = empty_host();
    let policy = admission_policy(PACKAGE_ARTIFACT_SHA256);
    let admitted = host
        .admit_provider_contract_package_v1(&policy, proposal(PACKAGE_ARTIFACT_SHA256))
        .expect("the matching proposal is independently admitted by Echo policy");
    let corroborated = corroborate_admitted_provider_contract_package_v1(package_proof, admitted)
        .expect("the exact package proof and admitted occurrence corroborate");

    let installed =
        install_digest_corroborated_provider_contract_package_v1(&mut host, corroborated)
            .expect("the corroborated package installs through the provider-native boundary");

    assert_eq!(
        installed.package_reference().coordinate(),
        "echo.edict-provider@1"
    );
    assert_eq!(
        installed.package_reference().digest(),
        format!("sha256:{PACKAGE_ARTIFACT_SHA256}")
    );
    assert_eq!(installed.occurrence(), &policy.expected_occurrence);
    assert_eq!(installed.registry(), &policy.expected_registry);
    assert_eq!(
        installed.mutation_operation_ids().collect::<Vec<_>>(),
        vec![generated::OPERATION_ID]
    );
    assert_eq!(
        host.engine()
            .installed_provider_contract_mutation_package_id(generated::OPERATION_ID),
        Some(installed.package_id())
    );
    assert!(host
        .engine()
        .installed_contract_mutation_package_id(generated::OPERATION_ID)
        .is_none());
    assert!(host
        .engine()
        .installed_contract_mutation_evidence(generated::OPERATION_ID)
        .is_none());
    assert_eq!(EXECUTOR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(FOOTPRINT_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn duplicate_provider_installation_refuses_without_changing_owned_indexes() {
    EXECUTOR_CALLS.store(0, Ordering::SeqCst);
    FOOTPRINT_CALLS.store(0, Ordering::SeqCst);

    let mut host = empty_host();
    let first = corroborated_package(&host, occurrence(PACKAGE_ARTIFACT_SHA256));
    let installed = install_digest_corroborated_provider_contract_package_v1(&mut host, first)
        .expect("the first exact provider occurrence installs");

    let duplicate = corroborated_package(&host, occurrence(PACKAGE_ARTIFACT_SHA256));
    let error = install_digest_corroborated_provider_contract_package_v1(&mut host, duplicate)
        .expect_err("the same provider package id must not install twice");
    assert_eq!(
        error.kind(),
        ProviderContractInstallationErrorKind::DuplicatePackageId
    );
    assert_eq!(error.subject(), "provider.package.id");
    assert_eq!(error.reference(), None);
    assert_eq!(
        host.engine()
            .installed_provider_contract_mutation_package_id(generated::OPERATION_ID),
        Some(installed.package_id())
    );
    assert_eq!(
        host.engine()
            .installed_provider_contract_package(&installed.package_id()),
        Some(&installed)
    );
    assert_eq!(EXECUTOR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(FOOTPRINT_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn one_exact_package_root_cannot_claim_two_runtime_occurrences() {
    EXECUTOR_CALLS.store(0, Ordering::SeqCst);
    FOOTPRINT_CALLS.store(0, Ordering::SeqCst);

    let mut host = empty_host();
    let first = corroborated_package(&host, occurrence(PACKAGE_ARTIFACT_SHA256));
    let installed = install_digest_corroborated_provider_contract_package_v1(&mut host, first)
        .expect("the first exact provider occurrence installs");

    let alternate_occurrence = ContractPackageIdentity {
        package_name: "echo.edict-provider",
        package_version: "1.0.0-alternate-claim",
        artifact_hash_hex: PACKAGE_ARTIFACT_SHA256,
    };
    let conflicting = corroborated_package(&host, alternate_occurrence);
    let error = install_digest_corroborated_provider_contract_package_v1(&mut host, conflicting)
        .expect_err("one exact package root must not acquire a second occurrence claim");
    assert_eq!(
        error.kind(),
        ProviderContractInstallationErrorKind::DuplicatePackageReference
    );
    assert_eq!(error.subject(), "provider.package.reference");
    assert_eq!(
        error.reference(),
        Some(format!("sha256:{PACKAGE_ARTIFACT_SHA256}").as_str())
    );
    let package_reference = ProviderPackageReferenceV1::new(
        "echo.edict-provider@1",
        format!("sha256:{PACKAGE_ARTIFACT_SHA256}"),
    );
    assert_eq!(
        host.engine()
            .installed_provider_contract_package_by_reference(&package_reference),
        Some(&installed)
    );
    assert_eq!(
        host.engine()
            .installed_provider_contract_mutation_package_id(generated::OPERATION_ID),
        Some(installed.package_id())
    );
    assert_eq!(EXECUTOR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(FOOTPRINT_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn package_generated_members_equal_the_current_checked_provider_corpus() {
    let package = assemble(false);
    let checked = checked_provider_artifact_corpus_v1()
        .expect("checked #652 provider artifact corpus is an exact bounded value");
    assert_eq!(checked.files().len(), 22);
    for checked_file in checked.files() {
        let package_path = format!("generated/{}", checked_file.relative_path());
        let packaged = package
            .files()
            .iter()
            .find(|file| file.relative_path() == package_path)
            .expect("every checked #652 file is a package member");
        assert_eq!(packaged.bytes(), checked_file.bytes());
    }
}

#[test]
fn manifest_routes_preserve_semantic_and_exact_byte_digest_classes() {
    let (input, primary, generator, provenance, review) = generate();
    let package = assemble_provider_package_v1(
        &input,
        &primary,
        &generator,
        &provenance,
        &review,
        components(false),
    )
    .expect("checked provider package assembles");

    for output in primary.artifacts() {
        let route = package
            .manifest()
            .artifacts
            .iter()
            .find(|artifact| artifact.role == output.role())
            .expect("canonical output has a manifest route");
        assert_eq!(route.resource.digest, output.domain_framed_digest());
        assert_ne!(route.resource.digest, output.content_reference().digest);
        let member = package
            .members()
            .iter()
            .find(|member| member.relative_path().contains(output.role()))
            .expect("canonical output has a physical member");
        assert_eq!(member.bytes(), output.canonical_bytes());
        assert_ne!(member.raw_digest(), route.resource.digest);
    }

    let schema = package
        .manifest()
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_kind == ProviderManifestArtifactKindV1::ArtifactSchema)
        .expect("schema route exists");
    assert_eq!(
        schema.resource.digest,
        primary.schema().content_reference().digest
    );
    let provenance_route = package
        .manifest()
        .artifacts
        .iter()
        .find(|artifact| {
            artifact.artifact_kind == ProviderManifestArtifactKindV1::GenerationProvenance
        })
        .expect("provenance route exists");
    assert_eq!(
        provenance_route.resource.digest,
        provenance.content_reference().digest
    );
    let review_route = package
        .manifest()
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_kind == ProviderManifestArtifactKindV1::ReviewArtifact)
        .expect("review route exists");
    assert_eq!(
        review_route.resource.digest,
        review.content_reference().digest
    );
}

#[test]
fn generated_routes_require_one_exact_source_and_generator_reference() {
    let package = assemble(false);
    let files = replace_manifest(&package, |manifest| {
        let source = match &mut manifest.artifacts[0].source {
            ProviderManifestArtifactSourceV1::Generated {
                semantic_source, ..
            } => semantic_source,
            ProviderManifestArtifactSourceV1::Component { .. } => {
                panic!("first artifact is generated")
            }
        };
        source.digest =
            "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_owned();
    });
    let error = admit_provider_package_v1(files, package.provider_reference())
        .expect_err("generated routes cannot disagree on exact source identity");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::ArtifactSourceMismatch
    );
}

#[test]
fn agreeing_generated_routes_must_match_packaged_wesley_provenance() {
    let package = assemble(false);
    let files = replace_manifest(&package, |manifest| {
        for artifact in &mut manifest.artifacts {
            if let ProviderManifestArtifactSourceV1::Generated {
                semantic_source, ..
            } = &mut artifact.source
            {
                semantic_source.digest =
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                        .to_owned();
            }
        }
    });
    let error = admit_provider_package_v1(files, package.provider_reference())
        .expect_err("agreeing fake routes cannot replace packaged provenance");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::GenerationProvenanceMismatch
    );
}

#[test]
fn packaged_wesley_evidence_preserves_the_typed_contract_failure() {
    let package = assemble(false);
    let provenance_path = "generated/evidence/provenance.provider-generation.json";
    let provenance_file = package
        .files()
        .iter()
        .find(|file| file.relative_path() == provenance_path)
        .expect("provenance file exists");
    let mut provenance: serde_json::Value =
        serde_json::from_slice(provenance_file.bytes()).expect("provenance is JSON");
    provenance["contractVersions"]["inputSchema"] =
        serde_json::Value::String("wesley.wrong/v1".to_owned());
    let changed_bytes = serde_json::to_vec(&provenance).expect("changed provenance serializes");
    let changed_digest = compute_generation_artifact_digest_v1(&changed_bytes);
    let mut files = replace_manifest(&package, |manifest| {
        manifest
            .artifacts
            .iter_mut()
            .find(|artifact| {
                artifact.artifact_kind == ProviderManifestArtifactKindV1::GenerationProvenance
            })
            .expect("provenance route exists")
            .resource
            .digest = changed_digest;
    });
    let index = files
        .iter()
        .position(|file| file.relative_path() == provenance_path)
        .expect("provenance file exists");
    files[index] = ProviderPackageFileV1::new(provenance_path, &changed_bytes)
        .expect("provenance path is valid");

    let error = admit_provider_package_v1(files, package.provider_reference())
        .expect_err("wrong Wesley contract version must fail before package use");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::GenerationProvenanceMismatch
    );
    assert_eq!(
        error.wesley_contract_kind(),
        Some(GenerationContractErrorKind::ContractVersionMismatch)
    );
}

#[test]
fn valid_looking_raw_digest_cannot_replace_a_canonical_artifact_identity() {
    let (input, primary, generator, provenance, review) = generate();
    let package = assemble_provider_package_v1(
        &input,
        &primary,
        &generator,
        &provenance,
        &review,
        components(false),
    )
    .expect("checked provider package assembles");
    let raw_lawpack_digest = primary
        .artifact("lawpack.echo-dpo")
        .expect("lawpack exists")
        .content_reference()
        .digest
        .clone();
    let files = replace_manifest(&package, |manifest| {
        manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.role == "lawpack.echo-dpo")
            .expect("lawpack route exists")
            .resource
            .digest = raw_lawpack_digest;
    });
    let error = admit_provider_package_v1(files, package.provider_reference())
        .expect_err("raw byte identity cannot impersonate the Edict semantic identity");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::ArtifactDigestMismatch
    );
}

#[test]
fn opaque_component_bytes_move_the_digest_root_without_proving_host_readiness() {
    let baseline = assemble(false);
    let (input, primary, generator, provenance, review) = generate();
    let mut changed_lowerer = LOWERER.to_vec();
    changed_lowerer[0] ^= 1;
    let changed = assemble_provider_package_v1(
        &input,
        &primary,
        &generator,
        &provenance,
        &review,
        vec![
            ProviderPackageComponentMaterialV1::new("lowerer.echo-dpo", &changed_lowerer)
                .expect("changed lowerer remains bounded explicit material"),
            ProviderPackageComponentMaterialV1::new("verifier.echo-dpo", VERIFIER)
                .expect("verifier remains bounded explicit material"),
        ],
    )
    .expect("coherently changed package assembles a different identity");
    assert_ne!(changed.provider_reference(), baseline.provider_reference());
    let error = admit_provider_package_v1(changed.files().to_vec(), baseline.provider_reference())
        .expect_err("coherent internal rebinding cannot cross the caller's old pin");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::ProviderIdentityMismatch
    );
}

#[test]
fn package_admission_rejects_manifest_member_and_expected_root_tampering() {
    let package = assemble(false);

    let mut alternate_manifest = package.files().to_vec();
    let manifest_index = alternate_manifest
        .iter()
        .position(|file| file.relative_path() == ECHO_PROVIDER_MANIFEST_PATH_V1)
        .expect("manifest file exists");
    let mut alternate_bytes = alternate_manifest[manifest_index].bytes().to_vec();
    alternate_bytes.push(b'\n');
    alternate_manifest[manifest_index] =
        ProviderPackageFileV1::new(ECHO_PROVIDER_MANIFEST_PATH_V1, &alternate_bytes)
            .expect("manifest path is valid");
    let error = admit_provider_package_v1(alternate_manifest, package.provider_reference())
        .expect_err("alternate JSON occurrence must fail exact rendering");
    assert_eq!(error.kind(), ProviderPackageErrorKind::ManifestNoncanonical);

    let mut resource_tamper = package.files().to_vec();
    let resource_index = resource_tamper
        .iter()
        .position(|file| file.relative_path() == "generated/resources/resource.target-ir.cbor")
        .expect("generated resource exists");
    let mut tampered_bytes = resource_tamper[resource_index].bytes().to_vec();
    tampered_bytes[0] ^= 1;
    resource_tamper[resource_index] = ProviderPackageFileV1::new(
        "generated/resources/resource.target-ir.cbor",
        &tampered_bytes,
    )
    .expect("resource path is valid");
    let error = admit_provider_package_v1(resource_tamper, package.provider_reference())
        .expect_err("unrouted resource tamper must move the pinned package root");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::ProviderIdentityMismatch
    );

    let mut component_tamper = package.files().to_vec();
    let lowerer_index = component_tamper
        .iter()
        .position(|file| file.relative_path() == "components/lowerer.echo-dpo.component.wasm")
        .expect("lowerer component exists");
    let mut tampered_bytes = component_tamper[lowerer_index].bytes().to_vec();
    tampered_bytes[0] ^= 1;
    component_tamper[lowerer_index] = ProviderPackageFileV1::new(
        "components/lowerer.echo-dpo.component.wasm",
        &tampered_bytes,
    )
    .expect("component path is valid");
    let error = admit_provider_package_v1(component_tamper, package.provider_reference())
        .expect_err("routed component tamper must fail its exact manifest identity");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::ArtifactDigestMismatch
    );

    let wrong_expected = ProviderManifestResourceRefV1 {
        coordinate: package.provider_reference().coordinate.clone(),
        digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            .to_owned(),
    };
    let error = admit_provider_package_v1(package.files().to_vec(), &wrong_expected)
        .expect_err("internally coherent package cannot replace caller identity");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::ProviderIdentityMismatch
    );
}

#[test]
fn component_material_closure_fails_without_discovery_or_winner_selection() {
    let (input, primary, generator, provenance, review) = generate();
    let error = assemble_provider_package_v1(
        &input,
        &primary,
        &generator,
        &provenance,
        &review,
        vec![
            ProviderPackageComponentMaterialV1::new("verifier.echo-dpo", VERIFIER)
                .expect("verifier material is bounded"),
        ],
    )
    .expect_err("missing lowerer bytes must fail closed");
    assert_eq!(error.kind(), ProviderPackageErrorKind::ComponentMissing);
    assert_eq!(error.subject(), "lowerer.echo-dpo");

    let error = assemble_provider_package_v1(
        &input,
        &primary,
        &generator,
        &provenance,
        &review,
        vec![
            ProviderPackageComponentMaterialV1::new("lowerer.echo-dpo", LOWERER)
                .expect("lowerer material is bounded"),
            ProviderPackageComponentMaterialV1::new("lowerer.echo-dpo", VERIFIER)
                .expect("conflicting duplicate remains explicit material"),
            ProviderPackageComponentMaterialV1::new("verifier.echo-dpo", VERIFIER)
                .expect("verifier material is bounded"),
        ],
    )
    .expect_err("conflicting duplicate roles must not select a winner");
    assert_eq!(error.kind(), ProviderPackageErrorKind::ComponentDuplicate);
    assert_eq!(error.subject(), "lowerer.echo-dpo");

    let mut with_unexpected = components(false);
    with_unexpected.push(
        ProviderPackageComponentMaterialV1::new("component.unexpected", b"explicit")
            .expect("unexpected material remains bounded"),
    );
    let error = assemble_provider_package_v1(
        &input,
        &primary,
        &generator,
        &provenance,
        &review,
        with_unexpected,
    )
    .expect_err("unexpected component material must not be ignored");
    assert_eq!(error.kind(), ProviderPackageErrorKind::ComponentUnexpected);
    assert_eq!(error.subject(), "component.unexpected");
}

#[test]
fn raw_admission_preserves_component_material_bounds() {
    let package = assemble(false);
    let empty_digest = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let mut files = replace_manifest(&package, |manifest| {
        let lowerer = manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.role == "lowerer.echo-dpo")
            .expect("lowerer route exists");
        lowerer.resource.digest = empty_digest.to_owned();
        match &mut lowerer.source {
            ProviderManifestArtifactSourceV1::Component { component } => {
                component.digest = empty_digest.to_owned();
            }
            ProviderManifestArtifactSourceV1::Generated { .. } => {
                panic!("lowerer has component provenance")
            }
        }
    });
    let lowerer_index = files
        .iter()
        .position(|file| file.relative_path() == "components/lowerer.echo-dpo.component.wasm")
        .expect("lowerer file exists");
    files[lowerer_index] =
        ProviderPackageFileV1::new("components/lowerer.echo-dpo.component.wasm", b"")
            .expect("empty bytes do not change the safe path");
    let error = admit_provider_package_v1(files, package.provider_reference())
        .expect_err("raw admission cannot bypass the component byte bound");
    assert_eq!(error.kind(), ProviderPackageErrorKind::ComponentInvalid);
}

#[test]
fn package_file_inventory_requires_exact_unique_path_order() {
    let package = assemble(false);

    let mut reversed = package.files().to_vec();
    reversed.reverse();
    let error = admit_provider_package_v1(reversed, package.provider_reference())
        .expect_err("admission must not normalize supplied file order");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::PackageMemberOutOfOrder
    );

    let mut missing = package.files().to_vec();
    missing.remove(0);
    let error = admit_provider_package_v1(missing, package.provider_reference())
        .expect_err("missing package member must fail closed");
    assert_eq!(error.kind(), ProviderPackageErrorKind::PackageMemberMissing);

    let mut duplicate = package.files().to_vec();
    duplicate.push(duplicate[0].clone());
    duplicate.sort_by(|left, right| left.relative_path().cmp(right.relative_path()));
    let error = admit_provider_package_v1(duplicate, package.provider_reference())
        .expect_err("identical duplicate member remains ambiguous");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::PackageMemberDuplicate
    );

    let mut unexpected = package.files().to_vec();
    unexpected[0] = ProviderPackageFileV1::new("components/unknown.component.wasm", b"unknown")
        .expect("unexpected path is syntactically safe");
    unexpected.sort_by(|left, right| left.relative_path().cmp(right.relative_path()));
    let error = admit_provider_package_v1(unexpected, package.provider_reference())
        .expect_err("safe but undeclared member must fail closed");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::PackageMemberUnexpected
    );
}

#[test]
fn manifest_contract_source_and_schema_mutations_fail_before_root_comparison() {
    let package = assemble(false);

    let files = replace_manifest(&package, |manifest| {
        manifest.schema_bindings[0].root_rule = "wrong-root".to_owned();
    });
    let error = admit_provider_package_v1(files, package.provider_reference())
        .expect_err("wrong existing schema route must fail as a schema disagreement");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::SchemaBindingMismatch
    );

    let generated_source = package
        .manifest()
        .artifacts
        .iter()
        .find_map(|artifact| match &artifact.source {
            ProviderManifestArtifactSourceV1::Generated { .. } => Some(artifact.source.clone()),
            ProviderManifestArtifactSourceV1::Component { .. } => None,
        })
        .expect("generated source exists");
    let files = replace_manifest(&package, |manifest| {
        manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.role == "lowerer.echo-dpo")
            .expect("lowerer route exists")
            .source = generated_source;
    });
    let error = admit_provider_package_v1(files, package.provider_reference())
        .expect_err("component role cannot use generated provenance");
    assert_eq!(
        error.kind(),
        ProviderPackageErrorKind::ArtifactSourceMismatch
    );

    let files = replace_manifest(&package, |manifest| {
        manifest.provider.digest =
            "SHA256:0000000000000000000000000000000000000000000000000000000000000000".to_owned();
    });
    let error = admit_provider_package_v1(files, package.provider_reference())
        .expect_err("digest textual aliases must not be normalized");
    assert_eq!(error.kind(), ProviderPackageErrorKind::DigestInvalid);
}
