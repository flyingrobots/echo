// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Public compiler output for authored `ContentAddressV1` identity calls
//! under `echo.dpo@2`. The oracle is the six frozen identity vectors in
//! `tests/fixtures/content-address-v1/`, derived from independently assembled
//! preimages; nothing here recomputes an expected identity.
use super::current_effect_publication_fixture as publication;
use super::public_read_effect::{basis, host, limits};
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as Value};
use warp_core::edict_effects::{EffectFrameError, EffectMutation};
use warp_core::edict_pure::EvaluationError;
use warp_core::{EchoOperationAdmissionPolicy, EchoOperationBudgetV1, NodeKey, TypeId, WarpId};

const PACKAGE: &str =
    include_str!("../fixtures/edict-content-address/identity-executable-operation-package.hex");
const OPERATION: &str = "consumer.identity.program@1.ids";
const AUTHORITY: &[u8] = b"consumer.identity.authority/v1";
const STORED_TYPE: TypeId = TypeId([0x5b; 32]);
const STORED_ATOM_TYPE: TypeId = TypeId([0x5c; 32]);
const ATOM: &[u8] = b"new";

/// The six frozen vectors, by the fixture output that must equal each.
const VECTORS: [(&str, &str); 6] = [
    (
        "nodeZeroEdges",
        include_str!("../fixtures/content-address-v1/node-zero-edges.hex"),
    ),
    (
        "nodeOneEdge",
        include_str!("../fixtures/content-address-v1/node-one-edge.hex"),
    ),
    (
        "nodeTwoEdges",
        include_str!("../fixtures/content-address-v1/node-two-edges.hex"),
    ),
    (
        "nodeTwoEdgesReordered",
        include_str!("../fixtures/content-address-v1/node-two-edges.hex"),
    ),
    (
        "contentEdge",
        include_str!("../fixtures/content-address-v1/content-edge.hex"),
    ),
    (
        "slotEdge",
        include_str!("../fixtures/content-address-v1/slot-edge.hex"),
    ),
];

fn retained() -> (Vec<u8>, [u8; 32]) {
    let bytes = hex::decode(PACKAGE.trim()).expect("retained public hex");
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    (bytes, identity)
}

fn policy(identity: [u8; 32]) -> EchoOperationAdmissionPolicy {
    EchoOperationAdmissionPolicy::exact(
        identity,
        OPERATION,
        publication::hash(b"echo:operation-profile:v1\0", AUTHORITY),
        EchoOperationBudgetV1::new(4096, 4096, 4096),
    )
}

/// The fixture README's inputs: 32 repetitions of the named byte.
fn input(edge_b: u8, low: u8) -> Vec<u8> {
    let id = |byte: u8| Value::Bytes(vec![byte; 32]);
    publication::encoded(&publication::map([
        ("warp", id(0x03)),
        ("nodeType", id(0x07)),
        ("atomType", id(0x02)),
        ("atom", Value::Bytes(ATOM.to_vec())),
        ("edgeA", id(0x06)),
        ("edgeB", id(edge_b)),
        ("low", id(low)),
        ("high", id(0x05)),
        ("storedType", Value::Bytes(STORED_TYPE.0.to_vec())),
        ("storedAtomType", Value::Bytes(STORED_ATOM_TYPE.0.to_vec())),
    ]))
}

fn run(input: &[u8], steps: u64) -> Result<(Value, Vec<EffectMutation>), EffectFrameError> {
    let (bytes, identity) = retained();
    let (mut host, head, _) = host();
    let admitted = host
        .admit_echo_operation_package(&policy(identity), bytes)
        .expect("exact public package admission");
    let installed = host
        .install_admitted_echo_operation_package(admitted)
        .expect("public package installation");
    let program = installed
        .program()
        .as_effect_program()
        .expect("effect program");
    let parent = host
        .echo_operation_evaluation_basis_v1(head, basis(input))
        .expect("basis");
    let mut frame = host
        .effect_evaluation_frame_v1(
            parent,
            &[],
            0,
            EchoOperationBudgetV1::new(steps, 4096, 4096),
            limits(),
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let output = frame.evaluate_effect(program, input, &[])?.to_vec();
    let buffered = frame.buffered_mutations().to_vec();
    Ok((decode(&output).expect("canonical output"), buffered))
}

#[test]
fn the_public_identity_package_returns_the_six_frozen_vectors() {
    let (output, _) = run(&input(0x08, 0x04), 4096).expect("public identity execution");
    for (field, vector) in VECTORS {
        assert_eq!(
            publication::get(&output, field),
            &Value::Bytes(hex::decode(vector.trim()).expect("frozen vector")),
            "{field}"
        );
    }
}

#[test]
fn the_public_identity_package_ensures_the_node_at_its_computed_address() {
    let (_, buffered) = run(&input(0x08, 0x04), 4096).expect("public identity execution");
    let [EffectMutation::EnsureNode { node, ty, atom }] = buffered.as_slice() else {
        panic!("exactly one ensure-node effect: {buffered:?}")
    };
    // Consistency, not an independent vector: the effect names the node the
    // authored `contentAddress` call computed under the stored types.
    let expected = warp_core::content_address_v1::content_node_id(
        WarpId([0x03; 32]),
        STORED_TYPE,
        STORED_ATOM_TYPE,
        ATOM,
        &[],
    )
    .expect("no edges");
    assert_eq!(
        *node,
        NodeKey {
            warp_id: WarpId([0x03; 32]),
            local_id: expected,
        }
    );
    assert_eq!(*ty, STORED_TYPE);
    assert_eq!(atom.type_id, STORED_ATOM_TYPE);
}

#[test]
fn the_public_identity_package_refuses_duplicate_outgoing_edges() {
    // edgeB = edgeA and low = high make `[b, a]` name one edge twice.
    assert_eq!(
        run(&input(0x06, 0x05), 4096).map(|_| ()),
        Err(EffectFrameError::from(
            EvaluationError::DuplicateOutgoingEdge
        ))
    );
}

#[test]
fn the_public_identity_package_refuses_an_insufficient_step_budget() {
    assert_eq!(
        run(&input(0x08, 0x04), 64).map(|_| ()),
        Err(EffectFrameError::from(EvaluationError::StepBudgetExceeded))
    );
}
