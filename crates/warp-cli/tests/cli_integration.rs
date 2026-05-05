// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration tests for `echo-cli` binary.
//!
//! These tests run the actual binary via `assert_cmd` and verify exit codes,
//! help output, and error messages.

#![allow(deprecated)] // assert_cmd::cargo::cargo_bin deprecation — no stable replacement in v2.x

use assert_cmd::cargo::cargo_bin;
use predicates::prelude::*;

fn echo_cli() -> assert_cmd::Command {
    assert_cmd::Command::new(cargo_bin("echo-cli"))
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
fn help_matches_golden() {
    let assert = echo_cli().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout, include_str!("golden/echo-cli-help.txt"));
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
        .stdout(predicate::str::contains("filter"));
}

#[test]
fn inspect_help_lists_tree_flag() {
    echo_cli()
        .args(["inspect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tree"));
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
