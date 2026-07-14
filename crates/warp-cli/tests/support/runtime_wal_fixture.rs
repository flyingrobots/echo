// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-produced filesystem WAL fixtures for CLI integration tests.

use std::error::Error;

use echo_registry_api::{
    ArgDef, ContractArtifactVerificationPolicy, ObjectDef, OpDef, OpKind, RegistryInfo,
    RegistryProvider,
};
use tempfile::TempDir;
use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, CausalTickReceiptRef,
    ContractMutationHandler, ContractPackageIdentity, EngineBuilder, GraphStore, GraphView, Hash,
    InboxPolicy, IngressEnvelope, IngressTarget, IntentOutcome, NodeId, NodeRecord,
    OpticAdmissionTicket, OpticArtifactHandle, PatternGraph, PlaybackMode, SchedulerKind,
    TickDelta, TrustedRuntimeHost, TrustedRuntimeWalConfig, WarpOp, WorldlineId, WorldlineRuntime,
    WorldlineState, WriterHead, WriterHeadKey, OPTIC_ADMISSION_TICKET_KIND,
    OPTIC_ARTIFACT_HANDLE_KIND,
};

type FixtureResult<T> = Result<T, Box<dyn Error>>;

const MUTATION_OP_ID: u32 = 7001;
const MUTATION_VARS: &[u8] = b"runtime-wal-cli=primary";
const SCHEMA_SHA256_HEX: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const RULE_NAME: &str =
    "cmd/contract/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/7001/runtimeWal";
const RULE_ID_LABEL: &str =
    "rule:cmd/contract/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/7001/runtimeWal";

static ARGS: &[ArgDef] = &[ArgDef {
    name: "input",
    ty: "RuntimeWalInput",
    required: true,
    list: false,
}];

static OPS: &[OpDef] = &[OpDef {
    kind: OpKind::Mutation,
    name: "runtimeWal",
    op_id: MUTATION_OP_ID,
    args: ARGS,
    result_ty: "RuntimeWalResult",
    directives_json: "{}",
    footprint_certificate: None,
}];

pub(crate) struct RuntimeWalSubmissionFixture {
    pub(crate) root: TempDir,
    pub(crate) submission_id: Hash,
    pub(crate) canonical_envelope_digest: Hash,
    pub(crate) receipt_ref: Option<CausalTickReceiptRef>,
    pub(crate) receipt_digest: Option<Hash>,
    pub(crate) ticket_digest: Option<Hash>,
}

struct Registry;

impl RegistryProvider for Registry {
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

pub(crate) fn accepted_pending_fixture() -> FixtureResult<RuntimeWalSubmissionFixture> {
    let root = TempDir::new()?;
    let (runtime, worldline_id) = runtime()?;
    let mut host = TrustedRuntimeHost::new(runtime, empty_engine())?;
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(root.path()))?;

    let envelope = eint_envelope(worldline_id)?;
    let canonical_envelope_digest = envelope.ingress_id();
    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(envelope)?
    };
    drop(host);

    Ok(RuntimeWalSubmissionFixture {
        root,
        submission_id: submission.submission_id,
        canonical_envelope_digest,
        receipt_ref: None,
        receipt_digest: None,
        ticket_digest: None,
    })
}

pub(crate) fn decided_applied_fixture() -> FixtureResult<RuntimeWalSubmissionFixture> {
    let root = TempDir::new()?;
    let (runtime, worldline_id) = runtime()?;
    let mut host = TrustedRuntimeHost::new(runtime, empty_engine())?;
    host.enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(root.path()))?;
    host.register_contract_package(package())?;

    let envelope = eint_envelope(worldline_id)?;
    let canonical_envelope_digest = envelope.ingress_id();
    let submission = {
        let mut app = host.app();
        app.submit_intent_with_runtime_wal_ack(envelope)?
    };
    let ticket = admission_ticket(31);
    host.stage_installed_contract_submission(submission.submission_id, &ticket)?;
    host.run_until_idle(4)?;

    let outcome = {
        let app = host.app();
        app.observe_intent_outcome(&submission.submission_id)
    };
    let IntentOutcome::Applied { receipt, .. } = outcome else {
        return Err("runtime WAL fixture did not apply submission".into());
    };
    drop(host);

    Ok(RuntimeWalSubmissionFixture {
        root,
        submission_id: submission.submission_id,
        canonical_envelope_digest,
        receipt_ref: Some(receipt.causal_receipt_ref),
        receipt_digest: Some(receipt.tick_receipt_digest),
        ticket_digest: Some(ticket.ticket_digest),
    })
}

fn empty_engine() -> warp_core::Engine {
    let mut store = GraphStore::default();
    let root = make_node_id("runtime-wal-cli-root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("runtime-wal-cli-world"),
        },
    );
    EngineBuilder::new(store, root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build()
}

fn result_node_id() -> NodeId {
    make_node_id("runtime-wal-cli-result")
}

fn contract_matches(view: GraphView<'_>, scope: &NodeId) -> bool {
    warp_core::eint_vars_for_op(view, scope, MUTATION_OP_ID) == Some(MUTATION_VARS)
}

fn contract_execute(view: GraphView<'_>, _scope: &NodeId, delta: &mut TickDelta) {
    let warp_id = view.warp_id();
    let result = result_node_id();
    delta.push(WarpOp::UpsertNode {
        node: warp_core::NodeKey {
            warp_id,
            local_id: result,
        },
        record: NodeRecord {
            ty: make_type_id("runtime-wal-cli-result"),
        },
    });
    delta.push(WarpOp::SetAttachment {
        key: warp_core::AttachmentKey::node_alpha(warp_core::NodeKey {
            warp_id,
            local_id: result,
        }),
        value: Some(warp_core::AttachmentValue::Atom(
            warp_core::AtomPayload::new(
                make_type_id("runtime-wal-cli-result"),
                bytes::Bytes::copy_from_slice(b"runtime-wal-cli-result"),
            ),
        )),
    });
}

fn contract_footprint(view: GraphView<'_>, scope: &NodeId) -> warp_core::Footprint {
    let mut footprint = warp_core::runtime_ingress_eint_read_footprint(view, scope);
    let warp_id = view.warp_id();
    let result = result_node_id();
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
        id: make_type_id(RULE_ID_LABEL).0,
        name: RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: contract_matches,
        executor: contract_execute,
        compute_footprint: contract_footprint,
        factor_mask: 0,
        conflict_policy: warp_core::ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn package() -> warp_core::InstalledContractPackage<'static> {
    static REGISTRY: Registry = Registry;
    warp_core::InstalledContractPackage {
        identity: ContractPackageIdentity {
            package_name: "runtime-wal-cli-fixture",
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
        query_observers: vec![],
    }
}

fn runtime() -> FixtureResult<(WorldlineRuntime, WorldlineId)> {
    let mut runtime = WorldlineRuntime::new();
    let worldline_id = WorldlineId::from_bytes([17; 32]);
    runtime.register_worldline(worldline_id, WorldlineState::empty())?;
    runtime.register_writer_head(WriterHead::with_routing(
        WriterHeadKey {
            worldline_id,
            head_id: make_head_id("runtime-wal-cli-default"),
        },
        PlaybackMode::Play,
        InboxPolicy::AcceptAll,
        None,
        true,
    ))?;
    Ok((runtime, worldline_id))
}

fn eint_envelope(worldline_id: WorldlineId) -> FixtureResult<IngressEnvelope> {
    let payload = echo_wasm_abi::pack_intent_v1(MUTATION_OP_ID, MUTATION_VARS)
        .map_err(|error| format!("failed to pack runtime WAL EINT fixture: {error:?}"))?;
    Ok(IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("echo.intent/eint-v1"),
        payload,
    ))
}

fn admission_ticket(seed: u8) -> OpticAdmissionTicket {
    OpticAdmissionTicket {
        kind: OPTIC_ADMISSION_TICKET_KIND.to_owned(),
        artifact_handle: OpticArtifactHandle {
            kind: OPTIC_ARTIFACT_HANDLE_KIND.to_owned(),
            id: format!("runtime-wal-cli-{seed}"),
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
