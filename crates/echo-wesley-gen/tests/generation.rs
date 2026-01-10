// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration test for the echo-wesley-gen CLI (Wesley IR -> Rust code).

use std::io::Write;
use std::process::Command;

#[test]
fn test_generate_from_json() {
    let ir = r#"{
        "ir_version": "echo-ir/v1",
        "schema_sha256": "abc123",
        "codec_id": "cbor-canon-v1",
        "registry_version": 7,
        "types": [
            {
                "name": "AppState",
                "kind": "OBJECT",
                "fields": [
                    { "name": "theme", "type": "Theme", "required": true },
                    { "name": "tags", "type": "String", "required": false, "list": true }
                ]
            },
            {
                "name": "Theme",
                "kind": "ENUM",
                "values": ["LIGHT", "DARK"]
            },
            {
                "name": "Mutation",
                "kind": "OBJECT",
                "fields": [
                    { "name": "setTheme", "type": "AppState", "required": true }
                ]
            },
            {
                "name": "Query",
                "kind": "OBJECT",
                "fields": [
                    { "name": "appState", "type": "AppState", "required": true }
                ]
            }
        ],
        "ops": [
            { "kind": "MUTATION", "name": "setTheme", "op_id": 111, "args": [], "result_type": "AppState" },
            { "kind": "QUERY", "name": "appState", "op_id": 222, "args": [], "result_type": "AppState" }
        ]
    }"#;

    let mut child = Command::new("cargo")
        .args(["run", "-p", "echo-wesley-gen", "--"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    let mut stdin = child.stdin.take().expect("failed to get stdin");
    stdin
        .write_all(ir.as_bytes())
        .expect("failed to write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("failed to wait on child");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("pub struct AppState"));
    assert!(stdout.contains("pub enum Theme"));
    assert!(stdout.contains("pub theme: Theme"));
    assert!(stdout.contains("pub tags: Option<Vec<String>>"));
    assert!(stdout.contains("pub const SCHEMA_SHA256: &str = \"abc123\""));
    assert!(stdout.contains("pub const CODEC_ID: &str = \"cbor-canon-v1\""));
    assert!(stdout.contains("pub const REGISTRY_VERSION: u32 = 7"));
    assert!(stdout.contains("pub const OP_SET_THEME: u32 = 111"));
    assert!(stdout.contains("pub const OP_APP_STATE: u32 = 222"));
}

#[test]
fn test_ops_catalog_present() {
    let ir = r#"{
        "ir_version": "echo-ir/v1",
        "types": [],
        "ops": [
            { "kind": "MUTATION", "name": "setTheme", "op_id": 123, "args": [], "result_type": "AppState" },
            { "kind": "QUERY", "name": "appState", "op_id": 456, "args": [], "result_type": "AppState" }
        ]
    }"#;

    let mut child = Command::new("cargo")
        .args(["run", "-p", "echo-wesley-gen", "--"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    let mut stdin = child.stdin.take().expect("failed to get stdin");
    stdin
        .write_all(ir.as_bytes())
        .expect("failed to write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("failed to wait on child");
    assert!(output.status.success());
}

#[test]
fn test_rejects_unknown_version() {
    let ir = r#"{
        "ir_version": "echo-ir/v2",
        "types": []
    }"#;

    let output = Command::new("cargo")
        .args(["run", "-p", "echo-wesley-gen", "--"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .expect("failed to get stdin")
                .write_all(ir.as_bytes())
                .expect("failed to write to stdin");
            child.wait_with_output()
        })
        .expect("failed to run process");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported ir_version"));
}
