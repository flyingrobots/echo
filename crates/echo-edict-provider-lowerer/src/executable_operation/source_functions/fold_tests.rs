// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic fold judgments; public compiler provenance is a separate witness.
use echo_edict_canonical::CanonicalValueV1 as Value;
fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(fields.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn boolean(b: bool) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("bool")), ("value", Value::Bool(b))]),
        ),
    ])
}
fn function(params: Vec<Value>, result: Value) -> Value {
    map([
        ("params", Value::Array(params)),
        ("returnType", text("Bool")),
        (
            "body",
            map([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                ("result", result),
            ]),
        ),
    ])
}
fn fixture(bound: Option<u64>) -> (Value, Value) {
    let parameter = map([
        ("id", text("param.0")),
        ("type", text("Bool")),
        ("alphaName", text("state")),
    ]);
    let value = map([("kind", text("local")), ("ref", parameter.clone())]);
    let toggle = map([
        ("kind", text("if")),
        (
            "predicate",
            map([
                ("kind", text("compare")),
                ("op", text("==")),
                ("left", value),
                ("right", boolean(true)),
            ]),
        ),
        ("then", boolean(false)),
        ("else", boolean(true)),
    ]);
    let result = bound.map_or_else(
        || boolean(true),
        |bound| {
            map([
                ("kind", text("fold")),
                ("initial", boolean(true)),
                ("step", text("iteration.example@1.toggle")),
                ("bound", Value::Integer(bound.into())),
                ("stateType", text("Bool")),
            ])
        },
    );
    (
        map([
            ("coordinate", text("iteration.example@1")),
            ("types", map([])),
            ("intents", map([])),
            (
                "requiredCoreCapabilities",
                Value::Array(vec![text("core.fold/v1")]),
            ),
            (
                "functions",
                map([
                    ("toggle", function(vec![parameter], toggle)),
                    ("make", function(vec![], result)),
                ]),
            ),
        ]),
        map([
            ("effects", Value::Array(vec![])),
            ("pureFunctions", Value::Array(vec![])),
        ]),
    )
}

#[test]
fn source_fold_accepts_invariant_state_for_zero_and_nonzero_counts() {
    let (core, exports) = fixture(None);
    assert!(
        super::validate(&core, &exports, false).is_ok(),
        "nonfold control"
    );
    for bound in [0, 1, 2, 3] {
        let (core, exports) = fixture(Some(bound));
        assert!(
            super::validate(&core, &exports, false).is_ok(),
            "fold bound {bound}"
        );
    }
}

fn get_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Value {
    let Value::Map(fields) = value else {
        unreachable!("fixture is a map")
    };
    let Some((_, value)) = fields.iter_mut().find(|(k, _)| k == &text(key)) else {
        unreachable!("fixture key exists")
    };
    value
}
fn folded(core: &mut Value) -> &mut Value {
    get_mut(
        get_mut(get_mut(get_mut(core, "functions"), "make"), "body"),
        "result",
    )
}
fn malformed_folds() -> Vec<(&'static str, Value, Value)> {
    let mut cases = Vec::new();
    for (name, key, value) in [
        ("missing step", "step", text("iteration.example@1.missing")),
        ("opaque intrinsic", "step", text("core.bytes.length")),
        ("negative bound", "bound", Value::Integer(-1)),
        ("noninteger bound", "bound", Value::Bool(false)),
        ("wrong invariant type", "stateType", text("U64")),
        (
            "wrong initial type",
            "initial",
            map([
                ("kind", text("const")),
                (
                    "value",
                    map([
                        ("kind", text("int")),
                        ("width", text("U64")),
                        ("value", Value::Integer(0)),
                    ]),
                ),
            ]),
        ),
        (
            "initial cannot see step parameter",
            "initial",
            map([
                ("kind", text("local")),
                (
                    "ref",
                    map([
                        ("id", text("param.0")),
                        ("type", text("Bool")),
                        ("alphaName", text("state")),
                    ]),
                ),
            ]),
        ),
    ] {
        let (mut core, exports) = fixture(Some(0));
        *get_mut(folded(&mut core), key) = value;
        cases.push((name, core, exports));
    }
    let (mut core, exports) = fixture(Some(0));
    *get_mut(&mut core, "requiredCoreCapabilities") = Value::Array(vec![]);
    cases.push(("missing fold capability", core, exports));
    let (mut core, exports) = fixture(Some(0));
    let cycle = folded(&mut core).clone();
    *get_mut(
        get_mut(get_mut(get_mut(&mut core, "functions"), "toggle"), "body"),
        "result",
    ) = cycle;
    cases.push(("cycle at bound zero", core, exports));
    let (mut core, exports) = fixture(Some(0));
    *get_mut(&mut core, "types") = map([("Flag", map([("kind", text("Bool"))]))]);
    *get_mut(
        get_mut(get_mut(&mut core, "functions"), "toggle"),
        "returnType",
    ) = text("Flag");
    cases.push(("return alias is not the declared invariant", core, exports));
    cases
}

#[test]
fn source_fold_refuses_invalid_authority_scope_type_and_zero_count_cycles() {
    for (name, core, exports) in malformed_folds() {
        assert!(super::validate(&core, &exports, false).is_err(), "{name}");
    }
}
#[test]
fn source_fold_refuses_unrepresentable_execution_charges() {
    let (core, exports) = fixture(Some(u64::MAX));
    assert!(super::validate(&core, &exports, false).is_err());
}

fn nested_fixture(outer: u64, inner: u64) -> (Value, Value) {
    let (mut core, exports) = fixture(Some(outer));
    let toggle = get_mut(get_mut(&mut core, "functions"), "toggle").clone();
    let mut inner_fold = folded(&mut core).clone();
    *get_mut(&mut inner_fold, "step") = text("iteration.example@1.inner");
    *get_mut(&mut inner_fold, "bound") = Value::Integer(inner.into());
    *get_mut(&mut inner_fold, "initial") = map([
        ("kind", text("local")),
        (
            "ref",
            map([
                ("id", text("param.0")),
                ("type", text("Bool")),
                ("alphaName", text("state")),
            ]),
        ),
    ]);
    *get_mut(
        get_mut(get_mut(get_mut(&mut core, "functions"), "toggle"), "body"),
        "result",
    ) = inner_fold;
    let Value::Map(functions) = get_mut(&mut core, "functions") else {
        unreachable!()
    };
    functions.push((text("inner"), toggle));
    (core, exports)
}

#[test]
fn source_fold_multiplies_nested_costs_without_unrolling_the_bound() {
    let (core, exports) = nested_fixture(3, 2);
    assert!(super::validate(&core, &exports, false).is_ok());
    let (core, exports) = nested_fixture(1 << 32, 1 << 32);
    assert!(
        super::validate(&core, &exports, false).is_err(),
        "nested cost exceeds u64 even though each bound fits"
    );
}
