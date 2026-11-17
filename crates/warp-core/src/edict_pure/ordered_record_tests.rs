// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! An ordered record whose fields all return builds the canonical value, and
//! charges, of the equivalent unordered record.
use super::{
    evaluate::{expression, Meter},
    model::MAX_PROGRAM_NODES,
    syntax::Parser,
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, encode_canonical_cbor_v1 as encode,
    CanonicalValueV1 as Value,
};
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
fn integer(n: i128) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", Value::Integer(n)),
            ]),
        ),
    ])
}
fn run(source: &Value) -> Result<(Value, (u64, u64)), Error> {
    let types = map([]);
    let mut parser = Parser {
        types: &types,
        coordinate: "generic.record@1",
        remaining: MAX_PROGRAM_NODES,
    };
    let expr = parser.expr(source, 0)?;
    let mut meter = Meter::new(EvaluationLimits {
        max_package_bytes: 4096,
        max_input_bytes: 4096,
        max_output_bytes: 4096,
        max_steps: 64,
        max_allocated_bytes: 4096,
    });
    let result = expression(&expr, &BTreeMap::new(), &BTreeMap::new(), &mut meter, 0)?;
    Ok((result, meter.usage()))
}

#[test]
fn a_total_ordered_record_equals_the_unordered_record() -> Result<(), Error> {
    // `bb` sorts after `z` canonically (shorter keys first), so neither
    // authored order below is the canonical one.
    let unordered = map([
        ("kind", text("record")),
        (
            "fields",
            map([("z", integer(1)), ("a", integer(2)), ("bb", integer(3))]),
        ),
    ]);
    // Core arrives canonically encoded, so the map is in canonical key order.
    let unordered = decode(&encode(&unordered).map_err(|_| Error::InvalidArtifact)?)
        .map_err(|_| Error::InvalidArtifact)?;
    let (expected, expected_usage) = run(&unordered)?;
    for names in [["bb", "z", "a"], ["z", "bb", "a"]] {
        let entries = names
            .iter()
            .map(|name| {
                let value = match *name {
                    "z" => 1,
                    "a" => 2,
                    _ => 3,
                };
                map([("name", text(name)), ("value", integer(value))])
            })
            .collect();
        let ordered = map([
            ("kind", text("orderedRecord")),
            ("fields", Value::Array(entries)),
        ]);
        let (value, usage) = run(&ordered)?;
        assert_eq!(value, expected, "{names:?}: same canonical value");
        assert_eq!(
            encode(&value).map_err(|_| Error::InvalidArtifact)?,
            encode(&expected).map_err(|_| Error::InvalidArtifact)?,
            "{names:?}: same canonical bytes"
        );
        assert_eq!(usage, expected_usage, "{names:?}: same charges");
    }
    Ok(())
}
