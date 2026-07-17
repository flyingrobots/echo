// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! Cargo-backed consumer contract for the checked generated Echo helper.

#[rustfmt::skip]
#[path = "../../../crates/echo-edict-provider-lowerer/tests/fixtures/generated_echo_dpo.rs"]
mod checked_generated_helper;

use checked_generated_helper::echo_dpo as generated;
use echo_wasm_abi::codec::CodecError;
use warp_core::{
    ContractPackageIdentity, Footprint, GraphView, NodeId, ProviderMutationHooksV1,
    ProviderMutationHostV1, ProviderPackageProposalErrorKind, TickDelta,
};

const SEMANTIC_DIGEST: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const RELEASE_DIGEST: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";
const PACKAGE_ARTIFACT_SHA256: &str =
    "3333333333333333333333333333333333333333333333333333333333333333";

const fn exact_pin() -> generated::ExpectedContractBundleIdentityV1<'static> {
    generated::ExpectedContractBundleIdentityV1 {
        semantic_digest_domain: generated::SEMANTIC_BUNDLE_DIGEST_DOMAIN,
        semantic_digest: SEMANTIC_DIGEST,
        release_digest_domain: generated::RELEASE_BUNDLE_DIGEST_DOMAIN,
        release_digest: RELEASE_DIGEST,
    }
}

const fn matching_identity() -> generated::ContractBundleIdentityV1<'static> {
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

fn bound_descriptor() -> generated::RegistrationDescriptorV1<'static> {
    let descriptor = generated::bind_contract_bundle(exact_pin(), &matching_identity())
        .expect("the exact checked claims bind the generated descriptor");
    assert_eq!(*descriptor.contract_bundle(), matching_identity());
    descriptor
}

const fn execute(_view: GraphView<'_>, _scope: &NodeId, _delta: &mut TickDelta) {}

fn footprint(_view: GraphView<'_>, _scope: &NodeId) -> Footprint {
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

const fn package_occurrence() -> ContractPackageIdentity<'static> {
    ContractPackageIdentity {
        package_name: "a.b",
        package_version: "1.0.0",
        artifact_hash_hex: PACKAGE_ARTIFACT_SHA256,
    }
}

#[test]
fn checked_generated_helper_refuses_the_wrong_value_codec_before_binding() {
    let identity = generated::ContractBundleIdentityV1 {
        value_codec: "wrong.codec/v1",
        ..matching_identity()
    };

    assert_eq!(
        generated::bind_contract_bundle(exact_pin(), &identity),
        Err(generated::BindingMismatchKind::Codec)
    );
}

#[test]
fn checked_generated_operation_codec_round_trips_exact_values() {
    let descriptor = bound_descriptor();
    assert_eq!(
        generated::Id::new("abc").expect("bounded id").into_string(),
        "abc"
    );
    let input = generated::Input::new("abc").expect("bounded ASCII input");
    let encoded = descriptor
        .encode_input(&input)
        .expect("bounded input encodes");
    assert_eq!(encoded, [3, 0, 0, 0, b'a', b'b', b'c']);
    assert_eq!(
        descriptor
            .decode_input(&encoded)
            .expect("canonical input decodes"),
        input
    );

    let output = generated::Output::new("abc").expect("bounded output");
    assert_eq!(output.id(), "abc");
    let output_bytes = descriptor
        .encode_output(&output)
        .expect("bounded output encodes");
    assert_eq!(
        descriptor
            .decode_output(&output_bytes)
            .expect("canonical output decodes"),
        output
    );

    let sixteen_four_byte_scalars = "💩".repeat(16);
    let boundary = generated::Input::new(sixteen_four_byte_scalars.clone())
        .expect("sixteen Unicode scalars are admitted regardless of UTF-8 width");
    let boundary_bytes = descriptor
        .encode_input(&boundary)
        .expect("the exact 64-byte UTF-8 boundary encodes");
    assert_eq!(&boundary_bytes[..4], &[64, 0, 0, 0]);
    assert_eq!(boundary_bytes.len(), 68);
    assert_eq!(
        descriptor
            .decode_input(&boundary_bytes)
            .expect("the exact scalar and physical bounds decode")
            .id(),
        sixteen_four_byte_scalars
    );
}

#[test]
fn checked_generated_operation_codec_rejects_invalid_or_non_exact_bytes() {
    let descriptor = bound_descriptor();

    assert_eq!(
        generated::Input::new("12345678901234567"),
        Err(CodecError::StringTooLong)
    );
    assert_eq!(
        generated::Input::new("💩".repeat(17)),
        Err(CodecError::StringTooLong)
    );
    assert_eq!(
        descriptor.decode_input(&[1, 0, 0, 0, 0xff]),
        Err(CodecError::InvalidUtf8)
    );
    assert_eq!(
        descriptor.decode_input(&[65, 0, 0, 0]),
        Err(CodecError::LengthTooLarge)
    );
    assert_eq!(
        descriptor.decode_input(&[3, 0, 0, 0, b'a', b'b']),
        Err(CodecError::OutOfBounds)
    );
    assert_eq!(
        descriptor.decode_input(&[1, 0, 0, 0, b'a', 0]),
        Err(CodecError::Trailing)
    );
}

#[test]
fn checked_generated_intent_uses_the_bound_operation_and_canonical_vars() {
    let descriptor = bound_descriptor();
    let input = generated::Input::new("abc").expect("bounded input");
    let intent = descriptor
        .pack_intent(&input)
        .expect("typed canonical EINT construction succeeds");
    let expected = [
        0x45, 0x49, 0x4e, 0x54, 0xb2, 0x34, 0x02, 0xca, 0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
        0x00, 0x61, 0x62, 0x63,
    ];
    assert_eq!(intent, expected);

    let (operation_id, vars) =
        echo_wasm_abi::unpack_intent_v1(&intent).expect("canonical EINT unpacks");
    assert_eq!(operation_id, generated::OPERATION_ID);
    assert_eq!(vars, &expected[12..]);
    assert_eq!(
        descriptor
            .decode_input(vars)
            .expect("the exact EINT vars decode as the generated input"),
        input
    );
}

#[test]
fn checked_generated_helper_constructs_the_exact_non_authoritative_package_proposal() {
    let descriptor = bound_descriptor();
    let registry = descriptor.provider_registry();
    let proposal = descriptor
        .propose_contract_package(
            package_occurrence(),
            ProviderMutationHooksV1::for_host::<TestHost>(
                descriptor.mutation_implementation_identity(),
            ),
        )
        .expect("exact implementation claims produce a package proposal");

    assert_eq!(proposal.occurrence(), &package_occurrence());
    assert_eq!(proposal.registry(), &registry);
    assert_eq!(
        proposal.mutation_operation_ids().collect::<Vec<_>>(),
        vec![generated::OPERATION_ID]
    );
    assert_eq!(registry.operations.len(), 1);
    assert_eq!(
        registry.operations[0].coordinate,
        generated::OPERATION_COORDINATE
    );
    assert_eq!(
        registry.operations[0].input.codec_id,
        generated::VALUE_CODEC_ID
    );
    assert_eq!(registry.bundle.semantic_digest, SEMANTIC_DIGEST);
    assert_eq!(registry.bundle.release_digest, RELEASE_DIGEST);
}

#[test]
fn checked_generated_helper_refuses_cross_bound_host_implementations() {
    let descriptor = bound_descriptor();
    let mut identity = descriptor.mutation_implementation_identity();
    identity.operation.target_ir.digest =
        "sha256:4444444444444444444444444444444444444444444444444444444444444444";
    let kind = descriptor
        .propose_contract_package(
            package_occurrence(),
            ProviderMutationHooksV1::for_host::<TestHost>(identity),
        )
        .err()
        .map(|error| error.kind());
    assert_eq!(
        kind,
        Some(ProviderPackageProposalErrorKind::TargetIrMismatch)
    );

    let mut identity = descriptor.mutation_implementation_identity();
    identity.operation.input.codec_id = "wrong-codec/v1";
    let kind = descriptor
        .propose_contract_package(
            package_occurrence(),
            ProviderMutationHooksV1::for_host::<TestHost>(identity),
        )
        .err()
        .map(|error| error.kind());
    assert_eq!(kind, Some(ProviderPackageProposalErrorKind::CodecMismatch));
}
