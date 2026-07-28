#![allow(clippy::expect_used, clippy::panic)]
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! External compiler-to-runtime witness for the generic Edict-operation runner.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static RUN_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempRunDir(PathBuf);

impl TempRunDir {
    fn new() -> Self {
        let root = PathBuf::from("target").join("xtask-test-tmp");
        fs::create_dir_all(&root).expect("the xtask fixture root is creatable");
        for _ in 0..1024 {
            let ordinal = RUN_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!("run-edict-operation-{ordinal}"));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
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

#[test]
fn compiler_emitted_operation_runs_durably_without_native_callbacks() {
    let fixture = Path::new("xtask/tests/fixtures/edict-operation");
    let run_dir = TempRunDir::new();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "run-edict-operation",
            "--package",
            fixture
                .join("executable-operation-package.cbor")
                .to_str()
                .expect("fixture path is UTF-8"),
            "--verification-report",
            fixture
                .join("verification-report.cbor")
                .to_str()
                .expect("fixture path is UTF-8"),
            "--lawpack-manifest",
            fixture
                .join("lawpack-manifest.cbor")
                .to_str()
                .expect("fixture path is UTF-8"),
            "--lawpack-adapter",
            fixture
                .join("lawpack-adapter.cbor")
                .to_str()
                .expect("fixture path is UTF-8"),
            "--target-configuration",
            fixture
                .join("target-configuration.cbor")
                .to_str()
                .expect("fixture path is UTF-8"),
            "--input",
            fixture
                .join("input.json")
                .to_str()
                .expect("fixture path is UTF-8"),
            "--wal-dir",
            run_dir.path().to_str().expect("run path is UTF-8"),
            "--json",
        ])
        .output()
        .expect("the generic Edict-operation runner starts");

    assert!(
        output.status.success(),
        "runner failed: status={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("runner output is JSON");
    assert_eq!(report["verdict"], "pass");
    assert_eq!(report["operation"], "examples.hello_echo@1.createGreeting");
    assert_eq!(
        report["artifacts"]["packageSha256"],
        "8a602e9bf2dfeae1a3bc033d299dc7b6b348a08c777756c7b1b855bd099dab93"
    );
    assert_eq!(report["artifacts"]["verificationOutcome"], "accepted");
    assert_eq!(report["submission"]["walCommittedBeforeAck"], true);
    assert_eq!(report["scheduler"]["actionCount"], 1);
    assert_eq!(report["state"]["valueUtf8"], "Hello Echo");
    assert_eq!(report["recovery"]["actionRecovered"], true);
    assert_eq!(report["recovery"]["tickRecovered"], true);
    assert_eq!(report["recovery"]["stateRecovered"], true);
    assert_eq!(report["recovery"]["outcomeRecovered"], true);
    assert_eq!(report["recovery"]["receiptRecovered"], true);
    assert_eq!(
        report["duplicate"]["obstruction"],
        "echo.executable-operation/precondition-mismatch/v1"
    );
    assert_eq!(report["duplicate"]["hiddenMutation"], false);
    assert_eq!(report["recovery"]["mutatedInitialStateRefused"], true);
}
