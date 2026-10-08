// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Projected CAS configuration and result-projection schema witnesses.

use echo_wesley_gen::provider_artifacts::{
    generate_provider_primary_artifacts_v1, ProviderPrimaryArtifactsV1,
};
use echo_wesley_gen::provider_canonical::{encode_canonical_cbor_v1, CanonicalValueV1};
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, AdmittedProviderContractPackV1,
};
use echo_wesley_gen::provider_generation::{
    build_provider_generation_input_v1, ProviderGenerationInputV1,
};

const SOURCE: &[u8] = include_bytes!("../assets/v1/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] = include_bytes!("../assets/v1/edict-provider/generation-settings-v1.json");
const CONTRACT_CDDL: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const CONTRACT_MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");

fn admitted_pack() -> AdmittedProviderContractPackV1 {
    admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked Edict provider contract pack is admitted")
}

fn build_input(source: &[u8], pack: &AdmittedProviderContractPackV1) -> ProviderGenerationInputV1 {
    build_provider_generation_input_v1(source, pack, SETTINGS)
        .expect("checked provider generation input builds")
}

fn generate(
    source: &[u8],
    pack: &AdmittedProviderContractPackV1,
) -> (ProviderGenerationInputV1, ProviderPrimaryArtifactsV1) {
    let input = build_input(source, pack);
    let artifacts = generate_provider_primary_artifacts_v1(&input, pack)
        .expect("checked primary provider artifacts generate");
    (input, artifacts)
}

fn text(value: &str) -> CanonicalValueV1 {
    CanonicalValueV1::Text(value.to_owned())
}
fn map(entries: Vec<(&str, CanonicalValueV1)>) -> CanonicalValueV1 {
    CanonicalValueV1::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

fn configuration(expected: bool, steps: i128) -> CanonicalValueV1 {
    let mut binding = vec![
        ("nodeIdDerivation", text("sha256-utf8/v1")),
        ("nodeKeyField", text("key")),
        ("replacementField", text("value")),
        ("warpIdSource", text("action-lane/v1")),
    ];
    if expected {
        binding.push(("expectedValueDigestField", text("expected")));
    }
    map(vec![
        (
            "apiVersion",
            text("echo.operation-lowering-configuration/v1"),
        ),
        (
            "programKind",
            text("anchored-node-attachment-compare-and-set/v1"),
        ),
        ("authorityProfile", text("test.authority/v1")),
        ("requiredNodeTypeProfile", text("test.node/v1")),
        ("requiredAttachmentTypeProfile", text("test.atom/v1")),
        ("maxReplacementBytes", CanonicalValueV1::Integer(256)),
        (
            "budgetCeiling",
            map(vec![
                ("steps", CanonicalValueV1::Integer(steps)),
                ("readBytes", CanonicalValueV1::Integer(1024)),
                ("writeBytes", CanonicalValueV1::Integer(1024)),
            ]),
        ),
        ("invocationBinding", map(binding)),
    ])
}

#[test]
fn cas_configuration_requires_digest_binding_and_four_steps() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    let bytes = encode_canonical_cbor_v1(&configuration(true, 4)).expect("canonical configuration");
    generated
        .schema()
        .validate_root_bytes("echo-operation-lowering-configuration", &bytes)
        .expect("valid CAS configuration");
    for value in [configuration(false, 4), configuration(true, 3)] {
        let bytes = encode_canonical_cbor_v1(&value).expect("canonical invalid configuration");
        let error = generated
            .schema()
            .validate_root_bytes("echo-operation-lowering-configuration", &bytes)
            .expect_err("invalid CAS configuration");
        assert_eq!(error.kind(), echo_wesley_gen::provider_artifacts::ProviderArtifactGenerationErrorKind::OwningRootRejected);
    }
}

#[test]
fn projected_cas_schema_accepts_digest_path_without_changing_create_shape() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    let mut fields = vec![
        (
            "application_input_node_key_path",
            CanonicalValueV1::Array(vec![text("key")]),
        ),
        (
            "application_input_replacement_path",
            CanonicalValueV1::Array(vec![text("value")]),
        ),
        ("artifact_bytes", CanonicalValueV1::Bytes(vec![0xa0])),
        ("artifact_identity", CanonicalValueV1::Bytes(vec![0; 32])),
        (
            "runtime_expression",
            map(vec![
                ("kind", text("source")),
                ("path", CanonicalValueV1::Array(vec![text("key")])),
                ("source", map(vec![("kind", text("applicationInput"))])),
            ]),
        ),
    ];
    for cas in [false, true] {
        if cas {
            fields.push((
                "application_input_expected_value_digest_path",
                CanonicalValueV1::Array(vec![text("expected")]),
            ));
        }
        let bytes =
            encode_canonical_cbor_v1(&map(fields.clone())).expect("canonical projection shape");
        generated
            .schema()
            .validate_root_bytes("echo-operation-application-result-projection", &bytes)
            .expect("valid projection shape");
    }
    for segment in [text(""), CanonicalValueV1::Integer(1)] {
        let mut invalid_fields = fields.clone();
        let (_, path) = invalid_fields
            .iter_mut()
            .find(|(key, _)| *key == "application_input_expected_value_digest_path")
            .expect("CAS digest path");
        *path = CanonicalValueV1::Array(vec![segment]);
        let bytes =
            encode_canonical_cbor_v1(&map(invalid_fields)).expect("canonical malformed projection");
        let error = generated
            .schema()
            .validate_root_bytes("echo-operation-application-result-projection", &bytes)
            .expect_err("malformed digest-path segment refuses");
        assert_eq!(
            error.kind(),
            echo_wesley_gen::provider_artifacts::ProviderArtifactGenerationErrorKind::OwningRootRejected
        );
    }
}
