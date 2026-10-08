// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Exact external compiler packages drive generic byte-comparison evidence.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::unwrap_used, clippy::panic)]

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use warp_core::edict_pure::{evaluate, EvaluationError, EvaluationLimits};

const HEAD_PACKAGE: &str =
    include_str!("fixtures/edict-byte-equality/head-executable-operation-package.cbor.hex");
const HEAD_REPORT: &str =
    include_str!("fixtures/edict-byte-equality/head-verification-report.cbor.hex");
const HEAD_PIN: &str = "3ac6f90290fe22683f8f4043761e5f0c409eeb51dd8ebd6a9025b2d7c28cdb96";
const PAYLOAD_PACKAGE: &str =
    include_str!("fixtures/edict-byte-equality/payload-executable-operation-package.cbor.hex");
const PAYLOAD_REPORT: &str =
    include_str!("fixtures/edict-byte-equality/payload-verification-report.cbor.hex");
const PAYLOAD_PIN: &str = "4f850549859ef9eda931c10cd084415a7ecd4606c4e1d9a702660204d9e0d196";

fn pin(value: &str) -> [u8; 32] {
    hex::decode(value).unwrap().try_into().unwrap()
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

fn input(head: bool, left: &[u8], right: &[u8]) -> Vec<u8> {
    let (first, second) = if head {
        ("basisHeadId", "observedHeadId")
    } else {
        ("basisPayload", "observedPayload")
    };
    encode(&record([
        (first, Value::Bytes(left.into())),
        (second, Value::Bytes(right.into())),
    ]))
    .unwrap()
}

fn expected(head: bool, same: bool) -> Vec<u8> {
    encode(&record([(
        if head { "sameHead" } else { "samePayload" },
        Value::Integer(i128::from(same)),
    )]))
    .unwrap()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    let Value::Map(fields) = value else {
        panic!("map")
    };
    &fields
        .iter()
        .find(|(name, _)| name == &Value::Text(key.into()))
        .unwrap()
        .1
}

fn field_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Value {
    let Value::Map(fields) = value else {
        panic!("map")
    };
    &mut fields
        .iter_mut()
        .find(|(name, _)| name == &Value::Text(key.into()))
        .unwrap()
        .1
}

#[test]
fn external_packages_have_independent_accepted_reports_and_exact_pins() {
    for (package, report, expected_pin) in [
        (HEAD_PACKAGE, HEAD_REPORT, HEAD_PIN),
        (PAYLOAD_PACKAGE, PAYLOAD_REPORT, PAYLOAD_PIN),
    ] {
        let package = decode(&hex::decode(package.trim()).unwrap()).unwrap();
        assert_eq!(
            digest("echo.operation-package/v1", &package).unwrap(),
            pin(expected_pin)
        );
        let report = decode(&hex::decode(report.trim()).unwrap()).unwrap();
        assert_eq!(field(&report, "outcome"), &Value::Text("accepted".into()));
        assert_eq!(
            field(field(&report, "package"), "digest"),
            &Value::Array(vec![
                Value::Text("sha256".into()),
                Value::Bytes(pin(expected_pin).into())
            ])
        );
    }
}

#[test]
fn compiled_nominal_identity_equality_distinguishes_every_byte() {
    let package = hex::decode(HEAD_PACKAGE.trim()).unwrap();
    let left: Vec<u8> = (0..32).collect();
    let same = evaluate(
        &package,
        pin(HEAD_PIN),
        &input(true, &left, &left),
        limits(),
    )
    .unwrap();
    assert_eq!(same.output, expected(true, true));
    for index in 0..32 {
        let mut right = left.clone();
        right[index] ^= 0xff;
        let supplied = input(true, &left, &right);
        let result = evaluate(&package, pin(HEAD_PIN), &supplied, limits()).unwrap();
        assert_eq!(result.output, expected(true, false), "byte {index}");
        assert_eq!(
            result.steps, same.steps,
            "cost cannot depend on mismatch position"
        );
        assert_eq!(result.allocated_bytes, same.allocated_bytes);
        assert_eq!(
            result,
            evaluate(&package, pin(HEAD_PIN), &supplied, limits()).unwrap()
        );
    }
}

#[test]
fn compiled_variable_byte_equality_meters_full_operand_aperture() {
    let package = hex::decode(PAYLOAD_PACKAGE.trim()).unwrap();
    let empty = evaluate(
        &package,
        pin(PAYLOAD_PIN),
        &input(false, &[], &[]),
        limits(),
    )
    .unwrap();
    assert_eq!(empty.output, expected(false, true));
    for length in [1, 2, 31, 32, 255, 1024] {
        let left = vec![0xff; length];
        for right in [
            left.clone(),
            vec![0; length],
            vec![0xff; length - 1],
            vec![0xff; length + 1],
        ] {
            let supplied = input(false, &left, &right);
            let result = evaluate(&package, pin(PAYLOAD_PIN), &supplied, limits()).unwrap();
            assert_eq!(result.output, expected(false, left == right));
            assert_eq!(
                result.steps,
                empty.steps + u64::try_from(length.max(right.len())).unwrap()
            );
            let exact = EvaluationLimits {
                max_steps: result.steps,
                ..limits()
            };
            assert_eq!(
                result,
                evaluate(&package, pin(PAYLOAD_PIN), &supplied, exact).unwrap()
            );
            assert_eq!(
                evaluate(
                    &package,
                    pin(PAYLOAD_PIN),
                    &supplied,
                    EvaluationLimits {
                        max_steps: result.steps - 1,
                        ..limits()
                    }
                ),
                Err(EvaluationError::StepBudgetExceeded)
            );
        }
    }
}

#[test]
fn identity_input_bounds_and_package_pin_remain_enforced() {
    let package = hex::decode(HEAD_PACKAGE.trim()).unwrap();
    for length in [0, 31, 33] {
        assert_eq!(
            evaluate(
                &package,
                pin(HEAD_PIN),
                &input(true, &vec![0; length], &[0; 32]),
                limits()
            ),
            Err(EvaluationError::InvalidInput)
        );
        assert_eq!(
            evaluate(
                &package,
                pin(HEAD_PIN),
                &input(true, &[0; 32], &vec![0; length]),
                limits()
            ),
            Err(EvaluationError::InvalidInput)
        );
    }
    let invalid = encode(&record([
        ("basisHeadId", Value::Integer(1)),
        ("observedHeadId", Value::Bytes(vec![0; 32])),
    ]))
    .unwrap();
    assert_eq!(
        evaluate(&package, pin(HEAD_PIN), &invalid, limits()),
        Err(EvaluationError::InvalidInput)
    );
    assert_eq!(
        evaluate(
            &package,
            [0; 32],
            &input(true, &[0; 32], &[0; 32]),
            limits()
        ),
        Err(EvaluationError::PackageIdentityMismatch)
    );
}

#[test]
fn forged_byte_ordering_and_mixed_operand_predicates_refuse() {
    // Deliberately self-pinned malformed test packages are not compiler/verifier evidence.
    for mixed in [false, true] {
        let mut package = decode(&hex::decode(HEAD_PACKAGE.trim()).unwrap()).unwrap();
        let Value::Bytes(program_bytes) = field(&package, "program") else {
            panic!("program bytes")
        };
        let mut program = decode(program_bytes).unwrap();
        let Value::Bytes(target_bytes) = field(&program, "target_ir_artifact") else {
            panic!("target bytes")
        };
        let mut target = decode(target_bytes).unwrap();
        let intent = field_mut(field_mut(&mut target, "intents"), "replaceRange");
        let Value::Array(bindings) = field_mut(intent, "pureBindings") else {
            panic!("bindings")
        };
        let predicate = field_mut(field_mut(&mut bindings[0], "value"), "predicate");
        if mixed {
            *field_mut(predicate, "right") = record([
                ("kind", Value::Text("const".into())),
                (
                    "value",
                    record([
                        ("kind", Value::Text("int".into())),
                        ("width", Value::Text("U64".into())),
                        ("value", Value::Integer(0)),
                    ]),
                ),
            ]);
        } else {
            *field_mut(predicate, "op") = Value::Text("<=".into());
        }
        *field_mut(&mut program, "target_ir_artifact") = Value::Bytes(encode(&target).unwrap());
        *field_mut(&mut package, "program") = Value::Bytes(encode(&program).unwrap());
        *field_mut(
            field_mut(&mut package, "semantic_closure"),
            "target_ir_identity",
        ) = Value::Bytes(
            digest("edict.target-ir.artifact/v1", &target)
                .unwrap()
                .into(),
        );
        let forged_pin = digest("echo.operation-package/v1", &package).unwrap();
        assert_eq!(
            evaluate(
                &encode(&package).unwrap(),
                forged_pin,
                &input(true, &[0; 32], &[0; 32]),
                limits()
            ),
            Err(EvaluationError::UnsupportedProgram)
        );
    }
}
