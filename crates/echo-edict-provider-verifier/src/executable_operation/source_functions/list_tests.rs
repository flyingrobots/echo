// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::{validate, Value};
fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn map(v: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(v.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn integer(n: u64) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", Value::Integer(n.into())),
            ]),
        ),
    ])
}
fn list(values: Vec<Value>) -> Value {
    map([("kind", text("list")), ("values", Value::Array(values))])
}
fn fixture(value: Value, result: &str) -> (Value, Value) {
    let body = map([
        ("locals", Value::Array(vec![])),
        ("bindings", Value::Array(vec![])),
        ("result", value),
    ]);
    let function = map([
        ("params", Value::Array(vec![])),
        ("returnType", text(result)),
        ("body", body),
    ]);
    (
        map([
            ("coordinate", text("generic.lists@1")),
            ("types", map([])),
            ("intents", map([])),
            ("functions", map([("literal", function)])),
        ]),
        map([
            ("effects", Value::Array(vec![])),
            ("pureFunctions", Value::Array(vec![])),
        ]),
    )
}
#[test]
fn bounded_lists_accept_empty_nonempty_and_nested_values() {
    for (value, result) in [
        (list(vec![]), "List<U64,max=0>"),
        (list(vec![integer(1), integer(2)]), "List<U64,max=2>"),
        (
            list(vec![list(vec![]), list(vec![integer(7)])]),
            "List<List<U64,max=2>,max=2>",
        ),
    ] {
        let (core, exports) = fixture(value, result);
        assert!(validate(&core, &exports, false).is_ok(), "{result}");
    }
}
#[test]
fn bounded_lists_refuse_overflow_wrong_elements_and_noncanonical_bounds() {
    for (value, result) in [
        (list(vec![integer(1), integer(2)]), "List<U64,max=1>"),
        (list(vec![integer(1)]), "List<Bool,max=1>"),
        (list(vec![]), "List<U64,max=01>"),
        (list(vec![integer(1)]), "List<U64,max=18446744073709551615>"),
    ] {
        let (core, exports) = fixture(value, result);
        assert!(validate(&core, &exports, false).is_err(), "{result}");
    }
}

#[test]
fn bounded_lists_without_helpers_enforce_whole_intent_budget() {
    let parameter = map([
        ("id", text("arg.0")),
        ("type", text("Bool")),
        ("alphaName", text("input")),
    ]);
    let exports = map([
        ("effects", Value::Array(vec![])),
        ("pureFunctions", Value::Array(vec![])),
    ]);
    for (steps, allocation, output, accepted) in [
        (100, 1024, 100, true),
        (0, 1024, 100, false),
        (100, 0, 100, false),
        (100, 1024, 0, false),
    ] {
        let intent = map([
            ("input", text("Bool")),
            ("output", text("List<U64,max=2>")),
            ("inputConstraints", Value::Array(vec![])),
            (
                "body",
                map([
                    ("locals", Value::Array(vec![parameter.clone()])),
                    ("nodes", Value::Array(vec![])),
                    ("result", list(vec![integer(7), integer(8)])),
                ]),
            ),
            (
                "coreEvaluationBudget",
                map([
                    ("maxSteps", Value::Integer(steps)),
                    ("maxAllocatedBytes", Value::Integer(allocation)),
                    ("maxOutputBytes", Value::Integer(output)),
                ]),
            ),
        ]);
        let core = map([
            ("coordinate", text("generic.lists@1")),
            ("types", map([])),
            ("intents", map([("make", intent)])),
        ]);
        assert_eq!(
            validate(&core, &exports, false).is_ok(),
            accepted,
            "{steps}/{allocation}/{output}"
        );
    }
}
