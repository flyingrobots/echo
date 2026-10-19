// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Reference trusted runtime host loop tests.
#![cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use echo_registry_api::{
    ArgDef, ContractArtifactVerificationPolicy, ObjectDef, OpDef, OpKind, RegistryInfo,
    RegistryProvider,
};
use warp_core::{
    causal_wal::{
        canonical_segment_path, recover_in_memory_store, recover_receipt_index,
        recover_submission_index, FilesystemWalFaultPlan, FilesystemWalFaultTarget,
        FilesystemWalStore, Lsn, RecoveredSubmissionPosture, RecoveryAccessMode,
        RecoveryTailPosture, WalBuildError, WalDurabilityMode, WalManifest, WalRecoveryError,
        WalSegmentId, WalStoreError, WalStorePort, WalTransactionKind, WriterEpochId,
        WriterEpochRequest,
    },
    make_head_id, make_intent_kind, make_node_id, make_type_id, AuthoredObserverPlan,
    CausalAnchorAdmissionRequest, CausalAnchorAppRootRole, CausalAnchorCasRole,
    CausalAnchorPurpose, CausalAnchorRoot, CausalAnchorRootSupportGrant,
    CausalAnchorRootSupportPolicy, CausalAnchorSubject, CausalAnchorSupportError,
    CausalFrontierRef, CausalTickReceiptRef, ContinuumOccurrence, ContractMutationHandler,
    ContractOperationKind, ContractPackageIdentity, ContractQueryObserver,
    ContractQueryObserverResult, EngineBuilder, GlobalTick, GraphStore, GraphView, Hash,
    InboxPolicy, IngressCausalParent, IngressEnvelope, IngressTarget, IntentOutcome, NodeId,
    NodeRecord, ObservationAt, ObservationCoordinate, ObservationFrame, ObservationPayload,
    ObservationProjection, ObservationReadBudget, ObservationRequest, ObserverPlanId,
    OpticAdmissionTicket, OpticArtifactHandle, PatternGraph, PlaybackMode, ProvenanceStore,
    RuntimeError, RuntimeWalActivationGap, SchedulerKind, TickDelta, TrustedRuntimeHost,
    TrustedRuntimeHostError, TrustedRuntimeWal, TrustedRuntimeWalConfig, TrustedRuntimeWalError,
    TrustedRuntimeWalStoreKind, WarpOp, WorldlineId, WorldlineRuntime, WorldlineState,
    WorldlineTick, WriterHead, WriterHeadKey, CAUSAL_ANCHOR_SCHEMA_VERSION,
    OPTIC_ADMISSION_TICKET_KIND, OPTIC_ARTIFACT_HANDLE_KIND,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

const SCHEMA_SHA256_HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MUTATION_OP_ID: u32 = 6001;
const QUERY_OP_ID: u32 = 6002;
const MUTATION_VARS: &[u8] = b"amount=7";
const QUERY_VARS: &[u8] = b"window=frontier";
const RESULT_TYPE: &str = "test/reference-host/result";
const RESULT_BYTES: &[u8] = b"value=7";
const QUERY_BYTES: &[u8] = b"window-value=7";
const MUTATION_RULE_NAME: &str =
    "cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/6001/increment";
const MUTATION_RULE_ID_LABEL: &str =
    "rule:cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/6001/increment";

fn deterministic_test_dir(prefix: &str, label: &str) -> PathBuf {
    let root = PathBuf::from("target").join("warp-core-test-tmp");
    fs::create_dir_all(&root).expect("test temp root should be created");
    for _ in 0..1024 {
        let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = root.join(format!("{prefix}-{label}-{unique}"));
        match fs::create_dir(&dir) {
            Ok(()) => return dir,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                fs::remove_dir_all(&dir).expect("stale test dir should be removable");
                match fs::create_dir(&dir) {
                    Ok(()) => return dir,
                    Err(retry_error) if retry_error.kind() == ErrorKind::AlreadyExists => {}
                    Err(retry_error) => panic!(
                        "failed to recreate deterministic test directory {}: {retry_error}",
                        dir.display()
                    ),
                }
            }
            Err(error) => panic!(
                "failed to create deterministic test directory {}: {error}",
                dir.display()
            ),
        }
    }
    panic!("exhausted deterministic test directory attempts for {prefix}-{label}");
}

fn temp_runtime_wal_dir(label: &str) -> PathBuf {
    deterministic_test_dir("echo-trusted-runtime-wal", label)
}

fn filesystem_wal_failure_digest(label: &str) -> Hash {
    blake3::hash(format!("trusted-runtime-wal-failure:{label}").as_bytes()).into()
}

fn causal_receipt_ref(worldline_id: WorldlineId, label: &str, tick: u64) -> CausalTickReceiptRef {
    CausalTickReceiptRef {
        worldline_id,
        worldline_tick_after: WorldlineTick::from_raw(tick),
        commit_global_tick: GlobalTick::from_raw(tick),
        commit_hash: filesystem_wal_failure_digest(&format!("{label}:commit")),
        submission_id: filesystem_wal_failure_digest(&format!("{label}:submission")),
        ticket_digest: filesystem_wal_failure_digest(&format!("{label}:ticket")),
        receipt_content_digest: filesystem_wal_failure_digest(&format!("{label}:content")),
    }
}

fn filesystem_wal_failure_epoch_id() -> WriterEpochId {
    WriterEpochId::from_hash(filesystem_wal_failure_digest("epoch"))
}

fn filesystem_wal_failure_epoch_request() -> WriterEpochRequest {
    WriterEpochRequest {
        epoch_id: filesystem_wal_failure_epoch_id(),
        storage_fencing_token: filesystem_wal_failure_digest("fence"),
        process_identity: filesystem_wal_failure_digest("process"),
        host_identity: filesystem_wal_failure_digest("host"),
        started_at_lsn: Lsn::from_raw(0),
        previous_epoch_id: None,
        previous_epoch_final_commit_digest: None,
        lease_or_lock_evidence: filesystem_wal_failure_digest("lease"),
    }
}

static INCREMENT_ARGS: &[ArgDef] = &[ArgDef {
    name: "input",
    ty: "IncrementInput",
    required: true,
    list: false,
}];

static OPS: &[OpDef] = &[
    OpDef {
        kind: OpKind::Mutation,
        name: "increment",
        op_id: MUTATION_OP_ID,
        args: INCREMENT_ARGS,
        result_ty: "CounterValue",
        directives_json: "{}",
        footprint_certificate: None,
    },
    OpDef {
        kind: OpKind::Query,
        name: "counterWindow",
        op_id: QUERY_OP_ID,
        args: INCREMENT_ARGS,
        result_ty: "CounterWindow",
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

fn contract_execute(
    view: &mut warp_core::ExecutionGraphView<'_, '_>,
    scope: &NodeId,
    delta: &mut TickDelta,
) {
    if warp_core::observed_eint_vars_for_op(view, scope, MUTATION_OP_ID) != Some(MUTATION_VARS) {
        return;
    }
    let warp_id = view.warp_id();
    let result = result_node_id(scope);
    delta.push(WarpOp::UpsertNode {
        node: warp_core::NodeKey {
            warp_id,
            local_id: result,
        },
        record: NodeRecord {
            ty: make_type_id(RESULT_TYPE),
        },
    });
    delta.push(WarpOp::SetAttachment {
        key: warp_core::AttachmentKey::node_alpha(warp_core::NodeKey {
            warp_id,
            local_id: result,
        }),
        value: Some(warp_core::AttachmentValue::Atom(
            warp_core::AtomPayload::new(
                make_type_id(RESULT_TYPE),
                bytes::Bytes::copy_from_slice(RESULT_BYTES),
            ),
        )),
    });
}

fn contract_matches(view: GraphView<'_>, scope: &NodeId) -> bool {
    warp_core::eint_vars_for_op(view, scope, MUTATION_OP_ID) == Some(MUTATION_VARS)
}

fn contract_footprint(view: GraphView<'_>, scope: &NodeId) -> warp_core::Footprint {
    let mut footprint = warp_core::runtime_ingress_eint_read_footprint(view, scope);
    let warp_id = view.warp_id();
    let result = result_node_id(scope);
    footprint.n_write.insert_with_warp(warp_id, result);
    footprint
        .a_write
        .insert(warp_core::AttachmentKey::node_alpha(warp_core::NodeKey {
            warp_id,
            local_id: result,
        }));
    footprint
}

fn contract_rule() -> warp_core::RewriteRule {
    warp_core::RewriteRule {
        id: make_type_id(MUTATION_RULE_ID_LABEL).0,
        name: MUTATION_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: contract_matches,
        executor: warp_core::RuleExecutor::observed(contract_execute),
        compute_footprint: contract_footprint,
        factor_mask: 0,
        conflict_policy: warp_core::ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn query_observer() -> ContractQueryObserver {
    ContractQueryObserver::new(QUERY_OP_ID, observer_plan(), |context| {
        assert_eq!(context.vars_bytes, QUERY_VARS);
        Ok(ContractQueryObserverResult::complete(QUERY_BYTES.to_vec()))
    })
}

fn observer_plan() -> AuthoredObserverPlan {
    AuthoredObserverPlan {
        plan_id: ObserverPlanId::from_bytes([11; 32]),
        artifact_hash: [12; 32],
        schema_hash: [13; 32],
        state_schema_hash: [14; 32],
        update_law_hash: [15; 32],
        emission_law_hash: [16; 32],
    }
}

fn package() -> warp_core::InstalledContractPackage<'static> {
    static REGISTRY: StaticRegistry = StaticRegistry;
    warp_core::InstalledContractPackage {
        identity: ContractPackageIdentity {
            package_name: "reference-counter",
            package_version: "0.1.0",
            artifact_hash_hex: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        },
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
            op_id: MUTATION_OP_ID,
            rule: contract_rule(),
        }],
        inverse_handlers: vec![],
        query_observers: vec![query_observer()],
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

fn runtime_pair() -> (WorldlineRuntime, WorldlineId, WorldlineId) {
    let mut runtime = WorldlineRuntime::new();
    let first = WorldlineId::from_bytes([1; 32]);
    let second = WorldlineId::from_bytes([2; 32]);
    for (worldline_id, head_label) in [(first, "default-a"), (second, "default-b")] {
        runtime
            .register_worldline(worldline_id, WorldlineState::empty())
            .expect("worldline should register");
        runtime
            .register_writer_head(WriterHead::with_routing(
                WriterHeadKey {
                    worldline_id,
                    head_id: make_head_id(head_label),
                },
                PlaybackMode::Play,
                InboxPolicy::AcceptAll,
                None,
                true,
            ))
            .expect("writer head should register");
    }
    (runtime, first, second)
}

fn eint_envelope(worldline_id: WorldlineId) -> IngressEnvelope {
    IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("echo.intent/eint-v1"),
        echo_wasm_abi::pack_intent_v1(MUTATION_OP_ID, MUTATION_VARS).expect("EINT should pack"),
    )
}

fn causal_eint_envelope(
    worldline_id: WorldlineId,
    causal_parent_receipts: Vec<CausalTickReceiptRef>,
) -> IngressEnvelope {
    IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("echo.intent/eint-v1"),
        echo_wasm_abi::pack_intent_v1(MUTATION_OP_ID, MUTATION_VARS).expect("EINT should pack"),
        causal_parent_receipts
            .into_iter()
            .map(|receipt_ref| IngressCausalParent::TickReceipt { receipt_ref })
            .collect(),
    )
}

fn occurred_eint_envelope(
    worldline_id: WorldlineId,
    occurrence_key: u8,
    applied_digest: Hash,
    vars: &[u8],
) -> IngressEnvelope {
    IngressEnvelope::continuum_occurred_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("echo.intent/eint-v1"),
        echo_wasm_abi::pack_intent_v1(MUTATION_OP_ID, vars).expect("EINT should pack"),
        Vec::new(),
        ContinuumOccurrence::new([occurrence_key; 32], applied_digest),
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
            query_id: QUERY_OP_ID,
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
            id: format!("reference-host-{seed}"),
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

fn causal_anchor_request(basis: CausalFrontierRef, label: &str) -> CausalAnchorAdmissionRequest {
    CausalAnchorAdmissionRequest {
        schema_version: CAUSAL_ANCHOR_SCHEMA_VERSION,
        subject: CausalAnchorSubject::new("jedit", "BufferWorldline", "worldline:main"),
        basis_frontier: basis,
        retained_roots: vec![CausalAnchorRoot::AppSubjectRoot {
            app_id: "jedit".to_owned(),
            subject_kind: "RopeHead".to_owned(),
            id: format!("head:{label}"),
            role: CausalAnchorAppRootRole::Authority,
        }],
        materialization_roots: vec![CausalAnchorRoot::CasObject {
            id: filesystem_wal_failure_digest(&format!("flat-text:{label}")),
            role: CausalAnchorCasRole::Materialization,
        }],
        purpose: CausalAnchorPurpose::UserSave,
    }
}

fn causal_anchor_support_policy(
    requests: &[CausalAnchorAdmissionRequest],
) -> CausalAnchorRootSupportPolicy {
    let mut grants = Vec::new();
    for request in requests {
        grants.extend(
            request
                .retained_roots
                .iter()
                .cloned()
                .map(|root| CausalAnchorRootSupportGrant::retained(request.subject.clone(), root)),
        );
        grants.extend(request.materialization_roots.iter().cloned().map(|root| {
            CausalAnchorRootSupportGrant::materialization(request.subject.clone(), root)
        }));
    }
    CausalAnchorRootSupportPolicy::new(grants)
}

fn result_node_id(scope: &NodeId) -> NodeId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"test.reference-runtime-host.result-node");
    hasher.update(scope.as_bytes());
    NodeId(hasher.finalize().into())
}

#[test]
fn reference_host_loop_keeps_tick_authority_out_of_app_surface() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.register_contract_package(package())
        .expect("host should install package");

    let envelope = eint_envelope(worldline_id);
    let submission = {
        let mut app = host.app();
        let submission = app
            .submit_intent(envelope)
            .expect("app should submit witnessed intent");
        assert_eq!(
            app.observe_intent_outcome(&submission.submission_id),
            IntentOutcome::Pending {
                submission_id: submission.submission_id,
                submission_generation: submission.submission_generation,
                ticketed_ingress_id: None,
            }
        );
        submission
    };

    assert_eq!(host.runtime().ticketed_runtime_ingress_count(), 0);
    assert_eq!(host.runtime().receipt_correlation_count(), 0);

    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(17))
        .expect("trusted host should stage package-supported ticketed ingress");
    assert_eq!(
        host.runtime()
            .observe_app_intent_outcome(&submission.submission_id),
        IntentOutcome::Pending {
            submission_id: submission.submission_id,
            submission_generation: submission.submission_generation,
            ticketed_ingress_id: host
                .runtime()
                .ticketed_runtime_ingress_records()
                .next()
                .map(|record| record.ticketed_ingress_id),
        }
    );

    let report = host
        .run_until_idle(4)
        .expect("trusted host should tick until idle");
    assert_eq!(report.committed_steps, 1);
    assert_eq!(report.scheduler_passes, 2);

    {
        let app = host.app();
        assert!(matches!(
            app.observe_intent_outcome(&submission.submission_id),
            IntentOutcome::Applied { .. }
        ));
        let reading = app
            .observe(query_request(worldline_id))
            .expect("app should observe through host query service");
        assert!(matches!(
            reading.payload,
            ObservationPayload::QueryBytes(bytes) if bytes == QUERY_BYTES
        ));
        let contract = reading
            .reading
            .contract
            .as_ref()
            .expect("installed query reading should carry package evidence");
        assert_eq!(contract.op_id, QUERY_OP_ID);
        assert_eq!(contract.op_kind, ContractOperationKind::Query);
        assert_eq!(contract.package_name, "reference-counter");
    }
}

#[test]
fn trusted_host_admits_installed_contract_submission_without_caller_ticket_authority() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.register_contract_package(package())
        .expect("host should install package");

    let submission = {
        let mut app = host.app();
        app.submit_intent(eint_envelope(worldline_id))
            .expect("app should submit witnessed intent")
    };

    let staged = host
        .admit_installed_contract_submission(submission.submission_id)
        .expect("trusted host should derive admission evidence and stage installed work");
    let admission_digest = match staged {
        warp_core::TicketedRuntimeIngressDisposition::Staged { record, .. }
        | warp_core::TicketedRuntimeIngressDisposition::Duplicate { record } => {
            record.ticket_digest
        }
    };
    assert_ne!(admission_digest, [0; 32]);

    host.run_until_idle(4)
        .expect("trusted host should tick until idle");
    let outcome = {
        let app = host.app();
        app.observe_intent_outcome(&submission.submission_id)
    };
    let IntentOutcome::Applied { receipt, .. } = outcome else {
        panic!("expected applied outcome");
    };
    assert_eq!(receipt.ticket_digest, admission_digest);
    let contract = receipt
        .contract
        .expect("installed mutation receipt should carry package evidence");
    let contract = contract
        .legacy_contract()
        .expect("legacy installed mutation retains legacy contract evidence");
    assert_eq!(contract.op_id, MUTATION_OP_ID);
    assert_eq!(contract.op_kind, ContractOperationKind::Mutation);
    assert_eq!(contract.package_name, "reference-counter");
}

#[test]
fn runtime_wal_ack_submit_commits_acceptance_before_returning_handle() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");

    let envelope = eint_envelope(worldline_id);
    let envelope_digest = envelope.ingress_id();
    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(envelope)
            .expect("runtime WAL ACK submit should return accepted evidence")
    };

    let runtime_wal = host
        .runtime_wal()
        .expect("runtime WAL should stay configured");
    assert_eq!(runtime_wal.submission_acceptance_count(), 1);
    assert_eq!(runtime_wal.commits().len(), 1);

    let mut store = runtime_wal
        .cloned_store()
        .expect("in-memory runtime WAL should expose test store clone");
    let report = recover_in_memory_store(&mut store, RecoveryAccessMode::ReadOnly)
        .expect("committed acceptance should recover");
    let recovered = recover_submission_index(&report)
        .expect("recovered acceptance should index by submission id");
    let entry = recovered
        .get(&submission.submission_id)
        .expect("submission should recover from committed WAL");
    assert_eq!(entry.acceptance.submission_id, submission.submission_id);
    assert_eq!(entry.acceptance.canonical_envelope_digest, envelope_digest);
    assert_eq!(entry.posture, RecoveredSubmissionPosture::AcceptedPending);
}

#[test]
fn runtime_wal_ack_rejects_contract_inverse_target_from_normal_submission() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");

    let envelope = IngressEnvelope::local_intent_with_causal_parents(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("echo.intent/eint-v1"),
        echo_wasm_abi::pack_intent_v1(MUTATION_OP_ID, MUTATION_VARS).expect("EINT should pack"),
        vec![IngressCausalParent::ContractInverseTarget {
            receipt_ref: causal_receipt_ref(worldline_id, "smuggled-inverse-target", 1),
        }],
    );

    let result = host.app().submit_intent_with_runtime_wal_ack(envelope);

    assert!(matches!(
        result,
        Err(TrustedRuntimeHostError::Runtime(error))
            if matches!(*error, RuntimeError::ContractInverseTargetRequiresContractAdmission)
    ));
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should remain configured")
            .submission_acceptance_count(),
        0
    );
}

#[test]
fn runtime_wal_ack_adapter_is_configured_by_trusted_host_boundary() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");

    host.enable_runtime_wal(TrustedRuntimeWalConfig::in_memory())
        .expect("host should own runtime WAL adapter configuration");
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should be configured")
            .store_kind(),
        TrustedRuntimeWalStoreKind::InMemory
    );

    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("app should only see ACK submission surface")
    };

    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should remain host-owned")
            .recover_read_only()
            .expect("runtime WAL should recover through adapter boundary")
            .submissions
            .get(&submission.submission_id)
            .expect("submission should recover")
            .posture,
        RecoveredSubmissionPosture::AcceptedPending
    );
}

#[test]
fn a_second_host_cannot_touch_the_log_of_a_live_writer() {
    let wal_root = temp_runtime_wal_dir("second-host-live-writer");
    let (initial_runtime, worldline_id) = runtime();
    let mut live = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    live.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("the first host takes the writer lease");
    live.register_contract_package(package())
        .expect("host should install package");
    {
        let mut app = live.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("the live writer commits a submission");
    }

    // The live writer is in the middle of a record: the segment ends in bytes
    // that are not yet a whole record. A reader sees an uncommitted tail.
    let segment = canonical_segment_path(&wal_root, WalSegmentId::from_raw(1));
    {
        use std::io::Write as _;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&segment)
            .expect("segment opens for append");
        file.write_all(b"partial")
            .expect("partial record bytes are written");
    }
    let before = fs::read(&segment).expect("segment is readable");

    let (second_runtime, _) = runtime();
    let mut second = TrustedRuntimeHost::new(second_runtime, empty_engine())
        .expect("second trusted host should initialize");
    let error = second
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect_err("a second host must not open a log that has a live writer");

    assert!(
        matches!(
            error,
            TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Store(
                WalStoreError::WriterEpochLeaseUnavailable
            ))
        ),
        "the refusal names the lease: {error:?}"
    );
    // Recovery that drops an uncommitted tail is a write. It must not run
    // before the second host holds the lease, which it never does here.
    assert_eq!(
        fs::read(&segment).expect("segment is readable"),
        before,
        "the live writer's segment is untouched"
    );
    drop(live);
    let _ = fs::remove_dir_all(wal_root);
}

#[test]
fn filesystem_runtime_wal_ack_reconstructs_submission_and_tick_from_root() {
    let wal_root = temp_runtime_wal_dir("ack-recovery");
    let (initial_runtime, worldline_id) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");

    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should be configured")
            .store_kind(),
        TrustedRuntimeWalStoreKind::Filesystem
    );
    host.register_contract_package(package())
        .expect("host should install package");

    let envelope = eint_envelope(worldline_id);
    let envelope_digest = envelope.ingress_id();
    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(envelope)
            .expect("filesystem WAL ACK submit should return after durable acceptance")
    };
    let ticket = admission_ticket(55);
    host.stage_installed_contract_submission(submission.submission_id, &ticket)
        .expect("trusted host should stage package-supported ticketed ingress");
    host.run_until_idle(4)
        .expect("trusted host should tick until idle");

    let outcome = {
        let app = host.app();
        app.observe_intent_outcome(&submission.submission_id)
    };
    let IntentOutcome::Applied { receipt, .. } = outcome else {
        panic!("expected applied outcome");
    };
    let causal_receipt_ref = receipt.causal_receipt_ref;
    drop(host);

    let (reconstructed_runtime, _) = runtime();
    let mut reconstructed_host = TrustedRuntimeHost::new(reconstructed_runtime, empty_engine())
        .expect("reconstructed trusted host should initialize");
    reconstructed_host
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("reconstructed host should reopen filesystem runtime WAL adapter");

    let recovery = reconstructed_host
        .runtime_wal()
        .expect("runtime WAL should be configured on reconstructed host")
        .recover_read_only()
        .expect("filesystem runtime WAL should recover read-only");
    let recovered_submission = recovery
        .submissions
        .get(&submission.submission_id)
        .expect("submission should recover from filesystem runtime WAL");

    assert_eq!(
        recovered_submission.acceptance.submission_id,
        submission.submission_id
    );
    assert_eq!(
        recovered_submission.acceptance.canonical_envelope_digest,
        envelope_digest
    );
    assert_eq!(
        recovered_submission.posture,
        RecoveredSubmissionPosture::DecidedApplied
    );
    assert_eq!(
        recovery
            .receipts
            .receipt_by_submission
            .get(&submission.submission_id),
        Some(&causal_receipt_ref)
    );
    assert_eq!(
        recovery
            .receipts
            .ticket_by_submission
            .get(&submission.submission_id),
        Some(&ticket.ticket_digest)
    );
    assert_eq!(recovery.certificate.committed_transactions_replayed, 2);
    assert_eq!(recovery.certificate.obstruction_count, 0);
    assert_eq!(
        recovery.certificate.recovered_indexes_root,
        recovery
            .recomputed_indexes_root()
            .expect("recovered evidence should reproduce the certificate root")
    );
    assert_eq!(recovery.witnessed_submissions.len(), 1);
    assert!(recovery.missing_submission_envelopes.is_empty());
}

#[test]
fn filesystem_runtime_wal_restores_witnessed_submission_material_after_restart() {
    let wal_root = temp_runtime_wal_dir("submission-material-recovery");
    let (initial_runtime, worldline_id) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");

    let envelope = causal_eint_envelope(
        worldline_id,
        vec![causal_receipt_ref(worldline_id, "retained-parent", 37)],
    );
    let submission = host
        .app()
        .submit_intent_with_runtime_wal_ack(envelope.clone())
        .expect("submission should cross the durable ACK boundary");
    assert_eq!(host.runtime().global_tick().as_u64(), 0);
    drop(host);

    let (reconstructed_runtime, _) = runtime();
    let mut reconstructed_host = TrustedRuntimeHost::new(reconstructed_runtime, empty_engine())
        .expect("reconstructed host should initialize");
    reconstructed_host
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("reopened WAL should restore witnessed submission material");

    let restored = reconstructed_host
        .runtime()
        .witnessed_submission(&submission.submission_id)
        .expect("witnessed submission should survive host restart");
    assert_eq!(restored.ingress_id, envelope.ingress_id());
    let restored_envelope = reconstructed_host
        .runtime()
        .witnessed_submission_envelope(&submission.submission_id)
        .expect("canonical envelope material should survive host restart");
    assert_eq!(restored_envelope.ingress_id(), envelope.ingress_id());
    assert_eq!(
        restored_envelope.causal_parents(),
        envelope.causal_parents()
    );
    assert_eq!(reconstructed_host.runtime().global_tick().as_u64(), 0);

    let duplicate = reconstructed_host
        .app()
        .submit_intent_with_runtime_wal_ack(envelope)
        .expect("restored submission should remain idempotent");
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.submission_id, submission.submission_id);
    assert_eq!(
        reconstructed_host
            .runtime_wal()
            .expect("runtime WAL should remain configured")
            .submission_acceptance_count(),
        1
    );
}

#[test]
fn filesystem_runtime_wal_recovers_receipt_causal_parents_after_host_restart() {
    let wal_root = temp_runtime_wal_dir("causal-parent-recovery");
    let (initial_runtime, worldline_id) = runtime();
    let target_receipt = causal_receipt_ref(worldline_id, "target-receipt", 41);
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    host.register_contract_package(package())
        .expect("host should install package");

    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(causal_eint_envelope(
            worldline_id,
            vec![target_receipt],
        ))
        .expect("causal intent should cross the durable ACK boundary")
    };
    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(58))
        .expect("trusted host should stage package-supported ticketed ingress");
    host.run_until_idle(4)
        .expect("trusted host should tick until idle");

    let outcome = host.app().observe_intent_outcome(&submission.submission_id);
    let IntentOutcome::Applied { receipt, .. } = outcome else {
        panic!("expected applied causal intent outcome");
    };
    assert_eq!(receipt.causal_parent_receipts, vec![target_receipt]);
    let inverse_receipt = receipt.causal_receipt_ref;
    drop(host);

    let (reconstructed_runtime, _) = runtime();
    let mut reconstructed_host = TrustedRuntimeHost::new(reconstructed_runtime, empty_engine())
        .expect("reconstructed trusted host should initialize");
    reconstructed_host
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("reconstructed host should reopen filesystem runtime WAL adapter");

    let recovery = reconstructed_host
        .runtime_wal()
        .expect("runtime WAL should be configured")
        .recover_read_only()
        .expect("filesystem runtime WAL should recover causal receipt ancestry");
    assert_eq!(
        recovery.receipts.causal_parent_receipts(&inverse_receipt),
        [target_receipt].as_slice()
    );
    assert_eq!(
        recovery.receipts.receipts_citing(&target_receipt),
        [inverse_receipt].as_slice()
    );
}

#[test]
fn filesystem_runtime_wal_recovers_replayable_provenance_after_restart() {
    let wal_root = temp_runtime_wal_dir("replayable-provenance-recovery");
    let (initial_runtime, worldline_id) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    host.register_contract_package(package())
        .expect("host should install package");

    let envelope = eint_envelope(worldline_id);
    let ticket = admission_ticket(59);
    let submission = host
        .app()
        .submit_intent_with_runtime_wal_ack(envelope.clone())
        .expect("submission should cross the durable ACK boundary");
    host.stage_installed_contract_submission(submission.submission_id, &ticket)
        .expect("trusted host should stage package-supported ticketed ingress");
    host.run_until_idle(4)
        .expect("trusted host should commit the mutation");
    let expected_entry = host
        .provenance()
        .entry(worldline_id, WorldlineTick::ZERO)
        .expect("committed mutation should have provenance");
    let expected_correlation = host
        .runtime()
        .receipt_correlation_for_submission(&submission.submission_id)
        .expect("committed mutation should have a receipt correlation")
        .clone();
    let expected_frontier = host
        .runtime()
        .worldlines()
        .get(&worldline_id)
        .expect("committed worldline should remain registered");
    let expected_frontier_tick = expected_frontier.frontier_tick();
    let expected_state_root = expected_frontier.state().state_root();
    let expected_outcome = host.app().observe_intent_outcome(&submission.submission_id);
    drop(host);

    let (reconstructed_runtime, _) = runtime();
    let mut reconstructed_host = TrustedRuntimeHost::new(reconstructed_runtime, empty_engine())
        .expect("reconstructed host should initialize");
    reconstructed_host
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("reopened WAL should recover retained provenance");
    let recovery = reconstructed_host
        .runtime_wal()
        .expect("runtime WAL should remain configured")
        .recover_read_only()
        .expect("read-only recovery should decode retained provenance");

    assert_eq!(recovery.provenance_entries, vec![expected_entry]);
    assert_eq!(
        reconstructed_host.runtime().global_tick().as_u64(),
        expected_correlation.commit_global_tick.as_u64()
    );
    let reconstructed_frontier = reconstructed_host
        .runtime()
        .worldlines()
        .get(&worldline_id)
        .expect("reconstructed worldline should remain registered");
    assert_eq!(
        reconstructed_frontier.frontier_tick(),
        expected_frontier_tick
    );
    assert_eq!(
        reconstructed_frontier.state().state_root(),
        expected_state_root
    );
    assert_eq!(
        reconstructed_host
            .app()
            .observe_intent_outcome(&submission.submission_id),
        expected_outcome
    );

    reconstructed_host
        .register_contract_package(package())
        .expect("reconstructed host should reinstall deterministic contract code");
    let duplicate = reconstructed_host
        .app()
        .submit_intent_with_runtime_wal_ack(envelope)
        .expect("recovered causal submission should remain idempotent");
    assert!(duplicate.duplicate);
    assert!(matches!(
        reconstructed_host
            .stage_installed_contract_submission(submission.submission_id, &ticket)
            .expect("recovered ticketed ingress should remain idempotent"),
        warp_core::TicketedRuntimeIngressDisposition::Duplicate { .. }
    ));
    let idle = reconstructed_host
        .run_until_idle(1)
        .expect("duplicate recovered work should not schedule another transition");
    assert_eq!(idle.committed_steps, 0);
    assert_eq!(
        reconstructed_host
            .provenance()
            .len(worldline_id)
            .expect("recovered provenance should remain readable"),
        1
    );
}

#[test]
fn runtime_wal_activation_rejects_process_only_committed_history() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.register_contract_package(package())
        .expect("host should install package");

    let submission = host
        .app()
        .submit_intent(eint_envelope(worldline_id))
        .expect("process-local submission should be accepted");
    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(60))
        .expect("trusted host should stage package-supported ticketed ingress");
    host.run_until_idle(4)
        .expect("process-local runtime should commit the mutation");

    let error = host
        .enable_in_memory_runtime_wal()
        .expect_err("WAL activation must reject already-committed process-only history");
    assert!(matches!(
        error,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
            gap: RuntimeWalActivationGap::WitnessedSubmission,
        })
    ));
}

#[test]
fn filesystem_runtime_wal_ack_reconstructed_host_appends_after_recovery() {
    let wal_root = temp_runtime_wal_dir("append-after-recovery");
    let (initial_runtime, first_worldline, _) = runtime_pair();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");

    let first_submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(first_worldline))
            .expect("first filesystem WAL ACK submit should commit")
    };
    drop(host);

    let (reconstructed_runtime, _, second_worldline) = runtime_pair();
    let mut reconstructed_host = TrustedRuntimeHost::new(reconstructed_runtime, empty_engine())
        .expect("reconstructed trusted host should initialize");
    reconstructed_host
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("reconstructed host should reopen filesystem runtime WAL adapter");
    let second_envelope = eint_envelope(second_worldline);
    let second_digest = second_envelope.ingress_id();
    let second_submission = {
        let mut app = reconstructed_host.app();
        app.submit_intent_with_runtime_wal_ack(second_envelope)
            .expect("reconstructed host should append after recovered WAL cursor")
    };

    let runtime_wal = reconstructed_host
        .runtime_wal()
        .expect("runtime WAL should remain configured");
    let commits = runtime_wal.commits();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].first_lsn, Lsn::from_raw(0));
    assert_eq!(commits[0].last_lsn, Lsn::from_raw(2));
    assert_eq!(commits[1].first_lsn, Lsn::from_raw(3));
    assert_eq!(commits[1].last_lsn, Lsn::from_raw(5));
    assert_eq!(
        commits[1].previous_committed_transaction_digest,
        commits[0].commit_digest
    );

    let recovery = runtime_wal
        .recover_read_only()
        .expect("filesystem runtime WAL should recover after restart append");
    assert_eq!(recovery.certificate.committed_transactions_replayed, 2);
    assert_eq!(
        recovery
            .submissions
            .get(&first_submission.submission_id)
            .expect("first submission should recover")
            .posture,
        RecoveredSubmissionPosture::AcceptedPending
    );
    let recovered_second = recovery
        .submissions
        .get(&second_submission.submission_id)
        .expect("second submission should recover");
    assert_eq!(
        recovered_second.acceptance.canonical_envelope_digest,
        second_digest
    );
    assert_eq!(
        recovered_second.posture,
        RecoveredSubmissionPosture::AcceptedPending
    );
}

#[test]
fn filesystem_runtime_wal_ack_commits_strict_filesystem_durability() {
    let wal_root = temp_runtime_wal_dir("strict-durability");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");

    {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("filesystem WAL ACK submit should commit");
    }

    let commits = host
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .commits();
    assert_eq!(commits.len(), 1);
    assert_eq!(
        commits[0].durability_mode,
        WalDurabilityMode::StrictFilesystem
    );
}

#[test]
fn filesystem_runtime_wal_empty_reopen_does_not_consume_a_log_position() {
    let root = temp_runtime_wal_dir("empty-reopen-continuation");
    let reopen = || {
        let (runtime, first, second) = runtime_pair();
        let mut host = TrustedRuntimeHost::new(runtime, empty_engine()).expect("host");
        host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
            .expect("reopen");
        (host, first, second)
    };
    let (mut first, lane, _) = reopen();
    first
        .app()
        .submit_intent_with_runtime_wal_ack(eint_envelope(lane))
        .expect("first append");
    drop(first);
    let (empty, _, _) = reopen();
    drop(empty);
    let (mut last, _, lane) = reopen();
    last.app()
        .submit_intent_with_runtime_wal_ack(eint_envelope(lane))
        .expect("second append");
    let recovered = last
        .runtime_wal()
        .expect("WAL")
        .recover_read_only()
        .expect("contiguous history");
    assert_eq!(recovered.certificate.committed_transactions_replayed, 2);
    let commits = last.runtime_wal().expect("WAL").commits();
    assert_eq!(commits[1].first_lsn, Lsn::from_raw(3));
}

#[test]
fn observation_change_discovery_retains_intervening_writes_after_values_are_restored() {
    fn toggle_root(view: GraphView<'_>, _: &NodeId, delta: &mut TickDelta) {
        let root = make_node_id("root");
        let current = view.node(&root).expect("root").ty;
        delta.push(WarpOp::UpsertNode {
            node: warp_core::NodeKey {
                warp_id: view.warp_id(),
                local_id: root,
            },
            record: NodeRecord {
                ty: if current == make_type_id("world") {
                    make_type_id("changed")
                } else {
                    make_type_id("world")
                },
            },
        });
    }
    fn footprint(view: GraphView<'_>, scope: &NodeId) -> warp_core::Footprint {
        let mut fp = warp_core::runtime_ingress_eint_read_footprint(view, scope);
        fp.n_read
            .insert_with_warp(view.warp_id(), make_node_id("root"));
        fp.n_write
            .insert_with_warp(view.warp_id(), make_node_id("root"));
        fp
    }
    fn matches(view: GraphView<'_>, scope: &NodeId) -> bool {
        warp_core::eint_vars_for_op(view, scope, MUTATION_OP_ID).is_some()
    }
    let path = temp_runtime_wal_dir("observation-aba");
    let (initial, lane) = runtime();
    let root = *initial
        .worldlines()
        .get(&lane)
        .expect("lane")
        .state()
        .root();
    let head = WriterHeadKey {
        worldline_id: lane,
        head_id: make_head_id("default"),
    };
    let mut host = TrustedRuntimeHost::new(initial, empty_engine()).expect("host");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&path))
        .expect("WAL");
    let mut package = package();
    package.mutation_handlers[0].rule.executor = warp_core::RuleExecutor::legacy(toggle_root);
    package.mutation_handlers[0].rule.matcher = matches;
    package.mutation_handlers[0].rule.compute_footprint = footprint;
    host.register_contract_package(package).expect("package");
    host.retain_echo_operation_observation_v1("watch", head, &[root])
        .expect("observe");
    let unrelated = warp_core::NodeKey {
        warp_id: root.warp_id,
        local_id: make_node_id("unrelated"),
    };
    host.retain_echo_operation_observation_v1("control", head, &[unrelated])
        .expect("control");
    for index in 0..2 {
        let envelope = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id: lane },
            make_intent_kind("echo.intent/eint-v1"),
            echo_wasm_abi::pack_intent_v1(MUTATION_OP_ID, &[index]).expect("EINT"),
        );
        let submission = host
            .app()
            .submit_intent_with_runtime_wal_ack(envelope)
            .expect("submit");
        host.stage_installed_contract_submission(
            submission.submission_id,
            &admission_ticket(59 + index),
        )
        .expect("stage");
        host.run_until_idle(4).expect("execute");
    }
    assert_eq!(
        host.runtime()
            .worldlines()
            .get(&lane)
            .expect("lane")
            .state()
            .store(&root.warp_id)
            .expect("store")
            .node(&root.local_id)
            .expect("root")
            .ty,
        make_type_id("world")
    );
    drop(host);
    let (initial, _) = runtime();
    let mut host = TrustedRuntimeHost::new(initial, empty_engine()).expect("fresh host");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&path))
        .expect("reopen");
    host.runtime_wal()
        .expect("WAL")
        .reset_recover_read_only_call_count_for_test();
    assert_eq!(
        host.echo_operation_observation_changes_v1("watch")
            .expect("changes"),
        vec![root]
    );
    assert_eq!(
        host.echo_operation_observation_change_commits_v1("watch")
            .expect("commits")
            .len(),
        2
    );
    assert!(host
        .echo_operation_observation_changes_v1("control")
        .expect("control changes")
        .is_empty());
    assert_eq!(
        host.runtime_wal()
            .expect("WAL")
            .recover_read_only_call_count_for_test(),
        0,
        "change discovery must reuse the recovered native provenance index"
    );
}

#[test]
fn observed_operation_contexts_recover_without_replacing_original_inputs() {
    let root = temp_runtime_wal_dir("observed-request-context");
    let reopen = || {
        let (runtime, lane, _) = runtime_pair();
        let node = *runtime
            .worldlines()
            .get(&lane)
            .expect("lane")
            .state()
            .root();
        let head = WriterHeadKey {
            worldline_id: lane,
            head_id: make_head_id("default-a"),
        };
        let mut host = TrustedRuntimeHost::new(runtime, empty_engine()).expect("host");
        host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
            .expect("WAL");
        (host, head, node)
    };
    let (mut host, head, node) = reopen();
    let observation = host
        .retain_echo_operation_observation_v1("attempt", head, &[node])
        .expect("retain reading");
    assert_eq!(
        observation.basis().application_basis(),
        warp_core::echo_operation_anchored_node_creation_application_basis_v1(
            node,
            warp_core::EchoOperationAnchoredNodeOccupancyV1::NodeOnly,
        ),
        "the canonical root exists without an alpha attachment"
    );
    let invocation = |value: &[u8]| {
        warp_core::EchoOperationInvocationV1::anchored_node_attachment_create_if_absent_with_application_input(
        warp_core::echo_operation_package_id_v1(b"context-only-test"), "test.context@1.create",
        observation.basis(), [1;32], warp_core::EchoOperationBudgetV1::new(16, 1024, 320),
        node, value.to_vec(), vec![0xa0],
    )
    };
    // Binding is durable before ingress. This test does not claim that the
    // deliberately uninstalled package can be admitted or executed.
    let original = host
        .bind_echo_operation_request_v1("request", "attempt", invocation(b"old"))
        .expect("bind request");
    drop(host);
    let (mut recovered, head, node) = reopen();
    assert_eq!(
        recovered
            .echo_operation_observation_v1("attempt")
            .expect("original reading"),
        observation
    );
    assert_eq!(
        recovered
            .bind_echo_operation_request_v1("request", "attempt", invocation(b"old"))
            .expect("retry"),
        original
    );
    assert!(recovered
        .bind_echo_operation_request_v1("request", "attempt", invocation(b"changed"))
        .is_err());
    assert!(recovered
        .retain_echo_operation_observation_v1("attempt", head, &[node])
        .is_err());
    assert!(recovered.echo_operation_observation_v1("missing").is_err());
    assert_eq!(recovered.runtime_wal().expect("WAL").commits().len(), 2);
}

#[test]
fn a_wal_root_refuses_observations_beyond_its_lifetime_limit_even_after_reopening() {
    const LIMIT: usize = 1024;
    let root = temp_runtime_wal_dir("context-lifetime-limit");
    let reopen = || {
        let (runtime, lane, _) = runtime_pair();
        let node = *runtime
            .worldlines()
            .get(&lane)
            .expect("lane")
            .state()
            .root();
        let head = WriterHeadKey {
            worldline_id: lane,
            head_id: make_head_id("default-a"),
        };
        let mut host = TrustedRuntimeHost::new(runtime, empty_engine()).expect("host");
        host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
            .expect("WAL");
        (host, head, node)
    };
    let (mut host, head, node) = reopen();
    for i in 0..LIMIT {
        host.retain_echo_operation_observation_v1(&format!("attempt-{i}"), head, &[node])
            .expect("within the lifetime limit");
    }
    let refused = host
        .retain_echo_operation_observation_v1("one-more", head, &[node])
        .expect_err("the root has retained its lifetime limit");
    assert!(refused.to_string().contains("never retired"), "{refused}");
    drop(host);

    let (mut reopened, head, node) = reopen();
    assert!(
        reopened
            .retain_echo_operation_observation_v1("one-more", head, &[node])
            .is_err(),
        "reopening does not reset the lifetime limit"
    );
    assert!(
        reopened.echo_operation_observation_v1("attempt-0").is_ok(),
        "retained observations still resolve"
    );
}

fn request_link_fixture(
    root: &std::path::Path,
) -> impl Fn() -> (TrustedRuntimeHost, WriterHeadKey, warp_core::NodeKey) + '_ {
    move || {
        let (runtime, lane, _) = runtime_pair();
        let node = *runtime
            .worldlines()
            .get(&lane)
            .expect("lane")
            .state()
            .root();
        let head = WriterHeadKey {
            worldline_id: lane,
            head_id: make_head_id("default-a"),
        };
        let mut host = TrustedRuntimeHost::new(runtime, empty_engine()).expect("host");
        host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(root))
            .expect("WAL");
        (host, head, node)
    }
}

fn bind_link_request(
    host: &mut TrustedRuntimeHost,
    head: WriterHeadKey,
    node: warp_core::NodeKey,
    request: &str,
) -> Vec<u8> {
    let observation = host
        .retain_echo_operation_observation_v1(request, head, &[node])
        .expect("capture");
    let invocation = warp_core::EchoOperationInvocationV1::anchored_node_attachment_create_if_absent_with_application_input(
        warp_core::echo_operation_package_id_v1(b"context-only-test"), "test.context@1.create",
        observation.basis(), [1;32], warp_core::EchoOperationBudgetV1::new(16, 1024, 320), node, b"value".to_vec(), vec![0xa0]);
    host.bind_echo_operation_request_v1(request, request, invocation)
        .expect("bind")
}

#[test]
fn a_submitted_request_is_durably_linked_to_its_submission() {
    let root = temp_runtime_wal_dir("request-outcome-link");
    let reopen = request_link_fixture(&root);
    let (mut host, head, node) = reopen();
    bind_link_request(&mut host, head, node, "request");
    assert_eq!(
        host.echo_operation_request_outcome_v1("request")
            .expect("query"),
        None,
        "a bound request is not linked until it is submitted"
    );
    let target = warp_core::IngressTarget::ExactHead { key: head };
    let submission = host
        .submit_bound_echo_operation_request_v1("request", target.clone())
        .expect("submit");
    assert_eq!(
        host.submit_bound_echo_operation_request_v1("request", target)
            .expect("retry"),
        submission,
        "retrying returns the same submission"
    );
    host.tick_once().expect("the Action is decided");
    let live = host
        .echo_operation_request_outcome_v1("request")
        .expect("query")
        .expect("linked");
    assert_eq!(live.0, submission);
    drop(host);

    let (recovered, _, _) = reopen();
    assert_eq!(
        recovered
            .echo_operation_request_outcome_v1("request")
            .expect("query"),
        Some(live),
        "the link and its outcome survive reopening"
    );
}

#[test]
fn an_unlinked_acceptance_is_linked_by_retry() {
    let root = temp_runtime_wal_dir("request-outcome-link-retry");
    let reopen = request_link_fixture(&root);
    let (mut host, head, node) = reopen();
    let bytes = bind_link_request(&mut host, head, node, "request");
    let target = warp_core::IngressTarget::ExactHead { key: head };
    // The crash window: the submission is durably accepted, but its link is
    // not yet retained.
    let accepted = host
        .app()
        .submit_intent_with_runtime_wal_ack(
            warp_core::echo_operation_action_envelope_v1(target.clone(), bytes).expect("envelope"),
        )
        .expect("accepted")
        .submission_id;
    drop(host);

    let (mut reopened, _, _) = reopen();
    assert_eq!(
        reopened
            .echo_operation_request_outcome_v1("request")
            .expect("query"),
        None
    );
    assert_eq!(
        reopened
            .submit_bound_echo_operation_request_v1("request", target)
            .expect("retry completes the link"),
        accepted
    );
    assert_eq!(
        reopened
            .echo_operation_request_outcome_v1("request")
            .expect("query")
            .map(|(submission, _)| submission),
        Some(accepted)
    );
}

#[test]
fn a_linked_request_refuses_another_target_and_unknown_requests_are_refused() {
    let root = temp_runtime_wal_dir("request-outcome-link-target");
    let reopen = request_link_fixture(&root);
    let (mut host, head, node) = reopen();
    bind_link_request(&mut host, head, node, "request");
    let submission = host
        .submit_bound_echo_operation_request_v1(
            "request",
            warp_core::IngressTarget::ExactHead { key: head },
        )
        .expect("submit");
    // A target that resolves to the same head is the same submission.
    assert_eq!(
        host.submit_bound_echo_operation_request_v1(
            "request",
            warp_core::IngressTarget::DefaultWriter {
                worldline_id: head.worldline_id,
            },
        )
        .expect("an equivalent target"),
        submission
    );
    // A different head would be a second submission of one request.
    let other_head = WriterHeadKey {
        worldline_id: WorldlineId::from_bytes([2; 32]),
        head_id: make_head_id("default-b"),
    };
    assert!(host
        .submit_bound_echo_operation_request_v1(
            "request",
            warp_core::IngressTarget::ExactHead { key: other_head },
        )
        .is_err());
    assert!(host
        .submit_bound_echo_operation_request_v1(
            "never-bound",
            warp_core::IngressTarget::ExactHead { key: head },
        )
        .is_err());
}

#[test]
fn healthy_contexts_advance_without_reconstructing_the_wal() {
    let root = temp_runtime_wal_dir("context-steady-prefix");
    let (runtime, lane, _) = runtime_pair();
    let node = *runtime
        .worldlines()
        .get(&lane)
        .expect("lane")
        .state()
        .root();
    let head = WriterHeadKey {
        worldline_id: lane,
        head_id: make_head_id("default-a"),
    };
    let mut host = TrustedRuntimeHost::new(runtime, empty_engine()).expect("host");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
        .expect("WAL");
    let before = host
        .runtime_wal()
        .and_then(warp_core::TrustedRuntimeWal::filesystem_recovery_work)
        .expect("filesystem WAL");
    for i in 0..8 {
        let attempt = format!("attempt-{i}");
        let observation = host
            .retain_echo_operation_observation_v1(&attempt, head, &[node])
            .expect("capture");
        assert_eq!(
            host.echo_operation_observation_v1(&attempt)
                .expect("reading"),
            observation
        );
        let invocation = warp_core::EchoOperationInvocationV1::anchored_node_attachment_create_if_absent_with_application_input(
            warp_core::echo_operation_package_id_v1(b"context-only-test"), "test.context@1.create",
            observation.basis(), [1;32], warp_core::EchoOperationBudgetV1::new(16, 1024, 320), node, b"value".to_vec(), vec![0xa0]);
        let original = host
            .bind_echo_operation_request_v1(&attempt, &attempt, invocation.clone())
            .expect("bind");
        assert_eq!(
            host.bind_echo_operation_request_v1(&attempt, &attempt, invocation)
                .expect("retry"),
            original
        );
    }
    let after = host
        .runtime_wal()
        .and_then(warp_core::TrustedRuntimeWal::filesystem_recovery_work)
        .expect("filesystem WAL");
    assert_eq!(
        after, before,
        "healthy operations rescanned committed records"
    );
}

#[test]
fn context_append_failure_reconciles_the_actual_committed_prefix() {
    for target in [
        FilesystemWalFaultTarget::AppendFrame,
        FilesystemWalFaultTarget::FlushCommit,
        FilesystemWalFaultTarget::CommitMarkerSynced,
    ] {
        let root = temp_runtime_wal_dir("context-uncertain-prefix");
        let (runtime, lane, _) = runtime_pair();
        let node = *runtime
            .worldlines()
            .get(&lane)
            .expect("lane")
            .state()
            .root();
        let head = WriterHeadKey {
            worldline_id: lane,
            head_id: make_head_id("default-a"),
        };
        let mut host = TrustedRuntimeHost::new(runtime, empty_engine()).expect("host");
        host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
            .expect("WAL");
        host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
            target,
        ))
        .expect("fault");
        assert!(host
            .retain_echo_operation_observation_v1("uncertain", head, &[node])
            .is_err());
        let committed = target == FilesystemWalFaultTarget::CommitMarkerSynced;
        assert_eq!(
            host.echo_operation_observation_v1("uncertain").is_ok(),
            committed
        );
        let retry = host.retain_echo_operation_observation_v1("uncertain", head, &[node]);
        assert_eq!(
            retry.is_err(),
            committed,
            "durable observation cannot be replaced"
        );
        host.retain_echo_operation_observation_v1("next", head, &[node])
            .expect("reconciled append");
        let report = host
            .runtime_wal()
            .expect("WAL")
            .recover_read_only()
            .expect("valid retained prefix");
        assert_eq!(report.certificate.committed_transactions_replayed, 2);
        assert_eq!(host.runtime_wal().expect("WAL").commits().len(), 2);
    }
}

#[test]
fn durable_request_error_resolves_original_bytes_before_retrying() {
    let root = temp_runtime_wal_dir("request-uncertain-prefix");
    let reopen = || {
        let (runtime, lane, _) = runtime_pair();
        let node = *runtime
            .worldlines()
            .get(&lane)
            .expect("lane")
            .state()
            .root();
        let head = WriterHeadKey {
            worldline_id: lane,
            head_id: make_head_id("default-a"),
        };
        let mut host = TrustedRuntimeHost::new(runtime, empty_engine()).expect("host");
        host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
            .expect("WAL");
        (host, head, node)
    };
    let (mut host, head, node) = reopen();
    let observation = host
        .retain_echo_operation_observation_v1("attempt", head, &[node])
        .expect("observation");
    let invocation = |value: &[u8]| {
        warp_core::EchoOperationInvocationV1::anchored_node_attachment_create_if_absent_with_application_input(
            warp_core::echo_operation_package_id_v1(b"context-only-test"), "test.context@1.create",
            observation.basis(), [1;32], warp_core::EchoOperationBudgetV1::new(16, 1024, 320), node, value.to_vec(), vec![0xa0])
    };
    host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
        FilesystemWalFaultTarget::CommitMarkerSynced,
    ))
    .expect("fault");
    assert!(host
        .bind_echo_operation_request_v1("request", "attempt", invocation(b"original"))
        .is_err());
    // The caller did not see success, but the semantic identity is already fixed.
    assert!(host
        .bind_echo_operation_request_v1("request", "attempt", invocation(b"replacement"))
        .is_err());
    let scans = host
        .runtime_wal()
        .and_then(warp_core::TrustedRuntimeWal::filesystem_recovery_work)
        .expect("filesystem WAL");
    let bytes = host
        .bind_echo_operation_request_v1("request", "attempt", invocation(b"original"))
        .expect("resolve durable request");
    assert_eq!(
        scans,
        host.runtime_wal()
            .and_then(warp_core::TrustedRuntimeWal::filesystem_recovery_work)
            .expect("filesystem WAL")
    );
    assert_eq!(host.runtime_wal().expect("WAL").commits().len(), 2);
    drop(host);
    let (mut recovered, _, _) = reopen();
    assert_eq!(
        recovered
            .bind_echo_operation_request_v1("request", "attempt", invocation(b"original"))
            .expect("original after reopen"),
        bytes
    );
    assert!(recovered
        .bind_echo_operation_request_v1("request", "attempt", invocation(b"replacement"))
        .is_err());
    assert_eq!(recovered.runtime_wal().expect("WAL").commits().len(), 2);
}

#[test]
fn filesystem_runtime_wal_ack_recovery_reports_uncommitted_tail_from_root() {
    let wal_root = temp_runtime_wal_dir("tail-report");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    let envelope = eint_envelope(worldline_id);
    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(envelope.clone())
            .expect("filesystem WAL ACK submit should commit")
    };
    drop(host);

    let mut raw_wal =
        TrustedRuntimeWal::from_config(TrustedRuntimeWalConfig::filesystem(&wal_root))
            .expect("filesystem WAL should reopen for test tail append");
    raw_wal
        .append_uncommitted_submission_acceptance_for_test(&envelope, submission)
        .expect("test fixture should append uncommitted filesystem tail");

    let recovery = raw_wal
        .recover_read_only()
        .expect("read-only recovery should report uncommitted tail");
    assert_eq!(
        recovery.certificate.tail_posture,
        RecoveryTailPosture::WouldTruncateAfter(Lsn::from_raw(2))
    );
}

#[test]
fn filesystem_runtime_wal_ack_recovery_rejects_corrupt_root() {
    let wal_root = temp_runtime_wal_dir("corrupt-root");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("filesystem WAL ACK submit should commit");
    }
    let runtime_wal = host
        .runtime_wal()
        .expect("runtime WAL should stay configured");
    fs::write(
        canonical_segment_path(&wal_root, WalSegmentId::from_raw(1)),
        b"not-a-valid-runtime-wal-segment",
    )
    .expect("test should corrupt filesystem WAL segment");

    let err = runtime_wal
        .recover_read_only()
        .expect_err("corrupt filesystem WAL should not recover as empty clean history");
    assert!(matches!(
        err,
        TrustedRuntimeWalError::Recovery(
            WalRecoveryError::Store(_) | WalRecoveryError::Validation(_)
        )
    ));
}

#[test]
fn filesystem_runtime_wal_ack_multi_head_tick_rejects_before_partial_filesystem_append() {
    let wal_root = temp_runtime_wal_dir("multi-head-atomic");
    let (runtime, worldline_a, worldline_b) = runtime_pair();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    host.register_contract_package(package())
        .expect("host should install package");

    let submission_a = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_a))
            .expect("first submission acceptance should commit before ACK")
    };
    let submission_b = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_b))
            .expect("second submission acceptance should commit before ACK")
    };
    host.stage_installed_contract_submission(submission_a.submission_id, &admission_ticket(56))
        .expect("trusted host should stage first ticketed ingress");
    host.stage_installed_contract_submission(submission_b.submission_id, &admission_ticket(57))
        .expect("trusted host should stage second ticketed ingress");

    let err = host
        .run_until_idle(4)
        .expect_err("filesystem WAL should reject multi-transaction tick batches for now");
    assert!(matches!(
        err,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::FilesystemAtomicBatchUnsupported {
            transaction_kind: WalTransactionKind::SchedulerTick,
            transaction_count: 2
        })
    ));

    let recovery = host
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .recover_read_only()
        .expect("filesystem WAL should recover accepted submissions");
    assert_eq!(recovery.certificate.committed_transactions_replayed, 2);
    for submission_id in [submission_a.submission_id, submission_b.submission_id] {
        assert_eq!(
            recovery
                .submissions
                .get(&submission_id)
                .expect("submission should remain accepted pending")
                .posture,
            RecoveredSubmissionPosture::AcceptedPending
        );
    }
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .scheduler_tick_count(),
        0
    );
}

#[test]
fn filesystem_runtime_wal_failure_submission_append_rolls_back_pre_ack_visibility() {
    let wal_root = temp_runtime_wal_dir("failure-submission-append");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(
        TrustedRuntimeWalConfig::filesystem_with_fault_plan_for_test(
            &wal_root,
            FilesystemWalFaultPlan::fail_next(FilesystemWalFaultTarget::AppendFrame),
        ),
    )
    .expect("host should configure faulting filesystem runtime WAL adapter");

    let err = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect_err("injected append failure should reject pre-ACK submission")
    };
    assert!(matches!(
        err,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Store(WalStoreError::Io(
            message
        ))) if message.contains("injected filesystem WAL append_frame failure")
    ));
    assert_eq!(host.runtime().witnessed_submission_count(), 0);
    let recovery = host
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .recover_read_only()
        .expect("failed append should leave a clean empty filesystem WAL");
    assert_eq!(recovery.certificate.committed_transactions_replayed, 0);
    assert!(recovery.submissions.is_empty());
}

#[test]
fn filesystem_runtime_wal_failure_submission_flush_rolls_back_pre_ack_visibility() {
    let wal_root = temp_runtime_wal_dir("failure-submission-flush");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(
        TrustedRuntimeWalConfig::filesystem_with_fault_plan_for_test(
            &wal_root,
            FilesystemWalFaultPlan::fail_next(FilesystemWalFaultTarget::FlushCommit),
        ),
    )
    .expect("host should configure faulting filesystem runtime WAL adapter");

    let err = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect_err("injected flush failure should reject pre-ACK submission")
    };
    assert!(matches!(
        err,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Store(WalStoreError::Io(
            message
        ))) if message.contains("injected filesystem WAL flush_commit failure")
    ));
    assert_eq!(host.runtime().witnessed_submission_count(), 0);
    let recovery = host
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .recover_read_only()
        .expect("failed flush should leave no committed submission after repair");
    assert_eq!(recovery.certificate.committed_transactions_replayed, 0);
    assert!(recovery.submissions.is_empty());
}

#[test]
fn filesystem_runtime_wal_failure_tick_append_rolls_back_visible_outcome() {
    let wal_root = temp_runtime_wal_dir("failure-tick-append");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    host.register_contract_package(package())
        .expect("host should install package");

    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("submission acceptance should commit before ACK")
    };
    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(106))
        .expect("trusted host should stage package-supported ticketed ingress");
    host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
        FilesystemWalFaultTarget::AppendFrame,
    ))
    .expect("host should inject filesystem append failure");

    let err = host
        .run_until_idle(4)
        .expect_err("injected append failure should reject tick publication");
    assert!(matches!(
        err,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Store(WalStoreError::Io(
            message
        ))) if message.contains("injected filesystem WAL append_frame failure")
    ));
    assert_eq!(host.runtime().receipt_correlation_count(), 0);
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .scheduler_tick_count(),
        0
    );
    let outcome = {
        let app = host.app();
        app.observe_intent_outcome(&submission.submission_id)
    };
    assert!(matches!(
        outcome,
        IntentOutcome::Pending {
            submission_id,
            ticketed_ingress_id: Some(_),
            ..
        } if submission_id == submission.submission_id
    ));
    let recovery = host
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .recover_read_only()
        .expect("failed tick append should leave only accepted submission evidence");
    assert_eq!(recovery.certificate.committed_transactions_replayed, 1);
    assert_eq!(
        recovery
            .submissions
            .get(&submission.submission_id)
            .expect("submission should remain accepted pending")
            .posture,
        RecoveredSubmissionPosture::AcceptedPending
    );
    assert!(recovery.receipts.receipt_by_submission.is_empty());
}

#[test]
fn filesystem_runtime_wal_failure_tick_flush_rolls_back_visible_outcome() {
    let wal_root = temp_runtime_wal_dir("failure-tick-flush");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    host.register_contract_package(package())
        .expect("host should install package");

    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("submission acceptance should commit before ACK")
    };
    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(107))
        .expect("trusted host should stage package-supported ticketed ingress");
    host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
        FilesystemWalFaultTarget::FlushCommit,
    ))
    .expect("host should inject filesystem flush failure");

    let err = host
        .run_until_idle(4)
        .expect_err("injected flush failure should reject tick publication");
    assert!(matches!(
        err,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Store(WalStoreError::Io(
            message
        ))) if message.contains("injected filesystem WAL flush_commit failure")
    ));
    assert_eq!(host.runtime().receipt_correlation_count(), 0);
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .scheduler_tick_count(),
        0
    );

    let outcome = {
        let app = host.app();
        app.observe_intent_outcome(&submission.submission_id)
    };
    assert!(matches!(
        outcome,
        IntentOutcome::Pending {
            submission_id,
            ticketed_ingress_id: Some(_),
            ..
        } if submission_id == submission.submission_id
    ));
    let recovery = host
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .recover_read_only()
        .expect("failed tick flush should leave only accepted submission evidence");
    assert_eq!(recovery.certificate.committed_transactions_replayed, 1);
    assert_eq!(
        recovery
            .submissions
            .get(&submission.submission_id)
            .expect("submission should remain accepted pending")
            .posture,
        RecoveredSubmissionPosture::AcceptedPending
    );
    assert!(recovery.receipts.receipt_by_submission.is_empty());
}

#[test]
fn filesystem_runtime_wal_failure_tick_restores_exact_runtime_and_provenance() {
    for target in [
        FilesystemWalFaultTarget::AppendFrame,
        FilesystemWalFaultTarget::FlushCommit,
    ] {
        let wal_root = temp_runtime_wal_dir("failure-tick-exact-state");
        let (runtime, worldline_id) = runtime();
        let mut host = TrustedRuntimeHost::new(runtime, empty_engine())
            .expect("trusted host should initialize");
        host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
            .expect("host should configure filesystem runtime WAL adapter");
        host.register_contract_package(package())
            .expect("host should install package");
        let submission = host
            .app()
            .submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("submission acceptance should commit before ACK");
        host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(109))
            .expect("trusted host should stage package-supported ticketed ingress");
        let runtime_before = format!("{:?}", host.runtime());
        let provenance_before = format!("{:?}", host.provenance());
        host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
            target,
        ))
        .expect("host should inject a filesystem failure");

        host.tick_once()
            .expect_err("an undurable tick must not remain visible");

        assert_eq!(
            format!("{:?}", host.runtime()),
            runtime_before,
            "{target:?}: a failed tick must restore the exact pre-tick runtime"
        );
        assert_eq!(
            format!("{:?}", host.provenance()),
            provenance_before,
            "{target:?}: a failed tick must restore the exact pre-tick provenance"
        );
        host.tick_once()
            .expect("the restored tick must commit once the WAL accepts it");
        assert_eq!(host.runtime().receipt_correlation_count(), 1);
        drop(host);
        fs::remove_dir_all(&wal_root).expect("WAL fixture should be removable");
    }
}

#[test]
fn filesystem_runtime_wal_failure_submission_restores_exact_runtime() {
    for target in [
        FilesystemWalFaultTarget::AppendFrame,
        FilesystemWalFaultTarget::FlushCommit,
    ] {
        let wal_root = temp_runtime_wal_dir("failure-submission-exact-state");
        let (runtime, worldline_id) = runtime();
        let mut host = TrustedRuntimeHost::new(runtime, empty_engine())
            .expect("trusted host should initialize");
        host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
            .expect("host should configure filesystem runtime WAL adapter");
        let runtime_before = format!("{:?}", host.runtime());
        host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
            target,
        ))
        .expect("host should inject a filesystem failure");

        host.app()
            .submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect_err("an undurable submission must not be acknowledged");

        assert_eq!(
            format!("{:?}", host.runtime()),
            runtime_before,
            "{target:?}: a failed submission must restore the exact pre-call runtime"
        );
        let retried = host
            .app()
            .submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("the same submission must be acceptable once the WAL accepts it");
        assert!(!retried.duplicate);
        drop(host);
        fs::remove_dir_all(&wal_root).expect("WAL fixture should be removable");
    }
}

#[test]
fn a_submission_whose_commit_sync_failed_is_not_acknowledged() {
    let wal_root = temp_runtime_wal_dir("commit-sync-failed-submission");
    let (initial_runtime, worldline_id) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    let runtime_before = format!("{:?}", host.runtime());
    // The commit record reaches the file and its sync fails. The record is
    // readable. It is not durable.
    host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
        FilesystemWalFaultTarget::CommitMarkerSync,
    ))
    .expect("host should inject a filesystem failure");

    host.app()
        .submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
        .expect_err("a readable record whose sync failed must not be acknowledged");

    assert_eq!(
        format!("{:?}", host.runtime()),
        runtime_before,
        "the failed submission is taken back"
    );
    // The host does not reconcile itself from the log. It refuses further
    // WAL work until the WAL is reopened.
    let error = host
        .app()
        .submit_intent_with_runtime_wal_ack(distinct_intents(worldline_id, 1).remove(0))
        .expect_err("the WAL is failed closed");
    assert!(
        matches!(
            error,
            TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::StorageFailed)
        ),
        "{error:?}"
    );
    drop(host);

    // Reopening syncs the log before trusting it. The record it holds is
    // then durable and recovers as an accepted, pending submission.
    let (reopened_runtime, _) = runtime();
    let mut reopened = TrustedRuntimeHost::new(reopened_runtime, empty_engine())
        .expect("trusted host should initialize");
    reopened
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("the reopened host recovers the log");
    let again = reopened
        .app()
        .submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
        .expect("a reopened WAL accepts work again");
    assert!(
        again.duplicate,
        "the record survived as an accepted submission"
    );
    drop(reopened);
    fs::remove_dir_all(&wal_root).expect("WAL fixture should be removable");
}

#[test]
fn a_tick_whose_commit_sync_failed_is_not_visible() {
    let wal_root = temp_runtime_wal_dir("commit-sync-failed-tick");
    let (initial_runtime, worldline_id) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    host.register_contract_package(package())
        .expect("host should install package");
    let submission = host
        .app()
        .submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
        .expect("submission acceptance should commit before ACK");
    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(109))
        .expect("trusted host should stage package-supported ticketed ingress");
    let runtime_before = format!("{:?}", host.runtime());
    let provenance_before = format!("{:?}", host.provenance());
    host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
        FilesystemWalFaultTarget::CommitMarkerSync,
    ))
    .expect("host should inject a filesystem failure");

    host.tick_once()
        .expect_err("a tick whose commit sync failed must not remain visible");

    assert_eq!(
        format!("{:?}", host.runtime()),
        runtime_before,
        "the failed tick restores the exact pre-tick runtime"
    );
    assert_eq!(
        format!("{:?}", host.provenance()),
        provenance_before,
        "the failed tick restores the exact pre-tick provenance"
    );
    assert_eq!(host.runtime().receipt_correlation_count(), 0);
    let outcome = host.app().observe_intent_outcome(&submission.submission_id);
    assert!(
        matches!(outcome, IntentOutcome::Pending { .. }),
        "no outcome is published for a tick that is not durable: {outcome:?}"
    );
    // The tick is not retried against a log that may hold its record.
    let error = host.tick_once().expect_err("the WAL is failed closed");
    assert!(
        matches!(
            error,
            TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::StorageFailed)
        ),
        "{error:?}"
    );
    drop(host);
    fs::remove_dir_all(&wal_root).expect("WAL fixture should be removable");
}

fn distinct_intents(worldline_id: WorldlineId, count: u8) -> Vec<IngressEnvelope> {
    (0..count)
        .map(|index| {
            IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                make_intent_kind("echo.intent/eint-v1"),
                vec![index; 4],
            )
        })
        .collect()
}

#[test]
fn batched_submission_makes_every_acceptance_durable_with_one_sync() {
    let wal_root = temp_runtime_wal_dir("batched-submission-one-sync");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    let syncs = |host: &TrustedRuntimeHost| {
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .filesystem_commit_sync_count_for_test()
            .expect("filesystem store")
    };

    let single_before = syncs(&host);
    let mut envelopes = distinct_intents(worldline_id, 8);
    let batch = envelopes.split_off(3);
    for envelope in envelopes {
        host.app()
            .submit_intent_with_runtime_wal_ack(envelope)
            .expect("single submission should be acknowledged");
    }
    assert_eq!(
        syncs(&host) - single_before,
        3,
        "each single submission waits for its own sync"
    );

    let batch_before = syncs(&host);
    let handles = host
        .app()
        .submit_intents_with_runtime_wal_ack(batch)
        .expect("batched submission should be acknowledged");
    assert_eq!(handles.len(), 5);
    assert!(handles.iter().all(|handle| !handle.duplicate));
    assert_eq!(
        syncs(&host) - batch_before,
        1,
        "a batch waits for one sync, not one per submission"
    );

    let submission_ids = handles
        .iter()
        .map(|handle| handle.submission_id)
        .collect::<Vec<_>>();
    drop(host);
    let (runtime, _) = crate::runtime();
    let mut reopened =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    reopened
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("a fresh host should recover the batch");
    let recovery = reopened
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .recover_read_only()
        .expect("batched acceptances should recover");
    assert_eq!(recovery.certificate.committed_transactions_replayed, 8);
    for submission_id in submission_ids {
        assert_eq!(
            recovery
                .submissions
                .get(&submission_id)
                .expect("every batched submission should recover")
                .posture,
            RecoveredSubmissionPosture::AcceptedPending
        );
    }
    drop(reopened);
    fs::remove_dir_all(&wal_root).expect("WAL fixture should be removable");
}

#[test]
fn batched_submission_append_failure_fails_closed_until_the_wal_is_reopened() {
    let wal_root = temp_runtime_wal_dir("batched-submission-failure");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    let runtime_before = format!("{:?}", host.runtime());
    // Let the first acceptance commit, unsynced, and fail the second.
    host.inject_runtime_wal_filesystem_fault_for_test(
        FilesystemWalFaultPlan::fail_next(FilesystemWalFaultTarget::FlushCommit).after(1),
    )
    .expect("host should inject a filesystem failure");

    let error = host
        .app()
        .submit_intents_with_runtime_wal_ack(distinct_intents(worldline_id, 4))
        .expect_err("a batch whose append fails must not be acknowledged");
    assert!(matches!(
        error,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Store(WalStoreError::Io(
            message
        ))) if message.contains("injected filesystem WAL flush_commit failure")
    ));
    assert_eq!(
        format!("{:?}", host.runtime()),
        runtime_before,
        "the acceptance appended before the failure must not stay visible"
    );
    // The first acceptance is readable in the log and was never synced. The
    // host must not treat it as an existing durable submission.
    let refused = host
        .app()
        .submit_intent_with_runtime_wal_ack(distinct_intents(worldline_id, 1).remove(0))
        .expect_err("the WAL must refuse work after a failed append in a batch");
    assert!(matches!(
        refused,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::StorageFailed)
    ));
    assert_eq!(format!("{:?}", host.runtime()), runtime_before);

    drop(host);
    let (runtime, _) = crate::runtime();
    let mut reopened =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    reopened
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("a fresh host should recover what storage holds");
    let single = reopened
        .app()
        .submit_intent_with_runtime_wal_ack(distinct_intents(worldline_id, 1).remove(0))
        .expect("a reopened WAL accepts work again");
    assert!(
        single.duplicate,
        "reopening syncs the log before trusting it, so the acceptance it holds is durable"
    );
    drop(reopened);
    fs::remove_dir_all(&wal_root).expect("WAL fixture should be removable");
}

#[test]
fn batched_submission_refused_before_storage_keeps_earlier_intents_and_the_wal() {
    let wal_root = temp_runtime_wal_dir("batched-submission-refused");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    let syncs_before = host
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .filesystem_commit_sync_count_for_test()
        .expect("filesystem store");
    // The first acceptance commits, unsynced. The second is refused before
    // anything is written for it.
    host.refuse_runtime_wal_append_before_storage_for_test(1)
        .expect("host should arm the refusal");

    let error = host
        .app()
        .submit_intents_with_runtime_wal_ack(distinct_intents(worldline_id, 3))
        .expect_err("a batch with a refused intent returns no handles");
    assert!(matches!(
        error,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::UncertainWriterPrefix)
    ));
    assert_eq!(
        host.runtime().witnessed_submission_count(),
        1,
        "the intent before the refusal stays accepted"
    );
    let runtime_wal = host
        .runtime_wal()
        .expect("runtime WAL should stay configured");
    assert_eq!(
        runtime_wal
            .filesystem_commit_sync_count_for_test()
            .expect("filesystem store")
            - syncs_before,
        1,
        "the intent before the refusal is synced"
    );
    assert_eq!(runtime_wal.submission_acceptance_count(), 1);

    // Nothing failed in storage, so the WAL keeps working.
    let mut intents = distinct_intents(worldline_id, 3);
    let first = host
        .app()
        .submit_intent_with_runtime_wal_ack(intents.remove(0))
        .expect("the accepted intent is returned as its existing submission");
    assert!(first.duplicate);
    let rest = host
        .app()
        .submit_intents_with_runtime_wal_ack(intents)
        .expect("the refused intent and its successor are accepted on retry");
    assert_eq!(rest.len(), 2);
    assert!(rest.iter().all(|handle| !handle.duplicate));
    assert_eq!(host.runtime().witnessed_submission_count(), 3);
    drop(host);
    fs::remove_dir_all(&wal_root).expect("WAL fixture should be removable");
}

#[test]
fn batched_submission_refused_before_storage_keeps_an_in_memory_wal_usable() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::in_memory())
        .expect("host should configure in-memory runtime WAL adapter");
    host.refuse_runtime_wal_append_before_storage_for_test(1)
        .expect("host should arm the refusal");

    host.app()
        .submit_intents_with_runtime_wal_ack(distinct_intents(worldline_id, 3))
        .expect_err("a batch with a refused intent returns no handles");
    assert_eq!(host.runtime().witnessed_submission_count(), 1);

    let mut intents = distinct_intents(worldline_id, 3);
    let first = host
        .app()
        .submit_intent_with_runtime_wal_ack(intents.remove(0))
        .expect("the accepted intent is returned as its existing submission");
    assert!(first.duplicate);
    let rest = host
        .app()
        .submit_intents_with_runtime_wal_ack(intents)
        .expect("the writer cursor is rebuilt, so later appends are accepted");
    assert_eq!(rest.len(), 2);
    assert!(rest.iter().all(|handle| !handle.duplicate));
}

#[test]
fn a_host_does_not_open_a_log_it_cannot_sync() {
    let wal_root = temp_runtime_wal_dir("open-sync-failure");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    host.app()
        .submit_intent_with_runtime_wal_ack(distinct_intents(worldline_id, 1).remove(0))
        .expect("the first host accepts a submission");
    drop(host);

    let (runtime, _) = crate::runtime();
    let mut reopened =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    let error = reopened
        .enable_runtime_wal(
            TrustedRuntimeWalConfig::filesystem_with_fault_plan_for_test(
                &wal_root,
                FilesystemWalFaultPlan::fail_next(FilesystemWalFaultTarget::RecoveredSegmentSync),
            ),
        )
        .expect_err("a log that cannot be synced must not be read as recovered history");
    assert!(matches!(
        error,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Store(WalStoreError::Io(
            message
        ))) if message.contains("recovered_segment_sync")
    ));
    assert!(reopened.runtime_wal().is_none());
    assert_eq!(reopened.runtime().witnessed_submission_count(), 0);
    drop(reopened);
    fs::remove_dir_all(&wal_root).expect("WAL fixture should be removable");
}

#[test]
fn batched_submission_sync_failure_rolls_back_intake_and_fails_closed() {
    let wal_root = temp_runtime_wal_dir("batched-submission-sync-failure");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    let runtime_before = format!("{:?}", host.runtime());
    host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
        FilesystemWalFaultTarget::DeferredCommitSync,
    ))
    .expect("host should inject a sync failure");

    let error = host
        .app()
        .submit_intents_with_runtime_wal_ack(distinct_intents(worldline_id, 4))
        .expect_err("a batch whose sync fails must not be acknowledged");
    assert!(matches!(
        error,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Store(WalStoreError::Io(
            message
        ))) if message.contains("injected filesystem WAL deferred_commit_sync failure")
    ));
    assert_eq!(
        format!("{:?}", host.runtime()),
        runtime_before,
        "a failed sync must not leave the batch visible as accepted intake"
    );
    let refused = host
        .app()
        .submit_intent_with_runtime_wal_ack(distinct_intents(worldline_id, 1).remove(0))
        .expect_err("the WAL must refuse work after a failed sync");
    assert!(matches!(
        refused,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::StorageFailed)
    ));
    assert_eq!(
        format!("{:?}", host.runtime()),
        runtime_before,
        "a refused submission must not stay visible either"
    );
    assert!(
        matches!(host.tick_once(), Ok(steps) if steps.is_empty()),
        "nothing from the failed batch may be scheduled"
    );
    let failed_wal = host
        .runtime_wal()
        .expect("runtime WAL should stay configured");
    assert!(
        matches!(
            failed_wal.recover_read_only(),
            Err(TrustedRuntimeWalError::StorageFailed)
        ),
        "the running host must not report readable records as recovered"
    );
    assert!(matches!(
        failed_wal.recover_evidence_catalog_read_only(),
        Err(TrustedRuntimeWalError::StorageFailed)
    ));
    assert!(
        failed_wal.evidence_catalog().is_none(),
        "the live catalog must not describe records that are not durable"
    );
    assert!(matches!(
        failed_wal.evidence_catalog_posture(),
        warp_core::EvidenceCatalogPosture::NeedsRebuild { .. }
    ));
    assert!(
        matches!(
            host.app().current_causal_anchor_basis(),
            Err(TrustedRuntimeHostError::Wal(
                TrustedRuntimeWalError::StorageFailed
            ))
        ),
        "a frontier over records that are not durable is not a basis"
    );

    // The records were written; only their sync failed. Reopening recovers
    // what storage holds, as it would after a process loss.
    drop(host);
    let (runtime, _) = crate::runtime();
    let mut reopened =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    reopened
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("a fresh host should recover what storage holds");
    let recovery = reopened
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .recover_read_only()
        .expect("the written prefix should recover");
    assert_eq!(recovery.certificate.committed_transactions_replayed, 4);
    assert!(recovery.receipts.receipt_by_submission.is_empty());
    drop(reopened);
    fs::remove_dir_all(&wal_root).expect("WAL fixture should be removable");
}

#[test]
fn batched_submission_handles_duplicates_and_an_empty_batch() {
    let wal_root = temp_runtime_wal_dir("batched-submission-duplicates");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    let syncs = |host: &TrustedRuntimeHost| {
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .filesystem_commit_sync_count_for_test()
            .expect("filesystem store")
    };

    let before = syncs(&host);
    let empty = host
        .app()
        .submit_intents_with_runtime_wal_ack(Vec::new())
        .expect("an empty batch is acknowledged without work");
    assert!(empty.is_empty());
    assert_eq!(syncs(&host), before, "an empty batch forces no sync");

    let mut envelopes = distinct_intents(worldline_id, 2);
    envelopes.insert(1, envelopes[0].clone());
    let handles = host
        .app()
        .submit_intents_with_runtime_wal_ack(envelopes)
        .expect("a batch that repeats an envelope should be acknowledged");
    assert_eq!(handles.len(), 3);
    assert!(!handles[0].duplicate);
    assert!(handles[1].duplicate);
    assert_eq!(handles[1].submission_id, handles[0].submission_id);
    assert!(!handles[2].duplicate);
    assert_eq!(syncs(&host) - before, 1);
    let runtime_wal = host
        .runtime_wal()
        .expect("runtime WAL should stay configured");
    assert_eq!(
        runtime_wal.submission_acceptance_count(),
        2,
        "a repeated envelope must not append a second acceptance"
    );
    assert_eq!(runtime_wal.commits().len(), 2);
    drop(host);
    fs::remove_dir_all(&wal_root).expect("WAL fixture should be removable");
}

#[test]
fn filesystem_runtime_wal_failure_manifest_publish_reports_store_error() {
    let wal_root = temp_runtime_wal_dir("failure-manifest-publish");
    let mut store = FilesystemWalStore::open_with_fault_plan_for_test(
        &wal_root,
        WalSegmentId::from_raw(1),
        FilesystemWalFaultPlan::fail_next(FilesystemWalFaultTarget::PublishManifest),
    )
    .expect("faulting filesystem WAL store should open");
    store
        .acquire_writer_epoch(filesystem_wal_failure_epoch_request())
        .expect("test writer epoch should acquire");
    let err = store
        .publish_manifest(
            filesystem_wal_failure_epoch_id(),
            WalManifest {
                manifest_digest: filesystem_wal_failure_digest("manifest"),
                last_committed_lsn: None,
                last_commit_digest: None,
                sealed_segment_count: 1,
            },
        )
        .expect_err("injected manifest failure should be reported");

    assert!(matches!(
        err,
        WalStoreError::Io(message)
            if message.contains("injected filesystem WAL publish_manifest failure")
    ));
    assert!(!wal_root.join("manifest.ecwal").exists());
}

#[test]
fn runtime_wal_ack_duplicate_submit_does_not_append_second_acceptance() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");

    let envelope = eint_envelope(worldline_id);
    let first = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(envelope.clone())
            .expect("first submission should be accepted")
    };
    let duplicate = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(envelope)
            .expect("duplicate submission should be recognized")
    };

    assert!(!first.duplicate);
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.submission_id, first.submission_id);
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .submission_acceptance_count(),
        1
    );
}

#[test]
fn runtime_wal_ack_duplicate_without_prior_wal_backfills_acceptance() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");

    let envelope = eint_envelope(worldline_id);
    let envelope_digest = envelope.ingress_id();
    let first = {
        let mut app = host.app();
        app.submit_intent(envelope.clone())
            .expect("legacy non-WAL submission should be accepted")
    };
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .submission_acceptance_count(),
        0
    );

    let duplicate = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(envelope)
            .expect("WAL ACK duplicate should backfill acceptance evidence")
    };

    assert!(duplicate.duplicate);
    assert_eq!(duplicate.submission_id, first.submission_id);
    let runtime_wal = host
        .runtime_wal()
        .expect("runtime WAL should stay configured");
    assert_eq!(runtime_wal.submission_acceptance_count(), 1);
    let mut store = runtime_wal
        .cloned_store()
        .expect("in-memory runtime WAL should expose test store clone");
    let report = recover_in_memory_store(&mut store, RecoveryAccessMode::ReadOnly)
        .expect("backfilled acceptance should recover");
    let recovered = recover_submission_index(&report)
        .expect("backfilled acceptance should rebuild submission index");
    assert_eq!(
        recovered
            .get(&first.submission_id)
            .expect("submission should recover from committed WAL")
            .acceptance
            .canonical_envelope_digest,
        envelope_digest
    );
}

#[test]
fn runtime_wal_ack_duplicate_ignores_uncommitted_acceptance_frames() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");

    let envelope = eint_envelope(worldline_id);
    let first = {
        let mut app = host.app();
        app.submit_intent(envelope.clone())
            .expect("legacy non-WAL submission should be accepted")
    };
    let mut raw_wal = TrustedRuntimeWal::new_in_memory().expect("test WAL should initialize");
    raw_wal
        .append_uncommitted_submission_acceptance_for_test(&envelope, first)
        .expect("test fixture should append raw acceptance frames");
    host.replace_runtime_wal_for_test(raw_wal);
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .submission_acceptance_count(),
        0
    );

    let duplicate = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(envelope)
            .expect("WAL ACK duplicate should commit recoverable acceptance evidence")
    };

    assert!(duplicate.duplicate);
    assert_eq!(duplicate.submission_id, first.submission_id);
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .submission_acceptance_count(),
        1
    );
}

#[test]
fn runtime_wal_ack_path_requires_configured_runtime_wal() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");

    let err = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect_err("ACK path without WAL should be explicit")
    };

    assert!(matches!(
        err,
        TrustedRuntimeHostError::RuntimeWalUnavailable
    ));
    assert_eq!(host.runtime().witnessed_submission_count(), 0);
}

#[test]
fn runtime_wal_ack_failure_rolls_back_intake_mutation() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    let overflowing_wal = TrustedRuntimeWal::new_in_memory_at_lsn_for_test(Lsn::from_raw(u64::MAX))
        .expect("overflow fixture WAL should initialize");
    host.replace_runtime_wal_for_test(overflowing_wal);

    let err = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect_err("overflowing WAL should reject ACK")
    };

    assert!(matches!(
        err,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Build(WalBuildError::LsnOverflow))
    ));
    assert_eq!(host.runtime().witnessed_submission_count(), 0);
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .submission_acceptance_count(),
        0
    );
}

#[test]
fn runtime_wal_ack_tick_commits_receipt_transaction_before_outcome_is_observed() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");
    host.register_contract_package(package())
        .expect("host should install package");

    let envelope = eint_envelope(worldline_id);
    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(envelope)
            .expect("runtime WAL ACK submit should return accepted evidence")
    };
    let ticket = admission_ticket(91);
    host.stage_installed_contract_submission(submission.submission_id, &ticket)
        .expect("trusted host should stage package-supported ticketed ingress");

    let report = host
        .run_until_idle(4)
        .expect("trusted host should tick until idle");
    assert_eq!(report.committed_steps, 1);

    let outcome = {
        let app = host.app();
        app.observe_intent_outcome(&submission.submission_id)
    };
    let IntentOutcome::Applied { receipt, .. } = outcome else {
        panic!("expected applied outcome");
    };
    let causal_receipt_ref = receipt.causal_receipt_ref;

    let runtime_wal = host
        .runtime_wal()
        .expect("runtime WAL should stay configured");
    assert_eq!(
        runtime_wal
            .commits()
            .into_iter()
            .filter(|commit| commit.transaction_kind == WalTransactionKind::SchedulerTick)
            .count(),
        1
    );

    let mut store = runtime_wal
        .cloned_store()
        .expect("in-memory runtime WAL should expose test store clone");
    let recovery = recover_in_memory_store(&mut store, RecoveryAccessMode::ReadOnly)
        .expect("committed tick receipt should recover");
    let submissions = recover_submission_index(&recovery)
        .expect("recovered tick receipt should update submission posture");
    assert_eq!(
        submissions
            .get(&submission.submission_id)
            .expect("submission should recover")
            .posture,
        RecoveredSubmissionPosture::DecidedApplied
    );

    let receipts =
        recover_receipt_index(&recovery).expect("recovered tick receipt should rebuild index");
    assert_eq!(
        receipts
            .receipt_by_submission
            .get(&submission.submission_id),
        Some(&causal_receipt_ref)
    );
    assert_eq!(
        receipts.ticket_by_submission.get(&submission.submission_id),
        Some(&ticket.ticket_digest)
    );
}

#[test]
fn runtime_wal_ack_tick_failure_rolls_back_visible_outcome() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");
    host.register_contract_package(package())
        .expect("host should install package");

    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("submission acceptance should commit before ACK")
    };
    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(92))
        .expect("trusted host should stage package-supported ticketed ingress");
    let overflowing_wal = TrustedRuntimeWal::new_in_memory_at_lsn_for_test(Lsn::from_raw(u64::MAX))
        .expect("overflow fixture WAL should initialize");
    host.replace_runtime_wal_for_test(overflowing_wal);

    let err = host
        .run_until_idle(4)
        .expect_err("tick WAL overflow should reject publication");
    assert!(matches!(
        err,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Build(WalBuildError::LsnOverflow))
    ));
    assert_eq!(host.runtime().receipt_correlation_count(), 0);
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .scheduler_tick_count(),
        0
    );

    let outcome = {
        let app = host.app();
        app.observe_intent_outcome(&submission.submission_id)
    };
    assert!(matches!(
        outcome,
        IntentOutcome::Pending {
            submission_id,
            ticketed_ingress_id: Some(_),
            ..
        } if submission_id == submission.submission_id
    ));
}

#[test]
fn runtime_wal_ack_multi_head_tick_failure_rolls_back_all_tick_records() {
    let (runtime, worldline_a, worldline_b) = runtime_pair();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");
    host.register_contract_package(package())
        .expect("host should install package");

    let submission_a = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_a))
            .expect("first submission acceptance should commit before ACK")
    };
    let submission_b = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_b))
            .expect("second submission acceptance should commit before ACK")
    };
    host.stage_installed_contract_submission(submission_a.submission_id, &admission_ticket(95))
        .expect("trusted host should stage first ticketed ingress");
    host.stage_installed_contract_submission(submission_b.submission_id, &admission_ticket(96))
        .expect("trusted host should stage second ticketed ingress");

    let overflowing_wal =
        TrustedRuntimeWal::new_in_memory_at_lsn_for_test(Lsn::from_raw(u64::MAX - 5))
            .expect("overflow fixture WAL should initialize");
    host.replace_runtime_wal_for_test(overflowing_wal);

    let err = host
        .run_until_idle(4)
        .expect_err("second tick WAL transaction should fail after first would have committed");
    assert!(matches!(
        err,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Build(WalBuildError::LsnOverflow))
    ));
    assert_eq!(
        host.runtime().receipt_correlation_count(),
        0,
        "failed scheduler pass must not leave receipt correlations visible"
    );
    assert_eq!(
        host.runtime_wal()
            .expect("runtime WAL should stay configured")
            .scheduler_tick_count(),
        0,
        "failed scheduler pass must roll back every tick WAL record from the attempt"
    );

    for submission_id in [submission_a.submission_id, submission_b.submission_id] {
        let outcome = {
            let app = host.app();
            app.observe_intent_outcome(&submission_id)
        };
        assert!(matches!(
            outcome,
            IntentOutcome::Pending {
                submission_id: observed_submission_id,
                ticketed_ingress_id: Some(_),
                ..
            } if observed_submission_id == submission_id
        ));
    }
}

#[test]
fn runtime_wal_ack_recover_read_only_rebuilds_submission_and_receipt_indexes() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");
    host.register_contract_package(package())
        .expect("host should install package");

    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("submission acceptance should commit before ACK")
    };
    let ticket = admission_ticket(93);
    host.stage_installed_contract_submission(submission.submission_id, &ticket)
        .expect("trusted host should stage package-supported ticketed ingress");
    host.run_until_idle(4)
        .expect("trusted host should tick until idle");

    let recovery = host
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .recover_read_only()
        .expect("runtime WAL recovery should rebuild indexes");

    assert_eq!(
        recovery
            .submissions
            .get(&submission.submission_id)
            .expect("submission should recover")
            .posture,
        RecoveredSubmissionPosture::DecidedApplied
    );
    assert_eq!(
        recovery
            .receipts
            .ticket_by_submission
            .get(&submission.submission_id),
        Some(&ticket.ticket_digest)
    );
    assert_eq!(
        recovery.certificate.recovered_indexes_root,
        recovery
            .recomputed_indexes_root()
            .expect("recovered evidence should reproduce the certificate root")
    );
    assert_eq!(recovery.witnessed_submissions.len(), 1);
    assert!(recovery.missing_submission_envelopes.is_empty());
}

#[test]
fn runtime_wal_ack_recover_read_only_exposes_recovery_certificate() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");
    host.register_contract_package(package())
        .expect("host should install package");

    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("submission acceptance should commit before ACK")
    };
    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(94))
        .expect("trusted host should stage package-supported ticketed ingress");
    host.run_until_idle(4)
        .expect("trusted host should tick until idle");

    let recovery = host
        .runtime_wal()
        .expect("runtime WAL should stay configured")
        .recover_read_only()
        .expect("runtime WAL recovery should expose certificate");

    assert_eq!(recovery.certificate.committed_transactions_replayed, 2);
    assert_eq!(recovery.certificate.obstruction_count, 0);
    assert!(recovery.certificate.first_lsn.is_some());
    assert!(recovery.certificate.last_lsn.is_some());
}

use warp_core::EvidenceCatalogPosture;

#[test]
fn runtime_wal_live_evidence_catalog_matches_read_only_recovery_after_submission() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");
    host.register_contract_package(package())
        .expect("host should install package");

    let _submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("submission acceptance should commit before ACK")
    };

    let wal = host.runtime_wal().expect("runtime WAL should exist");

    // Live catalog must exist and be fresh
    let live_catalog = wal.evidence_catalog().expect("live catalog should exist");
    assert_eq!(
        *wal.evidence_catalog_posture(),
        EvidenceCatalogPosture::Fresh,
        "live catalog should be fresh"
    );

    // Recovered catalog from pristine read-only scan
    let recovered_catalog = wal
        .recover_evidence_catalog_read_only()
        .expect("rebuild should succeed");

    assert_eq!(
        live_catalog.segments_by_id.len(),
        1,
        "submission should produce 1 base segment"
    );
    assert_eq!(
        live_catalog.segments_by_id.len(),
        recovered_catalog.segments_by_id.len(),
        "live catalog must match recovered catalog length"
    );

    for (id, live_seg) in &live_catalog.segments_by_id {
        let rec_seg = recovered_catalog
            .segments_by_id
            .get(id)
            .expect("recovered catalog missing segment");
        assert_eq!(
            live_seg.commit_digest, rec_seg.commit_digest,
            "segment commit digest must match"
        );
    }
}

#[test]
fn runtime_wal_live_evidence_catalog_rebuilds_after_recovered_filesystem_ack() {
    let wal_root = temp_runtime_wal_dir("catalog-recovered-filesystem-ack");
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(
        TrustedRuntimeWalConfig::filesystem_with_fault_plan_for_test(
            &wal_root,
            FilesystemWalFaultPlan::fail_next(FilesystemWalFaultTarget::CommitMarkerSynced),
        ),
    )
    .expect("host should configure faulting filesystem runtime WAL adapter");

    let _submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
            .expect("recoverable commit marker failure should still ACK")
    };

    let wal = host.runtime_wal().expect("runtime WAL should exist");
    assert_eq!(
        *wal.evidence_catalog_posture(),
        EvidenceCatalogPosture::Fresh,
        "recovered ACK should leave the live catalog fresh"
    );

    let live_catalog = wal.evidence_catalog().expect("live catalog should exist");
    let recovered_catalog = wal
        .recover_evidence_catalog_read_only()
        .expect("read-only rebuild should succeed");
    assert_eq!(
        recovered_catalog.segments_by_id.len(),
        1,
        "recovered WAL should contain the committed submission segment"
    );
    assert_eq!(
        live_catalog.segments_by_id.len(),
        recovered_catalog.segments_by_id.len(),
        "live catalog must include the recovered committed submission"
    );
}

#[test]
fn runtime_wal_live_evidence_catalog_failure_marks_needs_rebuild_without_failing_commit() {
    let (runtime, worldline_a, worldline_b) = runtime_pair();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");

    let _first = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_a))
            .expect("first submission acceptance should commit before ACK")
    };
    let last_good_commit = host
        .runtime_wal()
        .expect("runtime WAL should exist")
        .commits()
        .last()
        .expect("first commit should exist")
        .commit_digest;

    let mut faulting_wal = host
        .runtime_wal()
        .expect("runtime WAL should exist")
        .cloned_in_memory_for_test()
        .expect("test WAL should be in-memory");
    faulting_wal.fail_next_evidence_catalog_update_for_test();
    host.replace_runtime_wal_for_test(faulting_wal);

    let _second = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(eint_envelope(worldline_b))
            .expect("catalog update failure should not reject committed WAL acceptance")
    };

    let wal = host.runtime_wal().expect("runtime WAL should exist");
    assert_eq!(
        wal.commits().len(),
        2,
        "WAL commit should succeed even when the derived catalog fails"
    );
    assert!(matches!(
        wal.evidence_catalog_posture(),
        EvidenceCatalogPosture::NeedsRebuild {
            last_good_commit: observed,
            ..
        } if *observed == last_good_commit
    ));
}

#[test]
fn causal_anchor_admission_requires_runtime_wal_authority() {
    let (initial_runtime, _) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    let request = causal_anchor_request(CausalFrontierRef::from_digest([0x11; 32]), "no-wal");
    host.install_causal_anchor_root_support_policy(causal_anchor_support_policy(
        std::slice::from_ref(&request),
    ));

    let error = host
        .app()
        .admit_causal_anchor(request)
        .expect_err("application admission must require the trusted runtime WAL");

    assert!(matches!(
        error,
        TrustedRuntimeHostError::RuntimeWalUnavailable
    ));
}

#[test]
fn causal_anchor_basis_advances_for_non_anchor_causal_history() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");
    let before = host
        .app()
        .current_causal_anchor_basis()
        .expect("enabled WAL should expose its durable causal basis");

    host.app()
        .submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
        .expect("submission history should commit before acknowledgement");
    let after = host
        .app()
        .current_causal_anchor_basis()
        .expect("committed submission should expose the advanced basis");

    assert_ne!(before, after);
}

#[test]
fn causal_anchor_admission_rejects_stale_basis_and_unsupported_roots() {
    let (runtime, _) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");
    let original_basis = host
        .app()
        .current_causal_anchor_basis()
        .expect("enabled WAL should expose its durable causal basis");
    let first = causal_anchor_request(original_basis, "first");
    let stale = causal_anchor_request(original_basis, "stale");
    let missing_policy_error = host
        .app()
        .admit_causal_anchor(first.clone())
        .expect_err("application admission must require host-owned root support");
    assert!(matches!(
        missing_policy_error,
        TrustedRuntimeHostError::CausalAnchorSupportPolicyUnavailable
    ));
    host.install_causal_anchor_root_support_policy(causal_anchor_support_policy(&[
        first.clone(),
        stale.clone(),
    ]));

    host.app()
        .admit_causal_anchor(first)
        .expect("supported request at the current basis should be admitted");
    let stale_error = host
        .app()
        .admit_causal_anchor(stale)
        .expect_err("a committed anchor must advance the durable causal basis");
    assert!(matches!(
        stale_error,
        TrustedRuntimeHostError::CausalAnchorBasisStale { .. }
    ));

    let current_basis = host
        .app()
        .current_causal_anchor_basis()
        .expect("committed anchor should expose the advanced basis");
    let unsupported = causal_anchor_request(current_basis, "unsupported");
    let support_error = host
        .app()
        .admit_causal_anchor(unsupported)
        .expect_err("unregistered roots must not acquire Echo authority");
    assert!(matches!(
        support_error,
        TrustedRuntimeHostError::CausalAnchorSupport(
            CausalAnchorSupportError::UnsupportedRoot { .. }
        )
    ));
}

#[test]
fn filesystem_causal_anchor_admission_recovers_by_id_after_restart() {
    let wal_root = temp_runtime_wal_dir("causal-anchor-restart");
    let (initial_runtime, worldline_id) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("filesystem runtime WAL should initialize");
    let basis = host
        .app()
        .current_causal_anchor_basis()
        .expect("enabled WAL should expose its durable causal basis");
    let request = causal_anchor_request(basis, "restart");
    let policy = causal_anchor_support_policy(std::slice::from_ref(&request));
    let expected_policy_digest = *policy.policy_digest();
    host.install_causal_anchor_root_support_policy(policy);

    let admitted = host
        .app()
        .admit_causal_anchor(request.clone())
        .expect("supported anchor should commit before acknowledgement");
    let anchor_id = *admitted.fact().anchor_id();
    assert_eq!(
        admitted.receipt().support_policy_digest(),
        &expected_policy_digest
    );
    let admission_basis = host
        .app()
        .current_causal_anchor_basis()
        .expect("committed anchor should advance the durable causal basis");

    let before_observation = host
        .app()
        .causal_anchor_by_id_at_basis(&anchor_id, &basis)
        .expect("the pre-admission basis should remain observable");
    assert!(
        before_observation.is_none(),
        "an anchor must not be visible before its admission transition"
    );
    let after_observation = host
        .app()
        .causal_anchor_by_id_at_basis(&anchor_id, &admission_basis)
        .expect("the post-admission basis should remain observable")
        .expect("the admitted anchor should be visible after admission");
    assert_eq!(after_observation, admitted);
    host.app()
        .submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
        .expect("unrelated admitted history should advance the global basis");
    let expected_restarted_basis = host
        .app()
        .current_causal_anchor_basis()
        .expect("the later durable basis should remain observable");
    assert_ne!(admission_basis, expected_restarted_basis);
    assert_eq!(
        host.app()
            .causal_anchor_by_id_at_basis(&anchor_id, &expected_restarted_basis)
            .expect("a later non-anchor basis should support anchor observation"),
        Some(admitted.clone())
    );
    let unavailable_basis_error = host
        .app()
        .causal_anchor_by_id_at_basis(&anchor_id, &CausalFrontierRef::from_digest([0xff; 32]))
        .expect_err("an unknown basis must not produce an unsupported reading");
    assert!(matches!(
        unavailable_basis_error,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::CausalHistoryBasisUnavailable {
            requested
        }) if requested == [0xff; 32]
    ));
    drop(host);

    let (runtime, _) = runtime();
    let mut restarted =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("restarted host should initialize");
    restarted
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("restarted host should recover committed anchor authority");
    assert_eq!(
        restarted
            .app()
            .current_causal_anchor_basis()
            .expect("restarted host should rebuild the logical durable basis"),
        expected_restarted_basis
    );
    let recovered = restarted
        .app()
        .causal_anchor_by_id(&anchor_id)
        .expect("anchor lookup should recover from committed WAL history")
        .expect("committed anchor should remain addressable by id");
    assert_eq!(recovered.fact(), admitted.fact());
    assert_eq!(recovered.receipt(), admitted.receipt());
    let recovery = restarted
        .runtime_wal()
        .expect("restarted WAL should remain configured")
        .recover_read_only()
        .expect("anchor evidence should rebuild into the recovery certificate");
    assert_eq!(recovery.causal_anchor_history.len(), 1);
    assert_eq!(
        recovery.causal_anchor_history[0].basis_before(),
        &basis,
        "witnessed control history must retain the admission basis"
    );
    assert_eq!(
        recovery.causal_anchor_history[0].basis_after(),
        &admission_basis,
        "witnessed control history must name the frontier produced by admission"
    );
    assert_eq!(
        restarted
            .app()
            .causal_anchor_by_id_at_basis(&anchor_id, &basis)
            .expect("restart must rebuild the pre-admission reading"),
        None
    );
    assert_eq!(
        restarted
            .app()
            .causal_anchor_by_id_at_basis(&anchor_id, &admission_basis)
            .expect("restart must rebuild the post-admission reading"),
        Some(admitted.clone())
    );
    assert_eq!(
        restarted
            .app()
            .causal_anchor_by_id_at_basis(&anchor_id, &expected_restarted_basis)
            .expect("restart must rebuild later basis-pinned readings"),
        Some(admitted.clone())
    );
    assert_eq!(
        recovery.certificate.recovered_indexes_root,
        recovery
            .recomputed_indexes_root()
            .expect("recovered anchors should reproduce the certificate index root")
    );
    let retried = restarted
        .app()
        .admit_causal_anchor(request)
        .expect("exact retry should recover authority without mutable policy state");
    assert_eq!(retried, admitted);

    drop(restarted);
    fs::remove_dir_all(&wal_root).expect("causal-anchor WAL fixture should be removable");
}

#[test]
fn filesystem_causal_anchor_flush_failure_publishes_no_admission() {
    let wal_root = temp_runtime_wal_dir("causal-anchor-flush-failure");
    let (runtime, _) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_runtime_wal(
        TrustedRuntimeWalConfig::filesystem_with_fault_plan_for_test(
            &wal_root,
            FilesystemWalFaultPlan::fail_next(FilesystemWalFaultTarget::FlushCommit),
        ),
    )
    .expect("faulting filesystem runtime WAL should initialize");
    let basis = host
        .app()
        .current_causal_anchor_basis()
        .expect("enabled WAL should expose its durable causal basis");
    let request = causal_anchor_request(basis, "flush-failure");
    host.install_causal_anchor_root_support_policy(causal_anchor_support_policy(
        std::slice::from_ref(&request),
    ));

    let error = host
        .app()
        .admit_causal_anchor(request)
        .expect_err("failed commit flush must not acknowledge anchor authority");
    assert!(matches!(
        error,
        TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::Store(WalStoreError::Io(
            message
        ))) if message.contains("injected filesystem WAL flush_commit failure")
    ));
    let recovery = host
        .runtime_wal()
        .expect("runtime WAL should remain configured")
        .recover_read_only()
        .expect("failed anchor commit should leave recoverable WAL posture");
    assert!(recovery.causal_anchor_history.is_empty());
    assert_eq!(recovery.certificate.committed_transactions_replayed, 0);

    drop(host);
    fs::remove_dir_all(&wal_root).expect("failed-anchor WAL fixture should be removable");
}

#[test]
fn a_causal_anchor_whose_commit_sync_failed_is_not_admitted() {
    let wal_root = temp_runtime_wal_dir("causal-anchor-commit-sync-failure");
    let (initial_runtime, worldline_id) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(
        TrustedRuntimeWalConfig::filesystem_with_fault_plan_for_test(
            &wal_root,
            FilesystemWalFaultPlan::fail_next(FilesystemWalFaultTarget::CommitMarkerSync),
        ),
    )
    .expect("faulting filesystem runtime WAL should initialize");
    let basis = host
        .app()
        .current_causal_anchor_basis()
        .expect("enabled WAL should expose its durable causal basis");
    let request = causal_anchor_request(basis, "commit-sync-failure");
    host.install_causal_anchor_root_support_policy(causal_anchor_support_policy(
        std::slice::from_ref(&request),
    ));

    host.app()
        .admit_causal_anchor(request)
        .expect_err("a readable anchor record whose sync failed must not grant authority");

    // The host does not reconcile itself from the log.
    let error = host
        .app()
        .submit_intent_with_runtime_wal_ack(eint_envelope(worldline_id))
        .expect_err("the WAL is failed closed");
    assert!(
        matches!(
            error,
            TrustedRuntimeHostError::Wal(TrustedRuntimeWalError::StorageFailed)
        ),
        "{error:?}"
    );
    drop(host);
    fs::remove_dir_all(&wal_root).expect("failed-anchor WAL fixture should be removable");
}

#[test]
fn retained_native_fork_recovers_prefix_and_refuses_missing_source() {
    let root = temp_runtime_wal_dir("retained-native-fork");
    let (rt, lane) = runtime();
    let head = rt.heads().iter().next().expect("head").0.to_owned();
    let mut host = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
        .expect("wal");
    assert!(host
        .fork_local_operation_strand_v1(head, "candidate")
        .is_err());
    host.register_contract_package(package()).expect("package");
    let submission = host
        .app()
        .submit_intent_with_runtime_wal_ack(eint_envelope(lane))
        .expect("submit");
    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(17))
        .expect("stage");
    host.run_until_idle(4).expect("commit");
    let child = host
        .fork_local_operation_strand_v1(head, "candidate")
        .expect("native fork");
    let fork = host
        .runtime()
        .strands()
        .find_by_child_worldline(&child.worldline_id)
        .expect("strand")
        .fork_basis_ref();
    let before = host.runtime_wal().expect("wal").commits().len();
    assert_eq!(
        host.fork_local_operation_strand_v1(head, "candidate")
            .expect("retry"),
        child
    );
    assert_eq!(host.runtime_wal().expect("wal").commits().len(), before);
    assert!(host.fork_local_operation_strand_v1(head, "").is_err());
    drop(host);
    let (rt, _) = runtime();
    let mut restored = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    restored
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
        .expect("native topology replay");
    let recovered = restored
        .runtime()
        .strands()
        .find_by_child_worldline(&child.worldline_id)
        .expect("retained strand");
    assert_eq!(recovered.fork_basis_ref(), fork);
    assert_eq!(recovered.writer_heads(), &[child]);
    assert_eq!(
        restored
            .provenance()
            .entry(child.worldline_id, fork.fork_tick)
            .expect("prefix")
            .expected
            .commit_hash,
        fork.commit_hash
    );
}

#[test]
fn retained_fork_retry_reconciles_uncertain_commit_before_reopening() {
    for target in [
        FilesystemWalFaultTarget::CommitMarkerSynced,
        FilesystemWalFaultTarget::AppendFrame,
        FilesystemWalFaultTarget::FlushCommit,
    ] {
        let root = temp_runtime_wal_dir("retained-fork-uncertain");
        let (rt, lane) = runtime();
        let head = *rt.heads().iter().next().expect("head").0;
        let mut host = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
        host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
            .expect("wal");
        host.register_contract_package(package()).expect("package");
        let submission = host
            .app()
            .submit_intent_with_runtime_wal_ack(eint_envelope(lane))
            .expect("submit");
        host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(17))
            .expect("stage");
        host.run_until_idle(4).expect("source commit");
        let before = host.runtime_wal().expect("wal").commits().len();
        host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
            target,
        ))
        .expect("fault");
        assert!(host
            .fork_local_operation_strand_v1(head, "candidate")
            .is_err());
        let child = host
            .fork_local_operation_strand_v1(head, "candidate")
            .expect("retry original retained fork");
        assert_eq!(
            host.runtime_wal().expect("wal").commits().len(),
            before + 1,
            "retry must retain exactly one fork transaction"
        );
        let fork = host
            .runtime()
            .strands()
            .find_by_child_worldline(&child.worldline_id)
            .expect("fork")
            .fork_basis_ref();
        drop(host);
        let (rt, _) = runtime();
        let mut restored = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
        restored
            .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
            .expect("one retained fork reopens");
        assert_eq!(
            restored
                .fork_local_operation_strand_v1(head, "candidate")
                .expect("reopened retry"),
            child
        );
        assert_eq!(
            restored
                .runtime()
                .strands()
                .find_by_child_worldline(&child.worldline_id)
                .expect("retained fork")
                .fork_basis_ref(),
            fork
        );
        assert_eq!(
            restored.runtime_wal().expect("wal").commits().len(),
            before + 1
        );
        drop(restored);
        fs::remove_dir_all(root).expect("owned fixture cleanup");
    }
}

fn retained_fork_source_fixture(
    root: &std::path::Path,
) -> (TrustedRuntimeHost, WriterHeadKey, WorldlineId) {
    let (rt, lane) = runtime();
    let head = *rt.heads().iter().next().expect("head").0;
    let mut host = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(root))
        .expect("wal");
    host.register_contract_package(package()).expect("package");
    let submission = host
        .app()
        .submit_intent_with_runtime_wal_ack(eint_envelope(lane))
        .expect("submit");
    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(17))
        .expect("stage");
    host.run_until_idle(4).expect("source commit");
    (host, head, lane)
}

#[test]
fn retained_fork_pending_basis_survives_parent_advance_and_different_label() {
    let root = temp_runtime_wal_dir("retained-fork-parent-advance");
    let (mut host, head, lane) = retained_fork_source_fixture(&root);
    let original = host
        .provenance()
        .tip_ref(lane)
        .expect("tip")
        .expect("source");
    host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
        FilesystemWalFaultTarget::CommitMarkerSynced,
    ))
    .expect("fault");
    assert!(host
        .fork_local_operation_strand_v1(head, "original")
        .is_err());
    // A context write reconciles append permission without exposing topology.
    let node = *host
        .runtime()
        .worldlines()
        .get(&lane)
        .expect("lane")
        .state()
        .root();
    host.retain_echo_operation_observation_v1("parent-write-permission", head, &[node])
        .expect("writer prefix reconciled");
    assert_eq!(host.runtime().strands().len(), 0);
    let parent_receipt = host
        .runtime()
        .receipt_correlations()
        .next()
        .expect("source receipt")
        .causal_receipt_ref;
    let submission = host
        .app()
        .submit_intent_with_runtime_wal_ack(causal_eint_envelope(lane, vec![parent_receipt]))
        .expect("advance submit");
    host.stage_installed_contract_submission(submission.submission_id, &admission_ticket(18))
        .expect("advance stage");
    host.run_until_idle(4).expect("parent advance");
    let latest = host
        .provenance()
        .tip_ref(lane)
        .expect("tip")
        .expect("advanced source");
    assert!(latest.worldline_tick > original.worldline_tick);
    let before = host.runtime_wal().expect("wal").commits().len();
    let fresh = host
        .fork_local_operation_strand_v1(head, "fresh")
        .expect("different label");
    let old = host
        .fork_local_operation_strand_v1(head, "original")
        .expect("original retry");
    assert_ne!(old, fresh);
    assert_eq!(host.runtime().strands().len(), 2);
    assert_eq!(host.runtime_wal().expect("wal").commits().len(), before + 1);
    let basis = host
        .runtime()
        .strands()
        .find_by_child_worldline(&old.worldline_id)
        .expect("old fork")
        .fork_basis_ref();
    assert_eq!(
        (basis.fork_tick, basis.commit_hash),
        (original.worldline_tick, original.commit_hash)
    );
    assert_eq!(
        host.runtime()
            .strands()
            .find_by_child_worldline(&fresh.worldline_id)
            .expect("fresh fork")
            .fork_basis_ref()
            .fork_tick,
        latest.worldline_tick
    );
    drop(host);
    let (rt, _) = runtime();
    let mut restored = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    restored
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
        .expect("both forks reopen");
    assert_eq!(
        restored
            .fork_local_operation_strand_v1(head, "original")
            .expect("original"),
        old
    );
    assert_eq!(
        restored
            .fork_local_operation_strand_v1(head, "fresh")
            .expect("fresh"),
        fresh
    );
    assert_eq!(
        restored
            .runtime()
            .strands()
            .find_by_child_worldline(&old.worldline_id)
            .expect("old")
            .fork_basis_ref(),
        basis
    );
    drop(restored);
    fs::remove_dir_all(root).expect("owned fixture cleanup");
}

#[test]
fn retained_fork_pending_commit_counts_against_capacity_before_new_label() {
    let root = temp_runtime_wal_dir("retained-fork-capacity");
    let (mut host, head, _) = retained_fork_source_fixture(&root);
    for index in 0..63 {
        host.fork_local_operation_strand_v1(head, &format!("visible-{index}"))
            .expect("visible fork");
    }
    let before = host.runtime_wal().expect("wal").commits().len();
    host.inject_runtime_wal_filesystem_fault_for_test(FilesystemWalFaultPlan::fail_next(
        FilesystemWalFaultTarget::CommitMarkerSynced,
    ))
    .expect("fault");
    assert!(host
        .fork_local_operation_strand_v1(head, "pending")
        .is_err());
    assert_eq!(host.runtime().strands().len(), 63);
    assert!(host
        .fork_local_operation_strand_v1(head, "sixty-fifth")
        .is_err());
    assert_eq!(host.runtime().strands().len(), 64);
    let pending = host
        .fork_local_operation_strand_v1(head, "pending")
        .expect("pending retry");
    assert_eq!(host.runtime_wal().expect("wal").commits().len(), before + 1);
    drop(host);
    let (rt, _) = runtime();
    let mut restored = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    restored
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
        .expect("64 forks reopen");
    assert_eq!(restored.runtime().strands().len(), 64);
    assert_eq!(
        restored
            .fork_local_operation_strand_v1(head, "pending")
            .expect("pending"),
        pending
    );
    assert!(restored
        .fork_local_operation_strand_v1(head, "sixty-fifth")
        .is_err());
    drop(restored);
    fs::remove_dir_all(root).expect("owned fixture cleanup");
}

/// Issue #880: occurrence identity holds across restart and redelivery. One
/// occurrence is committed and one is still pending when the host stops; after
/// recovery each redelivery is a retry, each conflicting reuse of an
/// occurrence key is refused, and nothing new is written to the WAL.
#[test]
fn continuum_occurrence_identity_survives_host_restart_and_redelivery() {
    let wal_root = temp_runtime_wal_dir("continuum-occurrence");
    let (initial_runtime, worldline_id) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    host.register_contract_package(package())
        .expect("host should install package");

    let applied = [0xa1; 32];
    let committed_envelope = occurred_eint_envelope(worldline_id, 1, applied, MUTATION_VARS);
    let pending_envelope = occurred_eint_envelope(worldline_id, 2, applied, MUTATION_VARS);
    let committed = host
        .app()
        .submit_intent_with_runtime_wal_ack(committed_envelope.clone())
        .expect("the first occurrence is accepted");
    let pending = host
        .app()
        .submit_intent_with_runtime_wal_ack(pending_envelope.clone())
        .expect("a second occurrence of identical content is accepted");
    assert!(!committed.duplicate && !pending.duplicate);
    assert_ne!(
        committed.submission_id, pending.submission_id,
        "identical content in two occurrences is two submissions"
    );
    host.stage_installed_contract_submission(committed.submission_id, &admission_ticket(81))
        .expect("the first occurrence is staged");
    host.run_until_idle(4)
        .expect("trusted host should tick until idle");
    assert!(matches!(
        host.app().observe_intent_outcome(&committed.submission_id),
        IntentOutcome::Applied { .. }
    ));
    drop(host);

    let (reconstructed_runtime, _) = runtime();
    let mut restarted = TrustedRuntimeHost::new(reconstructed_runtime, empty_engine())
        .expect("reconstructed host should initialize");
    restarted
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("the WAL holding occurrence-bound submissions recovers");
    restarted
        .register_contract_package(package())
        .expect("host should reinstall package");
    assert_eq!(
        restarted
            .runtime()
            .witnessed_submission_envelope(&pending.submission_id)
            .and_then(IngressEnvelope::occurrence),
        pending_envelope.occurrence(),
        "the occurrence survives in retained envelope material"
    );

    for (envelope, original) in [(committed_envelope, committed), (pending_envelope, pending)] {
        let retry = restarted
            .app()
            .submit_intent_with_runtime_wal_ack(envelope)
            .expect("redelivering an occurrence is a retry");
        assert!(retry.duplicate);
        assert_eq!(retry.submission_id, original.submission_id);
    }

    let conflict = |result: Result<_, TrustedRuntimeHostError>, key: u8, existing: Hash| {
        matches!(
            result,
            Err(TrustedRuntimeHostError::Runtime(error))
                if matches!(
                    *error,
                    RuntimeError::ContinuumOccurrenceConflict {
                        worldline_id: conflict_worldline,
                        occurrence_key_digest,
                        existing_submission_id,
                    } if conflict_worldline == worldline_id
                        && occurrence_key_digest == [key; 32]
                        && existing_submission_id == existing
                )
        )
    };
    assert!(conflict(
        restarted
            .app()
            .submit_intent_with_runtime_wal_ack(occurred_eint_envelope(
                worldline_id,
                1,
                applied,
                b"amount=8",
            )),
        1,
        committed.submission_id,
    ));
    assert!(conflict(
        restarted
            .app()
            .submit_intent_with_runtime_wal_ack(occurred_eint_envelope(
                worldline_id,
                2,
                [0xb2; 32],
                MUTATION_VARS,
            )),
        2,
        pending.submission_id,
    ));
    assert_eq!(
        restarted
            .runtime_wal()
            .expect("runtime WAL should remain configured")
            .submission_acceptance_count(),
        2,
        "neither a retry nor a conflict writes an acceptance"
    );
    drop(restarted);
    fs::remove_dir_all(&wal_root).expect("WAL fixture cleanup succeeds");
}

/// Issue #886: after restart, a strand still inherits the occurrences of its
/// fork basis. Redelivering one to the child names the original submission and
/// writes no acceptance; reusing its key for other material is a conflict.
#[test]
fn a_strand_inherits_its_fork_basis_occurrences_across_restart() {
    let root = temp_runtime_wal_dir("strand-occurrence-inheritance");
    let (rt, lane) = runtime();
    let head = *rt.heads().iter().next().expect("head").0;
    let mut host = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
        .expect("wal");
    host.register_contract_package(package()).expect("package");
    let applied = [0xa1; 32];
    let original = host
        .app()
        .submit_intent_with_runtime_wal_ack(occurred_eint_envelope(lane, 1, applied, MUTATION_VARS))
        .expect("submit");
    host.stage_installed_contract_submission(original.submission_id, &admission_ticket(31))
        .expect("stage");
    host.run_until_idle(4).expect("commit");
    let child = host
        .fork_local_operation_strand_v1(head, "occurrence-child")
        .expect("native fork");
    drop(host);

    let (rt, _) = runtime();
    let mut restored = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    restored
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
        .expect("recovery");
    restored
        .register_contract_package(package())
        .expect("package");
    let to_child = |key: u8, vars: &[u8]| {
        IngressEnvelope::continuum_occurred_intent(
            IngressTarget::ExactHead { key: child },
            make_intent_kind("echo.intent/eint-v1"),
            echo_wasm_abi::pack_intent_v1(MUTATION_OP_ID, vars).expect("EINT should pack"),
            Vec::new(),
            ContinuumOccurrence::new([key; 32], applied),
        )
    };
    let acceptances = restored
        .runtime_wal()
        .expect("wal")
        .submission_acceptance_count();
    let witnessed = restored.runtime().witnessed_submission_count();
    let inherited = restored
        .app()
        .submit_intent_with_runtime_wal_ack(to_child(1, MUTATION_VARS))
        .expect("an inherited occurrence is an idempotency result");
    assert!(inherited.duplicate);
    assert_eq!(inherited.submission_id, original.submission_id);
    assert_eq!(inherited.head_key, original.head_key);
    let fork = restored
        .runtime()
        .strands()
        .find_by_child_worldline(&child.worldline_id)
        .expect("retained strand")
        .fork_basis_ref();
    assert_eq!(inherited.inherited_through, Some(fork.into()));
    assert_eq!(
        restored
            .runtime_wal()
            .expect("wal")
            .submission_acceptance_count(),
        acceptances,
        "an inherited occurrence writes no acceptance"
    );
    assert_eq!(restored.runtime().witnessed_submission_count(), witnessed);

    let conflict = restored
        .app()
        .submit_intent_with_runtime_wal_ack(to_child(1, b"amount=8"))
        .expect_err("reusing an inherited occurrence key for other content");
    assert!(matches!(
        conflict,
        TrustedRuntimeHostError::Runtime(error) if matches!(
            *error,
            RuntimeError::ContinuumOccurrenceConflict { existing_submission_id, .. }
                if existing_submission_id == original.submission_id
        )
    ));

    let fresh = restored
        .app()
        .submit_intent_with_runtime_wal_ack(to_child(2, MUTATION_VARS))
        .expect("an occurrence absent from the inherited past is admitted");
    assert!(!fresh.duplicate && fresh.inherited_through.is_none());
    assert_eq!(fresh.head_key, child);
    drop(restored);
    fs::remove_dir_all(&root).expect("WAL fixture cleanup succeeds");
}

const LEGACY_CAUSAL_SEGMENT: &[u8] =
    include_bytes!("fixtures/legacy_causal_wal/segments/segment-00000000000000000001.ecwal");
const LEGACY_CAUSAL_EPOCHS: &[u8] =
    include_bytes!("fixtures/legacy_causal_wal/writer-epochs.ecwal");
/// Decided causal submission in the frozen WAL, and its original ingress id.
const LEGACY_DECIDED: &str = "2348e63b4cd2b630e659d9bcac2e5456d56c35c7a5f04f717d07820a113888f7";
const LEGACY_DECIDED_INGRESS: &str =
    "811015cf74795d042fc637d41d506024f538206586707d533fede984a2dddcbf";
/// Pending causal Continuum occurrence in the frozen WAL.
const LEGACY_PENDING: &str = "8879b31036123e5b70962a068958f51e379750627e13abecf5b585d43fdd3f63";

fn hex_hash(value: &str) -> Hash {
    hex::decode(value)
        .expect("hex")
        .try_into()
        .expect("32 bytes")
}

/// A fresh copy of the frozen legacy causal WAL, checked against its digests.
fn legacy_causal_wal() -> PathBuf {
    use sha2::{Digest, Sha256};
    assert_eq!(
        hex::encode(Sha256::digest(LEGACY_CAUSAL_SEGMENT)),
        "95a762b44e674e7612ea74aff516034cdab192c8700c6c75d0a76b6386894457"
    );
    assert_eq!(
        hex::encode(Sha256::digest(LEGACY_CAUSAL_EPOCHS)),
        "bdef26980af969e059f52c3a212ade94cc26329083020b8bd1119313877efc61"
    );
    let root = temp_runtime_wal_dir("legacy-causal");
    fs::create_dir_all(root.join("segments")).expect("segments directory");
    fs::write(
        root.join("segments")
            .join("segment-00000000000000000001.ecwal"),
        LEGACY_CAUSAL_SEGMENT,
    )
    .expect("frozen segment copies");
    fs::write(root.join("writer-epochs.ecwal"), LEGACY_CAUSAL_EPOCHS)
        .expect("frozen epoch ledger copies");
    root
}

/// Issue #885: restart, then resend. Requests a writer accepted before the
/// separated causal recipe resolve to their original submissions when resent
/// in the new form: no acceptance is written, the decided one does not run
/// again, and the pending one is staged from its retained envelope and is
/// decided once. A request with a changed parent is a new act.
#[test]
fn causal_requests_accepted_before_the_separated_recipe_survive_restart_and_resend() {
    let root = legacy_causal_wal();
    let (rt, lane) = runtime();
    let mut host = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&root))
        .expect("the frozen WAL recovers");
    host.register_contract_package(package()).expect("package");
    let decided = hex_hash(LEGACY_DECIDED);
    let pending = hex_hash(LEGACY_PENDING);
    assert_eq!(
        host.runtime()
            .witnessed_submission_envelope(&decided)
            .expect("retained")
            .identity_version(),
        warp_core::IngressIdentityVersion::Original
    );
    let acceptances = host
        .runtime_wal()
        .expect("wal")
        .submission_acceptance_count();
    let ticks = host.provenance().len(lane).expect("history");

    let parent = causal_receipt_ref(lane, "legacy-parent", 41);
    let resend = host
        .app()
        .submit_intent_with_runtime_wal_ack(causal_eint_envelope(lane, vec![parent]))
        .expect("a resend of the decided request");
    assert!(resend.duplicate);
    assert_eq!(resend.submission_id, decided);
    assert_eq!(resend.ingress_id, hex_hash(LEGACY_DECIDED_INGRESS));
    let pending_resend = host
        .app()
        .submit_intent_with_runtime_wal_ack(IngressEnvelope::continuum_occurred_intent(
            IngressTarget::DefaultWriter { worldline_id: lane },
            make_intent_kind("echo.intent/eint-v1"),
            echo_wasm_abi::pack_intent_v1(MUTATION_OP_ID, b"amount=9").expect("EINT"),
            vec![IngressCausalParent::TickReceipt {
                receipt_ref: parent,
            }],
            ContinuumOccurrence::new([7; 32], [0xa7; 32]),
        ))
        .expect("a resend of the pending occurrence");
    assert!(pending_resend.duplicate);
    assert_eq!(pending_resend.submission_id, pending);
    assert_eq!(
        host.runtime_wal()
            .expect("wal")
            .submission_acceptance_count(),
        acceptances,
        "resends write no acceptance"
    );
    host.run_until_idle(4).expect("idle");
    assert_eq!(
        host.provenance().len(lane).expect("history"),
        ticks,
        "the decided request does not run again"
    );

    host.stage_installed_contract_submission(pending, &admission_ticket(62))
        .expect("the pending request stages from its retained envelope");
    host.run_until_idle(4).expect("commit");
    assert_eq!(host.provenance().len(lane).expect("history"), ticks + 1);
    // Decided once: the reference rule does not match `amount=9`.
    let outcome = host.app().observe_intent_outcome(&pending);
    assert!(
        matches!(outcome, IntentOutcome::NoMatchingRule { submission_id, .. } if submission_id == pending),
        "{outcome:?}"
    );
    host.run_until_idle(4).expect("idle");
    assert_eq!(host.provenance().len(lane).expect("history"), ticks + 1);

    let changed = host
        .app()
        .submit_intent_with_runtime_wal_ack(causal_eint_envelope(
            lane,
            vec![causal_receipt_ref(lane, "another-parent", 42)],
        ))
        .expect("a changed parent is a new request");
    assert!(!changed.duplicate);
    assert_eq!(
        host.runtime_wal()
            .expect("wal")
            .submission_acceptance_count(),
        acceptances + 1
    );
    drop(host);
    fs::remove_dir_all(&root).expect("WAL fixture cleanup succeeds");
}

/// PR #900 review: a runtime may hold accepted legacy causal material that a
/// newly enabled WAL does not. A resend in the separated form resolves to that
/// submission, and the WAL must record the retained (original-form) envelope
/// under it, so the log still recovers.
#[test]
fn a_legacy_resolved_resend_records_the_retained_envelope() {
    let legacy_root = legacy_causal_wal();
    let (rt, lane) = runtime();
    let mut recovered = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    recovered
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&legacy_root))
        .expect("the frozen WAL recovers");
    let pending = hex_hash(LEGACY_PENDING);
    let snapshot = recovered
        .runtime()
        .witnessed_submission_persistence_snapshot()
        .expect("snapshot");
    let pending_only = warp_core::WitnessedSubmissionPersistenceSnapshot::new(
        snapshot
            .records()
            .iter()
            .filter(|record| record.submission.submission_id == pending)
            .cloned()
            .collect(),
    );
    drop(recovered);
    fs::remove_dir_all(&legacy_root).expect("fixture cleanup");

    // A host whose WAL has never seen the accepted legacy request, given that
    // request through host-level reconfiguration of its runtime. (Enabling a
    // WAL on a runtime that already holds it is refused.)
    let fresh_root = temp_runtime_wal_dir("legacy-resolved-resend");
    let (rt, _) = runtime();
    let mut host = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&fresh_root))
        .expect("a fresh WAL");
    let mut parts = host.into_parts();
    parts
        .runtime_mut()
        .restore_witnessed_submission_persistence(pending_only)
        .expect("legacy material restores");
    let mut host = TrustedRuntimeHost::from_parts(parts);
    let parent = causal_receipt_ref(lane, "legacy-parent", 41);
    let resend = host
        .app()
        .submit_intent_with_runtime_wal_ack(IngressEnvelope::continuum_occurred_intent(
            IngressTarget::DefaultWriter { worldline_id: lane },
            make_intent_kind("echo.intent/eint-v1"),
            echo_wasm_abi::pack_intent_v1(MUTATION_OP_ID, b"amount=9").expect("EINT"),
            vec![IngressCausalParent::TickReceipt {
                receipt_ref: parent,
            }],
            ContinuumOccurrence::new([7; 32], [0xa7; 32]),
        ))
        .expect("the resend resolves to the legacy submission");
    assert!(resend.duplicate);
    assert_eq!(resend.submission_id, pending);
    drop(host);

    let (rt, _) = runtime();
    let mut restarted = TrustedRuntimeHost::new(rt, empty_engine()).expect("host");
    restarted
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&fresh_root))
        .expect("the WAL recorded the retained envelope and recovers");
    assert_eq!(
        restarted
            .runtime()
            .witnessed_submission_envelope(&pending)
            .expect("retained")
            .identity_version(),
        warp_core::IngressIdentityVersion::Original
    );
    drop(restarted);
    fs::remove_dir_all(&fresh_root).expect("WAL cleanup");
}

/// Study probe: a submission acknowledged durable and staged, but not yet
/// ticked, when the host stops. What does recovery restore, and is the work
/// applied exactly once afterwards?
#[test]
fn study_staged_unticked_submission_across_restart() {
    let wal_root = temp_runtime_wal_dir("study-staged-unticked");
    let (initial_runtime, worldline_id) = runtime();
    let mut host = TrustedRuntimeHost::new(initial_runtime, empty_engine())
        .expect("trusted host should initialize");
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("host should configure filesystem runtime WAL adapter");
    host.register_contract_package(package())
        .expect("host should install package");
    let envelope = eint_envelope(worldline_id);
    let ticket = admission_ticket(61);
    let submission = host
        .app()
        .submit_intent_with_runtime_wal_ack(envelope.clone())
        .expect("submission should cross the durable ACK boundary");
    let staged = host
        .stage_installed_contract_submission(submission.submission_id, &ticket)
        .expect("trusted host should stage the submission");
    println!(
        "STUDY before_restart staged={} ticketed={} pending_witnessed={} outcome={:?}",
        matches!(
            staged,
            warp_core::TicketedRuntimeIngressDisposition::Staged { .. }
        ),
        host.runtime().ticketed_runtime_ingress_count(),
        host.runtime().pending_witnessed_submission_count(),
        host.app().observe_intent_outcome(&submission.submission_id)
    );
    drop(host);

    let (reconstructed_runtime, _) = runtime();
    let mut restarted = TrustedRuntimeHost::new(reconstructed_runtime, empty_engine())
        .expect("reconstructed host should initialize");
    restarted
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&wal_root))
        .expect("the WAL with staged, unticked work recovers");
    restarted
        .register_contract_package(package())
        .expect("reconstructed host should reinstall deterministic contract code");
    println!(
        "STUDY after_recovery witnessed={} ticketed={} pending_witnessed={} outcome={:?}",
        restarted
            .runtime()
            .witnessed_submission(&submission.submission_id)
            .is_some(),
        restarted.runtime().ticketed_runtime_ingress_count(),
        restarted.runtime().pending_witnessed_submission_count(),
        restarted
            .app()
            .observe_intent_outcome(&submission.submission_id)
    );
    let idle_first = restarted
        .run_until_idle(4)
        .expect("ticking the recovered host succeeds");
    println!(
        "STUDY tick_before_restage committed_steps={} outcome={:?}",
        idle_first.committed_steps,
        restarted
            .app()
            .observe_intent_outcome(&submission.submission_id)
    );
    let retry = restarted
        .app()
        .submit_intent_with_runtime_wal_ack(envelope)
        .expect("redelivery is accepted or a duplicate");
    let restage = restarted.stage_installed_contract_submission(submission.submission_id, &ticket);
    println!(
        "STUDY redelivery duplicate={} restage={:?}",
        retry.duplicate,
        restage.as_ref().map(|d| matches!(
            d,
            warp_core::TicketedRuntimeIngressDisposition::Staged { .. }
        ))
    );
    let idle_second = restarted
        .run_until_idle(4)
        .expect("ticking after restaging succeeds");
    let applied = restarted
        .provenance()
        .len(worldline_id)
        .expect("provenance is readable");
    println!(
        "STUDY final committed_steps={} provenance_entries={applied} outcome={:?}",
        idle_second.committed_steps,
        restarted
            .app()
            .observe_intent_outcome(&submission.submission_id)
    );
    drop(restarted);
    fs::remove_dir_all(&wal_root).expect("WAL fixture cleanup succeeds");
    assert_eq!(applied, 1, "the acknowledged work applies exactly once");
}
