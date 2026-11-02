// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Actual public worklist results, with a closed-form oracle independent of fold execution.
use super::{
    anyhow, atom_read_expected, bytes, checked_pair, decode, edict_pure, emitted, encode, ensure,
    field, limits, record, text, EvaluationError, EvaluationLimits, Path, Result, Value,
};

fn numbers(values: &[u8]) -> Value {
    Value::Array(
        values
            .iter()
            .map(|value| Value::Integer((*value).into()))
            .collect(),
    )
}

fn pure(root: &Path, evidence: &serde_json::Value, name: &str, count: usize) -> Result<()> {
    let (package, report) = emitted(&root.join(name), evidence)?;
    let (package, pin) = checked_pair(package, report)?;
    let mut cases = 0;
    for (left, right) in [
        (vec![], vec![]),
        (vec![4], vec![9]),
        (vec![0, 128], vec![255]),
        (vec![9], vec![]),
    ] {
        // Length after k steps is min(3, initial length + k). Every new value
        // is the original head, or 7 if the original sequence was empty.
        // This oracle does not run Echo's step function or a second fold.
        let mut expected = [left.clone(), right.clone()].concat();
        let fill = expected.first().copied().unwrap_or(7);
        expected.resize((expected.len() + count).min(3), fill);
        for index in [0_u64, 1, 2, 3, u64::MAX] {
            let input = encode(&record([
                ("left", numbers(&left)),
                ("right", numbers(&right)),
                ("index", Value::Integer(index.into())),
            ]))?;
            let result = edict_pure::evaluate(&package, pin, &input, limits())
                .map_err(|error| anyhow!("{name}: runtime refused: {error:?}"))?;
            let selected = usize::try_from(index)
                .ok()
                .and_then(|i| expected.get(i))
                .copied()
                .unwrap_or(0);
            let output = record([
                ("items", numbers(&expected)),
                ("length", Value::Integer(expected.len() as i128)),
                ("selected", Value::Integer(selected.into())),
                ("nested", Value::Array(vec![numbers(&expected)])),
            ]);
            ensure!(
                result.output == encode(&output)?,
                "{name}: incorrect worklist contents/order/duplicates/index at {index}"
            );
            let exact = EvaluationLimits {
                max_steps: result.steps,
                max_allocated_bytes: result.allocated_bytes,
                max_output_bytes: result.output.len() as u64,
                ..limits()
            };
            ensure!(
                edict_pure::evaluate(&package, pin, &input, exact) == Ok(result.clone()),
                "{name}: exact limits or repeat changed"
            );
            for (ceiling, error) in [
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
                    edict_pure::evaluate(&package, pin, &input, ceiling) == Err(error),
                    "{name}: one-under limit admitted"
                );
            }
            cases += 1;
        }
    }
    for (left, right, index) in [
        (numbers(&[1, 2, 3]), numbers(&[]), Value::Integer(0)),
        (numbers(&[]), numbers(&[1, 2]), Value::Integer(0)),
        (
            Value::Array(vec![Value::Bool(true)]),
            numbers(&[]),
            Value::Integer(0),
        ),
        (
            Value::Array(vec![Value::Integer(-1)]),
            numbers(&[]),
            Value::Integer(0),
        ),
        (
            Value::Array(vec![Value::Integer(256)]),
            numbers(&[]),
            Value::Integer(0),
        ),
        (numbers(&[]), numbers(&[]), Value::Integer(-1)),
        (numbers(&[]), numbers(&[]), Value::Bool(false)),
    ] {
        let input = encode(&record([
            ("left", left),
            ("right", right),
            ("index", index),
        ]))?;
        ensure!(
            edict_pure::evaluate(&package, pin, &input, limits())
                == Err(EvaluationError::InvalidInput),
            "{name}: invalid typed input admitted"
        );
    }
    let input = encode(&record([
        ("left", numbers(&[])),
        ("right", numbers(&[])),
        ("index", Value::Integer(0)),
    ]))?;
    let mut wrong = pin;
    wrong[0] ^= 1;
    ensure!(
        edict_pure::evaluate(&package, wrong, &input, limits())
            == Err(EvaluationError::PackageIdentityMismatch),
        "{name}: substituted identity admitted"
    );
    println!("PUBLIC_WORKLIST_PURE_OK {name}: {cases} closed-form cases; exact/repeat and 3 one-under limits each; 7 invalid inputs; identity refusal");
    Ok(())
}

fn aliases(root: &Path, evidence: &serde_json::Value) -> Result<()> {
    let (package, report) = emitted(&root.join("worklist-alias"), evidence)?;
    let (package, pin) = checked_pair(package, report)?;
    let rows = |values: &[u8]| {
        Value::Array(
            values
                .iter()
                .map(|value| record([("x", Value::Integer((*value).into()))]))
                .collect(),
        )
    };
    for flag in [false, true] {
        let input = encode(&record([
            ("xs", rows(&[1, 2])),
            ("ys", rows(&[9])),
            ("flag", Value::Bool(flag)),
        ]))?;
        let actual = edict_pure::evaluate(&package, pin, &input, limits())
            .map_err(|error| anyhow!("structural aliases refused: {error:?}"))?;
        let expected = if flag {
            rows(&[1, 2, 1, 2])
        } else {
            rows(&[9, 9])
        };
        ensure!(
            decode(&actual.output)? == record([("items", expected)]),
            "structural alias concat changed fields or order"
        );
    }
    println!("PUBLIC_WORKLIST_ALIAS_OK both conditional branches preserve structural record types");
    Ok(())
}

pub(super) fn run(root: &Path, evidence: &serde_json::Value) -> Result<()> {
    for count in [0, 1, 2, 3, 5] {
        pure(root, evidence, &format!("worklist-pure-{count}"), count)?;
    }
    pure(root, evidence, "worklist-renamed", 3)?;
    aliases(root, evidence)?;
    for count in [0, 1, 3] {
        // The program selects the third stored payload. Zero/one iterations
        // cannot produce it; three iterations must return the actual atom.
        let expected: &[u8] = if count == 3 { b"alpha" } else { b"" };
        atom_read_expected(root, evidence, &format!("worklist-read-{count}"), expected)?;
    }
    let (package, report) = emitted(&root.join("worklist-read-nominal"), evidence)?;
    let (package, _) = checked_pair(package, report)?;
    let package = decode(&package)?;
    let program = decode(bytes(field(&package, "program")?)?)?;
    let core = decode(bytes(field(&program, "core_artifact")?)?)?;
    let nominal = field(field(&core, "types")?, "jedit.text@1.WitnessId")?;
    ensure!(
        text(field(nominal, "kind")?)? == "Nominal"
            && text(field(nominal, "contract")?)? == "jedit.text@1.WitnessId",
        "public list operation erased nominal identity"
    );
    atom_read_expected(root, evidence, "worklist-read-nominal", b"alpha")?;
    ensure!(
        evidence["results"]["worklist-read-nominal-bare"]["refusalBoundary"] == "TypeMismatch",
        "bare representation was not refused by the public compiler"
    );
    for name in ["old-worklist-pure", "old-worklist-read"] {
        ensure!(
            evidence["results"][name]["refusalBoundary"] == "ProviderLowererRefused",
            "old provider refusal missing"
        );
    }
    Ok(())
}
