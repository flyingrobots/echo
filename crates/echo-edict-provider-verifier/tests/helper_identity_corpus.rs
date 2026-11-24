// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code fails loudly"
)]
//! The verifier's verdicts on the shared helper-identity corpus
//! (flyingrobots/echo#1005), which the lowerer and warp-core must match.

#[path = "../../echo-edict-provider-lowerer/tests/fixtures/helper_identity_corpus.rs"]
#[allow(dead_code, reason = "the compiled program serves the lowerer")]
mod corpus;

#[test]
fn the_verifier_reaches_every_shared_helper_identity_verdict() {
    let all = corpus::cases();
    let mismatched: Vec<_> = all
        .iter()
        .filter(|case| {
            echo_edict_provider_verifier::effect_package::verify_effect_package(&case.package)
                .is_ok()
                != case.accepted
        })
        .map(|case| (case.name, case.accepted))
        .collect();
    assert_eq!(mismatched, Vec::<(&str, bool)>::new(), "(case, expected)");
    assert_eq!(all.len(), 5, "the corpus has every case");
}
