// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Serious external-consumer-shaped contract fixture.
#![cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use echo_cas::{MemoryTier, RetainedBlobIndex, RetainedBlobRole, SemanticBlobCoordinate};
use echo_registry_api::{
    ArgDef, ContractArtifactVerificationPolicy, ObjectDef, OpDef, OpKind, RegistryInfo,
    RegistryProvider,
};
use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, AuthoredObserverPlan,
    ContractEvidenceIdentity, ContractInverseAdmissionRequest, ContractInverseHandler,
    ContractInverseIntent, ContractMutationHandler, ContractOperationKind, ContractPackageIdentity,
    ContractQueryObserver, ContractQueryObserverResult, EngineBuilder, GraphStore, GraphView,
    InboxPolicy, IngressEnvelope, IngressTarget, IntentOutcome, NodeId, NodeRecord, ObservationAt,
    ObservationCoordinate, ObservationFrame, ObservationPayload, ObservationProjection,
    ObservationReadBudget, ObservationRequest, ObserverPlanId, OpticAdmissionTicket,
    OpticArtifactHandle, PatternGraph, PlaybackMode, SchedulerKind, TickDelta,
    TickReceiptRejection, TrustedRuntimeHost, TrustedRuntimeWalConfig, WarpOp, WorldlineId,
    WorldlineRuntime, WorldlineState, WorldlineTick, WriterHead, WriterHeadKey,
    OPTIC_ADMISSION_TICKET_KIND, OPTIC_ARTIFACT_HANDLE_KIND,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

const SCHEMA_SHA256_HEX: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ARTIFACT_HASH_HEX: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const REPLACE_RANGE_OP_ID: u32 = 7101;
const DOCUMENT_WINDOW_QUERY_ID: u32 = 7102;
const REPLACE_VARS_A: &[u8] = b"doc=alpha;range=0..5;text=hello";
const REPLACE_VARS_B: &[u8] = b"doc=alpha;range=0..5;text=hullo";
const REPLACE_VARS_C: &[u8] = b"doc=alpha;range=0..5;text=hallo";
const QUERY_VARS: &[u8] = b"doc=alpha;window=0..16";
const QUERY_BYTES: &[u8] = b"doc=alpha;window=hello";
const DOCUMENT_NODE_LABEL: &str = "external-consumer/jedit-like/document-alpha";
const DOCUMENT_VALUE_TYPE: &str = "external-consumer/jedit-like/document-value";
const REPLACE_RULE_NAME: &str =
    "cmd/contract/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/7101/replaceRange";
const REPLACE_RULE_ID_LABEL: &str =
    "rule:cmd/contract/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/7101/replaceRange";

static REPLACE_ARGS: &[ArgDef] = &[ArgDef {
    name: "input",
    ty: "ReplaceRangeInput",
    required: true,
    list: false,
}];

static OPS: &[OpDef] = &[
    OpDef {
        kind: OpKind::Mutation,
        name: "replaceRange",
        op_id: REPLACE_RANGE_OP_ID,
        args: REPLACE_ARGS,
        result_ty: "DocumentEditReceipt",
        directives_json: "{}",
        footprint_certificate: None,
    },
    OpDef {
        kind: OpKind::Query,
        name: "documentWindow",
        op_id: DOCUMENT_WINDOW_QUERY_ID,
        args: REPLACE_ARGS,
        result_ty: "DocumentWindow",
        directives_json: "{}",
        footprint_certificate: None,
    },
];

struct StaticRegistry;

impl RegistryProvider for StaticRegistry {
    fn info(&self) -> RegistryInfo {
        RegistryInfo {
            echo_abi_version: 1,
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: SCHEMA_SHA256_HEX,
            wesley_generator_version: "echo-wesley-gen/0.1.0",
            helper_api_version: 1,
        }
    }

    fn op_by_id(&self, op_id: u32) -> Option<&'static OpDef> {
        OPS.iter().find(|op| op.op_id == op_id)
    }

    fn all_ops(&self) -> &'static [OpDef] {
        OPS
    }

    fn all_enums(&self) -> &'static [echo_registry_api::EnumDef] {
        &[]
    }

    fn all_objects(&self) -> &'static [ObjectDef] {
        &[]
    }
}

fn empty_engine() -> warp_core::Engine {
    let mut store = GraphStore::default();
    let root = make_node_id("root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );
    EngineBuilder::new(store, root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build()
}

fn document_node_id() -> NodeId {
    make_node_id(DOCUMENT_NODE_LABEL)
}

fn replace_matches(view: GraphView<'_>, scope: &NodeId) -> bool {
    warp_core::eint_vars_for_op(view, scope, REPLACE_RANGE_OP_ID).is_some()
}

fn replace_execute(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let Some(vars) = warp_core::eint_vars_for_op(view, scope, REPLACE_RANGE_OP_ID) else {
        return;
    };
    let warp_id = view.warp_id();
    let document = document_node_id();
    delta.push(WarpOp::UpsertNode {
        node: warp_core::NodeKey {
            warp_id,
            local_id: document,
        },
        record: NodeRecord {
            ty: make_type_id(DOCUMENT_VALUE_TYPE),
        },
    });
    delta.push(WarpOp::SetAttachment {
        key: warp_core::AttachmentKey::node_alpha(warp_core::NodeKey {
            warp_id,
            local_id: document,
        }),
        value: Some(warp_core::AttachmentValue::Atom(
            warp_core::AtomPayload::new(
                make_type_id(DOCUMENT_VALUE_TYPE),
                bytes::Bytes::copy_from_slice(vars),
            ),
        )),
    });
}

fn replace_footprint(view: GraphView<'_>, scope: &NodeId) -> warp_core::Footprint {
    let mut footprint = warp_core::runtime_ingress_eint_read_footprint(view, scope);
    let warp_id = view.warp_id();
    let document = document_node_id();
    footprint.n_write.insert_with_warp(warp_id, document);
    footprint
        .a_write
        .insert(warp_core::AttachmentKey::node_alpha(warp_core::NodeKey {
            warp_id,
            local_id: document,
        }));
    footprint
}

fn replace_rule() -> warp_core::RewriteRule {
    warp_core::RewriteRule {
        id: make_type_id(REPLACE_RULE_ID_LABEL).0,
        name: REPLACE_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: replace_matches,
        executor: replace_execute,
        compute_footprint: replace_footprint,
        factor_mask: 0,
        conflict_policy: warp_core::ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn document_window_observer() -> ContractQueryObserver {
    ContractQueryObserver::new(DOCUMENT_WINDOW_QUERY_ID, observer_plan(), |context| {
        assert_eq!(context.vars_bytes, QUERY_VARS);
        Ok(ContractQueryObserverResult::complete(QUERY_BYTES.to_vec()))
    })
}

fn replace_inverse_handler() -> ContractInverseHandler {
    ContractInverseHandler::new(REPLACE_RANGE_OP_ID, |context| {
        if context.policy_bytes == b"missing-inverse-fragment" {
            return Err(
                warp_core::ContractInverseHandlerError::inverse_fragment_unavailable(
                    "fixture inverse fragment is unavailable",
                ),
            );
        }
        if context.target_vars_bytes != REPLACE_VARS_B {
            return Err(
                warp_core::ContractInverseHandlerError::causal_span_unmappable(
                    "fixture only maps the second replacement",
                ),
            );
        }
        Ok(ContractInverseIntent::new(
            REPLACE_RANGE_OP_ID,
            REPLACE_VARS_A.to_vec(),
        ))
    })
}

fn observer_plan() -> AuthoredObserverPlan {
    AuthoredObserverPlan {
        plan_id: ObserverPlanId::from_bytes([31; 32]),
        artifact_hash: [32; 32],
        schema_hash: [33; 32],
        state_schema_hash: [34; 32],
        update_law_hash: [35; 32],
        emission_law_hash: [36; 32],
    }
}

fn package() -> warp_core::InstalledContractPackage<'static> {
    package_with_identity(ContractPackageIdentity {
        package_name: "jedit-shaped-hot-text",
        package_version: "0.1.0",
        artifact_hash_hex: ARTIFACT_HASH_HEX,
    })
}

fn package_without_inverse() -> warp_core::InstalledContractPackage<'static> {
    let mut package = package();
    package.inverse_handlers.clear();
    package
}

fn package_with_identity(
    identity: ContractPackageIdentity<'static>,
) -> warp_core::InstalledContractPackage<'static> {
    static REGISTRY: StaticRegistry = StaticRegistry;
    warp_core::InstalledContractPackage {
        identity,
        registry: &REGISTRY,
        verification_policy: ContractArtifactVerificationPolicy {
            echo_abi_version: 1,
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: SCHEMA_SHA256_HEX,
            wesley_generator_version: "echo-wesley-gen/0.1.0",
            helper_api_version: 1,
            footprint_certificates: &[],
            require_mutation_footprint_certificates: false,
        },
        mutation_handlers: vec![ContractMutationHandler {
            op_id: REPLACE_RANGE_OP_ID,
            rule: replace_rule(),
        }],
        inverse_handlers: vec![replace_inverse_handler()],
        query_observers: vec![document_window_observer()],
    }
}

fn runtime() -> (WorldlineRuntime, WorldlineId) {
    let mut runtime = WorldlineRuntime::new();
    let worldline_id = WorldlineId::from_bytes([1; 32]);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .expect("worldline should register");
    runtime
        .register_writer_head(WriterHead::with_routing(
            WriterHeadKey {
                worldline_id,
                head_id: make_head_id("default"),
            },
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("writer head should register");
    (runtime, worldline_id)
}

fn replace_envelope(worldline_id: WorldlineId, vars: &[u8]) -> IngressEnvelope {
    IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("echo.intent/eint-v1"),
        echo_wasm_abi::pack_intent_v1(REPLACE_RANGE_OP_ID, vars).expect("EINT should pack"),
    )
}

fn query_request(worldline_id: WorldlineId) -> ObservationRequest {
    let mut request = ObservationRequest::builtin_one_shot(
        ObservationCoordinate {
            worldline_id,
            at: ObservationAt::Frontier,
        },
        ObservationFrame::QueryView,
        ObservationProjection::Query {
            query_id: DOCUMENT_WINDOW_QUERY_ID,
            vars_bytes: QUERY_VARS.to_vec(),
        },
    )
    .expect("query request should build");
    request.budget = ObservationReadBudget::Bounded {
        max_payload_bytes: 128,
        max_witness_refs: 1,
    };
    request
}

fn admission_ticket(seed: u8) -> OpticAdmissionTicket {
    OpticAdmissionTicket {
        kind: OPTIC_ADMISSION_TICKET_KIND.to_owned(),
        artifact_handle: OpticArtifactHandle {
            kind: OPTIC_ARTIFACT_HANDLE_KIND.to_owned(),
            id: format!("external-consumer-{seed}"),
        },
        artifact_hash: format!("artifact-hash-{seed}"),
        operation_id: format!("operation-{seed}"),
        requirements_digest: format!("requirements-{seed}"),
        canonical_variables_digest: vec![seed],
        basis_request_digest: [seed; 32],
        aperture_request_digest: [seed.wrapping_add(1); 32],
        budget_request_digest: [seed.wrapping_add(2); 32],
        law_witness_digest: [seed.wrapping_add(3); 32],
        ticket_digest: [seed.wrapping_add(4); 32],
    }
}

fn semantic_coordinate(
    contract: &ContractEvidenceIdentity,
    role: RetainedBlobRole,
    semantic_digest: [u8; 32],
) -> SemanticBlobCoordinate {
    SemanticBlobCoordinate {
        namespace: contract.package_name.clone(),
        schema_hash_hex: contract.schema_sha256_hex.clone(),
        artifact_hash_hex: contract.artifact_hash_hex.clone(),
        role,
        semantic_digest,
    }
}

fn temp_runtime_wal_dir(label: &str) -> PathBuf {
    let root = PathBuf::from("target").join("warp-core-test-tmp");
    fs::create_dir_all(&root).expect("test temp root should be created");
    temp_runtime_wal_dir_in(&root, label, &TEMP_COUNTER)
}

fn temp_runtime_wal_dir_in(root: &Path, label: &str, counter: &AtomicU64) -> PathBuf {
    for _ in 0..1024 {
        let unique = counter.fetch_add(1, Ordering::Relaxed);
        let dir = root.join(format!("echo-contract-inverse-{label}-{unique}"));
        match fs::create_dir(&dir) {
            Ok(()) => return dir,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => panic!(
                "failed to create deterministic test directory {}: {error}",
                dir.display()
            ),
        }
    }
    panic!("exhausted deterministic test directory attempts for {label}");
}

#[test]
fn temp_runtime_wal_dir_preserves_colliding_directory() {
    let root = temp_runtime_wal_dir("collision-fixture-root");
    let label = "collision";
    let collision = root.join(format!("echo-contract-inverse-{label}-0"));
    fs::create_dir(&collision).expect("collision directory should be created");
    let marker = collision.join("owner-marker");
    fs::write(&marker, b"owned elsewhere").expect("collision marker should be written");

    let allocated = temp_runtime_wal_dir_in(&root, label, &AtomicU64::new(0));
    let collision_preserved = marker.exists();
    fs::remove_dir_all(&allocated).expect("allocated test directory should be removable");
    if collision.exists() {
        fs::remove_dir_all(&collision).expect("collision test directory should be removable");
    }
    fs::remove_dir(&root).expect("collision fixture root should be removable");

    assert!(collision_preserved, "collision owner marker was deleted");
}

fn document_value(host: &TrustedRuntimeHost, worldline_id: WorldlineId) -> Vec<u8> {
    let state = host
        .runtime()
        .worldlines()
        .get(&worldline_id)
        .expect("worldline should exist")
        .state();
    let store = state
        .store(&state.root().warp_id)
        .expect("worldline root store should exist");
    let Some(warp_core::AttachmentValue::Atom(payload)) =
        store.node_attachment(&document_node_id())
    else {
        panic!("document attachment should exist");
    };
    payload.bytes.to_vec()
}

#[test]
fn inverse_intent_resolves_one_admitted_transition_after_restart() {
    let wal_root = temp_runtime_wal_dir("restart");
    let (initial_runtime, worldline_id) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("filesystem WAL should initialize");
    host.register_contract_package(package())
        .expect("external package should install");

    let first = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(replace_envelope(worldline_id, REPLACE_VARS_A))
            .expect("first replacement should be durably witnessed")
    };
    host.stage_installed_contract_submission(first.submission_id, &admission_ticket(50))
        .expect("first replacement should stage");
    host.run_until_idle(4)
        .expect("first replacement should apply");
    let first_receipt_ref = host
        .runtime()
        .receipt_correlation_for_submission(&first.submission_id)
        .expect("first replacement should have retained receipt evidence")
        .causal_receipt_ref;

    let second = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(replace_envelope(worldline_id, REPLACE_VARS_B))
            .expect("second replacement should be durably witnessed")
    };
    host.stage_installed_contract_submission(second.submission_id, &admission_ticket(51))
        .expect("second replacement should stage");
    host.run_until_idle(4)
        .expect("second replacement should apply");
    let target_receipt_ref = host
        .runtime()
        .receipt_correlation_for_submission(&second.submission_id)
        .expect("second replacement should have retained receipt evidence")
        .causal_receipt_ref;
    let third = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(replace_envelope(worldline_id, REPLACE_VARS_C))
            .expect("third replacement should be durably witnessed")
    };
    host.stage_installed_contract_submission(third.submission_id, &admission_ticket(52))
        .expect("third replacement should stage");
    host.run_until_idle(4)
        .expect("third replacement should apply");
    let current_basis_receipt_ref = host
        .runtime()
        .receipt_correlation_for_submission(&third.submission_id)
        .expect("third replacement should have retained receipt evidence")
        .causal_receipt_ref;
    assert_eq!(document_value(&host, worldline_id), REPLACE_VARS_C);
    assert_eq!(
        host.runtime()
            .worldlines()
            .get(&worldline_id)
            .expect("worldline should exist")
            .frontier_tick(),
        WorldlineTick::from_raw(3)
    );
    drop(host);

    let (reconstructed_runtime, _) = runtime();
    let mut mismatched = TrustedRuntimeHost::new(reconstructed_runtime, empty_engine())
        .expect("mismatched host should initialize");
    mismatched
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("mismatched host should recover causal history");
    mismatched
        .register_contract_package(package_with_identity(ContractPackageIdentity {
            package_name: "jedit-shaped-hot-text",
            package_version: "0.2.0",
            artifact_hash_hex: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        }))
        .expect("different contract artifact should install into a fresh host");
    let mismatch = {
        let mut app = mismatched.app();
        app.submit_contract_inverse_with_runtime_wal_ack(ContractInverseAdmissionRequest {
            target_receipt_ref,
            current_target: IngressTarget::DefaultWriter { worldline_id },
            expected_current_frontier_tick: WorldlineTick::from_raw(3),
            policy_bytes: b"exact-span-or-obstruct".to_vec(),
        })
        .expect_err("different contract artifact must not invert retained history")
    };
    assert!(matches!(
        mismatch,
        warp_core::TrustedRuntimeHostError::ContractInverse(
            warp_core::ContractInverseObstruction::ContractVersionMismatch { .. }
        )
    ));
    drop(mismatched);

    let (reconstructed_runtime, _) = runtime();
    let mut unsupported = TrustedRuntimeHost::new(reconstructed_runtime, empty_engine())
        .expect("unsupported host should initialize");
    unsupported
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("unsupported host should recover causal history");
    unsupported
        .register_contract_package(package_without_inverse())
        .expect("contract without inverse law should install");
    let unavailable = {
        let mut app = unsupported.app();
        app.submit_contract_inverse_with_runtime_wal_ack(ContractInverseAdmissionRequest {
            target_receipt_ref,
            current_target: IngressTarget::DefaultWriter { worldline_id },
            expected_current_frontier_tick: WorldlineTick::from_raw(3),
            policy_bytes: b"exact-span-or-obstruct".to_vec(),
        })
        .expect_err("missing inverse law must obstruct")
    };
    assert!(matches!(
        unavailable,
        warp_core::TrustedRuntimeHostError::ContractInverse(
            warp_core::ContractInverseObstruction::InverseHandlerUnavailable { .. }
        )
    ));
    drop(unsupported);

    let (reconstructed_runtime, _) = runtime();
    let mut reconstructed = TrustedRuntimeHost::new(reconstructed_runtime, empty_engine())
        .expect("reconstructed host should initialize");
    reconstructed
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("reconstructed host should recover causal history");
    reconstructed
        .register_contract_package(package())
        .expect("matching contract artifact should reinstall");
    assert_eq!(document_value(&reconstructed, worldline_id), REPLACE_VARS_C);

    let unmappable = {
        let mut app = reconstructed.app();
        app.submit_contract_inverse_with_runtime_wal_ack(ContractInverseAdmissionRequest {
            target_receipt_ref: first_receipt_ref,
            current_target: IngressTarget::DefaultWriter { worldline_id },
            expected_current_frontier_tick: WorldlineTick::from_raw(3),
            policy_bytes: b"exact-span-or-obstruct".to_vec(),
        })
        .expect_err("contract-defined unmappable span must obstruct")
    };
    assert!(matches!(
        unmappable,
        warp_core::TrustedRuntimeHostError::ContractInverse(
            warp_core::ContractInverseObstruction::Contract(
                warp_core::ContractInverseHandlerError::CausalSpanUnmappable { .. }
            )
        )
    ));

    let missing_fragment = {
        let mut app = reconstructed.app();
        app.submit_contract_inverse_with_runtime_wal_ack(ContractInverseAdmissionRequest {
            target_receipt_ref,
            current_target: IngressTarget::DefaultWriter { worldline_id },
            expected_current_frontier_tick: WorldlineTick::from_raw(3),
            policy_bytes: b"missing-inverse-fragment".to_vec(),
        })
        .expect_err("unavailable inverse fragment must obstruct")
    };
    assert!(matches!(
        missing_fragment,
        warp_core::TrustedRuntimeHostError::ContractInverse(
            warp_core::ContractInverseObstruction::Contract(
                warp_core::ContractInverseHandlerError::InverseFragmentUnavailable { .. }
            )
        )
    ));

    let mut forged_receipt_ref = target_receipt_ref;
    forged_receipt_ref.commit_hash = [0; 32];
    let forged = {
        let mut app = reconstructed.app();
        app.submit_contract_inverse_with_runtime_wal_ack(ContractInverseAdmissionRequest {
            target_receipt_ref: forged_receipt_ref,
            current_target: IngressTarget::DefaultWriter { worldline_id },
            expected_current_frontier_tick: WorldlineTick::from_raw(3),
            policy_bytes: b"exact-span-or-obstruct".to_vec(),
        })
        .expect_err("forged receipt coordinate must not resolve")
    };
    assert!(matches!(
        forged,
        warp_core::TrustedRuntimeHostError::ContractInverse(
            warp_core::ContractInverseObstruction::TargetReceiptUnavailable { .. }
        )
    ));

    let stale = {
        let mut app = reconstructed.app();
        app.submit_contract_inverse_with_runtime_wal_ack(ContractInverseAdmissionRequest {
            target_receipt_ref,
            current_target: IngressTarget::DefaultWriter { worldline_id },
            expected_current_frontier_tick: WorldlineTick::from_raw(2),
            policy_bytes: b"exact-span-or-obstruct".to_vec(),
        })
        .expect_err("stale frontier must obstruct inverse admission")
    };
    assert!(matches!(
        stale,
        warp_core::TrustedRuntimeHostError::ContractInverse(
            warp_core::ContractInverseObstruction::CurrentBasisMismatch { .. }
        )
    ));
    let provenance_before = reconstructed
        .provenance()
        .tip_ref(worldline_id)
        .expect("provenance lookup should succeed")
        .expect("recovered provenance tip should exist");

    let inverse = {
        let mut app = reconstructed.app();
        app.submit_contract_inverse_with_runtime_wal_ack(ContractInverseAdmissionRequest {
            target_receipt_ref,
            current_target: IngressTarget::DefaultWriter { worldline_id },
            expected_current_frontier_tick: WorldlineTick::from_raw(3),
            policy_bytes: b"exact-span-or-obstruct".to_vec(),
        })
        .expect("contract inverse should resolve from recovered causal evidence")
    };
    reconstructed
        .stage_installed_contract_submission(inverse.submission_id, &admission_ticket(53))
        .expect("inverse replacement should stage normally");
    reconstructed
        .run_until_idle(4)
        .expect("inverse replacement should apply normally");

    assert_eq!(document_value(&reconstructed, worldline_id), REPLACE_VARS_A);
    let provenance_after = reconstructed
        .provenance()
        .tip_ref(worldline_id)
        .expect("provenance lookup should succeed")
        .expect("inverse provenance tip should exist");
    assert_eq!(
        provenance_before.worldline_tick.checked_increment(),
        Some(provenance_after.worldline_tick)
    );
    assert_eq!(
        reconstructed
            .runtime()
            .worldlines()
            .get(&worldline_id)
            .expect("worldline should exist")
            .frontier_tick(),
        WorldlineTick::from_raw(4)
    );
    assert!(reconstructed
        .runtime()
        .receipt_correlation_for_submission(&second.submission_id)
        .is_some());
    let mut expected_parents = vec![target_receipt_ref, current_basis_receipt_ref];
    expected_parents.sort_unstable();
    assert_eq!(
        reconstructed
            .runtime()
            .receipt_correlation_for_submission(&inverse.submission_id)
            .expect("inverse should retain receipt evidence")
            .causal_parent_receipts,
        expected_parents
    );

    let inverse_receipt_ref = reconstructed
        .runtime()
        .receipt_correlation_for_submission(&inverse.submission_id)
        .expect("inverse should retain its own receipt evidence")
        .causal_receipt_ref;
    drop(reconstructed);

    let (history_runtime, _) = runtime();
    let mut history = TrustedRuntimeHost::new(history_runtime, empty_engine())
        .expect("history host should initialize");
    history
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("history host should recover inverse evidence");
    let derivation = {
        let app = history.app();
        assert_eq!(
            app.contract_inverse_derivation(&target_receipt_ref)
                .expect("ordinary edit history should remain readable"),
            None
        );
        app.contract_inverse_derivation(&inverse_receipt_ref)
            .expect("inverse history should remain readable")
            .expect("inverse receipt should retain a typed derivation")
    };
    assert_eq!(derivation.inverse_receipt_ref, inverse_receipt_ref);
    assert_eq!(derivation.target_receipt_ref, target_receipt_ref);
    assert_eq!(
        derivation.current_basis_receipt_refs,
        vec![current_basis_receipt_ref]
    );

    fs::remove_dir_all(wal_root).expect("test WAL directory should be removable");
}

#[test]
fn inverse_intent_refuses_a_rejected_target_receipt() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::in_memory())
        .expect("in-memory WAL should initialize");
    host.register_contract_package(package())
        .expect("external package should install");

    let (first, second) = {
        let mut app = host.app();
        let first = app
            .submit_intent_with_runtime_wal_ack(replace_envelope(worldline_id, REPLACE_VARS_A))
            .expect("first replacement should be witnessed");
        let second = app
            .submit_intent_with_runtime_wal_ack(replace_envelope(worldline_id, REPLACE_VARS_B))
            .expect("second replacement should be witnessed");
        (first, second)
    };
    host.stage_installed_contract_submission(first.submission_id, &admission_ticket(60))
        .expect("first replacement should stage");
    host.stage_installed_contract_submission(second.submission_id, &admission_ticket(61))
        .expect("second replacement should stage");
    host.run_until_idle(4)
        .expect("one replacement should apply and one should be rejected");

    let rejected_receipt_ref = [first.submission_id, second.submission_id]
        .into_iter()
        .find_map(
            |submission_id| match host.runtime().observe_app_intent_outcome(&submission_id) {
                IntentOutcome::Rejected { receipt, .. } => Some(receipt.causal_receipt_ref),
                _ => None,
            },
        )
        .expect("one conflicting replacement should have a rejected receipt");
    let current_frontier_tick = host
        .runtime()
        .worldlines()
        .get(&worldline_id)
        .expect("worldline should exist")
        .frontier_tick();

    let error = {
        let mut app = host.app();
        app.submit_contract_inverse_with_runtime_wal_ack(ContractInverseAdmissionRequest {
            target_receipt_ref: rejected_receipt_ref,
            current_target: IngressTarget::DefaultWriter { worldline_id },
            expected_current_frontier_tick: current_frontier_tick,
            policy_bytes: b"exact-span-or-obstruct".to_vec(),
        })
        .expect_err("a rejected mutation did not change state and cannot be inverted")
    };
    assert!(matches!(
        error,
        warp_core::TrustedRuntimeHostError::ContractInverse(
            warp_core::ContractInverseObstruction::TargetReceiptNotApplied {
                target_receipt_ref,
            }
        ) if *target_receipt_ref == rejected_receipt_ref
    ));
}

#[test]
fn serious_external_consumer_fixture_proves_hosted_contract_path() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.register_contract_package(package())
        .expect("external package should install");

    let (submission_a, submission_b) = {
        let mut app = host.app();
        let submission_a = app
            .submit_intent(replace_envelope(worldline_id, REPLACE_VARS_A))
            .expect("first edit should submit");
        let submission_b = app
            .submit_intent(replace_envelope(worldline_id, REPLACE_VARS_B))
            .expect("conflicting edit should submit");
        assert!(matches!(
            app.observe_intent_outcome(&submission_a.submission_id),
            IntentOutcome::Pending { .. }
        ));
        (submission_a, submission_b)
    };

    host.stage_installed_contract_submission(submission_a.submission_id, &admission_ticket(41))
        .expect("first edit should stage");
    host.stage_installed_contract_submission(submission_b.submission_id, &admission_ticket(42))
        .expect("second edit should stage");
    host.run_until_idle(4)
        .expect("trusted host should tick until idle");

    let outcome_a = host
        .runtime()
        .observe_app_intent_outcome(&submission_a.submission_id);
    let outcome_b = host
        .runtime()
        .observe_app_intent_outcome(&submission_b.submission_id);
    let rejected = [&outcome_a, &outcome_b]
        .into_iter()
        .filter_map(|outcome| match outcome {
            IntentOutcome::Rejected {
                reason, blocked_by, ..
            } => Some((*reason, blocked_by.as_slice())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].0, TickReceiptRejection::FootprintConflict);
    assert_eq!(rejected[0].1, &[0]);

    let reading = {
        let app = host.app();
        app.observe(query_request(worldline_id))
            .expect("external query should observe")
    };
    assert!(matches!(
        reading.payload,
        ObservationPayload::QueryBytes(bytes) if bytes == QUERY_BYTES
    ));
    let query_contract = reading
        .reading
        .contract
        .as_ref()
        .expect("external reading should carry contract evidence");
    assert_eq!(query_contract.op_kind, ContractOperationKind::Query);
    assert_eq!(query_contract.package_name, "jedit-shaped-hot-text");

    let mut blobs = MemoryTier::new();
    let mut retained = RetainedBlobIndex::default();
    let reading_identity = reading
        .reading
        .query_identity
        .as_ref()
        .expect("external reading should carry query identity");
    let reading_coord = semantic_coordinate(
        query_contract,
        RetainedBlobRole::ReadingPayload,
        reading_identity.reading_id,
    );
    let reading_descriptor = retained
        .retain(&mut blobs, reading_coord.clone(), QUERY_BYTES)
        .expect("reading payload should retain");
    assert_eq!(reading_descriptor.byte_len, QUERY_BYTES.len() as u64);
    assert_eq!(
        retained
            .load_range(&blobs, &reading_coord, 4, 9, 9)
            .expect("semantic retained reading range should load")
            .bytes
            .as_ref(),
        b"alpha;win"
    );

    let receipt_correlation = host
        .runtime()
        .receipt_correlation_for_submission(&submission_a.submission_id)
        .or_else(|| {
            host.runtime()
                .receipt_correlation_for_submission(&submission_b.submission_id)
        })
        .expect("one external edit should carry receipt correlation");
    let receipt_contract = receipt_correlation
        .contract
        .as_ref()
        .and_then(warp_core::InstalledInvocationEvidence::legacy_contract)
        .expect("external legacy receipt should carry legacy contract evidence");
    assert_eq!(receipt_contract.op_kind, ContractOperationKind::Mutation);
    let receipt_coord = semantic_coordinate(
        receipt_contract,
        RetainedBlobRole::ContractReceipt,
        receipt_correlation.tick_receipt_digest,
    );
    let receipt_descriptor = retained
        .retain(&mut blobs, receipt_coord, b"external-jedit-shaped-receipt")
        .expect("receipt evidence should retain");
    assert_eq!(receipt_descriptor.byte_len, 29);
}
