// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The runtime meter against the shared `edict.core-cost/v2` corpus.
//!
//! Oracles: each program's budget is the compiler minimum recorded in the
//! corpus (Amblicit `da1574f1`, `edict.core-cost/v2`); each expected result is
//! computed by hand from the program text and also equals the copying
//! evaluator's result. The copying usages are change-detection goldens
//! measured with the copying evaluator as it is on `origin/main` at `ed37a03d`;
//! this change does not touch that evaluator.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test fixtures fail loudly"
)]
use super::{
    evaluate,
    functions::{self, CostSchedule},
    model::{Binding, Program, MAX_PROGRAM_NODES},
    syntax::Parser,
    values::{array, field, text_field},
    EvaluationError as Error, EvaluationLimits, EvaluationResult,
};
use echo_edict_canonical::{
    decode_canonical_cbor_v1, encode_canonical_cbor_v1, CanonicalValueV1 as Value,
};

const READS: &str = include_str!("../../tests/fixtures/edict-core-cost-v2/reads.core.cbor.hex");
const FILL: &str = include_str!("../../tests/fixtures/edict-core-cost-v2/fill.core.cbor.hex");
const NESTED: &str = include_str!("../../tests/fixtures/edict-core-cost-v2/nested.core.cbor.hex");
const MATCHES: &str = include_str!("../../tests/fixtures/edict-core-cost-v2/matches.core.cbor.hex");
const SHARED: &str = "core.cost.shared-reads/v1";

fn core(hex: &str) -> Value {
    let bytes = (0..hex.trim().len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("corpus is hex"))
        .collect::<Vec<_>>();
    decode_canonical_cbor_v1(&bytes).expect("corpus is canonical CBOR")
}

fn entry<'a>(value: &'a mut Value, key: &str) -> &'a mut Value {
    let Value::Map(fields) = value else {
        panic!("expected a map around {key}")
    };
    &mut fields
        .iter_mut()
        .find(|(name, _)| *name == Value::Text(key.into()))
        .unwrap_or_else(|| panic!("missing {key}"))
        .1
}

/// The same module without the shared-read capability: the copying schedule.
fn copying(mut core: Value) -> Value {
    let Value::Array(capabilities) = entry(&mut core, "requiredCoreCapabilities") else {
        panic!("capabilities are a list")
    };
    capabilities.retain(|capability| *capability != Value::Text(SHARED.into()));
    core
}

fn exports() -> Value {
    Value::Map(vec![
        (Value::Text("effects".into()), Value::Array(vec![])),
        (Value::Text("pureFunctions".into()), Value::Array(vec![])),
    ])
}

/// The intent's own Core budget, which the corpus set to the compiler minimum.
fn minimum(core: &Value) -> EvaluationLimits {
    let budget = field(
        field(field(core, "intents").unwrap(), "run").unwrap(),
        "coreEvaluationBudget",
    )
    .unwrap();
    let number = |key| super::values::number(field(budget, key).unwrap()).unwrap();
    EvaluationLimits {
        max_package_bytes: usize::MAX,
        max_input_bytes: usize::MAX,
        max_steps: number("maxSteps"),
        max_allocated_bytes: number("maxAllocatedBytes"),
        max_output_bytes: number("maxOutputBytes"),
    }
}

const GENEROUS: EvaluationLimits = EvaluationLimits {
    max_package_bytes: usize::MAX,
    max_input_bytes: usize::MAX,
    max_steps: u64::MAX / 4,
    max_allocated_bytes: u64::MAX / 4,
    max_output_bytes: u64::MAX / 4,
};

/// The program a pure package for intent `run` would carry, built from Core.
fn program(core: &Value, limits: EvaluationLimits) -> Result<Program, Error> {
    let intent = field(field(core, "intents")?, "run")?;
    let body = field(intent, "body")?;
    let mut parser = Parser {
        types: field(core, "types")?,
        coordinate: text_field(core, "coordinate")?,
        remaining: MAX_PROGRAM_NODES,
    };
    let input_type = parser.ty(text_field(intent, "input")?, 0)?;
    let output_type = parser.ty(text_field(intent, "output")?, 0)?;
    let mut bindings = Vec::new();
    for node in array(field(body, "nodes")?)? {
        let local = field(node, "binding")?;
        bindings.push(Binding {
            id: text_field(local, "id")?.to_owned(),
            ty: parser.ty(text_field(local, "type")?, 0)?,
            value: parser.expr(field(node, "value")?, 0)?,
        });
    }
    let result = parser.expr(field(body, "result")?, 0)?;
    let (helpers, schedule) = functions::decode_pure(core, &exports(), &mut parser)?;
    Ok(Program {
        input_id: "arg.0".into(),
        input_type,
        output_type,
        constraints: Vec::new(),
        bindings,
        helpers,
        result,
        limits,
        schedule,
    })
}

fn run(core: &Value, limits: EvaluationLimits, input: &Value) -> Result<EvaluationResult, Error> {
    evaluate::run(&program(core, limits)?, input.clone())
}

fn record(fields: &[(&str, Value)]) -> Value {
    Value::Map(
        fields
            .iter()
            .map(|(key, value)| (text_value(key), value.clone()))
            .collect(),
    )
}

fn text_value(value: &str) -> Value {
    Value::Text(value.into())
}

/// `n` items `{ key: i, weight: 7 }`, the worst case for each list bound.
fn items(n: i128) -> Value {
    Value::Array(
        (0..n)
            .map(|i| record(&[("key", Value::Integer(i)), ("weight", Value::Integer(7))]))
            .collect(),
    )
}

fn encoded(value: &Value) -> Vec<u8> {
    encode_canonical_cbor_v1(value).expect("result encodes")
}

/// The shared schedule runs at the compiler minimum, produces `expected`, and
/// meters no more than the minimum on either axis.
fn assert_within_minimum(hex: &str, input: &Value, expected: &Value) -> EvaluationResult {
    let core = core(hex);
    let minimum = minimum(&core);
    assert_eq!(
        program(&core, minimum).map(|program| program.schedule),
        Ok(CostSchedule::SharedReads)
    );
    let result = run(&core, minimum, input)
        .unwrap_or_else(|error| panic!("refused at the compiler minimum: {error:?}"));
    assert_eq!(result.output, encoded(expected));
    assert!(
        result.steps <= minimum.max_steps,
        "{} > {}",
        result.steps,
        minimum.max_steps
    );
    assert!(
        result.allocated_bytes <= minimum.max_allocated_bytes,
        "{} > {}",
        result.allocated_bytes,
        minimum.max_allocated_bytes
    );
    result
}

/// One step or one byte below the metered work, the meter refuses.
fn assert_metered_work_is_enforced(hex: &str, input: &Value) {
    let core = core(hex);
    let used = run(&core, GENEROUS, input).expect("runs under a generous budget");
    let steps = EvaluationLimits {
        max_steps: used.steps - 1,
        ..GENEROUS
    };
    let bytes = EvaluationLimits {
        max_allocated_bytes: used.allocated_bytes - 1,
        ..GENEROUS
    };
    assert_eq!(run(&core, steps, input), Err(Error::StepBudgetExceeded));
    assert_eq!(
        run(&core, bytes, input),
        Err(Error::AllocationBudgetExceeded)
    );
}

/// Without the capability the copying evaluator runs and meters as before.
fn copying_usage(hex: &str, input: &Value, expected: &Value) -> (u64, u64) {
    let core = copying(core(hex));
    let result = run(&core, GENEROUS, input).expect("copying evaluator runs");
    assert_eq!(result.output, encoded(expected));
    (result.steps, result.allocated_bytes)
}

fn reads_input() -> Value {
    record(&[("items", items(32))])
}

fn reads_output() -> Value {
    // 32 items of weight 7 each pass the guard once; later iterations keep the state.
    record(&[("total", Value::Integer(32))])
}

fn fill_input() -> Value {
    record(&[("items", items(32))])
}

fn fill_output() -> Value {
    // Every source item is appended once, in order.
    record(&[("acc", items(32))])
}

fn nested_input() -> Value {
    record(&[("rounds", Value::Integer(0))])
}

fn nested_output() -> Value {
    // Each round refills 16 items, so the last inner count is 16.
    record(&[("n", Value::Integer(16))])
}

fn matches_input() -> Value {
    record(&[("items", items(0))])
}

fn matches_output() -> Value {
    // 32 appends reach capacity, then the walk is Done(32).
    record(&[("n", Value::Integer(32))])
}

#[test]
fn shared_reads_of_a_full_list_run_within_the_compiler_minimum() {
    assert_within_minimum(READS, &reads_input(), &reads_output());
}

#[test]
fn appends_to_capacity_run_within_the_compiler_minimum() {
    assert_within_minimum(FILL, &fill_input(), &fill_output());
}

#[test]
fn nested_list_rebuilding_folds_run_within_the_compiler_minimum() {
    assert_within_minimum(NESTED, &nested_input(), &nested_output());
}

#[test]
fn variant_state_with_payload_binders_runs_within_the_compiler_minimum() {
    assert_within_minimum(MATCHES, &matches_input(), &matches_output());
}

#[test]
fn shared_reads_meter_refuses_one_below_its_metered_work() {
    assert_metered_work_is_enforced(READS, &reads_input());
}

#[test]
fn appends_meter_refuses_one_below_its_metered_work() {
    assert_metered_work_is_enforced(FILL, &fill_input());
}

#[test]
fn nested_folds_meter_refuses_one_below_its_metered_work() {
    assert_metered_work_is_enforced(NESTED, &nested_input());
}

#[test]
fn variant_state_meter_refuses_one_below_its_metered_work() {
    assert_metered_work_is_enforced(MATCHES, &matches_input());
}

#[test]
fn copying_schedule_meters_the_corpus_exactly_as_before() {
    assert_eq!(
        copying_usage(READS, &reads_input(), &reads_output()),
        (133_083, 6_196_452),
        "reads"
    );
    assert_eq!(
        copying_usage(FILL, &fill_input(), &fill_output()),
        (96_508, 4_969_516),
        "fill"
    );
    assert_eq!(
        copying_usage(NESTED, &nested_input(), &nested_output()),
        (53_793, 2_398_377),
        "nested"
    );
    assert_eq!(
        copying_usage(MATCHES, &matches_input(), &matches_output()),
        (21_489, 948_882),
        "matches"
    );
}

#[test]
fn copying_schedule_is_selected_without_the_capability() {
    let core = copying(core(READS));
    assert_eq!(
        program(&core, GENEROUS).map(|program| program.schedule),
        Ok(CostSchedule::Copying)
    );
}

#[test]
fn read_and_effect_helper_decoding_refuses_the_shared_schedule() {
    // `functions::decode` serves the read and effect routes, which have no
    // shared-read meter; only the pure route admits the capability.
    let core = core(READS);
    let mut parser = Parser {
        types: field(&core, "types").unwrap(),
        coordinate: text_field(&core, "coordinate").unwrap(),
        remaining: MAX_PROGRAM_NODES,
    };
    assert!(matches!(
        functions::decode(&core, &exports(), &mut parser),
        Err(Error::UnsupportedProgram)
    ));
}

#[test]
fn duplicate_shared_capability_is_refused() {
    let mut core = core(READS);
    let Value::Array(capabilities) = entry(&mut core, "requiredCoreCapabilities") else {
        panic!("capabilities are a list")
    };
    capabilities.push(text_value(SHARED));
    assert!(matches!(
        program(&core, GENEROUS),
        Err(Error::UnsupportedProgram)
    ));
}

mod generated {
    //! Generated inputs: the minimum holds for every input the type admits,
    //! not only the worst case above, and the result equals the copying
    //! evaluator's (a differential oracle). Replay a failure with
    //! `PROPTEST_RNG_SEED=<seed> cargo test -p warp-core --lib shared_reads_tests::generated`.
    use super::{
        copying, core, minimum, record, run, Value, FILL, GENEROUS, MATCHES, NESTED, READS,
    };
    use proptest::prelude::*;

    fn item() -> impl Strategy<Value = Value> {
        (any::<u64>(), prop_oneof![Just(7_u64), any::<u64>()]).prop_map(|(key, weight)| {
            record(&[
                ("key", Value::Integer(i128::from(key))),
                ("weight", Value::Integer(i128::from(weight))),
            ])
        })
    }

    fn items() -> impl Strategy<Value = Value> {
        proptest::collection::vec(item(), 0..=32).prop_map(Value::Array)
    }

    /// The input, by corpus program: `items` for three, `rounds` for nested.
    fn case() -> impl Strategy<Value = (&'static str, Value)> {
        prop_oneof![
            items().prop_map(|items| (READS, record(&[("items", items)]))),
            items().prop_map(|items| (FILL, record(&[("items", items)]))),
            items().prop_map(|items| (MATCHES, record(&[("items", items)]))),
            any::<u64>().prop_map(|rounds| {
                (
                    NESTED,
                    record(&[("rounds", Value::Integer(i128::from(rounds)))]),
                )
            }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        #[test]
        fn any_admitted_input_runs_within_the_minimum_and_agrees_with_copying(
            (hex, input) in case()
        ) {
            let shared = core(hex);
            let minimum = minimum(&shared);
            let result = run(&shared, minimum, &input);
            let expected = run(&copying(shared), GENEROUS, &input);
            prop_assert!(result.is_ok(), "refused at the minimum: {result:?}");
            let result = result.unwrap();
            prop_assert!(result.steps <= minimum.max_steps);
            prop_assert!(result.allocated_bytes <= minimum.max_allocated_bytes);
            prop_assert_eq!(Ok(result.output), expected.map(|r| r.output));
        }
    }
}
