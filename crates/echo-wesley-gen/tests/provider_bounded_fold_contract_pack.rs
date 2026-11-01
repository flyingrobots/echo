// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Current publication structural conformance; not runtime execution evidence.
use echo_wesley_gen::provider_canonical::{encode_canonical_cbor_v1, CanonicalValueV1 as Value};
use echo_wesley_gen::provider_contract_pack::admit_provider_contract_pack_v1;
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

fn replace(value: &mut Value, key: &str, next: Value) {
    let Value::Map(entries) = value else {
        panic!("map")
    };
    let (_, target) = entries
        .iter_mut()
        .find(|(k, _)| k == &text(key))
        .expect("field");
    *target = next;
}

#[test]
fn current_fold_shape_requires_closed_fields_and_unsigned_bound() {
    use echo_wesley_gen::provider_contract_pack::ProviderContractValidationErrorKind as Kind;
    let new = admit_provider_contract_pack_v1(SCHEMA, MANIFEST).expect("current publication");
    let function = identity_function(Some(text("U64")));
    let baseline = core_module(Some(map([("identity", function.clone())])));
    let baseline_bytes = encode_canonical_cbor_v1(&baseline).expect("baseline bytes");
    new.validate_contract_bytes("core-module", &baseline_bytes)
        .expect("non-fold control");
    // Structural specimens deliberately do not establish typing or execution.
    let initial = map([("kind", text("local")), ("ref", parameter())]);
    let fold = map([
        ("kind", text("fold")),
        ("initial", initial),
        ("step", text("conformance.source-functions@1.identity")),
        ("bound", Value::Integer(3)),
        ("stateType", text("U64")),
    ]);
    let specimen = |expression| {
        let mut run = function.clone();
        replace(
            &mut run,
            "body",
            map([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                ("result", expression),
            ]),
        );
        core_module(Some(map([("identity", function.clone()), ("run", run)])))
    };
    let bytes = encode_canonical_cbor_v1(&specimen(fold.clone())).expect("fold bytes");
    new.validate_contract_bytes("core-module", &bytes)
        .expect("new schema admits fold shape");
    for key in ["initial", "step", "bound", "stateType"] {
        let mut invalid = fold.clone();
        let Value::Map(ref mut entries) = invalid else {
            panic!("map")
        };
        entries.retain(|(k, _)| k != &text(key));
        let bytes = encode_canonical_cbor_v1(&specimen(invalid)).expect("invalid bytes");
        assert_eq!(
            new.validate_contract_bytes("core-module", &bytes)
                .expect_err("missing field")
                .kind(),
            Kind::SchemaMismatch
        );
    }
    let mut extra = fold.clone();
    let Value::Map(ref mut fields) = extra else {
        panic!("map")
    };
    fields.push((text("ignored"), Value::Bool(true)));
    let bytes = encode_canonical_cbor_v1(&specimen(extra)).expect("extra field bytes");
    assert_eq!(
        new.validate_contract_bytes("core-module", &bytes)
            .expect_err("unknown field")
            .kind(),
        Kind::SchemaMismatch
    );
    let mut invalid = fold;
    replace(&mut invalid, "bound", Value::Integer(-1));
    let bytes = encode_canonical_cbor_v1(&specimen(invalid)).expect("negative bound bytes");
    assert_eq!(
        new.validate_contract_bytes("core-module", &bytes)
            .expect_err("negative bound")
            .kind(),
        Kind::SchemaMismatch
    );
}
