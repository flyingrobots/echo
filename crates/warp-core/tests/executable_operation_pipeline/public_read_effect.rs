// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! An authored effect program with an effect step under the `echo.dpo@2`
//! effect target profile (#985), published by the public compiler, admitted,
//! installed and run by the generic runtime.
//!
//! The package is unchanged public compiler output retained under
//! `fixtures/edict-effect-step/`; its identity is recorded here independently
//! of the bytes. The program reads one atom through
//! `echo.dpo@2.node-atom-read` and returns it, mapping a missing atom to
//! `consumer.effects@1.FactMissing` (reported under the source's `fx` alias, as
//! every authored obstruction is). Oracles: the authored program's own law,
//! written by hand, over a graph built here; the refusal is the runtime
//! owner's exact admission policy.
use super::current_effect_publication_fixture as publication;
use echo_edict_canonical::CanonicalValueV1 as Value;
use warp_core::edict_effects::{EffectFrameError, TypedNodeRead};
use warp_core::edict_pure::EvaluationLimits;
use warp_core::{
    make_head_id, AtomPayload, AttachmentValue, EchoOperationAdmissionErrorKind,
    EchoOperationAdmissionPolicy, EchoOperationApplicationBasisV1, EchoOperationBudgetV1,
    EngineBuilder, GraphStore, InboxPolicy, NodeId, NodeKey, NodeRecord, PlaybackMode,
    TrustedRuntimeHost, TypeId, WarpId, WorldlineId, WorldlineRuntime, WorldlineState, WriterHead,
    WriterHeadKey,
};

const PACKAGE: &str =
    include_str!("../fixtures/edict-effect-step/read-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "7a3955e1d08a5d16444c6374cb744fa073da7e197ea98ec6806a1158f745871b";
const OPERATION: &str = "consumer.effects.program@1.read";
const AUTHORITY_LABEL: &[u8] = b"consumer.effects.authority/v1";
/// The node and atom types the program's graph schema and footprint law name.
const NODE_TYPE: TypeId = TypeId([0x5b; 32]);
const ATOM_TYPE: TypeId = TypeId([0x5c; 32]);
const FACT: &[u8] = b"public fact";

fn retained() -> (Vec<u8>, [u8; 32]) {
    let bytes = hex::decode(PACKAGE.trim()).expect("retained hex");
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    assert_eq!(
        hex::encode(identity),
        PACKAGE_PIN,
        "retained package identity"
    );
    (bytes, identity)
}

/// The runtime owner's exact policy for the public package.
fn policy(identity: [u8; 32]) -> EchoOperationAdmissionPolicy {
    EchoOperationAdmissionPolicy::exact(
        identity,
        OPERATION,
        publication::hash(b"echo:operation-profile:v1\0", AUTHORITY_LABEL),
        EchoOperationBudgetV1::new(4096, 4096, 4096),
    )
}

/// A worldline whose root node carries the fact the program reads.
pub(super) fn host() -> (TrustedRuntimeHost, WriterHeadKey, NodeKey) {
    let node = NodeKey {
        warp_id: WarpId([0x31; 32]),
        local_id: NodeId([0x32; 32]),
    };
    let mut store = GraphStore::new(node.warp_id);
    store.insert_node(node.local_id, NodeRecord { ty: NODE_TYPE });
    store.set_node_attachment(
        node.local_id,
        Some(AttachmentValue::Atom(AtomPayload {
            type_id: ATOM_TYPE,
            bytes: FACT.to_vec().into(),
        })),
    );
    let state = WorldlineState::from_root_store(store.clone(), node.local_id).expect("state");
    let head = WriterHeadKey {
        worldline_id: WorldlineId::from_bytes([0x33; 32]),
        head_id: make_head_id("public-read-effect"),
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
        .expect("head");
    let engine = EngineBuilder::new(store, node.local_id).build();
    (
        TrustedRuntimeHost::new(runtime, engine).expect("host"),
        head,
        node,
    )
}

fn input(node: NodeKey) -> Vec<u8> {
    publication::encoded(&publication::map([
        ("anchor", Value::Bytes(vec![0x5a; 32])),
        (
            "address",
            publication::map([
                ("nodeId", Value::Bytes(node.local_id.0.to_vec())),
                ("typeId", Value::Bytes(ATOM_TYPE.0.to_vec())),
                ("warpId", Value::Bytes(node.warp_id.0.to_vec())),
            ]),
        ),
    ]))
}

pub(super) fn basis(input: &[u8]) -> EchoOperationApplicationBasisV1 {
    EchoOperationApplicationBasisV1::new(
        publication::hash(
            b"echo:operation-profile:v1\0",
            b"echo.operation.basis.edict-effect/v1",
        ),
        publication::hash(b"echo:operation-edict-effect-basis:v1\0", input),
    )
}

pub(super) fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 1_048_576,
        max_input_bytes: 65_536,
        max_steps: 4_096,
        max_allocated_bytes: 1_048_576,
        max_output_bytes: 65_536,
    }
}

/// Admits and installs the public package, then evaluates it for `node`.
fn run(node_of: impl FnOnce(NodeKey) -> NodeKey) -> Result<Vec<u8>, EffectFrameError> {
    let (bytes, identity) = retained();
    let (mut host, head, root) = host();
    let admitted = host
        .admit_echo_operation_package(&policy(identity), bytes)
        .expect("the exact policy admits the public package");
    let installed = host
        .install_admitted_echo_operation_package(admitted)
        .expect("installation");
    let program = installed
        .program()
        .as_effect_program()
        .expect("an effect program");
    let node = node_of(root);
    let input = input(node);
    let parent = host
        .echo_operation_evaluation_basis_v1(head, basis(&input))
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            parent,
            &[],
            0,
            EchoOperationBudgetV1::new(4096, 4096, 4096),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let output = frame
        .evaluate_effect(
            program,
            &input,
            &[TypedNodeRead {
                node,
                absent_node_type: NODE_TYPE,
            }],
        )?
        .to_vec();
    let completed = frame
        .into_effect_evaluation(4096)
        .expect("completed evidence");
    assert_eq!(completed.output(), output.as_slice());
    assert!(completed.mutations().is_empty(), "a read writes nothing");
    assert_eq!(completed.input_basis(), basis(&input));
    Ok(output)
}

#[test]
fn the_public_read_effect_package_is_admitted_installed_and_reads_its_fact() {
    assert_eq!(
        run(|root| root).expect("the program reads the fact"),
        publication::encoded(&Value::Bytes(FACT.to_vec()))
    );
}

#[test]
fn the_public_read_effect_maps_a_missing_atom_to_its_declared_obstruction() {
    let missing = |root: NodeKey| NodeKey {
        local_id: NodeId([0x34; 32]),
        ..root
    };
    assert_eq!(
        run(missing),
        Err(EffectFrameError::AuthoredObstruction {
            reason_kind: "fx.FactMissing".into(),
            payload: publication::encoded(&publication::map([])),
        })
    );
}

#[test]
fn another_package_is_refused_under_the_public_read_effect_policy() {
    let (_, identity) = retained();
    let (host, _, _) = host();
    let other = hex::decode(
        include_str!("../fixtures/edict-effect-program/flat-executable-operation-package.hex")
            .trim(),
    )
    .expect("flat package");
    assert_eq!(
        host.admit_echo_operation_package(&policy(identity), other)
            .expect_err("refused at admission")
            .kind(),
        EchoOperationAdmissionErrorKind::PackageIdentityMismatch
    );
}
