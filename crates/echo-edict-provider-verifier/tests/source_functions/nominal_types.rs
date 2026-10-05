// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic Core extensions, not public compiler acceptance evidence.
//! The retained source and imported authority remain unchanged. The independent
//! carrier helper rebinds changed Core/Target/package identities without lowering.
use super::*;

fn add_nominal_types(core: &mut CanonicalValueV1) {
    for name in ["A", "B"] {
        // Exact bare keys and matching contracts avoid the separate qualified-key
        // lookup question. Equal U64 representation does not equate A with B.
        add_field(
            map_field_mut(core, "types"),
            name,
            map([
                ("kind", text("Nominal")),
                ("contract", text(name)),
                ("representation", text("U64")),
            ]),
        );
    }
}

fn return_fixture(result_type: &str) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    edited_fixture(|core, _| {
        add_nominal_types(core);
        let parameter = local("arg.0", "A");
        add_field(
            core,
            "functions",
            map([(
                "unusedIdentity",
                function(
                    vec![parameter.clone()],
                    result_type,
                    vec![],
                    vec![],
                    reference(&parameter),
                ),
            )]),
        );
    })
}

fn argument_fixture(parameter_type: &str) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    edited_fixture(|core, _| {
        add_nominal_types(core);
        let coordinate = text_field(core, "coordinate").expect("coordinate");
        let call_name = format!("{coordinate}.constant");
        let argument = local("arg.0", "A");
        add_field(
            core,
            "functions",
            map([
                (
                    "constant",
                    function(
                        vec![local("arg.0", parameter_type)],
                        "U64",
                        vec![],
                        vec![],
                        literal("U64", 0),
                    ),
                ),
                (
                    "unusedCaller",
                    function(
                        vec![argument.clone()],
                        "U64",
                        vec![],
                        vec![],
                        call(&call_name, vec![reference(&argument)]),
                    ),
                ),
            ]),
        );
    })
}

fn assert_admitted((names, fixture, package): (FixtureNames<'_>, RawFixture, Vec<u8>)) {
    let lowered = lowerer::lower(lowering_request(names, &fixture))
        .expect("same-nominal source signature is valid");
    assert_eq!(
        lowered.outputs[0].artifact.bytes, package,
        "independently rebound package matches actual lowering"
    );
    assert!(
        verifier_accepts(names, &fixture, package),
        "independent verifier admits the same-nominal control"
    );
}

#[test]
fn unused_same_nominal_return_is_admitted() {
    assert_admitted(return_fixture("A"));
}

#[test]
fn lowerer_rejects_distinct_nominal_return_with_identical_representation() {
    // Only the unused helper's return declaration differs from the valid A -> A.
    let (names, fixture, _) = return_fixture("B");
    assert!(
        lowerer::lower(lowering_request(names, &fixture)).is_err(),
        "U64 representation must not authorize returning A as distinct nominal B"
    );
}

#[test]
fn verifier_independently_rejects_distinct_nominal_return_with_identical_representation() {
    let (names, fixture, package) = return_fixture("B");
    assert!(
        !verifier_accepts(names, &fixture, package),
        "coherent carriers must not authorize returning A as distinct nominal B"
    );
}

#[test]
fn unused_same_nominal_call_argument_is_admitted() {
    assert_admitted(argument_fixture("A"));
}

#[test]
fn lowerer_rejects_distinct_nominal_call_argument_with_identical_representation() {
    // Both helpers still return a valid U64. Only the callee's parameter type
    // changes; its literal body is valid for either signature. The caller has A.
    let (names, fixture, _) = argument_fixture("B");
    assert!(
        lowerer::lower(lowering_request(names, &fixture)).is_err(),
        "U64 representation must not authorize passing A to a parameter of nominal B"
    );
}

#[test]
fn verifier_independently_rejects_distinct_nominal_call_argument_with_identical_representation() {
    let (names, fixture, package) = argument_fixture("B");
    assert!(
        !verifier_accepts(names, &fixture, package),
        "coherent carriers must not authorize passing A to a parameter of nominal B"
    );
}
