// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::{validate, Value};

#[test]
fn bounded_lists_join_record_elements_and_conditional_results() -> Result<(), String> {
    let record = |bytes: Vec<u8>| {
        map([
            ("kind", text("record")),
            (
                "fields",
                map([(
                    "payload",
                    map([
                        ("kind", text("const")),
                        (
                            "value",
                            map([("kind", text("bytes")), ("value", Value::Bytes(bytes))]),
                        ),
                    ]),
                )]),
            ),
        ])
    };
    let alternatives = [
        list(vec![record(vec![]), record(vec![0, 255, 3])]),
        map([
            ("kind", text("if")),
            (
                "predicate",
                map([
                    ("kind", text("compare")),
                    ("op", text("==")),
                    ("left", integer(1)),
                    ("right", integer(1)),
                ]),
            ),
            ("then", list(vec![record(vec![1])])),
            (
                "else",
                list(vec![record(vec![2, 3]), record(vec![4, 5, 6])]),
            ),
        ]),
    ];
    for value in alternatives {
        let (mut core, exports) = fixture(value, "List<Row,max=2>");
        let Value::Map(fields) = &mut core else {
            return Err("fixture Core is not a map".into());
        };
        fields
            .iter_mut()
            .find(|(key, _)| key == &text("types"))
            .ok_or("fixture types missing")?
            .1 = map([(
            "Row",
            map([
                ("kind", text("Record")),
                ("fields", map([("payload", text("Bytes<max=4>"))])),
            ]),
        )]);
        validate(&core, &exports, false)
            .map_err(|refusal| format!("record list refused: {refusal:?}"))?;
    }
    Ok(())
}

#[test]
fn bounded_lists_refuse_malformed_literals() {
    for value in [
        map([("kind", text("list"))]),
        map([("kind", text("list")), ("values", Value::Integer(0))]),
        map([
            ("kind", text("list")),
            ("values", Value::Array(vec![])),
            ("unexpected", Value::Bool(true)),
        ]),
    ] {
        let (core, exports) = fixture(value, "List<U64,max=65537>");
        assert!(validate(&core, &exports, false).is_err());
    }
}

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
        // Bool input copy/validation: 2 steps, 64 bytes; list evaluation:
        // 1 container + 2*(1 expression + 1 copy) steps, 192 bytes;
        // output copy/validation: 6 steps, 192 bytes; encoded ceiling: 9+2*9.
        (13, 448, 27, true),
        (12, 448, 27, false),
        (13, 447, 27, false),
        (13, 448, 26, false),
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

#[test]
fn bounded_lists_isolate_expression_depth_and_work_limits() {
    let mut value = integer(1);
    for _ in 0..64 {
        value = list(vec![value]);
    }
    assert!(infer_expression(&value, 65_536).is_ok());
    assert_eq!(infer_expression(&list(vec![value]), 65_536), Err(()));
    let value = list(vec![integer(1)]);
    assert_eq!(infer_expression(&value, 3), Ok(()));
    assert_eq!(infer_expression(&value, 2), Err(()));
}

#[test]
fn bounded_lists_bound_helper_free_discovery() {
    // One root + each array child consumes one discovery unit, independently
    // of type resolution, expression checking, or execution budgets.
    let at_limit = Value::Array(vec![Value::Bool(false); 65_535]);
    assert_eq!(super::unsigned_addition::present(&at_limit), Ok(false));
    let over_limit = Value::Array(vec![Value::Bool(false); 65_536]);
    assert_eq!(super::unsigned_addition::present(&over_limit), Err(()));
    // Types alone must activate whole-intent checks even without helpers.
    for definition in [
        text("List<U64,max=1>"),
        map([
            ("kind", text("List")),
            ("item", text("U64")),
            ("max", Value::Integer(1)),
        ]),
    ] {
        let core = map([
            ("types", map([("Numbers", definition)])),
            ("intents", map([])),
        ]);
        assert_eq!(super::unsigned_addition::present(&core), Ok(true));
    }
}

fn infer_expression(value: &Value, work: usize) -> super::Check<()> {
    let core = map([("types", map([]))]);
    let mut types = super::Types::new(&core);
    types.proof_work(65_536 - work)?;
    let definitions = super::BTreeMap::new();
    let summaries = super::BTreeMap::new();
    let mut checker = super::Checker {
        types: &mut types,
        definitions: &definitions,
        summaries: &summaries,
        proven: vec![],
        path: vec![],
        bound: vec![],
        proven_sum: None,
    };
    let frame = super::Frame {
        declared: super::BTreeMap::new(),
        values: super::BTreeMap::new(),
    };
    checker.expression(value, &frame, 0).map(|_| ())
}
