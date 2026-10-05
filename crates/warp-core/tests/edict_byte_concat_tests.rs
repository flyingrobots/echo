// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Execute exact compiler-produced concatenation and falsify runtime boundaries.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::unwrap_used, clippy::panic)]

#[path = "support/edict_byte_concat.rs"]
mod support;

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use sha2::{Digest, Sha256};
use support::{field, input, integer, limits, mutate, package, pin, record, set_field};
use warp_core::edict_pure::{evaluate, EvaluationError as Error, EvaluationLimits};

#[test]
fn compiler_produced_concat_executes_all_twelve_authored_cases() {
    // Literal rows transcribed unchanged from the retained Jim cases.json.
    for (name, left, right, expected) in [
        ("both-empty", "", "", Some("")),
        ("first-empty", "", "6162", Some("6162")),
        ("second-empty", "6162", "", Some("6162")),
        ("in-order", "61626364", "65666768", Some("6162636465666768")),
        (
            "different-order",
            "65666768",
            "61626364",
            Some("6566676861626364"),
        ),
        ("binary-octets", "00ff", "807f", Some("00ff807f")),
        ("codepoint-fragments", "f09f", "98ba", Some("f09f98ba")),
        (
            "incomplete-codepoint-remains-bytes",
            "f0",
            "9f",
            Some("f09f"),
        ),
        (
            "full-apertures",
            "0001020304050607",
            "08090a0b0c0d0e0f",
            Some("000102030405060708090a0b0c0d0e0f"),
        ),
        (
            "repeated-bytes-preserved",
            "ff00ff",
            "ff00ff",
            Some("ff00ffff00ff"),
        ),
        ("first-over-bound", "000102030405060708", "", None),
        ("second-over-bound", "", "000102030405060708", None),
    ] {
        let supplied = input(&hex::decode(left).unwrap(), &hex::decode(right).unwrap());
        let actual = evaluate(&package(), pin(), &supplied, limits());
        match expected {
            Some(bytes) => {
                let result = actual.unwrap_or_else(|error| panic!("{name}: {error:?}"));
                assert_eq!(
                    result.output,
                    encode(&record([(
                        "bytes",
                        Value::Bytes(hex::decode(bytes).unwrap())
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
            None => assert_eq!(actual, Err(Error::InvalidInput), "{name}"),
        }
    }
}

#[test]
fn exact_package_and_independent_report_bind_the_fixture() {
    assert_eq!(
        hex::encode(Sha256::digest(include_bytes!(
            "fixtures/edict-byte-concat/RangeAssembly.edict"
        ))),
        "8e8d2703759fb30cb49b73650ba8ddfd637d68b33b86145661bfca3033c968fc"
    );
    assert_eq!(
        hex::encode(Sha256::digest(include_bytes!(
            "fixtures/edict-byte-concat/cases.json"
        ))),
        "5c5901b20aea3b4e318dd76407b2880eb2bc3279f4a0a25c8b7bbe83dc4ceee2"
    );
    assert_eq!(
        hex::encode(Sha256::digest(package())),
        "e889d4680435139fe76f45762f0529c090afcf3d73ef7d17f787d44a49bda534"
    );
    assert_eq!(
        hex::encode(Sha256::digest(
            hex::decode(
                include_str!("fixtures/edict-byte-concat/verification-report.cbor.hex").trim()
            )
            .unwrap()
        )),
        "7eec90854e0663aa2346ec5005ff7d05eeb50fc2229b32b407dda3cfa0280077"
    );
    assert_eq!(
        digest("echo.operation-package/v1", &decode(&package()).unwrap()).unwrap(),
        pin()
    );
    let report = decode(
        &hex::decode(
            include_str!("fixtures/edict-byte-concat/verification-report.cbor.hex").trim(),
        )
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
        evaluate(&package(), [0; 32], &input(b"a", b"b"), limits()),
        Err(Error::PackageIdentityMismatch)
    );
}

#[test]
fn concat_rejects_malformed_calls_and_static_bound_overflow() {
    for (name, value, expected) in [
        (
            "callee",
            Value::Text("core.bytes.concat.extra".into()),
            Error::UnsupportedProgram,
        ),
        ("args", Value::Array(vec![]), Error::UnsupportedProgram),
        (
            "args",
            Value::Array(vec![integer(0, "U64")]),
            Error::UnsupportedProgram,
        ),
        (
            "args",
            Value::Array(vec![integer(0, "U64"); 3]),
            Error::UnsupportedProgram,
        ),
        ("typeArgs", Value::Array(vec![]), Error::UnsupportedProgram),
        (
            "typeArgs",
            Value::Array(vec![Value::Text("Bytes<max=8>".into())]),
            Error::UnsupportedProgram,
        ),
        (
            "typeArgs",
            Value::Array(vec![Value::Text("Bytes<max=8>".into()); 3]),
            Error::UnsupportedProgram,
        ),
    ] {
        let (artifact, verified) = mutate(|call| set_field(call, name, value));
        assert_eq!(
            evaluate(&artifact, verified, &input(b"a", b"b"), limits()),
            Err(expected)
        );
    }
    for (left, right, expected) in [
        ("U64", "Bytes<max=8>", Error::UnsupportedProgram),
        ("Bytes<max=8>", "U32", Error::UnsupportedProgram),
        ("Bytes<max=08>", "Bytes<max=8>", Error::InvalidArtifact),
        ("Bytes<max=8>", "Bytes<min=8,max=8>", Error::InvalidArtifact),
        (
            "Bytes<max=18446744073709551615>",
            "Bytes<max=1>",
            Error::InvalidArtifact,
        ),
        (
            "Bytes<max=1>",
            "Bytes<max=18446744073709551615>",
            Error::InvalidArtifact,
        ),
    ] {
        let (artifact, verified) = mutate(|call| {
            set_field(
                call,
                "typeArgs",
                Value::Array(vec![Value::Text(left.into()), Value::Text(right.into())]),
            )
        });
        assert_eq!(
            evaluate(&artifact, verified, &input(b"a", b"b"), limits()),
            Err(expected)
        );
    }
}

#[test]
fn concat_validates_both_operand_values_and_each_declared_bound() {
    for index in 0..2 {
        let (artifact, verified) = mutate(|call| {
            let Value::Array(args) = support::field_mut(call, "args") else {
                panic!("args")
            };
            args[index] = integer(1, "U64");
        });
        assert_eq!(
            evaluate(&artifact, verified, &input(b"ab", b"cde"), limits()),
            Err(Error::InvalidArtifact)
        );
    }
    for (left, right, valid) in [
        ("Bytes<exact=2>", "Bytes<min=1,max=4>", true),
        ("Bytes<min=1,max=3>", "Bytes<exact=3>", true),
        ("Bytes<max=1>", "Bytes<max=8>", false),
        ("Bytes<max=8>", "Bytes<max=2>", false),
        ("Bytes<exact=3>", "Bytes<max=8>", false),
        ("Bytes<max=8>", "Bytes<min=4,max=8>", false),
    ] {
        let (artifact, verified) = mutate(|call| {
            set_field(
                call,
                "typeArgs",
                Value::Array(vec![Value::Text(left.into()), Value::Text(right.into())]),
            )
        });
        let result = evaluate(&artifact, verified, &input(b"ab", b"cde"), limits());
        if valid {
            assert_eq!(
                result.unwrap().output,
                encode(&record([("bytes", Value::Bytes(b"abcde".to_vec()))])).unwrap()
            );
        } else {
            assert_eq!(result, Err(Error::InvalidArtifact));
        }
    }
}

#[test]
fn concat_cost_depends_on_total_byte_count_and_host_limits_are_exact() {
    let empty = evaluate(&package(), pin(), &input(b"", b""), limits()).unwrap();
    for (left, right) in [
        (b"a".as_slice(), b"".as_slice()),
        (b"", b"a"),
        (b"ab", b"cde"),
        (b"12345678", b"abcdefgh"),
    ] {
        let supplied = input(left, right);
        let result = evaluate(&package(), pin(), &supplied, limits()).unwrap();
        assert_eq!(
            result.steps,
            empty.steps + (left.len() + right.len()) as u64
        );
        let swapped = evaluate(&package(), pin(), &input(right, left), limits()).unwrap();
        assert_eq!(
            (result.steps, result.allocated_bytes),
            (swapped.steps, swapped.allocated_bytes)
        );
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
fn concat_rejects_invalid_input_and_decode_apertures() {
    for name in ["firstFragment", "secondFragment"] {
        let mut supplied = decode(&input(b"a", b"b")).unwrap();
        set_field(&mut supplied, name, Value::Integer(1));
        assert_eq!(
            evaluate(&package(), pin(), &encode(&supplied).unwrap(), limits()),
            Err(Error::InvalidInput)
        );
    }
    for (bound, error) in [
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
            evaluate(&package(), pin(), &input(b"a", b"b"), bound),
            Err(error)
        );
    }
}

#[test]
fn application_names_do_not_select_concat_behavior_or_costs() {
    let (neutral, verified) = support::neutral_package();
    assert_ne!(verified, pin());
    for (left, right) in [
        (b"a".as_slice(), b"b".as_slice()),
        (b"", b""),
        (b"\xff\0", b"\x80"),
        (b"123456789", b""),
    ] {
        let supplied = input(left, right);
        let original = evaluate(&package(), pin(), &supplied, limits());
        let mut renamed = decode(&supplied).unwrap();
        support::rename(&mut renamed);
        assert_eq!(
            evaluate(&neutral, verified, &encode(&renamed).unwrap(), limits()),
            original
        );
    }
}

#[test]
fn concat_respects_package_budgets_and_result_binding_bounds() {
    let supplied = input(b"ab", b"cde");
    let result = evaluate(&package(), pin(), &supplied, limits()).unwrap();
    for (package_key, core_key, amount, error) in [
        (
            "max_steps",
            "maxSteps",
            result.steps,
            Error::StepBudgetExceeded,
        ),
        (
            "max_allocated_bytes",
            "maxAllocatedBytes",
            result.allocated_bytes,
            Error::AllocationBudgetExceeded,
        ),
        (
            "max_output_bytes",
            "maxOutputBytes",
            result.output.len() as u64,
            Error::OutputBudgetExceeded,
        ),
    ] {
        let (exact, verified) = support::budget(package_key, core_key, amount);
        assert_eq!(
            evaluate(&exact, verified, &supplied, limits()).unwrap(),
            result
        );
        let (short, verified) = support::budget(package_key, core_key, amount - 1);
        assert_eq!(evaluate(&short, verified, &supplied, limits()), Err(error));
    }
    let (narrow, verified) = support::narrow_binding();
    assert_eq!(
        evaluate(&narrow, verified, &supplied, limits()),
        Err(Error::InvalidArtifact)
    );
}
