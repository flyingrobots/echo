// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! An authored program calling all five `echo.dpo@2` mutation intrinsics
//! without a failure map (#985, amblicit#311), published by the public
//! compiler, admitted, installed and run by the generic runtime.
//!
//! The package is unchanged public compiler output retained under
//! `fixtures/edict-effect-mutations/`; its identity is recorded here
//! independently of the bytes. Oracle: the authored program's own law, written
//! by hand: one intent per mutation, in source order, carrying exactly the
//! input fields each mutation names.
use super::current_effect_publication_fixture as publication;
use super::public_read_effect::{basis, host, limits};
use echo_edict_canonical::CanonicalValueV1 as Value;
use warp_core::edict_effects::EffectMutation;
use warp_core::{
    AtomPayload, EchoOperationAdmissionPolicy, EchoOperationBudgetV1, EdgeId, EdgeKey, NodeId,
    NodeKey, TypeId,
};

const PACKAGE: &str =
    include_str!("../fixtures/edict-effect-mutations/write-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "06f14e50c1b46f71f2b413b4262ac9622877ac0e6895cca00a36b33c45daa55f";
const OPERATION: &str = "consumer.writes.program@1.write";
const AUTHORITY_LABEL: &[u8] = b"consumer.writes.authority/v1";
const NODE_TYPE: TypeId = TypeId([0x5b; 32]);
const ATOM_TYPE: TypeId = TypeId([0x5c; 32]);
const EDGE_TYPE: TypeId = TypeId([0x5d; 32]);

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

fn id(byte: u8) -> Value {
    Value::Bytes(vec![byte; 32])
}

fn record(fields: Vec<(&'static str, Value)>) -> Value {
    publication::map(fields)
}

#[test]
fn the_public_mutation_package_emits_each_intent_in_source_order() {
    let (bytes, identity) = retained();
    let (mut host, head, root) = host();
    let warp = Value::Bytes(root.warp_id.0.to_vec());
    let policy = EchoOperationAdmissionPolicy::exact(
        identity,
        OPERATION,
        publication::hash(b"echo:operation-profile:v1\0", AUTHORITY_LABEL),
        EchoOperationBudgetV1::new(4096, 4096, 4096),
    );
    let admitted = host
        .admit_echo_operation_package(&policy, bytes)
        .expect("the exact policy admits the public package");
    let installed = host
        .install_admitted_echo_operation_package(admitted)
        .expect("installation");
    let program = installed
        .program()
        .as_effect_program()
        .expect("an effect program");
    let node_write = |node: u8, atom: &[u8]| {
        record(vec![
            ("warpId", warp.clone()),
            ("nodeId", id(node)),
            ("nodeType", Value::Bytes(NODE_TYPE.0.to_vec())),
            ("atomType", Value::Bytes(ATOM_TYPE.0.to_vec())),
            ("atomBytes", Value::Bytes(atom.to_vec())),
        ])
    };
    let input = publication::encoded(&record(vec![
        ("anchor", id(0x40)),
        ("node", node_write(0x41, b"ensured")),
        ("created", node_write(0x42, b"created")),
        (
            "atom",
            record(vec![
                ("warpId", warp.clone()),
                ("nodeId", id(0x41)),
                ("expectedAtomDigest", id(0x43)),
                ("atomType", Value::Bytes(ATOM_TYPE.0.to_vec())),
                ("atomBytes", Value::Bytes(b"replaced".to_vec())),
            ]),
        ),
        (
            "edge",
            record(vec![
                ("warpId", warp.clone()),
                ("edgeId", id(0x44)),
                ("fromId", id(0x41)),
                ("toId", id(0x42)),
                ("edgeType", Value::Bytes(EDGE_TYPE.0.to_vec())),
            ]),
        ),
        (
            "retarget",
            record(vec![
                ("warpId", warp),
                ("edgeId", id(0x44)),
                ("expectedToId", id(0x42)),
                ("toId", id(0x41)),
            ]),
        ),
    ]));
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
        .evaluate_effect(program, &input, &[])
        .expect("the program runs")
        .to_vec();
    assert_eq!(output, publication::encoded(&publication::map([])));
    let node = |byte: u8| NodeKey {
        warp_id: root.warp_id,
        local_id: NodeId([byte; 32]),
    };
    let edge = EdgeKey {
        warp_id: root.warp_id,
        local_id: EdgeId([0x44; 32]),
    };
    let atom = |bytes: &[u8]| AtomPayload {
        type_id: ATOM_TYPE,
        bytes: bytes.to_vec().into(),
    };
    let expected = vec![
        EffectMutation::EnsureNode {
            node: node(0x41),
            ty: NODE_TYPE,
            atom: atom(b"ensured"),
        },
        EffectMutation::CreateNode {
            node: node(0x42),
            ty: NODE_TYPE,
            atom: atom(b"created"),
        },
        EffectMutation::ReplaceAtom {
            node: node(0x41),
            expected: [0x43; 32],
            atom: atom(b"replaced"),
        },
        EffectMutation::EnsureEdge {
            edge,
            from: NodeId([0x41; 32]),
            to: NodeId([0x42; 32]),
            ty: EDGE_TYPE,
        },
        EffectMutation::ReplaceEdge {
            edge,
            expected_to: NodeId([0x42; 32]),
            to: NodeId([0x41; 32]),
        },
    ];
    assert_eq!(frame.buffered_mutations(), expected.as_slice());
    let completed = frame
        .into_effect_evaluation(4096)
        .expect("completed evidence");
    assert_eq!(completed.mutations(), expected.as_slice());
}
