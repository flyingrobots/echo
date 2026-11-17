// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! warp-core's verdicts on the shared function-precondition corpus, which the
//! provider lowerer and verifier must match. warp-core judges every helper
//! body and every call to a precondition-bearing function in the module when
//! it decodes the module's helpers.
use super::{
    functions::decode, model::MAX_PROGRAM_NODES, syntax::Parser, values::field,
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;

#[path = "../../../echo-edict-provider-lowerer/tests/fixtures/function_precondition_corpus.rs"]
mod corpus;
use corpus::cases;

fn judged(core: &Value, exports: &Value) -> Result<(), Error> {
    let mut parser = Parser {
        types: field(core, "types")?,
        coordinate: "preconditions.example@1",
        remaining: MAX_PROGRAM_NODES,
    };
    decode(core, exports, &mut parser).map(|_| ())
}

#[test]
fn warp_core_reaches_every_shared_function_precondition_verdict() {
    let all = cases();
    let mismatched: Vec<_> = all
        .iter()
        .filter(|case| judged(&case.core, &case.exports).is_ok() != case.accepted)
        .map(|case| (case.name.as_str(), case.accepted))
        .collect();
    assert_eq!(mismatched, Vec::<(&str, bool)>::new(), "(case, expected)");
    assert!(all.len() > 80, "the corpus has {} cases", all.len());
}
