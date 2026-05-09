// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Tests for METHOD backlog pull.
#![allow(clippy::expect_used)]

use std::fs;

use method::pull::pull_backlog_item;
use method::workspace::MethodWorkspace;

fn scaffold(root: &std::path::Path) {
    for lane in &["inbox", "asap", "up-next", "cool-ideas", "bad-code"] {
        fs::create_dir_all(root.join(format!("docs/method/backlog/{lane}"))).expect("create lane");
    }
    fs::create_dir_all(root.join("docs/design/0001-existing-cycle")).expect("create design");
    fs::create_dir_all(root.join("docs/method/retro")).expect("create retro");
}

#[test]
fn pull_by_task_id_moves_backlog_file_to_next_design_cycle() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    let source = tmp
        .path()
        .join("docs/method/backlog/asap/PLATFORM_build-spaceship.md");
    fs::write(&source, "# Build Spaceship\n").expect("write backlog");
    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");

    let result = pull_backlog_item(&workspace, "M001").expect("pull item");

    assert_eq!(result.cycle_number, "0002");
    assert_eq!(result.cycle, "0002-build-spaceship");
    assert_eq!(
        result
            .design_path
            .strip_prefix(tmp.path())
            .expect("relative design path"),
        std::path::Path::new("docs/design/0002-build-spaceship/build-spaceship.md")
    );
    assert!(!source.exists());
    assert_eq!(
        fs::read_to_string(result.design_path).expect("read moved design"),
        "# Build Spaceship\n"
    );
}

#[test]
fn pull_by_unprefixed_stem_strips_legend_prefix() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    fs::write(
        tmp.path()
            .join("docs/method/backlog/asap/KERNEL_determinism-torture.md"),
        "# Determinism Torture\n",
    )
    .expect("write backlog");
    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");

    let result = pull_backlog_item(&workspace, "determinism-torture").expect("pull item");

    assert_eq!(result.cycle, "0002-determinism-torture");
    assert!(result
        .design_path
        .ends_with("docs/design/0002-determinism-torture/determinism-torture.md"));
}

#[test]
fn pull_by_native_section_id_moves_containing_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    let source = tmp
        .path()
        .join("docs/method/backlog/asap/PLATFORM_two-tasks.md");
    fs::write(
        &source,
        "# Two Tasks\n\n## T-1-1-1: First\n\n## T-1-1-2: Second\n",
    )
    .expect("write backlog");
    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");

    let result = pull_backlog_item(&workspace, "T-1-1-2").expect("pull containing file");

    assert_eq!(result.cycle, "0002-two-tasks");
    assert!(!source.exists());
    assert!(result.design_path.is_file());
}
