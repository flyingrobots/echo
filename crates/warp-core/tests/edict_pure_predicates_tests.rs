// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A real public compiler/verifier package with nested conjunctive guards.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::unwrap_used, clippy::panic)]
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use warp_core::edict_pure::{evaluate, EvaluationError, EvaluationLimits};
const PACKAGE: &str =
    include_str!("fixtures/edict-pure-predicates/executable-operation-package.cbor.hex");
const REPORT: &str = include_str!("fixtures/edict-pure-predicates/verification-report.cbor.hex");
const PIN: &str = "8dfbc0e76c2bce33b604832a6b8f4d94d6791c2863e4b1235c24d836baba72d0";
fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Map(fields) = value else {
        panic!("record")
    };
    &fields
        .iter()
        .find(|(key, _)| *key == Value::Text(name.into()))
        .unwrap()
        .1
}
fn input(start: u64, end: u64) -> Vec<u8> {
    input_with_replacement(start, end, "λ\r\n".as_bytes())
}
fn input_with_replacement(start: u64, end: u64, replacement: &[u8]) -> Vec<u8> {
    input_bytes_with_replacement(b"0123456789abcdef", start, end, replacement)
}
fn input_bytes_with_replacement(
    original: &[u8],
    start: u64,
    end: u64,
    replacement: &[u8],
) -> Vec<u8> {
    encode(&Value::Map(
        [
            ("bufferId", Value::Bytes(vec![1; 32])),
            ("basisHeadId", Value::Bytes(vec![2; 32])),
            ("text", Value::Bytes(original.to_vec())),
            ("replacement", Value::Bytes(replacement.to_vec())),
            ("startByte", Value::Integer(start.into())),
            ("endByte", Value::Integer(end.into())),
        ]
        .into_iter()
        .map(|(key, value)| (Value::Text(key.into()), value))
        .collect(),
    ))
    .unwrap()
}
fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 32768,
        max_input_bytes: 4194304,
        max_steps: 10000,
        max_allocated_bytes: 16777216,
        max_output_bytes: 4194304,
    }
}
#[test]
fn public_compiler_conjunction_executes_and_rejects_each_failed_range_guard() {
    let bytes = hex::decode(PACKAGE.trim()).unwrap();
    let pin: [u8; 32] = hex::decode(PIN).unwrap().try_into().unwrap();
    assert_eq!(
        digest("echo.operation-package/v1", &decode(&bytes).unwrap()).unwrap(),
        pin
    );
    let report = decode(&hex::decode(REPORT.trim()).unwrap()).unwrap();
    assert_eq!(field(&report, "outcome"), &Value::Text("accepted".into()));
    assert_eq!(
        field(field(&report, "package"), "digest"),
        &Value::Array(vec![
            Value::Text("sha256".into()),
            Value::Bytes(pin.to_vec())
        ])
    );
    for (start, end, expected) in [
        (7, 7, "0123456λ\r\n789abcdef"),
        (7, 11, "0123456λ\r\nbcdef"),
    ] {
        let supplied = input(start, end);
        let result = evaluate(&bytes, pin, &supplied, limits()).unwrap();
        assert_eq!(
            field(&decode(&result.output).unwrap(), "updatedText"),
            &Value::Bytes(expected.as_bytes().to_vec())
        );
        assert_eq!(result, evaluate(&bytes, pin, &supplied, limits()).unwrap());
    }
    for (start, end) in [(11, 7), (7, 17), (17, 17), (0, u64::MAX)] {
        assert_eq!(
            evaluate(&bytes, pin, &input(start, end), limits()),
            Err(EvaluationError::InputConstraintFailed("where.0".into()))
        );
    }
}

#[test]
fn public_compiler_negation_rejects_unchanged_text_after_checked_range() {
    let bytes = hex::decode(
        include_str!("fixtures/edict-pure-negation/executable-operation-package.cbor.hex").trim(),
    )
    .unwrap();
    let pin: [u8; 32] =
        hex::decode("54ed142d19a230398da34a4754d3650842943401338678579833d725da785dd9")
            .unwrap()
            .try_into()
            .unwrap();
    assert_eq!(
        digest("echo.operation-package/v1", &decode(&bytes).unwrap()).unwrap(),
        pin
    );
    let report = decode(
        &hex::decode(
            include_str!("fixtures/edict-pure-negation/verification-report.cbor.hex").trim(),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(field(&report, "outcome"), &Value::Text("accepted".into()));
    assert_eq!(
        field(field(&report, "package"), "digest"),
        &Value::Array(vec![
            Value::Text("sha256".into()),
            Value::Bytes(pin.to_vec())
        ])
    );
    for (start, end, replacement, expected) in [
        (7, 11, &b"X"[..], &b"0123456Xbcdef"[..]),
        (7, 11, &b""[..], &b"0123456bcdef"[..]),
        (0, 0, &b"A"[..], &b"A0123456789abcdef"[..]),
    ] {
        let supplied = input_with_replacement(start, end, replacement);
        let result = evaluate(&bytes, pin, &supplied, limits()).unwrap();
        assert_eq!(
            field(&decode(&result.output).unwrap(), "updatedText"),
            &Value::Bytes(expected.to_vec())
        );
        assert_eq!(result, evaluate(&bytes, pin, &supplied, limits()).unwrap());
    }
    for (start, end, replacement) in [
        (7, 11, &b"789a"[..]),
        (0, 0, &b""[..]),
        (16, 16, &b""[..]),
        (0, 16, &b"0123456789abcdef"[..]),
        (11, 7, &b"X"[..]),
        (7, 17, &b"X"[..]),
    ] {
        assert_eq!(
            evaluate(
                &bytes,
                pin,
                &input_with_replacement(start, end, replacement),
                limits()
            ),
            Err(EvaluationError::InputConstraintFailed("where.0".into()))
        );
    }
}

#[path = "edict_pure_predicates/utf8.rs"]
mod utf8;
