// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Integration tests for the canonical Echo provider generation input.

use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
use echo_wesley_gen::provider_generation::{
    build_provider_generation_input_v1, ProviderGenerationErrorKind,
};
use echo_wesley_gen::provider_semantics::ProviderSemanticSourceErrorKind;
use serde_json::{json, Value};
use wesley_core::compute_generation_artifact_digest_v1;

const SOURCE: &[u8] =
    include_bytes!("../../../schemas/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] =
    include_bytes!("../../../schemas/edict-provider/generation-settings-v1.json");
const CONTRACT_CDDL: &[u8] =
    include_bytes!("../../../schemas/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const CONTRACT_MANIFEST: &[u8] =
    include_bytes!("../../../schemas/edict-provider/contracts/v1/manifest.json");

fn build(
    source: &[u8],
    settings: &[u8],
) -> echo_wesley_gen::provider_generation::ProviderGenerationInputV1 {
    let pack = admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked Edict provider contract pack is admitted");
    build_provider_generation_input_v1(source, &pack, settings)
        .expect("checked provider generation inputs build")
}

fn reordered_source() -> Vec<u8> {
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    for pointer in [
        "/types/0/shape/fields",
        "/types/1/shape/fields",
        "/types/2/shape/fields",
        "/effects/0/guardKinds",
        "/effects/0/failures",
        "/profiles/0/sourceNames",
        "/profiles/0/allowedWriteClasses",
        "/profiles/0/guardKinds",
        "/operations/0/obstructionMappings",
        "/lawpackProjection/acceptedCoreAbis",
        "/lawpackProjection/dependencies",
        "/lawpackProjection/targetAdapters",
        "/lawpackProjection/targetAdapters/0/effects",
        "/targetProfileProjection/acceptedCoreAbis",
        "/targetProfileProjection/generatedArtifactProfileRoles",
        "/targetProfileProjection/acceptedLawpackAdapterAbis",
    ] {
        source
            .pointer_mut(pointer)
            .and_then(Value::as_array_mut)
            .expect("nested set-like field is an array")
            .reverse();
    }
    for key in [
        "authoritySources",
        "types",
        "writeClasses",
        "obstructions",
        "effects",
        "profiles",
        "budgets",
        "capabilities",
        "directAdapters",
        "operations",
        "artifactResources",
        "generatedArtifacts",
        "invocationInputs",
        "invocationOutputs",
        "schemaBindings",
    ] {
        source[key]
            .as_array_mut()
            .expect("set-like declaration family is an array")
            .reverse();
    }
    serde_json::to_vec(&source).expect("reordered source serializes")
}

#[test]
fn checked_inputs_build_one_canonical_wesley_generation_input() {
    let built = build(SOURCE, SETTINGS);
    let input = built.wesley_input();

    assert_eq!(input.shape_ir.version, "1.0.0");
    assert!(input.shape_ir.metadata.is_none());
    assert!(input.shape_ir.types.is_empty());
    assert!(input.operations.is_empty());
    assert!(input
        .operations
        .iter()
        .all(|operation| operation.field_name != "a.b@1.t"));
    assert!(input.law.is_none());
    assert_eq!(
        input
            .owner_declarations
            .iter()
            .map(|reference| reference.coordinate.as_str())
            .collect::<Vec<_>>(),
        vec![
            "echo.semantic-schema@1",
            "edict.provider-contract-pack.cddl@1",
            "edict.provider-contract-pack.manifest@1",
        ]
    );
    assert_eq!(
        input.projection_roles,
        vec![
            "authority-facts.echo-dpo",
            "authority-facts.echo-lawpack",
            "generated-artifact-profile.echo-dpo-registration",
            "lawpack.echo-dpo",
            "schema.echo-provider-artifacts",
            "target-profile.echo-dpo",
        ]
    );
    assert_eq!(
        built.canonical_bytes(),
        input.canonical_bytes().expect("Wesley input encodes")
    );
    assert_eq!(built.digest(), input.digest().expect("Wesley input hashes"));
    assert_eq!(built.settings_bytes(), SETTINGS);
    assert_eq!(
        input.settings_digest,
        compute_generation_artifact_digest_v1(SETTINGS)
    );

    let source_artifacts = built.source_artifacts();
    assert_eq!(source_artifacts.len(), 3);
    assert_eq!(source_artifacts[0].coordinate, "echo.semantic-schema@1");
    assert_eq!(source_artifacts[0].bytes.as_slice(), SOURCE);
    assert_eq!(
        source_artifacts[1].coordinate,
        "edict.provider-contract-pack.cddl@1"
    );
    assert_eq!(source_artifacts[1].bytes.as_slice(), CONTRACT_CDDL);
    assert_eq!(
        source_artifacts[2].coordinate,
        "edict.provider-contract-pack.manifest@1"
    );
    assert_eq!(source_artifacts[2].bytes.as_slice(), CONTRACT_MANIFEST);
    assert_eq!(
        input.owner_declarations,
        source_artifacts
            .iter()
            .map(wesley_core::GenerationArtifactContentV1::reference)
            .collect::<Vec<_>>()
    );
}

#[test]
fn semantic_set_reordering_preserves_the_model_but_moves_exact_source_evidence() {
    let baseline = build(SOURCE, SETTINGS);
    let reordered = build(&reordered_source(), SETTINGS);

    assert_eq!(baseline.semantic_source(), reordered.semantic_source());
    assert_ne!(baseline.digest(), reordered.digest());
}

#[test]
fn exact_settings_bytes_move_generation_input_identity() {
    let baseline = build(SOURCE, SETTINGS);
    let compact = serde_json::to_vec(
        &serde_json::from_slice::<Value>(SETTINGS).expect("checked settings are JSON"),
    )
    .expect("settings compact");
    let reformatted = build(SOURCE, &compact);

    assert_ne!(baseline.digest(), reformatted.digest());
}

#[test]
fn invalid_generation_settings_have_stable_failure_kinds() {
    let pack = admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked Edict provider contract pack is admitted");
    let malformed = build_provider_generation_input_v1(SOURCE, &pack, b"{")
        .expect_err("malformed settings fail");
    assert_eq!(
        malformed.kind(),
        ProviderGenerationErrorKind::SettingsMalformed
    );

    let unsupported_api = serde_json::to_vec(&json!({
        "apiVersion": "echo.edict-provider-generation-settings/v2",
        "shapeSource": "none",
        "canonicalArtifactEncoding": "edict.canonical-cbor/v1",
        "contractPack": "edict.provider-contract-pack.cddl@1",
        "generatorAbi": "wesley.extension-generator/v1"
    }))
    .expect("unsupported settings serialize");
    let unsupported = build_provider_generation_input_v1(SOURCE, &pack, &unsupported_api)
        .expect_err("unsupported settings API fails");
    assert_eq!(
        unsupported.kind(),
        ProviderGenerationErrorKind::UnsupportedSettingsApiVersion
    );

    let wrong_shape = serde_json::to_vec(&json!({
        "apiVersion": "echo.edict-provider-generation-settings/v1",
        "shapeSource": "graphql-sdl",
        "canonicalArtifactEncoding": "edict.canonical-cbor/v1",
        "contractPack": "edict.provider-contract-pack.cddl@1",
        "generatorAbi": "wesley.extension-generator/v1"
    }))
    .expect("wrong settings serialize");
    let mismatch = build_provider_generation_input_v1(SOURCE, &pack, &wrong_shape)
        .expect_err("undeclared GraphQL shape fails");
    assert_eq!(
        mismatch.kind(),
        ProviderGenerationErrorKind::SettingsContractMismatch
    );
}

#[test]
fn graphql_authority_requires_explicit_shape_bytes() {
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    source["authoritySources"]
        .as_array_mut()
        .expect("authority sources are an array")
        .push(json!({
            "coordinate": "echo.provider-graphql@1",
            "kind": "graphql",
            "artifact": "schemas/edict-provider/provider.graphql"
        }));
    let id_type = source["types"]
        .as_array_mut()
        .expect("types are an array")
        .iter_mut()
        .find(|declaration| declaration["identity"]["coordinate"] == "a.b@1.Id")
        .expect("id type exists");
    id_type["identity"]["authority"] = Value::String("echo.provider-graphql@1".to_owned());
    let source = serde_json::to_vec(&source).expect("GraphQL-owned source serializes");
    let pack = admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked Edict provider contract pack is admitted");

    let error = build_provider_generation_input_v1(&source, &pack, SETTINGS)
        .expect_err("GraphQL authority without explicit SDL must fail");
    assert_eq!(
        error.kind(),
        ProviderGenerationErrorKind::GraphqlSourceMissing
    );
    assert_eq!(error.subject(), "echo.provider-graphql@1");
    assert_eq!(error.reference(), "schemas/edict-provider/provider.graphql");
}

#[test]
fn semantic_source_failures_preserve_the_typed_source_kind() {
    let mut source = serde_json::from_slice::<Value>(SOURCE).expect("checked source is JSON");
    source["operations"][0]["implementation"]["capability"] =
        Value::String("echo.dpo@1.missing".to_owned());
    let source = serde_json::to_vec(&source).expect("invalid source serializes");
    let pack = admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked Edict provider contract pack is admitted");

    let error = build_provider_generation_input_v1(&source, &pack, SETTINGS)
        .expect_err("invalid semantic source must fail");
    assert_eq!(
        error.kind(),
        ProviderGenerationErrorKind::SemanticSourceInvalid
    );
    assert_eq!(
        error.semantic_source_kind(),
        Some(ProviderSemanticSourceErrorKind::UnknownCapability)
    );
    assert_eq!(error.subject(), "a.b@1.t");
    assert_eq!(error.reference(), "echo.dpo@1.missing");
    assert_eq!(
        error.to_string(),
        "provider generation semantic-source-invalid: a.b@1.t -> echo.dpo@1.missing"
    );
}
