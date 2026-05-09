// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Tests for METHOD cycle closeout scaffolding.
#![allow(clippy::expect_used)]

use std::fs;

use method::close::close_cycle;
use method::workspace::MethodWorkspace;

fn scaffold(root: &std::path::Path) {
    fs::create_dir_all(root.join("docs/method/backlog/inbox")).expect("create inbox");
    fs::create_dir_all(root.join("docs/design/0001-first-cycle")).expect("create first design");
    fs::create_dir_all(root.join("docs/design/0002-second-cycle")).expect("create second design");
    fs::create_dir_all(root.join("docs/method/retro/0001-first-cycle"))
        .expect("create closed first retro");
}

#[test]
fn close_defaults_to_most_recent_active_cycle() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");

    let result = close_cycle(&workspace, None).expect("close cycle");

    assert_eq!(result.cycle, "0002-second-cycle");
    assert!(result.retro_path.ends_with("retro.md"));
    assert!(result.witness_dir.ends_with("witness"));
    assert!(result.witness_dir.is_dir());

    let retro = fs::read_to_string(result.retro_path).expect("read retro");
    assert!(retro.contains("# Retro: 0002-second-cycle"));
    assert!(retro.contains("Witness: [`witness/`](./witness/)"));
}

#[test]
fn close_accepts_numeric_cycle_selector() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    fs::create_dir_all(tmp.path().join("docs/design/0003-third-cycle"))
        .expect("create third design");
    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");

    let result = close_cycle(&workspace, Some("0002")).expect("close selected cycle");

    assert_eq!(result.cycle, "0002-second-cycle");
}

#[test]
fn close_refuses_to_overwrite_existing_retro() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");

    let _ = close_cycle(&workspace, Some("0002")).expect("close cycle once");
    let err = close_cycle(&workspace, Some("0002")).expect_err("overwrite should fail");

    assert!(
        err.contains("refusing to overwrite existing retro directory"),
        "unexpected error: {err}"
    );
}
