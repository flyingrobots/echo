// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Consume exact public package/report pairs and inspect their actual list results.
use super::{
    anyhow, atom_read, bytes, checked_pair, decode, edict_pure, emitted, encode, ensure, field,
    limits, record, text, Context, EvaluationError, EvaluationLimits, Path, Result, Value,
};

pub(super) fn run(root: &Path, evidence: &serde_json::Value, read_usage: (u64, u64)) -> Result<()> {
    let values = Value::Array(vec![
        Value::Integer(4),
        Value::Integer(9),
        Value::Integer(4),
    ]);
    let input = encode(&record([("values", values.clone())]))?;
    let numbers = Value::Array(vec![
        Value::Integer(7),
        Value::Integer(2),
        Value::Integer(7),
    ]);
    for (case, expected) in [
        ("empty", Value::Array(vec![])),
        ("nonempty", numbers.clone()),
        ("nested", Value::Array(vec![Value::Array(vec![]), numbers])),
        (
            "record",
            Value::Array(vec![
                record([("payload", Value::Bytes(vec![]))]),
                record([("payload", Value::Bytes(vec![0, 255]))]),
                record([("payload", Value::Bytes(vec![b'x']))]),
            ]),
        ),
        ("input", values),
    ] {
        let name = format!("list-pure-{case}");
        let (package, report) = emitted(&root.join(&name), evidence)?;
        let (package, pin) = checked_pair(package, report)?;
        let result = edict_pure::evaluate(&package, pin, &input, limits())
            .map_err(|error| anyhow!("{name}: runtime refused: {error:?}"))?;
        ensure!(
            decode(&result.output)? == record([("items", expected)]),
            "{name}: list contents, order or duplicates changed"
        );
        let exact = EvaluationLimits {
            max_steps: result.steps,
            max_allocated_bytes: result.allocated_bytes,
            max_output_bytes: result.output.len() as u64,
            ..limits()
        };
        ensure!(
            edict_pure::evaluate(&package, pin, &input, exact) == Ok(result.clone()),
            "{name}: exact limits or repeat differ"
        );
        for (ceiling, expected_error) in [
            (
                EvaluationLimits {
                    max_steps: result.steps - 1,
                    ..exact
                },
                EvaluationError::StepBudgetExceeded,
            ),
            (
                EvaluationLimits {
                    max_allocated_bytes: result.allocated_bytes - 1,
                    ..exact
                },
                EvaluationError::AllocationBudgetExceeded,
            ),
            (
                EvaluationLimits {
                    max_output_bytes: result.output.len() as u64 - 1,
                    ..exact
                },
                EvaluationError::OutputBudgetExceeded,
            ),
        ] {
            ensure!(
                edict_pure::evaluate(&package, pin, &input, ceiling) == Err(expected_error),
                "{name}: one-under limit did not refuse"
            );
        }
        // Values above U64 cannot cross the canonical wire boundary at all.
        let unencodable = record([(
            "values",
            Value::Array(vec![Value::Integer(i128::from(u64::MAX) + 1)]),
        )]);
        ensure!(
            encode(&unencodable).is_err(),
            "over-U64 integer was encoded"
        );
        for invalid in [
            Value::Array(vec![Value::Integer(1); 4]),
            Value::Array(vec![Value::Bool(true), Value::Integer(2)]),
            Value::Array(vec![Value::Integer(1), Value::Bool(true)]),
            Value::Array(vec![Value::Integer(-1)]),
            Value::Integer(1),
        ] {
            let invalid = encode(&record([("values", invalid)]))?;
            ensure!(
                edict_pure::evaluate(&package, pin, &invalid, limits())
                    == Err(EvaluationError::InvalidInput),
                "{name}: invalid list element or length admitted"
            );
        }
        let mut wrong = pin;
        wrong[0] ^= 1;
        ensure!(
            edict_pure::evaluate(&package, wrong, &input, limits())
                == Err(EvaluationError::PackageIdentityMismatch),
            "{name}: changed package identity admitted"
        );
        println!("PUBLIC_LIST_RUNTIME_OK {name}: exact contents/order/duplicates, exact/repeated limits, 3 one-under budgets, 5 input refusals, canonical integer refusal, identity refusal");
    }
    let mut empty_usage = None;
    for case in ["empty", "nonempty", "nested", "record", "nominal"] {
        let name = format!("list-read-{case}");
        if case == "nominal" {
            let (package, report) = emitted(&root.join(&name), evidence)?;
            let (package, _) = checked_pair(package, report)?;
            let package = decode(&package)?;
            let program = decode(bytes(field(&package, "program")?)?)?;
            let core = decode(bytes(field(&program, "core_artifact")?)?)?;
            let definition = field(field(&core, "types")?, "jedit.text@1.WitnessId")?;
            ensure!(
                text(field(definition, "kind")?)? == "Nominal"
                    && text(field(definition, "contract")?)? == "jedit.text@1.WitnessId",
                "public list element lost nominal authority"
            );
        }
        let usage = atom_read(root, evidence, &name)?;
        ensure!(
            usage.0 > read_usage.0 && usage.1 > read_usage.1,
            "{name}: list materialization was not charged"
        );
        if case == "empty" {
            empty_usage = Some(usage);
        } else if case == "nonempty" {
            let empty = empty_usage.context("empty list control")?;
            ensure!(
                usage.0 > empty.0 && usage.1 > empty.1,
                "read list children were not charged"
            );
        }
    }
    Ok(())
}
