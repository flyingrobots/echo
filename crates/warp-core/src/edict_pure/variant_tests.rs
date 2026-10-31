// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic runtime boundaries; no compiler-package or provider provenance claim.
use super::{
    evaluate::{expression, validate, Meter},
    functions,
    model::{Expr, Helper, Program, RuntimeType, MAX_PROGRAM_NODES},
    syntax::Parser,
    values::field,
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;
fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(fields.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn array(v: Vec<Value>) -> Value {
    Value::Array(v)
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
fn constructor(ty: &str, case: &str, payload: Option<Value>) -> Value {
    let mut fields = vec![
        ("kind", text("variant")),
        ("type", text(ty)),
        ("case", text(case)),
    ];
    if let Some(payload) = payload {
        fields.push(("payload", payload));
    }
    map(fields)
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
fn call(name: &str) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(name)),
        ("args", array(vec![])),
        ("typeArgs", array(vec![])),
    ])
}
fn helper(result: Value, ty: &str) -> Value {
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
fn core(result: Value, ty: &str) -> Value {
    map([
        ("coordinate", text("choices.example@1")),
        (
            "types",
            map([
                ("Choice", union("U8")),
                ("Other", union("U8")),
                ("Outer", union("Choice")),
            ]),
        ),
        ("intents", map([])),
        ("functions", map([("make", helper(result, ty))])),
    ])
}
fn checked(core: &Value) -> Result<BTreeMap<String, Helper>, Error> {
    let exports = map([("effects", array(vec![])), ("pureFunctions", array(vec![]))]);
    let mut parser = Parser {
        types: field(core, "types")?,
        coordinate: "choices.example@1",
        remaining: MAX_PROGRAM_NODES,
    };
    let helpers = functions::decode(core, &exports, &mut parser)?;
    functions::check_roots(&helpers, [], [])?;
    Ok(helpers)
}
fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 1_048_576,
        max_input_bytes: 1_048_576,
        max_steps: 65536,
        max_allocated_bytes: 1_048_576,
        max_output_bytes: 1_048_576,
    }
}
fn invoke(helpers: &BTreeMap<String, Helper>, meter: &mut Meter) -> Result<Value, Error> {
    expression(
        &Expr::Call("choices.example@1.make".into(), vec![]),
        &BTreeMap::new(),
        helpers,
        meter,
        0,
    )
}
fn value(case: &str, payload: Option<Value>) -> Value {
    let mut fields = vec![
        ("type", text("choices.example@1.Choice")),
        ("case", text(case)),
    ];
    if let Some(payload) = payload {
        fields.push(("payload", payload));
    }
    map(fields)
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

#[test]
fn variant_constructors_execute_exact_metadata_and_copy_charges() -> Result<(), Error> {
    for (case, payload, expected_steps, cells, key_bytes) in [
        ("Empty", None, 7, 5, 8),
        ("Data", Some(integer("U8", 255)), 10, 7, 15),
    ] {
        let helpers = checked(&core(
            constructor("Choice", case, payload),
            "choices.example@1.Choice",
        ))?;
        let mut meter = Meter::new(limits());
        let actual = invoke(&helpers, &mut meter)?;
        let expected = value(
            case,
            if case == "Data" {
                Some(Value::Integer(255))
            } else {
                None
            },
        );
        assert_eq!(actual, expected);
        let bytes =
            cells * 64 + key_bytes + "choices.example@1.Choice".len() as u64 + case.len() as u64;
        assert_eq!(meter.usage(), (expected_steps, bytes));
        let locals = BTreeMap::from([("copied", actual)]);
        let before = meter.usage();
        assert_eq!(
            expression(
                &Expr::Local("copied".into()),
                &locals,
                &helpers,
                &mut meter,
                0
            )?,
            expected
        );
        assert_eq!(meter.usage(), (before.0 + cells + 1, before.1 + bytes));
    }
    Ok(())
}

#[test]
fn variant_values_require_exact_identity_case_payload_and_fields() -> Result<(), Error> {
    let helpers = checked(&core(
        constructor("Choice", "Data", Some(integer("U8", 1))),
        "Choice",
    ))?;
    let ty = &helpers["choices.example@1.make"].ty;
    for invalid in [
        value("Missing", None),
        value("Data", None),
        value("Empty", Some(Value::Null)),
        value("Data", Some(Value::Bool(true))),
        value("Data", Some(Value::Integer(256))),
        map([
            ("type", text("choices.example@1.Other")),
            ("case", text("Empty")),
        ]),
        map([
            ("type", text("choices.example@1.Choice")),
            ("case", text("Empty")),
            ("wireTag", Value::Integer(0)),
        ]),
        map([
            ("type", text("choices.example@1.Choice")),
            ("case", text("Data")),
            ("payload", Value::Null),
        ]),
    ] {
        assert_eq!(
            validate(&invalid, ty, &mut Meter::new(limits()), 0),
            Err(Error::InvalidArtifact)
        );
    }
    validate(&value("Empty", None), ty, &mut Meter::new(limits()), 0)?;
    validate(
        &value("Data", Some(Value::Integer(255))),
        ty,
        &mut Meter::new(limits()),
        0,
    )?;
    Ok(())
}

#[test]
fn variant_admission_checks_payload_types_and_distinct_union_returns() {
    for result in [
        constructor("Choice", "Missing", None),
        constructor("Choice", "Data", None),
        constructor("Choice", "Empty", Some(integer("U8", 0))),
        constructor("Choice", "Data", Some(integer("U32", 0))),
        constructor("Other", "Empty", None),
    ] {
        assert!(matches!(
            checked(&core(result, "Choice")),
            Err(Error::InvalidArtifact)
        ));
    }
}

#[test]
fn variant_nested_values_and_payload_helper_calls_execute() -> Result<(), Error> {
    let helpers = checked(&core(
        constructor("Outer", "Data", Some(constructor("Choice", "Empty", None))),
        "Outer",
    ))?;
    let expected = map([
        ("type", text("choices.example@1.Outer")),
        ("case", text("Data")),
        ("payload", value("Empty", None)),
    ]);
    assert_eq!(invoke(&helpers, &mut Meter::new(limits()))?, expected);
    let mut with_call = core(
        constructor("Choice", "Data", Some(call("choices.example@1.word"))),
        "Choice",
    );
    let Value::Map(functions) = get_mut(&mut with_call, "functions") else {
        unreachable!()
    };
    functions.push((text("word"), helper(integer("U8", 17), "U8")));
    let helpers = checked(&with_call)?;
    assert_eq!(
        invoke(&helpers, &mut Meter::new(limits()))?,
        value("Data", Some(Value::Integer(17)))
    );
    let Value::Map(functions) = get_mut(&mut with_call, "functions") else {
        unreachable!()
    };
    functions[1].1 = helper(call("choices.example@1.word"), "U8");
    assert!(matches!(checked(&with_call), Err(Error::InvalidArtifact)));
    Ok(())
}

#[test]
fn variant_execution_refuses_each_resource_limit_before_success() -> Result<(), Error> {
    let helpers = checked(&core(
        constructor("Choice", "Data", Some(integer("U8", 255))),
        "Choice",
    ))?;
    let mut meter = Meter::new(limits());
    invoke(&helpers, &mut meter)?;
    let (steps, bytes) = meter.usage();
    let mut cap = limits();
    cap.max_steps = steps - 1;
    assert_eq!(
        invoke(&helpers, &mut Meter::new(cap)),
        Err(Error::StepBudgetExceeded)
    );
    cap = limits();
    cap.max_allocated_bytes = bytes - 1;
    assert_eq!(
        invoke(&helpers, &mut Meter::new(cap)),
        Err(Error::AllocationBudgetExceeded)
    );
    let mut program = Program {
        input_id: "arg.0".into(),
        input_type: RuntimeType::Unsigned(255),
        output_type: helpers["choices.example@1.make"].ty.clone(),
        constraints: vec![],
        bindings: vec![],
        helpers,
        result: Expr::Call("choices.example@1.make".into(), vec![]),
        limits: limits(),
    };
    let first = super::evaluate::run(&program, Value::Integer(0))?;
    assert_eq!(super::evaluate::run(&program, Value::Integer(0))?, first);
    program.limits.max_output_bytes = first.output.len() as u64 - 1;
    assert_eq!(
        super::evaluate::run(&program, Value::Integer(0)),
        Err(Error::OutputBudgetExceeded)
    );
    Ok(())
}

#[test]
fn variant_arguments_move_once_into_fresh_frames_then_charge_local_copies() -> Result<(), Error> {
    let parameter = map([
        ("id", text("arg.0")),
        ("alphaName", text("$arg0")),
        ("type", text("Choice")),
    ]);
    let reference = map([("kind", text("local")), ("ref", parameter.clone())]);
    let mut identity = helper(reference, "Choice");
    *get_mut(&mut identity, "params") = array(vec![parameter]);
    let mut invocation = call("choices.example@1.identity");
    *get_mut(&mut invocation, "args") = array(vec![constructor(
        "Choice",
        "Data",
        Some(integer("U8", 255)),
    )]);
    let mut source = core(invocation, "Choice");
    let Value::Map(functions) = get_mut(&mut source, "functions") else {
        unreachable!()
    };
    functions.push((text("identity"), identity));
    let helpers = checked(&source)?;
    let mut meter = Meter::new(limits());
    assert_eq!(
        invoke(&helpers, &mut meter)?,
        value("Data", Some(Value::Integer(255)))
    );
    let bytes = 7 * 64 + 15 + "choices.example@1.Choice".len() as u64 + 4;
    assert_eq!(meter.usage(), (27, 2 * bytes));
    Ok(())
}

#[test]
fn variant_imports_and_nominal_payloads_keep_original_static_identities() -> Result<(), Error> {
    let foreign = "shared.union@2.Choice";
    let mut source = core(constructor(foreign, "Empty", None), foreign);
    let Value::Map(types) = get_mut(&mut source, "types") else {
        unreachable!()
    };
    types.push((text(foreign), union("U8")));
    let helpers = checked(&source)?;
    assert_eq!(
        invoke(&helpers, &mut Meter::new(limits()))?,
        map([("type", text(foreign)), ("case", text("Empty"))])
    );
    *get_mut(
        get_mut(get_mut(&mut source, "functions"), "make"),
        "returnType",
    ) = text("Choice");
    assert!(matches!(checked(&source), Err(Error::InvalidArtifact)));
    for (parameter_type, accepted) in [("shared.nominal@2.Tag", true), ("U8", false)] {
        let p = map([
            ("id", text("arg.0")),
            ("alphaName", text("$arg0")),
            ("type", text(parameter_type)),
        ]);
        let mut source = core(
            constructor(
                "Choice",
                "Data",
                Some(map([("kind", text("local")), ("ref", p.clone())])),
            ),
            "Choice",
        );
        *get_mut(get_mut(get_mut(&mut source, "functions"), "make"), "params") = array(vec![p]);
        *get_mut(get_mut(&mut source, "types"), "Choice") = union("shared.nominal@2.Tag");
        let Value::Map(types) = get_mut(&mut source, "types") else {
            unreachable!()
        };
        types.push((
            text("shared.nominal@2.Tag"),
            map([
                ("kind", text("Nominal")),
                ("contract", text("shared.nominal@2.Tag")),
                ("representation", text("U8")),
            ]),
        ));
        if accepted {
            let helpers = checked(&source)?;
            let result = expression(
                &Expr::Call(
                    "choices.example@1.make".into(),
                    vec![Expr::Constant(Value::Integer(7))],
                ),
                &BTreeMap::new(),
                &helpers,
                &mut Meter::new(limits()),
                0,
            )?;
            assert_eq!(result, value("Data", Some(Value::Integer(7))));
        } else {
            assert!(matches!(checked(&source), Err(Error::InvalidArtifact)));
        }
    }
    Ok(())
}

#[test]
fn variant_type_graphs_refuse_empty_recursive_overflowing_deep_and_wide_shapes() -> Result<(), Error>
{
    for definition in [
        union("Choice"),
        union("Bytes<max=18446744073709551615>"),
        map([("kind", text("Variant")), ("cases", map([]))]),
    ] {
        let mut source = core(constructor("Choice", "Empty", None), "Choice");
        *get_mut(get_mut(&mut source, "types"), "Choice") = definition;
        assert!(matches!(
            checked(&source),
            Err(Error::UnsupportedProgram | Error::InvalidArtifact)
        ));
    }
    for (levels, accepted) in [(8, true), (65, false)] {
        let mut source = core(constructor("Choice", "Empty", None), "Choice");
        *get_mut(get_mut(&mut source, "types"), "Choice") = union("V0");
        let Value::Map(types) = get_mut(&mut source, "types") else {
            unreachable!()
        };
        for index in 0..levels {
            let payload = if index + 1 == levels {
                "U8".to_owned()
            } else {
                format!("V{}", index + 1)
            };
            types.push((text(&format!("V{index}")), union(&payload)));
        }
        if accepted {
            checked(&source)?;
        } else {
            assert!(matches!(checked(&source), Err(Error::UnsupportedProgram)));
        }
    }
    let mut source = core(constructor("Choice", "Empty", None), "Choice");
    let mut cases = vec![(text("Empty"), map([]))];
    for index in 0..65_536 {
        cases.push((text(&format!("C{index}")), map([])));
    }
    *get_mut(get_mut(&mut source, "types"), "Choice") =
        map([("kind", text("Variant")), ("cases", Value::Map(cases))]);
    assert!(matches!(checked(&source), Err(Error::UnsupportedProgram)));
    Ok(())
}

fn matched(case: &str, payload: Option<Value>) -> Value {
    let binder = map([
        ("id", text("match.0.payload")),
        ("type", text("U8")),
        ("alphaName", text("payload")),
    ]);
    map([
        ("kind", text("match")),
        ("scrutinee", constructor("Choice", case, payload)),
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
fn exhaustive_match_selects_payload_and_empty_arms() -> Result<(), Error> {
    for (case, payload, expected) in [("Empty", None, 0), ("Data", Some(integer("U8", 37)), 37)] {
        let helpers = checked(&core(matched(case, payload), "U8"))?;
        assert_eq!(
            invoke(&helpers, &mut Meter::new(limits()))?,
            Value::Integer(expected)
        );
    }
    Ok(())
}

#[test]
fn match_parser_rejects_duplicate_and_empty_arm_lists() -> Result<(), Error> {
    let source = core(integer("U8", 0), "U8");
    for duplicate in [true, false] {
        let mut value = matched("Empty", None);
        let Value::Array(arms) = get_mut(&mut value, "arms") else {
            unreachable!()
        };
        if duplicate {
            // Keep two syntactically valid, binder-free bodies. The case tag
            // alone is duplicated; helper inference is deliberately bypassed.
            arms[1] = arms[0].clone();
        } else {
            arms.clear();
        }
        let mut parser = Parser {
            types: field(&source, "types")?,
            coordinate: "choices.example@1",
            remaining: MAX_PROGRAM_NODES,
        };
        assert!(
            matches!(parser.expr(&value, 0), Err(Error::InvalidArtifact)),
            "duplicate={duplicate}"
        );
    }
    Ok(())
}

#[test]
fn match_parser_leaves_typed_coverage_to_provider_judgments() -> Result<(), Error> {
    let source = core(integer("U8", 0), "U8");
    let mut value = matched("Data", Some(integer("U8", 37)));
    for complete in [true, false] {
        if !complete {
            let Value::Array(arms) = get_mut(&mut value, "arms") else {
                unreachable!()
            };
            arms.pop();
        }
        // This is the expression parser used for intent bindings/results in
        // decode.rs, not source-helper inference. It has no local type scope.
        let mut parser = Parser {
            types: field(&source, "types")?,
            coordinate: "choices.example@1",
            remaining: MAX_PROGRAM_NODES,
        };
        let expr = parser.expr(&value, 0)?;
        let result = expression(
            &expr,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &mut Meter::new(limits()),
            0,
        );
        assert_eq!(
            result,
            if complete {
                Ok(Value::Integer(37))
            } else {
                Err(Error::InvalidArtifact)
            }
        );
    }
    Ok(())
}

#[test]
fn match_rejects_missing_duplicate_unknown_and_ill_scoped_arms() -> Result<(), Error> {
    for mutation in 0..8 {
        let mut result = matched("Empty", None);
        let Value::Array(arms) = get_mut(&mut result, "arms") else {
            unreachable!()
        };
        match mutation {
            0 => {
                arms.pop();
            }
            1 => {
                *get_mut(&mut arms[1], "case") = text("Empty");
            }
            2 => {
                *get_mut(&mut arms[1], "case") = text("Unknown");
            }
            3 => {
                *get_mut(get_mut(&mut arms[1], "binder"), "type") = text("Bool");
                *get_mut(&mut arms[1], "body") = integer("U8", 0);
            }
            4 => {
                let binder = field(&arms[1], "binder")?.clone();
                let Value::Map(fields) = &mut arms[0] else {
                    unreachable!()
                };
                fields.push((text("binder"), binder));
            }
            5 => {
                *get_mut(&mut arms[0], "body") = field(&arms[1], "body")?.clone();
            }
            6 => {
                *get_mut(get_mut(&mut arms[1], "binder"), "alphaName") = text("");
                *get_mut(&mut arms[1], "body") = integer("U8", 0);
            }
            7 => {
                *get_mut(get_mut(&mut arms[1], "binder"), "alphaName") = text("renamed");
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(checked(&core(result, "U8")), Err(Error::InvalidArtifact)),
            "mutation {mutation}"
        );
    }
    Ok(())
}

#[test]
fn match_dispatch_obeys_exact_meter_limits() -> Result<(), Error> {
    let source = core(matched("Data", Some(integer("U8", 37))), "U8");
    let helpers = checked(&source)?;
    let mut meter = Meter::new(limits());
    invoke(&helpers, &mut meter)?;
    let (steps, bytes) = meter.usage();
    for (step_limit, byte_limit, expected) in [
        (steps - 1, bytes, Error::StepBudgetExceeded),
        (steps, bytes - 1, Error::AllocationBudgetExceeded),
    ] {
        let mut budget = limits();
        budget.max_steps = step_limit;
        budget.max_allocated_bytes = byte_limit;
        assert_eq!(invoke(&helpers, &mut Meter::new(budget)), Err(expected));
    }
    let mut exact = limits();
    exact.max_steps = steps;
    exact.max_allocated_bytes = bytes;
    assert_eq!(
        invoke(&helpers, &mut Meter::new(exact))?,
        Value::Integer(37)
    );
    Ok(())
}

mod frame_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/edict-match-frame.rs"
    ));
}

#[test]
fn match_populated_frame_accounting() -> Result<(), Error> {
    for binder in [false, true] {
        for selected in 0..2 {
            let f = frame_fixture::fixture(binder, selected);
            let mut parser = Parser {
                types: field(&f.core, "types")?,
                coordinate: "choices.example@1",
                remaining: MAX_PROGRAM_NODES,
            };
            let expr = parser.expr(&f.expression, 0)?;
            let locals: BTreeMap<_, _> = ["word", "bytes", "record", "choice"]
                .into_iter()
                .zip(f.values.iter().cloned())
                .collect();
            let mut meter = Meter::new(limits());
            assert_eq!(
                expression(&expr, &locals, &BTreeMap::new(), &mut meter, 0)?,
                Value::Integer(9)
            );
            let measured = meter.usage();
            assert_eq!(measured, frame_fixture::runtime_cost(binder, selected));
            // The ordinary Local operation is an independent copy oracle:
            // subtract only its expression-entry step, then sum all locals.
            let mut copy = (0, 0);
            for id in locals.keys() {
                let mut m = Meter::new(limits());
                expression(
                    &Expr::Local((*id).into()),
                    &locals,
                    &BTreeMap::new(),
                    &mut m,
                    0,
                )?;
                let (steps, bytes) = m.usage();
                copy.0 += steps - 1;
                copy.1 += bytes;
            }
            assert_eq!(copy, frame_fixture::FRAME_COPY);
            for (steps, bytes, error) in [
                (measured.0 - 1, measured.1, Some(Error::StepBudgetExceeded)),
                (
                    measured.0,
                    measured.1 - 1,
                    Some(Error::AllocationBudgetExceeded),
                ),
                (measured.0, measured.1, None),
            ] {
                let mut budget = limits();
                budget.max_steps = steps;
                budget.max_allocated_bytes = bytes;
                let result =
                    expression(&expr, &locals, &BTreeMap::new(), &mut Meter::new(budget), 0);
                assert_eq!(result, error.map_or(Ok(Value::Integer(9)), Err));
            }
            f.report("runtime", binder, selected, measured)
                .map_err(|_| Error::InvalidArtifact)?;
        }
    }
    Ok(())
}

#[test]
fn match_payload_may_reuse_pending_id_but_not_its_declaration() -> Result<(), Error> {
    for wrong_declaration in [false, true] {
        let mut value = matched("Data", Some(integer("U8", 37)));
        let Value::Array(arms) = get_mut(&mut value, "arms") else {
            unreachable!()
        };
        let mut pending = field(&arms[1], "binder")?.clone();
        *get_mut(&mut pending, "alphaName") = text("pending_result");
        if wrong_declaration {
            *get_mut(&mut arms[1], "body") =
                map([("kind", text("local")), ("ref", pending.clone())]);
        }
        let mut source = core(integer("U8", 0), "U8");
        *get_mut(get_mut(get_mut(&mut source, "functions"), "make"), "body") = map([
            ("locals", array(vec![pending.clone()])),
            (
                "bindings",
                array(vec![map([
                    ("kind", text("let")),
                    ("binding", pending.clone()),
                    ("value", value),
                ])]),
            ),
            ("result", map([("kind", text("local")), ("ref", pending)])),
        ]);
        if wrong_declaration {
            assert!(matches!(checked(&source), Err(Error::InvalidArtifact)));
        } else {
            assert_eq!(
                invoke(&checked(&source)?, &mut Meter::new(limits()))?,
                Value::Integer(37)
            );
        }
    }
    Ok(())
}

#[test]
fn match_binder_bytes_require_declared_payload_equality() {
    for (width, accepted) in [(8, true), (4, false), (16, false)] {
        let payload = map([
            ("kind", text("const")),
            (
                "value",
                map([
                    ("kind", text("bytes")),
                    ("value", Value::Bytes(vec![1, 2, 3])),
                ]),
            ),
        ]);
        let mut value = matched("Data", Some(payload));
        let Value::Array(arms) = get_mut(&mut value, "arms") else {
            unreachable!()
        };
        *get_mut(get_mut(&mut arms[1], "binder"), "type") = text(&format!("Bytes<max={width}>"));
        // The concrete three-byte payload fits every binder. Only equality
        // with the declared max=8 payload type distinguishes these cases.
        // A constant U8 body excludes local-ref and return-type mismatches.
        *get_mut(&mut arms[1], "body") = integer("U8", 0);
        let mut source = core(value, "U8");
        *get_mut(get_mut(&mut source, "types"), "Choice") = union("Bytes<max=8>");
        assert_eq!(checked(&source).is_ok(), accepted, "binder max={width}");
    }
}
