// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Execute the exact externally compiled application and falsify its boundaries.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::unwrap_used, clippy::panic)]

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use warp_core::edict_pure::{evaluate, EvaluationError, EvaluationLimits};

const PACKAGE_HEX: &str =
    include_str!("fixtures/edict-pure-jedit/executable-operation-package.cbor.hex");
const REPORT_HEX: &str = include_str!("fixtures/edict-pure-jedit/verification-report.cbor.hex");
const PACKAGE_DIGEST: &str = "b9052d3c40878a9fcba7768be67c8dba50cf0f751682339dfcdd04155871415b";

fn package() -> Vec<u8> {
    hex::decode(PACKAGE_HEX.trim()).unwrap()
}

fn pin() -> [u8; 32] {
    hex::decode(PACKAGE_DIGEST).unwrap().try_into().unwrap()
}

fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 32_768,
        max_input_bytes: 2_097_152,
        max_steps: 10_000,
        max_allocated_bytes: 16_777_216,
        max_output_bytes: 2_097_152,
    }
}

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}

fn input(start: u64, end: u64) -> Value {
    record([
        ("bufferId", Value::Bytes(vec![1; 32])),
        ("basisHeadId", Value::Bytes(vec![2; 32])),
        ("startByte", Value::Integer(start.into())),
        ("endByte", Value::Integer(end.into())),
        ("replacement", Value::Bytes("λ\r\n".as_bytes().to_vec())),
    ])
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    let Value::Map(fields) = value else {
        panic!("expected record")
    };
    &fields
        .iter()
        .find(|(name, _)| name == &Value::Text(key.into()))
        .unwrap()
        .1
}

fn set_field(value: &mut Value, key: &str, replacement: Value) {
    let Value::Map(fields) = value else {
        panic!("expected record")
    };
    fields
        .iter_mut()
        .find(|(name, _)| name == &Value::Text(key.into()))
        .unwrap()
        .1 = replacement;
}

#[test]
fn exact_jedit_compiler_package_executes_both_authored_branches_and_imported_helper() {
    // A constant stub, a reversed predicate, or skipped helper body breaks this witness.
    let package = package();
    assert_eq!(
        digest("echo.operation-package/v1", &decode(&package).unwrap()).unwrap(),
        pin()
    );
    let report = decode(&hex::decode(REPORT_HEX.trim()).unwrap()).unwrap();
    assert_eq!(field(&report, "outcome"), &Value::Text("accepted".into()));
    assert_eq!(
        field(field(&report, "package"), "digest"),
        &Value::Array(vec![
            Value::Text("sha256".into()),
            Value::Bytes(pin().to_vec())
        ])
    );
    for (start, end, empty) in [(7, 7, 1), (7, 11, 0)] {
        let supplied = encode(&input(start, end)).unwrap();
        let result = evaluate(&package, pin(), &supplied, limits()).unwrap();
        let expected = record([
            ("bufferId", Value::Bytes(vec![1; 32])),
            ("basisHeadId", Value::Bytes(vec![2; 32])),
            ("startByte", Value::Integer(start.into())),
            ("endByte", Value::Integer(end.into())),
            ("replacement", Value::Bytes("λ\r\n".as_bytes().to_vec())),
            ("rangeIsEmpty", Value::Integer(empty)),
            ("createdLeafCeiling", Value::Integer(4096)),
        ]);
        assert_eq!(result.output, encode(&expected).unwrap());
        assert_eq!(
            result,
            evaluate(&package, pin(), &supplied, limits()).unwrap()
        );
        assert!(result.steps > 0);
        assert!(result.allocated_bytes > 0);
    }
}

#[test]
fn authored_input_constraint_prevents_evaluation_of_reversed_range() {
    assert_eq!(
        evaluate(&package(), pin(), &encode(&input(11, 7)).unwrap(), limits()),
        Err(EvaluationError::InputConstraintFailed("where.0".into()))
    );
}

#[test]
fn runtime_validates_exact_nominal_representation_integer_and_record_bounds() {
    for (name, value) in [
        ("bufferId", Value::Bytes(vec![1; 31])),
        ("basisHeadId", Value::Bytes(vec![2; 33])),
        ("startByte", Value::Integer(-1)),
        ("endByte", Value::Text("11".into())),
        ("replacement", Value::Bytes(vec![0; 1_048_577])),
    ] {
        let mut invalid = input(7, 11);
        set_field(&mut invalid, name, value);
        assert_eq!(
            evaluate(&package(), pin(), &encode(&invalid).unwrap(), limits()),
            Err(EvaluationError::InvalidInput),
            "field {name}"
        );
    }
    let mut extra = input(7, 11);
    let Value::Map(fields) = &mut extra else {
        unreachable!()
    };
    fields.push((Value::Text("extra".into()), Value::Null));
    assert_eq!(
        evaluate(&package(), pin(), &encode(&extra).unwrap(), limits()),
        Err(EvaluationError::InvalidInput)
    );
}

#[test]
fn package_substitution_cannot_execute_under_the_verified_pin() {
    let mut substituted = decode(&package()).unwrap();
    set_field(
        &mut substituted,
        "operation_coordinate",
        Value::Text("other@1.operation".into()),
    );
    assert_eq!(
        evaluate(
            &encode(&substituted).unwrap(),
            pin(),
            &encode(&input(0, 0)).unwrap(),
            limits()
        ),
        Err(EvaluationError::PackageIdentityMismatch)
    );
}

#[test]
fn host_limits_bound_decode_execution_allocation_and_output() {
    let input = encode(&input(0, 0)).unwrap();
    let mut cases = Vec::new();
    let mut bound = limits();
    bound.max_package_bytes = 1;
    cases.push((bound, EvaluationError::PackageTooLarge));
    let mut bound = limits();
    bound.max_input_bytes = 1;
    cases.push((bound, EvaluationError::InputTooLarge));
    let mut bound = limits();
    bound.max_steps = 1;
    cases.push((bound, EvaluationError::StepBudgetExceeded));
    let mut bound = limits();
    bound.max_allocated_bytes = 1;
    cases.push((bound, EvaluationError::AllocationBudgetExceeded));
    let mut bound = limits();
    bound.max_output_bytes = 1;
    cases.push((bound, EvaluationError::OutputBudgetExceeded));
    for (bound, expected) in cases {
        assert_eq!(evaluate(&package(), pin(), &input, bound), Err(expected));
    }
}

#[test]
fn public_compiler_source_mutation_changes_runtime_behavior() {
    // This is newly compiled source, not a patched executable artifact.
    let mutated = hex::decode(
        include_str!("fixtures/edict-pure-jedit/mutated-executable-operation-package.cbor.hex")
            .trim(),
    )
    .unwrap();
    let mutated_pin: [u8; 32] =
        hex::decode("1af8c0d9a872b46855138b36b49d10bf70e3b576d403c76b09953af079495f62")
            .unwrap()
            .try_into()
            .unwrap();
    assert_ne!(mutated_pin, pin());
    let report = decode(
        &hex::decode(
            include_str!("fixtures/edict-pure-jedit/mutated-verification-report.cbor.hex").trim(),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(field(&report, "outcome"), &Value::Text("accepted".into()));
    assert_eq!(
        field(field(&report, "package"), "digest"),
        &Value::Array(vec![
            Value::Text("sha256".into()),
            Value::Bytes(mutated_pin.to_vec())
        ])
    );
    for (end, wanted) in [(7, 2), (11, 0)] {
        let output = evaluate(
            &mutated,
            mutated_pin,
            &encode(&input(7, end)).unwrap(),
            limits(),
        )
        .unwrap();
        assert_eq!(
            field(&decode(&output.output).unwrap(), "rangeIsEmpty"),
            &Value::Integer(wanted)
        );
    }
    assert_eq!(
        evaluate(&mutated, pin(), &encode(&input(7, 7)).unwrap(), limits()),
        Err(EvaluationError::PackageIdentityMismatch)
    );
}

#[test]
fn exact_budget_boundaries_and_noncanonical_input_are_enforced() {
    let input = encode(&input(7, 11)).unwrap();
    let result = evaluate(&package(), pin(), &input, limits()).unwrap();
    let exact = EvaluationLimits {
        max_steps: result.steps,
        max_allocated_bytes: result.allocated_bytes,
        max_output_bytes: result.output.len() as u64,
        ..limits()
    };
    assert_eq!(evaluate(&package(), pin(), &input, exact).unwrap(), result);
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
        assert_eq!(evaluate(&package(), pin(), &input, bound), Err(error));
    }
    let mut trailing = input;
    trailing.push(0);
    assert_eq!(
        evaluate(&package(), pin(), &trailing, limits()),
        Err(EvaluationError::InvalidInput)
    );
}
