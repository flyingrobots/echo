// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Source validation is independent on both provider routes.
use super::*;

#[test]
fn lowerer_refuses_pure_source_with_another_coordinate() {
    let (names, fixture, _) = pure_relation::compiler_fixture();
    let mut request = lowering_request(names, &fixture);
    request
        .semantic_inputs
        .iter_mut()
        .find(|input| input.artifact.artifact.domain == "edict.source/v1")
        .expect("source input")
        .artifact
        .reference
        .coordinate = "another.application@1".to_owned();
    let refusal = lowerer::lower(request).expect_err("unrelated source coordinate must refuse");
    assert_eq!(
        refusal.kind,
        lowerer::ProviderRefusalKind::UnsupportedSemantics
    );
}

#[test]
fn verifier_refuses_pure_source_with_another_coordinate() {
    let (names, fixture, package) = pure_relation::compiler_fixture();
    let mut request = verification_request(names, &fixture, package);
    request
        .semantic_inputs
        .iter_mut()
        .find(|input| input.artifact.artifact.domain == "edict.source/v1")
        .expect("source input")
        .artifact
        .reference
        .coordinate = "another.application@1".to_owned();
    assert_verifier_refusal(request, verifier::ProviderRefusalKind::UnsupportedSemantics);
}

#[test]
fn lowerer_refuses_pure_source_that_is_not_a_byte_string() {
    assert_lowerer_source_refused(text("package unrelated@1;"));
}

#[test]
fn lowerer_refuses_pure_source_that_is_not_utf8() {
    assert_lowerer_source_refused(CanonicalValueV1::Bytes(vec![0xff]));
}

#[test]
fn verifier_refuses_pure_source_that_is_not_a_byte_string() {
    assert_verifier_source_refused(text("package unrelated@1;"));
}

#[test]
fn verifier_refuses_pure_source_that_is_not_utf8() {
    assert_verifier_source_refused(CanonicalValueV1::Bytes(vec![0xff]));
}

fn assert_lowerer_source_refused(source: CanonicalValueV1) {
    let (names, mut fixture, _) = pure_relation::compiler_fixture();
    fixture.source = canonical_bytes(&source);
    let refusal = lowerer::lower(lowering_request(names, &fixture))
        .expect_err("malformed source must refuse even with its correct digest");
    assert_eq!(
        refusal.kind,
        lowerer::ProviderRefusalKind::InvalidSemanticArtifact
    );
}

fn assert_verifier_source_refused(source: CanonicalValueV1) {
    let (names, mut fixture, package) = pure_relation::compiler_fixture();
    fixture.source = canonical_bytes(&source);
    // Rebind only the source identity in the retained package. Do not invoke the
    // lowerer: its refusal must not hide a missing independent verifier check.
    let mut package = decode_canonical_cbor_v1(&package).expect("retained package");
    let source_digest = raw_ref(names.application, "edict.source/v1", &fixture.source).digest;
    *map_field_mut(
        map_field_mut(&mut package, "semantic_closure"),
        "edict_source_identity",
    ) = CanonicalValueV1::Bytes(source_digest);
    assert_verifier_refusal(
        verification_request(names, &fixture, canonical_bytes(&package)),
        verifier::ProviderRefusalKind::InvalidSemanticArtifact,
    );
}

fn assert_verifier_refusal(
    request: verifier::VerificationRequestV1,
    expected: verifier::ProviderRefusalKind,
) {
    match verifier::verify(request) {
        Err(refusal) => assert_eq!(refusal.kind, expected),
        Ok(success) => {
            let report = decode_canonical_cbor_v1(&success.outputs[0].artifact.bytes)
                .expect("returned report is canonical");
            panic!(
                "invalid source returned a report: {:?}",
                text_field(&report, "outcome")
            );
        }
    }
}
