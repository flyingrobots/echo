// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Public emitted package with reviewed scan budgets and literal text/refusal cases.
use super::*;
fn package() -> (Vec<u8>, [u8; 32]) {
    let bytes = hex::decode(
        include_str!("../fixtures/edict-pure-utf8/executable-operation-package.cbor.hex").trim(),
    )
    .unwrap();
    let pin: [u8; 32] =
        hex::decode("4c2b7859f291e97b6b38fb265086e6cb0581ed030d54d38152b0585152120397")
            .unwrap()
            .try_into()
            .unwrap();
    assert_eq!(
        digest("echo.operation-package/v1", &decode(&bytes).unwrap()).unwrap(),
        pin
    );
    let report = decode(
        &hex::decode(
            include_str!("../fixtures/edict-pure-utf8/verification-report.cbor.hex").trim(),
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
    (bytes, pin)
}
fn input(original: &[u8], start: u64, end: u64, replacement: &[u8]) -> Vec<u8> {
    input_bytes_with_replacement(original, start, end, replacement)
}
fn updated_text(output: &[u8]) -> Vec<u8> {
    let value = decode(output).unwrap();
    let Value::Bytes(bytes) = field(&value, "updatedText") else {
        panic!("byte result")
    };
    bytes.clone()
}
#[test]
fn compiled_flat_text_replacement_matches_literal_utf8_cases() {
    let (package, pin) = package();
    for (name, before, start, end, replacement, expected) in [
        ("empty", "", 0, 0, "λ\r\n", "λ\r\n"),
        ("prefix", "world", 0, 0, "hello ", "hello world"),
        ("suffix", "hello", 5, 5, " 🌍", "hello 🌍"),
        ("replace", "hello world", 6, 11, "λ", "hello λ"),
        ("delete", "AλB", 1, 3, "", "AB"),
        ("crlf", "one\r\ntwo\r\n", 5, 8, "three", "one\r\nthree\r\n"),
        ("whole", "αβ", 0, 4, "🙂", "🙂"),
        ("multibyte", "a🙂z", 1, 5, "é", "aéz"),
        ("nul", "A\0B", 1, 2, "λ", "AλB"),
    ] {
        let supplied = input(before.as_bytes(), start, end, replacement.as_bytes());
        let result = evaluate(&package, pin, &supplied, limits())
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(updated_text(&result.output), expected.as_bytes(), "{name}");
        assert_eq!(
            result,
            evaluate(&package, pin, &supplied, limits()).unwrap(),
            "{name}"
        );
        assert!(result.steps > 0 && result.allocated_bytes > 0);
    }
}

#[test]
fn compiled_flat_text_replacement_refuses_invalid_utf8_boundaries_and_noops() {
    let (package, pin) = package();
    for (name, before, start, end, replacement) in [
        ("invalid-original", &b"A\xffB"[..], 0, 0, &b"X"[..]),
        ("invalid-replacement", &b"ABC"[..], 1, 2, &b"\xff"[..]),
        ("overlong", &[0xc0, 0x80][..], 0, 0, &b"X"[..]),
        ("surrogate", &[0xed, 0xa0, 0x80][..], 0, 0, &b"X"[..]),
        (
            "past-unicode",
            &[0xf4, 0x90, 0x80, 0x80][..],
            0,
            0,
            &b"X"[..],
        ),
        ("truncated", &[0xe2, 0x82][..], 0, 0, &b"X"[..]),
        ("start-inside-lambda", "aλz".as_bytes(), 2, 3, &b"X"[..]),
        ("end-inside-lambda", "aλz".as_bytes(), 1, 2, &b"X"[..]),
        ("inside-emoji", "a🙂z".as_bytes(), 2, 4, &b"X"[..]),
        ("unchanged-lambda", "AλB".as_bytes(), 1, 3, "λ".as_bytes()),
        ("empty-noop", &b""[..], 0, 0, &b""[..]),
        ("reversed", &b"ABC"[..], 2, 1, &b"X"[..]),
        ("past-end", &b"ABC"[..], 1, 4, &b"X"[..]),
    ] {
        assert_eq!(
            evaluate(
                &package,
                pin,
                &input(before, start, end, replacement),
                limits()
            ),
            Err(warp_core::edict_pure::EvaluationError::InputConstraintFailed("where.0".into())),
            "{name}"
        );
    }
}

#[test]
fn public_utf8_replacement_refuses_each_host_resource_aperture() {
    let (package, pin) = package();
    let input = input(b"abc", 1, 2, b"X");
    for (limits, error) in [
        (
            EvaluationLimits {
                max_steps: 1,
                ..limits()
            },
            EvaluationError::StepBudgetExceeded,
        ),
        (
            EvaluationLimits {
                max_allocated_bytes: 1,
                ..limits()
            },
            EvaluationError::AllocationBudgetExceeded,
        ),
        (
            EvaluationLimits {
                max_output_bytes: 1,
                ..limits()
            },
            EvaluationError::OutputBudgetExceeded,
        ),
        (
            EvaluationLimits {
                max_input_bytes: input.len() - 1,
                ..limits()
            },
            EvaluationError::InputTooLarge,
        ),
        (
            EvaluationLimits {
                max_package_bytes: package.len() - 1,
                ..limits()
            },
            EvaluationError::PackageTooLarge,
        ),
    ] {
        assert_eq!(evaluate(&package, pin, &input, limits), Err(error));
    }
}
