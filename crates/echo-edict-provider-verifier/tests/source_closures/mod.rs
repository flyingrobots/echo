// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The verifier judges the exact source closure independently of lowering.
use super::*;

fn envelope(coordinate: &str, source: &[u8]) -> Vec<u8> {
    let source = decode_canonical_cbor_v1(source).expect("original source bytes");
    let digest =
        digest_canonical_value_bytes_v1("edict.source/v1", &source).expect("unit identity");
    encode_canonical_cbor_v1(&map([
        ("apiVersion", text("edict.source-closure/v1")),
        ("root", text(coordinate)),
        (
            "modules",
            dynamic_map([(
                coordinate,
                map([
                    ("source", source),
                    (
                        "digest",
                        CanonicalValueV1::Array(vec![
                            text("sha256"),
                            CanonicalValueV1::Bytes(digest.to_vec()),
                        ]),
                    ),
                ]),
            )]),
        ),
    ]))
    .expect("closure bytes")
}

#[test]
fn verifier_accepts_new_source_form_and_rejects_a_validly_rebound_closure() {
    let names = FIXTURES[0];
    let fixture = raw_fixture(names);
    let source = envelope(names.application, &fixture.source);
    let mut request = lowering_request(names, &fixture);
    let input = request
        .semantic_inputs
        .iter_mut()
        .find(|input| input.kind == lowerer::SemanticInputKind::Auxiliary("edict-source".into()))
        .expect("source input");
    input.kind = lowerer::SemanticInputKind::Auxiliary("edict-source-closure".into());
    input.artifact = lowerer_bound(names.application, "edict.source-closure/v1", &source);
    let lowered = lowerer::lower(request).expect("new source lowers");
    let package = lowered.outputs[0].artifact.bytes.clone();
    let mut request = verification_request(names, &fixture, package);
    let input = request
        .semantic_inputs
        .iter_mut()
        .find(|input| input.kind == verifier::SemanticInputKind::Auxiliary("edict-source".into()))
        .expect("verifier source input");
    input.kind = verifier::SemanticInputKind::Auxiliary("edict-source-closure".into());
    input.artifact = verifier_bound(names.application, "edict.source-closure/v1", &source);
    let accepted = verifier::verify(request.clone()).expect("new source verifies");
    let report = decode_canonical_cbor_v1(&accepted.outputs[0].artifact.bytes).expect("report");
    assert_eq!(text_field(&report, "outcome"), Some("accepted"));
    let CanonicalValueV1::Bytes(mut bytes) =
        decode_canonical_cbor_v1(&fixture.source).expect("legacy source")
    else {
        panic!("source byte string")
    };
    bytes.extend_from_slice(b"\n// validly rebound source bytes");
    let changed = envelope(
        names.application,
        &encode_canonical_cbor_v1(&CanonicalValueV1::Bytes(bytes)).expect("canonical source"),
    );
    let input = request
        .semantic_inputs
        .iter_mut()
        .find(|input| {
            input.kind == verifier::SemanticInputKind::Auxiliary("edict-source-closure".into())
        })
        .expect("source closure");
    input.artifact = verifier_bound(names.application, "edict.source-closure/v1", &changed);
    let rejected = verifier::verify(request).expect("a relation rejection produces a report");
    let report = decode_canonical_cbor_v1(&rejected.outputs[0].artifact.bytes).expect("report");
    assert_eq!(text_field(&report, "outcome"), Some("rejected"));
}

#[test]
fn verifier_refuses_ambiguous_source_forms_at_the_original_input_arity() {
    let names = FIXTURES[0];
    let fixture = raw_fixture(names);
    let lowered = lowerer::lower(lowering_request(names, &fixture)).expect("legacy package");
    let mut request =
        verification_request(names, &fixture, lowered.outputs[0].artifact.bytes.clone());
    let configuration = request
        .semantic_inputs
        .iter()
        .position(|input| {
            input.kind == verifier::SemanticInputKind::Auxiliary("target-configuration".into())
        })
        .expect("configuration input");
    request.semantic_inputs[configuration].kind =
        verifier::SemanticInputKind::Auxiliary("edict-source-closure".into());
    request.semantic_inputs[configuration].artifact = verifier_bound(
        names.application,
        "edict.source-closure/v1",
        &envelope(names.application, &fixture.source),
    );
    assert_eq!(request.semantic_inputs.len(), 7);
    let error = verifier::verify(request)
        .expect_err("ambiguous source forms refuse before report generation");
    assert_eq!(
        error.kind,
        verifier::ProviderRefusalKind::UnsupportedSemantics
    );
    assert!(
        error.diagnostics.iter().any(|diagnostic| diagnostic.message
            == "unsupported semantics: multiple source artifact forms are ambiguous"),
        "{error:?}"
    );
}
