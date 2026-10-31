// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The same adversarial input corpus exercises each independently owned judgment.
use super::{validate, Value};

fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn map(v: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(v.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn array(v: Vec<Value>) -> Value {
    Value::Array(v)
}
fn put(v: &mut Value, name: &str, value: Value) {
    let Value::Map(fields) = v else {
        unreachable!()
    };
    if let Some((_, old)) = fields.iter_mut().find(|(k, _)| k == &text(name)) {
        *old = value;
    } else {
        fields.push((text(name), value));
    }
}
fn get_mut<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    let Value::Map(fields) = v else {
        unreachable!()
    };
    let Some((_, value)) = fields.iter_mut().find(|(k, _)| k == &text(name)) else {
        unreachable!()
    };
    value
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
fn construct(ty: &str, case: &str, payload: Option<Value>) -> Value {
    let mut value = map([
        ("kind", text("variant")),
        ("type", text(ty)),
        ("case", text(case)),
    ]);
    if let Some(payload) = payload {
        put(&mut value, "payload", payload);
    }
    value
}
fn union(payload: &str) -> Value {
    map([
        ("kind", text("Variant")),
        (
            "cases",
            map([
                ("Empty", map([])),
                ("Data", map([("payload", text(payload))])),
            ]),
        ),
    ])
}
fn function(result: Value, ty: &str) -> Value {
    map([
        ("params", array(vec![])),
        ("returnType", text(ty)),
        (
            "body",
            map([
                ("locals", array(vec![])),
                ("bindings", array(vec![])),
                ("result", result),
            ]),
        ),
    ])
}
fn call(name: &str) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(name)),
        ("typeArgs", array(vec![])),
        ("args", array(vec![])),
    ])
}
fn fixture(result: Value, ty: &str) -> (Value, Value) {
    (
        map([
            ("coordinate", text("choices.example@1")),
            (
                "types",
                map([
                    ("Choice", union("U8")),
                    ("Other", union("U8")),
                    ("Outer", union("choices.example@1.Choice")),
                ]),
            ),
            ("intents", map([])),
            ("functions", map([("make", function(result, ty))])),
        ]),
        map([("effects", array(vec![])), ("pureFunctions", array(vec![]))]),
    )
}
fn refused(core: &Value, exports: &Value) {
    assert!(
        matches!(validate(core,exports,false),Err(refusal) if refusal.kind==crate::ProviderRefusalKind::UnsupportedSemantics)
    );
}

#[test]
fn variants_accept_named_payloadless_payload_and_nested_constructors() {
    for (value, ty) in [
        (
            construct("Choice", "Empty", None),
            "choices.example@1.Choice",
        ),
        (
            construct("choices.example@1.Choice", "Data", Some(integer("U8", 255))),
            "Choice",
        ),
        (
            construct("Outer", "Data", Some(construct("Choice", "Empty", None))),
            "Outer",
        ),
    ] {
        let (core, exports) = fixture(value, ty);
        assert!(validate(&core, &exports, false).is_ok(), "{ty}");
    }
}

#[test]
fn variants_refuse_unknown_metadata_arity_width_and_distinct_union_identity() {
    let mut extra = construct("Choice", "Empty", None);
    put(&mut extra, "payload", Value::Null);
    let mut unknown = construct("Choice", "Empty", None);
    put(&mut unknown, "wireTag", Value::Integer(0));
    for value in [
        construct("Choice", "Missing", None),
        construct("Missing", "Empty", None),
        construct("U8", "Empty", None),
        construct("Choice", "Data", None),
        construct("Choice", "Empty", Some(integer("U8", 1))),
        construct("Choice", "Data", Some(integer("U32", 1))),
        construct("Choice", "Data", Some(integer("U8", 256))),
        construct("Other", "Empty", None),
        extra,
        unknown,
    ] {
        let (core, exports) = fixture(value, "Choice");
        refused(&core, &exports);
    }
    let (mut core, exports) = fixture(call("choices.example@1.other"), "Choice");
    put(
        get_mut(&mut core, "functions"),
        "other",
        function(construct("Other", "Empty", None), "Other"),
    );
    refused(&core, &exports);
}

#[test]
fn variants_refuse_malformed_recursive_and_overflowing_type_graphs() {
    let mut duplicate = union("U8");
    let Value::Map(cases) = get_mut(&mut duplicate, "cases") else {
        unreachable!()
    };
    cases.push((text("Empty"), map([])));
    let mut bad_case = union("U8");
    put(
        get_mut(get_mut(&mut bad_case, "cases"), "Empty"),
        "payload",
        Value::Null,
    );
    for definition in [
        union("Choice"),
        union("Missing"),
        union("Bytes<max=18446744073709551615>"),
        map([("kind", text("Variant")), ("cases", map([]))]),
        duplicate,
        bad_case,
    ] {
        let (mut core, exports) = fixture(construct("Choice", "Empty", None), "Choice");
        put(get_mut(&mut core, "types"), "Choice", definition);
        refused(&core, &exports);
    }
}

#[test]
fn variant_payloads_cannot_hide_cycles_capture_or_partial_calls() {
    let captured = map([
        ("kind", text("local")),
        (
            "ref",
            map([
                ("id", text("arg.9")),
                ("alphaName", text("$arg9")),
                ("type", text("U8")),
            ]),
        ),
    ]);
    let lookup = map([
        ("kind", text("call")),
        ("callee", text("core.bytes.at")),
        ("typeArgs", array(vec![text("Bytes<max=1>")])),
        (
            "args",
            array(vec![
                map([
                    ("kind", text("const")),
                    (
                        "value",
                        map([("kind", text("bytes")), ("value", Value::Bytes(vec![]))]),
                    ),
                ]),
                integer("U64", 0),
            ]),
        ),
    ]);
    for payload in [call("choices.example@1.make"), captured, lookup] {
        let (core, exports) = fixture(construct("Choice", "Data", Some(payload)), "Choice");
        refused(&core, &exports);
    }
    let (mut core, exports) = fixture(
        construct("Choice", "Data", Some(call("choices.example@1.word"))),
        "Choice",
    );
    put(
        get_mut(&mut core, "functions"),
        "word",
        function(integer("U8", 7), "U8"),
    );
    assert!(
        validate(&core, &exports, false).is_ok(),
        "payload dependency precedes caller"
    );
}

#[test]
fn variant_maximum_payload_bounds_participate_in_the_whole_operation_budget() {
    for (payload, accepted) in [("U8", true), ("Bytes<max=1048576>", false)] {
        let (mut core, exports) = fixture(construct("Choice", "Empty", None), "Choice");
        put(get_mut(&mut core, "types"), "Choice", union(payload));
        let input = map([
            ("id", text("arg.0")),
            ("alphaName", text("$arg0")),
            ("type", text("U8")),
        ]);
        let binding = map([
            ("id", text("local.0")),
            ("alphaName", text("$local0")),
            ("type", text("Choice")),
        ]);
        let intent = map([
            ("input", text("U8")),
            ("output", text("Choice")),
            ("inputConstraints", array(vec![])),
            (
                "coreEvaluationBudget",
                map([
                    ("maxSteps", Value::Integer(65536)),
                    ("maxAllocatedBytes", Value::Integer(4096)),
                    ("maxOutputBytes", Value::Integer(4096)),
                ]),
            ),
            (
                "body",
                map([
                    ("locals", array(vec![input.clone(), binding.clone()])),
                    (
                        "nodes",
                        array(vec![map([
                            ("kind", text("let")),
                            ("binding", binding.clone()),
                            ("value", call("choices.example@1.make")),
                        ])]),
                    ),
                    ("result", map([("kind", text("local")), ("ref", binding)])),
                ]),
            ),
        ]);
        put(&mut core, "intents", map([("run", intent)]));
        assert_eq!(
            validate(&core, &exports, false).is_ok(),
            accepted,
            "{payload}"
        );
    }
}

#[test]
fn variants_preserve_original_imported_and_nominal_payload_identities() {
    let foreign = "shared.union@2.Choice";
    let (mut core, exports) = fixture(construct(foreign, "Empty", None), foreign);
    put(get_mut(&mut core, "types"), foreign, union("U8"));
    assert!(validate(&core, &exports, false).is_ok());
    put(
        get_mut(get_mut(&mut core, "functions"), "make"),
        "returnType",
        text("Choice"),
    );
    refused(&core, &exports);
    for (parameter_type, accepted) in [("shared.nominal@2.Tag", true), ("U8", false)] {
        let parameter = map([
            ("id", text("arg.0")),
            ("alphaName", text("$arg0")),
            ("type", text(parameter_type)),
        ]);
        let payload = map([("kind", text("local")), ("ref", parameter.clone())]);
        let (mut core, exports) = fixture(construct("Choice", "Data", Some(payload)), "Choice");
        put(
            get_mut(&mut core, "types"),
            "Choice",
            union("shared.nominal@2.Tag"),
        );
        put(
            get_mut(&mut core, "types"),
            "shared.nominal@2.Tag",
            map([
                ("kind", text("Nominal")),
                ("contract", text("shared.nominal@2.Tag")),
                ("representation", text("U8")),
            ]),
        );
        put(
            get_mut(get_mut(&mut core, "functions"), "make"),
            "params",
            array(vec![parameter]),
        );
        assert_eq!(validate(&core, &exports, false).is_ok(), accepted);
    }
}

#[test]
fn variant_case_work_and_combined_expansion_depth_remain_bounded() {
    for (levels, accepted) in [(8, true), (65, false)] {
        let (mut core, exports) = fixture(construct("Choice", "Empty", None), "Choice");
        put(get_mut(&mut core, "types"), "Choice", union("V0"));
        for index in 0..levels {
            let payload = if index + 1 == levels {
                "U8".to_owned()
            } else {
                format!("V{}", index + 1)
            };
            put(
                get_mut(&mut core, "types"),
                &format!("V{index}"),
                union(&payload),
            );
        }
        assert_eq!(
            validate(&core, &exports, false).is_ok(),
            accepted,
            "{levels}"
        );
    }
    let (mut core, exports) = fixture(construct("Choice", "Empty", None), "Choice");
    let mut cases = vec![(text("Empty"), map([]))];
    for index in 0..65_536 {
        cases.push((text(&format!("C{index}")), map([])));
    }
    put(
        get_mut(&mut core, "types"),
        "Choice",
        map([("kind", text("Variant")), ("cases", Value::Map(cases))]),
    );
    refused(&core, &exports);
}

fn matched() -> Value {
    let binder = map([
        ("id", text("match.0")),
        ("type", text("U8")),
        ("alphaName", text("payload")),
    ]);
    map([
        ("kind", text("match")),
        (
            "scrutinee",
            construct("Choice", "Data", Some(integer("U8", 9))),
        ),
        (
            "arms",
            array(vec![
                map([("case", text("Empty")), ("body", integer("U8", 0))]),
                map([
                    ("case", text("Data")),
                    ("binder", binder.clone()),
                    ("body", map([("kind", text("local")), ("ref", binder)])),
                ]),
            ]),
        ),
    ])
}

#[test]
fn matches_accept_exhaustive_payload_binding() {
    let (core, exports) = fixture(matched(), "U8");
    assert!(validate(&core, &exports, false).is_ok());
}

#[test]
fn matches_refuse_bad_coverage_payload_identity_and_scope() {
    for mutation in 0..7 {
        let mut result = matched();
        let Value::Array(arms) = get_mut(&mut result, "arms") else {
            unreachable!()
        };
        match mutation {
            0 => {
                arms.pop();
            }
            1 => {
                put(&mut arms[1], "case", text("Empty"));
            }
            2 => {
                put(&mut arms[1], "case", text("Unknown"));
            }
            3 => {
                put(get_mut(&mut arms[1], "binder"), "type", text("Bool"));
            }
            4 => {
                let binder = get_mut(&mut arms[1], "binder").clone();
                put(&mut arms[0], "binder", binder);
            }
            5 => {
                let body = get_mut(&mut arms[1], "body").clone();
                put(&mut arms[0], "body", body);
            }
            6 => {
                put(get_mut(&mut arms[1], "binder"), "alphaName", text(""));
            }
            _ => unreachable!(),
        }
        let (core, exports) = fixture(result, "U8");
        refused(&core, &exports);
    }
}

#[test]
fn matches_require_intent_budget_even_without_a_function_table() {
    for (budget, accepted) in [(4096, true), (1, false)] {
        let (mut core, exports) = fixture(matched(), "U8");
        let input = map([
            ("id", text("arg.0")),
            ("alphaName", text("input")),
            ("type", text("U8")),
        ]);
        let intent = map([
            ("input", text("U8")),
            ("output", text("U8")),
            ("inputConstraints", array(vec![])),
            (
                "coreEvaluationBudget",
                map([
                    ("maxSteps", Value::Integer(budget)),
                    ("maxAllocatedBytes", Value::Integer(4096)),
                    ("maxOutputBytes", Value::Integer(4096)),
                ]),
            ),
            (
                "body",
                map([
                    ("locals", array(vec![input])),
                    ("nodes", array(vec![])),
                    ("result", matched()),
                ]),
            ),
        ]);
        put(&mut core, "intents", map([("run", intent)]));
        let Value::Map(fields) = &mut core else {
            unreachable!()
        };
        fields.retain(|(key, _)| key != &text("functions"));
        assert_eq!(
            validate(&core, &exports, false).is_ok(),
            accepted,
            "budget {budget}"
        );
    }
}

#[test]
fn matches_validate_unselected_arms_and_reject_outer_binder_capture() {
    for callee in ["choices.example@1.make", "choices.example@1.missing"] {
        let mut result = matched();
        let Value::Array(arms) = get_mut(&mut result, "arms") else {
            unreachable!()
        };
        // The constructor selects Data, but Empty still belongs to the static proof.
        put(&mut arms[0], "body", call(callee));
        let (core, exports) = fixture(result, "U8");
        refused(&core, &exports);
    }
    let mut result = matched();
    let Value::Array(arms) = get_mut(&mut result, "arms") else {
        unreachable!()
    };
    let binder = get_mut(&mut arms[1], "binder").clone();
    let (mut core, exports) = fixture(result, "U8");
    put(
        get_mut(get_mut(&mut core, "functions"), "make"),
        "params",
        array(vec![binder]),
    );
    refused(&core, &exports);
}

mod frame_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/edict-match-frame.rs"
    ));
}
#[test]
fn match_populated_frame_accounting() -> super::Check<()> {
    for binder in [false, true] {
        for selected in 0..2 {
            let f = frame_fixture::fixture(binder, selected);
            let mut proof = super::Judgment {
                core: &f.core,
                functions: super::BTreeMap::new(),
                completed: super::BTreeMap::new(),
                active: super::BTreeSet::new(),
                work: super::WORK,
                proven: vec![],
                path: vec![],
                bound: vec![],
                proven_sum: None,
            };
            let mut scope = super::Scope::new();
            for declaration in &f.declarations {
                proof.declaration(declaration, &mut scope)?;
            }
            let fact = proof.expression(&f.expression, &scope, true, 0)?;
            let measured = (fact.cost.steps, fact.cost.bytes);
            // Static cost bounds both arms; dispatch may inspect two cases.
            assert_eq!(measured, frame_fixture::runtime_cost(binder, 1));
            let concrete = frame_fixture::runtime_cost(binder, selected);
            assert!(measured.0 >= concrete.0 && measured.1 >= concrete.1);
            f.report("lowerer", binder, selected, measured)
                .map_err(|_| ())?;
        }
    }
    Ok(())
}

fn frame_intent(binder: bool) -> (Value, Value) {
    let f = frame_fixture::fixture(binder, 1);
    let mut core = f.core;
    let input = map([
        ("id", text("arg.0")),
        ("alphaName", text("input")),
        ("type", text("Frame")),
    ]);
    put(
        get_mut(&mut core, "types"),
        "Frame",
        map([
            ("kind", text("Record")),
            (
                "fields",
                map([
                    ("word", text("U8")),
                    ("bytes", text("Bytes<exact=4>")),
                    ("record", text("Flags")),
                    ("choice", text("Choice")),
                ]),
            ),
        ]),
    );
    let mut helper = function(f.expression, "U8");
    put(&mut helper, "params", array(f.declarations));
    put(&mut core, "functions", map([("make", helper)]));
    let mut result = call("choices.example@1.make");
    put(
        &mut result,
        "args",
        array(
            ["word", "bytes", "record", "choice"]
                .into_iter()
                .map(|name| {
                    map([
                        ("kind", text("field")),
                        (
                            "base",
                            map([("kind", text("local")), ("ref", input.clone())]),
                        ),
                        ("field", text(name)),
                    ])
                })
                .collect(),
        ),
    );
    put(
        &mut core,
        "intents",
        map([(
            "run",
            map([
                ("input", text("Frame")),
                ("output", text("U8")),
                ("inputConstraints", array(vec![])),
                (
                    "coreEvaluationBudget",
                    map([
                        ("maxSteps", Value::Integer(32768)),
                        ("maxAllocatedBytes", Value::Integer(32768)),
                        ("maxOutputBytes", Value::Integer(4096)),
                    ]),
                ),
                (
                    "body",
                    map([
                        ("locals", array(vec![input])),
                        ("nodes", array(vec![])),
                        ("result", result),
                    ]),
                ),
            ]),
        )]),
    );
    (
        core,
        map([("effects", array(vec![])), ("pureFunctions", array(vec![]))]),
    )
}
fn frame_budget(core: &mut Value, axis: &str, n: u64) {
    put(
        get_mut(
            get_mut(get_mut(core, "intents"), "run"),
            "coreEvaluationBudget",
        ),
        axis,
        Value::Integer(n.into()),
    );
}
#[test]
fn match_populated_frame_accounting_whole_intent_budgets() {
    let mut minimums = Vec::new();
    for binder in [false, true] {
        let (core, exports) = frame_intent(binder);
        assert!(validate(&core, &exports, false).is_ok());
        let mut minimum = [0; 2];
        for (index, axis) in ["maxSteps", "maxAllocatedBytes"].into_iter().enumerate() {
            // A bounded binary search observes the admission boundary without
            // consulting either implementation's private cost summary.
            let mut candidate = core.clone();
            let (mut low, mut high) = (0, 32768);
            while low < high {
                let middle = low + (high - low) / 2;
                frame_budget(&mut candidate, axis, middle);
                if validate(&candidate, &exports, false).is_ok() {
                    high = middle;
                } else {
                    low = middle + 1;
                }
            }
            assert!(low > 0);
            frame_budget(&mut candidate, axis, low);
            assert!(validate(&candidate, &exports, false).is_ok());
            frame_budget(&mut candidate, axis, low - 1);
            assert!(validate(&candidate, &exports, false).is_err());
            minimum[index] = low;
        }
        let mut exact = core.clone();
        frame_budget(&mut exact, "maxSteps", minimum[0]);
        frame_budget(&mut exact, "maxAllocatedBytes", minimum[1]);
        assert!(validate(&exact, &exports, false).is_ok());
        minimums.push(minimum);
    }
    assert_eq!(
        (
            minimums[1][0] - minimums[0][0],
            minimums[1][1] - minimums[0][1]
        ),
        frame_fixture::FRAME_COPY
    );
}
