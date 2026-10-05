// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deliberately rebound native-provider tests, not compiler acceptance evidence.
use super::*;

fn fixture_with_functions(functions: CanonicalValueV1) -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    let (names, mut fixture, package) = pure_relation::compiler_fixture();
    let mut core = decode_canonical_cbor_v1(&fixture.core).expect("Core");
    let CanonicalValueV1::Map(fields) = &mut core else { panic!("Core map") };
    fields.push((text("functions"), functions));
    fixture.core = canonical_bytes(&core);
    let core_reference = raw_ref(names.application, "edict.core.module/v1", &fixture.core);
    let mut target = decode_canonical_cbor_v1(&fixture.target_ir).expect("Target");
    *map_field_mut(map_field_mut(&mut target, "semanticClosure"), "sourceCore") = map([
        ("id", text(names.application)),
        ("digest", CanonicalValueV1::Array(vec![text("sha256"), CanonicalValueV1::Bytes(core_reference.digest.clone())])),
    ]);
    fixture.target_ir = canonical_bytes(&target);
    // Construct the independent verifier request without asking the lowerer to
    // endorse this changed Core. Rebind the retained package's exact carriers.
    let mut package = decode_canonical_cbor_v1(&package).expect("package");
    let CanonicalValueV1::Bytes(bytes) = map_field(&package, "program") else { panic!("program") };
    let mut program = decode_canonical_cbor_v1(bytes).expect("program");
    for (name, bytes) in [("core_artifact", &fixture.core), ("target_ir_artifact", &fixture.target_ir)] {
        *map_field_mut(&mut program, name) = CanonicalValueV1::Bytes(bytes.clone());
    }
    *map_field_mut(&mut package, "program") = CanonicalValueV1::Bytes(canonical_bytes(&program));
    let closure = map_field_mut(&mut package, "semantic_closure");
    *map_field_mut(closure, "core_identity") = CanonicalValueV1::Bytes(core_reference.digest);
    *map_field_mut(closure, "target_ir_identity") = CanonicalValueV1::Bytes(raw_ref("echo.span-ir/v1", "edict.target-ir.artifact/v1", &fixture.target_ir).digest);
    (names, fixture, canonical_bytes(&package))
}

#[test]
fn lowerer_rejects_unused_malformed_source_function_authority() {
    let (names, fixture, _) = fixture_with_functions(map([("unused", CanonicalValueV1::Null)]));
    assert!(lowerer::lower(lowering_request(names, &fixture)).is_err(),
        "lowerer ignored authenticated malformed source function");
}

#[test]
fn verifier_independently_rejects_unused_malformed_source_function_authority() {
    let (names, fixture, package) = fixture_with_functions(map([("unused", CanonicalValueV1::Null)]));
    match verifier::verify(verification_request(names, &fixture, package)) {
        Err(_) => {}
        Ok(success) => {
            let report = decode_canonical_cbor_v1(&success.outputs[0].artifact.bytes).expect("report");
            assert_ne!(text_field(&report, "outcome"), Some("accepted"),
                "verifier ignored authenticated malformed source function");
        }
    }
}
