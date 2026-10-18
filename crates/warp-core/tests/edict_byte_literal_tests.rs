// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Exact externally compiled bytes, including octets that are not UTF-8.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::unwrap_used, clippy::panic)]

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use warp_core::edict_pure::{evaluate, EvaluationError, EvaluationLimits};

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    let Value::Map(fields) = value else {
        panic!("record")
    };
    &fields
        .iter()
        .find(|(name, _)| name == &Value::Text(key.into()))
        .unwrap()
        .1
}

fn fixture() -> (Vec<u8>, [u8; 32]) {
    let package = hex::decode(
        include_str!("fixtures/edict-byte-literals/executable-operation-package.cbor.hex").trim(),
    )
    .unwrap();
    let report = decode(
        &hex::decode(
            include_str!("fixtures/edict-byte-literals/verification-report.cbor.hex").trim(),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(field(&report, "outcome"), &Value::Text("accepted".into()));
    let pin = digest("echo.operation-package/v1", &decode(&package).unwrap()).unwrap();
    assert_eq!(
        field(field(&report, "package"), "digest"),
        &Value::Array(vec![
            Value::Text("sha256".into()),
            Value::Bytes(pin.to_vec())
        ])
    );
    (package, pin)
}

fn input() -> Vec<u8> {
    encode(&record([
        ("bufferId", Value::Bytes(vec![1; 32])),
        ("basisHeadId", Value::Bytes(vec![2; 32])),
        ("startByte", Value::Integer(7)),
        ("endByte", Value::Integer(11)),
        ("replacement", Value::Bytes("λ\r\n".as_bytes().to_vec())),
    ]))
    .unwrap()
}

fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 32768,
        max_input_bytes: 2097152,
        max_steps: 10000,
        max_allocated_bytes: 16777216,
        max_output_bytes: 2097152,
    }
}

#[test]
fn compiler_produced_byte_constant_preserves_binary_octets() {
    let (package, pin) = fixture();
    let result = evaluate(&package, pin, &input(), limits()).unwrap();
    let output = decode(&result.output).unwrap();
    assert_eq!(field(&output, "literal"), &Value::Bytes(vec![13, 0, 255]));
    assert_eq!(
        field(&output, "replacement"),
        &Value::Bytes("λ\r\n".as_bytes().to_vec())
    );
    assert_eq!(result, evaluate(&package, pin, &input(), limits()).unwrap());
}

#[test]
fn byte_constant_obeys_exact_evaluation_budgets() {
    let (package, pin) = fixture();
    let result = evaluate(&package, pin, &input(), limits()).unwrap();
    let exact = EvaluationLimits {
        max_steps: result.steps,
        max_allocated_bytes: result.allocated_bytes,
        max_output_bytes: result.output.len() as u64,
        ..limits()
    };
    assert_eq!(result, evaluate(&package, pin, &input(), exact).unwrap());
    for (bound, error) in [
        (
            EvaluationLimits {
                max_steps: exact.max_steps - 1,
                ..exact
            },
            EvaluationError::StepBudgetExceeded,
        ),
        (
            EvaluationLimits {
                max_allocated_bytes: exact.max_allocated_bytes - 1,
                ..exact
            },
            EvaluationError::AllocationBudgetExceeded,
        ),
        (
            EvaluationLimits {
                max_output_bytes: exact.max_output_bytes - 1,
                ..exact
            },
            EvaluationError::OutputBudgetExceeded,
        ),
    ] {
        assert_eq!(evaluate(&package, pin, &input(), bound), Err(error));
    }
}
