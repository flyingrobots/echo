// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A flat authored effect program published by the public compiler, admitted,
//! installed and run by the generic runtime.
//!
//! The package is unchanged public compiler output retained under
//! `fixtures/edict-effect-program/`. Its identity is recorded here
//! independently of the bytes. Oracles: the expected result is the authored
//! program's own law (`return input.count` through one pure binding), written
//! by hand; the refusals are the runtime owner's exact admission policy.
use super::current_effect_publication_fixture as publication;
use super::fixture_host;
use echo_edict_canonical::{decode_canonical_cbor_v1, CanonicalValueV1};
use warp_core::edict_pure::EvaluationLimits;
use warp_core::{
    EchoOperationAdmissionErrorKind, EchoOperationAdmissionPolicy, EchoOperationApplicationBasisV1,
    EchoOperationBudgetV1, TrustedRuntimeHost,
};

const PACKAGE: &str =
    include_str!("../fixtures/edict-effect-program/flat-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "2ac246d890ee7c965878a43f4da21166f0a2dab81c677f2fe29c5e84ce5a47e8";
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

fn input(count: u64) -> Vec<u8> {
    publication::encoded(&publication::map([
        ("anchor", CanonicalValueV1::Bytes(vec![0x5a; 32])),
        ("count", CanonicalValueV1::Integer(i128::from(count))),
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

fn admission_refusal(
    host: &TrustedRuntimeHost,
    identity: [u8; 32],
    bytes: Vec<u8>,
) -> EchoOperationAdmissionErrorKind {
    host.admit_echo_operation_package(&policy(identity), bytes)
        .expect_err("refused at admission")
        .kind()
}

#[test]
fn the_public_flat_effect_package_is_admitted_installed_and_run() {
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
    let input = input(7);
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
    let seven = publication::encoded(&CanonicalValueV1::Integer(7));
    assert_eq!(
        frame
            .evaluate_effect(program, &input, &[])
            .expect("execute"),
        seven
    );
    let completed = frame
        .into_effect_evaluation(64)
        .expect("completed evidence");
    assert_eq!(completed.output(), seven);
    assert!(completed.mutations().is_empty());
    assert_eq!(completed.input_basis(), basis(&input));
}

#[test]
fn another_or_re_bound_package_is_refused_under_the_public_policy() {
    let (bytes, identity) = retained();
    let (host, _, _) = fixture_host();
    // A different, well-formed effect package.
    let other = publication::encoded(&publication::package(&publication::program()));
    assert_eq!(
        admission_refusal(&host, identity, other),
        EchoOperationAdmissionErrorKind::PackageIdentityMismatch
    );
    // The public package re-bound to a smaller step ceiling: coherent, so its
    // own exact policy would admit it, but never the public one.
    let mut rebound = decode_canonical_cbor_v1(&bytes).expect("package");
    publication::set(
        publication_field_mut(&mut rebound, "budget_ceiling"),
        "steps",
        CanonicalValueV1::Integer(4095),
    );
    let rebound = publication::encoded(&rebound);
    let own = publication::hash(b"echo:operation-package:v1\0", &rebound);
    assert!(host
        .admit_echo_operation_package(&policy(own), rebound.clone())
        .is_ok());
    assert_eq!(
        admission_refusal(&host, identity, rebound),
        EchoOperationAdmissionErrorKind::PackageIdentityMismatch
    );
}

fn publication_field_mut<'a>(
    value: &'a mut CanonicalValueV1,
    key: &str,
) -> &'a mut CanonicalValueV1 {
    let CanonicalValueV1::Map(fields) = value else {
        panic!("map")
    };
    &mut fields
        .iter_mut()
        .find(|(name, _)| name == &CanonicalValueV1::Text(key.to_owned()))
        .expect("field")
        .1
}
