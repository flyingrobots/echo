// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::{
    evaluate::{expression, validate, Meter},
    model::MAX_PROGRAM_NODES,
    syntax::Parser,
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;
fn map(values: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        values
            .into_iter()
            .map(|(k, v)| (Value::Text(k.into()), v))
            .collect(),
    )
}
fn limits(steps: u64, allocated: u64) -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 4096,
        max_input_bytes: 4096,
        max_output_bytes: 4096,
        max_steps: steps,
        max_allocated_bytes: allocated,
    }
}
fn run(steps: u64, allocated: u64) -> Result<(Value, (u64, u64)), Error> {
    let types = map([]);
    let mut parser = Parser {
        types: &types,
        coordinate: "generic.lists@1",
        remaining: MAX_PROGRAM_NODES,
    };
    let values = [0, 255]
        .into_iter()
        .map(|n| {
            map([
                ("kind", Value::Text("const".into())),
                (
                    "value",
                    map([
                        ("kind", Value::Text("int".into())),
                        ("width", Value::Text("U64".into())),
                        ("value", Value::Integer(n)),
                    ]),
                ),
            ])
        })
        .collect();
    let expr = parser.expr(
        &map([
            ("kind", Value::Text("list".into())),
            ("values", Value::Array(values)),
        ]),
        0,
    )?;
    let mut meter = Meter::new(limits(steps, allocated));
    let result = expression(&expr, &BTreeMap::new(), &BTreeMap::new(), &mut meter, 0)?;
    Ok((result, meter.usage()))
}
#[test]
fn bounded_lists_execute_with_exact_charges() -> Result<(), Error> {
    assert_eq!(
        run(5, 192)?,
        (
            Value::Array(vec![Value::Integer(0), Value::Integer(255)]),
            (5, 192)
        )
    );
    assert_eq!(run(4, 192), Err(Error::StepBudgetExceeded));
    assert_eq!(run(5, 191), Err(Error::AllocationBudgetExceeded));
    Ok(())
}
#[test]
fn bounded_lists_validate_every_element_and_length() -> Result<(), Error> {
    let types = map([]);
    let mut parser = Parser {
        types: &types,
        coordinate: "generic.lists@1",
        remaining: MAX_PROGRAM_NODES,
    };
    let ty = parser.ty("List<U8,max=2>", 0)?;
    for value in [
        Value::Array(vec![]),
        Value::Array(vec![Value::Integer(0), Value::Integer(255)]),
    ] {
        assert_eq!(
            validate(&value, &ty, &mut Meter::new(limits(3, 0)), 0),
            Ok(())
        );
    }
    for value in [
        Value::Array(vec![Value::Integer(256)]),
        Value::Array(vec![Value::Bool(false)]),
        Value::Array(vec![Value::Integer(0); 3]),
        Value::Map(vec![]),
    ] {
        assert_eq!(
            validate(&value, &ty, &mut Meter::new(limits(8, 0)), 0),
            Err(Error::InvalidArtifact)
        );
    }
    let value = Value::Array(vec![Value::Integer(1), Value::Integer(2)]);
    assert_eq!(
        validate(&value, &ty, &mut Meter::new(limits(2, 0)), 0),
        Err(Error::StepBudgetExceeded)
    );
    Ok(())
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn list(values: Vec<Value>) -> Value {
    map([("kind", text("list")), ("values", Value::Array(values))])
}
fn record(bytes: Vec<u8>) -> Value {
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
}
fn call(name: &str) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(&format!("generic.lists@1.{name}"))),
        ("args", Value::Array(vec![])),
        ("typeArgs", Value::Array(vec![])),
    ])
}

#[test]
fn bounded_lists_decode_and_execute_transitive_record_helpers() -> Result<(), Error> {
    let types = map([(
        "Row",
        map([
            ("kind", text("Record")),
            ("fields", map([("payload", text("Bytes<max=4>"))])),
        ]),
    )]);
    let helper = |result| {
        map([
            ("params", Value::Array(vec![])),
            ("returnType", text("List<Row,max=2>")),
            (
                "body",
                map([
                    ("locals", Value::Array(vec![])),
                    ("bindings", Value::Array(vec![])),
                    ("result", result),
                ]),
            ),
        ])
    };
    let core = map([
        ("types", types.clone()),
        ("intents", map([])),
        (
            "functions",
            map([
                (
                    "make",
                    helper(list(vec![record(vec![]), record(vec![0, 255, 3])])),
                ),
                ("relay", helper(call("make"))),
            ]),
        ),
    ]);
    let exports = map([
        ("pureFunctions", Value::Array(vec![])),
        ("effects", Value::Array(vec![])),
    ]);
    let mut parser = Parser {
        types: &types,
        coordinate: "generic.lists@1",
        remaining: MAX_PROGRAM_NODES,
    };
    // This invokes helper inference and closure checks before evaluation.
    let helpers = super::functions::decode(&core, &exports, &mut parser)?;
    let expr = parser.expr(&call("relay"), 0)?;
    let expected = Value::Array(vec![
        map([("payload", Value::Bytes(vec![]))]),
        map([("payload", Value::Bytes(vec![0, 255, 3]))]),
    ]);
    let mut meter = Meter::new(limits(1024, 16_384));
    let actual = expression(&expr, &BTreeMap::new(), &helpers, &mut meter, 0)?;
    assert_eq!(actual, expected);
    validate(&actual, &parser.ty("List<Row,max=2>", 0)?, &mut meter, 0)?;
    Ok(())
}

#[test]
fn bounded_lists_record_joins_preserve_field_names_and_nominals() -> Result<(), Error> {
    use super::{functions::join, model::RuntimeType as Ty};
    let row = |name: &str, value| Ty::Record(vec![(name.into(), value)]);
    let bytes = |length| Ty::Bytes {
        min: length,
        max: length,
    };
    assert_eq!(
        join(row("payload", bytes(0)), row("payload", bytes(3)))?,
        row("payload", Ty::Bytes { min: 0, max: 3 })
    );
    assert_eq!(
        join(row("left", bytes(0)), row("right", bytes(3))),
        Err(Error::UnsupportedProgram)
    );
    assert_eq!(
        join(
            row("payload", Ty::Nominal("A".into(), Box::new(bytes(3)))),
            row("payload", Ty::Nominal("B".into(), Box::new(bytes(3))))
        ),
        Err(Error::UnsupportedProgram)
    );
    Ok(())
}
