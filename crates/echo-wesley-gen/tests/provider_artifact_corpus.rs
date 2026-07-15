// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Checked deterministic corpus for Echo's generated Edict provider artifacts.

use echo_wesley_gen::provider_artifacts::{
    generate_provider_primary_artifacts_v1, ProviderPrimaryArtifactsV1,
};
use echo_wesley_gen::provider_contract_pack::{
    admit_provider_contract_pack_v1, AdmittedProviderContractPackV1,
};
use echo_wesley_gen::provider_corpus::{
    build_provider_generator_source_bundle_v1, checked_provider_generator_source_bundle_v1,
    diff_provider_artifact_corpus_v1, render_provider_artifact_corpus_v1,
    ProviderArtifactCorpusDriftKind, ProviderArtifactCorpusErrorKind, ProviderArtifactCorpusFileV1,
    ProviderGeneratorSourceFileV1,
};
use echo_wesley_gen::provider_generation::{
    build_provider_generation_input_v1, ProviderGenerationInputV1,
};
use echo_wesley_gen::provider_provenance::{
    generate_provider_generation_provenance_v1, ProviderGenerationProvenanceV1,
    ProviderGeneratorMaterialV1,
};
use echo_wesley_gen::provider_review::{
    generate_provider_generation_review_v1, ProviderGenerationReviewV1,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const SOURCE: &[u8] =
    include_bytes!("../../../schemas/edict-provider/echo-provider-semantics-v1.json");
const SETTINGS: &[u8] =
    include_bytes!("../../../schemas/edict-provider/generation-settings-v1.json");
const CONTRACT_CDDL: &[u8] =
    include_bytes!("../../../schemas/edict-provider/contracts/v1/edict-provider-contracts.cddl");
const CONTRACT_MANIFEST: &[u8] =
    include_bytes!("../../../schemas/edict-provider/contracts/v1/manifest.json");

const GENERATOR_SOURCE_PATHS: [&str; 16] = [
    "Cargo.lock",
    "Cargo.toml",
    "crates/echo-edict-canonical/Cargo.toml",
    "crates/echo-edict-canonical/src/lib.rs",
    "crates/echo-wesley-gen/Cargo.toml",
    "crates/echo-wesley-gen/src/bin/echo-edict-provider-artifacts.rs",
    "crates/echo-wesley-gen/src/lib.rs",
    "crates/echo-wesley-gen/src/provider_artifacts.rs",
    "crates/echo-wesley-gen/src/provider_canonical.rs",
    "crates/echo-wesley-gen/src/provider_contract_pack.rs",
    "crates/echo-wesley-gen/src/provider_corpus.rs",
    "crates/echo-wesley-gen/src/provider_generation.rs",
    "crates/echo-wesley-gen/src/provider_provenance.rs",
    "crates/echo-wesley-gen/src/provider_review.rs",
    "crates/echo-wesley-gen/src/provider_semantics.rs",
    "rust-toolchain.toml",
];

const CORPUS_PATHS: [&str; 22] = [
    "evidence/provenance.provider-generation.json",
    "evidence/review.provider-generation.json",
    "primary/authority-facts.echo-dpo.cbor",
    "primary/authority-facts.echo-lawpack.cbor",
    "primary/generated-artifact-profile.echo-dpo-registration.cbor",
    "primary/lawpack.echo-dpo.cbor",
    "primary/schema.echo-provider-artifacts.cddl",
    "primary/target-profile.echo-dpo.cbor",
    "resources/resource.conformance-corpus.cbor",
    "resources/resource.lawpack-compatibility.cbor",
    "resources/resource.lawpack-exports.cbor",
    "resources/resource.lawpack-target-adapter.cbor",
    "resources/resource.lawpack-verifier.cbor",
    "resources/resource.target-bundle-profile.cbor",
    "resources/resource.target-cost-algebra.cbor",
    "resources/resource.target-footprint-algebra.cbor",
    "resources/resource.target-intrinsics.cbor",
    "resources/resource.target-ir.cbor",
    "resources/resource.target-lowerer-contract.cbor",
    "resources/resource.target-obstruction-taxonomy.cbor",
    "resources/resource.target-operation-profiles.cbor",
    "resources/resource.target-verifier-contract.cbor",
];

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "echo-provider-corpus-{label}-{}-{sequence}",
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

fn admitted_pack() -> AdmittedProviderContractPackV1 {
    admit_provider_contract_pack_v1(CONTRACT_CDDL, CONTRACT_MANIFEST)
        .expect("checked Edict provider contract pack is admitted")
}

fn generate() -> (
    ProviderGenerationInputV1,
    ProviderPrimaryArtifactsV1,
    ProviderGeneratorMaterialV1,
    ProviderGenerationProvenanceV1,
    ProviderGenerationReviewV1,
) {
    let pack = admitted_pack();
    let input = build_provider_generation_input_v1(SOURCE, &pack, SETTINGS)
        .expect("checked provider generation input builds");
    let primary = generate_provider_primary_artifacts_v1(&input, &pack)
        .expect("checked primary provider artifacts generate");
    let generator = checked_provider_generator_source_bundle_v1()
        .expect("checked generator source bundle builds")
        .generator_material()
        .expect("checked generator material builds");
    let provenance = generate_provider_generation_provenance_v1(&input, &primary, &generator)
        .expect("checked provider provenance generates");
    let review = generate_provider_generation_review_v1(&input, &provenance)
        .expect("checked provider review generates");
    (input, primary, generator, provenance, review)
}

#[test]
fn source_bundle_has_one_exact_order_independent_binary_frame() {
    let first = build_provider_generator_source_bundle_v1(vec![
        ProviderGeneratorSourceFileV1::new("b.rs", b"B").expect("test source path is valid"),
        ProviderGeneratorSourceFileV1::new("a.rs", b"A").expect("test source path is valid"),
    ])
    .expect("test source bundle builds");
    let reordered = build_provider_generator_source_bundle_v1(vec![
        ProviderGeneratorSourceFileV1::new("a.rs", b"A").expect("test source path is valid"),
        ProviderGeneratorSourceFileV1::new("b.rs", b"B").expect("test source path is valid"),
    ])
    .expect("reordered test source bundle builds");

    let mut expected = b"echo.provider-artifact-generator.source-bundle/v1\0".to_vec();
    expected.extend_from_slice(&2_u64.to_be_bytes());
    for (path, bytes) in [("a.rs", b"A".as_slice()), ("b.rs", b"B".as_slice())] {
        expected.extend_from_slice(
            &u64::try_from(path.len())
                .expect("test path length fits u64")
                .to_be_bytes(),
        );
        expected.extend_from_slice(path.as_bytes());
        expected.extend_from_slice(
            &u64::try_from(bytes.len())
                .expect("test content length fits u64")
                .to_be_bytes(),
        );
        expected.extend_from_slice(bytes);
    }

    assert_eq!(first, reordered);
    assert_eq!(first.canonical_bytes(), expected);
    assert_eq!(
        first
            .source_files()
            .iter()
            .map(ProviderGeneratorSourceFileV1::path)
            .collect::<Vec<_>>(),
        ["a.rs", "b.rs"]
    );
}

#[test]
fn checked_source_bundle_pins_the_exact_non_circular_generator_closure() {
    let first = checked_provider_generator_source_bundle_v1()
        .expect("checked generator source bundle builds");
    let second = checked_provider_generator_source_bundle_v1()
        .expect("repeated checked generator source bundle builds");
    let first_material = first
        .generator_material()
        .expect("checked generator material builds");
    let second_material = second
        .generator_material()
        .expect("repeated checked generator material builds");

    assert_eq!(first, second);
    assert_eq!(first_material, second_material);
    assert_eq!(
        first
            .source_files()
            .iter()
            .map(ProviderGeneratorSourceFileV1::path)
            .collect::<Vec<_>>(),
        GENERATOR_SOURCE_PATHS
    );
    assert_eq!(
        first_material.identity().coordinate,
        "echo-wesley-gen.provider-artifact-generator@1"
    );
    assert_eq!(first_material.identity().version, "0.1.0");
    assert!(first.source_files().iter().all(|source| {
        !source
            .path()
            .starts_with("schemas/edict-provider/generated/v1/")
    }));
}

#[test]
fn source_path_or_content_changes_move_identity_and_invalid_closures_fail_closed() {
    let baseline =
        build_provider_generator_source_bundle_v1(vec![ProviderGeneratorSourceFileV1::new(
            "a.rs", b"A",
        )
        .expect("test source path is valid")])
        .expect("baseline source bundle builds");
    let changed_path =
        build_provider_generator_source_bundle_v1(vec![ProviderGeneratorSourceFileV1::new(
            "b.rs", b"A",
        )
        .expect("changed test source path is valid")])
        .expect("changed-path source bundle builds");
    let changed_content =
        build_provider_generator_source_bundle_v1(vec![ProviderGeneratorSourceFileV1::new(
            "a.rs", b"B",
        )
        .expect("test source path is valid")])
        .expect("changed-content source bundle builds");

    assert_ne!(
        baseline
            .generator_material()
            .expect("baseline generator material builds")
            .identity()
            .digest,
        changed_path
            .generator_material()
            .expect("changed-path generator material builds")
            .identity()
            .digest
    );
    assert_ne!(
        baseline
            .generator_material()
            .expect("baseline generator material builds")
            .identity()
            .digest,
        changed_content
            .generator_material()
            .expect("changed-content generator material builds")
            .identity()
            .digest
    );

    let duplicate = build_provider_generator_source_bundle_v1(vec![
        ProviderGeneratorSourceFileV1::new("a.rs", b"A").expect("test source path is valid"),
        ProviderGeneratorSourceFileV1::new("a.rs", b"B")
            .expect("duplicate test source path is structurally valid"),
    ])
    .expect_err("duplicate source paths fail closed");
    assert_eq!(
        duplicate.kind(),
        ProviderArtifactCorpusErrorKind::GeneratorSourceDuplicate
    );
    assert_eq!(duplicate.subject(), "a.rs");

    let generated_path = ProviderGeneratorSourceFileV1::new(
        "schemas/edict-provider/generated/v1/evidence/provenance.provider-generation.json",
        b"self reference",
    )
    .expect_err("generated output paths cannot enter generator identity");
    assert_eq!(
        generated_path.kind(),
        ProviderArtifactCorpusErrorKind::GeneratorSourcePathInvalid
    );
}

#[test]
fn rendered_corpus_is_byte_identical_complete_and_reports_sorted_drift() {
    let (input, primary, generator, provenance, review) = generate();
    let first =
        render_provider_artifact_corpus_v1(&input, &primary, &generator, &provenance, &review)
            .expect("checked provider corpus renders");
    let second =
        render_provider_artifact_corpus_v1(&input, &primary, &generator, &provenance, &review)
            .expect("repeated checked provider corpus renders");

    assert_eq!(first, second);
    assert_eq!(
        first
            .files()
            .iter()
            .map(ProviderArtifactCorpusFileV1::relative_path)
            .collect::<Vec<_>>(),
        CORPUS_PATHS
    );
    assert!(first.files().iter().all(|file| !file.bytes().is_empty()));
    assert!(diff_provider_artifact_corpus_v1(&first, first.files())
        .expect("identical corpus inventory compares")
        .is_empty());

    let mut actual = first.files().to_vec();
    actual.retain(|file| file.relative_path() != "primary/lawpack.echo-dpo.cbor");
    let changed = actual
        .iter()
        .position(|file| file.relative_path() == "evidence/review.provider-generation.json")
        .expect("review file exists");
    actual[changed] =
        ProviderArtifactCorpusFileV1::new("evidence/review.provider-generation.json", b"tampered")
            .expect("changed corpus path is valid");
    actual.push(
        ProviderArtifactCorpusFileV1::new("unexpected.bin", b"unexpected")
            .expect("unexpected corpus path is valid"),
    );

    let drift = diff_provider_artifact_corpus_v1(&first, &actual)
        .expect("drifted corpus inventory compares");
    assert_eq!(
        drift
            .iter()
            .map(|entry| (entry.kind(), entry.relative_path()))
            .collect::<Vec<_>>(),
        [
            (
                ProviderArtifactCorpusDriftKind::Missing,
                "primary/lawpack.echo-dpo.cbor",
            ),
            (
                ProviderArtifactCorpusDriftKind::Changed,
                "evidence/review.provider-generation.json",
            ),
            (
                ProviderArtifactCorpusDriftKind::Unexpected,
                "unexpected.bin",
            ),
        ]
    );
}

#[test]
fn corpus_rejects_provenance_that_does_not_bind_the_supplied_generator() {
    let (input, primary, _generator, provenance, review) = generate();
    let other_generator = ProviderGeneratorMaterialV1::new(
        "echo-wesley-gen.provider-artifact-generator@1",
        "0.1.0",
        b"other generator material",
    )
    .expect("alternate generator material is structurally valid");

    let error = render_provider_artifact_corpus_v1(
        &input,
        &primary,
        &other_generator,
        &provenance,
        &review,
    )
    .expect_err("mismatched generator material fails closed");
    assert_eq!(
        error.kind(),
        ProviderArtifactCorpusErrorKind::ProvenanceInvalid
    );
    assert!(error.provenance_kind().is_some());
}

#[test]
fn oversized_corpus_paths_preserve_the_corpus_error_category() {
    let path = format!("{}.cbor", "a".repeat(513));
    let error = ProviderArtifactCorpusFileV1::new(&path, b"content")
        .expect_err("oversized corpus paths fail closed");

    assert_eq!(
        error.kind(),
        ProviderArtifactCorpusErrorKind::CorpusPathInvalid
    );
    assert_eq!(error.subject(), path);
    assert_eq!(error.reference(), path.len().to_string());
}

#[test]
fn platform_specific_or_traversing_corpus_paths_fail_closed_as_non_posix() {
    for path in [r"..\escape", r"C:\outside", "C:/outside"] {
        let error = ProviderArtifactCorpusFileV1::new(path, b"content")
            .expect_err("non-POSIX or prefixed corpus paths fail closed");
        assert_eq!(
            error.kind(),
            ProviderArtifactCorpusErrorKind::CorpusPathInvalid
        );
        assert_eq!(error.subject(), path);
        assert_eq!(error.reference(), "relative-posix-path");
    }
}

#[test]
fn check_mode_reports_drift_without_rewriting_or_creating_files() {
    let directory = TestDirectory::new("no-write-check");
    let changed_path = directory
        .path()
        .join("evidence/review.provider-generation.json");
    let unexpected_path = directory.path().join("unexpected file.txt");
    std::fs::create_dir_all(
        changed_path
            .parent()
            .expect("changed test path has a parent"),
    )
    .expect("evidence test directory is created");
    std::fs::write(&changed_path, b"tampered").expect("changed test file is written");
    std::fs::write(&unexpected_path, b"unexpected").expect("unexpected test file is written");

    let output = Command::new(env!("CARGO_BIN_EXE_echo-edict-provider-artifacts"))
        .arg("--check")
        .arg("--out")
        .arg(directory.path())
        .output()
        .expect("provider corpus checker executes");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("checker diagnostics are UTF-8");
    let missing = stderr
        .find("  missing: ")
        .expect("checker reports missing files");
    let changed = stderr
        .find("  changed: evidence/review.provider-generation.json")
        .expect("checker reports the changed review");
    let unexpected = stderr
        .find("  unexpected: unexpected file.txt")
        .expect("checker reports the unexpected file");
    assert!(missing < changed);
    assert!(changed < unexpected);
    assert_eq!(
        std::fs::read(&changed_path).expect("changed test file remains readable"),
        b"tampered"
    );
    assert_eq!(
        std::fs::read(&unexpected_path).expect("unexpected test file remains readable"),
        b"unexpected"
    );
    assert!(!directory.path().join("primary").exists());
    assert!(!directory.path().join("resources").exists());
}

#[test]
fn generation_refuses_unexpected_entries_before_writing_expected_files() {
    let directory = TestDirectory::new("generation-unexpected-entry");
    let unexpected_path = directory.path().join("operator-owned.txt");
    std::fs::write(&unexpected_path, b"operator bytes").expect("unexpected test file is written");

    let output = Command::new(env!("CARGO_BIN_EXE_echo-edict-provider-artifacts"))
        .arg("--out")
        .arg(directory.path())
        .output()
        .expect("provider corpus generator executes");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("refusing to generate over unexpected corpus entry operator-owned.txt"));
    assert_eq!(
        std::fs::read(&unexpected_path).expect("unexpected test file remains readable"),
        b"operator bytes"
    );
    assert!(!directory.path().join("evidence").exists());
    assert!(!directory.path().join("primary").exists());
    assert!(!directory.path().join("resources").exists());
}

#[cfg(unix)]
#[test]
fn generation_refuses_symlinked_root_parent_and_leaf_without_writing_through_them() {
    use std::os::unix::fs::symlink;

    let root_parent = TestDirectory::new("symlink-root-parent");
    let root_destination = TestDirectory::new("symlink-root-destination");
    let root_link = root_parent.path().join("corpus-link");
    symlink(root_destination.path(), &root_link).expect("root test symlink is created");
    let root_output = Command::new(env!("CARGO_BIN_EXE_echo-edict-provider-artifacts"))
        .arg("--out")
        .arg(&root_link)
        .output()
        .expect("provider corpus generator executes against a symlink root");
    assert!(!root_output.status.success());
    assert!(
        std::fs::read_dir(root_destination.path())
            .expect("root destination remains readable")
            .next()
            .is_none(),
        "generator must not write through a symlinked root"
    );

    let parent_root = TestDirectory::new("symlink-parent-root");
    let parent_destination = TestDirectory::new("symlink-parent-destination");
    symlink(
        parent_destination.path(),
        parent_root.path().join("evidence"),
    )
    .expect("parent test symlink is created");
    let parent_output = Command::new(env!("CARGO_BIN_EXE_echo-edict-provider-artifacts"))
        .arg("--out")
        .arg(parent_root.path())
        .output()
        .expect("provider corpus generator executes against a symlink parent");
    assert!(!parent_output.status.success());
    assert!(
        std::fs::read_dir(parent_destination.path())
            .expect("parent destination remains readable")
            .next()
            .is_none(),
        "generator must not write through a symlinked parent"
    );
    assert!(!parent_root.path().join("primary").exists());

    let leaf_root = TestDirectory::new("symlink-leaf-root");
    let leaf_destination = TestDirectory::new("symlink-leaf-destination");
    let outside_file = leaf_destination.path().join("owner.txt");
    std::fs::write(&outside_file, b"owner bytes").expect("outside owner file is written");
    std::fs::create_dir_all(leaf_root.path().join("evidence"))
        .expect("leaf test parent is created");
    symlink(
        &outside_file,
        leaf_root
            .path()
            .join("evidence/provenance.provider-generation.json"),
    )
    .expect("leaf test symlink is created");
    let leaf_output = Command::new(env!("CARGO_BIN_EXE_echo-edict-provider-artifacts"))
        .arg("--out")
        .arg(leaf_root.path())
        .output()
        .expect("provider corpus generator executes against a symlink leaf");
    assert!(!leaf_output.status.success());
    assert_eq!(
        std::fs::read(&outside_file).expect("outside owner file remains readable"),
        b"owner bytes"
    );
    assert!(!leaf_root
        .path()
        .join("evidence/review.provider-generation.json")
        .exists());
}

#[cfg(unix)]
#[test]
fn generation_replaces_a_hard_link_without_mutating_its_external_inode() {
    let corpus_root = TestDirectory::new("hard-link-root");
    let external_root = TestDirectory::new("hard-link-external");
    let external_file = external_root.path().join("owner.txt");
    std::fs::write(&external_file, b"owner bytes").expect("outside owner file is written");
    let corpus_file = corpus_root
        .path()
        .join("evidence/provenance.provider-generation.json");
    std::fs::create_dir_all(
        corpus_file
            .parent()
            .expect("hard-link corpus path has a parent"),
    )
    .expect("hard-link corpus parent is created");
    std::fs::hard_link(&external_file, &corpus_file).expect("hard-link corpus leaf is created");

    let output = Command::new(env!("CARGO_BIN_EXE_echo-edict-provider-artifacts"))
        .arg("--out")
        .arg(corpus_root.path())
        .output()
        .expect("provider corpus generator executes against a hard-linked leaf");

    assert!(
        output.status.success(),
        "safe corpus replacement failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(&external_file).expect("outside owner file remains readable"),
        b"owner bytes"
    );
    assert_ne!(
        std::fs::read(&corpus_file).expect("generated corpus file is readable"),
        b"owner bytes"
    );
}

#[cfg(unix)]
#[test]
fn check_mode_classifies_a_non_regular_expected_leaf_without_following_it() {
    use std::os::unix::fs::symlink;

    let corpus_root = TestDirectory::new("check-symlink-root");
    let external_root = TestDirectory::new("check-symlink-external");
    let external_file = external_root.path().join("owner.txt");
    std::fs::write(&external_file, b"owner bytes").expect("outside owner file is written");
    let corpus_file = corpus_root
        .path()
        .join("evidence/review.provider-generation.json");
    std::fs::create_dir_all(
        corpus_file
            .parent()
            .expect("check symlink corpus path has a parent"),
    )
    .expect("check symlink corpus parent is created");
    symlink(&external_file, &corpus_file).expect("check symlink corpus leaf is created");

    let output = Command::new(env!("CARGO_BIN_EXE_echo-edict-provider-artifacts"))
        .arg("--check")
        .arg("--out")
        .arg(corpus_root.path())
        .output()
        .expect("provider corpus checker executes against a symlink leaf");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("  changed: evidence/review.provider-generation.json"));
    assert_eq!(
        std::fs::read(&external_file).expect("outside owner file remains readable"),
        b"owner bytes"
    );
}

#[test]
fn checked_first_corpus_matches_the_renderer_without_missing_or_extra_files() {
    let corpus_root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/edict-provider/generated/v1");
    let output = Command::new(env!("CARGO_BIN_EXE_echo-edict-provider-artifacts"))
        .arg("--check")
        .arg("--out")
        .arg(&corpus_root)
        .output()
        .expect("provider corpus checker executes against the checked snapshot");

    assert!(
        output.status.success(),
        "checked provider corpus drifted:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("is current"));
}
