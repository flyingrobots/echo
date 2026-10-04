// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Deterministic primary artifact generation for the Echo Edict provider.

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

fn configuration(max_reads: i128, max_bytes: i128) -> CanonicalValueV1 {
    CanonicalValueV1::Map(vec![
        (
            CanonicalValueV1::Text("apiVersion".to_owned()),
            CanonicalValueV1::Text("echo.operation-lowering-configuration/v1".to_owned()),
        ),
        (
            CanonicalValueV1::Text("programKind".to_owned()),
            CanonicalValueV1::Text("compiler-produced-bounded-read/v1".to_owned()),
        ),
        (
            CanonicalValueV1::Text("maxReads".to_owned()),
            CanonicalValueV1::Integer(max_reads),
        ),
        (
            CanonicalValueV1::Text("maxReadBytes".to_owned()),
            CanonicalValueV1::Integer(max_bytes),
        ),
    ])
}

#[test]
fn read_configuration_requires_explicit_bounded_limits() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    for (reads, bytes) in [(1, 1), (64, 1_048_576), (65_536, 67_108_864)] {
        let encoded =
            encode_canonical_cbor_v1(&configuration(reads, bytes)).expect("canonical limits");
        generated
            .schema()
            .validate_root_bytes("echo-operation-lowering-configuration", &encoded)
            .expect("supported read bounds");
    }
    for (reads, bytes) in [(0, 1), (1, 0), (65_537, 1), (1, 67_108_865), (-1, 1)] {
        let encoded = encode_canonical_cbor_v1(&configuration(reads, bytes))
            .expect("canonical invalid limits");
        generated
            .schema()
            .validate_root_bytes("echo-operation-lowering-configuration", &encoded)
            .expect_err("unsupported read bounds");
    }
    let CanonicalValueV1::Map(fields) = configuration(64, 1_048_576) else {
        panic!("map");
    };
    for missing in ["maxReads", "maxReadBytes"] {
        let incomplete = CanonicalValueV1::Map(
            fields
                .iter()
                .filter(|(key, _)| *key != CanonicalValueV1::Text(missing.to_owned()))
                .cloned()
                .collect(),
        );
        let encoded =
            encode_canonical_cbor_v1(&incomplete).expect("canonical incomplete configuration");
        generated
            .schema()
            .validate_root_bytes("echo-operation-lowering-configuration", &encoded)
            .expect_err("read bounds cannot be implicit");
    }
    let mut specialized = fields;
    specialized.push((
        CanonicalValueV1::Text("operation".to_owned()),
        CanonicalValueV1::Text("consumer.operation@1".to_owned()),
    ));
    let encoded = encode_canonical_cbor_v1(&CanonicalValueV1::Map(specialized))
        .expect("canonical extra field");
    generated
        .schema()
        .validate_root_bytes("echo-operation-lowering-configuration", &encoded)
        .expect_err("application vocabulary is not provider configuration");
}

#[test]
fn compiler_produced_read_package_is_a_distinct_generic_package_variant() {
    let pack = admitted_pack();
    let (_, generated) = generate(SOURCE, &pack);
    let hash = || CanonicalValueV1::Bytes(vec![0x42; 32]);
    let package = CanonicalValueV1::Map(vec![
        (
            CanonicalValueV1::Text("authority_profile_identity".to_owned()),
            hash(),
        ),
        (
            CanonicalValueV1::Text("budget_ceiling".to_owned()),
            CanonicalValueV1::Map(vec![
                (
                    CanonicalValueV1::Text("max_allocated_bytes".to_owned()),
                    CanonicalValueV1::Integer(1024),
                ),
                (
                    CanonicalValueV1::Text("max_output_bytes".to_owned()),
                    CanonicalValueV1::Integer(1024),
                ),
                (
                    CanonicalValueV1::Text("max_steps".to_owned()),
                    CanonicalValueV1::Integer(64),
                ),
            ]),
        ),
        (
            CanonicalValueV1::Text("footprint_contract_identity".to_owned()),
            hash(),
        ),
        (
            CanonicalValueV1::Text("interpreter_profile_identity".to_owned()),
            hash(),
        ),
        (
            CanonicalValueV1::Text("operation_coordinate".to_owned()),
            CanonicalValueV1::Text("consumer.operation@1.run".to_owned()),
        ),
        (
            CanonicalValueV1::Text("package_kind".to_owned()),
            CanonicalValueV1::Text("compiler-produced-bounded-read/v1".to_owned()),
        ),
        (
            CanonicalValueV1::Text("program".to_owned()),
            CanonicalValueV1::Bytes(vec![0xa0]),
        ),
        (
            CanonicalValueV1::Text("schema".to_owned()),
            CanonicalValueV1::Text("echo.operation-package/v1".to_owned()),
        ),
        (
            CanonicalValueV1::Text("semantic_closure".to_owned()),
            CanonicalValueV1::Map(vec![
                (
                    CanonicalValueV1::Text("application_schema_coordinate".to_owned()),
                    CanonicalValueV1::Text("consumer.schema@1".to_owned()),
                ),
                (
                    CanonicalValueV1::Text("application_schema_identity".to_owned()),
                    hash(),
                ),
                (
                    CanonicalValueV1::Text("canonical_meaning_identity".to_owned()),
                    hash(),
                ),
                (CanonicalValueV1::Text("core_identity".to_owned()), hash()),
                (
                    CanonicalValueV1::Text("edict_source_identity".to_owned()),
                    hash(),
                ),
                (
                    CanonicalValueV1::Text("lawpack_coordinate".to_owned()),
                    CanonicalValueV1::Text("consumer.law@1".to_owned()),
                ),
                (
                    CanonicalValueV1::Text("lawpack_identity".to_owned()),
                    hash(),
                ),
                (
                    CanonicalValueV1::Text("target_ir_identity".to_owned()),
                    hash(),
                ),
            ]),
        ),
        (
            CanonicalValueV1::Text("target_profile_identity".to_owned()),
            hash(),
        ),
    ]);
    let package_bytes = encode_canonical_cbor_v1(&package)
        .expect("compiler-produced read package is canonical CBOR");

    generated
        .schema()
        .validate_root_bytes("echo-operation-package", &package_bytes)
        .expect("the generic bounded-read executable package variant is admitted");
}
