// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Installed contract intent pipeline tests.
#![cfg(all(feature = "native_rule_bootstrap", feature = "host_test"))]
#![allow(clippy::expect_used, clippy::panic)]

use echo_cas::{MemoryTier, RetainedBlobIndex, RetainedBlobRole, SemanticBlobCoordinate};
use echo_registry_api::{
    ArgDef, ContractArtifactVerificationPolicy, ObjectDef, OpDef, OpKind, RegistryInfo,
    RegistryProvider,
};
use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, AuthoredObserverPlan,
    ContractMutationHandler, ContractPackageIdentity, ContractQueryObserver,
    ContractQueryObserverResult, Engine, EngineBuilder, GraphStore, GraphView, InboxPolicy,
    IngressEnvelope, IngressSubmissionGeneration, IngressTarget, IntentOutcomeDecision,
    IntentOutcomeObservation, IntentSubmissionDisposition, NodeId, NodeRecord, ObservationAt,
    ObservationCoordinate, ObservationFrame, ObservationPayload, ObservationProjection,
    ObservationReadBudget, ObservationRequest, ObservationService, ObserverPlanId,
    OpticAdmissionTicket, OpticArtifactHandle, PatternGraph, PlaybackMode, ProvenanceService,
    ReadingBudgetPosture, RuntimeError, SchedulerCoordinator, SchedulerKind, TickDelta,
    TickReceiptRejection, TicketedRuntimeIngressAuthority, WorldlineId, WorldlineRuntime,
    WorldlineState, WriterHead, WriterHeadKey, OPTIC_ADMISSION_TICKET_KIND,
    OPTIC_ARTIFACT_HANDLE_KIND,
};

const SCHEMA_SHA256_HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MUTATION_OP_ID: u32 = 1001;
const CONFLICT_OP_ID: u32 = 1002;
const QUERY_OP_ID: u32 = 1003;
const UNKNOWN_OP_ID: u32 = 9999;
const MUTATION_VARS: &[u8] = b"amount=42";
const CONFLICT_VARS_A: &[u8] = b"amount=1";
const CONFLICT_VARS_B: &[u8] = b"amount=2";
const RESULT_TYPE: &str = "test/toy-counter/increment-result";
const RESULT_BYTES: &[u8] = b"value=42";
const MUTATION_RULE_NAME: &str =
    "cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/1001/increment";
const MUTATION_RULE_ID_LABEL: &str =
    "rule:cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/1001/increment";
const CONFLICT_RULE_NAME: &str =
    "cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/1002/conflict";
const CONFLICT_RULE_ID_LABEL: &str =
    "rule:cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/1002/conflict";
const SHARED_CONFLICT_RESULT: &str = "test/toy-counter/shared-conflict-result";

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
        kind: OpKind::Mutation,
        name: "conflict",
        op_id: CONFLICT_OP_ID,
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
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: SCHEMA_SHA256_HEX,
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

fn empty_engine() -> Engine {
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

fn package_identity() -> ContractPackageIdentity<'static> {
    ContractPackageIdentity {
        package_name: "toy-counter",
        package_version: "0.1.0",
        artifact_hash_hex: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    }
}

fn verification_policy() -> ContractArtifactVerificationPolicy<'static> {
    ContractArtifactVerificationPolicy {
        codec_id: "cbor-canon-v1",
        registry_version: 1,
        schema_sha256_hex: SCHEMA_SHA256_HEX,
        footprint_certificates: &[],
        require_mutation_footprint_certificates: false,
    }
}

fn result_node_id(scope: &NodeId) -> NodeId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"test.installed-contract.pipeline.result-node");
    hasher.update(scope.as_bytes());
    NodeId(hasher.finalize().into())
}

fn contract_matches(view: GraphView<'_>, scope: &NodeId) -> bool {
    warp_core::eint_vars_for_op(view, scope, MUTATION_OP_ID) == Some(MUTATION_VARS)
}

fn contract_execute(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    if warp_core::eint_vars_for_op(view, scope, MUTATION_OP_ID) != Some(MUTATION_VARS) {
        return;
    }
    let warp_id = view.warp_id();
    let result = result_node_id(scope);
    delta.push(warp_core::WarpOp::UpsertNode {
        node: warp_core::NodeKey {
            warp_id,
            local_id: result,
        },
        record: NodeRecord {
            ty: make_type_id(RESULT_TYPE),
        },
    });
    delta.push(warp_core::WarpOp::SetAttachment {
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

fn shared_conflict_node_id() -> NodeId {
    make_node_id(SHARED_CONFLICT_RESULT)
}

fn conflict_matches(view: GraphView<'_>, scope: &NodeId) -> bool {
    warp_core::eint_vars_for_op(view, scope, CONFLICT_OP_ID).is_some()
}

fn conflict_execute(view: GraphView<'_>, _scope: &NodeId, delta: &mut TickDelta) {
    let warp_id = view.warp_id();
    let result = shared_conflict_node_id();
    delta.push(warp_core::WarpOp::UpsertNode {
        node: warp_core::NodeKey {
            warp_id,
            local_id: result,
        },
        record: NodeRecord {
            ty: make_type_id(RESULT_TYPE),
        },
    });
}

fn conflict_footprint(view: GraphView<'_>, scope: &NodeId) -> warp_core::Footprint {
    let mut footprint = warp_core::runtime_ingress_eint_read_footprint(view, scope);
    footprint
        .n_write
        .insert_with_warp(view.warp_id(), shared_conflict_node_id());
    footprint
}

fn conflict_rule() -> warp_core::RewriteRule {
    warp_core::RewriteRule {
        id: make_type_id(CONFLICT_RULE_ID_LABEL).0,
        name: CONFLICT_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: conflict_matches,
        executor: conflict_execute,
        compute_footprint: conflict_footprint,
        factor_mask: 0,
        conflict_policy: warp_core::ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn query_observer() -> ContractQueryObserver {
    ContractQueryObserver::new(QUERY_OP_ID, query_observer_plan(), |context| {
        let mut bytes = b"window:".to_vec();
        bytes.extend_from_slice(context.vars_bytes);
        bytes.extend_from_slice(b":value=42");
        Ok(ContractQueryObserverResult::complete(bytes))
    })
}

fn query_observer_plan() -> AuthoredObserverPlan {
    AuthoredObserverPlan {
        plan_id: ObserverPlanId::from_bytes([0x51; 32]),
        artifact_hash: [0x52; 32],
        schema_hash: [0x53; 32],
        state_schema_hash: [0x54; 32],
        update_law_hash: [0x55; 32],
        emission_law_hash: [0x56; 32],
    }
}

fn install_contract(engine: &mut Engine) {
    static REGISTRY: StaticRegistry = StaticRegistry;
    engine
        .install_contract_package(warp_core::InstalledContractPackage {
            identity: package_identity(),
            registry: &REGISTRY,
            verification_policy: verification_policy(),
            mutation_handlers: vec![
                ContractMutationHandler {
                    op_id: MUTATION_OP_ID,
                    rule: contract_rule(),
                },
                ContractMutationHandler {
                    op_id: CONFLICT_OP_ID,
                    rule: conflict_rule(),
                },
            ],
            query_observers: vec![query_observer()],
        })
        .expect("contract package should install");
}

fn register_head(runtime: &mut WorldlineRuntime, worldline_id: WorldlineId) -> WriterHeadKey {
    let key = WriterHeadKey {
        worldline_id,
        head_id: make_head_id("default"),
    };
    runtime
        .register_writer_head(WriterHead::with_routing(
            key,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("writer head should register");
    key
}

fn runtime_store(runtime: &WorldlineRuntime, worldline_id: WorldlineId) -> &GraphStore {
    let frontier = runtime
        .worldlines()
        .get(&worldline_id)
        .expect("worldline should exist");
    frontier
        .state()
        .warp_state()
        .store(&frontier.state().root().warp_id)
        .expect("frontier store should exist")
}

fn provenance_for(runtime: &WorldlineRuntime) -> ProvenanceService {
    let mut provenance = ProvenanceService::new();
    for (worldline_id, frontier) in runtime.worldlines().iter() {
        provenance
            .register_worldline(*worldline_id, frontier.state())
            .expect("provenance should register");
    }
    provenance
}

fn admission_ticket(seed: u8) -> OpticAdmissionTicket {
    OpticAdmissionTicket {
        kind: OPTIC_ADMISSION_TICKET_KIND.to_owned(),
        artifact_handle: OpticArtifactHandle {
            kind: OPTIC_ARTIFACT_HANDLE_KIND.to_owned(),
            id: format!("installed-contract-pipeline-{seed}"),
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

fn ticketed_authority() -> TicketedRuntimeIngressAuthority {
    TicketedRuntimeIngressAuthority::assume_runtime_owner()
}

fn pipeline_runtime() -> (WorldlineRuntime, Engine, WorldlineId, WriterHeadKey) {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    install_contract(&mut engine);
    let worldline_id = WorldlineId::from_bytes([1; 32]);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .expect("worldline should register");
    let head = register_head(&mut runtime, worldline_id);
    (runtime, engine, worldline_id, head)
}

fn eint_envelope(worldline_id: WorldlineId, op_id: u32, vars: &[u8]) -> IngressEnvelope {
    IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("echo.intent/eint-v1"),
        echo_wasm_abi::pack_intent_v1(op_id, vars).expect("EINT should pack"),
    )
}

fn query_request(worldline_id: WorldlineId, vars: &[u8]) -> ObservationRequest {
    let mut request = ObservationRequest::builtin_one_shot(
        ObservationCoordinate {
            worldline_id,
            at: ObservationAt::Frontier,
        },
        ObservationFrame::QueryView,
        ObservationProjection::Query {
            query_id: QUERY_OP_ID,
            vars_bytes: vars.to_vec(),
        },
    )
    .expect("query request should build");
    request.budget = ObservationReadBudget::Bounded {
        max_payload_bytes: 128,
        max_witness_refs: 1,
    };
    request
}

fn semantic_coordinate(
    contract: &warp_core::ContractEvidenceIdentity,
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

#[test]
fn installed_contract_mutation_dispatches_only_through_ticketed_scheduler_tick() {
    let (mut runtime, mut engine, worldline_id, head) = pipeline_runtime();
    let envelope = eint_envelope(worldline_id, MUTATION_OP_ID, MUTATION_VARS);
    let event_id = NodeId(envelope.ingress_id());
    let result = result_node_id(&event_id);

    let submission = match runtime
        .submit_intent(envelope.clone())
        .expect("submission should be witnessed")
    {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => {
            panic!("first submission should not be duplicate")
        }
    };

    assert!(
        runtime_store(&runtime, worldline_id)
            .node(&result)
            .is_none(),
        "witnessed submission must not execute installed contract handlers"
    );

    let ticket = admission_ticket(7);
    runtime
        .ingest_installed_contract_invocation(
            &ticketed_authority(),
            &engine,
            submission,
            &ticket,
            envelope,
        )
        .expect("package-supported ticketed ingress should stage");

    assert!(
        runtime_store(&runtime, worldline_id)
            .node(&result)
            .is_none(),
        "ticketed runtime ingress must stage work without executing"
    );

    let mut provenance = provenance_for(&runtime);
    let records = SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
        .expect("scheduler-owned tick should execute installed contract handler");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].head_key, head);
    assert_eq!(records[0].admitted_count, 1);
    let store = runtime_store(&runtime, worldline_id);
    assert_eq!(
        store.node(&result).map(|record| record.ty),
        Some(make_type_id(RESULT_TYPE))
    );
    assert!(matches!(
        store.node_attachment(&result),
        Some(warp_core::AttachmentValue::Atom(payload))
            if payload.type_id == make_type_id(RESULT_TYPE)
                && payload.bytes.as_ref() == RESULT_BYTES
    ));
    assert!(runtime
        .receipt_correlation_for_submission(&submission)
        .is_some());
    let correlation = runtime
        .receipt_correlation_for_submission(&submission)
        .expect("receipt correlation should exist");
    let contract = correlation
        .contract
        .as_ref()
        .expect("installed mutation receipt correlation must carry contract evidence");
    assert_eq!(contract.op_id, MUTATION_OP_ID);
    assert_eq!(contract.op_kind, warp_core::ContractOperationKind::Mutation);
    assert_eq!(contract.schema_sha256_hex, SCHEMA_SHA256_HEX);
    assert_eq!(contract.package_name, "toy-counter");

    assert_eq!(
        runtime.observe_intent_outcome(&submission),
        IntentOutcomeObservation::Decided {
            correlation: Box::new(
                runtime
                    .receipt_correlation_for_submission(&submission)
                    .expect("receipt correlation should exist")
                    .clone(),
            ),
            decision: IntentOutcomeDecision::Applied {
                receipt_entry_index: 0,
                rule_id: make_type_id(MUTATION_RULE_ID_LABEL).0,
            },
        }
    );
}

#[test]
fn unsupported_installed_contract_mutation_cannot_enter_ticketed_runtime_ingress() {
    let (mut runtime, engine, worldline_id, head) = pipeline_runtime();
    let envelope = eint_envelope(worldline_id, UNKNOWN_OP_ID, MUTATION_VARS);
    let submission = match runtime
        .submit_intent(envelope.clone())
        .expect("unsupported submission should still be witnessed")
    {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => {
            panic!("first submission should not be duplicate")
        }
    };
    let ticket = admission_ticket(8);

    let err = runtime
        .ingest_installed_contract_invocation(
            &ticketed_authority(),
            &engine,
            submission,
            &ticket,
            envelope,
        )
        .expect_err("unsupported contract op must not stage runtime ingress");

    assert!(matches!(
        err,
        RuntimeError::UnsupportedInstalledContractMutation {
            op_id: UNKNOWN_OP_ID
        }
    ));
    assert_eq!(runtime.ticketed_runtime_ingress_count(), 0);
    assert_eq!(
        runtime
            .heads()
            .get(&head)
            .expect("head should exist")
            .inbox()
            .pending_count(),
        0
    );
}

#[test]
fn footprint_conflict_is_final_without_hidden_retry() {
    let (mut runtime, mut engine, worldline_id, _head) = pipeline_runtime();
    let envelope_a = eint_envelope(worldline_id, CONFLICT_OP_ID, CONFLICT_VARS_A);
    let envelope_b = eint_envelope(worldline_id, CONFLICT_OP_ID, CONFLICT_VARS_B);

    let submission_a = match runtime
        .submit_intent(envelope_a.clone())
        .expect("first submission should be witnessed")
    {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => {
            panic!("first submission must not be duplicate")
        }
    };
    let submission_b = match runtime
        .submit_intent(envelope_b.clone())
        .expect("second submission should be witnessed")
    {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => {
            panic!("second submission must not be duplicate")
        }
    };

    runtime
        .ingest_installed_contract_invocation(
            &ticketed_authority(),
            &engine,
            submission_a,
            &admission_ticket(11),
            envelope_a,
        )
        .expect("first conflict candidate should stage");
    runtime
        .ingest_installed_contract_invocation(
            &ticketed_authority(),
            &engine,
            submission_b,
            &admission_ticket(12),
            envelope_b.clone(),
        )
        .expect("second conflict candidate should stage");

    let mut provenance = provenance_for(&runtime);
    SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
        .expect("conflict rejection is a lawful tick outcome");

    let decisions = [
        runtime.observe_intent_outcome(&submission_a),
        runtime.observe_intent_outcome(&submission_b),
    ];
    let applied = decisions
        .iter()
        .filter(|decision| {
            matches!(
                decision,
                IntentOutcomeObservation::Decided {
                    decision: IntentOutcomeDecision::Applied { .. },
                    ..
                }
            )
        })
        .count();
    let rejected = decisions
        .iter()
        .filter_map(|decision| match decision {
            IntentOutcomeObservation::Decided {
                decision:
                    IntentOutcomeDecision::Rejected {
                        reason, blocked_by, ..
                    },
                ..
            } => Some((*reason, blocked_by.as_slice())),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(applied, 1, "one conflicting candidate should apply");
    assert_eq!(rejected.len(), 1, "one conflicting candidate should reject");
    assert_eq!(rejected[0].0, TickReceiptRejection::FootprintConflict);
    assert_eq!(
        rejected[0].1,
        &[0],
        "conflict receipt should attribute the blocking applied candidate"
    );

    assert!(matches!(
        runtime
            .submit_intent(envelope_b)
            .expect("duplicate submit should be observed"),
        IntentSubmissionDisposition::Duplicate { submission_id, .. }
            if submission_id == submission_b
    ));
    assert_eq!(
        runtime.ticketed_runtime_ingress_count(),
        2,
        "duplicate submission must not create a hidden retry ingress"
    );
}

#[test]
fn replay_witnessed_submissions_rejects_invalid_batch_without_partial_import() {
    let (mut runtime, _engine, worldline_id, _head) = pipeline_runtime();
    let envelope_a = eint_envelope(worldline_id, MUTATION_OP_ID, MUTATION_VARS);
    let envelope_b = eint_envelope(worldline_id, CONFLICT_OP_ID, CONFLICT_VARS_A);

    runtime
        .submit_intent(envelope_a)
        .expect("first submission should be witnessed");
    runtime
        .submit_intent(envelope_b)
        .expect("second submission should be witnessed");
    let mut replay_records = runtime.witnessed_submission_replay_records();
    replay_records
        .get_mut(1)
        .expect("second replay record should exist")
        .submission_id = [0xA5; 32];

    let (mut replayed, _engine, replay_worldline_id, _head) = pipeline_runtime();
    assert!(matches!(
        replayed.replay_witnessed_submissions(replay_records),
        Err(RuntimeError::IntentSubmissionReplayMismatch(_))
    ));

    assert_eq!(
        replayed.witnessed_submission_count(),
        0,
        "failed replay import must not retain earlier records from the same batch"
    );
    assert!(matches!(
        replayed
            .submit_intent(eint_envelope(replay_worldline_id, MUTATION_OP_ID, MUTATION_VARS))
            .expect("live submission after failed replay should still work"),
        IntentSubmissionDisposition::Accepted {
            submission_generation,
            ..
        } if submission_generation == IngressSubmissionGeneration::from_raw(1)
    ));
}

#[test]
fn replay_witnessed_submissions_rejects_zero_generation() {
    let (mut runtime, _engine, worldline_id, _head) = pipeline_runtime();
    let envelope = eint_envelope(worldline_id, MUTATION_OP_ID, MUTATION_VARS);

    runtime
        .submit_intent(envelope)
        .expect("submission should be witnessed");
    let mut replay_records = runtime.witnessed_submission_replay_records();
    replay_records
        .get_mut(0)
        .expect("replay record should exist")
        .submission_generation = IngressSubmissionGeneration::ZERO;

    let (mut replayed, _engine, _worldline_id, _head) = pipeline_runtime();
    assert!(matches!(
        replayed.replay_witnessed_submissions(replay_records),
        Err(RuntimeError::IntentSubmissionReplayMismatch(_))
    ));
    assert_eq!(
        replayed.witnessed_submission_count(),
        0,
        "zero-generation replay records must not enter witnessed history"
    );
}

#[test]
fn replay_witnessed_submissions_rejects_duplicate_generations() {
    let (mut runtime, _engine, worldline_id, _head) = pipeline_runtime();
    let envelope_a = eint_envelope(worldline_id, MUTATION_OP_ID, MUTATION_VARS);
    let envelope_b = eint_envelope(worldline_id, CONFLICT_OP_ID, CONFLICT_VARS_A);

    runtime
        .submit_intent(envelope_a)
        .expect("first submission should be witnessed");
    runtime
        .submit_intent(envelope_b)
        .expect("second submission should be witnessed");
    let mut replay_records = runtime.witnessed_submission_replay_records();
    let duplicate_generation = replay_records
        .first()
        .expect("first replay record should exist")
        .submission_generation;
    replay_records
        .get_mut(1)
        .expect("second replay record should exist")
        .submission_generation = duplicate_generation;

    let (mut replayed, _engine, _worldline_id, _head) = pipeline_runtime();
    assert!(matches!(
        replayed.replay_witnessed_submissions(replay_records),
        Err(RuntimeError::IntentSubmissionReplayMismatch(_))
    ));
    assert_eq!(
        replayed.witnessed_submission_count(),
        0,
        "duplicate replay generations must not enter witnessed history"
    );
}

#[test]
fn witnessed_submission_replay_restores_pending_history_without_runtime_ingress() {
    let (mut runtime, _engine, worldline_id, _head) = pipeline_runtime();
    let envelope_a = eint_envelope(worldline_id, MUTATION_OP_ID, MUTATION_VARS);
    let envelope_b = eint_envelope(worldline_id, CONFLICT_OP_ID, CONFLICT_VARS_A);

    let submission_a = match runtime
        .submit_intent(envelope_a.clone())
        .expect("first submission should be witnessed")
    {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => {
            panic!("first submission must not be duplicate")
        }
    };
    let submission_b = match runtime
        .submit_intent(envelope_b)
        .expect("second submission should be witnessed")
    {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => {
            panic!("second submission must not be duplicate")
        }
    };
    let replay_records = runtime.witnessed_submission_replay_records();

    let (mut replayed, _engine, _worldline_id, replay_head) = pipeline_runtime();
    replayed
        .replay_witnessed_submissions(replay_records)
        .expect("witnessed submission replay should import");

    assert_eq!(replayed.witnessed_submission_count(), 2);
    assert_eq!(replayed.ticketed_runtime_ingress_count(), 0);
    assert_eq!(
        replayed
            .heads()
            .get(&replay_head)
            .expect("replay head should exist")
            .inbox()
            .pending_count(),
        0,
        "replaying witnessed submissions must not stage runtime ingress"
    );
    assert_eq!(replayed.global_tick().as_u64(), 0);

    assert!(matches!(
        replayed.observe_intent_outcome(&submission_a),
        IntentOutcomeObservation::Pending {
            submission_id,
            ticketed_ingress_id: None,
            ..
        } if submission_id == submission_a
    ));
    assert!(matches!(
        replayed.observe_intent_outcome(&submission_b),
        IntentOutcomeObservation::Pending {
            submission_id,
            ticketed_ingress_id: None,
            ..
        } if submission_id == submission_b
    ));
    assert!(matches!(
        replayed
            .submit_intent(envelope_a)
            .expect("replayed duplicate should be recognized"),
        IntentSubmissionDisposition::Duplicate { submission_id, .. }
            if submission_id == submission_a
    ));
}

#[test]
fn witnessed_submission_replay_preserves_generation_continuity() {
    let (mut runtime, _engine, worldline_id, _head) = pipeline_runtime();
    let envelope_a = eint_envelope(worldline_id, MUTATION_OP_ID, MUTATION_VARS);
    let envelope_b = eint_envelope(worldline_id, CONFLICT_OP_ID, CONFLICT_VARS_A);
    let envelope_c = eint_envelope(worldline_id, CONFLICT_OP_ID, CONFLICT_VARS_B);

    runtime
        .submit_intent(envelope_a)
        .expect("first submission should be witnessed");
    runtime
        .submit_intent(envelope_b)
        .expect("second submission should be witnessed");
    let replay_records = runtime.witnessed_submission_replay_records();

    let (mut replayed, _engine, _worldline_id, _head) = pipeline_runtime();
    replayed
        .replay_witnessed_submissions(replay_records)
        .expect("witnessed submission replay should import");

    assert!(matches!(
        replayed
            .submit_intent(envelope_c)
            .expect("next live submit should be witnessed"),
        IntentSubmissionDisposition::Accepted {
            submission_generation,
            ..
        } if submission_generation == IngressSubmissionGeneration::from_raw(3)
    ));
}

#[test]
fn external_contract_fixture_proves_mutation_query_retention_and_replay() {
    let (mut runtime, mut engine, worldline_id, _head) = pipeline_runtime();
    let envelope = eint_envelope(worldline_id, MUTATION_OP_ID, MUTATION_VARS);
    let ticket = admission_ticket(31);
    let submission = match runtime
        .submit_intent(envelope.clone())
        .expect("external fixture submission should be witnessed")
    {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => {
            panic!("first external fixture submission must not be duplicate")
        }
    };
    let replay_records = runtime.witnessed_submission_replay_records();

    runtime
        .ingest_installed_contract_invocation(
            &ticketed_authority(),
            &engine,
            submission,
            &ticket,
            envelope.clone(),
        )
        .expect("external fixture ticketed ingress should stage");
    let mut provenance = provenance_for(&runtime);
    SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine)
        .expect("external fixture scheduler-owned tick should commit");
    let outcome = runtime.observe_intent_outcome(&submission);
    assert!(matches!(
        outcome,
        IntentOutcomeObservation::Decided {
            decision: IntentOutcomeDecision::Applied { .. },
            ..
        }
    ));

    let query_vars = b"start=0;len=8";
    let reading = ObservationService::observe(
        &runtime,
        &provenance,
        &engine,
        query_request(worldline_id, query_vars),
    )
    .expect("external fixture QueryView reading should observe");
    let query_payload = match &reading.payload {
        ObservationPayload::QueryBytes(bytes) => bytes.clone(),
        other => panic!("external fixture expected QueryBytes, got {other:?}"),
    };
    assert_eq!(query_payload, b"window:start=0;len=8:value=42");
    match reading.reading.budget_posture {
        ReadingBudgetPosture::Bounded {
            max_payload_bytes,
            payload_bytes,
            max_witness_refs,
            witness_refs,
        } => {
            assert_eq!(max_payload_bytes, 128);
            assert!(payload_bytes >= query_payload.len() as u64);
            assert!(payload_bytes <= max_payload_bytes);
            assert_eq!(max_witness_refs, 1);
            assert_eq!(witness_refs, 1);
        }
        other @ ReadingBudgetPosture::UnboundedOneShot => {
            panic!("external fixture expected bounded reading posture, got {other:?}");
        }
    }

    let mut blobs = MemoryTier::new();
    let mut retained = RetainedBlobIndex::default();
    let query_identity = reading
        .reading
        .query_identity
        .as_ref()
        .expect("external fixture reading must carry query identity");
    let query_contract = reading
        .reading
        .contract
        .as_ref()
        .expect("external fixture reading must carry contract evidence");
    let reading_coord = semantic_coordinate(
        query_contract,
        RetainedBlobRole::ReadingPayload,
        query_identity.reading_id,
    );
    let reading_descriptor = retained.retain(&mut blobs, reading_coord.clone(), &query_payload);
    assert_eq!(
        retained
            .load_range(&blobs, &reading_coord, 7, 13, 13)
            .expect("retained bounded reading range should load")
            .bytes
            .as_ref(),
        b"start=0;len=8"
    );

    let correlation = runtime
        .receipt_correlation_for_submission(&submission)
        .expect("external fixture receipt correlation should exist");
    let receipt_contract = correlation
        .contract
        .as_ref()
        .expect("external fixture receipt must carry contract evidence");
    let receipt_coord = semantic_coordinate(
        receipt_contract,
        RetainedBlobRole::ContractReceipt,
        correlation.tick_receipt_digest,
    );
    let receipt_descriptor = retained.retain(
        &mut blobs,
        receipt_coord.clone(),
        &correlation.tick_receipt_digest,
    );

    assert_eq!(
        retained
            .load(&blobs, &reading_coord)
            .expect("retained reading payload should load")
            .descriptor,
        reading_descriptor
    );
    assert_eq!(
        retained
            .load(&blobs, &receipt_coord)
            .expect("retained receipt digest material should load")
            .descriptor,
        receipt_descriptor
    );

    let (mut replayed_runtime, mut replayed_engine, _worldline_id, _head) = pipeline_runtime();
    replayed_runtime
        .replay_witnessed_submissions(replay_records)
        .expect("external fixture replay should import witnessed submissions");
    replayed_runtime
        .ingest_installed_contract_invocation(
            &ticketed_authority(),
            &replayed_engine,
            submission,
            &ticket,
            envelope,
        )
        .expect("external fixture replay ticketed ingress should stage");
    let mut replayed_provenance = provenance_for(&replayed_runtime);
    SchedulerCoordinator::super_tick(
        &mut replayed_runtime,
        &mut replayed_provenance,
        &mut replayed_engine,
    )
    .expect("external fixture replay tick should commit");

    assert_eq!(
        replayed_runtime.observe_intent_outcome(&submission),
        outcome,
        "external fixture replay must reproduce the same observed outcome"
    );
}

#[test]
fn installed_contract_pipeline_replays_to_same_receipt_and_outcome() {
    let (mut original_runtime, mut original_engine, worldline_id, _head) = pipeline_runtime();
    let envelope = eint_envelope(worldline_id, MUTATION_OP_ID, MUTATION_VARS);
    let ticket = admission_ticket(21);
    let submission = match original_runtime
        .submit_intent(envelope.clone())
        .expect("submission should be witnessed")
    {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => {
            panic!("first submission must not be duplicate")
        }
    };
    let replay_records = original_runtime.witnessed_submission_replay_records();
    original_runtime
        .ingest_installed_contract_invocation(
            &ticketed_authority(),
            &original_engine,
            submission,
            &ticket,
            envelope.clone(),
        )
        .expect("original ticketed ingress should stage");
    let mut original_provenance = provenance_for(&original_runtime);
    let original_steps = SchedulerCoordinator::super_tick(
        &mut original_runtime,
        &mut original_provenance,
        &mut original_engine,
    )
    .expect("original tick should commit");
    let original_correlation = original_runtime
        .receipt_correlation_for_submission(&submission)
        .expect("original receipt correlation should exist")
        .clone();
    let original_outcome = original_runtime.observe_intent_outcome(&submission);

    let (mut replayed_runtime, mut replayed_engine, _worldline_id, _head) = pipeline_runtime();
    replayed_runtime
        .replay_witnessed_submissions(replay_records)
        .expect("witnessed submission replay should import");
    replayed_runtime
        .ingest_installed_contract_invocation(
            &ticketed_authority(),
            &replayed_engine,
            submission,
            &ticket,
            envelope,
        )
        .expect("replayed ticketed ingress should stage");
    let mut replayed_provenance = provenance_for(&replayed_runtime);
    let replayed_steps = SchedulerCoordinator::super_tick(
        &mut replayed_runtime,
        &mut replayed_provenance,
        &mut replayed_engine,
    )
    .expect("replayed tick should commit");
    let replayed_correlation = replayed_runtime
        .receipt_correlation_for_submission(&submission)
        .expect("replayed receipt correlation should exist")
        .clone();
    let replayed_outcome = replayed_runtime.observe_intent_outcome(&submission);

    assert_eq!(replayed_steps, original_steps);
    assert_eq!(
        replayed_correlation.tick_receipt_digest,
        original_correlation.tick_receipt_digest
    );
    assert_eq!(
        replayed_correlation.commit_hash,
        original_correlation.commit_hash
    );
    assert_eq!(replayed_outcome, original_outcome);
}
