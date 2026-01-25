// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration tests for echo-ttd-gen code generation.
//!
//! These tests validate that the code generator produces valid Rust code
//! from various TTD IR fixtures.

use std::io::Write;
use std::process::{Command, Stdio};

/// Helper to run echo-ttd-gen with the given IR JSON and return the generated code.
fn generate_from_json(json: &str) -> Result<String, String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_echo-ttd-gen"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn echo-ttd-gen");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(json.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Helper to load a fixture file and generate code from it.
fn generate_from_fixture(fixture_name: &str) -> Result<String, String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture_path = format!("{}/tests/fixtures/{}", manifest_dir, fixture_name);
    let json = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fixture_path, e));
    generate_from_json(&json)
}

/// Verify generated code parses as valid Rust using syn.
fn assert_valid_rust(code: &str) {
    syn::parse_file(code).unwrap_or_else(|e| {
        panic!(
            "Generated code is not valid Rust:\n{}\n\nError: {}",
            code, e
        )
    });
}

// ─── Minimal Fixture Tests ───────────────────────────────────────────────────

#[test]
fn test_minimal_fixture_generates() {
    let code = generate_from_fixture("minimal.json").expect("Failed to generate from minimal.json");
    assert_valid_rust(&code);
}

#[test]
fn test_minimal_has_schema_constants() {
    let code = generate_from_fixture("minimal.json").unwrap();
    assert!(
        code.contains("pub const SCHEMA_SHA256"),
        "Missing SCHEMA_SHA256 constant"
    );
    assert!(
        code.contains("pub const GENERATED_AT"),
        "Missing GENERATED_AT constant"
    );
}

// ─── Counter Fixture Tests ───────────────────────────────────────────────────

#[test]
fn test_counter_fixture_generates() {
    let code = generate_from_fixture("counter.json").expect("Failed to generate from counter.json");
    assert_valid_rust(&code);
}

#[test]
fn test_counter_has_enums() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub enum CounterState"),
        "Missing CounterState enum"
    );
    assert!(code.contains("IDLE"), "Missing IDLE variant");
    assert!(code.contains("COUNTING"), "Missing COUNTING variant");
    assert!(code.contains("PAUSED"), "Missing PAUSED variant");
    assert!(code.contains("COMPLETED"), "Missing COMPLETED variant");
}

#[test]
fn test_counter_has_types() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub struct Counter"),
        "Missing Counter struct"
    );
    assert!(
        code.contains("pub struct CounterIncremented"),
        "Missing CounterIncremented event"
    );
    assert!(
        code.contains("pub struct CounterDecremented"),
        "Missing CounterDecremented event"
    );
    assert!(
        code.contains("pub struct CounterReset"),
        "Missing CounterReset event"
    );
}

#[test]
fn test_counter_has_channels() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub const CHANNEL_COUNTER"),
        "Missing CHANNEL_COUNTER constant"
    );
    assert!(
        code.contains("pub struct ChannelInfo"),
        "Missing ChannelInfo struct"
    );
    assert!(
        code.contains("pub const CHANNELS"),
        "Missing CHANNELS array"
    );
}

#[test]
fn test_counter_has_ops() {
    let code = generate_from_fixture("counter.json").unwrap();
    // Op constants
    assert!(
        code.contains("pub const OP_INCREMENT"),
        "Missing OP_INCREMENT constant"
    );
    assert!(
        code.contains("pub const OP_DECREMENT"),
        "Missing OP_DECREMENT constant"
    );
    assert!(
        code.contains("pub const OP_RESET"),
        "Missing OP_RESET constant"
    );
    assert!(
        code.contains("pub const OP_GETCOUNTER"),
        "Missing OP_GETCOUNTER constant"
    );

    // Op Args structs
    assert!(
        code.contains("pub struct IncrementArgs"),
        "Missing IncrementArgs struct"
    );
    assert!(
        code.contains("pub struct DecrementArgs"),
        "Missing DecrementArgs struct"
    );
    assert!(
        code.contains("pub struct ResetArgs"),
        "Missing ResetArgs struct"
    );
    assert!(
        code.contains("pub struct GetCounterArgs"),
        "Missing GetCounterArgs struct"
    );
}

#[test]
fn test_counter_has_rules() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub struct RuleInfo"),
        "Missing RuleInfo struct"
    );
    assert!(code.contains("pub const RULES"), "Missing RULES array");
    // Check rule names are present
    assert!(code.contains("stay_counting"), "Missing stay_counting rule");
    assert!(
        code.contains("decrement_rule"),
        "Missing decrement_rule rule"
    );
    assert!(code.contains("reset_to_idle"), "Missing reset_to_idle rule");
}

#[test]
fn test_counter_has_footprints() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub struct FootprintInfo"),
        "Missing FootprintInfo struct"
    );
    assert!(
        code.contains("pub const FOOTPRINTS"),
        "Missing FOOTPRINTS array"
    );
}

#[test]
fn test_counter_has_registry() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub struct RegistryEntryInfo"),
        "Missing RegistryEntryInfo struct"
    );
    assert!(
        code.contains("pub const REGISTRY"),
        "Missing REGISTRY array"
    );
}

#[test]
fn test_counter_has_invariants() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub struct InvariantInfo"),
        "Missing InvariantInfo struct"
    );
    assert!(
        code.contains("pub const INVARIANTS"),
        "Missing INVARIANTS array"
    );
    assert!(
        code.contains("value_non_negative"),
        "Missing value_non_negative invariant"
    );
    assert!(
        code.contains("value_bounded"),
        "Missing value_bounded invariant"
    );
}

#[test]
fn test_counter_has_emissions() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub struct EmissionInfo"),
        "Missing EmissionInfo struct"
    );
    assert!(
        code.contains("pub const EMISSIONS"),
        "Missing EMISSIONS array"
    );
}

// ─── Error Handling Tests ────────────────────────────────────────────────────

#[test]
fn test_rejects_invalid_ir_version() {
    let json = r#"{
        "ir_version": "ttd-ir/v99",
        "schema_sha256": "test",
        "generated_at": "2026-01-25T00:00:00Z",
        "channels": [], "ops": [], "rules": [], "footprints": [],
        "registry": [], "types": [], "enums": [], "invariants": [],
        "emissions": [], "codecs": [], "metadata": {}
    }"#;

    let result = generate_from_json(json);
    assert!(result.is_err(), "Should reject invalid IR version");
    let err = result.unwrap_err();
    assert!(
        err.contains("Unsupported ir_version"),
        "Error should mention unsupported version: {}",
        err
    );
}

#[test]
fn test_rejects_missing_ir_version() {
    let json = r#"{
        "schema_sha256": "test",
        "generated_at": "2026-01-25T00:00:00Z",
        "channels": [], "ops": [], "rules": [], "footprints": [],
        "registry": [], "types": [], "enums": [], "invariants": [],
        "emissions": [], "codecs": [], "metadata": {}
    }"#;

    let result = generate_from_json(json);
    assert!(result.is_err(), "Should reject missing IR version");
    let err = result.unwrap_err();
    assert!(
        err.contains("Missing ir_version"),
        "Error should mention missing version: {}",
        err
    );
}

#[test]
fn test_rejects_invalid_json() {
    let result = generate_from_json("not valid json");
    assert!(result.is_err(), "Should reject invalid JSON");
}

// ─── Generated Code Structure Tests ──────────────────────────────────────────

#[test]
fn test_counter_op_lookup_functions() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub fn op_by_id"),
        "Missing op_by_id function"
    );
    assert!(
        code.contains("pub fn op_by_name"),
        "Missing op_by_name function"
    );
}

#[test]
fn test_counter_rule_lookup_functions() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub fn rules_for_op"),
        "Missing rules_for_op function"
    );
    assert!(
        code.contains("pub fn rules_from_state"),
        "Missing rules_from_state function"
    );
}

#[test]
fn test_counter_emission_lookup_functions() {
    let code = generate_from_fixture("counter.json").unwrap();
    assert!(
        code.contains("pub fn emissions_for_op"),
        "Missing emissions_for_op function"
    );
    assert!(
        code.contains("pub fn emissions_for_channel"),
        "Missing emissions_for_channel function"
    );
}

#[test]
fn test_enum_default_impl() {
    let code = generate_from_fixture("counter.json").unwrap();
    // Check that Default is implemented for CounterState
    assert!(
        code.contains("impl Default for CounterState"),
        "Missing Default impl for CounterState"
    );
    // The first variant (IDLE) should be the default
    assert!(
        code.contains("Self::IDLE"),
        "Default should return IDLE (first variant)"
    );
}

// ─── Guard Expression Tests ──────────────────────────────────────────────────

#[test]
fn test_rule_guard_preserved() {
    let code = generate_from_fixture("counter.json").unwrap();
    // The decrement_rule has a guard: "value >= amount"
    assert!(
        code.contains("value >= amount"),
        "Guard expression should be preserved in generated code"
    );
}

// ─── Emission Condition Tests ────────────────────────────────────────────────

#[test]
fn test_emission_condition_preserved() {
    let code = generate_from_fixture("counter.json").unwrap();
    // The CounterDecremented emission has condition: "amount > 0"
    assert!(
        code.contains("amount > 0"),
        "Emission condition should be preserved in generated code"
    );
}
