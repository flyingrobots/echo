// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Tests for `method::status` and `method::workspace`.
#![allow(clippy::expect_used)]

use std::fs;

use method::status::StatusReport;
use method::workspace::MethodWorkspace;

/// Helper: scaffold a valid METHOD workspace in a temp dir.
fn scaffold(root: &std::path::Path) {
    for lane in &["inbox", "asap", "up-next", "cool-ideas", "bad-code"] {
        fs::create_dir_all(root.join(format!("docs/method/backlog/{lane}"))).ok();
    }
    fs::create_dir_all(root.join("docs/design")).ok();
    fs::create_dir_all(root.join("docs/method/retro")).ok();
    fs::create_dir_all(root.join("docs/method/legends")).ok();
}

/// Helper: write a dummy .md file.
fn touch_md(path: &std::path::Path) {
    fs::write(path, "# placeholder\n").ok();
}

// ── Workspace discovery ─────────────────────────────────────────────

#[test]
fn discover_fails_for_missing_backlog() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = MethodWorkspace::discover(tmp.path());
    assert!(result.is_err(), "should fail when backlog dir is missing");
}

#[test]
fn discover_succeeds_for_valid_workspace() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());
    let result = MethodWorkspace::discover(tmp.path());
    assert!(result.is_ok(), "should succeed for valid METHOD workspace");
}

// ── Lane counts ─────────────────────────────────────────────────────

#[test]
fn status_counts_lane_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());

    // 3 items in asap, 1 in inbox, 0 in the rest
    let asap = tmp.path().join("docs/method/backlog/asap");
    touch_md(&asap.join("KERNEL_foo.md"));
    touch_md(&asap.join("PLATFORM_bar.md"));
    touch_md(&asap.join("DOCS_baz.md"));
    touch_md(&tmp.path().join("docs/method/backlog/inbox/idea.md"));

    let ws = MethodWorkspace::discover(tmp.path()).expect("discover");
    let report = StatusReport::build(&ws).expect("status");

    assert_eq!(report.lanes.get("asap"), Some(&3));
    assert_eq!(report.lanes.get("inbox"), Some(&1));
    assert_eq!(report.lanes.get("up-next"), Some(&0));
    assert_eq!(report.lanes.get("cool-ideas"), Some(&0));
    assert_eq!(report.lanes.get("bad-code"), Some(&0));
    assert_eq!(report.total_items, 4);
}

// ── Active cycle detection ──────────────────────────────────────────

#[test]
fn status_detects_active_cycles() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());

    // Cycle 0001 has a retro (closed). Cycle 0002 does not (active).
    fs::create_dir_all(tmp.path().join("docs/design/0001-bootstrap")).ok();
    fs::create_dir_all(tmp.path().join("docs/method/retro/0001-bootstrap")).ok();
    fs::create_dir_all(tmp.path().join("docs/design/0002-something")).ok();

    let ws = MethodWorkspace::discover(tmp.path()).expect("discover");
    let report = StatusReport::build(&ws).expect("status");

    assert_eq!(report.active_cycles.len(), 1);
    assert_eq!(report.active_cycles[0].number, "0002");
    assert_eq!(report.active_cycles[0].slug, "something");
}

// ── Legend prefix parsing ───────────────────────────────────────────

#[test]
fn status_counts_legend_load() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());

    let asap = tmp.path().join("docs/method/backlog/asap");
    touch_md(&asap.join("KERNEL_a.md"));
    touch_md(&asap.join("KERNEL_b.md"));
    touch_md(&asap.join("PLATFORM_c.md"));

    let cool = tmp.path().join("docs/method/backlog/cool-ideas");
    touch_md(&cool.join("KERNEL_d.md"));
    touch_md(&cool.join("MATH_e.md"));

    let ws = MethodWorkspace::discover(tmp.path()).expect("discover");
    let report = StatusReport::build(&ws).expect("status");

    assert_eq!(report.legend_load.get("KERNEL"), Some(&3));
    assert_eq!(report.legend_load.get("PLATFORM"), Some(&1));
    assert_eq!(report.legend_load.get("MATH"), Some(&1));
}

#[test]
fn status_counts_unprefixed_items() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());

    let inbox = tmp.path().join("docs/method/backlog/inbox");
    touch_md(&inbox.join("some-idea.md"));
    touch_md(&inbox.join("another-idea.md"));

    let ws = MethodWorkspace::discover(tmp.path()).expect("discover");
    let report = StatusReport::build(&ws).expect("status");

    // Items without a LEGEND_ prefix should appear under "(none)" or similar.
    let unprefixed = report
        .legend_load
        .iter()
        .find(|(k, _)| !k.chars().all(|c| c.is_uppercase() || c == '_'));
    assert!(
        unprefixed.is_some(),
        "unprefixed items should have a legend_load entry"
    );
    let (_, count) = unprefixed.expect("just asserted");
    assert_eq!(*count, 2);
}

// ── JSON serialization (agent surface) ──────────────────────────────

#[test]
fn status_report_serializes_to_json_with_expected_keys() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());

    let asap = tmp.path().join("docs/method/backlog/asap");
    touch_md(&asap.join("KERNEL_foo.md"));

    let ws = MethodWorkspace::discover(tmp.path()).expect("discover");
    let report = StatusReport::build(&ws).expect("status");
    let json = serde_json::to_value(&report).expect("serialize");

    assert!(json.get("lanes").is_some(), "JSON must have 'lanes'");
    assert!(
        json.get("active_cycles").is_some(),
        "JSON must have 'active_cycles'"
    );
    assert!(
        json.get("legend_load").is_some(),
        "JSON must have 'legend_load'"
    );
    assert!(
        json.get("total_items").is_some(),
        "JSON must have 'total_items'"
    );
}
