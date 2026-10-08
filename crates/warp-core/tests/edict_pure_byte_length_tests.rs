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
    include_str!("fixtures/edict-pure-byte-length/executable-operation-package.cbor.hex");
const REPORT_HEX: &str =
    include_str!("fixtures/edict-pure-byte-length/verification-report.cbor.hex");
const PACKAGE_DIGEST: &str = "cf3a9c1eaefc60d6826e639d51ad8c7beaefad93cdac49b80c759d334992d85b";

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
fn compiler_produced_length_preserves_literal_results_and_prior_behavior() {
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
    for (start, end, empty, difference) in [
        (7, 7, 1, 0),
        (7, 11, 0, 4),
        (0, u64::MAX, 0, i128::from(u64::MAX)),
        (u64::MAX - 1, u64::MAX, 0, 1),
    ] {
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
            ("deletedByteCount", Value::Integer(difference)),
            ("insertedByteCount", Value::Integer(4)),
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

fn field_mut<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    let Value::Map(fields) = value else {
        panic!("expected record")
    };
    &mut fields
        .iter_mut()
        .find(|(key, _)| key == &Value::Text(name.into()))
        .unwrap()
        .1
}

// Mutations deliberately get a matching host pin to exercise defense in depth.
// They are not independently verified compiler artifacts or admission evidence.
fn mutate_length(change: impl FnOnce(&mut Value)) -> (Vec<u8>, [u8; 32]) {
    let mut package = decode(&package()).unwrap();
    let Value::Bytes(bytes) = field(&package, "program") else {
        panic!("program bytes")
    };
    let mut program = decode(bytes).unwrap();
    let Value::Bytes(bytes) = field(&program, "target_ir_artifact") else {
        panic!("target bytes")
    };
    let mut target = decode(bytes).unwrap();
    let intent = field_mut(field_mut(&mut target, "intents"), "replaceRange");
    let Value::Array(bindings) = field_mut(intent, "pureBindings") else {
        panic!("bindings")
    };
    let binding = bindings.iter_mut().find(|binding| {
        let value = field(binding, "value");
        matches!(value, Value::Map(fields) if fields.iter().any(|(key, value)|
            key == &Value::Text("callee".into()) && value == &Value::Text("core.bytes.length".into())))
    }).unwrap();
    change(field_mut(binding, "value"));
    let identity = digest("edict.target-ir.artifact/v1", &target).unwrap();
    set_field(
        &mut program,
        "target_ir_artifact",
        Value::Bytes(encode(&target).unwrap()),
    );
    set_field(
        &mut package,
        "program",
        Value::Bytes(encode(&program).unwrap()),
    );
    set_field(
        field_mut(&mut package, "semantic_closure"),
        "target_ir_identity",
        Value::Bytes(identity.to_vec()),
    );
    let pin = digest("echo.operation-package/v1", &package).unwrap();
    (encode(&package).unwrap(), pin)
}

fn integer(value: i128, width: &str) -> Value {
    record([
        ("kind", Value::Text("const".into())),
        (
            "value",
            record([
                ("kind", Value::Text("int".into())),
                ("value", Value::Integer(value)),
                ("width", Value::Text(width.into())),
            ]),
        ),
    ])
}

#[test]
fn byte_length_counts_raw_bytes_including_empty_and_non_ascii_inputs() {
    for (replacement, expected) in [
        (vec![], 0),
        ("猫\r\n".as_bytes().to_vec(), 5),
        ("λ\r\n".as_bytes().to_vec(), 4),
        (vec![255, 0], 2),
    ] {
        let mut supplied = input(7, 11);
        set_field(
            &mut supplied,
            "replacement",
            Value::Bytes(replacement.clone()),
        );
        let result = evaluate(&package(), pin(), &encode(&supplied).unwrap(), limits()).unwrap();
        let output = decode(&result.output).unwrap();
        assert_eq!(
            field(&output, "insertedByteCount"),
            &Value::Integer(expected)
        );
        assert_eq!(field(&output, "replacement"), &Value::Bytes(replacement));
    }
}

#[test]
fn byte_length_rejects_malformed_call_signatures() {
    for (name, value) in [
        ("args", Value::Array(vec![])),
        ("args", Value::Array(vec![integer(1, "U64"); 2])),
        ("typeArgs", Value::Array(vec![])),
        ("typeArgs", Value::Array(vec![Value::Text("U64".into()); 2])),
        ("typeArgs", Value::Array(vec![Value::Text("U64".into())])),
    ] {
        let (package, pin) = mutate_length(|call| set_field(call, name, value));
        assert_eq!(
            evaluate(&package, pin, &encode(&input(7, 11)).unwrap(), limits()),
            Err(EvaluationError::UnsupportedProgram),
            "{name}"
        );
    }
}

#[test]
fn byte_length_validates_declared_bounds_and_operand_representation() {
    let (package, pin) = mutate_length(|call| {
        set_field(
            call,
            "typeArgs",
            Value::Array(vec![Value::Text("jedit.text@1.NodeId".into())]),
        );
    });
    assert_eq!(
        evaluate(&package, pin, &encode(&input(7, 11)).unwrap(), limits()),
        Err(EvaluationError::InvalidArtifact)
    );
    let (package, pin) = mutate_length(|call| {
        set_field(call, "args", Value::Array(vec![integer(4, "U64")]));
    });
    assert_eq!(
        evaluate(&package, pin, &encode(&input(7, 11)).unwrap(), limits()),
        Err(EvaluationError::InvalidArtifact)
    );
}

#[test]
fn byte_length_decodes_canonical_structural_byte_bounds() {
    for coordinate in ["Bytes<max=4>", "Bytes<exact=4>", "Bytes<min=1,max=5>"] {
        let (package, pin) = mutate_length(|call| {
            set_field(
                call,
                "typeArgs",
                Value::Array(vec![Value::Text(coordinate.into())]),
            );
        });
        let result = evaluate(&package, pin, &encode(&input(7, 11)).unwrap(), limits()).unwrap();
        assert_eq!(
            field(&decode(&result.output).unwrap(), "insertedByteCount"),
            &Value::Integer(4)
        );
    }
    for coordinate in [
        "Bytes<max=04>",
        "Bytes<exact=-1>",
        "Bytes<min=5,max=4>",
        "Bytes<min=4,max=4>",
        "Bytes<max=18446744073709551616>",
        "Bytes<max=4,extra=1>",
    ] {
        let (package, pin) = mutate_length(|call| {
            set_field(
                call,
                "typeArgs",
                Value::Array(vec![Value::Text(coordinate.into())]),
            );
        });
        assert_eq!(
            evaluate(&package, pin, &encode(&input(7, 11)).unwrap(), limits()),
            Err(EvaluationError::InvalidArtifact),
            "{coordinate}"
        );
    }
}

#[test]
fn fresh_compiler_package_with_inline_bytes_executes() {
    let package = hex::decode(
        include_str!(
            "fixtures/edict-pure-byte-length/inline-executable-operation-package.cbor.hex"
        )
        .trim(),
    )
    .unwrap();
    let report = decode(
        &hex::decode(
            include_str!("fixtures/edict-pure-byte-length/inline-verification-report.cbor.hex")
                .trim(),
        )
        .unwrap(),
    )
    .unwrap();
    let Value::Array(identity) = field(field(&report, "package"), "digest") else {
        panic!("digest")
    };
    let Value::Bytes(bytes) = &identity[1] else {
        panic!("digest bytes")
    };
    let pin: [u8; 32] = bytes.clone().try_into().unwrap();
    assert_eq!(field(&report, "outcome"), &Value::Text("accepted".into()));
    assert_eq!(
        digest("echo.operation-package/v1", &decode(&package).unwrap()).unwrap(),
        pin
    );
    let result = evaluate(&package, pin, &encode(&input(7, 11)).unwrap(), limits()).unwrap();
    assert_eq!(
        field(&decode(&result.output).unwrap(), "insertedByteCount"),
        &Value::Integer(4)
    );
}
