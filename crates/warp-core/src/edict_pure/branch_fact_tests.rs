// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! warp-core's verdicts on the shared branch-fact corpus, which the provider
//! lowerer and verifier must match. warp-core judges helper bodies when it
//! prepares a package, so every case is a helper.
use super::{
    functions::decode, model::MAX_PROGRAM_NODES, syntax::Parser, values::field,
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;

#[path = "../../../echo-edict-provider-lowerer/tests/fixtures/branch_fact_corpus.rs"]
#[allow(
    dead_code,
    reason = "warp-core does not judge pure intent bodies; only the provider tests use the intent fixture"
)]
mod corpus;
use corpus::{cases, largest_accepted, unneeded_branch_facts};

fn judged(core: &Value, exports: &Value) -> Result<(), Error> {
    let mut parser = Parser {
        types: field(core, "types")?,
        coordinate: "generic.branch@1",
        remaining: MAX_PROGRAM_NODES,
    };
    decode(core, exports, &mut parser).map(|_| ())
}

#[test]
fn warp_core_reaches_every_shared_branch_fact_verdict() {
    let mismatched: Vec<_> = cases()
        .into_iter()
        .filter(|case| judged(&case.core, &case.exports).is_ok() != case.accepted)
        .map(|case| (case.name, case.accepted))
        .collect();
    assert_eq!(mismatched, Vec::<(&str, bool)>::new(), "(case, expected)");
}

/// A refused case is refused as unsupported, not as a malformed artifact.
#[test]
fn refused_branch_fact_cases_are_unsupported_programs() {
    let malformed: Vec<_> = cases()
        .into_iter()
        .filter(|case| !case.accepted)
        .filter_map(|case| match judged(&case.core, &case.exports) {
            Err(Error::UnsupportedProgram) => None,
            other => Some((case.name, other)),
        })
        .collect();
    assert_eq!(malformed, Vec::new());
}

/// Branch facts are read only when a judgment refuses without them, so sums
/// that fit by their own literals spend the same program budget under 64
/// unneeded bounds as outside them.
#[test]
fn unneeded_branch_facts_cost_a_sum_nothing() {
    let largest = |guarded: bool| {
        largest_accepted(8192, |size| {
            let (core, exports) = unneeded_branch_facts(guarded, size);
            judged(&core, &exports).is_ok()
        })
    };
    let outside = largest(false);
    assert!(
        matches!(outside, Some(limit) if limit < 8192),
        "the bisection found a real limit: {outside:?}"
    );
    assert_eq!(largest(true), outside);
}
