// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Effect programs under the shared-read Core cost schedule (echo#1016).
//!
//! Requiring `core.cost.shared-reads/v1` changes how an effect program's
//! expressions are metered, never what it computes. Oracle: the same package
//! without the capability, run by the copying executor on the same input and
//! graph (a differential).
use super::*;

const SHARED: &str = "core.cost.shared-reads/v1";

/// The package with its Core requiring one more capability, repinned.
fn requiring(package: &V, capability: &str) -> V {
    rebuild(package, |core, _, _| {
        let V::Array(capabilities) = value_field_mut(core, "requiredCoreCapabilities") else {
            panic!("capabilities are a list")
        };
        capabilities.push(text(capability));
    })
}

/// The package with its Core requiring the shared-read schedule, repinned.
pub(super) fn shared(package: &V) -> V {
    requiring(package, SHARED)
}

/// Control for the repinning itself: an unused capability added the same way
/// still prepares, so a refusal below is the schedule's, not the edit's.
#[test]
fn an_unused_capability_added_by_the_same_edit_still_prepares() {
    let package = requiring(&retained("slice-proven"), "core.variant/v1");
    prepare(&package).expect("the repinned control prepares");
}

/// Both schedules reach the same result, obstruction and mutations.
fn assert_same_effect(name: &str, package: &V, input: &[u8], frame: &Frame) {
    let (copying, copying_mutations, _, _) = run_traced(package, input, frame);
    let (sharing, sharing_mutations, _, _) = run_traced(&shared(package), input, frame);
    assert_eq!(sharing, copying, "{name}: result");
    assert_eq!(sharing_mutations, copying_mutations, "{name}: mutations");
}

#[test]
fn a_read_whose_require_fails_obstructs_as_under_the_copying_schedule() {
    assert_same_effect(
        "read-control",
        &retained("read-control"),
        &read_input(),
        &Frame::widest(Some(64)),
    );
}

/// Byte operations under the shared schedule (echo#1016, family 5): each
/// program, given exactly the lowerer's shared minimum (as
/// `byte_operation_minimums_under_each_schedule` in
/// `echo-edict-provider-lowerer` pins it), runs to the copying schedule's
/// result. `typed-failure-control` runs both ways through its `attempt`.
#[test]
fn byte_effects_run_within_the_lowerer_minimum() {
    let slice = encoded(&map([
        ("at", V::Integer(1)),
        ("other", V::Bytes(b"xy".to_vec())),
        ("text", V::Bytes(b"hello".to_vec())),
    ]));
    for (name, input, steps, bytes) in [
        ("slice-proven", slice, 870, 971),
        ("typed-failure-control", typed_failure_input(0), 373, 1295),
        ("typed-failure-control", typed_failure_input(2), 373, 1295),
        ("typed-failure-control", typed_failure_input(9), 373, 1295),
    ] {
        let package = retained(name);
        let frame = Frame::widest(None);
        let copying = run_on(&package, &input, &frame);
        let exact = with_allocation(&with_budget(&shared(&package), steps), bytes);
        assert_eq!(run_on(&exact, &input, &frame), copying, "{name}");
    }
}

#[test]
fn structured_loops_emit_the_same_mutations_as_the_copying_schedule() {
    assert_same_effect(
        "structured-loop-control",
        &retained("structured-loop-control"),
        &largest_structured_loop_input(),
        &Frame::widest(None),
    );
}

#[test]
fn a_node_ensure_emits_the_same_mutation_as_the_copying_schedule() {
    assert_same_effect(
        "ensure-node-control",
        &retained("ensure-node-control"),
        &largest_ensure_node_input(),
        &Frame::widest(None),
    );
}

#[test]
fn structured_branches_and_nesting_match_the_copying_schedule() {
    for name in ["structured-branch-control", "structured-nested-control"] {
        assert_same_effect(
            name,
            &retained(name),
            &largest_structured_loop_input(),
            &Frame::widest(None),
        );
    }
}

/// The provider lowerer's minimum budgets for each program under the shared
/// schedule, as `effect_program_minimums_under_each_schedule` in
/// `echo-edict-provider-lowerer` measures and pins them: (steps, bytes).
const LOWERER_SHARED_MINIMUMS: [(&str, u64, u64); 5] = [
    ("read-control", 2_097_257, 4_195_841),
    ("ensure-node-control", 29, 2301),
    ("structured-loop-control", 107, 6903),
    ("structured-branch-control", 177, 7031),
    ("structured-nested-control", 254, 13_742),
];

/// The runtime never charges more than the lowerer's static bound: each
/// program, given exactly the lowerer's shared minimum as its Core budget,
/// runs its largest input here to the copying schedule's result. The read
/// runs a 400,000-byte atom, the largest the frame's fixed step ceiling lets
/// either schedule read.
#[test]
fn effect_runs_never_exceed_the_lowerer_minimum() {
    for (name, steps, bytes) in LOWERER_SHARED_MINIMUMS {
        let (input, frame) = match name {
            "read-control" => (read_input(), Frame::widest(Some(400_000))),
            "ensure-node-control" => (largest_ensure_node_input(), Frame::widest(None)),
            _ => (largest_structured_loop_input(), Frame::widest(None)),
        };
        let package = retained(name);
        let exact = with_allocation(&with_budget(&shared(&package), steps), bytes);
        let copying = run_on(&package, &input, &frame);
        assert_eq!(run_on(&exact, &input, &frame), copying, "{name}");
    }
}
