// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Preparation witnesses for capability scans and combined validation depth.
use super::{
    functions,
    model::{Expr, Helper, MAX_PROGRAM_NODES},
    syntax::Parser,
    EvaluationError as Error,
};
use echo_edict_canonical::{
    decode_canonical_cbor_v1, encode_canonical_cbor_v1, CanonicalValueV1 as Value,
};
use std::collections::BTreeMap;

const COORDINATE: &str = "generic.worklist@1";
fn t(s: &str) -> Value {
    Value::Text(s.into())
}
fn m(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (t(key), value))
            .collect(),
    )
}
fn local(ty: &str) -> Value {
    m([("id", t("arg.0")), ("alphaName", t("xs")), ("type", t(ty))])
}
fn reference(ty: &str) -> Value {
    m([("kind", t("local")), ("ref", local(ty))])
}
fn literal() -> Value {
    m([
        ("kind", t("const")),
        (
            "value",
            m([
                ("kind", t("int")),
                ("width", t("U64")),
                ("value", Value::Integer(0)),
            ]),
        ),
    ])
}
fn empty_bytes() -> Value {
    m([
        ("kind", t("const")),
        (
            "value",
            m([("kind", t("bytes")), ("value", Value::Bytes(vec![]))]),
        ),
    ])
}
fn call(name: &str, types: &[&str], args: Vec<Value>) -> Value {
    m([
        ("kind", t("call")),
        ("callee", t(name)),
        (
            "typeArgs",
            Value::Array(types.iter().map(|ty| t(ty)).collect()),
        ),
        ("args", Value::Array(args)),
    ])
}
fn function(params: Vec<Value>, ty: &str, result: Value) -> Value {
    m([
        ("params", Value::Array(params)),
        ("returnType", t(ty)),
        (
            "body",
            m([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                ("result", result),
            ]),
        ),
    ])
}
fn wrapped(mut value: Value, count: usize) -> Value {
    for _ in 0..count {
        value = m([
            ("kind", t("if")),
            (
                "predicate",
                m([
                    ("kind", t("compare")),
                    ("op", t("==")),
                    ("left", literal()),
                    ("right", literal()),
                ]),
            ),
            ("then", value),
            ("else", literal()),
        ]);
    }
    value
}
fn decode(
    definitions: Vec<(&'static str, Value)>,
    capability: bool,
    imported: Vec<Value>,
) -> Result<BTreeMap<String, Helper>, Error> {
    let mut core = vec![
        ("coordinate", t(COORDINATE)),
        ("types", m([])),
        ("intents", m([])),
        ("functions", m(definitions)),
    ];
    if capability {
        core.push((
            "requiredCoreCapabilities",
            Value::Array(vec![t("core.list/v1")]),
        ));
    }
    // Exercise the real canonical artifact bound before runtime preparation.
    let encoded = encode_canonical_cbor_v1(&m(core));
    assert!(encoded.is_ok(), "canonical fixture encoding: {encoded:?}");
    let decoded = decode_canonical_cbor_v1(&encoded.map_err(|_| Error::UnsupportedProgram)?);
    assert!(decoded.is_ok(), "canonical fixture admission: {decoded:?}");
    let core = decoded.map_err(|_| Error::UnsupportedProgram)?;
    let exports = m([
        ("pureFunctions", Value::Array(imported)),
        ("effects", Value::Array(vec![])),
    ]);
    let types = m([]);
    let mut parser = Parser {
        types: &types,
        coordinate: COORDINATE,
        remaining: MAX_PROGRAM_NODES,
    };
    functions::decode(&core, &exports, &mut parser)
}

#[test]
fn list_capability_scan_preserves_supported_non_list_depth() -> Result<(), Error> {
    for capability in [true, false] {
        let mut result = empty_bytes();
        for _ in 0..40 {
            result = call(
                "core.bytes.concat",
                &["Bytes<exact=0>", "Bytes<exact=0>"],
                vec![result, empty_bytes()],
            );
        }
        let helpers = decode(
            vec![("deep", function(vec![], "Bytes<exact=0>", result))],
            capability,
            vec![],
        )?;
        functions::check_roots(&helpers, [], [])?;
    }
    Ok(())
}

#[test]
fn list_capability_scan_preserves_supported_non_list_width() -> Result<(), Error> {
    // Wire maps/keys outnumber expression nodes. Declaring an unused list
    // capability must not change whether a supported record helper prepares.
    for capability in [true, false] {
        let fields = (0..2800)
            .map(|i| (t(&format!("f{i:04}")), t("U64")))
            .collect();
        let values = (0..2800)
            .map(|i| (t(&format!("f{i:04}")), literal()))
            .collect();
        let types = m([(
            "Wide",
            m([("kind", t("Record")), ("fields", Value::Map(fields))]),
        )]);
        let result = m([("kind", t("record")), ("fields", Value::Map(values))]);
        let mut members = vec![
            ("coordinate", t(COORDINATE)),
            ("types", types),
            ("intents", m([])),
            ("functions", m([("wide", function(vec![], "Wide", result))])),
        ];
        if capability {
            members.push((
                "requiredCoreCapabilities",
                Value::Array(vec![t("core.list/v1")]),
            ));
        }
        let encoded =
            encode_canonical_cbor_v1(&m(members)).map_err(|_| Error::UnsupportedProgram)?;
        assert!(encoded.len() < 1_048_576, "bounded canonical fixture");
        let core = decode_canonical_cbor_v1(&encoded).map_err(|_| Error::UnsupportedProgram)?;
        let types = super::values::field(&core, "types")?;
        let imported = (0..1200)
            .map(|i| {
                m([
                    ("coordinate", t(&format!("library@1.helper{i}"))),
                    ("source", t("edict")),
                    ("parameterTypes", Value::Array(vec![])),
                    ("typeParameters", Value::Array(vec![])),
                    ("returnType", t("U64")),
                    ("body", function(vec![], "U64", literal())),
                ])
            })
            .collect();
        // Core and the lawpack exports are separately decoded artifacts. Each
        // is within the canonical node ceiling; parser work spans both.
        let exports = m([
            ("pureFunctions", Value::Array(imported)),
            ("effects", Value::Array(vec![])),
        ]);
        let exports = decode_canonical_cbor_v1(
            &encode_canonical_cbor_v1(&exports).map_err(|_| Error::UnsupportedProgram)?,
        )
        .map_err(|_| Error::UnsupportedProgram)?;
        let mut parser = Parser {
            types,
            coordinate: COORDINATE,
            remaining: MAX_PROGRAM_NODES,
        };
        let prepared = functions::decode(&core, &exports, &mut parser);
        assert!(
            prepared.is_ok(),
            "unused capability {capability}: {:?}",
            prepared.err()
        );
    }
    Ok(())
}

#[test]
fn list_capability_scan_refuses_unused_source_and_imported_operations() {
    let length = || {
        call(
            "core.list.length",
            &["List<U8,max=1>"],
            vec![m([("kind", t("list")), ("values", Value::Array(vec![]))])],
        )
    };
    for imported in [false, true] {
        let (source, exports) = if imported {
            (
                vec![],
                vec![m([
                    ("coordinate", t("library@1.unused")),
                    ("source", t("edict")),
                    ("parameterTypes", Value::Array(vec![])),
                    ("typeParameters", Value::Array(vec![])),
                    ("returnType", t("U64")),
                    ("body", function(vec![], "U64", length())),
                ])],
            )
        } else {
            (vec![("unused", function(vec![], "U64", length()))], vec![])
        };
        assert!(
            decode(source.clone(), true, exports.clone()).is_ok(),
            "capability control"
        );
        assert!(matches!(
            decode(source, false, exports),
            Err(Error::UnsupportedProgram)
        ));
    }
}

#[test]
fn list_operand_validation_depth_is_proved_across_calls() -> Result<(), Error> {
    let deep_type = format!("{}U8{}", "List<".repeat(40), ",max=1>".repeat(40));
    for caller_wrappers in [0, 20] {
        let length = call(
            "core.list.length",
            &[&deep_type],
            vec![reference(&deep_type)],
        );
        let prepared = decode(
            vec![
                (
                    "inner",
                    function(vec![local(&deep_type)], "U64", wrapped(length, 10)),
                ),
                (
                    "outer",
                    function(
                        vec![local(&deep_type)],
                        "U64",
                        wrapped(
                            call("generic.worklist@1.inner", &[], vec![reference(&deep_type)]),
                            caller_wrappers,
                        ),
                    ),
                ),
            ],
            true,
            vec![],
        );
        if caller_wrappers == 20 && matches!(&prepared, Err(Error::UnsupportedProgram)) {
            // decode can discharge the helper aperture before root expressions
            // are checked. Both boundaries precede instruction execution.
            continue;
        }
        let helpers = prepared?;
        let root = Expr::Call(
            "generic.worklist@1.outer".into(),
            vec![Expr::Local("input".into())],
        );
        let checked = functions::check_roots(&helpers, [&root], []);
        let mut input = Value::Integer(7);
        for _ in 0..40 {
            input = Value::Array(vec![input]);
        }
        let locals = BTreeMap::from([("input", input)]);
        let mut meter = super::evaluate::Meter::new(super::EvaluationLimits {
            max_steps: 1_000_000,
            max_allocated_bytes: 16 * 1024 * 1024,
            max_output_bytes: 1024,
            max_package_bytes: 1_048_576,
            max_input_bytes: 1_048_576,
        });
        let runtime = super::evaluate::expression(&root, &locals, &helpers, &mut meter, 0);
        if caller_wrappers == 0 {
            checked?;
            assert_eq!(runtime, Ok(Value::Integer(1)));
        } else {
            assert_eq!(
                runtime,
                Err(Error::UnsupportedProgram),
                "runtime depth control"
            );
            assert_eq!(
                checked,
                Err(Error::UnsupportedProgram),
                "preparation must refuse before execution"
            );
        }
    }
    Ok(())
}
