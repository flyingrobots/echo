// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independently rebound read guards that count bytes, not public compiler artifacts.
use super::*;
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
fn needle(value: &[u8]) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("bytes")),
                ("value", Value::Bytes(value.to_vec())),
            ]),
        ),
    ])
}
fn guarded(old: Value, case: &str) -> Value {
    let operand = field(&old, "left").clone();
    let counted = match case {
        "two-byte-needle" => needle(b"\r\n"),
        "empty-needle" => needle(b""),
        "integer-needle" => integer(10),
        _ => needle(b"\n"),
    };
    let mut scan = map([
        ("kind", text("call")),
        ("callee", text("core.bytes.count")),
        (
            "typeArgs",
            Value::Array(vec![text("jedit.text@1.FactBytes")]),
        ),
        ("args", Value::Array(vec![operand, counted])),
    ]);
    if case == "wide-type" {
        *field_mut(&mut scan, "typeArgs") = Value::Array(vec![text("Bytes<max=2097152>")]);
    }
    let compare = map([
        ("kind", text("compare")),
        ("op", text("<=")),
        ("left", scan),
        ("right", integer(u64::MAX)),
    ]);
    map([
        ("kind", text("all")),
        ("values", Value::Array(vec![old, compare])),
    ])
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
fn source(case: &str, budget: u64) -> LoweringRequestV1 {
    source_with(case, budget, true)
}
fn source_with(case: &str, budget: u64, helper: bool) -> LoweringRequestV1 {
    let mut source = read_with_budget(budget, helper);
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
fn verification(case: &str, budget: u64) -> verifier::VerificationRequestV1 {
    verification_with(case, budget, true)
}
fn verification_with(case: &str, budget: u64, helper: bool) -> verifier::VerificationRequestV1 {
    // The new guard is never submitted to the lowerer to obtain this carrier.
    let mut request = request(read_with_budget(budget, helper));
    changed_core(&mut request, |core| edit_core(core, case));
    changed_target(&mut request, |intent| edit_target(intent, case));
    request
}
#[test]
fn bounded_read_count_guards_lower_after_a_real_read() {
    lower(source("valid", 4_194_304)).expect("count read guard lowers");
}
#[test]
fn bounded_read_count_guards_verify_independently() {
    let response =
        verifier::verify(verification("valid", 4_194_304)).expect("independent verifier response");
    let report = decode(&response.outputs[0].artifact.bytes).expect("canonical rebound artifact");
    assert_eq!(field(&report, "outcome"), &text("accepted"));
}
#[test]
fn bounded_read_count_rejects_needle_type_and_step_budget_forgeries() {
    for case in [
        "wide-type",
        "two-byte-needle",
        "empty-needle",
        "integer-needle",
    ] {
        assert!(lower(source(case, 4_194_304)).is_err(), "{case}");
        assert_rejected(verification(case, 4_194_304));
    }
    lower(read_helper_with_budget(2_500_000))
        .expect("without scan, baseline fits this exact same step ceiling");
    assert!(lower(source("valid", 2_500_000)).is_err());
    assert_rejected(verification("valid", 2_500_000));
}
#[test]
fn a_count_guard_is_charged_even_without_a_function_table() {
    // With no source functions and no addition, the count's full-byte scan is
    // still the only judgment that charges it against the step ceiling.
    lower(read_with_budget(2_500_000, false))
        .expect("without scan, the plain read fits this step ceiling");
    lower(source_with("valid", 4_194_304, false)).expect("a fitting count guard lowers");
    let response = verifier::verify(verification_with("valid", 4_194_304, false))
        .expect("independent verifier response");
    let report = decode(&response.outputs[0].artifact.bytes).expect("canonical report");
    assert_eq!(field(&report, "outcome"), &text("accepted"));
    assert!(lower(source_with("valid", 2_500_000, false)).is_err());
    assert_rejected(verification_with("valid", 2_500_000, false));
}
