// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! An authored effect program whose source helpers use every checked integer
//! form and `min`/`max`, published by the public compiler, admitted, installed
//! and run by the generic runtime (#998).
//!
//! The package is unchanged public compiler output retained under
//! `fixtures/edict-checked-integer/`. Its identity is recorded here
//! independently of the bytes. Oracle: Amblicit's specified results, written
//! by hand: unsigned arithmetic whose `none` case (overflow, underflow, a zero
//! divisor) the program maps to 42, and the program's own bound checks, which
//! return 0 when every U8 or U32 check holds and otherwise the first failing
//! one. Forged Core is refused by all three judgments in the shared
//! checked-integer corpus.
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
    include_str!("../fixtures/edict-checked-integer/checked-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "93dc1e3a29c87ae59d498e5146cddd5ac461151910b627a681c83d718061f77b";
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

/// One run: the selector, the two U64 operands and the U8 and U32 probes.
#[derive(Clone, Copy)]
struct Run {
    op: u64,
    start: u64,
    end: u64,
    small: u8,
    mid: u32,
}

fn input(run: Run) -> Vec<u8> {
    publication::encoded(&publication::map([
        ("anchor", CanonicalValueV1::Bytes(vec![0x5a; 32])),
        ("op", CanonicalValueV1::Integer(i128::from(run.op))),
        ("start", CanonicalValueV1::Integer(i128::from(run.start))),
        ("end", CanonicalValueV1::Integer(i128::from(run.end))),
        ("small", CanonicalValueV1::Integer(i128::from(run.small))),
        ("mid", CanonicalValueV1::Integer(i128::from(run.mid))),
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

/// Runs the installed public package once.
fn run(run: Run) -> Result<Vec<u8>, EffectFrameError> {
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
    let input = input(run);
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

const ABSENT: u64 = 42;

fn binary(op: u64, start: u64, end: u64) -> Run {
    Run {
        op,
        start,
        end,
        small: 1,
        mid: 1,
    }
}

#[test]
fn every_checked_u64_form_returns_its_specified_option_at_the_bounds() {
    let max = u64::MAX;
    // (op, a, b, specified result; ABSENT for none)
    let table = [
        (0, max, 0, max),
        (0, max, 1, ABSENT),
        (0, max - 1, 1, max),
        (1, 5, 5, 0),
        (1, 0, 1, ABSENT),
        (1, max, max, 0),
        (2, max, 1, max),
        (2, max, 2, ABSENT),
        (2, 0, max, 0),
        (3, 7, 2, 3),
        (3, max, 0, ABSENT),
        (3, max, max, 1),
        (4, 7, 3, 1),
        (4, 7, 0, ABSENT),
        (5, max, 0, 0),
        (5, 3, 9, 3),
        (6, max, 0, max),
        (6, 3, 9, 9),
    ];
    for (op, a, b, expected) in table {
        assert_eq!(
            run(binary(op, a, b)),
            Ok(publication::encoded(&CanonicalValueV1::Integer(
                i128::from(expected)
            ))),
            "op {op} on ({a}, {b})"
        );
    }
}

#[test]
fn every_u8_and_u32_bound_check_holds_at_the_width_maximum() {
    for (op, small, mid) in [(7, u8::MAX, 1), (8, 1, u32::MAX)] {
        assert_eq!(
            run(Run {
                op,
                start: 0,
                end: 0,
                small,
                mid,
            }),
            Ok(publication::encoded(&CanonicalValueV1::Integer(0))),
            "bounds op {op}"
        );
    }
}

#[test]
fn a_re_bound_public_checked_package_is_refused_under_the_public_policy() {
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
