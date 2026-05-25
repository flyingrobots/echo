// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Reference trusted runtime host loop tests.
#![cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
#![allow(clippy::expect_used, clippy::panic)]

use echo_registry_api::{
    ArgDef, ContractArtifactVerificationPolicy, ObjectDef, OpDef, OpKind, RegistryInfo,
    RegistryProvider,
};
use warp_core::{
    causal_wal::{
        recover_in_memory_store, recover_receipt_index, recover_submission_index,
        recovered_submission_receipt_index_root, Lsn, RecoveredSubmissionPosture,
        RecoveryAccessMode, WalBuildError, WalTransactionKind,
    },
    make_head_id, make_intent_kind, make_node_id, make_type_id, AuthoredObserverPlan,
    ContractMutationHandler, ContractOperationKind, ContractPackageIdentity, ContractQueryObserver,
    ContractQueryObserverResult, EngineBuilder, GraphStore, GraphView, InboxPolicy,
    IngressEnvelope, IngressTarget, IntentOutcome, NodeId, NodeRecord, ObservationAt,
    ObservationCoordinate, ObservationFrame, ObservationPayload, ObservationProjection,
    ObservationReadBudget, ObservationRequest, ObserverPlanId, OpticAdmissionTicket,
    OpticArtifactHandle, PatternGraph, PlaybackMode, SchedulerKind, TickDelta, TrustedRuntimeHost,
    TrustedRuntimeHostError, TrustedRuntimeWal, TrustedRuntimeWalError, WarpOp, WorldlineId,
    WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey, OPTIC_ADMISSION_TICKET_KIND,
    OPTIC_ARTIFACT_HANDLE_KIND,
};

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
    host.install_contract_package(package())
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

    let mut store = runtime_wal.cloned_store();
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
    let mut store = runtime_wal.cloned_store();
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
    host.install_contract_package(package())
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
    let tick_receipt_digest = receipt.tick_receipt_digest;

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

    let mut store = runtime_wal.cloned_store();
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
        Some(&tick_receipt_digest)
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
    host.install_contract_package(package())
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
    host.install_contract_package(package())
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
    host.install_contract_package(package())
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
        recovered_submission_receipt_index_root(&recovery.submissions, &recovery.receipts)
    );
}

#[test]
fn runtime_wal_ack_recover_read_only_exposes_recovery_certificate() {
    let (runtime, worldline_id) = runtime();
    let mut host =
        TrustedRuntimeHost::new(runtime, empty_engine()).expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");
    host.install_contract_package(package())
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
