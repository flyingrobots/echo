// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration test for the echo-wesley-gen CLI (Wesley IR -> Rust code).

use std::io::Write;
use std::process::Command;

#[test]
fn test_generate_from_json() {
    let ir = r#"{
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
            }
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
}
