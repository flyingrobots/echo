// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)]
//! Current publication structural conformance; not runtime execution evidence.
use echo_wesley_gen::provider_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, ProviderContractValidationErrorKind,
};
const SCHEMA: &[u8] =
    include_bytes!("../assets/v1/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const MANIFEST: &[u8] = include_bytes!("../assets/v1/edict-provider/contracts/v1/manifest.json");
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
fn current_schema_accepts_function_free_and_source_function_core() {
    let pack = admit_provider_contract_pack_v1(SCHEMA, MANIFEST).expect("current schema");
    for value in [
        core_module(None),
        core_module(Some(map([(
            "identity",
            identity_function(Some(text("U64"))),
        )]))),
    ] {
        pack.validate_contract_bytes(
            "core-module",
            &encode_canonical_cbor_v1(&value).expect("canonical Core"),
        )
        .expect("current Core shape");
    }
}

#[test]
fn schema_refuses_empty_function_tables_and_missing_or_malformed_return_types() {
    let source = admit_provider_contract_pack_v1(SCHEMA, MANIFEST).expect("source schema");
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
