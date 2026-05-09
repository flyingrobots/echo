// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Tests for METHOD playback drift coverage.
#![allow(clippy::expect_used)]

use std::fs;

use method::drift::drift_report;
use method::workspace::MethodWorkspace;

fn scaffold(root: &std::path::Path) {
    fs::create_dir_all(root.join("docs/method/backlog/inbox")).expect("create inbox");
    fs::create_dir_all(root.join("docs/design/0001-playback-check")).expect("create design");
    fs::create_dir_all(root.join("docs/method/retro")).expect("create retro");
    fs::create_dir_all(root.join("crates/demo/tests")).expect("create test dir");
}

#[test]
fn drift_marks_playback_question_covered_by_test_description() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    fs::write(
        tmp.path().join("docs/design/0001-playback-check/design.md"),
        "# Playback Check\n\n## Human playback\n\n1. Does the status command emit JSON?\n",
    )
    .expect("write design");
    fs::write(
        tmp.path().join("crates/demo/tests/status_tests.rs"),
        "// Does the status command emit JSON?\n#[test]\nfn status_json() {}\n",
    )
    .expect("write test");
    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");

    let report = drift_report(&workspace, Some("0001")).expect("drift report");

    assert!(report.covered());
    assert_eq!(report.missing_count(), 0);
    assert_eq!(report.questions.len(), 1);
    assert_eq!(
        report.questions[0].matches,
        vec![std::path::PathBuf::from(
            "crates/demo/tests/status_tests.rs"
        )]
    );
}

#[test]
fn drift_reports_missing_playback_question() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    fs::write(
        tmp.path()
            .join("docs/design/0001-playback-check/design.md"),
        "# Playback Check\n\n## Agent playback\n\n| Question | Expected |\n| --- | --- |\n| Can the agent parse the frontier? | Yes |\n",
    )
    .expect("write design");
    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");

    let report = drift_report(&workspace, None).expect("drift report");

    assert!(!report.covered());
    assert_eq!(report.missing_count(), 1);
    assert!(report.questions[0].matches.is_empty());
}
