// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Exact owner/carrier and Cargo-archive witnesses for provider package assets.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const EXPECTED_ASSET_FILE_COUNT: usize = 38;
const EXPECTED_LOWERER_RESOURCE_COUNT: usize = 4;
const EXPECTED_VERIFIER_RESOURCE_COUNT: usize = 15;
static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "echo-provider-assets-{label}-{}-{sequence}",
            std::process::id()
        ));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("stale asset test directory is removable");
        }
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.0));
    }
}

fn run_assets(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_echo-edict-provider-assets"))
        .args(arguments)
        .output()
        .expect("provider asset synchronizer executes")
}

fn count_files(path: &Path) -> usize {
    std::fs::read_dir(path)
        .expect("asset directory is readable")
        .map(|entry| entry.expect("asset entry is readable"))
        .map(|entry| {
            let file_type = entry.file_type().expect("asset entry type is readable");
            if file_type.is_dir() {
                count_files(&entry.path())
            } else {
                usize::from(file_type.is_file())
            }
        })
        .sum()
}

#[test]
fn fixed_assets_match_their_current_owners_and_cargo_archive() {
    let checked = run_assets(&["--check-package-list"]);
    assert!(
        checked.status.success(),
        "provider asset/archive check failed:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

#[test]
fn asset_check_reports_drift_without_repair_and_write_restores_exact_bytes() {
    let directory = TestDirectory::new("check-write");
    let assets_root = directory.path().join("v1");
    let assets_root_text = assets_root.to_string_lossy();

    let written = run_assets(&["--write", "--assets-root", &assets_root_text]);
    assert!(
        written.status.success(),
        "provider asset write failed:\n{}",
        String::from_utf8_lossy(&written.stderr)
    );
    assert_eq!(count_files(&assets_root), EXPECTED_ASSET_FILE_COUNT);

    let changed_path = assets_root.join("repository/rust-toolchain.toml.source");
    std::fs::write(&changed_path, b"tampered toolchain")
        .expect("asset carrier is deliberately changed");
    let checked = run_assets(&["--assets-root", &assets_root_text]);
    assert!(!checked.status.success());
    assert!(String::from_utf8_lossy(&checked.stderr)
        .contains("changed: repository/rust-toolchain.toml.source"));
    assert_eq!(
        std::fs::read(&changed_path).expect("changed carrier remains readable"),
        b"tampered toolchain"
    );

    let restored = run_assets(&["--write", "--assets-root", &assets_root_text]);
    assert!(
        restored.status.success(),
        "provider asset restore failed:\n{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let final_check = run_assets(&["--assets-root", &assets_root_text]);
    assert!(
        final_check.status.success(),
        "restored provider assets do not check:\n{}",
        String::from_utf8_lossy(&final_check.stderr)
    );
}

#[test]
fn component_resource_check_reports_drift_and_write_restores_exact_bytes() {
    let directory = TestDirectory::new("component-resources");
    let assets_root = directory.path().join("assets-v1");
    let lowerer_root = directory.path().join("lowerer-resources");
    let verifier_root = directory.path().join("verifier-resources");
    let assets_root_text = assets_root.to_string_lossy();
    let lowerer_root_text = lowerer_root.to_string_lossy();
    let verifier_root_text = verifier_root.to_string_lossy();
    let roots = [
        "--assets-root",
        assets_root_text.as_ref(),
        "--check-package-list",
        "--sync-component-resources",
        "--lowerer-resources-root",
        lowerer_root_text.as_ref(),
        "--verifier-resources-root",
        verifier_root_text.as_ref(),
    ];

    let mut write_arguments = vec!["--write"];
    write_arguments.extend(roots);
    let written = run_assets(&write_arguments);
    assert!(
        written.status.success(),
        "component resource write failed:\n{}",
        String::from_utf8_lossy(&written.stderr)
    );
    let written_stdout = String::from_utf8_lossy(&written.stdout);
    assert!(written_stdout.contains("Package-local provider assets synchronized"));
    assert!(written_stdout.contains("Cargo package selects the exact provider asset tree"));
    assert!(written_stdout.contains("Provider component resources synchronized"));
    assert_eq!(count_files(&assets_root), EXPECTED_ASSET_FILE_COUNT);
    assert_eq!(count_files(&lowerer_root), EXPECTED_LOWERER_RESOURCE_COUNT);
    assert_eq!(
        count_files(&verifier_root),
        EXPECTED_VERIFIER_RESOURCE_COUNT
    );

    let changed_path = verifier_root.join("generated-artifact-profile.echo-dpo-registration.cbor");
    std::fs::write(&changed_path, b"tampered verifier resource")
        .expect("verifier resource is deliberately changed");
    let checked = run_assets(&roots);
    assert!(!checked.status.success());
    assert!(String::from_utf8_lossy(&checked.stderr)
        .contains("changed: generated-artifact-profile.echo-dpo-registration.cbor"));
    assert_eq!(
        std::fs::read(&changed_path).expect("changed verifier resource remains readable"),
        b"tampered verifier resource"
    );

    let restored = run_assets(&write_arguments);
    assert!(
        restored.status.success(),
        "component resource restore failed:\n{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let final_check = run_assets(&roots);
    assert!(
        final_check.status.success(),
        "restored component resources do not check:\n{}",
        String::from_utf8_lossy(&final_check.stderr)
    );
    let final_stdout = String::from_utf8_lossy(&final_check.stdout);
    assert!(final_stdout.contains("Package-local provider assets are current"));
    assert!(final_stdout.contains("Cargo package selects the exact provider asset tree"));
    assert!(final_stdout.contains("Provider component resources are current"));
}
