// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! An authored effect program whose source helper declares a precondition,
//! published by the public compiler, admitted, installed and run by the
//! generic runtime (#989).
//!
//! The package is unchanged public compiler output retained under
//! `fixtures/edict-function-precondition/`. Its identity is recorded here
//! independently of the bytes. Oracles: the expected result is the authored
//! program's own law (`span(start, end) = end - start`, callable only where
//! `start <= end`, which the intent's `where` clause establishes), written by
//! hand; the refusals are the runtime owner's exact admission policy and the
//! intent's named input constraint. Forged Core whose call does not establish
//! the precondition is refused by all three judgments in the shared
//! function-precondition corpus.
use super::current_effect_publication_fixture as publication;
use super::fixture_host;
use echo_edict_canonical::CanonicalValueV1;
use warp_core::edict_effects::EffectFrameError;
use warp_core::edict_pure::{EvaluationError, EvaluationLimits};
use warp_core::edict_read::ReadError;
use warp_core::{
    EchoOperationAdmissionErrorKind, EchoOperationAdmissionPolicy, EchoOperationApplicationBasisV1,
    EchoOperationBudgetV1,
};

const PACKAGE: &str = include_str!(
    "../fixtures/edict-function-precondition/precondition-executable-operation-package.hex"
);
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "6cb9f7896a27ce9ce5477b17a5e66c3b29f902e0d8f3cc1eb85a71e957382021";
const OPERATION: &str = "consumer.effect.program@1.settle";
const AUTHORITY_LABEL: &[u8] = b"consumer.effect.authority/v1";
/// The coordinate the compiler gives the intent's `where` clause.
const CONSTRAINT: &str = "where.0";

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
fn the_public_precondition_package_runs_its_helper_where_the_call_is_proven() {
    for (start, end) in [(3, 10), (0, 0), (7, 7), (0, u64::MAX)] {
        assert_eq!(
            run(start, end),
            Ok(publication::encoded(&CanonicalValueV1::Integer(
                i128::from(end - start)
            ))),
            "span({start}, {end})"
        );
    }
}

#[test]
fn an_input_that_breaks_the_where_clause_never_reaches_the_helper() {
    // `end - start` would underflow; the intent's constraint fails first, by
    // its compiler-given name.
    for (start, end) in [(10, 3), (1, 0), (u64::MAX, 0)] {
        assert_eq!(
            run(start, end),
            Err(EffectFrameError::BoundedRead(ReadError::Evaluation(
                EvaluationError::InputConstraintFailed(CONSTRAINT.to_owned())
            ))),
            "span({start}, {end})"
        );
    }
}

#[test]
fn a_re_bound_public_package_is_refused_under_the_public_policy() {
    let (bytes, identity) = retained();
    let (host, _, _) = fixture_host();
    let mut rebound = bytes.clone();
    // Any other package, even one differing only in its trailing byte's
    // encoding, has another identity; the exact policy admits only the one.
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
