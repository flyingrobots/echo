#![allow(clippy::expect_used, clippy::panic)]
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! External compiler-to-runtime witness for the generic Edict-operation runner.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use echo_edict_canonical::{decode_canonical_cbor_v1, encode_canonical_cbor_v1, CanonicalValueV1};

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
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command
        .arg("run-edict-operation")
        .arg("--package")
        .arg(package)
        .arg("--verification-report")
        .arg(verification_report)
        .arg("--lawpack-manifest")
        .arg(fixture_path("lawpack-manifest.cbor"))
        .arg("--lawpack-adapter")
        .arg(fixture_path("lawpack-adapter.cbor"))
        .arg("--target-configuration")
        .arg(fixture_path("target-configuration.cbor"))
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
        "8a602e9bf2dfeae1a3bc033d299dc7b6b348a08c777756c7b1b855bd099dab93"
    );
    assert_eq!(
        report["artifacts"]["verificationReport"]["algorithm"],
        "sha256"
    );
    assert_eq!(
        report["artifacts"]["verificationReport"]["digestHex"],
        "8a72bff53649a448d7f880ed27bb21a52daab3a43eb2e0579e46ac35296e34dd"
    );
    assert_eq!(
        report["artifacts"]["lawpackManifest"]["algorithm"],
        "sha256"
    );
    assert_eq!(
        report["artifacts"]["lawpackManifest"]["digestHex"],
        "7bb901c984a92ed50795f8b5f7efe8d0648124574fa82250c6373a30e94333c9"
    );
    assert_eq!(report["submission"]["walCommittedBeforeAck"], true);
    assert_eq!(report["scheduler"]["actionCount"], 1);
    assert_eq!(report["state"]["valueUtf8"], "Hello Echo");
    assert_eq!(report["recovery"]["pendingActionRecovered"], true);
    assert_eq!(report["recovery"]["actionRecovered"], true);
    assert_eq!(report["recovery"]["tickRecovered"], true);
    assert_eq!(report["recovery"]["stateRecovered"], true);
    assert_eq!(report["recovery"]["outcomeRecovered"], true);
    assert_eq!(report["recovery"]["receiptRecovered"], true);
    assert_eq!(
        report["duplicate"]["obstruction"],
        "echo.executable-operation/precondition-mismatch/v1"
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
fn fixed_seed_keys_preserve_the_generic_durable_witness_under_bounded_stress() {
    const FIXED_SEED: u64 = 0x5eed_cafe_f00d_beef;
    const CASES: usize = 8;

    let run_dir = TempRunDir::new();
    let mut state = FIXED_SEED;
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
        assert_eq!(
            report["duplicate"]["obstruction"],
            "echo.executable-operation/precondition-mismatch/v1"
        );
    }
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
