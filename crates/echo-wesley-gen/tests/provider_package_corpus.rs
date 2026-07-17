// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Checked filesystem publication for Echo's digest-locked Edict provider package.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest as _, Sha256};

const EXPECTED_PROVIDER_DIGEST: &str =
    "sha256:e0ccd4503c7f5830a1affa1c5a676f866aa0fab976a5ec2a0075c70916a64b69";
const EXPECTED_MANIFEST_RAW_SHA256: &str =
    "fe5d70f5581247c29569924679a5fb3fdb46e9b81f252b42488aecda0c61b9fb";

const PACKAGE_PATHS: [&str; 25] = [
    "components/lowerer.echo-dpo.component.wasm",
    "components/verifier.echo-dpo.component.wasm",
    "generated/evidence/provenance.provider-generation.json",
    "generated/evidence/review.provider-generation.json",
    "generated/primary/authority-facts.echo-dpo.cbor",
    "generated/primary/authority-facts.echo-lawpack.cbor",
    "generated/primary/generated-artifact-profile.echo-dpo-registration.cbor",
    "generated/primary/lawpack.echo-dpo.cbor",
    "generated/primary/schema.echo-provider-artifacts.cddl",
    "generated/primary/target-profile.echo-dpo.cbor",
    "generated/resources/resource.conformance-corpus.cbor",
    "generated/resources/resource.lawpack-compatibility.cbor",
    "generated/resources/resource.lawpack-exports.cbor",
    "generated/resources/resource.lawpack-target-adapter.cbor",
    "generated/resources/resource.lawpack-verifier.cbor",
    "generated/resources/resource.target-bundle-profile.cbor",
    "generated/resources/resource.target-cost-algebra.cbor",
    "generated/resources/resource.target-footprint-algebra.cbor",
    "generated/resources/resource.target-intrinsics.cbor",
    "generated/resources/resource.target-ir.cbor",
    "generated/resources/resource.target-lowerer-contract.cbor",
    "generated/resources/resource.target-obstruction-taxonomy.cbor",
    "generated/resources/resource.target-operation-profiles.cbor",
    "generated/resources/resource.target-verifier-contract.cbor",
    "provider-manifest.echo.json",
];

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "echo-provider-package-{label}-{}-{sequence}",
            std::process::id()
        ));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("stale test directory is removable");
        }
        std::fs::create_dir_all(&path).expect("test directory is created");
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

fn run_package(root: &Path, check: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_echo-edict-provider-package"));
    if check {
        command.arg("--check");
    }
    command
        .arg("--out")
        .arg(root)
        .output()
        .expect("provider package command executes")
}

fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    collect_files(root, root, &mut files);
    files
}

fn collect_files(root: &Path, directory: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    let mut entries = std::fs::read_dir(directory)
        .expect("package directory is readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("package directory entries are readable");
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let file_type = entry.file_type().expect("package entry type is readable");
        if file_type.is_dir() {
            collect_files(root, &entry.path(), files);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(root)
            .expect("package entry is below the corpus root")
            .components()
            .map(|component| {
                component
                    .as_os_str()
                    .to_str()
                    .expect("package paths are UTF-8")
            })
            .collect::<Vec<_>>()
            .join("/");
        files.insert(
            relative,
            std::fs::read(entry.path()).expect("package file is readable"),
        );
    }
}

fn expected_paths() -> Vec<String> {
    PACKAGE_PATHS
        .iter()
        .map(|path| (*path).to_owned())
        .collect()
}

#[test]
fn generation_writes_the_exact_package_and_check_mode_is_read_only() {
    let directory = TestDirectory::new("write-check");
    let generated = run_package(directory.path(), false);
    assert!(
        generated.status.success(),
        "provider package generation failed:\n{}",
        String::from_utf8_lossy(&generated.stderr)
    );

    let before = snapshot(directory.path());
    assert_eq!(before.keys().cloned().collect::<Vec<_>>(), expected_paths());
    let manifest_bytes = before
        .get("provider-manifest.echo.json")
        .expect("provider manifest is present");
    let manifest: serde_json::Value =
        serde_json::from_slice(manifest_bytes).expect("provider manifest is JSON");
    assert_eq!(
        manifest["provider"]["digest"].as_str(),
        Some(EXPECTED_PROVIDER_DIGEST)
    );
    assert_eq!(
        hex::encode(Sha256::digest(manifest_bytes)),
        EXPECTED_MANIFEST_RAW_SHA256
    );

    let checked = run_package(directory.path(), true);
    assert!(
        checked.status.success(),
        "provider package check failed:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(String::from_utf8_lossy(&checked.stdout).contains("is current"));
    assert_eq!(snapshot(directory.path()), before);
}

#[test]
fn check_mode_reports_drift_without_rewriting_or_creating_members() {
    let directory = TestDirectory::new("no-write-check");
    let generated = run_package(directory.path(), false);
    assert!(
        generated.status.success(),
        "provider package generation failed:\n{}",
        String::from_utf8_lossy(&generated.stderr)
    );

    let changed_path = directory.path().join("provider-manifest.echo.json");
    let missing_path = directory
        .path()
        .join("components/lowerer.echo-dpo.component.wasm");
    let unexpected_path = directory.path().join("operator-owned.txt");
    std::fs::write(&changed_path, b"tampered manifest")
        .expect("changed package manifest is written");
    std::fs::remove_file(&missing_path).expect("lowerer package member is removed");
    std::fs::write(&unexpected_path, b"operator bytes")
        .expect("unexpected operator file is written");

    let checked = run_package(directory.path(), true);
    assert!(!checked.status.success());
    let stderr = String::from_utf8(checked.stderr).expect("checker diagnostics are UTF-8");
    assert!(stderr.contains("missing: components/lowerer.echo-dpo.component.wasm"));
    assert!(stderr.contains("changed: provider-manifest.echo.json"));
    assert!(stderr.contains("unexpected: operator-owned.txt"));
    assert_eq!(
        std::fs::read(&changed_path).expect("tampered manifest remains readable"),
        b"tampered manifest"
    );
    assert!(!missing_path.exists());
    assert_eq!(
        std::fs::read(&unexpected_path).expect("operator file remains readable"),
        b"operator bytes"
    );
}

#[test]
fn generation_refuses_unexpected_entries_before_writing_package_members() {
    let directory = TestDirectory::new("unexpected-entry");
    let unexpected_path = directory.path().join("operator-owned.txt");
    std::fs::write(&unexpected_path, b"operator bytes")
        .expect("unexpected operator file is written");

    let generated = run_package(directory.path(), false);
    assert!(!generated.status.success());
    assert!(String::from_utf8_lossy(&generated.stderr)
        .contains("refusing to generate over unexpected corpus entry operator-owned.txt"));
    assert_eq!(
        std::fs::read(&unexpected_path).expect("operator file remains readable"),
        b"operator bytes"
    );
    assert!(!directory.path().join("components").exists());
    assert!(!directory.path().join("generated").exists());
    assert!(!directory
        .path()
        .join("provider-manifest.echo.json")
        .exists());
}

#[test]
fn checked_provider_package_matches_the_current_renderer() {
    let package_root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/v1/edict-provider/package/v1");
    let checked = run_package(&package_root, true);

    assert!(
        checked.status.success(),
        "checked provider package drifted:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(String::from_utf8_lossy(&checked.stdout).contains("is current"));
}
