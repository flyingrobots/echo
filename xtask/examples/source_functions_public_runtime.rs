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
use warp_core::edict_read::{ReadBasis, ReadView};
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
fn atom_read(root: &Path, evidence: &serde_json::Value) -> Result<()> {
    let (package_bytes, report_bytes) = emitted(&root.join("read"), evidence)?;
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
    let input = encode(&record([
        (
            "address",
            record([
                ("warpId", Value::Bytes(key.warp_id.0.to_vec())),
                ("nodeId", Value::Bytes(key.local_id.0.to_vec())),
                ("typeId", Value::Bytes(ty.0.to_vec())),
            ]),
        ),
        ("expected", Value::Bytes(b"alpha".to_vec())),
    ]))?;
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
        decode(&result.output)? == Value::Bytes(b"alpha".to_vec()),
        "wrong stored-atom result"
    );
    ensure!(
        result.basis == basis
            && result.reads == 1
            && result.read_bytes == 5
            && frontier.state().state_root() == before,
        "wrong read evidence or mutation"
    );
    println!("PUBLIC_READ_RUNTIME_OK 1 5");
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
    pure(root, "pure", false, &evidence)?;
    pure(root, "renamed-pure", false, &evidence)?;
    pure(root, "changed-body", true, &evidence)?;
    atom_read(root, &evidence)?;
    println!("PUBLIC_SOURCE_FUNCTION_RUNTIME_ACCEPTED");
    Ok(())
}
