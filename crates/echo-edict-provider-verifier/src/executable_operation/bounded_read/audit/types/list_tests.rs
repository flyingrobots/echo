// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Read type-judgment witnesses; these do not claim public compiler provenance.
use super::*;

#[test]
fn ordered_list_guards_accept_strict_word_comparisons() {
    let core = core();
    let operation = |name: &str, args| {
        m([
            ("kind", t("call")),
            ("callee", t(name)),
            ("typeArgs", Value::Array(vec![t("Numbers")])),
            ("args", Value::Array(args)),
        ])
    };
    let length = operation("core.list.length", vec![call("numbers", vec![])]);
    let access = operation("core.list.at", vec![call("numbers", vec![]), integer(0)]);
    let conditional = |left| {
        m([
            ("kind", t("if")),
            (
                "predicate",
                m([
                    ("kind", t("compare")),
                    ("op", t("<")),
                    ("left", left),
                    ("right", length.clone()),
                ]),
            ),
            ("then", access.clone()),
            ("else", integer(0)),
        ])
    };
    assert!(check(&core, &conditional(integer(0)), "U64").is_ok());
    let wrong_width = m([
        ("kind", t("const")),
        (
            "value",
            m([
                ("kind", t("int")),
                ("width", t("U8")),
                ("value", Value::Integer(0)),
            ]),
        ),
    ]);
    assert_refusal(check(&core, &conditional(wrong_width), "U64"));
}

#[test]
fn ordered_list_operations_preserve_types_after_domain_validation() {
    let core = core();
    let operation = |name: &str, types: &[&str], args| {
        m([
            ("kind", t("call")),
            ("callee", t(name)),
            (
                "typeArgs",
                Value::Array(types.iter().map(|s| t(s)).collect()),
            ),
            ("args", Value::Array(args)),
        ])
    };
    let numbers = || call("numbers", vec![]);
    // This type relation runs after the separate whole-intent domain audit.
    for (value, expected) in [
        (
            operation("core.list.length", &["Numbers"], vec![numbers()]),
            "U64",
        ),
        (
            operation("core.list.at", &["Numbers"], vec![numbers(), integer(0)]),
            "U64",
        ),
        (
            operation(
                "core.list.append",
                &["Numbers"],
                vec![numbers(), integer(7)],
            ),
            "Numbers",
        ),
        (
            operation(
                "core.list.concat",
                &["Numbers", "Numbers"],
                vec![numbers(), numbers()],
            ),
            "List<U64,max=4>",
        ),
    ] {
        assert!(check(&core, &value, expected).is_ok(), "{expected}");
    }
    for (value, expected) in [
        (
            operation(
                "core.list.append",
                &["List<U64,max=3>"],
                vec![numbers(), integer(7)],
            ),
            "List<U64,max=3>",
        ),
        (
            operation(
                "core.list.concat",
                &["Numbers", "Numbers"],
                vec![numbers(), numbers()],
            ),
            "List<U64,max=3>",
        ),
        (
            operation(
                "core.list.at",
                &["List<U8,max=2>"],
                vec![numbers(), integer(0)],
            ),
            "U8",
        ),
        (
            operation(
                "core.list.concat",
                &["List<U64,max=18446744073709551615>", "Numbers"],
                vec![numbers(), numbers()],
            ),
            "Numbers",
        ),
    ] {
        assert_refusal(check(&core, &value, expected));
    }
}

fn t(value: &str) -> Value {
    Value::Text(value.into())
}
fn m(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (t(key), value))
            .collect(),
    )
}
fn integer(value: u64) -> Value {
    m([
        ("kind", t("const")),
        (
            "value",
            m([
                ("kind", t("int")),
                ("width", t("U64")),
                ("value", Value::Integer(value.into())),
            ]),
        ),
    ])
}
fn list(values: Vec<Value>) -> Value {
    m([("kind", t("list")), ("values", Value::Array(values))])
}
fn call(name: &str, args: Vec<Value>) -> Value {
    m([
        ("kind", t("call")),
        ("callee", t(&format!("example@1.{name}"))),
        ("args", Value::Array(args)),
        ("typeArgs", Value::Array(vec![])),
    ])
}
fn signature(result: &str) -> Value {
    m([("params", Value::Array(vec![])), ("returnType", t(result))])
}
fn row(value: Value) -> Value {
    m([("kind", t("record")), ("fields", m([("payload", value)]))])
}
fn core() -> Value {
    m([
        ("coordinate", t("example@1")),
        ("pureFunctions", Value::Array(vec![])),
        (
            "types",
            m([
                (
                    "Choice",
                    m([
                        ("kind", t("Variant")),
                        ("cases", m([("Small", m([])), ("Large", m([]))])),
                    ]),
                ),
                (
                    "Numbers",
                    m([
                        ("kind", t("List")),
                        ("item", t("U64")),
                        ("max", Value::Integer(2)),
                    ]),
                ),
                (
                    "Row",
                    m([
                        ("kind", t("Record")),
                        ("fields", m([("payload", t("Bytes<max=4>"))])),
                    ]),
                ),
                (
                    "A",
                    m([
                        ("kind", t("Nominal")),
                        ("contract", t("A")),
                        ("representation", t("Bytes<max=4>")),
                    ]),
                ),
                (
                    "B",
                    m([
                        ("kind", t("Nominal")),
                        ("contract", t("B")),
                        ("representation", t("Bytes<max=4>")),
                    ]),
                ),
            ]),
        ),
        (
            "functions",
            m([
                ("small", signature("Bytes<exact=1>")),
                ("large", signature("Bytes<exact=3>")),
                ("a", signature("A")),
                ("b", signature("B")),
                ("numbers", signature("Numbers")),
                ("choice", signature("Choice")),
                (
                    "consume",
                    m([
                        (
                            "params",
                            Value::Array(vec![m([
                                ("id", t("arg.0")),
                                ("alphaName", t("values")),
                                ("type", t("Numbers")),
                            ])]),
                        ),
                        ("returnType", t("Bool")),
                    ]),
                ),
            ]),
        ),
    ])
}
#[test]
fn ordered_lists_accept_empty_named_nested_records_and_calls() {
    let core = core();
    for (value, expected) in [
        (list(vec![]), "List<U64,max=0>"),
        (list(vec![integer(1), integer(2)]), "Numbers"),
        (
            list(vec![list(vec![]), list(vec![integer(7)])]),
            "List<Numbers,max=2>",
        ),
        (
            list(vec![row(call("small", vec![])), row(call("large", vec![]))]),
            "List<Row,max=2>",
        ),
        (
            list(vec![call("a", vec![]), call("a", vec![])]),
            "List<A,max=2>",
        ),
        (
            list(vec![call("numbers", vec![])]),
            "List<List<U64,max=2>,max=1>",
        ),
        (call("consume", vec![list(vec![integer(1)])]), "Bool"),
    ] {
        assert!(check(&core, &value, expected).is_ok(), "{expected}");
    }
}
#[test]
fn ordered_lists_reject_bad_elements_bounds_identity_and_shape() {
    let core = core();
    for (value, expected) in [
        (list(vec![integer(1), integer(2)]), "List<U64,max=1>"),
        (list(vec![integer(1)]), "List<Bool,max=1>"),
        (list(vec![]), "List<U64,max=01>"),
        (list(vec![]), "List<U64,max=-1>"),
        (list(vec![]), "List<U64,max=18446744073709551616>"),
        (
            list(vec![call("a", vec![]), call("b", vec![])]),
            "List<A,max=2>",
        ),
        (list(vec![call("a", vec![])]), "List<Bytes<max=4>,max=1>"),
        (list(vec![row(integer(1)), integer(2)]), "List<Row,max=2>"),
        (
            m([
                ("kind", t("list")),
                ("values", Value::Array(vec![])),
                ("extra", Value::Bool(true)),
            ]),
            "Numbers",
        ),
        (
            m([("kind", t("list")), ("values", Value::Integer(0))]),
            "Numbers",
        ),
    ] {
        assert_refusal(check(&core, &value, expected));
    }
}
#[test]
fn ordered_lists_join_exhaustive_match_and_conditional_branches() {
    let core = core();
    let choose = m([
        ("kind", t("match")),
        ("scrutinee", call("choice", vec![])),
        (
            "arms",
            Value::Array(vec![
                m([("case", t("Small")), ("body", list(vec![]))]),
                m([("case", t("Large")), ("body", list(vec![integer(1)]))]),
            ]),
        ),
    ]);
    assert!(check(&core, &choose, "Numbers").is_ok());
    let conditional = m([
        ("kind", t("if")),
        (
            "predicate",
            m([
                ("kind", t("compare")),
                ("op", t("==")),
                ("left", integer(1)),
                ("right", integer(1)),
            ]),
        ),
        ("then", list(vec![])),
        ("else", list(vec![integer(1), integer(2)])),
    ]);
    assert!(check(&core, &conditional, "Numbers").is_ok());
}

#[test]
fn ordered_lists_bound_depth_and_proof_work() {
    let core = core();
    let mut deep = integer(1);
    for _ in 0..64 {
        deep = list(vec![deep]);
    }
    // Audit the expression itself, without any expected-type comparison.
    assert!(symbols(&core).expression(&deep, 0).is_ok());
    assert!(matches!(
        symbols(&core).expression(&list(vec![deep]), 0),
        Err(())
    ));
    let value = list(vec![integer(1)]);
    let mut judgment = symbols(&core);
    judgment.work = 3;
    assert!(judgment.expression(&value, 0).is_ok());
    assert_eq!(judgment.work, 0);
    judgment.work = 2;
    assert!(matches!(judgment.expression(&value, 0), Err(())));
}

fn symbols(core: &Value) -> Symbols<'_> {
    Symbols {
        core,
        exports: core,
        declared: BTreeMap::new(),
        live: BTreeSet::new(),
        claimed: BTreeSet::new(),
        work: 65_536,
    }
}
fn check(core: &Value, value: &Value, expected: &str) -> Check<()> {
    let mut symbols = symbols(core);
    let declared = symbols.schema(expected, 0)?;
    let actual = symbols.expression(value, 0)?;
    if declared.includes(&actual) {
        Ok(())
    } else {
        Err(())
    }
}
fn assert_refusal(result: Check<()>) {
    assert_eq!(result, Err(()));
}

#[test]
fn list_inline_record_types_preserve_structural_and_nominal_meaning() {
    let core = core();
    let mut judgment = symbols(&core);
    assert!(judgment.schema("Row", 0) == judgment.schema("Record<payload:Bytes<max=4>>", 0));
    assert!(judgment
        .schema("List<Record<a:A,nested:List<Record<x:U8>,max=2>>,max=3>", 0)
        .is_ok());
    assert!(judgment.schema("Record<a:A>", 0) != judgment.schema("Record<a:B>", 0));
    assert!(judgment.schema("Record<>", 0).is_ok());
    for invalid in [
        "Record<x:U8,x:U8>",
        "Record<z:U8,a:U8>",
        "Record<x:U8,>",
        "Record<:U8>",
        "Record<0x:U8>",
        "Record<x:>",
        "Record<x:List<U8,max=1>>>",
        "Record<x:Unknown>",
    ] {
        assert!(judgment.schema(invalid, 0).is_err(), "{invalid}");
    }
    judgment.work = 3;
    assert!(judgment.schema("Record<long:U8,z:U8>", 0).is_err());
    let deep = format!("{}U8{}", "Record<x:".repeat(65), ">".repeat(65));
    assert!(symbols(&core).schema(&deep, 0).is_err());
}
