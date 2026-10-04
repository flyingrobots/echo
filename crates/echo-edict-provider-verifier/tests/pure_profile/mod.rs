// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure package authority requires the adapter's effect-free profile contract.
use super::*;

#[test]
fn lowerer_refuses_effectful_pure_profile() {
    assert_lowerer_refuses(add_effect);
}

#[test]
fn verifier_refuses_effectful_pure_profile() {
    assert_verifier_refuses(add_effect);
}

#[test]
fn lowerer_refuses_pure_profile_without_budget_obligation() {
    assert_lowerer_refuses(remove_budget);
}

#[test]
fn verifier_refuses_pure_profile_without_budget_obligation() {
    assert_verifier_refuses(remove_budget);
}

fn add_effect(profile: &mut CanonicalValueV1) {
    *map_field_mut(profile, "semanticEffects") =
        CanonicalValueV1::Array(vec![text("application.write/v1")]);
}

fn remove_budget(profile: &mut CanonicalValueV1) {
    let CanonicalValueV1::Map(entries) = profile else {
        panic!("profile map");
    };
    entries.retain(|(key, _)| key != &text("budgetObligation"));
}

fn assert_lowerer_refuses(mutate: impl FnOnce(&mut CanonicalValueV1)) {
    let names = FIXTURES[0];
    let fixture = pure_raw_fixture_with_profile(names, mutate);
    let refusal = lowerer::lower(lowering_request(names, &fixture))
        .expect_err("invalid pure adapter profile must refuse");
    assert_eq!(
        refusal.kind,
        lowerer::ProviderRefusalKind::UnsupportedSemantics
    );
}

fn assert_verifier_refuses(mutate: impl FnOnce(&mut CanonicalValueV1)) {
    let names = FIXTURES[0];
    let baseline = pure_raw_fixture(names);
    let package = lower_package(names, &baseline);
    let fixture = pure_raw_fixture_with_profile(names, mutate);
    let mut package = decode_canonical_cbor_v1(&package).expect("valid package");
    let CanonicalValueV1::Bytes(program_bytes) = map_field_mut(&mut package, "program") else {
        panic!("program bytes");
    };
    let mut program = decode_canonical_cbor_v1(program_bytes).expect("valid program");
    *map_field_mut(&mut program, "target_ir_artifact") =
        CanonicalValueV1::Bytes(fixture.target_ir.clone());
    *program_bytes = canonical_bytes(&program);
    let closure = map_field_mut(&mut package, "semantic_closure");
    for (field, coordinate, domain, bytes) in [
        (
            "lawpack_identity",
            names.lawpack,
            "edict.lawpack/v1",
            &fixture.lawpack,
        ),
        (
            "target_ir_identity",
            "echo.span-ir/v1",
            "edict.target-ir.artifact/v1",
            &fixture.target_ir,
        ),
    ] {
        *map_field_mut(closure, field) =
            CanonicalValueV1::Bytes(raw_ref(coordinate, domain, bytes).digest);
    }
    match verifier::verify(verification_request(
        names,
        &fixture,
        canonical_bytes(&package),
    )) {
        Err(refusal) => assert_eq!(
            refusal.kind,
            verifier::ProviderRefusalKind::UnsupportedSemantics
        ),
        Ok(success) => {
            let report =
                decode_canonical_cbor_v1(&success.outputs[0].artifact.bytes).expect("report");
            panic!(
                "invalid pure profile returned {:?}",
                text_field(&report, "outcome")
            );
        }
    }
}
