// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout)]
//! Consumes unchanged public compiler package/report pairs; never synthesizes IR.
use anyhow::{anyhow, bail, ensure, Context, Result};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use warp_core::edict_pure::{self, EvaluationError, EvaluationLimits};
use warp_core::edict_read::{ReadBasis, ReadError, ReadView};
use warp_core::{
    make_node_id, make_type_id, AtomPayload, AttachmentValue, GraphStore, NodeKey, NodeRecord,
    WorldlineFrontier, WorldlineId, WorldlineState,
};

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value> {
    let Value::Map(fields) = value else {
        bail!("expected map");
    };
    fields
        .iter()
        .find_map(|(key, value)| (key == &Value::Text(name.into())).then_some(value))
        .context("missing field")
}
fn text(value: &Value) -> Result<&str> {
    let Value::Text(value) = value else {
        bail!("expected text");
    };
    Ok(value)
}
fn bytes(value: &Value) -> Result<&[u8]> {
    let Value::Bytes(value) = value else {
        bail!("expected bytes");
    };
    Ok(value)
}
fn hash(value: &Value) -> Result<[u8; 32]> {
    bytes(value)?.try_into().context("expected bytes32")
}
fn resource(value: &Value, id: &str) -> Result<[u8; 32]> {
    ensure!(text(field(value, "id")?)? == id, "wrong resource identity");
    let Value::Array(values) = field(value, "digest")? else {
        bail!("expected digest pair");
    };
    ensure!(
        values.len() == 2 && text(&values[0])? == "sha256",
        "wrong digest algorithm"
    );
    hash(&values[1])
}
fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}
fn read(path: &Path) -> Result<Vec<u8>> {
    let metadata = path.symlink_metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= 1_048_576,
        "unbounded/nonregular witness input"
    );
    let mut data = Vec::new();
    std::fs::File::open(path)?
        .take(1_048_577)
        .read_to_end(&mut data)?;
    ensure!(
        data.len() <= 1_048_576,
        "witness input grew above its bound"
    );
    Ok(data)
}
fn emitted(root: &Path, evidence: &serde_json::Value) -> Result<(Vec<u8>, Vec<u8>)> {
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .context("variant name")?;
    let result = &evidence["results"][name];
    ensure!(
        result["firstBuild"]["exitCode"] == 0
            && result["firstBuild"]["diagnostics"]
                .as_array()
                .is_some_and(Vec::is_empty),
        "compiler/verifier did not succeed"
    );
    let package = read(&root.join("executable-operation-package.cbor"))?;
    let report = read(&root.join("verification-report.cbor"))?;
    for (filename, data) in [
        ("executable-operation-package.cbor", &package),
        ("verification-report.cbor", &report),
    ] {
        let expected = result["firstBuild"]["artifacts"][filename]
            .as_str()
            .context("missing emitted digest")?;
        ensure!(
            result["repeatedBuild"]["artifacts"][filename].as_str() == Some(expected),
            "repeat did not corroborate exact bytes"
        );
        ensure!(
            hex::encode(Sha256::digest(data)) == expected,
            "retained emitted bytes changed"
        );
    }
    Ok((package, report))
}
fn checked_pair(package: Vec<u8>, report_bytes: Vec<u8>) -> Result<(Vec<u8>, [u8; 32])> {
    let value = decode(&package)?;
    let report = decode(&report_bytes)?;
    ensure!(
        text(field(&report, "apiVersion")?)? == "echo.operation-package-verifier-report/v1",
        "wrong report ABI"
    );
    ensure!(
        text(field(&report, "outcome")?)? == "accepted"
            && bytes(field(&report, "diagnosticBytes")?)?.is_empty(),
        "verifier did not accept"
    );
    let pin = resource(
        field(&report, "package")?,
        "executable-operation-package.echo",
    )?;
    ensure!(
        pin == digest("echo.operation-package/v1", &value)?,
        "report/package mismatch"
    );
    let closure = field(&value, "semantic_closure")?;
    let target = field(&report, "targetIr")?;
    ensure!(
        ["echo.span-ir/v1", "echo.span-ir/v2"].contains(&text(field(target, "id")?)?),
        "unsupported Target IR role"
    );
    ensure!(
        resource(target, text(field(target, "id")?)?)?
            == hash(field(closure, "target_ir_identity")?)?,
        "report/Target IR mismatch"
    );
    let program = decode(bytes(field(&value, "program")?)?)?;
    let projection = decode(bytes(field(&program, "result_projection_artifact")?)?)?;
    ensure!(
        resource(
            field(&report, "applicationResultProjection")?,
            text(field(&value, "operation_coordinate")?)?
        )? == digest("edict.result-projection.artifact/v1", &projection)?,
        "report/projection mismatch"
    );
    let _ = resource(field(&report, "diagnosticAbi")?, "edict.diagnostics/v1")?;
    Ok((package, pin))
}
fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 1_048_576,
        max_input_bytes: 32_768,
        max_steps: 16_777_216,
        max_allocated_bytes: 67_108_864,
        max_output_bytes: 32_768,
    }
}
fn pure(root: &Path, name: &str, reversed: bool, evidence: &serde_json::Value) -> Result<usize> {
    let directory = root.join(name);
    let (package_bytes, report_bytes) = emitted(&directory, evidence)?;
    let (package, pin) = checked_pair(package_bytes, report_bytes)?;
    let case_bytes = read(&root.join("pure/cases.json"))?;
    let case_hash = hex::encode(Sha256::digest(&case_bytes));
    ensure!(
        evidence["results"]["pure"]["casesSha256"].as_str() == Some(case_hash.as_str())
            && evidence["selectedInputs"]["pureCasesSha256"].as_str() == Some(case_hash.as_str()),
        "original literal cases changed"
    );
    let cases: serde_json::Value = serde_json::from_slice(&case_bytes)?;
    ensure!(
        cases["schema"] == "jedit.range-fragment-assembly-cases/v1"
            && cases["operation"] == "jedit.text.replace_range@1.assembleRange",
        "unexpected authored case identity"
    );
    let cases = cases["cases"].as_array().context("cases")?;
    ensure!(cases.len() == 12, "expected twelve literal cases");
    for case in cases {
        let first = hex::decode(case["firstHex"].as_str().context("firstHex")?)?;
        let second = hex::decode(case["secondHex"].as_str().context("secondHex")?)?;
        let input = encode(&record([
            ("firstFragment", Value::Bytes(first.clone())),
            ("secondFragment", Value::Bytes(second.clone())),
        ]))?;
        let actual = edict_pure::evaluate(&package, pin, &input, limits());
        if case["refusal"].as_str() == Some("input-type") {
            ensure!(
                actual == Err(EvaluationError::InvalidInput),
                "expected input-type refusal"
            );
            continue;
        }
        let expected = if reversed {
            [second, first].concat()
        } else {
            hex::decode(case["bytesHex"].as_str().context("bytesHex")?)?
        };
        ensure!(
            decode(
                &actual
                    .map_err(|error| anyhow!("pure runtime refused: {error:?}"))?
                    .output
            )? == record([("bytes", Value::Bytes(expected))]),
            "wrong literal assembly output"
        );
    }
    println!("PUBLIC_PURE_RUNTIME_OK {name} {}", cases.len());
    Ok(cases.len())
}
fn unsigned_bytes(root: &Path, evidence: &serde_json::Value) -> Result<()> {
    for (name, identity) in [("u8-identity", true), ("u8-literal", false)] {
        let (package, report) = emitted(&root.join(name), evidence)?;
        let (package, pin) = checked_pair(package, report)?;
        for value in [0_i128, 128, 255] {
            let input = encode(&record([
                ("firstFragment", Value::Integer(value)),
                ("secondFragment", Value::Integer(0)),
            ]))?;
            let outcome = edict_pure::evaluate(&package, pin, &input, limits())
                .map_err(|error| anyhow!("unsigned-byte runtime refused: {error:?}"))?;
            let expected = if identity || value == 0 { value } else { 255 };
            ensure!(
                decode(&outcome.output)? == record([("bytes", Value::Integer(expected))]),
                "wrong unsigned-byte result"
            );
            ensure!(
                Ok(outcome) == edict_pure::evaluate(&package, pin, &input, limits()),
                "unsigned-byte result or accounting changed on repeat"
            );
        }
        for value in [
            Value::Integer(256),
            Value::Integer(-1),
            Value::Bytes(vec![255]),
        ] {
            let input = encode(&record([
                ("firstFragment", value),
                ("secondFragment", Value::Integer(0)),
            ]))?;
            ensure!(
                edict_pure::evaluate(&package, pin, &input, limits())
                    == Err(EvaluationError::InvalidInput),
                "unsigned-byte input domain did not refuse"
            );
        }
        let input = encode(&record([
            ("firstFragment", Value::Integer(255)),
            ("secondFragment", Value::Integer(0)),
        ]))?;
        for (limits, expected) in [
            (
                EvaluationLimits {
                    max_steps: 1,
                    ..limits()
                },
                EvaluationError::StepBudgetExceeded,
            ),
            (
                EvaluationLimits {
                    max_allocated_bytes: 1,
                    ..limits()
                },
                EvaluationError::AllocationBudgetExceeded,
            ),
            (
                EvaluationLimits {
                    max_output_bytes: 1,
                    ..limits()
                },
                EvaluationError::OutputBudgetExceeded,
            ),
        ] {
            ensure!(
                edict_pure::evaluate(&package, pin, &input, limits) == Err(expected),
                "unsigned-byte host resource limit did not refuse"
            );
        }
        let mut wrong = pin;
        wrong[0] ^= 1;
        ensure!(
            edict_pure::evaluate(&package, wrong, &input, limits())
                == Err(EvaluationError::PackageIdentityMismatch),
            "unsigned-byte package substitution did not refuse"
        );
        println!("PUBLIC_U8_RUNTIME_OK {name}: 3 literal cases, 3 input refusals, 3 resource refusals, 1 identity refusal");
    }
    Ok(())
}

fn byte_literals(root: &Path, evidence: &serde_json::Value) -> Result<()> {
    let (package_bytes, report_bytes) = emitted(&root.join("byte-literals"), evidence)?;
    let (package, pin) = checked_pair(package_bytes, report_bytes)?;
    let cases: [(&[u8], &[u8], &[u8]); 3] = [
        (b"", b"unused", b""),
        (b"a", b"", &[0, 255, 10, 13]),
        (&[255], b"x", &[0, 255, 10, 13]),
    ];
    for (first, second, expected) in cases {
        let input = encode(&record([
            ("firstFragment", Value::Bytes(first.to_vec())),
            ("secondFragment", Value::Bytes(second.to_vec())),
        ]))?;
        let outcome = edict_pure::evaluate(&package, pin, &input, limits())
            .map_err(|error| anyhow!("byte literal runtime refused: {error:?}"))?;
        ensure!(
            decode(&outcome.output)? == record([("bytes", Value::Bytes(expected.to_vec()))]),
            "wrong authored byte literal result"
        );
        ensure!(
            Ok(outcome) == edict_pure::evaluate(&package, pin, &input, limits()),
            "byte literal output or accounting changed on repeat"
        );
    }
    let input = encode(&record([
        ("firstFragment", Value::Bytes(b"a".to_vec())),
        ("secondFragment", Value::Bytes(vec![])),
    ]))?;
    for (limits, expected) in [
        (
            EvaluationLimits {
                max_steps: 1,
                ..limits()
            },
            EvaluationError::StepBudgetExceeded,
        ),
        (
            EvaluationLimits {
                max_allocated_bytes: 1,
                ..limits()
            },
            EvaluationError::AllocationBudgetExceeded,
        ),
        (
            EvaluationLimits {
                max_output_bytes: 1,
                ..limits()
            },
            EvaluationError::OutputBudgetExceeded,
        ),
    ] {
        ensure!(
            edict_pure::evaluate(&package, pin, &input, limits) == Err(expected),
            "byte literal host resource limit did not refuse"
        );
    }
    let invalid = encode(&record([
        ("firstFragment", Value::Bytes(vec![1; 9])),
        ("secondFragment", Value::Bytes(vec![])),
    ]))?;
    ensure!(
        edict_pure::evaluate(&package, pin, &invalid, limits())
            == Err(EvaluationError::InvalidInput),
        "byte literal input bound did not refuse"
    );
    let mut wrong_pin = pin;
    wrong_pin[0] ^= 1;
    ensure!(
        edict_pure::evaluate(&package, wrong_pin, &input, limits())
            == Err(EvaluationError::PackageIdentityMismatch),
        "byte literal package substitution did not refuse"
    );
    println!("PUBLIC_BYTE_LITERAL_RUNTIME_OK 3 literal cases, 3 resource refusals, input and identity refusals");
    Ok(())
}
fn byte_lookup(root: &Path, evidence: &serde_json::Value) -> Result<()> {
    for (name, lazy) in [("byte-lookup", false), ("byte-lookup-lazy-or", true)] {
        let (package, report) = emitted(&root.join(name), evidence)?;
        let (package, pin) = checked_pair(package, report)?;
        let input =
            |bytes: Value, index: Value| encode(&record([("bytes", bytes), ("index", index)]));
        for (bytes, index) in [
            (&b""[..], 0_u64),
            (&[0, 128, 255][..], 0),
            (&[0, 128, 255][..], 1),
            (&[0, 128, 255][..], 2),
            (&[0, 128, 255][..], 3),
            (&b"a"[..], u64::MAX),
        ] {
            let selected = usize::try_from(index).ok().and_then(|i| bytes.get(i));
            let expected = if lazy {
                if selected.is_none_or(|byte| *byte < 128) {
                    0
                } else {
                    255
                }
            } else {
                selected.map_or(0, |byte| i128::from(*byte))
            };
            let input = input(Value::Bytes(bytes.to_vec()), Value::Integer(index.into()))?;
            let result = edict_pure::evaluate(&package, pin, &input, limits())
                .map_err(|error| anyhow!("byte lookup refused: {error:?}"))?;
            ensure!(
                decode(&result.output)? == record([("value", Value::Integer(expected))]),
                "wrong byte lookup"
            );
            ensure!(
                Ok(result) == edict_pure::evaluate(&package, pin, &input, limits()),
                "lookup repeat changed result/accounting"
            );
        }
        for (bytes, index) in [
            (Value::Bytes(vec![255]), Value::Integer(-1)),
            (Value::Bytes(vec![255]), Value::Bool(true)),
            (Value::Integer(255), Value::Integer(0)),
        ] {
            ensure!(
                edict_pure::evaluate(&package, pin, &input(bytes, index)?, limits())
                    == Err(EvaluationError::InvalidInput),
                "invalid lookup input admitted"
            );
        }
        let input = input(Value::Bytes(vec![255]), Value::Integer(0))?;
        for (limits, expected) in [
            (
                EvaluationLimits {
                    max_steps: 1,
                    ..limits()
                },
                EvaluationError::StepBudgetExceeded,
            ),
            (
                EvaluationLimits {
                    max_allocated_bytes: 1,
                    ..limits()
                },
                EvaluationError::AllocationBudgetExceeded,
            ),
            (
                EvaluationLimits {
                    max_output_bytes: 1,
                    ..limits()
                },
                EvaluationError::OutputBudgetExceeded,
            ),
        ] {
            ensure!(
                edict_pure::evaluate(&package, pin, &input, limits) == Err(expected),
                "lookup resource limit admitted"
            );
        }
        let mut wrong = pin;
        wrong[0] ^= 1;
        ensure!(
            edict_pure::evaluate(&package, wrong, &input, limits())
                == Err(EvaluationError::PackageIdentityMismatch),
            "lookup substitution admitted"
        );
        println!("PUBLIC_BYTE_LOOKUP_RUNTIME_OK {name}: 6 cases, 3 input refusals, 3 resource refusals, 1 identity refusal");
    }
    Ok(())
}
#[path = "source_functions_public_runtime/lists.rs"]
mod lists;
#[path = "source_functions_public_runtime/worklists.rs"]
mod worklists;

fn atom_read(root: &Path, evidence: &serde_json::Value, name: &str) -> Result<(u64, u64)> {
    atom_read_expected(root, evidence, name, b"alpha")
}

fn atom_read_expected(
    root: &Path,
    evidence: &serde_json::Value,
    name: &str,
    expected: &[u8],
) -> Result<(u64, u64)> {
    let (package_bytes, report_bytes) = emitted(&root.join(name), evidence)?;
    let (package, pin) = checked_pair(package_bytes, report_bytes)?;
    let mut store = GraphStore::default();
    let node = make_node_id("public-source-function-read");
    let ty = make_type_id("document");
    store.insert_node(
        node,
        NodeRecord {
            ty: make_type_id("node"),
        },
    );
    store.set_node_attachment(
        node,
        Some(AttachmentValue::Atom(AtomPayload::new(
            ty,
            b"alpha".to_vec().into(),
        ))),
    );
    let key = NodeKey {
        warp_id: store.warp_id(),
        local_id: node,
    };
    let frontier = WorldlineFrontier::new(
        WorldlineId::from_bytes([7; 32]),
        WorldlineState::from_root_store(store, node)?,
    );
    let before = frontier.state().state_root();
    let basis = ReadBasis::at(&frontier);
    let aperture = [key];
    let view = ReadView::new(&frontier, basis, &aperture)
        .map_err(|error| anyhow!("view refused: {error:?}"))?;
    let mut input_fields = vec![
        (
            "address",
            record([
                ("warpId", Value::Bytes(key.warp_id.0.to_vec())),
                ("nodeId", Value::Bytes(key.local_id.0.to_vec())),
                ("typeId", Value::Bytes(ty.0.to_vec())),
            ]),
        ),
        ("expected", Value::Bytes(b"alpha".to_vec())),
    ];
    if name.starts_with("read-byte-lookup") {
        input_fields.push(("probe", Value::Bytes(vec![0, 128, 255])));
    }
    if name.starts_with("worklist-read-") {
        input_fields.push(("fallback", Value::Bytes(vec![])));
    }
    if matches!(name, "list-read-nominal" | "worklist-read-nominal") {
        input_fields.push(("identity", Value::Bytes(vec![7; 32])));
        input_fields.push(("raw", Value::Bytes(vec![7; 32])));
    }
    let input = encode(&record(input_fields))?;
    let result = warp_core::edict_read::evaluate(
        &package,
        pin,
        &input,
        &view,
        warp_core::edict_read::ReadLimits {
            evaluation: limits(),
            max_reads: 1,
            max_read_bytes: 5,
        },
    )
    .map_err(|error| anyhow!("read runtime refused: {error:?}"))?;
    ensure!(
        decode(&result.output)? == Value::Bytes(expected.to_vec()),
        "wrong stored-atom result"
    );
    ensure!(
        result.basis == basis
            && result.reads == 1
            && result.read_bytes == 5
            && frontier.state().state_root() == before,
        "wrong read evidence or mutation"
    );
    let read_limits = warp_core::edict_read::ReadLimits {
        evaluation: EvaluationLimits {
            max_steps: result.steps,
            max_allocated_bytes: result.allocated_bytes,
            max_output_bytes: result.output.len() as u64,
            ..limits()
        },
        max_reads: 1,
        max_read_bytes: 5,
    };
    ensure!(
        Ok(result.clone())
            == warp_core::edict_read::evaluate(&package, pin, &input, &view, read_limits),
        "read output or accounting changed on repeat"
    );
    for (limits, expected) in [
        (
            warp_core::edict_read::ReadLimits {
                max_reads: 0,
                ..read_limits
            },
            ReadError::ReadCountExceeded,
        ),
        (
            warp_core::edict_read::ReadLimits {
                max_read_bytes: 4,
                ..read_limits
            },
            ReadError::ReadBytesExceeded,
        ),
        (
            warp_core::edict_read::ReadLimits {
                evaluation: EvaluationLimits {
                    max_steps: result.steps - 1,
                    ..read_limits.evaluation
                },
                ..read_limits
            },
            ReadError::Evaluation(EvaluationError::StepBudgetExceeded),
        ),
        (
            warp_core::edict_read::ReadLimits {
                evaluation: EvaluationLimits {
                    max_allocated_bytes: result.allocated_bytes - 1,
                    ..read_limits.evaluation
                },
                ..read_limits
            },
            ReadError::Evaluation(EvaluationError::AllocationBudgetExceeded),
        ),
        (
            warp_core::edict_read::ReadLimits {
                evaluation: EvaluationLimits {
                    max_output_bytes: result.output.len() as u64 - 1,
                    ..read_limits.evaluation
                },
                ..read_limits
            },
            ReadError::Evaluation(EvaluationError::OutputBudgetExceeded),
        ),
    ] {
        ensure!(
            warp_core::edict_read::evaluate(&package, pin, &input, &view, limits) == Err(expected),
            "explicit read limit did not refuse"
        );
        ensure!(
            frontier.state().state_root() == before,
            "refused read changed state"
        );
    }
    println!("PUBLIC_READ_RUNTIME_OK {name}: 1 read, 5 bytes, 5 resource refusals, steps={} allocated={}", result.steps, result.allocated_bytes);
    Ok((result.steps, result.allocated_bytes))
}
fn variant_package(
    root: &Path,
    name: &str,
    evidence: &serde_json::Value,
) -> Result<(Vec<u8>, [u8; 32])> {
    let (package, report) = emitted(&root.join(name), evidence)?;
    let (package, pin) = checked_pair(package, report)?;
    let value = decode(&package)?;
    let program = decode(bytes(field(&value, "program")?)?)?;
    let core = decode(bytes(field(&program, "core_artifact")?)?)?;
    ensure!(
        field(&core, "requiredCoreCapabilities")?
            == &Value::Array(vec![Value::Text("core.variant/v1".into())]),
        "public compiler variant capability was lost or changed"
    );
    ensure!(
        text(field(
            field(field(&core, "types")?, "WitnessChoice")?,
            "kind"
        )?)? == "Variant",
        "public compiler variant declaration missing"
    );
    if name.starts_with("match-") {
        let selector = field(field(&core, "functions")?, "selectChoice")?;
        ensure!(
            text(field(field(field(selector, "body")?, "result")?, "kind")?)? == "match",
            "public compiler match body missing"
        );
    }
    Ok((package, pin))
}

fn closed_variants(
    root: &Path,
    evidence: &serde_json::Value,
    read_usage: (u64, u64),
    prefix: &str,
) -> Result<()> {
    let input = encode(&record([
        ("firstFragment", Value::Bytes(b"first".to_vec())),
        ("secondFragment", Value::Bytes(b"last".to_vec())),
    ]))?;
    let (baseline, report) = emitted(&root.join("pure"), evidence)?;
    let (baseline, pin) = checked_pair(baseline, report)?;
    let baseline = edict_pure::evaluate(&baseline, pin, &input, limits())
        .map_err(|e| anyhow!("variant control refused: {e:?}"))?;
    let mut previous = (baseline.steps, baseline.allocated_bytes);
    let mut previous_read = read_usage;
    for case in ["empty", "payload"] {
        let name = format!("{prefix}-pure-{case}");
        pure(root, &name, false, evidence)?;
        let (package, pin) = variant_package(root, &name, evidence)?;
        let result = edict_pure::evaluate(&package, pin, &input, limits())
            .map_err(|e| anyhow!("variant runtime refused: {e:?}"))?;
        ensure!(
            result.output == baseline.output,
            "constructor changed the authored byte result"
        );
        ensure!(
            result.steps > previous.0 && result.allocated_bytes > previous.1,
            "constructor or payload work was not charged"
        );
        ensure!(
            Ok(result.clone()) == edict_pure::evaluate(&package, pin, &input, limits()),
            "variant output/accounting changed on repeat"
        );
        for (limit, expected) in [
            (
                EvaluationLimits {
                    max_steps: result.steps - 1,
                    ..limits()
                },
                EvaluationError::StepBudgetExceeded,
            ),
            (
                EvaluationLimits {
                    max_allocated_bytes: result.allocated_bytes - 1,
                    ..limits()
                },
                EvaluationError::AllocationBudgetExceeded,
            ),
            (
                EvaluationLimits {
                    max_output_bytes: result.output.len() as u64 - 1,
                    ..limits()
                },
                EvaluationError::OutputBudgetExceeded,
            ),
        ] {
            ensure!(
                edict_pure::evaluate(&package, pin, &input, limit) == Err(expected),
                "variant one-under resource ceiling did not refuse"
            );
        }
        let mut wrong = pin;
        wrong[0] ^= 1;
        ensure!(
            edict_pure::evaluate(&package, wrong, &input, limits())
                == Err(EvaluationError::PackageIdentityMismatch),
            "variant package substitution did not refuse"
        );
        previous = (result.steps, result.allocated_bytes);
        let read_name = format!("{prefix}-read-{case}");
        variant_package(root, &read_name, evidence)?;
        let current = atom_read(root, evidence, &read_name)?;
        ensure!(
            current.0 > previous_read.0 && current.1 > previous_read.1,
            "read constructor or payload work was not charged"
        );
        previous_read = current;
        println!("PUBLIC_VARIANT_RUNTIME_OK {prefix}-{case}: capability retained, 12 pure cases, repeat, 3 one-under budgets, identity refusal, bounded read");
    }
    Ok(())
}

fn state_folds(root: &Path, evidence: &serde_json::Value) -> Result<()> {
    // The zero-bound public package supplies only the fixed wrapper cost.
    // Independently counted toggle work: 2 admission + 1 if + 1 compare +
    // 3*2 expression/copy + 1 result = 11 steps, and 3*64 = 192 copied bytes.
    // This checks deltas, not an independently derived absolute wrapper cost.
    let mut baseline = [(0, 0); 4];
    for bound in [0_u64, 1, 2, 3] {
        let name = format!("fold-pure-{bound}");
        let (package, report) = emitted(&root.join(&name), evidence)?;
        let (package, pin) = checked_pair(package, report)?;
        for (index, value) in [0_i128, 1, 128, 255].into_iter().enumerate() {
            let input = encode(&record([
                ("firstFragment", Value::Integer(value)),
                ("secondFragment", Value::Integer(0)),
            ]))?;
            let result = edict_pure::evaluate(&package, pin, &input, limits())
                .map_err(|error| anyhow!("fold runtime refused: {error:?}"))?;
            let expected = if bound == 0 {
                value
            } else if value == 0 {
                i128::from(bound % 2)
            } else {
                i128::from((bound - 1) % 2)
            };
            ensure!(
                decode(&result.output)? == record([("bytes", Value::Integer(expected))]),
                "wrong fold result at bound {bound}, input {value}"
            );
            if bound == 0 {
                baseline[index] = (result.steps, result.allocated_bytes);
            }
            ensure!(
                (result.steps, result.allocated_bytes)
                    == (
                        baseline[index].0 + 11 * bound,
                        baseline[index].1 + 192 * bound
                    ),
                "fold iteration accounting mismatch"
            );
            let exact = EvaluationLimits {
                max_steps: result.steps,
                max_allocated_bytes: result.allocated_bytes,
                max_output_bytes: result.output.len() as u64,
                ..limits()
            };
            ensure!(
                Ok(result.clone()) == edict_pure::evaluate(&package, pin, &input, exact),
                "fold exact budget or repeat failed"
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
                    "fold one-under resource limit did not refuse"
                );
            }
            let mut wrong = pin;
            wrong[0] ^= 1;
            ensure!(
                edict_pure::evaluate(&package, wrong, &input, limits())
                    == Err(EvaluationError::PackageIdentityMismatch),
                "fold package identity substitution admitted"
            );
        }
        for invalid in [
            Value::Integer(-1),
            Value::Integer(256),
            Value::Bytes(vec![0]),
        ] {
            let input = encode(&record([
                ("firstFragment", invalid),
                ("secondFragment", Value::Integer(0)),
            ]))?;
            ensure!(
                edict_pure::evaluate(&package, pin, &input, limits())
                    == Err(EvaluationError::InvalidInput),
                "fold input domain did not refuse"
            );
        }
        println!("PUBLIC_FOLD_RUNTIME_OK {name}: 4 inputs, parity, exact budgets, iteration deltas, one-under limits, identity and input refusals");
    }
    let mut previous = None;
    for bound in [0, 1, 3] {
        let usage = atom_read(root, evidence, &format!("fold-read-{bound}"))?;
        if let Some((steps, bytes)) = previous {
            ensure!(
                usage.0 > steps && usage.1 > bytes,
                "read fold iterations were not charged"
            );
        }
        previous = Some(usage);
    }
    Ok(())
}

fn main() -> Result<()> {
    ensure!(
        Path::new("/.dockerenv").is_file(),
        "run inside the guarded copied-source Docker worker"
    );
    let root = std::env::args_os()
        .nth(1)
        .context("expected retained public compiler work root")?;
    let selected = std::env::args()
        .nth(2)
        .context("expected independently selected compiler evidence SHA-256")?;
    let root = Path::new(&root);
    let evidence_bytes = read(&root.join("evidence.json"))?;
    ensure!(
        hex::encode(Sha256::digest(&evidence_bytes)) == selected,
        "compiler evidence selection mismatch"
    );
    let evidence: serde_json::Value = serde_json::from_slice(&evidence_bytes)?;
    ensure!(
        evidence["outcome"] == "public-compiler-and-provider-verifier-accepted",
        "compiler witness incomplete"
    );
    if evidence["schema"] == "echo.list-worklist-publication/v1" {
        worklists::run(root, &evidence)?;
        println!("PUBLIC_WORKLIST_RUNTIME_ACCEPTED");
        return Ok(());
    }
    pure(root, "pure", false, &evidence)?;
    pure(root, "renamed-pure", false, &evidence)?;
    pure(root, "changed-body", true, &evidence)?;
    let read_usage = atom_read(root, &evidence, "read")?;
    if evidence["results"].get("variant-pure-payload").is_some() {
        closed_variants(root, &evidence, read_usage, "variant")?;
    }
    if evidence["results"].get("match-pure-payload").is_some() {
        closed_variants(root, &evidence, read_usage, "match")?;
    }
    if evidence["results"].get("u8-literal").is_some() {
        unsigned_bytes(root, &evidence)?;
        let u8_usage = atom_read(root, &evidence, "read-u8-constant")?;
        ensure!(
            u8_usage.0 > read_usage.0 && u8_usage.1 > read_usage.1,
            "the additional U8 binding incurred no runtime work/storage"
        );
    }
    if evidence["results"].get("byte-literals").is_some() {
        byte_literals(root, &evidence)?;
    }
    if evidence["results"].get("byte-lookup").is_some() {
        byte_lookup(root, &evidence)?;
        let lookup_usage = atom_read(root, &evidence, "read-byte-lookup")?;
        ensure!(
            lookup_usage.0 > read_usage.0 && lookup_usage.1 > read_usage.1,
            "read lookup incurred no extra work/storage"
        );
        let constant_usage = atom_read(root, &evidence, "read-byte-lookup-constant")?;
        ensure!(
            lookup_usage.0 > constant_usage.0 && lookup_usage.1 > constant_usage.1,
            "read lookup not charged beyond its same-input constant-marker control"
        );
    }
    if evidence["results"].get("fold-pure-0").is_some() {
        state_folds(root, &evidence)?;
    }
    if evidence["results"].get("list-pure-empty").is_some() {
        lists::run(root, &evidence, read_usage)?;
    }
    println!("PUBLIC_SOURCE_FUNCTION_RUNTIME_ACCEPTED");
    Ok(())
}
