// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Read type-judgment witnesses; these do not claim public compiler provenance.
use super::*;
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
    ] {
        assert_refusal(check(&core, &value, expected));
    }
    let malformed = m([("kind", t("list")), ("values", Value::Integer(0))]);
    let result = check(&core, &malformed, "Numbers");
    assert!(
        matches!(&result, Err(refusal)
        if refusal.kind == crate::ProviderRefusalKind::UnsupportedSemantics
        && refusal.subject.as_deref() == Some(SUBJECT)),
        "{result:?}"
    );
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
    // Direct inference avoids a shallower expected type or type-parser refusal.
    assert!(scope(&core).expression(&deep, 0).is_ok());
    assert_eq!(
        scope(&core).expression(&list(vec![deep]), 0),
        Err(invalid())
    );
    let value = list(vec![integer(1)]);
    let judgment = scope(&core);
    judgment.remaining_type_work.set(3);
    assert!(judgment.expression(&value, 0).is_ok());
    assert_eq!(judgment.remaining_type_work.get(), 0);
    judgment.remaining_type_work.set(2);
    assert_eq!(judgment.expression(&value, 0), Err(invalid()));
}

fn scope(core: &Value) -> Scope<'_> {
    Scope {
        core,
        exports: core,
        inventory: BTreeMap::new(),
        available: BTreeMap::new(),
        used: BTreeSet::new(),
        input: core,
        remaining_type_work: Cell::new(65_536),
    }
}
fn check(core: &Value, value: &Value, expected: &str) -> Result<(), ProviderRefusalV1> {
    scope(core).matches_output(value, expected)
}
fn assert_refusal(result: Result<(), ProviderRefusalV1>) {
    assert!(
        matches!(&result, Err(refusal)
        if refusal.kind == invalid().kind
        && refusal.subject.as_deref() == Some(SUBJECT)),
        "{result:?}"
    );
}

#[test]
fn list_inline_record_types_preserve_structural_and_nominal_meaning() {
    let core = core();
    let judgment = scope(&core);
    assert_eq!(
        judgment.ty("Row"),
        judgment.ty("Record<payload:Bytes<max=4>>")
    );
    assert!(judgment
        .ty("List<Record<a:A,nested:List<Record<x:U8>,max=2>>,max=3>")
        .is_ok());
    assert_ne!(judgment.ty("Record<a:A>"), judgment.ty("Record<a:B>"));
    assert!(judgment.ty("Record<>").is_ok());
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
        assert!(judgment.ty(invalid).is_err(), "{invalid}");
    }
    judgment.remaining_type_work.set(3);
    assert!(judgment.ty("Record<long:U8,z:U8>").is_err());
    let deep = format!("{}U8{}", "Record<x:".repeat(65), ">".repeat(65));
    assert!(scope(&core).ty(&deep).is_err());
}
