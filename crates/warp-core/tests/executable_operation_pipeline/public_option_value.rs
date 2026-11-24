// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! An authored effect program whose source helpers build, test and consume an
//! `Option<U64>`, published by the public compiler, admitted, installed and
//! run by the generic runtime (#995).
//!
//! The package is unchanged public compiler output retained under
//! `fixtures/edict-option-value/`. Its identity is recorded here independently
//! of the bytes. Oracle: the authored program's own law, written by hand:
//! `above(v, floor)` is `some(v)` when `v > floor` and `none()` otherwise, and
//! `pick` unwraps it when present and otherwise defaults to 0, so the result is
//! `end` when `end > start` and 0 otherwise. Forged Option Core (unproven
//! `unwrap`, ill-typed calls, Option matches) is refused by all three
//! judgments in the shared Option corpus.
use super::current_effect_publication_fixture as publication;
use super::fixture_host;
use echo_edict_canonical::CanonicalValueV1;
use warp_core::edict_effects::EffectFrameError;
use warp_core::edict_pure::EvaluationLimits;
use warp_core::{
    EchoOperationAdmissionErrorKind, EchoOperationAdmissionPolicy, EchoOperationApplicationBasisV1,
    EchoOperationBudgetV1,
};

const PACKAGE: &str =
    include_str!("../fixtures/edict-option-value/option-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "1cc35ca7b89543c2f50ff017f6e43470e69ed7e24ad0ad5538a967d35aef95e1";
const OPERATION: &str = "consumer.effect.program@1.settle";
const AUTHORITY_LABEL: &[u8] = b"consumer.effect.authority/v1";

fn retained() -> (Vec<u8>, [u8; 32]) {
    let bytes = hex::decode(PACKAGE.trim()).expect("retained hex");
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    assert_eq!(
        hex::encode(identity),
        PACKAGE_PIN,
        "retained package identity"
    );
    (bytes, identity)
}

/// The runtime owner's exact policy for the public package.
fn policy(identity: [u8; 32]) -> EchoOperationAdmissionPolicy {
    EchoOperationAdmissionPolicy::exact(
        identity,
        OPERATION,
        publication::hash(b"echo:operation-profile:v1\0", AUTHORITY_LABEL),
        EchoOperationBudgetV1::new(4096, 0, 0),
    )
}

fn input(start: u64, end: u64) -> Vec<u8> {
    publication::encoded(&publication::map([
        ("anchor", CanonicalValueV1::Bytes(vec![0x5a; 32])),
        ("start", CanonicalValueV1::Integer(i128::from(start))),
        ("end", CanonicalValueV1::Integer(i128::from(end))),
    ]))
}

fn basis(input: &[u8]) -> EchoOperationApplicationBasisV1 {
    EchoOperationApplicationBasisV1::new(
        publication::hash(
            b"echo:operation-profile:v1\0",
            b"echo.operation.basis.edict-effect/v1",
        ),
        publication::hash(b"echo:operation-edict-effect-basis:v1\0", input),
    )
}

/// Runs the installed public package once on `(start, end)`.
fn run(start: u64, end: u64) -> Result<Vec<u8>, EffectFrameError> {
    let (bytes, identity) = retained();
    let (mut host, head, _) = fixture_host();
    let admitted = host
        .admit_echo_operation_package(&policy(identity), bytes)
        .expect("the exact policy admits the public package");
    let installed = host
        .install_admitted_echo_operation_package(admitted)
        .expect("installation");
    let program = installed
        .program()
        .as_effect_program()
        .expect("an effect program");
    let input = input(start, end);
    let parent = host
        .echo_operation_evaluation_basis_v1(head, basis(&input))
        .expect("basis");
    let limits = EvaluationLimits {
        max_package_bytes: 1_048_576,
        max_input_bytes: 65_536,
        max_steps: 4_096,
        max_allocated_bytes: 1_048_576,
        max_output_bytes: 65_536,
    };
    let mut frame = host
        .effect_evaluation_frame_v1(
            parent,
            &[],
            0,
            EchoOperationBudgetV1::new(4096, 0, 0),
            limits,
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let output = frame.evaluate_effect(program, &input, &[])?.to_vec();
    let completed = frame
        .into_effect_evaluation(64)
        .expect("completed evidence");
    assert_eq!(completed.output(), output);
    assert!(completed.mutations().is_empty());
    Ok(output)
}

#[test]
fn the_public_option_package_unwraps_a_present_option_and_defaults_an_absent_one() {
    for (start, end) in [
        (3, 10),
        (0, 1),
        (0, u64::MAX),
        (7, 7),
        (10, 3),
        (u64::MAX, 0),
    ] {
        let expected = if end > start { end } else { 0 };
        assert_eq!(
            run(start, end),
            Ok(publication::encoded(&CanonicalValueV1::Integer(
                i128::from(expected)
            ))),
            "pick({end}, {start})"
        );
    }
}

#[test]
fn a_re_bound_public_option_package_is_refused_under_the_public_policy() {
    let (bytes, identity) = retained();
    let (host, _, _) = fixture_host();
    let mut rebound = bytes.clone();
    rebound.push(0);
    assert_eq!(
        host.admit_echo_operation_package(&policy(identity), rebound)
            .expect_err("refused at admission")
            .kind(),
        EchoOperationAdmissionErrorKind::PackageIdentityMismatch
    );
    assert!(host
        .admit_echo_operation_package(&policy(identity), bytes)
        .is_ok());
}
