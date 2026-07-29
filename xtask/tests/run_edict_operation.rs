#![allow(clippy::expect_used, clippy::panic)]
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! External compiler-to-runtime witness for the generic Edict-operation runner.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_bytes_v1, encode_canonical_cbor_v1,
    CanonicalValueV1,
};

static RUN_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempRunDir(PathBuf);

impl TempRunDir {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask belongs to the Echo workspace")
            .join("target")
            .join("xtask-test-tmp");
        fs::create_dir_all(&root).expect("the xtask fixture root is creatable");
        for _ in 0..1024 {
            let ordinal = RUN_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(
                "run-edict-operation-{}-{ordinal}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    return Self(
                        fs::canonicalize(&path)
                            .expect("the xtask fixture directory is canonicalizable"),
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("failed to create {}: {error}", path.display()),
            }
        }
        panic!("exhausted deterministic xtask fixture attempts");
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRunDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("edict-operation")
        .join(name)
}

fn runner_command(
    package: &Path,
    verification_report: &Path,
    input: &Path,
    wal_dir: &Path,
) -> Command {
    runner_command_with_closure(
        package,
        verification_report,
        &fixture_path("lawpack-manifest.cbor"),
        &fixture_path("lawpack-adapter.cbor"),
        &fixture_path("target-configuration.cbor"),
        input,
        wal_dir,
    )
}

fn runner_command_with_closure(
    package: &Path,
    verification_report: &Path,
    lawpack_manifest: &Path,
    lawpack_adapter: &Path,
    target_configuration: &Path,
    input: &Path,
    wal_dir: &Path,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command
        .arg("run-edict-operation")
        .arg("--package")
        .arg(package)
        .arg("--verification-report")
        .arg(verification_report)
        .arg("--lawpack-manifest")
        .arg(lawpack_manifest)
        .arg("--lawpack-adapter")
        .arg(lawpack_adapter)
        .arg("--target-configuration")
        .arg(target_configuration)
        .arg("--input")
        .arg(input)
        .arg("--wal-dir")
        .arg(wal_dir)
        .arg("--json");
    command
}

fn run_fixture(wal_dir: &Path) -> Output {
    runner_command(
        &fixture_path("executable-operation-package.cbor"),
        &fixture_path("verification-report.cbor"),
        &fixture_path("input.json"),
        wal_dir,
    )
    .output()
    .expect("the generic Edict-operation runner starts")
}

fn assert_rejected(output: &Output, expected_reason: &str) {
    assert!(
        !output.status.success(),
        "invalid boundary input unexpectedly passed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stdout.is_empty(),
        "a rejected run must not emit a passing witness report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected_reason),
        "rejection did not name `{expected_reason}`:\n{stderr}"
    );
}

#[test]
fn compiler_emitted_operation_runs_durably_without_native_callbacks() {
    let run_dir = TempRunDir::new();
    let output = run_fixture(run_dir.path());

    assert!(
        output.status.success(),
        "runner failed: status={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("runner output is JSON");
    assert_eq!(report["operation"], "examples.hello_echo@1.createGreeting");
    assert_eq!(report["artifacts"]["package"]["algorithm"], "sha256");
    assert_eq!(
        report["artifacts"]["package"]["digestHex"],
        "3665d692cdd120f116f18067f2fd583e841448d057b5e35515f57264f853d0f6"
    );
    assert_eq!(
        report["artifacts"]["verificationReport"]["algorithm"],
        "sha256"
    );
    assert_eq!(
        report["artifacts"]["verificationReport"]["digestHex"],
        "2541c8263d95fdad52f4f5a3bbfed48fdefd9f20969156c3c3fd56be912b66dd"
    );
    assert_eq!(
        report["artifacts"]["lawpackManifest"]["algorithm"],
        "sha256"
    );
    assert_eq!(
        report["artifacts"]["lawpackManifest"]["digestHex"],
        "7bb901c984a92ed50795f8b5f7efe8d0648124574fa82250c6373a30e94333c9"
    );
    assert_eq!(report["causalSite"]["basis"], "u0");
    assert_eq!(report["causalSite"]["nodeKey"], "greeting");
    for identity in [
        "worldlineId",
        "warpId",
        "nodeId",
        "submissionId",
        "tickCommitId",
        "receiptDigest",
    ] {
        assert_eq!(
            report["causalSite"][identity]
                .as_str()
                .expect("causal identity is a string")
                .len(),
            64,
            "{identity} must retain one exact 32-byte identity"
        );
    }
    assert!(report["causalSite"]["commitGlobalTick"]
        .as_u64()
        .is_some_and(|tick| tick > 0));
    assert!(report["causalSite"]["worldlineTickAfter"]
        .as_u64()
        .is_some_and(|tick| tick > 0));
    assert_eq!(report["submission"]["walCommittedBeforeAck"], true);
    assert_eq!(report["scheduler"]["actionCount"], 1);
    assert_eq!(report["state"]["valueUtf8"], "Hello Echo");
    assert_eq!(
        report["applicationResult"]["projectionIdentity"],
        "791fb36bb4d42273eb558ce4d03d68d90a678d15891fb9cbe4ad8a20bb56fa82"
    );
    assert_eq!(
        report["applicationResult"]["outputType"],
        "examples.hello_echo@1.GreetingCreated"
    );
    assert_eq!(
        report["applicationResult"]["canonicalBytesHex"],
        "a2636b6579686772656574696e67676d6573736167656a48656c6c6f204563686f"
    );
    assert_eq!(
        report["applicationResult"]["resultIdentity"],
        "bfc50f30e68ac57742ef0fb0ccc41506c1af4a9ecdeca32c0b934a1adccb9860"
    );
    assert_eq!(report["recovery"]["pendingActionRecovered"], true);
    assert_eq!(report["recovery"]["actionRecovered"], true);
    assert_eq!(report["recovery"]["tickRecovered"], true);
    assert_eq!(report["recovery"]["stateRecovered"], true);
    assert_eq!(report["recovery"]["outcomeRecovered"], true);
    assert_eq!(report["recovery"]["receiptRecovered"], true);
    assert_eq!(report["recovery"]["applicationResultRecovered"], true);
    assert_eq!(
        report["duplicate"]["obstruction"],
        "causal.cell@1.AlreadyExists"
    );
    for field in [
        "applicationStateRootBefore",
        "applicationStateRootAfter",
        "targetValueDigestBefore",
        "targetValueDigestAfter",
    ] {
        let digest = report["duplicate"][field]
            .as_str()
            .unwrap_or_else(|| panic!("duplicate witness is missing {field}"));
        assert_eq!(digest.len(), 64, "{field} must be one 32-byte digest");
        assert!(
            digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "{field} must be canonical lowercase hexadecimal"
        );
    }
    assert_eq!(
        report["duplicate"]["applicationStateRootBefore"],
        report["duplicate"]["applicationStateRootAfter"],
        "the duplicate obstruction must leave authoritative application state unchanged"
    );
    assert_eq!(
        report["duplicate"]["targetValueDigestBefore"],
        report["duplicate"]["targetValueDigestAfter"],
        "the duplicate obstruction must leave the target value unchanged"
    );
    assert_eq!(
        report["recovery"]["mutatedInitialStateRefusal"],
        "echo-operation-execution-mismatch/action-basis"
    );
}

#[test]
fn malformed_or_mismatched_compiler_artifacts_fail_closed() {
    let run_dir = TempRunDir::new();
    let malformed_report = runner_command(
        &fixture_path("executable-operation-package.cbor"),
        &fixture_path("executable-operation-package.cbor"),
        &fixture_path("input.json"),
        &run_dir.path().join("malformed-report-wal"),
    )
    .output()
    .expect("the malformed-report case starts");
    assert_rejected(
        &malformed_report,
        "verification report is missing apiVersion",
    );

    let mismatched_package = runner_command(
        &fixture_path("verification-report.cbor"),
        &fixture_path("verification-report.cbor"),
        &fixture_path("input.json"),
        &run_dir.path().join("mismatched-package-wal"),
    )
    .output()
    .expect("the mismatched-package case starts");
    assert_rejected(&mismatched_package, "package is missing schema");
}

#[test]
fn malformed_input_and_nonempty_wal_fail_closed() {
    let run_dir = TempRunDir::new();
    let malformed_input = runner_command(
        &fixture_path("executable-operation-package.cbor"),
        &fixture_path("verification-report.cbor"),
        &fixture_path("ORIGIN.toml"),
        &run_dir.path().join("malformed-input-wal"),
    )
    .output()
    .expect("the malformed-input case starts");
    assert_rejected(&malformed_input, "operation input is not valid JSON");

    let nonempty_wal = run_dir.path().join("nonempty-wal");
    fs::create_dir(&nonempty_wal).expect("the nonempty WAL fixture directory is creatable");
    fs::write(nonempty_wal.join("foreign-state"), b"occupied")
        .expect("the nonempty WAL fixture is writable");
    assert_rejected(
        &run_fixture(&nonempty_wal),
        "runtime WAL directory must be empty",
    );
}

#[test]
fn replacement_bound_is_enforced_before_runtime_submission() {
    let run_dir = TempRunDir::new();
    let input = run_dir.path().join("oversized-input.json");
    fs::write(
        &input,
        serde_json::to_vec(&serde_json::json!({
            "basis": "u0",
            "key": "oversized",
            "value": "x".repeat(1_025),
        }))
        .expect("the oversized input encodes"),
    )
    .expect("the oversized input fixture is writable");
    let output = runner_command(
        &fixture_path("executable-operation-package.cbor"),
        &fixture_path("verification-report.cbor"),
        &input,
        &run_dir.path().join("oversized-input-wal"),
    )
    .output()
    .expect("the oversized-input case starts");
    assert_rejected(
        &output,
        "operation replacement exceeds the configured byte bound",
    );

    let host_oversized_input = run_dir.path().join("host-oversized-input.json");
    fs::write(&host_oversized_input, vec![b'x'; 65_537])
        .expect("the host-oversized input fixture is writable");
    let host_bounded = runner_command(
        &fixture_path("executable-operation-package.cbor"),
        &fixture_path("verification-report.cbor"),
        &host_oversized_input,
        &run_dir.path().join("host-oversized-input-wal"),
    )
    .output()
    .expect("the host-oversized-input case starts");
    assert_rejected(
        &host_bounded,
        "operation input exceeds the 65536-byte host bound",
    );
}

#[test]
fn verification_report_target_ir_must_match_the_package_semantic_closure() {
    let run_dir = TempRunDir::new();
    let report_path = run_dir.path().join("target-ir-substitution.cbor");
    let mut report = decode_canonical_cbor_v1(
        &fs::read(fixture_path("verification-report.cbor"))
            .expect("the verification report fixture is readable"),
    )
    .expect("the verification report fixture is canonical");
    let target_ir = map_field_mut(&mut report, "targetIr");
    let digest = resource_digest_mut(target_ir);
    digest[0] ^= 0xff;
    fs::write(
        &report_path,
        encode_canonical_cbor_v1(&report).expect("the substituted report re-encodes"),
    )
    .expect("the substituted report is writable");

    let output = runner_command(
        &fixture_path("executable-operation-package.cbor"),
        &report_path,
        &fixture_path("input.json"),
        &run_dir.path().join("target-ir-substitution-wal"),
    )
    .output()
    .expect("the Target IR substitution case starts");
    assert_rejected(
        &output,
        "verification report Target IR does not bind the package semantic closure",
    );
}

#[test]
fn verification_report_must_bind_the_compiler_result_projection() {
    let run_dir = TempRunDir::new();
    let report_path = run_dir.path().join("result-projection-substitution.cbor");
    let mut report = decode_fixture("verification-report.cbor");
    let result_projection = map_field_mut(&mut report, "applicationResultProjection");
    resource_digest_mut(result_projection)[0] ^= 0xff;
    fs::write(
        &report_path,
        encode_canonical_cbor_v1(&report).expect("the substituted report re-encodes"),
    )
    .expect("the substituted report is writable");

    let output = runner_command(
        &fixture_path("executable-operation-package.cbor"),
        &report_path,
        &fixture_path("input.json"),
        &run_dir.path().join("result-projection-substitution-wal"),
    )
    .output()
    .expect("the result-projection substitution case starts");
    assert_rejected(
        &output,
        "verification report does not bind the package application-result projection",
    );
}

#[test]
fn corrupted_result_projection_bytes_fail_echo_package_admission() {
    let run_dir = TempRunDir::new();
    let package_path = run_dir
        .path()
        .join("corrupt-result-projection-package.cbor");
    let report_path = run_dir.path().join("corrupt-result-projection-report.cbor");

    let mut package = decode_fixture("executable-operation-package.cbor");
    let projection = map_field_mut(&mut package, "application_result_projection");
    bytes_field_mut(projection, "artifact_bytes")[0] ^= 0xff;
    fs::write(
        &package_path,
        encode_canonical_cbor_v1(&package).expect("the corrupted package re-encodes"),
    )
    .expect("the corrupted package is writable");

    let package_digest = digest_canonical_value_bytes_v1("echo.operation-package/v1", &package)
        .expect("the rebound package identity is computable");
    let mut report = decode_fixture("verification-report.cbor");
    resource_digest_mut(map_field_mut(&mut report, "package")).copy_from_slice(&package_digest);
    fs::write(
        &report_path,
        encode_canonical_cbor_v1(&report).expect("the rebound report re-encodes"),
    )
    .expect("the rebound report is writable");

    let output = runner_command(
        &package_path,
        &report_path,
        &fixture_path("input.json"),
        &run_dir.path().join("corrupt-result-projection-wal"),
    )
    .output()
    .expect("the corrupted projection case starts");
    assert_rejected(
        &output,
        "Echo independently refused the compiler-produced package",
    );
}

#[test]
fn target_configuration_must_belong_to_the_selected_effect() {
    let run_dir = TempRunDir::new();
    let adapter_path = run_dir.path().join("cross-effect-adapter.cbor");
    let manifest_path = run_dir.path().join("cross-effect-manifest.cbor");
    let package_path = run_dir.path().join("cross-effect-package.cbor");
    let report_path = run_dir.path().join("cross-effect-report.cbor");

    let mut adapter = decode_fixture("lawpack-adapter.cbor");
    let implementations = map_field_mut(&mut adapter, "effectImplementations");
    let CanonicalValueV1::Map(entries) = implementations else {
        panic!("effect implementations must be a canonical map");
    };
    let selected_index = entries
        .iter()
        .position(|(key, _)| {
            matches!(
                key,
                CanonicalValueV1::Text(key) if key == "causal.cell@1.createIfAbsent"
            )
        })
        .expect("fixture adapter exposes the selected create-if-absent effect");
    let mut unrelated = entries[selected_index].1.clone();
    resource_digest_mut(map_field_mut(
        &mut entries[selected_index].1,
        "targetConfiguration",
    ))[0] ^= 0xff;
    *map_field_mut(&mut unrelated, "targetIntrinsic") =
        CanonicalValueV1::Text("echo.dpo@1.unrelated".to_owned());
    entries.push((
        CanonicalValueV1::Text("causal.cell@1.unrelated".to_owned()),
        unrelated,
    ));
    fs::write(
        &adapter_path,
        encode_canonical_cbor_v1(&adapter).expect("cross-effect adapter re-encodes"),
    )
    .expect("cross-effect adapter is writable");

    let adapter_digest = digest_canonical_value_bytes_v1("causal.cell.echo-adapter/v1", &adapter)
        .expect("cross-effect adapter identity is computable");
    let mut manifest = decode_fixture("lawpack-manifest.cbor");
    let target_adapters = array_field_mut(&mut manifest, "targetAdapters");
    resource_digest_mut(map_field_mut(&mut target_adapters[0], "adapter"))
        .copy_from_slice(&adapter_digest);
    fs::write(
        &manifest_path,
        encode_canonical_cbor_v1(&manifest).expect("cross-effect manifest re-encodes"),
    )
    .expect("cross-effect manifest is writable");

    let manifest_digest = digest_canonical_value_bytes_v1("edict.lawpack/v1", &manifest)
        .expect("cross-effect manifest identity is computable");
    let mut package = decode_fixture("executable-operation-package.cbor");
    let semantic_closure = map_field_mut(&mut package, "semantic_closure");
    bytes_field_mut(semantic_closure, "lawpack_identity").copy_from_slice(&manifest_digest);
    fs::write(
        &package_path,
        encode_canonical_cbor_v1(&package).expect("cross-effect package re-encodes"),
    )
    .expect("cross-effect package is writable");

    let package_digest = digest_canonical_value_bytes_v1("echo.operation-package/v1", &package)
        .expect("cross-effect package identity is computable");
    let mut report = decode_fixture("verification-report.cbor");
    resource_digest_mut(map_field_mut(&mut report, "package")).copy_from_slice(&package_digest);
    fs::write(
        &report_path,
        encode_canonical_cbor_v1(&report).expect("cross-effect report re-encodes"),
    )
    .expect("cross-effect report is writable");

    let output = runner_command_with_closure(
        &package_path,
        &report_path,
        &manifest_path,
        &adapter_path,
        &fixture_path("target-configuration.cbor"),
        &fixture_path("input.json"),
        &run_dir.path().join("cross-effect-wal"),
    )
    .output()
    .expect("the cross-effect substitution case starts");
    assert_rejected(
        &output,
        "selected lawpack effect implementation does not bind the supplied target configuration",
    );
}

#[test]
fn fixed_seed_keys_preserve_the_generic_durable_witness_under_bounded_stress() {
    const FIXED_SEED: u64 = 0x5eed_cafe_f00d_beef;
    const CASES: usize = 8;

    let run_dir = TempRunDir::new();
    let mut state = FIXED_SEED;
    let mut retained_receipts = BTreeSet::new();
    for index in 0..CASES {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let replacement = format!("value-{state:016x}");
        let input = run_dir.path().join(format!("input-{index}.json"));
        fs::write(
            &input,
            serde_json::to_vec(&serde_json::json!({
                "basis": format!("basis-{index}"),
                "key": format!("key-{state:016x}"),
                "value": replacement,
            }))
            .expect("the fixed-seed input encodes"),
        )
        .expect("the fixed-seed input fixture is writable");
        let output = runner_command(
            &fixture_path("executable-operation-package.cbor"),
            &fixture_path("verification-report.cbor"),
            &input,
            &run_dir.path().join(format!("wal-{index}")),
        )
        .output()
        .expect("the fixed-seed runner case starts");
        assert!(
            output.status.success(),
            "fixed-seed case {index} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("fixed-seed output is JSON");
        assert_eq!(report["state"]["valueUtf8"], replacement);
        assert_eq!(report["causalSite"]["basis"], format!("basis-{index}"));
        assert_eq!(report["causalSite"]["nodeKey"], format!("key-{state:016x}"));
        assert_eq!(
            report["applicationResult"]["outputType"],
            "examples.hello_echo@1.GreetingCreated"
        );
        assert_eq!(report["recovery"]["applicationResultRecovered"], true);
        let result = decode_canonical_cbor_v1(
            &hex::decode(
                report["applicationResult"]["canonicalBytesHex"]
                    .as_str()
                    .expect("application result bytes are hexadecimal"),
            )
            .expect("application result bytes use valid hexadecimal"),
        )
        .expect("application result bytes remain canonical");
        assert_eq!(text_field(&result, "key"), format!("key-{state:016x}"));
        assert_eq!(text_field(&result, "message"), replacement);
        assert!(
            retained_receipts.insert(
                report["causalSite"]["receiptDigest"]
                    .as_str()
                    .expect("causal receipt identity is a string")
                    .to_owned()
            ),
            "fixed-seed cases must retain distinct causal receipts"
        );
        assert_eq!(
            report["duplicate"]["obstruction"],
            "causal.cell@1.AlreadyExists"
        );
    }
    assert_eq!(retained_receipts.len(), CASES);
}

fn map_field_mut<'a>(value: &'a mut CanonicalValueV1, name: &str) -> &'a mut CanonicalValueV1 {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("fixture value must be a canonical map");
    };
    entries
        .iter_mut()
        .find_map(|(key, value)| match key {
            CanonicalValueV1::Text(key) if key == name => Some(value),
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture map is missing {name}"))
}

fn text_field<'a>(value: &'a CanonicalValueV1, name: &str) -> &'a str {
    let CanonicalValueV1::Map(entries) = value else {
        panic!("fixture value must be a canonical map");
    };
    entries
        .iter()
        .find_map(|(key, value)| match (key, value) {
            (CanonicalValueV1::Text(key), CanonicalValueV1::Text(value)) if key == name => {
                Some(value.as_str())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture map is missing text field {name}"))
}

fn array_field_mut<'a>(
    value: &'a mut CanonicalValueV1,
    name: &str,
) -> &'a mut Vec<CanonicalValueV1> {
    let value = map_field_mut(value, name);
    let CanonicalValueV1::Array(values) = value else {
        panic!("fixture field {name} must be an array");
    };
    values
}

fn bytes_field_mut<'a>(value: &'a mut CanonicalValueV1, name: &str) -> &'a mut Vec<u8> {
    let value = map_field_mut(value, name);
    let CanonicalValueV1::Bytes(bytes) = value else {
        panic!("fixture field {name} must be bytes");
    };
    bytes
}

fn resource_digest_mut(resource: &mut CanonicalValueV1) -> &mut Vec<u8> {
    let digest = map_field_mut(resource, "digest");
    let CanonicalValueV1::Array(parts) = digest else {
        panic!("fixture resource digest must be an array");
    };
    let [CanonicalValueV1::Text(algorithm), CanonicalValueV1::Bytes(bytes)] = parts.as_mut_slice()
    else {
        panic!("fixture resource digest must be [algorithm, bytes]");
    };
    assert_eq!(algorithm, "sha256");
    assert_eq!(bytes.len(), 32);
    bytes
}

fn decode_fixture(name: &str) -> CanonicalValueV1 {
    decode_canonical_cbor_v1(
        &fs::read(fixture_path(name)).expect("the canonical fixture is readable"),
    )
    .expect("the canonical fixture decodes")
}
