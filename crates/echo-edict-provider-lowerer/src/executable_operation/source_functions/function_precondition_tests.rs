// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The lowerer's verdicts on the shared function-precondition corpus, which
//! warp-core and the verifier must match.
use super::validate;

#[path = "../../../tests/fixtures/function_precondition_corpus.rs"]
mod corpus;
use corpus::cases;

#[test]
fn the_lowerer_reaches_every_shared_function_precondition_verdict() {
    let all = cases();
    let mismatched: Vec<_> = all
        .iter()
        .filter(|case| validate(&case.core, &case.exports, false, 0).is_ok() != case.accepted)
        .map(|case| (case.name.as_str(), case.accepted))
        .collect();
    assert_eq!(mismatched, Vec::<(&str, bool)>::new(), "(case, expected)");
    assert!(all.len() > 80, "the corpus has {} cases", all.len());
}
