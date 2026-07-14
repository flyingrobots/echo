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
    CausalFrontierRef, CausalTickReceiptRef, ContractMutationHandler, ContractOperationKind,
    ContractPackageIdentity, ContractQueryObserver, ContractQueryObserverResult, EngineBuilder,
    GlobalTick, GraphStore, GraphView, Hash, InboxPolicy, IngressCausalParent, IngressEnvelope,
    IngressTarget, IntentOutcome, NodeId, NodeRecord, ObservationAt, ObservationCoordinate,
    ObservationFrame, ObservationPayload, ObservationProjection, ObservationReadBudget,
    ObservationRequest, ObserverPlanId, OpticAdmissionTicket, OpticArtifactHandle, PatternGraph,
    PlaybackMode, ProvenanceStore, RuntimeError, RuntimeWalActivationGap, SchedulerKind, TickDelta,
    TrustedRuntimeHost, TrustedRuntimeHostError, TrustedRuntimeWal, TrustedRuntimeWalConfig,
    TrustedRuntimeWalError, TrustedRuntimeWalStoreKind, WarpOp, WorldlineId, WorldlineRuntime,
    WorldlineState, WorldlineTick, WriterHead, WriterHeadKey, CAUSAL_ANCHOR_SCHEMA_VERSION,
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

fn contract_execute(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    if warp_core::eint_vars_for_op(view, scope, MUTATION_OP_ID) != Some(MUTATION_VARS) {
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
        executor: contract_execute,
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
        .clone();
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
    let (initial_runtime, _) = runtime();
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
    let anchor_id = *admitted.fact.anchor_id();
    assert_eq!(
        admitted.receipt.support_policy_digest(),
        &expected_policy_digest
    );
    let expected_restarted_basis = host
        .app()
        .current_causal_anchor_basis()
        .expect("committed anchor should advance the durable causal basis");
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
    assert_eq!(recovered.fact, admitted.fact);
    assert_eq!(recovered.receipt, admitted.receipt);
    let recovery = restarted
        .runtime_wal()
        .expect("restarted WAL should remain configured")
        .recover_read_only()
        .expect("anchor evidence should rebuild into the recovery certificate");
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
    assert!(recovery.causal_anchors.is_empty());
    assert_eq!(recovery.certificate.committed_transactions_replayed, 0);

    drop(host);
    fs::remove_dir_all(&wal_root).expect("failed-anchor WAL fixture should be removable");
}
