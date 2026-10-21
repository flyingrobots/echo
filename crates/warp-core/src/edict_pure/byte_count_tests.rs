// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Raw single-byte counting, its structural needle and its helper-frame bound.
use super::{
    evaluate::{expression, Meter},
    functions::{check_roots, decode},
    model::MAX_PROGRAM_NODES,
    syntax::Parser,
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(k, v)| (Value::Text(k.into()), v))
            .collect(),
    )
}
fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn bytes(value: &[u8]) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("bytes")),
                ("value", Value::Bytes(value.to_vec())),
            ]),
        ),
    ])
}
fn integer(value: u64) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", Value::Integer(i128::from(value))),
            ]),
        ),
    ])
}
fn local(id: &str, ty: &str) -> Value {
    map([
        ("id", text(id)),
        ("alphaName", text(id)),
        ("type", text(ty)),
    ])
}
fn reference(local: &Value) -> Value {
    map([("kind", text("local")), ("ref", local.clone())])
}
fn call(callee: &str, types: &[&str], args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        (
            "typeArgs",
            Value::Array(types.iter().map(|ty| text(ty)).collect()),
        ),
        ("args", Value::Array(args)),
    ])
}
fn count(operand: Value) -> Value {
    call(
        "core.bytes.count",
        &["Bytes<max=32>"],
        vec![operand, bytes(b"\n")],
    )
}
fn add(left: Value, right: Value) -> Value {
    call("core.integer.add", &["U64"], vec![left, right])
}
fn limits(steps: u64, allocated: u64) -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 1024,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        max_steps: steps,
        max_allocated_bytes: allocated,
    }
}
fn parsed(wire: &Value) -> Result<super::model::Expr, Error> {
    let types = Value::Map(vec![]);
    let mut parser = Parser {
        types: &types,
        coordinate: "count.example@1",
        remaining: MAX_PROGRAM_NODES,
    };
    parser.expr(wire, 0)
}
fn usage(operand: &[u8], needle: u8) -> Result<(Value, (u64, u64)), Error> {
    let wire = call(
        "core.bytes.count",
        &["Bytes<max=32>"],
        vec![bytes(operand), bytes(&[needle])],
    );
    let expr = parsed(&wire)?;
    check_roots(&BTreeMap::new(), [&expr], [])?;
    let mut meter = Meter::new(limits(1024, 4096));
    let value = expression(&expr, &BTreeMap::new(), &BTreeMap::new(), &mut meter, 0)?;
    Ok((value, meter.usage()))
}

#[test]
fn count_is_raw_byte_counting_with_one_complete_scan() -> Result<(), Error> {
    let (_, (base_steps, base_allocated)) = usage(b"", b'\n')?;
    for (operand, needle, want) in [
        (&b""[..], b'\n', 0),
        (&b"abc"[..], b'\n', 0),
        (&b"\n\n\n"[..], b'\n', 3),
        (&b"a\nb\r\nc"[..], b'\n', 2),
        (&b"\r"[..], b'\n', 0),
        (&b"\0a\0"[..], 0, 2),
        (&[0xff, 0x00, 0xff, 0xff][..], 0xff, 3),
        (&[0x80, b'\n', 0xc0, 0x80][..], 0x80, 2),
    ] {
        let (value, (steps, allocated)) = usage(operand, needle)?;
        assert_eq!(value, Value::Integer(want), "{operand:?} counting {needle}");
        let length = operand.len() as u64;
        assert_eq!(steps, base_steps + length, "one complete scan");
        assert_eq!(allocated, base_allocated + length);
        let wire = call(
            "core.bytes.count",
            &["Bytes<max=32>"],
            vec![bytes(operand), bytes(&[needle])],
        );
        let expr = parsed(&wire)?;
        assert_eq!(
            expression(
                &expr,
                &BTreeMap::new(),
                &BTreeMap::new(),
                &mut Meter::new(limits(steps - 1, allocated)),
                0
            ),
            Err(Error::StepBudgetExceeded)
        );
    }
    Ok(())
}

#[test]
fn count_parser_requires_one_structural_byte_needle_and_exact_signature() {
    let operand = bytes(b"a\nb");
    for (types, args) in [
        (vec!["Bytes<max=32>"], vec![operand.clone(), bytes(b"")]),
        (vec!["Bytes<max=32>"], vec![operand.clone(), bytes(b"\r\n")]),
        (vec!["Bytes<max=32>"], vec![operand.clone(), integer(10)]),
        (
            vec!["Bytes<max=32>"],
            vec![
                operand.clone(),
                reference(&local("arg.0", "Bytes<exact=1>")),
            ],
        ),
        (vec!["Bytes<max=32>"], vec![operand.clone()]),
        (
            vec!["Bytes<max=32>"],
            vec![operand.clone(), bytes(b"\n"), bytes(b"\n")],
        ),
        (vec![], vec![operand.clone(), bytes(b"\n")]),
        (vec!["U64"], vec![operand.clone(), bytes(b"\n")]),
        (
            vec!["Bytes<max=32>", "Bytes<max=32>"],
            vec![operand, bytes(b"\n")],
        ),
    ] {
        let wire = call("core.bytes.count", &types, args.clone());
        assert!(parsed(&wire).is_err(), "{types:?} {args:?}");
    }
}

fn helpers(functions: Vec<(&'static str, Value)>) -> Result<(), Error> {
    let core = map([
        ("functions", map(functions)),
        ("intents", Value::Map(vec![])),
        ("types", Value::Map(vec![])),
    ]);
    let exports = map([
        ("pureFunctions", Value::Array(vec![])),
        ("effects", Value::Array(vec![])),
    ]);
    let types = Value::Map(vec![]);
    let mut parser = Parser {
        types: &types,
        coordinate: "count.example@1",
        remaining: MAX_PROGRAM_NODES,
    };
    decode(&core, &exports, &mut parser).map(|_| ())
}
fn function(bindings: Vec<(Value, Value)>, result: Value) -> Value {
    let locals = bindings.iter().map(|(local, _)| local.clone()).collect();
    map([
        (
            "params",
            Value::Array(vec![local("arg.0", "Bytes<max=32>")]),
        ),
        ("returnType", text("U64")),
        (
            "body",
            map([
                ("locals", Value::Array(locals)),
                (
                    "bindings",
                    Value::Array(
                        bindings
                            .into_iter()
                            .map(|(binding, value)| {
                                map([
                                    ("kind", text("let")),
                                    ("binding", binding),
                                    ("value", value),
                                ])
                            })
                            .collect(),
                    ),
                ),
                ("result", result),
            ]),
        ),
    ])
}

#[test]
fn helper_addition_uses_the_count_bound_directly_and_through_let_bindings() {
    let text_bytes = reference(&local("arg.0", "Bytes<max=32>"));
    let breaks = local("local.0", "U64");
    for (addend, accepted) in [(1, true), (u64::MAX - 32, true), (u64::MAX - 31, false)] {
        let direct = function(vec![], add(count(text_bytes.clone()), integer(addend)));
        let bound = function(
            vec![(breaks.clone(), count(text_bytes.clone()))],
            add(reference(&breaks), integer(addend)),
        );
        for (shape, helper) in [("direct", direct), ("let", bound)] {
            assert_eq!(
                helpers(vec![("lines", helper)]).is_ok(),
                accepted,
                "{shape} + {addend}"
            );
        }
    }
}

#[test]
fn helper_results_do_not_carry_the_count_bound() {
    let text_bytes = reference(&local("arg.0", "Bytes<max=32>"));
    let breaks = function(vec![], count(text_bytes.clone()));
    let lines = function(
        vec![],
        add(
            call("count.example@1.breaks", &[], vec![text_bytes]),
            integer(1),
        ),
    );
    assert_eq!(
        helpers(vec![("breaks", breaks.clone()), ("lines", lines)]),
        Err(Error::UnsupportedProgram)
    );
    assert_eq!(helpers(vec![("breaks", breaks)]), Ok(()));
}
