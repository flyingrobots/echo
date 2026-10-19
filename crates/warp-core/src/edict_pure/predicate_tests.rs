// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Closed predicate decoding, metering and combined traversal boundaries.
use super::{
    evaluate::{predicate, Meter},
    functions::check_roots,
    model::{Comparison, Expr, Predicate, MAX_PROGRAM_NODES},
    syntax::Parser,
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;
fn limits(steps: u64) -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 32768,
        max_input_bytes: 4194304,
        max_steps: steps,
        max_allocated_bytes: 16777216,
        max_output_bytes: 4194304,
    }
}
fn compare(left: i128, right: i128) -> Predicate {
    Predicate::Compare {
        op: Comparison::Equal,
        left: Expr::Constant(Value::Integer(left)),
        right: Expr::Constant(Value::Integer(right)),
    }
}
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}
fn all(values: Vec<Value>) -> Value {
    map([
        ("kind", Value::Text("all".into())),
        ("values", Value::Array(values)),
    ])
}
fn parse(value: &Value, remaining: usize) -> Result<Predicate, Error> {
    Parser {
        types: &Value::Map(vec![]),
        coordinate: "generic.predicates@1",
        remaining,
    }
    .predicate(value, 0)
}
#[test]
fn conjunction_truth_and_exact_short_circuit_metering() -> Result<(), Error> {
    // Compare charges one predicate step plus expression/copy steps for each constant.
    for (values, expected, steps) in [
        (vec![], true, 1),
        (vec![compare(1, 1)], true, 6),
        (vec![compare(1, 1), compare(2, 2)], true, 11),
        (vec![compare(1, 2), compare(2, 2)], false, 6),
        (vec![compare(1, 1), compare(2, 3)], false, 11),
        (vec![Predicate::All(vec![]), compare(1, 1)], true, 7),
    ] {
        let value = Predicate::All(values);
        let mut meter = Meter::new(limits(steps));
        assert_eq!(
            predicate(&value, &BTreeMap::new(), &BTreeMap::new(), &mut meter, 0)?,
            expected
        );
        assert_eq!(meter.usage().0, steps);
        let mut too_small = Meter::new(limits(steps - 1));
        assert_eq!(
            predicate(
                &value,
                &BTreeMap::new(),
                &BTreeMap::new(),
                &mut too_small,
                0
            ),
            Err(Error::StepBudgetExceeded)
        );
    }
    Ok(())
}
#[test]
fn conjunction_decoder_checks_closed_shape_and_all_children() {
    let empty = all(vec![]);
    assert_eq!(parse(&empty, MAX_PROGRAM_NODES), Ok(Predicate::All(vec![])));
    for value in [
        map([("kind", Value::Text("all".into()))]),
        map([
            ("kind", Value::Text("all".into())),
            ("values", Value::Integer(0)),
        ]),
        map([
            ("kind", Value::Text("all".into())),
            ("values", Value::Array(vec![])),
            ("extra", Value::Integer(0)),
        ]),
    ] {
        assert_eq!(
            parse(&value, MAX_PROGRAM_NODES),
            Err(Error::InvalidArtifact)
        );
    }
    let unknown = map([("kind", Value::Text("unsupported".into()))]);
    assert!(matches!(
        parse(&all(vec![empty, unknown]), MAX_PROGRAM_NODES),
        Err(Error::InvalidArtifact | Error::UnsupportedProgram)
    ));
}
#[test]
fn conjunction_decoder_and_combined_depth_refuse_exact_boundary_overruns() {
    assert_eq!(
        parse(&all(vec![all(vec![])]), 1),
        Err(Error::UnsupportedProgram)
    );
    for (wrappers, accepted) in [(63, true), (64, false), (65, false)] {
        let mut value = all(vec![]);
        for _ in 0..wrappers {
            value = all(vec![value]);
        }
        let result = parse(&value, MAX_PROGRAM_NODES).and_then(|value| {
            check_roots(&BTreeMap::new(), [], [&value])?;
            Ok(value)
        });
        if accepted {
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(Error::UnsupportedProgram));
        }
    }
}
