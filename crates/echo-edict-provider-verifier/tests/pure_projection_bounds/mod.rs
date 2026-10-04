// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Provider projection bounds follow the compiler-owned projection contract.
use super::*;

#[test]
fn lowerer_refuses_extra_pure_projection_fields() {
    assert_lowerer_refuses(add_extra_field);
}

#[test]
fn verifier_refuses_extra_pure_projection_fields_independently() {
    let (names, mut fixture, package) = pure_relation::compiler_fixture();
    let mut projection = decode_canonical_cbor_v1(&fixture.result_projection).expect("projection");
    add_extra_field(&mut projection);
    fixture.result_projection = canonical_bytes(&projection);
    let mut package = decode_canonical_cbor_v1(&package).expect("package");
    let CanonicalValueV1::Bytes(program) = map_field_mut(&mut package, "program") else {
        panic!("program bytes");
    };
    let mut decoded = decode_canonical_cbor_v1(program).expect("program");
    *map_field_mut(&mut decoded, "result_projection_artifact") =
        CanonicalValueV1::Bytes(fixture.result_projection.clone());
    *program = canonical_bytes(&decoded);
    match verifier::verify(verification_request(
        names,
        &fixture,
        canonical_bytes(&package),
    )) {
        Err(refusal) => assert_eq!(
            refusal.kind,
            verifier::ProviderRefusalKind::InvalidSemanticArtifact
        ),
        Ok(success) => {
            let report =
                decode_canonical_cbor_v1(&success.outputs[0].artifact.bytes).expect("report");
            panic!(
                "malformed projection returned {:?}",
                text_field(&report, "outcome")
            );
        }
    }
}

#[test]
fn lowerer_refuses_pure_projection_with_33_path_segments() {
    assert_lowerer_refuses(|projection| {
        let mut source = source();
        *map_field_mut(&mut source, "path") = CanonicalValueV1::Array(vec![text("field"); 33]);
        *map_field_mut(projection, "expression") = source;
    });
}

#[test]
fn lowerer_refuses_pure_projection_with_non_text_path() {
    assert_lowerer_refuses(|projection| {
        let mut source = source();
        *map_field_mut(&mut source, "path") = CanonicalValueV1::Array(vec![integer(1)]);
        *map_field_mut(projection, "expression") = source;
    });
}

#[test]
fn lowerer_refuses_pure_projection_above_text_bound() {
    assert_lowerer_refuses(|projection| {
        *map_field_mut(projection, "outputType") = text("x".repeat(1_025));
    });
}

#[test]
fn lowerer_refuses_pure_projection_above_node_bound() {
    assert_lowerer_refuses(|projection| {
        *map_field_mut(projection, "expression") = record(256, 3);
    });
}

#[test]
fn lowerer_refuses_pure_projection_above_artifact_bound() {
    assert_lowerer_refuses(|projection| {
        // Every text and path fits its bound; 71 nodes fit the aggregate limit.
        // Their combined canonical bytes exceed the separate 64 KiB ceiling.
        *map_field_mut(projection, "expression") = record(70, 1_024);
        assert!(canonical_bytes(projection).len() > 64 * 1_024);
    });
}

#[test]
fn lowerer_accepts_exact_projection_shape_limits() {
    let (names, mut fixture, _) = pure_relation::compiler_fixture();
    let mut projection = decode_canonical_cbor_v1(&fixture.result_projection).expect("projection");
    *map_field_mut(&mut projection, "expression") = record(255, 3);
    *map_field_mut(&mut projection, "outputType") = text("x".repeat(1_024));
    fixture.result_projection = canonical_bytes(&projection);
    lowerer::lower(lowering_request(names, &fixture)).expect("256 nodes and 1024-byte text fit");
    let mut expression = source();
    *map_field_mut(&mut expression, "path") = CanonicalValueV1::Array(vec![text("x"); 32]);
    *map_field_mut(&mut projection, "expression") = expression;
    fixture.result_projection = canonical_bytes(&projection);
    lowerer::lower(lowering_request(names, &fixture)).expect("32 path segments fit");
    // These are structural lowerer witnesses, not Core/result semantic proofs.
}

#[test]
fn pure_output_budget_can_exceed_the_anchored_route_ceiling() {
    let (names, mut fixture, _) = pure_relation::compiler_fixture();
    let mut core = decode_canonical_cbor_v1(&fixture.core).expect("core");
    let intent = map_field_mut(map_field_mut(&mut core, "intents"), names.intent);
    let budget = map_field_mut(intent, "coreEvaluationBudget");
    *map_field_mut(budget, "maxOutputBytes") = integer(65_537);
    *map_field_mut(budget, "maxAllocatedBytes") = integer(131_072);
    let budget = budget.clone();
    fixture.core = canonical_bytes(&core);
    let mut target = decode_canonical_cbor_v1(&fixture.target_ir).expect("target");
    let target_intent = map_field_mut(map_field_mut(&mut target, "intents"), names.intent);
    *map_field_mut(target_intent, "coreEvaluationBudget") = budget;
    *map_field_mut(map_field_mut(&mut target, "semanticClosure"), "sourceCore") = resource_ref(
        &raw_ref(names.application, "edict.core.module/v1", &fixture.core),
    );
    fixture.target_ir = canonical_bytes(&target);
    let mut projection = decode_canonical_cbor_v1(&fixture.result_projection).expect("projection");
    *map_field_mut(&mut projection, "maxOutputBytes") = integer(65_537);
    fixture.result_projection = canonical_bytes(&projection);
    let package = lower_package(names, &fixture);
    let success = verifier::verify(verification_request(names, &fixture, package))
        .expect("pure output budget is declared by Core, not the anchored route");
    let report = decode_canonical_cbor_v1(&success.outputs[0].artifact.bytes).expect("report");
    assert_eq!(text_field(&report, "outcome"), Some("accepted"));
}

fn add_extra_field(projection: &mut CanonicalValueV1) {
    let CanonicalValueV1::Map(fields) = map_field_mut(projection, "expression") else {
        panic!("expression map");
    };
    fields.push((text("ignored"), integer(1)));
}

fn source() -> CanonicalValueV1 {
    owned_map([
        ("kind", text("source")),
        ("source", owned_map([("kind", text("applicationInput"))])),
        ("path", CanonicalValueV1::Array(Vec::new())),
    ])
}

fn record(count: usize, key_bytes: usize) -> CanonicalValueV1 {
    let fields = (0..count)
        .map(|index| {
            let name = format!("{index:03}{}", "x".repeat(key_bytes - 3));
            (text(name), source())
        })
        .collect();
    owned_map([
        ("kind", text("record")),
        ("fields", CanonicalValueV1::Map(fields)),
    ])
}

fn assert_lowerer_refuses(mutate: impl FnOnce(&mut CanonicalValueV1)) {
    let (names, mut fixture, _) = pure_relation::compiler_fixture();
    let mut projection = decode_canonical_cbor_v1(&fixture.result_projection).expect("projection");
    mutate(&mut projection);
    fixture.result_projection = canonical_bytes(&projection);
    let refusal = lowerer::lower(lowering_request(names, &fixture))
        .expect_err("malformed or oversized projection must refuse");
    assert_eq!(
        refusal.kind,
        lowerer::ProviderRefusalKind::InvalidSemanticArtifact
    );
}
