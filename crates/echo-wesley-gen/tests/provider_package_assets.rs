// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Exact owner/carrier and Cargo-archive witnesses for provider package assets.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const EXPECTED_ASSET_FILE_COUNT: usize = 35;
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
