// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The verifier's verdicts on the shared Option corpus, which
//! warp-core and the lowerer must match.
use super::validate;

#[path = "../../../../echo-edict-provider-lowerer/tests/fixtures/option_value_corpus.rs"]
#[allow(dead_code, reason = "runs and the main coordinate serve warp-core")]
mod corpus;
use corpus::cases;

#[test]
fn the_verifier_reaches_every_shared_option_verdict() {
    let all = cases();
    let mismatched: Vec<_> = all
        .iter()
        .filter(|case| validate(&case.core, &case.exports, false, 0).is_ok() != case.accepted)
        .map(|case| (case.name.as_str(), case.accepted))
        .collect();
    assert_eq!(mismatched, Vec::<(&str, bool)>::new(), "(case, expected)");
    assert!(all.len() > 40, "the corpus has {} cases", all.len());
}
