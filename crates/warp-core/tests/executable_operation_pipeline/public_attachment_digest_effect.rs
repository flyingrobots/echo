// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! An authored program that bumps a Buffer by `ReplaceAtom` (echo#1011),
//! published by the public compiler, admitted, installed, run and derived by
//! the generic runtime.
//!
//! The program reads the Buffer atom through `echo.dpo@2.node-atom-read`,
//! computes `expectedAtomDigest` with `echo.dpo@2.attachmentValueDigest` over
//! the bytes it read, and replaces the atom with the caller's next encoding.
//! A control program under `controls/unread-bytes/` digests the next bytes
//! instead, which it never read.
//!
//! Oracles: the frozen `attachment-new.hex` vector, derived from an
//! independently assembled preimage, for a Buffer holding its atom; and the runtime owner's `ReplaceAtom` admission, which accepts
//! only the attachment digest of the atom at the evaluation basis.
use super::current_effect_publication_fixture as publication;
use echo_edict_canonical::CanonicalValueV1 as Value;
use warp_core::edict_effects::{
    CompletedEffectEvaluation, EffectDerivationKind, EffectFrameError, EffectMutation,
    TypedNodeRead,
};
use warp_core::edict_pure::{EvaluationError, EvaluationLimits};
use warp_core::{
    make_head_id, AdmittedEchoOperationInvocation, AtomPayload, AttachmentValue,
    EchoOperationAdmissionPolicy, EchoOperationApplicationBasisV1, EchoOperationBudgetV1,
    EchoOperationInvocationAdmissionPolicy, EngineBuilder, GraphStore, InboxPolicy, NodeId,
    NodeKey, NodeRecord, PlaybackMode, TrustedRuntimeHost, TypeId, WarpId, WarpOp, WorldlineId,
    WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

const PACKAGE: &str =
    include_str!("../fixtures/edict-attachment-digest/bump-executable-operation-package.hex");
const UNREAD: &str = include_str!(
    "../fixtures/edict-attachment-digest/controls/unread-bytes/unread-executable-operation-package.hex"
);
const ATTACHMENT_NEW: &str = include_str!("../fixtures/content-address-v1/attachment-new.hex");
const OPERATION: &str = "consumer.buffers.program@1.bump";
const AUTHORITY_LABEL: &[u8] = b"consumer.buffers.authority/v1";
const BUFFER_TYPE: TypeId = TypeId([0x7b; 32]);
/// The Buffer atom type, which is the frozen vector's atom type `02…`.
const ATOM_TYPE: TypeId = TypeId([0x02; 32]);
const WARP: WarpId = WarpId([0x71; 32]);
const ROOT: NodeId = NodeId([0x72; 32]);
const BUFFER: NodeId = NodeId([0x73; 32]);
/// The lawpack's budget ceiling, which admission and invocation delegate.
const ADMITTED: u64 = 4096;
const CURRENT: &[u8] = b"buffer version 1";
const NEXT: &[u8] = b"buffer version 2";

fn authority() -> [u8; 32] {
    publication::hash(b"echo:operation-profile:v1\0", AUTHORITY_LABEL)
}

fn budget(steps: u64) -> EchoOperationBudgetV1 {
    EchoOperationBudgetV1::new(steps, 4096, 4096)
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

fn host(atom_type: TypeId, atom: &[u8]) -> (TrustedRuntimeHost, WriterHeadKey) {
    let mut store = GraphStore::new(WARP);
    store.insert_node(ROOT, NodeRecord { ty: BUFFER_TYPE });
    store.insert_node(BUFFER, NodeRecord { ty: BUFFER_TYPE });
    store.set_node_attachment(
        BUFFER,
        Some(AttachmentValue::Atom(AtomPayload::new(
            atom_type,
            atom.to_vec().into(),
        ))),
    );
    let state = WorldlineState::from_root_store(store.clone(), ROOT).expect("state");
    let head = WriterHeadKey {
        worldline_id: WorldlineId::from_bytes([0x74; 32]),
        head_id: make_head_id("public-attachment-digest-effect"),
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
    let engine = EngineBuilder::new(store, ROOT).build();
    (
        TrustedRuntimeHost::new(runtime, engine).expect("host"),
        head,
    )
}

fn id(bytes: impl AsRef<[u8]>) -> Value {
    Value::Bytes(bytes.as_ref().to_vec())
}

fn input() -> Vec<u8> {
    publication::encoded(&publication::map([
        ("anchor", Value::Bytes(vec![0x75; 32])),
        (
            "buffer",
            publication::map([
                ("nodeId", id(BUFFER.0)),
                ("typeId", id(ATOM_TYPE.0)),
                ("warpId", id(WARP.0)),
            ]),
        ),
        ("next", id(NEXT)),
    ]))
}

fn application_basis(input: &[u8]) -> EchoOperationApplicationBasisV1 {
    EchoOperationApplicationBasisV1::new(
        publication::hash(
            b"echo:operation-profile:v1\0",
            b"echo.operation.basis.edict-effect/v1",
        ),
        publication::hash(b"echo:operation-edict-effect-basis:v1\0", input),
    )
}

fn invocation(
    installed: &warp_core::InstalledEchoOperation,
    head: WriterHeadKey,
    basis: warp_core::EchoOperationEvaluationBasisV1,
    input: &[u8],
    steps: u64,
) -> Vec<u8> {
    let app = basis.application_basis();
    publication::encoded(&publication::map([
        ("schema", publication::text("echo.operation-invocation/v1")),
        ("kind", publication::text("edict-effect-program/v1")),
        ("package_id", id(installed.package_identity())),
        (
            "operation_coordinate",
            publication::text(installed.operation_coordinate()),
        ),
        ("authority_grant_identity", id([9; 32])),
        (
            "delegated_budget",
            publication::map([
                ("steps", Value::Integer(i128::from(steps))),
                ("read_bytes", Value::Integer(4096)),
                ("write_bytes", Value::Integer(4096)),
            ]),
        ),
        ("application_input_bytes", Value::Bytes(input.to_vec())),
        (
            "evaluation_basis",
            publication::map([
                ("worldline_id", id(head.worldline_id.as_bytes())),
                ("head_id", id(head.head_id.as_bytes())),
                (
                    "worldline_tick",
                    Value::Integer(i128::from(basis.worldline_tick().as_u64())),
                ),
                ("commit_global_tick", Value::Null),
                ("state_root", id(basis.state_root())),
                ("commit_id", id(basis.commit_id())),
                (
                    "application_basis_schema_identity",
                    id(app.schema_identity()),
                ),
                ("application_basis_value_identity", id(app.value_identity())),
            ]),
        ),
    ]))
}

struct Run {
    host: TrustedRuntimeHost,
    output: Result<Vec<u8>, EffectFrameError>,
    completed: Option<(AdmittedEchoOperationInvocation, CompletedEffectEvaluation)>,
}

/// Admits, installs and evaluates `package` over a Buffer holding `atom`,
/// with `steps` for the evaluation frame.
fn run(package: &str, atom_type: TypeId, atom: &[u8], steps: u64) -> Run {
    let bytes = hex::decode(package.trim()).expect("retained hex");
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    let (mut host, head) = host(atom_type, atom);
    let policy =
        EchoOperationAdmissionPolicy::exact(identity, OPERATION, authority(), budget(ADMITTED));
    let admitted = host
        .admit_echo_operation_package(&policy, bytes)
        .expect("the exact policy admits the public package");
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
    let invocation = host
        .admit_echo_operation_invocation(
            &EchoOperationInvocationAdmissionPolicy::new(authority(), [9; 32], budget(ADMITTED)),
            &invocation(&installed, head, parent, &input, ADMITTED),
        )
        .expect("invocation");
    let (output, completed) = {
        let mut frame = host
            .effect_evaluation_frame_v1(parent, &[], 0, budget(steps), limits())
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        let output = frame
            .evaluate_effect(
                &program,
                &input,
                &[TypedNodeRead {
                    node: NodeKey {
                        warp_id: WARP,
                        local_id: BUFFER,
                    },
                    absent_node_type: BUFFER_TYPE,
                }],
            )
            .map(<[u8]>::to_vec);
        let completed = output.is_ok().then(|| {
            frame
                .into_effect_evaluation(1 << 20)
                .expect("completed evidence")
        });
        (output, completed)
    };
    Run {
        host,
        output,
        completed: completed.map(|completed| (invocation, completed)),
    }
}

fn digest_of(output: &[u8]) -> [u8; 32] {
    let Value::Bytes(digest) =
        echo_edict_canonical::decode_canonical_cbor_v1(output).expect("canonical output")
    else {
        panic!("the program returns its expected digest")
    };
    digest.try_into().expect("32 bytes")
}

#[test]
fn the_public_bump_digests_the_atom_it_read_as_the_frozen_vector() {
    // The Buffer holds exactly the vector's atom: type `02…`, bytes `new`.
    let run = run(PACKAGE, ATOM_TYPE, b"new", ADMITTED);
    assert_eq!(
        hex::encode(digest_of(&run.output.expect("bump"))),
        ATTACHMENT_NEW.trim()
    );
}

#[test]
fn the_public_bump_replaces_the_buffer_atom_it_read() {
    let run = run(PACKAGE, ATOM_TYPE, CURRENT, ADMITTED);
    let digest = digest_of(run.output.as_deref().expect("bump"));
    let (admitted, completed) = run.completed.expect("completed");
    let node = NodeKey {
        warp_id: WARP,
        local_id: BUFFER,
    };
    assert_eq!(
        completed.mutations(),
        [EffectMutation::ReplaceAtom {
            node,
            expected: digest,
            atom: AtomPayload::new(ATOM_TYPE, NEXT.to_vec().into()),
        }]
    );
    let derived = run
        .host
        .derive_echo_operation_effects(&admitted, completed)
        .expect("admission accepts the digest of the atom the program read");
    assert!(derived.patch().ops().iter().any(|op| matches!(
        op,
        WarpOp::SetAttachment { value: Some(AttachmentValue::Atom(atom)), .. }
            if atom.type_id == ATOM_TYPE && atom.bytes.as_ref() == NEXT
    )));
}

#[test]
fn a_digest_of_bytes_the_program_did_not_read_is_refused() {
    let run = run(UNREAD, ATOM_TYPE, CURRENT, ADMITTED);
    let (admitted, completed) = run.completed.expect("the control evaluates");
    assert_eq!(
        run.host
            .derive_echo_operation_effects(&admitted, completed)
            .map(|_| ())
            .map_err(|error| error.kind()),
        Err(EffectDerivationKind::PreconditionMismatch)
    );
}

#[test]
fn the_public_bump_refuses_an_insufficient_step_budget() {
    let run = run(PACKAGE, ATOM_TYPE, CURRENT, 64);
    assert_eq!(
        run.output.map(|_| ()),
        Err(EffectFrameError::from(EvaluationError::StepBudgetExceeded))
    );
}
