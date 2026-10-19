// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Native provider judgments over deliberately rebound Core, not compiler proof.
use super::*;
fn boolean(value: bool) -> CanonicalValueV1 {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("bool")),
                ("value", CanonicalValueV1::Bool(value)),
            ]),
        ),
    ])
}
fn utf8_fixture(case: &str) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    called_fixture(|core, _| {
        let parameter = local("arg.0", "Bytes<max=32>");
        let mut scan = call("core.bytes.isUtf8", vec![reference(&parameter)]);
        *map_field_mut(&mut scan, "typeArgs") = list(vec![text("Bytes<max=32>")]);
        match case {
            "missing-type" => *map_field_mut(&mut scan, "typeArgs") = list(vec![]),
            "wrong-type" => *map_field_mut(&mut scan, "typeArgs") = list(vec![text("U64")]),
            "wide-type" => {
                *map_field_mut(&mut scan, "typeArgs") = list(vec![text("Bytes<max=64>")]);
            }
            "wrong-operand" => *map_field_mut(&mut scan, "args") = list(vec![literal("U64", 1)]),
            "extra-operand" => {
                *map_field_mut(&mut scan, "args") =
                    list(vec![reference(&parameter), reference(&parameter)]);
            }
            _ => {}
        }
        let mut predicate = map([
            ("kind", text("compare")),
            ("op", text("==")),
            ("left", scan),
            ("right", boolean(true)),
        ]);
        if case == "ordering" {
            *map_field_mut(&mut predicate, "op") = text("<=");
        }
        if case == "integer-bool" {
            *map_field_mut(&mut predicate, "right") = literal("U64", 1);
        }
        let mut predicates = vec![
            predicate,
            map([
                ("kind", text("not")),
                (
                    "value",
                    map([
                        ("kind", text("compare")),
                        ("op", text("==")),
                        ("left", boolean(false)),
                        ("right", boolean(true)),
                    ]),
                ),
            ]),
        ];
        if case == "empty-all" {
            predicates.clear();
        }
        if case == "malformed-later-child" {
            predicates.push(map([("kind", text("unknown"))]));
        }
        let body = map([
            ("kind", text("if")),
            (
                "predicate",
                map([("kind", text("all")), ("values", list(predicates))]),
            ),
            ("then", boolean(true)),
            ("else", boolean(false)),
        ]);
        let mut definition = function(vec![parameter], "Bool", vec![], vec![], body);
        if case == "wrong-result" {
            *map_field_mut(&mut definition, "returnType") = text("U64");
        }
        add_field(map_field_mut(core, "functions"), "utf8", definition);
    })
}
#[test]
fn source_function_utf8_and_boolean_guards_lower() {
    let (names, fixture, package) = utf8_fixture("valid");
    let lowered =
        lowerer::lower(lowering_request(names, &fixture)).expect("typed UTF8 helper lowers");
    assert_eq!(lowered.outputs[0].artifact.bytes, package);
}
#[test]
fn source_function_utf8_and_boolean_guards_verify_independently() {
    let (names, fixture, package) = utf8_fixture("valid");
    assert!(
        verifier_accepts(names, &fixture, package),
        "verifier refused independently constructed typed UTF8 helper"
    );
}
#[test]
fn source_function_utf8_rejects_signature_boolean_and_closed_shape_forgeries() {
    for case in [
        "missing-type",
        "wrong-type",
        "wide-type",
        "wrong-operand",
        "extra-operand",
        "ordering",
        "integer-bool",
        "empty-all",
        "malformed-later-child",
        "wrong-result",
    ] {
        let (names, fixture, package) = utf8_fixture(case);
        both_refuse(names, &fixture, package, case);
    }
}
