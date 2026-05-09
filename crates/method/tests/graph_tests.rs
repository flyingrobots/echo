// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Tests for METHOD task graph parsing and scheduling queries.
#![allow(clippy::expect_used)]

use std::fs;

use method::graph::TaskGraph;
use method::workspace::MethodWorkspace;

fn scaffold(root: &std::path::Path) {
    for lane in &["inbox", "asap", "up-next", "cool-ideas", "bad-code"] {
        fs::create_dir_all(root.join(format!("docs/method/backlog/{lane}"))).expect("create lane");
    }
    fs::create_dir_all(root.join("docs/design")).expect("create design");
    fs::create_dir_all(root.join("docs/method/retro")).expect("create retro");
}

#[test]
fn graph_builds_file_level_depends_on_edges() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());

    fs::write(
        tmp.path().join("docs/method/backlog/asap/A_first.md"),
        "# First\n",
    )
    .expect("write first");
    fs::write(
        tmp.path().join("docs/method/backlog/asap/A_second.md"),
        "# Second\n\nDepends on:\n\n- [First](./A_first.md)\n\n## Goal\n\nDo it.\n",
    )
    .expect("write second");

    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");
    let graph = TaskGraph::build(&workspace).expect("graph");

    assert_eq!(graph.tasks.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].prerequisite, "M001");
    assert_eq!(graph.edges[0].dependent, "M002");

    let frontier = graph.frontier();
    assert_eq!(frontier.len(), 1);
    assert_eq!(frontier[0].task.title, "First");
}

#[test]
fn graph_splits_legacy_t_sections_and_blocks_by_native_id() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());

    fs::write(
        tmp.path()
            .join("docs/method/backlog/asap/PLATFORM_legacy.md"),
        r"# Legacy

## T-1-1-1: First task

**Blocked By:** none
**Blocking:** T-1-1-2

## T-1-1-2: Second task

**Blocked By:** T-1-1-1
**Blocking:** none
",
    )
    .expect("write legacy");

    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");
    let graph = TaskGraph::build(&workspace).expect("graph");

    assert_eq!(graph.tasks.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].prerequisite, "M001");
    assert_eq!(graph.edges[0].dependent, "M002");
    assert_eq!(
        graph.frontier()[0].task.native_id.as_deref(),
        Some("T-1-1-1")
    );
}

#[test]
fn matrix_csv_has_square_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());

    fs::write(
        tmp.path().join("docs/method/backlog/asap/A_first.md"),
        "# First\n",
    )
    .expect("write first");
    fs::write(
        tmp.path().join("docs/method/backlog/asap/A_second.md"),
        "# Second\n\nDepends on:\n\n- [First](./A_first.md)\n",
    )
    .expect("write second");

    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");
    let graph = TaskGraph::build(&workspace).expect("graph");
    let csv = graph.render_matrix_csv();
    let rows = csv
        .trim_end()
        .lines()
        .map(|line| line.split(',').count())
        .collect::<Vec<_>>();

    assert_eq!(rows, vec![3, 3, 3]);
    assert!(csv.contains("depends on"));
}

#[test]
fn completed_backlog_cards_satisfy_blockers_without_becoming_open() {
    let tmp = tempfile::tempdir().expect("tempdir");
    scaffold(tmp.path());

    fs::write(
        tmp.path().join("docs/method/backlog/asap/A_design.md"),
        "# Design\n\nStatus: design packet complete.\n",
    )
    .expect("write design");
    fs::write(
        tmp.path().join("docs/method/backlog/asap/A_impl.md"),
        "# Impl\n\nDepends on:\n\n- [Design](./A_design.md)\n",
    )
    .expect("write impl");

    let workspace = MethodWorkspace::discover(tmp.path()).expect("discover");
    let graph = TaskGraph::build(&workspace).expect("graph");
    let frontier = graph.frontier();

    assert_eq!(frontier.len(), 1);
    assert_eq!(frontier[0].task.title, "Impl");
    assert!(graph.render_dot().contains("DONE"));
    assert!(graph.render_dot().contains("OPEN"));
}
