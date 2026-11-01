// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Real read-producer relation with synthetic, independently rebound list calls.
use super::*;

const STEPS: u64 = 16_777_216;
const ALLOCATION: u64 = 67_108_864;

fn value(call: &mut Value, case: &str) {
    let Value::Array(args) = field_mut(call, "args") else {
        panic!("arguments")
    };
    let count = match case {
        "empty" => 0,
        "too many" => 3,
        _ => 2,
    };
    let mut list = map([
        ("kind", text("list")),
        ("values", Value::Array(vec![args[0].clone(); count])),
    ]);
    if case == "malformed" {
        insert(&mut list, "unexpected", Value::Bool(true));
    } else if case == "wrong element" {
        *field_mut(&mut list, "values") = Value::Array(vec![map([
            ("kind", text("const")),
            (
                "value",
                map([
                    ("kind", text("int")),
                    ("width", text("U64")),
                    ("value", Value::Integer(1)),
                ]),
            ),
        ])]);
    }
    args.push(list);
}

fn edit_core(core: &mut Value, case: &str) {
    let function = field_mut(field_mut(core, "functions"), "retain");
    let Value::Array(params) = field_mut(function, "params") else {
        panic!("params")
    };
    params.push(local("arg.1", "List<jedit.text@1.FactBytes,max=2>"));
    let Value::Array(nodes) = field_mut(field_mut(intent(core), "body"), "nodes") else {
        panic!("nodes")
    };
    value(field_mut(nodes.last_mut().expect("retain"), "value"), case);
    let Value::Map(fields) = core else {
        panic!("Core")
    };
    let omitted = match case {
        "missing capability" => Some("requiredCoreCapabilities"),
        "missing helper" => Some("functions"),
        _ => None,
    };
    if let Some(name) = omitted {
        fields.retain(|(key, _)| key != &text(name));
    }
}
fn edit_target(intent: &mut Value, case: &str) {
    let Value::Array(bindings) = field_mut(intent, "pureBindings") else {
        panic!("bindings")
    };
    value(
        field_mut(bindings.last_mut().expect("retain"), "value"),
        case,
    );
}
fn source(case: &str) -> LoweringRequestV1 {
    let mut source = read_helper_with_limits(STEPS, Some(ALLOCATION));
    let mut core = decode(&source.core.artifact.bytes).expect("Core");
    let mut target = decode(&source.semantic_inputs[5].artifact.artifact.bytes).expect("Target");
    edit_core(&mut core, case);
    edit_target(intent(&mut target), case);
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
fn verification(case: &str) -> verifier::VerificationRequestV1 {
    // The lowerer only sees the unchanged scalar control.
    let mut candidate = request(read_helper_with_limits(STEPS, Some(ALLOCATION)));
    changed_core(&mut candidate, |core| edit_core(core, case));
    changed_target(&mut candidate, |intent| edit_target(intent, case));
    candidate
}

#[test]
fn bounded_read_lists_follow_an_authenticated_read_producer() {
    for case in ["empty", "nonempty"] {
        lower(source(case)).expect("read list arguments lower");
        let response = verifier::verify(verification(case)).expect("independent judgment");
        let report = decode(&response.outputs[0].artifact.bytes).expect("report");
        assert_eq!(field(&report, "outcome"), &text("accepted"), "{case}");
    }
}
#[test]
fn bounded_read_lists_refuse_wrong_bounds_shapes_elements_and_missing_authority() {
    for case in [
        "too many",
        "malformed",
        "wrong element",
        "missing capability",
        "missing helper",
    ] {
        assert!(lower(source(case)).is_err(), "{case}");
        assert_rejected(verification(case));
    }
}
#[test]
fn bounded_read_lists_refuse_coherently_rebound_target_value_substitution() {
    let mut candidate = verification("nonempty");
    changed_target(&mut candidate, |intent| {
        let Value::Array(bindings) = field_mut(intent, "pureBindings") else {
            panic!("bindings")
        };
        let Value::Array(args) = field_mut(
            field_mut(bindings.last_mut().expect("retain"), "value"),
            "args",
        ) else {
            panic!("arguments")
        };
        *field_mut(&mut args[1], "values") = Value::Array(vec![]);
    });
    assert_rejected(candidate);
}
