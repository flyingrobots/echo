// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Public compiler output that calls the `echo.dpo@2` identity constructor
//! `contentAddress` from a source helper and from a fold step
//! (flyingrobots/echo#1005), and warp-core's verdicts on the shared
//! helper-identity corpus, which the provider lowerer and verifier must match.
//!
//! Oracle: the frozen `node-zero-edges` vector in
//! `tests/fixtures/content-address-v1/`, derived from an independently
//! assembled preimage, for the helper's id. The folded id has no frozen
//! vector; its check is consistency with the identity law the runtime
//! publishes (`content_address_v1`), labelled where it is made.
use super::current_effect_publication_fixture as publication;
use super::public_read_effect::{basis, host, limits};
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as Value};
use warp_core::content_address_v1::{content_node_id, OutgoingEdge};
use warp_core::edict_effects::{EffectFrameError, EffectMutation};
use warp_core::{EchoOperationAdmissionPolicy, EchoOperationBudgetV1, NodeId, TypeId, WarpId};

#[path = "../../../echo-edict-provider-lowerer/tests/fixtures/helper_identity_corpus.rs"]
#[allow(dead_code, reason = "the compiled program serves the lowerer")]
mod corpus;

const PACKAGE: &str =
    include_str!("../fixtures/edict-helper-identity/helper-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "703b0377a51065211576fd86c1521a4305a400bbb7a29c3474b2114fa9311936";
const OPERATION: &str = "consumer.identity.program@1.ids";
const AUTHORITY: &[u8] = b"consumer.identity.authority/v1";
const STORED_TYPE: TypeId = TypeId([0x5b; 32]);
const STORED_ATOM_TYPE: TypeId = TypeId([0x5c; 32]);
const ATOM: &[u8] = b"new";
const NODE_ZERO_EDGES: &str = include_str!("../fixtures/content-address-v1/node-zero-edges.hex");

fn retained() -> (Vec<u8>, [u8; 32]) {
    let bytes = hex::decode(PACKAGE.trim()).expect("retained public hex");
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    assert_eq!(
        hex::encode(identity),
        PACKAGE_PIN,
        "retained package identity"
    );
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

/// The content-address fixture's inputs: 32 repetitions of the named byte.
fn input() -> Vec<u8> {
    let id = |byte: u8| Value::Bytes(vec![byte; 32]);
    publication::encoded(&publication::map([
        ("warp", id(0x03)),
        ("nodeType", id(0x07)),
        ("atomType", id(0x02)),
        ("atom", Value::Bytes(ATOM.to_vec())),
        ("edgeA", id(0x06)),
        ("storedType", Value::Bytes(STORED_TYPE.0.to_vec())),
        ("storedAtomType", Value::Bytes(STORED_ATOM_TYPE.0.to_vec())),
    ]))
}

fn run() -> Result<(Value, Vec<EffectMutation>), EffectFrameError> {
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
    let input = input();
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
    let output = frame.evaluate_effect(program, &input, &[])?.to_vec();
    let buffered = frame.buffered_mutations().to_vec();
    Ok((decode(&output).expect("canonical output"), buffered))
}

fn id(value: &Value) -> NodeId {
    let Value::Bytes(bytes) = value else {
        panic!("an id")
    };
    NodeId(bytes.as_slice().try_into().expect("32 bytes"))
}

#[test]
fn a_helper_computes_the_frozen_zero_edge_address() {
    let (output, _) = run().expect("public helper identity execution");
    assert_eq!(
        publication::get(&output, "helperNode"),
        &Value::Bytes(hex::decode(NODE_ZERO_EDGES.trim()).expect("frozen vector")),
    );
}

#[test]
fn a_fold_step_chains_each_address_into_the_next() {
    let (output, _) = run().expect("public helper identity execution");
    // Consistency with the published identity law, not an independent vector:
    // each step addresses a node whose one `edgeA` edge points at the previous id.
    let warp = WarpId([0x03; 32]);
    let step = |previous: NodeId| {
        content_node_id(
            warp,
            TypeId([0x07; 32]),
            TypeId([0x02; 32]),
            ATOM,
            &[OutgoingEdge {
                edge_type: TypeId([0x06; 32]),
                to: previous,
            }],
        )
        .expect("one edge")
    };
    let helper = id(publication::get(&output, "helperNode"));
    assert_eq!(id(publication::get(&output, "folded")), step(step(helper)));
}

#[test]
fn the_effect_ensures_the_node_a_helper_addressed() {
    let (_, buffered) = run().expect("public helper identity execution");
    let [EffectMutation::EnsureNode { node, ty, .. }] = buffered.as_slice() else {
        panic!("exactly one ensure-node effect: {buffered:?}")
    };
    // Consistency: the stored node's id is the helper's address under the
    // stored types.
    let expected = content_node_id(WarpId([0x03; 32]), STORED_TYPE, STORED_ATOM_TYPE, ATOM, &[])
        .expect("no edges");
    assert_eq!(node.local_id, expected);
    assert_eq!(*ty, STORED_TYPE);
}

#[test]
fn warp_core_reaches_every_shared_helper_identity_verdict() {
    let all = corpus::cases();
    let mismatched: Vec<_> = all
        .iter()
        .filter(|case| {
            let identity = publication::hash(b"echo:operation-package:v1\0", &case.package);
            let (host, _, _) = host();
            host.admit_echo_operation_package(&policy(identity), case.package.clone())
                .is_ok()
                != case.accepted
        })
        .map(|case| (case.name, case.accepted))
        .collect();
    assert_eq!(mismatched, Vec::<(&str, bool)>::new(), "(case, expected)");
    assert_eq!(all.len(), 5, "the corpus has every case");
}
