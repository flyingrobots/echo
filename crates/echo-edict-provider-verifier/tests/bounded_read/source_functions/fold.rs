// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Native read-provider fold relations, not public compiler provenance.
use super::*;

// A 1 MiB FactBytes state is copied by the retain helper at each iteration.
// These adapter-owned ceilings cover three iterations and the existing read.
const STEPS: u64 = 16_777_216;
const ALLOCATION: u64 = 67_108_864;

fn expression(call: &Value, bound: u64, missing_step: bool) -> Value {
    let Value::Array(args) = field(call, "args") else {
        panic!("retain arguments")
    };
    map([
        ("kind", text("fold")),
        ("initial", args[0].clone()),
        (
            "step",
            if missing_step {
                text("jedit.text.replace_range@1.missing")
            } else {
                field(call, "callee").clone()
            },
        ),
        ("bound", Value::Integer(i128::from(bound))),
        ("stateType", text("jedit.text@1.FactBytes")),
    ])
}
fn edit_core(core: &mut Value, bound: u64, case: &str) {
    *field_mut(core, "requiredCoreCapabilities") = Value::Array(if case == "missing capability" {
        vec![]
    } else {
        vec![text("core.fold/v1")]
    });
    if case == "functionless" {
        let Value::Map(fields) = core else {
            panic!("Core map")
        };
        fields.retain(|(key, _)| key != &text("functions"));
    }
    let Value::Array(nodes) = field_mut(field_mut(intent(core), "body"), "nodes") else {
        panic!("nodes")
    };
    let node = nodes.last_mut().expect("retain binding");
    *field_mut(node, "value") = expression(field(node, "value"), bound, case == "missing step");
}
fn edit_target(intent: &mut Value, bound: u64, case: &str) {
    let Value::Array(bindings) = field_mut(intent, "pureBindings") else {
        panic!("bindings")
    };
    let node = bindings.last_mut().expect("retain binding");
    *field_mut(node, "value") = expression(field(node, "value"), bound, case == "missing step");
}
fn source(bound_value: u64, case: &str) -> LoweringRequestV1 {
    let mut source = read_helper_with_limits(STEPS, Some(ALLOCATION));
    let mut core = decode(&source.core.artifact.bytes).expect("Core");
    let mut target = decode(&source.semantic_inputs[5].artifact.artifact.bytes).expect("Target");
    edit_core(&mut core, bound_value, case);
    edit_target(intent(&mut target), bound_value, case);
    source.core = bound(
        &source.core.reference.coordinate,
        "edict.core.module/v1",
        encode(&core).expect("Core"),
    );
    *field_mut(
        field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore"),
        "digest",
    ) = Value::Array(vec![
        text("sha256"),
        Value::Bytes(source.core.reference.digest.bytes.clone()),
    ]);
    let artifact = &mut source.semantic_inputs[5].artifact;
    *artifact = bound(
        &artifact.reference.coordinate,
        &artifact.artifact.domain,
        encode(&target).expect("Target"),
    );
    source
}
fn verification(bound: u64, case: &str) -> verifier::VerificationRequestV1 {
    // Only the unchanged call control is lowered. Fold candidates are rebound
    // independently, so a lowerer rejection cannot hide verifier acceptance.
    let mut candidate = request(read_helper_with_limits(STEPS, Some(ALLOCATION)));
    changed_core(&mut candidate, |core| edit_core(core, bound, case));
    changed_target(&mut candidate, |intent| edit_target(intent, bound, case));
    candidate
}
#[test]
fn bounded_read_fold_accepts_zero_and_repeated_fact_bytes_state_independently() {
    for bound in [0, 1, 2, 3] {
        let source = source(bound, "valid");
        let lowered = lower(source.clone()).expect("fold after real read lowers");
        assert_eq!(lowered, lower(source).expect("repeat lowering"));
        let candidate = verification(bound, "valid");
        let response = verifier::verify(candidate.clone()).expect("independent judgment");
        let report = decode(&response.outputs[0].artifact.bytes).expect("report");
        assert_eq!(
            field(&report, "outcome"),
            &text("accepted"),
            "bound {bound}"
        );
        assert_eq!(
            response,
            verifier::verify(candidate).expect("repeat verification")
        );
    }
}
#[test]
fn bounded_read_fold_refuses_missing_authority_even_at_zero() {
    for bound in [0, 3] {
        for case in ["missing capability", "missing step", "functionless"] {
            assert!(lower(source(bound, case)).is_err(), "{case}, bound {bound}");
            assert_rejected(verification(bound, case));
        }
    }
}
#[test]
fn bounded_read_fold_refuses_cost_overflow() {
    assert!(lower(source(u64::MAX, "valid")).is_err());
    assert_rejected(verification(u64::MAX, "valid"));
}
#[test]
fn bounded_read_fold_verifier_refuses_coherently_rebound_target_iteration_substitution() {
    let mut candidate = verification(1, "valid");
    changed_target(&mut candidate, |intent| {
        let Value::Array(bindings) = field_mut(intent, "pureBindings") else {
            panic!("bindings")
        };
        *field_mut(
            field_mut(bindings.last_mut().expect("fold binding"), "value"),
            "bound",
        ) = Value::Integer(2);
    });
    assert_rejected(candidate);
}
