// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! Explicit upstream publication admission is separate from executable support.

use echo_wesley_gen::provider_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_for_publication_v1, admit_provider_contract_pack_v1,
    AdmittedProviderContractPackV1, ProviderContractPackError, ProviderContractPackErrorKind,
    ProviderContractPublicationV1, ProviderContractValidationErrorKind,
};
use sha2::{Digest, Sha256};

const LEGACY_CDDL: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const LEGACY_MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
const ORDERED_CDDL: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/ordered/edict-provider-contracts.cddl");
const ORDERED_MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/ordered/manifest.json");
const SOURCE_CDDL: &[u8] = include_bytes!(
    "../assets/v1/edict-provider/contracts/source-functions-v1/edict-provider-contracts.cddl"
);
const SOURCE_MANIFEST: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/source-functions-v1/manifest.json");

#[test]
fn exact_upstream_source_function_pair_is_the_test_input() {
    assert_eq!(SOURCE_CDDL.len(), 34_460);
    assert_eq!(SOURCE_MANIFEST.len(), 75_989);
    assert_eq!(
        hex::encode(Sha256::digest(SOURCE_CDDL)),
        "e484eabd615584a38cb57454b52747786f2314c135c13f99da6f6bae619a709f"
    );
    assert_eq!(
        hex::encode(Sha256::digest(SOURCE_MANIFEST)),
        "aebc2e4133407ae433c18b55421a89bf15dc5d489368f82ce4874f948d4f9c79"
    );
}

#[test]
fn existing_publications_still_admit_their_exact_bytes() {
    let legacy = admit_provider_contract_pack_v1(LEGACY_CDDL, LEGACY_MANIFEST)
        .expect("default admission retains the original publication");
    assert_eq!(
        legacy.publication(),
        ProviderContractPublicationV1::PureBindings
    );
    assert_eq!(legacy.schema_bytes(), LEGACY_CDDL);
    assert_eq!(legacy.manifest_bytes(), LEGACY_MANIFEST);
    let ordered = admit_provider_contract_pack_for_publication_v1(
        ProviderContractPublicationV1::OrderedInstructions,
        ORDERED_CDDL,
        ORDERED_MANIFEST,
    )
    .expect("explicit ordered admission retains its original publication");
    assert_eq!(ordered.schema_bytes(), ORDERED_CDDL);
    assert_eq!(ordered.manifest_bytes(), ORDERED_MANIFEST);
}

#[test]
fn source_function_bytes_never_silently_upgrade_an_existing_selection() {
    for publication in [
        ProviderContractPublicationV1::PureBindings,
        ProviderContractPublicationV1::OrderedInstructions,
    ] {
        assert_eq!(
            admit_provider_contract_pack_for_publication_v1(
                publication,
                SOURCE_CDDL,
                SOURCE_MANIFEST,
            )
            .expect_err("an existing publication must not discover the new schema")
            .kind(),
            ProviderContractPackErrorKind::ManifestSizeExceeded,
        );
    }
    assert_eq!(
        admit_provider_contract_pack_v1(SOURCE_CDDL, SOURCE_MANIFEST)
            .expect_err("default admission must retain its original publication")
            .kind(),
        ProviderContractPackErrorKind::ManifestSizeExceeded,
    );
}

#[test]
fn source_function_publication_has_an_explicit_admission_route() {
    // Exactly one explicit selector admits these bytes; neither existing
    // publication is upgraded by recognizing a new manifest.
    let publications = [
        ProviderContractPublicationV1::PureBindings,
        ProviderContractPublicationV1::OrderedInstructions,
        ProviderContractPublicationV1::SourceFunctions,
    ];
    let results = publications.map(|publication| {
        admit_provider_contract_pack_for_publication_v1(publication, SOURCE_CDDL, SOURCE_MANIFEST)
    });
    assert_eq!(
        results.iter().filter(|result| result.is_ok()).count(),
        1,
        "one explicit publication must admit the pinned source-function pair: {results:?}",
    );
    let selected = results
        .into_iter()
        .find_map(Result::ok)
        .expect("the exact source-function publication has been selected");
    assert_eq!(
        selected.publication(),
        ProviderContractPublicationV1::SourceFunctions
    );
    assert_eq!(selected.schema_bytes(), SOURCE_CDDL);
    assert_eq!(selected.manifest_bytes(), SOURCE_MANIFEST);
    assert_eq!(selected.contract_count(), 11);
    assert_eq!(selected.domain_count(), 7);
    assert_eq!(selected.resource_count(), 5);
}

fn admit_source_functions(
    schema: &[u8],
    manifest: &[u8],
) -> Result<AdmittedProviderContractPackV1, ProviderContractPackError> {
    admit_provider_contract_pack_for_publication_v1(
        ProviderContractPublicationV1::SourceFunctions,
        schema,
        manifest,
    )
}

#[test]
fn source_selection_rejects_old_publications_and_crossed_pairs() {
    for (schema, manifest) in [
        (LEGACY_CDDL, LEGACY_MANIFEST),
        (ORDERED_CDDL, ORDERED_MANIFEST),
    ] {
        assert_eq!(
            admit_source_functions(schema, manifest)
                .expect_err("explicit new selection cannot accept an old publication")
                .kind(),
            ProviderContractPackErrorKind::SchemaDigestMismatch,
        );
        for (crossed_schema, crossed_manifest) in
            [(SOURCE_CDDL, manifest), (schema, SOURCE_MANIFEST)]
        {
            assert_eq!(
                admit_source_functions(crossed_schema, crossed_manifest)
                    .expect_err("schema and manifest must come from one publication")
                    .kind(),
                ProviderContractPackErrorKind::SchemaBytesMismatch,
            );
        }
    }
}

#[test]
fn source_publication_rejects_tampering_and_self_consistent_republication() {
    let mut schema = SOURCE_CDDL.to_vec();
    schema[0] ^= 1;
    assert_eq!(
        admit_source_functions(&schema, SOURCE_MANIFEST)
            .expect_err("changed CDDL must reject")
            .kind(),
        ProviderContractPackErrorKind::SchemaBytesMismatch,
    );
    let mut manifest: serde_json::Value = serde_json::from_slice(SOURCE_MANIFEST).expect("JSON");
    manifest["schema"]["bytesHex"] = hex::encode(&schema).into();
    manifest["schema"]["rawSha256"] = hex::encode(Sha256::digest(&schema)).into();
    assert_eq!(
        admit_source_functions(&schema, &serde_json::to_vec(&manifest).expect("JSON"))
            .expect_err("internal consistency does not authorize a new publication")
            .kind(),
        ProviderContractPackErrorKind::SchemaDigestMismatch,
    );
}

#[test]
fn source_manifest_requires_exact_bytes_and_checks_size_before_parsing() {
    let manifest: serde_json::Value = serde_json::from_slice(SOURCE_MANIFEST).expect("JSON");
    assert_eq!(
        admit_source_functions(SOURCE_CDDL, &serde_json::to_vec(&manifest).expect("JSON"))
            .expect_err("reformatting changes the pinned manifest")
            .kind(),
        ProviderContractPackErrorKind::ManifestDigestMismatch,
    );
    for oversized in [
        [SOURCE_MANIFEST, b" "].concat(),
        vec![b'{'; SOURCE_MANIFEST.len() + 1],
    ] {
        assert_eq!(
            admit_source_functions(SOURCE_CDDL, &oversized)
                .expect_err("size ceiling applies before JSON parsing")
                .kind(),
            ProviderContractPackErrorKind::ManifestSizeExceeded,
        );
    }
}

#[test]
fn source_manifest_still_checks_api_and_embedded_resources() {
    for (field, expected) in [
        ("api", ProviderContractPackErrorKind::UnsupportedApiVersion),
        (
            "resource",
            ProviderContractPackErrorKind::ResourceRawDigestMismatch,
        ),
    ] {
        let mut manifest: serde_json::Value =
            serde_json::from_slice(SOURCE_MANIFEST).expect("JSON");
        if field == "api" {
            manifest["apiVersion"] = "edict.provider-contract-pack/v99".into();
        } else {
            manifest["resources"][0]["rawSha256"] = "0".repeat(64).into();
        }
        assert_eq!(
            admit_source_functions(SOURCE_CDDL, &serde_json::to_vec(&manifest).expect("JSON"))
                .expect_err("invalid publication")
                .kind(),
            expected,
        );
    }
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

fn parameter() -> Value {
    map([
        ("id", text("arg.0")),
        ("alphaName", text("arg.0")),
        ("type", text("U64")),
    ])
}

fn identity_function(return_type: Option<Value>) -> Value {
    let mut fields = vec![
        ("params", Value::Array(vec![parameter()])),
        (
            "body",
            map([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                (
                    "result",
                    map([("kind", text("local")), ("ref", parameter())]),
                ),
            ]),
        ),
    ];
    if let Some(return_type) = return_type {
        fields.push(("returnType", return_type));
    }
    map(fields)
}

// Structural CDDL specimens only. Public compiler/provider/runtime witnesses
// independently establish source meaning and executable capability support.
fn core_module(functions: Option<Value>) -> Value {
    let mut fields = vec![
        ("apiVersion", text("edict.core/v1")),
        ("coordinate", text("conformance.source-functions@1")),
        ("imports", Value::Array(vec![])),
        ("types", map([])),
        (
            "intents",
            map([(
                "observe",
                map([
                    ("input", text("U64")),
                    ("output", text("U64")),
                    (
                        "requiredOperationProfile",
                        text("continuum.profile.read-only/v1"),
                    ),
                    ("inputConstraints", Value::Array(vec![])),
                    (
                        "coreEvaluationBudget",
                        map([
                            ("maxSteps", Value::Integer(100)),
                            ("maxAllocatedBytes", Value::Integer(1024)),
                            ("maxOutputBytes", Value::Integer(64)),
                        ]),
                    ),
                    (
                        "body",
                        map([
                            ("locals", Value::Array(vec![])),
                            ("nodes", Value::Array(vec![])),
                            (
                                "result",
                                map([
                                    ("kind", text("const")),
                                    (
                                        "value",
                                        map([
                                            ("kind", text("int")),
                                            ("width", text("U64")),
                                            ("value", Value::Integer(7)),
                                        ]),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            )]),
        ),
        ("requiredCoreCapabilities", Value::Array(vec![])),
    ];
    if let Some(functions) = functions {
        fields.push(("functions", functions));
    }
    map(fields)
}

#[test]
fn schema_preserves_function_free_core_and_requires_new_selection_for_functions() {
    let source = admit_source_functions(SOURCE_CDDL, SOURCE_MANIFEST).expect("source schema");
    let legacy = admit_provider_contract_pack_v1(LEGACY_CDDL, LEGACY_MANIFEST).expect("legacy");
    let ordered = admit_provider_contract_pack_for_publication_v1(
        ProviderContractPublicationV1::OrderedInstructions,
        ORDERED_CDDL,
        ORDERED_MANIFEST,
    )
    .expect("ordered");
    let baseline = encode_canonical_cbor_v1(&core_module(None)).expect("canonical control");
    let functions = encode_canonical_cbor_v1(&core_module(Some(map([(
        "identity",
        identity_function(Some(text("U64"))),
    )]))))
    .expect("canonical source-function specimen");
    for pack in [&legacy, &ordered, &source] {
        pack.validate_contract_bytes("core-module", &baseline)
            .expect("function-free Core keeps its old schema shape");
    }
    source
        .validate_contract_bytes("core-module", &functions)
        .expect("new schema admits source-owned function shape");
    for pack in [&legacy, &ordered] {
        assert_eq!(
            pack.validate_contract_bytes("core-module", &functions)
                .expect_err("old schemas must not discover source authority")
                .kind(),
            ProviderContractValidationErrorKind::SchemaMismatch,
        );
    }
}

#[test]
fn schema_refuses_empty_function_tables_and_missing_or_malformed_return_types() {
    let source = admit_source_functions(SOURCE_CDDL, SOURCE_MANIFEST).expect("source schema");
    for table in [
        map([]),
        map([("identity", identity_function(None))]),
        map([("identity", identity_function(Some(Value::Null)))]),
    ] {
        let bytes = encode_canonical_cbor_v1(&core_module(Some(table))).expect("canonical");
        assert_eq!(
            source
                .validate_contract_bytes("core-module", &bytes)
                .expect_err("malformed function table must reject")
                .kind(),
            ProviderContractValidationErrorKind::SchemaMismatch,
        );
    }
}
