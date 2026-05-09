// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration tests for `echo-cli` binary.
//!
//! These tests run the actual binary via `assert_cmd` and verify exit codes,
//! help output, and error messages.

#![allow(deprecated)] // assert_cmd::cargo::cargo_bin deprecation — no stable replacement in v2.x

use std::error::Error;
use std::fs;

use assert_cmd::cargo::cargo_bin;
use predicates::prelude::*;
use tempfile::TempDir;
use warp_core::wsc::{build_one_warp_input, write_wsc_one_warp};
use warp_core::{
    make_edge_id, make_node_id, make_type_id, make_warp_id, EdgeRecord, GraphStore, NodeRecord,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn echo_cli() -> assert_cmd::Command {
    assert_cmd::Command::new(cargo_bin("echo-cli"))
}

fn make_demo_wsc() -> TestResult<Vec<u8>> {
    let warp = make_warp_id("test");
    let node_ty = make_type_id("Actor");
    let child_ty = make_type_id("Item");
    let edge_ty = make_type_id("HasItem");
    let root = make_node_id("root");
    let child1 = make_node_id("child1");
    let child2 = make_node_id("child2");

    let mut store = GraphStore::new(warp);
    store.insert_node(root, NodeRecord { ty: node_ty });
    store.insert_node(child1, NodeRecord { ty: child_ty });
    store.insert_node(child2, NodeRecord { ty: child_ty });
    store.insert_edge(
        root,
        EdgeRecord {
            id: make_edge_id("root->child1"),
            from: root,
            to: child1,
            ty: edge_ty,
        },
    );
    store.insert_edge(
        root,
        EdgeRecord {
            id: make_edge_id("root->child2"),
            from: root,
            to: child2,
            ty: edge_ty,
        },
    );

    let input = build_one_warp_input(&store, root);
    Ok(write_wsc_one_warp(&input, [0u8; 32], 42)?)
}

fn write_demo_snapshot() -> TestResult<TempDir> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("state.wsc"), make_demo_wsc()?)?;
    Ok(temp)
}

#[test]
fn help_shows_all_subcommands() {
    echo_cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Echo developer CLI"))
        .stdout(predicate::str::contains("verify"))
        .stdout(predicate::str::contains("bench"))
        .stdout(predicate::str::contains("inspect"));
}

#[test]
fn help_output_has_no_trailing_whitespace() {
    let assert = echo_cli().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let offenders = stdout
        .lines()
        .enumerate()
        .filter(|(_, line)| line.ends_with(' '))
        .map(|(index, _)| format!("line {}", index + 1))
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "help output contains trailing whitespace on {}",
        offenders.join(", ")
    );
}

#[test]
fn help_matches_golden() {
    let assert = echo_cli().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout, include_str!("golden/echo-cli-help.txt"));
}

#[test]
fn help_golden_has_no_trailing_whitespace() {
    let golden = include_str!("golden/echo-cli-help.txt");
    let offenders = golden
        .lines()
        .enumerate()
        .filter(|(_, line)| line.ends_with(' '))
        .map(|(index, _)| format!("line {}", index + 1))
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "help golden contains trailing whitespace on {}",
        offenders.join(", ")
    );
}

#[test]
fn verify_help_lists_snapshot_arg() {
    echo_cli()
        .args(["verify", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("snapshot"));
}

#[test]
fn bench_help_lists_filter() {
    echo_cli()
        .args(["bench", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("filter"))
        .stdout(predicate::str::contains("baseline"));
}

#[test]
fn inspect_help_lists_tree_flag() {
    echo_cli()
        .args(["inspect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tree"))
        .stdout(predicate::str::contains("raw"));
}

#[test]
fn inspect_text_reports_metadata_stats_and_tree() -> TestResult {
    let temp = write_demo_snapshot()?;
    let assert = echo_cli()
        .current_dir(temp.path())
        .args(["inspect", "state.wsc", "--tree"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    assert!(stdout.contains("echo-cli inspect"));
    assert!(stdout.contains("File: state.wsc"));
    assert!(stdout.contains("Tick: 42"));
    assert!(
        stdout.contains("Schema: 0000000000000000000000000000000000000000000000000000000000000000")
    );
    assert!(stdout.contains("Warps: 1"));
    assert!(stdout
        .contains("ID:         6939dc0fbdb5004cb5d9d1aca2d096042456f4257b88ee8c7fdbfca163f10f11"));
    assert!(stdout
        .contains("Root node:  401e1d8fcbc26350901be9100a153e8eaf644560386edf68f876ffc1335cccf0"));
    assert!(stdout
        .contains("State root: 5934ceb0b331755a406e85fbe1a6dda3d6ce5278f7c1802713a50c4e754c84a6"));
    assert!(stdout.contains("Nodes:      3"));
    assert!(stdout.contains("Edges:      2"));
    assert!(stdout.contains("Components: 1"));
    assert!(stdout.contains("Node types:"));
    assert!(stdout.contains("1e27b4d0: 1"));
    assert!(stdout.contains("d9f7db5f: 2"));
    assert!(stdout.contains("Edge types:"));
    assert!(stdout.contains("8e4ee065: 2"));
    assert!(stdout.contains("Tree:"));
    assert!(stdout.contains("[401e1d8f] type=1e27b4d0"));
    assert!(stdout
        .lines()
        .any(|line| line.starts_with("    ") && line.contains("type=d9f7db5f")));

    Ok(())
}

#[test]
fn inspect_json_reports_structured_metadata_and_stats() -> TestResult {
    let temp = write_demo_snapshot()?;
    let assert = echo_cli()
        .current_dir(temp.path())
        .args(["--format", "json", "inspect", "state.wsc"])
        .assert()
        .success();
    let json: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout)?;

    assert_eq!(json["metadata"]["file"], "state.wsc");
    assert_eq!(json["metadata"]["tick"], 42);
    assert_eq!(
        json["metadata"]["schema_hash"],
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(json["metadata"]["warp_count"], 1);
    assert_eq!(json["warps"][0]["total_nodes"], 3);
    assert_eq!(json["warps"][0]["total_edges"], 2);
    assert_eq!(json["warps"][0]["connected_components"], 1);
    assert_eq!(json["warps"][0]["node_types"]["1e27b4d0"], 1);
    assert_eq!(json["warps"][0]["node_types"]["d9f7db5f"], 2);
    assert_eq!(json["warps"][0]["edge_types"]["8e4ee065"], 2);
    assert!(json.get("tree").is_none());

    let node_type_sum = json["warps"][0]["node_types"]
        .as_object()
        .into_iter()
        .flat_map(serde_json::Map::values)
        .filter_map(serde_json::Value::as_u64)
        .sum::<u64>();
    assert_eq!(node_type_sum, 3);

    Ok(())
}

#[test]
fn inspect_corrupt_snapshot_exits_nonzero_without_panic() -> TestResult {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("bad.wsc"), b"not a wsc")?;

    echo_cli()
        .current_dir(temp.path())
        .args(["inspect", "bad.wsc"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("failed to open WSC file")
                .or(predicate::str::contains("WSC validation failed")),
        );

    Ok(())
}

#[test]
fn unknown_subcommand_exits_2() {
    echo_cli().arg("bogus").assert().code(2);
}

#[test]
fn no_subcommand_exits_2() {
    echo_cli().assert().code(2);
}

#[test]
fn verify_missing_file_exits_nonzero() {
    echo_cli()
        .args(["verify", "/nonexistent/path/state.wsc"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to open WSC file"));
}

#[test]
fn format_flag_is_global() {
    // --format should work before and after the subcommand.
    echo_cli()
        .args(["--format", "json", "verify", "--help"])
        .assert()
        .success();
}
