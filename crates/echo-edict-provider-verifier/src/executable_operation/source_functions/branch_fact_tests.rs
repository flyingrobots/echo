// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The verifier's verdicts on the shared branch-fact corpus, which warp-core
//! and the lowerer must match.
use super::validate;

#[path = "../../../../echo-edict-provider-lowerer/tests/fixtures/branch_fact_corpus.rs"]
mod corpus;
use corpus::{cases, guarded_intent, largest_accepted, unneeded_branch_facts, IntentShape};

#[test]
fn the_verifier_reaches_every_shared_branch_fact_verdict() {
    let mismatched: Vec<_> = cases()
        .into_iter()
        .filter(|case| validate(&case.core, &case.exports, false, 0).is_ok() != case.accepted)
        .map(|case| (case.name, case.accepted))
        .collect();
    assert_eq!(mismatched, Vec::<(&str, bool)>::new(), "(case, expected)");
}

/// Jedit's `intent-guarded-add.edict`: the `then` arm's addition is bounded by
/// the branch predicate. The `else` arm gains nothing, and the same bound as
/// an input constraint is the control that shows the rest of the module is
/// admissible.
#[test]
fn a_branch_predicate_bounds_an_intent_addition_in_its_then_arm() {
    let verdicts: Vec<_> = [
        IntentShape::ThenArm,
        IntentShape::ElseArm,
        IntentShape::InputConstraint,
    ]
    .into_iter()
    .map(|shape| {
        let (core, exports) = guarded_intent(shape);
        (shape, validate(&core, &exports, false, 0).is_ok())
    })
    .collect();
    assert_eq!(
        format!("{verdicts:?}"),
        "[(ThenArm, true), (ElseArm, false), (InputConstraint, true)]"
    );
}

/// Branch facts are read only when a judgment refuses without them, so sums
/// that fit by their own literals cost the same work under 64 unneeded
/// bounds as outside them.
#[test]
fn unneeded_branch_facts_cost_a_sum_nothing() {
    let largest = |guarded: bool| {
        largest_accepted(8192, |size| {
            let (core, exports) = unneeded_branch_facts(guarded, size);
            validate(&core, &exports, false, 0).is_ok()
        })
    };
    let outside = largest(false);
    assert!(
        matches!(outside, Some(limit) if limit < 8192),
        "the bisection found a real limit: {outside:?}"
    );
    assert_eq!(largest(true), outside);
}
