// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Semantic relation checks over retained public-compiler output.
use super::*;

pub(super) fn compiler_fixture() -> (FixtureNames<'static>, RawFixture, Vec<u8>) {
    let names = FixtureNames {
        application: "jedit.text.replace_range@1",
        intent: "replaceRange",
        lawpack: "jedit.text@1",
        lawpack_id: "jedit.text",
        lawpack_version: "1",
        exports: "jedit.text.exports/v1",
        adapter: "jedit.text.echo-adapter/v1",
        configuration: "echo.operation-lowering-configuration/v1",
        ..FIXTURES[0]
    };
    let hex = include_str!("../fixtures/compiler-produced-pure/package.cbor.hex").trim();
    let package: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).expect("retained hex"))
        .collect();
    let decoded = decode_canonical_cbor_v1(&package).expect("retained package");
    let program =
        decode_canonical_cbor_v1(&bytes_field(&decoded, "program")).expect("retained program");
    let fixture = RawFixture {
        core: bytes_field(&program, "core_artifact"),
        target_ir: bytes_field(&program, "target_ir_artifact"),
        exports: bytes_field(&program, "lawpack_exports_artifact"),
        result_projection: bytes_field(&program, "result_projection_artifact"),
        target_profile: TARGET_PROFILE.to_vec(),
        source: canonical_bytes(&CanonicalValueV1::Bytes(
            include_bytes!("../fixtures/compiler-produced-pure/source.edict").to_vec(),
        )),
        lawpack: include_bytes!("../fixtures/compiler-produced-pure/lawpack.cbor").to_vec(),
        adapter: include_bytes!("../fixtures/compiler-produced-pure/adapter.cbor").to_vec(),
        configuration: include_bytes!("../fixtures/compiler-produced-pure/configuration.cbor")
            .to_vec(),
    };
    (names, fixture, package)
}

fn bytes_field(value: &CanonicalValueV1, name: &str) -> Vec<u8> {
    let CanonicalValueV1::Bytes(bytes) = map_field(value, name) else {
        panic!("retained byte field");
    };
    bytes.clone()
}

#[test]
fn retained_compiler_package_round_trips_through_independent_provider() {
    let (names, fixture, package) = compiler_fixture();
    assert_eq!(lower_package(names, &fixture), package);
    let accepted = verifier::verify(verification_request(names, &fixture, package))
        .expect("valid compiler relation");
    let report = decode_canonical_cbor_v1(&accepted.outputs[0].artifact.bytes).expect("report");
    assert_eq!(text_field(&report, "outcome"), Some("accepted"));
}

#[test]
fn verifier_refuses_changed_target_literal_with_unchanged_core() {
    assert_target_mutation_refused(|target| assert!(replace_unsigned_one(target)));
}

#[test]
fn verifier_refuses_dropped_input_constraints() {
    assert_target_mutation_refused(|target| {
        let CanonicalValueV1::Array(constraints) =
            map_field_mut(target_intent(target), "inputConstraints")
        else {
            panic!("input constraints");
        };
        assert!(!constraints.is_empty());
        constraints.clear();
    });
}

#[test]
fn verifier_refuses_changed_result() {
    assert_target_mutation_refused(|target| {
        let intent = target_intent(target);
        let basis = map_field(intent, "basis").clone();
        assert_ne!(map_field(intent, "result"), &basis);
        *map_field_mut(intent, "result") = basis;
    });
}

#[test]
fn verifier_refuses_dropped_basis() {
    assert_target_mutation_refused(|target| {
        let CanonicalValueV1::Map(fields) = target_intent(target) else {
            panic!("intent map");
        };
        let before = fields.len();
        fields.retain(|(key, _)| key != &text("basis"));
        assert_eq!(fields.len(), before - 1);
    });
}

#[test]
fn verifier_refuses_extra_target_intent() {
    assert_target_mutation_refused(|target| {
        let CanonicalValueV1::Map(intents) = map_field_mut(target, "intents") else {
            panic!("intents map");
        };
        intents.push((text("unownedIntent"), intents[0].1.clone()));
    });
}

#[test]
fn verifier_refuses_duplicated_binding() {
    assert_target_mutation_refused(|target| {
        let CanonicalValueV1::Array(bindings) =
            map_field_mut(target_intent(target), "pureBindings")
        else {
            panic!("pure bindings");
        };
        bindings.push(bindings[0].clone());
    });
}

#[test]
fn verifier_refuses_reordered_bindings_even_with_canonical_ids() {
    assert_target_mutation_refused(|target| {
        let CanonicalValueV1::Array(bindings) =
            map_field_mut(target_intent(target), "pureBindings")
        else {
            panic!("pure bindings");
        };
        assert!(bindings.len() >= 2);
        bindings.swap(0, 1);
        for (index, binding) in bindings.iter_mut().enumerate() {
            *map_field_mut(binding, "id") = text(format!("replaceRange.binding.{index}"));
        }
    });
}

fn target_intent(target: &mut CanonicalValueV1) -> &mut CanonicalValueV1 {
    map_field_mut(map_field_mut(target, "intents"), "replaceRange")
}

fn assert_target_mutation_refused(mutate: impl FnOnce(&mut CanonicalValueV1)) {
    let (names, mut fixture, _) = compiler_fixture();
    let mut target = decode_canonical_cbor_v1(&fixture.target_ir).expect("target");
    mutate(&mut target);
    fixture.target_ir = canonical_bytes(&target);
    let package = lower_package(names, &fixture);
    match verifier::verify(verification_request(names, &fixture, package)) {
        Err(refusal) => assert_eq!(
            refusal.kind,
            verifier::ProviderRefusalKind::UnsupportedSemantics
        ),
        Ok(result) => {
            let report = decode_canonical_cbor_v1(&result.outputs[0].artifact.bytes)
                .expect("verification report");
            assert_eq!(text_field(&report, "outcome"), Some("rejected"));
        }
    }
}

fn replace_unsigned_one(value: &mut CanonicalValueV1) -> bool {
    if text_field(value, "width") == Some("U32")
        && matches!(map_field(value, "value"), CanonicalValueV1::Integer(1))
    {
        *map_field_mut(value, "value") = CanonicalValueV1::Integer(2);
        return true;
    }
    match value {
        CanonicalValueV1::Map(entries) => entries
            .iter_mut()
            .any(|(_, value)| replace_unsigned_one(value)),
        CanonicalValueV1::Array(values) => values.iter_mut().any(replace_unsigned_one),
        _ => false,
    }
}

#[test]
fn verifier_refuses_projection_that_changes_an_output_field() {
    assert_projection_mutation_refused(|projection| {
        let fields = map_field_mut(map_field_mut(projection, "expression"), "fields");
        let end = map_field(fields, "endByte").clone();
        assert_ne!(map_field(fields, "startByte"), &end);
        *map_field_mut(fields, "startByte") = end;
    });
}

#[test]
fn verifier_refuses_projection_with_a_different_output_contract() {
    assert_projection_mutation_refused(|projection| {
        *map_field_mut(projection, "outputType") = text("unrelated.Output");
    });
}

#[test]
fn verifier_refuses_projection_with_a_different_output_budget() {
    assert_projection_mutation_refused(|projection| {
        let CanonicalValueV1::Integer(limit) = map_field_mut(projection, "maxOutputBytes") else {
            panic!("output limit");
        };
        *limit += 1;
    });
}

#[test]
fn verifier_refuses_projection_with_a_dropped_result_field() {
    assert_projection_mutation_refused(|projection| {
        let CanonicalValueV1::Map(fields) =
            map_field_mut(map_field_mut(projection, "expression"), "fields")
        else {
            panic!("projection fields");
        };
        assert!(fields.pop().is_some());
    });
}

#[test]
fn verifier_refuses_projection_rebound_to_another_declared_binding() {
    assert_projection_mutation_refused(|projection| {
        let fields = map_field_mut(map_field_mut(projection, "expression"), "fields");
        let other_source = map_field(map_field(fields, "createdLeafCeiling"), "source").clone();
        let source = map_field_mut(map_field_mut(fields, "rangeIsEmpty"), "source");
        assert_eq!(text_field(source, "kind"), Some("pureBinding"));
        assert_ne!(source, &other_source);
        *source = other_source;
    });
}

fn assert_projection_mutation_refused(mutate: impl FnOnce(&mut CanonicalValueV1)) {
    let (names, mut fixture, _) = compiler_fixture();
    let mut projection = decode_canonical_cbor_v1(&fixture.result_projection).expect("projection");
    mutate(&mut projection);
    fixture.result_projection = canonical_bytes(&projection);
    let package = lower_package(names, &fixture);
    match verifier::verify(verification_request(names, &fixture, package)) {
        Err(refusal) => assert_eq!(
            refusal.kind,
            verifier::ProviderRefusalKind::UnsupportedSemantics
        ),
        Ok(result) => {
            let report =
                decode_canonical_cbor_v1(&result.outputs[0].artifact.bytes).expect("report");
            assert_eq!(text_field(&report, "outcome"), Some("rejected"));
        }
    }
}
