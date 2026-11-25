// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! A downstream consumer runs Echo's retained public `link` package on
//! generic Echo, with `trusted_runtime` and without `native_rule_bootstrap`
//! (echo#1017).
//!
//! The consumer builds a graph, admits the exact package under an exact
//! policy, admits one invocation, evaluates it in a recording frame and
//! derives the proposal, all through [`EffectPackageHost`]. Oracles: the
//! package identity is pinned independently of the bytes, and the expected
//! mutation is the `link` program's own law written by hand (it links
//! `fromId` to the target of the first `E` edge it reads from the source).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test code fails loudly"
)]

use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
use warp_core::edict_effects::{
    CompletedEffectEvaluation, EffectDerivationKind, EffectMutation, TypedNodeRead,
};
use warp_core::edict_pure::EvaluationLimits;
use warp_core::{
    make_head_id, AdmittedEchoOperationInvocation, EchoOperationAdmissionErrorKind,
    EchoOperationAdmissionPolicy, EchoOperationApplicationBasisV1, EchoOperationBudgetV1,
    EchoOperationEvaluationBasisV1, EchoOperationInvocationAdmissionPolicy, EdgeId, EdgeKey,
    EdgeRecord, EffectPackageHost, EngineBuilder, GraphStore, InboxPolicy, InstalledEchoOperation,
    NodeId, NodeKey, NodeRecord, PlaybackMode, TypeId, WarpId, WarpOp, WorldlineId,
    WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

const PACKAGE: &str = include_str!(
    "../../crates/warp-core/tests/fixtures/edict-effect-graph-read/link-executable-operation-package.hex"
);
const OTHER_PACKAGE: &str = include_str!(
    "../../crates/warp-core/tests/fixtures/edict-effect-step/read-executable-operation-package.hex"
);
/// `echo:operation-package:v1` identity of the retained package bytes,
/// recorded when the package was published (echo#982).
const PACKAGE_PIN: &str = "cb362ed3001604563d6487e165d8fbc31e78c9bb87d633c6f8838bcf2fc6f6b3";
const OPERATION: &str = "consumer.links.program@1.link";
const AUTHORITY_LABEL: &[u8] = b"consumer.links.authority/v1";
const GRANT: [u8; 32] = [9; 32];

const WARP: WarpId = WarpId([0x61; 32]);
const NODE_TYPE: TypeId = TypeId([0x6b; 32]);
const PORTAL_TYPE: TypeId = TypeId([0x6c; 32]);
const EDGE_TYPE: TypeId = TypeId([0x6d; 32]);
const ROOT: NodeId = NodeId([0x62; 32]);
const SOURCE: NodeId = NodeId([0x63; 32]);
const TARGET: NodeId = NodeId([0x64; 32]);
const FROM: NodeId = NodeId([0x65; 32]);

fn hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&u64::try_from(bytes.len()).unwrap().to_le_bytes());
    hasher.update(bytes);
    hasher.finalize().into()
}

fn bytes(value: impl AsRef<[u8]>) -> Value {
    Value::Bytes(value.as_ref().to_vec())
}

fn map<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}

fn authority() -> [u8; 32] {
    hash(b"echo:operation-profile:v1\0", AUTHORITY_LABEL)
}

fn budget() -> EchoOperationBudgetV1 {
    EchoOperationBudgetV1::new(16_384, 4096, 4096)
}

fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 1_048_576,
        max_input_bytes: 65_536,
        max_steps: 16_384,
        max_allocated_bytes: 1_048_576,
        max_output_bytes: 65_536,
    }
}

fn link_edge() -> EdgeId {
    warp_core::content_address_v1::slot_edge_id(WARP, FROM, EDGE_TYPE)
}

/// A portal-typed root, the source with one `E` edge to the target, and the
/// node the program links from.
fn host() -> (EffectPackageHost, WriterHeadKey) {
    let mut store = GraphStore::new(WARP);
    store.insert_node(ROOT, NodeRecord { ty: PORTAL_TYPE });
    for node in [SOURCE, TARGET, FROM] {
        store.insert_node(node, NodeRecord { ty: NODE_TYPE });
    }
    store.insert_edge(
        SOURCE,
        EdgeRecord {
            id: EdgeId([0x66; 32]),
            from: SOURCE,
            to: TARGET,
            ty: EDGE_TYPE,
        },
    );
    let state = WorldlineState::from_root_store(store.clone(), ROOT).expect("state");
    let head = WriterHeadKey {
        worldline_id: WorldlineId::from_bytes([0x6a; 32]),
        head_id: make_head_id("public-effect-host-consumer"),
    };
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(head.worldline_id, state)
        .expect("worldline");
    runtime
        .register_writer_head(WriterHead::with_routing(
            head,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("writer head");
    let engine = EngineBuilder::new(store, ROOT).build();
    (EffectPackageHost::new(runtime, engine).expect("host"), head)
}

fn input() -> Vec<u8> {
    encode(&map([
        ("anchor", bytes([0x5a; 32])),
        (
            "read",
            map([
                ("edgeType", bytes(EDGE_TYPE.0)),
                ("nodeId", bytes(SOURCE.0)),
                ("warpId", bytes(WARP.0)),
            ]),
        ),
        ("edgeId", bytes(link_edge().0)),
        ("fromId", bytes(FROM.0)),
    ]))
    .expect("canonical input")
}

fn application_basis(input: &[u8]) -> EchoOperationApplicationBasisV1 {
    EchoOperationApplicationBasisV1::new(
        hash(
            b"echo:operation-profile:v1\0",
            b"echo.operation.basis.edict-effect/v1",
        ),
        hash(b"echo:operation-edict-effect-basis:v1\0", input),
    )
}

fn invocation(
    installed: &InstalledEchoOperation,
    head: WriterHeadKey,
    basis: EchoOperationEvaluationBasisV1,
    input: &[u8],
) -> Vec<u8> {
    let app = basis.application_basis();
    encode(&map([
        ("schema", Value::Text("echo.operation-invocation/v1".into())),
        ("kind", Value::Text("edict-effect-program/v1".into())),
        ("package_id", bytes(installed.package_identity())),
        (
            "operation_coordinate",
            Value::Text(installed.operation_coordinate().into()),
        ),
        ("authority_grant_identity", bytes(GRANT)),
        (
            "delegated_budget",
            map([
                ("steps", Value::Integer(16_384)),
                ("read_bytes", Value::Integer(4096)),
                ("write_bytes", Value::Integer(4096)),
            ]),
        ),
        ("application_input_bytes", Value::Bytes(input.to_vec())),
        (
            "evaluation_basis",
            map([
                ("worldline_id", bytes(head.worldline_id.as_bytes())),
                ("head_id", bytes(head.head_id.as_bytes())),
                (
                    "worldline_tick",
                    Value::Integer(i128::from(basis.worldline_tick().as_u64())),
                ),
                ("commit_global_tick", Value::Null),
                ("state_root", bytes(basis.state_root())),
                ("commit_id", bytes(basis.commit_id())),
                (
                    "application_basis_schema_identity",
                    bytes(app.schema_identity()),
                ),
                (
                    "application_basis_value_identity",
                    bytes(app.value_identity()),
                ),
            ]),
        ),
    ]))
    .expect("canonical invocation")
}

fn retained() -> Vec<u8> {
    let package = hex::decode(PACKAGE.trim()).expect("package hex");
    assert_eq!(
        hex::encode(hash(b"echo:operation-package:v1\0", &package)),
        PACKAGE_PIN,
        "retained package identity"
    );
    package
}

fn link_policy() -> EchoOperationAdmissionPolicy {
    EchoOperationAdmissionPolicy::exact(
        hex::decode(PACKAGE_PIN).unwrap().try_into().unwrap(),
        OPERATION,
        authority(),
        budget(),
    )
}

/// Admits and installs the link package in `host`, admits one invocation
/// and evaluates it in a recording frame at the current parent.
fn evaluate_link(
    host: &mut EffectPackageHost,
    head: WriterHeadKey,
) -> (AdmittedEchoOperationInvocation, CompletedEffectEvaluation) {
    let admitted = host
        .admit_echo_operation_package(&link_policy(), retained())
        .expect("the exact policy admits the package");
    let installed = host
        .install_admitted_echo_operation_package(admitted)
        .expect("installation");
    let program = installed
        .program()
        .as_effect_program()
        .expect("an effect program")
        .clone();
    let input = input();
    let parent = host
        .echo_operation_evaluation_basis_v1(head, application_basis(&input))
        .expect("basis");
    let admitted_invocation = host
        .admit_echo_operation_invocation(
            &EchoOperationInvocationAdmissionPolicy::new(authority(), GRANT, budget()),
            &invocation(&installed, head, parent, &input),
        )
        .expect("invocation admission");
    let mut frame = host
        .effect_evaluation_frame_v1(parent, &[EDGE_TYPE], 8, budget(), limits())
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("footprint law");
    frame
        .evaluate_effect(
            &program,
            &input,
            &[TypedNodeRead {
                node: NodeKey {
                    warp_id: WARP,
                    local_id: SOURCE,
                },
                absent_node_type: NODE_TYPE,
            }],
        )
        .expect("evaluation");
    let completed = frame.into_effect_evaluation(1 << 20).expect("evidence");
    (admitted_invocation, completed)
}

#[test]
fn a_trusted_runtime_consumer_runs_the_admitted_link_package_to_a_derived_proposal() {
    let (mut host, head) = host();
    let (admitted_invocation, completed) = evaluate_link(&mut host, head);
    let link = EffectMutation::EnsureEdge {
        edge: EdgeKey {
            warp_id: WARP,
            local_id: link_edge(),
        },
        from: FROM,
        to: TARGET,
        ty: EDGE_TYPE,
    };
    assert_eq!(completed.mutations(), [link]);
    let derived = host
        .derive_echo_operation_effects(&admitted_invocation, completed)
        .expect("the runtime owner derives the proposal");
    let expected = WarpOp::UpsertEdge {
        warp_id: WARP,
        record: EdgeRecord {
            id: link_edge(),
            from: FROM,
            to: TARGET,
            ty: EDGE_TYPE,
        },
    };
    assert!(
        derived.patch().ops().contains(&expected),
        "the derived patch writes the link: {:?}",
        derived.patch().ops()
    );
}

#[test]
fn a_trusted_runtime_consumer_cannot_admit_another_package_under_the_link_policy() {
    let (host, _) = host();
    let policy = link_policy();
    // A different, valid public package (echo#982's read step).
    let other = hex::decode(OTHER_PACKAGE.trim()).expect("other package hex");
    let refusal = host
        .admit_echo_operation_package(&policy, other)
        .expect_err("a different package is refused");
    assert_eq!(
        refusal.kind(),
        EchoOperationAdmissionErrorKind::PackageIdentityMismatch
    );
}

// Each host owns its evaluation authority, so the public route cannot
// launder another runtime owner's work.
#[test]
fn a_trusted_runtime_consumer_cannot_derive_work_evaluated_by_another_host() {
    let (mut producing, head) = host();
    let (admitted_invocation, completed) = evaluate_link(&mut producing, head);
    let (mut other, other_head) = host();
    drop(evaluate_link(&mut other, other_head));
    let refusal = other
        .derive_echo_operation_effects(&admitted_invocation, completed)
        .expect_err("another host's evidence is refused");
    assert_eq!(refusal.kind(), EffectDerivationKind::AuthorityMismatch);
}
