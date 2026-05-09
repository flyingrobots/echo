// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration test for the echo-wesley-gen CLI (Wesley IR -> Rust code).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const TOY_COUNTER_IR: &str = include_str!("fixtures/toy-counter/echo-ir-v1.json");

/// Spawns `cargo run -p echo-wesley-gen --`, pipes `ir` to stdin, and returns the output.
fn run_wesley_gen(ir: &str) -> Output {
    run_wesley_gen_with_args(ir, &[])
}

/// Spawns `cargo run -p echo-wesley-gen -- <args>`, pipes `ir` to stdin, and returns the output.
fn run_wesley_gen_with_args(ir: &str, args: &[&str]) -> Output {
    let mut child = Command::new("cargo")
        .args(["run", "-p", "echo-wesley-gen", "--"])
        .args(args)
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

fn run_wesley_gen_schema(schema_path: &Path) -> Output {
    Command::new("cargo")
        .args(["run", "-p", "echo-wesley-gen", "--", "--schema"])
        .arg(schema_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run cargo run")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root ancestor missing")
        .to_path_buf()
}

#[test]
fn test_generate_from_graphql_schema_uses_wesley_core() {
    let workspace = workspace_root();
    let fixture_dir = workspace
        .join("target")
        .join("echo-wesley-gen-schema-fixture")
        .join(std::process::id().to_string());
    fs::create_dir_all(&fixture_dir).expect("failed to create schema fixture dir");
    let schema_path = fixture_dir.join("counter.graphql");
    fs::write(
        &schema_path,
        r#"
directive @wes_op(name: String!) on FIELD_DEFINITION
directive @wes_footprint(reads: [String!], writes: [String!]) on FIELD_DEFINITION

type CounterValue {
  value: Int!
}

input IncrementInput {
  amount: Int!
}

type Query {
  counterValue: CounterValue! @wes_op(name: "counterValue")
}

type Mutation {
  increment(input: IncrementInput!): CounterValue!
    @wes_op(name: "increment")
    @wes_footprint(reads: ["CounterValue"], writes: ["CounterValue"])
}
"#,
    )
    .expect("failed to write schema fixture");

    let output = run_wesley_gen_schema(&schema_path);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("pub struct CounterValue"));
    assert!(stdout.contains("pub struct IncrementInput"));
    assert!(stdout.contains("pub const CODEC_ID: &str = \"cbor-canon-v1\""));
    assert!(stdout.contains("pub const REGISTRY_VERSION: u32 = 1"));
    assert!(stdout.contains("pub const OP_COUNTER_VALUE: u32 ="));
    assert!(stdout.contains("pub const OP_INCREMENT: u32 ="));
    assert!(stdout.contains("pub struct CounterValueVars"));
    assert!(stdout.contains("pub struct IncrementVars"));
    assert!(stdout.contains("pub fn counter_value_observe_optic_request"));
    assert!(stdout.contains("pub fn increment_dispatch_optic_intent_request"));
    assert!(stdout.contains("directives_json:"));
    assert!(stdout.contains("\\\"wes_footprint\\\""));
    assert!(stdout.contains("OP_INCREMENT_FOOTPRINT_CERTIFICATE"));
    assert!(stdout.contains("footprint_certificate: Some(&OP_INCREMENT_FOOTPRINT_CERTIFICATE)"));
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
    let warp_wasm_path = workspace.join("crates/warp-wasm");
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
warp-wasm = {{ path = "{}" }}
serde = {{ version = "1.0", features = ["derive"] }}
"#,
            registry_path.display(),
            wasm_abi_path.display(),
            warp_wasm_path.display()
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
        __echo_wesley_generated::{CounterValueVars, IncrementVars},
        counter_value_observation_request, counter_value_observation_request_raw_vars,
        counter_value_observe_optic_request, counter_value_observe_optic_request_raw_vars,
        encode_counter_value_vars, increment_dispatch_optic_intent_request, pack_increment_intent,
        IncrementInput, CODEC_ID, OP_COUNTER_VALUE, OP_INCREMENT,
        OP_INCREMENT_FOOTPRINT_CERTIFICATE_HASH, REGISTRY, REGISTRY_VERSION, SCHEMA_SHA256,
    };
    use echo_registry_api::{OpKind, RegistryProvider};
    use echo_wasm_abi::kernel_port::{
        AbiError, AdmissionLawId, BuiltinObserverPlan, CoordinateAt, DispatchOpticIntentRequest,
        DispatchResponse, EchoCoordinate, IntentFamilyId, KernelPort, ObservationArtifact,
        ObservationAt, ObservationBasisPosture, ObservationFrame, ObservationPayload,
        ObservationProjection, ObserveOpticRequest, OpticActorId, OpticApertureShape,
        OpticCapability, OpticCapabilityId, OpticCause, OpticFocus, OpticIntentPayload,
        OpticReadBudget, ProjectionVersion, ReadingBudgetPosture, ReadingEnvelope,
        ReadingObserverBasis, ReadingObserverPlan, ReadingResidualPosture, ReadingRightsPosture,
        ReadingWitnessRef, RegistryInfo, ResolvedObservationCoordinate, RunCompletion, OkEnvelope,
        SchedulerState, SchedulerStatus, WorkState, WorldlineId, WorldlineTick, ABI_VERSION,
    };
    use echo_wasm_abi::{decode_cbor, encode_cbor, unpack_intent_v1};

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
                    observer_instance: None,
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
        assert!(
            REGISTRY
                .op_by_id(OP_INCREMENT)
                .unwrap()
                .directives_json
                .contains("\"wes_footprint\"")
        );
        let increment_certificate = REGISTRY
            .op_by_id(OP_INCREMENT)
            .unwrap()
            .footprint_certificate
            .expect("increment operation must carry a footprint certificate");
        assert_eq!(increment_certificate.op_id, OP_INCREMENT);
        assert_eq!(increment_certificate.op_name, "increment");
        assert_eq!(increment_certificate.schema_sha256_hex, SCHEMA_SHA256);
        assert_eq!(
            increment_certificate.certificate_hash_hex,
            OP_INCREMENT_FOOTPRINT_CERTIFICATE_HASH
        );
        assert!(REGISTRY
            .op_by_id(OP_INCREMENT)
            .unwrap()
            .footprint_certificate_matches(SCHEMA_SHA256, OP_INCREMENT_FOOTPRINT_CERTIFICATE_HASH));
        assert!(!REGISTRY
            .op_by_id(OP_INCREMENT)
            .unwrap()
            .footprint_certificate_matches(SCHEMA_SHA256, "wrong-hash"));
        assert_eq!(increment_certificate.reads, &["CounterValue"]);
        assert_eq!(increment_certificate.writes, &["CounterValue"]);
        assert_eq!(REGISTRY.op_by_id(OP_COUNTER_VALUE).unwrap().kind, OpKind::Query);
        assert_eq!(
            REGISTRY
                .op_by_id(OP_COUNTER_VALUE)
                .unwrap()
                .footprint_certificate,
            None
        );

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

        let optic_id = echo_wasm_abi::kernel_port::OpticId::from_bytes([10; 32]);
        let capability_id = OpticCapabilityId::from_bytes([11; 32]);
        let actor = OpticActorId::from_bytes([12; 32]);
        let intent_family = IntentFamilyId::from_bytes([13; 32]);
        let focus = OpticFocus::Worldline { worldline_id };
        let coordinate = EchoCoordinate::Worldline {
            worldline_id,
            at: CoordinateAt::Frontier,
        };
        let budget = OpticReadBudget {
            max_bytes: Some(4096),
            max_nodes: Some(64),
            max_ticks: Some(8),
            max_attachments: Some(0),
        };
        let optic_read = counter_value_observe_optic_request(
            optic_id,
            focus.clone(),
            coordinate.clone(),
            capability_id,
            ProjectionVersion(1),
            None,
            budget,
            &query_vars,
        )
        .unwrap();
        let raw_optic_read = counter_value_observe_optic_request_raw_vars(
            optic_id,
            focus.clone(),
            coordinate.clone(),
            capability_id,
            ProjectionVersion(1),
            None,
            budget,
            &encoded_query_vars,
        );
        assert_eq!(optic_read, raw_optic_read);
        let decoded_read: ObserveOpticRequest =
            decode_cbor(&encode_cbor(&optic_read).unwrap()).unwrap();
        assert_eq!(decoded_read, optic_read);
        assert!(matches!(
            optic_read.aperture.shape,
            OpticApertureShape::QueryBytes { query_id, ref vars_digest }
                if query_id == OP_COUNTER_VALUE
                    && vars_digest.len() == 32
                    && vars_digest != &encoded_query_vars
        ));

        let capability = OpticCapability {
            capability_id,
            actor,
            issuer_ref: None,
            policy_hash: vec![14; 32],
            allowed_focus: focus.clone(),
            projection_version: ProjectionVersion(1),
            reducer_version: None,
            allowed_intent_family: intent_family,
            max_budget: budget,
        };
        let cause = OpticCause {
            actor,
            cause_hash: vec![15; 32],
            label: Some("generated optic dispatch".into()),
        };
        let dispatch = increment_dispatch_optic_intent_request(
            optic_id,
            coordinate.clone(),
            intent_family,
            focus,
            cause,
            capability,
            AdmissionLawId::from_bytes([16; 32]),
            &IncrementVars {
                input: IncrementInput { amount: 42 },
            },
        )
        .unwrap();
        let decoded_dispatch: DispatchOpticIntentRequest =
            decode_cbor(&encode_cbor(&dispatch).unwrap()).unwrap();
        assert_eq!(decoded_dispatch, dispatch);
        assert_eq!(dispatch.base_coordinate, coordinate);
        let OpticIntentPayload::EintV1 { bytes } = &dispatch.payload;
        let (op_id, vars_bytes) = unpack_intent_v1(bytes).unwrap();
        assert_eq!(op_id, OP_INCREMENT);
        let decoded: IncrementVars = decode_cbor(vars_bytes).unwrap();
        assert_eq!(decoded.input.amount, 42);
    }

    #[test]
    fn generated_contract_runs_through_installed_warp_wasm_kernel() {
        let kernel = ToyKernel::default();
        warp_wasm::install_kernel(Box::new(kernel));

        let registry_envelope: OkEnvelope<RegistryInfo> =
            decode_cbor(&warp_wasm::get_registry_info_cbor()).unwrap();
        assert_eq!(registry_envelope.data.codec_id.as_deref(), Some(CODEC_ID));
        assert_eq!(
            registry_envelope.data.registry_version.as_deref(),
            Some(REGISTRY_VERSION.to_string().as_str())
        );
        assert_eq!(
            registry_envelope.data.schema_sha256_hex.as_deref(),
            Some(SCHEMA_SHA256)
        );

        let intent = pack_increment_intent(&IncrementVars {
            input: IncrementInput { amount: 42 },
        })
        .unwrap();
        let dispatch_envelope: OkEnvelope<DispatchResponse> =
            decode_cbor(&warp_wasm::dispatch_intent_cbor(&intent)).unwrap();
        assert!(dispatch_envelope.data.accepted);
        assert_eq!(dispatch_envelope.data.intent_id, vec![7; 32]);

        let worldline_id = WorldlineId::from_bytes([9; 32]);
        let request = counter_value_observation_request(worldline_id, &CounterValueVars {})
            .unwrap();
        let request_bytes = encode_cbor(&request).unwrap();
        let observe_envelope: OkEnvelope<ObservationArtifact> =
            decode_cbor(&warp_wasm::observe_cbor(&request_bytes)).unwrap();
        assert_eq!(observe_envelope.data.frame, ObservationFrame::QueryView);
        assert_eq!(
            observe_envelope.data.payload,
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

fn write_basic_generated_crate(generated: &str, label: &str, no_std: bool) -> PathBuf {
    let workspace = workspace_root();
    let crate_dir = workspace
        .join("target")
        .join("echo-wesley-gen-basic-smoke")
        .join(std::process::id().to_string())
        .join(label);
    if crate_dir.exists() {
        fs::remove_dir_all(&crate_dir).expect("failed to remove old generated crate");
    }
    fs::create_dir_all(crate_dir.join("src")).expect("failed to create generated crate");

    let registry_path = workspace.join("crates/echo-registry-api");
    let wasm_abi_path = workspace.join("crates/echo-wasm-abi");
    let registry_dependency = if no_std {
        format!(
            r#"echo-registry-api = {{ path = "{}", default-features = false }}"#,
            registry_path.display()
        )
    } else {
        format!(
            r#"echo-registry-api = {{ path = "{}" }}"#,
            registry_path.display()
        )
    };
    let wasm_abi_dependency = if no_std {
        format!(
            r#"echo-wasm-abi = {{ path = "{}", default-features = false, features = ["alloc"] }}"#,
            wasm_abi_path.display()
        )
    } else {
        format!(
            r#"echo-wasm-abi = {{ path = "{}" }}"#,
            wasm_abi_path.display()
        )
    };
    let serde_dependency = if no_std {
        r#"serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }"#
    } else {
        r#"serde = { version = "1.0", features = ["derive"] }"#
    };
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "echo-wesley-gen-basic-smoke-{label}"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[dependencies]
{registry_dependency}
{wasm_abi_dependency}
{serde_dependency}
"#,
        ),
    )
    .expect("failed to write generated Cargo.toml");
    fs::write(crate_dir.join("src/generated.rs"), generated)
        .expect("failed to write generated module");
    let lib = if no_std {
        r"#![no_std]
extern crate alloc;

mod generated;
"
    } else {
        "mod generated;\n"
    };
    fs::write(crate_dir.join("src/lib.rs"), lib).expect("failed to write generated lib.rs");
    crate_dir
}

fn write_optic_binding_smoke_crate() -> PathBuf {
    let workspace = workspace_root();
    let crate_dir = workspace
        .join("target")
        .join("echo-wesley-gen-optic-binding-smoke")
        .join(std::process::id().to_string());
    if crate_dir.exists() {
        fs::remove_dir_all(&crate_dir).expect("failed to remove old optic smoke crate");
    }
    fs::create_dir_all(crate_dir.join("src")).expect("failed to create optic smoke crate");

    let wasm_abi_path = workspace.join("crates/echo-wasm-abi");
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "echo-wesley-gen-optic-binding-smoke"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[dependencies]
echo-wasm-abi = {{ path = "{}" }}
serde = {{ version = "1.0", features = ["derive"] }}
"#,
            wasm_abi_path.display()
        ),
    )
    .expect("failed to write optic smoke Cargo.toml");

    fs::write(
        crate_dir.join("src/generated.rs"),
        r"
use echo_wasm_abi::kernel_port::{
    AdmissionLawId, AttachmentDescentPolicy, DispatchOpticIntentRequest, EchoCoordinate,
    IntentFamilyId, ObserveOpticRequest, OpticAperture, OpticApertureShape, OpticCapability,
    OpticCause, OpticFocus, OpticId, OpticIntentPayload, OpticReadBudget, ProjectionVersion,
    ReducerVersion,
};
use echo_wasm_abi::pack_intent_v1;

pub const OP_INCREMENT: u32 = 1001;
pub const OP_COUNTER_VALUE: u32 = 1002;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IncrementVars {
    pub amount: i32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CounterValueVars {}

#[derive(Debug)]
pub enum GeneratedOpticIntentError {
    EncodeVars(echo_wasm_abi::CanonError),
    PackEnvelope(echo_wasm_abi::EnvelopeError),
}

pub fn encode_increment_vars(vars: &IncrementVars) -> Result<Vec<u8>, echo_wasm_abi::CanonError> {
    echo_wasm_abi::encode_cbor(vars)
}

pub fn encode_counter_value_vars(
    vars: &CounterValueVars,
) -> Result<Vec<u8>, echo_wasm_abi::CanonError> {
    echo_wasm_abi::encode_cbor(vars)
}

fn generated_vars_digest(vars_bytes: &[u8]) -> Vec<u8> {
    let mut digest = vec![0u8; 32];
    for (index, byte) in vars_bytes.iter().enumerate() {
        digest[index % 32] ^= *byte;
    }
    digest
}

pub fn counter_value_observe_optic_request(
    optic_id: OpticId,
    focus: OpticFocus,
    coordinate: EchoCoordinate,
    capability: echo_wasm_abi::kernel_port::OpticCapabilityId,
    projection_version: ProjectionVersion,
    reducer_version: Option<ReducerVersion>,
    budget: OpticReadBudget,
    vars: &CounterValueVars,
) -> Result<ObserveOpticRequest, echo_wasm_abi::CanonError> {
    let vars_bytes = encode_counter_value_vars(vars)?;
    let vars_digest = generated_vars_digest(&vars_bytes);
    Ok(counter_value_observe_optic_request_raw_vars_digest(
        optic_id,
        focus,
        coordinate,
        capability,
        projection_version,
        reducer_version,
        budget,
        vars_digest,
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn counter_value_observe_optic_request_raw_vars_digest(
    optic_id: OpticId,
    focus: OpticFocus,
    coordinate: EchoCoordinate,
    capability: echo_wasm_abi::kernel_port::OpticCapabilityId,
    projection_version: ProjectionVersion,
    reducer_version: Option<ReducerVersion>,
    budget: OpticReadBudget,
    vars_digest: Vec<u8>,
) -> ObserveOpticRequest {
    ObserveOpticRequest {
        optic_id,
        focus,
        coordinate,
        aperture: OpticAperture {
            shape: OpticApertureShape::QueryBytes {
                query_id: OP_COUNTER_VALUE,
                vars_digest,
            },
            budget,
            attachment_descent: AttachmentDescentPolicy::BoundaryOnly,
        },
        projection_version,
        reducer_version,
        capability,
    }
}

pub fn increment_dispatch_optic_intent_request(
    optic_id: OpticId,
    base_coordinate: EchoCoordinate,
    intent_family: IntentFamilyId,
    focus: OpticFocus,
    cause: OpticCause,
    capability: OpticCapability,
    admission_law: AdmissionLawId,
    vars: &IncrementVars,
) -> Result<DispatchOpticIntentRequest, GeneratedOpticIntentError> {
    let vars_bytes = encode_increment_vars(vars).map_err(GeneratedOpticIntentError::EncodeVars)?;
    let bytes =
        pack_intent_v1(OP_INCREMENT, &vars_bytes).map_err(GeneratedOpticIntentError::PackEnvelope)?;
    Ok(DispatchOpticIntentRequest {
        optic_id,
        base_coordinate,
        intent_family,
        focus,
        cause,
        capability,
        admission_law,
        payload: OpticIntentPayload::EintV1 { bytes },
    })
}
",
    )
    .expect("failed to write optic generated module");

    fs::write(
        crate_dir.join("src/lib.rs"),
        r#"
mod generated;

#[cfg(test)]
mod tests {
    use super::generated::{
        counter_value_observe_optic_request, encode_counter_value_vars,
        increment_dispatch_optic_intent_request, CounterValueVars, IncrementVars, OP_COUNTER_VALUE,
        OP_INCREMENT,
    };
    use echo_wasm_abi::kernel_port::{
        AdmissionLawId, CoordinateAt, DispatchOpticIntentRequest, EchoCoordinate, IntentFamilyId,
        ObserveOpticRequest, OpticActorId, OpticCapability, OpticCapabilityId, OpticCause,
        OpticFocus, OpticId, OpticIntentPayload, OpticReadBudget, OpticApertureShape,
        ProjectionVersion, WorldlineId,
    };
    use echo_wasm_abi::{decode_cbor, encode_cbor, unpack_intent_v1};

    #[test]
    fn generated_optic_helpers_build_abi_requests() {
        let worldline_id = WorldlineId::from_bytes([1; 32]);
        let optic_id = OpticId::from_bytes([2; 32]);
        let capability_id = OpticCapabilityId::from_bytes([3; 32]);
        let actor = OpticActorId::from_bytes([4; 32]);
        let intent_family = IntentFamilyId::from_bytes([5; 32]);
        let focus = OpticFocus::Worldline { worldline_id };
        let coordinate = EchoCoordinate::Worldline {
            worldline_id,
            at: CoordinateAt::Frontier,
        };
        let budget = OpticReadBudget {
            max_bytes: Some(4096),
            max_nodes: Some(64),
            max_ticks: Some(8),
            max_attachments: Some(0),
        };

        let query_vars = CounterValueVars {};
        let observe = counter_value_observe_optic_request(
            optic_id,
            focus.clone(),
            coordinate.clone(),
            capability_id,
            ProjectionVersion(1),
            None,
            budget,
            &query_vars,
        )
        .unwrap();
        let decoded: ObserveOpticRequest = decode_cbor(&encode_cbor(&observe).unwrap()).unwrap();
        assert_eq!(decoded, observe);
        assert_eq!(observe.optic_id, optic_id);
        assert!(matches!(
            observe.aperture.shape,
            OpticApertureShape::QueryBytes { query_id, ref vars_digest }
                if query_id == OP_COUNTER_VALUE
                    && vars_digest.len() == 32
                    && vars_digest != &encode_counter_value_vars(&query_vars).unwrap()
        ));

        let capability = OpticCapability {
            capability_id,
            actor,
            issuer_ref: None,
            policy_hash: vec![6; 32],
            allowed_focus: focus.clone(),
            projection_version: ProjectionVersion(1),
            reducer_version: None,
            allowed_intent_family: intent_family,
            max_budget: budget,
        };
        let cause = OpticCause {
            actor,
            cause_hash: vec![7; 32],
            label: Some("generated optic dispatch".into()),
        };
        let dispatch = increment_dispatch_optic_intent_request(
            optic_id,
            coordinate.clone(),
            intent_family,
            focus,
            cause,
            capability,
            AdmissionLawId::from_bytes([8; 32]),
            &IncrementVars { amount: 42 },
        )
        .unwrap();
        let decoded: DispatchOpticIntentRequest =
            decode_cbor(&encode_cbor(&dispatch).unwrap()).unwrap();
        assert_eq!(decoded, dispatch);
        assert_eq!(dispatch.base_coordinate, coordinate);
        let OpticIntentPayload::EintV1 { bytes } = &dispatch.payload;
        let (op_id, vars_bytes) = unpack_intent_v1(bytes).unwrap();
        assert_eq!(op_id, OP_INCREMENT);
        let vars: IncrementVars = decode_cbor(vars_bytes).unwrap();
        assert_eq!(vars.amount, 42);
    }
}
"#,
    )
    .expect("failed to write optic smoke lib.rs");

    crate_dir
}

fn assert_generated_crate_checks(crate_dir: &Path) {
    let output = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(crate_dir.join("Cargo.toml"))
        .output()
        .expect("failed to run generated crate check");

    assert!(
        output.status.success(),
        "generated crate check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_generated_optic_helper_shape_compiles_against_abi() {
    let crate_dir = write_optic_binding_smoke_crate();
    let output = Command::new("cargo")
        .args(["test", "--manifest-path"])
        .arg(crate_dir.join("Cargo.toml"))
        .output()
        .expect("failed to run optic generated smoke crate");

    assert!(
        output.status.success(),
        "optic generated smoke crate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
    assert!(stdout.contains("pub fn propose_set_theme_dispatch_optic_intent_request"));
    assert!(
        !stdout.contains("pub fn set_theme_dispatch_optic_intent_request"),
        "setter-like mutation names must be proposal builders, not set_* helpers"
    );
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
    let output = run_wesley_gen(TOY_COUNTER_IR);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("pub const OP_INCREMENT: u32 = 1001"));
    assert!(stdout.contains("pub const OP_COUNTER_VALUE: u32 = 1002"));
    assert!(stdout.contains("pub static REGISTRY: GeneratedRegistry"));
    assert!(stdout.contains("directives_json:"));
    assert!(stdout.contains("\\\"wes_footprint\\\""));
    assert!(stdout.contains("pub const OP_INCREMENT_FOOTPRINT_READS: &[&str]"));
    assert!(stdout.contains("pub const OP_INCREMENT_FOOTPRINT_WRITES: &[&str]"));
    assert!(stdout.contains("pub const OP_INCREMENT_FOOTPRINT_ARTIFACT_HASH: &str"));
    assert!(stdout.contains("pub const OP_INCREMENT_FOOTPRINT_CERTIFICATE_HASH: &str"));
    assert!(stdout.contains("pub const OP_INCREMENT_FOOTPRINT_CERTIFICATE: FootprintCertificate"));
    assert!(stdout.contains("footprint_certificate: Some(&OP_INCREMENT_FOOTPRINT_CERTIFICATE)"));
    assert!(stdout.contains("footprint_certificate: None"));

    for required in [
        "use echo_wasm_abi::pack_intent_v1;",
        "pub mod __echo_wesley_generated",
        "pub struct IncrementVars",
        "pub struct CounterValueVars",
        "pub fn encode_increment_vars",
        "pub fn encode_counter_value_vars",
        "pub fn pack_increment_intent",
        "pub fn pack_increment_intent_raw_vars",
        "pack_intent_v1(super::OP_INCREMENT",
        "pub fn counter_value_observation_request",
        "pub fn counter_value_observation_request_raw_vars",
        "pub fn counter_value_observe_optic_request",
        "pub fn counter_value_observe_optic_request_raw_vars",
        "pub fn increment_dispatch_optic_intent_request",
        "pub fn increment_dispatch_optic_intent_request_raw_vars",
        "DispatchOpticIntentRequest",
        "ObserveOpticRequest",
        "base_coordinate: EchoCoordinate",
    ] {
        assert!(
            stdout.contains(required),
            "generated toy contract output is missing first-consumer bridge: {required}"
        );
    }
    assert!(
        !stdout.contains("pub fn set_"),
        "generated optic helpers should not expose setter-style function names"
    );
}

#[test]
fn test_query_only_contract_does_not_import_intent_packer() {
    let ir = r#"{
        "ir_version": "echo-ir/v1",
        "schema_sha256": "abc123",
        "codec_id": "cbor-canon-v1",
        "registry_version": 1,
        "types": [],
        "ops": [
            { "kind": "QUERY", "name": "counterValue", "op_id": 222, "args": [], "result_type": "Int" }
        ]
    }"#;

    let output = run_wesley_gen(ir);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("pub fn counter_value_observation_request"));
    assert!(stdout.contains("pub fn counter_value_observe_optic_request"));
    assert!(
        !stdout.contains("pack_intent_v1"),
        "query-only generated code should not import or use EINT packing"
    );
}

#[test]
fn test_operation_vars_type_collision_uses_helper_namespace() {
    let ir = r#"{
        "ir_version": "echo-ir/v1",
        "schema_sha256": "abc123",
        "codec_id": "cbor-canon-v1",
        "registry_version": 1,
        "types": [
            {
                "name": "IncrementVars",
                "kind": "OBJECT",
                "fields": [
                    { "name": "value", "type": "Int", "required": true }
                ]
            }
        ],
        "ops": [
            { "kind": "MUTATION", "name": "increment", "op_id": 111, "args": [], "result_type": "IncrementVars" }
        ]
    }"#;

    let output = run_wesley_gen(ir);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pub struct IncrementVars"));
    assert!(stdout.contains("pub mod __echo_wesley_generated"));
    assert!(stdout.contains("pub use __echo_wesley_generated::"));
    assert_generated_crate_checks(&write_basic_generated_crate(
        stdout.as_ref(),
        "vars-collision",
        false,
    ));
}

#[test]
fn test_generated_intent_error_user_type_does_not_collide_with_helper_error() {
    let ir = r#"{
        "ir_version": "echo-ir/v1",
        "schema_sha256": "abc123",
        "codec_id": "cbor-canon-v1",
        "registry_version": 1,
        "types": [
            {
                "name": "GeneratedIntentError",
                "kind": "OBJECT",
                "fields": [
                    { "name": "message", "type": "String", "required": true }
                ]
            }
        ],
        "ops": [
            { "kind": "MUTATION", "name": "increment", "op_id": 111, "args": [], "result_type": "GeneratedIntentError" }
        ]
    }"#;

    let output = run_wesley_gen(ir);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pub struct GeneratedIntentError"));
    assert!(stdout.contains("pub enum GeneratedIntentError"));
    assert!(stdout.contains("pub mod __echo_wesley_generated"));
    assert_generated_crate_checks(&write_basic_generated_crate(
        stdout.as_ref(),
        "intent-error-collision",
        false,
    ));
}

#[test]
fn test_query_mutation_operation_name_collision_fails_with_clear_diagnostic() {
    let ir = r#"{
        "ir_version": "echo-ir/v1",
        "schema_sha256": "abc123",
        "codec_id": "cbor-canon-v1",
        "registry_version": 1,
        "types": [],
        "ops": [
            { "kind": "MUTATION", "name": "value", "op_id": 111, "args": [], "result_type": "Int" },
            { "kind": "QUERY", "name": "value", "op_id": 222, "args": [], "result_type": "Int" }
        ]
    }"#;

    let output = run_wesley_gen(ir);
    assert!(
        !output.status.success(),
        "generator should reject duplicate generated operation item names"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("generated Rust item name collision"));
    assert!(stderr.contains("OP_VALUE"));
}

#[test]
fn test_toy_contract_generated_output_compiles_in_consumer_crate() {
    let output = run_wesley_gen(TOY_COUNTER_IR);
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
fn test_toy_contract_no_std_generated_output_checks_in_consumer_crate() {
    let output = run_wesley_gen_with_args(TOY_COUNTER_IR, &["--no-std"]);
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let generated = String::from_utf8_lossy(&output.stdout);
    assert!(generated.contains("extern crate alloc;"));
    assert!(generated.contains("pub mod __echo_wesley_generated"));
    assert!(generated.contains("use alloc::vec::Vec;"));
    assert!(generated.contains("pub fn counter_value_observe_optic_request"));
    assert!(generated.contains("pub fn increment_dispatch_optic_intent_request"));
    assert_generated_crate_checks(&write_basic_generated_crate(
        generated.as_ref(),
        "toy-no-std",
        true,
    ));
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
