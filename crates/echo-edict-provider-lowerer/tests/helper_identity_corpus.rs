// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code fails loudly"
)]
//! The lowerer's verdicts on the shared helper-identity corpus
//! (flyingrobots/echo#1005), which the verifier and warp-core must match.

#[path = "fixtures/helper_identity_corpus.rs"]
mod corpus;

#[test]
fn the_lowerer_reaches_every_shared_helper_identity_verdict() {
    let all = corpus::cases();
    let mismatched: Vec<_> = all
        .iter()
        .filter(|case| {
            echo_edict_provider_lowerer::effect_publication::judge_effect_program(
                &case.compiled_program(),
            )
            .is_ok()
                != case.accepted
        })
        .map(|case| (case.name, case.accepted))
        .collect();
    assert_eq!(mismatched, Vec::<(&str, bool)>::new(), "(case, expected)");
    assert_eq!(all.len(), 5, "the corpus has every case");
}
