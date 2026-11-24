// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The public compiler's identity-intrinsic witnesses under the shared-read
//! schedule (echo#1016, family 3).
//!
//! `fixtures/edict-content-address` calls `contentAddress` (with zero, one
//! and two edges), `contentEdgeId` and `slotEdgeId`, then ensures a node;
//! `fixtures/edict-attachment-digest` reads a Buffer atom, digests it with
//! `attachmentValueDigest` and replaces it. Both packages are unchanged
//! public compiler output, repinned in place to require
//! `core.cost.shared-reads/v1` with exactly the lowerer's shared minimum as
//! their Core budget. Oracles: the copying executor on the same input and
//! graph (a differential), and the lowerer minimums that
//! `identity_witness_minimums_under_each_schedule` in
//! `echo-edict-provider-lowerer` measures and pins.
use super::closure_witness::repinned;
use super::*;
use warp_core::edict_effects::TypedNodeRead;
use warp_core::{
    AtomPayload, AttachmentValue, GraphStore, NodeId, NodeKey, NodeRecord, TypeId, WarpId,
};

/// One witness: its package, the graph it runs on, its input, its typed
/// reads and the lowerer's shared minimum (steps, bytes).
pub(super) struct Witness {
    pub(super) package: &'static str,
    pub(super) warp: WarpId,
    pub(super) store: fn() -> GraphStore,
    pub(super) root: NodeId,
    pub(super) input: fn() -> Vec<u8>,
    pub(super) reads: &'static [(NodeId, TypeId)],
    pub(super) minimum: (u64, u64),
}

const IDENTITY_WARP: WarpId = WarpId([0x03; 32]);
const IDENTITY_ROOT: NodeId = NodeId([0x31; 32]);
const BUFFER_WARP: WarpId = WarpId([0x71; 32]);
const BUFFER_ROOT: NodeId = NodeId([0x72; 32]);
const BUFFER: NodeId = NodeId([0x73; 32]);
const BUFFER_TYPE: TypeId = TypeId([0x7b; 32]);

fn id(byte: u8) -> V {
    V::Bytes(vec![byte; 32])
}

fn identity_store() -> GraphStore {
    let mut store = GraphStore::new(IDENTITY_WARP);
    store.insert_node(
        IDENTITY_ROOT,
        NodeRecord {
            ty: TypeId([0x5b; 32]),
        },
    );
    store
}

/// The identity fixture README's inputs.
fn identity_input() -> Vec<u8> {
    encoded(&map([
        ("warp", id(0x03)),
        ("nodeType", id(0x07)),
        ("atomType", id(0x02)),
        ("atom", V::Bytes(b"new".to_vec())),
        ("edgeA", id(0x06)),
        ("edgeB", id(0x08)),
        ("low", id(0x04)),
        ("high", id(0x05)),
        ("storedType", id(0x5b)),
        ("storedAtomType", id(0x5c)),
    ]))
}

fn buffer_store() -> GraphStore {
    let mut store = GraphStore::new(BUFFER_WARP);
    store.insert_node(BUFFER_ROOT, NodeRecord { ty: BUFFER_TYPE });
    store.insert_node(BUFFER, NodeRecord { ty: BUFFER_TYPE });
    store.set_node_attachment(
        BUFFER,
        Some(AttachmentValue::Atom(AtomPayload::new(
            TypeId([0x02; 32]),
            b"buffer version 1".to_vec().into(),
        ))),
    );
    store
}

fn buffer_input() -> Vec<u8> {
    encoded(&map([
        ("anchor", V::Bytes(vec![0x75; 32])),
        (
            "buffer",
            map([
                ("nodeId", V::Bytes(BUFFER.0.to_vec())),
                ("typeId", id(0x02)),
                ("warpId", V::Bytes(BUFFER_WARP.0.to_vec())),
            ]),
        ),
        ("next", V::Bytes(b"buffer version 2".to_vec())),
    ]))
}

const WITNESSES: [(&str, Witness); 2] = [
    (
        "identity",
        Witness {
            package: include_str!(
                "../../fixtures/edict-content-address/identity-executable-operation-package.hex"
            ),
            warp: IDENTITY_WARP,
            store: identity_store,
            root: IDENTITY_ROOT,
            input: identity_input,
            reads: &[],
            minimum: (1246, 11_580),
        },
    ),
    (
        "attachment digest",
        Witness {
            package: include_str!(
                "../../fixtures/edict-attachment-digest/bump-executable-operation-package.hex"
            ),
            warp: BUFFER_WARP,
            store: buffer_store,
            root: BUFFER_ROOT,
            input: buffer_input,
            reads: &[(BUFFER, BUFFER_TYPE)],
            minimum: (283, 3209),
        },
    ),
];

/// Runs `package` as `witness`, returning its output and the steps and bytes
/// the runtime charged.
pub(super) fn run(witness: &Witness, package: &V) -> (Result<Vec<u8>, EffectFrameError>, u64, u64) {
    let prepared = prepare(package).expect("package prepares");
    let program = prepared.program().as_effect_program().expect("effect");
    let root = NodeKey {
        warp_id: witness.warp,
        local_id: witness.root,
    };
    let (host, head, _) = super::super::execution::host_with_store((witness.store)(), root);
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
        )
        .expect("basis");
    let frame = Frame::widest(None);
    let mut evaluation = host
        .effect_evaluation_frame_v1(
            basis,
            &[],
            0,
            EchoOperationBudgetV1::new(frame.steps, 64 * 1024 * 1024, frame.write_bytes),
            frame.limits,
        )
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    let reads: Vec<_> = witness
        .reads
        .iter()
        .map(|(node, ty)| TypedNodeRead {
            node: NodeKey {
                warp_id: witness.warp,
                local_id: *node,
            },
            absent_node_type: *ty,
        })
        .collect();
    let result = evaluation
        .evaluate_effect(program, &(witness.input)(), &reads)
        .map(<[u8]>::to_vec);
    (
        result,
        evaluation.consumed().steps(),
        evaluation.allocated_bytes(),
    )
}

/// Given exactly the lowerer's shared minimum as its Core budget, each
/// witness computes the copying schedule's identities, and the runtime's
/// charge stays at or below that static bound on both axes.
#[test]
fn the_identity_witnesses_run_within_the_lowerer_minimum() {
    for (name, witness) in &WITNESSES {
        let package = echo_edict_canonical::decode_canonical_cbor_v1(
            &hex::decode(witness.package.trim()).expect("hex"),
        )
        .expect("canonical package");
        let (copying, _, _) = run(witness, &package);
        let expected = copying.unwrap_or_else(|error| panic!("{name}: copying: {error:?}"));
        let (control, _, _) = run(witness, &repinned(&package, "core.variant/v1", None));
        assert_eq!(
            control.as_deref(),
            Ok(expected.as_slice()),
            "{name}: control"
        );
        let (steps, bytes) = witness.minimum;
        let exact = repinned(&package, "core.cost.shared-reads/v1", Some((steps, bytes)));
        let (sharing, charged_steps, charged_bytes) = run(witness, &exact);
        assert_eq!(
            sharing.as_deref(),
            Ok(expected.as_slice()),
            "{name}: result"
        );
        assert!(
            charged_steps <= steps && charged_bytes <= bytes,
            "{name}: runtime charge ({charged_steps}, {charged_bytes}) exceeds the static bound ({steps}, {bytes})"
        );
    }
}
