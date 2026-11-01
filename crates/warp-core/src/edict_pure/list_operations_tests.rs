// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Concrete immutable list results, strict runtime checks and portable charges.
use super::{
    evaluate::{expression, Meter},
    model::MAX_PROGRAM_NODES,
    syntax::Parser,
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}
fn integer(width: &str, n: i128) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text(width)),
                ("value", Value::Integer(n)),
            ]),
        ),
    ])
}
fn list(values: Vec<Value>) -> Value {
    map([("kind", text("list")), ("values", Value::Array(values))])
}
fn octets(values: &[u8]) -> Value {
    list(values.iter().map(|n| integer("U8", (*n).into())).collect())
}
fn call(name: &str, types: &[&str], arguments: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(name)),
        (
            "typeArgs",
            Value::Array(types.iter().map(|s| text(s)).collect()),
        ),
        ("args", Value::Array(arguments)),
    ])
}
fn run(source: &Value, steps: u64, bytes: u64) -> Result<(Value, (u64, u64)), Error> {
    let types = map([]);
    let mut parser = Parser {
        types: &types,
        coordinate: "generic.worklist@1",
        remaining: MAX_PROGRAM_NODES,
    };
    let expr = parser.expr(source, 0)?;
    let mut meter = Meter::new(EvaluationLimits {
        max_package_bytes: 4096,
        max_input_bytes: 4096,
        max_output_bytes: 4096,
        max_steps: steps,
        max_allocated_bytes: bytes,
    });
    let result = expression(&expr, &BTreeMap::new(), &BTreeMap::new(), &mut meter, 0)?;
    Ok((result, meter.usage()))
}

#[test]
fn list_operations_execute_exact_results_and_resource_limits() -> Result<(), Error> {
    let expected =
        |ns: &[u8]| Value::Array(ns.iter().map(|n| Value::Integer((*n).into())).collect());
    for (source, want, steps, bytes) in [
        (
            call("core.list.length", &["List<U8,max=0>"], vec![octets(&[])]),
            Value::Integer(0),
            4,
            128,
        ),
        (
            call(
                "core.list.length",
                &["List<U8,max=2>"],
                vec![octets(&[0, 255])],
            ),
            Value::Integer(2),
            10,
            256,
        ),
        (
            call(
                "core.list.at",
                &["List<U8,max=2>"],
                vec![octets(&[0, 255]), integer("U64", 0)],
            ),
            Value::Integer(0),
            13,
            320,
        ),
        (
            call(
                "core.list.at",
                &["List<U8,max=2>"],
                vec![octets(&[0, 255]), integer("U64", 1)],
            ),
            Value::Integer(255),
            13,
            320,
        ),
        (
            call(
                "core.list.append",
                &["List<U8,max=2>"],
                vec![octets(&[1]), integer("U8", 7)],
            ),
            expected(&[1, 7]),
            12,
            384,
        ),
        (
            call(
                "core.list.concat",
                &["List<U8,max=1>", "List<U8,max=2>"],
                vec![octets(&[1]), octets(&[2, 3])],
            ),
            expected(&[1, 2, 3]),
            18,
            576,
        ),
    ] {
        // Input expression evaluation + each declared operand validation +
        // one complete result copy, using fixed 64-byte cells.
        assert_eq!(run(&source, steps, bytes)?, (want, (steps, bytes)));
        assert_eq!(
            run(&source, steps - 1, bytes),
            Err(Error::StepBudgetExceeded)
        );
        assert_eq!(
            run(&source, steps, bytes - 1),
            Err(Error::AllocationBudgetExceeded)
        );
    }
    Ok(())
}

#[test]
fn list_operations_reject_invalid_indices_full_append_and_bad_elements() {
    for source in [
        call(
            "core.list.at",
            &["List<U8,max=2>"],
            vec![octets(&[]), integer("U64", 0)],
        ),
        call(
            "core.list.at",
            &["List<U8,max=2>"],
            vec![octets(&[0, 1]), integer("U64", 2)],
        ),
        call(
            "core.list.append",
            &["List<U8,max=2>"],
            vec![octets(&[0, 1]), integer("U8", 2)],
        ),
        call(
            "core.list.append",
            &["List<U8,max=2>"],
            vec![octets(&[0]), integer("U64", 256)],
        ),
        call(
            "core.list.length",
            &["List<U8,max=1>"],
            vec![octets(&[0, 1])],
        ),
        call(
            "core.list.concat",
            &["List<U8,max=18446744073709551615>", "List<U8,max=1>"],
            vec![octets(&[]), octets(&[])],
        ),
    ] {
        assert_eq!(run(&source, 1024, 8192), Err(Error::InvalidArtifact));
    }
}

#[test]
fn list_operations_charge_nested_selected_payload_storage() -> Result<(), Error> {
    let bytes = map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("bytes")),
                ("value", Value::Bytes(b"abc".to_vec())),
            ]),
        ),
    ]);
    let source = call(
        "core.list.at",
        &["List<List<Bytes<max=3>,max=1>,max=1>"],
        vec![list(vec![list(vec![bytes])]), integer("U64", 0)],
    );
    assert_eq!(
        run(&source, 13, 390)?,
        (Value::Array(vec![Value::Bytes(b"abc".to_vec())]), (13, 390))
    );
    assert_eq!(run(&source, 13, 389), Err(Error::AllocationBudgetExceeded));
    Ok(())
}
