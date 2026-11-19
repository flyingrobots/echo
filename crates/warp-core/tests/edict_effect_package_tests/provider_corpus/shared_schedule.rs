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
fn shared(package: &V) -> V {
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

/// Byte operations have no shared-read charge in Echo yet (echo#1016's last
/// family), so a program using them is refused before any of its work runs.
#[test]
fn byte_operations_are_refused_under_the_shared_schedule() {
    let input = encoded(&map([
        ("at", V::Integer(1)),
        ("other", V::Bytes(b"xy".to_vec())),
        ("text", V::Bytes(b"hello".to_vec())),
    ]));
    let package = retained("slice-proven");
    run(&package, &input).expect("the copying schedule runs the slice");
    assert_eq!(
        run(&shared(&package), &input),
        Err(EffectFrameError::BoundedRead(ReadError::Evaluation(
            EvaluationError::UnsupportedProgram
        )))
    );
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
