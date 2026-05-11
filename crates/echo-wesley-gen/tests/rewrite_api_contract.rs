// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::unwrap_used)]
//! Consumer-side proof that Echo can compile against Wesley's bounded rewrite API.

use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const GENERATED_REWRITE_API: &str = include_str!("fixtures/rewrite_api.generated.rs");
const GENERATED_STRUCTURED_REWRITE_API: &str =
    include_str!("fixtures/rewrite_api_structured.generated.rs");
const VALID_ECHO_SIDE_IMPLEMENTATION: &str =
    include_str!("fixtures/rewrite_contract_valid_echo_side.rs");
const DISHONEST_ECHO_SIDE_IMPLEMENTATION: &str =
    include_str!("fixtures/rewrite_contract_dishonest_echo_side.rs");
const VALID_STRUCTURED_ECHO_SIDE_IMPLEMENTATION: &str =
    include_str!("fixtures/rewrite_contract_valid_structured_echo_side.rs");
const DISHONEST_STRUCTURED_ECHO_SIDE_IMPLEMENTATION: &str =
    include_str!("fixtures/rewrite_contract_dishonest_structured_echo_side.rs");

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_rust(source: &str) -> std::process::Output {
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "echo-wesley-gen-rewrite-proof-{}-{}-{}",
        std::process::id(),
        nanos,
        unique
    ));
    create_dir_all(&dir).expect("failed to create temp dir");

    let src_path: PathBuf = dir.join("proof.rs");
    let out_path: PathBuf = dir.join("proof.rlib");
    write(&src_path, source).expect("failed to write proof source");

    let output = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            src_path.to_str().expect("non-utf8 source path"),
            "-o",
            out_path.to_str().expect("non-utf8 output path"),
        ])
        .output()
        .expect("failed to invoke rustc");

    remove_dir_all(&dir).expect("failed to remove temp dir");
    output
}

#[test]
fn fixture_exposes_only_declared_counter_capabilities() {
    assert!(GENERATED_REWRITE_API.contains("pub trait ReadCounter"));
    assert!(GENERATED_REWRITE_API.contains("pub trait WriteCounter"));
    assert!(GENERATED_REWRITE_API.contains("pub trait IncrementCounterContext"));
    assert!(GENERATED_REWRITE_API.contains("pub trait IncrementCounterRewrite"));
    assert!(!GENERATED_REWRITE_API.contains("DeleteCounter"));
}

#[test]
fn valid_echo_side_implementation_compiles() {
    let compile = compile_rust(&format!(
        "{GENERATED_REWRITE_API}\n{VALID_ECHO_SIDE_IMPLEMENTATION}"
    ));

    assert!(
        compile.status.success(),
        "rustc failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
}

#[test]
fn dishonest_echo_side_implementation_fails_to_compile() {
    let compile = compile_rust(&format!(
        "{GENERATED_REWRITE_API}\n{DISHONEST_ECHO_SIDE_IMPLEMENTATION}"
    ));

    assert!(
        !compile.status.success(),
        "expected rustc failure, got success"
    );
    let stderr = String::from_utf8_lossy(&compile.stderr);
    assert!(
        stderr.contains("delete_counter"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn structured_fixture_exposes_only_declared_replace_range_capabilities() {
    assert!(
        GENERATED_STRUCTURED_REWRITE_API.contains("pub trait ReplaceRangeAsTickReadWorldlineSlot")
    );
    assert!(GENERATED_STRUCTURED_REWRITE_API
        .contains("pub trait ReplaceRangeAsTickReadTouchedRopeClosure"));
    assert!(
        GENERATED_STRUCTURED_REWRITE_API.contains("pub trait ReplaceRangeAsTickCreateNextHeadSlot")
    );
    assert!(GENERATED_STRUCTURED_REWRITE_API
        .contains("pub trait ReplaceRangeAsTickUpdateWorldlineCanonicalHead"));
    assert!(GENERATED_STRUCTURED_REWRITE_API.contains(
        "// ReplaceRangeAsTick forbidden surfaces: AstState, Diagnostics, GitWitness, UiState"
    ));
    assert!(!GENERATED_STRUCTURED_REWRITE_API.contains("read_ast_state_slot"));
}

#[test]
fn valid_structured_echo_side_implementation_compiles() {
    let compile = compile_rust(&format!(
        "{GENERATED_STRUCTURED_REWRITE_API}\n{VALID_STRUCTURED_ECHO_SIDE_IMPLEMENTATION}"
    ));

    assert!(
        compile.status.success(),
        "rustc failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
}

#[test]
fn dishonest_structured_echo_side_implementation_fails_to_compile() {
    let compile = compile_rust(&format!(
        "{GENERATED_STRUCTURED_REWRITE_API}\n{DISHONEST_STRUCTURED_ECHO_SIDE_IMPLEMENTATION}"
    ));

    assert!(
        !compile.status.success(),
        "expected rustc failure, got success"
    );
    let stderr = String::from_utf8_lossy(&compile.stderr);
    assert!(
        stderr.contains("read_ast_state_slot"),
        "unexpected stderr: {stderr}"
    );
}
