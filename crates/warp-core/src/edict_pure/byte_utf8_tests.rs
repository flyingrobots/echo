// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! UTF-8 truth, Boolean values and deterministic full-aperture work.
use super::{
    evaluate::{expression, validate, Meter},
    functions::check_roots,
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
fn constant(kind: &str, value: Value) -> Value {
    map([
        ("kind", Value::Text("const".into())),
        (
            "value",
            map([("kind", Value::Text(kind.into())), ("value", value)]),
        ),
    ])
}
fn call(bytes: &[u8]) -> Value {
    map([
        ("kind", Value::Text("call".into())),
        ("callee", Value::Text("core.bytes.isUtf8".into())),
        (
            "typeArgs",
            Value::Array(vec![Value::Text("Bytes<max=32>".into())]),
        ),
        (
            "args",
            Value::Array(vec![constant("bytes", Value::Bytes(bytes.to_vec()))]),
        ),
    ])
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
#[test]
fn utf8_validity_and_metering_cover_complete_valid_and_invalid_sequences() -> Result<(), Error> {
    for (bytes, want) in [
        (&b""[..], true),
        (&b"A\0B"[..], true),
        ("λ🙂".as_bytes(), true),
        (&[0xf4, 0x8f, 0xbf, 0xbf][..], true),
        (&[0x80][..], false),
        (&[0xc0, 0x80][..], false),
        (&[0xe2, 0x82][..], false),
        (&[0xed, 0xa0, 0x80][..], false),
        (&[0xf4, 0x90, 0x80, 0x80][..], false),
        (&[0xff, b'A', b'B', b'C'][..], false),
    ] {
        let types = Value::Map(vec![]);
        let mut parser = Parser {
            types: &types,
            coordinate: "utf8.example@1",
            remaining: MAX_PROGRAM_NODES,
        };
        let expr = parser.expr(&call(bytes), 0)?;
        check_roots(&BTreeMap::new(), [&expr], [])?;
        let steps = 5 + bytes.len() as u64;
        let allocated = 128 + bytes.len() as u64;
        let mut meter = Meter::new(limits(steps, allocated));
        assert_eq!(
            expression(&expr, &BTreeMap::new(), &BTreeMap::new(), &mut meter, 0)?,
            Value::Bool(want)
        );
        assert_eq!(meter.usage(), (steps, allocated));
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
        assert_eq!(
            expression(
                &expr,
                &BTreeMap::new(),
                &BTreeMap::new(),
                &mut Meter::new(limits(steps, allocated - 1)),
                0
            ),
            Err(Error::AllocationBudgetExceeded)
        );
    }
    Ok(())
}
#[test]
fn boolean_values_are_typed_and_integer_substitution_is_refused() -> Result<(), Error> {
    let types = Value::Map(vec![]);
    let mut parser = Parser {
        types: &types,
        coordinate: "utf8.example@1",
        remaining: MAX_PROGRAM_NODES,
    };
    let ty = parser.ty("Bool", 0)?;
    for expected in [false, true] {
        let expr = parser.expr(&constant("bool", Value::Bool(expected)), 0)?;
        let value = expression(
            &expr,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &mut Meter::new(limits(10, 256)),
            0,
        )?;
        assert_eq!(value, Value::Bool(expected));
        validate(&value, &ty, &mut Meter::new(limits(10, 256)), 0)?;
    }
    assert_eq!(
        validate(&Value::Integer(1), &ty, &mut Meter::new(limits(10, 256)), 0),
        Err(Error::InvalidArtifact)
    );
    assert_eq!(
        parser.expr(&constant("bool", Value::Integer(1)), 0),
        Err(Error::InvalidArtifact)
    );
    Ok(())
}
