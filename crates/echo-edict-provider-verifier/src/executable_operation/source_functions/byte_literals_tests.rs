// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Byte constants in the production source-function judgment, including unused helpers.
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
fn fixture(literal: Value, result: &str) -> (Value, Value) {
    let body = map([
        ("locals", Value::Array(vec![])),
        ("bindings", Value::Array(vec![])),
        ("result", map([("kind", text("const")), ("value", literal)])),
    ]);
    let function = map([
        ("params", Value::Array(vec![])),
        ("returnType", text(result)),
        ("body", body),
    ]);
    let core = map([
        ("coordinate", text("generic.bytes@1")),
        ("types", map([])),
        ("intents", map([])),
        ("functions", map([("literal", function)])),
    ]);
    let exports = map([
        ("effects", Value::Array(vec![])),
        ("pureFunctions", Value::Array(vec![])),
    ]);
    (core, exports)
}
fn bytes(value: Value) -> Value {
    map([("kind", text("bytes")), ("value", value)])
}

#[test]
fn byte_literals_accept_empty_nul_and_non_utf8_octets_in_source_helpers() {
    for value in [vec![], vec![0], vec![b'\n', b'\r', 0], vec![0xff, 0x80]] {
        let result = format!("Bytes<exact={}>", value.len());
        let (core, exports) = fixture(bytes(Value::Bytes(value)), &result);
        assert!(validate(&core, &exports, false).is_ok(), "{result}");
    }
}

#[test]
fn byte_literals_refuse_wrong_payload_extra_fields_and_narrow_results() {
    let mut extra = bytes(Value::Bytes(vec![1]));
    let Value::Map(fields) = &mut extra else {
        unreachable!()
    };
    fields.push((text("width"), text("U32")));
    for (literal, result) in [
        (bytes(text("not raw octets")), "Bytes<max=32>"),
        (extra, "Bytes<max=1>"),
        (bytes(Value::Bytes(vec![1, 2])), "Bytes<max=1>"),
    ] {
        let (core, exports) = fixture(literal, result);
        let refusal = validate(&core, &exports, false).expect_err("invalid byte literal");
        assert_eq!(
            refusal.kind,
            crate::ProviderRefusalKind::UnsupportedSemantics
        );
    }
}
