// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Production helper judgment for the closed unsigned-byte literal domain.
use super::{validate, Value};

fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}
fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn literal(width: &str, value: Value) -> Value {
    map([
        ("kind", text("int")),
        ("width", text(width)),
        ("value", value),
    ])
}
fn checked(value: Value, result: &str) -> Result<(), crate::ProviderRefusalV1> {
    checked_expression(
        map([("kind", text("const")), ("value", value)]),
        vec![],
        result,
    )
}
fn checked_expression(
    expression: Value,
    params: Vec<Value>,
    result: &str,
) -> Result<(), crate::ProviderRefusalV1> {
    let body = map([
        ("locals", Value::Array(vec![])),
        ("bindings", Value::Array(vec![])),
        ("result", expression),
    ]);
    let function = map([
        ("params", Value::Array(params)),
        ("returnType", text(result)),
        ("body", body),
    ]);
    let core = map([
        ("coordinate", text("generic.width@1")),
        ("types", map([])),
        ("intents", map([])),
        ("functions", map([("literal", function)])),
    ]);
    let exports = map([
        ("effects", Value::Array(vec![])),
        ("pureFunctions", Value::Array(vec![])),
    ]);
    validate(&core, &exports, false)
}

#[test]
fn source_u8_literals_accept_closed_domain_and_preserve_other_unsigned_widths() {
    for width in ["U8", "U32", "U64"] {
        for value in [0_u64, 128, 255] {
            assert!(
                checked(literal(width, Value::Integer(value.into())), width).is_ok(),
                "{width}: {value}"
            );
        }
    }
}

#[test]
fn source_u8_literals_refuse_width_coercion_and_malformed_values() {
    let mut extra = literal("U8", Value::Integer(255));
    let Value::Map(fields) = &mut extra else {
        unreachable!()
    };
    fields.push((text("extra"), Value::Bool(true)));
    for (value, result) in [
        (literal("U8", Value::Integer(256)), "U8"),
        (literal("U8", Value::Integer(-1)), "U8"),
        (literal("U8", Value::Bool(true)), "U8"),
        (literal("U8", Value::Bytes(vec![255])), "U8"),
        (extra, "U8"),
        (literal("U8", Value::Integer(255)), "U32"),
        (literal("U32", Value::Integer(255)), "U8"),
        (literal("U16", Value::Integer(255)), "U16"),
    ] {
        assert!(matches!(checked(value, result), Err(ref refusal)
            if refusal.kind == crate::ProviderRefusalKind::UnsupportedSemantics));
    }
}

#[test]
fn source_u8_literals_do_not_admit_arithmetic_outside_runtime_widths() {
    for width in ["U8", "U32", "U64"] {
        let parameter = map([
            ("id", text("param.0")),
            ("type", text(width)),
            ("alphaName", text("value")),
        ]);
        let value = map([("kind", text("local")), ("ref", parameter.clone())]);
        let call = map([
            ("kind", text("call")),
            ("callee", text("core.integer.subtract")),
            ("args", Value::Array(vec![value.clone(), value])),
            ("typeArgs", Value::Array(vec![text(width)])),
        ]);
        let outcome = checked_expression(call, vec![parameter], width);
        if width == "U8" {
            assert!(
                matches!(outcome, Err(ref refusal) if refusal.kind == crate::ProviderRefusalKind::UnsupportedSemantics)
            );
        } else {
            assert!(
                outcome.is_ok(),
                "existing {width} subtraction must remain admitted"
            );
        }
    }
}
