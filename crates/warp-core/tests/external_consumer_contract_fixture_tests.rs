// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Serious external-consumer-shaped contract fixture.
#![cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
#![allow(clippy::expect_used, clippy::panic)]

use echo_cas::{MemoryTier, RetainedBlobIndex, RetainedBlobRole, SemanticBlobCoordinate};
use echo_registry_api::{
    ArgDef, ContractArtifactVerificationPolicy, ObjectDef, OpDef, OpKind, RegistryInfo,
    RegistryProvider,
};
use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, AuthoredObserverPlan,
    ContractEvidenceIdentity, ContractMutationHandler, ContractOperationKind,
    ContractPackageIdentity, ContractQueryObserver, ContractQueryObserverResult, EngineBuilder,
    GraphStore, GraphView, InboxPolicy, IngressEnvelope, IngressTarget, IntentOutcome, NodeId,
    NodeRecord, ObservationAt, ObservationCoordinate, ObservationFrame, ObservationPayload,
    ObservationProjection, ObservationReadBudget, ObservationRequest, ObserverPlanId,
    OpticAdmissionTicket, OpticArtifactHandle, PatternGraph, PlaybackMode, SchedulerKind,
    TickDelta, TickReceiptRejection, TrustedRuntimeHost, WarpOp, WorldlineId, WorldlineRuntime,
    WorldlineState, WriterHead, WriterHeadKey, OPTIC_ADMISSION_TICKET_KIND,
    OPTIC_ARTIFACT_HANDLE_KIND,
};

const SCHEMA_SHA256_HEX: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ARTIFACT_HASH_HEX: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const REPLACE_RANGE_OP_ID: u32 = 7101;
const DOCUMENT_WINDOW_QUERY_ID: u32 = 7102;
const REPLACE_VARS_A: &[u8] = b"doc=alpha;range=0..5;text=hello";
const REPLACE_VARS_B: &[u8] = b"doc=alpha;range=0..5;text=hullo";
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
    static REGISTRY: StaticRegistry = StaticRegistry;
    warp_core::InstalledContractPackage {
        identity: ContractPackageIdentity {
            package_name: "jedit-shaped-hot-text",
            package_version: "0.1.0",
            artifact_hash_hex: ARTIFACT_HASH_HEX,
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
            op_id: REPLACE_RANGE_OP_ID,
            rule: replace_rule(),
        }],
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
        .expect("external receipt should carry contract evidence");
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
