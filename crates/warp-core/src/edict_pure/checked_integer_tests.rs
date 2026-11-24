// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! warp-core's verdicts on the shared checked-integer corpus, which the provider lowerer
//! and verifier must match, and the result of every accepted case's runs.
use super::{
    evaluate::{expression, Meter},
    functions::{check_roots, decode},
    model::{Expr, Helper, MAX_PROGRAM_NODES},
    syntax::Parser,
    values::{field, text_field},
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

#[path = "../../../echo-edict-provider-lowerer/tests/fixtures/checked_integer_corpus.rs"]
mod corpus;
use corpus::{cases, main_coordinate, Case};

/// The helpers warp-core admits, after the intent's boundary types are
/// admitted as the package decoder admits them.
fn judged(case: &Case) -> Result<BTreeMap<String, Helper>, Error> {
    let mut parser = Parser {
        types: field(&case.core, "types")?,
        coordinate: "integers.example@1",
        remaining: MAX_PROGRAM_NODES,
    };
    let intent = field(field(&case.core, "intents")?, "evaluate")?;
    for boundary in ["input", "output"] {
        parser.boundary_ty(text_field(intent, boundary)?, 0)?;
    }
    let helpers = decode(&case.core, &case.exports, &mut parser)?;
    check_roots(&helpers, [], [])?;
    Ok(helpers)
}

fn run(helpers: &BTreeMap<String, Helper>, input: u64) -> Result<Value, Error> {
    let mut meter = Meter::new(EvaluationLimits {
        max_package_bytes: 1 << 20,
        max_input_bytes: 1 << 20,
        max_steps: 4096,
        max_allocated_bytes: 1 << 20,
        max_output_bytes: 1 << 20,
    });
    let call = Expr::Call(
        main_coordinate(),
        vec![Expr::Constant(Value::Integer(i128::from(input)))],
    );
    expression(&call, &BTreeMap::new(), helpers, &mut meter, 0)
}

#[test]
fn warp_core_reaches_every_shared_checked_integer_verdict() {
    let all = cases();
    let mismatched: Vec<_> = all
        .iter()
        .filter(|case| judged(case).is_ok() != case.accepted)
        .map(|case| (case.name.as_str(), case.accepted))
        .collect();
    assert_eq!(mismatched, Vec::<(&str, bool)>::new(), "(case, expected)");
    assert!(all.len() > 60, "the corpus has {} cases", all.len());
}

#[test]
fn warp_core_executes_every_accepted_checked_integer_case() {
    let mut wrong = Vec::new();
    for case in cases().iter().filter(|case| case.accepted) {
        assert!(!case.runs.is_empty(), "{} has no run", case.name);
        let Ok(helpers) = judged(case) else {
            wrong.push((case.name.clone(), None, None));
            continue;
        };
        for (input, expected) in &case.runs {
            let actual = run(&helpers, *input).ok();
            if actual.as_ref() != Some(expected) {
                wrong.push((case.name.clone(), Some(*input), actual));
            }
        }
    }
    assert_eq!(wrong, Vec::new(), "(case, input, actual)");
}
