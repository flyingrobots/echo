// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Tests for METHOD inbox capture.
#![allow(clippy::expect_used)]

use std::fs;

use method::inbox::{create_inbox_item, filename_from_title};
use method::workspace::MethodWorkspace;

fn scaffold(root: &std::path::Path) {
    fs::create_dir_all(root.join("docs/method/backlog/inbox")).expect("create inbox");
}

#[test]
fn filename_from_title_slugifies_plain_text() {
    assert_eq!(
        filename_from_title("Fix local iteration times!").expect("filename"),
        "fix-local-iteration-times.md"
    );
}

#[test]
fn filename_from_title_collapses_punctuation_and_whitespace() {
    assert_eq!(
        filename_from_title("  WARP/TTD: Reading Envelope?  ").expect("filename"),
        "warp-ttd-reading-envelope.md"
    );
}

#[test]
fn filename_from_title_rejects_empty_slug() {
    let err = filename_from_title("!!!").expect_err("empty slug should fail");
    assert!(
        err.contains("ASCII letter or digit"),
        "unexpected error: {err}"
    );
}

#[test]
fn create_inbox_item_writes_method_markdown() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");

    let path = create_inbox_item(&workspace, "Build the spaceship").expect("create item");

    assert_eq!(
        path.strip_prefix(tmp.path()).expect("relative path"),
        std::path::Path::new("docs/method/backlog/inbox/build-the-spaceship.md")
    );
    let content = fs::read_to_string(path).expect("read item");
    assert!(content.starts_with("<!-- SPDX-License-Identifier:"));
    assert!(content.contains("# Build the spaceship"));
    assert!(content.contains("Captured with `cargo xtask method inbox`."));
}

#[test]
fn create_inbox_item_refuses_to_overwrite() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");

    let path = tmp
        .path()
        .join("docs/method/backlog/inbox/build-the-spaceship.md");
    fs::write(&path, "existing\n").expect("seed file");

    let err =
        create_inbox_item(&workspace, "Build the spaceship").expect_err("overwrite should fail");

    assert!(
        err.contains("refusing to overwrite"),
        "unexpected error: {err}"
    );
    assert_eq!(
        fs::read_to_string(path).expect("read seeded file"),
        "existing\n"
    );
}
