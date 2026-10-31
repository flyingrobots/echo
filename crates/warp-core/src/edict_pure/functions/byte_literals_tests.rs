// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime helper admission, separate from provider and expression-parser checks.
use super::*;

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}

fn checked(literal: Value, result_type: &str) -> Result<BTreeMap<String, Helper>, Error> {
    let body = record([
        ("locals", Value::Array(vec![])),
        ("bindings", Value::Array(vec![])),
        (
            "result",
            record([("kind", Value::Text("const".into())), ("value", literal)]),
        ),
    ]);
    let core = record([
        ("coordinate", Value::Text("consumer.bytes@1".into())),
        ("types", record([])),
        ("intents", record([])),
        (
            "functions",
            record([(
                "literal",
                record([
                    ("params", Value::Array(vec![])),
                    ("returnType", Value::Text(result_type.into())),
                    ("body", body),
                ]),
            )]),
        ),
    ]);
    let exports = record([
        ("effects", Value::Array(vec![])),
        ("pureFunctions", Value::Array(vec![])),
    ]);
    let mut parser = Parser {
        types: field(&core, "types")?,
        coordinate: "consumer.bytes@1",
        remaining: MAX_PROGRAM_NODES,
    };
    decode(&core, &exports, &mut parser)
}

fn literal(value: Value) -> Value {
    record([("kind", Value::Text("bytes".into())), ("value", value)])
}

#[test]
fn source_byte_literals_admit_exact_empty_and_non_utf8_types_in_unused_helpers() {
    for value in [vec![], vec![0], vec![0, 255, 10, 13]] {
        let expected = RuntimeType::Bytes {
            min: value.len() as u64,
            max: value.len() as u64,
        };
        let helpers = checked(
            literal(Value::Bytes(value.clone())),
            &format!("Bytes<exact={}>", value.len()),
        );
        assert!(
            matches!(helpers, Ok(ref helpers) if helpers["consumer.bytes@1.literal"].ty == expected)
        );
    }
}

#[test]
fn source_byte_literals_refuse_nonbytes_extra_fields_and_narrow_returns() {
    let mut extra = literal(Value::Bytes(vec![0]));
    let Value::Map(fields) = &mut extra else {
        unreachable!()
    };
    fields.push((Value::Text("width".into()), Value::Text("U32".into())));
    for (value, result_type) in [
        (
            literal(Value::Text("not raw bytes".into())),
            "Bytes<max=32>",
        ),
        (extra, "Bytes<max=1>"),
        (literal(Value::Bytes(vec![0, 255])), "Bytes<max=1>"),
    ] {
        assert!(matches!(
            checked(value, result_type),
            Err(Error::InvalidArtifact)
        ));
    }
}
