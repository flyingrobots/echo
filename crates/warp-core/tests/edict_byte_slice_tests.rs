// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Execute exact compiler-produced byte slicing, with malformed-artifact controls.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::unwrap_used, clippy::panic)]

#[path = "support/edict_byte_slice.rs"]
mod support;

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use support::{field, input, integer, limits, mutate, package, pin, record, set_field};
use warp_core::edict_pure::{evaluate, EvaluationError as Error, EvaluationLimits};

#[test]
fn compiler_produced_slice_executes_all_twelve_authored_cases() {
    // Literal expectations transcribed from Jim's 381cd4 leaf-slice/cases.json.
    // Inside-codepoint output remains raw bytes; this primitive is not UTF-8 policy.
    for (name, blob, start, end, expected) in [
        ("interior", "61626364", 1, 3, "6263"),
        ("empty-blob", "", 0, 0, ""),
        ("full-blob", "61626364", 0, 4, "61626364"),
        ("empty-interior", "61626364", 2, 2, ""),
        ("empty-at-end", "61626364", 4, 4, ""),
        ("binary-octets", "00017fff80", 2, 5, "7fff80"),
        ("unicode-codepoint", "61f09f98ba62", 1, 5, "f09f98ba"),
        (
            "inside-codepoint-remains-bytes",
            "61f09f98ba62",
            2,
            4,
            "9f98",
        ),
    ] {
        let supplied = input(&hex::decode(blob).unwrap(), start, end);
        let result = evaluate(&package(), pin(), &supplied, limits()).unwrap();
        assert_eq!(
            result.output,
            encode(&record([(
                "bytes",
                Value::Bytes(hex::decode(expected).unwrap())
            )]))
            .unwrap(),
            "{name}"
        );
        assert_eq!(
            result,
            evaluate(&package(), pin(), &supplied, limits()).unwrap(),
            "{name}"
        );
    }
    for (name, start, end, coordinate) in [
        ("reversed", 3, 2, "where.0"),
        ("end-past-blob", 1, 5, "where.1"),
        ("both-past-blob", 5, 5, "where.1"),
        ("u64-max", 0, u64::MAX, "where.1"),
    ] {
        assert_eq!(
            evaluate(&package(), pin(), &input(b"abcd", start, end), limits()),
            Err(Error::InputConstraintFailed(coordinate.into())),
            "{name}"
        );
    }
}

#[test]
fn exact_package_and_independent_report_are_bound_to_the_fixture() {
    let artifact = decode(&package()).unwrap();
    assert_eq!(
        digest("echo.operation-package/v1", &artifact).unwrap(),
        pin()
    );
    let report = decode(
        &hex::decode(include_str!("fixtures/edict-byte-slice/verification-report.cbor.hex").trim())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(field(&report, "outcome"), &Value::Text("accepted".into()));
    assert_eq!(field(&report, "diagnosticBytes"), &Value::Bytes(vec![]));
    assert_eq!(
        field(field(&report, "package"), "digest"),
        &Value::Array(vec![
            Value::Text("sha256".into()),
            Value::Bytes(pin().into())
        ])
    );
    assert_eq!(
        evaluate(&package(), [0; 32], &input(b"abcd", 1, 3), limits()),
        Err(Error::PackageIdentityMismatch)
    );
}

#[test]
fn slice_copy_work_is_selected_length_and_budgets_are_exact() {
    let bytes = vec![0xff; 1024];
    let empty = evaluate(&package(), pin(), &input(&bytes, 0, 0), limits()).unwrap();
    for length in [1, 2, 31, 32, 255, 1024] {
        let supplied = input(&bytes, 0, length);
        let result = evaluate(&package(), pin(), &supplied, limits()).unwrap();
        assert_eq!(result.steps, empty.steps + length);
        // Slice result, result projection's local copy, and encoding scratch.
        assert_eq!(result.allocated_bytes, empty.allocated_bytes + 3 * length);
        let exact = EvaluationLimits {
            max_steps: result.steps,
            max_allocated_bytes: result.allocated_bytes,
            max_output_bytes: result.output.len() as u64,
            ..limits()
        };
        assert_eq!(
            evaluate(&package(), pin(), &supplied, exact).unwrap(),
            result
        );
        for (bound, error) in [
            (
                EvaluationLimits {
                    max_steps: result.steps - 1,
                    ..exact
                },
                Error::StepBudgetExceeded,
            ),
            (
                EvaluationLimits {
                    max_allocated_bytes: result.allocated_bytes - 1,
                    ..exact
                },
                Error::AllocationBudgetExceeded,
            ),
            (
                EvaluationLimits {
                    max_output_bytes: result.output.len() as u64 - 1,
                    ..exact
                },
                Error::OutputBudgetExceeded,
            ),
        ] {
            assert_eq!(evaluate(&package(), pin(), &supplied, bound), Err(error));
        }
    }
}

#[test]
fn slice_rejects_invalid_input_domains_and_decode_apertures() {
    for value in [
        Value::Integer(-1),
        Value::Integer(-i128::from(u64::MAX) - 1),
        Value::Bytes(vec![]),
    ] {
        for name in ["startByte", "endByte"] {
            let mut supplied = decode(&input(b"abcd", 1, 3)).unwrap();
            set_field(&mut supplied, name, value.clone());
            assert_eq!(
                evaluate(&package(), pin(), &encode(&supplied).unwrap(), limits()),
                Err(Error::InvalidInput)
            );
        }
    }
    let mut supplied = decode(&input(b"abcd", 1, 3)).unwrap();
    set_field(&mut supplied, "blobBytes", Value::Bytes(vec![0; 1_048_577]));
    assert_eq!(
        evaluate(&package(), pin(), &encode(&supplied).unwrap(), limits()),
        Err(Error::InvalidInput)
    );
    set_field(&mut supplied, "blobBytes", Value::Integer(4));
    assert_eq!(
        evaluate(&package(), pin(), &encode(&supplied).unwrap(), limits()),
        Err(Error::InvalidInput)
    );
    for (bound, expected) in [
        (
            EvaluationLimits {
                max_package_bytes: 1,
                ..limits()
            },
            Error::PackageTooLarge,
        ),
        (
            EvaluationLimits {
                max_input_bytes: 1,
                ..limits()
            },
            Error::InputTooLarge,
        ),
    ] {
        assert_eq!(
            evaluate(&package(), pin(), &input(b"abcd", 1, 3), bound),
            Err(expected)
        );
    }
}

#[test]
fn forged_slice_signatures_and_coordinates_refuse() {
    for (name, value, expected) in [
        ("args", Value::Array(vec![]), Error::UnsupportedProgram),
        (
            "args",
            Value::Array(vec![integer(0, "U64"); 2]),
            Error::UnsupportedProgram,
        ),
        (
            "args",
            Value::Array(vec![integer(0, "U64"); 4]),
            Error::UnsupportedProgram,
        ),
        ("typeArgs", Value::Array(vec![]), Error::UnsupportedProgram),
        (
            "typeArgs",
            Value::Array(vec![Value::Text("Bytes<max=4>".into()); 2]),
            Error::UnsupportedProgram,
        ),
        (
            "typeArgs",
            Value::Array(vec![Value::Text("U64".into())]),
            Error::UnsupportedProgram,
        ),
        (
            "typeArgs",
            Value::Array(vec![Value::Text("Bytes<max=04>".into())]),
            Error::InvalidArtifact,
        ),
        (
            "typeArgs",
            Value::Array(vec![Value::Text("Bytes<min=4,max=4>".into())]),
            Error::InvalidArtifact,
        ),
    ] {
        let (artifact, verified) = mutate(|call| set_field(call, name, value));
        assert_eq!(
            evaluate(&artifact, verified, &input(b"abcd", 1, 3), limits()),
            Err(expected)
        );
    }
}

#[test]
fn forged_slice_values_cannot_bypass_runtime_bounds() {
    for (index, value, expected) in [
        (0, integer(4, "U64"), Error::InvalidArtifact),
        (1, integer(4, "U64"), Error::InvalidArtifact),
        (2, integer(5, "U64"), Error::InvalidArtifact),
        (
            2,
            integer(i128::from(u64::MAX), "U64"),
            Error::InvalidArtifact,
        ),
        (1, integer(-1, "U64"), Error::InvalidArtifact),
        (
            1,
            integer(-i128::from(u64::MAX) - 1, "U64"),
            Error::InvalidArtifact,
        ),
        (1, integer(1, "I64"), Error::UnsupportedProgram),
        (
            2,
            integer(i128::from(u32::MAX) + 1, "U32"),
            Error::UnsupportedProgram,
        ),
    ] {
        let (artifact, verified) = mutate(|call| {
            let Value::Array(args) = support::field_mut(call, "args") else {
                panic!("args")
            };
            args[index] = value;
        });
        assert_eq!(
            evaluate(&artifact, verified, &input(b"abcd", 1, 3), limits()),
            Err(expected)
        );
    }
    let (artifact, verified) = mutate(|call| {
        let Value::Array(args) = support::field_mut(call, "args") else {
            panic!("args")
        };
        args[1] = args[0].clone();
    });
    assert_eq!(
        evaluate(&artifact, verified, &input(b"abcd", 1, 3), limits()),
        Err(Error::InvalidArtifact)
    );
}

#[test]
fn slice_validates_byte_bounds_but_does_not_preserve_exact_result_length() {
    for coordinate in ["Bytes<max=4>", "Bytes<exact=4>", "Bytes<min=1,max=5>"] {
        let (artifact, verified) = mutate(|call| {
            set_field(
                call,
                "typeArgs",
                Value::Array(vec![Value::Text(coordinate.into())]),
            );
        });
        let result = evaluate(&artifact, verified, &input(b"abcd", 2, 2), limits()).unwrap();
        assert_eq!(
            result.output,
            encode(&record([("bytes", Value::Bytes(vec![]))])).unwrap()
        );
    }
    for coordinate in ["Bytes<max=3>", "Bytes<exact=5>", "Bytes<min=5,max=6>"] {
        let (artifact, verified) = mutate(|call| {
            set_field(
                call,
                "typeArgs",
                Value::Array(vec![Value::Text(coordinate.into())]),
            );
        });
        assert_eq!(
            evaluate(&artifact, verified, &input(b"abcd", 1, 3), limits()),
            Err(Error::InvalidArtifact)
        );
    }
}

#[test]
fn application_names_do_not_select_slice_behavior_or_costs() {
    let (neutral, verified) = support::neutral_package();
    assert_ne!(verified, pin());
    for (bytes, start, end) in [
        (b"abcd".as_slice(), 1, 3),
        (b"", 0, 0),
        (b"\xff\x00\x80", 0, 3),
        (b"abcd", 3, 2),
    ] {
        let supplied = input(bytes, start, end);
        let original = evaluate(&package(), pin(), &supplied, limits());
        let mut renamed = decode(&supplied).unwrap();
        support::rename(&mut renamed);
        assert_eq!(
            evaluate(&neutral, verified, &encode(&renamed).unwrap(), limits()),
            original
        );
    }
}
