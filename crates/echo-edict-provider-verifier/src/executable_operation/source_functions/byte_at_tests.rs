// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Byte lookup and the exact signed truth that dominates its evaluation.
use super::{validate, Value};
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(k, v)| (Value::Text(k.into()), v))
            .collect(),
    )
}
fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn array(v: Vec<Value>) -> Value {
    Value::Array(v)
}
fn local(id: &str, ty: &str) -> Value {
    map([
        ("id", text(id)),
        ("alphaName", text(id)),
        ("type", text(ty)),
    ])
}
fn reference(v: &Value) -> Value {
    map([("kind", text("local")), ("ref", v.clone())])
}
fn integer(width: &str, n: u64) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text(width)),
                ("value", Value::Integer(n.into())),
            ]),
        ),
    ])
}
fn call(name: &str, ty: &str, args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(name)),
        ("typeArgs", array(vec![text(ty)])),
        ("args", array(args)),
    ])
}
fn compare(op: &str, left: Value, right: Value) -> Value {
    map([
        ("kind", text("compare")),
        ("op", text(op)),
        ("left", left),
        ("right", right),
    ])
}
fn branch(p: Value, a: Value, b: Value) -> Value {
    map([
        ("kind", text("if")),
        ("predicate", p),
        ("then", a),
        ("else", b),
    ])
}
fn boolean(kind: &str, values: Vec<Value>) -> Value {
    map([("kind", text(kind)), ("values", array(values))])
}
fn fixture(op: &str, wrong_operand: bool, wrong_arm: bool, lazy: bool) -> (Value, Value) {
    let bytes = local("arg.0", "Bytes<max=8>");
    let index = local("arg.1", "U64");
    let other = local("arg.2", "Bytes<max=8>");
    let length = call(
        "core.bytes.length",
        "Bytes<max=8>",
        vec![reference(if wrong_operand { &other } else { &bytes })],
    );
    let guard = compare(op, reference(&index), length);
    let lookup = call(
        "core.bytes.at",
        "Bytes<max=8>",
        vec![reference(&bytes), reference(&index)],
    );
    let result = if lazy {
        branch(
            boolean("any", vec![guard, compare("<", lookup, integer("U8", 128))]),
            integer("U8", 0),
            integer("U8", 255),
        )
    } else if wrong_arm {
        branch(guard, integer("U8", 0), lookup)
    } else {
        branch(guard, lookup, integer("U8", 0))
    };
    let function = map([
        ("params", array(vec![bytes, index, other])),
        ("returnType", text("U8")),
        (
            "body",
            map([
                ("locals", array(vec![])),
                ("bindings", array(vec![])),
                ("result", result),
            ]),
        ),
    ]);
    (
        map([
            ("coordinate", text("generic.lookup@1")),
            ("types", map([])),
            ("intents", map([])),
            ("functions", map([("lookup", function)])),
        ]),
        map([("effects", array(vec![])), ("pureFunctions", array(vec![]))]),
    )
}
#[test]
fn byte_lookup_accepts_strict_own_arm_and_lazy_false_prefix() {
    for (op, arm, lazy) in [
        ("<", false, false),
        (">=", true, false),
        (">=", false, true),
    ] {
        let (core, exports) = fixture(op, false, arm, lazy);
        assert!(
            validate(&core, &exports, false).is_ok(),
            "{op} {arm} {lazy}"
        );
    }
}
#[test]
fn byte_lookup_rejects_equal_length_unrelated_and_non_dominating_guards() {
    for (op, wrong, arm, lazy) in [
        ("<=", false, false, false),
        ("<", true, false, false),
        ("<", false, true, false),
        ("<", false, false, true),
    ] {
        let (core, exports) = fixture(op, wrong, arm, lazy);
        assert!(
            validate(&core, &exports, false).is_err(),
            "{op} {wrong} {arm} {lazy}"
        );
    }
}

#[test]
fn byte_lookup_root_without_function_table_requires_a_real_presence_bound() {
    let exports = map([("effects", array(vec![])), ("pureFunctions", array(vec![]))]);
    for ty in ["Bytes<max=8>", "Bytes<exact=1>"] {
        let input = local("arg.0", ty);
        let output = local("local.0", "U8");
        let intent = map([
            ("input", text(ty)),
            ("output", text("U8")),
            ("inputConstraints", array(vec![])),
            (
                "coreEvaluationBudget",
                map([
                    ("maxSteps", Value::Integer(10000)),
                    ("maxAllocatedBytes", Value::Integer(10000)),
                    ("maxOutputBytes", Value::Integer(10000)),
                ]),
            ),
            (
                "body",
                map([
                    ("locals", array(vec![input.clone(), output.clone()])),
                    (
                        "nodes",
                        array(vec![map([
                            ("kind", text("let")),
                            ("binding", output.clone()),
                            (
                                "value",
                                call(
                                    "core.bytes.at",
                                    ty,
                                    vec![reference(&input), integer("U64", 0)],
                                ),
                            ),
                        ])]),
                    ),
                    ("result", reference(&output)),
                ]),
            ),
        ]);
        let core = map([
            ("coordinate", text("generic.lookup@1")),
            ("types", map([])),
            ("intents", map([("read", intent)])),
        ]);
        assert_eq!(
            validate(&core, &exports, false).is_ok(),
            ty == "Bytes<exact=1>",
            "{ty}"
        );
    }
}
