// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independently rebound read guards that slice the read fact at a runtime
//! offset, as Jim's character-boundary check does, not compiler artifacts.
use super::*;
const FACT: &str = "jedit.text@1.FactBytes";
/// One slice and one UTF-8 scan of the 1 MiB fact, above the read baseline.
const BUDGET: u64 = 8_388_608;
/// Each reference to a 1 MiB value is charged as a full copy, so the guarded
/// boundary check needs slightly more than the fixture's 16 MiB.
const ALLOCATION: u64 = 33_554_432;
fn call(callee: &str, args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        ("typeArgs", Value::Array(vec![text(FACT)])),
        ("args", Value::Array(args)),
    ])
}
fn integer(value: u64) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", Value::Integer(i128::from(value))),
            ]),
        ),
    ])
}
fn compare(op: &str, left: Value, right: Value) -> Value {
    map([
        ("kind", text("compare")),
        ("op", text(op)),
        ("left", left),
        ("right", right),
    ])
}
/// `at <= len(actual) && isUtf8(slice(actual, 0, at))`, with the runtime
/// offset `at = len(input.expected)`.
fn guarded(old: Value, case: &str) -> Value {
    let actual = field(&old, "left").clone();
    let at = call("core.bytes.length", vec![field(&old, "right").clone()]);
    let guard = compare(
        "<=",
        at.clone(),
        call("core.bytes.length", vec![actual.clone()]),
    );
    let boundary = compare(
        "==",
        call(
            "core.bytes.isUtf8",
            vec![call("core.bytes.slice", vec![actual, integer(0), at])],
        ),
        map([
            ("kind", text("const")),
            (
                "value",
                map([("kind", text("bool")), ("value", Value::Bool(true))]),
            ),
        ]),
    );
    let children = match case {
        "unguarded" => vec![old, boundary],
        "guard after use" => vec![old, boundary, guard],
        _ => vec![old, guard, boundary],
    };
    map([("kind", text("all")), ("values", Value::Array(children))])
}
fn edit_core(core: &mut Value, case: &str) {
    let Value::Array(nodes) = field_mut(field_mut(intent(core), "body"), "nodes") else {
        panic!("nodes")
    };
    let old = field(&nodes[1], "predicate").clone();
    *field_mut(&mut nodes[1], "predicate") = guarded(old, case);
}
fn edit_target(intent: &mut Value, case: &str) {
    let Value::Array(requirements) = field_mut(intent, "requirements") else {
        panic!("requirements")
    };
    let old = field(&requirements[0], "predicate").clone();
    *field_mut(&mut requirements[0], "predicate") = guarded(old, case);
}
fn source(case: &str) -> LoweringRequestV1 {
    let mut source = read_helper_with_limits(BUDGET, Some(ALLOCATION));
    let mut core = decode(&source.core.artifact.bytes).expect("canonical rebound artifact");
    let mut target = decode(&source.semantic_inputs[5].artifact.artifact.bytes)
        .expect("canonical rebound artifact");
    edit_core(&mut core, case);
    edit_target(intent(&mut target), case);
    source.core = bound(
        "jedit.text.replace_range@1",
        "edict.core.module/v1",
        encode(&core).expect("canonical rebound artifact"),
    );
    *field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore") = map([
        ("id", text("jedit.text.replace_range@1")),
        (
            "digest",
            Value::Array(vec![
                text("sha256"),
                Value::Bytes(source.core.reference.digest.bytes.clone()),
            ]),
        ),
    ]);
    source.semantic_inputs[5].artifact = bound(
        "echo.span-ir/v2",
        "edict.target-ir.artifact/v1",
        encode(&target).expect("canonical rebound artifact"),
    );
    source
}
fn verification(case: &str) -> verifier::VerificationRequestV1 {
    // The new guard is never submitted to the lowerer to obtain this carrier.
    let mut request = request(read_helper_with_limits(BUDGET, Some(ALLOCATION)));
    changed_core(&mut request, |core| edit_core(core, case));
    changed_target(&mut request, |intent| edit_target(intent, case));
    request
}
#[test]
fn bounded_read_runtime_slices_lower_behind_a_conjunct_guard() {
    lower(source("valid")).expect("guarded runtime slice lowers");
}
#[test]
fn bounded_read_runtime_slices_verify_independently() {
    let response = verifier::verify(verification("valid")).expect("independent verifier response");
    let report = decode(&response.outputs[0].artifact.bytes).expect("canonical rebound artifact");
    assert_eq!(field(&report, "outcome"), &text("accepted"));
}
#[test]
fn bounded_read_runtime_slices_without_a_dominating_guard_refuse() {
    for case in ["unguarded", "guard after use"] {
        assert!(lower(source(case)).is_err(), "{case}");
        assert_rejected(verification(case));
    }
}
