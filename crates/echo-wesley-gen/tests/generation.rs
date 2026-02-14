// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration test for the echo-wesley-gen CLI (Wesley IR -> Rust code).

use std::io::Write;
use std::process::{Command, Output, Stdio};

/// Spawns `cargo run -p echo-wesley-gen --`, pipes `ir` to stdin, and returns the output.
fn run_wesley_gen(ir: &str) -> Output {
    let mut child = Command::new("cargo")
        .args(["run", "-p", "echo-wesley-gen", "--"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn cargo run");

    let mut stdin = child.stdin.take().expect("failed to get stdin");
    stdin
        .write_all(ir.as_bytes())
        .expect("failed to write to stdin");
    drop(stdin);

    child.wait_with_output().expect("failed to wait on child")
}

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

    let output = run_wesley_gen(ir);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
    assert!(stdout.contains("use echo_registry_api::{"));
    assert!(stdout.contains("pub const OPS: &[OpDef]"));
    assert!(stdout.contains("pub static REGISTRY: GeneratedRegistry"));
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

    let output = run_wesley_gen(ir);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pub const OP_SET_THEME: u32 = 123"));
    assert!(stdout.contains("pub const OP_APP_STATE: u32 = 456"));
    assert!(
        stdout.contains("pub const OPS: &[OpDef]"),
        "ops catalog not found in output"
    );
}

#[test]
fn test_generate_no_std_minicbor() {
    let ir = r#"{
        "ir_version": "echo-ir/v1",
        "types": [
            {
                "name": "Node",
                "kind": "OBJECT",
                "fields": [
                    { "name": "id", "type": "ID", "required": true },
                    { "name": "pos", "type": "Float", "required": true, "list": true }
                ]
            },
            {
                "name": "Status",
                "kind": "ENUM",
                "values": ["ACTIVE", "INACTIVE"]
            }
        ],
        "ops": []
    }"#;

    // Run with flags
    let mut child = Command::new("cargo")
        .args([
            "run",
            "-p",
            "echo-wesley-gen",
            "--",
            "--no-std",
            "--minicbor",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn cargo run");

    let mut stdin = child.stdin.take().expect("failed to get stdin");
    stdin
        .write_all(ir.as_bytes())
        .expect("failed to write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("failed to wait on child");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify no_std artifacts
    assert!(stdout.contains("extern crate alloc;"));
    assert!(stdout.contains("use alloc::string::String;"));
    assert!(stdout.contains("use alloc::vec::Vec;"));

    // Verify minicbor artifacts
    assert!(stdout.contains("use minicbor::{Encode, Decode};"));
    assert!(stdout
        .contains("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]"));
    assert!(stdout.contains("#[cbor(index_only)]"));
    assert!(stdout.contains("#[n(0u64)]"));
    assert!(stdout.contains("#[n(1u64)]"));

    // Verify ID -> [u8; 32] mapping for no_std
    assert!(stdout.contains("pub id: [u8; 32]"));
    assert!(stdout.contains("pub pos: Vec<f32>"));
}

#[test]
fn test_rejects_unknown_version() {
    let ir = r#"{
        "ir_version": "echo-ir/v2",
        "types": []
    }"#;

    let output = run_wesley_gen(ir);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported ir_version"));
}

#[test]
fn test_rejects_missing_version() {
    let ir = r#"{
        "types": []
    }"#;

    let output = run_wesley_gen(ir);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Missing ir_version"));
}
