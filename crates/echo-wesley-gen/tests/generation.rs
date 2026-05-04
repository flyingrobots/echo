// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration test for the echo-wesley-gen CLI (Wesley IR -> Rust code).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
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

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root ancestor missing")
        .to_path_buf()
}

fn write_consumer_smoke_crate(generated: &str) -> PathBuf {
    let workspace = workspace_root();
    let crate_dir = workspace
        .join("target")
        .join("echo-wesley-gen-consumer-smoke")
        .join(std::process::id().to_string());
    if crate_dir.exists() {
        fs::remove_dir_all(&crate_dir).expect("failed to remove old smoke crate");
    }
    fs::create_dir_all(crate_dir.join("src")).expect("failed to create smoke crate");

    let registry_path = workspace.join("crates/echo-registry-api");
    let wasm_abi_path = workspace.join("crates/echo-wasm-abi");
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "echo-wesley-gen-consumer-smoke"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[dependencies]
echo-registry-api = {{ path = "{}" }}
echo-wasm-abi = {{ path = "{}" }}
serde = {{ version = "1.0", features = ["derive"] }}
"#,
            registry_path.display(),
            wasm_abi_path.display()
        ),
    )
    .expect("failed to write smoke Cargo.toml");
    fs::write(crate_dir.join("src/generated.rs"), generated)
        .expect("failed to write generated module");
    fs::write(
        crate_dir.join("src/lib.rs"),
        r#"
mod generated;

#[cfg(test)]
mod tests {
    use super::generated::{
        counter_value_observation_request, counter_value_observation_request_raw_vars,
        encode_counter_value_vars, pack_increment_intent, CounterValueVars, IncrementInput,
        IncrementVars, CODEC_ID, OP_COUNTER_VALUE, OP_INCREMENT, REGISTRY, REGISTRY_VERSION,
        SCHEMA_SHA256,
    };
    use echo_registry_api::{OpKind, RegistryProvider};
    use echo_wasm_abi::kernel_port::{
        AbiError, BuiltinObserverPlan, DispatchResponse, KernelPort, ObservationArtifact,
        ObservationAt, ObservationBasisPosture, ObservationFrame, ObservationPayload,
        ObservationProjection, ReadingBudgetPosture, ReadingEnvelope, ReadingObserverBasis,
        ReadingObserverPlan, ReadingResidualPosture, ReadingRightsPosture, ReadingWitnessRef,
        RegistryInfo, ResolvedObservationCoordinate, RunCompletion, SchedulerState,
        SchedulerStatus, WorkState, WorldlineId, WorldlineTick, ABI_VERSION,
    };
    use echo_wasm_abi::{decode_cbor, unpack_intent_v1};

    #[derive(Default)]
    struct ToyKernel {
        accepted_intent_count: usize,
    }

    fn idle_status() -> SchedulerStatus {
        SchedulerStatus {
            state: SchedulerState::Inactive,
            active_mode: None,
            work_state: WorkState::Quiescent,
            run_id: None,
            latest_cycle_global_tick: None,
            latest_commit_global_tick: None,
            last_quiescent_global_tick: None,
            last_run_completion: Some(RunCompletion::Quiesced),
        }
    }

    impl KernelPort for ToyKernel {
        fn dispatch_intent(
            &mut self,
            intent_bytes: &[u8],
        ) -> Result<DispatchResponse, AbiError> {
            let (op_id, vars) = unpack_intent_v1(intent_bytes).map_err(|error| AbiError {
                code: 1,
                message: error.to_string(),
            })?;
            assert_eq!(op_id, OP_INCREMENT);
            let decoded: IncrementVars = decode_cbor(vars).map_err(|error| AbiError {
                code: 2,
                message: error.to_string(),
            })?;
            assert_eq!(decoded.input.amount, 42);
            self.accepted_intent_count += 1;
            Ok(DispatchResponse {
                accepted: true,
                intent_id: vec![7; 32],
                scheduler_status: idle_status(),
            })
        }

        fn observe(
            &self,
            request: echo_wasm_abi::kernel_port::ObservationRequest,
        ) -> Result<ObservationArtifact, AbiError> {
            assert_eq!(request.frame, ObservationFrame::QueryView);
            let ObservationProjection::Query {
                query_id,
                vars_bytes,
            } = &request.projection
            else {
                panic!("expected query projection");
            };
            assert_eq!(*query_id, OP_COUNTER_VALUE);
            let _decoded: CounterValueVars =
                decode_cbor(vars_bytes).map_err(|error| AbiError {
                    code: 3,
                    message: error.to_string(),
                })?;

            let worldline_id = request.coordinate.worldline_id;
            let state_root = vec![0; 32];
            let commit_hash = vec![1; 32];
            Ok(ObservationArtifact {
                resolved: ResolvedObservationCoordinate {
                    observation_version: 1,
                    worldline_id,
                    requested_at: ObservationAt::Frontier,
                    resolved_worldline_tick: WorldlineTick::ZERO,
                    commit_global_tick: None,
                    observed_after_global_tick: None,
                    state_root: state_root.clone(),
                    commit_hash: commit_hash.clone(),
                },
                reading: ReadingEnvelope {
                    observer_plan: ReadingObserverPlan::Builtin {
                        plan: BuiltinObserverPlan::QueryBytes,
                    },
                    observer_basis: ReadingObserverBasis::QueryView,
                    witness_refs: vec![ReadingWitnessRef::EmptyFrontier {
                        worldline_id,
                        state_root,
                        commit_hash,
                    }],
                    parent_basis_posture: ObservationBasisPosture::Worldline,
                    budget_posture: ReadingBudgetPosture::UnboundedOneShot,
                    rights_posture: ReadingRightsPosture::KernelPublic,
                    residual_posture: ReadingResidualPosture::Complete,
                },
                frame: request.frame,
                projection: request.projection,
                artifact_hash: vec![2; 32],
                payload: ObservationPayload::QueryBytes {
                    data: b"counter=42".to_vec(),
                },
            })
        }

        fn registry_info(&self) -> RegistryInfo {
            RegistryInfo {
                codec_id: Some(CODEC_ID.to_string()),
                registry_version: Some(REGISTRY_VERSION.to_string()),
                schema_sha256_hex: Some(SCHEMA_SHA256.to_string()),
                abi_version: ABI_VERSION,
            }
        }

        fn scheduler_status(&self) -> Result<SchedulerStatus, AbiError> {
            Ok(idle_status())
        }
    }

    #[test]
    fn generated_contract_dispatches_and_observes_through_kernel_port() {
        let registry_info = REGISTRY.info();
        assert_eq!(registry_info.codec_id, CODEC_ID);
        assert_eq!(registry_info.registry_version, REGISTRY_VERSION);
        assert_eq!(registry_info.schema_sha256_hex, SCHEMA_SHA256);
        assert_eq!(REGISTRY.op_by_id(OP_INCREMENT).unwrap().kind, OpKind::Mutation);
        assert_eq!(REGISTRY.op_by_id(OP_COUNTER_VALUE).unwrap().kind, OpKind::Query);

        let intent = pack_increment_intent(&IncrementVars {
            input: IncrementInput { amount: 42 },
        })
        .unwrap();
        let (op_id, vars) = unpack_intent_v1(&intent).unwrap();
        assert_eq!(op_id, OP_INCREMENT);
        let decoded: IncrementVars = decode_cbor(vars).unwrap();
        assert_eq!(decoded.input.amount, 42);

        let mut kernel = ToyKernel::default();
        let response = KernelPort::dispatch_intent(&mut kernel, &intent).unwrap();
        assert!(response.accepted);
        assert_eq!(kernel.accepted_intent_count, 1);

        let host_registry = KernelPort::registry_info(&kernel);
        assert_eq!(host_registry.codec_id.as_deref(), Some(CODEC_ID));
        assert_eq!(
            host_registry.registry_version.as_deref(),
            Some(REGISTRY_VERSION.to_string().as_str())
        );
        assert_eq!(host_registry.schema_sha256_hex.as_deref(), Some(SCHEMA_SHA256));

        let worldline_id = WorldlineId::from_bytes([9; 32]);
        let query_vars = CounterValueVars {};
        let encoded_query_vars = encode_counter_value_vars(&query_vars).unwrap();
        let request = counter_value_observation_request(worldline_id, &query_vars).unwrap();
        let raw_request =
            counter_value_observation_request_raw_vars(worldline_id, &encoded_query_vars);
        assert_eq!(request, raw_request);
        let artifact = KernelPort::observe(&kernel, request).unwrap();
        assert_eq!(artifact.frame, ObservationFrame::QueryView);
        assert_eq!(
            artifact.payload,
            ObservationPayload::QueryBytes {
                data: b"counter=42".to_vec()
            }
        );
    }
}
"#,
    )
    .expect("failed to write smoke lib.rs");

    crate_dir
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
fn test_toy_contract_generates_eint_and_observation_helpers() {
    let ir = r#"{
        "ir_version": "echo-ir/v1",
        "schema_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "codec_id": "cbor-canon-v1",
        "registry_version": 1,
        "types": [
            {
                "name": "CounterValue",
                "kind": "OBJECT",
                "fields": [
                    { "name": "value", "type": "Int", "required": true }
                ]
            },
            {
                "name": "IncrementInput",
                "kind": "INPUT_OBJECT",
                "fields": [
                    { "name": "amount", "type": "Int", "required": true }
                ]
            },
            {
                "name": "Mutation",
                "kind": "OBJECT",
                "fields": [
                    { "name": "increment", "type": "CounterValue", "required": true }
                ]
            },
            {
                "name": "Query",
                "kind": "OBJECT",
                "fields": [
                    { "name": "counterValue", "type": "CounterValue", "required": true }
                ]
            }
        ],
        "ops": [
            {
                "kind": "MUTATION",
                "name": "increment",
                "op_id": 1001,
                "args": [
                    { "name": "input", "type": "IncrementInput", "required": true }
                ],
                "result_type": "CounterValue"
            },
            {
                "kind": "QUERY",
                "name": "counterValue",
                "op_id": 1002,
                "args": [],
                "result_type": "CounterValue"
            }
        ]
    }"#;

    let output = run_wesley_gen(ir);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("pub const OP_INCREMENT: u32 = 1001"));
    assert!(stdout.contains("pub const OP_COUNTER_VALUE: u32 = 1002"));
    assert!(stdout.contains("pub static REGISTRY: GeneratedRegistry"));

    for required in [
        "use echo_wasm_abi::pack_intent_v1;",
        "pub struct IncrementVars",
        "pub struct CounterValueVars",
        "pub fn encode_increment_vars",
        "pub fn encode_counter_value_vars",
        "pub fn pack_increment_intent",
        "pub fn pack_increment_intent_raw_vars",
        "pack_intent_v1(OP_INCREMENT",
        "pub fn counter_value_observation_request",
        "pub fn counter_value_observation_request_raw_vars",
    ] {
        assert!(
            stdout.contains(required),
            "generated toy contract output is missing first-consumer bridge: {required}"
        );
    }
}

#[test]
fn test_toy_contract_generated_output_compiles_in_consumer_crate() {
    let ir = r#"{
        "ir_version": "echo-ir/v1",
        "schema_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "codec_id": "cbor-canon-v1",
        "registry_version": 1,
        "types": [
            {
                "name": "CounterValue",
                "kind": "OBJECT",
                "fields": [
                    { "name": "value", "type": "Int", "required": true }
                ]
            },
            {
                "name": "IncrementInput",
                "kind": "INPUT_OBJECT",
                "fields": [
                    { "name": "amount", "type": "Int", "required": true }
                ]
            }
        ],
        "ops": [
            {
                "kind": "MUTATION",
                "name": "increment",
                "op_id": 1001,
                "args": [
                    { "name": "input", "type": "IncrementInput", "required": true }
                ],
                "result_type": "CounterValue"
            },
            {
                "kind": "QUERY",
                "name": "counterValue",
                "op_id": 1002,
                "args": [],
                "result_type": "CounterValue"
            }
        ]
    }"#;

    let output = run_wesley_gen(ir);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let generated = String::from_utf8_lossy(&output.stdout);
    let crate_dir = write_consumer_smoke_crate(&generated);
    let output = Command::new("cargo")
        .args(["test", "--manifest-path"])
        .arg(crate_dir.join("Cargo.toml"))
        .output()
        .expect("failed to run generated consumer smoke");

    assert!(
        output.status.success(),
        "generated consumer smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
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
